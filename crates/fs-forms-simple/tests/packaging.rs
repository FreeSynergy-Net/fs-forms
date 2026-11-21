//! A4 (fs-forms v0.0.3; moved here from the excluded `fs-forms-bin`, so it runs again): the package of the frontend `simple` puts its manifest where the starter looks, so that after an
//! installation `fs-forms` does not end with `frontend-missing`: `<XDG_DATA_DIRS>/freesynergy/frontends/forms/simple.metainfo.xml`
//! (`usr/share/...`). The starter reads `<fs:frontend-id>.metainfo.xml` in the folder named by the program (`forms`).
//!
//! Tests: example-based, reads the manifests of the repository. Mutants: the deb asset goes to another folder (`frontend/`,
//! `forms-simple/`); the rpm asset is missing; the file is renamed; the frontend id of the manifest differs from the file name.
//! Not covered (needs the release tools, run by the release workflow): that cargo-deb and cargo-generate-rpm resolve the source paths.

use std::fs;
use std::path::PathBuf;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn simple_manifest() -> String {
    fs::read_to_string(repo().join("crates/fs-forms-simple/Cargo.toml")).unwrap()
}

const DIR: &str = "usr/share/freesynergy/frontends/forms/";

#[test]
fn the_deb_of_the_frontend_installs_its_manifest_in_the_xdg_folder_of_the_program() {
    let manifest = simple_manifest();
    let deb = manifest
        .split("[package.metadata.deb]")
        .nth(1)
        .expect("a deb table")
        .split("\n[")
        .next()
        .unwrap();
    assert!(
        deb.contains(&format!("\"{DIR}\"")),
        "the manifest asset is not in {DIR}: {deb}"
    );
    assert!(deb.contains("simple.metainfo.xml"), "{deb}");
    assert!(
        deb.contains("usr/bin/"),
        "the binary is still an asset: {deb}"
    );
}

#[test]
fn the_rpm_of_the_frontend_installs_its_manifest_under_its_id() {
    let manifest = simple_manifest();
    let rpm = manifest
        .split("[package.metadata.generate-rpm]")
        .nth(1)
        .expect("an rpm table")
        .split("\n[")
        .next()
        .unwrap();
    assert!(
        rpm.contains(&format!("dest = \"/{DIR}simple.metainfo.xml\"")),
        "{rpm}"
    );
    assert!(
        rpm.contains("dest = \"/usr/bin/fs-forms-simple\""),
        "the binary is still an asset: {rpm}"
    );
}

#[test]
fn the_file_exists_and_its_frontend_id_is_its_file_name() {
    let text = fs::read_to_string(repo().join("packaging/frontends/simple.metainfo.xml")).unwrap();
    assert!(
        text.contains("<value key=\"fs:frontend-id\">simple</value>"),
        "the starter wants <id>.metainfo.xml with the same id"
    );
    assert!(text.contains("<extends>net.freesynergy.Forms</extends>"));
    // the source of the deb asset is that very file
    assert!(simple_manifest().contains("../../packaging/frontends/simple.metainfo.xml"));
}
