//! The client against the real server, in this process: the right contract is accepted, another one is refused.
//!
//! Tests: example-based, on a real socket in a temporary folder of the test target.

use std::path::Path;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use fs_forms_client::{EXPECTED_CONTRACT, Error, Hello};

mod support;
use support::wait_for;

// `Control.stop` ends every server of this process (fs-rpc keeps one stop flag), so tests that start a server take this lock.
static ONE_SERVER_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn nobody_starts_a_server() -> std::io::Result<()> {
    Err(std::io::Error::other("this test starts its server itself"))
}

fn start(socket: &Path, contract: &'static str) -> JoinHandle<()> {
    let socket = socket.to_owned();
    std::thread::spawn(move || {
        fs_forms_server::run_on(&socket, contract).unwrap();
    })
}

fn stop(socket: &Path, server: JoinHandle<()>) {
    fs_forms_client::stop(socket).unwrap();
    server.join().unwrap();
}

#[test]
fn a_server_with_the_expected_contract_is_accepted_and_says_it_offers_nothing_yet() {
    let _one = ONE_SERVER_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("forms.sock");
    let server = start(&socket, fs_forms_server::CONTRACT_VERSION);
    wait_for(&socket);
    let hello = Hello::fetch(&socket, &nobody_starts_a_server, EXPECTED_CONTRACT).unwrap();
    assert_eq!(hello.contract, "0.1");
    assert_eq!(hello.ids, Vec::<String>::new());
    stop(&socket, server);
}

#[test]
fn a_server_with_another_main_level_is_refused_with_both_versions_named() {
    let _one = ONE_SERVER_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("forms.sock");
    let server = start(&socket, "9.0");
    wait_for(&socket);
    let err = Hello::fetch(&socket, &nobody_starts_a_server, EXPECTED_CONTRACT).unwrap_err();
    match &err {
        Error::Contract { found, expected } => {
            assert_eq!(found, "9.0");
            assert_eq!(expected, EXPECTED_CONTRACT);
        }
        other => panic!("expected a contract error, got {other}"),
    }
    assert!(err.to_string().contains("\"9.0\""));
    stop(&socket, server);
}

#[test]
fn the_server_and_the_client_name_the_same_contract() {
    assert_eq!(fs_forms_server::CONTRACT_VERSION, EXPECTED_CONTRACT);
}

#[test]
fn no_server_and_a_starter_that_cannot_start_one_is_a_visible_error() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("none.sock");
    let err = Hello::fetch(&socket, &nobody_starts_a_server, EXPECTED_CONTRACT).unwrap_err();
    assert!(matches!(err, Error::Unreachable(_)), "{err}");
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
