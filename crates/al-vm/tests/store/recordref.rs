//! RecordRef / FieldRef / KeyRef end to end: AL source → compile → run.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{int, text};
use al_vm::{Engine, Session};

const TABLE: &str = r#"
enum 50901 "ALT E"
{
    value(0; Zero) { Caption = 'Nothing'; }
    value(5; Five) { }
    value(10; Ten) { Caption = 'Tenner'; }
}
table 50900 "ALT T"
{
    Caption = 'The Table';
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { Caption = 'The Amount'; }
        field(3; Grp; Integer) { trigger OnValidate() begin Descr := 'validated ' + Format(Grp); end; }
        field(4; Descr; Text[50]) { }
        field(5; Kind; Enum "ALT E") { }
        field(6; Opt; Option) { OptionMembers = A,B,C; OptionCaption = 'Alpha,Beta,Gamma'; }
        field(7; Flag; Boolean) { }
        field(8; Tot; Decimal) { FieldClass = FlowField; }
    }
    keys { key(PK; "No.") { } key(ByGrp; Grp, "No.") { } }
}
table 50910 "ALT Removed"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Txt; Text[20]) { }
        field(3; Gone; Integer) { ObsoleteState = Removed; ObsoleteReason = 'probe'; ObsoleteTag = '1.0'; }
        field(4; Last; Text[20]) { }
    }
    keys { key(PK; Id) { } }
}
table 50902 "ALT Counter"
{
    fields
    {
        field(1; Id; Code[20]) { }
        field(3; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("ALT T"); }
    }
    keys { key(PK; Id) { } }
}
table 50904 "ALT Keys"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; A; Integer) { }
        field(3; B; Text[10]) { }
        field(4; C; Code[10]) { }
        field(5; D; Decimal) { }
    }
    keys { key(PK; Id) { } key(KA; A) { } key(KBA; B, A) { } key(KD; D) { } }
}
table 50903 "ALT Library Types"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Pic; Blob) { }
        field(3; Rid; RecordId) { }
        field(4; Flt; TableFilter) { }
        field(5; Img; Media) { }
        field(6; Imgs; MediaSet) { }
    }
    keys { key(PK; Id) { } }
}
table 50960 "ALT RV"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } }
    keys { key(PK; Id) { } }
}
tableextension 50961 "ALT RV Ext" extends "ALT RV"
{
    keys { key("RV Key"; SystemRowVersion) { } }
}
"#;

fn run(body: &str, routine: &str) -> Result<Value, Signal> {
    let src = format!("{TABLE}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
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
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("{routine} is blocked: {r}");
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn open_number_and_isempty() {
    // corpus recordref/TestRecordRefOpen.al:14-37
    let r = run(
        "procedure F(): Text var RR: RecordRef; begin
            RR.Open(50900);
            exit(Format(RR.Number) + Format(RR.IsEmpty()));
        end;",
        "F",
    );
    assert_eq!(text(r), "50900Yes");
}

fn err(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message,
        other => panic!("expected an AL error, got {other:?}"),
    }
}

fn unsupported(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m,
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

#[test]
fn insert_through_fieldref_is_visible_to_record_and_ref() {
    // corpus recordref/TestRecordRefCRUD.al:16-27, TestRecordVsRecordRef.al:8-30
    let r = run(
        "procedure F(): Text var RR: RecordRef; FR: FieldRef; R: Record \"ALT T\"; begin
            RR.Open(50900);
            RR.Init();
            FR := RR.Field(1);
            FR.Value := 'a';
            RR.Field(3).Value := 42;
            RR.Insert();
            R.Get('A');
            exit(Format(RR.Count()) + '/' + Format(R.Grp));
        end;",
        "F",
    );
    assert_eq!(text(r), "1/42");
}

#[test]
fn fieldref_value_reads_the_found_row() {
    // corpus recordref/TestRecordRefCRUD.al:98-115 (FindLast then Field(1).Value)
    let r = run(
        "procedure F(): Integer var RR: RecordRef; R: Record \"ALT T\"; i: Integer; begin
            for i := 1 to 3 do begin R.\"No.\" := Format(i); R.Grp := i * 10; R.Insert(); end;
            RR.Open(50900);
            RR.FindLast();
            exit(RR.Field(3).Value());
        end;",
        "F",
    );
    assert_eq!(int(r), 30);
}

#[test]
fn findset_next_walks_a_fieldref_range() {
    // corpus recordref/TestRecordRefFilter.al:62-85
    let r = run(
        "procedure F(): Integer var RR: RecordRef; FR: FieldRef; R: Record \"ALT T\"; i: Integer; n: Integer; begin
            for i := 1 to 5 do begin R.\"No.\" := Format(i); R.Grp := i; R.Insert(); end;
            RR.Open(50900);
            FR := RR.Field(3);
            FR.SetRange(2, 4);
            if RR.FindSet() then
                repeat n += 1; until RR.Next() = 0;
            exit(n);
        end;",
        "F",
    );
    assert_eq!(int(r), 3);
}

#[test]
fn modify_delete_deleteall_and_rename_through_the_ref() {
    // corpus recordref/TestRecordRefCRUD.al:30-84, TestRecordVsRecordRef.al:141-167
    let r = run(
        "procedure F(): Text var RR: RecordRef; R: Record \"ALT T\"; s: Text; begin
            R.\"No.\" := 'A'; R.Grp := 1; R.Insert();
            R.\"No.\" := 'B'; R.Grp := 2; R.Insert();
            R.\"No.\" := 'C'; R.Grp := 3; R.Insert();
            RR.Open(50900);
            RR.Field(1).SetRange('A');
            RR.FindFirst();
            RR.Field(3).Value := 99;
            RR.Modify();
            R.Get('A'); s += Format(R.Grp);
            RR.Rename('Z');
            s += Format(R.Get('A')) + Format(R.Get('Z'));
            RR.Reset();
            RR.Field(1).SetRange('B');
            RR.FindFirst();
            RR.Delete();
            RR.Reset();
            s += Format(RR.Count());
            RR.DeleteAll();
            s += Format(RR.IsEmpty());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "99NoYes2Yes");
}

#[test]
fn field_metadata_number_name_length_active_class() {
    // corpus recordref/TestRecordRefField.al:14-23,118-131; fieldref/TestFieldRefMetadata.al:100-136
    let r = run(
        "procedure F(): Text var RR: RecordRef; FR: FieldRef; begin
            RR.Open(50900);
            FR := RR.Field(4);
            exit(Format(FR.Number) + FR.Name + Format(FR.Length()) + Format(FR.Active()) + Format(RR.FieldCount() >= 4) + Format(RR.FieldExist(3)) + Format(RR.FieldExist(99)));
        end;",
        "F",
    );
    assert_eq!(text(r), "4Descr50YesYesYesNo");
}

#[test]
fn open_of_a_missing_table_raises_an_al_error() {
    // corpus recordref/TestRecordRefOpenMissingTable.al:31-49
    let m = err(run("procedure F() var RR: RecordRef; begin RR.Open(99991); end;", "F"));
    assert!(
        m.contains("You tried to invoke the Table object with the ID 99991 from the object"),
        "{m}"
    );
    assert!(
        m.contains("An object with that ID does not exist in the current application"),
        "{m}"
    );
}

#[test]
fn keyref_counts_and_names_key_fields() {
    // corpus recordref/TestBCMetadataContracts.al:30-66, TestRecordRefKeys.al
    let r = run(
        "procedure F(): Text var RR: RecordRef; KR: KeyRef; begin
            RR.Open(50900);
            KR := RR.KeyIndex(2);
            exit(Format(RR.KeyCount()) + Format(KR.FieldCount()) + KR.FieldIndex(1).Name() + Format(KR.FieldIndex(2).Number()) + Format(RR.KeyIndex(1).Active()));
        end;",
        "F",
    );
    // The implicit SystemId key follows the declared keys (corpus recordref/TestRecordRefRuntimeContracts.al:
    // KeyRefAndFieldRef_Active_AreTrue_KeyCount).
    assert_eq!(text(r), "32Grp1Yes");
}

#[test]
fn format_of_a_keyref_lists_its_field_numbers() {
    // recordref/lt-y12-keyref-format/Y12KeyRefFormat.al:33-87 (corpus branch al-runtime-rs-staging-y12, al-runner issue 6437):
    // `Field<no>` in key order joined by commas; format numbers 0, 1, 2 and 9 give the same text; a length cuts it.
    let r = run(
        "procedure F(): Text var RR: RecordRef; KR: KeyRef; begin
            RR.Open(50900);
            KR := RR.KeyIndex(2);
            exit(Format(RR.KeyIndex(1)) + '|' + Format(KR) + '|' + Format(KR, 0, 1) + '|' + Format(KR, 0, 2) + '|'
                + Format(KR, 0, 9) + '|' + Format(KR, 5));
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "Field1|Field3,Field1|Field3,Field1|Field3,Field1|Field3,Field1|Field"
    );
}

#[test]
fn format_of_a_keyref_refuses_what_is_not_pinned() {
    // Only format numbers 0, 1, 2 and 9 and a length that cuts are pinned (Y12KeyRefFormat.al:49-69); the implicit SystemId
    // key is not formatted there.
    for call in [
        "Format(RR.KeyIndex(1), 0, 3)",
        "Format(RR.KeyIndex(1), 30)",
        "Format(RR.KeyIndex(3))",
    ] {
        let r = run(
            &format!("procedure F(): Text var RR: RecordRef; begin RR.Open(50900); exit({call}); end;"),
            "F",
        );
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{call}: {r:?}");
    }
}

#[test]
fn keyref_field_index_past_the_key_raises_index_out_of_bounds() {
    // types/TestTypeSystemContracts.al:240-262 pins the fragment (both operands of `and` run, so
    // FieldIndex(2) on a one-field key raises); recordref/TestBCSumIndexFieldContracts.al:188-198.
    let r = run(
        "procedure F(): Text var RR: RecordRef; KR: KeyRef; m: Boolean; begin
            RR.Open(50900);
            KR := RR.KeyIndex(1);
            asserterror m := (KR.FieldCount() = 2) and (KR.FieldIndex(2).Name() = 'x');
            exit(GetLastErrorText() + Format(m));
        end;",
        "F",
    );
    assert_eq!(text(r), "Index out of boundsNo");
}

#[test]
fn gettable_copies_the_record_in_settable_copies_it_out() {
    // corpus recordref/TestRecordRefGetSetTableContracts.al:30-60 (GetTable leaves the Record
    // alone, the ref takes the Record's buffer), :106-130 (SetTable: Rec is overwritten by the ref).
    let r = run(
        "procedure F(): Text var RR: RecordRef; R: Record \"ALT T\"; R2: Record \"ALT T\"; s: Text; begin
            R.\"No.\" := 'K'; R.Grp := 7; R.Insert();
            RR.Open(50900);
            RR.Field(3).Value := 5;
            RR.SetTable(R2);
            s += Format(R2.Grp);
            R2.Grp := 8;
            RR.GetTable(R2);
            s += Format(RR.Field(3).Value());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "58");
}

// Microsoft's Tests-ERM SalesInvoiceTryPreview (Gen. Jnl.-Post Preview) passes a RecordRef to `GetTable`: the ref takes the
// other ref's table, buffer and filters, which are not shared afterwards (recordref-gettable-of-recordref, docs/probes-needed.md).
#[test]
fn gettable_of_a_record_ref_copies_the_other_ref() {
    let r = run(
        "procedure F(): Text var A: RecordRef; B: RecordRef; s: Text; begin
            A.Open(50900); A.Field(3).Value := 5;
            B.GetTable(A);
            s += Format(B.Field(3).Value());
            A.Field(3).Value := 6;
            s += Format(B.Field(3).Value());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "55");
}

#[test]
fn clear_closes_the_ref() {
    // corpus recordref/TestRecordRefOpenCompilationTarget.al:70-81 (Clear(RecRef) then Open again)
    let r = run(
        "procedure F(): Integer var RR: RecordRef; begin
            RR.Open(50900); Clear(RR); RR.Open(50900);
            exit(RR.Number);
        end;",
        "F",
    );
    assert_eq!(int(r), 50900);
}

#[test]
fn a_ref_that_is_not_open_raises_record_not_opened_where_the_corpus_pins_it() {
    // recordref/lt-refs/RfTOpen.al:Closed_Errors (Count, Name, Find, ...), P92Position.al:GetPosition_UnopenedAndNotFound.
    let m = err(run(
        "procedure F(): Integer var RR: RecordRef; begin exit(RR.Count()); end;",
        "F",
    ));
    assert_eq!(m, "The record is not open.");
    // A method the corpus does not pin on a ref never opened is refused.
    let m = unsupported(run("procedure F() var RR: RecordRef; begin RR.ClearMarks(); end;", "F"));
    assert!(m.contains("not open"), "{m}");
}

#[test]
fn recordref_metadata_caption_company_duplicate_system_field_numbers() {
    // corpus recordref/TestRecordRefGetSet.al:16-30,53-72; TestRecordRefExtended.al; TestRecordVsRecordRef.al:333-352
    let r = run(
        "procedure F(): Text var RR: RecordRef; D: RecordRef; R: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Insert(); R.\"No.\" := 'B'; R.Insert();
            RR.Open(50900);
            D := RR.Duplicate();
            D.Field(1).SetRange('A');
            exit(RR.Caption + '|' + Format(RR.CurrentCompany() <> '') + '|' + Format(RR.Count()) + Format(D.Count()) + '|' + Format(RR.SystemIdNo()) + '|' + Format(RR.SystemModifiedAtNo()) + '|' + Format(RR.IsTemporary()));
        end;",
        "F",
    );
    assert_eq!(text(r), "The Table|Yes|21|2000000000|2000000003|No");
}

#[test]
fn views_filters_and_has_filter_on_a_recordref() {
    // corpus recordref/TestRecordRefFilter.al:14-60, 98-121; TestRecordVsRecordRef.al:150-180
    let r = run(
        "procedure F(): Text var RR: RecordRef; D: RecordRef; R: Record \"ALT T\"; i: Integer; s: Text; begin
            for i := 1 to 5 do begin R.\"No.\" := Format(i); R.Grp := i; R.Insert(); end;
            RR.Open(50900);
            s += Format(RR.HasFilter());
            RR.SetView('SORTING(Grp) WHERE(Grp=FILTER(1..3))');
            s += Format(RR.HasFilter()) + Format(RR.Count()) + Format(RR.GetFilters() <> '') + Format(RR.GetView() <> '');
            D.Open(50900);
            D.SetView(RR.GetView());
            s += Format(D.Count());
            RR.Reset();
            s += Format(RR.Count()) + Format(RR.HasFilter());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "NoYes3YesYes3".to_string() + "5No");
}

#[test]
fn setview_sorting_picks_the_key() {
    // docs: SORTING(...) is SetCurrentKey; corpus recordref/TestRecordRefFilter.al:14-30 uses it with the primary key
    let r = run(
        "procedure F(): Text var RR: RecordRef; R: Record \"ALT T\"; s: Text; begin
            R.\"No.\" := 'A'; R.Grp := 2; R.Insert(); R.\"No.\" := 'B'; R.Grp := 1; R.Insert();
            RR.Open(50900);
            RR.SetView('SORTING(Grp,No.)');
            RR.FindFirst();
            exit(Format(RR.Field(1).Value()));
        end;",
        "F",
    );
    assert_eq!(text(r), "B");
}

#[test]
fn fieldref_caption_validate_and_getfilter() {
    // corpus fieldref/TestFieldRefMetadata.al:36-54, TestFieldRefValidate.al:10-30,70-90, TestFieldRefFilter.al:127-145
    let r = run(
        "procedure F(): Text var RR: RecordRef; FR: FieldRef; s: Text; begin
            RR.Open(50900);
            s := RR.Field(2).Caption() + '|' + RR.Field(3).Caption() + '|';
            FR := RR.Field(3);
            FR.Validate(4);
            s += Format(FR.Value()) + RR.Field(4).Value() + '|';
            FR.SetRange(1, 3);
            s += FR.GetFilter();
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "The Amount|Grp|4validated 4|1..3");
}

#[test]
fn fieldref_enum_and_option_metadata() {
    // corpus fieldref/TestFieldRefEnumOrdinalByIndex.al (index/ordinal/caption/name, out of range),
    // TestFieldRefEnum.al:112-124, TestPrecompiledFieldOptionCaption.al:34-48
    let r = run(
        "procedure F(): Text var RR: RecordRef; E: FieldRef; O: FieldRef; s: Text; begin
            RR.Open(50900);
            E := RR.Field(5);
            O := RR.Field(6);
            s := Format(E.IsEnum()) + Format(O.IsEnum()) + Format(RR.Field(3).IsEnum()) + '|' + Format(E.EnumValueCount()) + '|';
            s += Format(E.GetEnumValueOrdinal(2)) + E.GetEnumValueName(3) + E.GetEnumValueCaption(1) + E.GetEnumValueCaption(2) + '|';
            s += Format(E.GetEnumValueOrdinal(0)) + Format(E.GetEnumValueOrdinal(4)) + '[' + E.GetEnumValueCaption(4) + ']|';
            s += E.GetEnumValueNameFromOrdinalValue(10) + E.GetEnumValueCaptionFromOrdinalValue(10) + '|';
            s += O.OptionCaption() + '/' + O.OptionMembers() + '/' + O.OptionString() + '|' + Format(O.GetEnumValueOrdinal(3)) + O.GetEnumValueCaption(3);
            exit(s);
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "YesNoNo|3|5TenNothingFive|-1-1[]|TenTenner|Alpha,Beta,Gamma/A,B,C/A,B,C|2Gamma"
    );
}

#[test]
fn fieldref_testfield_and_fielderror() {
    // corpus fieldref/TestFieldRefFieldError.al:10-100
    let m = err(run(
        "procedure F() var RR: RecordRef; begin RR.Open(50900); RR.Field(3).TestField(); end;",
        "F",
    ));
    assert!(m.starts_with("Grp must have a value in The Table"), "{m}");
    let m = err(run(
        "procedure F() var RR: RecordRef; begin RR.Open(50900); RR.Field(3).Value := 1; RR.Field(3).TestField(2); end;",
        "F",
    ));
    assert!(m.starts_with("Grp must be equal to '2'  in The Table"), "{m}");
    let m = err(run(
        "procedure F() var RR: RecordRef; begin RR.Open(50900); RR.Field(3).FieldError('Custom msg'); end;",
        "F",
    ));
    assert!(m.contains("Grp") && m.contains("Custom msg"), "{m}");
}

#[test]
fn fieldref_record_gives_the_same_buffer_back() {
    let r = run(
        "procedure F(): Integer var RR: RecordRef; B: RecordRef; begin
            RR.Open(50900);
            RR.Field(3).Value := 9;
            B := RR.Field(3).Record();
            exit(B.Field(3).Value());
        end;",
        "F",
    );
    assert_eq!(int(r), 9);
}

#[test]
fn record_id_table_no_equality_format_and_variant() {
    // corpus record/TestRecordRecordId.al:14-56, recordref/TestBCMetadataContracts.al:98-150,
    // types/TestVariantExtended.al:123-130
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; A: RecordId; B: RecordId; V: Variant; s: Text; begin
            R.\"No.\" := 'X1'; R.Grp := 1; R.Insert();
            R.\"No.\" := 'X2'; R.Insert();
            R.Get('X1'); A := R.RecordId();
            R.Get('X1'); B := R.RecordId();
            s := Format(A = B) + Format(A.TableNo());
            R.Get('X2'); B := R.RecordId();
            s += Format(A = B) + '|' + Format(A);
            V := A;
            s += '|' + Format(V.IsRecordId());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes50900No|ALT T: X1|Yes");
}

#[test]
fn recordref_recordid_and_get_by_recordid() {
    // corpus recordref/TestRecordRefKeys.al:35-52 (RecRef.Get(Rec.RecordId())), TestRecordRefGetSet.al:32-50
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; RR: RecordRef; s: Text; begin
            R.\"No.\" := 'X1'; R.Grp := 11; R.Insert();
            R.\"No.\" := 'X2'; R.Grp := 22; R.Insert();
            RR.Open(50900);
            s := Format(RR.Get(R.RecordId())) + Format(RR.Field(3).Value());
            R.Get('X1');
            s += Format(RR.RecordId() = R.RecordId());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes22No");
}

#[test]
fn field_type_and_class_are_system_options() {
    // corpus fieldref/TestFieldRefMetadata.al:56-98 (Type non-empty, Class = Normal),
    // TestFieldRefEnumKeywordCase.al:95-106 (an enum field's Type formats as 'Option'),
    // tableextensiontrigger/TestTableExtTargetNamespace_Tests.al:119-121 (FieldType::Integer / ::Text)
    let r = run(
        "procedure F(): Text var RR: RecordRef; ft: FieldType; fc: FieldClass; s: Text; begin
            RR.Open(50900);
            ft := RR.Field(3).Type();
            fc := RR.Field(3).Class();
            s := Format(ft) + '/' + Format(fc) + '/' + Format(ft = FieldType::Integer) + Format(fc = FieldClass::Normal) + Format(ft = FieldType::Text);
            s += '|' + Format(RR.Field(5).Type()) + Format(RR.Field(1).Type()) + Format(RR.Field(4).Type()) + Format(RR.Field(7).Type());
            s += '|' + Format(RR.Field(8).Class()) + Format(RR.Field(8).Class() = FieldClass::FlowField);
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Integer/Normal/YesYesNo|OptionCodeTextBoolean|FlowFieldYes");
}

#[test]
fn gettable_works_on_a_ref_that_was_never_opened_even_through_a_variant() {
    // corpus _fixtures/Assert.al:95-130 (Assert.RecordIsEmpty(RecVariant): `RecRef.GetTable(RecVariant)` on a fresh ref)
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; V: Variant; RR: RecordRef; s: Text; begin
            R.\"No.\" := 'A'; R.Insert();
            R.SetRange(Grp, 5);
            V := R;
            RR.GetTable(V);
            s := Format(RR.Number) + Format(RR.IsEmpty()) + Format(RR.HasFilter()) + RR.GetFilters();
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "50900YesYesGrp: 5");
}

#[test]
fn field_ref_calc_field_calculates_a_flow_field() {
    // fieldref/TestRelationTargetNamespaceScope.al:63-75: `RecRef.Field(3).CalcField()` makes the
    // FlowField read its count.
    let r = run(
        "procedure F(): Text var T: Record \"ALT T\"; RR: RecordRef; a: Text; begin
            T.\"No.\" := 'A'; T.Insert(); T.\"No.\" := 'B'; T.Insert();
            RR.Open(50902); RR.Init(); RR.Field(1).Value := 'X';
            a := Format(RR.Field(3).Value);
            RR.Field(3).CalcField();
            exit(a + '|' + Format(RR.Field(3).Value));
        end;",
        "F",
    );
    assert_eq!(text(r), "0|2");
}

#[test]
fn field_ref_calc_field_of_a_stored_field_is_refused() {
    let r = run(
        "procedure F(): Text var RR: RecordRef; begin RR.Open(50900); RR.Field(2).CalcField(); exit(''); end;",
        "F",
    );
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("not a FlowField")),
        "{r:?}"
    );
}

#[test]
fn field_type_of_library_typed_fields_formats_as_the_type_name() {
    // corpus recordref/TestPlatformOptionsLockingContracts.al:46-63 (13=BLOB), :66-76 (RecordID,
    // TableFilter, MediaSet name themselves; Format(.., 0, 2) prints the name, never an ordinal).
    let r = run(
        "procedure F(): Text var RR: RecordRef; s: Text; begin
            RR.Open(50903);
            s := Format(RR.Field(2).Type()) + '|' + Format(RR.Field(3).Type()) + '|' + Format(RR.Field(4).Type())
                + '|' + Format(RR.Field(5).Type()) + '|' + Format(RR.Field(6).Type());
            s += '|' + Format(FieldType::BLOB, 0, 2) + '|' + Format(RR.Field(2).Type(), 0, 2);
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "BLOB|RecordID|TableFilter|Media|MediaSet|BLOB|BLOB");
}

#[test]
fn security_filter_ordinals() {
    // corpus recordref/TestPlatformOptionsLockingContracts.al:36-43 (read with Format(x, 0, 2), BC 27.5).
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; f: SecurityFilter; s: Text; begin
            s := Format(SecurityFilter::Validated, 0, 2) + Format(SecurityFilter::Filtered, 0, 2)
                + Format(SecurityFilter::Ignored, 0, 2) + Format(SecurityFilter::Disallowed, 0, 2);
            Evaluate(f, '3');
            s += '|' + Format(f) + '|' + Format(R.SecurityFiltering());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "0123|Disallowed|Validated");
}

#[test]
fn format_of_a_table_filter_field_nobody_set_is_empty() {
    // Base App's Change Log Management.HasValue ends in `Format(FldRef.Value) <> ''` for a TableFilter
    // field; 16 tests (permissions/TestMetadataExpandedPermission.al:219-235) insert such a row and
    // pass on real BC. An unset filter is empty text (assumption: docs/probes-needed.md).
    let r = run(
        "procedure F(): Text var RR: RecordRef; begin
            RR.Open(50903);
            exit('[' + Format(RR.Field(4).Value) + ']');
        end;",
        "F",
    );
    assert_eq!(text(r), "[]");
}

#[test]
fn ascending_sets_and_reads_the_sort_direction_like_the_record_method() {
    // Doc (RecordRef.Ascending): the same as Record.Ascending, which record/TestRecordSort.al:111 pins.
    // Base Application's "Selection Filter Management" calls `TempRecRef.Ascending(true)`.
    let r = run(
        "procedure F(): Text var RR: RecordRef; R: Text; begin
            RR.Open(50900);
            RR.Field(1).Value := 'A'; RR.Insert(); RR.Init(); RR.Field(1).Value := 'B'; RR.Insert();
            R := Format(RR.Ascending());
            RR.Ascending(false);
            R += Format(RR.Ascending());
            RR.FindSet(); R += Format(RR.Field(1).Value);
            RR.Ascending(true); RR.FindSet(); R += Format(RR.Field(1).Value);
            exit(R);
        end;",
        "F",
    );
    assert_eq!(text(r), "YesNoBA");
}

#[test]
fn set_rec_filter_filters_the_primary_key_to_the_current_values() {
    // Doc (RecordRef.SetRecFilter): the same as Record.SetRecFilter, pinned by
    // record/TestFilterAdvancedContracts.al:144-175 (one record, the one that was current).
    let r = run(
        "procedure F(): Text var RR: RecordRef; R: Text; begin
            RR.Open(50900);
            RR.Field(1).Value := 'A'; RR.Insert(); RR.Init(); RR.Field(1).Value := 'B'; RR.Insert();
            RR.FindFirst();
            RR.SetRecFilter();
            R := Format(RR.Count()) + RR.Field(1).GetFilter();
            exit(R);
        end;",
        "F",
    );
    assert_eq!(text(r), "1A");
}

#[test]
fn current_key_index_numbers_the_keys_and_the_hidden_system_id_key() {
    // recordref/TestRecordRefKeyIndexSetRecFilterContracts.al:75-105 (KeyCount counts the hidden key last;
    // CurrentKeyIndex(n) selects, the hidden key is `System ID`), :107-124 (outside 1..KeyCount raises
    // DB:KeyNotFound), :126-148 (the direction stays, Reset goes back to key 1), :150-176 (GetTable
    // follows the record's key; fields no key starts with report -1).
    let r = run(
        "procedure F(): Text var RR: RecordRef; R: Record \"ALT Keys\"; KR: KeyRef; i: Integer; s: Text; begin
            RR.Open(50904);
            KR := RR.KeyIndex(5); s := KR.FieldIndex(1).Name() + ';';
            s += Format(RR.KeyCount()) + '|' + Format(RR.CurrentKeyIndex()) + RR.CurrentKey() + '|';
            for i := 1 to RR.KeyCount() do begin
                RR.CurrentKeyIndex(i);
                s += Format(RR.CurrentKeyIndex()) + '=' + RR.CurrentKey() + ';';
            end;
            asserterror RR.CurrentKeyIndex(0);
            s += '|' + GetLastErrorText() + '|' + GetLastErrorCode();
            asserterror RR.CurrentKeyIndex(6);
            s += '|' + GetLastErrorText();
            RR.CurrentKeyIndex(2); RR.Ascending(false); RR.CurrentKeyIndex(1);
            s += '|' + Format(RR.Ascending());
            RR.CurrentKeyIndex(4); RR.Reset();
            s += '|' + Format(RR.CurrentKeyIndex());
            R.SetCurrentKey(B, A); RR.GetTable(R);
            s += '|' + Format(RR.CurrentKeyIndex()) + RR.CurrentKey();
            R.SetCurrentKey(C); RR.GetTable(R);
            s += '|' + Format(RR.CurrentKeyIndex()) + RR.CurrentKey();
            exit(s);
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "$systemId;5|1Id|1=Id;2=A;3=B,A;4=D;5=System ID;|The ALT Keys table does not have an active key at index 0.|DB:KeyNotFound|The ALT Keys table does not have an active key at index 6.|No|1|3B,A|-1C"
    );
}

#[test]
fn a_tableextension_key_on_systemrowversion_is_visible_through_keyref() {
    // A key a table extension declares on SystemRowVersion is a key of the table; KeyRef sees it
    // after the declared keys and before the implicit SystemId key (same order as
    // recordref/TestRecordRefRuntimeContracts.al:KeyRefAndFieldRef_Active_AreTrue_KeyCount).
    let r = run(
        "procedure F(): Text var RR: RecordRef; KR: KeyRef; i: Integer; s: Text; begin
            RR.Open(50960);
            for i := 1 to RR.KeyCount() do begin
                KR := RR.KeyIndex(i);
                s += Format(KR.FieldCount()) + ':' + KR.FieldIndex(1).Name() + ';';
            end;
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "1:Id;1:timestamp;1:$systemId;");
}

#[test]
fn systemrowversion_is_field_0_through_fieldno_number_and_field() {
    // Corpus recordref/TestKeyMetadataRowVersionRemoved.al:51,:85 pins that KeyRef.FieldIndex(1).Number()
    // equals FieldNo(SystemRowVersion); the rowversion sits at field id 0 (record/TestCalcFormulaSystemFieldsTests
    // .Codeunit.al:213-215), which is also what a real project reads (`KeyRef.FieldIndex(1).Number = 0`).
    // Pinned by record/TestRowVersionFieldNumberFieldActiveContracts.al (name `timestamp`, FieldExist(0) false). The other system fields keep their numbers.
    let r = run(
        "procedure F(): Text var RR: RecordRef; KR: KeyRef; R: Record \"ALT RV\"; begin
            RR.Open(50960);
            KR := RR.KeyIndex(2);
            exit(Format(KR.FieldIndex(1).Number()) + '|' + Format(R.FieldNo(SystemRowVersion)) + '|' +
                Format(R.FieldNo(SystemId)) + '|' + Format(RR.Field(0).Name()) + '|' + Format(RR.FieldExist(0)) + '|' +
                Format(RR.Field(R.FieldNo(SystemRowVersion)).Number()));
        end;",
        "F",
    );
    assert_eq!(text(r), "0|0|2000000000|timestamp|No|0");
}

// recordref/PWRecordMisc.al:109-150 (BC 27.5): a field with ObsoleteState = Removed is not counted, not listed and does
// not exist; RecordRef.Field of it raises MetadataObsolete with the reason.
#[test]
fn a_removed_field_is_not_enumerated_and_its_field_call_raises_the_obsolete_error() {
    let out = text(run(
        "procedure F(): Text var R: RecordRef; F: FieldRef; S: Text; I: Integer; begin
            R.Open(50910); S := Format(R.FieldCount()) + Format(R.FieldExist(3)) + Format(R.FieldExist(4)) + ',';
            for I := 1 to R.FieldCount() do S += R.FieldIndex(I).Name + '(' + Format(R.FieldIndex(I).Number) + ') ';
            asserterror F := R.Field(3); exit(S + '|' + GetLastErrorText() + '|' + GetLastErrorCode()); end;",
        "F",
    ));
    assert_eq!(
        out,
        "3NoYes,Id(1) Txt(2) Last(4) |Field Gone (3) of table ALT Removed (50910) is obsoleted, reason: probe|MetadataObsolete"
    );
}

#[test]
fn filter_group_assigns_as_a_property_of_a_record_ref() {
    // `RecRef.FilterGroup := 2` is the setter form of `RecRef.FilterGroup(2)` (Base Application's Find Record
    // Management does it). RecordRef.FilterGroup itself is an assumption: no corpus test calls it
    // (docs/probes-needed.md).
    let r = run(
        "procedure F(): Text var RR: RecordRef; s: Text; begin
            RR.Open(50900);
            s := Format(RR.FilterGroup());
            RR.FilterGroup := 2;
            s += '|' + Format(RR.FilterGroup);
            RR.FilterGroup := 0;
            exit(s + '|' + Format(RR.FilterGroup()));
        end;",
        "F",
    );
    assert_eq!(text(r), "0|2|0");
}

#[test]
fn get_of_a_record_id_on_a_record_ref_never_opened_opens_it_when_the_row_exists() {
    // misc/lt-probes-needed-4 batch 4 (W4 RecRef Get Tests): Get_OnUnopenedRecordRef_ExistingRow and _MissingRow.
    let r = run(
        "procedure F(): Text var RR: RecordRef; T: Record \"ALT T\"; Ok: Boolean; s: Text; begin
            T.\"No.\" := 'A'; T.Insert();
            Ok := RR.Get(T.RecordId());
            s := Format(Ok) + '|' + Format(RR.Number() = T.RecordId().TableNo());
            T.\"No.\" := 'MISSING';
            Clear(RR);
            Ok := RR.Get(T.RecordId());
            s += '|' + Format(Ok) + '|' + Format(RR.Number());
            if not RR.Get(T.RecordId()) then RR.Open(T.RecordId().TableNo());
            exit(s + '|' + Format(RR.Number() = T.RecordId().TableNo()));
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes|Yes|No|0|Yes");
}

#[test]
fn date_and_boolean_to_text_are_the_default_format() {
    // Customer Mgt.CalcLastPaymentInfo: `Posting Date.ToText()`, `(a < b).ToText()`.
    let r = run(
        "procedure F(): Text var D: Date; B: Boolean; begin D := DMY2Date(5, 3, 2024); B := true;
            exit(D.ToText() + '|' + B.ToText() + '|' + Format(D)); end;",
        "F",
    );
    let t = text(r);
    let parts: Vec<&str> = t.split('|').collect();
    assert_eq!(parts[0], parts[2], "{t}");
    assert_eq!(parts[1], "Yes", "{t}");
}

#[test]
fn decimal_to_text_is_the_default_format() {
    // Customer Mgt.CalcLastPaymentInfo: `(-Amount).ToText()` (55 Tests-ERM tests).
    let r = run(
        "procedure F(): Text var D: Decimal; begin D := -12.5; exit(D.ToText() + '|' + Format(D)); end;",
        "F",
    );
    let t = text(r);
    let parts: Vec<&str> = t.split('|').collect();
    assert_eq!(parts[0], parts[1], "{t}");
}

/// Agent Creation Control (2000000297), declared by the System app from BC 28 on, is refused to a Cloud-target app like
/// Object Metadata: `RecordRef.Open` raises before any row is read (platform/lt-aw-fresh-tenant/AwTests.al:590-616). An
/// OnPrem app opens it (onprem platform/lt-aw-fresh-tenant/AwOnPremTests.al:234-253); an app of no known target is refused.
#[test]
fn recordref_open_of_agent_creation_control_depends_on_the_app_target() {
    let src = r#"
table 2000000297 "Agent Creation Control" { fields { field(1; ID; Integer) { } } keys { key(PK; ID) { } } }
codeunit 50100 C { procedure Run() var RR: RecordRef; begin RR.Open(2000000297); end; }"#;
    let run = |target: Option<&str>| {
        let mut l = al_testkit::compile(src);
        for a in &mut std::sync::Arc::make_mut(&mut l.program).apps {
            a.target = target.map(String::from);
        }
        al_testkit::Built::new(l).run("Run")
    };
    assert_eq!(
        al_testkit::error_message(run(Some("Cloud"))),
        "You cannot open record 2000000297 from a RecordRef data type when you are using target Cloud."
    );
    assert!(run(Some("OnPrem")).is_ok());
    assert!(matches!(run(None), Err(Signal::Unsupported(_))));
}
