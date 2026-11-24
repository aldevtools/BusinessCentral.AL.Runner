//! `ClearAll()` of the price store of open corpus pull request 1564 (commit 65262a80),
//! tests/al-language/codeunits/clear-instance/ClearSharedCodeunitStoreShapes.al ("Z:" below), measured on BC 29.0
//! (al-runner issue 8912): a temporary record, an Enum, an Integer and a Label with Comment, the store global of a plain
//! owner that implements an interface. Next to it, the shapes the file lists as not measured, which refuse. Every measured
//! cell runs through assumption `clear-shared-codeunit`.

use crate::clear_shared_codeunit::run_cell;

const SOURCE: &str = "
table 50900 PRow { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
enum 50901 PKind { value(0; None) { } value(1; Alpha) { } value(2; Beta) { } }
interface PLine { procedure SetLine(); }
codeunit 50902 PStore { var T1: Record PRow temporary; Kind: Enum PKind; Int: Integer; LblC: Label 'store label', Comment = 'a comment';
    procedure Mark() begin Kind := Kind::Beta; Int := 7; end;
    procedure WriteT1(V: Integer) begin T1.No := V; T1.Insert(); end;
    procedure EmptyT1() begin T1.DeleteAll(); end;
    procedure ShareT1(var R: Record PRow temporary) begin R.Copy(T1, true); end;
    procedure SetInt(V: Integer) begin Int := V; end;
    procedure State(): Text begin exit(StrSubstNo('t1=%1 kind=%2 int=%3 lc=%4', T1.Count(), Kind, Int, LblC)); end; }
codeunit 50903 PPlainLabel { var T1: Record PRow temporary; Kind: Enum PKind; Int: Integer; Lbl: Label 'store label';
    procedure WriteT1(V: Integer) begin T1.No := V; T1.Insert(); end; }
codeunit 50904 PLockedLabel { var T1: Record PRow temporary; Kind: Enum PKind; Int: Integer; Lbl: Label 'store label', Comment = 'c', Locked = true;
    procedure WriteT1(V: Integer) begin T1.No := V; T1.Insert(); end; }
codeunit 50905 PMore { var T1: Record PRow temporary; Kind: Enum PKind; Int: Integer; LblC: Label 'store label', Comment = 'c'; D: Decimal;
    procedure WriteT1(V: Integer) begin T1.No := V; T1.Insert(); end; }
codeunit 50906 POwner implements PLine { var S: Codeunit PStore; Cnt: Integer; LineNo: Integer;
    procedure SetLine() begin ClearAll(); LineNo := 1; end;
    procedure Prepare() begin S.Mark(); S.WriteT1(1); S.WriteT1(2); Cnt := 5; end;
    procedure State(): Text begin exit(S.State() + ' cnt=' + Format(Cnt) + ' line=' + Format(LineNo)); end;
    procedure SetInt(V: Integer) begin S.SetInt(V); end;
    procedure GetStore(var X: Codeunit PStore) begin X := S; end;
    procedure WriteT1(V: Integer) begin S.WriteT1(V); end;
    procedure EmptyT1() begin S.EmptyT1(); end;
    procedure ShareT1(var R: Record PRow temporary) begin S.ShareT1(R); end; }
codeunit 50907 PSiOwner { SingleInstance = true; var S: Codeunit PStore;
    procedure Run() begin S.WriteT1(1); ClearAll(); end; }
codeunit 50908 PTableNoOwner { TableNo = PRow; var S: Codeunit PStore;
    trigger OnRun() begin S.WriteT1(1); ClearAll(); end; }
codeunit 50909 POtherOwner { var A: Codeunit PPlainLabel; B: Codeunit PLockedLabel; C: Codeunit PMore;
    procedure PlainLabel() begin A.WriteT1(1); ClearAll(); end;
    procedure LockedLabel() begin B.WriteT1(1); ClearAll(); end;
    procedure More() begin C.WriteT1(1); ClearAll(); end; }
interface PFace { procedure GetN(): Integer; }
codeunit 50911 PIntStore implements PFace { var N: Integer;
    procedure SetN(V: Integer) begin N := V; end;
    procedure GetN(): Integer begin exit(N); end; }
codeunit 50912 PFaceHolder { var F: Interface PFace;
    procedure Take(NewF: Interface PFace) begin F := NewF; end;
    procedure ClearAllHere() begin ClearAll(); end; }
codeunit 50913 PVariantHolder { var V: Variant;
    procedure Take(X: Codeunit PIntStore) begin V := X; end;
    procedure ClearAllHere() begin ClearAll(); end; }
codeunit 50914 PIntOwner { var S: Codeunit PIntStore;
    procedure SetN(V: Integer) begin S.SetN(V); end;
    procedure GetN(): Integer begin exit(S.GetN()); end;
    procedure ShareFace(var H: Codeunit PFaceHolder) begin H.Take(S); end;
    procedure ShareVariant(var H: Codeunit PVariantHolder) begin H.Take(S); end; }
codeunit 50910 T {
    procedure FaceHolder(): Text var O: Codeunit PIntOwner; H: Codeunit PFaceHolder;
    begin O.SetN(7); O.ShareFace(H); H.ClearAllHere(); exit(Format(O.GetN())); end;
    procedure VariantHolder(): Text var O: Codeunit PIntOwner; H: Codeunit PVariantHolder;
    begin O.SetN(7); O.ShareVariant(H); H.ClearAllHere(); exit(Format(O.GetN())); end;
    procedure ViaInterface(): Text var O: Codeunit POwner; L: Interface PLine; S: Text;
    begin O.Prepare(); L := O; L.SetLine(); S := O.State(); O.WriteT1(10); exit(S + '|' + O.State()); end;
    procedure Direct(): Text var O: Codeunit POwner; S: Text;
    begin O.Prepare(); O.SetLine(); S := O.State(); O.WriteT1(10); exit(S + '|' + O.State()); end;
    procedure Fresh(): Text var O: Codeunit POwner; L: Interface PLine; begin L := O; L.SetLine(); exit(O.State()); end;
    procedure Emptied(): Text var O: Codeunit POwner; L: Interface PLine; S: Text;
    begin O.Prepare(); O.EmptyT1(); L := O; L.SetLine(); S := O.State(); O.WriteT1(10); exit(S + '|' + O.State()); end;
    procedure Twice(): Text var O: Codeunit POwner; L: Interface PLine;
    begin O.Prepare(); L := O; L.SetLine(); O.WriteT1(10); O.SetInt(8); L.SetLine(); exit(O.State()); end;
    procedure Alias(): Text var O: Codeunit POwner; L: Interface PLine; X: Codeunit PStore; S: Text;
    begin O.Prepare(); O.GetStore(X); L := O; L.SetLine(); S := O.State() + '|' + X.State(); X.SetInt(10);
        S += '|' + O.State(); O.SetInt(11); exit(S + '|' + X.State()); end;
    procedure CopyShared(): Text var O: Codeunit POwner; L: Interface PLine; R: Record PRow temporary; S: Text;
    begin O.Prepare(); O.ShareT1(R); L := O; L.SetLine(); S := Format(R.Count()) + '|' + O.State();
        R.No := 30; R.Insert(); S += '|' + O.State(); O.WriteT1(31); exit(S + '|' + Format(R.Count())); end;
    procedure CopySharedEmpty(): Text var O: Codeunit POwner; R: Record PRow temporary;
    begin O.ShareT1(R); O.SetLine(); exit('done'); end;
    procedure AliasNeverWritten(): Text var O: Codeunit POwner; X: Codeunit PStore; begin O.GetStore(X); O.SetLine(); exit('done'); end;
    procedure AliasMarkedNoRow(): Text var O: Codeunit POwner; X: Codeunit PStore;
    begin O.GetStore(X); X.Mark(); O.SetLine(); exit(O.State() + '|' + X.State()); end;
    procedure AliasAndCopy(): Text var O: Codeunit POwner; X: Codeunit PStore; R: Record PRow temporary;
    begin O.Prepare(); O.GetStore(X); O.ShareT1(R); O.SetLine(); exit('done'); end;
    procedure SiOwner(): Text var O: Codeunit PSiOwner; begin O.Run(); exit('done'); end;
    procedure TableNoOwner(): Text var O: Codeunit PTableNoOwner; R: Record PRow; begin O.Run(R); exit('done'); end;
    procedure PlainLabel(): Text var O: Codeunit POtherOwner; begin O.PlainLabel(); exit('done'); end;
    procedure LockedLabel(): Text var O: Codeunit POtherOwner; begin O.LockedLabel(); exit('done'); end;
    procedure More(): Text var O: Codeunit POtherOwner; begin O.More(); exit('done'); end;
}
";

/// What `name` of [`SOURCE`] gives and whether it ran a path of `clear-shared-codeunit` ([`run_cell`]).
fn cell(name: &str) -> (String, bool) {
    run_cell(SOURCE, name, vec![])
}

fn assert_refused(name: &str, why: &str) {
    crate::clear_shared_codeunit::assert_refused(name, &cell(name).0, why);
}

const RESET: &str = "kind=None int=0 lc=store label";

// The price store nothing else holds, rows now, ClearAll in SetLine through an interface (Z:1206) or directly (Z:1225):
// reset in place, T1 keeps its rows, the Enum and the Integer go to their defaults, the Label keeps its text; twice, the
// rows add up (Z:1277).
#[test]
fn clear_all_of_the_price_store_resets_it_in_place() {
    let want = format!("t1=2 {RESET} cnt=0 line=1|t1=3 {RESET} cnt=0 line=1");
    assert_eq!(cell("ViaInterface"), (want.clone(), true));
    assert_eq!(cell("Direct"), (want, true));
    assert_eq!(cell("Twice"), (format!("t1=3 {RESET} cnt=0 line=1"), true));
}

// Never prepared (Z:1242) or T1 emptied before (Z:1257): main's fresh instance reads the same, no mark.
#[test]
fn clear_all_of_an_empty_price_store_keeps_mains_answer() {
    assert_eq!(cell("Fresh"), (format!("t1=0 {RESET} cnt=0 line=1"), false));
    assert_eq!(
        cell("Emptied"),
        (format!("t1=0 {RESET} cnt=0 line=1|t1=1 {RESET} cnt=0 line=1"), false)
    );
}

// An alias of the store taken before (Z:1296) stays on the one reset instance, writes cross; T1 shared by R.Copy(T1, true)
// (Z:1322) stays shared, r=2, writes cross.
#[test]
fn clear_all_of_a_shared_price_store_keeps_one_instance_and_the_shared_table() {
    assert_eq!(
        cell("Alias"),
        (
            format!(
                "t1=2 {RESET} cnt=0 line=1|t1=2 {RESET}|t1=2 kind=None int=10 lc=store label cnt=0 line=1|t1=2 kind=None \
                 int=11 lc=store label"
            ),
            true
        )
    );
    assert_eq!(
        cell("CopyShared"),
        (format!("2|t1=2 {RESET} cnt=0 line=1|t1=3 {RESET} cnt=0 line=1|4"), true)
    );
}

// Shared while T1 holds no row, after a call made the instance, runs flagged (corpus PR 1564 ClearSharedCodeunitHolderKinds.al
// Price_VarParam_Never :1323, ClearSharedCodeunitHolderSlots.al :4702 to :5167: Mark() or SetInt first). With no call
// made, no cell: refused.
#[test]
fn clear_all_of_a_shared_price_store_with_no_row_runs_once_a_call_made_it() {
    assert_eq!(
        cell("AliasMarkedNoRow"),
        (format!("t1=0 {RESET} cnt=0 line=1|t1=0 {RESET}"), true)
    );
    assert_refused("AliasNeverWritten", "whose instance another variable shares");
}

// The shapes Z lists as not measured refuse: shared and Copy-shared at once, Copy-shared while T1 holds no row, under
// a SingleInstance or a TableNo owner, a store whose Label has no Comment or is Locked, or that has one more global.
#[test]
fn clear_all_of_a_price_store_in_an_unmeasured_shape_refuses() {
    assert_refused("AliasAndCopy", "a temporary table that another record shares");
    assert_refused("CopySharedEmpty", "a temporary table that another record shares");
    for name in ["SiOwner", "TableNoOwner", "PlainLabel", "LockedLabel", "More"] {
        assert_refused(name, "whose instance holds temporary rows");
    }
}

// ClearAll in a codeunit whose interface global shares a store instance: Z:2001 measures it for another store only (BC
// resets the instance in place), so it refuses; main cleared the interface and left the instance as it was. A Variant
// global that holds the shared instance keeps main's answer, with no mark.
#[test]
fn clear_all_of_an_interface_or_variant_global_sharing_a_store() {
    assert_refused(
        "FaceHolder",
        "ClearAll of a global interface variable whose codeunit instance another variable shares",
    );
    assert_eq!(cell("VariantHolder"), ("7".to_string(), false));
}
