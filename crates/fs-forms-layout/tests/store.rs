//! K53, K73, K74, K76, K77, K93, K94 and atomic writes on the memory store.
#![allow(clippy::assert_is_empty, clippy::many_single_char_names)] // tests: short names and plain asserts read best
mod support;

use fs_forms_layout::*;
use support::*;

fn err<T>(r: Result<T, Coded<LayoutCode>>) -> Coded<LayoutCode> {
    match r {
        Ok(_) => panic!("expected an error"),
        Err(e) => e,
    }
}

#[test]
fn bad_doc_is_a_report_and_changes_nothing() {
    let s = store();
    let mut d = doc("board");
    d.messages[0].v.key = "A b".into();
    let r = block_on(s.create(d, &who())).unwrap();
    assert_eq!(r.problems[0].code, LayoutCode::KeyInvalid);
    assert_eq!(block_on(s.stamp()).unwrap(), Stamp(0));
    assert!(
        block_on(s.list(&LayoutFilter::default(), None, 10))
            .unwrap()
            .items
            .is_empty()
    );
}

#[test]
fn dry_run_writes_nothing() {
    let s = store();
    let r = block_on(s.import(doc("board"), Mode::DryRun, &who())).unwrap();
    assert!(r.problems.is_empty() && r.layout.is_some());
    assert_eq!(block_on(s.stamp()).unwrap(), Stamp(0));
}

#[test]
fn k73_rev_is_compared() {
    let s = store();
    let id = block_on(s.create(doc("board"), &who()))
        .unwrap()
        .layout
        .unwrap();
    let e = err(block_on(s.save(&id, Rev(0), doc("board"), &who())));
    assert_eq!(
        (e.code, e.param("expected"), e.param("found")),
        (LayoutCode::RevChanged, Some("0"), Some("1"))
    );
    let stamp = block_on(s.stamp()).unwrap();
    assert_eq!(stamp, Stamp(1));
    block_on(s.save(&id, Rev(1), doc("board"), &who())).unwrap();
    assert_eq!(block_on(s.stamp()).unwrap(), Stamp(2));
}

#[test]
fn k53_package_originals_are_read_only() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    assert_eq!(
        err(block_on(s.save(&id, Rev(1), doc("board"), &who()))).code,
        LayoutCode::PackageOwned
    );
    assert_eq!(
        err(block_on(s.delete(&id, Rev(1), &who()))).code,
        LayoutCode::PackageOwned
    );
}

#[test]
fn k74_one_live_key_and_tombstone_frees_it() {
    let s = store();
    install(&s, doc("board"), "1.0");
    assert_eq!(
        err(block_on(s.create(doc("board"), &who()))).code,
        LayoutCode::Exists
    );
    block_on(s.uninstall("net.example.pkg", &who())).unwrap();
    let mine = block_on(s.create(doc("board"), &who()))
        .unwrap()
        .layout
        .unwrap();
    let e = err(block_on(s.install(
        doc("board"),
        &pkg("1.0"),
        Mode::Apply,
        &who(),
    )));
    assert_eq!(e.code, LayoutCode::Exists);
    assert!(block_on(s.original(&mine)).unwrap().is_some());
}

#[test]
fn k76_delete_is_a_tombstone() {
    let s = store();
    let id = block_on(s.create(doc("board"), &who()))
        .unwrap()
        .layout
        .unwrap();
    block_on(s.delete(&id, Rev(1), &who())).unwrap();
    let all = LayoutFilter {
        with_deleted: true,
        ..LayoutFilter::default()
    };
    assert!(
        block_on(s.list(&LayoutFilter::default(), None, 10))
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        block_on(s.list(&all, None, 10)).unwrap().items[0]
            .deleted
            .is_some()
    );
    assert!(block_on(s.original(&id)).unwrap().is_none());
    let new = block_on(s.create(doc("board"), &who()))
        .unwrap()
        .layout
        .unwrap();
    assert_ne!(new, id);
}

#[test]
fn list_pages_and_page_size() {
    let s = store();
    for f in ["a", "b", "c"] {
        block_on(s.create(doc(f), &who())).unwrap();
    }
    let p1 = block_on(s.list(&LayoutFilter::default(), None, 2)).unwrap();
    assert_eq!(p1.items.len(), 2);
    let p2 = block_on(s.list(&LayoutFilter::default(), p1.next.as_ref(), 2)).unwrap();
    assert_eq!((p2.items.len(), p2.next), (1, None));
    assert_eq!(
        err(block_on(s.list(&LayoutFilter::default(), None, 0))).code,
        LayoutCode::PageSize
    );
}

#[test]
fn k77_every_write_is_logged_with_service_time() {
    let s = store();
    let id = block_on(s.create(doc("board"), &who()))
        .unwrap()
        .layout
        .unwrap();
    block_on(s.save(&id, Rev(1), doc("board"), &who())).unwrap();
    block_on(s.delete(&id, Rev(2), &who())).unwrap();
    let log = block_on(s.log(None, 10)).unwrap();
    let actions: Vec<LogAction> = log.iter().map(|l| l.action).collect();
    assert_eq!(
        actions,
        vec![LogAction::Create, LogAction::Save, LogAction::Delete]
    );
    assert!(log.windows(2).all(|w| w[0].at < w[1].at)); // times come from the service clock
}

#[test]
fn k94_uninstall_orphans_and_reinstall_revives_the_same_id() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let o = block_on(s.create_overlay(&id, None, None, &who())).unwrap();
    block_on(s.replace_ops(&o, Rev(1), vec![set_text(Field::TextKey, "x")], &who())).unwrap();
    block_on(s.uninstall("net.example.pkg", &who())).unwrap();
    assert_eq!(
        block_on(s.overlay(&o)).unwrap().unwrap().state,
        OverlayState::Orphaned
    );
    assert_eq!(
        err(block_on(s.replace_ops(&o, Rev(2), vec![], &who()))).code,
        LayoutCode::Orphaned
    );
    let again = install(&s, doc("board"), "1.0");
    assert_eq!(again, id);
    assert_eq!(
        block_on(s.overlay(&o)).unwrap().unwrap().state,
        OverlayState::Applied
    );
}

#[test]
fn k93_failed_update_changes_nothing() {
    let s = store();
    let id = install(&s, doc("board"), "1.0");
    let before = block_on(s.original(&id)).unwrap().unwrap().0.digest();
    let mut bad = doc("board");
    bad.messages[0].v.lang = "de_DE".into();
    let r = block_on(s.install(bad, &pkg("2.0"), Mode::Apply, &who())).unwrap();
    assert!(!r.problems.is_empty());
    let (l, sum) = block_on(s.original(&id)).unwrap().unwrap();
    assert_eq!(
        (l.digest(), sum.rev, block_on(s.stamp()).unwrap()),
        (before, Rev(1), Stamp(1))
    );
}
