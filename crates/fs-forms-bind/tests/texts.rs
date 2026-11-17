//! MF2 texts de + en exist for all 18 codes (ADR 0057); H-K17: no toml / JSON / file access in the crate.
use fs_forms_bind::BindCode;

fn keys(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|l| l.strip_prefix("bind-error-"))
        .filter_map(|l| l.split_once(" = "))
        .map(|(k, _)| k)
        .collect()
}

#[test]
fn both_languages_carry_every_code() {
    let de = include_str!("../locales/de.mf2");
    let en = include_str!("../locales/en.mf2");
    for c in BindCode::ALL {
        assert!(keys(de).contains(&c.name()), "de: {}", c.name());
        assert!(keys(en).contains(&c.name()), "en: {}", c.name());
    }
    assert_eq!(keys(de).len(), 18);
    assert_eq!(keys(en).len(), 18);
}

#[test]
fn no_toml_json_or_file_access() {
    let cargo = include_str!("../Cargo.toml");
    let deps = cargo
        .split("[dependencies]")
        .nth(1)
        .unwrap()
        .split("[features]")
        .next()
        .unwrap();
    assert!(
        !deps.contains("toml") && !deps.contains("serde"),
        "no runtime dependency at all"
    );
    for f in [
        "build",
        "build_menu",
        "build_place",
        "catalog",
        "coded",
        "codes",
        "commands",
        "keys",
        "menu",
        "model",
        "plan",
        "providers",
        "runtime",
        "types",
    ] {
        let src =
            std::fs::read_to_string(format!("{}/src/{f}.rs", env!("CARGO_MANIFEST_DIR"))).unwrap();
        for bad in [
            "std::fs",
            "File::open",
            "read_to_string",
            "toml::",
            "serde_json",
        ] {
            assert!(!src.contains(bad), "{f}.rs uses {bad}");
        }
    }
}
