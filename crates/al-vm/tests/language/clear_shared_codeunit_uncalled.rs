//! ClearAll in a codeunit with a TableNo of a register tree that another variable shares and no call has made yet (open
//! corpus PR 1650, ClearSharedCodeunitNeverCalledTree.al; assumption clear-all-uncalled-register-tree): the one other
//! holder keeps the instance, flagged; a second other holder refuses. The neighbor app
//! tests/neighbors/scm-clearall-lost-passes runs the same shapes and their neighbors against a main binary.

use al_builtins::assumptions::{Assumption, take_hits};

use super::clear_shared_codeunit::assert_refused;

const TREE: &str = r#"
table 50100 Row { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
codeunit 50101 U { SingleInstance = true; var R: Record Row; }
codeunit 50102 A { var U: Codeunit U; }
codeunit 50103 Reg
{
    TableNo = Row;
    var Cnt: Integer; A: Codeunit A;
    procedure SetCnt(V: Integer) begin Cnt := V; end;
    procedure GetCnt(): Integer begin exit(Cnt); end;
}
codeunit 50104 Owner
{
    TableNo = Row;
    var Reg: Codeunit Reg;
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit Reg) begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}
codeunit 50105 T
{
    procedure Post(Two: Boolean): Integer
    var R: Codeunit Reg; R2: Codeunit Reg; O: Codeunit Owner; Row: Record Row temporary;
    begin
        if Two then R2 := R;
        O.SetReg(R);
        Row."No." := 1; Row.Insert();
        O.Run(Row);
        R.SetCnt(5);
        exit(O.GetCnt());
    end;
    procedure One(): Text begin exit(Format(Post(false))); end;
    procedure Two(): Text begin exit(Format(Post(true))); end;
}
"#;

/// What routine `cell` of [`TREE`] gives (its value, or `refused: ...`) and whether it ran a path of assumption
/// clear-all-uncalled-register-tree.
fn tree(cell: &str) -> (String, bool) {
    take_hits();
    let got = match al_testkit::run_with(TREE, "T", cell, vec![]).map(|(v, _)| v) {
        Err(al_builtins::native::Signal::Unsupported(why)) => format!("refused: {why}"),
        other => al_testkit::text(other),
    };
    (got, take_hits().contains(&Assumption::ClearAllUncalledRegisterTree))
}

// One other holder, never called: the owner keeps the shared instance, so the holder's write is read through the owner.
#[test]
fn a_never_called_register_with_one_other_holder_stays_shared_flagged() {
    assert_eq!(tree("One"), ("5".to_string(), true));
}

// A second other holder: no cell measures it, refused as main.
#[test]
fn a_never_called_register_with_two_other_holders_refuses() {
    let (got, hit) = tree("Two");
    assert_refused(
        "Two",
        &got,
        "a holder other than the one variable the corpus measures shares the register",
    );
    assert!(!hit);
}
