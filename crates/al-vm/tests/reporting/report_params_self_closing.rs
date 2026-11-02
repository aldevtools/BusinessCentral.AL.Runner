//! Report parameters with empty options: a self-closing `<Field name="X" />` or an empty `<Field name="X"></Field>` sets the
//! variable to its blank value, over what the request page's OnInit set; the other types' filled forms; the root and Options
//! forms without fields; a SaveAs with parameters after the request page's OnInit
//! (report/lt-y68-selfclosing-params/Y68SelfClosingParams.al, corpus master 0d01b0cc).

use al_builtins::native::Signal;

const SRC: &str = r#"
enum 91590 "RSC Enum" { Extensible = true; value(0; First) { } value(5; Second) { } }
report 91591 "RSC Report"
{
    ProcessingOnly = true;
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(TxtCtl; TxtVal) { }
                field(CodeCtl; CodeVal) { }
                field(DateCtl; DateVal) { }
                field(TimeCtl; TimeVal) { }
                field(DtCtl; DtVal) { }
                field(IntCtl; IntVal) { }
                field(DecCtl; DecVal) { }
                field(BoolCtl; BoolVal) { }
                field(OptCtl; OptVal) { }
                field(EnumCtl; EnumVal) { }
                field(BigCtl; BigVal) { }
            }
        }
        trigger OnInit()
        begin
            TxtVal := 'tinit';
            CodeVal := 'CINIT';
            DateVal := DMY2Date(15, 1, 2024);
            TimeVal := 101500T;
            DtVal := CreateDateTime(DMY2Date(2, 3, 2024), 080910T);
            IntVal := 7;
            DecVal := 2.5;
            BoolVal := true;
            OptVal := OptVal::Gamma;
            EnumVal := EnumVal::Second;
            BigVal := 99;
        end;
    }
    var
        TxtVal: Text[30];
        CodeVal: Code[10];
        DateVal: Date;
        TimeVal: Time;
        DtVal: DateTime;
        IntVal: Integer;
        DecVal: Decimal;
        BoolVal: Boolean;
        OptVal: Option Alpha,Beta,Gamma;
        EnumVal: Enum "RSC Enum";
        BigVal: BigInteger;
        AtPre: Text;
    trigger OnPreReport()
    begin
        AtPre := 't=[' + TxtVal + '] c=[' + CodeVal + '] d=[' + Format(DateVal, 0, 9) + '] tm=[' + Format(TimeVal, 0, 9) + '] dt=[' + Format(DtVal, 0, 9) +
            '] i=' + Format(IntVal) + ' dc=' + Format(DecVal) + ' b=' + Format(BoolVal) + ' o=' + Format(OptVal) + ' e=' + Format(EnumVal) + ' bg=' + Format(BigVal);
    end;
    procedure Result(): Text begin exit(AtPre); end;
}
report 91592 "RSC Data"
{
    DefaultLayout = RDLC;
    dataset { dataitem(Num; "RSC Blob") { column(N; Id) { } } }
    requestpage
    {
        layout { area(Content) { field(TxtCtl; TxtVal) { } field(IntCtl; IntVal) { } } }
        trigger OnInit() begin TxtVal := 'tinit'; IntVal := 7; end;
    }
    var
        TxtVal: Text[30];
        IntVal: Integer;
        AtPre: Text;
    trigger OnPreReport() begin AtPre := 't=[' + TxtVal + '] i=' + Format(IntVal); end;
    procedure Result(): Text begin exit(AtPre); end;
}
table 91594 "RSC Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 91593 C
{
    local procedure Run(Options: Text): Text
    var
        A: Report "RSC Report";
    begin
        A.Execute('<?xml version="1.0" standalone="yes"?><ReportParameters name="RSC Report" id="91591"><Options>' + Options + '</Options></ReportParameters>');
        exit(A.Result());
    end;
    local procedure RunRaw(Xml: Text): Text
    var
        A: Report "RSC Report";
    begin
        A.Execute(Xml);
        exit(A.Result());
    end;
    procedure SelfClosingAll(): Text
    begin
        exit(Run('<Field name="TxtVal" /><Field name="CodeVal" /><Field name="DateVal" /><Field name="TimeVal" /><Field name="DtVal" />' +
            '<Field name="IntVal" /><Field name="DecVal" /><Field name="BoolVal" /><Field name="OptVal" /><Field name="EnumVal" /><Field name="BigVal" />'));
    end;
    procedure OneSelfClosing(): Text begin exit(Run('<Field name="TxtVal" />')); end;
    procedure EndTagEmpty(): Text begin exit(Run('<Field name="DateVal"></Field><Field name="IntVal"></Field>')); end;
    procedure Mixed(): Text begin exit(Run('<Field name="TxtVal" /><Field name="CodeVal">xyz</Field>')); end;
    procedure FilledAll(): Text
    begin
        exit(Run('<Field name="TxtVal">a b</Field><Field name="CodeVal">xy</Field><Field name="DateVal">2025-12-31</Field><Field name="TimeVal">23:59:58</Field><Field name="DtVal">2025-12-31T23:59:58Z</Field>' +
            '<Field name="IntVal">-3</Field><Field name="DecVal">0.75</Field><Field name="BoolVal">false</Field><Field name="OptVal">1</Field><Field name="EnumVal">0</Field><Field name="BigVal">5000000000</Field>'));
    end;
    procedure OptionByName(): Text begin exit(Run('<Field name="OptVal">Beta</Field>')); end;
    procedure BadInteger(): Text begin exit(Run('<Field name="IntVal">abc</Field>')); end;
    procedure EnumByName(): Text begin exit(Run('<Field name="EnumVal">Second</Field>')); end;
    procedure BooleanYes(): Text begin exit(Run('<Field name="BoolVal">Yes</Field>')); end;
    procedure OptionsSelfClosing(): Text
    begin
        exit(RunRaw('<?xml version="1.0" standalone="yes"?><ReportParameters name="RSC Report" id="91591"><Options /></ReportParameters>'));
    end;
    procedure RootSelfClosing(): Text
    begin
        exit(RunRaw('<?xml version="1.0" standalone="yes"?><ReportParameters name="RSC Report" id="91591" />'));
    end;
    procedure NoOptions(): Text
    begin
        exit(RunRaw('<?xml version="1.0" standalone="yes"?><ReportParameters name="RSC Report" id="91591"></ReportParameters>'));
    end;
    procedure ExecuteDataReport(): Text
    var
        A: Report "RSC Data";
    begin
        A.Execute('<?xml version="1.0" standalone="yes"?><ReportParameters name="RSC Data" id="91592"><Options><Field name="TxtVal" /></Options></ReportParameters>');
        exit(A.Result());
    end;
    procedure SaveAsAfterOnInit(): Text
    var
        A: Report "RSC Data";
        B: Record "RSC Blob" temporary;
        OutS: OutStream;
    begin
        B.Data.CreateOutStream(OutS);
        A.SaveAs('<?xml version="1.0" standalone="yes"?><ReportParameters name="RSC Data" id="91592"><Options><Field name="TxtVal" /></Options></ReportParameters>', ReportFormat::Xml, OutS);
        exit(A.Result());
    end;
}
"#;

const INIT: &str = "t=[tinit] c=[CINIT] d=[2024-01-15] tm=[10:15:00] dt=[2024-03-02T08:09:10Z] i=7 dc=2.5 b=Yes o=Gamma e=Second bg=99";

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

#[test]
fn a_self_closing_option_blanks_every_type() {
    // Y68SelfClosingParams.al:263-343 (Q1_Execute_SelfClosing_TextLike, _OtherTypes).
    assert_eq!(
        al_testkit::text(run("SelfClosingAll")),
        "t=[] c=[] d=[] tm=[] dt=[] i=0 dc=0 b=No o=Alpha e=First bg=0"
    );
    assert_eq!(
        al_testkit::text(run("OneSelfClosing")),
        INIT.replacen("t=[tinit]", "t=[]", 1)
    );
}

#[test]
fn an_empty_option_with_an_end_tag_blanks_too_and_mixes_with_filled_ones() {
    // Y68SelfClosingParams.al:318-343 (Q1_Execute_EmptyWithEndTag_AndMixed).
    assert_eq!(
        al_testkit::text(run("EndTagEmpty")),
        INIT.replacen("d=[2024-01-15]", "d=[]", 1).replacen("i=7", "i=0", 1)
    );
    assert_eq!(
        al_testkit::text(run("Mixed")),
        INIT.replacen("t=[tinit]", "t=[]", 1)
            .replacen("c=[CINIT]", "c=[XYZ]", 1)
    );
}

#[test]
fn filled_options_of_every_type_land() {
    // Y68SelfClosingParams.al:345-371 (Q1_Execute_Filled_AllTypes `filled all`).
    assert_eq!(
        al_testkit::text(run("FilledAll")),
        "t=[a b] c=[XY] d=[2025-12-31] tm=[23:59:58] dt=[2025-12-31T23:59:58Z] i=-3 dc=0.75 b=No o=Beta e=First bg=5000000000"
    );
}

#[test]
fn an_option_by_name_and_a_bad_integer_raise_the_pinned_errors() {
    // Y68SelfClosingParams.al:358-370 (BC 28 and later; the testkit runs as BC 29).
    assert_eq!(
        al_testkit::error_message(run("OptionByName")),
        "The value \"Beta\" can't be evaluated into type Option."
    );
    assert_eq!(al_testkit::error_message(run("BadInteger")), "NavType");
}

#[test]
fn unpinned_option_texts_are_refused() {
    for routine in ["EnumByName", "BooleanYes"] {
        match run(routine) {
            Err(Signal::Unsupported(_)) => {}
            other => panic!("{routine}: {other:?}"),
        }
    }
}

#[test]
fn a_parameters_text_without_fields_keeps_the_oninit_values() {
    // Y68SelfClosingParams.al:373-405 (Q1_Execute_SelfClosing_OptionsAndRoot).
    for routine in ["OptionsSelfClosing", "RootSelfClosing", "NoOptions"] {
        assert_eq!(al_testkit::text(run(routine)), INIT, "{routine}");
    }
}

#[test]
fn a_saveas_with_parameters_applies_them_after_the_request_page_oninit() {
    // Y68SelfClosingParams.al:420-444 (Q1_SaveAs_SelfClosing_DataReport `text`).
    assert_eq!(al_testkit::text(run("SaveAsAfterOnInit")), "t=[] i=7");
}

#[test]
fn an_execute_of_a_report_whose_default_layout_names_no_layout_fails() {
    // Y68SelfClosingParams.al:420-444 (Q1_SaveAs_SelfClosing_DataReport `execute text`: DefaultLayout = RDLC and no layout).
    assert_eq!(
        al_testkit::error_message(run("ExecuteDataReport")),
        "Report 91592 does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom."
    );
}
