//! Test helper: wait until a server really accepts connections.
//!
//! A socket file that exists (or a bound name) is not a listening server: under load the server thread binds first and
//! listens a moment later, and on Windows there is no file at all (a named pipe). So the wait makes the attempt the
//! frontend makes: a connection through the client crate (the transport of `fs-rpc`, the same on every platform) with a
//! starter that starts nothing. It retries with short pauses and fails with a plain message when the time is up (red,
//! never hanging). Any answer except "unreachable" (also a refused contract) means: the server accepted the connection.

use std::path::Path;
use std::time::{Duration, Instant};

use fs_forms_client::{EXPECTED_CONTRACT, Error, Hello};

/// Longest wait for a server to accept a connection: only a guard against a server that never comes, never a measure.
pub const WAIT: Duration = Duration::from_secs(30);

/// Pause between two attempts.
const PAUSE: Duration = Duration::from_millis(20);

fn starts_nothing() -> std::io::Result<()> {
    Err(std::io::Error::other(
        "the wait only connects, it starts no server",
    ))
}

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
    let started = Instant::now();
    let mut attempts = 0_u32;
    let last = loop {
        attempts += 1;
        match Hello::fetch(socket, &starts_nothing, EXPECTED_CONTRACT) {
            Err(Error::Unreachable(why)) => {
                if started.elapsed() >= limit {
                    break why;
                }
                std::thread::sleep(PAUSE);
            }
            Ok(_) | Err(_) => return,
        }
    };
    panic!(
        "the server on {} accepted no connection within {:?} ({attempts} attempts); last error: {last}",
        socket.display(),
        limit
    );
}
