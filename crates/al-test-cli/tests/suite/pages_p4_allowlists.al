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
