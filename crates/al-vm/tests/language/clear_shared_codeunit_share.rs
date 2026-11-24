//! `ClearAll()` cells of open corpus pull request 1564 (commit d8215564), tests/al-language/codeunits/clear-instance/,
//! measured on BC 29.0 (al-runner issue 8912): the register with globals of mixed kinds that a setter shares
//! (ClearSharedCodeunitSetterOwner.al, "S:" below), the Label store shared through an interface or a by-value parameter
//! (ClearSharedCodeunitLabelShare.al, "L:"), arrays of temporary records shared by `Copy(x, true)` and arrays of two
//! dimensions (ClearSharedCodeunitTempArrayShare.al, "T:"). Next to each, the shapes the file lists as not measured,
//! which refuse. Every measured cell runs through assumption `clear-shared-codeunit`.

use al_builtins::assumptions::{Assumption, take_hits};

const SOURCE: &str = "
table 50880 Row { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50881 RNested { var N: Integer; procedure Set(V: Integer) begin N := V; end; procedure Get(): Integer begin exit(N); end; }
codeunit 50882 RReg { TableNo = Row;
    var Rows: Record Row temporary; Rec: Record Row; Dec: Decimal; Flag: Boolean; Cd: Code[20]; Nested: Codeunit RNested;
    trigger OnRun() begin Cd := 'RUN'; end;
    procedure Mark() begin Dec := 1.5; Flag := true; Cd := 'MARK'; Rec.SetRange(No, 7); Rec.No := 9; Nested.Set(4); end;
    procedure WriteRow(V: Integer) begin Rows.Init(); Rows.No := V; Rows.Insert(); end;
    procedure EmptyRows() begin Rows.DeleteAll(); end;
    procedure ReadRec() begin if Rec.FindFirst() then; end;
    procedure ShareRows(var R: Record Row temporary) begin R.Copy(Rows, true); end;
    procedure State(): Text begin exit(StrSubstNo('rows=%1 dec=%2 flag=%3 code=%4 recfilter=%5 recno=%6 nested=%7',
        Rows.Count(), Format(Dec, 0, 9), Flag, Cd, Rec.GetFilters(), Rec.No, Nested.Get())); end; }
codeunit 50883 RRegText { TableNo = Row;
    var Rows: Record Row temporary; Rec: Record Row; Dec: Decimal; Flag: Boolean; Cd: Code[20]; Nested: Codeunit RNested; T: Text;
    procedure WriteRow(V: Integer) begin Rows.No := V; Rows.Insert(); end; }
codeunit 50884 RTableNoOwner { TableNo = Row; var Reg: Codeunit RReg; Cnt: Integer;
    trigger OnRun() begin ClearAll(); end;
    procedure SetRegister(var NewReg: Codeunit RReg) begin Reg := NewReg; end;
    procedure SetRegisterByValue(NewReg: Codeunit RReg) begin Reg := NewReg; end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure StateReg(): Text begin exit(Reg.State() + ' cnt=' + Format(Cnt)); end;
    procedure WriteReg(V: Integer) begin Reg.WriteRow(V); end;
    procedure MarkReg() begin Reg.Mark(); end;
    procedure SetCnt(V: Integer) begin Cnt := V; end; }
codeunit 50885 RPlainOwner { var Reg: Codeunit RReg; Other: Codeunit RRegText;
    trigger OnRun() begin ClearAll(); end;
    procedure SetRegister(var NewReg: Codeunit RReg) begin Reg := NewReg; end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure StateReg(): Text begin exit(Reg.State()); end;
    procedure WriteReg(V: Integer) begin Reg.WriteRow(V); end;
    procedure MarkReg() begin Reg.Mark(); end;
    procedure EmptyReg() begin Reg.EmptyRows(); end;
    procedure ShareReg(var R: Record Row temporary) begin Reg.ShareRows(R); end;
    procedure WriteOther() begin Other.WriteRow(1); end; }
codeunit 50886 RSiOwner { SingleInstance = true; var Reg: Codeunit RReg;
    procedure SetRegister(var NewReg: Codeunit RReg) begin Reg := NewReg; end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50887 RNormalOwner { Subtype = Normal; var Reg: Codeunit RReg;
    procedure WriteAndClear(): Text begin Reg.WriteRow(1); ClearAll(); exit('done'); end; }
codeunit 50888 RCaller { TableNo = Row; var Reg: Codeunit RReg; TOwner: Codeunit RTableNoOwner; POwner: Codeunit RPlainOwner;
    procedure Prepare(Rows: Integer; Kind: Integer; ByValue: Boolean)
    var I: Integer;
    begin Reg.Mark(); for I := 1 to Rows do Reg.WriteRow(I);
        if Kind in [1, 2] then begin if ByValue then TOwner.SetRegisterByValue(Reg) else TOwner.SetRegister(Reg); TOwner.SetCnt(5); end
        else POwner.SetRegister(Reg); end;
    procedure ShareRows(var R: Record Row temporary) begin Reg.ShareRows(R); end;
    procedure EmptyReg() begin Reg.EmptyRows(); end;
    procedure ClearOwner(Kind: Integer) var Row: Record Row;
    begin case Kind of 1: TOwner.Run(Row); 2: TOwner.ClearAllGlobals(); 3: POwner.Run(); 4: POwner.ClearAllGlobals(); end; end;
    procedure CallerState(): Text begin exit(Reg.State()); end;
    procedure OwnerState(Kind: Integer): Text begin if Kind in [1, 2] then exit(TOwner.StateReg()); exit(POwner.StateReg()); end;
    procedure WriteReg(V: Integer) begin Reg.WriteRow(V); end;
    procedure OwnerWrite(Kind: Integer; V: Integer) begin if Kind in [1, 2] then TOwner.WriteReg(V) else POwner.WriteReg(V); end;
    procedure ReadRec() begin Reg.ReadRec(); end;
    procedure HandToSi(var S: Codeunit RSiOwner) begin S.SetRegister(Reg); end; }
interface ILabel { procedure Write(V: Integer); procedure State(): Text; }
codeunit 50890 LStore implements ILabel { var Rows: Record Row temporary; Refs: array[2, 3] of RecordRef; Ints: array[3] of Integer; L: Label 'kept label text';
    procedure Write(V: Integer) begin Rows.No := V; Rows.Insert(); Ints[1] := V; end;
    procedure State(): Text begin exit(StrSubstNo('rows=%1 int1=%2 label=%3', Rows.Count(), Ints[1], L)); end; }
codeunit 50891 LHolder { var X: Codeunit LStore;
    procedure WriteX(V: Integer) begin X.Write(V); end;
    procedure Hand(var I: Interface ILabel) begin I := X; end;
    procedure StateX(): Text begin exit(X.State()); end;
    procedure ClearAllGlobals() begin ClearAll(); end;
    procedure ValueParam(): Text begin exit(ClearThenWrite(X)); end;
    procedure ValueParamOuter(): Text begin exit(Outer(X)); end;
    local procedure Outer(P: Codeunit LStore): Text begin ClearAll(); P.Write(5); exit(X.State()); end;
    local procedure ClearThenWrite(P: Codeunit LStore): Text begin ClearAll(); P.Write(5); exit(X.State()); end; }
codeunit 50892 AOwner { var A: array[2] of Record Row temporary; B: array[2, 2] of Record Row temporary; C: array[2, 2, 2] of Record Row temporary;
    procedure ShareA1(var R: Record Row temporary) begin R.Copy(A[1], true); end;
    procedure ShareB11(var R: Record Row temporary) begin R.Copy(B[1, 1], true); end;
    procedure WriteA(V: Integer) begin A[1].No := V; A[1].Insert(); end;
    procedure WriteB(V: Integer) begin B[1, 1].No := V; B[1, 1].Insert(); end;
    procedure WriteC(V: Integer) begin C[1, 1, 1].No := V; C[1, 1, 1].Insert(); end;
    procedure State(): Text begin exit(StrSubstNo('a1=%1 a2=%2 b11=%3 b22=%4', A[1].Count(), A[2].Count(), B[1, 1].Count(), B[2, 2].Count())); end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50893 ASiOwner { SingleInstance = true; var A: array[2] of Record Row temporary; B: array[2, 2] of Record Row temporary;
    procedure ShareA1(var R: Record Row temporary) begin R.Copy(A[1], true); end;
    procedure WriteA(V: Integer) begin A[1].No := V; A[1].Insert(); end;
    procedure WriteB(V: Integer) begin B[1, 1].No := V; B[1, 1].Insert(); end;
    procedure State(): Text begin exit(StrSubstNo('a1=%1 a2=%2 b11=%3 b22=%4', A[1].Count(), A[2].Count(), B[1, 1].Count(), B[2, 2].Count())); end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50894 ANormalOwner { Subtype = Normal; var A: array[2] of Record Row temporary;
    procedure Run(): Text var R: Record Row temporary; begin R.Copy(A[1], true); ClearAll(); exit('done'); end; }
codeunit 50895 ATestOwner { Subtype = Test; var B: array[2, 2] of Record Row temporary;
    procedure Run(): Text begin B[1, 1].No := 1; B[1, 1].Insert(); ClearAll(); exit('done'); end; }
codeunit 50896 B2Store { var B: array[2, 2] of Record Row temporary; Ints: array[2] of Integer;
    procedure Write(V: Integer) begin B[1, 1].No := V; B[1, 1].Insert(); Ints[1] := V; end;
    procedure State(): Text begin exit(StrSubstNo('b11=%1 b22=%2 int1=%3', B[1, 1].Count(), B[2, 2].Count(), Ints[1])); end; }
codeunit 50897 B2Holder { var X: Codeunit B2Store;
    procedure WriteX(V: Integer) begin X.Write(V); end;
    procedure Share(var Y: Codeunit B2Store) begin Y := X; end;
    procedure StateX(): Text begin exit(X.State()); end;
    procedure ClearAllGlobals() begin ClearAll(); end; }
codeunit 50899 T {
    procedure RegShared(Rows: Integer; Kind: Integer; ByValue: Boolean): Text var C: Codeunit RCaller; S: Text;
    begin C.Prepare(Rows, Kind, ByValue); C.ClearOwner(Kind); S := C.CallerState() + '|' + C.OwnerState(Kind);
        C.WriteReg(11); S += '|' + C.OwnerState(Kind); C.OwnerWrite(Kind, 12); exit(S + '|' + C.CallerState()); end;
    procedure RegSharedTableNoRun(): Text begin exit(RegShared(2, 1, false)); end;
    procedure RegSharedTableNoProc(): Text begin exit(RegShared(0, 2, false)); end;
    procedure RegSharedPlainRun(): Text begin exit(RegShared(2, 3, false)); end;
    procedure RegSharedByValue(): Text begin exit(RegShared(2, 1, true)); end;
    procedure RegSharedEmptied(): Text var C: Codeunit RCaller;
    begin C.Prepare(1, 4, false); C.EmptyReg(); C.ClearOwner(4); exit(C.CallerState()); end;
    procedure RegSharedCopy(): Text var C: Codeunit RCaller; R: Record Row temporary; S: Text;
    begin C.Prepare(2, 4, false); C.ShareRows(R); C.ClearOwner(4); S := C.CallerState() + '|' + Format(R.Count());
        R.No := 13; R.Insert(); exit(S + '|' + C.CallerState()); end;
    procedure RegUnsharedRows(): Text var O: Codeunit RPlainOwner;
    begin O.MarkReg(); O.WriteReg(1); O.WriteReg(2); O.ClearAllGlobals(); exit(O.StateReg()); end;
    procedure RegUnsharedEmptied(): Text var O: Codeunit RPlainOwner;
    begin O.MarkReg(); O.WriteReg(1); O.EmptyReg(); O.ClearAllGlobals(); exit(O.StateReg()); end;
    procedure RegUnsharedCopy(): Text var O: Codeunit RPlainOwner; R: Record Row temporary;
    begin O.WriteReg(1); O.ShareReg(R); O.ClearAllGlobals(); exit('done'); end;
    procedure RegSiOwner(): Text var C: Codeunit RCaller; S: Codeunit RSiOwner;
    begin C.WriteReg(1); C.HandToSi(S); S.ClearAllGlobals(); exit('done'); end;
    procedure RegNormalOwner(): Text var C: Codeunit RNormalOwner; begin exit(C.WriteAndClear()); end;
    procedure RegRecRead(): Text var C: Codeunit RCaller; Stored: Record Row;
    begin Stored.No := 7; Stored.Insert(); C.Prepare(1, 4, false); C.ReadRec(); C.ClearOwner(4); exit('done'); end;
    procedure RegOtherGlobal(): Text var O: Codeunit RPlainOwner; begin O.WriteOther(); O.ClearAllGlobals(); exit('done'); end;
    procedure LabelIface(): Text var H: Codeunit LHolder; I: Interface ILabel; S: Text;
    begin H.WriteX(3); H.Hand(I); H.ClearAllGlobals(); S := I.State() + '|' + H.StateX(); I.Write(5); exit(S + '|' + H.StateX()); end;
    procedure LabelIfaceUnwritten(): Text var H: Codeunit LHolder; I: Interface ILabel;
    begin H.Hand(I); H.ClearAllGlobals(); exit('done'); end;
    procedure LabelValueParamWritten(): Text var H: Codeunit LHolder; begin H.WriteX(3); exit(H.ValueParam()); end;
    procedure LabelValueParamOuterWritten(): Text var H: Codeunit LHolder; begin H.WriteX(3); exit(H.ValueParamOuter()); end;
    procedure ArrShared(Written: Boolean): Text var O: Codeunit AOwner; R: Record Row temporary; S: Text;
    begin if Written then O.WriteA(1); O.ShareA1(R); O.ClearAllGlobals(); S := O.State() + '|' + Format(R.Count());
        R.No := 10; R.Insert(); exit(S + '|' + O.State()); end;
    procedure ArrNeverShared(): Text begin exit(ArrShared(false)); end;
    procedure ArrWrittenShared(): Text begin exit(ArrShared(true)); end;
    procedure SiArrShared(Written: Boolean): Text var O: Codeunit ASiOwner; R: Record Row temporary; S: Text;
    begin if Written then O.WriteA(1); O.ShareA1(R); O.ClearAllGlobals(); S := O.State() + '|' + Format(R.Count());
        R.No := 10; R.Insert(); exit(S + '|' + O.State()); end;
    procedure SiArrNeverShared(): Text begin exit(SiArrShared(false)); end;
    procedure SiArrWrittenShared(): Text begin exit(SiArrShared(true)); end;
    procedure ArrNormalShared(): Text var O: Codeunit ANormalOwner; begin exit(O.Run()); end;
    procedure TwoDimWritten(): Text var O: Codeunit AOwner; begin O.WriteB(1); O.ClearAllGlobals(); O.WriteB(2); exit(O.State()); end;
    procedure TwoDimNever(): Text var O: Codeunit AOwner; begin O.ClearAllGlobals(); exit(O.State()); end;
    procedure SiTwoDimWritten(): Text var O: Codeunit ASiOwner; begin O.WriteB(1); O.ClearAllGlobals(); exit(O.State()); end;
    procedure TwoDimShared(): Text var O: Codeunit AOwner; R: Record Row temporary;
    begin O.WriteB(1); O.ShareB11(R); O.ClearAllGlobals(); exit('done'); end;
    procedure ThreeDimWritten(): Text var O: Codeunit AOwner; begin O.WriteC(1); O.ClearAllGlobals(); exit('done'); end;
    procedure TwoDimTestOwner(): Text var O: Codeunit ATestOwner; begin exit(O.Run()); end;
    procedure B2InStore(): Text var H: Codeunit B2Holder; begin H.WriteX(1); H.ClearAllGlobals(); exit(H.StateX()); end;
    procedure B2InStoreAlias(): Text var H: Codeunit B2Holder; Y: Codeunit B2Store; S: Text;
    begin H.Share(Y); Y.Write(1); H.ClearAllGlobals(); S := Y.State() + '|' + H.StateX(); Y.Write(2); exit(S + '|' + H.StateX()); end;
}
";

/// What `cell` of [`SOURCE`] gives (its text, or `refused: ...`) and whether it ran a path of `clear-shared-codeunit`.
fn cell(cell: &str) -> (String, bool) {
    take_hits();
    let got = match al_testkit::Built::from_source(SOURCE).run_in("T", cell) {
        Err(al_builtins::native::Signal::Unsupported(why)) => format!("refused: {why}"),
        other => al_testkit::text(other),
    };
    (got, take_hits().contains(&Assumption::ClearSharedCodeunit))
}

fn assert_refused(name: &str, why: &str) {
    let got = cell(name).0;
    assert!(got.starts_with("refused: ") && got.contains(why), "{name}: {got}");
}

const RESET: &str = "dec=0 flag=No code= recfilter= recno=0 nested=0";

// The register a setter shares, ClearAll in a TableNo or a plain owner, in OnRun or a procedure, setter var or by value:
// one instance, reset in place; the scalars, the record that is not temporary and the nested codeunit go back, the
// temporary table keeps what it holds now (S:318 TableNo Run Rows, S:562 TableNo procedure Never, S:698 plain Run Rows,
// S:1078 by-value setter Rows, S:915 plain procedure Emptied). A write through either side shows through the other.
#[test]
fn clear_all_of_a_shared_register_resets_it_in_place() {
    let cnt = |rows: u8, s: &str| format!("rows={rows} {RESET}{s}");
    for (name, rows, owner_cnt) in [
        ("RegSharedTableNoRun", 2, " cnt=0"),
        ("RegSharedTableNoProc", 0, " cnt=0"),
        ("RegSharedPlainRun", 2, ""),
        ("RegSharedByValue", 2, " cnt=0"),
    ] {
        let want = format!(
            "{}|{}|{}|{}",
            cnt(rows, ""),
            cnt(rows, owner_cnt),
            cnt(rows + 1, owner_cnt),
            cnt(rows + 2, "")
        );
        assert_eq!(cell(name), (want, true), "{name}");
    }
    assert_eq!(cell("RegSharedEmptied"), (format!("rows=0 {RESET}"), true));
}

// A table the register shares with a record of the test through Copy(Rows, true) stays shared: the record reads the kept
// rows, an insert through it shows through the register (S:888 ... S:1040, SharedRows :987 in the plain owner).
#[test]
fn clear_all_of_a_shared_register_keeps_its_copy_shared_table() {
    assert_eq!(
        cell("RegSharedCopy"),
        (format!("rows=2 {RESET}|2|rows=3 {RESET}"), true)
    );
}

// The owner's own register, nothing sharing it: rows kept, the marks cleared (S:1208 plain Rows); written then emptied
// reads 0 (S:1229), main's fresh instance reads the same, with no mark.
#[test]
fn clear_all_of_an_unshared_register_keeps_its_rows() {
    assert_eq!(cell("RegUnsharedRows"), (format!("rows=2 {RESET}"), true));
    assert_eq!(cell("RegUnsharedEmptied"), (format!("rows=0 {RESET}"), false));
}

// The register shapes ClearSharedCodeunitSetterOwner.al lists as not measured refuse: a SingleInstance or Subtype = Normal
// owner, a record that is not temporary that read a row, a register with one more global, and an unshared register whose
// table a record outside shares.
#[test]
fn clear_all_of_a_register_in_an_unmeasured_shape_refuses() {
    assert_refused("RegSiOwner", "whose instance another variable shares");
    assert_refused("RegNormalOwner", "whose instance holds temporary rows");
    assert_refused("RegRecRead", "whose instance another variable shares");
    assert_refused("RegOtherGlobal", "whose instance holds temporary rows");
    assert_refused("RegUnsharedCopy", "a temporary table that another record shares");
}

// The Label store written once, shared through an interface (L:451) or a by-value parameter of the clearing routine
// (L:503) or of its caller (L:519): reset in place, rows kept, the Label keeps its text, a write through the sharer shows
// through X. Never written, the holder kind is not measured (L:451 to L:783 all write X first): refused.
#[test]
fn clear_all_of_a_label_store_shared_by_interface_or_value_resets_it_in_place() {
    let reset = "rows=1 int1=0 label=kept label text";
    assert_eq!(
        cell("LabelIface"),
        (format!("{reset}|{reset}|rows=2 int1=5 label=kept label text"), true)
    );
    for name in ["LabelValueParamWritten", "LabelValueParamOuterWritten"] {
        assert_eq!(
            cell(name),
            ("rows=2 int1=5 label=kept label text".to_string(), true),
            "{name}"
        );
    }
    assert_refused("LabelIfaceUnwritten", "whose instance another variable shares");
}

// An array of temporary records whose element 1 a record of the test shares through Copy(A[1], true), never written or
// written once, keeps the shared table after ClearAll in a plain (T:519, T:555) and a SingleInstance owner (T:736, T:772):
// the record reads the kept rows, an insert through it shows through both elements.
#[test]
fn clear_all_keeps_the_copy_shared_table_of_an_array() {
    for (name, rows) in [
        ("ArrNeverShared", 0),
        ("ArrWrittenShared", 1),
        ("SiArrNeverShared", 0),
        ("SiArrWrittenShared", 1),
    ] {
        let after = rows + 1;
        let want = format!("a1={rows} a2={rows} b11=0 b22=0|{rows}|a1={after} a2={after} b11=0 b22=0");
        assert_eq!(cell(name), (want, true), "{name}");
    }
    assert_refused("ArrNormalShared", "a temporary table that another record shares");
}

// An array[2, 2] of temporary records keeps its rows after ClearAll as a global of a plain owner (T:655, next write 2) and
// of a SingleInstance owner (T:872), and inside a store with an array of Integer under a plain holder (T:688), with an
// alias that wrote before on the same instance (T:707). Never written it reads 0 (T:673), main's answer with no mark.
#[test]
fn clear_all_keeps_the_rows_of_a_two_dimensional_array() {
    assert_eq!(cell("TwoDimWritten"), ("a1=0 a2=0 b11=2 b22=2".to_string(), true));
    assert_eq!(cell("TwoDimNever"), ("a1=0 a2=0 b11=0 b22=0".to_string(), false));
    assert_eq!(cell("SiTwoDimWritten"), ("a1=0 a2=0 b11=1 b22=1".to_string(), true));
    assert_eq!(cell("B2InStore"), ("b11=1 b22=1 int1=0".to_string(), true));
    assert_eq!(
        cell("B2InStoreAlias"),
        (
            "b11=1 b22=1 int1=0|b11=1 b22=1 int1=0|b11=2 b22=2 int1=2".to_string(),
            true
        )
    );
}

// The array shapes ClearSharedCodeunitTempArrayShare.al lists as not measured refuse: an array of two dimensions shared by
// Copy, one of three dimensions, one of two dimensions in a test codeunit.
#[test]
fn clear_all_of_an_array_in_an_unmeasured_shape_refuses() {
    assert_refused("TwoDimShared", "a temporary table that another record shares");
    assert_refused("ThreeDimWritten", "array of temporary records whose table holds rows");
    assert_refused("TwoDimTestOwner", "array of temporary records whose table holds rows");
}
