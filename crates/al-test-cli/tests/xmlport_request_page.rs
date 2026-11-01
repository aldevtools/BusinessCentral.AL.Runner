//! An xmlport with a request page with content: Export and Import from code (instance and static) never open the page and run
//! with the globals as code set them; `Xmlport.Run(Id, false, false)` runs; `Xmlport.Run(Id, true, false)` and the instance
//! `Run()` run OnInitXmlPort, then end in `Unhandled UI: RequestPage <id>` (MissingUIHandler), as no handler can take an
//! xmlport's request page (corpus master ac3499d6, xmlport/lt-y40-xmlport-request-page/Y40XmlPortRequestPage.al:283-423).

use al_testrun::Outcome;

const SRC: &str = r#"
table 50250 "XR Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 50251 "XR Log" { SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end; }
xmlport 50252 "XR Export" { Direction = Export; Format = Xml;
    schema { textelement(Root) { tableelement(Row; "XR Row") {
        fieldelement(Id; Row.Id) { }
        trigger OnPreXmlItem() begin if OnlyId <> 0 then Row.SetRange(Id, OnlyId); end;
        trigger OnAfterGetRecord() begin Log.Add('row ' + Format(Row.Id)); end; } } }
    requestpage { layout { area(Content) { field(OnlyIdCtl; OnlyId) { Caption = 'Only Id'; } } }
        trigger OnOpenPage() begin Log.Add('rp-open'); end; }
    trigger OnInitXmlPort() begin Log.Add('init'); end;
    trigger OnPreXmlPort() begin Log.Add('pre only=' + Format(OnlyId)); end;
    trigger OnPostXmlPort() begin Log.Add('post'); end;
    var Log: Codeunit "XR Log"; OnlyId: Integer;
    procedure SetOnlyId(N: Integer) begin OnlyId := N; end; }
codeunit 50253 T { Subtype = Test;
    var Log: Codeunit "XR Log";
    local procedure Seed() var R: Record "XR Row"; I: Integer;
    begin R.DeleteAll(); for I := 1 to 2 do begin R.Id := I; R.Insert(); end; Log.Take(); end;
    local procedure Check(Got: Text; Want: Text) begin if Got <> Want then Error('got %1', Got); end;
    [Test] procedure ExportFromCode()
    var Xp: XmlPort "XR Export"; OutS: OutStream;
    begin
        Seed();
        TempBlobOut(OutS);
        Xp.SetOnlyId(2);
        Xp.SetDestination(OutS);
        Xp.Export();
        Check(Log.Take(), 'init;pre only=2;row 2;post;');
    end;
    local procedure TempBlobOut(var OutS: OutStream) var R: Record "XR Blob"; begin R.B.CreateOutStream(OutS); end;
    [Test] procedure RunWithoutWindow()
    begin
        Seed();
        Xmlport.Run(Xmlport::"XR Export", false, false);
        Check(Log.Take(), 'init;pre only=0;row 1;row 2;post;');
    end;
    [Test] procedure RunWithWindow()
    begin
        Log.Take();
        asserterror Xmlport.Run(Xmlport::"XR Export", true, false);
        Check(GetLastErrorText() + '|' + GetLastErrorCode(), 'Unhandled UI: RequestPage 50252|MissingUIHandler');
        Check(Log.Take(), 'init;');
    end;
    [Test] procedure InstanceRun()
    var Xp: XmlPort "XR Export";
    begin
        Log.Take();
        Xp.SetOnlyId(1);
        asserterror Xp.Run();
        Check(GetLastErrorText() + '|' + GetLastErrorCode(), 'Unhandled UI: RequestPage 50252|MissingUIHandler');
        Check(Log.Take(), 'init;');
    end;
}
table 50254 "XR Blob" { fields { field(1; K; Integer) { } field(2; B; Blob) { } } keys { key(PK; K) { } } }
"#;

#[test]
fn xmlport_request_page_runs() {
    let got = al_testkit::outcomes_in(SRC, "T");
    for (name, outcome) in &got {
        assert_eq!(outcome, &Outcome::Pass, "{name}");
    }
    assert_eq!(got.len(), 4);
}
