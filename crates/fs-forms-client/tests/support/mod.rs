//! Test helper: wait until a server really accepts connections.
//!
//! A socket file that exists (or a bound name) is not a listening server: under load the server thread binds first and
//! listens a moment later, and on Windows there is no file at all (a named pipe). So the wait makes the attempt a client
//! makes: it connects through the transport of `fs-rpc` (the same on every platform), retries with short pauses and
//! fails with a plain message when the time is up (red, never hanging).

use std::path::Path;
use std::time::{Duration, Instant};

/// Longest wait for a server to accept a connection: only a guard against a server that never comes, never a measure.
pub const WAIT: Duration = Duration::from_secs(30);

/// Pause between two attempts.
const PAUSE: Duration = Duration::from_millis(20);

/// Longest time one single attempt may take (a connect that hangs must not eat the whole wait).
const ONE_ATTEMPT: Duration = Duration::from_secs(2);

/// Wait up to [`WAIT`] until a connection to `socket` is accepted.
///
/// # Panics
/// When no connection was accepted in time, with the last error in the message.
pub fn wait_for(socket: &Path) {
    wait_for_within(socket, WAIT);
}

/// [`wait_for`] with another time limit (for the test of the failure itself).
///
/// # Panics
/// As for [`wait_for`].
pub fn wait_for_within(socket: &Path, limit: Duration) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime for the connection attempts");
    let local = tokio::task::LocalSet::new();
    let started = Instant::now();
    let mut attempts = 0_u32;
    let last = loop {
        attempts += 1;
        let error = match local.block_on(&runtime, try_once(socket)) {
            Ok(()) => return,
            Err(e) => e,
        };
        if started.elapsed() >= limit {
            break error;
        }
        std::thread::sleep(PAUSE);
    };
    panic!(
        "the server on {} accepted no connection within {:?} ({attempts} attempts); last error: {last}",
        socket.display(),
        limit
    );
}

/// One connection attempt; the connection is closed again at once.
async fn try_once(socket: &Path) -> Result<(), String> {
    match tokio::time::timeout(
        ONE_ATTEMPT,
        fs_rpc::connect::<capnp::capability::Client>(socket),
    )
    .await
    {
        Ok(Ok(_connection)) => Ok(()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(_) => Err(format!("one attempt took longer than {ONE_ATTEMPT:?}")),
    }
}
