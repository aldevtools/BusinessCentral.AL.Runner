//! `ClearAll()` of a TableNo register that holds SingleInstance codeunits (`Session::clear_all_in_place`,
//! `Store::TableNoSingle`, assumption clear-all-register-single-instance, merged corpus PR 1599
//! ClearSharedCodeunitNestedSingleInstance.al). The corpus cells run in `tests/neighbors/fix-8964-single`; these tests hold
//! the refusals the run-time checks give: the register's SingleInstance variable shared, a nested record that read a row.

use crate::clear_shared_codeunit::{assert_refused, run_cell};

const SOURCE: &str = "
table 50960 SiRow { fields { field(1; No; Integer) { } } keys { key(PK; No) { } } }
codeunit 50961 SiLeaf { SingleInstance = true; var N: Integer;
    procedure SetN(V: Integer) begin N := V; end;
    procedure State(): Text begin exit(Format(N)); end; }
codeunit 50962 SiReg { TableNo = SiRow; var Loc: Record SiRow; Child: Codeunit SiLeaf; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime(V: Integer) begin Loc.No := 1; Child.SetN(V); Entry := 5; end;
    procedure GetChild(var C: Codeunit SiLeaf) begin C := Child; end;
    procedure State(): Text begin exit(StrSubstNo('loc=%1 child=%2 entry=%3', Loc.No, Child.State(), Entry)); end; }
codeunit 50963 SiOwner { TableNo = SiRow; var Reg: Codeunit SiReg;
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit SiReg) begin Reg := R; end; }
codeunit 50964 SiRead { var Cached: Record SiRow; N: Integer;
    procedure SetN(V: Integer) begin N := V; if Cached.FindFirst() then; end;
    procedure State(): Text begin exit(Format(N)); end; }
codeunit 50965 SiReadReg { TableNo = SiRow; var Child: Codeunit SiRead; Inner: Codeunit SiLeaf; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime(V: Integer) begin Child.SetN(V); Inner.SetN(V); Entry := 5; end; }
codeunit 50966 SiReadOwner { TableNo = SiRow; var Reg: Codeunit SiReadReg;
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit SiReadReg) begin Reg := R; end; }
codeunit 50920 T {
    procedure Kept(): Text var Reg: Codeunit SiReg; O: Codeunit SiOwner; Ext: Codeunit SiLeaf; Row: Record SiRow;
    begin Reg.Prime(10); O.SetReg(Reg); O.Run(Row); exit(Reg.State() + ' ext=' + Ext.State()); end;
    procedure HandedOut(): Text var Reg: Codeunit SiReg; O: Codeunit SiOwner; C: Codeunit SiLeaf; Row: Record SiRow;
    begin Reg.Prime(10); Reg.GetChild(C); O.SetReg(Reg); O.Run(Row); exit(Reg.State()); end;
    procedure RowRead(): Text var Reg: Codeunit SiReadReg; O: Codeunit SiReadOwner; Row: Record SiRow;
    begin Row.No := 1; Row.Insert(); Reg.Prime(10); O.SetReg(Reg); O.Run(Row); exit('ran'); end;
}";

const REFUSED: &str = "ClearAll of a global codeunit variable whose instance another variable shares, in a shape the corpus \
                       does not pin: ";

/// D1P_Written_OwnerClearAll :1933: the register's globals reset, its SingleInstance codeunit keeps 10.
#[test]
fn a_single_instance_codeunit_in_the_register_keeps_its_state() {
    assert_eq!(run_cell(SOURCE, "Kept", vec![]).0, "loc=0 child=10 entry=0 ext=10");
}

/// 1599 never shares the register's SingleInstance variable itself.
#[test]
fn the_register_single_instance_variable_handed_out_refuses() {
    let got = run_cell(SOURCE, "HandedOut", vec![]).0;
    assert_refused("HandedOut", &got, REFUSED);
    assert!(got.contains("shares one of its nested"), "{got}");
}

/// 1599 nested codeunits set field values of their records and read no row ("CKS Trk U" `SetN` :1377-1385).
#[test]
fn a_nested_record_that_read_a_row_refuses() {
    let got = run_cell(SOURCE, "RowRead", vec![]).0;
    assert_refused("RowRead", &got, REFUSED);
    assert!(got.contains("read or wrote a row"), "{got}");
}
