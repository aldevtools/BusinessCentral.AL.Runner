//! The incremental compile of a warm run (ADR-029): after a change of the code of routines, the program
//! the warm compile holds is the program a complete compile of the new texts gives, and the tests have
//! the outcomes they would have had. A change of a declaration of a codeunit is taken in with the objects that
//! name or call it; an object with a call or member that did not resolve before the change, and other kinds, compile
//! everything again.

use al_sema::workspace::AppSource;
use al_testrun::compile_cache::CompileCache;
use al_testrun::runtime::{Compiled, CorpusRuntime, FileChange, Manifest, RunOptions, SuiteRun, Warm, WarmUse};

const BASE: &str = r#"
table 50100 Cust { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[50]) { } } keys { key(PK; "No.") { } } }
interface IGreet { procedure Hello(): Text; }
codeunit 50101 DefaultGreet implements IGreet { procedure Hello(): Text begin exit('dep'); end; }
codeunit 50102 Dep
{
    procedure Count(): Integer var C: Record Cust; begin exit(C.Count()); end;
    procedure Say(G: Interface IGreet): Text begin exit(G.Hello()); end;
}
"#;

const LIB: &str = r#"codeunit 60100 Lib
{
    procedure Twice(N: Integer): Integer
    begin
        exit(N * 2);
    end;

    procedure Label(): Text
    begin
        exit('lib');
    end;

    procedure Add(var C: Record Cust)
    begin
        C.Name := 'n';
        C.Modify();
    end;
}
"#;

const TESTS: &str = r#"codeunit 60101 Tests
{
    Subtype = Test;

    [Test]
    procedure Inserts()
    var
        C: Record Cust;
        D: Codeunit Dep;
        L: Codeunit Lib;
    begin
        C."No." := 'A';
        C.Insert();
        L.Add(C);
        if D.Count() <> 1 then
            Error('count');
        if L.Twice(2) <> 4 then
            Error('twice');
    end;

    [Test]
    procedure Greets()
    var
        L: Codeunit Lib;
    begin
        if L.Label() <> 'lib' then
            Error('label');
    end;

    [Test]
    procedure Documents()
    var
        D: Record Doc;
    begin
        D."No." := 'X';
        D.Insert(true);
        D.Validate(Name, 'q');
        D.Modify(true);
        D.Bump();
        if D.Double() <> 2 then
            Error('double');
    end;
}
"#;

const TABLES: &str = r#"table 60110 Doc
{
    fields
    {
        field(1; "No."; Code[20])
        {
            trigger OnValidate()
            begin
                Rec.Name := 'validated';
            end;
        }
        field(2; Name; Text[50]) { }
        field(3; Amount; Decimal) { }
    }
    keys { key(PK; "No.") { } }

    trigger OnInsert()
    begin
        Rec.Amount := 1;
    end;

    procedure Double(): Decimal
    begin
        exit(Rec.Amount * 2);
    end;
}

tableextension 60111 DocExt extends Doc
{
    fields
    {
        field(60111; Extra; Integer) { }
        modify(Name)
        {
            trigger OnBeforeValidate()
            begin
                Rec.Extra := 5;
            end;
        }
    }

    trigger OnModify()
    begin
        Rec.Extra := Rec.Extra + 1;
    end;

    procedure Bump()
    begin
        Rec.Extra += 1;
    end;
}
"#;

const OTHER: &str = r#"codeunit 60102 Other
{
    Subtype = Test;

    [Test]
    procedure Fails()
    var
        C: Record Cust;
    begin
        C.Init();
        Error('always');
    end;
}
"#;

fn files(texts: &[&str]) -> Vec<(String, String)> {
    ["Lib.al", "Tests.al", "Other.al", "Tables.al"]
        .iter()
        .zip(texts)
        .map(|(p, t)| (p.to_string(), t.to_string()))
        .collect()
}

fn manifests() -> Vec<Manifest> {
    ["Me", "Microsoft"]
        .iter()
        .zip(["1.0.0.0", "27.5.0.0"])
        .map(|(p, v)| Manifest {
            publisher: p.to_string(),
            version: v.into(),
            resources: vec![],
            include_source: false,
            target: String::new(),
            logo: None,
            description: None,
            layout_files: Vec::new(),
            id_ranges: Vec::new(),
            translations: false,
            package_data: false,
            package_data_files: Vec::new(),
            translation_files: Vec::new(),
            translation_source: None,
            runtime: None,
            app_json_keys: Vec::new(),
            platform_internals_visible_to: Vec::new(),
            symbols: vec![],
        })
        .collect()
}

fn build(texts: &[&str]) -> (Vec<AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>) {
    let (mut apps, mut kinds, mut m) = build_main(texts);
    // A second workspace app (`SECOND`): one file, Lib.al, the relative path of a file of the main app.
    if let Some(text) = SECOND.with(|s| s.borrow().clone()) {
        apps.push(AppSource {
            id: "second".into(),
            name: "second".into(),
            dependencies: vec!["main".into(), "base".into()],
            files: vec![("Lib.al".into(), text)],
        });
        kinds.push(al_sema::AppKind::Workspace);
        m.push(m[0].clone());
    }
    (apps, kinds, m)
}

fn build_main(texts: &[&str]) -> (Vec<AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>) {
    (
        vec![
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["base".into()],
                files: files(texts),
            },
            AppSource {
                id: "base".into(),
                name: "base".into(),
                dependencies: vec![],
                files: vec![("base.al".into(), BASE.into())],
            },
        ],
        vec![al_sema::AppKind::Workspace, al_sema::AppKind::Dependency],
        manifests(),
    )
}

fn opts(dir: &std::path::Path) -> RunOptions {
    RunOptions {
        compile_cache: Some(CompileCache {
            dir: dir.to_path_buf(),
            require: false,
            rebuild: false,
            country: String::new(),
        }),
        only: SELECTION.with(|s| s.borrow().clone()),
        tdd: TDD.with(|t| *t.borrow()),
        ..Default::default()
    }
}

thread_local! {
    /// The selection the helpers run with (the tests run on their own threads).
    static SELECTION: std::cell::RefCell<Option<Vec<(usize, u32)>>> = const { std::cell::RefCell::new(None) };
    /// Whether the helpers compile with `--tdd` (ADR-051).
    static TDD: std::cell::RefCell<bool> = const { std::cell::RefCell::new(false) };
    /// The text of a second workspace app's Lib.al, when the helpers build one.
    static SECOND: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

fn changes<'a>(fs: &'a [(String, String)]) -> Vec<FileChange<'a>> {
    fs.iter()
        .map(|(p, t)| FileChange {
            app: 0,
            path: p,
            text: t,
        })
        .collect()
}

fn outcomes(rt: &CorpusRuntime) -> Vec<String> {
    rt.results()
        .iter()
        .map(|r| format!("{} {} {} {:?}", r.app, r.codeunit, r.test, r.outcome))
        .collect()
}

/// The warm run on `texts`, and a complete compile of the same texts to compare with.
fn warm_run(warm: &mut Warm, texts: &[&str], dir: &std::path::Path) -> (WarmUse, Vec<String>) {
    let fs = files(texts);
    let o = opts(dir);
    let (rt, used) = warm
        .run(
            "k",
            &changes(&fs),
            || build(texts),
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &o,
        )
        .unwrap();
    (used, outcomes(&rt))
}

/// The complete compile holds every test codeunit, as a warm one does whatever the selection (al-runner issue 7978);
/// the run runs the selection.
fn complete(texts: &[&str], dir: &std::path::Path) -> (Compiled, Vec<String>) {
    let (apps, kinds, m) = build(texts);
    let o = opts(dir);
    let c = CorpusRuntime::compile_suites(
        apps,
        kinds,
        &m,
        &Default::default(),
        None,
        &[SuiteRun::everything()],
        &RunOptions {
            only: None,
            ..o.clone()
        },
    )
    .unwrap();
    let rt = CorpusRuntime::run_compiled(&c, None, Default::default(), &o);
    (c, outcomes(&rt))
}

/// The warm compile and a complete one hold the same program, refusals, call graph and diagnostics.
fn assert_same(warm: &Warm, texts: &[&str], dir: &std::path::Path, what: &str) {
    let (full, _) = complete(texts, dir);
    warm.compiled()
        .unwrap()
        .same_lowering_as(&full)
        .unwrap_or_else(|why| panic!("{what}: {why}"));
}

fn edit(texts: [&str; 4], which: usize, from: &str, to: &str) -> [String; 4] {
    let mut out = texts.map(|t| t.to_string());
    assert!(out[which].contains(from), "{from}");
    out[which] = out[which].replacen(from, to, 1);
    out
}

#[test]
fn a_change_of_code_gives_the_program_a_complete_compile_gives() {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    let (used, first) = warm_run(&mut warm, &[LIB, TESTS, OTHER, TABLES], dir.path());
    assert_eq!(used, WarmUse::Compiled);
    assert!(first.iter().any(|l| l.contains("Pass")), "{first:?}");
    let steps: Vec<(usize, &str, &str, &str)> = vec![
        (0, "exit(N * 2);", "exit(N + N);", "an expression"),
        (
            0,
            "exit(N + N);",
            "if N > 100 then Error('big'); exit(N * 2);",
            "a statement more",
        ),
        (0, "exit('lib');", "exit('Lib');", "a literal that a test reads"),
        (1, "Error('count');", "Error('count of rows');", "an error text"),
        (
            1,
            "L: Codeunit Lib;\n    begin\n        C.\"No.\"",
            "L: Codeunit Lib;\n        Extra: Integer;\n    begin\n        C.\"No.\"",
            "a local variable more",
        ),
        (
            2,
            "Error('always');",
            "// nothing\n        Error('always');",
            "a comment",
        ),
        (
            0,
            "C.Name := 'n';",
            "C.Name := 'n';\n        C.Name := 'm';",
            "a line more in a procedure with a var parameter",
        ),
    ];
    let mut cur: [String; 4] = [LIB.into(), TESTS.into(), OTHER.into(), TABLES.into()];
    for (which, from, to, what) in steps {
        let next = edit([&cur[0], &cur[1], &cur[2], &cur[3]], which, from, to);
        cur = next;
        let texts = [cur[0].as_str(), cur[1].as_str(), cur[2].as_str(), cur[3].as_str()];
        let (used, got) = warm_run(&mut warm, &texts, dir.path());
        assert_eq!(used, WarmUse::Updated, "{what}");
        assert_same(&warm, &texts, dir.path(), what);
        let (_, want) = complete(&texts, dir.path());
        assert_eq!(got, want, "{what}: outcomes");
    }
}

/// Warm run on `texts`, then on `changed`: the second must be an update and equal a complete compile.
fn assert_updated(from: [&str; 4], to: [&str; 4], what: &str) {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    warm_run(&mut warm, &from, dir.path());
    let (used, got) = warm_run(&mut warm, &to, dir.path());
    assert_eq!(used, WarmUse::Updated, "{what}");
    assert_same(&warm, &to, dir.path(), what);
    let (_, want) = complete(&to, dir.path());
    assert_eq!(got, want, "{what}: outcomes");
}

#[test]
fn a_change_of_a_declaration_of_a_codeunit_and_the_code_that_uses_it_is_taken_in() {
    let lib = LIB.replace("procedure Twice(N: Integer)", "procedure Twice(N: Integer; M: Integer)");
    let tests = TESTS.replace("L.Twice(2)", "L.Twice(2, 0)");
    assert_updated(
        [LIB, TESTS, OTHER, TABLES],
        [lib.as_str(), tests.as_str(), OTHER, TABLES],
        "a new parameter and its caller",
    );
}

#[test]
fn a_declaration_the_readers_did_not_change_with_is_seen_by_them() {
    // A procedure more in a codeunit that other code names, and a signature the callers have not followed
    // (the complete compile refuses the callers; the update must say the same).
    let more = LIB.replace(
        "    procedure Label(): Text",
        "    procedure Extra(): Integer\n    begin\n        exit(1);\n    end;\n\n    procedure Label(): Text",
    );
    assert_updated(
        [LIB, TESTS, OTHER, TABLES],
        [more.as_str(), TESTS, OTHER, TABLES],
        "a procedure more",
    );
    let broken = LIB.replace("procedure Twice(N: Integer)", "procedure Twice(N: Integer; M: Integer)");
    assert_updated(
        [LIB, TESTS, OTHER, TABLES],
        [broken.as_str(), TESTS, OTHER, TABLES],
        "a signature the caller did not follow",
    );
    let gone = LIB.replace("procedure Label(): Text", "procedure Label2(): Text");
    assert_updated(
        [LIB, TESTS, OTHER, TABLES],
        [gone.as_str(), TESTS, OTHER, TABLES],
        "a procedure renamed",
    );
}

#[test]
fn a_declaration_of_a_codeunit_nothing_names_is_taken_in_alone() {
    let other = OTHER.replace(
        "    [Test]\n    procedure Fails()",
        "    procedure Added() begin end;\n\n    [Test]\n    procedure Fails()",
    );
    assert_updated(
        [LIB, TESTS, OTHER, TABLES],
        [LIB, TESTS, other.as_str(), TABLES],
        "a procedure more",
    );
}

#[test]
fn a_change_of_a_declaration_of_a_table_compiles_everything_again() {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    warm_run(&mut warm, &[LIB, TESTS, OTHER, TABLES], dir.path());
    let tables = TABLES.replace(
        "field(3; Amount; Decimal) { }",
        "field(3; Amount; Decimal) { }\n        field(4; More; Integer) { }",
    );
    let texts = [LIB, TESTS, OTHER, tables.as_str()];
    let (used, got) = warm_run(&mut warm, &texts, dir.path());
    assert_eq!(used, WarmUse::Compiled);
    assert_same(&warm, &texts, dir.path(), "a field more");
    let (_, want) = complete(&texts, dir.path());
    assert_eq!(got, want);
}

#[test]
fn a_text_that_does_not_parse_is_compiled_all_over_and_the_next_good_one_is_taken_in() {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    warm_run(&mut warm, &[LIB, TESTS, OTHER, TABLES], dir.path());
    let broken = LIB.replace("exit(N * 2);", "exit(N * ;");
    let (used, _) = warm_run(&mut warm, &[broken.as_str(), TESTS, OTHER, TABLES], dir.path());
    assert_eq!(used, WarmUse::Compiled);
    let fixed = LIB.replace("exit(N * 2);", "exit(N * 3);");
    let texts = [fixed.as_str(), TESTS, OTHER, TABLES];
    let (used, got) = warm_run(&mut warm, &texts, dir.path());
    assert_eq!(used, WarmUse::Updated);
    assert_same(&warm, &texts, dir.path(), "after a broken text");
    let (_, want) = complete(&texts, dir.path());
    assert_eq!(got, want);
}

/// With a selection, a change in a test codeunit it does not select updates the compile as a complete compile does: the
/// warm compile holds every test codeunit, the selection only picks what runs (al-runner issue 7978).
#[test]
fn with_a_selection_a_change_in_an_unselected_test_codeunit_updates_like_a_complete_compile() {
    SELECTION.with(|s| *s.borrow_mut() = Some(vec![(0, 60101)]));
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    warm_run(&mut warm, &[LIB, TESTS, OTHER, TABLES], dir.path());
    let other = OTHER.replace("C.Init();", "// EventSubscriber\n        C.Init();");
    let texts = [LIB, TESTS, other.as_str(), TABLES];
    let (used, got) = warm_run(&mut warm, &texts, dir.path());
    assert_eq!(used, WarmUse::Updated);
    assert_same(&warm, &texts, dir.path(), "a changed unselected test codeunit");
    let (_, want) = complete(&texts, dir.path());
    assert_eq!(got, want);
    // And back.
    let (used, _) = warm_run(&mut warm, &[LIB, TESTS, OTHER, TABLES], dir.path());
    assert_eq!(used, WarmUse::Updated);
    assert_same(
        &warm,
        &[LIB, TESTS, OTHER, TABLES],
        dir.path(),
        "an unselected test codeunit changed back",
    );
}

/// The stubs of the program the warm compile holds (`--tdd`, ADR-051).
fn stubs(warm: &Warm) -> Vec<String> {
    warm.compiled().unwrap().lowered().program.tdd_stubs.clone()
}

#[test]
fn with_tdd_the_stubs_follow_the_objects_compiled_again_and_the_real_procedure_ends_one() {
    TDD.with(|t| *t.borrow_mut() = true);
    let thrice = r#"Lib: procedure "Thrice"(Arg1: Integer): Integer"#;
    let ready = r#"Lib: procedure "Ready"(): Boolean"#;
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    let tests = TESTS.replace(
        "if L.Label() <> 'lib' then",
        "if L.Thrice(2) <> 0 then\n            Error('thrice');\n        if L.Label() <> 'lib' then",
    );
    let mut cur: [String; 4] = [LIB.into(), tests, OTHER.into(), TABLES.into()];
    let texts = |c: &[String; 4]| [c[0].clone(), c[1].clone(), c[2].clone(), c[3].clone()];
    let run = |warm: &mut Warm, c: &[String; 4]| {
        let t = texts(c);
        let t: Vec<&str> = t.iter().map(String::as_str).collect();
        let (used, got) = warm_run(warm, &t, dir.path());
        assert_same(warm, &t, dir.path(), "tdd");
        let (_, want) = complete(&t, dir.path());
        assert_eq!(got, want, "outcomes");
        (used, got)
    };
    let (used, got) = run(&mut warm, &cur);
    assert_eq!(used, WarmUse::Compiled);
    assert!(got.iter().any(|l| l.contains("Greets Pass")), "{got:?}");
    assert_eq!(stubs(&warm), vec![thrice.to_string()]);
    // A stub more in the tests: only they are compiled again, and both stubs are in the list.
    cur[1] = cur[1].replacen("Error('thrice');", "Error('thrice');\n        if L.Ready() then;", 1);
    assert_eq!(run(&mut warm, &cur).0, WarmUse::Updated);
    assert_eq!(stubs(&warm), vec![ready.to_string(), thrice.to_string()]);
    // A change in another object keeps the stubs of the tests.
    cur[2] = cur[2].replacen("Error('always');", "// nothing\n        Error('always');", 1);
    assert_eq!(run(&mut warm, &cur).0, WarmUse::Updated);
    assert_eq!(stubs(&warm), vec![ready.to_string(), thrice.to_string()]);
    // The real procedure is added: a declaration change, and its stub is gone.
    cur[0] = cur[0].replacen(
        "    procedure Label(): Text",
        "    procedure Thrice(N: Integer): Integer\n    begin\n        exit(N * 3);\n    end;\n\n    procedure Label(): Text",
        1,
    );
    let (used, got) = run(&mut warm, &cur);
    assert_eq!(used, WarmUse::Updated);
    assert_eq!(stubs(&warm), vec![ready.to_string()]);
    assert!(
        got.iter().any(|l| l.contains("Greets Fail")),
        "the real Thrice returns 6: {got:?}"
    );
}

/// A change that makes a selected test call a procedure of a test codeunit the selection does not select gives the
/// outcome of a complete compile.
#[test]
fn a_change_that_names_an_unselected_test_codeunit_gives_the_outcome_of_a_complete_compile() {
    SELECTION.with(|s| *s.borrow_mut() = Some(vec![(0, 60101)]));
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    let other = OTHER.replace(
        "    [Test]\n    procedure Fails()",
        "    procedure Seven(): Integer\n    begin\n        exit(7);\n    end;\n\n    [Test]\n    procedure Fails()",
    );
    assert_ne!(other, OTHER);
    warm_run(&mut warm, &[LIB, TESTS, other.as_str(), TABLES], dir.path());
    let tests = TESTS.replace(
        "        if L.Label() <> 'lib' then",
        "        if Codeunit_Other.Seven() <> 7 then\n            Error('seven');\n        if L.Label() <> 'lib' then",
    );
    let tests = tests.replace(
        "    procedure Greets()\n    var\n",
        "    procedure Greets()\n    var\n        Codeunit_Other: Codeunit Other;\n",
    );
    assert!(tests.contains("Codeunit_Other: Codeunit Other;"));
    let texts = [LIB, tests.as_str(), other.as_str(), TABLES];
    let (_, got) = warm_run(&mut warm, &texts, dir.path());
    assert_same(
        &warm,
        &texts,
        dir.path(),
        "a test that names an unselected test codeunit",
    );
    let (_, want) = complete(&texts, dir.path());
    assert_eq!(got, want);
    assert!(got.iter().any(|l| l.contains("60101 Greets Pass")), "{got:?}");
}

/// Codeunit Inner and codeunit Middle, whose procedure returns an Inner: code that calls `M.Get().Value()` reaches Inner
/// without naming it. Added to Lib.al and Tables.al.
const INNER: &str =
    "\ncodeunit 60120 Inner\n{\n    procedure Value(): Integer\n    begin\n        exit(7);\n    end;\n}\n";
const MIDDLE: &str = "\ncodeunit 60121 Middle\n{\n    procedure Get(): Codeunit Inner\n    var\n        I: Codeunit Inner;\n    begin\n        exit(I);\n    end;\n}\n";

/// The texts with Inner and Middle, and `reader` added to Other.al. The reader names neither Inner nor its id.
fn through_middle(inner: &str, reader: &str) -> [String; 4] {
    assert!(!reader.to_ascii_lowercase().contains("inner") && !reader.contains("60120"));
    [
        LIB.to_string() + inner,
        TESTS.to_string(),
        OTHER.to_string() + reader,
        TABLES.to_string() + MIDDLE,
    ]
}

fn strs(t: &[String; 4]) -> [&str; 4] {
    [t[0].as_str(), t[1].as_str(), t[2].as_str(), t[3].as_str()]
}

/// Al-runner issue 7625 (item 5): a reader that does not name the changed codeunit, because it reaches it through a
/// procedure of another codeunit that returns it, is lowered again too: its call follows the new return type. One reader
/// is a test codeunit, one a table procedure.
#[test]
fn a_reader_that_reaches_the_changed_codeunit_through_another_one_follows_its_new_return_type() {
    const OUTER: &str = "\ncodeunit 60122 Outer\n{\n    Subtype = Test;\n\n    [Test]\n    procedure ReadsThroughTheMiddle()\n    var\n        M: Codeunit Middle;\n    begin\n        if Kind(M.Get().Value()) <> 'text' then\n            Error('the overload for %1', Kind(M.Get().Value()));\n    end;\n\n    [Test]\n    procedure ReadsThroughATable()\n    var\n        R: Record ReaderTable;\n    begin\n        if R.ReadKind() <> 'text' then\n            Error('the table overload for %1', R.ReadKind());\n    end;\n\n    local procedure Kind(X: Integer): Text\n    begin\n        exit('integer');\n    end;\n\n    local procedure Kind(X: Text): Text\n    begin\n        exit('text');\n    end;\n}\n\ntable 60123 ReaderTable\n{\n    fields { field(1; Id; Integer) { } }\n\n    procedure ReadKind(): Text\n    var\n        M: Codeunit Middle;\n    begin\n        exit(Kind(M.Get().Value()));\n    end;\n\n    local procedure Kind(X: Integer): Text\n    begin\n        exit('integer');\n    end;\n\n    local procedure Kind(X: Text): Text\n    begin\n        exit('text');\n    end;\n}\n";
    let from = through_middle(INNER, OUTER);
    let mut to = from.clone();
    to[0] = from[0]
        .replace("procedure Value(): Integer", "procedure Value(): Text")
        .replace("exit(7);", "exit('7');");
    assert_updated(
        strs(&from),
        strs(&to),
        "a return type from Integer to Text, read through another codeunit",
    );
}

/// Warm run on `from`, then on `to`: the second compiles everything again (the update cannot tell which objects read the
/// change) and equals a complete compile.
fn assert_compiled_again(from: [&str; 4], to: [&str; 4], what: &str) {
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    warm_run(&mut warm, &from, dir.path());
    let (used, got) = warm_run(&mut warm, &to, dir.path());
    assert_same(&warm, &to, dir.path(), what);
    let (_, want) = complete(&to, dir.path());
    assert_eq!(got, want, "{what}: outcomes");
    assert_eq!(used, WarmUse::Compiled, "{what}");
}

/// A call through Middle that did not resolve before the change resolves after it: a procedure added to Inner. The
/// reader's call was a compile error (AL0132) and is in no call list of the program before the change.
#[test]
fn a_call_that_did_not_resolve_before_a_procedure_was_added_gives_the_outcome_of_a_complete_compile() {
    const READER: &str = "\ncodeunit 60150 OnlyNew\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Probe()\n    var\n        M: Codeunit Middle;\n    begin\n        Error('extra=%1', M.Get().Extra());\n    end;\n}\n";
    let from = through_middle(INNER, READER);
    let mut to = from.clone();
    to[0] = from[0].replace(
        "    procedure Value(): Integer",
        "    procedure Extra(): Integer\n    begin\n        exit(9);\n    end;\n\n    procedure Value(): Integer",
    );
    assert_compiled_again(
        strs(&from),
        strs(&to),
        "a procedure added that a call through another codeunit wanted",
    );
}

/// The same with a call whose argument count fits only after the change; before it the checker resolved no callee and
/// the routine was refused.
#[test]
fn a_call_whose_argument_count_fits_only_after_the_change_gives_the_outcome_of_a_complete_compile() {
    const TEXT_INNER: &str =
        "\ncodeunit 60120 Inner\n{\n    procedure Value(): Text\n    begin\n        exit('one');\n    end;\n}\n";
    const READER: &str = "\ncodeunit 60150 ArgCount\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Probe()\n    var\n        M: Codeunit Middle;\n    begin\n        Error('value=%1', M.Get().Value(5));\n    end;\n}\n";
    let from = through_middle(TEXT_INNER, READER);
    let mut to = from.clone();
    to[0] = from[0].replace("procedure Value(): Text", "procedure Value(X: Integer): Text");
    assert_compiled_again(
        strs(&from),
        strs(&to),
        "a parameter added that a call through another codeunit passed",
    );
}

/// A call whose `var` argument fits only after the change, in an object that names neither codeunit: the compile before
/// the change held an error in its file (AL0130), so the update compiles everything again.
#[test]
fn a_var_argument_that_fits_only_after_the_change_gives_the_outcome_of_a_complete_compile() {
    const VAR_INNER: &str = "\ncodeunit 60120 Inner\n{\n    procedure Value(var X: Integer): Text\n    begin\n        exit('one');\n    end;\n}\n";
    const READER: &str = "\ncodeunit 60150 VarArgument\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Probe()\n    var\n        M: Codeunit Middle;\n    begin\n        Error('value=%1', M.Get().Value(5));\n    end;\n}\n";
    let from = through_middle(VAR_INNER, READER);
    let mut to = from.clone();
    to[0] = from[0].replace("Value(var X: Integer)", "Value(X: Integer)");
    assert_compiled_again(strs(&from), strs(&to), "a var parameter made a value parameter");
}

/// A call that did not resolve before the change, in an object that names the changed codeunit: the object is a reader
/// and is lowered again with it, so the update goes on.
#[test]
fn an_unresolved_call_in_a_reader_that_names_the_changed_codeunit_is_taken_in_by_the_update() {
    const READER: &str = "\ncodeunit 60150 NamesIt\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Probe()\n    var\n        M: Codeunit Middle;\n    begin\n        Error('extra=%1', M.Get().Extra());\n    end;\n\n    procedure Unused(I: Codeunit Inner)\n    begin\n    end;\n}\n";
    let from = [
        LIB.to_string() + INNER,
        TESTS.to_string(),
        OTHER.to_string() + READER,
        TABLES.to_string() + MIDDLE,
    ];
    let mut to = from.clone();
    to[0] = from[0].replace(
        "    procedure Value(): Integer",
        "    procedure Extra(): Integer\n    begin\n        exit(9);\n    end;\n\n    procedure Value(): Integer",
    );
    assert_updated(
        strs(&from),
        strs(&to),
        "a procedure added that a reader that names Inner wanted",
    );
}

/// The same with a member read without parentheses: the checker raised AL0132 for it, and lowering refused the read
/// without reaching a call.
#[test]
fn a_member_read_without_parentheses_that_did_not_resolve_before_gives_the_outcome_of_a_complete_compile() {
    const READER: &str = "\ncodeunit 60150 NoParens\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Probe()\n    var\n        M: Codeunit Middle;\n    begin\n        Error('extra=%1', M.Get().Extra);\n    end;\n}\n";
    let from = through_middle(INNER, READER);
    let mut to = from.clone();
    to[0] = from[0].replace(
        "    procedure Value(): Integer",
        "    procedure Extra(): Integer\n    begin\n        exit(9);\n    end;\n\n    procedure Value(): Integer",
    );
    assert_compiled_again(
        strs(&from),
        strs(&to),
        "a procedure added that a member read through another codeunit wanted",
    );
}

/// An object outside the readers whose routine is refused for another reason than an unresolved call does not stop the
/// update.
#[test]
fn a_refused_routine_outside_the_readers_does_not_stop_the_update() {
    const REFUSED: &str = "\ncodeunit 60151 Refused\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Binds()\n    var\n        X: DotNet Missing;\n    begin\n    end;\n}\n";
    let from = through_middle(INNER, REFUSED);
    let mut to = from.clone();
    to[0] = from[0].replace(
        "    procedure Value(): Integer",
        "    procedure Extra(): Integer\n    begin\n        exit(9);\n    end;\n\n    procedure Value(): Integer",
    );
    let dir = tempfile::tempdir().unwrap();
    let (c, _) = complete(&strs(&from), dir.path());
    let refused = c
        .lowered()
        .unsupported
        .iter()
        .flatten()
        .flatten()
        .any(|r| r.reason().contains("DotNet 'Missing'"));
    assert!(
        refused,
        "Refused.Binds is refused: {:?}",
        c.lowered()
            .unsupported
            .iter()
            .flatten()
            .flatten()
            .map(|r| r.reason().to_string())
            .collect::<Vec<_>>()
    );
    assert_updated(strs(&from), strs(&to), "a procedure added, a routine refused elsewhere");
}

/// `--tdd`: a procedure stub site in an object that neither names nor calls the changed codeunit
/// (`M.Get().Extra()` before Inner has Extra) is a call with no callee the checker resolved. After Extra is added the
/// call resolves to it, so the update compiles everything again. One site has a typed use, one is refused (the return
/// type is not inferred inside `Error`).
#[test]
fn with_tdd_a_stub_site_outside_the_readers_gives_the_outcome_of_a_complete_compile() {
    TDD.with(|t| *t.borrow_mut() = true);
    const TYPED: &str = "\ncodeunit 60150 Typed\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Probe()\n    var\n        M: Codeunit Middle;\n        Y: Integer;\n    begin\n        Y := M.Get().Extra();\n        Error('extra=%1', Y);\n    end;\n}\n";
    const REFUSED: &str = "\ncodeunit 60150 InError\n{\n    Subtype = Test;\n\n    [Test]\n    procedure Probe()\n    var\n        M: Codeunit Middle;\n    begin\n        Error('extra=%1', M.Get().Extra());\n    end;\n}\n";
    for (reader, what) in [(TYPED, "a typed stub site"), (REFUSED, "a refused stub site")] {
        let from = through_middle(INNER, reader);
        let mut to = from.clone();
        to[0] = from[0].replace(
            "    procedure Value(): Integer",
            "    procedure Extra(): Integer\n    begin\n        exit(9);\n    end;\n\n    procedure Value(): Integer",
        );
        assert_compiled_again(strs(&from), strs(&to), what);
    }
}

/// A body-only change of the main app's Lib.al keeps the compile error of the second app's Lib.al (the same relative
/// path, another app): the kept diagnostics are dropped by app and file together.
#[test]
fn a_body_change_keeps_the_errors_of_a_file_of_another_app_with_the_same_path() {
    SECOND.with(|s| {
        *s.borrow_mut() = Some(
            "codeunit 60190 Broken\n{\n    procedure Run()\n    var\n        L: Codeunit Lib;\n    begin\n        L.Missing();\n    end;\n}\n"
                .into(),
        )
    });
    let lib = LIB.replace("exit(N * 2);", "exit(N + N);");
    let dir = tempfile::tempdir().unwrap();
    let (c, _) = complete(&[LIB, TESTS, OTHER, TABLES], dir.path());
    assert!(
        c.lowered()
            .al_diagnostics
            .iter()
            .any(|d| d.app == 2 && d.file == "Lib.al"),
        "the second app's Lib.al does not compile: {:?}",
        c.lowered().al_diagnostics
    );
    assert_updated(
        [LIB, TESTS, OTHER, TABLES],
        [lib.as_str(), TESTS, OTHER, TABLES],
        "a body change in a path another app has too",
    );
}
