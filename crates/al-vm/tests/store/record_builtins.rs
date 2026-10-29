//! Record builtins that need table metadata: AL source → compile → run.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::text;
use al_vm::{Engine, Session};

const TABLE: &str = r#"
table 50900 "ALT T"
{
    Caption = 'The Table';
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { Caption = 'The Amount'; }
        field(3; Grp; Integer) { }
        field(4; Descr; Text[50]) { }
        field(5; Flag; Boolean) { }
        field(6; Total; Decimal) { FieldClass = FlowField; }
    }
    keys { key(PK; "No.") { } key(ByGrp; Grp, "No.") { } key(ByAmt; Amount) { } }
}
enum 50901 "ALT Blank Kind" { value(0; " ") { Caption = '(All)'; } value(1; Item) { } }
table 50902 "ALT Blank"
{
    fields
    {
        field(1; Id; Code[10]) { }
        field(2; Kind; Option) { OptionMembers = " ",A,B; }
        field(3; Kind2; Enum "ALT Blank Kind") { }
        field(4; Kind3; Option) { OptionMembers = First,Second; }
    }
    keys { key(PK; Id) { } }
}
table 50903 "ALT EO Enum" { fields { field(1; Id; Code[10]) { } field(2; Kind; Enum "ALT Blank Kind") { } } keys { key(PK; Id) { } } }
table 50904 "ALT EO Option" { fields { field(1; Id; Code[10]) { } field(2; Kind; Option) { OptionMembers = " ",A,B; } } keys { key(PK; Id) { } } }
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

/// The message of the AL error a routine raises.
fn error_text(body: &str) -> String {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"ALT T\"; begin R.\"No.\" := 'A'; R.Amount := 0; R.Grp := 7; R.Insert(); asserterror begin {body} end; exit(GetLastErrorText()); end;"
        ),
        "F",
    );
    text(r)
}

#[test]
fn field_and_table_metadata() {
    // corpus fieldref/TestPrecompiledFieldOptionCaption.al:100-125 (Caption property, else the name),
    // record/TestRecordTableCaption.al, recordref/TestRecordRefField.al:118-131
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            exit(R.FieldCaption(Amount) + '|' + R.FieldCaption(Grp) + '|' + R.FieldName(Amount) + '|' + Format(R.FieldNo(Grp)) + '|' + R.TableCaption() + '|' + R.TableName());
        end;",
        "F",
    );
    assert_eq!(text(r), "The Amount|Grp|Amount|3|The Table|ALT T");
}

#[test]
fn testfield_on_an_empty_field_names_field_table_and_key() {
    // corpus record/TestRecordTestFieldLookupPageId_Tests.al:54-72 (exact text)
    assert_eq!(
        error_text("R.TestField(Amount);"),
        "The Amount must have a value in The Table: No.=A. It cannot be zero or empty."
    );
}

#[test]
fn testfield_passes_on_a_populated_field_and_on_a_matching_value() {
    // corpus record/TestRecordTestField.al:25-40, :60-72
    let r = run(
        "procedure F(): Integer var R: Record \"ALT T\"; begin
            R.Grp := 7; R.TestField(Grp); R.TestField(Grp, 7); R.Descr := 'x'; R.TestField(Descr, 'x');
            exit(R.Grp);
        end;",
        "F",
    );
    assert_eq!(text_or_int(r), 7);
}

fn text_or_int(r: Result<Value, Signal>) -> i32 {
    match r {
        Ok(Value::Integer(i)) => i,
        other => panic!("{other:?}"),
    }
}

#[test]
fn testfield_with_a_wrong_value_names_the_field() {
    // corpus error-handling/TestErrorTextContracts.al:68-79 pins only the field caption; the
    // rest of the wording is Assert.al's documented format (docs/probes-needed.md).
    let m = error_text("R.TestField(Grp, 99);");
    assert!(m.contains("Grp"), "{m}");
    assert!(m.contains("99") && m.contains("7"), "{m}");
}

#[test]
fn testfield_rejects_every_kind_of_empty_value() {
    // corpus record/TestRecordTestField.al:42-52 (Text), :246-262 (Code), BigInteger, Decimal, Boolean
    let m = error_text("R.TestField(Descr);");
    assert!(m.starts_with("Descr must have a value in"), "{m}");
    let m = error_text("R.TestField(\"No.\"); R.Flag := false; R.TestField(Flag);");
    assert!(m.starts_with("Flag must have a value in"), "{m}");
}

#[test]
fn truncate_deletes_every_row_and_returns_true() {
    // corpus record/TestRecordDelete.al:169-188, TestBCSystemFieldContracts.al:180-205
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; ok: Boolean; begin
            R.\"No.\" := 'A'; R.Insert(); R.\"No.\" := 'B'; R.Insert();
            ok := R.Truncate(false);
            exit(Format(ok) + Format(R.IsEmpty()));
        end;",
        "F",
    );
    assert_eq!(text(r), "YesYes");
}

#[test]
fn truncate_on_an_empty_table_does_not_throw() {
    // corpus record/TestBCSystemFieldContracts.al:207-215
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin R.Truncate(); exit('ok'); end;",
        "F",
    );
    assert_eq!(text(r), "ok");
}

#[test]
fn truncate_refuses_a_filter_on_a_flowfield() {
    // corpus record/TestRecordTruncateEventAndFilterGuards.al:112-125 (exact text)
    assert_eq!(
        error_text("R.SetRange(Total, 0, 100); R.Truncate();"),
        "Truncate does not support filters on FlowFields."
    );
}

#[test]
fn truncate_inside_a_try_function_is_refused_even_nested() {
    // corpus record/TestRecordTruncateTryScope.al:36-110 (exact text, direct and nested)
    let r = run(
        "[TryFunction] local procedure Direct() var R: Record \"ALT T\"; begin R.Truncate(); end;
         local procedure Helper() var R: Record \"ALT T\"; begin R.Truncate(); end;
         [TryFunction] local procedure Nested() begin Helper(); end;
         procedure F(): Text var s: Text; begin
            if Direct() then s := 'direct ok';
            s += GetLastErrorText();
            if Nested() then s += 'nested ok';
            s += '/' + GetLastErrorText();
            Helper();
            exit(s);
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "Truncate is not supported in try functions./Truncate is not supported in try functions."
    );
}

#[test]
fn rename_is_refused_when_a_referencing_relation_has_a_shape_that_is_not_understood() {
    // The conditional-relation shapes themselves propagate (rename.rs); a where() form the parser does
    // not read (`upperlimit`) cannot be applied faithfully, so the rename of the table it names refuses.
    let src = r#"
table 50910 "Parent B" { fields { field(1; Code; Code[20]) { } field(2; Lim; Code[20]) { } } keys { key(PK; Code) { } } }
table 50912 "Child" {
    fields {
        field(1; Id; Integer) { }
        field(3; Ref; Code[20]) { TableRelation = "Parent B".Code where(Lim = upperlimit(Id)); }
    }
    keys { key(PK; Id) { } }
}
codeunit 50100 C { procedure F() var P: Record "Parent B"; begin P.Code := 'OLD'; P.Insert(); P.Rename('NEW'); end; }
"#;
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let r = s.call(oi, 0, inst, vec![]);
    assert!(
        matches!(r, Err(Signal::Unsupported(ref m)) if m.contains("where()") && m.contains("not understood")),
        "{r:?}"
    );
}

#[test]
fn system_field_captions_only_where_the_corpus_pins_them() {
    // corpus record/TestRecordSystemId.al:178 pins SystemId's caption; the audit fields are
    // refused at lowering when AL names them (SystemCreatedAt/By, SystemModifiedAt/By).
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin exit(R.FieldCaption(SystemId)); end;",
        "F",
    );
    assert_eq!(text(r), "System ID");
}

#[test]
fn a_tableextension_modify_changes_the_caption_the_ref_and_the_record_answer() {
    // corpus tableextension/TestTableExtFieldModify.al:40-100
    let src = r#"
table 50930 T { fields { field(1; A; Integer) { Caption = 'Orig A'; } field(2; B; Integer) { Caption = 'Orig B'; } } keys { key(PK; A) { } } }
tableextension 50931 X extends T { fields { modify(A) { Caption = 'New A'; } } }
codeunit 50100 C { procedure F(): Text var R: Record T; RR: RecordRef; begin RR.Open(50930); exit(R.FieldCaption(A) + '|' + R.FieldCaption(B) + '|' + RR.Field(1).Caption()); end; }
"#;
    let l = al_testkit::compile(src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let r = s.call(oi, 0, inst, vec![]).map(|(v, _)| v);
    assert_eq!(text(r), "New A|Orig B|New A");
}

#[test]
fn ascending_current_key_and_is_temporary() {
    // corpus record/TestRecordSort.al:111-112 (default true), TestSortingKeyContracts.al:224,
    // TestCopyOntoTemporaryRecord.al:89-93 (CurrentKey is the key's field names)
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; T: Record \"ALT T\" temporary; s: Text; begin
            s := Format(R.Ascending()) + '|' + R.CurrentKey() + '|';
            R.Ascending(false);
            R.SetCurrentKey(Grp);
            s += Format(R.Ascending()) + '|' + R.CurrentKey() + '|' + Format(R.IsTemporary()) + Format(T.IsTemporary);
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes|No.|No|Grp,No.|NoYes");
}

#[test]
fn filters_read_back_per_field_and_in_total() {
    // corpus record/TestCopyOntoTemporaryRecord.al:91-92 ('<>L0001', 'Included'), TestFilterAdvancedContracts.al:204-211
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; begin
            s := Format(R.HasFilter()) + '[' + R.GetFilters() + ']';
            R.SetRange(Grp, 1, 3);
            R.SetFilter(\"No.\", '<>%1', 'A');
            s += Format(R.HasFilter()) + R.GetFilter(Grp) + '|' + R.GetFilter(\"No.\") + '|' + R.GetFilter(Descr) + '|' + R.GetFilters();
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "No[]Yes1..3|<>A||No.: <>A, Grp: 1..3");
}

#[test]
fn getview_setview_round_trip_and_setrecfilter() {
    // corpus record/TestFilterAdvancedContracts.al:125-141,144-160, TestRecordFilter.al:344-385
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; R2: Record \"ALT T\"; i: Integer; s: Text; begin
            for i := 1 to 5 do begin R.\"No.\" := Format(i); R.Grp := i; R.Insert(); end;
            R.SetRange(Grp, 2, 4);
            s := Format(R.GetView() <> '') + Format(StrPos(R.GetView(true), 'Grp') > 0);
            R2.SetView(R.GetView());
            s += Format(R2.Count()) + R2.GetView(false);
            R.Reset();
            R.Get('5');
            R.SetRecFilter();
            s += '|' + Format(R.Count()) + R.GetFilter(\"No.\") + Format(R.HasFilter());
            exit(s);
        end;",
        "F",
    );
    // GetView starts with VERSION(1); without names the filter kind is a number (corpus
    // record/TestRecordRuntimeTextsAndViews.al:Record_GetView_And_GetFilters_Texts).
    assert_eq!(
        text(r),
        "YesYes3VERSION(1) SORTING(Field1) WHERE(Field3=1(2..4))|1".to_string() + "5Yes"
    );
}

#[test]
fn copy_filters_and_copy_onto_a_temporary_record() {
    // corpus record/TestFilterContracts.al:137-152 (CopyFilters), TestCopyOntoTemporaryRecord.al:66-110
    let r = run(
        "procedure F(): Text var Src: Record \"ALT T\"; B: Record \"ALT T\" temporary; C: Record \"ALT T\"; i: Integer; s: Text; begin
            for i := 1 to 4 do begin B.\"No.\" := 'T' + Format(i); B.Grp := i; B.Insert(); end;
            Src.\"No.\" := 'S1'; Src.Insert();
            Src.SetCurrentKey(Grp);
            Src.Ascending(false);
            Src.SetFilter(Grp, '>1');
            B.Copy(Src);
            s := Format(B.IsTemporary) + Format(B.Count()) + B.GetFilter(Grp) + '|' + B.CurrentKey() + Format(B.Ascending());
            B.FindFirst();
            s += B.\"No.\";
            C.CopyFilters(Src);
            s += '|' + Format(C.Count());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes3>1|Grp,No.NoT4|0");
}

#[test]
fn filter_groups_combine_as_and_and_reset_clears_all() {
    // corpus record/TestFilterContracts.al:98-113,154-175, TestFilterAdvancedContracts.al:181-212,
    // TestRecordFilter.al:270-305
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; i: Integer; s: Text; begin
            for i := 1 to 5 do begin R.\"No.\" := Format(i); R.Grp := i; R.Insert(); end;
            s := Format(R.FilterGroup());
            R.FilterGroup(2);
            s += Format(R.FilterGroup());
            R.SetRange(Grp, 1, 3);
            s += '|' + R.GetFilter(Grp);
            R.FilterGroup(0);
            s += '|' + R.GetFilter(Grp) + '|' + Format(R.Count());
            R.SetRange(Grp, 2, 5);
            s += '|' + Format(R.Count()) + R.GetFilter(Grp);
            R.Reset();
            s += '|' + Format(R.Count());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "02|1..3||3|22..5|5");
}

// SetRecFilter keeps the filters on other fields (record/TestRecordViewCopyMarkContracts.al:41,
// SetRecFilter_KeepsFiltersOnOtherFields_AndFiltersOnThePrimaryKey).
#[test]
fn setrecfilter_keeps_a_filter_on_another_field() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin R.SetRange(Grp, 1); R.\"No.\" := 'A'; R.SetRecFilter(); exit(R.GetFilters()); end;",
        "F",
    );
    assert_eq!(text(r), "No.: A, Grp: 1");
}

#[test]
fn insert_with_system_id_keeps_the_assigned_id_and_get_by_system_id_finds_the_row() {
    // corpus record/TestRecordInsert.al:84-110, TestRecordSystemId.al:10-60,285-330
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; F: Record \"ALT T\"; G: Guid; s: Text; begin
            G := CreateGuid();
            R.\"No.\" := 'A'; R.SystemId := G; R.Insert(false, true);
            s := Format(R.SystemId = G);
            R.\"No.\" := 'B'; R.SystemId := G; R.Insert(false, false);
            s += Format(R.SystemId = G) + Format(IsNullGuid(R.SystemId));
            s += '|' + Format(F.GetBySystemId(G)) + F.\"No.\" + Format(F.GetBySystemId(CreateGuid()));
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "YesNoNo|YesANo");
}

#[test]
fn a_duplicate_system_id_is_refused_until_the_row_is_gone() {
    // corpus record/TestRecordSystemId.al:86-168 ('unique index', the caption and the quoted value),
    // :330-400 (a deleted or deleted-all row frees its id)
    let m = error_text(
        "R.\"No.\" := 'B'; R.SystemId := CreateGuid(); R.Insert(false, true); R.\"No.\" := 'C'; R.Insert(false, true);",
    );
    assert!(m.contains("unique index"), "{m}");
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; S: Record \"ALT T\"; G: Guid; begin
            G := CreateGuid();
            R.\"No.\" := 'A'; R.SystemId := G; R.Insert(false, true);
            R.Delete();
            S.\"No.\" := 'B'; S.SystemId := G; S.Insert(false, true);
            exit(Format(R.GetBySystemId(G)) + R.\"No.\");
        end;",
        "F",
    );
    assert_eq!(text(r), "YesB");
    // The error names the field by its caption and quotes the formatted value.
    let m = text(run(
        "procedure F(): Text var R: Record \"ALT T\"; S: Record \"ALT T\"; G: Guid; begin
            G := CreateGuid();
            R.\"No.\" := 'A'; R.SystemId := G; R.Insert(false, true);
            S.\"No.\" := 'B'; S.SystemId := G; asserterror S.Insert(false, true);
            exit(GetLastErrorText() + '#' + Format(G));
        end;",
        "F",
    ));
    let (msg, g) = m.split_once('#').unwrap();
    assert!(msg.contains(&format!("the following fields: System ID='{g}'")), "{msg}");
}

#[test]
fn modify_never_changes_the_stored_system_id() {
    // corpus record/TestRecordSystemId.al:176-215,285-300: the buffer's SystemId is not written
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; F: Record \"ALT T\"; A: Guid; B: Guid; begin
            R.\"No.\" := 'A'; R.Insert(); A := R.SystemId; Clear(R);
            R.\"No.\" := 'B'; R.Insert(); B := R.SystemId;
            R.Get('A');
            R.\"No.\" := 'B'; R.Grp := 42; R.Modify();
            F.Get('B');
            exit(Format(F.SystemId = B) + Format(F.Grp) + Format(F.GetBySystemId(A)) + F.\"No.\");
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes42YesA");
}

#[test]
fn a_refused_write_hands_the_buffer_no_row_version() {
    // corpus record/TestRowVersion*: 60247 RefusedInsert_/RefusedModify_LeavesRecordWithoutRowVersion
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; D: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Insert();
            D.\"No.\" := 'A';
            asserterror begin D.Insert(); R.Count(); end;
            exit(Format(D.SystemRowVersion));
        end;",
        "F",
    );
    // Insert() without using the result fails at the next access of the table (the table has no
    // Blob field: corpus record/TestRecordRuntimeTextsAndViews.al:Record_Insert_DuplicateKey_
    // WithoutBlobField_IsDeferred_...); the buffer stays at 0.
    assert_eq!(text(r), "0");
    let r = run(
        "procedure F(): BigInteger var R: Record \"ALT T\"; D: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Insert();
            D.\"No.\" := 'A';
            if not D.Insert() then;
            exit(D.SystemRowVersion);
        end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::BigInteger(0))), "{r:?}");
}

#[test]
fn modify_all_updates_every_filtered_row_and_refuses_system_id() {
    // corpus record/TestRecordModify.al:87-142, TestRecordSystemId.al:215-270
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; C: Record \"ALT T\"; i: Integer; begin
            for i := 1 to 5 do begin R.\"No.\" := Format(i); R.Grp := i; R.Insert(); end;
            R.SetRange(Grp, 1, 3);
            R.ModifyAll(Grp, 77, true);
            C.SetRange(Grp, 77);
            exit(Format(C.Count()) + Format(R.Count()));
        end;",
        "F",
    );
    assert_eq!(text(r), "30");
    let m = error_text("R.ModifyAll(SystemId, CreateGuid());");
    assert!(m.contains("cannot change the value of the SystemId field"), "{m}");
}

#[test]
fn get_range_min_and_max_read_the_set_range() {
    // corpus record/TestFilterContracts.al:214-235, TestRecordFilter.al:436-458
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.SetRange(Grp, 5, 10);
            exit(Format(R.GetRangeMin(Grp)) + '|' + Format(R.GetRangeMax(Grp)));
        end;",
        "F",
    );
    assert_eq!(text(r), "5|10");
    // Without a filter: DB:NoFilter (corpus record/lt-get-range-filter/GetRangeFilterGroups.al: P168_GetRange_FilterOnlyInGroup2).
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin exit(Format(R.GetRangeMin(Grp))); end;",
        "F",
    );
    assert!(
        matches!(r, Err(Signal::Error(ref e)) if e.code == "DB:NoFilter"),
        "{r:?}"
    );
}

#[test]
fn marks_restrict_the_set_when_marked_only() {
    // corpus record/TestRecordFind.al:225-300, TestRecordNavContracts.al:283-302
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; i: Integer; begin
            for i := 1 to 3 do begin R.\"No.\" := Format(i); R.Insert(); end;
            R.MarkedOnly(true);
            s := Format(R.FindFirst());
            R.MarkedOnly(false);
            R.Get('2');
            R.Mark(true);
            s += Format(R.Mark());
            R.MarkedOnly(true);
            R.FindFirst();
            s += R.\"No.\" + Format(R.Count());
            R.Mark(false);
            s += Format(R.FindFirst());
            R.MarkedOnly(false);
            R.Get('1'); R.Mark(true); R.Get('3'); R.Mark(true);
            R.MarkedOnly(true);
            s += Format(R.Count());
            R.ClearMarks();
            s += Format(R.Count());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "NoYes21No20");
}

#[test]
fn a_table_of_table_type_temporary_is_temporary_for_every_variable_and_ref() {
    // corpus record (60198) TempOnly_IsTemporary_AlwaysTrue: TableType = Temporary without the modifier
    let src = r#"
table 50940 "Temp Only" { TableType = Temporary; fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 50100 C { procedure F(): Text var R: Record "Temp Only"; S: Record "Temp Only"; RR: RecordRef; begin
    R.K := 1; R.Insert();
    RR.Open(50940);
    exit(Format(R.IsTemporary) + Format(S.Count()) + Format(RR.IsTemporary())); end; }
"#;
    let l = al_testkit::compile(src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let r = s.call(oi, 0, inst, vec![]).map(|(v, _)| v);
    assert_eq!(text(r), "Yes0Yes");
}

#[test]
fn locking_isolation_and_security_settings_round_trip() {
    // corpus record/TestRecordLock.al:13-104, TestSecurityFilteringContracts.al:16-175, session/TestBCLockingContracts.al
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; RR: RecordRef; s: Text; begin
            R.LockTable(); R.LockTable(true); R.Consistent(true); R.Consistent(false);
            s := Format(R.ReadConsistency()) + Format(R.ReadPermission()) + Format(R.WritePermission()) + Format(R.RecordLevelLocking());
            s += '|' + Format(R.SecurityFiltering()) + '|' + Format(R.SecurityFiltering() = SecurityFilter::Validated);
            R.SecurityFiltering(SecurityFilter::Ignored);
            s += Format(R.SecurityFiltering() = SecurityFilter::Ignored) + Format(R.SecurityFiltering() = SecurityFilter::Filtered);
            R.SecurityFiltering(SecurityFilter::Filtered);
            s += Format(R.SecurityFiltering());
            s += '|' + Format(R.ReadIsolation());
            R.ReadIsolation(IsolationLevel::ReadCommitted);
            s += Format(R.ReadIsolation() = IsolationLevel::ReadCommitted);
            RR.Open(50900);
            RR.SecurityFiltering(SecurityFilter::Ignored);
            s += '|' + Format(RR.SecurityFiltering()) + Format(RR.ReadPermission()) + Format(RR.CountApprox());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "NoYesYesYes|Validated|YesYesNoFiltered|DefaultYes|IgnoredYes0");
}

#[test]
fn lock_table_without_waiting_fails() {
    // record/TestRecordLock.al:38 and record/lt-record-write/RcBMeta.al:415-419 pin the text and code.
    let r = run(
        "procedure F() var R: Record \"ALT T\"; begin R.LockTable(false, true); end;",
        "F",
    );
    assert!(
        matches!(r, Err(Signal::Error(ref e)) if e.code.as_str() == "AL:LockTableBadParm"
            && e.message == "The request to lock the ALT T table with a Wait parameter of FALSE is not supported."),
        "{r:?}"
    );
}

#[test]
fn count_approx_and_current_company() {
    // corpus record/TestRecordCount.al:52-76 (> 0 with rows), TestRecordChangeCompanyRows.al (CurrentCompany names the company)
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; i: Integer; begin
            for i := 1 to 3 do begin R.\"No.\" := Format(i); R.Insert(); end;
            exit(Format(R.CountApprox()) + Format(R.CurrentCompany() <> ''));
        end;",
        "F",
    );
    assert_eq!(text(r), "3Yes");
}

#[test]
fn positions_round_trip_and_set_position_restores_the_key_fields() {
    // corpus record/TestRecordExtended.al:188-206, TestRecordFilter.al:480-520, recordref/TestBCMetadataContracts.al:195-212
    let r = run(
        "procedure F(): Text var A: Record \"ALT T\"; B: Record \"ALT T\"; P: Text; s: Text; begin
            A.\"No.\" := 'K1'; A.Grp := 5; A.Insert(); A.\"No.\" := 'K2'; A.Grp := 6; A.Insert();
            A.FindFirst();
            P := A.GetPosition();
            s := P + '|' + A.GetPosition(false) + '|';
            B.FindLast();
            B.SetPosition(P);
            s += B.\"No.\" + Format(B.Grp);
            B.Find('=');
            s += '/' + Format(B.Grp);
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "No.=CONST(K1)|Field1=0(K1)|K16/5");
}

#[test]
fn transfer_fields_copies_by_field_number_and_honours_the_key_flag() {
    // corpus record/TestRecordTransferFields.al:14-95
    let r = run(
        "procedure F(): Text var S: Record \"ALT T\"; D: Record \"ALT T\"; res: Text; begin
            S.\"No.\" := 'SRC'; S.Grp := 42; S.Descr := 'hello';
            D.TransferFields(S);
            res := D.\"No.\" + Format(D.Grp) + D.Descr;
            Clear(D);
            D.\"No.\" := 'KEEP';
            D.TransferFields(S, false);
            res += '|' + D.\"No.\" + Format(D.Grp);
            Clear(D);
            D.TransferFields(S, true, true);
            res += '|' + D.\"No.\";
            exit(res);
        end;",
        "F",
    );
    assert_eq!(text(r), "SRC42hello|KEEP42|SRC");
}

// A field of what a procedure returns: Base Application's `exit(GetLocation("Location Code")."Bin Mandatory")` (Warehouse
// Receipt Header.BinCodeMandatory, 50 Tests-SCM tests). The result is a copy of the record.
#[test]
fn a_field_of_a_procedure_result_is_read() {
    let r = run(
        "procedure Get(Code: Code[20]): Record \"ALT T\" var R: Record \"ALT T\"; begin R.\"No.\" := Code; R.Grp := 7; R.Descr := 'x'; exit(R); end;
         procedure F(): Text begin exit(Get('K').\"No.\" + '|' + Format(Get('Z').Grp) + '|' + Get('Q').Descr); end;",
        "F",
    );
    assert_eq!(text(r), "K|7|x");
}

// An Enum field and an Option field with the same field number transfer by ordinal (Base Application's Sales Order
// Entity Buffer next to Sales Header: assumed, docs/probes-needed.md). An ordinal the enum lacks is refused.
#[test]
fn transfer_fields_moves_the_ordinal_between_an_enum_and_an_option_field() {
    let r = run(
        "procedure F(): Text var E: Record \"ALT EO Enum\"; O: Record \"ALT EO Option\"; res: Text; begin
            E.Id := 'E'; E.Kind := E.Kind::Item;
            O.TransferFields(E);
            res := Format(O.Kind) + '|' + Format(O.Kind, 0, 2);
            Clear(E);
            O.Kind := O.Kind::A;
            E.TransferFields(O);
            exit(res + '|' + Format(E.Kind, 0, 2));
        end;",
        "F",
    );
    // The Option holds ordinal 1 (member `A`); moving it back to the Enum has the ordinal 1, `Item`.
    assert_eq!(text(r), "A|1|1");
    let r = run(
        "procedure F() var E: Record \"ALT EO Enum\"; O: Record \"ALT EO Option\"; begin
            O.Kind := O.Kind::B; E.TransferFields(O);
        end;",
        "F",
    );
    // The ordinal moves although the Enum lacks it, with no error (corpus TransferEnumOption.al: P98_OptionToEnum_MissingOrdinal).
    assert!(r.is_ok(), "{r:?}");
}

// record/TestRecordSort.al:184-203 (SetAscending(false) makes FindFirst return the largest) and :205-.
#[test]
fn set_ascending_on_a_single_field_key_reverses_the_order() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; begin
            R.\"No.\" := 'A'; R.Amount := 5; R.Insert();
            R.\"No.\" := 'B'; R.Amount := 15; R.Insert();
            R.SetCurrentKey(Amount);
            R.SetAscending(Amount, false);
            R.FindFirst();
            s := Format(R.Amount);
            R.SetAscending(Amount, true);
            R.FindFirst();
            exit(s + '|' + Format(R.Amount));
        end;",
        "F",
    );
    assert_eq!(text(r), "15|5");
}

// record/TestRecordSort.al:158-181: GetAscending reads what SetAscending wrote; the default is true.
// `Ascending()` after a per-field SetAscending is not pinned and is refused.
#[test]
fn get_ascending_reads_the_direction_of_a_key_field() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; begin
            R.SetCurrentKey(Amount);
            s := Format(R.GetAscending(Amount));
            R.SetAscending(Amount, false);
            exit(s + '|' + Format(R.GetAscending(Amount)));
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes|No");
    let r = run(
        "procedure F(): Boolean var R: Record \"ALT T\"; begin
            R.SetCurrentKey(Amount);
            R.SetAscending(Amount, false);
            exit(R.Ascending());
        end;",
        "F",
    );
    // record/TestProbeRecordRefContracts.al:GetTable_CopiesBufferFiltersKeyDirection (`rr_ascending`): the property reads the direction set.
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
}

// A field outside the current sorting raises the pinned error (record/lt-filters-keys-finds/P68D_Ascending.al:Field_Not_In_Key);
// a field of a multi-field key has a direction of its own (same file: Composite_Key_Second_Descending).
#[test]
fn set_ascending_outside_the_sorting_raises_and_inside_a_composite_key_is_per_field() {
    for (body, text) in [
        (
            "R.SetCurrentKey(Amount); R.SetAscending(Grp, false);",
            "Cannot call SetAscending on field Grp because it is not part of the current sorting.",
        ),
        (
            "R.SetCurrentKey(Amount); if R.GetAscending(Grp) then;",
            "Cannot call GetAscending on field Grp because it is not part of the current sorting.",
        ),
    ] {
        let r = run(
            &format!("procedure F() var R: Record \"ALT T\"; begin {body} end;"),
            "F",
        );
        assert!(
            matches!(&r, Err(Signal::Error(e)) if e.message == text && e.code == "DB:Ok"),
            "{body}: {r:?}"
        );
    }
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.SetCurrentKey(Grp, \"No.\"); R.SetAscending(Grp, false);
            exit(Format(R.GetAscending(Grp)) + Format(R.GetAscending(\"No.\")) + Format(R.Ascending()));
        end;",
        "F",
    );
    assert_eq!(text(r), "NoYesNo");
}

// A key value whose Format is not pinned (a Duration) used to panic the whole codeunit when a failed
// write named the row; it is a refusal the test can report instead.
#[test]
fn a_duplicate_key_of_a_duration_names_the_value_in_words() {
    let src = r#"
table 50902 "ALT Dur" { fields { field(1; Span; Duration) { } } keys { key(PK; Span) { } } }
codeunit 50100 C { procedure F() var R: Record "ALT Dur"; begin
    R.Span := 5000; R.Insert(); R.Insert(); R.Count();
end; }"#;
    let l = al_testkit::compile(src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    // The key value is spelled with Format (types/TestFormatEvaluateTextContracts.al:Format_Duration_StandardFormats).
    let r = s.call(oi, 0, inst, vec![]);
    assert!(
        matches!(&r, Err(Signal::Error(e)) if e.message.ends_with("Span='5 seconds'")),
        "{r:?}"
    );
}

#[test]
fn set_current_key_on_fields_no_key_starts_with_orders_by_them_then_the_primary_key() {
    // record/TestRecordNavContracts.al:180-195 (SetCurrentKey on a non-key field does not throw and
    // the record still finds), recordref/TestRecordRefRuntimeContracts.al:115-143 (CurrentKey is
    // just `Text Field`; GetTable and SetTable carry the key and the direction),
    // xmlport/TestXmlPortTriggersReplaceViewContracts.al:339-375 (ascending: by the field, ties by
    // primary key; Ascending(false) reverses both).
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; C: Record \"ALT T\"; RR: RecordRef; s: Text; begin
            R.\"No.\" := 'A'; R.Descr := 'k'; R.Insert();
            R.\"No.\" := 'B'; R.Descr := ''; R.Insert();
            R.\"No.\" := 'C'; R.Descr := ''; R.Insert();
            R.Reset();
            R.SetCurrentKey(Descr);
            s := R.CurrentKey() + '|';
            if R.FindSet() then repeat s += R.\"No.\"; until R.Next() = 0;
            R.Ascending(false);
            s += '|';
            if R.FindSet() then repeat s += R.\"No.\"; until R.Next() = 0;
            RR.GetTable(R);
            RR.SetTable(C);
            s += '|' + C.CurrentKey() + Format(C.Ascending()) + '|';
            if C.FindSet() then repeat s += C.\"No.\"; until C.Next() = 0;
            R.Reset();
            s += '|' + R.CurrentKey();
            if R.FindSet() then repeat s += R.\"No.\"; until R.Next() = 0;
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Descr|BCA|ACB|DescrNo|ACB|No.ABC");
}

#[test]
fn set_ascending_true_after_ascending_false_keeps_ties_in_descending_primary_key_order() {
    // record/lt-record-sort-ascending/RecordSortAscendingViews.al:P161_SetAscending_Single_AscendingMethodAndTies `back.walk`.
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; begin
            R.\"No.\" := 'A'; R.Descr := 'k'; R.Insert();
            R.\"No.\" := 'B'; R.Descr := ''; R.Insert();
            R.\"No.\" := 'C'; R.Descr := ''; R.Insert();
            R.Reset();
            R.SetCurrentKey(Descr);
            R.Ascending(false);
            R.SetAscending(Descr, true);
            if R.FindSet() then repeat s += R.\"No.\"; until R.Next() = 0;
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "CBA");
}

#[test]
fn view_and_ascending_after_set_ascending_true_following_ascending_false_ignore_the_tie_order() {
    // record/lt-record-sort-ascending/RecordSortAscendingViews.al:P161_SetAscending_Single_AscendingMethodAndTies `back.asc`, `back.view`.
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.SetCurrentKey(Descr);
            R.Ascending(false);
            R.SetAscending(Descr, true);
            exit(Format(R.Ascending()) + '|' + R.GetView());
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes|VERSION(1) SORTING(Descr)");
}

#[test]
fn set_current_key_on_a_field_that_is_not_stored_is_refused() {
    let r = run(
        "procedure F() var R: Record \"ALT T\"; begin R.SetCurrentKey(Total); end;",
        "F",
    );
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("not pinned"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn testfield_on_the_blank_member_of_an_option_or_enum_says_must_not_be_the_caption() {
    // record/TestTestFieldOptionEnumContracts.al:TestField_ZeroMember_RaisesMustNotBeTheCaption.
    for field in ["Kind", "Kind2", "Kind3"] {
        let r = run(
            &format!("procedure F() var R: Record \"ALT Blank\"; begin R.Id := 'A'; R.TestField({field}); end;"),
            "F",
        );
        assert!(
            matches!(&r, Err(Signal::Error(e)) if e.message.contains("must not be") && e.code == "TestField"),
            "{field}: {r:?}"
        );
    }
    // A non-blank member passes.
    let r = run(
        "procedure F(): Text var R: Record \"ALT Blank\"; begin R.Kind := R.Kind::B; R.TestField(Kind); exit('ok'); end;",
        "F",
    );
    assert_eq!(text(r), "ok");
}

#[test]
fn field_active_is_true_for_an_enabled_field() {
    // corpus record/TestRecordTriggers.al:223-231 (Record_FieldActive_EnabledField_ReturnsTrue)
    let r = run(
        "procedure F(): Boolean var R: Record \"ALT T\"; begin exit(R.FieldActive(Amount) and R.FieldActive(Grp)); end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Boolean(true))), "{r:?}");
}

#[test]
fn field_active_of_a_disabled_field_is_false() {
    // record/TestRowVersionFieldNumberFieldActiveContracts.al:FieldActive_FalseForADisabledField_TrueForPendingObsolete.
    let src = r#"
table 50935 T2 { fields { field(1; A; Integer) { } field(2; B; Integer) { Enabled = false; } } keys { key(PK; A) { } } }
codeunit 50101 C2 { procedure F(): Boolean var R: Record T2; begin exit(R.FieldActive(B)); end; }
"#;
    let l = al_testkit::compile(src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C2").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let r = s.call(oi, 0, inst, vec![]).map(|(v, _)| v);
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
}

#[test]
fn special_filter_values_are_quoted_escaped_and_round_trip() {
    // TestFilterQuotedLiterals.al:95-105 pins doubled apostrophes in quoted literals;
    // TestSetFilterSpecialTextContracts.al:94-103 and filters/W4Filters.al:174-191 pin operator quoting.
    for (value, quoted) in [
        ("Diana's (Meals)", "'Diana''s (Meals)'"),
        ("'A|B'", "'''A|B'''"),
        ("A'&B", "'A''&B'"),
        ("A'=B", "'A''=B'"),
        ("A'..B", "'A''..B'"),
        ("A'<B", "'A''<B'"),
        ("a>b", "'a>b'"),
        (">b", "'>b'"),
        ("a>", "'a>'"),
        ("<64Y>", "'<64Y>'"),
    ] {
        let literal = value.replace('\'', "''");
        for setup in [
            format!("R.SetRange(Descr, '{literal}');"),
            format!("R.SetFilter(Descr, '%1', '{literal}');"),
        ] {
            let r = run(
                &format!(
                    r#"procedure F(): Text var R: Record "ALT T"; Other: Record "ALT T"; FilterText: Text; ViewText: Text;
                    begin
                        R."No." := 'MATCH'; R.Descr := '{literal}'; R.Insert();
                        R."No." := 'OTHER'; R.Descr := 'not the value'; R.Insert();
                        {setup}
                        FilterText := R.GetFilter(Descr);
                        ViewText := R.GetView();
                        Other.SetFilter(Descr, FilterText);
                        if Other.Count() <> 1 then Error('Filter does not round trip');
                        Other.FindFirst();
                        if Other."No." <> 'MATCH' then Error('Filter selects the wrong row');
                        Other.Reset();
                        Other.SetView(ViewText);
                        if Other.Count() <> 1 then Error('View does not round trip');
                        exit(FilterText + '|' + R.GetFilters());
                    end;"#
                ),
                "F",
            );
            assert_eq!(text(r), format!("{quoted}|Descr: {quoted}"), "{setup}");
        }
    }
}

#[test]
fn code_filter_with_angle_brackets_round_trips_through_a_view() {
    let r = run(
        r#"procedure F(): Text var R: Record "ALT T"; Other: Record "ALT T"; begin
            R."No." := '<64Y>'; R.Insert();
            R."No." := 'OTHER'; R.Insert();
            R.SetRange("No.", '<64Y>');
            Other.SetView(R.GetView());
            if Other.Count() <> 1 then Error('Code view does not round trip');
            Other.FindFirst();
            if Other."No." <> '<64Y>' then Error('Wrong code selected');
            exit(R.GetFilter("No."));
        end;"#,
        "F",
    );
    assert_eq!(text(r), "'<64Y>'");
}

// A parenthesis inside a quoted value does not count as a group of the view: an unmatched ( or ) round trips
// (the quoting of a value with ( or ) is record/TestSetFilterSpecialTextContracts.al, filters/lt-uc-quoted-values/
// UcQuotedValues.al:Value05_ApostropheAndParens).
#[test]
fn a_value_with_an_unmatched_parenthesis_round_trips_through_a_view() {
    for (value, quoted) in [
        ("Diana's (Meals", "'Diana''s (Meals'"),
        ("A>B)", "'A>B)'"),
        ("(x", "'(x'"),
        ("x)", "'x)'"),
    ] {
        let literal = value.replace('\'', "''");
        for setup in [
            format!("R.SetRange(Descr, '{literal}');"),
            format!("R.SetFilter(Descr, '%1', '{literal}');"),
        ] {
            let r = run(
                &format!(
                    r#"procedure F(): Text var R: Record "ALT T"; Other: Record "ALT T"; ViewText: Text;
                    begin
                        R."No." := 'MATCH'; R.Descr := '{literal}'; R.Insert();
                        R."No." := 'OTHER'; R.Descr := 'not the value'; R.Insert();
                        {setup}
                        ViewText := R.GetView();
                        Other.SetView(ViewText);
                        if Other.Count() <> 1 then Error('View does not round trip');
                        Other.FindFirst();
                        if Other."No." <> 'MATCH' then Error('View selects the wrong row');
                        exit(Other.GetFilter(Descr));
                    end;"#
                ),
                "F",
            );
            assert_eq!(text(r), quoted, "{setup}");
        }
    }
}

// filters/lt-uc-quoted-values/UcQuotedValues.al:SetView_FilterGreaterB: in a view given to SetView, FILTER('>B') is
// the literal value (`view_cd_ids` 2,16, GetFilter `'>B'`) and FILTER(>B) a comparison (`view_cd_plain_ids` 5,14,15).
#[test]
fn set_view_reads_a_quoted_greater_than_as_a_literal_and_a_bare_one_as_a_comparison() {
    let r = run(
        r#"procedure F(): Text var R: Record "ALT T"; Res: Text;
        begin
            R."No." := 'B'; R.Descr := 'B'; R.Insert();
            R."No." := 'A'; R.Descr := 'A'; R.Insert();
            R."No." := 'C'; R.Descr := 'C'; R.Insert();
            R."No." := '>B'; R.Descr := '>b'; R.Insert();
            R.SetView('WHERE("No."=FILTER(''>B''))');
            Res := Format(R.Count()) + ':' + R.GetFilter("No.") + ';';
            R.Reset();
            R.SetView('WHERE(Descr=FILTER(''>b''))');
            Res += Format(R.Count()) + ':' + R.GetFilter(Descr) + ';';
            R.Reset();
            R.SetView('WHERE("No."=FILTER(>B))');
            Res += Format(R.Count()) + ';';
            R.Reset();
            R.SetView('WHERE(Descr=FILTER(>B))');
            Res += Format(R.Count());
            exit(Res);
        end;"#,
        "F",
    );
    assert_eq!(text(r), "1:'>B';1:'>b';1;1");
}

// record/TestSetFilterSpecialTextContracts.al pins a case-insensitive `@` value with & | ( ) < = or .. read back
// quoted, `@` inside the quotes; one with > is not pinned.
#[test]
fn setfilter_placeholder_case_insensitive_text_with_a_greater_than_is_refused() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.SetFilter(Descr, '%1', '@a>b'); exit(R.GetFilter(Descr));
        end;",
        "F",
    );
    assert!(matches!(&r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn setfilter_placeholder_text_with_an_unpinned_filter_character_is_refused() {
    // Wildcards mixed with operators remain outside this implementation.
    for filter_value in ["'a*&b'", "'a*>b'"] {
        let r = run(
            &format!(
                "procedure F(): Integer var R: Record \"ALT T\"; begin
                    R.SetFilter(Descr, '%1', {filter_value}); exit(R.Count());
                end;"
            ),
            "F",
        );
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("filter character")),
            "{filter_value}: {r:?}"
        );
    }
    // A plain text is an exact match, as before.
    let r = run(
        "procedure F(): Integer var R: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Descr := 'plain text'; R.Insert();
            R.SetFilter(Descr, '%1', 'plain text'); exit(R.Count());
        end;",
        "F",
    );
    assert_eq!(text_or_int(r), 1);
}

// RecordRef.GetBySystemId, as probed on BC 27.5: corpus branch al-runtime-rs-staging (ae3fe88),
// tests/al-language/types/PXEnumRecordRef.al:199-246 (found loads every field, a missing or null
// Guid answers false and leaves the buffer), :249-270 (the result unused: a miss raises
// DB:RecordNotFound), :273-301 (found outside the filters, the filters stay; a RecordRef never
// opened answers false and Number() 0).
#[test]
fn record_ref_get_by_system_id_loads_the_row_and_a_miss_keeps_the_buffer() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; RR: RecordRef; Id: Guid; s: Text; begin
            R.\"No.\" := 'A'; R.Descr := 'first'; R.Amount := 1.5; R.Insert(); Id := R.SystemId;
            R.\"No.\" := 'B'; R.Descr := 'second'; R.Amount := 2.5; R.Insert();
            RR.Open(Database::\"ALT T\");
            s := Format(RR.GetBySystemId(Id)) + Format(RR.Field(1).Value) + Format(RR.Field(4).Value) + Format(RR.Field(2).Value);
            s += Format(Format(RR.Field(RR.SystemIdNo()).Value) = Format(Id)) + Format(RR.Count());
            s += '|' + Format(RR.GetBySystemId(CreateGuid())) + Format(RR.Field(1).Value) + Format(RR.Field(4).Value);
            s += '|' + Format(RR.GetBySystemId('00000000-0000-0000-0000-000000000000'));
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "YesAfirst1.5Yes2|NoAfirst|No");
}

#[test]
fn record_ref_get_by_system_id_with_the_result_unused_raises_on_a_miss() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; RR: RecordRef; Id: Guid; G: Guid; s: Text; begin
            R.\"No.\" := 'A'; R.Insert(); Id := R.SystemId;
            RR.Open(Database::\"ALT T\");
            RR.GetBySystemId(Id);
            s := Format(RR.Field(1).Value);
            G := CreateGuid();
            asserterror RR.GetBySystemId(G);
            exit(s + '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(G));
        end;",
        "F",
    );
    let t = text(r);
    let parts: Vec<&str> = t.split('|').collect();
    assert_eq!(parts[0], "A");
    assert_eq!(
        parts[1],
        format!(
            "The The Table does not exist. Identification fields and values: {}",
            parts[3]
        )
    );
    assert_eq!(parts[2], "DB:RecordNotFound");
}

#[test]
fn record_ref_get_by_system_id_ignores_the_filters_and_a_never_opened_ref_answers_false() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; RR: RecordRef; Closed: RecordRef; Id: Guid; s: Text; begin
            R.\"No.\" := 'A'; R.Descr := 'x'; R.Insert(); Id := R.SystemId;
            RR.Open(Database::\"ALT T\");
            RR.Field(4).SetRange('other');
            s := Format(RR.GetBySystemId(Id)) + '|' + RR.GetFilters();
            s += '|' + Format(Closed.GetBySystemId(Id)) + Format(Closed.Number());
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes|Descr: other|No0");
}

// The direction characters of Find are tried in the order written, and the first that finds a row wins
// (types/PYLoadFindThis.al:Find_DirectionCharacters_Orders, all eleven orders on BC 27.5 and 29.0): `=<>` takes the
// row itself, `<>=` the row before it even when the key exists.
#[test]
fn find_tries_the_directions_in_the_order_written() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; begin
            R.\"No.\" := 'A'; R.Insert(); R.\"No.\" := 'C'; R.Insert();
            R.\"No.\" := 'A'; s := Format(R.Find('=<>')) + R.\"No.\";
            R.\"No.\" := 'C'; s += Format(R.Find('<>=')) + R.\"No.\";
            exit(s);
        end;",
        "F",
    );
    assert_eq!(text(r), "YesAYesA");
}

// record/lt-filters-keys-finds/P68C_EmptyFinds.al:215-250 (FindFirst, FindLast; the BC 27 oracle passes it too), ErTTriggerErrors.al:140-147
// (FindSet): an unfiltered, empty table.
#[test]
fn find_first_last_and_set_on_an_empty_unfiltered_table_say_the_table_is_empty() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; begin
            R.DeleteAll();
            asserterror R.FindFirst(); s := GetLastErrorText() + '|' + GetLastErrorCode();
            asserterror R.FindLast(); s += '#' + GetLastErrorText() + '|' + GetLastErrorCode();
            asserterror R.FindSet(); s += '#' + GetLastErrorText() + '|' + GetLastErrorCode();
            exit(s);
        end;",
        "F",
    );
    let one = "The The Table table is empty.|DB:TableIsEmpty"; // the test table has no caption of its own
    assert_eq!(text(r), format!("{one}#{one}#{one}"));
}

// record/lt-find-empty-table/FindEmptyTable.al:Find_EmptyTable: Find with the result unused on an empty, unfiltered table.
#[test]
fn find_on_an_empty_unfiltered_table_with_an_unused_result_raises_the_pinned_errors() {
    let one = |call: &str| {
        format!("R.DeleteAll(); asserterror {call}; s += GetLastErrorText() + '|' + GetLastErrorCode() + '#';")
    };
    let body = format!(
        "procedure F(): Text var R: Record \"ALT T\"; s: Text; begin {} {} {} {} {} {} {} exit(s); end;",
        one("R.Find()"),
        one("R.Find('=')"),
        one("R.Find('-')"),
        one("R.Find('+')"),
        one("R.Find('>')"),
        one("R.Find('<')"),
        one("R.Find('=><')"),
    );
    let r = run(&body, "F");
    let id = "Identification fields and values:";
    let nf = |w: &str| format!("The The Table does not exist. {w}\\\\{id}\\\\No.=''|DB:RecordNotFound#");
    let eq = format!("The The Table does not exist. {id} No.=''|DB:RecordNotFound#");
    let empty = "The The Table table is empty.|DB:TableIsEmpty#";
    assert_eq!(
        text(r),
        format!("{eq}{eq}{empty}{empty}{}{}{}", nf(">"), nf("<"), nf("=><"))
    );
}

// Find of a missing key in a table that has rows, and on an empty table with a key value other than the default: the
// record's key in the text (writer TV, corpus branch al-runtime-rs-staging-tv, FindEmptyTable.al TV_Find_TableWithRows_NoMatch,
// TV_Find_EmptyTable_KeyNotDefault; assumption find-miss-key-text).
#[test]
fn find_of_a_missing_key_in_a_table_with_rows_names_the_key() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.DeleteAll(); R.\"No.\" := 'B'; R.Insert(); R.\"No.\" := 'A'; asserterror R.Find();
            exit(GetLastErrorText() + '|' + GetLastErrorCode());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "The The Table does not exist. Identification fields and values: No.='A'|DB:RecordNotFound"
    );
}

#[test]
fn find_on_an_empty_table_with_a_non_default_key_names_the_key() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.DeleteAll(); R.\"No.\" := 'A'; asserterror R.Find('>');
            exit(GetLastErrorText() + '|' + GetLastErrorCode());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "The The Table does not exist. >\\\\Identification fields and values:\\\\No.='A'|DB:RecordNotFound"
    );
}
