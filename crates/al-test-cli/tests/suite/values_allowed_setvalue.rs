//! TestPage SetValue on an Enum or Option control with ValuesAllowed (pages/lt-y37-values-allowed/Y37ValuesAllowedSetValue.al):
//! an allowed value is taken, one outside the list is a TestValidation error naming the text typed (or, for an Enum or an
//! Integer, the ordinal the control holds), and Value then shows that ordinal. Yes on a ConfirmationDialog that holds such a
//! refused entry reports Cancel.

const SRC: &str = r#"
enum 61960 "VA Status" { Extensible = false; value(0; Simulated) { } value(1; Planned) { } value(2; "Firm Planned") { } value(3; Released) { } }
table 61961 "VA Order" { fields { field(1; "No."; Code[20]) { } field(2; Status; Enum "VA Status") { } } keys { key(PK; "No.") { } } }
page 61962 "VA Dialog"
{
    PageType = ConfirmationDialog;
    layout
    {
        area(Content)
        {
            field(NewStatus; Ord.Status) { ValuesAllowed = "Firm Planned", Released; trigger OnValidate() begin Log += 'v;'; end; }
            field(OptCtl; OptVar) { ValuesAllowed = B, C; }
        }
    }
    var
        Ord: Record "VA Order";
        OptVar: Option A,B,C;
        Log: Text;
    procedure Set(O: Record "VA Order") begin Ord := O; OptVar := OptVar::B; end;
    procedure Result(): Text begin exit(Format(Ord.Status) + '|' + Format(OptVar) + '|' + Log); end;
}
codeunit 61963 C
{
    Subtype = Test;

    var
        Seen: Text;

    [Test]
    [HandlerFunctions('H')]
    procedure Run()
    var
        O: Record "VA Order";
        Pg: Page "VA Dialog";
        R: Action;
        Got: Text;
    begin
        O.Status := O.Status::Released;
        Pg.Set(O);
        R := Pg.RunModal();
        Got := Seen + '|' + Format(R) + '|' + Pg.Result();
        if Got <> 'Firm Planned' +
            '|Validation error for Field: NewStatus,  Message = ''Your entry of ''2'' is not an acceptable value for ''NewStatus''.''' +
            '|TestValidation|2' +
            '|Validation error for Field: OptCtl,  Message = ''Your entry of ''A'' is not an acceptable value for ''OptCtl''.''' +
            '|Cancel|Firm Planned|B|v;'
        then
            Error(Got);
    end;

    [ModalPageHandler]
    procedure H(var P: TestPage "VA Dialog")
    begin
        P.NewStatus.SetValue(Enum::"VA Status"::"Firm Planned");
        Seen := P.NewStatus.Value();
        asserterror P.NewStatus.SetValue(Enum::"VA Status"::Planned);
        Seen += '|' + GetLastErrorText() + '|' + GetLastErrorCode() + '|' + P.NewStatus.Value();
        asserterror P.OptCtl.SetValue('A');
        Seen += '|' + GetLastErrorText();
        P.Yes().Invoke();
    end;
}
"#;

#[test]
fn values_allowed_takes_a_listed_value_and_refuses_another() {
    let got = al_testkit::outcomes_in(SRC, "C");
    assert!(matches!(got.as_slice(), [(_, al_testrun::Outcome::Pass)]), "{got:?}");
}
