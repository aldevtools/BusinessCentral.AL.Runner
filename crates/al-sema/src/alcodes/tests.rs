use super::*;
use crate::workspace::AppSource;
use al_builtins::catalog::Catalog;

fn load(files: &[&str]) -> Workspace {
    Workspace::load(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: files
                .iter()
                .enumerate()
                .map(|(i, s)| (format!("f{i}.al"), s.to_string()))
                .collect(),
        }],
        &Default::default(),
    )
}

/// The AL codes of a compile of `files`, sorted.
fn codes(files: &[&str]) -> Vec<String> {
    let ws = load(files);
    let m = crate::members::build(&ws);
    let r = crate::check::check(&ws, &m, &Catalog::load());
    let mut v: Vec<String> = r
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("AL"))
        .map(|d| d.code.to_string())
        .collect();
    v.sort();
    v
}

// Must-not-compile case 2253 (corpus branch al-runtime-rs-staging-pub-tp): a text attribute after a text element.
#[test]
fn a_text_attribute_after_a_text_element_is_al0335() {
    let bad = "xmlport 50000 X { schema { textelement(root) { textelement(entry) { textelement(inner) { } textattribute(kind) { } } } } }";
    assert_eq!(codes(&[bad]), vec!["AL0335".to_string()]);
    let good = "xmlport 50000 X { schema { textelement(root) { textelement(entry) { textattribute(kind) { } textelement(inner) { } } } } }";
    assert!(codes(&[good]).is_empty());
}

// Must-not-compile cases 2330-2332 (corpus branch al-runtime-rs-staging-y12, al-runner issue 6436): a trailing
// comma after a Label property is AL0301, two commas after the text AL0107; one comma after the text compiles
// (text/lt-label-textbuilder/TxCLabel.al Y12_Label_TrailingComma).
#[test]
fn label_trailing_commas_al0301_and_al0107() {
    let cu = |decl: &str| format!("codeunit 50000 \"MNC Codeunit\"\n{{\n    var\n        SetupLbl: {decl};\n}}\n");
    assert_eq!(codes(&[&cu("Label 'L', Locked = true,")]), vec!["AL0301".to_string()]);
    assert_eq!(codes(&[&cu("Label 'C', Comment = 'c',")]), vec!["AL0301".to_string()]);
    assert_eq!(codes(&[&cu("Label 'T',,")]), vec!["AL0107".to_string()]);
    assert!(codes(&[&cu("Label 'Document Setup',")]).is_empty());
    assert!(codes(&[&cu("Label 'x' ,")]).is_empty());
    assert!(codes(&[&cu("Label 'x', Locked = true")]).is_empty());
}

#[test]
fn al0301_is_checked_only_for_a_label_trailing_comma() {
    assert_eq!(
        not_checked_reason_for("AL0301", "A list must end with a member; not a separator ,."),
        None
    );
    assert!(not_checked_reason_for("AL0301", "A list must end with a member; not a separator ;.").is_some());
}

fn ctx_codes(src: &str, ctx: &AppContext) -> Vec<String> {
    let ws = load(&[src]);
    check_context(&ws, 0, ctx).iter().map(|d| d.code.to_string()).collect()
}

const BASE: &str = "table 50000 T { fields { field(1; Id; Integer) { } } }
        page 50001 BP { PageType = Card; SourceTable = T; var Hidden: Integer; protected var Shown: Integer;
          local procedure Secret(): Integer begin exit(1); end;
          protected procedure Bump(): Integer begin exit(1); end;
          procedure Plus(a: Integer): Integer begin exit(a); end; }";

fn ext(body: &str) -> String {
    format!("pageextension 50002 BPE extends BP {{ {body} }}")
}

/// The platform table Integer lives in namespace System.Utilities: code in a namespace names it bare only through
/// `using System.Utilities;` (must-not-compile 2081, 2082: AL0185 `Table 'Integer' is missing`; misc/lt-inferred-behaviors/
/// RiNsPlatformTable.al:19 compiles with the using; al-runner issue 5694).
#[test]
fn al0185_integer_from_a_namespace_without_its_using() {
    const PLATFORM: &str =
        "namespace System.Utilities; table 2000000026 Integer { fields { field(1; Number; Integer) { } } }";
    let rec = |head: &str| format!("{head} codeunit 50100 C {{ procedure F() var N: Record Integer; begin end; }}");
    let dataitem = |head: &str| {
        format!("{head} report 50101 R {{ ProcessingOnly = true; dataset {{ dataitem(Number; Integer) {{ }} }} }}")
    };
    for platform in [&[PLATFORM][..], &[][..]] {
        let with = |src: String| {
            let mut f: Vec<&str> = platform.to_vec();
            f.push(Box::leak(src.into_boxed_str()));
            codes(&f)
        };
        assert_eq!(with(rec("namespace Repro;")), vec!["AL0185"]);
        assert_eq!(with(dataitem("namespace Repro;")), vec!["AL0185"]);
        assert!(with(rec("namespace Repro; using System.Utilities;")).is_empty());
        assert!(with(dataitem("namespace Repro; using System.Utilities;")).is_empty());
        assert!(with(rec("")).is_empty());
        assert!(with(dataitem("")).is_empty());
    }
    let ws = load(&[PLATFORM, &rec("namespace Repro;")]);
    let r = crate::check::check(&ws, &crate::members::build(&ws), &Catalog::load());
    let d = r.diagnostics.iter().find(|d| d.code == "AL0185").expect("AL0185");
    assert_eq!(d.message, "Table 'Integer' is missing");
    // A table Integer of the file's own namespace is found.
    let own = "namespace Repro; table 50102 Integer { fields { field(1; Number; Integer) { } } }";
    assert!(codes(&[PLATFORM, own, &rec("namespace Repro;")]).is_empty());
}

#[test]
fn al0162_oninit_in_a_page_extension() {
    let e = ext("trigger OnInit() begin end;");
    assert_eq!(codes(&[BASE, &e]), vec!["AL0162"]);
    // OnOpenPage is allowed.
    let ok = ext("trigger OnOpenPage() begin end;");
    assert!(codes(&[BASE, &ok]).is_empty());
    // Corpus 0017, 0018.
    let find = ext("trigger OnFindRecord(Which: Text): Boolean begin exit(true); end;");
    assert_eq!(codes(&[BASE, &find]), vec!["AL0162"]);
    let next = ext("trigger OnNextRecord(Steps: Integer): Integer begin exit(0); end;");
    assert_eq!(codes(&[BASE, &next]), vec!["AL0162"]);
}

#[test]
fn al0440_extension_procedure_with_the_signature_of_a_base_procedure() {
    assert_eq!(
        codes(&[BASE, &ext("procedure Bump(): Integer begin exit(2); end;")]),
        vec!["AL0440"]
    );
    // Another parameter list is an overload, not a clash.
    assert!(codes(&[BASE, &ext("procedure Plus(a: Text): Integer begin exit(2); end;")]).is_empty());
    // A local base procedure is not reachable, so it is not a clash.
    assert!(codes(&[BASE, &ext("procedure Secret(): Integer begin exit(2); end;")]).is_empty());
    // al-runner issue 6043: a local procedure of the extension is not reachable from outside it, so it does not
    // clash with a protected or public base procedure (alc compiles it, same app or another app).
    assert!(codes(&[BASE, &ext("local procedure Bump(): Integer begin exit(2); end;")]).is_empty());
    assert!(
        codes(&[
            BASE,
            &ext("local procedure Plus(a: Integer): Integer begin exit(2); end;")
        ])
        .is_empty()
    );
    // A protected extension procedure still clashes (alc: AL0440 in the same app).
    assert_eq!(
        codes(&[BASE, &ext("protected procedure Bump(): Integer begin exit(2); end;")]),
        vec!["AL0440"]
    );
    // An internal extension procedure compiles next to a protected base procedure in the same app (alc: only a bare
    // call of it in the extension is AL0196). Another app and the other access pairs: al-test-cli
    // tests/user_reports.rs, issue_6043_extension_procedure_named_like_a_base_procedure_compiles_as_alc_does_and_its_calls_refuse.
    assert!(codes(&[BASE, &ext("internal procedure Bump(): Integer begin exit(2); end;")]).is_empty());
}

/// Must-not-compile 2238 to 2240 and 2244 (corpus branch al-runtime-rs-staging-pub-ti): in one app, a bare call of a
/// local or internal extension procedure that a base page procedure repeats is ambiguous (AL0196).
#[test]
fn al0196_bare_call_of_an_extension_procedure_the_base_page_repeats() {
    let call = |decl: &str| {
        ext(&format!(
            "{decl} trigger OnOpenPage() var i: Integer; begin i := Bump(); end;"
        ))
    };
    assert_eq!(
        codes(&[BASE, &call("local procedure Bump(): Integer begin exit(2); end;")]),
        vec!["AL0196"]
    );
    assert_eq!(
        codes(&[BASE, &call("internal procedure Bump(): Integer begin exit(2); end;")]),
        vec!["AL0196"]
    );
    // Declared and not called: it compiles.
    assert!(codes(&[BASE, &ext("local procedure Bump(): Integer begin exit(2); end;")]).is_empty());
    // A call on the page from elsewhere is not the bare call.
    assert!(codes(&[BASE, &ext("local procedure Other(): Integer begin exit(2); end;")]).is_empty());
}

/// al-runner issue 6012: a page and its extension each get a generated `__visible_N` routine for a `Visible = Flag;`
/// property, named from the property's offset in its own object. When both offsets are equal the names are equal, but
/// the routines are not AL methods: alc compiles the app, so neither AL0440 nor any other AL code is reported. The
/// offsets must not depend on the line endings either (the reporter's file failed with CRLF and compiled with LF).
#[test]
fn al0440_not_reported_for_generated_control_routines_of_a_page_and_its_extension() {
    let table = "table 50000 T { fields { field(1; Id; Integer) { } } }";
    let page = |nl: &str, pad: usize| {
        format!(
            "page 50001 BP{nl}{{{nl}PageType = Card; SourceTable = T;{nl}layout {{ area(Content) {{{nl}{}field(Id; Rec.Id) {{ Visible = Flag; }}{nl}}} }}{nl}var Flag: Boolean;{nl}}}",
            " ".repeat(pad)
        )
    };
    let ext = |nl: &str, pad: usize| {
        format!(
            "pageextension 50002 BPE extends BP{nl}{{{nl}layout {{ addlast(Content) {{{nl}{}field(Id2; Rec.Id) {{ Visible = Flag2; }}{nl}}} }}{nl}var Flag2: Boolean;{nl}}}",
            " ".repeat(pad)
        )
    };
    let at = |s: &str| s.find("Visible").unwrap();
    for (page_nl, ext_nl) in [("\n", "\n"), ("\r\n", "\r\n"), ("\r\n", "\n"), ("\n", "\r\n")] {
        // Pad the earlier of the two so the Visible properties sit at the same offset of their objects.
        let (p0, e0) = (at(&page(page_nl, 0)), at(&ext(ext_nl, 0)));
        let (p, e) = (page(page_nl, e0.saturating_sub(p0)), ext(ext_nl, p0.saturating_sub(e0)));
        assert_eq!(at(&p), at(&e));
        let got = codes(&[table, &p, &e]);
        assert!(got.is_empty(), "{got:?} {:?}", (page_nl, ext_nl));
    }
}

#[test]
fn al0440_second_procedure_with_the_signature_of_an_earlier_one_in_a_codeunit() {
    let dup =
        "codeunit 50000 C { local procedure Foo(A: Integer) begin end; local procedure foo(B: Integer) begin end; }";
    assert_eq!(codes(&[dup]), vec!["AL0440"]);
    // Another parameter type or var-ness is an overload; a table is not judged.
    let overload = "codeunit 50000 C { procedure Foo(A: Integer) begin end; procedure Foo(A: Text) begin end; procedure Foo(var A: Integer) begin end; }";
    assert!(codes(&[overload]).is_empty());
    // Corpus 0042: a report extension procedure with the signature of a protected base procedure.
    let table = "table 50001 T { fields { field(1; Id; Integer) { } } }";
    let base = "report 50002 BR { dataset { dataitem(Row; T) { column(Id; Id) { } } } protected procedure Bump(): Integer begin exit(1); end; }";
    let ext = "reportextension 50003 BRE extends BR { procedure Bump(): Integer begin exit(2); end; }";
    assert_eq!(codes(&[table, base, ext]), vec!["AL0440"]);
}

#[test]
fn al0161_non_protected_variable_and_local_procedure_of_the_base() {
    let hidden = ext("trigger OnOpenPage() begin Hidden := 1; end;");
    assert_eq!(codes(&[BASE, &hidden]), vec!["AL0161"]);
    let secret = ext("trigger OnOpenPage() var V: Integer; begin V := Secret(); end;");
    assert_eq!(codes(&[BASE, &secret]), vec!["AL0161"]);
    // Protected variable and protected procedure are fine; so is the extension's own variable
    // named like a base variable.
    let ok = ext(
        "var Hidden: Integer; trigger OnOpenPage() var V: Integer; begin Shown := 1; Hidden := 1; V := Bump(); end;",
    );
    assert!(codes(&[BASE, &ok]).is_empty());
}

/// The codes of `files` compiled as an app that declares the dependency `dep`; `loaded` is the source of
/// that dependency, or `None` when it is declared but not loaded (`--no-deps`).
fn codes_with_dependency(files: &[&str], loaded: Option<&str>) -> Vec<String> {
    let mut v: Vec<String> = dependency_diagnostics(files, loaded)
        .into_iter()
        .map(|m| m.split(' ').next().unwrap_or_default().to_string())
        .collect();
    v.sort();
    v
}

/// The `code message` lines of `files` compiled with the dependency `dep` loaded from `loaded` (see `codes_with_dependency`).
fn messages_with_dependency(files: &[&str], loaded: &str) -> Vec<String> {
    dependency_diagnostics(files, Some(loaded))
}

fn dependency_diagnostics(files: &[&str], loaded: Option<&str>) -> Vec<String> {
    let mut apps = vec![AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec!["dep".into()],
        files: files
            .iter()
            .enumerate()
            .map(|(i, s)| (format!("f{i}.al"), s.to_string()))
            .collect(),
    }];
    let mut kinds = vec![crate::workspace::AppKind::Workspace];
    if let Some(src) = loaded {
        apps.push(AppSource {
            id: "dep".into(),
            name: "dep".into(),
            dependencies: vec![],
            files: vec![("d.al".into(), src.to_string())],
        });
        kinds.push(crate::workspace::AppKind::Dependency);
        // With every dependency loaded, the implicit one, the platform app, is loaded too.
        apps.push(AppSource {
            id: crate::workspace::PLATFORM_APP_ID.into(),
            name: "System".into(),
            dependencies: vec![],
            files: vec![],
        });
        kinds.push(crate::workspace::AppKind::Dependency);
    }
    let ws = Workspace::load_apps(apps, kinds, &Default::default());
    let m = crate::members::build(&ws);
    let r = crate::check::check(&ws, &m, &Catalog::load());
    r.diagnostics
        .iter()
        .filter(|d| d.code.starts_with("AL"))
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn al0118_codeunit_name_is_not_judged_when_a_dependency_is_not_loaded() {
    // Microsoft's "Global Triggers" lives in Base Application. Without the dependency (`--no-deps`) the
    // name may exist, so neither the expression nor the subscriber is refused.
    let user = "codeunit 50000 C { procedure P(): Integer begin exit(Codeunit::\"Global Triggers\"); end; }";
    let sub = "codeunit 50001 S { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnX', '', false, false)] local procedure H() begin end; }";
    assert!(codes_with_dependency(&[user, sub], None).is_empty());
    // The dependency is loaded and has no such codeunit: the name does not exist.
    let other = "codeunit 60000 Other { }";
    assert_eq!(codes_with_dependency(&[user], Some(other)), vec!["AL0118"]);
    assert_eq!(codes_with_dependency(&[sub], Some(other)), vec!["AL0118"]);
    // And it resolves when the dependency has it.
    let gt = "codeunit 60001 \"Global Triggers\" { }";
    assert!(codes_with_dependency(&[user, sub], Some(gt)).is_empty());
}

/// The codes of `files` compiled as an app next to the platform app System (`PLATFORM_APP_ID`), whose
/// source is `platform`. Every app sees the platform app without declaring it.
fn codes_with_platform(files: &[&str], platform: &str) -> Vec<String> {
    let mut v: Vec<String> = messages_with_platform(files, platform)
        .into_iter()
        .map(|m| m.split(' ').next().unwrap_or_default().to_string())
        .collect();
    v.sort();
    v
}

/// The `code message` lines of `files` compiled next to the platform app (see `codes_with_platform`).
fn messages_with_platform(files: &[&str], platform: &str) -> Vec<String> {
    let apps = vec![
        AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: files
                .iter()
                .enumerate()
                .map(|(i, s)| (format!("f{i}.al"), s.to_string()))
                .collect(),
        },
        AppSource {
            id: crate::workspace::PLATFORM_APP_ID.into(),
            name: "System".into(),
            dependencies: vec![],
            files: vec![("s.al".into(), platform.to_string())],
        },
    ];
    let kinds = vec![
        crate::workspace::AppKind::Workspace,
        crate::workspace::AppKind::Dependency,
    ];
    let ws = Workspace::load_apps(apps, kinds, &Default::default());
    let m = crate::members::build(&ws);
    crate::check::check(&ws, &m, &Catalog::load())
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("AL"))
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

/// must-not-compile 2514-currreport-requestoptionspage.al: `CurrReport.RequestOptionsPage` is AL0161; the bare
/// `RequestOptionsPage` compiles (report/lt-y40-bare-report-run/Y40BareReportRun.al:156-166).
#[test]
fn al0161_currreport_request_options_page() {
    let src = |body: &str| {
        format!(
            "report 50000 R {{ requestpage {{ layout {{ area(Content) {{ field(N; Note) {{ }} }} }} }}
    var Note: Text[30];
    procedure Refresh() begin {body} end; }}"
        )
    };
    assert_eq!(
        messages(&src("CurrReport.RequestOptionsPage.Update();")),
        vec!["AL0161 'RequestOptionsPage' is inaccessible due to its protection level".to_string()]
    );
    assert!(codes(&[&src("RequestOptionsPage.Update(false);")]).is_empty());
}

/// al-runner #6631: real BC compiles `CurrReport.RequestOptionsPage.Update(false)` in all request page code (control,
/// action and request page triggers, a procedure of the requestpage) and gives AL0161 everywhere else in the report: its
/// triggers, its data item triggers (must-not-compile 2580-2584) and its procedures (2514). Pin:
/// report/lt-request-options-page-access/RequestOptionsPageAccess.al.
#[test]
fn currreport_request_options_page_by_location() {
    let src = |field_trigger: &str, page_trigger: &str, report_trigger: &str, item_trigger: &str| {
        format!(
            "report 50100 \"RO Report\" {{ ProcessingOnly = true;
    dataset {{ dataitem(\"Integer\"; \"Integer\") {{ trigger OnPreDataItem() begin {item_trigger} end; }} }}
    requestpage {{
        layout {{ area(content) {{ field(Direction; Direction) {{
            trigger OnValidate() begin {field_trigger} end; }} }} }}
        trigger OnOpenPage() begin {page_trigger} end;
    }}
    trigger OnPreReport() begin {report_trigger} end;
    var Direction: Option Export,Import;
    procedure Refresh() begin RequestOptionsPage.Update(false); end;
    local procedure RefreshLocal() begin RequestOptionsPage.Update(); end; }}"
        )
    };
    let update = "CurrReport.RequestOptionsPage.Update(false);";
    let denied = vec!["AL0161 'RequestOptionsPage' is inaccessible due to its protection level".to_string()];
    // Request page code compiles.
    assert!(codes(&[&src(update, "", "", "")]).is_empty());
    assert!(codes(&[&src("", update, "", "")]).is_empty());
    assert!(codes(&[&src("", "CurrReport.RequestOptionsPage.Caption := 'x';", "", "")]).is_empty());
    // Report triggers and data item triggers are AL0161.
    assert_eq!(messages(&src("", "", update, "")), denied);
    assert_eq!(messages(&src("", "", "", update)), denied);
}

/// must-not-compile 2513-testrequestpage-typed-on-xmlport.al: a `TestRequestPage` parameter typed on an xmlport (no report of
/// that name) is AL0185 `Report 'X' is missing`, and on a RequestPageHandler also AL0240.
#[test]
fn al0185_test_request_page_typed_on_an_xmlport() {
    let src = "xmlport 50000 \"MNC XP\" { schema { textelement(Root) { } } }
codeunit 50001 H { Subtype = Test;
    [RequestPageHandler]
    procedure XmlPortRequestPage(var RP: TestRequestPage \"MNC XP\") begin end; }";
    let other = "codeunit 2000000001 Other { }";
    assert_eq!(
            messages_with_platform(&[src], other),
            vec![
                "AL0240 The signature of procedure 'XmlPortRequestPage' does not match the signature required by attribute \
                 'RequestPageHandler': parameter 1 is expected to be of type 'var TestRequestPage' but found type 'var '. The \
                 expected signature is: XmlPortRequestPage(var RequestPage: TestRequestPage)."
                    .to_string(),
                "AL0185 Report 'MNC XP' is missing".to_string(),
            ]
        );
    // On a report of that name: no error.
    let report = "report 50002 \"MNC XP\" { }";
    assert!(codes_with_platform(&[src, report], other).is_empty());
}

/// must-not-compile 0213-unknown-codeunit-var.al:7: a variable of a codeunit type nobody declares is AL0185
/// `Codeunit 'X' is missing`. Judged only when every app the file sees is loaded, the platform app included.
#[test]
fn al0185_variable_of_an_unknown_codeunit_type() {
    let local = "codeunit 50000 C { procedure P() var X: Codeunit \"No Such\"; begin end; }";
    let global = "codeunit 50001 G { var X: Codeunit NoSuch; }";
    let other = "codeunit 2000000001 Other { }";
    assert_eq!(codes_with_platform(&[local], other), vec!["AL0185"]);
    assert_eq!(codes_with_platform(&[global], other), vec!["AL0185"]);
    assert_eq!(
        messages_with_platform(&[local], other),
        vec!["AL0185 Codeunit 'No Such' is missing".to_string()]
    );
    // Found in the app itself, in the platform app, or named by id: no error.
    let own = "codeunit 50002 \"No Such\" { }";
    assert!(codes_with_platform(&[local, own], other).is_empty());
    assert!(codes_with_platform(&[local], "codeunit 2000000002 \"No Such\" { }").is_empty());
    let by_id = "codeunit 50003 I { procedure P() var X: Codeunit 50002; begin end; }";
    assert!(codes_with_platform(&[by_id, own], other).is_empty());
    // Without the platform app, or with a dependency that is not loaded, the name may exist.
    assert!(codes(&[local]).is_empty());
    assert!(codes_with_dependency(&[local], None).is_empty());
    assert_eq!(codes_with_dependency(&[local], Some(other)), vec!["AL0185"]);
    assert!(codes_with_dependency(&[local], Some("codeunit 60000 \"No Such\" { }")).is_empty());
}

/// A real app states an `application` floor, so the runner gives it the `Application` aggregate as a dependency next
/// to the apps the aggregate propagates (`al_deps::load::Loaded::implied_dependencies`). The aggregate has no source
/// and is never loaded; it declares no objects, so it hides no name once Base Application, whose dependencies are
/// the rest of the aggregate, is loaded. Without Base Application the name stays undecided.
#[test]
fn al0185_is_judged_when_only_the_application_aggregate_is_not_loaded() {
    let local = "codeunit 50000 C { procedure P() var X: Codeunit \"No Such\"; begin end; }";
    let base = crate::workspace::BASE_APPLICATION_APP_ID;
    let aggregate = crate::workspace::APPLICATION_AGGREGATE_APP_ID;
    let run = |deps: Vec<&str>, with_base: bool| -> Vec<String> {
        let file = |s: &str| vec![("f.al".to_string(), s.to_string())];
        let mut apps = vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: file(local),
        }];
        let mut kinds = vec![crate::workspace::AppKind::Workspace];
        if with_base {
            apps.push(AppSource {
                id: base.into(),
                name: "Base Application".into(),
                dependencies: vec![],
                files: file("codeunit 80 \"Sales-Post\" { }"),
            });
            kinds.push(crate::workspace::AppKind::Dependency);
        }
        apps.push(AppSource {
            id: crate::workspace::PLATFORM_APP_ID.into(),
            name: "System".into(),
            dependencies: vec![],
            files: file("codeunit 2000000001 Other { }"),
        });
        kinds.push(crate::workspace::AppKind::Dependency);
        let ws = Workspace::load_apps(apps, kinds, &Default::default());
        let r = crate::check::check(&ws, &crate::members::build(&ws), &Catalog::load());
        r.diagnostics
            .iter()
            .filter(|d| d.code.starts_with("AL"))
            .map(|d| d.code.to_string())
            .collect()
    };
    assert_eq!(run(vec![aggregate, base], true), vec!["AL0185"]);
    // Base Application not loaded: the aggregate may hold the name.
    assert!(run(vec![aggregate], false).is_empty());
    // Another dependency that is not loaded still keeps the name.
    assert!(run(vec![aggregate, base, "other"], true).is_empty());
}

#[test]
fn al0118_codeunit_name_is_not_judged_when_the_platform_app_is_not_loaded() {
    // must-not-compile 0740-subscribe-internal-event-get-role-center-banner.al:5 subscribes to
    // Codeunit::"System Action Triggers", a codeunit of the platform app System. BC always provides that
    // app, so without its source the name may exist and is not refused.
    let user = "codeunit 50000 C { procedure P(): Integer begin exit(Codeunit::\"System Action Triggers\"); end; }";
    let sub = "codeunit 50001 S { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"System Action Triggers\", 'OnX', '', false, false)] local procedure H() begin end; }";
    assert!(codes(&[user, sub]).is_empty());
    // The platform app is loaded and has no such codeunit: the name does not exist.
    let other = "codeunit 2000000001 Other { }";
    assert_eq!(codes_with_platform(&[user], other), vec!["AL0118"]);
    assert_eq!(codes_with_platform(&[sub], other), vec!["AL0118"]);
    // And it resolves when the platform app has it.
    let sat = "codeunit 2000000002 \"System Action Triggers\" { }";
    assert!(codes_with_platform(&[user, sub], sat).is_empty());
}

#[test]
fn al0322_and_al0208_of_control_visible_expressions() {
    // must-not-compile 2630-2632 (corpus branch al-runtime-rs-staging-y65, al-runner issue 6675).
    let t = "table 50000 T { fields { field(1; Id; Code[10]) { } field(2; CueA; Integer) { } } }";
    let arr = "page 50000 P { PageType = CardPart; SourceTable = T; layout { area(Content) { cuegroup(Cues) { \
                   field(CueA; Rec.CueA) { Visible = ShowArr[1]; } } } } var ShowArr: array[2] of Boolean; }";
    assert_eq!(codes(&[t, arr]), vec!["AL0322"]);
    let method = "page 50000 P { PageType = CardPart; SourceTable = T; layout { area(Content) { cuegroup(Cues) { \
                      field(CueA; Rec.CueA) { Visible = not Rec.IsEmpty(); } } } } }";
    assert_eq!(codes(&[t, method]), vec!["AL0322"]);
    let int = "page 50000 P { PageType = CardPart; SourceTable = T; layout { area(Content) { cuegroup(Cues) { Visible = 5; \
                   field(CueA; Rec.CueA) { } } } } }";
    assert_eq!(codes(&[t, int]), vec!["AL0208"]);
    // A cuegroup takes an array element, a method call and a procedure call (pages/lt-y65-cue-visible/Y65CueVisible.al).
    let ok = "page 50000 P { PageType = CardPart; SourceTable = T; layout { area(Content) { \
                  cuegroup(A) { Visible = ShowArr[2]; field(CueA; Rec.CueA) { Visible = Rec.CueA <> 0; } } \
                  cuegroup(B) { Visible = not Rec.IsEmpty(); } cuegroup(C) { Visible = IsShown(); } } } \
                  var ShowArr: array[2] of Boolean; local procedure IsShown(): Boolean begin exit(true); end; }";
    assert!(codes(&[t, ok]).is_empty(), "{:?}", codes(&[t, ok]));
}

#[test]
fn al0322_a_field_control_property_bound_to_a_procedure_call() {
    // must-not-compile 2000: Visible, Enabled and Editable of a field control bound to a call of a page procedure.
    let t = "table 52000 T { fields { field(1; Id; Integer) { } field(2; A; Integer) { } } }";
    let page = "page 52000 P { PageType = Card; SourceTable = T; layout { area(Content) { field(Id; Rec.Id) { Visible = IsOk(); } \
                    field(A; Rec.A) { Enabled = IsOk(); Editable = IsOk(); } } } procedure IsOk(): Boolean begin exit(true); end; }";
    assert_eq!(codes(&[t, page]), vec!["AL0322", "AL0322", "AL0322"]);
    // On an action the same binding compiles (warning AL0573, pages/lt-qq-procedure-bound-action/QqBoundObjects.al).
    let action = "page 52001 Q { PageType = Card; actions { area(Processing) { action(X) { Visible = IsOk(); Enabled = IsOk(); } } } \
                      procedure IsOk(): Boolean begin exit(true); end; }";
    assert!(codes(&[action]).is_empty());
}

#[test]
fn al0118_bare_names_are_not_judged_when_a_dependency_is_not_loaded() {
    // A codeunit with `TableNo` of a table of the missing dependency has an implicit `Rec`; a page with a
    // source table of it assigns the table's fields by their bare names.
    let cu = "codeunit 50000 C { TableNo = \"Sales Header\"; trigger OnRun() begin Rec.Foo := 1; end; }";
    let page = "page 50001 P { PageType = Card; SourceTable = \"Sales Header\"; trigger OnOpenPage() begin Foo := 1; Bar(); end; }";
    assert!(codes_with_dependency(&[cu, page], None).is_empty());
    // With the dependency loaded (a table that has no such field) the page's names are judged.
    let t = "table 60000 \"Sales Header\" { fields { field(1; Id; Integer) { } } }";
    assert!(codes_with_dependency(&[page], Some(t)).contains(&"AL0118".to_string()));
    // A page extension of a base page of the missing dependency calls the base page's procedure.
    let ext = "pageextension 50002 PE extends \"Customer Card\" { trigger OnOpenPage() begin BaseProc(); Rec.Foo := 1; end; }";
    assert!(codes_with_dependency(&[ext], None).is_empty());
}

#[test]
fn al0118_bare_call_of_an_unknown_name_in_a_codeunit() {
    let cu = "codeunit 50000 C { procedure Make(): Guid begin exit(CreateSequentialGuid()); end; }";
    assert_eq!(codes(&[cu]), vec!["AL0118"]);
    let ok = "codeunit 50000 C { procedure Make(): Guid begin exit(CreateGuid()); end; procedure Other() begin Make(); end; }";
    assert!(codes(&[ok]).is_empty());
}

#[test]
fn al0118_bare_call_of_an_unknown_name_in_a_table_page_report_and_page_extension() {
    let t = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } }
            procedure Run() begin NoSuchProcedure(); end; }";
    assert_eq!(codes(&[t]), vec!["AL0118"]);
    let t = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
    let page = "page 50001 P { PageType = Card; SourceTable = T; trigger OnOpenPage() begin NoSuchProcedure(); end; }";
    assert_eq!(codes(&[t, page]), vec!["AL0118"]);
    let report = "report 50002 R { dataset { dataitem(Row; T) { column(Id; Id) { } } } trigger OnPreReport() begin NoSuchProcedure(); end; }";
    assert_eq!(codes(&[t, report]), vec!["AL0118"]);
    let ext = ext("trigger OnOpenPage() begin NoSuchProcedure(); end;");
    assert_eq!(codes(&[BASE, &ext]), vec!["AL0118"]);
    // What a table, page or report can call is not judged: an own procedure, an implicit record method,
    // a page method, a system function.
    let ok = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } }
            procedure Own() begin end;
            procedure Run() begin Own(); Init(); SetRange(Id, 1); Message('x'); end; }";
    assert!(codes(&[ok]).is_empty());
    let ok_page = "page 50001 P { PageType = Card; SourceTable = T; trigger OnOpenPage() begin SetRange(Id, 1); Update(false); end; }";
    assert!(codes(&[t, ok_page]).is_empty());
}

#[test]
fn al0118_is_not_given_for_the_codeunits_own_run() {
    // A bare `Run()` in a codeunit runs the codeunit itself (a method of the Codeunit type, called on the
    // running instance); real BC compiles it (NP Retail's scanner import codeunits do).
    let cu = "codeunit 50000 C { trigger OnRun() begin end;
            procedure Go() begin if not Run() then Message('failed'); end; }";
    assert!(codes(&[cu]).is_empty(), "{:?}", codes(&[cu]));
    // A name that is a method of no type is still refused.
    let bad = "codeunit 50000 C { procedure Go() begin NoSuchProcedure(); end; }";
    assert_eq!(codes(&[bad]), vec!["AL0118"]);
}

#[test]
fn al0118_is_not_given_for_a_procedure_of_a_sibling_page_extension() {
    // Microsoft's test library calls `RefreshFactboxes`, an `internal procedure` of another page extension
    // of the same base page (Tests-TestLibraries UsersPBTTest.PageExt.al, Base Application
    // UsersPerfFactboxes.PageExt.al); real BC compiles it.
    let sibling = "pageextension 50003 BPE2 extends BP { internal procedure Refresh(a: Integer) begin end; }";
    let caller = "pageextension 50002 BPE extends BP { trigger OnOpenPage() begin Refresh(1); end; }";
    assert!(
        codes(&[BASE, sibling, caller]).is_empty(),
        "{:?}",
        codes(&[BASE, sibling, caller])
    );
    // Without the sibling the name is unknown.
    assert_eq!(codes(&[BASE, caller]), vec!["AL0118"]);
}

#[test]
fn al0118_rec_in_a_codeunit_without_table_no() {
    let cu = "table 50000 T { fields { field(1; Id; Integer) { } } }
            codeunit 50001 C { procedure P() begin Rec.Id := 1; end; }";
    assert_eq!(codes(&[cu]), vec!["AL0118"]);
    let ok = "table 50000 T { fields { field(1; Id; Integer) { } } }
            codeunit 50001 C { TableNo = T; trigger OnRun() begin Rec.Id := 1; end; }";
    assert!(codes(&[ok]).is_empty());
}

#[test]
fn al0132_record_field_or_method_that_does_not_exist() {
    let t = "table 50000 T { fields { field(1; Id; Integer) { } } }";
    let body = |stmt: &str| format!("codeunit 50001 C {{ procedure P() var R: Record T; begin {stmt} end; }}");
    assert_eq!(codes(&[t, &body("R.NoSuchField := 1;")]), vec!["AL0132"]);
    assert_eq!(codes(&[t, &body("R.NoSuchMethod();")]), vec!["AL0132"]);
    assert!(codes(&[t, &body("R.Id := 1; R.Init(); R.SetRange(Id, 1);")]).is_empty());
    // A table that an extension adds to is not judged.
    let ext = "tableextension 50002 TE extends T { fields { field(50000; Extra; Integer) { } } }";
    assert!(codes(&[t, ext, &body("R.Extra := 1;")]).is_empty());
}

#[test]
fn al0161_in_a_report_extension() {
    let t = "table 50000 T { fields { field(1; Id; Integer) { } } }";
    let base = "report 50001 BR { dataset { dataitem(Row; T) { column(Id; Id) { } } } var Hidden: Integer;
            local procedure SecretLocal(): Integer begin exit(1); end; protected var Shown: Integer; }";
    let var = "reportextension 50002 E extends BR { trigger OnPreReport() begin Hidden := 1; end; }";
    assert_eq!(codes(&[t, base, var]), vec!["AL0161"]);
    let proc_ =
        "reportextension 50002 E extends BR { trigger OnPreReport() var V: Integer; begin V := SecretLocal(); end; }";
    assert_eq!(codes(&[t, base, proc_]), vec!["AL0161"]);
    let ok = "reportextension 50002 E extends BR { trigger OnPreReport() begin Shown := 1; end; }";
    assert!(codes(&[t, base, ok]).is_empty());
}

#[test]
fn al0132_execution_mode_members_that_do_not_exist() {
    for m in ["Install", "Uninstall", "Upgrade", "Report", "Profile"] {
        let cu = format!("codeunit 50000 C {{ procedure M(): ExecutionMode begin exit(ExecutionMode::{m}); end; }}");
        assert_eq!(codes(&[&cu]), vec!["AL0132"], "{m}");
    }
    let ok = "codeunit 50000 C { procedure M(): ExecutionMode begin exit(ExecutionMode::Standard); end; }";
    assert!(codes(&[ok]).is_empty());
}

fn enum_pair(src_compat: bool, dst_compat: bool) -> String {
    format!(
            "enum 50000 \"MNC Src\" {{ Extensible = true; AssignmentCompatibility = {src_compat}; value(0; None) {{ }} value(1; One) {{ }} value(7; Seven) {{ }} }}
             enum 50001 \"MNC Dst\" {{ Extensible = true; AssignmentCompatibility = {dst_compat}; value(0; None) {{ }} value(1; One) {{ }} value(2; Two) {{ }} }}
             codeunit 50002 \"MNC Assign\" {{ procedure Do(): Integer var S: Enum \"MNC Src\"; D: Enum \"MNC Dst\"; begin D := S; exit(D.AsInteger()); end; }}"
        )
}

/// The AL diagnostics of a compile of `src`, as `code message`.
fn messages(src: &str) -> Vec<String> {
    let ws = load(&[src]);
    let m = crate::members::build(&ws);
    crate::check::check(&ws, &m, &Catalog::load())
        .diagnostics
        .iter()
        .filter(|d| d.code.starts_with("AL"))
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

// must-not-compile 0074 (only the source states AssignmentCompatibility) and 0075 (neither does):
// AL0122. A target that states it compiles whatever the source says
// (types/PXEnumRecordRef.al:7-10, :74-108); so does an Integer into any enum (:11-12, :137-196).
#[test]
fn al0122_an_enum_of_another_type_into_a_target_without_assignment_compatibility() {
    let want = vec!["AL0122 Cannot implicitly convert type 'Enum \"MNC Src\"' to 'Enum \"MNC Dst\"'".to_string()];
    assert_eq!(messages(&enum_pair(true, false)), want, "0074");
    assert_eq!(messages(&enum_pair(false, false)), want, "0075");
    assert!(messages(&enum_pair(false, true)).is_empty());
    assert!(messages(&enum_pair(true, true)).is_empty());
    // The property left out (false by default), into a table field.
    let field = "enum 50000 \"MNC Src\" { value(0; None) { } value(1; One) { } }
             enum 50001 \"MNC Dst\" { value(0; None) { } value(1; One) { } }
             table 50003 \"MNC Row\" { fields { field(1; Id; Integer) { } field(2; Kind; Enum \"MNC Dst\") { } } }
             codeunit 50002 \"MNC Assign\" { procedure Do() var S: Enum \"MNC Src\"; R: Record \"MNC Row\"; begin R.Kind := S; end; }";
    assert_eq!(messages(field), want, "field");
    let same = "enum 50001 \"MNC Dst\" { value(0; None) { } value(1; One) { } }
             codeunit 50002 \"MNC Assign\" { procedure Do() var D: Enum \"MNC Dst\"; E: Enum \"MNC Dst\"; I: Integer; begin D := E; I := 1; D := I; end; }";
    assert!(messages(same).is_empty());
}

// must-not-compile 2291 (corpus PR 808 at branch commit 910da83e; al-runner issue 6215): a codeunit true branch and an
// interface false branch is AL0122; the other order compiles (types/lt-up-ternary/UpTernary.al:60, :70, :81).
#[test]
fn al0122_ternary_codeunit_true_branch_interface_false_branch() {
    let src = |ternary: &str| {
        format!(
                "interface \"MNC UP Preview\" {{ procedure GetPreview(): Text; }}
                 codeunit 50001 \"MNC UP Default\" implements \"MNC UP Preview\" {{ procedure GetPreview(): Text begin exit('d'); end; }}
                 codeunit 50002 \"MNC UP Factory\" {{
                     procedure Get(UseDefault: Boolean; Other: Interface \"MNC UP Preview\"): Interface \"MNC UP Preview\"
                     var Default: Codeunit \"MNC UP Default\";
                     begin exit({ternary}); end; }}"
            )
    };
    assert_eq!(
        messages(&src("UseDefault ? Default : Other")),
        vec![
            "AL0122 Cannot implicitly convert type 'Interface \"MNC UP Preview\"' to 'Codeunit \"MNC UP Default\"'"
                .to_string()
        ]
    );
    assert!(messages(&src("UseDefault ? Other : Default")).is_empty());
}

// must-not-compile 0069 (`T := I`, an Integer variable into a Text variable) and 0070 (`I := 'text'`, a
// text literal into an Integer variable): AL0122. A one-character literal is a Char, which an Integer takes;
// the other shapes (a Text variable into an Integer, an Integer into a Code) are not judged.
#[test]
fn al0122_integer_into_text_and_text_literal_into_integer() {
    let cu = |body: &str| {
        format!(
            "codeunit 50001 \"MNC Codeunit\" {{ procedure DoIt() var I: Integer; T: Text; L: Text[30]; C: Code[20]; begin {body} end; }}"
        )
    };
    assert_eq!(
        messages(&cu("T := I;")),
        vec!["AL0122 Cannot implicitly convert type 'Integer' to 'Text'".to_string()]
    );
    assert_eq!(
        messages(&cu("I := 'text';")),
        vec!["AL0122 Cannot implicitly convert type 'Text' to 'Integer'".to_string()]
    );
    assert_eq!(messages(&cu("L := I;")).len(), 1);
    for ok in [
        "I := 'A';",
        "T := Format(I);",
        "I := 5; T := 'x';",
        "C := 'x';",
        "I := T.IndexOf('x');",
    ] {
        assert!(messages(&cu(ok)).is_empty(), "{ok}");
    }
}

// must-not-compile 0500 (corpus tests/must-not-compile/cases/0500-char-minus-text.al:1-2): `C - T` and `T - C`
// with a Char variable and an unbounded Text variable. AL0175. A Text literal with a Char compiles (an
// Integer). `C - L` with a Text[30] is AL0175 too: the compile matrix (crates/al-sema/tests/fixtures/
// conversion-matrix/compile-matrix.csv, context sub, Char - Text10 and Text3) rejects a Char minus any Text.
#[test]
fn al0175_char_minus_text_variable() {
    let cu = |body: &str| {
        format!(
            "codeunit 50001 \"MNC Codeunit\" {{ procedure DoIt() var C: Char; T: Text; L: Text[30]; I: Integer; begin {body} end; }}"
        )
    };
    assert_eq!(
        messages(&cu("I := C - T;")),
        vec!["AL0175 Operator '-' cannot be applied to operands of type 'Char' and 'Text'".to_string()]
    );
    assert_eq!(
        messages(&cu("I := T - C;")),
        vec!["AL0175 Operator '-' cannot be applied to operands of type 'Text' and 'Char'".to_string()]
    );
    assert_eq!(
        messages(&cu("I := C - L;")),
        vec!["AL0175 Operator '-' cannot be applied to operands of type 'Char' and 'Text[30]'".to_string()]
    );
    for ok in [
        "I := C - 'a';",
        "I := 'a' - C;",
        "I := C - I;",
        "I := C - C;",
        "T := T + T;",
    ] {
        assert!(messages(&cu(ok)).iter().all(|m| !m.starts_with("AL0175")), "{ok}");
    }
}

// must-not-compile 1250: `I := T`, `C := I` into a Code[20] and `T := I + 1` are AL0122. `exit(1 + 2)` in a procedure
// returning Text is the third shape through exit (al-runner issue 5588, comment of 2026-10-24).
#[test]
fn al0122_integer_text_code_assignments() {
    let cu = |body: &str| {
        format!(
                "codeunit 50000 \"MNC BB Codeunit\" {{ procedure DoIt() var I: Integer; T: Text; C: Code[20]; begin {body} end;
                   procedure Name(): Text begin exit(1 + 2); end; }}"
            )
    };
    let only = |body: &str| -> Vec<String> {
        messages(&cu(body))
            .into_iter()
            .filter(|m| m.starts_with("AL0122"))
            .collect()
    };
    let exit_msg = "AL0122 Cannot implicitly convert type 'Integer' to 'Text'".to_string();
    assert_eq!(
        only("I := T;"),
        vec![
            "AL0122 Cannot implicitly convert type 'Text' to 'Integer'".to_string(),
            exit_msg.clone()
        ]
    );
    assert_eq!(
        only("C := I;"),
        vec![
            "AL0122 Cannot implicitly convert type 'Integer' to 'Code[20]'".to_string(),
            exit_msg.clone()
        ]
    );
    assert_eq!(only("T := I + 1;"), vec![exit_msg.clone(), exit_msg.clone()]);
    for ok in [
        "T := Format(I + 1);",
        "I := I + 1;",
        "C := Format(I);",
        "Evaluate(I, T);",
    ] {
        assert_eq!(only(ok), vec![exit_msg.clone()], "{ok}");
    }
}

// must-not-compile 1190 (Boolean into Text), 1191 (Boolean into Code[10]), 1193 (Date into DateTime), 1251 (a text
// literal of two characters into a BigInteger and a Decimal) and 1833 (BigText into Text): AL0122.
#[test]
fn al0122_more_scalar_pairs() {
    let cu = |body: &str| {
        format!(
            "codeunit 50000 \"MNC C\" {{ procedure DoIt() var B: Boolean; T: Text; C: Code[10]; D: Date; DT: DateTime;
                   BI: BigInteger; Dec: Decimal; BT: BigText; begin {body} end; }}"
        )
    };
    let only = |body: &str| -> Vec<String> {
        messages(&cu(body))
            .into_iter()
            .filter(|m| m.starts_with("AL0122"))
            .collect()
    };
    for (body, want) in [
        ("T := B;", vec!["Boolean' to 'Text'"]),
        ("C := B;", vec!["Boolean' to 'Code[10]'"]),
        ("DT := D;", vec!["Date' to 'DateTime'"]),
        (
            "BI := 'xy'; Dec := 'xy';",
            vec!["Text' to 'BigInteger'", "Text' to 'Decimal'"],
        ),
        ("T := BT;", vec!["BigText' to 'Text'"]),
    ] {
        let want: Vec<String> = want
            .iter()
            .map(|w| format!("AL0122 Cannot implicitly convert type '{w}"))
            .collect();
        assert_eq!(only(body), want, "{body}");
    }
    for ok in [
        "T := Format(B);",
        "DT := CreateDateTime(D, 0T);",
        "BI := 'x';",
        "BT.GetSubText(T, 1);",
        "B := true;",
    ] {
        assert!(only(ok).is_empty(), "{ok}");
    }
}

// must-not-compile 0501: a Text literal and a Text variable assigned to a Decimal. AL0122.
#[test]
fn al0122_text_into_decimal() {
    let cu = |body: &str| {
        format!(
            "codeunit 50001 \"MNC Codeunit\" {{ procedure DoIt() var D: Decimal; T: Text; V: Variant; begin {body} end; }}"
        )
    };
    let want = "AL0122 Cannot implicitly convert type 'Text' to 'Decimal'".to_string();
    assert_eq!(messages(&cu("D := '100';")), vec![want.clone()]);
    assert_eq!(messages(&cu("D := T;")), vec![want]);
    for ok in ["D := V;", "Evaluate(D, T);", "D := 5;"] {
        assert!(messages(&cu(ok)).is_empty(), "{ok}");
    }
}

// Writer TL's must-not-compile 2249-2251 (branch al-runtime-rs-staging-pub-tl, al-runner issue 6060): an Enum variable,
// an enum value and an Enum field into a Decimal are AL0122; an Option into a Decimal compiles
// (types/lt-implicit-conversions/P79OptionDecimal.al on the same branch).
#[test]
fn al0122_enum_into_decimal() {
    let src = |body: &str| {
        format!(
                "enum 52249 \"MNC E\" {{ value(0; A) {{ }} value(5; B) {{ }} }}
                 table 52254 \"MNC R\" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Kind; Enum \"MNC E\") {{ }}
                   field(3; Amount; Decimal) {{ }} field(4; Opt; Option) {{ OptionMembers = X,Y; }} }}
                   keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
                 codeunit 52250 \"MNC C\" {{ procedure DoIt() var K: Enum \"MNC E\"; D: Decimal; O: Option P,Q; S: Record \"MNC R\";
                   T: Record \"MNC R\" temporary; begin {body} end; }}"
            )
    };
    let want = "AL0122 Cannot implicitly convert type 'Enum \"MNC E\"' to 'Decimal'".to_string();
    for bad in ["D := K;", "D := \"MNC E\"::B;", "T.Amount := S.Kind;"] {
        assert_eq!(messages(&src(bad)), vec![want.clone()], "{bad}");
    }
    for ok in ["D := O;", "T.Amount := S.Opt;", "D := K.AsInteger();"] {
        assert!(messages(&src(ok)).is_empty(), "{ok}");
    }
}

// Writer TJ's must-not-compile 2248 (branch al-runtime-rs-staging-pub-tj): ReadState = NoLocks is AL0169; the
// values ReadUncommitted, ReadCommitted and ReadShared compile.
#[test]
fn al0169_query_read_state_value() {
    let src = |v: &str| {
        format!(
                "table 50000 \"MNC T\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}
                 query 50001 \"MNC Q\" {{ QueryType = Normal; ReadState = {v}; elements {{ dataitem(T; \"MNC T\") {{ column(Id; Id) {{ }} }} }} }}"
            )
    };
    assert_eq!(
        messages(&src("NoLocks")),
        vec!["AL0169 The option value 'NoLocks' is not valid. Check the enum definition for valid values.".to_string()]
    );
    for ok in ["ReadUncommitted", "ReadCommitted", "ReadShared"] {
        assert!(messages(&src(ok)).is_empty(), "{ok}");
    }
}

// must-not-compile 0036 (two codeunits) and 0296 (two tables): AL0264 for every declaration of an id one app
// declares twice for one object type (al-runner issue 5854).
#[test]
fn al0264_every_declaration_of_a_repeated_id_is_reported() {
    let want = |kind: &str, id: u32| {
        format!("AL0264 An application object of type '{kind}' with ID '{id}' is already declared by the extension")
    };
    assert_eq!(
        messages("codeunit 50000 \"MNC First\" { } codeunit 50000 \"MNC Second\" { }"),
        vec![want("Codeunit", 50000), want("Codeunit", 50000)]
    );
    let table = |name: &str| {
        format!(
            "table 50109 \"{name}\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}"
        )
    };
    assert_eq!(
        messages(&format!("{} {}", table("MNC T296A"), table("MNC T296B"))),
        vec![want("Table", 50109), want("Table", 50109)]
    );
    // One id for objects of two types is fine.
    assert!(codes(&["codeunit 50000 A { } table 50000 B { fields { field(1; Id; Integer) { } } }"]).is_empty());
}

// must-not-compile 2136-2146: AL0264 for every object type with an id, not only codeunits and tables (al-runner
// issue 5588). The type is spelled as alc spells it ('XmlPort', 'PermissionSetExtension').
#[test]
fn al0264_every_object_type_with_an_id() {
    let want = |kind: &str| {
        format!("AL0264 An application object of type '{kind}' with ID '50001' is already declared by the extension")
    };
    let base = "table 50000 \"MNC T\" { fields { field(1; Id; Integer) { } } }
                    page 50000 \"MNC P\" { SourceTable = \"MNC T\"; }
                    report 50000 \"MNC R\" { }
                    enum 50000 \"MNC E\" { value(0; A) { } }";
    for (kind, a, b) in [
        (
            "Page",
            "page 50001 \"MNC A\" { SourceTable = \"MNC T\"; }",
            "page 50001 \"MNC B\" { SourceTable = \"MNC T\"; }",
        ),
        ("Report", "report 50001 \"MNC A\" { }", "report 50001 \"MNC B\" { }"),
        (
            "Query",
            "query 50001 \"MNC A\" { elements { dataitem(T; \"MNC T\") { column(Id; Id) { } } } }",
            "query 50001 \"MNC B\" { elements { dataitem(T; \"MNC T\") { column(Id; Id) { } } } }",
        ),
        ("XmlPort", "xmlport 50001 \"MNC A\" { }", "xmlport 50001 \"MNC B\" { }"),
        (
            "Enum",
            "enum 50001 \"MNC A\" { value(0; A) { } }",
            "enum 50001 \"MNC B\" { value(0; A) { } }",
        ),
        (
            "TableExtension",
            "tableextension 50001 \"MNC A\" extends \"MNC T\" { }",
            "tableextension 50001 \"MNC B\" extends \"MNC T\" { }",
        ),
        (
            "PageExtension",
            "pageextension 50001 \"MNC A\" extends \"MNC P\" { }",
            "pageextension 50001 \"MNC B\" extends \"MNC P\" { }",
        ),
        (
            "EnumExtension",
            "enumextension 50001 \"MNC A\" extends \"MNC E\" { value(10; X) { } }",
            "enumextension 50001 \"MNC B\" extends \"MNC E\" { value(11; Y) { } }",
        ),
        (
            "ReportExtension",
            "reportextension 50001 \"MNC A\" extends \"MNC R\" { }",
            "reportextension 50001 \"MNC B\" extends \"MNC R\" { }",
        ),
        (
            "PermissionSet",
            "permissionset 50001 \"MNC A\" { }",
            "permissionset 50001 \"MNC B\" { }",
        ),
    ] {
        let got: Vec<String> = messages(&format!("{base} {a} {b}"))
            .into_iter()
            .filter(|m| m.starts_with("AL0264"))
            .collect();
        assert_eq!(got, vec![want(kind), want(kind)], "{kind}");
    }
}

// must-not-compile 2148 (a codeunit) and 2149 (a table): an object with the id of an object of the same type in a
// direct dependency is AL0264, reported once, on this app's declaration. Without the dependency it compiles.
#[test]
fn al0264_an_id_a_dependency_declares() {
    let dep = "codeunit 61100 \"Dep C\" { }
                   table 61101 \"Dep T\" { fields { field(1; Id; Integer) { } } }";
    assert_eq!(
        codes_with_dependency(&["codeunit 61100 \"MNC Same\" { }"], Some(dep)),
        vec!["AL0264"]
    );
    assert_eq!(
        codes_with_dependency(
            &["table 61101 \"MNC Same\" { fields { field(1; Id; Integer) { } } }"],
            Some(dep)
        ),
        vec!["AL0264"]
    );
    // Another type with the id is fine, and so is the same file without the dependency loaded.
    assert!(codes_with_dependency(&["codeunit 61101 \"MNC Other\" { }"], Some(dep)).is_empty());
    assert!(codes_with_dependency(&["codeunit 61100 \"MNC Same\" { }"], None).is_empty());
}

// must-not-compile 2120-2131: AL0305 for a name over 30 characters of a codeunit (31 and 36 characters), table,
// page, report, query, xmlport, table, page and report extension and profile; 20 for a permission set.
#[test]
fn al0305_every_pinned_object_type() {
    let name = "MNC SH Name Length Probe Object"; // 31 characters
    let want = format!("AL0305 The length of the application object identifier '{name}' cannot exceed 30 characters.");
    let base = "table 50000 \"MNC T\" { fields { field(1; Id; Integer) { } } }
                    page 50002 \"MNC P\" { SourceTable = \"MNC T\"; }
                    report 50002 \"MNC R\" { }";
    for obj in [
        format!("codeunit 50001 \"{name}\" {{ }}"),
        "codeunit 50001 \"MNC SH Name Length Probe Object Abcd\" { }".to_string(),
        format!("table 50001 \"{name}\" {{ fields {{ field(1; Id; Integer) {{ }} }} }}"),
        format!("page 50001 \"{name}\" {{ SourceTable = \"MNC T\"; }}"),
        format!("report 50001 \"{name}\" {{ }}"),
        format!("query 50001 \"{name}\" {{ elements {{ dataitem(T; \"MNC T\") {{ column(Id; Id) {{ }} }} }} }}"),
        format!("xmlport 50001 \"{name}\" {{ }}"),
        format!("tableextension 50001 \"{name}\" extends \"MNC T\" {{ }}"),
        format!("pageextension 50001 \"{name}\" extends \"MNC P\" {{ }}"),
        format!("reportextension 50001 \"{name}\" extends \"MNC R\" {{ }}"),
        format!("profile \"{name}\" {{ Caption = 'X'; RoleCenter = \"MNC P\"; }}"),
    ] {
        let got: Vec<String> = messages(&format!("{base} {obj}"))
            .into_iter()
            .filter(|m| m.starts_with("AL0305"))
            .collect();
        assert_eq!(got.len(), 1, "{obj}: {got:?}");
        assert!(got[0].starts_with(&want[..want.find('\'').unwrap()]), "{obj}: {got:?}");
    }
    assert_eq!(
            messages("permissionset 50001 \"MNC SH Name Length Pr\" { Assignable = true; }"),
            vec![
                "AL0305 The length of the application object identifier 'MNC SH Name Length Pr' cannot exceed 20 characters."
                    .to_string()
            ]
        );
    // 30 characters compile, a permission set of 20 compiles, and an enum has no such limit
    // (al-language/misc/lt-cv-odd-leads/CvEnumLongNames.al, 38 characters).
    for ok in [
        "table 50001 \"MNC SH Name Length Probe Objec\" { fields { field(1; Id; Integer) { } } }",
        "permissionset 50001 \"MNC SH Name Length P\" { Assignable = true; }",
        "enum 50001 \"MNC SH Enum Name That Is Rather Long Ok\" { value(0; A) { } }",
    ] {
        assert!(codes(&[ok]).iter().all(|c| c != "AL0305"), "{ok}");
    }
}

// al-runner issue 6445: the 20-character permission set limit holds only with `Assignable = true` (case 2131). Every
// other permission set has the limit of 30 (cases 2333, 2334; Y12PermSetNames.al compiles 21 to 30).
#[test]
fn al0305_permission_set_limit_only_when_assignable_true() {
    for ok in [
        "permissionset 50100 \"ABC-DEF Process Files\" { Access = Internal; Assignable = false; }",
        "permissionset 50100 \"ABC-DEF Process Files\" { Assignable = false; }",
        "permissionset 50100 \"ABC-DEF Process Files\" { Access = Internal; }",
        "permissionset 50100 \"ABC-DEF Process Files\" { }",
        "permissionset 50100 \"ABC-DEF Process Files Thirty O\" { Assignable = false; }",
    ] {
        assert!(codes(&[ok]).iter().all(|c| c != "AL0305"), "{ok}");
    }
    for bad in [
        "permissionset 50100 \"MNC Y12 Not Assignable 31 chars\" { Assignable = false; }",
        "permissionset 50100 \"MNC Y12 No Assignable Prop 31ch\" { }",
        "permissionset 50100 \"ABC-DEF Process Files\" { Assignable = true; }",
        "permissionset 50100 \"ABC-DEF Process Files\" { Access = Public; Assignable = true; }",
    ] {
        assert_eq!(codes(&[bad]), vec!["AL0305"], "{bad}");
    }
}

// must-not-compile 0055: AL0305 for an object name of 37 characters (al-runner issue 5845).
#[test]
fn al0305_an_object_name_too_long() {
    assert_eq!(
            messages("codeunit 50000 \"MNC Object Name That Is Much Too Long\" { }"),
            vec![
                "AL0305 The length of the application object identifier 'MNC Object Name That Is Much Too Long' cannot exceed 30 characters."
                    .to_string()
            ]
        );
    assert!(codes(&["codeunit 50000 \"Thirty Characters Name Is Fine\" { }"]).is_empty());
    // A test codeunit of 31 characters (must-not-compile 2115, al-runner issue 5845); 30 compiles
    // (codeunit/TestScopeIsolationContracts.al:1).
    assert_eq!(
        codes(&["codeunit 50000 \"MNC Test Codeunit Name Length31\" { Subtype = Test; }"]),
        vec!["AL0305"]
    );
    assert!(codes(&["codeunit 60170 \"Test Scope Isolation Contracts\" { Subtype = Test; }"]).is_empty());
}

// must-not-compile 0048 (a procedure) and 0348 (the OnRun trigger): AL0139 for exit(value) in a codeunit routine
// without a return value, also where nothing calls it (al-runner issue 5852).
#[test]
fn al0139_exit_with_a_value_without_a_return_value() {
    assert_eq!(
        messages("codeunit 50001 \"MNC Codeunit\" { procedure DoIt() begin exit(1); end; }"),
        vec!["AL0139 Since 'DoIt' doesn't have a return value, EXIT cannot be called with a value".to_string()]
    );
    assert_eq!(
        messages("codeunit 50166 \"MNC C348\" { trigger OnRun() begin exit(1); end; }"),
        vec!["AL0139 Since 'OnRun' doesn't have a return value, EXIT cannot be called with a value".to_string()]
    );
    // Inside a statement, as in the issue's `Bump`.
    assert_eq!(
        codes(&["codeunit 50001 C { procedure Bump() var I: Integer; begin if I = 0 then exit(5); end; }"]),
        vec!["AL0139"]
    );
    for ok in [
        "codeunit 50001 C { procedure DoIt() begin exit; end; }",
        "codeunit 50001 C { procedure DoIt(): Integer begin exit(1); end; }",
        "codeunit 50001 C { procedure DoIt() R: Integer begin exit(1); end; }",
    ] {
        assert!(codes(&[ok]).is_empty(), "{ok}");
    }
}

// must-not-compile 2150-2165: AL0139 in the triggers and procedures of tables, fields, pages, actions, reports, data
// items, xmlports, queries, table and page extensions and in an event subscriber (al-runner issue 5588).
#[test]
fn al0139_in_every_pinned_object_type() {
    let t = "table 50000 \"MNC T\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
    let p = "page 50001 \"MNC P\" { SourceTable = \"MNC T\"; }";
    for (name, src) in [
            ("OnInsert", "table 50009 \"MNC X\" { fields { field(1; Id; Integer) { } } trigger OnInsert() begin exit(1); end; }".to_string()),
            ("OnValidate", "table 50009 \"MNC X\" { fields { field(1; Id; Integer) { trigger OnValidate() begin exit(true); end; } } }".to_string()),
            ("DoIt", "table 50009 \"MNC X\" { fields { field(1; Id; Integer) { } } procedure DoIt() begin exit('x'); end; }".to_string()),
            ("OnOpenPage", "page 50009 \"MNC X\" { SourceTable = \"MNC T\"; trigger OnOpenPage() begin exit(1); end; }".to_string()),
            ("DoIt", "page 50009 \"MNC X\" { SourceTable = \"MNC T\"; local procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnAction", "page 50009 \"MNC X\" { SourceTable = \"MNC T\"; actions { area(Processing) { action(Go) { trigger OnAction() begin exit(1); end; } } } }".to_string()),
            ("OnPreReport", "report 50009 \"MNC X\" { ProcessingOnly = true; trigger OnPreReport() begin exit(1); end; }".to_string()),
            ("OnAfterGetRecord", "report 50009 \"MNC X\" { ProcessingOnly = true; dataset { dataitem(B; \"MNC T\") { trigger OnAfterGetRecord() begin exit(1); end; } } }".to_string()),
            ("DoIt", "report 50009 \"MNC X\" { ProcessingOnly = true; procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnPreXmlPort", "xmlport 50009 \"MNC X\" { schema { textelement(Root) { } } trigger OnPreXmlPort() begin exit(1); end; }".to_string()),
            ("DoIt", "xmlport 50009 \"MNC X\" { schema { textelement(Root) { } } procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnBeforeOpen", "query 50009 \"MNC X\" { elements { dataitem(B; \"MNC T\") { column(Id; Id) { } } } trigger OnBeforeOpen() begin exit(1); end; }".to_string()),
            ("DoIt", "query 50009 \"MNC X\" { elements { dataitem(B; \"MNC T\") { column(Id; Id) { } } } procedure DoIt() begin exit(1); end; }".to_string()),
            ("OnAfterInsert", "tableextension 50009 \"MNC X\" extends \"MNC T\" { trigger OnAfterInsert() begin exit(1); end; }".to_string()),
            ("DoIt", "pageextension 50009 \"MNC X\" extends \"MNC P\" { procedure DoIt() begin exit(1); end; }".to_string()),
        ] {
            let got: Vec<String> = messages(&format!("{t} {p} {src}"))
                .into_iter()
                .filter(|m| m.starts_with("AL0139"))
                .collect();
            assert_eq!(
                got,
                vec![format!("AL0139 Since '{name}' doesn't have a return value, EXIT cannot be called with a value")],
                "{src}"
            );
        }
    // A page trigger that returns a value keeps compiling.
    let ok = format!(
        "{t} page 50009 \"MNC X\" {{ SourceTable = \"MNC T\"; trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(true); end; }}"
    );
    assert!(codes(&[&ok]).iter().all(|c| c != "AL0139"));
}

// must-not-compile 2167-2188: AL0135 through a codeunit variable, a record variable, `this`, a bare call inside a
// table, and alc's rendering of every pinned parameter type in the signature (al-runner issue 5588).
#[test]
fn al0135_calls_on_variables_and_parameter_types() {
    let src = |lib: &str, call: &str| {
        format!(
                "table 50000 \"MNC SH Table\" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }}
                   procedure Take(A: Integer; B: Integer) begin end; }}
                 enum 50000 \"MNC SH Enum\" {{ value(0; None) {{ }} }}
                 interface \"MNC SH Iface\" {{ procedure X(); }}
                 page 50003 \"MNC SH Page\" {{ SourceTable = \"MNC SH Table\"; }}
                 codeunit 50001 \"MNC SH Lib\" {{ {lib} }}
                 codeunit 50002 \"MNC SH Caller\" {{ procedure Run1() var Lib: Codeunit \"MNC SH Lib\"; R: Record \"MNC SH Table\"; begin {call} end; }}"
            )
    };
    let al0135 = |src: &str| -> Vec<String> { messages(src).into_iter().filter(|m| m.starts_with("AL0135")).collect() };
    let want = |p: &str, sig: &str| {
        format!("AL0135 There is no argument given that corresponds to the required formal parameter '{p}' of '{sig}'")
    };
    assert_eq!(
        al0135(&src(
            "procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end;",
            "Lib.Add(1);"
        )),
        vec![want("B", "Add(Integer, Integer)")]
    );
    assert_eq!(
        al0135(&src("", "R.Take(1);")),
        vec![want("B", "Take(Integer, Integer)")]
    );
    assert_eq!(
        al0135(&src("procedure Sig(A: Integer) begin end;", "Lib.Sig();")),
        vec![want("A", "Sig(Integer)")]
    );
    for (decl, sig) in [
        ("R: Record \"MNC SH Table\"", "Record \"MNC SH Table\""),
        ("var R: Record \"MNC SH Table\"", "var Record \"MNC SH Table\""),
        (
            "var R: Record \"MNC SH Table\" temporary",
            "var Record \"MNC SH Table\" temporary",
        ),
        ("R: Code[20]", "Code[20]"),
        ("R: Option Alpha,Beta", "Option"),
        ("R: Enum \"MNC SH Enum\"", "Enum \"MNC SH Enum\""),
        ("R: Decimal", "Decimal"),
        ("R: Date", "Date"),
        ("var R: Integer", "var Integer"),
        ("R: Boolean", "Boolean"),
        ("R: Time", "Time"),
        ("R: DateTime", "DateTime"),
        ("R: Guid", "Guid"),
        ("R: BigInteger", "BigInteger"),
        ("R: Char", "Char"),
        ("R: Text[30]", "Text[30]"),
        ("R: Duration", "Duration"),
        ("R: DateFormula", "DateFormula"),
        ("R: Variant", "Variant"),
        ("R: RecordRef", "RecordRef"),
        ("R: FieldRef", "FieldRef"),
        ("R: List of [Text]", "List of [Text]"),
        ("R: Dictionary of [Integer, Text]", "Dictionary of [Integer, Text]"),
        ("R: Codeunit \"MNC SH Lib\"", "Codeunit \"MNC SH Lib\""),
        ("R: Page \"MNC SH Page\"", "Page \"MNC SH Page\""),
        ("R: JsonObject", "JsonObject"),
        ("R: array[3] of Integer", "array[3] of Integer"),
        ("R: SecretText", "SecretText"),
        ("R: Interface \"MNC SH Iface\"", "Interface \"MNC SH Iface\""),
        ("R: InStream", "InStream"),
        ("R: ErrorInfo", "ErrorInfo"),
    ] {
        assert_eq!(
            al0135(&src(
                &format!("procedure Sig(X: Integer; {decl}) begin end;"),
                "Lib.Sig(1);"
            )),
            vec![want("R", &format!("Sig(Integer, {sig})"))],
            "{decl}"
        );
    }
    // A bare call and a `this.` call inside the table itself (2169, 2170).
    let table = |call: &str| {
        format!(
            "table 50000 \"MNC T\" {{ fields {{ field(1; Id; Integer) {{ trigger OnValidate() begin {call} end; }} }}
                   procedure Take(A: Integer; B: Integer) begin end; procedure Run1() begin {call} end; }}"
        )
    };
    assert_eq!(al0135(&table("Take(1);")), vec![want("B", "Take(Integer, Integer)"); 2]);
    assert_eq!(
        al0135(&table("this.Take(1);")),
        vec![want("B", "Take(Integer, Integer)"); 2]
    );
    // Enough arguments compile.
    assert!(
        al0135(&src(
            "procedure Add(A: Integer; B: Integer) begin end;",
            "Lib.Add(1, 2); R.Take(1, 2);"
        ))
        .is_empty()
    );
}

// must-not-compile 0242 and 0066: AL0135 for a call of an own procedure with too few arguments, also where
// nothing calls the caller (al-runner issue 5853).
#[test]
fn al0135_own_procedure_called_with_too_few_arguments() {
    assert_eq!(
            messages(
                "codeunit 50048 \"MNC C242\" { local procedure Take(A: Integer; B: Integer) begin end; procedure Run1() begin Take(1); end; }"
            ),
            vec![
                "AL0135 There is no argument given that corresponds to the required formal parameter 'B' of 'Take(Integer, Integer)'"
                    .to_string()
            ]
        );
    // The issue's shape: the call is an argument of exit, the procedure returns a value.
    assert_eq!(
            messages(
                "codeunit 50600 L { procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end; procedure Twice(A: Integer): Integer begin exit(Add(A)); end; }"
            ),
            vec![
                "AL0135 There is no argument given that corresponds to the required formal parameter 'B' of 'Add(Integer, Integer)'"
                    .to_string()
            ]
        );
    assert!(
            codes(&["codeunit 50048 C { local procedure Take(A: Integer; B: Integer) begin end; procedure Run1() begin Take(1, 2); end; }"])
                .is_empty()
        );
}

// must-not-compile 2369 (corpus writer Y29, al-runner issue 6526): `X.M := v` is checked as the call `X.M(v)`, so a
// builtin whose only form takes two arguments is AL0135; the one-argument setter compiles.
#[test]
fn al0135_assignment_form_of_a_two_parameter_builtin() {
    assert_eq!(
            messages(
                "codeunit 50000 \"MNC C2369\" { procedure Run1() var M: XmlNamespaceManager; begin M.AddNamespace := 'urn:x'; end; }"
            ),
            vec![
                "AL0135 There is no argument given that corresponds to the required formal parameter 'Prefix' of 'AddNamespace(Text, Text)'"
                    .to_string()
            ]
        );
    assert!(codes(&[
            "codeunit 50000 C { procedure Run1() var M: XmlNamespaceManager; D: XmlDocument; begin M.NameTable := D.NameTable; end; }"
        ])
        .is_empty());
}

// must-not-compile 2359 (corpus writer Y20, al-runner issue 6488): an alias the app declares outside a namespace, with the
// name of one a dependency declares in namespace System, is ambiguous when named bare. `DotNet System.String` and the
// dependency's alias alone compile (al-language-onprem-dotnet/lt-y20-dotnet-fullname/Y20DotNetFullTypeName.al:27-58).
#[test]
fn al0275_bare_dotnet_alias_declared_by_the_app_and_in_namespace_system() {
    const SYSTEM: &str = "namespace System;\ndotnet { assembly(mscorlib) { type(\"System.String\"; String) { } type(\"System.DateTime\"; DateTime) { } } }";
    const OWN: &str = "dotnet { assembly(mscorlib) { type(\"System.Text.StringBuilder\"; String) { } } }";
    let cu = |ty: &str| format!("codeunit 52359 C {{ procedure P() var B: DotNet {ty}; begin end; }}");
    assert_eq!(
        codes_with_dependency(&[OWN, &cu("String")], Some(SYSTEM)),
        vec!["AL0275"]
    );
    assert_eq!(
        messages_with_dependency(&[OWN, &cu("String")], SYSTEM),
        vec!["AL0275 'String' is an ambiguous reference between 'String' defined by the extension".to_string()]
    );
    assert!(codes_with_dependency(&[OWN, &cu("System.String")], Some(SYSTEM)).is_empty());
    assert!(codes_with_dependency(&[OWN, &cu("DateTime")], Some(SYSTEM)).is_empty());
    assert!(codes_with_dependency(&[&cu("String")], Some(SYSTEM)).is_empty());
    // The dependency not loaded: nothing is known about its aliases.
    assert!(codes_with_dependency(&[OWN, &cu("String")], None).is_empty());
}

// must-not-compile 2365, 2366, 2370, 2371 (corpus writer Y29, al-runner issue 6526): `X.M := v` where the builtin M has
// no form that takes the value (no parameter, or only optional ones and more than one) is AL0129.
#[test]
fn al0129_assignment_form_of_a_builtin_without_a_one_argument_form() {
    const AL0129: &str = "AL0129 The left-hand side of an assignment must be a variable or field";
    for (vars, stmt) in [
        ("D: XmlDocument; N: XmlNameTable;", "D.NameTable := N;"),
        ("E: XmlElement;", "E.InnerText := 'x';"),
        ("R: Record \"MNC Y29 Row\";", "R.LockTable := true;"),
        ("M: XmlNamespaceManager;", "M.PushScope := true;"),
    ] {
        let src = format!(
            "codeunit 50000 C {{ procedure Run1() var {vars} begin {stmt} end; }} table 50001 \"MNC Y29 Row\" {{ fields {{ field(1; Id; Integer) {{ }} }} }}"
        );
        assert_eq!(messages(&src), vec![AL0129.to_string()], "{stmt}");
    }
}

// must-not-compile 2367 (corpus writer Y29, al-runner issue 6526): a compound assignment to a builtin method is AL0129,
// also where the plain assignment form compiles.
#[test]
fn al0129_compound_assignment_to_a_builtin_method() {
    assert_eq!(
        messages("codeunit 50000 C { procedure Run1() var C: HttpClient; begin C.Timeout += 1; end; }"),
        vec!["AL0129 The left-hand side of an assignment must be a variable or field".to_string()]
    );
}

// must-not-compile 2368 (corpus writer Y29, al-runner issue 6526): two one-argument overloads, so the assignment form is
// checked as a call without arguments. alc's text goes on with the candidates; the first sentence is reported.
#[test]
fn al0126_assignment_form_of_a_builtin_with_two_one_argument_overloads() {
    assert_eq!(
        messages("codeunit 50000 C { procedure Run1() var J: JsonObject; begin J.ReadFrom := '{}'; end; }"),
        vec!["AL0126 No overload for method 'ReadFrom' takes 0 arguments.".to_string()]
    );
}

// must-not-compile 2372 (corpus writer Y29, al-runner issue 6526): the value is the argument of the one-parameter form.
#[test]
fn al0133_assignment_form_with_a_value_of_another_xml_type() {
    assert_eq!(
        messages(
            "codeunit 50000 C { procedure Run1() var D: XmlDocument; begin D.SetDeclaration := D.GetChildNodes(); end; }"
        ),
        vec!["AL0133 Argument 1: cannot convert from 'XmlNodeList' to 'XmlDeclaration'".to_string()]
    );
}

// The assignment forms real BC compiles: record/ProbePropertyAssignmentForms.al:375-660 (corpus writer Y29) and
// xml/XmlXPathNamespaces.al (`Mgr.NameTable := NT`).
#[test]
fn assignment_forms_bc_compiles_report_nothing() {
    let src = "codeunit 50000 C {
            procedure Run1()
            var
                C: HttpClient; R: HttpRequestMessage; Ct: HttpContent; A: XmlAttribute; Decl: XmlDeclaration;
                E: XmlElement; WO: XmlWriteOptions; B: TextBuilder; N: Notification; G: Guid; RR: RecordRef;
                F: FilterPageBuilder; S: SessionSettings; L: List of [Integer]; Dc: Dictionary of [Integer, Text];
                Txt: Text; Dlg: Dialog; Doc: XmlDocument; Row: Record \"MNC Y29 Row\"; M: XmlNamespaceManager; I: Integer;
            begin
                C.Timeout := 5000; C.Timeout := C.Timeout; C.SetBaseAddress := 'https://example.com/';
                C.UseResponseCookies := true; R.Method := 'POST'; R.SetRequestUri := 'https://example.com/';
                R.Content := Ct; A.Value := '2'; Decl.Version := '1.0'; E.Add := 'txt'; WO.PreserveWhitespace := true;
                B.Append := 'abc'; B.Length := 2; B.Capacity := 200; N.Message := 'm'; N.Id := G;
                N.Scope := NotificationScope::LocalScope; RR.CurrentKeyIndex := 2; F.PageCaption := 'cap';
                S.LanguageId := 1031; L.Add := 5; Dc.Remove := 1; Txt.Contains := 'b'; Dlg.HideSubsequentDialogs := true;
                Doc.GetRoot := E; Row.Get := 2; Row.ReadIsolation := IsolationLevel::ReadCommitted;
                Row.SecurityFiltering := SecurityFilter::Filtered; M.NameTable := Doc.NameTable; Doc.SetDeclaration := Decl;
                Row.Id += 1; I += 1;
            end;
        }
        table 50001 \"MNC Y29 Row\" { fields { field(1; Id; Integer) { } } }";
    assert_eq!(messages(src), Vec::<String>::new());
}

#[test]
fn al0197_every_declaration_of_a_repeated_name_is_reported() {
    let cu = "codeunit 50000 \"Same\" { } codeunit 50001 \"same\" { }";
    assert_eq!(codes(&[cu]), vec!["AL0197", "AL0197"]);
    // Another object type with the name is fine.
    let ok = "codeunit 50000 \"Same\" { } table 50001 \"Same\" { fields { field(1; Id; Integer) { } } }";
    assert!(codes(&[ok]).is_empty());
}

const PARENT: &str = "table 50000 P { fields { field(1; Id; Integer) { } field(2; Pid; Integer) { } } }";

#[test]
fn al0365_crossjoin_with_a_data_item_link() {
    let q = |props: &str| {
        format!(
            "query 50002 Q {{ elements {{ dataitem(Parent; P) {{ column(Id; Id) {{ }}
                   dataitem(Child; P) {{ {props} column(CId; Id) {{ }} }} }} }} }}"
        )
    };
    assert_eq!(
        codes(&[PARENT, &q("SqlJoinType = CrossJoin; DataItemLink = Pid = Parent.Id;")]),
        vec!["AL0365"]
    );
    assert!(codes(&[PARENT, &q("SqlJoinType = InnerJoin; DataItemLink = Pid = Parent.Id;")]).is_empty());
    assert!(codes(&[PARENT, &q("SqlJoinType = CrossJoin;")]).is_empty());
}

// must-not-compile 2010: a global variable is no named field; a field written with a data item name compiles
// (errors/lt-ms-refusals-138/P138ColCaptionQualified.al, QS_*).
#[test]
fn al0385_include_caption_on_a_global_variable_column() {
    let r =
        |col: &str, vars: &str| format!("report 50001 R {{ dataset {{ dataitem(Row; P) {{ {col} }} }} var {vars} }}");
    assert_eq!(
        codes(&[
            PARENT,
            &r("column(C; GCounter) { IncludeCaption = true; }", "GCounter: Integer;")
        ]),
        vec!["AL0385"]
    );
    assert!(codes(&[PARENT, &r("column(C; GCounter) { }", "GCounter: Integer;")]).is_empty());
    // A field of a data item's table with the name of a global is not judged.
    assert!(codes(&[PARENT, &r("column(C; Pid) { IncludeCaption = true; }", "Pid: Integer;")]).is_empty());
    assert!(
        codes(&[
            PARENT,
            &r("column(C; Row.Id) { IncludeCaption = true; }", "GCounter: Integer;")
        ])
        .is_empty()
    );
}

#[test]
fn al0385_include_caption_on_an_expression_column() {
    let r = |col: &str| format!("report 50001 R {{ dataset {{ dataitem(Row; P) {{ {col} }} }} }}");
    assert_eq!(
        codes(&[PARENT, &r("column(IdText; Format(Id)) { IncludeCaption = true; }")]),
        vec!["AL0385"]
    );
    assert!(codes(&[PARENT, &r("column(IdText; Id) { IncludeCaption = true; }")]).is_empty());
    // An indexed variable's field is a named field (Base Application's "Quantity Explosion of BOM").
    let indexed =
        "report 50001 R { dataset { dataitem(Row; P) { column(IdText; Arr[Level].Id) { IncludeCaption = true; } } }
            var Arr: array[3] of Record P; Level: Integer; }";
    assert!(codes(&[PARENT, indexed]).is_empty(), "{:?}", codes(&[PARENT, indexed]));
    assert!(codes(&[PARENT, &r("column(IdText; Format(Id)) { }")]).is_empty());
}

fn ctx(target: AppTarget, runtime: Option<(u32, u32)>, keys: &[&str]) -> AppContext {
    AppContext {
        target,
        runtime,
        app_json_keys: keys.iter().map(|k| k.to_string()).collect(),
        bc_major: None,
    }
}

#[test]
fn al0296_al0161_and_al0433_for_the_platform_tables_from_bc_27() {
    let cu = |t: &str| {
        format!("codeunit 50000 C {{ procedure P(): Integer var R: Record \"{t}\"; begin exit(R.Count()); end; }}")
    };
    let at = |major, t: &str| {
        let mut c = ctx(AppTarget::Cloud, None, &[]);
        c.bc_major = Some(major);
        let mut v = ctx_codes(&cu(t), &c);
        v.sort();
        v
    };
    assert_eq!(at(27, "Designed Query"), vec!["AL0296"]);
    assert_eq!(at(27, "Agent Data"), vec!["AL0161", "AL0296"]);
    assert_eq!(at(27, "Profile"), vec!["AL0296", "AL0433"]);
    // BC 28 and 29 give the same verdicts (must-not-compile 1240, `@from 28`); a table that is not on the list is
    // not judged, and no major is not judged either.
    assert_eq!(at(28, "Designed Query"), vec!["AL0296"]);
    assert_eq!(at(29, "Agent Data"), vec!["AL0161", "AL0296"]);
    assert_eq!(at(29, "Profile"), vec!["AL0296", "AL0433"]);
    assert!(at(27, "Company").is_empty());
    let mut none = ctx(AppTarget::Cloud, None, &[]);
    none.bc_major = None;
    assert!(ctx_codes(&cu("Designed Query"), &none).is_empty());
    // Cases 0006 and 0014 hold for every major.
    assert_eq!(at(28, "Published Application"), vec!["AL0296"]);
}

#[test]
fn al0296_cloud_app_names_an_onprem_platform_table() {
    let cu = "codeunit 50000 C { procedure P(): Integer var R: Record \"Published Application\"; begin exit(R.Count()); end; }";
    assert_eq!(ctx_codes(cu, &ctx(AppTarget::Cloud, None, &[])), vec!["AL0296"]);
    assert!(ctx_codes(cu, &ctx(AppTarget::OnPrem, None, &[])).is_empty());
    let other = "codeunit 50000 C { procedure P(): Integer var R: Record \"Company\"; begin exit(R.Count()); end; }";
    assert!(ctx_codes(other, &ctx(AppTarget::Cloud, None, &[])).is_empty());
}

#[test]
fn al0124_assembly_probing_paths_at_runtime_16() {
    let cu = "codeunit 50000 C { }";
    let keys = ["assemblyProbingPaths"];
    assert_eq!(
        ctx_codes(cu, &ctx(AppTarget::Cloud, Some((16, 0)), &keys)),
        vec!["AL0124"]
    );
    for r in [(15, 0), (16, 1), (17, 0), (17, 1), (18, 0)] {
        assert_eq!(
            ctx_codes(cu, &ctx(AppTarget::Cloud, Some(r), &keys)),
            vec!["AL0124"],
            "{r:?}"
        );
    }
    // Another runtime, another target and no key are not judged.
    assert!(ctx_codes(cu, &ctx(AppTarget::Cloud, Some((19, 0)), &keys)).is_empty());
    assert!(ctx_codes(cu, &ctx(AppTarget::OnPrem, Some((16, 0)), &keys)).is_empty());
    assert!(ctx_codes(cu, &ctx(AppTarget::Cloud, Some((16, 0)), &[])).is_empty());
    assert!(ctx_codes(cu, &ctx(AppTarget::Cloud, None, &keys)).is_empty());
}

// must-not-compile 2359 needs System Application's aliases, and the must-not-compile runner loads no dependency.
#[test]
fn al0275_is_not_checked_without_the_dependency() {
    assert!(not_checked_reason_for("AL0275", "'String' is an ambiguous reference").is_some());
}

#[test]
fn every_implemented_code_is_listed_once() {
    let mut v: Vec<_> = IMPLEMENTED.iter().map(|(c, _)| *c).collect();
    v.sort();
    v.dedup();
    assert_eq!(v.len(), IMPLEMENTED.len());
    assert_eq!(not_checked_reason("AL0161"), None);
    assert_eq!(not_checked_reason("AL9999"), Some("not implemented"));
}

fn cloud() -> AppContext {
    AppContext {
        target: AppTarget::Cloud,
        runtime: Some((16, 0)),
        app_json_keys: Vec::new(),
        bc_major: Some(27),
    }
}

#[test]
fn al0296_a_cloud_app_calls_a_method_with_scope_onprem() {
    let last = "codeunit 50000 C { procedure P() var I: ErrorInfo; begin I := GetLastErrorObject(); end; }";
    assert_eq!(ctx_codes(last, &cloud()), vec!["AL0296"]);
    // A procedure of the app with that name is not the platform's.
    let own = "codeunit 50000 C { procedure GetLastErrorObject(): Integer begin end; procedure P() begin GetLastErrorObject(); end; }";
    assert!(ctx_codes(own, &cloud()).is_empty());
    let unwrap = "codeunit 50000 C { procedure P(S: SecretText): Text begin exit(S.Unwrap()); end; }";
    assert_eq!(ctx_codes(unwrap, &cloud()), vec!["AL0296"]);
    let file = "codeunit 50000 C { procedure P(): Boolean var F: File; begin exit(F.Open('x')); end; }";
    assert_eq!(ctx_codes(file, &cloud()), vec!["AL0296"]);
    // Another type's Unwrap, and an OnPrem app, are fine.
    let other = "codeunit 50000 C { procedure P(S: Text): Text begin exit(S.Unwrap()); end; }";
    assert!(ctx_codes(other, &cloud()).is_empty());
    let onprem = AppContext {
        target: AppTarget::OnPrem,
        ..cloud()
    };
    assert!(ctx_codes(file, &onprem).is_empty());
}

/// The AL0296 messages of [`check_resolved_scope`] for `src` in an app with `ctx`.
fn scope_msgs(src: &str, ctx: &AppContext) -> Vec<String> {
    let ws = load(&[src]);
    let r = crate::check::check(&ws, &crate::members::build(&ws), &Catalog::load());
    check_resolved_scope(&ws, 0, ctx, &r)
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

fn onprem_msg(name: &str, scope: &str) -> String {
    format!(
        "AL0296 The application object or method '{name}' has scope '{scope}' and cannot be used for 'Cloud' development."
    )
}

// Must-not-compile 0563 to 0575, 0643 to 0646, 0651 to 0653, 0660, 0661, 1310 to 1312.
#[test]
fn al0296_a_cloud_app_calls_a_builtin_overload_with_scope_onprem() {
    let cu = |body: &str| format!("report 50001 R {{ }} codeunit 50000 C {{ procedure P() {body} }}");
    let one = |body: &str, name: &str| {
        assert_eq!(
            scope_msgs(&cu(body), &cloud()),
            vec![onprem_msg(name, "OnPrem")],
            "{body}"
        );
        let onprem = AppContext {
            target: AppTarget::OnPrem,
            ..cloud()
        };
        assert!(scope_msgs(&cu(body), &onprem).is_empty(), "{body}");
    };
    one(
        "var U: Integer; begin U := SessionInformation.AITokensUsed(); end;",
        "AITokensUsed",
    );
    one(
        "var R: RecordRef; K: KeyRef; begin K := R.KeyIndex(1); Database.AlterKey(K, true); end;",
        "AlterKey",
    );
    one(
        "var B: Boolean; begin B := Database.ChangeUserPassword('a', 'b'); end;",
        "ChangeUserPassword",
    );
    one(
        "begin Database.SetUserPassword(CreateGuid(), 'x'); end;",
        "SetUserPassword",
    );
    one("var F: Text; begin Database.ExportData(false, F); end;", "ExportData");
    one("var F: Text; begin Database.ImportData(false, F); end;", "ImportData");
    one("begin ExportEncryptionKey('x'); end;", "ExportEncryptionKey");
    one("begin ImportEncryptionKey('f', 'p'); end;", "ImportEncryptionKey");
    one(
        "var D: InStream; U: Text; begin U := ImportStreamWithUrlAccess(D, 'f', 5); end;",
        "ImportStreamWithUrlAccess",
    );
    one(
        "var B: Boolean; begin B := File.IsPathTemporary('x'); end;",
        "IsPathTemporary",
    );
    one(
        "var T: Text; B: Boolean; begin B := File.Download('a', 'b', '', '', T); end;",
        "Download",
    );
    one(
        "var C: HttpClient; B: Boolean; begin B := C.UseDefaultNetworkWindowsAuthentication(); end;",
        "UseDefaultNetworkWindowsAuthentication",
    );
    one(
        "var C: HttpClient; B: Boolean; begin B := C.UseWindowsAuthentication('u', 'p', 'd'); end;",
        "UseWindowsAuthentication",
    );
    one(
        "var C: HttpClient; S: SecretText; B: Boolean; begin B := C.UseWindowsAuthentication(S, S, S); end;",
        "UseWindowsAuthentication",
    );
    one(
        "var R: Report \"R\"; B: Boolean; begin B := R.SaveAsPdf('f'); end;",
        "SaveAsPdf",
    );
    one("var B: Boolean; begin B := Report.SaveAsXml(1, 'f'); end;", "SaveAsXml");
    // Another method of the same types compiles in a Cloud app.
    let none = |body: &str| assert!(scope_msgs(&cu(body), &cloud()).is_empty(), "{body}");
    none("var C: HttpClient; begin C.Clear(); end;");
    none("var I: Integer; begin I := SessionInformation.SqlStatementsExecuted(); end;");
}

// Must-not-compile 0981 (Scope('OnPrem')) and 0982 (Scope('Internal')): a procedure of the Cloud app with that scope,
// called from the app.
#[test]
fn al0296_a_cloud_app_calls_its_own_procedure_with_scope_onprem_or_internal() {
    let src = |scope: &str| {
        format!(
            "codeunit 50000 C {{ [Scope('{scope}')] procedure Scoped() begin end; procedure R() begin Scoped(); end; }}"
        )
    };
    assert_eq!(
        scope_msgs(&src("OnPrem"), &cloud()),
        vec![onprem_msg("Scoped", "OnPrem")]
    );
    assert_eq!(
        scope_msgs(&src("Internal"), &cloud()),
        vec![onprem_msg("Scoped", "Internal")]
    );
    assert!(scope_msgs(&src("Cloud"), &cloud()).is_empty());
    let onprem = AppContext {
        target: AppTarget::OnPrem,
        ..cloud()
    };
    assert!(scope_msgs(&src("OnPrem"), &onprem).is_empty());
    // Declared without a call: nothing.
    let uncalled = "codeunit 50000 C { [Scope('OnPrem')] procedure Scoped() begin end; }";
    assert!(scope_msgs(uncalled, &cloud()).is_empty());
}

// Must-not-compile 0983: the HttpClientHandler attribute in a Cloud app.
#[test]
fn al0296_the_httpclienthandler_attribute_in_a_cloud_app() {
    let src = "codeunit 50000 C { Subtype = Test; [HttpClientHandler] procedure H(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean begin end; }";
    assert_eq!(
        scope_msgs(src, &cloud()),
        vec![onprem_msg("HttpClientHandler", "OnPrem")]
    );
    let onprem = AppContext {
        target: AppTarget::OnPrem,
        ..cloud()
    };
    assert!(scope_msgs(src, &onprem).is_empty());
}

// Must-not-compile 1712 (tabledata) and 1713 (table): a permission set of a Cloud app names a Scope = OnPrem table.
#[test]
fn al0296_a_cloud_permission_set_names_an_onprem_table() {
    for entry in ["tabledata \"User Property\" = R", "table \"User Property\" = X"] {
        let src = format!("permissionset 50000 \"P\" {{ Assignable = true; Permissions = {entry}; }}");
        for major in [27, 28, 29] {
            let mut c = cloud();
            c.bc_major = Some(major);
            let got: Vec<String> = ctx_codes(&src, &c);
            assert_eq!(got, vec!["AL0296"], "{entry} {major}");
        }
    }
    let fine = "permissionset 50000 \"P\" { Assignable = true; Permissions = tabledata Company = R, page * = X; }";
    assert!(ctx_codes(fine, &cloud()).is_empty());
}

// TestHandlers is a codeunit property: BC 29 compiles a test codeunit that names one (corpus
// al-language-testlibraries/lt-bi-test-framework/BiTestHandler.al:98, CI job test-testlibraries on BC 29), and on BC 27 the
// compiler answers AL0666 and AL0928 for it, never AL0124 (must-not-compile 0784).
#[test]
fn testhandlers_is_a_codeunit_property() {
    let src = "codeunit 50000 C { Subtype = Test; TestHandlers = \"My Handler\"; }";
    assert!(!codes(&[src]).contains(&"AL0124".to_string()), "{:?}", codes(&[src]));
}

#[test]
fn al0124_orderby_on_a_query_data_item_and_al0132_length_on_a_text() {
    let q = "table 50000 T { fields { field(1; Id; Integer) { } } }
            query 50001 Q { elements { dataitem(Row; T) { OrderBy = ascending(Id); column(Id; Id) { } } } }";
    assert_eq!(codes(&[q]), vec!["AL0124"]);
    let len = "codeunit 50000 C { procedure P(): Integer var V: Text; begin exit(V.Length()); end; }";
    assert_eq!(codes(&[len]), vec!["AL0132"]);
    let strlen = "codeunit 50000 C { procedure P(): Integer var V: Text; begin exit(StrLen(V)); end; }";
    assert!(codes(&[strlen]).is_empty());
}

#[test]
fn al0161_this_names_a_base_variable_that_is_not_protected() {
    let base = "table 50000 T { fields { field(1; Id; Integer) { } } }
            report 50001 R { dataset { dataitem(Row; T) { } } var BaseVar: Text; protected var Shown: Text; }";
    let bad = "reportextension 50002 E extends R { trigger OnPreReport() begin this.BaseVar := 'x'; end; }";
    assert_eq!(codes(&[base, bad]), vec!["AL0161"]);
    let ok = "reportextension 50002 E extends R { trigger OnPreReport() begin Message(this.Shown); end; }";
    assert!(codes(&[base, ok]).is_empty());
}

// must-not-compile 0200 and 0205: an undeclared name in a codeunit (assigned, or passed as an argument).
// Declared variables, labels, procedures, system functions without parentheses and the implicit Rec of a
// codeunit with TableNo are names.
#[test]
fn al0118_undeclared_name_in_a_codeunit() {
    let assign = "codeunit 50001 C { procedure Run1() begin Missing := 1; end; }";
    assert_eq!(
        messages(assign),
        vec!["AL0118 The name 'Missing' does not exist in the current context."]
    );
    let arg = "codeunit 50001 C { procedure Run1() begin Error(MissingLbl); end; }";
    assert_eq!(codes(&[arg]), vec!["AL0118"]);
    let ok = "table 50000 T { fields { field(1; Id; Integer) { } } }
            codeunit 50001 C { var G: Integer; Lbl: Label 'x';
              procedure Run1(P: Integer): Integer var L: Integer; begin L := P + G; Error(Lbl); L := Today; exit(Helper()); end;
              procedure Helper(): Integer begin exit(1); end; }
            codeunit 50002 D { TableNo = T; trigger OnRun() begin Id := 1; end; }";
    assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
}

// must-not-compile 0206 to 0210: an undeclared name assigned in an object-level trigger of a table, page,
// report, query or xmlport. Fields, data items, columns and text elements of the object are names.
#[test]
fn al0118_undeclared_name_in_an_object_trigger() {
    let t = "table 50000 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
    let bodies = [
        "table 50009 T2 { fields { field(1; Id; Integer) { } } trigger OnInsert() begin Undeclared := 5; end; }",
        "page 50010 P { PageType = Card; trigger OnOpenPage() begin Undeclared := 5; end; }",
        "report 50011 R { trigger OnPreReport() begin Undeclared := 5; end; }",
        "query 50013 Q { elements { dataitem(Row; T) { column(Id; Id) { } } } trigger OnBeforeOpen() begin Undeclared := 5; end; }",
        "xmlport 50014 X { trigger OnPreXmlPort() begin Undeclared := 5; end; }",
    ];
    for b in bodies {
        assert_eq!(codes(&[t, b]), vec!["AL0118"], "{b}");
    }
    let ok = [
        "table 50009 T2 { fields { field(1; Id; Integer) { } } trigger OnInsert() begin Id := 5; Rec.Id := 1; xRec.Id := 2; end; }",
        "page 50010 P { PageType = Card; SourceTable = T; trigger OnOpenPage() begin Id := 5; end; }",
        "report 50011 R { dataset { dataitem(Row; T) { } } trigger OnPreReport() begin Row.Id := 5; end; }",
        "query 50013 Q { elements { dataitem(Row; T) { column(Id; Id) { } } } trigger OnBeforeOpen() begin SetFilter(Id, '1'); end; }",
        "xmlport 50014 X { schema { textelement(Root) { } } trigger OnPreXmlPort() begin Root := 'x'; end; }",
    ];
    for b in ok {
        assert!(codes(&[t, b]).is_empty(), "{b}: {:?}", messages(b));
    }
}

// must-not-compile 0223 and 0222: a codeunit named by an id-or-name that does not exist.
#[test]
fn al0118_codeunit_that_does_not_exist() {
    // The name is judged only with the platform app loaded (see
    // al0118_codeunit_name_is_not_judged_when_the_platform_app_is_not_loaded).
    let platform = "codeunit 2000000001 \"Some Platform Codeunit\" { }";
    let run = "codeunit 50001 C { procedure Run1() begin Codeunit.Run(Codeunit::\"MNC No Such Codeunit\"); end; }";
    assert_eq!(
        messages_with_platform(&[run], platform),
        vec!["AL0118 The name '\"MNC No Such Codeunit\"' does not exist in the current context."]
    );
    let sub = "codeunit 50001 C { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"MNC No Such Codeunit\", 'OnX', '', false, false)] local procedure Handle() begin end; }";
    assert_eq!(codes_with_platform(&[sub], platform), vec!["AL0118"]);
    let ok = "codeunit 50002 Other { } codeunit 50001 C { procedure Run1() begin Codeunit.Run(Codeunit::Other); end; }";
    assert!(codes_with_platform(&[ok], platform).is_empty());
}

// must-not-compile 0224 to 0226, 0228 to 0230, 0233 to 0236: a method the type does not have.
#[test]
fn al0132_method_that_a_library_type_does_not_have() {
    let cases = [
        (
            "T: Text",
            "T.Contain('x')",
            "'Text' does not contain a definition for 'Contain'",
        ),
        ("T: Text", "T.Size()", "'Text' does not contain a definition for 'Size'"),
        (
            "L: List of [Text]",
            "L.Length()",
            "'List of [Text]' does not contain a definition for 'Length'",
        ),
        (
            "D: Dictionary of [Text, Integer]",
            "D.Size()",
            "'Dictionary of [Text, Integer]' does not contain a definition for 'Size'",
        ),
        (
            "J: JsonObject",
            "J.Put('a', 1)",
            "'JsonObject' does not contain a definition for 'Put'",
        ),
        (
            "J: JsonToken",
            "J.GetName()",
            "'JsonToken' does not contain a definition for 'GetName'",
        ),
        (
            "H: HttpClient; M: HttpRequestMessage; R: HttpResponseMessage",
            "H.Send2(M, R)",
            "'HttpClient' does not contain a definition for 'Send2'",
        ),
        (
            "RR: RecordRef",
            "RR.Fieldd(1)",
            "'RecordRef' does not contain a definition for 'Fieldd'",
        ),
        (
            "B: Boolean",
            "B.Invert()",
            "'Boolean' does not contain a definition for 'Invert'",
        ),
        (
            "TB: TextBuilder",
            "TB.Add('x')",
            "'TextBuilder' does not contain a definition for 'Add'",
        ),
        // must-not-compile 2048: JsonArray has RemoveAt(Index), no Remove(JsonToken).
        (
            "J: JsonArray; T: JsonToken",
            "J.Remove(T)",
            "'JsonArray' does not contain a definition for 'Remove'",
        ),
    ];
    for (var, stmt, message) in cases {
        let cu = format!("codeunit 50001 C {{ procedure Run1() var {var}; begin {stmt}; end; }}");
        assert_eq!(messages(&cu), vec![format!("AL0132 {message}")], "{stmt}");
    }
    let ok = [
        ("T: Text", "T.Contains('x')"),
        ("L: List of [Text]", "L.Count()"),
        ("D: Dictionary of [Text, Integer]", "D.ContainsKey('a')"),
        ("J: JsonObject", "J.Add('a', 1)"),
        ("J: JsonToken", "J.IsObject()"),
        (
            "H: HttpClient; M: HttpRequestMessage; R: HttpResponseMessage",
            "H.Send(M, R)",
        ),
        ("RR: RecordRef", "RR.Field(1)"),
        ("TB: TextBuilder", "TB.Append('x')"),
    ];
    for (var, stmt) in ok {
        let cu = format!("codeunit 50001 C {{ procedure Run1() var {var}; begin {stmt}; end; }}");
        assert!(codes(&[&cu]).is_empty(), "{stmt}: {:?}", messages(&cu));
    }
}

// must-not-compile 1780: a method the codeunit of a Codeunit variable does not declare (al-runner issue 5917). A
// procedure of it (also a local one, which is AL0161), and the built-in Run, are no AL0132.
#[test]
fn al0132_method_the_codeunit_does_not_have() {
    let logic =
        "codeunit 50600 \"Probe Logic\" { procedure Add(A: Integer; B: Integer): Integer begin exit(A + B); end;
            local procedure Hidden() begin end; }";
    let call = |stmt: &str| {
        format!(
            "codeunit 50650 \"Probe Tests\" {{ procedure T() var Logic: Codeunit \"Probe Logic\"; begin {stmt}; end; }}"
        )
    };
    assert_eq!(
        messages(&format!(
            "{logic} {}",
            call("if Logic.NoSuch(1, 1) <> 2 then Error('x')")
        )),
        vec!["AL0132 'Codeunit \"Probe Logic\"' does not contain a definition for 'NoSuch'"]
    );
    for ok in ["Logic.Add(1, 1)", "Logic.Run()"] {
        assert!(codes(&[logic, &call(ok)]).is_empty(), "{ok}");
    }
    assert_eq!(codes(&[logic, &call("Logic.Hidden()")]), vec!["AL0161"]);
}

// must-not-compile 0204: an enum value the enum does not have. A value an enum extension adds is one.
#[test]
fn al0132_enum_value_that_the_enum_does_not_have() {
    let e = "enum 50006 \"MNC E204\" { Extensible = true; value(0; Red) { } value(1; Green) { } }";
    let cu = |v: &str| {
        format!("codeunit 50007 C {{ procedure Run1() var E: Enum \"MNC E204\"; begin E := \"MNC E204\"::{v}; end; }}")
    };
    let bad = cu("Purple");
    assert_eq!(
        messages(&format!("{e} {bad}")),
        vec!["AL0132 '\"MNC E204\"' does not contain a definition for 'Purple'"]
    );
    assert!(codes(&[e, &cu("Green")]).is_empty());
    let ext = "enumextension 50008 \"MNC E204 X\" extends \"MNC E204\" { value(50000; Purple) { } }";
    assert!(codes(&[e, ext, &bad]).is_empty());
}

// must-not-compile 0212: a page field bound to a column the source table does not have.
#[test]
fn al0132_page_field_bound_to_a_column_the_table_does_not_have() {
    let t = "table 50016 \"MNC T212\" { fields { field(1; Id; Integer) { } field(2; \"Long Name\"; Text[30]) { } }
            procedure Calc(): Integer begin end; }";
    let page = |src: &str| {
        format!(
            "page 50017 \"MNC P212\" {{ PageType = List; SourceTable = \"MNC T212\";
                  layout {{ area(Content) {{ repeater(Rep) {{ field(X; {src}) {{ ApplicationArea = All; }} }} }} }} }}"
        )
    };
    assert_eq!(
        messages(&format!("{t} {}", page("Rec.NoSuchColumn"))),
        vec!["AL0132 'Record \"MNC T212\"' does not contain a definition for 'NoSuchColumn'"]
    );
    for ok in [
        "Rec.Id",
        "Rec.\"Long Name\"",
        "Rec.SystemId",
        "Rec.Calc()",
        "Id",
        "Rec.Count",
    ] {
        assert!(codes(&[t, &page(ok)]).is_empty(), "{ok}");
    }
    let ext = "tableextension 50018 TE extends \"MNC T212\" { fields { field(50000; Extra; Integer) { } } }";
    assert!(codes(&[t, ext, &page("Rec.Extra")]).is_empty());
}

// must-not-compile 0260 to 0268: a variable assigned to a variable of a type it does not convert to.
#[test]
fn al0122_variable_assigned_to_a_type_it_does_not_convert_to() {
    let defs = "enum 50071 \"MNC E\" { value(0; A) { } }
            table 50075 \"MNC T\" { fields { field(1; Id; Integer) { } } }";
    let cases = [
        ("B: Boolean; V: Decimal", "B := V", "Decimal", "Boolean"),
        ("B: Boolean; V: Integer", "B := V", "Integer", "Boolean"),
        ("B: Text; V: Date", "B := V", "Date", "Text"),
        ("B: Date; V: Boolean", "B := V", "Boolean", "Date"),
        ("B: Date; V: Text", "B := V", "Text", "Date"),
        ("B: Date; V: Decimal", "B := V", "Decimal", "Date"),
        ("B: Enum \"MNC E\"; V: Variant", "B := V", "Variant", "Enum \"MNC E\""),
        (
            "B: Integer; V: Record \"MNC T\"",
            "B := V",
            "Record \"MNC T\"",
            "Integer",
        ),
        ("B: Enum \"MNC E\"", "B := 'A'", "Char", "Enum \"MNC E\""),
    ];
    for (vars, stmt, from, to) in cases {
        let cu = format!("{defs} codeunit 50001 C {{ procedure Run1() var {vars}; begin {stmt}; end; }}");
        assert_eq!(
            messages(&cu),
            vec![format!("AL0122 Cannot implicitly convert type '{from}' to '{to}'")],
            "{vars}: {stmt}"
        );
    }
    let ok = [
        "B: Boolean; V: Boolean",
        "B: Text; V: Text",
        "B: Date; V: Date",
        "B: Decimal; V: Integer",
        "B: Variant; V: Date",
    ];
    for vars in ok {
        let cu = format!("{defs} codeunit 50001 C {{ procedure Run1() var {vars}; begin B := V; end; }}");
        assert!(codes(&[&cu]).is_empty(), "{vars}: {:?}", messages(&cu));
    }
    let text = format!("{defs} codeunit 50001 C {{ procedure Run1() var B: Text; begin B := 'A'; end; }}");
    assert!(codes(&[&text]).is_empty());
}

// must-not-compile 0269 to 0273, 0285, 0286: the scalar pairs, a condition that is no Boolean.
#[test]
fn al0122_more_scalar_pairs_and_conditions() {
    let cases = [
        ("D: Date; M: DateTime", "D := M", "DateTime", "Date"),
        ("B: Boolean", "B := 'true'", "Text", "Boolean"),
        ("I: Integer; M: Time", "I := M", "Time", "Integer"),
        ("J: JsonObject; K: JsonToken", "J := K", "JsonToken", "JsonObject"),
        ("I: Integer", "if I then I := 0", "Integer", "Boolean"),
        ("T: Text", "while T do T := ''", "Text", "Boolean"),
    ];
    for (vars, stmt, from, to) in cases {
        let cu = format!("codeunit 50001 C {{ procedure Run1() var {vars}; begin {stmt}; end; }}");
        assert_eq!(
            messages(&cu),
            vec![format!("AL0122 Cannot implicitly convert type '{from}' to '{to}'")],
            "{vars}: {stmt}"
        );
    }
    let ok = [
        "B: Boolean; V: Boolean; if V then B := false",
        "T: Text; V: Boolean; while V do T := ''",
        "J: JsonToken; K: JsonObject; J := K",
        "B: Boolean; V: Text; B := V = 'x'",
        "B: Boolean; T: Text; B := T = ''",
    ];
    for stmt in ok {
        let (vars, body) = stmt.rsplit_once("; ").unwrap();
        let cu = format!("codeunit 50001 C {{ procedure Run1() var {vars}; begin {body}; end; }}");
        assert!(codes(&[&cu]).is_empty(), "{stmt}: {:?}", messages(&cu));
    }
}

// must-not-compile 0274 and 0275.
#[test]
fn al0122_exit_value_and_a_procedure_without_a_result() {
    let exit = "codeunit 50001 C { procedure Run1(): Integer begin exit('abc'); end; }";
    assert_eq!(
        messages(exit),
        vec!["AL0122 Cannot implicitly convert type 'Text' to 'Integer'"]
    );
    let none = "codeunit 50001 C { local procedure Nothing() begin end; procedure Run1() var I: Integer; begin I := Nothing(); end; }";
    assert_eq!(
        messages(none),
        vec!["AL0122 Cannot implicitly convert type 'None' to 'Integer'"]
    );
    let ok = "codeunit 50001 C { local procedure One(): Integer begin exit(1); end; procedure Run1(): Text var I: Integer; begin I := One(); exit('a'); end; procedure Run2(): Integer begin exit(One()); end; }";
    assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
    // A TryFunction returns a Boolean without declaring one (corpus errors/lt-error-handling/ErTTryBasics.al:81).
    let try_fn = "codeunit 50001 C { [TryFunction] local procedure TryOk() begin end; procedure Run1() var Ok: Boolean; begin Ok := TryOk(); end; }";
    assert!(codes(&[try_fn]).is_empty(), "{:?}", messages(try_fn));
}

// TestType on a Test codeunit compiles on BC (corpus al-language/codeunit/lt-x-object-props/XObjectProps.al:11,
// codeunits 75020-75023, every value of the enum: UnitTest, IntegrationTest, Uncategorized, AITest).
#[test]
fn al0124_testtype_on_a_test_codeunit_compiles() {
    for value in ["UnitTest", "IntegrationTest", "Uncategorized", "AITest"] {
        let cu = format!("codeunit 50001 C {{ Subtype = Test; TestType = {value}; RequiredTestIsolation = None; }}");
        assert!(codes(&[&cu]).is_empty(), "{value}: {:?}", messages(&cu));
    }
}

// must-not-compile 0305 to 0310, 0312, 0315: a property the object type does not have.
#[test]
fn al0124_a_property_the_object_does_not_have() {
    let table = "table 50120 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";
    let cases = [
        (
            format!("{table} codeunit 50121 C {{ SourceTable = T; }}"),
            "SourceTable",
        ),
        (
            "table 50122 T { ApplicationArea = All; fields { field(1; Id; Integer) { } } }".to_string(),
            "ApplicationArea",
        ),
        ("report 50123 R { Editable = false; }".to_string(), "Editable"),
        (
            "table 50124 T { PageType = List; fields { field(1; Id; Integer) { } } }".to_string(),
            "PageType",
        ),
        ("page 50125 P { TableNo = 18; }".to_string(), "TableNo"),
        (
            "codeunit 50126 C { DataPerCompany = false; }".to_string(),
            "DataPerCompany",
        ),
        (
            "codeunit 50128 C { NoSuchProperty = true; }".to_string(),
            "NoSuchProperty",
        ),
        (
            format!(
                "{table} page 50132 P {{ PageType = List; SourceTable = T; layout {{ area(Content) {{ repeater(Rep) {{ field(Id; Rec.Id) {{ ApplicationArea = All; InitValue = 5; }} }} }} }} }}"
            ),
            "InitValue",
        ),
    ];
    for (src, prop) in cases {
        assert_eq!(
            messages(&src),
            vec![format!(
                "AL0124 The property '{prop}' cannot be used in this context. Verify the property is available for the current object type."
            )],
            "{src}"
        );
    }
    let ok = [
        "codeunit 50001 C { Subtype = Test; TestPermissions = Disabled; SingleInstance = true; TableNo = 18; Permissions = tabledata T = r; Access = Internal; Description = 'x'; EventSubscriberInstance = Manual; }",
        "table 50001 T { DataPerCompany = false; Caption = 'x'; LookupPageId = 5; fields { field(1; Id; Integer) { InitValue = 5; } } }",
        "page 50001 P { PageType = List; ApplicationArea = All; Editable = false; UsageCategory = Lists; SourceTable = T; }",
        "report 50001 R { ApplicationArea = All; UsageCategory = Lists; ProcessingOnly = true; }",
    ];
    for src in ok {
        let full = format!("table 50000 T {{ fields {{ field(1; Id; Integer) {{ }} }} }} {src}")
            .replace("table 50001 T", "table 50002 T2");
        assert!(codes(&[&full]).is_empty(), "{src}: {:?}", messages(&full));
    }
}

// must-not-compile 0316 to 0321: a trigger the object type does not have.
#[test]
fn al0162_a_trigger_the_object_does_not_have() {
    let cases = [
        ("codeunit 50001 C", "OnOpenPage"),
        ("page 50001 P { PageType = Card; }", "OnInsert"),
        ("table 50001 T { fields { field(1; Id; Integer) { } } }", "OnRun"),
        ("codeunit 50001 C", "OnPreReport"),
        ("table 50001 T { fields { field(1; Id; Integer) { } } }", "OnOpenPage"),
        ("report 50001 R", "OnPreXmlPort"),
    ];
    for (head, trg) in cases {
        let head = head.strip_suffix('}').map_or(format!("{head} {{"), |h| format!("{h} "));
        let src = format!("{head} trigger {trg}() begin end; }}");
        assert_eq!(
            messages(&src),
            vec![format!(
                "AL0162 '{trg}' is not a valid trigger for this object type. Check the object documentation for supported triggers."
            )],
            "{src}"
        );
    }
    let ok = [
            "codeunit 50001 C { trigger OnRun() begin end; }",
            // The install and upgrade triggers (corpus install/TestInstallSeeder.al).
            "codeunit 50001 C { Subtype = Install; trigger OnInstallAppPerCompany() begin end; trigger OnInstallAppPerDatabase() begin end; }",
            "codeunit 50001 C { Subtype = \"Install\"; trigger OnInstallAppPerCompany() begin end; }",
            "codeunit 50001 C { Subtype = Upgrade; trigger OnUpgradePerCompany() begin end; trigger OnValidateUpgradePerDatabase() begin end; }",
            // The triggers of a test runner codeunit (corpus events/lt-trigger-pins/PTestRunnerTriggers.al:107, :116).
            "codeunit 50001 C { Subtype = TestRunner; trigger OnBeforeTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions): Boolean begin exit(true); end;
               trigger OnAfterTestRun(CodeunitId: Integer; CodeunitName: Text; FunctionName: Text; FunctionTestPermissions: TestPermissions; IsSuccess: Boolean) begin end; }",
            "table 50001 T { fields { field(1; Id; Integer) { } } trigger OnInsert() begin end; trigger OnModify() begin end; trigger OnDelete() begin end; trigger OnRename() begin end; }",
            "page 50001 P { PageType = Card; trigger OnOpenPage() begin end; trigger OnInit() begin end; trigger OnNewRecord(BelowxRec: Boolean) begin end; }",
            "report 50001 R { trigger OnPreReport() begin end; trigger OnPostReport() begin end; trigger OnInitReport() begin end; }",
        ];
    for src in ok {
        assert!(codes(&[src]).is_empty(), "{src}: {:?}", messages(src));
    }
}

// must-not-compile 0324.
#[test]
fn al0122_a_codeunit_that_does_not_implement_the_interface() {
    let head = "interface I { procedure Calc(A: Integer): Integer; } codeunit 50001 C { }";
    let user = "codeunit 50002 D { procedure Run1() var I: Interface I; C: Codeunit C; begin I := C; end; }";
    assert_eq!(
        messages(&format!("{head} {user}")),
        vec!["AL0122 Cannot implicitly convert type 'Codeunit \"C\"' to 'Interface \"I\"'"]
    );
    let ok = "interface I { procedure Calc(A: Integer): Integer; } interface J extends I { } \
            codeunit 50001 C implements I { procedure Calc(A: Integer): Integer begin end; } \
            codeunit 50003 C2 implements J { procedure Calc(A: Integer): Integer begin end; } \
            codeunit 50002 D { procedure Run1() var I: Interface I; C: Codeunit C; C2: Codeunit C2; begin I := C; I := C2; end; }";
    assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
}

// must-not-compile 0351 to 0355: system variables an object type does not have.
#[test]
fn al0118_system_variables_the_object_does_not_have() {
    let cases = [
        (
            "codeunit 50001 C { procedure Run1() begin CurrPage.Update(); end; }",
            "CurrPage",
        ),
        (
            "codeunit 50001 C { procedure Run1() begin CurrReport.Skip(); end; }",
            "CurrReport",
        ),
        ("codeunit 50001 C { procedure Run1() begin xRec.Init(); end; }", "xRec"),
        (
            "table 50001 T { fields { field(1; Id; Integer) { } } trigger OnInsert() begin CurrPage.Update(); end; }",
            "CurrPage",
        ),
        ("report 50001 R { trigger OnPreReport() begin Rec.Init(); end; }", "Rec"),
    ];
    for (src, name) in cases {
        assert_eq!(
            messages(src),
            vec![format!(
                "AL0118 The name '{name}' does not exist in the current context."
            )],
            "{src}"
        );
    }
    let ok = [
        "table 50000 T { fields { field(1; Id; Integer) { } } } codeunit 50001 C { TableNo = T; procedure Run1() begin xRec.Init(); Rec.Init(); end; }",
        "table 50001 T { fields { field(1; Id; Integer) { } } trigger OnInsert() begin Rec.Init(); xRec.Init(); end; }",
        "report 50001 R { trigger OnPreReport() begin CurrReport.Skip(); end; }",
        "page 50001 P { trigger OnOpenPage() begin CurrPage.Update(); end; }",
    ];
    for src in ok {
        assert!(codes(&[src]).is_empty(), "{src}: {:?}", messages(src));
    }
}

// must-not-compile 0356 and 0358: a local procedure and a global variable of another codeunit.
#[test]
fn al0161_a_local_procedure_or_a_variable_of_another_codeunit() {
    let local = "codeunit 50001 C { local procedure Secret() begin end; } \
            codeunit 50002 D { procedure Run1() var C: Codeunit C; begin C.Secret(); end; }";
    assert_eq!(
        messages(local),
        vec!["AL0161 'Secret()' is inaccessible due to its protection level"]
    );
    let var = "codeunit 50001 C { var Shown: Integer; } \
            codeunit 50002 D { procedure Run1() var C: Codeunit C; I: Integer; begin I := C.Shown; end; }";
    assert_eq!(
        messages(var),
        vec!["AL0161 'Shown' is inaccessible due to its protection level"]
    );
    let ok = "codeunit 50001 C { var Shown: Integer; procedure Open() begin end; local procedure Secret() begin end; \
            procedure Self() var Me: Codeunit C; begin Me.Secret(); end; } \
            codeunit 50002 D { procedure Run1() var C: Codeunit C; begin C.Open(); C.Run(); end; }";
    assert!(codes(&[ok]).is_empty(), "{:?}", messages(ok));
}

// must-not-compile 0401, 0403, 0405, 0407, 0412 to 0419, 0430 to 0438, 0445, 0448: more scalar, option and enum pairs.
#[test]
fn al0122_enum_option_and_boolean_pairs() {
    let e = "enum 50010 E { Extensible = true; AssignmentCompatibility = true; value(0; None) { } value(1; One) { } } \
            table 50011 T { fields { field(1; Id; Integer) { } field(2; Kind; Enum E) { } } } ";
    let cases = [
        ("C: Code[20]; T: Enum E", "C := T", "Enum \"E\"", "Code[20]"),
        ("X: Text; T: Enum E", "X := T", "Enum \"E\"", "Text"),
        ("X: Text; O: Option A,B,C", "O := X", "Text", "Option"),
        ("D: Decimal; T: Enum E", "T := D", "Decimal", "Enum \"E\""),
        ("C: Code[20]; T: Enum E", "T := C", "Code[20]", "Enum \"E\""),
        ("X: Text; T: Enum E", "T := X", "Text", "Enum \"E\""),
        ("D: Decimal; R: Record T", "R.Kind := D", "Decimal", "Enum \"E\""),
        ("X: Text; R: Record T", "R.Kind := X", "Text", "Enum \"E\""),
        ("B: Boolean; T: Enum E", "T := B", "Boolean", "Enum \"E\""),
        ("D: Date; T: Enum E", "T := D", "Date", "Enum \"E\""),
        ("C: Char; T: Enum E", "T := C", "Char", "Enum \"E\""),
        ("B: Boolean; G: BigInteger", "B := G", "BigInteger", "Boolean"),
        ("B: Boolean", "B := 1", "Integer", "Boolean"),
        ("B: Boolean; I: Integer", "I := B", "Boolean", "Integer"),
        ("B: Boolean; G: BigInteger", "G := B", "Boolean", "BigInteger"),
        ("B: Boolean; O: Option A,B,C", "B := O", "Option", "Boolean"),
        ("B: Boolean; O: Option A,B,C", "O := B", "Boolean", "Option"),
        ("G: BigInteger", "if G then G := 0", "BigInteger", "Boolean"),
        ("O: Option A,B,C", "if O then O := O", "Option", "Boolean"),
        ("B: Boolean; T: Enum E", "B := T", "Enum \"E\"", "Boolean"),
    ];
    for (vars, stmt, from, to) in cases {
        let cu = format!("{e} codeunit 50001 C {{ procedure Run1() var {vars}; begin {stmt}; end; }}");
        assert_eq!(
            messages(&cu),
            vec![format!("AL0122 Cannot implicitly convert type '{from}' to '{to}'")],
            "{vars}: {stmt}"
        );
    }
    let ok = [
        "B: Boolean; V: Boolean; B := V",
        "I: Integer; O: Option A,B; I := O",
        "O: Option A,B; I: Integer; O := I",
        "T: Enum E; I: Integer; I := T.AsInteger()",
        "T: Enum E; X: Text; X := Format(T)",
        "T: Enum E; U: Enum E; T := U",
        "B: Boolean; O: Option A,B; B := O = O::A",
        "G: BigInteger; I: Integer; G := I",
        "G: BigInteger; I: Integer; I := G",
    ];
    for stmt in ok {
        let (vars, body) = stmt.rsplit_once("; ").unwrap();
        let cu = format!("{e} codeunit 50001 C {{ procedure Run1() var {vars}; begin {body}; end; }}");
        assert!(codes(&[&cu]).is_empty(), "{stmt}: {:?}", messages(&cu));
    }
    // exit(value) of an enum into a procedure whose enum does not state AssignmentCompatibility (0405).
    let exit = "enum 50020 A { Extensible = true; value(0; None) { } } enum 50021 B { Extensible = true; value(0; None) { } } \
            codeunit 50001 C { procedure Run1(): Enum B var S: Enum A; begin exit(S); end; }";
    assert_eq!(
        messages(exit),
        vec!["AL0122 Cannot implicitly convert type 'Enum \"A\"' to 'Enum \"B\"'"]
    );
}

// must-not-compile 0359: a local procedure of a table, called through a record.
#[test]
fn al0161_a_local_procedure_of_a_table_through_a_record() {
    let t = "table 50000 T { fields { field(1; Id; Integer) { } } local procedure Secret() begin end; procedure Open() begin end; \
            procedure Self() var R: Record T; begin R.Secret(); end; } ";
    let bad = format!("{t} codeunit 50001 C {{ procedure Run1() var R: Record T; begin R.Secret(); end; }}");
    assert_eq!(
        messages(&bad),
        vec!["AL0161 'Secret()' is inaccessible due to its protection level"]
    );
    let ok = format!("{t} codeunit 50001 C {{ procedure Run1() var R: Record T; begin R.Open(); end; }}");
    assert!(codes(&[&ok]).is_empty(), "{:?}", messages(&ok));
}

// must-not-compile 0361 to 0363: File.Exists, File.Erase and File.Copy have scope OnPrem.
#[test]
fn al0296_static_file_methods_in_a_cloud_app() {
    for call in ["B := File.Exists('x')", "File.Erase('x')", "File.Copy('a', 'b')"] {
        let src = format!("codeunit 50000 C {{ procedure P() var B: Boolean; begin {call}; end; }}");
        assert_eq!(ctx_codes(&src, &cloud()), vec!["AL0296"], "{call}");
        let onprem = AppContext {
            target: AppTarget::OnPrem,
            ..cloud()
        };
        assert!(ctx_codes(&src, &onprem).is_empty(), "{call}");
    }
    let var = "codeunit 50000 C { procedure P() var File: Codeunit D; begin File.Exists('x'); end; } codeunit 50001 D { procedure Exists(S: Text) begin end; }";
    assert!(ctx_codes(var, &cloud()).is_empty());
}

// must-not-compile 2570, 2571: TestPage and TestPart have no GetRecord (al-runner issue 6519, writer Y54). Their methods and
// the page's controls, actions and parts are no AL0132.
#[test]
fn al0132_method_a_test_page_does_not_have() {
    let objs = "table 50570 \"MNC Y54 Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        page 50570 \"MNC Y54 List\" { PageType = List; SourceTable = \"MNC Y54 Row\";
            layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
            actions { area(Processing) { action(Go) { trigger OnAction() begin end; } } } }
        page 50571 \"MNC Y54 Host\" { PageType = Card; SourceTable = \"MNC Y54 Row\";
            layout { area(Content) { field(HostId; Rec.Id) { } part(Lines; \"MNC Y54 List\") { } } } }";
    let call = |stmt: &str| {
        format!(
            "codeunit 50572 C {{ procedure T() var Pg: TestPage \"MNC Y54 List\"; Host: TestPage \"MNC Y54 Host\";
                Row: Record \"MNC Y54 Row\"; begin {stmt}; end; }}"
        )
    };
    assert_eq!(
        messages(&format!("{objs} {}", call("Pg.GetRecord(Row)"))),
        vec!["AL0132 'TestPage MNC Y54 List' does not contain a definition for 'GetRecord'"]
    );
    assert_eq!(
        messages(&format!("{objs} {}", call("Host.Lines.GetRecord(Row)"))),
        vec!["AL0132 'Lines' does not contain a definition for 'GetRecord'"]
    );
    for ok in [
        "Pg.OpenView()",
        "Pg.Id.SetValue(1)",
        "Pg.Go.Invoke()",
        "Host.Lines.Next()",
        "Host.Lines.Id.Value()",
        "Host.HostId.Value()",
        "Pg.Close()",
    ] {
        assert!(codes(&[objs, &call(ok)]).is_empty(), "{ok}");
    }
}
