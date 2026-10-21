//! The incremental compile of a warm run (ADR-029): after a change of the code of routines, the program
//! the warm compile holds is the program a complete compile of the new texts gives, and the tests have
//! the outcomes they would have had. A change of a declaration of a codeunit is taken in with the objects that
//! name it; other kinds compile everything again.

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
            layout_files: Vec::new(),
            id_ranges: Vec::new(),
            symbols: vec![],
        })
        .collect()
}

fn build(texts: &[&str]) -> (Vec<AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>) {
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
        &o,
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

#[test]
fn a_test_codeunit_the_selection_leaves_out_is_left_out_or_not_by_its_own_text() {
    SELECTION.with(|s| *s.borrow_mut() = Some(vec![(0, 60101)]));
    let dir = tempfile::tempdir().unwrap();
    let mut warm = Warm::new();
    warm_run(&mut warm, &[LIB, TESTS, OTHER, TABLES], dir.path());
    // A word in a comment makes the left-out test codeunit stay compiled (it might subscribe to events).
    let other = OTHER.replace("C.Init();", "// EventSubscriber\n        C.Init();");
    let texts = [LIB, TESTS, other.as_str(), TABLES];
    let (used, got) = warm_run(&mut warm, &texts, dir.path());
    assert_eq!(used, WarmUse::Updated);
    assert_same(&warm, &texts, dir.path(), "a left-out test codeunit that stays");
    let (_, want) = complete(&texts, dir.path());
    assert_eq!(got, want);
    // And back.
    let (used, _) = warm_run(&mut warm, &[LIB, TESTS, OTHER, TABLES], dir.path());
    assert_eq!(used, WarmUse::Updated);
    assert_same(
        &warm,
        &[LIB, TESTS, OTHER, TABLES],
        dir.path(),
        "a left-out test codeunit that goes",
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
