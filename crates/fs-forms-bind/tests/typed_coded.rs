//! The decisions on a `Coded` run over typed fields set by the producer; the code text is only what goes out.
use fs_forms_bind::{BindCode, Cause, Coded, Family, Reason, SlotRole};

#[test]
fn family_is_what_the_producer_set_not_what_the_text_starts_with() {
    let said = Coded::new("accepts-error-contract-missing").of(Family::Accepts);
    assert_eq!(said.family(), Family::Accepts);
    assert!(matches!(Cause::from_coded(said), Cause::Accepts(_)));
    // Gegenprobe: the same text without a family is NOT guessed.
    let plain = Coded::new("accepts-error-contract-missing");
    assert_eq!(plain.family(), Family::Unknown);
    assert!(matches!(Cause::from_coded(plain), Cause::Unknown(_)));
    // A text that starts like another family, set by its producer, keeps the producer's family.
    let odd = Coded::new("bind-error-looks-like-bind").of(Family::Layout);
    assert!(matches!(Cause::from_coded(odd), Cause::Layout(_)));
}

#[test]
fn bind_codes_come_with_the_bind_family() {
    assert_eq!(BindCode::Locked.coded().family(), Family::Bind);
}

#[test]
fn reason_and_slot_role_are_set_by_the_producer_only() {
    let plain = Coded::new("layout-error-schemes-missing");
    assert_eq!(plain.reason(), None);
    assert_eq!(plain.slot_role(), None);
    let typed = plain
        .because(Reason::SchemesMissing)
        .for_slot(SlotRole::Heads);
    assert_eq!(typed.reason(), Some(Reason::SchemesMissing));
    assert_eq!(typed.slot_role(), Some(SlotRole::Heads));
}
