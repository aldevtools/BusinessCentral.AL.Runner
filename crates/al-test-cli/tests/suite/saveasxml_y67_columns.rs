//! TestRequestPage.SaveAsXml data file columns that Y60 does not cover (al-language-onprem/report/lt-y67-saveasxml-columns/
//! Y67SaveAsXmlColumns.al): Label, Format(..), caption, literal and procedure columns with no row (:326), Decimal formatters of
//! AutoFormatType 2 and 10 (:370) and Media/MediaSet columns with no row (:388).

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62810 "SY Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Amt; Decimal) { DecimalPlaces = 2 : 2; }
    field(5; Pic; Media) { } field(6; Pics; MediaSet) { } } keys { key(PK; Id) { } } }
report 62811 "SY Labels"
{
    ProcessingOnly = false;
    dataset { dataitem(Loop; "SY Row") {
        column(CapLbl; CapLbl) { } column(CapExpr; CapLbl + '!') { } column(FmtInt; Format(GInt)) { }
        column(FmtPic; Format(GDec, 0, '<Precision,2:2><Standard Format,0>')) { } column(Cap; GRec.FieldCaption(Name)) { }
        column(TCap; GRec.TableCaption()) { } column(Lit; 'abc') { } column(LitNum; 42) { } column(Proc; Describe()) { } column(FmtRec; Format(GRec.Amt)) { } } }
    requestpage { layout { area(Content) { field(Ctl; Ctl) { } } } }
    var Ctl: Integer; CapLbl: Label 'Dimensions'; GInt: Integer; GDec: Decimal; GRec: Record "SY Row";
    local procedure Describe(): Text begin exit('d'); end;
}
report 62812 "SY Decimals"
{
    ProcessingOnly = false;
    dataset { dataitem(Item; "SY Row") {
        column(Af2; Amt) { AutoFormatType = 2; }
        column(Af10; Amt) { AutoFormatType = 10; AutoFormatExpression = '<Precision,3:3><Standard Format,0>'; }
        column(ExprDp; Amt * 2) { DecimalPlaces = 3; } } }
    requestpage { layout { area(Content) { field(Ctl; Ctl) { } } } }
    var Ctl: Integer;
}
report 62813 "SY Media"
{
    ProcessingOnly = false;
    dataset { dataitem(Item; "SY Row") {
        column(PicC; Pic) { } column(PicsC; Pics) { } column(PicHas; Pic.HasValue()) { } column(PicsCount; Pics.Count()) { } } }
    requestpage { layout { area(Content) { field(Ctl; Ctl) { } } } }
    var Ctl: Integer;
}
codeunit 62890 T
{
    Subtype = Test;
    var DataFile: Text;
    local procedure Content(): Text
    var F: File; InS: InStream; Line: Text; R: Text;
    begin
        F.Open(DataFile); F.CreateInStream(InS);
        while not InS.EOS() do begin InS.ReadText(Line); R += Line.Trim() + '|'; end;
        F.Close();
        exit(R);
    end;
    local procedure Expect(Got: Text; Part: Text)
    begin
        if not Got.Contains(Part) then Error('missing [%1] in %2', Part, Got);
    end;
    [Test] [HandlerFunctions('LabelsH')] procedure Labels_NoRow()
    var C: Text; R: Record "SY Row";
    begin
        R.DeleteAll();
        DataFile := TemporaryPath() + 'sy-labels.xml'; Commit();
        Report.Run(Report::"SY Labels");
        C := Content();
        Expect(C, '<xs:element name="CapLbl" type="xs:string"');
        Expect(C, '<xs:element name="CapExpr" type="xs:string"');
        Expect(C, '<xs:element name="FmtInt" type="xs:string"');
        Expect(C, '<xs:element name="FmtPic" type="xs:string"');
        Expect(C, '<xs:element name="Cap" type="xs:string"');
        Expect(C, '<xs:element name="TCap" type="xs:string"');
        Expect(C, '<xs:element name="Lit" type="xs:string"');
        Expect(C, '<xs:element name="LitNum" type="xs:int"');
        Expect(C, '<xs:element name="Proc" type="xs:string"');
        Expect(C, '<xs:element name="FmtRec" type="xs:string"');
    end;
    [Test] [HandlerFunctions('DecimalsH')] procedure Decimals_OneRow()
    var C: Text; R: Record "SY Row";
    begin
        R.DeleteAll(); R.Id := 1; R.Amt := 1234.5; R.Insert();
        DataFile := TemporaryPath() + 'sy-dec.xml'; Commit();
        Report.Run(Report::"SY Decimals");
        C := Content();
        Expect(C, '<Af2Format>#,##0.00###</Af2Format>');
        Expect(C, '<Af10Format>#,##0.000</Af10Format>');
        Expect(C, '<ExprDpFormat>#,##0.000</ExprDpFormat>');
    end;
    [Test] [HandlerFunctions('MediaH')] procedure Media_NoRow()
    var C: Text; R: Record "SY Row";
    begin
        R.DeleteAll();
        DataFile := TemporaryPath() + 'sy-media.xml'; Commit();
        Report.Run(Report::"SY Media");
        C := Content();
        Expect(C, '<xs:element name="PicC" msdata:DataType="System.Guid');
        Expect(C, '<xs:element name="PicHas" type="xs:boolean"');
        Expect(C, '<xs:element name="PicsCount" type="xs:int"');
        R.Id := 1; R.Insert();
        DataFile := TemporaryPath() + 'sy-media2.xml'; Commit();
        Report.Run(Report::"SY Media");
        C := Content();
        Expect(C, '<xs:element name="PicC" msdata:DataType="System.Guid');
        Expect(C, '<Result>|<PicHas>false</PicHas>|<PicsCount>0</PicsCount>|</Result>');
    end;
    [RequestPageHandler] procedure LabelsH(var RP: TestRequestPage "SY Labels") begin RP.SaveAsXml(TemporaryPath() + 'p.xml', DataFile); end;
    [RequestPageHandler] procedure DecimalsH(var RP: TestRequestPage "SY Decimals") begin RP.SaveAsXml(TemporaryPath() + 'p.xml', DataFile); end;
    [RequestPageHandler] procedure MediaH(var RP: TestRequestPage "SY Media") begin RP.SaveAsXml(TemporaryPath() + 'p.xml', DataFile); end;
}"#;

fn outcomes() -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(OBJECTS, "T")
}

#[test]
fn y67_column_types_and_formatters_are_written() {
    let o = outcomes();
    for (name, outcome) in &o {
        assert_eq!(*outcome, Outcome::Pass, "{name}: {outcome:?}");
    }
    assert_eq!(o.len(), 3);
}
