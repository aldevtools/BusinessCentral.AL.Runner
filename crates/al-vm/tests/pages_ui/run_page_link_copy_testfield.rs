//! `TestField` on a by-value copy of Rec, in a field's OnValidate on a new row of a list an action opened through RunPageLink
//! (al-runner issue 7444). The copy holds Rec's fields (testpage/TestActionRunPageLinkFilterGroup.al:292-369,
//! testpage/R7138RunPageLinkCopyGet.al), and TestField reads only the copy's own field, so it runs as on any record.

const SRC: &str = r#"
codeunit 69960 "RT Log"
{
    SingleInstance = true;
    var Trail: Text;
    procedure Add(T: Text) begin if Trail <> '' then Trail += ';'; Trail += T; end;
    procedure Take() R: Text begin R := Trail; Trail := ''; end;
}
table 69961 "RT Loc" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 69962 "RT Zone"
{
    fields
    {
        field(1; "Location Code"; Code[10]) { }
        field(2; Code; Code[10]) { }
        field(3; Pick; Boolean) { }
        field(4; Multi; Boolean)
        {
            trigger OnValidate()
            var M: Codeunit "RT Mgt";
            begin
                M.Check(Rec);
            end;
        }
    }
    keys { key(PK; "Location Code", Code) { } }
}
codeunit 69963 "RT Mgt"
{
    procedure Check(Zone: Record "RT Zone")
    var L: Codeunit "RT Log"; Plain: Record "RT Zone"; Expected: Text;
    begin
        Plain."Location Code" := Zone."Location Code";
        Plain.Code := Zone.Code;
        Plain.Pick := Zone.Pick;
        if not TryTest(Plain) then
            Expected := GetLastErrorText();
        ClearLastError();
        if TryTest(Zone) then
            L.Add('ok')
        else
            L.Add('error same=' + Format(GetLastErrorText() = Expected) + ' empty=' + Format(Expected = ''));
    end;

    [TryFunction]
    local procedure TryTest(Z: Record "RT Zone")
    begin
        Z.TestField(Pick);
    end;
}
page 69964 "RT Zones"
{
    PageType = List; SourceTable = "RT Zone"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(CodeCtl; Rec.Code) { } field(PickCtl; Rec.Pick) { } field(MultiCtl; Rec.Multi) { } } } }
}
page 69965 "RT Loc Card"
{
    PageType = Card; SourceTable = "RT Loc"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(CodeCtl; Rec.Code) { } } }
    actions { area(Processing) { action(Zones) { RunObject = page "RT Zones"; RunPageLink = "Location Code" = field(Code); } } }
}
codeunit 69966 C
{
    local procedure Run(Pick: Boolean): Text
    var Loc: Record "RT Loc"; L: Codeunit "RT Log"; Card: TestPage "RT Loc Card"; Zones: TestPage "RT Zones";
    begin
        Loc.Code := 'L1';
        Loc.Insert();
        Card.Trap();
        Page.Run(Page::"RT Loc Card", Loc);
        Zones.Trap();
        Card.Zones.Invoke();
        Zones.New();
        Zones.CodeCtl.SetValue('Z01');
        if Pick then
            Zones.PickCtl.SetValue(true);
        Zones.MultiCtl.SetValue(true);
        exit(L.Take());
    end;

    procedure Passes(): Text begin exit(Run(true)); end;
    procedure Fails(): Text begin exit(Run(false)); end;
}
"#;

fn run(proc_name: &str) -> String {
    let mut built = al_testkit::Built::from_source(SRC);
    al_testkit::text(built.run_in("C", proc_name))
}

#[test]
fn testfield_on_the_copy_passes_on_a_set_field() {
    assert_eq!(run("Passes"), "ok");
}

#[test]
fn testfield_on_the_copy_raises_the_error_of_any_record() {
    assert_eq!(run("Fails"), "error same=Yes empty=No");
}
