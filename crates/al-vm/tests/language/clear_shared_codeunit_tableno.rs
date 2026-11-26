//! `ClearAll()` of a store codeunit with a TableNo that another variable shares (`Session::clear_all_in_place`,
//! `Store::TableNoKinds`, assumption clear-all-tableno-register, merged corpus PR 1596
//! ClearSharedCodeunitTableNoRegister.al). The corpus cells themselves run in `tests/neighbors/fix-8964`; these tests hold
//! the cycle, which would reset forever without the once-per-instance walk, and the nesting, sharing and back-link
//! edges that refuse.

use crate::clear_shared_codeunit::{assert_refused, run_cell};

const SOURCE: &str = "
table 50930 TnRow { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50931 TnCyc { var Back: Codeunit TnReg; I: Integer;
    procedure Link(var R: Codeunit TnReg) begin Back := R; I := 5; end;
    procedure State(): Text begin exit(StrSubstNo('%1/%2', I, Back.Peek())); end; }
codeunit 50932 TnReg { TableNo = TnRow; var Cyc: Codeunit TnCyc; Entry: Integer; Txt: Text;
    trigger OnRun() begin end;
    procedure Prime(var Self: Codeunit TnReg) begin Cyc.Link(Self); Entry := 5; Txt := 'x'; end;
    procedure Peek(): Integer begin exit(Entry); end;
    procedure Bump() begin Entry += 1; end;
    procedure State(): Text begin exit(StrSubstNo('cyc=%1 entry=%2 txt=%3', Cyc.State(), Entry, Txt)); end; }
codeunit 50933 TnOwner { TableNo = TnRow; var Reg: Codeunit TnReg; After: Text;
    trigger OnRun() begin ClearAll(); After := Reg.State(); Reg.Bump(); end;
    procedure SetReg(var R: Codeunit TnReg) begin Reg := R; end;
    procedure GetAfter(): Text begin exit(After); end; }
codeunit 50934 TnL5 { var N: Integer; procedure SetN(V: Integer) begin N := V; end; }
codeunit 50935 TnL4 { var Next: Codeunit TnL5; N: Integer; procedure SetN(V: Integer) begin N := V; Next.SetN(V); end; }
codeunit 50936 TnL3 { var Next: Codeunit TnL4; N: Integer; procedure SetN(V: Integer) begin N := V; Next.SetN(V); end; }
codeunit 50937 TnL2 { var Next: Codeunit TnL3; N: Integer; procedure SetN(V: Integer) begin N := V; Next.SetN(V); end; }
codeunit 50938 TnL1 { var Next: Codeunit TnL2; N: Integer; procedure SetN(V: Integer) begin N := V; Next.SetN(V); end; }
codeunit 50939 TnDeep4 { TableNo = TnRow; var Chain: Codeunit TnL2; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin Chain.SetN(3); Entry := 5; end;
    procedure Peek(): Integer begin exit(Entry); end; }
codeunit 50940 TnDeep5 { TableNo = TnRow; var Chain: Codeunit TnL1; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin Chain.SetN(3); Entry := 5; end;
    procedure Peek(): Integer begin exit(Entry); end; }
codeunit 50941 TnDeepOwner { TableNo = TnRow; var D4: Codeunit TnDeep4; D5: Codeunit TnDeep5;
    trigger OnRun() begin ClearAll(); end;
    procedure Set4(var R: Codeunit TnDeep4) begin D4 := R; end;
    procedure Set5(var R: Codeunit TnDeep5) begin D5 := R; end; }
codeunit 50942 TnByteReg { TableNo = TnRow; var B: Byte; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin Entry := 5; end;
    procedure Peek(): Integer begin exit(Entry); end; }
codeunit 50943 TnByteOwner { TableNo = TnRow; var Reg: Codeunit TnByteReg;
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit TnByteReg) begin Reg := R; end; }
codeunit 50944 TnPlainOwner { var D4: Codeunit TnDeep4; procedure Set4(var R: Codeunit TnDeep4) begin D4 := R; end;
    procedure Go() begin ClearAll(); end; }
codeunit 50945 TnMadeOwner { var Reg: Codeunit TnReg;
    procedure Go(): Text begin Reg.Prime(Reg); ClearAll(); exit(Reg.State()); end; }
codeunit 50946 TnMid { var Leaf: Codeunit TnL5; I: Integer;
    procedure SetN(V: Integer) begin I := V; Leaf.SetN(V); end;
    procedure Share(var L: Codeunit TnL5) begin L := Leaf; end; }
codeunit 50947 TnShareReg { TableNo = TnRow; var Mid: Codeunit TnMid; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin Mid.SetN(3); Entry := 5; end;
    procedure ShareLeaf(var L: Codeunit TnL5) begin Mid.Share(L); end; }
codeunit 50948 TnShareOwner { TableNo = TnRow; var Reg: Codeunit TnShareReg;
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit TnShareReg) begin Reg := R; end; }
codeunit 50920 T {
    procedure Cycle(): Text var Reg: Codeunit TnReg; O: Codeunit TnOwner; Row: Record TnRow;
    begin Reg.Prime(Reg); O.SetReg(Reg); O.Run(Row); exit(O.GetAfter() + ' | ' + Reg.State()); end;
    procedure Deep4(): Text var Reg: Codeunit TnDeep4; O: Codeunit TnDeepOwner; Row: Record TnRow;
    begin Reg.Prime(); O.Set4(Reg); O.Run(Row); exit(Format(Reg.Peek())); end;
    procedure Deep5(): Text var Reg: Codeunit TnDeep5; O: Codeunit TnDeepOwner; Row: Record TnRow;
    begin Reg.Prime(); O.Set5(Reg); O.Run(Row); exit(Format(Reg.Peek())); end;
    procedure OtherBack(): Text var Reg: Codeunit TnReg; Other: Codeunit TnReg; O: Codeunit TnOwner; Row: Record TnRow;
    begin Other.Bump(); Reg.Prime(Other); O.SetReg(Reg); O.Run(Row); exit(Format(Other.Peek())); end;
    procedure DepthTwo(): Text var Reg: Codeunit TnShareReg; L: Codeunit TnL5; O: Codeunit TnShareOwner; Row: Record TnRow;
    begin Reg.Prime(); Reg.ShareLeaf(L); O.SetReg(Reg); O.Run(Row); exit('ran'); end;
    procedure OwnerMade(): Text var O: Codeunit TnMadeOwner; begin exit(O.Go()); end;
    procedure PlainDeep(): Text var Reg: Codeunit TnDeep4; O: Codeunit TnPlainOwner;
    begin Reg.Prime(); O.Set4(Reg); O.Go(); exit(Format(Reg.Peek())); end;
    procedure ByteReg(): Text var Reg: Codeunit TnByteReg; O: Codeunit TnByteOwner; Row: Record TnRow;
    begin Reg.Prime(); O.SetReg(Reg); O.Run(Row); exit(Format(Reg.Peek())); end;
}";

const REFUSED: &str = "ClearAll of a global codeunit variable whose instance another variable shares, in a shape the corpus \
                       does not pin: ";

/// ClearSharedCodeunitTableNoRegister.al CycReg_Primed_OwnerRun :824: the nested Integer reset, the back link kept.
#[test]
fn a_cycle_back_to_the_register_resets_once_and_keeps_the_link() {
    let got = run_cell(SOURCE, "Cycle", vec![]).0;
    assert_eq!(got, "cyc=0/0 entry=0 txt= | cyc=0/1 entry=1 txt=");
}

/// DeepReg_Primed_OwnerRun :767: four levels below the register reset in place.
#[test]
fn a_chain_four_deep_resets() {
    assert_eq!(run_cell(SOURCE, "Deep4", vec![]).0, "0");
}

#[test]
fn a_chain_five_deep_refuses() {
    let got = run_cell(SOURCE, "Deep5", vec![]).0;
    assert_refused("Deep5", &got, REFUSED);
    assert!(got.contains("store \"TnDeep5\""), "{got}");
}

#[test]
fn a_register_global_of_another_kind_refuses_naming_it() {
    let got = run_cell(SOURCE, "ByteReg", vec![]).0;
    assert_refused("ByteReg", &got, REFUSED);
    assert!(got.contains("global \"B\" of \"TnByteReg\""), "{got}");
}

/// A back link holding another register instance is not the measured cycle (CycReg_Primed_OwnerRun :824 links the store).
#[test]
fn a_back_link_to_another_register_refuses() {
    let got = run_cell(SOURCE, "OtherBack", vec![]).0;
    assert_refused("OtherBack", &got, REFUSED);
    assert!(got.contains("a back link holds another register instance"), "{got}");
}

#[test]
fn a_nested_instance_shared_two_levels_down_refuses() {
    let got = run_cell(SOURCE, "DepthTwo", vec![]).0;
    assert_refused("DepthTwo", &got, REFUSED);
    assert!(got.contains("shares one of its nested codeunits"), "{got}");
}

#[test]
fn a_register_only_its_own_back_link_shares_refuses() {
    let got = run_cell(SOURCE, "OwnerMade", vec![]).0;
    assert_refused(
        "OwnerMade",
        &got,
        "ClearAll of a global codeunit variable that only a codeunit nested in it shares",
    );
}

/// The plain owner is measured with "CKT Reg" only (TableNoReg_Primed_PlainOwner :725), three levels deep.
#[test]
fn a_chain_four_deep_under_the_plain_owner_refuses() {
    let got = run_cell(SOURCE, "PlainDeep", vec![]).0;
    assert_refused("PlainDeep", &got, REFUSED);
    assert!(got.contains("three levels deep at most"), "{got}");
}
