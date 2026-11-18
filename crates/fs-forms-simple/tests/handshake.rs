//! The handshake with a wrong contract version: exit code 3 and a message, never silent; a right one passes (ADR 0094 draft, item 8).
//!
//! Tests: example-based, a real server in a thread, the real binary as a process.

use std::path::Path;
use std::process::Command;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

mod support;
use support::wait_for;

// `Control.stop` ends every server of this process (fs-rpc keeps one stop flag), so tests that start a server take this lock.
static ONE_SERVER_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn start(socket: &Path, contract: &'static str) -> JoinHandle<()> {
    let socket = socket.to_owned();
    std::thread::spawn(move || fs_forms_server::run_on(&socket, contract).unwrap())
}

fn check(socket: &Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_fs-forms-simple"))
        .arg("--check")
        .arg("--socket")
        .arg(socket)
        .output()
        .unwrap()
}

fn with_server(contract: &'static str, then: impl FnOnce(&Path)) {
    let _one = ONE_SERVER_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("forms.sock");
    let server = start(&socket, contract);
    wait_for(&socket);
    then(&socket);
    fs_forms_client::stop(&socket).unwrap();
    server.join().unwrap();
}

#[test]
fn a_server_with_a_wrong_contract_ends_the_check_with_3_and_a_message() {
    with_server("9.0", |socket| {
        let out = check(socket);
        assert_eq!(out.status.code(), Some(3));
        let said = String::from_utf8_lossy(&out.stderr);
        assert!(
            said.contains("9.0") && said.contains("this frontend needs"),
            "{said}"
        );
    });
}

#[test]
fn a_server_with_the_right_contract_passes_the_check() {
    with_server("0.1", |socket| {
        let out = check(socket);
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    });
}

#[test]
fn no_server_ends_the_check_with_1_and_a_message() {
    let dir = tempfile::tempdir().unwrap();
    let out = check(&dir.path().join("nobody.sock"));
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("no fitting server"));
}

/// The message of the panic the wait ends in. The wait runs in a thread of its own and is given 10 s more than its limit,
/// so a wait that never ends makes THIS test red instead of hanging (the thread is left behind, the process ends).
fn failure_of_the_wait(socket: &Path, limit: Duration) -> (String, Duration) {
    let socket = socket.to_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let ended = std::panic::catch_unwind(|| support::wait_for_within(&socket, limit));
        let _ = tx.send(ended);
    });
    let ended = rx
        .recv_timeout(limit + Duration::from_secs(10))
        .expect("the wait did not end within its limit plus 10 s");
    let took = started.elapsed();
    let payload = ended.expect_err("the wait must end in a panic, there is no server");
    let text = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
        .unwrap_or_default();
    (text, took)
}

#[test]
fn a_socket_nobody_listens_on_ends_the_wait_after_its_limit_with_a_clear_message() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("nobody.sock");
    let (text, took) = failure_of_the_wait(&socket, Duration::from_millis(400));
    assert!(text.contains("accepted no connection within"), "{text}");
    assert!(text.contains("nobody.sock"), "{text}");
    assert!(
        took >= Duration::from_millis(400),
        "ended too early: {took:?}"
    );
}

#[test]
fn a_file_in_the_place_of_the_socket_is_not_a_server() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("forms.sock");
    std::fs::write(&socket, b"not a server").unwrap();
    let (text, _) = failure_of_the_wait(&socket, Duration::from_millis(300));
    assert!(text.contains("accepted no connection within"), "{text}");
}

#[test]
fn the_wait_lasts_until_a_late_server_listens() {
    let _one = ONE_SERVER_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("forms.sock");
    let late = socket.clone();
    let server = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(600));
        fs_forms_server::run_on(&late, fs_forms_server::CONTRACT_VERSION).unwrap();
    });
    wait_for(&socket);
    // The wait is over, so the server accepts at once: nothing may start or replace it now.
    fs_forms_client::Hello::fetch(
        &socket,
        &|| Err(std::io::Error::other("no starter wanted")),
        fs_forms_client::EXPECTED_CONTRACT,
    )
    .unwrap();
    fs_forms_client::stop(&socket).unwrap();
    server.join().unwrap();
}
