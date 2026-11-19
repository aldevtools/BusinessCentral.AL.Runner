use super::*;

/// The search finds a word that starts, ends or sits alone in the text, and does not run past either end when the
/// letter it scans for is near an end.
#[test]
fn the_word_search_handles_the_ends_of_the_text() {
    for text in [
        "",
        "x",
        "xx",
        "maxlength",
        "MAXLENGTH",
        "axlength",
        "maxlengt",
        "xmaxlength",
        "maxlengthx",
        "xxmaxlengthxx",
    ] {
        let naive = text.as_bytes().windows(9).any(|w| w.eq_ignore_ascii_case(b"maxlength"));
        assert_eq!(names_max_length_in(text.as_bytes()), naive, "{text:?}");
    }
    assert!(names_access_by_permission_in(b"AccessByPermission"));
    assert!(!names_access_by_permission_in(b"ccessByPermission"));
    assert!(!names_access_by_permission_in(b"b"));
}

/// The search for `maxlength` finds it in any case and anywhere, as the plain window search did.
#[test]
fn max_length_is_found_in_any_case_anywhere() {
    let naive = |b: &[u8]| b.windows(9).any(|w| w.eq_ignore_ascii_case(b"maxlength"));
    for text in [
        "",
        "maxlength",
        "MaxLength",
        "MAXLENGTH",
        "xmaxlengthx",
        "field(1; A; Text[30]) { MaxLength = 5; }",
        "maxlengt",
        "maxlengthmaxlength",
        "mmaxlength",
        "MaxLengt",
        "m",
        "ma xlength",
        "axlength",
        "mmmmmmmmm",
    ] {
        assert_eq!(names_max_length_in(text.as_bytes()), naive(text.as_bytes()), "{text:?}");
    }
    assert!(names_max_length_in(b"aaaaaaaaaaaaaaaaaaaaaaMaXlEnGtH"));
}

/// The search for `accessbypermission` finds it in any case and anywhere, as the plain window search does.
#[test]
fn access_by_permission_is_found_in_any_case_anywhere() {
    let naive = |b: &[u8]| b.windows(18).any(|w| w.eq_ignore_ascii_case(b"accessbypermission"));
    for text in [
        "",
        "accessbypermission",
        "AccessByPermission = tabledata X = R;",
        "ACCESSBYPERMISSION",
        "xaccessbypermissionx",
        "accessbypermissio",
        "aaccessbypermission",
        "access by permission",
        "a",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        assert_eq!(
            names_access_by_permission_in(text.as_bytes()),
            naive(text.as_bytes()),
            "{text:?}"
        );
    }
}

/// A parse is a function of the text. When a dependency file fails to parse, it is parsed once more
/// (a fresh token cache, a fresh expansion); a second result that differs is a defect somewhere (a
/// race, a torn read of the archive) and is reported with both texts, and the clean parse is kept.
#[test]
fn a_second_parse_that_differs_is_reported_and_the_clean_one_is_kept() {
    let bad = Attempt {
        green: 1,
        problem: Some("f.al:7: expected `;` (`oAdd`)".into()),
    };
    let good = Attempt {
        green: 2,
        problem: None,
    };
    let (kept, flake) = settle(bad, || good);
    assert_eq!(kept.green, 2);
    assert!(kept.problem.is_none());
    let flake = flake.expect("the two parses differ");
    assert!(
        flake.contains("oAdd") && flake.contains("parsed again") && flake.contains("clean"),
        "{flake}"
    );
}

#[test]
fn a_failure_that_repeats_is_the_files_own_and_is_not_a_flake() {
    let bad = || Attempt {
        green: 1,
        problem: Some("f.al:7: expected `;` (`x`)".into()),
    };
    let (kept, flake) = settle(bad(), bad);
    assert_eq!(kept.problem.as_deref(), Some("f.al:7: expected `;` (`x`)"));
    assert!(flake.is_none());
    // A clean first parse is never parsed again.
    let (kept, flake) = settle(
        Attempt {
            green: 3,
            problem: None,
        },
        || panic!("no second parse"),
    );
    assert_eq!(kept.green, 3);
    assert!(flake.is_none());
}

fn app(id: &str, deps: &[&str], files: &[(&str, &str)]) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
    }
}

fn load(apps: Vec<AppSource>) -> Workspace {
    Workspace::load(apps, &al_version::corpus_symbols(27))
}

fn codes(ws: &Workspace) -> Vec<(&'static str, String)> {
    ws.diagnostics.iter().map(|d| (d.code, d.message.clone())).collect()
}

#[test]
fn a_table_moved_to_another_app_does_not_make_its_name_ambiguous() {
    // Base Application 27.5 keeps `table 230 "Source Code"` with `ObsoleteState = Moved` while
    // Business Foundation declares the live one; `Record "Source Code"` in Company-Initialize
    // names the live table.
    let live = app(
        "foundation",
        &[],
        &[(
            "sc.al",
            "namespace Microsoft.Foundation.AuditCodes; table 230 \"Source Code\" { fields { field(1; Code; Code[10]) { } } }",
        )],
    );
    let old = app(
        "base",
        &["foundation"],
        &[
            (
                "sc.al",
                "namespace Microsoft.Foundation.AuditCodes; table 230 \"Source Code\" { ObsoleteState = Moved; MovedTo = 'f3552374'; fields { field(1; Code; Code[10]) { } } }",
            ),
            (
                "ci.al",
                "namespace Microsoft.Foundation.Company; using Microsoft.Foundation.AuditCodes; codeunit 2 CI { var S: Record \"Source Code\"; }",
            ),
        ],
    );
    let ws = Workspace::load_apps(
        vec![live, old],
        vec![AppKind::Dependency, AppKind::Dependency],
        &al_version::corpus_symbols(27),
    );
    let file = ws.objects.iter().find(|o| o.name == "CI").unwrap().file;
    match ws.resolve_ref(file, ObjKind::Table, &["Source Code".to_string()]) {
        Resolution::Found(i) => assert_eq!(ws.apps[ws.objects[i].app].id, "foundation"),
        other => panic!("{other:?}"),
    }
}

mod relation_target;

/// Issue 15: one eager run of Microsoft's apps reported a parse error in a report that parsed fine in every
/// other run (a stray token next to a synthetic column routine). A parse is a function of the text, so
/// many threads parsing many files with synthetic routines must agree with one thread, every time. Not
/// a reproduction (the failure did not repeat in 36 runs); the guard against it.
#[test]
fn parsing_report_and_page_files_in_parallel_gives_the_same_trees_as_parsing_them_alone() {
    let report = |i: usize| {
        format!(
            "report {} R{i} {{ ProcessingOnly = true; dataset {{ dataitem(A; Integer) {{ column(Num; Number) {{ }} column(Twice; Number * 2) {{ }} dataitem(B; Integer) {{ column(Inner; Number + {i}) {{ }} }} }} }} \
             requestpage {{ layout {{ area(Content) {{ field(F; Opt) {{ ApplicationArea = All; Visible = Opt > {i}; }} }} }} }} var Opt: Integer; }}",
            60000 + i
        )
    };
    let page = |i: usize| {
        format!(
            "page {} P{i} {{ PageType = Card; layout {{ area(Content) {{ field(F; Opt) {{ ApplicationArea = All; Editable = Opt > {i}; Caption = 'x'; }} }} }} var Opt: Integer; }}",
            61000 + i
        )
    };
    let files: Vec<(String, String)> = (0..120)
        .map(|i| (format!("f{i:03}.al"), if i % 2 == 0 { report(i) } else { page(i) }))
        .collect();
    let dep = AppSource {
        id: "d".into(),
        name: "d".into(),
        dependencies: vec![],
        files,
    };
    let parse_all = |parallel: bool| {
        let ws = Workspace::load_apps(
            vec![dep.clone()],
            vec![AppKind::Dependency],
            &al_version::corpus_symbols(27),
        );
        let ids: Vec<usize> = (0..ws.files.len()).collect();
        if parallel {
            ids.par_iter().for_each(|&fi| {
                ws.green(fi);
            });
        }
        let trees: Vec<String> = ids.iter().map(|&fi| format!("{:?}", ws.green(fi))).collect();
        let synthetic: Vec<usize> = ids.iter().map(|&fi| ws.synthetic(fi).len()).collect();
        assert!(ws.lazy_parse_problems().is_empty(), "{:?}", ws.lazy_parse_problems());
        assert!(ws.parse_flakes().is_empty(), "{:?}", ws.parse_flakes());
        (trees, synthetic)
    };
    let alone = parse_all(false);
    assert!(alone.1.iter().all(|&n| n > 0), "every file has synthetic routines");
    for round in 0..40 {
        assert!(
            parse_all(true) == alone,
            "round {round}: a parallel parse differs from a serial one"
        );
    }
}

#[test]
fn files_objects_and_diagnostics_keep_input_order_with_parallel_parsing() {
    // Many files so that rayon really splits the work; a parse error and a
    // duplicate id are planted in the middle and must be reported in file order.
    let mut files: Vec<(String, String)> = (0..200)
        .map(|i| (format!("f{i:03}.al"), format!("codeunit {} C{i} {{ }}", 50000 + i)))
        .collect();
    files[100].1 = "codeunit 50100 C100 { procedure ( }".into();
    files[150].1 = "codeunit 50007 Dup { }".into();
    let a = AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files,
    };
    let b = app("b", &["a"], &[("z.al", "codeunit 60000 Z { }")]);
    let ws = load(vec![a.clone(), b.clone()]);
    let paths: Vec<_> = ws.files.iter().map(|f| f.path.as_str()).collect();
    let want: Vec<String> = a.files.iter().chain(&b.files).map(|f| f.0.clone()).collect();
    assert_eq!(paths, want.iter().map(String::as_str).collect::<Vec<_>>());
    let names: Vec<_> = ws.objects.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(names[..3], ["C0", "C1", "C2"]);
    assert_eq!(names.last(), Some(&"Z"));
    let diag_files: Vec<_> = ws.diagnostics.iter().map(|d| d.file.as_str()).collect();
    let mut sorted = diag_files.clone();
    sorted.sort();
    assert_eq!(diag_files, sorted, "diagnostics come in file order");
    assert!(diag_files.contains(&"f100.al") && diag_files.contains(&"f150.al"));
    // Loading is deterministic.
    let again = load(vec![a, b]);
    assert_eq!(codes(&ws), codes(&again));
    assert_eq!(
        ws.objects.iter().map(|o| (o.file, o.offset)).collect::<Vec<_>>(),
        again.objects.iter().map(|o| (o.file, o.offset)).collect::<Vec<_>>()
    );
}

#[test]
fn reference_diagnostics_keep_object_then_source_order_over_many_files() {
    // `resolve_references` runs per object on all cores (owner decision 2026-10-15, ADR-026
    // option 4). The list of diagnostics must be the list a serial loop gives: files in input
    // order, within a file by offset, and several diagnostics of one object in the order of
    // `extends`, `implements`, then type references in source order.
    let files: Vec<(String, String)> = (0..300)
        .map(|i| {
            (
                format!("f{i:03}.al"),
                format!(
                    "codeunit {} C{i} implements MissingI{i} {{ var R: Record MissingT{i}; L: List of [Codeunit MissingC{i}]; \
                     procedure P(x: Page MissingP{i}) begin end; }}\n\
                     tableextension {} E{i} extends MissingBase{i} {{ }}",
                    50000 + i,
                    70000 + i
                ),
            )
        })
        .collect();
    let mut want = Vec::new();
    for i in 0..300 {
        for (kind, name) in [
            ("interface", format!("MissingI{i}")),
            ("table", format!("MissingT{i}")),
            ("codeunit", format!("MissingC{i}")),
            ("page", format!("MissingP{i}")),
            ("table", format!("MissingBase{i}")),
        ] {
            want.push(("AR0002", format!("unresolved {kind} `{name}`")));
        }
    }
    let a = AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![],
        files,
    };
    for round in 0..10 {
        let ws = load(vec![a.clone()]);
        assert_eq!(codes(&ws), want, "round {round}");
        let offsets: Vec<_> = ws.diagnostics.iter().map(|d| (d.file.clone(), d.offset)).collect();
        let mut sorted = offsets.clone();
        sorted.sort();
        assert_eq!(offsets, sorted);
    }
}

#[test]
fn finds_objects_by_name_case_insensitively_and_by_id() {
    let ws = load(vec![app("a", &[], &[("t.al", "table 50900 \"ALT Universal\" { }")])]);
    assert_eq!(
        ws.find(ObjKind::Table, "alt universal"),
        Some(("ALT Universal".into(), Some(50900)))
    );
    assert_eq!(
        ws.find(ObjKind::Table, "50900"),
        Some(("ALT Universal".into(), Some(50900)))
    );
    assert_eq!(ws.find(ObjKind::Codeunit, "ALT Universal"), None);
}

#[test]
fn resolved_type_references_produce_no_diagnostics() {
    let ws = load(vec![app(
        "a",
        &[],
        &[
            ("t.al", "table 50900 \"ALT Universal\" { }"),
            (
                "c.al",
                "codeunit 60100 T { var R: Record \"ALT Universal\"; L: List of [Integer]; procedure P(x: Codeunit T) begin end; }",
            ),
        ],
    )]);
    assert_eq!(codes(&ws), vec![]);
}

#[test]
fn unresolved_object_reference_is_diagnosed() {
    let ws = load(vec![app(
        "a",
        &[],
        &[("c.al", "codeunit 1 C { var R: Record Customer; }")],
    )]);
    assert_eq!(codes(&ws), vec![("AR0002", "unresolved table `Customer`".to_string())]);
}

#[test]
fn references_inside_routines_and_nested_type_args_are_checked() {
    let ws = load(vec![app(
        "a",
        &[],
        &[(
            "c.al",
            "codeunit 1 C { procedure P(): Record X var L: List of [Codeunit Y]; begin end; }",
        )],
    )]);
    assert_eq!(
        codes(&ws),
        vec![
            ("AR0002", "unresolved table `X`".to_string()),
            ("AR0002", "unresolved codeunit `Y`".to_string())
        ]
    );
}

#[test]
fn objects_of_a_dependency_are_visible_but_not_the_reverse() {
    let ws = load(vec![
        app("base", &[], &[("t.al", "table 1 T { var C: Codeunit Top; }")]),
        app("top", &["base"], &[("c.al", "codeunit 2 Top { var R: Record T; }")]),
    ]);
    assert_eq!(codes(&ws), vec![("AR0002", "unresolved codeunit `Top`".to_string())]);
}

#[test]
fn duplicate_object_ids_and_names_are_diagnosed() {
    let ws = load(vec![app(
        "a",
        &[],
        &[("x.al", "codeunit 1 A { }\ncodeunit 1 B { }\ncodeunit 2 a { }")],
    )]);
    assert_eq!(
        codes(&ws),
        vec![
            ("AR0001", "duplicate codeunit id 1".to_string()),
            ("AR0001", "duplicate codeunit name `a`".to_string())
        ]
    );
}

#[test]
fn extends_and_implements_are_resolved() {
    let ws = load(vec![app(
        "a",
        &[],
        &[(
            "x.al",
            "interface I { }\nenum 1 E implements I, J { }\ntableextension 2 X extends Missing { }",
        )],
    )]);
    assert_eq!(
        codes(&ws),
        vec![
            ("AR0002", "unresolved interface `J`".to_string()),
            ("AR0002", "unresolved table `Missing`".to_string())
        ]
    );
}

#[test]
fn namespace_qualified_reference_resolves_in_that_namespace() {
    let ws = load(vec![app(
        "a",
        &[],
        &[
            ("b.al", "namespace System.Utilities;\ncodeunit 1 \"Temp Blob\" { }"),
            (
                "u.al",
                "codeunit 2 U { var B: Codeunit System.Utilities.\"Temp Blob\"; }",
            ),
        ],
    )]);
    assert_eq!(codes(&ws), vec![]);
}

#[test]
fn qualified_reference_does_not_fall_back_to_an_object_without_namespace() {
    let ws = load(vec![app(
        "a",
        &[],
        &[(
            "x.al",
            "codeunit 1 \"Temp Blob\" { }\ncodeunit 2 U { var B: Codeunit System.Utilities.\"Temp Blob\"; }",
        )],
    )]);
    assert_eq!(
        codes(&ws),
        vec![("AR0002", "unresolved codeunit `System.Utilities.Temp Blob`".to_string())]
    );
}

#[test]
fn namespace_qualified_reference_to_a_name_containing_a_dot() {
    let ws = load(vec![app(
        "a",
        &[],
        &[
            ("t.al", "namespace A.B;\ntable 1 \"Config. Package Table\" { }"),
            ("u.al", "codeunit 2 U { var R: Record A.B.\"Config. Package Table\"; }"),
        ],
    )]);
    assert_eq!(codes(&ws), vec![]);
}

fn lazy_ws() -> Workspace {
    let dep = app(
        "dep",
        &[],
        &[
            (
                "Dep/cust.al",
                "namespace Dep.Sales;\ntable 18 Customer { fields { field(1; \"No.\"; Code[20]) { } } }",
            ),
            ("Dep/item.al", "namespace Dep.Inv;\ntable 27 Item { }"),
            (
                "Dep/ext.al",
                "namespace Dep.Sales;\ntableextension 50 \"Cust Ext\" extends Customer { }",
            ),
            (
                "Dep/sub.al",
                "codeunit 60 Subs { [EventSubscriber(ObjectType::Table, Database::Customer, 'OnAfterInsertEvent', '', false, false)] local procedure A() begin end; }",
            ),
            ("Dep/impl.al", "codeunit 61 Impl implements \"Dep I\" { }"),
            (
                "Dep/broken.al",
                "table 70 Broken { fields { field(1; F; Integer) { } } procedure P( begin end; }",
            ),
        ],
    );
    let main = app(
        "main",
        &["dep"],
        &[("m.al", "codeunit 1 M { var C: Record Customer; }")],
    );
    Workspace::load_apps(
        vec![main, dep],
        vec![AppKind::Workspace, AppKind::Dependency],
        &["BC27PLUS".to_string()].into(),
    )
}

#[test]
fn dependency_files_are_scanned_at_load_and_parsed_on_first_use() {
    let ws = lazy_ws();
    let file = |p: &str| ws.files.iter().position(|f| f.path == p).unwrap();
    assert!(
        ws.files[file("m.al")].green.get().is_some(),
        "workspace files parse at load"
    );
    // The index knows every object, with its namespace.
    let customer = ws.objects.iter().position(|o| o.name == "Customer").unwrap();
    assert_eq!(ws.objects[customer].namespace.as_deref(), Some("Dep.Sales"));
    assert_eq!(ws.objects[customer].id, Some(18));
    // The workspace's own reference to Customer resolved without a diagnostic. That parsed
    // the file of Customer and no other.
    assert_eq!(ws.diagnostics, vec![]);
    assert!(ws.files[file("Dep/cust.al")].green.get().is_some());
    for p in [
        "Dep/item.al",
        "Dep/ext.al",
        "Dep/sub.al",
        "Dep/impl.al",
        "Dep/broken.al",
    ] {
        assert!(ws.files[file(p)].green.get().is_none(), "{p} is only scanned");
    }
}

/// A compile of a run that nobody edits drops the parse trees once it is lowered (al-testrun
/// `compile_within_limit`); what still needs a tree parses its file again and gets the same tree.
#[test]
fn dropped_trees_are_parsed_again_the_same() {
    let mut ws = lazy_ws();
    let all: Vec<usize> = (0..ws.files.len()).collect();
    let trees = |ws: &Workspace| all.iter().map(|&fi| format!("{:?}", ws.green(fi))).collect::<Vec<_>>();
    let found = |ws: &Workspace| {
        all.iter()
            .map(|&fi| (ws.file_problem(fi).map(str::to_string), ws.synthetic(fi).to_vec()))
            .collect::<Vec<_>>()
    };
    let before = trees(&ws);
    let found_before = found(&ws);
    ws.drop_trees();
    assert!(ws.files.iter().all(|f| f.green.get().is_none()), "no tree is kept");
    // What the first parse found besides the tree is kept: reading it parses nothing.
    assert_eq!(found(&ws), found_before);
    assert!(
        ws.files.iter().all(|f| f.green.get().is_none()),
        "reading what a parse found parses nothing"
    );
    assert_eq!(trees(&ws), before);
}

#[test]
fn extensions_and_subscribers_are_found_from_the_index() {
    let ws = lazy_ws();
    let ix = |n: &str| ws.objects.iter().position(|o| o.name == n).unwrap();
    assert_eq!(ws.extension_target(ix("Cust Ext")), Some(ix("Customer")));
    assert_eq!(ws.extensions_of(ix("Customer")), [ix("Cust Ext")]);
    assert_eq!(ws.extension_target(ix("Customer")), None);
    let subs = ws.subscribers_of(ix("Customer"));
    assert_eq!(subs.len(), 1);
    assert_eq!(subs[0].0, ix("Subs"));
    assert_eq!(ws.objects[subs[0].0].subscribers[subs[0].1].event, "OnAfterInsertEvent");
    assert_eq!(ws.implementers_of("Dep I"), [ix("Impl")]);
    assert!(ws.subscribers_of(ix("Item")).is_empty());
}

#[test]
fn a_dependency_object_whose_file_does_not_parse_is_unusable_and_says_why() {
    let ws = lazy_ws();
    let broken = ws.objects.iter().position(|o| o.name == "Broken").unwrap();
    assert!(!ws.usable(broken));
    let why = ws.unusable_reason(broken).unwrap();
    assert!(why.contains("Dep/broken.al") && why.contains("does not parse"), "{why}");
    // A reference to it resolves to nothing rather than to half a table.
    let fi = ws.files.iter().position(|f| f.path == "m.al").unwrap();
    assert_eq!(
        ws.resolve_ref(fi, ObjKind::Table, &["Broken".to_string()]),
        Resolution::Missing
    );
    assert!(ws.lazy_parse_problems().iter().any(|(p, _)| p == "Dep/broken.al"));
    // The healthy objects are unaffected.
    let customer = ws.objects.iter().position(|o| o.name == "Customer").unwrap();
    assert!(ws.usable(customer));
    assert_eq!(ws.unusable_reason(customer), None);
}

#[test]
fn clashes_between_dependency_objects_are_not_reported() {
    let ws = Workspace::load_apps(
        vec![app(
            "d",
            &[],
            &[("a.al", "codeunit 1 A { }\ncodeunit 1 B { }\ncodeunit 2 a { }")],
        )],
        vec![AppKind::Dependency],
        &Default::default(),
    );
    assert_eq!(ws.diagnostics, vec![]);
}

fn resolved(ws: &Workspace, file: &str, kind: ObjKind, parts: &[&str]) -> Option<(Option<String>, u32)> {
    let fi = ws.files.iter().position(|f| f.path == file).unwrap();
    let parts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
    match ws.resolve_ref(fi, kind, &parts) {
        Resolution::Found(i) => Some((ws.objects[i].namespace.clone(), ws.objects[i].id.unwrap())),
        _ => None,
    }
}

fn ns_app() -> Workspace {
    load(vec![app(
        "a",
        &[],
        &[
            (
                "local.al",
                "namespace X.Local;\ntable 1 \"Ship\" { }\ntable 4 \"Only Local\" { }",
            ),
            ("base.al", "namespace X.Base;\ntable 2 \"Ship\" { }"),
            ("global.al", "table 3 \"Glob\" { }"),
            ("user_local.al", "namespace X.Local;\ncodeunit 10 A { }"),
            ("user_using.al", "namespace X.Other;\nusing X.Base;\ncodeunit 11 B { }"),
            ("user_none.al", "codeunit 12 C { }"),
        ],
    )])
}

#[test]
fn qualified_reference_resolves_only_in_the_named_namespace() {
    let ws = ns_app();
    assert_eq!(
        resolved(&ws, "user_none.al", ObjKind::Table, &["X", "Base", "Ship"]),
        Some((Some("X.Base".into()), 2))
    );
    assert_eq!(
        resolved(&ws, "user_none.al", ObjKind::Table, &["X", "Local", "Ship"]),
        Some((Some("X.Local".into()), 1))
    );
    assert_eq!(
        resolved(&ws, "user_none.al", ObjKind::Table, &["X", "Base", "Only Local"]),
        None
    );
}

#[test]
fn unqualified_reference_prefers_the_own_namespace_then_usings() {
    let ws = ns_app();
    assert_eq!(
        resolved(&ws, "user_local.al", ObjKind::Table, &["Ship"]),
        Some((Some("X.Local".into()), 1))
    );
    assert_eq!(
        resolved(&ws, "user_using.al", ObjKind::Table, &["Ship"]),
        Some((Some("X.Base".into()), 2))
    );
}

#[test]
fn unqualified_reference_falls_back_to_a_unique_match_anywhere() {
    // Corpus evidence: files without namespace or using reference Base App
    // tables such as `Record Customer` (namespace Microsoft.Sales.Customer) and compile.
    let ws = ns_app();
    assert_eq!(
        resolved(&ws, "user_none.al", ObjKind::Table, &["Glob"]),
        Some((None, 3))
    );
    assert_eq!(
        resolved(&ws, "user_none.al", ObjKind::Table, &["Only Local"]),
        Some((Some("X.Local".into()), 4))
    );
}

#[test]
fn unqualified_reference_matching_several_namespaces_is_ambiguous() {
    let ws = ns_app();
    let fi = ws.files.iter().position(|f| f.path == "user_none.al").unwrap();
    assert!(matches!(
        ws.resolve_ref(fi, ObjKind::Table, &["Ship".to_string()]),
        Resolution::Ambiguous(_)
    ));
}

#[test]
fn qualified_extends_resolves_by_namespace() {
    let ws = load(vec![app(
        "a",
        &[],
        &[
            ("p.al", "namespace N.Local;\npage 1 \"Svc\" { }"),
            (
                "e.al",
                "pageextension 2 E1 extends N.Local.\"Svc\" { }\npageextension 3 E2 extends N.Other.\"Svc\" { }",
            ),
        ],
    )]);
    assert_eq!(
        codes(&ws),
        vec![("AR0002", "unresolved page `N.Other.Svc`".to_string())]
    );
}

/// A table that moved between apps stays in the old app as a stub marked `ObsoleteState = Moved`
/// (Base Application's "No. Series", which Business Foundation now declares). Microsoft's own
/// test libraries compile against both apps, so the name must reach the real table. Nothing in
/// the corpus pins this (docs/probes-needed.md, "Probe batch 4").
#[test]
fn a_moved_stub_does_not_make_the_real_table_ambiguous() {
    let bf = app(
        "bf",
        &[],
        &[(
            "t.al",
            "namespace M.NoSeries; table 308 \"No. Series\" { fields { field(1; Code; Code[20]) { } } }",
        )],
    );
    let base = app(
        "base",
        &["bf"],
        &[(
            "t.al",
            "namespace M.NoSeries; table 308 \"No. Series\" { ObsoleteState = Moved; MovedTo = 'x'; \
             fields { field(1; Code; Code[20]) { } } }",
        )],
    );
    let lib = app("lib", &["base", "bf"], &[("c.al", "codeunit 131000 \"Lib\" { }")]);
    let ws = Workspace::load_apps(
        vec![bf, base, lib],
        vec![AppKind::Dependency; 3],
        &al_version::corpus_symbols(27),
    );
    let file = ws.objects.iter().find(|o| o.name == "Lib").unwrap().file;
    let name = vec!["No. Series".to_string()];
    let Resolution::Found(t) = ws.resolve_ref(file, ObjKind::Table, &name) else {
        panic!("the moved stub made the name ambiguous");
    };
    assert_eq!(ws.apps[ws.objects[t].app].name, "bf");
    // Both stubs: nothing real to prefer, so it stays ambiguous.
    let only_stubs = Workspace::load_apps(
        vec![
            app("s1", &[], &[("t.al", "table 5 T { ObsoleteState = Moved; }")]),
            app("s2", &[], &[("t.al", "table 5 T { ObsoleteState = Moved; }")]),
            app("lib", &["s1", "s2"], &[("c.al", "codeunit 9 \"Lib\" { }")]),
        ],
        vec![AppKind::Dependency; 3],
        &al_version::corpus_symbols(27),
    );
    let file = only_stubs.objects.iter().find(|o| o.name == "Lib").unwrap().file;
    assert!(matches!(
        only_stubs.resolve_ref(file, ObjKind::Table, &["T".to_string()]),
        Resolution::Ambiguous(_)
    ));
}

/// A file with neither namespace nor `using` names a table that two visible apps declare in different
/// namespaces, one of them `ObsoleteState = Removed` (Subscription Billing's SubContrAnalysisEntry.Table.al:238
/// `TableRelation = "Dimension Set Entry"`: Base Application's table and Power BI Report embeddings' removed
/// table 36950 of the same name). A reference to a removed table does not compile (corpus must-not-compile
/// 0970-obsolete-removed-table.al, AL0433), and BC compiles the file, so the name means the table that is not
/// removed. Two removed candidates stay ambiguous.
#[test]
fn a_removed_table_does_not_make_the_real_table_ambiguous() {
    let base = app(
        "base",
        &[],
        &[(
            "t.al",
            "namespace M.Finance.Dimension; table 480 \"Dimension Set Entry\" { fields { field(1; Id; Integer) { } } }",
        )],
    );
    let pbi = app(
        "pbi",
        &["base"],
        &[(
            "t.al",
            "namespace M.PowerBIReports; table 36950 \"Dimension Set Entry\" { Access = Internal; \
             ObsoleteState = Removed; ObsoleteReason = 'r'; ObsoleteTag = '25.0'; fields { field(1; Id; Integer) { } } }",
        )],
    );
    let sub = app(
        "sub",
        &["base", "pbi"],
        &[(
            "e.al",
            "table 8019 \"Entry\" { fields { field(1; Id; Integer) { TableRelation = \"Dimension Set Entry\"; } } }",
        )],
    );
    let ws = Workspace::load_apps(
        vec![base, pbi, sub],
        vec![AppKind::Dependency; 3],
        &al_version::corpus_symbols(29),
    );
    let file = ws.objects.iter().find(|o| o.name == "Entry").unwrap().file;
    let name = vec!["Dimension Set Entry".to_string()];
    let Resolution::Found(t) = ws.resolve_relation_target(file, ObjKind::Table, &name) else {
        panic!("the removed table made the name ambiguous");
    };
    assert_eq!(ws.apps[ws.objects[t].app].name, "base");
    let only_removed = Workspace::load_apps(
        vec![
            app(
                "r1",
                &[],
                &[("t.al", "namespace A; table 5 T { ObsoleteState = Removed; }")],
            ),
            app(
                "r2",
                &[],
                &[("t.al", "namespace B; table 6 T { ObsoleteState = Removed; }")],
            ),
            app("lib", &["r1", "r2"], &[("c.al", "codeunit 9 \"Lib\" { }")]),
        ],
        vec![AppKind::Dependency; 3],
        &al_version::corpus_symbols(29),
    );
    let file = only_removed.objects.iter().find(|o| o.name == "Lib").unwrap().file;
    assert!(matches!(
        only_removed.resolve_ref(file, ObjKind::Table, &["T".to_string()]),
        Resolution::Ambiguous(_)
    ));
}

#[test]
fn parse_errors_are_reported_as_diagnostics() {
    let ws = load(vec![app("a", &[], &[("x.al", "codeunit 1 A { 42 }")])]);
    assert_eq!(ws.diagnostics.len(), 1);
    assert_eq!(ws.diagnostics[0].code, "AR0000");
}

/// Everything a load builds, as text: the maps sorted, so two workspaces compare field by field.
fn fingerprint(ws: &Workspace) -> String {
    fn sorted<K: std::fmt::Debug, V: std::fmt::Debug>(m: &HashMap<K, V>) -> Vec<String> {
        let mut v: Vec<String> = m.iter().map(|(k, v)| format!("{k:?}={v:?}")).collect();
        v.sort();
        v
    }
    let files: Vec<String> = ws
        .files
        .iter()
        .map(|f| {
            format!(
                "{} {} {} {} {:?} {:?}",
                f.path, f.app, f.file_in_app, f.lazy, f.namespace, f.usings
            )
        })
        .collect();
    let dotnet: Vec<String> = ws
        .dotnet
        .iter()
        .map(|d| format!("{} {} {:?}", d.app, d.file, d.decl))
        .collect();
    format!(
        "{:?}\n{files:?}\n{dotnet:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{} {} {}",
        ws.objects,
        ws.diagnostics,
        ws.kinds,
        ws.visible,
        ws.symbols,
        ws.ext_target,
        sorted(&ws.by_name),
        sorted(&ws.by_id),
        sorted(&ws.ext_of),
        sorted(&ws.subscribers_of),
        sorted(&ws.implementers_of),
        (sorted(&ws.iface_base), sorted(&ws.iface_derived)),
        ws.apps.len(),
        ws.usable_cache.len(),
        ws.lazy_text.len(),
    )
}

/// ADR-022 amendment 2026-10-27, decision 2: a workspace grown by `append_apps` equals one loaded with all the apps.
#[test]
fn a_workspace_grown_by_appended_apps_equals_one_loaded_with_them_all() {
    let base = app(
        "base",
        &[],
        &[
            ("t.al", "table 10 Cust { fields { field(1; No; Code[20]) { } } }"),
            (
                "i.al",
                "interface IGreet { procedure Hello(): Text; } interface IMore extends IGreet { procedure More(); }",
            ),
            ("e.al", "enum 11 Kind { Extensible = true; value(0; None) { } }"),
            (
                "c.al",
                "codeunit 12 Pub { [IntegrationEvent(false, false)] procedure OnThing() begin end; }",
            ),
        ],
    );
    let lib = app(
        "lib",
        &["base"],
        &[(
            "l.al",
            "tableextension 20 LibCust extends Cust { fields { field(20; Note; Text[20]) { } } }
             codeunit 21 LibGreet implements IGreet { procedure Hello(): Text begin end; }
             codeunit 22 LibSub { [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)] local procedure S() begin end; }",
        )],
    );
    let top = app(
        "top",
        &["lib", "base"],
        &[
            (
                "t1.al",
                "tableextension 30 TopCust extends Cust { fields { field(30; Top; Text[20]) { } } }
                 enumextension 31 TopKind extends Kind { value(1; Top) { } }
                 interface ITop extends IMore { procedure Top(); }",
            ),
            (
                "t2.al",
                "codeunit 32 TopGreet implements IGreet { procedure Hello(): Text begin end; }
                 codeunit 33 TopSub { [EventSubscriber(ObjectType::Codeunit, 12, 'OnThing', '', false, false)] local procedure S() begin end; }
                 codeunit 34 Broken { procedure P() begin x := ; end; }",
            ),
        ],
    );
    let symbols = al_version::corpus_symbols(27);
    let lib_symbols: BTreeSet<String> = ["LIBSYM".to_string()].into();
    let all = vec![base.clone(), lib.clone(), top.clone()];
    let app_symbols = vec![None, Some(lib_symbols.clone()), None];
    let whole = Workspace::load_apps_symbols(all, vec![AppKind::Dependency; 3], &symbols, &app_symbols, Vec::new());
    let mut grown = Workspace::load_apps_symbols(vec![base], vec![AppKind::Dependency], &symbols, &[], Vec::new());
    // Something asked of the smaller workspace, as a layer build does, must not change the result.
    for i in 0..grown.objects.len() {
        grown.usable(i);
    }
    grown.restrict_to(vec![false; grown.objects.len()]);
    grown.append_apps(vec![lib], &symbols, &[Some(lib_symbols)]).unwrap();
    grown.append_apps(vec![top], &symbols, &[]).unwrap();
    assert_eq!(grown.loaded_files(), whole.loaded_files());
    assert_eq!(fingerprint(&grown), fingerprint(&whole));
    assert!(grown.compiled.get().is_none());
}

/// ADR-063: a workspace that holds workspace apps grows by workspace and dependency apps as one loaded with them all.
#[test]
fn a_workspace_with_workspace_apps_grown_by_apps_of_both_kinds_equals_one_loaded_with_them_all() {
    let base = app(
        "base",
        &[],
        &[(
            "b.al",
            "table 10 Cust { fields { field(1; No; Code[20]) { } } }
             codeunit 12 Pub { [IntegrationEvent(false, false)] procedure OnThing() begin end; }",
        )],
    );
    let main = app(
        "main",
        &["base"],
        &[("m.al", "codeunit 50 M { procedure P() var C: Record Cust; begin end; }")],
    );
    let extra = app(
        "extra",
        &["base"],
        &[(
            "x.al",
            "tableextension 20 XCust extends Cust { fields { field(20; Note; Text[20]) { } } }
             codeunit 22 XSub { [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)] local procedure S() begin end; }",
        )],
    );
    let suite = app(
        "suite",
        &["main", "extra", "base"],
        &[(
            "s.al",
            "codeunit 60 S { procedure P() var M: Codeunit M; begin y := ; end; }",
        )],
    );
    let symbols = al_version::corpus_symbols(27);
    let kinds = vec![
        AppKind::Workspace,
        AppKind::Dependency,
        AppKind::Dependency,
        AppKind::Workspace,
    ];
    let whole = Workspace::load_apps_symbols(
        vec![main.clone(), base.clone(), extra.clone(), suite.clone()],
        kinds,
        &symbols,
        &[],
        Vec::new(),
    );
    let mut grown = Workspace::load_apps_symbols(
        vec![main, base],
        vec![AppKind::Workspace, AppKind::Dependency],
        &symbols,
        &[],
        Vec::new(),
    );
    grown.restrict_to(vec![false; grown.objects.len()]);
    grown
        .append_apps_of(
            vec![extra, suite],
            vec![AppKind::Dependency, AppKind::Workspace],
            &symbols,
            &[],
        )
        .unwrap();
    assert_eq!(grown.kinds, whole.kinds);
    assert_eq!(grown.loaded_files(), whole.loaded_files());
    assert_eq!(fingerprint(&grown), fingerprint(&whole));
    assert!(grown.compiled.get().is_none());
}

/// ADR-063: a workspace cut back to its first apps equals one loaded with those apps only, and grows again as one.
#[test]
fn a_workspace_cut_back_to_its_first_apps_equals_one_loaded_with_them() {
    let base = app(
        "base",
        &[],
        &[(
            "b.al",
            "table 10 Cust { fields { field(1; No; Code[20]) { } } }
             interface IGreet { procedure Hello(): Text; }
             codeunit 12 Pub { [IntegrationEvent(false, false)] procedure OnThing() begin end; }",
        )],
    );
    let main = app(
        "main",
        &["base"],
        &[("m.al", "codeunit 50 M { procedure P() var C: Record Cust; begin end; }")],
    );
    let suite = app(
        "suite",
        &["main", "base"],
        &[(
            "s.al",
            "tableextension 20 SCust extends Cust { fields { field(20; Note; Text[20]) { } } }
             codeunit 21 SGreet implements IGreet { procedure Hello(): Text begin end; }
             interface IMore extends IGreet { procedure More(); }
             codeunit 22 SSub { [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)] local procedure S() begin end; }
             dotnet { assembly(mscorlib) { type(System.Text.StringBuilder; SB) { } } }
             codeunit 23 Broken { procedure P() begin x := ; end; }",
        )],
    );
    let symbols = al_version::corpus_symbols(27);
    let two = vec![AppKind::Workspace, AppKind::Dependency];
    let alone = Workspace::load_apps_symbols(vec![main.clone(), base.clone()], two.clone(), &symbols, &[], Vec::new());
    let mut cut = Workspace::load_apps_symbols(
        vec![main.clone(), base.clone(), suite.clone()],
        vec![AppKind::Workspace, AppKind::Dependency, AppKind::Workspace],
        &symbols,
        &[],
        Vec::new(),
    );
    cut.restrict_to(vec![false; cut.objects.len()]);
    let removed = cut.truncate_apps(2);
    assert_eq!(removed.len(), 1, "the cut apps are given back");
    assert_eq!(fingerprint(&cut), fingerprint(&alone));
    assert!(cut.compiled.get().is_none());
    cut.append_apps_of(vec![suite.clone()], vec![AppKind::Workspace], &symbols, &[])
        .unwrap();
    let whole = Workspace::load_apps_symbols(
        vec![main, base, suite],
        vec![AppKind::Workspace, AppKind::Dependency, AppKind::Workspace],
        &symbols,
        &[],
        Vec::new(),
    );
    assert_eq!(fingerprint(&cut), fingerprint(&whole));
}

/// An app already loaded that names a dependency only an appended app provides would see more after the append
/// than it did: the workspace refuses to grow then, and stays as it was.
#[test]
fn an_append_that_would_change_what_a_loaded_app_sees_is_refused() {
    let symbols = al_version::corpus_symbols(27);
    let early = app("early", &["late"], &[("a.al", "codeunit 1 A { }")]);
    let mut ws = Workspace::load_apps_symbols(vec![early], vec![AppKind::Dependency], &symbols, &[], Vec::new());
    let before = fingerprint(&ws);
    let late = app("late", &[], &[("b.al", "codeunit 2 B { }")]);
    assert!(ws.append_apps(vec![late], &symbols, &[]).is_err());
    assert_eq!(fingerprint(&ws), before);
}
