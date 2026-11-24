//! SQ-3: the blocks `tree` and `table` draw the forms `tree@1.0` and `rows@1.0` BY CHANNEL, from the port `Sources` (no module
//! client in the host). Tests: (1) `--sample-data` with the groups description draws a window that differs from the empty one and
//! says "Sample data"; (2) a module that leaves a required channel out is locked with `form-channel-unmapped` and a picture run
//! ends with 3 (the PNG is written); (3) a contract the host cannot connect is said, not ignored.
//! Mutants: the match is off (no lock, rc 0); the sample data is not drawn (pictures equal).
//! Set `FS_PICTURE_OUT=<dir>` to keep the pictures.

mod support;
use std::path::Path;
use std::sync::Arc;

use fs_forms_run::{Coded, FormRecords, Records, Sources, run_code_with, sample_groups};
use support::{draw, out_dir};

fn groups() -> String {
    format!("{}/tests/fixtures/groups.toml", env!("CARGO_MANIFEST_DIR"))
}

/// A host that connects the groups module: here a service with fixed data, minus the channels in `without`.
struct Groups {
    without: &'static [&'static str],
}

impl Sources for Groups {
    fn connect(&self, _: &str, _: Option<&Path>, _: &Path) -> Result<Arc<dyn Records>, Coded> {
        let mut forms = sample_groups();
        for f in &mut forms {
            f.declared
                .map
                .retain(|(c, _)| !self.without.contains(&c.as_str()));
        }
        Ok(Arc::new(FormRecords {
            provider: "fake".into(),
            forms,
            ..FormRecords::default()
        }))
    }
}

fn run(extra: &[&str], sources: &dyn Sources, png: &Path) -> i32 {
    let mut args: Vec<String> = ["--description", &groups(), "--picture"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    args.push(png.to_string_lossy().into_owned());
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    run_code_with(&args, sources)
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn the_sample_groups_draw_a_tree_and_a_member_list_that_the_empty_window_does_not_have() {
    let (_keep, dir) = out_dir();
    let g = groups();
    let Some(sample) = draw(
        &["--description", &g, "--sample-data"],
        "light",
        &dir.join("groups-sample-light.png"),
    ) else {
        return;
    };
    let Some(empty) = draw(
        &["--description", &g],
        "light",
        &dir.join("groups-empty.png"),
    ) else {
        return;
    };
    assert_eq!((sample.w, sample.h), (1000, 760));
    assert_ne!(sample.rgb, empty.rgb, "the sample data is drawn");
    draw(
        &["--description", &g, "--sample-data"],
        "dark",
        &dir.join("groups-sample-dark.png"),
    );
}

#[test]
#[cfg_attr(
    windows,
    ignore = "wgpu Validation Error on windows-2022 without a GPU adapter"
)]
fn a_connected_module_fits_and_a_required_channel_left_out_locks_the_block_and_ends_with_3() {
    let (_keep, dir) = out_dir();
    let ok = run(
        &[],
        &Groups { without: &[] },
        &dir.join("groups-service.png"),
    );
    if ok == 1 {
        eprintln!("skipped: no graphics adapter");
        return;
    }
    assert_eq!(ok, 0, "a module that fits");
    let bad = run(
        &[],
        &Groups {
            without: &["label"],
        },
        &dir.join("groups-unmapped.png"),
    );
    assert_eq!(bad, 3, "form-channel-unmapped ends the picture run with 3");
    assert!(
        dir.join("groups-unmapped.png").exists(),
        "the picture is written all the same"
    );
    let bad = run(
        &[],
        &Groups {
            without: &["items"],
        },
        &dir.join("groups-unmapped2.png"),
    );
    assert_eq!(bad, 3, "the channel `items` is only in the form `rows`");
}

#[test]
fn the_lock_names_the_code_of_the_rule() {
    let r = Groups {
        without: &["label"],
    }
    .connect("groups@>=0.1, <0.2", None, Path::new("x"))
    .unwrap();
    let codes = fs_forms_run::form_problems("tree", r.as_ref());
    assert!(
        codes.iter().any(|c| c.code == "form-channel-unmapped"
            && c.params.iter().any(|(k, v)| k == "channel" && v == "label")),
        "{codes:?}"
    );
    // `rows` names its rows by `label` too, so the same channel left out locks the table as well
    let table = fs_forms_run::form_problems("table", r.as_ref());
    assert!(
        table.iter().any(|c| c.code == "form-channel-unmapped"),
        "{table:?}"
    );
}
