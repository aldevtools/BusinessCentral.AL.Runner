//! Table captions, RecordId texts and caption classes (lane T4 of the 2026-10-20 failure triage, bump 29).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

fn refused(r: Result<Value, Signal>, needle: &str) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal that names `{needle}`, got {other:?}"),
    }
}

/// `Format(RecordId, 0, 1)` names the table by its caption; no number, 0, 2 and 9 name it by its name
/// (record/lt-record-write/RcBMeta.al:556-579).
#[test]
fn format_1_of_a_record_id_names_the_table_caption() {
    let src = "table 50100 \"LT Meta\" { Caption = 'Meta Caption'; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        codeunit 50100 C { procedure F(): Text var R: Record \"LT Meta\"; Id: RecordId; Back: RecordId; begin R.Id := 7; Id := R.RecordId(); \
        exit(Format(Id) + '|' + Format(Id, 0, 0) + '|' + Format(Id, 0, 1) + '|' + Format(Id, 0, 2) + '|' + Format(Id, 0, 9) + '|' + Format(Evaluate(Back, Format(Id, 0, 1)))); end; }";
    assert_eq!(
        text(al_testkit::run_in(src, "C", "F")),
        "LT Meta: 7|LT Meta: 7|Meta Caption: 7|LT Meta: 7|LT Meta: 7|No"
    );
}

/// `Format(RecordId, 0, 9)` of a table in a namespace writes `[<namespace>.<name>]`, and Evaluate reads that text back; the
/// other numbers write the plain name (misc/lt-inferred-behaviors/P69NsConsumers.al:194-217).
#[test]
fn format_9_of_a_record_id_of_a_namespaced_table_names_it_qualified() {
    let src = "namespace LT.Data; table 50100 \"LT Ns Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        codeunit 50100 C { procedure F(): Text var R: Record \"LT Ns Row\"; Id: RecordId; Back: RecordId; Ok: Boolean; begin R.Id := 2; Id := R.RecordId(); \
        Ok := Evaluate(Back, Format(Id, 0, 9)); \
        exit(Format(Id, 0, 1) + '|' + Format(Id, 0, 9) + '|' + Format(Ok) + '|' + Format(Back = Id) + '|' + Format(Evaluate(Back, 'LT.Data.LT Ns Row: 2'))); end; }";
    assert_eq!(
        text(al_testkit::run_in(src, "C", "F")),
        "LT Ns Row: 2|[LT.Data.LT Ns Row]: 2|Yes|Yes|No"
    );
}

/// Only a namespaced table whose name holds a space is pinned for format 9: the brackets around a name without one are refused.
#[test]
fn format_9_of_a_record_id_of_a_namespaced_table_without_a_space_is_refused() {
    let src = "namespace LT.Data; table 50100 LTRow { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        codeunit 50100 C { procedure F(): Text var R: Record LTRow; begin R.Id := 2; exit(Format(R.RecordId(), 0, 9)); end; }";
    refused(al_testkit::run_in(src, "C", "F"), "namespace");
}

/// A tableextension's Caption is the caption of the table it extends when that table declares none
/// (tableextension/TestTableExtFieldModify.al:168-178).
#[test]
fn a_table_extension_caption_is_the_table_caption() {
    let src = "table 50100 Base { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        tableextension 50101 BaseExt extends Base { Caption = 'Ext Caption'; } \
        codeunit 50100 C { procedure F(): Text var R: Record Base; RR: RecordRef; begin RR.Open(Database::Base); exit(R.TableCaption() + '|' + RR.Caption()); end; }";
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "Ext Caption|Ext Caption");
}

/// One extension's Caption over the table's own wins (corpus writer Y5, branch al-runtime-rs-staging-y5,
/// tableextension/lt-y5/Y5TestFieldExtCaption.al, al-runner issue 6378). Which one wins when two extensions set one is not
/// pinned: refused.
#[test]
fn a_table_extension_caption_over_another_caption_is_refused() {
    let both = "table 50100 Base { Caption = 'Base Caption'; fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        tableextension 50101 BaseExt extends Base { Caption = 'Ext Caption'; } \
        codeunit 50100 C { procedure F(): Text var R: Record Base; begin exit(R.TableCaption()); end; }";
    assert_eq!(text(al_testkit::run_in(both, "C", "F")), "Ext Caption");
    let two = "table 50100 Base { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        tableextension 50101 ExtA extends Base { Caption = 'A Caption'; } \
        tableextension 50102 ExtB extends Base { Caption = 'B Caption'; } \
        codeunit 50100 C { procedure F(): Text var R: Record Base; begin exit(R.TableCaption()); end; }";
    refused(al_testkit::run_in(two, "C", "F"), "caption");
}

const CAPTION_SUBSCRIBER: &str = "codeunit 50101 Sub { SingleInstance = true; var Log: Text; \
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', false, false)] \
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) \
    begin Log += 'resolve(' + CaptionArea + ')'; if CaptionArea = 'PS' then begin Caption := 'resolved:' + CaptionExpr; Resolved := true; end; end; \
    procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end; } \
    codeunit 42 \"Caption Class\" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }";

/// CaptionClassTranslate keeps what it translated for the session: the same text a second time raises no
/// OnResolveCaptionClass, claimed or not; after SelectLatestVersion it does again (types/PSCaptionClass.al:94-115).
#[test]
fn caption_class_translate_keeps_its_translations_until_select_latest_version() {
    let src = format!(
        "{CAPTION_SUBSCRIBER} codeunit 50100 C {{ procedure F(): Text var S: Codeunit Sub; R: Text; begin S.Take(); \
        R := CaptionClassTranslate('PS,one') + '/' + S.Take() + '|'; \
        R += CaptionClassTranslate('PS,one') + '/' + S.Take() + '|'; \
        R += CaptionClassTranslate('ZZ,two') + '/' + S.Take() + '|'; \
        R += CaptionClassTranslate('ZZ,two') + '/' + S.Take() + '|'; \
        SelectLatestVersion(); \
        R += CaptionClassTranslate('PS,one') + '/' + S.Take(); exit(R); end; }}"
    );
    assert_eq!(
        text(al_testkit::run_in(&src, "C", "F")),
        "resolved:one/resolve(PS)|resolved:one/|ZZ,two/resolve(ZZ)|ZZ,two/|resolved:one/resolve(PS)"
    );
}

/// Whether the kept translation holds after GlobalLanguage changes is not pinned: refused.
#[test]
fn caption_class_translate_after_a_language_change_is_refused() {
    let src = format!(
        "{CAPTION_SUBSCRIBER} codeunit 50100 C {{ procedure F(): Text var R: Text; begin R := CaptionClassTranslate('PS,one'); \
        GlobalLanguage(1031); exit(CaptionClassTranslate('PS,one')); end; }}"
    );
    refused(al_testkit::run_in(&src, "C", "F"), "GlobalLanguage");
}

/// A table field whose CaptionClass is an expression (a procedure call) is evaluated in the record's context, and its
/// `3,<text>` result reads as <text> for FieldCaption and for the caption of a page control over the field: a `modify`
/// block with FieldNo as the argument and a new field whose expression is a procedure without parentheses
/// (pageext/lt-page-ext-properties/TableExtCaptionClass.al:56-74, BP_TableExtCaptionClass_ProcedureCall :124-149).
const EXPR_CAPTION_CLASS: &str = "table 50100 Base { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
    tableextension 50101 BaseExt extends Base { fields { modify(Id) { CaptionClass = Cc(FieldNo(Id)); } \
    field(50101; Style; Code[20]) { CaptionClass = Cc50101; } } \
    procedure Cc(No: Integer): Text[80] begin exit('3,Stand in ' + Format(No)); end; \
    procedure Cc50101(): Text begin exit(Cc(50101)); end; } \
    page 50102 P { PageType = Card; SourceTable = Base; layout { area(Content) { field(Id; Rec.Id) { } field(StyleCtl; Rec.Style) { } } } } \
    codeunit 50100 C { procedure F(): Text var R: Record Base; begin exit(R.FieldCaption(Id) + '|' + R.FieldCaption(Style)); end; \
    procedure G(): Text var R: Record Base; P: TestPage P; begin R.Id := 1; R.Insert(); P.OpenView(); exit(P.Id.Caption() + '|' + P.StyleCtl.Caption()); end; \
    procedure H(): Text var Ref: RecordRef; begin Ref.Open(Database::Base); exit(Ref.Field(1).Caption()); end; }";

#[test]
fn a_caption_class_expression_on_a_table_field_is_evaluated() {
    assert_eq!(
        text(al_testkit::run_in(EXPR_CAPTION_CLASS, "C", "F")),
        "Stand in 1|Stand in 50101"
    );
    assert_eq!(
        text(al_testkit::run_in(EXPR_CAPTION_CLASS, "C", "G")),
        "Stand in 1|Stand in 50101"
    );
    assert_eq!(text(al_testkit::run_in(EXPR_CAPTION_CLASS, "C", "H")), "Stand in 1");
}

/// The expression of a table's own field runs with the record that reads the caption as Rec (Base Application's
/// `Sales Line`.`No.`: `CaptionClass = GetCaptionClass(FieldNo("No."))`).
#[test]
fn a_caption_class_expression_of_a_tables_own_field_sees_the_record() {
    let src = "table 50100 T { fields { field(1; Id; Integer) { } field(2; \"No.\"; Code[20]) { Caption = 'No.'; CaptionClass = GetCaptionClass(FieldNo(\"No.\")); } } \
        keys { key(PK; Id) { } } \
        procedure GetCaptionClass(FieldNumber: Integer): Text[80] begin exit('3,' + Format(Id) + '/' + Format(FieldNumber)); end; } \
        codeunit 50100 C { procedure F(): Text var R: Record T; begin R.Id := 7; exit(R.FieldCaption(\"No.\")); end; }";
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "7/2");
}

/// An expression result reads as a literal does: a numeric area nobody resolves stays as written, `3,` reads as the field's
/// Caption, here its name (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:262-296 `expr9`, `expr3`). An area that is
/// not a number is refused.
#[test]
fn a_caption_class_expression_with_another_area_reads_as_a_literal() {
    let src = "table 50100 T { fields { field(1; Id; Integer) { } field(2; Amt; Decimal) { CaptionClass = Cc(); } \
        field(3; Empty; Decimal) { CaptionClass = Blank(); } } keys { key(PK; Id) { } } \
        procedure Cc(): Text begin exit('2,0,Amount'); end; procedure Blank(): Text begin exit('3,'); end; } \
        codeunit 50100 C { procedure F(): Text var R: Record T; begin exit(R.FieldCaption(Amt)); end; \
        procedure G(): Text var R: Record T; begin exit(R.FieldCaption(Empty)); end; \
        procedure H(): Text var R: Record T; begin exit(R.FieldCaption(Id)); end; }";
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "2,0,Amount");
    assert_eq!(text(al_testkit::run_in(src, "C", "G")), "Empty");
    assert_eq!(text(al_testkit::run_in(src, "C", "H")), "Id");
    let other = src.replace("exit('2,0,Amount')", "exit('AB,Amount')");
    refused(al_testkit::run_in(&other, "C", "F"), "CaptionClass");
}

/// An error text that names a table whose caption is refused (two tableextensions set one) is not pinned: the error is
/// raised, and a read of its text or code refuses.
#[test]
fn an_error_that_names_a_refused_table_caption_is_unpinned() {
    let src = "table 50100 Base { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        tableextension 50101 ExtA extends Base { Caption = 'A Caption'; } \
        tableextension 50102 ExtB extends Base { Caption = 'B Caption'; } \
        codeunit 50100 C { procedure F(): Text var R: Record Base; begin asserterror R.Get(5); exit(GetLastErrorText()); end; \
        procedure G(): Text var R: Record Base; begin asserterror R.Get(5); exit(GetLastErrorCode()); end; \
        procedure H(): Boolean var R: Record Base; begin exit(R.Get(5)); end; }";
    refused(al_testkit::run_in(src, "C", "F"), "not pinned");
    refused(al_testkit::run_in(src, "C", "G"), "not pinned");
    assert!(matches!(al_testkit::run_in(src, "C", "H"), Ok(Value::Boolean(false))));
}

/// Which CaptionClass BC reads when two table extensions set one on the same field, or an extension sets one over the
/// base field's expression, is not pinned: refused where the caption is read.
#[test]
fn a_caption_class_set_twice_with_an_expression_is_refused() {
    let two_exts = "table 50100 Base { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } } \
        tableextension 50101 E1 extends Base { fields { modify(Id) { CaptionClass = A(); } } procedure A(): Text begin exit('3,A'); end; } \
        tableextension 50102 E2 extends Base { fields { modify(Id) { CaptionClass = B(); } } procedure B(): Text begin exit('3,B'); end; } \
        page 50103 P { PageType = Card; SourceTable = Base; layout { area(Content) { field(Id; Rec.Id) { } } } } \
        codeunit 50100 C { procedure F(): Text var R: Record Base; begin exit(R.FieldCaption(Id)); end; \
        procedure G(): Text var R: Record Base; P: TestPage P; begin R.Id := 1; R.Insert(); P.OpenView(); exit(P.Id.Caption()); end; }";
    refused(al_testkit::run_in(two_exts, "C", "F"), "CaptionClass");
    refused(al_testkit::run_in(two_exts, "C", "G"), "CaptionClass");
    let over_base = "table 50100 Base { fields { field(1; Id; Integer) { CaptionClass = A(); } } keys { key(PK; Id) { } } \
        procedure A(): Text begin exit('3,A'); end; } \
        tableextension 50101 E1 extends Base { fields { modify(Id) { CaptionClass = '3,Lit'; } } } \
        codeunit 50100 C { procedure F(): Text var R: Record Base; begin exit(R.FieldCaption(Id)); end; }";
    refused(al_testkit::run_in(over_base, "C", "F"), "CaptionClass");
}
