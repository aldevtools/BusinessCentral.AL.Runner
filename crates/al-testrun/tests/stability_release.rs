//! A stability run (ADR-058 section 7) runs one variant after the other in one process. What the AL code of one
//! variant made is gone once the variant is done, also an instance that a reference cycle `instance_cycles::collect`
//! cannot see keeps alive (al-runner issue 6924: the reporter's run grew by gigabytes from variant to variant). Its own
//! test binary: the count of live instances is one per process.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, Manifest, RunOptions, SuiteRun};
use al_testrun::stability::Perturb;

/// A codeunit whose global query holds the codeunit itself: the reference goes through a query's globals, which
/// `collect` does not look into, so the codeunit looks held from outside.
const APP: &str = r#"
table 50310 "Release Row"
{
    fields { field(1; K; Code[10]) { } }
    keys { key(PK; K) { } }
}

query 50311 "Release Query"
{
    elements
    {
        dataitem(Row; "Release Row")
        {
            column(K; K) { }
        }
    }

    var
        Helper: Codeunit "Release Helper";

    procedure SetHelper(var H: Codeunit "Release Helper")
    begin
        Helper := H;
    end;
}

codeunit 50312 "Release Helper"
{
    var
        Q: Query "Release Query";

    procedure Link(var Self: Codeunit "Release Helper")
    begin
        Q.SetHelper(Self);
    end;
}

"#;

/// The test codeunits: `n` of them, so the parallel part of a run gives some to worker threads.
fn tests(n: u32) -> String {
    (0..n)
        .map(|i| {
            format!(
                r#"
codeunit {id} "Release Tests {i}"
{{
    Subtype = Test;

    [Test]
    procedure BuildsACycle()
    var
        H: Codeunit "Release Helper";
    begin
        H.Link(H);
    end;
}}
"#,
                id = 60310 + i
            )
        })
        .collect()
}

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

/// Runs `n` variants (the base variant, then the library seed moved) and notes the instances the process holds when
/// each variant starts and once the run is done.
fn run_variants(n: usize, codeunits: u32) -> (Vec<CorpusRuntime>, Vec<usize>, usize) {
    let src = format!("{APP}{}", tests(codeunits));
    let perturbs: Vec<Perturb> = (0..n)
        .map(|i| Perturb {
            library_seed: (i > 0).then_some(i as i32),
            ..Default::default()
        })
        .collect();
    let mut at_start = Vec::new();
    let rts = CorpusRuntime::try_run_variants(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("App.al".into(), src)],
        }],
        vec![al_sema::AppKind::Workspace],
        &[manifest()],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
        &perturbs,
        |_| at_start.push(al_builtins::instance_cycles::live()),
    )
    .expect("the run starts");
    (rts, at_start, al_builtins::instance_cycles::live())
}

/// One test codeunit and twelve (some of them run on worker threads); in one test, as the count is the process's.
#[test]
fn each_variant_starts_with_no_instance_of_the_variant_before() {
    for codeunits in [1, 12] {
        check(codeunits);
    }
}

fn check(codeunits: u32) {
    let (rts, at_start, at_end) = run_variants(4, codeunits);
    for rt in &rts {
        let outcomes: Vec<String> = rt
            .results()
            .iter()
            .map(|r| format!("{} {:?}", r.test, r.outcome))
            .collect();
        assert_eq!(outcomes, vec!["BuildsACycle Pass".to_string(); codeunits as usize]);
    }
    assert_eq!(
        at_start,
        vec![0; 4],
        "{codeunits} codeunits: instances held when each variant started"
    );
    assert_eq!(at_end, 0, "{codeunits} codeunits: instances held after the run");
}
