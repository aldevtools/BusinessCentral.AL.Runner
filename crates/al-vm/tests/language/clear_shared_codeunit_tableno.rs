//! `ClearAll()` of a store codeunit with a TableNo that another variable shares (`Session::clear_all_in_place`,
//! `Store::TableNoKinds`, assumption clear-all-tableno-register, merged corpus PR 1596
//! ClearSharedCodeunitTableNoRegister.al). The corpus cells themselves run in `tests/neighbors/fix-8964`; these tests hold
//! the cycle, which would reset forever without the once-per-instance walk, and the nesting edges that refuse.

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
codeunit 50920 T {
    procedure Cycle(): Text var Reg: Codeunit TnReg; O: Codeunit TnOwner; Row: Record TnRow;
    begin Reg.Prime(Reg); O.SetReg(Reg); O.Run(Row); exit(O.GetAfter() + ' | ' + Reg.State()); end;
    procedure Deep4(): Text var Reg: Codeunit TnDeep4; O: Codeunit TnDeepOwner; Row: Record TnRow;
    begin Reg.Prime(); O.Set4(Reg); O.Run(Row); exit(Format(Reg.Peek())); end;
    procedure Deep5(): Text var Reg: Codeunit TnDeep5; O: Codeunit TnDeepOwner; Row: Record TnRow;
    begin Reg.Prime(); O.Set5(Reg); O.Run(Row); exit(Format(Reg.Peek())); end;
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
