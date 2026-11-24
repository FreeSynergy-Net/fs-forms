//! A5 F6: the catalog of the contract (C2) as the rules of the layout check: places, takes, attribute names.
//! Counter-proofs: a block on a place that takes only frames, too few children, an attribute the block does not set.
use fs_components_catalog::BlockCatalog;
use fs_forms_layout::CatalogRules;
use fs_forms_run::{Blocks, ContractRules};

const CONTRACT: &str = "[contract]\nversion = \"0.4.0\"\n[vocabulary]\nstates = [\"default\"]\nreads = [\"bounds\"]\nsets = [\"axis\", \"min-width\"]\n";
const BLOCKS: &str = "[block.pair]\naccepts = []\nparts = [\"pane\", \"bar\"]\nsets = [\"axis\"]\n\
places = [{ place = \"pane\", takes = [\"form\", \"frame\"], min = 2, max = \"n\" }, { place = \"bar\", takes = [\"command\"], min = 0, max = 1 }]\n\n\
[block.list]\naccepts = [{ slot = \"rows\", kind = \"records\", required = true }]\nsets = [\"min-width\"]\n\n\
[block.press]\naccepts = [{ slot = \"go\", kind = \"command\" }]\n\n\
[block.plain]\naccepts = []\n";

fn catalog() -> BlockCatalog {
    BlockCatalog::read(CONTRACT, &[("t.toml", BLOCKS)]).expect("catalog reads")
}

#[test]
fn places_of_turns_the_classes_into_names_read_from_the_catalog() {
    let c = catalog();
    let r = ContractRules(&c);
    let places = r.places_of("pair").expect("a frame says its places");
    assert_eq!(places.len(), 2);
    assert_eq!(
        (places[0].place.as_str(), places[0].min, places[0].max),
        ("pane", 2, u32::MAX)
    );
    // form = has a records slot; frame = holds places
    assert_eq!(places[0].takes, ["list", "pair"]);
    assert_eq!(places[1].takes, ["press"]);
    assert_eq!((places[1].min, places[1].max), (0, 1));
}

#[test]
fn counter_proof_a_block_without_places_says_nothing_and_an_unknown_one_too() {
    let c = catalog();
    let r = ContractRules(&c);
    assert_eq!(r.places_of("plain"), None);
    assert_eq!(r.places_of("no-such-block"), None);
    assert_eq!(r.sets_of("no-such-block"), None);
}

#[test]
fn sets_of_lists_the_attribute_names_of_the_block() {
    let c = catalog();
    let r = ContractRules(&c);
    assert_eq!(r.sets_of("pair"), Some(vec!["axis".to_owned()]));
    assert_eq!(r.sets_of("plain"), Some(vec![]), "known, sets nothing");
    assert!(r.block_known("list") && !r.block_known("x"));
    assert!(r.slot_known("list", "rows") && !r.slot_known("list", "other"));
}

#[test]
fn the_rules_of_this_build_read_the_real_contract() {
    let r = Blocks::rules();
    assert!(r.block_known("workbench") && r.block_known("board"));
}
