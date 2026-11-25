//! The writes of `SampleModule` stay in the process (condition of C1, 25.11.2026). One test in its own file: it counts the open
//! descriptors of the process, so no other test may run beside it.

mod module_support;

use std::cell::RefCell;
use std::rc::Rc;

use fs_interfaces::module::{
    ChangeKind, ChangesQuery, Invocation, ModuleCore, PageQuery, Refusal, StepRef, Value,
};
use module_support::{BARS, bars_with_writes, calendar, cx, module, run};

fn call(command: &str, record: Option<&str>, args: &[(&str, &str)]) -> Invocation {
    let mut i = Invocation::default();
    command.clone_into(&mut i.command);
    i.record = record.map(str::to_owned);
    i.args = args.iter().map(|(p, t)| Value::new(p, t)).collect();
    i
}

fn refused<T>(r: Result<T, Refusal>) -> Refusal {
    match r {
        Err(e) => e,
        Ok(_) => panic!("expected a refusal"),
    }
}

const MOVE: [(&str, &str); 3] = [
    ("start", "2026-05-12T10:00:00+02:00"),
    ("end", "2026-05-12T11:00:00+02:00"),
    ("lane", "moved"),
];

#[test]
fn nothing_goes_out_but_what_the_feed_tells_in_process() {
    let m = module(&calendar(true, 1, Vec::new())).declare(&bars_with_writes());
    let heard = Rc::new(RefCell::new(Vec::new()));
    let spy = Rc::clone(&heard);
    m.feed(Box::new(move |c| {
        spy.borrow_mut().push((c.kind, c.object.clone()));
    }));
    let mut q = PageQuery::default();
    q.form = BARS.to_owned();
    let key = run(m.page(&cx("p"), q)).unwrap().records[0].id.clone();
    // One runtime for the whole block: the runtime itself opens descriptors, the module must open none.
    let fds = run(async {
        let fds = open_files();
        let done = m
            .apply(&cx("p"), call("move", Some(&key), &MOVE))
            .await
            .unwrap();
        let _ = refused(
            m.apply(&cx("p"), call("move", Some(&key), &[("zz", "x")]))
                .await,
        );
        m.revert(&cx("p"), &done.change.unwrap().activity, StepRef::default())
            .await
            .unwrap();
        (fds, open_files())
    });
    assert_eq!(
        *heard.borrow(),
        vec![(ChangeKind::Update, key.clone()), (ChangeKind::Undo, key)],
        "each change once, a refusal never"
    );
    assert_eq!(
        fds.0, fds.1,
        "no file and no socket was opened by the writes"
    );
    let all = run(m.changes(&cx("p"), ChangesQuery::default())).unwrap();
    assert_eq!(
        all.changes.len(),
        2,
        "the log of the asking program: the move and its undo"
    );
    assert_eq!(all.changes[0].kind, ChangeKind::Undo, "newest first");
}

/// The number of open file descriptors of this process (files and sockets); `None` where /proc does not exist.
fn open_files() -> Option<usize> {
    std::fs::read_dir("/proc/self/fd").ok().map(Iterator::count)
}
