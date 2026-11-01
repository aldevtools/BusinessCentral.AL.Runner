//! Pins of the corpus writer Y17 for pages that need test codeunits with handlers: a Page-typed PageHandler for Page.Run with
//! a record and for a Page variable that was set up (pages/lt-y17-page-handler-record/Y17PageHandlerRecord.al).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const PAGE_HANDLER_RECORD: &str = r#"
table 69720 "YH Row"
{
    fields { field(1; Code; Code[20]) { } field(2; Store; Text[30]) { } field(3; Note; Text[30]) { } }
    keys { key(PK; Code) { } }
    trigger OnModify() var Log: Codeunit "YH Log"; begin Log.Add('T.OnModify:' + Rec.Code + '/' + Rec.Note); end;
}
codeunit 69721 "YH Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(E: Text) begin T += E + ';'; end;
    procedure Take() R: Text begin R := T; T := ''; end;
}
page 69722 "YH Card"
{
    PageType = Card; SourceTable = "YH Row"; UsageCategory = None; ApplicationArea = All;
    layout { area(Content) { field(Code; Rec.Code) { } field(Store; Rec.Store) { } field(Note; Rec.Note) { } } }
    var Log: Codeunit "YH Log"; Tag: Text;
    trigger OnOpenPage() begin Log.Add('P.OnOpenPage:' + Rec.Code); end;
    trigger OnAfterGetRecord() begin Log.Add('P.AGR:' + Rec.Code); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('P.AGCR:' + Rec.Code); end;
    trigger OnClosePage() begin Log.Add('P.OnClosePage:' + Rec.Code); end;
    procedure SetTag(T: Text) begin Tag := T; Log.Add('set(' + T + ')'); end;
    procedure GetTag(): Text begin exit(Tag); end;
    procedure RecCode(): Code[20] begin Log.Add('reccode(' + Rec.Code + ')'); exit(Rec.Code); end;
    procedure TouchRec() begin Rec.Note := 'touched'; Rec.Modify(); end;
}
page 69723 "YH List"
{
    PageType = List; SourceTable = "YH Row"; UsageCategory = None; ApplicationArea = All;
    layout { area(Content) { repeater(Rows) { field(Code; Rec.Code) { } field(Store; Rec.Store) { } } } }
    var Log: Codeunit "YH Log";
    trigger OnOpenPage() begin Log.Add('L.OnOpenPage:' + Rec.Code); end;
    trigger OnAfterGetRecord() begin Log.Add('L.AGR:' + Rec.Code); end;
}
codeunit 69724 T
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "YH Log"; Runs: Integer;
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    local procedure Seed()
    var R: Record "YH Row";
    begin
        R.DeleteAll();
        R.Code := 'A'; R.Store := 'sa'; R.Insert();
        R.Code := 'B'; R.Store := 'sb'; R.Insert();
        R.Code := 'C'; R.Store := 'sc'; R.Insert();
        Log.Take(); Runs := 0;
    end;
    local procedure Stored() Result: Text
    var R: Record "YH Row";
    begin
        if R.FindSet() then repeat Result += R.Code + '/' + R.Note + ';'; until R.Next() = 0;
    end;

    [Test]
    [HandlerFunctions('GetRecordHandler')]
    procedure RunWithRecord_GetRecord()
    var R: Record "YH Row";
    begin
        Seed(); R.Get('B');
        Page.Run(Page::"YH Card", R);
        Check('handler;cur=B/sb/find=Yes;T.OnModify:B/h;modify=Yes;', Log.Take());
        Check('A/;B/h;C/;', Stored());
    end;
    [PageHandler]
    procedure GetRecordHandler(var Pg: Page "YH Card")
    var Cur: Record "YH Row";
    begin
        Log.Add('handler'); Cur.Init(); Pg.GetRecord(Cur);
        Log.Add('cur=' + Cur.Code + '/' + Cur.Store + '/find=' + Format(Cur.Find()));
        Cur.Note := 'h'; Log.Add('modify=' + Format(Cur.Modify(true)));
    end;

    [Test]
    [HandlerFunctions('ProcedureHandler')]
    procedure RunWithRecord_PageProcedure()
    var R: Record "YH Row";
    begin
        Seed(); R.Get('B');
        Page.Run(Page::"YH Card", R);
        Check('handler;reccode(B);got=B;handler end;', Log.Take());
        Check('A/;B/touched;C/;', Stored());
    end;
    [PageHandler]
    procedure ProcedureHandler(var Pg: Page "YH Card")
    begin
        Log.Add('handler'); Log.Add('got=' + Pg.RecCode()); Pg.TouchRec(); Log.Add('handler end');
    end;

    [Test]
    [HandlerFunctions('CloseHandler')]
    procedure RunWithRecord_Close()
    var R: Record "YH Row";
    begin
        Seed(); R.Get('B');
        Page.Run(Page::"YH Card", R);
        Check('handler;closed;', Log.Take());
    end;
    [PageHandler]
    procedure CloseHandler(var Pg: Page "YH Card") begin Log.Add('handler'); Pg.Close(); Log.Add('closed'); end;

    [Test]
    [HandlerFunctions('ListGetRecordHandler')]
    procedure RunWithFilteredRecord_List()
    var R: Record "YH Row";
    begin
        Seed(); R.SetRange(Code, 'B', 'C'); R.FindLast();
        Page.Run(Page::"YH List", R);
        Check('handler;cur=C/sc/filter=;', Log.Take());
    end;
    [PageHandler]
    procedure ListGetRecordHandler(var Pg: Page "YH List")
    var Cur: Record "YH Row";
    begin
        Log.Add('handler'); Pg.GetRecord(Cur);
        Log.Add('cur=' + Cur.Code + '/' + Cur.Store + '/filter=' + Cur.GetFilters());
    end;

    [Test]
    [HandlerFunctions('GetRecordOnlyHandler')]
    procedure RunWithUnstoredRecord()
    var R: Record "YH Row";
    begin
        Seed(); R.Init(); R.Code := 'NEW'; R.Store := 'snew';
        Page.Run(Page::"YH Card", R);
        Check('handler;cur=NEW/snew/find=No;', Log.Take());
    end;
    [Test]
    [HandlerFunctions('GetRecordOnlyHandler')]
    procedure RunWithRecordAndField()
    var R: Record "YH Row";
    begin
        Seed(); R.Get('B');
        Page.Run(Page::"YH Card", R, R.FieldNo(Note));
        Check('handler;cur=B/sb/find=Yes;', Log.Take());
    end;
    [PageHandler]
    procedure GetRecordOnlyHandler(var Pg: Page "YH Card")
    var Cur: Record "YH Row";
    begin
        Log.Add('handler'); Pg.GetRecord(Cur);
        Log.Add('cur=' + Cur.Code + '/' + Cur.Store + '/find=' + Format(Cur.Find()));
    end;

    [Test]
    [HandlerFunctions('TagHandler')]
    procedure SetUpVariable_RunTwice()
    var R: Record "YH Row"; Pg: Page "YH Card";
    begin
        Seed(); R.Get('B');
        Pg.SetTag('pre'); Pg.SetRecord(R);
        Pg.Run();
        Check('set(pre);handler1 tag=pre;reccode(B);got=B;set(in1);', Log.Take());
        Pg.Run();
        Check('handler2 tag=;reccode();got=;set(in2);', Log.Take());
        Check('', Pg.GetTag());
    end;
    [Test]
    [HandlerFunctions('TagHandler')]
    procedure SetUpVariable_SetTableView_Run()
    var R: Record "YH Row"; Pg: Page "YH Card";
    begin
        Seed(); R.SetRange(Code, 'C');
        Pg.SetTag('pre'); Pg.SetTableView(R);
        Pg.Run();
        Check('set(pre);handler1 tag=pre;reccode();got=;set(in1);', Log.Take());
        Check('', Pg.GetTag());
    end;
    [PageHandler]
    procedure TagHandler(var Pg: Page "YH Card")
    begin
        Runs += 1;
        Log.Add('handler' + Format(Runs) + ' tag=' + Pg.GetTag());
        Log.Add('got=' + Pg.RecCode());
        Pg.SetTag('in' + Format(Runs));
    end;
}
"#;

/// Y17PageHandlerRecord.al:218 (GetRecord), :273 (page procedure), :403-476 (set-up variable), and the neighbours between.
#[test]
fn y17_page_typed_page_handler_with_record() {
    let out = outcomes_in(PAGE_HANDLER_RECORD, "T");
    let bad: Vec<_> = out.iter().filter(|(_, o)| !matches!(o, Outcome::Pass)).collect();
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(out.len(), 8);
}

const ACTIVATE_NEW_HOST: &str = r#"
table 69730 "YA Header"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[30]) { }
        field(3; "Customer No."; Code[20]) { trigger OnValidate() begin Log.Add('H.OnValidate(Customer No.):' + "Customer No."); end; }
    }
    keys { key(PK; "No.") { } }
    trigger OnInsert() begin Log.Add('H.OnInsert:' + "No."); if "No." = '' then "No." := 'AUTO1'; end;
    trigger OnModify() begin Log.Add('H.OnModify:' + "No."); end;
    var Log: Codeunit "YA Log";
}
table 69731 "YA Line"
{
    fields { field(1; "Document No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Description; Text[30]) { } }
    keys { key(PK; "Document No.", "Line No.") { } }
}
page 69732 "YA Lines"
{
    PageType = ListPart; SourceTable = "YA Line"; AutoSplitKey = true; UsageCategory = None; ApplicationArea = All;
    layout { area(Content) { repeater(Rows) { field(Description; Rec.Description) { } } } }
    trigger OnAfterGetRecord() begin Log.Add('LP.agr:' + Rec."Document No." + '|' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('LP.agcr:' + Rec."Document No." + '|' + Format(Rec."Line No.")); end;
    var Log: Codeunit "YA Log";
}
page 69733 "YA Document"
{
    PageType = Document; SourceTable = "YA Header"; DelayedInsert = false; UsageCategory = None; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            group(General) { field("No."; Rec."No.") { } field(Name; Rec.Name) { } field("Customer No."; Rec."Customer No.") { } }
            part(Lines; "YA Lines") { SubPageLink = "Document No." = field("No."); }
        }
    }
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('HP.OnNewRecord:' + Rec."No."); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Log.Add('HP.OnInsertRecord:' + Rec."No."); exit(true); end;
    trigger OnModifyRecord(): Boolean begin Log.Add('HP.OnModifyRecord:' + Rec."No."); exit(true); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('HP.agcr:' + Rec."No."); end;
    var Log: Codeunit "YA Log";
}
codeunit 69734 "YA Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(E: Text) begin T += E + ';'; end;
    procedure Take() R: Text begin R := T; T := ''; end;
}
codeunit 69735 T
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "YA Log";
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    local procedure Seed(WithLines: Boolean)
    var H: Record "YA Header"; L: Record "YA Line";
    begin
        H.DeleteAll(); L.DeleteAll();
        if WithLines then begin
            L."Line No." := 10000; L.Description := 'seeded'; L.Insert();
            L."Line No." := 20000; L.Insert();
        end;
        Log.Take();
    end;
    local procedure Stored() R: Text
    var H: Record "YA Header"; L: Record "YA Line";
    begin
        if H.FindSet() then repeat R += '[' + H."No." + '|' + H."Customer No." + ']'; until H.Next() = 0;
        R += '/' + Format(L.Count());
    end;

    [Test]
    procedure ActivateHostField_Ok()
    var Host: TestPage "YA Document";
    begin
        Seed(true);
        Host.OpenNew();
        Check('HP.OnNewRecord:;HP.agcr:;LP.agr:|10000;LP.agr:|20000;LP.agr:|10000;LP.agr:|10000;LP.agcr:|10000;', Log.Take());
        Host."Customer No.".Activate();
        Check('HP.OnInsertRecord:;H.OnInsert:;LP.agcr:AUTO1|0;LP.agcr:AUTO1|0;LP.agcr:AUTO1|0;', Log.Take());
        Check('AUTO1', Host."No.".Value());
        Host.OK().Invoke();
        Check('', Log.Take());
        Check('[AUTO1|]/2', Stored());
    end;

    [Test]
    procedure ActivateHostField_ThenPartFirst_Close()
    var Host: TestPage "YA Document";
    begin
        Seed(true);
        Host.OpenNew();
        Log.Take();
        Host."Customer No.".Activate();
        Log.Take();
        Check('No', Format(Host.Lines.First()));
        Check('', Host.Lines.Description.Value());
        Check('', Log.Take());
        Host.Close();
        Check('', Log.Take());
        Check('[AUTO1|]/2', Stored());
    end;

    [Test]
    procedure ActivateHostField_ThenSetValue_Ok()
    var Host: TestPage "YA Document";
    begin
        Seed(true);
        Host.OpenNew();
        Host."Customer No.".Activate();
        Log.Take();
        Host."Customer No.".SetValue('C1');
        Check('H.OnValidate(Customer No.):C1;', Log.Take());
        Host.OK().Invoke();
        Check('HP.OnModifyRecord:AUTO1;H.OnModify:AUTO1;', Log.Take());
        Check('[AUTO1|C1]/2', Stored());
    end;

    [Test]
    procedure ActivateKeyControl_Ok()
    var Host: TestPage "YA Document";
    begin
        Seed(true);
        Host.OpenNew();
        Log.Take();
        Host."No.".Activate();
        Check('', Log.Take());
        Check('', Host."No.".Value());
        Host.OK().Invoke();
        Check('', Log.Take());
        Check('/2', Stored());
    end;

    [Test]
    procedure HostNext()
    var Host: TestPage "YA Document";
    begin
        Seed(true);
        Host.OpenNew();
        Log.Take();
        Check('No', Format(Host.Next()));
        Check('', Log.Take());
        Check('', Host."No.".Value());
        Host.Close();
        Check('', Log.Take());
        Check('/2', Stored());
    end;

    [Test]
    procedure HostLast()
    var Host: TestPage "YA Document";
    begin
        Seed(true);
        Host.OpenNew();
        Log.Take();
        Check('No', Format(Host.Last()));
        Check('', Log.Take());
        Host.Close();
        Check('/2', Stored());
    end;
}
"#;

/// Y17ActivateNewHost.al:382 (Activate, read No., OK), :408 (then part First), :442 (then SetValue), :473 (the key control),
/// :573 and :602 (host Next and Last).
#[test]
fn y17_activate_new_host_with_blank_key_part() {
    let out = outcomes_in(ACTIVATE_NEW_HOST, "T");
    let bad: Vec<_> = out.iter().filter(|(_, o)| !matches!(o, Outcome::Pass)).collect();
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(out.len(), 6);
}
