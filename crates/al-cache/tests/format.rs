//! The entry file: round trip, determinism, and what a damaged file does.

use al_cache::{AppKey, CacheError, Entry, FORMAT, Key, build, build_layer};
use al_sema::provided::{DepLayer, DepSet, DepSource, LayerSource, Stack};
use al_sema::workspace::AppSource;

const SRC: &str = r#"
table 50000 T { fields { field(1; A; Integer) { } field(2; B; Code[20]) { } } keys { key(PK; A) { } } }
page 50002 P { PageType = Card; SourceTable = T; layout { area(Content) { field(AC; Rec.A) { } } } }
interface IFoo { procedure Bump(): Integer; }
codeunit 50004 DImpl implements IFoo { procedure Bump(): Integer begin exit(1); end; }
codeunit 50005 DUse { procedure CallIt(F: Interface IFoo): Integer begin exit(F.Bump()); end; }
codeunit 50008 DPlain { procedure Plain(): Integer begin exit(7); end; procedure Twice(): Integer begin exit(Plain() * 2); end; }
"#;

fn apps() -> Vec<AppSource> {
    vec![AppSource {
        id: "d".into(),
        name: "d".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), SRC.into())],
    }]
}

fn key() -> Key {
    Key::new(
        [9; 32],
        "BC 27.5".into(),
        vec![AppKey {
            id: "d".into(),
            name: "d".into(),
            dependencies: vec![],
            publisher: "p".into(),
            version: "1.0.0.0".into(),
            symbols: vec![],
            source: [3; 32],
        }],
    )
}

fn set() -> DepSet {
    al_sema::lower::lower_for_cache(apps(), &Default::default(), None)
}

fn blocks(e: &Entry) -> Vec<al_sema::provided::DepBlock> {
    (0..e.len()).map(|i| e.block(i).expect("block")).collect()
}

fn heads(e: &Entry) -> Vec<al_ir::ObjectHead> {
    (0..e.len()).map(|i| e.head(i).clone()).collect()
}

#[test]
fn a_set_survives_the_round_trip() {
    let set = set();
    assert!(set.facts.len() >= 5);
    let bytes = build(&set, &key());
    let e = Entry::open(bytes, &key()).expect("opens");
    e.verify().expect("verifies");
    assert_eq!(e.len(), set.facts.len());
    for i in 0..e.len() {
        assert_eq!(e.facts(i), &set.facts[i], "facts of object {i}");
        assert_eq!(e.head(i), &set.heads[i], "head of object {i}");
    }
    assert!(blocks(&e) == set.blocks, "blocks differ");
    // ADR-054: the heads are stored, and a table's head carries its linked table.
    assert!(set.heads.iter().any(|h| h.table.is_some()));
}

#[test]
fn two_builds_give_the_same_bytes() {
    assert!(build(&set(), &key()) == build(&set(), &key()));
}

#[test]
fn another_key_or_format_is_refused_at_open() {
    let bytes = build(&set(), &key());
    let mut other = key();
    other.apps[0].source[0] ^= 1;
    assert_eq!(Entry::open(bytes.clone(), &other).err(), Some(CacheError::KeyMismatch));
    let mut newer = key();
    newer.format = FORMAT + 1;
    assert!(matches!(
        Entry::open(bytes, &newer).err(),
        Some(CacheError::Format { .. })
    ));
}

#[test]
fn a_truncated_file_is_refused_whatever_its_length() {
    let bytes = build(&set(), &key());
    for len in (0..bytes.len()).step_by(7).chain([bytes.len() - 1]) {
        assert!(
            Entry::open(bytes[..len].to_vec(), &key()).is_err(),
            "a file cut to {len} of {} bytes was opened",
            bytes.len()
        );
    }
}

/// Flip every byte of the file in turn. Either the damage is found (at open or when the blocks
/// and heads are read), or what is read is exactly what was written. It is never something else.
#[test]
fn a_damaged_file_is_refused_or_reads_the_same() {
    let good = build(&set(), &key());
    let want = {
        let e = Entry::open(good.clone(), &key()).unwrap();
        (blocks(&e), heads(&e))
    };
    let (mut refused, mut same) = (0, 0);
    for at in 0..good.len() {
        let mut bad = good.clone();
        bad[at] ^= 0x41;
        let Ok(e) = Entry::open(bad, &key()) else {
            refused += 1;
            continue;
        };
        match e.verify() {
            Err(_) => refused += 1,
            Ok(()) => {
                let got = (blocks(&e), heads(&e));
                assert!(got == want, "a flipped byte at {at} changed what is read");
                same += 1;
            }
        }
    }
    assert!(
        refused > good.len() * 9 / 10,
        "{refused} of {} flips refused ({same} read the same)",
        good.len()
    );
}

#[test]
fn garbage_is_not_a_cache_file() {
    assert!(Entry::open(vec![], &key()).is_err());
    assert!(Entry::open(vec![0; 4096], &key()).is_err());
    assert!(Entry::open(b"ALRCDEP\0 and then nothing useful".to_vec(), &key()).is_err());
}

/// Offsets and lengths are variable-width and the index is compressed: a set of a few thousand
/// objects exercises every width, and the layout must not depend on the size of the index.
#[test]
fn a_set_of_thousands_of_objects_round_trips() {
    let mut src = String::new();
    for i in 0..3000 {
        src.push_str(&format!(
            "codeunit {} \"Gen {i}\" {{ procedure P{i}(): Integer begin exit({i}); end; }}\n",
            70000 + i
        ));
    }
    let set = al_sema::lower::lower_for_cache(
        vec![AppSource {
            id: "g".into(),
            name: "g".into(),
            dependencies: vec![],
            files: vec![("g.al".into(), src)],
        }],
        &Default::default(),
        None,
    );
    assert_eq!(set.facts.len(), 3000);
    let bytes = build(&set, &key());
    let e = Entry::open(bytes, &key()).expect("opens");
    e.verify().expect("verifies");
    assert!(blocks(&e) == set.blocks);
}

/// A dependency file that does not parse is reported by the run that builds the set. A run that
/// takes the set from the file parses none of it, so the file has to carry the report.
#[test]
fn the_files_that_did_not_parse_are_kept_in_the_entry() {
    let set = al_sema::lower::lower_for_cache(
        vec![AppSource {
            id: "d".into(),
            name: "d".into(),
            dependencies: vec![],
            files: vec![
                ("good.al".into(), SRC.into()),
                (
                    "bad.al".into(),
                    "codeunit 59999 Broken { procedure P( begin end; }".into(),
                ),
            ],
        }],
        &Default::default(),
        None,
    );
    assert_eq!(set.parse_problems.len(), 1, "{:?}", set.parse_problems);
    assert_eq!(set.parse_problems[0].0, "bad.al");
    let e = Entry::open(build(&set, &key()), &key()).unwrap();
    assert_eq!(e.parse_problems(), &set.parse_problems[..]);
}

const REPORTS: &str = r#"
table 50000 T { fields { field(1; A; Integer) { } } keys { key(PK; A) { } } }
report 50010 R1 { ProcessingOnly = true; dataset { dataitem(D; T) { } } }
report 50011 R2 { ProcessingOnly = true; dataset { dataitem(D; T) { } } }
reportextension 50012 R1E1 extends R1 { trigger OnPreReport() begin end; }
reportextension 50014 R2E2 extends R2 { trigger OnPostReport() begin end; }
reportextension 50013 R2E1 extends R2 { trigger OnPreReport() begin end; }
"#;

/// ADR-025: the reportextensions that change a run are in the facts of the report, in object id
/// order, for one extension and for two; they survive the entry file.
#[test]
fn one_and_two_report_extensions_round_trip_in_object_id_order() {
    let apps = vec![AppSource {
        id: "d".into(),
        name: "d".into(),
        dependencies: vec![],
        files: vec![("f.al".into(), REPORTS.into())],
    }];
    let set = al_sema::lower::lower_for_cache(apps, &Default::default(), None);
    let lens: Vec<usize> = set.facts.iter().map(|f| f.report_extensions.len()).collect();
    assert!(lens.contains(&1) && lens.contains(&2), "{lens:?}");
    let two = set.facts.iter().find(|f| f.report_extensions.len() == 2).unwrap();
    let ids: Vec<Option<u32>> = two
        .report_extensions
        .iter()
        .map(|&i| set.blocks[i as usize].object.as_ref().and_then(|o| o.id))
        .collect();
    assert_eq!(ids, vec![Some(50013), Some(50014)], "declared in the other order");
    let bytes = build(&set, &key());
    let e = Entry::open(bytes, &key()).expect("opens");
    e.verify().expect("verifies");
    for i in 0..e.len() {
        assert_eq!(e.facts(i), &set.facts[i], "facts of object {i}");
    }
}

/// An entry written at format 1 (one `report_extension` per report) is refused at open.
#[test]
fn a_format_1_entry_is_refused() {
    let mut old = key();
    old.format = 1;
    let bytes = build(&set(), &old);
    assert!(matches!(
        Entry::open(bytes, &key()).err(),
        Some(CacheError::Format { .. })
    ));
}

const UPPER: &str = r#"
tableextension 60000 UT extends T { fields { field(20; UX; Integer) { } } }
codeunit 60004 UUse { procedure W() var R: Record T; begin R.UX := 1; R.A := 2; end; }
"#;

fn upper_app() -> AppSource {
    AppSource {
        id: "u".into(),
        name: "u".into(),
        dependencies: vec!["d".into()],
        files: vec![("u.al".into(), UPPER.into())],
    }
}

fn upper_key(below: &Key) -> Key {
    Key::layered(
        [9; 32],
        "BC 27.5".into(),
        vec![AppKey {
            id: "u".into(),
            name: "u".into(),
            dependencies: vec!["d".into()],
            publisher: "p".into(),
            version: "1.0.0.0".into(),
            symbols: vec![],
            source: [4; 32],
        }],
        below.digest(),
    )
}

/// ADR-022, layers (2026-10-15): a layer file holds its own objects and the objects of the layers
/// below that it changes; the files stacked give what one complete compile gives.
#[test]
fn layer_files_stacked_equal_the_complete_compile() {
    let base_set = set();
    let base_layer = DepLayer::whole(base_set.clone());
    let mut all = apps();
    all.push(upper_app());
    let layer = al_sema::lower::lower_layer(
        all.clone(),
        1,
        &Default::default(),
        None,
        &Stack::new(vec![&base_layer as &dyn LayerSource]).unwrap(),
    )
    .unwrap();
    assert!(
        layer.objects.iter().any(|&o| (o as usize) < layer.base_len),
        "the table is overridden"
    );
    let (kb, ku) = (key(), upper_key(&key()));
    let eb = Entry::open(build(&base_set, &kb), &kb).unwrap();
    let eu = Entry::open(al_cache::build_layer_with_dictionary(&layer, &ku, eb.dictionary()), &ku).unwrap();
    eu.verify().unwrap();
    assert_eq!(eu.base_len(), eb.len());
    let stack = Stack::new(vec![&eb as &dyn LayerSource, &eu]).unwrap();
    let complete = al_sema::lower::lower_for_cache(all, &Default::default(), None);
    assert_eq!(stack.len(), complete.facts.len());
    for i in 0..stack.len() {
        assert_eq!(stack.facts(i), &complete.facts[i], "facts of object {i}");
        assert_eq!(stack.head(i), &complete.heads[i], "head of object {i}");
        assert!(stack.block(i).unwrap() == complete.blocks[i], "block {i}");
    }
}

/// The layer's file is for the layer below it as it was built: another key below, another file.
#[test]
fn a_layer_file_is_refused_when_the_layer_below_changed() {
    let base_layer = DepLayer::whole(set());
    let mut all = apps();
    all.push(upper_app());
    let layer = al_sema::lower::lower_layer(
        all,
        1,
        &Default::default(),
        None,
        &Stack::new(vec![&base_layer as &dyn LayerSource]).unwrap(),
    )
    .unwrap();
    let ku = upper_key(&key());
    let bytes = build_layer(&layer, &ku);
    let mut other_below = key();
    other_below.apps[0].source[0] ^= 1;
    assert_eq!(
        Entry::open(bytes, &upper_key(&other_below)).err(),
        Some(CacheError::KeyMismatch)
    );
}
