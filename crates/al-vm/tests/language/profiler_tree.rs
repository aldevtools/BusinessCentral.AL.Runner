//! The call tree the profiler builds for `al-runner test --profile` (`.alcpuprofile` files): one node per call path,
//! call counts, times, statement hits and the record operations per routine ("Database (in memory)" nodes).

use al_testkit::Built;
use al_vm::{ProfileMode, ProfileTree};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const SRC: &str = r#"
table 50700 "Prof Row"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } }
    keys { key(PK; Id) { } }
    trigger OnInsert() begin Touch(); end;
    local procedure Touch() begin end;
}
codeunit 50701 Prof
{
    procedure A()
    begin
        B();
        B();
        C();
    end;
    procedure B()
    var i: Integer;
    begin
        for i := 1 to 3 do
            C();
    end;
    procedure C()
    begin
    end;
    procedure Fails()
    begin
        B();
        Error('boom');
    end;
    procedure Recs()
    var r: Record "Prof Row"; t: Record "Prof Row" temporary;
    begin
        r.Init();
        r.Id := 1;
        r.Insert(true);
        r.Get(1);
        r.Modify();
        r.FindSet();
        r.Next();
        t.Id := 7;
        t.Insert();
    end;
    procedure Spin()
    var i: Integer; d: Decimal;
    begin
        for i := 1 to 20000 do
            d += i / 3;
    end;
}
"#;

fn name(b: &Built, t: &ProfileTree, n: usize) -> String {
    match &t.nodes[n].frame {
        Some(f) => b.lowered.program.objects[f.object].routines[f.routine].name.clone(),
        None => "(root)".into(),
    }
}

fn child(b: &Built, t: &ProfileTree, n: usize, routine: &str) -> usize {
    *t.nodes[n]
        .children
        .iter()
        .find(|&&c| name(b, t, c) == routine)
        .unwrap_or_else(|| panic!("no child {routine} under {}", name(b, t, n)))
}

fn instrumented(routines: &[&str]) -> (Built, ProfileTree) {
    let mut b = Built::from_source(SRC);
    b.session
        .start_profiler(ProfileMode::Instrumentation { statement_times: true });
    for r in routines {
        let _ = b.run_in("Prof", r);
    }
    let t = b.session.take_profile().expect("a profile");
    (b, t)
}

#[test]
fn off_by_default() {
    let mut b = Built::from_source(SRC);
    b.run_in("Prof", "A").unwrap();
    assert!(b.session.take_profile().is_none());
}

#[test]
fn one_node_per_call_path_with_call_counts() {
    let (b, t) = instrumented(&["A"]);
    assert!(t.nodes[0].frame.is_none(), "node 0 is the root");
    let a = child(&b, &t, 0, "A");
    let ab = child(&b, &t, a, "B");
    let ac = child(&b, &t, a, "C");
    let abc = child(&b, &t, ab, "C");
    assert_eq!(t.nodes[a].calls, 1);
    assert_eq!(t.nodes[ab].calls, 2);
    assert_eq!(t.nodes[ac].calls, 1);
    assert_eq!(t.nodes[abc].calls, 6, "A -> B -> C is one path, called 3 times per B");
    assert_eq!(t.nodes[a].children.len(), 2);
    assert_eq!(t.nodes[abc].parent, Some(ab));
}

#[test]
fn times_nest_and_child_time_is_counted() {
    let (b, t) = instrumented(&["A"]);
    let a = child(&b, &t, 0, "A");
    let ab = child(&b, &t, a, "B");
    let ac = child(&b, &t, a, "C");
    let n = |i: usize| &t.nodes[i];
    assert!(n(a).total_ns > 0);
    assert!(n(a).total_ns >= n(ab).total_ns + n(ac).total_ns);
    assert_eq!(n(a).child_ns, n(ab).total_ns + n(ac).total_ns);
    assert_eq!(
        n(0).total_ns,
        n(a).total_ns,
        "the root's total is the sum of its children"
    );
}

#[test]
fn statements_get_hit_counts_and_inclusive_times() {
    let (b, t) = instrumented(&["A"]);
    let a = child(&b, &t, 0, "A");
    let ab = child(&b, &t, a, "B");
    // B's `C();` inside the loop runs 3 times per call of B, twice.
    let program = &b.lowered.program;
    let line_of = |p: u32| program.pos(p).unwrap().line;
    let call_c = t.nodes[ab]
        .positions
        .iter()
        .find(|s| s.hits == 6)
        .unwrap_or_else(|| panic!("no statement hit 6 times: {:?}", t.nodes[ab].positions));
    assert_eq!(line_of(call_c.pos), 21, "the `C();` line of B in SRC (1-based)");
    // A's first statement `B();` takes at least the time of one call of B (inclusive).
    let first = t.nodes[a].positions.iter().min_by_key(|s| s.pos).unwrap();
    assert_eq!(first.hits, 1);
    assert!(first.ns > 0 && first.ns <= t.nodes[a].total_ns);
}

#[test]
fn an_error_unwinds_the_profile_stack() {
    let (b, t) = instrumented(&["Fails", "A"]);
    let f = child(&b, &t, 0, "Fails");
    assert_eq!(t.nodes[f].calls, 1);
    assert_eq!(t.nodes[child(&b, &t, f, "B")].calls, 1);
    // A runs after the error at the top level, not under Fails.
    let a = child(&b, &t, 0, "A");
    assert_eq!(t.nodes[a].parent, Some(0));
}

#[test]
fn record_operations_are_counted_per_routine_and_table() {
    let (b, t) = instrumented(&["Recs"]);
    let r = child(&b, &t, 0, "Recs");
    let db = &t.nodes[r].db;
    let count = |op: &str, temporary: bool| {
        db.iter()
            .filter(|d| format!("{:?}", d.op) == op && d.table == "Prof Row" && d.temporary == temporary)
            .map(|d| d.count)
            .sum::<u64>()
    };
    assert_eq!(count("Insert", false), 1);
    assert_eq!(count("Get", false), 1);
    assert_eq!(count("Modify", false), 1);
    assert_eq!(count("FindSet", false), 1);
    assert_eq!(count("Next", false), 1);
    assert_eq!(count("Insert", true), 1, "a temporary record is counted apart");
    assert!(
        !db.iter().any(|d| format!("{:?}", d.op) == "Init"),
        "Init does not reach the database"
    );
    // The OnInsert trigger runs inside Insert: it is a child node, and its time is not the database's.
    let trig = child(&b, &t, r, "OnInsert");
    assert_eq!(t.nodes[trig].calls, 1);
    let insert = db
        .iter()
        .find(|d| format!("{:?}", d.op) == "Insert" && !d.temporary)
        .unwrap();
    assert!(insert.ns <= t.nodes[r].total_ns - t.nodes[trig].total_ns);
}

#[test]
fn sampling_takes_a_tick_into_the_running_node() {
    let mut b = Built::from_source(SRC);
    let tick = Arc::new(AtomicBool::new(true));
    b.session.start_profiler(ProfileMode::Sampling { tick: tick.clone() });
    b.run_in("Prof", "Spin").unwrap();
    let t = b.session.take_profile().unwrap();
    assert!(t.sampling);
    let s = child(&b, &t, 0, "Spin");
    assert_eq!(t.nodes[s].samples, 1, "one tick, one sample");
    assert_eq!(t.samples, 1);
    assert!(!tick.load(std::sync::atomic::Ordering::Relaxed), "the tick was taken");
    assert_eq!(t.nodes[s].total_ns, 0, "sampling reads no clock per call");
}
