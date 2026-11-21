//! The real planning service for a test: the program `fs-modules-planning` on a socket in a temporary folder (the XDG variables
//! point there too, so no folder of the user is touched; the child is ended when the value is dropped, also when the test fails).
//!
//! Where the program file is: `FS_MODULES_PLANNING_BIN`, else next to the test target (`target/debug/fs-modules-planning`),
//! else it is built once from the pinned tag of the repository (`cargo install --git`, into the target folder of the test).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use fs_forms_run::PlanningRecords;

/// The repository and tag of the service program: the tag of `fs-modules-planning-client` in `Cargo.toml` (keep the two equal; `tests/service_tag.rs` checks it).
const SERVICE_REPO: &str = "https://github.com/FreeSynergy/fs-modules-planning";
const SERVICE_TAG: &str = "v0.0.3";

/// The service program built from the pinned tag into the target of the test (once per test program; `cargo install --locked`).
fn installed_bin() -> PathBuf {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("service");
        let bin = root.join("bin").join("fs-modules-planning");
        if !bin.is_file() {
            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let status = Command::new(cargo)
                .args([
                    "install",
                    "--locked",
                    "--git",
                    SERVICE_REPO,
                    "--tag",
                    SERVICE_TAG,
                ])
                .arg("--root")
                .arg(&root)
                .arg("fs-modules-planning")
                .env("CARGO_TARGET_DIR", root.join("build"))
                .status()
                .expect("cargo runs");
            assert!(
                status.success(),
                "the service program could not be built from {SERVICE_REPO} {SERVICE_TAG}"
            );
        }
        bin
    })
    .clone()
}

pub fn service_bin() -> PathBuf {
    if let Some(p) = std::env::var_os("FS_MODULES_PLANNING_BIN") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().unwrap();
    // target/debug/deps/<test> -> target/debug/fs-modules-planning
    let beside = exe
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("fs-modules-planning");
    if beside.is_file() {
        beside
    } else {
        installed_bin()
    }
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

/// Where the pictures go: `FS_PICTURE_OUT`, else the target of the test.
pub fn pictures_dir() -> PathBuf {
    let dir = if let Some(d) = std::env::var_os("FS_PICTURE_OUT") {
        PathBuf::from(d)
    } else {
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("b4-pictures")
    };
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
