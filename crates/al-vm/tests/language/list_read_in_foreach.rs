//! A List read inside a `foreach` over the same list. `IndexOf` there does not count as a change: the corpus calls
//! `Ordinals.IndexOf(Index)` inside `foreach Index in Ordinals` (misc/lt-u-obsolete-pending/UObsoletePending.al:35-36) and
//! passes on real BC. The methods that change a list raise "Collection was modified" (collections/RefSemantics.al:343, :365,
//! :387, :408). Count, Get, Get(i, var), Contains, LastIndexOf and GetRange do not either (collections/RefSemantics.al:566
//! PW_Foreach_ListReadsWhileIterating), also through a by-value List parameter (:594 RW_Foreach_ReadAndAddThroughByValueParam).

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn run(read: &str) -> Result<Value, Signal> {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Integer var L: List of [Integer]; R: List of [Integer]; i: Integer; x: Integer; s: Integer; begin \
        L.Add(10); L.Add(20); L.Add(30); \
        foreach i in L do begin {read} end; \
        exit(s); end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn index_of_inside_a_foreach_over_the_list_runs() {
    match run("s += L.IndexOf(i);") {
        Ok(Value::Integer(n)) => assert_eq!(n, 6),
        other => panic!("{other:?}"),
    }
}

#[test]
fn other_reads_inside_a_foreach_over_the_list_run() {
    for (read, want) in [
        ("s += L.Count();", 9),
        ("s += L.Get(1);", 30),
        ("if L.Get(2, x) then s += x;", 60),
        ("if L.Contains(i) then s += 1;", 3),
        ("s += L.LastIndexOf(i);", 6),
        ("R := L.GetRange(1, 2); s += R.Count();", 6),
    ] {
        match run(read) {
            Ok(Value::Integer(n)) => assert_eq!(n, want, "{read}"),
            other => panic!("{read}: {other:?}"),
        }
    }
}
