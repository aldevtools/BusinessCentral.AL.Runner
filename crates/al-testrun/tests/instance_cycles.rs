//! A warm server that runs the same request again holds no more object instances than after the first one, also when
//! the tests' AL code builds a reference cycle (al-runner issue 7975: Microsoft's SAF-T tests grew the daemon by about
//! 0.8 MB per request). Its own test binary: the count of live instances is one per process.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{FileChange, Manifest, RunOptions, SuiteRun, Warm};

/// A table whose global codeunit holds a record of the table, whose table instance is the table's own; a page whose
/// instance holds its `CurrPage`.
const APP: &str = r#"
table 50300 "Cycle Holder"
{
    fields { field(1; K; Code[10]) { } }
    keys { key(PK; K) { } }

    var
        Helper: Codeunit "Cycle Helper";

    procedure SetHelper(var H: Codeunit "Cycle Helper")
    begin
        Helper := H;
    end;
}

codeunit 50301 "Cycle Helper"
{
    var
        R: Record "Cycle Holder";

    procedure Link(var Self: Codeunit "Cycle Helper")
    begin
        R.SetHelper(Self);
    end;
}

page 50302 "Cycle Page"
{
    PageType = Card;
    SourceTable = "Cycle Holder";

    procedure Touch()
    begin
    end;
}

codeunit 60300 "Cycle Tests"
{
    Subtype = Test;

    [Test]
    procedure BuildsCycles()
    var
        H: Codeunit "Cycle Helper";
        P: Page "Cycle Page";
    begin
        H.Link(H);
        P.Touch();
    end;
}
"#;

fn manifest() -> Manifest {
    Manifest {
        publisher: "Me".into(),
        version: "1.0.0.0".into(),
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
    }
}

fn build() -> (Vec<AppSource>, Vec<al_sema::AppKind>, Vec<Manifest>) {
    (
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("App.al".into(), APP.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        vec![manifest()],
    )
}

/// One request; its outcomes, then the instances the process holds once it is done.
fn request(warm: &mut Warm) -> (Vec<String>, usize) {
    let changes = [FileChange {
        app: 0,
        path: "App.al",
        text: APP,
    }];
    let (rt, _) = warm
        .run(
            "k",
            &changes,
            build,
            &Default::default(),
            None,
            Default::default(),
            &[SuiteRun::everything()],
            &RunOptions::default(),
        )
        .unwrap();
    let outcomes = rt
        .results()
        .iter()
        .map(|r| format!("{} {:?}", r.test, r.outcome))
        .collect();
    drop(rt);
    (outcomes, al_builtins::instance_cycles::live())
}

#[test]
fn a_repeated_request_frees_the_instances_of_reference_cycles() {
    let mut warm = Warm::new();
    let (first, after_first) = request(&mut warm);
    assert_eq!(first, vec!["BuildsCycles Pass".to_string()]);
    for n in 2..=5 {
        let (outcomes, live) = request(&mut warm);
        assert_eq!(outcomes, first, "request {n}");
        assert_eq!(live, after_first, "request {n} left instances behind");
    }
}
