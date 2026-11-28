//! ClearAll in a codeunit with a TableNo of a register tree that another variable shares and no call has made yet (open
//! corpus PR 1650, ClearSharedCodeunitNeverCalledTree.al; assumption clear-all-uncalled-register-tree): the one other
//! holder, a global of a plain codeunit that ran the owner, keeps the instance, flagged. Every other holder and call path
//! refuses, as main does. The neighbor app tests/neighbors/scm-clearall-lost-passes runs the same shapes and their
//! neighbors against a main binary.

use al_builtins::assumptions::{Assumption, take_hits};

use super::clear_shared_codeunit::assert_refused;

/// The shape of 1650 ("CNC Owner" :287-347, "CNC Holder" :349-419), cut down: `Owner`'s OnRun reaches its ClearAll through
/// two local procedures; the plain `H` holds the register and the owner in globals and runs the owner by the statement
/// form. Each other codeunit flips one fact.
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
    var Reg: Codeunit Reg; Mode: Integer;
    trigger OnRun() begin if Mode = 1 then ClearAll() else RunWithCheck(); end;
    local procedure RunWithCheck() begin if Mode = 2 then ClearAll() else ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure SetMode(M: Integer) begin Mode := M; end;
    procedure SetReg(var R: Codeunit Reg) begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}
codeunit 50106 OwnerInternal
{
    TableNo = Row;
    var Reg: Codeunit Reg;
    trigger OnRun() begin RunWithCheck(); end;
    internal procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit Reg) begin Reg := R; end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}
codeunit 50105 H
{
    var Reg: Codeunit Reg; Reg2: Codeunit Reg; O: Codeunit Owner; OI: Codeunit OwnerInternal;
    procedure Post(Mode: Integer): Integer
    var Row: Record Row temporary; LocalO: Codeunit Owner;
    begin
        Row."No." := 1; Row.Insert();
        case Mode of
            0, 1, 2:
                begin O.SetMode(Mode); O.SetReg(Reg); O.Run(Row); end;
            3:
                begin Reg2 := Reg; O.SetReg(Reg); O.Run(Row); end;
            4:
                begin LocalO.SetReg(Reg); LocalO.Run(Row); Reg.SetCnt(5); exit(LocalO.GetCnt()); end;
            5:
                begin OI.SetReg(Reg); OI.Run(Row); Reg.SetCnt(5); exit(OI.GetCnt()); end;
            6:
                begin O.SetReg(Reg); if O.Run(Row) then; end;
        end;
        Reg.SetCnt(5);
        exit(O.GetCnt());
    end;
    local procedure PostLocal(): Integer begin exit(Post(0)); end;
    procedure ViaLocal(): Integer
    var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert();
        exit(RunFromLocal(Row));
    end;
    local procedure RunFromLocal(var Row: Record Row temporary): Integer
    begin
        O.SetReg(Reg); O.Run(Row); Reg.SetCnt(5); exit(O.GetCnt());
    end;
}
codeunit 50107 HTableNo
{
    TableNo = Row;
    var Reg: Codeunit Reg; O: Codeunit Owner;
    procedure Post(): Integer
    var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert();
        O.SetReg(Reg); O.Run(Row); Reg.SetCnt(5); exit(O.GetCnt());
    end;
}
codeunit 50108 T
{
    procedure Measured(): Text var X: Codeunit H; begin exit(Format(X.Post(0))); end;
    procedure ClearAllInOnRun(): Text var X: Codeunit H; begin exit(Format(X.Post(1))); end;
    procedure OneLocalProcedure(): Text var X: Codeunit H; begin exit(Format(X.Post(2))); end;
    procedure TwoHolders(): Text var X: Codeunit H; begin exit(Format(X.Post(3))); end;
    procedure OwnerInALocal(): Text var X: Codeunit H; begin exit(Format(X.Post(4))); end;
    procedure InternalProcedure(): Text var X: Codeunit H; begin exit(Format(X.Post(5))); end;
    procedure GuardedRun(): Text var X: Codeunit H; begin exit(Format(X.Post(6))); end;
    procedure RunFromLocalProcedure(): Text var X: Codeunit H; begin exit(Format(X.ViaLocal())); end;
    procedure HolderWithTableNo(): Text var X: Codeunit HTableNo; begin exit(Format(X.Post())); end;
    procedure HolderIsALocal(): Text
    var R: Codeunit Reg; O: Codeunit Owner; Row: Record Row temporary;
    begin
        O.SetReg(R);
        Row."No." := 1; Row.Insert();
        O.Run(Row);
        R.SetCnt(5);
        exit(Format(O.GetCnt()));
    end;
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

// The measured shape: the owner keeps the shared instance, so the holder's write is read through the owner.
#[test]
fn a_never_called_register_held_by_the_running_holder_stays_shared_flagged() {
    assert_eq!(tree("Measured"), ("5".to_string(), true));
}

// A second other holder: no cell measures it, refused as main.
#[test]
fn a_never_called_register_with_two_other_holders_refuses() {
    let (got, hit) = tree("TwoHolders");
    assert_refused(
        "TwoHolders",
        &got,
        "a holder other than the one variable the corpus measures shares the register",
    );
    assert!(!hit);
}

// Every other holder and call path: refused as main.
#[test]
fn a_never_called_register_in_an_unmeasured_holder_or_path_refuses() {
    for cell in [
        "ClearAllInOnRun",
        "OneLocalProcedure",
        "OwnerInALocal",
        "InternalProcedure",
        "GuardedRun",
        "RunFromLocalProcedure",
        "HolderWithTableNo",
        "HolderIsALocal",
    ] {
        let (got, hit) = tree(cell);
        assert_refused(
            cell,
            &got,
            "the ClearAll runs in a call path or a holder of the register other than the one the corpus measures",
        );
        assert!(!hit, "{cell}");
    }
}
