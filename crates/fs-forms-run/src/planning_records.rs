//! The records of the planning module: [`PlanningRecords`] takes the board from the service of `fs-modules-planning` (through its
//! client crate, which the host LINKS for now: transition for K2/K3, a host that loads modules at run time replaces that).
//!
//! The client works on a `LocalSet` (capnp-rpc), the port is synchronous: the client lives on a thread of its own and the port
//! sends it one job at a time and waits for the answer. `can` is what the service reported at the connect (add card, move, add
//! column, rename, limit: the limit only when it names `column.limit`); a conflict of the revision is a refusal, never silence.
//!
//! Pattern: Adapter (the port `Records` over the client) + Active object (one worker thread owns the connection).

use std::path::Path;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use fs_components_kanban::{Answer, Card, Column, Operation};
use fs_modules_planning_client as client;
use fs_rpc::{Expected, SingleTimes};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use crate::records::{Records, SideItem};

/// How long one call may take before the service counts as gone (a dead service must not freeze the window).
const CALL: Duration = Duration::from_secs(10);

type Reply<T> = SyncSender<Result<T, client::Error>>;

/// One job for the thread that owns the client.
enum Job {
    Columns(Reply<Vec<client::Column>>),
    AddCard(String, String, Reply<()>),
    Move(String, String, Reply<()>),
    AddColumn(String, Reply<()>),
    Rename(String, String, Reply<()>),
    Limit(String, Option<u32>, Reply<()>),
}

/// The board of the planning service as the records of a window.
pub struct PlanningRecords {
    jobs: UnboundedSender<Job>,
    can: Vec<Operation>,
    /// The columns of the last answer: shown again when a later read fails (the failure itself shows at the next change).
    last: Mutex<Vec<Column>>,
}

fn unreachable_error(text: &str) -> client::Error {
    client::Error {
        kind: client::Kind::Unreachable,
        text: text.to_owned(),
    }
}

/// The code (tail of `kanban.refused.<code>`) the board shows for an error of the client.
fn code(kind: client::Kind) -> &'static str {
    match kind {
        client::Kind::Unreachable => "unreachable",
        client::Kind::Conflict => "conflict",
        client::Kind::Rejected => "rejected",
        client::Kind::Unsupported => "unsupported",
        client::Kind::NoBoard => "no-board",
        client::Kind::Failed => "failed",
    }
}

fn answer(result: Result<(), client::Error>) -> Answer {
    match result {
        Ok(()) => Answer::Accepted,
        Err(e) => Answer::Refused(code(e.kind).to_owned()),
    }
}

fn column_of(c: &client::Column) -> Column {
    let mut out = Column::new(&c.id, &c.name);
    out.limit = c.limit.and_then(|l| usize::try_from(l).ok());
    out.cards = c
        .cards
        .iter()
        .map(|card| Card::new(&card.uid, &card.title))
        .collect();
    out
}

/// The thread of the client: reach (or start) the service, connect, report what it can, then serve the jobs until the records go.
fn work(
    socket: &Path,
    running: &Path,
    mut jobs: tokio::sync::mpsc::UnboundedReceiver<Job>,
    ready: &SyncSender<Result<Vec<Operation>, client::Error>>,
) {
    // `ensure_service_found` builds a runtime of its own: it must run outside the async context below
    let expected = Expected::new("").with_program(client::SERVICE_PROGRAM);
    if let Err(e) =
        client::ensure_service_found(socket, &expected, running, &SingleTimes::default())
    {
        let _ = ready.send(Err(e));
        return;
    }
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            let _ = ready.send(Err(unreachable_error(&e.to_string())));
            return;
        }
    };
    tokio::task::LocalSet::new().block_on(&runtime, async {
        let mut planning = match client::Planning::connect(socket).await {
            Ok(p) => p,
            Err(e) => {
                let _ = ready.send(Err(e));
                return;
            }
        };
        let can = Operation::ALL
            .into_iter()
            .filter(|op| planning.can(client_op(*op)))
            .collect();
        let _ = ready.send(Ok(can));
        while let Some(job) = jobs.recv().await {
            match job {
                Job::Columns(r) => {
                    let _ = r.send(planning.columns().await);
                }
                Job::AddCard(column, title, r) => {
                    let _ = r.send(planning.add_card(&column, &title).await.map(drop));
                }
                Job::Move(key, to, r) => {
                    let _ = r.send(planning.move_card(&key, &to, None).await);
                }
                Job::AddColumn(name, r) => {
                    let _ = r.send(planning.add_column(&name).await.map(drop));
                }
                Job::Rename(column, name, r) => {
                    let _ = r.send(planning.rename_column(&column, &name).await);
                }
                Job::Limit(column, limit, r) => {
                    let _ = r.send(planning.set_limit(&column, limit).await);
                }
            }
        }
    });
}

/// The operation of the client behind an operation of the port.
fn client_op(op: Operation) -> client::Operation {
    match op {
        Operation::AddCard => client::Operation::AddCard,
        Operation::Move => client::Operation::Move,
        Operation::AddColumn => client::Operation::AddColumn,
        Operation::Rename => client::Operation::Rename,
        Operation::Limit => client::Operation::Limit,
    }
}

impl PlanningRecords {
    /// Reach the service on `socket` (starting `fs-modules-planning` from next to `running`, or from `/usr/bin`, when nobody
    /// serves it), connect and ask what it can do.
    ///
    /// # Errors
    /// [`client::Kind::Unreachable`]: no service could be reached or started, or it does not answer.
    pub fn connect(socket: &Path, running: &Path) -> Result<Self, client::Error> {
        let (jobs, rx) = unbounded_channel();
        let (ready, heard) = sync_channel(1);
        let (socket, running) = (socket.to_path_buf(), running.to_path_buf());
        std::thread::Builder::new()
            .name("planning-client".into())
            .spawn(move || work(&socket, &running, rx, &ready))
            .map_err(|e| unreachable_error(&e.to_string()))?;
        let can = heard
            .recv()
            .map_err(|_| unreachable_error("the client thread ended"))??;
        Ok(Self {
            jobs,
            can,
            last: Mutex::new(Vec::new()),
        })
    }

    /// Send a job and wait for its answer.
    fn ask<T>(&self, job: impl FnOnce(Reply<T>) -> Job) -> Result<T, client::Error> {
        let (reply, heard) = sync_channel(1);
        self.jobs
            .send(job(reply))
            .map_err(|_| unreachable_error("the client thread ended"))?;
        heard
            .recv_timeout(CALL)
            .map_err(|_| unreachable_error("no answer of the planning service"))?
    }
}

impl Records for PlanningRecords {
    fn columns(&self) -> Vec<Column> {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        if let Ok(cols) = self.ask(Job::Columns) {
            *last = cols.iter().map(column_of).collect();
        }
        last.clone()
    }
    fn side(&self) -> Vec<SideItem> {
        Vec::new()
    }
    fn can(&self, op: Operation) -> bool {
        self.can.contains(&op)
    }
    fn add_card(&self, column: &str, title: &str) -> Answer {
        answer(self.ask(|r| Job::AddCard(column.to_owned(), title.to_owned(), r)))
    }
    fn move_card(&self, key: &str, _from: &str, to: &str) -> Answer {
        answer(self.ask(|r| Job::Move(key.to_owned(), to.to_owned(), r)))
    }
    fn add_column(&self, title: &str) -> Answer {
        answer(self.ask(|r| Job::AddColumn(title.to_owned(), r)))
    }
    fn rename_column(&self, column: &str, name: &str) -> Answer {
        answer(self.ask(|r| Job::Rename(column.to_owned(), name.to_owned(), r)))
    }
    fn set_limit(&self, column: &str, limit: Option<u32>) -> Answer {
        answer(self.ask(|r| Job::Limit(column.to_owned(), limit, r)))
    }
}
