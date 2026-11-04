//! Fixes found by running Tests-SCM (round 2). Each test names what pins it.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::text;
use al_vm::{Engine, Session};

fn compile(src: &str) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            // `//--file` starts another file (a namespace is declared once per file).
            files: src
                .split("//--file\n")
                .enumerate()
                .map(|(i, f)| (format!("f{i}.al"), f.to_string()))
                .collect(),
        }],
        &Default::default(),
    )
}

fn run(src: &str, routine: &str) -> Result<Value, Signal> {
    let (ws, l) = compile(src);
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn reasons(src: &str) -> Vec<String> {
    let (_, l) = compile(src);
    l.unsupported.iter().flatten().flatten().cloned().collect()
}

#[test]
fn a_dataitem_of_a_namespace_qualified_table_resolves_its_fields() {
    let src = r#"
namespace Microsoft.Test.Forecast;
table 69300 "Fc Entry" { fields { field(1; Name; Code[10]) { } field(2; Qty; Integer) { } } keys { key(PK; Name) { } } }
//--file
namespace Microsoft.Test.Reports;
using Microsoft.Test.Forecast;
report 69301 "Fc Report"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem("Fc Entry"; Microsoft.Test.Forecast."Fc Entry")
        {
            trigger OnPreDataItem() begin SetRange(Name, 'A'); SetRange("Name", 'A'); end;
        }
    }
}
"#;
    let r = reasons(src);
    assert!(r.is_empty(), "{r:?}");
}

#[test]
fn control_without_namespaces() {
    let src = r#"
table 69300 "Fc Entry" { fields { field(1; Name; Code[10]) { } field(2; Qty; Integer) { } } keys { key(PK; Name) { } } }
report 69301 "Fc Report"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem("Fc Entry"; "Fc Entry")
        {
            trigger OnPreDataItem() begin SetRange(Name, 'A'); end;
        }
    }
}
"#;
    let r = reasons(src);
    assert!(r.is_empty(), "{r:?}");
}

const ROWS: &str = r#"
table 69310 "Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69311 C
{
    procedure Steps(): Text
    var R: Record "Row"; Freq: Integer; Days: Decimal; D: Date;
    begin
        for Freq := 1 to 6 do begin R.Id := Freq; R.Insert(); end;
        Freq := 2;
        R.FindFirst();
        R.Next(4 / Freq);
        Days := 6 div 2;
        D := 20240101D;
        exit(Format(R.Id) + '|' + Format(D + Days) + '|' + Format(D - Days));
    end;
    procedure Select(): Text
    var T: Option "Standard Cost","Unit Price";
    begin
        T := T::"Unit Price";
        exit(SelectStr(T, 'zero,one,two'));
    end;
}
"#;

// Next(Decimal) and Date +/- Decimal with a whole number, SelectStr with an Option: the same conversions as an
// assignment to an Integer (ops::convert); a fraction stays refused.
#[test]
fn whole_decimals_step_and_an_option_selects() {
    assert_eq!(text(run(ROWS, "Steps")), "3|01/04/24|12/29/23");
    // SelectStr counts from 1 and the Option member `Unit Price` is ordinal 1.
    assert_eq!(text(run(ROWS, "Select")), "zero");
}

#[test]
fn an_empty_set_filter_removes_the_filter_of_the_field() {
    let src = r#"
table 69310 "Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69311 C
{
    procedure F(): Text
    var R: Record "Row"; s: Text;
    begin
        R.Id := 1; R.Insert(); R.Id := 2; R.Insert();
        R.SetFilter(Id, '>1');
        s := Format(R.Count()) + '|' + R.GetFilters();
        R.SetFilter(Id, '');
        exit(s + '|' + Format(R.Count()) + '|' + R.GetFilters());
    end;
}
"#;
    assert_eq!(text(run(src, "F")), "1|Id: >1|2|");
}

// A table's OnValidate hands `Rec` to a codeunit by var and the codeunit modifies it (Base Application's Item
// Type validation runs ItemCostManagement.UpdateUnitCost, which ends in Item.Modify): the caller's buffer is the
// one that was modified, so its own Modify afterwards is not stale (SMB Service Item TestItem_Reserve).
#[test]
fn a_modify_through_a_var_rec_in_validate_leaves_the_callers_buffer_current() {
    let src = r#"
table 69320 "Doc" { fields { field(1; Id; Integer) { } field(2; Kind; Integer) { trigger OnValidate() var M: Codeunit Mgt; begin M.Touch(Rec); end; } field(3; Cost; Decimal) { } }
  keys { key(PK; Id) { } } }
codeunit 69321 Mgt { procedure Touch(var D: Record Doc) var Check: Record Doc; begin D.Cost := 5; if Check.Get(D.Id) then D.Modify(true); end; }
codeunit 69322 C
{
    procedure F(): Text
    var D: Record Doc;
    begin
        D.Id := 1; D.Insert(true);
        D.Validate(Kind, 3);
        D.Modify(true);
        exit(Format(D.Cost) + '|' + Format(D.Kind));
    end;
}
"#;
    assert_eq!(text(run(src, "F")), "5|3");
}

// Reservation Management has SetReservSource(Variant) and SetReservSource(RecordRef); a RecordRef argument picks the
// RecordRef overload, otherwise the Variant one calls itself with the RecordRef and GetTable fails on it.
#[test]
fn an_overload_taking_the_argument_type_wins_over_a_variant_overload() {
    let src = r#"
table 69330 "Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69331 Mgt
{
    var Seen: Text;
    procedure Take(V: Variant) var RR: RecordRef; begin RR.GetTable(V); Take(RR); end;
    procedure Take(RR: RecordRef) begin Seen := 'recordref ' + Format(RR.Number()); end;
    procedure Get(): Text begin exit(Seen); end;
}
codeunit 69332 C
{
    procedure F(): Text
    var R: Record "Row"; M: Codeunit Mgt;
    begin
        M.Take(R);
        exit(M.Get());
    end;
}
"#;
    assert_eq!(text(run(src, "F")), "recordref 69330");
}

// Base Application's Bin Content.Bin Code relates with `if ("Zone Code" = filter('')) Bin.Code where(...) else
// if ("Zone Code" = filter(<> '')) Bin.Code where(...)`. The arm whose filter the field's value passes applies
// (an assumption for `filter`, the `const` form is pinned by recordref-fieldref/fieldref/TestFieldRefRelation.al; docs/probes-needed.md).
#[test]
fn a_table_relation_with_filter_conditions_picks_the_arm_the_value_passes() {
    let src = r#"
table 69340 "Bin" { fields { field(1; Loc; Code[10]) { } field(2; Zone; Code[10]) { } field(3; Code; Code[10]) { } } keys { key(PK; Loc, Code) { } } }
table 69341 "Line"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Loc; Code[10]) { }
        field(3; Zone; Code[10]) { }
        field(4; BinCode; Code[10])
        {
            TableRelation = if (Zone = filter('')) Bin.Code where(Loc = field(Loc))
                else if (Zone = filter(<> '')) Bin.Code where(Loc = field(Loc), Zone = field(Zone));
        }
    }
    keys { key(PK; Id) { } }
}
codeunit 69342 C
{
    procedure Try(L: Record Line; B: Code[10]): Text
    begin
        if TryValidate(L, B) then exit('ok');
        exit('no');
    end;
    [TryFunction]
    local procedure TryValidate(var L: Record Line; B: Code[10]) begin L.Validate(BinCode, B); end;
    procedure F(): Text
    var Bn: Record Bin; L: Record Line; s: Text;
    begin
        Bn.Loc := 'L1'; Bn.Zone := ''; Bn.Code := 'B1'; Bn.Insert();
        Bn.Loc := 'L1'; Bn.Zone := 'Z1'; Bn.Code := 'B2'; Bn.Insert();
        L.Loc := 'L1';
        L.Zone := '';
        s := Try(L, 'B1') + Try(L, 'B2');
        L.Zone := 'Z1';
        exit(s + '|' + Try(L, 'B2'));
    end;
    procedure G(): Text
    var Bn: Record Bin; L: Record Line;
    begin
        Bn.Loc := 'L1'; Bn.Zone := ''; Bn.Code := 'B1'; Bn.Insert();
        L.Loc := 'L1';
        L.Zone := 'Z1';
        exit(Try(L, 'B9'));
    end;
}
"#;
    // No zone: any bin of the location (B1 and B2 exist). Zone Z1: the bins of that zone only (B2).
    assert_eq!(text(run(src, "F")), "okok|ok");
    // A failing check with several where() conditions is refused: the error code BC answers is not pinned.
    assert!(matches!(
        run(src, "G"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}

// A failing TableRelation check with several where() conditions, none on a primary key field of the related table, answers
// DB:NothingInsideFilter like one condition on a non-key field does (record/table-relation/RelationWhereEmptyTerm.al:173-177);
// the several-condition form is an assumption (table-relation-several-where-non-key). A mix of key and non-key conditions stays refused.
#[test]
fn a_failing_relation_with_several_non_key_conditions_answers_nothing_inside_filter() {
    let src = r#"
table 69345 "Itm" { fields { field(1; No; Code[10]) { } field(2; Blocked; Boolean) { } field(3; SBlocked; Boolean) { } } keys { key(PK; No) { } } }
table 69346 "Ln"
{
    fields { field(1; Id; Integer) { } field(2; ItemNo; Code[10]) { TableRelation = Itm.No where(Blocked = const(false), SBlocked = const(false)); } }
    keys { key(PK; Id) { } }
}
codeunit 69347 C
{
    procedure F(): Text
    var I: Record Itm; L: Record Ln;
    begin
        I.No := 'A'; I.Blocked := true; I.Insert();
        asserterror L.Validate(ItemNo, 'A');
        exit(GetLastErrorCode());
    end;
}
"#;
    assert_eq!(text(run(src, "F")), "DB:NothingInsideFilter");
}

#[test]
fn get_takes_a_record_id() {
    let src = r#"
table 69350 "Cat" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[20]) { } } keys { key(PK; Code) { } } }
codeunit 69351 C
{
    procedure F(): Text
    var A: Record Cat; B: Record Cat; Id: RecordId;
    begin
        A.Code := 'X'; A.Name := 'found'; A.Insert();
        Id := A.RecordId();
        B.Get(Id);
        exit(B.Name);
    end;
}
"#;
    assert_eq!(text(run(src, "F")), "found");
}

// A buffer that wrote its row is current: after Insert, Modify and a Commit its next Modify is not stale
// (record/TestRecordStaleBufferWrite.al pins the buffer older than ANOTHER buffer's committed change).
#[test]
fn a_buffer_that_wrote_the_row_can_modify_it_after_a_commit() {
    let src = r#"
table 69360 "Doc" { fields { field(1; Id; Integer) { } field(2; Kind; Integer) { } field(3; Cost; Decimal) { } }
  keys { key(PK; Id) { } } }
codeunit 69361 Lib { procedure Create(var D: Record Doc) begin D.Id := 1; D.Insert(true); D.Cost := 2; D.Modify(true); end; }
codeunit 69362 C
{
    procedure F(): Text
    var D: Record Doc; L: Codeunit Lib;
    begin
        L.Create(D);
        Commit();
        D.Validate(Kind, 3);
        D.Modify(true);
        D.Validate(Kind, 4);
        D.Modify(true);
        exit(Format(D.Kind));
    end;
}
"#;
    assert_eq!(text(run(src, "F")), "4");
}

// After an asserterror rolled the row back to the last Commit, the buffer that wrote the row meanwhile holds a newer
// row version than the stored row; saving it again is not a stale write (SMB Service Item TestItem_Reserve).
#[test]
fn a_buffer_newer_than_its_rolled_back_row_can_be_saved() {
    let src = r#"
table 69360 "Doc" { fields { field(1; Id; Integer) { } field(2; Kind; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69362 C
{
    procedure F(): Text
    var D: Record Doc;
    begin
        D.Id := 1; D.Insert(true);
        Commit();
        D.Kind := 3; D.Modify(true);
        asserterror Error('boom');
        D.Kind := 4; D.Modify(true);
        exit(Format(D.Kind));
    end;
}
"#;
    assert_eq!(text(run(src, "F")), "4");
}

// A Guid assigns into a Text as its text form, also through a Dictionary of [Text, Text] (Config. Package Management
// sends `Dimensions.Add('PackageCode', Rec.SystemId)`); assumed, docs/probes-needed.md.
#[test]
fn a_guid_assigns_into_a_text_as_its_text_form() {
    let src = r#"
codeunit 69370 C
{
    procedure F(): Text
    var G: Guid; D: Dictionary of [Text, Text]; T: Text[40];
    begin
        Evaluate(G, '63ca2fa4-4f03-4f2b-a480-172fef340d3f');
        T := G;
        D.Add('k', G);
        exit(T + '|' + D.Get('k'));
    end;
}
"#;
    assert_eq!(
        text(run(src, "F")),
        "{63CA2FA4-4F03-4F2B-A480-172FEF340D3F}|{63CA2FA4-4F03-4F2B-A480-172FEF340D3F}"
    );
}
