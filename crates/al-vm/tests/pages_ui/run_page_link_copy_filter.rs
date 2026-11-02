//! The filter reads and copies of Rec itself in the OnOpenPage of a page an action opens through RunPageLink (run_page_link_open.rs,
//! pages/runobject.rs). Each test names the corpus test of pages/lt-y61-page-probes/Y61RunPageLinkCopyFilter.al that pins it.

const SRC: &str = r#"
codeunit 69950 "RC Log"
{
    SingleInstance = true;
    var Trail: Text; Mode: Text;
    procedure Add(T: Text) begin if Trail <> '' then Trail += ';'; Trail += T; end;
    procedure Take() R: Text begin R := Trail; Trail := ''; end;
    procedure SetMode(M: Text) begin Mode := M; end;
    procedure GetMode(): Text begin exit(Mode); end;
}
table 69951 "RC P" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[30]) { } field(3; "Date Filter"; Date) { FieldClass = FlowFilter; } } keys { key(PK; Code) { } } }
table 69952 "RC Q" { fields { field(1; "Parent Code"; Code[20]) { } field(2; Name; Code[10]) { } field(3; "Date Filter"; Date) { FieldClass = FlowFilter; } } keys { key(PK; "Parent Code", Name) { } } }
page 69953 "RC Part"
{
    PageType = ListPart; SourceTable = "RC Q";
    layout { area(Content) { repeater(Rows) { field(ParentCtl; Rec."Parent Code") { } field(NameCtl; Rec.Name) { } } } }
    procedure UpdateForm(var HostRec: Record "RC P"): Text
    begin
        Rec.SetRange("Parent Code", HostRec.Code);
        HostRec.CopyFilter("Date Filter", Rec."Date Filter");
        exit('part=' + Rec.GetFilters() + ';host=' + HostRec.GetFilters());
    end;
}
page 69954 "RC Target"
{
    PageType = ListPlus; SourceTable = "RC P"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(General) { field(CodeCtl; Rec.Code) { } } part(Sub; "RC Part") { } } }
    trigger OnOpenPage()
    var L: Codeunit "RC Log"; Other: Record "RC Q";
    begin
        L.Add('open=' + Rec.Code);
        case L.GetMode() of
            'code':
                begin
                    Other.SetRange("Parent Code", 'ZZ');
                    Rec.CopyFilter(Code, Other."Parent Code");
                    L.Add(Other.GetFilter("Parent Code") + ' ' + Format(Other.HasFilter()) + ' ' + Format(Other.Count()));
                end;
            'date':
                begin
                    Other.SetRange("Date Filter", 20240101D, 20240131D);
                    Rec.CopyFilter("Date Filter", Other."Date Filter");
                    L.Add('[' + Other.GetFilter("Date Filter") + '] ' + Format(Other.HasFilter()));
                end;
            'reads':
                begin
                    L.Add(Format(Rec.HasFilter()) + ' ' + Rec.GetFilter(Code) + ' [' + Rec.GetFilter("Date Filter") + ']');
                    L.Add(Rec.GetView(false));
                    L.Add(Format(Rec.FilterGroup()) + ' ' + Format(Rec.Count()) + ' ' + Format(Rec.FindFirst()) + ' ' + Rec.Name);
                end;
            'flow':
                begin
                    L.Add(Rec.GetFilter("Date Filter"));
                    Rec.CopyFilter("Date Filter", Other."Date Filter");
                    Rec.CopyFilter(Code, Other."Parent Code");
                    L.Add(Format(Other.GetFilter("Date Filter") = Rec.GetFilter("Date Filter")) + ' ' + Other.GetFilter("Parent Code"));
                end;
            'part':
                L.Add(CurrPage.Sub.Page.UpdateForm(Rec));
        end;
    end;
}
page 69955 "RC Host"
{
    PageType = Card; SourceTable = "RC P"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(CodeCtl; Rec.Code) { } } }
    actions
    {
        area(Processing)
        {
            action(Open) { RunObject = page "RC Target"; RunPageLink = Code = field(Code); }
            action(OpenWithDate) { RunObject = page "RC Target"; RunPageLink = Code = field(Code), "Date Filter" = field("Date Filter"); }
        }
    }
}
codeunit 69956 C
{
    local procedure Open(Mode: Text; WithDate: Boolean; var Target: TestPage "RC Target"): Text
    var P: Record "RC P"; Q: Record "RC Q"; L: Codeunit "RC Log"; Host: TestPage "RC Host";
    begin
        P.Code := 'P1'; P.Name := 'one'; P.Insert();
        P.Code := 'P2'; P.Name := 'two'; P.Insert();
        Q."Parent Code" := 'P1'; Q.Name := 'A'; Q.Insert();
        Q."Parent Code" := 'P1'; Q.Name := 'B'; Q.Insert();
        Q."Parent Code" := 'P2'; Q.Name := 'C'; Q.Insert();
        L.SetMode(Mode);
        P.SetRange("Date Filter", 20240301D, 20240331D);
        P.Get('P1');
        Host.Trap();
        Page.Run(Page::"RC Host", P);
        L.Take();
        Target.Trap();
        if WithDate then Host.OpenWithDate.Invoke() else Host.Open.Invoke();
        exit(L.Take());
    end;

    procedure Probe(Mode: Text): Text
    var Target: TestPage "RC Target";
    begin
        exit(Open(Mode, false, Target));
    end;

    procedure Code(): Text begin exit(Probe('code')); end;
    procedure Date(): Text begin exit(Probe('date')); end;
    procedure Reads(): Text begin exit(Probe('reads')); end;

    procedure Flow(): Text
    var Target: TestPage "RC Target";
    begin
        exit(Open('flow', true, Target));
    end;

    procedure Part(): Text
    var Target: TestPage "RC Target"; R: Text;
    begin
        R := Open('part', false, Target) + '|';
        if Target.Sub.First() then
            repeat
                R += Target.Sub.ParentCtl.Value() + '/' + Target.Sub.NameCtl.Value() + ',';
            until not Target.Sub.Next();
        exit(R);
    end;
}
"#;

fn run(proc_name: &str) -> String {
    let mut built = al_testkit::Built::from_source(SRC);
    al_testkit::text(built.run_in("C", proc_name))
}

/// Y61RunPageLinkCopyFilter.al:302 G616_CopyFilter_LinkedField_ToOther: the link filter replaces the target's own filter.
#[test]
fn copy_filter_of_the_linked_field() {
    assert_eq!(run("Code"), "open=P1;P1 Yes 2");
}

/// Y61RunPageLinkCopyFilter.al:315 G616_CopyFilter_FlowFilterNotLinked_ToOther: a FlowFilter the link does not set clears the target's.
#[test]
fn copy_filter_of_a_flow_filter_outside_the_link_clears_it() {
    assert_eq!(run("Date"), "open=P1;[] No");
}

/// Y61RunPageLinkCopyFilter.al:394 G616_HasFilter_GetView_FilterGroup_Count_FindFirst.
#[test]
fn the_filter_reads_of_rec_see_the_link() {
    assert_eq!(
        run("Reads"),
        "open=P1;Yes P1 [];VERSION(1) SORTING(Field1) WHERE(Field1=1(P1));0 1 Yes one"
    );
}

/// Y61RunPageLinkCopyFilter.al:407 G616_FlowFilterLink_CopyFilter: a FlowFilter in the link is Rec's filter, and CopyFilter hands it.
#[test]
fn a_flow_filter_in_the_link_is_copied() {
    assert_eq!(run("Flow"), "open=P1;03/01/24..03/31/24;Yes P1");
}

/// Y61RunPageLinkCopyFilter.al:328 G616_PartUpdateForm_VarRec: the part reads Rec's filters through its var parameter, and the filter it
/// sets on its own Rec in the host's OnOpenPage does not narrow the rows it shows.
#[test]
fn a_part_handed_rec_by_var_in_on_open_page() {
    assert_eq!(
        run("Part"),
        "open=P1;part=Parent Code: P1;host=Code: P1|P1/A,P1/B,P2/C,/,"
    );
}
