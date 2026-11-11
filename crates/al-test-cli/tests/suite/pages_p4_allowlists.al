// Reviewer scenarios of the pages P4 pull request (al-runner PR 7807): long lists, long trees, the preloaded temporary
// Worksheet and a key typed on a CurrPage.Run list. Each test ends with Error(<what it saw>).
table 50900 "RV Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Indent; Integer) { }
        field(4; "Code"; Code[10]) { }
    }
    keys { key(PK; Id) { Clustered = true; } key(K2; Name) { } }
}
table 50901 "RV CRow"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Seq; Integer) { }
    }
    keys { key(PK; "Code") { Clustered = true; } key(K2; Seq) { } }
}
codeunit 50902 "RV Log"
{
    SingleInstance = true;
    var
        T: Text;
        TreeRows: Integer;
    procedure Add(S: Text) begin T += S + ' '; end;
    procedure Take() R: Text begin R := T; T := ''; end;
    procedure SetTreeRows(N: Integer) begin TreeRows := N; end;
    procedure GetTreeRows(): Integer begin exit(TreeRows); end;
}
page 50903 "RV List"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 50904 "RV List New"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    trigger OnNewRecord(B: Boolean) begin Log.Add('N'); end;
    var Log: Codeunit "RV Log";
}
page 50905 "RV List Ext"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
pageextension 50906 "RV List Ext X" extends "RV List Ext"
{
    trigger OnAfterGetCurrRecord() var Log: Codeunit "RV Log"; begin Log.Add('XC'); end;
}
page 50907 "RV Tree"
{
    PageType = List;
    SourceTable = "RV Row";
    SourceTableTemporary = true;
    Editable = false;
    layout { area(Content) { repeater(Rows) {
        ShowAsTree = true;
        IndentationColumn = Rec.Indent;
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage()
    var i: Integer;
    begin
        for i := 1 to Log.GetTreeRows() do begin
            Rec.Init(); Rec.Id := i;
            if (i - 1) mod 5 = 0 then Rec.Indent := 0 else Rec.Indent := 1;
            Rec.Insert();
        end;
        if Rec.FindFirst() then;
    end;
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 50908 "RV Tree Ed"
{
    PageType = List;
    SourceTable = "RV Row";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) {
        ShowAsTree = true;
        IndentationColumn = Rec.Indent;
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage()
    var i: Integer;
    begin
        for i := 1 to Log.GetTreeRows() do begin
            Rec.Init(); Rec.Id := i;
            if (i - 1) mod 5 = 0 then Rec.Indent := 0 else Rec.Indent := 1;
            Rec.Insert();
        end;
        if Rec.FindFirst() then;
    end;
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 50909 "RV CList"
{
    PageType = List;
    SourceTable = "RV CRow";
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Rec."Code"); end;
    var Log: Codeunit "RV Log";
}
page 50910 "RV CWs"
{
    PageType = Worksheet;
    SourceTable = "RV CRow";
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { }
        field(Name; Rec.Name) { }
    } } }
}
page 50911 "RV Temp Pick"
{
    PageType = List;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { }
        field(Name; Rec.Name) { }
    } } }
    procedure Preload(Count: Integer)
    var i: Integer;
    begin
        for i := 1 to Count do begin
            Rec.Init(); Rec."Code" := 'P' + Format(i); Rec.Name := 'preloaded'; Rec.Insert();
        end;
    end;
    procedure RunPick() begin CurrPage.Run(); end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + ';'; until Rec.Next() = 0;
    end;
}
page 50912 "RV Temp Ws"
{
    PageType = Worksheet;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq);
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name)
        {
            trigger OnValidate() begin
                case Mode of
                    0: CurrPage.Update(false);
                    1: CurrPage.Update(true);
                    2: ;
                end;
            end;
        }
        field(Seq; Rec.Seq) { }
    } } }
    actions { area(Processing) { action(Upd) { trigger OnAction() begin CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer; NewMode: Integer)
    var i: Integer;
    begin
        Mode := NewMode;
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
    end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log"; Mode: Integer;
}
codeunit 50920 "RV Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "RV Log"; Out: Text; PickMode: Integer; WsMode: Integer;

    local procedure Seed(N: Integer)
    var R: Record "RV Row"; i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to N do begin R.Init(); R.Id := i; R.Name := 'r' + Format(i); R.Insert(); end;
        Log.Take();
    end;
    local procedure LastOn(N: Integer; Edit: Boolean)
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(N);
        if Edit then P.OpenEdit() else P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' prev=' + Format(P.Previous()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' first=' + Format(P.First()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' next=' + Format(P.Next()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        P.Close();
        Error(S);
    end;
    [Test] procedure Edit20_T() begin LastOn(20, true); end;
    [Test] procedure Edit21_T() begin LastOn(21, true); end;
    [Test] procedure Edit25_T() begin LastOn(25, true); end;
    [Test] procedure Edit39_T() begin LastOn(39, true); end;
    [Test] procedure Edit40_T() begin LastOn(40, true); end;
    [Test] procedure Edit41_T() begin LastOn(41, true); end;
    [Test] procedure View20_T() begin LastOn(20, false); end;
    [Test] procedure View25_T() begin LastOn(25, false); end;
    [Test] procedure View40_T() begin LastOn(40, false); end;
    [Test] procedure View45_T() begin LastOn(45, false); end;
    [Test] procedure View45Filter_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(90);
        P.OpenView();
        P.Filter.SetFilter(Id, '..45');
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure View45NextThenLast_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(45);
        P.OpenView();
        P.Next(); P.Next();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure Edit45SetValueThenLast_T()
    var P: TestPage "RV List"; S: Text; R: Record "RV Row";
    begin
        Seed(45);
        P.OpenEdit();
        P.Name.SetValue('zz');
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        R.Get(1);
        Error(S + ' db=' + R.Name);
    end;
    [Test] procedure Edit45LastNew_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(45);
        P.OpenEdit();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' next=' + Format(P.Next()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        P.New();
        S += ' new on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure NewPage45Edit_T()
    var P: TestPage "RV List New"; S: Text;
    begin
        Seed(45);
        P.OpenEdit();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure NewPage25Edit_T()
    var P: TestPage "RV List New"; S: Text;
    begin
        Seed(25);
        P.OpenEdit();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure ExtPage45View_T()
    var P: TestPage "RV List Ext"; S: Text;
    begin
        Seed(45);
        P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure ExtPage25View_T()
    var P: TestPage "RV List Ext"; S: Text;
    begin
        Seed(25);
        P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    local procedure Tree(N: Integer; Edit: Boolean)
    var P: TestPage "RV Tree"; S: Text; C: Integer;
    begin
        Log.Take(); Log.SetTreeRows(N);
        if Edit then P.OpenEdit() else P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        S += ' walk=' + Format(C) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure Tree60View_T() begin Tree(60, false); end;
    [Test] procedure Tree60Edit_T() begin Tree(60, true); end;
    [Test] procedure Tree21View_T() begin Tree(21, false); end;
    [Test] procedure Tree20View_T() begin Tree(20, false); end;
    [Test] procedure Tree95View_T() begin Tree(95, false); end;
    [Test] procedure Tree100View_T() begin Tree(100, false); end;
    [Test] procedure Tree60Last_T()
    var P: TestPage "RV Tree"; S: Text;
    begin
        Log.Take(); Log.SetTreeRows(60);
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        P.Expand(true);
        S += ' expand, next=' + Format(P.Next()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure TreeEd60View_T()
    var P: TestPage "RV Tree Ed"; S: Text; C: Integer;
    begin
        Log.Take(); Log.SetTreeRows(60);
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        Error(S + ' walk=' + Format(C) + ' [' + Log.Take() + ']');
    end;
    [Test] procedure TreeEd60Edit_T()
    var P: TestPage "RV Tree Ed"; S: Text; C: Integer;
    begin
        Log.Take(); Log.SetTreeRows(60);
        P.OpenEdit();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        Error(S + ' walk=' + Format(C) + ' [' + Log.Take() + ']');
    end;

    local procedure SeedC()
    var R: Record "RV CRow";
    begin
        R.DeleteAll();
        R."Code" := 'A'; R.Name := 'a'; R.Insert();
        R."Code" := 'B'; R.Name := 'b'; R.Insert();
        R."Code" := 'C'; R.Name := 'c'; R.Insert();
        Log.Take();
    end;
    [Test] procedure GoToKeyLowerListWithTrigger_T()
    var P: TestPage "RV CList"; S: Text;
    begin
        SeedC(); P.OpenView(); Log.Take();
        S := Format(P.GoToKey('b')) + ' on ' + P."Code".Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure GoToKeyLowerWorksheet_T()
    var P: TestPage "RV CWs"; S: Text;
    begin
        SeedC(); P.OpenView(); Log.Take();
        S := Format(P.GoToKey('b')) + ' on ' + P."Code".Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure GoToKeyWrongTypeListWithTrigger_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(3); P.OpenView(); Log.Take();
        S := Format(P.GoToKey('abc')) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure GoToKeyUpperListWithTrigger_T()
    var P: TestPage "RV CList"; S: Text;
    begin
        SeedC(); P.OpenView(); Log.Take();
        S := Format(P.GoToKey('B')) + ' on ' + P."Code".Value + ' [' + Log.Take() + ']';
        Error(S);
    end;

    local procedure Pick(Mode: Integer)
    var P: Page "RV Temp Pick";
    begin
        PickMode := Mode; Out := '';
        P.Preload(2);
        P.RunPick();
        Error(Out + ' | ' + P.Dump());
    end;
    [Test] [HandlerFunctions('PickHandler')] procedure PickKeyNext_T() begin Pick(0); end;
    [Test] [HandlerFunctions('PickHandler')] procedure PickKeyClose_T() begin Pick(1); end;
    [Test] [HandlerFunctions('PickHandler')] procedure PickKeyAndNameNext_T() begin Pick(2); end;
    [Test] [HandlerFunctions('PickHandler')] procedure PickNameThenKeyNext_T() begin Pick(3); end;
    [Test] [HandlerFunctions('PickHandler')] procedure PickKeyPrevious_T() begin Pick(4); end;
    [Test] [HandlerFunctions('PickHandler')] procedure PickKeyOnFirstRowNext_T() begin Pick(5); end;
    [PageHandler] procedure PickHandler(var P: TestPage "RV Temp Pick")
    begin
        Out := 'on ' + P."Code".Value;
        case PickMode of
            0: begin P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
            1: begin P."Code".SetValue('T1'); P.Close(); Out += ' closed'; exit; end;
            2: begin P."Code".SetValue('T1'); P.Name.SetValue('x'); Out += ' next=' + Format(P.Next()); end;
            3: begin P.Name.SetValue('x'); P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
            4: begin P."Code".SetValue('T1'); Out += ' prev=' + Format(P.Previous()); end;
            5: begin P.First(); P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
        end;
        Out += ' on ' + P."Code".Value + '/' + P.Name.Value;
    end;

    local procedure Ws(N: Integer; Mode: Integer; HMode: Integer)
    var P: Page "RV Temp Ws";
    begin
        WsMode := HMode; Out := ''; Log.Take();
        P.Fill(N, Mode);
        P.Run();
        Error(Out + ' | ' + P.Dump());
    end;
    [Test] [HandlerFunctions('WsHandler')] procedure WsUpdFalseFirstRow_T() begin Ws(4, 0, 0); end;
    [Test] [HandlerFunctions('WsHandler')] procedure WsUpdTrueFirstRow_T() begin Ws(4, 1, 0); end;
    [Test] [HandlerFunctions('WsHandler')] procedure WsNoUpdFirstRow_T() begin Ws(4, 2, 0); end;
    [Test] [HandlerFunctions('WsHandler')] procedure WsActionUpd_T() begin Ws(4, 2, 1); end;
    [Test] [HandlerFunctions('WsHandler')] procedure WsEditThenActionUpd_T() begin Ws(4, 2, 2); end;
    [Test] [HandlerFunctions('WsHandler')] procedure WsNoUpdEditNext_T() begin Ws(4, 2, 3); end;
    [Test] [HandlerFunctions('WsHandler')] procedure WsUpdFalseSeq_T() begin Ws(4, 0, 4); end;
    [PageHandler] procedure WsHandler(var P: TestPage "RV Temp Ws")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.First();
        Out += ' first on ' + P."Code".Value + ' [' + Log.Take() + ']';
        case WsMode of
            0: P.Name.SetValue('edited');
            1: P.Upd.Invoke();
            2: begin P.Name.SetValue('edited'); P.Upd.Invoke(); end;
            3: begin P.Name.SetValue('edited'); Out += ' next=' + Format(P.Next()); end;
            4: begin P.Seq.SetValue(5); Out += ' next=' + Format(P.Next()); end;
        end;
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
}
page 50930 "RV List Sub"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
codeunit 50931 "RV Sub"
{
    [EventSubscriber(ObjectType::Page, Page::"RV List Sub", 'OnAfterGetCurrRecordEvent', '', false, false)]
    local procedure OnCurr(var Rec: Record "RV Row")
    var Log: Codeunit "RV Log";
    begin
        Log.Add('SC');
    end;
}
page 50932 "RV Tree Top"
{
    PageType = List;
    SourceTable = "RV Row";
    SourceTableTemporary = true;
    Editable = false;
    layout { area(Content) { repeater(Rows) {
        ShowAsTree = true;
        IndentationColumn = Rec.Indent;
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage()
    var i: Integer;
    begin
        for i := 1 to 60 do begin
            Rec.Init(); Rec.Id := i; Rec.Indent := 0; Rec.Insert();
        end;
        if Rec.FindFirst() then;
    end;
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 50933 "RV Pin Pick"
{
    PageType = List;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage() begin Log.Add('O:' + Rec."Code"); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Rec.Name := 'new'; Log.Add('N'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."Code"); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Log.Add('I:' + Rec."Code"); end;
    trigger OnClosePage() begin Log.Add('X'); end;
    procedure Preload(Count: Integer)
    var i: Integer;
    begin
        for i := 1 to Count do begin
            Rec.Init(); Rec."Code" := 'P' + Format(i); Rec.Name := 'preloaded'; Rec.Insert();
        end;
    end;
    procedure RunPick() begin CurrPage.Run(); end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log";
}
page 50934 "RV Pin Pick Ext"
{
    PageType = List;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage() begin Log.Add('O:' + Rec."Code"); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Rec.Name := 'new'; Log.Add('N'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."Code"); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Log.Add('I:' + Rec."Code"); end;
    trigger OnClosePage() begin Log.Add('X'); end;
    procedure Preload(Count: Integer)
    var i: Integer;
    begin
        for i := 1 to Count do begin
            Rec.Init(); Rec."Code" := 'P' + Format(i); Rec.Name := 'preloaded'; Rec.Insert();
        end;
    end;
    procedure RunPick() begin CurrPage.Run(); end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log";
}
pageextension 50935 "RV Pin Pick Ext X" extends "RV Pin Pick Ext"
{
    trigger OnAfterGetRecord() var Log: Codeunit "RV Log"; begin Log.Add('XR'); end;
}
page 50936 "RV Temp Ws2"
{
    PageType = Worksheet;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq);
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Seq; Rec.Seq) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer; At: Text)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
        if At = 'ZZ' then Rec."Code" := 'ZZ' else if At <> '' then Rec.Get(At);
    end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log";
}
page 50937 "RV Temp Ws3"
{
    PageType = Worksheet;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq);
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Seq; Rec.Seq) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
    end;
    var Log: Codeunit "RV Log";
}
pageextension 50938 "RV Temp Ws3 X" extends "RV Temp Ws3"
{
    trigger OnAfterGetCurrRecord() var Log: Codeunit "RV Log"; begin Log.Add('XC:' + Rec."Code"); end;
}
codeunit 50940 "RV Tests 2"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "RV Log"; Out: Text; PickMode: Integer; WsMode: Integer;

    local procedure Seed(N: Integer)
    var R: Record "RV Row"; i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to N do begin R.Init(); R.Id := i; R.Name := 'r' + Format(i); R.Insert(); end;
        Log.Take();
    end;
    [Test] procedure SubPage45View_T()
    var P: TestPage "RV List Sub"; S: Text;
    begin
        Seed(45);
        P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure View101_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(101);
        P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure Edit60_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(60);
        P.OpenEdit();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;
    [Test] procedure TreeTop60View_T()
    var P: TestPage "RV Tree Top"; S: Text;
    begin
        Log.Take();
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S);
    end;

    local procedure PinPick(Mode: Integer)
    var P: Page "RV Pin Pick";
    begin
        PickMode := Mode; Out := ''; Log.Take();
        P.Preload(2);
        P.RunPick();
        Error(Out + ' | ' + P.Dump());
    end;
    [Test] [HandlerFunctions('PinPickHandler')] procedure PinKeyNext_T() begin PinPick(0); end;
    [Test] [HandlerFunctions('PinPickHandler')] procedure PinKeyClose_T() begin PinPick(1); end;
    [Test] [HandlerFunctions('PinPickHandler')] procedure PinKeyAndNameNext_T() begin PinPick(2); end;
    [Test] [HandlerFunctions('PinPickHandler')] procedure PinKeyPrevious_T() begin PinPick(3); end;
    [Test] [HandlerFunctions('PinPickHandler')] procedure PinFirstRowKeyNext_T() begin PinPick(4); end;
    [PageHandler] procedure PinPickHandler(var P: TestPage "RV Pin Pick")
    begin
        Out := 'on ' + P."Code".Value + ' [' + Log.Take() + ']';
        case PickMode of
            0: begin P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
            1: begin P."Code".SetValue('T1'); P.Close(); Out += ' closed'; exit; end;
            2: begin P."Code".SetValue('T1'); P.Name.SetValue('x'); Out += ' next=' + Format(P.Next()); end;
            3: begin P."Code".SetValue('T1'); Out += ' prev=' + Format(P.Previous()); end;
            4: begin P.First(); P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
        end;
        Out += ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('PinPickExtHandler')] procedure PinExtKeyNext_T()
    var P: Page "RV Pin Pick Ext";
    begin
        Out := ''; Log.Take();
        P.Preload(2);
        P.RunPick();
        Error(Out + ' | ' + P.Dump());
    end;
    [PageHandler] procedure PinPickExtHandler(var P: TestPage "RV Pin Pick Ext")
    begin
        Out := 'on ' + P."Code".Value;
        P."Code".SetValue('T1');
        Out += ' next=' + Format(P.Next()) + ' [' + Log.Take() + ']';
    end;

    local procedure Ws2(N: Integer; At: Text; HMode: Integer)
    var P: Page "RV Temp Ws2";
    begin
        WsMode := HMode; Out := ''; Log.Take();
        P.Fill(N, At);
        P.Run();
        Error(Out + ' | ' + P.Dump());
    end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure WsOffKey_T() begin Ws2(4, 'ZZ', 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure Ws21Rows_T() begin Ws2(21, '', 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure Ws25RowsMiddle_T() begin Ws2(25, 'C10', 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure WsUpdOffFirstRow_T() begin Ws2(4, '', 1); end;
    [PageHandler] procedure Ws2Handler(var P: TestPage "RV Temp Ws2")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        case WsMode of
            0: P.Name.SetValue('edited');
            1: begin P.Next(); P.Name.SetValue('edited'); end;
        end;
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('Ws3Handler')] procedure WsExtTrigger_T()
    var P: Page "RV Temp Ws3";
    begin
        Out := ''; Log.Take();
        P.Fill(4);
        P.Run();
        Error(Out);
    end;
    [PageHandler] procedure Ws3Handler(var P: TestPage "RV Temp Ws3")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.Name.SetValue('edited');
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
}
// Round 3 of the review: the reviewer's probes (N...), each against the pinned scenario of one rule.
codeunit 51500 "RV Sub3"
{
    [EventSubscriber(ObjectType::Page, Page::"RV List Sub3", 'OnAfterGetCurrRecordEvent', '', false, false)]
    local procedure S(var Rec: Record "RV Row") var Log: Codeunit "RV Log"; begin Log.Add('SC'); end;
}
codeunit 51501 "RV Man"
{
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Page, Page::"RV List Man", 'OnAfterGetRecordEvent', '', false, false)]
    local procedure S(var Rec: Record "RV Row") var Log: Codeunit "RV Log"; begin Log.Add('MR'); end;
}
page 51502 "RV List Sub3"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51503 "RV List Man"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51504 "RV Fact"
{
    PageType = CardPart;
    SourceTable = "RV Row";
    layout { area(Content) { field(Name; Rec.Name) { } } }
    trigger OnAfterGetRecord() begin Log.Add('PR' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('PC' + Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51505 "RV List Part"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } }
    area(FactBoxes) { part(Fact; "RV Fact") { SubPageLink = Id = field(Id); } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51506 "RV List Fn"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
        field(Calc; Calc()) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    local procedure Calc(): Text begin Log.Add('F'); exit('c'); end;
    var Log: Codeunit "RV Log";
}
page 51507 "RV List Flt"
{
    PageType = List;
    SourceTable = "RV Row";
    SourceTableView = where(Id = filter(>= 10));
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51508 "RV List Mov"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage() begin Rec.Get(30); end;
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51509 "RV Tree Pos"
{
    PageType = List;
    SourceTable = "RV Row";
    SourceTableTemporary = true;
    Editable = false;
    layout { area(Content) { repeater(Rows) {
        ShowAsTree = true;
        IndentationColumn = Rec.Indent;
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage()
    var i: Integer;
    begin
        for i := 1 to 60 do begin
            Rec.Init(); Rec.Id := i;
            if (i - 1) mod 5 = 0 then Rec.Indent := 0 else Rec.Indent := 1;
            Rec.Insert();
        end;
        Rec.Get(Log.GetTreeRows());
    end;
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51510 "RV Tree Flt"
{
    PageType = List;
    SourceTable = "RV Row";
    SourceTableTemporary = true;
    Editable = false;
    layout { area(Content) { repeater(Rows) {
        ShowAsTree = true;
        IndentationColumn = Rec.Indent;
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage()
    var i: Integer;
    begin
        for i := 1 to 90 do begin
            Rec.Init(); Rec.Id := i;
            if (i - 1) mod 5 = 0 then Rec.Indent := 0 else Rec.Indent := 1;
            Rec.Insert();
        end;
        Rec.SetRange(Id, 31, 90);
        if Rec.FindFirst() then;
    end;
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51511 "RV Tree Deep"
{
    PageType = List;
    SourceTable = "RV Row";
    SourceTableTemporary = true;
    Editable = false;
    layout { area(Content) { repeater(Rows) {
        ShowAsTree = true;
        IndentationColumn = Rec.Indent;
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage()
    var i: Integer;
    begin
        for i := 1 to 60 do begin
            Rec.Init(); Rec.Id := i;
            Rec.Indent := (i - 1) mod 5;
            if Rec.Indent > 2 then Rec.Indent := 2;
            Rec.Insert();
        end;
        if Rec.FindFirst() then;
    end;
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    procedure RunIt() begin CurrPage.Run(); end;
    var Log: Codeunit "RV Log";
}
table 51512 "RV TRow"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Seq; Integer) { }
        field(4; Note; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } key(K2; Seq) { } }
    trigger OnModify() var Log: Codeunit "RV Log"; begin Log.Add('TM'); end;
    trigger OnRename() var Log: Codeunit "RV Log"; begin Log.Add('TR'); end;
}
page 51513 "RV Pick2"
{
    PageType = List;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage() begin Log.Add('O:' + Rec."Code"); end;
    trigger OnNewRecord(B: Boolean) begin Rec.Name := 'new'; Log.Add('N'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."Code"); end;
    trigger OnInsertRecord(B: Boolean): Boolean begin end;
    trigger OnClosePage() begin Log.Add('X'); end;
    procedure Preload(Count: Integer)
    var i: Integer;
    begin
        for i := 1 to Count do begin
            Rec.Init(); Rec."Code" := 'P' + Format(i); Rec.Name := 'preloaded'; Rec.Insert();
        end;
    end;
    procedure RunPick() begin CurrPage.Run(); end;
    procedure RunOther() var Q: Page "RV Pick2"; begin Q.Preload(2); Q.Run(); end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log";
}
page 51514 "RV Pick3"
{
    PageType = List;
    SourceTable = "RV TRow";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { trigger OnValidate() begin Log.Add('V'); end; }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage() begin Log.Add('O:' + Rec."Code"); end;
    trigger OnNewRecord(B: Boolean) begin Rec.Name := 'new'; Log.Add('N'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."Code"); end;
    trigger OnInsertRecord(B: Boolean): Boolean begin end;
    trigger OnClosePage() begin Log.Add('X'); end;
    procedure Preload(Count: Integer)
    var i: Integer;
    begin
        for i := 1 to Count do begin
            Rec.Init(); Rec."Code" := 'P' + Format(i); Rec.Name := 'preloaded'; Rec.Insert();
        end;
    end;
    procedure RunPick() begin CurrPage.Run(); end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log";
}
page 51515 "RV Pick4"
{
    PageType = List;
    SourceTable = "RV CRow";
    SourceTableTemporary = true;
    layout { area(Content) { repeater(Rows) {
        field("Code"; Rec."Code") { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnOpenPage()
    begin
        Rec.Init(); Rec."Code" := 'P1'; Rec.Name := 'preloaded'; Rec.Insert();
        Rec.Init(); Rec."Code" := 'P2'; Rec.Name := 'preloaded'; Rec.Insert();
        Log.Add('O:' + Rec."Code");
    end;
    trigger OnNewRecord(B: Boolean) begin Rec.Name := 'new'; Log.Add('N'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C:' + Rec."Code"); end;
    trigger OnInsertRecord(B: Boolean): Boolean begin end;
    trigger OnClosePage() begin Log.Add('X'); end;
    procedure RunPick() begin CurrPage.Run(); end;
    var Log: Codeunit "RV Log";
}
page 51516 "RV Ws2"
{
    PageType = Worksheet;
    SourceTable = "RV TRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq);
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Note; Rec.Note) { }
        field(Seq; Rec.Seq) { Editable = false; }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer; How: Integer)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
        case How of
            1: Rec.SetRange(Seq, 97, 99);
            2: begin Rec.SetRange(Seq, 97, 99); Rec.FindFirst(); end;
            3: Rec.Get('C2');
            4: Rec.Ascending(false);
        end;
    end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + '/' + Rec.Note + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log";
}
page 51517 "RV Ws Flt"
{
    PageType = Worksheet;
    SourceTable = "RV TRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq) where(Seq = filter(> 97));
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Seq; Rec.Seq) { Editable = false; }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
    end;
    var Log: Codeunit "RV Log";
}
page 51518 "RV Ws Desc"
{
    PageType = Worksheet;
    SourceTable = "RV TRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq) order(descending);
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Seq; Rec.Seq) { Editable = false; }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
    end;
    var Log: Codeunit "RV Log";
}
codeunit 50950 "RV Tests 3"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "RV Log"; Out: Text; PickMode: Integer; WsMode: Integer;

    local procedure Seed(N: Integer)
    var R: Record "RV Row"; i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to N do begin R.Init(); R.Id := i; R.Name := 'r' + Format(i); R.Insert(); end;
        Log.Take();
    end;
    local procedure LastOn(N: Integer; Edit: Boolean)
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(N);
        if Edit then P.OpenEdit() else P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' prev=' + Format(P.Previous()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' first=' + Format(P.First()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' next=' + Format(P.Next()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        P.Close();
        Error(S);
    end;
    [Test] procedure NSub45_T()
    var P: TestPage "RV List Sub3";
    begin
        Seed(45); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NMan45_T()
    var P: TestPage "RV List Man"; M: Codeunit "RV Man";
    begin
        Seed(45); BindSubscription(M); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NPart45_T()
    var P: TestPage "RV List Part";
    begin
        Seed(45); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NFn45_T()
    var P: TestPage "RV List Fn";
    begin
        Seed(45); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NFlt55_T()
    var P: TestPage "RV List Flt";
    begin
        Seed(55); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NMov45_T()
    var P: TestPage "RV List Mov";
    begin
        Seed(45); P.OpenView();
        Error('open on ' + P.Id.Value + '[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NGoToKey45_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(45); P.OpenView();
        S := 'open[' + Log.Take() + '] goto=' + Format(P.GoToKey(30)) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S + ' last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NLastTwice45_T()
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(45); P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' first=' + Format(P.First()) + ' [' + Log.Take() + ']';
        Error(S + ' last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NDeleted45_T()
    var P: TestPage "RV List"; S: Text; R: Record "RV Row";
    begin
        Seed(46); P.OpenView();
        R.Get(20); R.Delete();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NView41_T() begin LastOn(41, false); end;
    [Test] procedure NView100_T() begin LastOn(100, false); end;
    [Test] procedure NView101_T() begin LastOn(101, false); end;
    [Test] [HandlerFunctions('ListRunHandler')] procedure NPageRun45_T()
    begin
        Seed(45); Out := '';
        Page.Run(Page::"RV List");
        Error(Out);
    end;
    [PageHandler] procedure ListRunHandler(var P: TestPage "RV List")
    begin
        Out := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
    end;
    [Test] procedure NTreePos30_T()
    var P: TestPage "RV Tree Pos"; S: Text; C: Integer;
    begin
        Log.Take(); Log.SetTreeRows(31);
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        Error(S + ' walk=' + Format(C) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NTreePos60_T()
    var P: TestPage "RV Tree Pos"; S: Text; C: Integer;
    begin
        Log.Take(); Log.SetTreeRows(60);
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        Error(S + ' walk=' + Format(C) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NTreeFlt_T()
    var P: TestPage "RV Tree Flt"; S: Text; C: Integer;
    begin
        Log.Take();
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        Error(S + ' walk=' + Format(C) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NTreeDeep_T()
    var P: TestPage "RV Tree Deep"; S: Text; C: Integer;
    begin
        Log.Take();
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        Error(S + ' walk=' + Format(C) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] [HandlerFunctions('TreeDeepHandler')] procedure NTreeDeepRun_T()
    var P: Page "RV Tree Deep";
    begin
        Log.Take(); Out := '';
        P.Run();
        Error(Out);
    end;
    [PageHandler] procedure TreeDeepHandler(var P: TestPage "RV Tree Deep")
    var C: Integer;
    begin
        Out := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until not P.Next();
        Out += ' walk=' + Format(C) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
    end;
    local procedure Pick2(Mode: Integer; N: Integer)
    var P: Page "RV Pick2";
    begin
        PickMode := Mode; Out := ''; Log.Take();
        if Mode = 20 then begin P.RunOther(); Error(Out + ' | log ' + Log.Take()); end;
        P.Preload(N);
        P.RunPick();
        Error(Out + ' | ' + P.Dump() + ' log ' + Log.Take());
    end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2Pin_T() begin Pick2(0, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2Three_T() begin Pick2(0, 3); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2One_T() begin Pick2(0, 1); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2Existing_T() begin Pick2(1, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2Twice_T() begin Pick2(2, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2PrevNextType_T() begin Pick2(3, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2NameSavedThenKey_T() begin Pick2(4, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2Blank_T() begin Pick2(5, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2After_T() begin Pick2(6, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2Filter_T() begin Pick2(7, 2); end;
    [Test] [HandlerFunctions('Pick2Handler')] procedure NPick2Other_T() begin Pick2(20, 2); end;
    [PageHandler] procedure Pick2Handler(var P: TestPage "RV Pick2")
    begin
        Out := 'on ' + P."Code".Value + ' [' + Log.Take() + ']';
        case PickMode of
            0, 20: begin P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
            1: begin P."Code".SetValue('P1'); Out += ' next=' + Format(P.Next()); end;
            2: begin P."Code".SetValue('T1'); P."Code".SetValue('T2'); Out += ' next=' + Format(P.Next()); end;
            3: begin P.Previous(); P.Next(); P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
            4: begin P.Name.SetValue('x'); P.Previous(); P.Next(); P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
            5: begin P."Code".SetValue(''); Out += ' next=' + Format(P.Next()); end;
            6: begin P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()) + ' on ' + P."Code".Value + ' [' + Log.Take() + ']';
                 Out += ' prev=' + Format(P.Previous()) + ' on ' + P."Code".Value + ' [' + Log.Take() + ']';
                 P.Name.SetValue('y'); Out += ' next=' + Format(P.Next()); end;
            7: begin P.Filter.SetFilter(Name, 'preloaded'); P.Last(); P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next()); end;
        end;
        Out += ' on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('Pick3Handler')] procedure NPick3TableTrig_T()
    var P: Page "RV Pick3";
    begin
        Out := ''; Log.Take();
        P.Preload(2);
        P.RunPick();
        Error(Out + ' | ' + P.Dump() + ' log ' + Log.Take());
    end;
    [PageHandler] procedure Pick3Handler(var P: TestPage "RV Pick3")
    begin
        Out := 'on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next());
        Out += ' on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('Pick4Handler')] procedure NPick4CurrRun_T()
    var P: Page "RV Pick4";
    begin
        Out := ''; Log.Take();
        P.RunPick();
        Error(Out + ' log ' + Log.Take());
    end;
    [Test] [HandlerFunctions('Pick4Handler')] procedure NPick4Static_T()
    begin
        Out := ''; Log.Take();
        Page.Run(Page::"RV Pick4");
        Error(Out + ' log ' + Log.Take());
    end;
    [PageHandler] procedure Pick4Handler(var P: TestPage "RV Pick4")
    begin
        Out := 'on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P."Code".SetValue('T1'); Out += ' next=' + Format(P.Next());
        Out += ' on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    local procedure Ws2(N: Integer; How: Integer; HMode: Integer)
    var P: Page "RV Ws2";
    begin
        WsMode := HMode; Out := ''; Log.Take();
        P.Fill(N, How);
        P.Run();
        Error(Out + ' | ' + P.Dump());
    end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2Pin_T() begin Ws2(4, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2DirtyThenUpd_T() begin Ws2(4, 0, 1); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2SecondRow_T() begin Ws2(4, 0, 2); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2EditTwice_T() begin Ws2(4, 0, 3); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2EditThenWalk_T() begin Ws2(4, 0, 4); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2FilterRecOutside_T() begin Ws2(4, 1, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2FilterRecInside_T() begin Ws2(4, 2, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2Middle_T() begin Ws2(4, 3, 5); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2Desc_T() begin Ws2(4, 4, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2Rows20_T() begin Ws2(20, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2Rows21_T() begin Ws2(21, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure NWs2NoteNext_T() begin Ws2(4, 0, 6); end;
    [PageHandler] procedure Ws2Handler(var P: TestPage "RV Ws2")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        if WsMode <> 5 then begin
            P.First();
            Out += ' first on ' + P."Code".Value + ' [' + Log.Take() + ']';
        end;
        case WsMode of
            0, 5: P.Name.SetValue('edited');
            1: begin P.Note.SetValue('q'); P.Name.SetValue('edited'); end;
            2: begin P.Next(); P.Name.SetValue('edited'); end;
            3: begin P.Name.SetValue('edited'); Out += ' mid on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']'; P.Name.SetValue('again'); end;
            4: begin P.Name.SetValue('edited'); Out += ' mid [' + Log.Take() + '] next=' + Format(P.Next()) + ' on ' + P."Code".Value + ' [' + Log.Take() + '] last=' + Format(P.Last()); end;
            6: begin P.Note.SetValue('q'); Out += ' next=' + Format(P.Next()); end;
        end;
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + '/' + P.Note.Value + ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('WsFltHandler')] procedure NWsViewFilterOutside_T()
    var P: Page "RV Ws Flt";
    begin
        Out := ''; Log.Take();
        P.Fill(4);
        P.Run();
        Error(Out);
    end;
    [PageHandler] procedure WsFltHandler(var P: TestPage "RV Ws Flt")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.First();
        Out += ' first on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.Name.SetValue('edited');
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('WsDescHandler')] procedure NWsViewDesc_T()
    var P: Page "RV Ws Desc";
    begin
        Out := ''; Log.Take();
        P.Fill(4);
        P.Run();
        Error(Out);
    end;
    [PageHandler] procedure WsDescHandler(var P: TestPage "RV Ws Desc")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.First();
        Out += ' first on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.Name.SetValue('edited');
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    [Test] procedure NFn25_T()
    var P: TestPage "RV List Fn";
    begin
        Seed(25); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NTreePos31Open_T()
    var P: TestPage "RV Tree Pos";
    begin
        Log.Take(); Log.SetTreeRows(31);
        P.OpenView();
        Error('open on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NTreePos60Open_T()
    var P: TestPage "RV Tree Pos"; S: Text;
    begin
        Log.Take(); Log.SetTreeRows(60);
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' first=' + Format(P.First()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        Error(S + ' next=' + Format(P.Next()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure NTreePos6_T()
    var P: TestPage "RV Tree Pos"; S: Text; C: Integer;
    begin
        Log.Take(); Log.SetTreeRows(6);
        P.OpenView();
        S := 'open on ' + P.Id.Value + ' [' + Log.Take() + ']';
        if P.First() then repeat C += 1; until (not P.Next()) or (C > 100);
        Error(S + ' walk=' + Format(C) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    local procedure Ws2M(N: Integer; How: Integer; HMode: Integer)
    var P: Page "RV Ws2";
    begin
        WsMode := HMode; Out := ''; Log.Take();
        P.Fill(N, How);
        P.RunModal();
        Error(Out + ' | ' + P.Dump() + ' log ' + Log.Take());
    end;
    [Test] [HandlerFunctions('Ws2MHandler')] procedure NWs2MPin_T() begin Ws2M(4, 0, 0); end;
    [Test] [HandlerFunctions('Ws2MHandler')] procedure NWs2MDirtyThenUpd_T() begin Ws2M(4, 0, 1); end;
    [Test] [HandlerFunctions('Ws2MHandler')] procedure NWs2MDirtyThenUpdNext_T() begin Ws2M(4, 0, 7); end;
    [ModalPageHandler] procedure Ws2MHandler(var P: TestPage "RV Ws2")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.First();
        case WsMode of
            0: P.Name.SetValue('edited');
            1: begin P.Note.SetValue('q'); P.Name.SetValue('edited'); end;
            7: begin P.Note.SetValue('q'); P.Name.SetValue('edited'); Out += ' mid ' + P.Name.Value + '/' + P.Note.Value + ' [' + Log.Take() + '] next=' + Format(P.Next()); end;
        end;
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + '/' + P.Note.Value + ' [' + Log.Take() + ']';
        P.OK().Invoke();
    end;
}
// The worksheet probes again over a table without code, as the pin's table is (C...), and more neighbors of the pins.
table 51600 "RV NRow"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Name; Text[30]) { }
        field(3; Seq; Integer) { }
        field(4; Note; Text[30]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } key(K2; Seq) { } }
}
page 51601 "RV List Act"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    actions { area(Processing) { action(Ping) { trigger OnAction() begin end; } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51602 "RV List Delayed"
{
    PageType = List;
    SourceTable = "RV Row";
    DelayedInsert = true;
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { }
    } } }
    trigger OnAfterGetRecord() begin Log.Add(Format(Rec.Id)); end;
    var Log: Codeunit "RV Log";
}
page 51616 "RV WsN2"
{
    PageType = Worksheet;
    SourceTable = "RV NRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq);
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Note; Rec.Note) { }
        field(Seq; Rec.Seq) { Editable = false; }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer; How: Integer)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
        case How of
            1: Rec.SetRange(Seq, 97, 99);
            2: begin Rec.SetRange(Seq, 97, 99); Rec.FindFirst(); end;
            3: Rec.Get('C2');
            4: Rec.Ascending(false);
        end;
    end;
    procedure Dump() Result: Text
    begin
        Result := 'on ' + Rec."Code" + ': ';
        if Rec.FindSet() then repeat Result += Rec."Code" + '=' + Rec.Name + '/' + Rec.Note + ';'; until Rec.Next() = 0;
    end;
    var Log: Codeunit "RV Log";
}
page 51617 "RV WsN Flt"
{
    PageType = Worksheet;
    SourceTable = "RV NRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq) where(Seq = filter(> 97));
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Seq; Rec.Seq) { Editable = false; }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
    end;
    var Log: Codeunit "RV Log";
}
page 51618 "RV WsN Desc"
{
    PageType = Worksheet;
    SourceTable = "RV NRow";
    SourceTableTemporary = true;
    SourceTableView = sorting(Seq) order(descending);
    InsertAllowed = false;
    layout { area(Content) { repeater(Lines) {
        field("Code"; Rec."Code") { Editable = false; }
        field(Name; Rec.Name) { trigger OnValidate() begin CurrPage.Update(false); end; }
        field(Seq; Rec.Seq) { Editable = false; }
    } } }
    trigger OnAfterGetRecord() begin Log.Add('R:' + Rec."Code"); end;
    procedure Fill(N: Integer)
    var i: Integer;
    begin
        for i := 1 to N do begin
            Rec.Init(); Rec."Code" := 'C' + Format(i); Rec.Name := 'n' + Format(i); Rec.Seq := 100 - i; Rec.Insert();
        end;
    end;
    var Log: Codeunit "RV Log";
}
codeunit 50960 "RV Tests 4"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "RV Log"; Out: Text; PickMode: Integer; WsMode: Integer;

    local procedure Seed(N: Integer)
    var R: Record "RV Row"; i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to N do begin R.Init(); R.Id := i; R.Name := 'r' + Format(i); R.Insert(); end;
        Log.Take();
    end;
    local procedure LastOn(N: Integer; Edit: Boolean)
    var P: TestPage "RV List"; S: Text;
    begin
        Seed(N);
        if Edit then P.OpenEdit() else P.OpenView();
        S := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' prev=' + Format(P.Previous()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' first=' + Format(P.First()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        S += ' next=' + Format(P.Next()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        P.Close();
        Error(S);
    end;
    local procedure Ws2(N: Integer; How: Integer; HMode: Integer)
    var P: Page "RV WsN2";
    begin
        WsMode := HMode; Out := ''; Log.Take();
        P.Fill(N, How);
        P.Run();
        Error(Out + ' | ' + P.Dump());
    end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2Pin_T() begin Ws2(4, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2DirtyThenUpd_T() begin Ws2(4, 0, 1); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2SecondRow_T() begin Ws2(4, 0, 2); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2EditTwice_T() begin Ws2(4, 0, 3); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2EditThenWalk_T() begin Ws2(4, 0, 4); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2FilterRecOutside_T() begin Ws2(4, 1, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2FilterRecInside_T() begin Ws2(4, 2, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2Middle_T() begin Ws2(4, 3, 5); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2Desc_T() begin Ws2(4, 4, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2Rows20_T() begin Ws2(20, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2Rows21_T() begin Ws2(21, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2NoteNext_T() begin Ws2(4, 0, 6); end;
    [PageHandler] procedure Ws2Handler(var P: TestPage "RV WsN2")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        if WsMode <> 5 then begin
            P.First();
            Out += ' first on ' + P."Code".Value + ' [' + Log.Take() + ']';
        end;
        case WsMode of
            0, 5: P.Name.SetValue('edited');
            1: begin P.Note.SetValue('q'); P.Name.SetValue('edited'); end;
            2: begin P.Next(); P.Name.SetValue('edited'); end;
            3: begin P.Name.SetValue('edited'); Out += ' mid on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']'; P.Name.SetValue('again'); end;
            4: begin P.Name.SetValue('edited'); Out += ' mid [' + Log.Take() + '] next=' + Format(P.Next()) + ' on ' + P."Code".Value + ' [' + Log.Take() + '] last=' + Format(P.Last()); end;
            6: begin P.Note.SetValue('q'); Out += ' next=' + Format(P.Next()); end;
        end;
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + '/' + P.Note.Value + ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('WsFltHandler')] procedure CWsViewFilterOutside_T()
    var P: Page "RV WsN Flt";
    begin
        Out := ''; Log.Take();
        P.Fill(4);
        P.Run();
        Error(Out);
    end;
    [PageHandler] procedure WsFltHandler(var P: TestPage "RV WsN Flt")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.First();
        Out += ' first on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.Name.SetValue('edited');
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    [Test] [HandlerFunctions('WsDescHandler')] procedure CWsViewDesc_T()
    var P: Page "RV WsN Desc";
    begin
        Out := ''; Log.Take();
        P.Fill(4);
        P.Run();
        Error(Out);
    end;
    [PageHandler] procedure WsDescHandler(var P: TestPage "RV WsN Desc")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.First();
        Out += ' first on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.Name.SetValue('edited');
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + ' [' + Log.Take() + ']';
    end;
    local procedure Ws2M(N: Integer; How: Integer; HMode: Integer)
    var P: Page "RV WsN2";
    begin
        WsMode := HMode; Out := ''; Log.Take();
        P.Fill(N, How);
        P.RunModal();
        Error(Out + ' | ' + P.Dump() + ' log ' + Log.Take());
    end;
    [Test] [HandlerFunctions('Ws2MHandler')] procedure CWs2MPin_T() begin Ws2M(4, 0, 0); end;
    [Test] [HandlerFunctions('Ws2MHandler')] procedure CWs2MDirtyThenUpd_T() begin Ws2M(4, 0, 1); end;
    [Test] [HandlerFunctions('Ws2MHandler')] procedure CWs2MDirtyThenUpdNext_T() begin Ws2M(4, 0, 7); end;
    [ModalPageHandler] procedure Ws2MHandler(var P: TestPage "RV WsN2")
    begin
        Out := 'open on ' + P."Code".Value + ' [' + Log.Take() + ']';
        P.First();
        case WsMode of
            0: P.Name.SetValue('edited');
            1: begin P.Note.SetValue('q'); P.Name.SetValue('edited'); end;
            7: begin P.Note.SetValue('q'); P.Name.SetValue('edited'); Out += ' mid ' + P.Name.Value + '/' + P.Note.Value + ' [' + Log.Take() + '] next=' + Format(P.Next()); end;
        end;
        Out += ' after on ' + P."Code".Value + '/' + P.Name.Value + '/' + P.Note.Value + ' [' + Log.Take() + ']';
        P.OK().Invoke();
    end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2Rows2_T() begin Ws2(2, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2Rows5_T() begin Ws2(5, 0, 0); end;
    [Test] [HandlerFunctions('Ws2Handler')] procedure CWs2ThirdRow_T() begin Ws2(4, 3, 0); end;
    [Test] procedure View61_T() begin LastOn(61, false); end;
    [Test] procedure Edit99_T() begin LastOn(99, true); end;
    [Test] procedure Edit100_T() begin LastOn(100, true); end;
    [Test] procedure Act45View_T()
    var P: TestPage "RV List Act";
    begin
        Seed(45); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    [Test] procedure Cap45View_T()
    var P: TestPage "RV List Delayed";
    begin
        Seed(45); P.OpenView();
        Error('open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']');
    end;
    local procedure Lookup(N: Integer; How: Integer)
    var L: Page "RV List"; R: Record "RV Row"; A: Action;
    begin
        Seed(N); Out := '';
        case How of
            0: begin L.Editable(false); L.LookupMode(true); A := L.RunModal(); L.GetRecord(R); end;
            1: A := Page.RunModal(Page::"RV List", R);
            2: begin L.LookupMode(true); A := L.RunModal(); L.GetRecord(R); end;
        end;
        Error(Out + ' ' + Format(A) + ' rec ' + Format(R.Id));
    end;
    [Test] [HandlerFunctions('LookupHandler')] procedure LookupVar45_T() begin Lookup(45, 0); end;
    [Test] [HandlerFunctions('LookupHandler')] procedure LookupVar101_T() begin Lookup(101, 0); end;
    [Test] [HandlerFunctions('LookupHandler')] procedure LookupVarEditable45_T() begin Lookup(45, 2); end;
    [Test] [HandlerFunctions('LookupHandler')] procedure StaticModal45_T() begin Lookup(45, 1); end;
    [Test] [HandlerFunctions('LookupHandler')] procedure StaticModal100_T() begin Lookup(100, 1); end;
    [ModalPageHandler] procedure LookupHandler(var P: TestPage "RV List")
    begin
        Out := 'open[' + Log.Take() + '] last=' + Format(P.Last()) + ' on ' + P.Id.Value + ' [' + Log.Take() + ']';
        P.OK().Invoke();
    end;
}
