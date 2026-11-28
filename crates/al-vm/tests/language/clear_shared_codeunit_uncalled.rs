//! ClearAll in a codeunit with a TableNo of a register tree that another variable shares and no call has made yet (open
//! corpus PR 1650, ClearSharedCodeunitNeverCalledTree.al and ClearSharedCodeunitNeverCalledShapes.al; assumption
//! clear-all-uncalled-register-tree): the one other holder, a global of the codeunit that ran the owner in a measured call
//! path, keeps the instance, flagged. Every other holder and call path refuses, as main does. The neighbor app tests/neighbors/scm-clearall-lost-passes runs the same shapes and their
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
    var Reg: Codeunit Reg; Mode: Integer;
    trigger OnRun() begin if Mode = 1 then ClearAllPublic() else RunWithCheck(); end;
    internal procedure RunWithCheck() begin if Mode = 2 then ClearAll() else ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure ClearAllPublic() begin ClearAllVariables(); end;
    procedure SetMode(M: Integer) begin Mode := M; end;
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
            7, 8:
                begin OI.SetMode(9 - Mode); OI.SetReg(Reg); OI.Run(Row); Reg.SetCnt(5); exit(OI.GetCnt()); end;
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
    var Reg: Codeunit Reg; O: Codeunit Owner; OI: Codeunit OwnerInternal; Got: Integer;
    trigger OnRun() begin Got := Post(); end;
    procedure Post(): Integer
    var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert();
        O.SetReg(Reg); O.Run(Row); Reg.SetCnt(5); exit(O.GetCnt());
    end;
    procedure PostInternal(): Integer
    var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert();
        OI.SetReg(Reg); OI.Run(Row); Reg.SetCnt(5); exit(OI.GetCnt());
    end;
    procedure GetGot(): Integer begin exit(Got); end;
}
// "Whse.-Activity-Post" running "Sales-Post": OnRun -> Locals local procedures -> the owner, a local of the last.
codeunit 50109 HActivity
{
    TableNo = Row;
    var Reg: Codeunit Reg; Locals: Integer; ByVar: Boolean; Got: Integer;
    trigger OnRun() begin if Locals = 2 then Second() else Code(); end;
    local procedure Code() begin if ByVar then PostPassing() else if Locals = 4 then Extra() else Second(); end;
    local procedure Extra() begin Second(); end;
    local procedure Second() begin PostSourceDocument(); end;
    local procedure PostSourceDocument()
    var Row: Record Row temporary; SalesPost: Codeunit OwnerInternal;
    begin
        Row."No." := 1; Row.Insert();
        SalesPost.SetReg(Reg); SalesPost.Run(Row); Reg.SetCnt(5); Got := SalesPost.GetCnt();
    end;
    local procedure PostPassing() var SalesPost: Codeunit OwnerInternal; begin RunPassed(SalesPost); end;
    local procedure RunPassed(var SalesPost: Codeunit OwnerInternal)
    var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert();
        SalesPost.SetReg(Reg); SalesPost.Run(Row); Reg.SetCnt(5); Got := SalesPost.GetCnt();
    end;
    procedure Set(L: Integer; V: Boolean) begin Locals := L; ByVar := V; end;
    procedure GetGot(): Integer begin exit(Got); end;
}
// A plain codeunit between the test and the holder.
codeunit 50110 Mid
{
    var X: Codeunit HActivity;
    procedure Base(): Text var H: Codeunit H; begin exit(Format(H.Post(0))); end;
    procedure Activity(): Text var Row: Record Row temporary; begin X.Set(3, false); X.Run(Row); exit(Format(X.GetGot())); end;
}
codeunit 50111 HLocalPair
{
    var Reg: Codeunit Reg;
    procedure Post(): Integer begin exit(PostSourceDocument()); end;
    local procedure PostSourceDocument(): Integer
    var Row: Record Row temporary; SalesPost: Codeunit Owner;
    begin
        Row."No." := 1; Row.Insert();
        SalesPost.SetReg(Reg); SalesPost.Run(Row); Reg.SetCnt(5); exit(SalesPost.GetCnt());
    end;
}
codeunit 50112 HActivityPublic
{
    TableNo = Row;
    var Reg: Codeunit Reg;
    procedure Post(): Integer begin exit(PostSourceDocument()); end;
    local procedure PostSourceDocument(): Integer
    var Row: Record Row temporary; SalesPost: Codeunit OwnerInternal;
    begin
        Row."No." := 1; Row.Insert();
        SalesPost.SetReg(Reg); SalesPost.Run(Row); Reg.SetCnt(5); exit(SalesPost.GetCnt());
    end;
}
// The owner held by more than the one variable that runs it (review of PR 9382): a parameter copy of the holder's
// global, the test's own variable, or a SingleInstance codeunit's global.
codeunit 50113 HShared
{
    var Reg: Codeunit Reg; O: Codeunit Owner;
    procedure SetOwner(var X: Codeunit Owner) begin O := X; end;
    procedure Post(): Integer var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert();
        O.SetReg(Reg); O.Run(Row); Reg.SetCnt(5); exit(O.GetCnt());
    end;
    procedure PostByVar(): Integer begin O.SetReg(Reg); RunByVar(O); Reg.SetCnt(5); exit(O.GetCnt()); end;
    local procedure RunByVar(var SalesPost: Codeunit Owner) var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert(); SalesPost.Run(Row);
    end;
    procedure PostByValue(): Integer begin O.SetReg(Reg); RunByValue(O); Reg.SetCnt(5); exit(O.GetCnt()); end;
    local procedure RunByValue(SalesPost: Codeunit Owner) var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert(); SalesPost.Run(Row);
    end;
    procedure PostLocalKept(): Integer
    var Row: Record Row temporary; LocalO: Codeunit Owner; K: Codeunit Keeper;
    begin
        Row."No." := 1; Row.Insert();
        LocalO.SetReg(Reg); K.Keep(LocalO); LocalO.Run(Row); Reg.SetCnt(5); exit(LocalO.GetCnt());
    end;
}
codeunit 50114 Keeper
{
    SingleInstance = true;
    var Kept: Codeunit Owner;
    procedure Keep(var X: Codeunit Owner) begin Kept := X; end;
}
// The owner's OnRun runs the chain on another instance of its own codeunit (review of PR 9382): 1650 runs the whole
// chain on the instance run.
codeunit 50115 OwnerOther
{
    TableNo = Row;
    var Reg: Codeunit Reg; Other: Codeunit OwnerOther;
    trigger OnRun() begin Other.RunWithCheck(); end;
    internal procedure RunWithCheck() begin ClearAllVariables(); end;
    local procedure ClearAllVariables() begin ClearAll(); end;
    procedure SetOtherReg(var R: Codeunit Reg) begin Other.SetReg(R); end;
    procedure SetReg(var R: Codeunit Reg) begin Reg := R; end;
    procedure GetOtherCnt(): Integer begin exit(Other.GetCnt()); end;
    procedure GetCnt(): Integer begin exit(Reg.GetCnt()); end;
}
codeunit 50116 HOther
{
    var Reg: Codeunit Reg; O: Codeunit OwnerOther;
    procedure Post(): Integer var Row: Record Row temporary;
    begin
        Row."No." := 1; Row.Insert();
        O.SetOtherReg(Reg); O.Run(Row); Reg.SetCnt(5); exit(O.GetOtherCnt());
    end;
}
codeunit 50108 T
{
    Subtype = Test;
    procedure Measured(): Text var X: Codeunit H; begin exit(Format(X.Post(0))); end;
    procedure ClearAllInOnRun(): Text var X: Codeunit H; begin exit(Format(X.Post(1))); end;
    procedure OneLocalProcedure(): Text var X: Codeunit H; begin exit(Format(X.Post(2))); end;
    procedure TwoHolders(): Text var X: Codeunit H; begin exit(Format(X.Post(3))); end;
    procedure OwnerInALocal(): Text var X: Codeunit H; begin exit(Format(X.Post(4))); end;
    procedure InternalProcedure(): Text var X: Codeunit H; begin exit(Format(X.Post(5))); end;
    procedure GuardedRun(): Text var X: Codeunit H; begin exit(Format(X.Post(6))); end;
    procedure RunFromLocalProcedure(): Text var X: Codeunit H; begin exit(Format(X.ViaLocal())); end;
    procedure HolderWithTableNo(): Text var X: Codeunit HTableNo; begin exit(Format(X.Post())); end;
    procedure HolderWithTableNoRun(): Text
    var X: Codeunit HTableNo; Row: Record Row temporary;
    begin
        X.Run(Row);
        exit(Format(X.GetGot()));
    end;
    procedure HolderWithTableNoGuardedRun(): Text
    var X: Codeunit HTableNo; Row: Record Row temporary;
    begin
        if X.Run(Row) then;
        exit(Format(X.GetGot()));
    end;
    procedure HolderWithTableNoInternal(): Text var X: Codeunit HTableNo; begin exit(Format(X.PostInternal())); end;
    local procedure RunActivity(Locals: Integer; ByVar: Boolean): Text
    var X: Codeunit HActivity; Row: Record Row temporary;
    begin
        X.Set(Locals, ByVar);
        X.Run(Row);
        exit(Format(X.GetGot()));
    end;
    procedure Activity(): Text begin exit(RunActivity(3, false)); end;
    procedure ActivityFourLocals(): Text begin exit(RunActivity(4, false)); end;
    procedure ActivityTwoLocals(): Text begin exit(RunActivity(2, false)); end;
    procedure ActivityOwnerByVar(): Text begin exit(RunActivity(3, true)); end;
    procedure ActivityGuarded(): Text
    var X: Codeunit HActivity; Row: Record Row temporary;
    begin
        X.Set(3, false);
        if X.Run(Row) then;
        exit(Format(X.GetGot()));
    end;
    procedure ActivityByCodeunitId(): Text
    var Row: Record Row temporary;
    begin
        Codeunit.Run(Codeunit::HActivity, Row);
        exit('ran');
    end;
    procedure ActivityFromAPlainCodeunit(): Text var M: Codeunit Mid; begin exit(M.Activity()); end;
    procedure BaseFromAPlainCodeunit(): Text var M: Codeunit Mid; begin exit(M.Base()); end;
    procedure ActivityFromAPublicProcedure(): Text var X: Codeunit HActivityPublic; begin exit(Format(X.Post())); end;
    procedure LocalProcedureAndLocalOwner(): Text var X: Codeunit HLocalPair; begin exit(Format(X.Post())); end;
    procedure InternalWithClearAllInIt(): Text var X: Codeunit H; begin exit(Format(X.Post(7))); end;
    procedure PublicProcedureBeforeClearAll(): Text var X: Codeunit H; begin exit(Format(X.Post(8))); end;
    procedure HolderGlobalRunThroughVarParameter(): Text var X: Codeunit HShared; begin exit(Format(X.PostByVar())); end;
    procedure HolderGlobalRunThroughValueParameter(): Text var X: Codeunit HShared; begin exit(Format(X.PostByValue())); end;
    procedure OwnerAlsoHeldByTheTest(): Text
    var X: Codeunit HShared; O: Codeunit Owner;
    begin
        X.SetOwner(O);
        exit(Format(X.Post()));
    end;
    procedure LocalOwnerAlsoHeldBySingleInstanceGlobal(): Text var X: Codeunit HShared; begin exit(Format(X.PostLocalKept())); end;
    procedure ChainOnAnotherOwnerInstance(): Text var X: Codeunit HOther; begin exit(Format(X.Post())); end;
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

// The measured shapes: the owner keeps the shared instance, so the holder's write is read through the owner.
#[test]
fn a_never_called_register_in_a_measured_call_path_stays_shared_flagged() {
    for cell in [
        "Measured",
        "HolderWithTableNo",
        "HolderWithTableNoRun",
        "RunFromLocalProcedure",
        "OwnerInALocal",
        "InternalProcedure",
        "Activity",
    ] {
        assert_eq!(tree(cell), ("5".to_string(), true), "{cell}");
    }
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

// Every other holder and call path: refused as main. Two facts of the probe without the others, the "Whse.-Activity-Post"
// shape with another chain of local procedures (four is BC's own), a guarded Run or `Codeunit.Run(Codeunit::...)` of the
// holder, a holder run from a codeunit that is no test codeunit, the owner passed by var, and other ClearAll paths.
#[test]
fn a_never_called_register_in_an_unmeasured_holder_or_path_refuses() {
    for cell in [
        "ClearAllInOnRun",
        "OneLocalProcedure",
        "GuardedRun",
        "HolderIsALocal",
        "HolderWithTableNoGuardedRun",
        "HolderWithTableNoInternal",
        "ActivityFourLocals",
        "ActivityTwoLocals",
        "ActivityOwnerByVar",
        "ActivityGuarded",
        "ActivityByCodeunitId",
        "ActivityFromAPlainCodeunit",
        "BaseFromAPlainCodeunit",
        "ActivityFromAPublicProcedure",
        "LocalProcedureAndLocalOwner",
        "InternalWithClearAllInIt",
        "PublicProcedureBeforeClearAll",
        "ChainOnAnotherOwnerInstance",
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

// The owner held by another variable besides the one that runs it: a parameter copy (var or by value) of the holder's
// global, the test's own variable set into the holder, or a SingleInstance codeunit's global beside the running local.
// No cell of 1650 measures them; refused as main.
#[test]
fn a_never_called_register_whose_owner_two_variables_hold_refuses() {
    let wrong: Vec<_> = [
        "HolderGlobalRunThroughVarParameter",
        "HolderGlobalRunThroughValueParameter",
        "OwnerAlsoHeldByTheTest",
        "LocalOwnerAlsoHeldBySingleInstanceGlobal",
    ]
    .into_iter()
    .map(|cell| (cell, tree(cell)))
    .filter(|(_, (got, hit))| *hit || !got.starts_with("refused: "))
    .collect();
    assert!(wrong.is_empty(), "{wrong:?}");
}
