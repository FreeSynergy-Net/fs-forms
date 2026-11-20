//! The real planning service for a test: the program `fs-modules-planning` on a socket in a temporary folder (the XDG variables
//! point there too, so no folder of the user is touched; the child is ended when the value is dropped, also when the test fails).
//!
//! Where the program file is: `FS_MODULES_PLANNING_BIN`, else next to the test target (`target/debug/fs-modules-planning`: build
//! it with `cargo build --manifest-path <fs-modules-planning>/Cargo.toml -p fs-modules-planning` and the same target dir).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use fs_forms_run::PlanningRecords;

/// The folder the pictures for the reader of the report go to, when it exists.
const SHOWN: &str = "/srv/FreeSynergy/.scratch/bilder/forms-tasks";

pub fn service_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("FS_MODULES_PLANNING_BIN") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    // target/debug/deps/<test> -> target/debug/fs-modules-planning
    exe.parent()
        .and_then(Path::parent)
        .unwrap()
        .join("fs-modules-planning")
}

/// A running service on a socket in a temporary folder.
pub struct Service {
    child: Child,
    pub home: tempfile::TempDir,
    pub socket: PathBuf,
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn private(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).unwrap();
}

pub fn start() -> Service {
    let bin = service_bin();
    assert!(
        bin.is_file(),
        "the service program is not at {}",
        bin.display()
    );
    let home = tempfile::Builder::new().prefix("b4").tempdir().unwrap();
    private(&home.path().join("data"));
    private(&home.path().join("run"));
    let socket = home.path().join("run").join("p.sock");
    let child = Command::new(&bin)
        .args(["serve", "--socket"])
        .arg(&socket)
        .env_clear()
        .env("HOME", home.path())
        .env("XDG_DATA_HOME", home.path().join("data"))
        .env("XDG_RUNTIME_DIR", home.path().join("run"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let service = Service {
        child,
        home,
        socket,
    };
    let deadline = Instant::now() + Duration::from_secs(20);
    while !service.socket.exists() {
        assert!(Instant::now() < deadline, "the service did not come up");
        std::thread::sleep(Duration::from_millis(50));
    }
    service
}

pub fn connect(service: &Service) -> PlanningRecords {
    PlanningRecords::connect(&service.socket, &service_bin()).expect("the service answers")
}

/// Where the pictures go: `FS_PICTURE_OUT`, else the folder of the report when it exists, else the target of the test.
pub fn pictures_dir() -> PathBuf {
    let dir = if let Some(d) = std::env::var_os("FS_PICTURE_OUT") {
        PathBuf::from(d)
    } else if Path::new(SHOWN).is_dir() {
        PathBuf::from(SHOWN)
    } else {
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("b4-pictures")
    };
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
