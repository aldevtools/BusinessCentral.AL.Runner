//! A reportextension that changes the run and binds request page controls to fields of its own record variable (al-runner issue
//! 6924). Mirrors corpus reports/extensions/W3RpExtRequestPage.al:1043-1430 (corpus PR 1202): the value a handler sets reaches
//! the extension's trigger; the parameters text lists the base's controls first and names the extension's by their source as
//! written (quotes kept); a parameters text sets them by that name only.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62740 "RO Params" { fields { field(1; "Code"; Code[10]) { } field(2; "Per Zone"; Boolean) { } field(3; Factor; Decimal) { } } keys { key(PK; "Code") { } } }
table 62741 "RO Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 62742 "RO Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
report 62743 "RO Base"
{
    ProcessingOnly = true;
    dataset { dataitem(BaseRow; "RO Row") { } }
    requestpage { layout { area(Content) { group(Opts) { field(BaseCtl; BaseVal) { } } } } }
    protected var BaseVal: Integer;
}
reportextension 62744 "RO Ext" extends "RO Base"
{
    dataset { modify(BaseRow) { trigger OnBeforePreDataItem() begin Log.Add('zone=' + Format(Params."Per Zone") + ' factor=' + Format(Params.Factor)); end; } }
    requestpage { layout { addfirst(Content) { group(ExtOpts) { field(PerZoneCtl; Params."Per Zone") { } field(FactorCtl; Params.Factor) { } } } } }
    protected var Params: Record "RO Params";
    var Log: Codeunit "RO Log";
}
"#;

fn outcome(body: &str, handler: &str) -> Outcome {
    let src = format!(
        r#"{OBJECTS}
codeunit 62790 T
{{
    Subtype = Test;
    var Log: Codeunit "RO Log";
    local procedure Params(Options: Text): Text
    begin
        exit('<?xml version="1.0" standalone="yes"?><ReportParameters name="RO Base" id="62743"><Options>' + Options + '</Options><DataItems></DataItems></ReportParameters>');
    end;
    local procedure Opts(P: Text): Text
    begin
        exit(CopyStr(P, P.IndexOf('<Options>') + 9, P.IndexOf('</Options>') - P.IndexOf('<Options>') - 9));
    end;
    [Test] [HandlerFunctions('H')] procedure Run()
    var P: Text;
    begin
        Log.Take();
        {body}
    end;
    [RequestPageHandler] procedure H(var RP: TestRequestPage "RO Base")
    begin
        {handler}
        RP.OK().Invoke();
    end;
}}"#
    );
    let o = al_testkit::outcomes_in(&src, "T");
    o[0].1.clone()
}

#[test]
fn a_control_on_the_extension_record_reaches_the_run() {
    let o = outcome(
        "Report.Run(Report::\"RO Base\"); P := Log.Take(); if P <> 'zone=Yes factor=2.5;' then Error('%1', P);",
        "RP.PerZoneCtl.SetValue(true); RP.FactorCtl.SetValue(2.5);",
    );
    assert_eq!(o, Outcome::Pass, "{o:?}");
}

#[test]
fn the_parameters_text_lists_the_base_first_and_the_extension_fields_as_written() {
    let o = outcome(
        "P := Report.RunRequestPage(Report::\"RO Base\"); \
         if Opts(P) <> '<Field name=\"BaseVal\">0</Field><Field name=\"Params.&quot;Per Zone&quot;\">true</Field><Field name=\"Params.Factor\">0</Field>' then Error('%1', P);",
        "RP.PerZoneCtl.SetValue(true);",
    );
    assert_eq!(o, Outcome::Pass, "{o:?}");
}

#[test]
fn a_parameters_text_sets_the_extension_fields_by_their_source_only() {
    let o = outcome(
        "P := Report.RunRequestPage(Report::\"RO Base\", Params('<Field name=\"Params.&quot;Per Zone&quot;\">true</Field><Field name=\"Params.Per Zone\">false</Field><Field name=\"FactorCtl\">3</Field>')); \
         if Opts(P) <> '<Field name=\"BaseVal\">0</Field><Field name=\"Params.&quot;Per Zone&quot;\">true</Field><Field name=\"Params.Factor\">0</Field>' then Error('%1', P);",
        "",
    );
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
