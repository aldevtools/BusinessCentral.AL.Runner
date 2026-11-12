//! The three paths of al-runner PR 7805 run only on their pinned scenario (review round 3): the pages, the filters and the
//! test's calls of the corpus file, compared for equality (crates/al-vm/src/pages/pinned_shape.rs `part_pages_are`,
//! `part_steps`). Every other shape here keeps what the runner did before the path existed: the same reads, or the refusal.

use al_testrun::Outcome;

/// AutoSplitKey on a part read in another order (`split_key_view.rs`, pin pages/parts/PartSplitKeyOtherSortKey.al).
const SPLIT: &str = r#"
table 70200 "RX Head" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 70201 "RX Line"
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "Seq No."; Integer) { } field(4; Descr; Text[30]) { } field(5; Val; Text[30]) { } }
    keys { key(PK; "Doc No.", "Line No.") { } key(Seq; "Doc No.", "Seq No.") { } }
}
codeunit 70202 "RX Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Line: Text) begin if Entries <> '' then Entries += ' | '; Entries += Line; end;
    procedure Take() Result: Text begin Result := '[' + Entries + ']'; Entries := ''; end;
}
codeunit 70203 "RX Lib"
{
    procedure Seed(N: Integer)
    var Head: Record "RX Head"; Line: Record "RX Line"; L: Codeunit "RX Log"; i: Integer;
    begin
        Line.DeleteAll(); Head.DeleteAll();
        Head."No." := 'D1'; Head.Insert(); Head."No." := 'D2'; Head.Insert();
        for i := 1 to N do begin Line.Init(); Line."Doc No." := 'D1'; Line."Line No." := i * 10000; Line."Seq No." := i; Line.Descr := 'r' + Format(i); Line.Val := 'old'; Line.Insert(); end;
        L.Take();
    end;
    procedure Rows(): Text
    var Line: Record "RX Line"; T: Text;
    begin
        if Line.FindSet() then repeat T += Format(Line."Line No.") + ':' + Format(Line."Seq No.") + ':' + Line.Descr + ':' + Line.Val + ' '; until Line.Next() = 0;
        exit(T);
    end;
}
page 70220 "RX Split Lines"
{
    PageType = ListPart; SourceTable = "RX Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } } } }
    trigger OnNewRecord(B: Boolean) var L: Codeunit "RX Log"; begin L.Add('NEW ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No.")); end;
    trigger OnInsertRecord(B: Boolean): Boolean var L: Codeunit "RX Log"; begin L.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No.")); exit(true); end;
}
page 70221 "RX Split Card Seq"
{
    PageType = Card; SourceTable = "RX Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RX Split Lines") { SubPageLink = "Doc No." = field("No."); SubPageView = sorting("Doc No.", "Seq No."); } } }
}
page 70222 "RX Split Card SeqWhere"
{
    PageType = Card; SourceTable = "RX Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RX Split Lines") { SubPageLink = "Doc No." = field("No."); SubPageView = sorting("Doc No.", "Seq No.") where(Descr = filter(<> 'r2')); } } }
}
page 70223 "RX Split Card SeqWhereNone"
{
    PageType = Card; SourceTable = "RX Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RX Split Lines") { SubPageLink = "Doc No." = field("No."); SubPageView = sorting("Doc No.", "Seq No.") where(Descr = const('zzz')); } } }
}
page 70224 "RX Split Lines OwnFilter"
{
    PageType = ListPart; SourceTable = "RX Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    SourceTableView = sorting("Doc No.", "Seq No.");
    layout { area(Content) { repeater(R) { field("Line No."; Rec."Line No.") { } field("Seq No."; Rec."Seq No.") { } field(Descr; Rec.Descr) { } } } }
    trigger OnOpenPage() begin Rec.FilterGroup(2); Rec.SetFilter(Descr, '<>r2'); Rec.FilterGroup(0); end;
    trigger OnNewRecord(B: Boolean) var L: Codeunit "RX Log"; begin L.Add('NEW ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No.")); end;
    trigger OnInsertRecord(B: Boolean): Boolean var L: Codeunit "RX Log"; begin L.Add('INS ' + Format(Rec."Line No.") + ' x=' + Format(xRec."Line No.")); exit(true); end;
}
page 70225 "RX Split Card OwnFilter"
{
    PageType = Card; SourceTable = "RX Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RX Split Lines OwnFilter") { SubPageLink = "Doc No." = field("No."); } } }
}
codeunit 70300 "RX S"
{
    Subtype = Test;
    var Log: Codeunit "RX Log"; Lib: Codeunit "RX Lib";

    // The pinned calls (PartSplitKeyOtherSortKey.al:599 SeqAsc_NewAfterLast) on a view that only sorts.
    [Test] procedure S0_Pinned() var P: TestPage "RX Split Card Seq";
    begin Lib.Seed(3); P.OpenEdit(); P.GoToKey('D1'); P.Lines.Last(); Log.Take(); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Close(); Error('rows=%1 log=%2', Lib.Rows(), Log.Take()); end;

    // Not pinned: a SubPageView whose `where` hides the row after the one the line is started below.
    [Test] procedure S1_SubPageViewWhereHidesNextRow() var P: TestPage "RX Split Card SeqWhere";
    begin Lib.Seed(3); P.OpenEdit(); P.GoToKey('D1'); P.Lines.First(); Log.Take(); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Close(); Error('rows=%1 log=%2', Lib.Rows(), Log.Take()); end;

    // Not pinned: a SubPageView whose `where` hides every row.
    [Test] procedure S2_SubPageViewWhereHidesAll() var P: TestPage "RX Split Card SeqWhereNone";
    begin Lib.Seed(2); P.OpenEdit(); P.GoToKey('D1'); Log.Take(); P.Lines.New(); P.Lines."Seq No.".SetValue(9); P.Close(); Error('rows=%1 log=%2', Lib.Rows(), Log.Take()); end;

    // Not pinned: the part's OnOpenPage sets a filter in FilterGroup(2).
    [Test] procedure S3_PartOwnFilterGroup2HidesNextRow() var P: TestPage "RX Split Card OwnFilter";
    begin Lib.Seed(3); P.OpenEdit(); P.GoToKey('D1'); P.Lines.First(); Log.Take(); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Close(); Error('rows=%1 log=%2', Lib.Rows(), Log.Take()); end;

    // Not pinned: the host moves away and back between two new rows.
    [Test] procedure S6_HostMovedBetweenNews() var P: TestPage "RX Split Card Seq";
    begin Lib.Seed(3); P.OpenEdit(); P.GoToKey('D1'); P.Lines.Last(); P.Next(); P.Previous(); P.Lines.Last(); P.Lines.New(); P.Lines.Descr.SetValue('y'); P.Close(); Error('rows=%1 log=%2', Lib.Rows(), Log.Take()); end;
}
"#;

/// A new part row after a variable control's OnValidate ran CurrPage.Update() (`part_unsettled.rs`, pin
/// pages/parts/PartNewRowPageVariable.al).
const UNSETTLED: &str = r#"
table 70200 "RX Head" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 70201 "RX Line"
{
    fields { field(1; "Doc No."; Code[10]) { } field(2; "Line No."; Integer) { } field(3; "Seq No."; Integer) { } field(4; Descr; Text[30]) { } field(5; Val; Text[30]) { } }
    keys { key(PK; "Doc No.", "Line No.") { } key(Seq; "Doc No.", "Seq No.") { } }
}
codeunit 70202 "RX Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Line: Text) begin if Entries <> '' then Entries += ' | '; Entries += Line; end;
    procedure Take() Result: Text begin Result := '[' + Entries + ']'; Entries := ''; end;
}
codeunit 70203 "RX Lib"
{
    procedure Seed(N: Integer)
    var Head: Record "RX Head"; Line: Record "RX Line"; L: Codeunit "RX Log"; i: Integer;
    begin
        Line.DeleteAll(); Head.DeleteAll();
        Head."No." := 'D1'; Head.Insert(); Head."No." := 'D2'; Head.Insert();
        for i := 1 to N do begin Line.Init(); Line."Doc No." := 'D1'; Line."Line No." := i * 10000; Line."Seq No." := i; Line.Descr := 'r' + Format(i); Line.Val := 'old'; Line.Insert(); end;
        L.Take();
    end;
    procedure Rows(): Text
    var Line: Record "RX Line"; T: Text;
    begin
        if Line.FindSet() then repeat T += Format(Line."Line No.") + ':' + Format(Line."Seq No.") + ':' + Line.Descr + ':' + Line.Val + ' '; until Line.Next() = 0;
        exit(T);
    end;
}
page 70210 "RX Var Lines"
{
    PageType = ListPart; SourceTable = "RX Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) {
        field("Line No."; Rec."Line No.") { }
        field(Descr; Rec.Descr) { }
        field(Seq; Rec."Seq No.") { }
        field(Upd; UpdTxt) { trigger OnValidate() var L: Codeunit "RX Log"; begin L.Add('V-UPD ' + Format(Rec."Line No.")); Rec.Validate(Val, UpdTxt); CurrPage.Update(); end; }
    } } }
    var UpdTxt: Text;
    trigger OnAfterGetRecord() var L: Codeunit "RX Log"; begin L.Add('AGR ' + Format(Rec."Line No.")); UpdTxt := Rec.Val; end;
    trigger OnNewRecord(B: Boolean) var L: Codeunit "RX Log"; begin L.Add('NEW ' + Format(Rec."Line No.")); end;
    trigger OnInsertRecord(B: Boolean): Boolean var L: Codeunit "RX Log"; begin L.Add('INS ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "RX Log"; begin L.Add('MOD ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
}
page 70214 "RX Var Lines RecUpd"
{
    PageType = ListPart; SourceTable = "RX Line"; AutoSplitKey = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) {
        field("Line No."; Rec."Line No.") { }
        field(Descr; Rec.Descr) { }
        field(Seq; Rec."Seq No.") { }
        field(DescrUpd; Rec.Val) { trigger OnValidate() var L: Codeunit "RX Log"; begin L.Add('V-REC ' + Format(Rec."Line No.")); CurrPage.Update(); end; }
        field(Upd; UpdTxt) { trigger OnValidate() var L: Codeunit "RX Log"; begin L.Add('V-UPD ' + Format(Rec."Line No.")); Rec.Validate(Val, UpdTxt); CurrPage.Update(); end; }
    } } }
    var UpdTxt: Text;
    trigger OnAfterGetRecord() var L: Codeunit "RX Log"; begin L.Add('AGR ' + Format(Rec."Line No.")); UpdTxt := Rec.Val; end;
    trigger OnNewRecord(B: Boolean) var L: Codeunit "RX Log"; begin L.Add('NEW ' + Format(Rec."Line No.")); end;
    trigger OnInsertRecord(B: Boolean): Boolean var L: Codeunit "RX Log"; begin L.Add('INS ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "RX Log"; begin L.Add('MOD ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
}
page 70211 "RX Var Doc"
{
    PageType = Document; SourceTable = "RX Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RX Var Lines") { SubPageLink = "Doc No." = field("No."); } } }
}
page 70215 "RX Var Doc RecUpd"
{
    PageType = Document; SourceTable = "RX Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RX Var Lines RecUpd") { SubPageLink = "Doc No." = field("No."); } } }
}
page 70212 "RX Plain Lines"
{
    PageType = ListPart; SourceTable = "RX Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) {
        field("Line No."; Rec."Line No.") { }
        field(Descr; Rec.Descr) { }
        field(Upd; UpdTxt) { trigger OnValidate() var L: Codeunit "RX Log"; begin L.Add('V-UPD ' + Format(Rec."Line No.")); Rec.Validate(Val, UpdTxt); CurrPage.Update(); end; }
    } } }
    var UpdTxt: Text;
    trigger OnAfterGetRecord() var L: Codeunit "RX Log"; begin L.Add('AGR ' + Format(Rec."Line No.")); UpdTxt := Rec.Val; end;
    trigger OnNewRecord(B: Boolean) var L: Codeunit "RX Log"; begin L.Add('NEW ' + Format(Rec."Line No.")); end;
    trigger OnInsertRecord(B: Boolean): Boolean var L: Codeunit "RX Log"; begin L.Add('INS ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "RX Log"; begin L.Add('MOD ' + Format(Rec."Line No.") + ' ' + Rec.Val); exit(true); end;
}
page 70213 "RX Plain Doc"
{
    PageType = Document; SourceTable = "RX Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(Lines; "RX Plain Lines") { SubPageLink = "Doc No." = field("No."); } } }
}
codeunit 70290 "RX U"
{
    Subtype = Test;
    var Log: Codeunit "RX Log"; Lib: Codeunit "RX Lib";

    // The pinned calls (PartNewRowPageVariable.al:1218): New, two record fields, the variable control, Close.
    [Test] procedure U1_Pinned() var P: TestPage "RX Var Doc"; T: Text;
    begin Lib.Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Lines.Seq.SetValue(7); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); P.Close(); Error('upd=%1 closed=%2', T, Log.Take()); end;

    // Not pinned: the test deletes the row between the update and the close.
    [Test] procedure U4_RowDeletedBeforeClose() var P: TestPage "RX Var Doc"; T: Text; Line: Record "RX Line";
    begin Lib.Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Lines.Seq.SetValue(7); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); Line.Get('D1', 20000); Line.Delete(); P.Close(); Error('upd=%1 closed=%2 rows=%3', T, Log.Take(), Lib.Rows()); end;

    // Not pinned: a second value in the variable control.
    [Test] procedure U5_SecondUpdate() var P: TestPage "RX Var Doc"; T: Text; U: Text;
    begin Lib.Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Lines.Seq.SetValue(7); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); P.Lines.Upd.SetValue('2'); U := Log.Take(); P.Close(); Error('upd=%1 upd2=%2 closed=%3', T, U, Log.Take()); end;

    // Not pinned: the update runs in the OnValidate of a control over a field of Rec.
    [Test] procedure U6_RecordFieldControlUpdate() var P: TestPage "RX Var Doc RecUpd"; T: Text;
    begin Lib.Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('x'); Log.Take(); P.Lines.DescrUpd.SetValue('1'); T := Log.Take(); P.Close(); Error('upd=%1 closed=%2', T, Log.Take()); end;

    // Not pinned: a part without AutoSplitKey whose key the test types.
    [Test] procedure U7_PlainPart_TypedKey() var P: TestPage "RX Plain Doc"; T: Text;
    begin Lib.Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines."Line No.".SetValue(20000); P.Lines.Descr.SetValue('x'); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); P.Close(); Error('upd=%1 closed=%2', T, Log.Take()); end;

    // Not pinned: a read of a host field between the update and the close.
    [Test] procedure U8_HostFieldReadThenClose() var P: TestPage "RX Var Doc"; T: Text; V: Text;
    begin Lib.Seed(1); P.OpenEdit(); P.GoToKey('D1'); P.Lines.New(); P.Lines.Descr.SetValue('x'); P.Lines.Seq.SetValue(7); Log.Take(); P.Lines.Upd.SetValue('1'); T := Log.Take(); V := P."No.".Value; P.Close(); Error('upd=%1 closed=%2', T, Log.Take()); end;
}
"#;

/// First() and Last() that read nothing on a find ListPart after a TestPart filter (`end_move_no_read.rs`,
/// `part_filter_find.rs`, pin testpage/parts/PartFilterExpressionControl.al).
const FIND: &str = r#"
table 70400 "RY Item" { fields { field(1; "No."; Code[10]) { } } keys { key(PK; "No.") { } } }
table 70401 "RY Entry"
{
    fields { field(1; "Item No."; Code[10]) { } field(2; "Entry No."; Integer) { } field(3; Day; Date) { } }
    keys { key(PK; "Item No.", "Entry No.") { } }
}
codeunit 70402 "RY Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Line: Text) begin Entries += Line + ' '; end;
    procedure Take() Result: Text begin Result := '[' + Entries + ']'; Entries := ''; end;
}
codeunit 70403 "RY Lib"
{
    procedure Seed()
    var Item: Record "RY Item"; E: Record "RY Entry"; L: Codeunit "RY Log"; i: Integer;
    begin
        E.DeleteAll(); Item.DeleteAll();
        Item."No." := 'I1'; Item.Insert(); Item."No." := 'I22'; Item.Insert();
        for i := 1 to 5 do begin E.Init(); E."Item No." := 'I1'; E."Entry No." := i; E.Day := DMY2Date(i, 1, 2025); E.Insert(); end;
        for i := 6 to 7 do begin E.Init(); E."Item No." := 'I22'; E."Entry No." := i; E.Day := DMY2Date(i, 1, 2025); E.Insert(); end;
        L.Take();
    end;
}
page 70410 "RY Find Entries"
{
    PageType = ListPart; SourceTable = "RY Entry"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } field(Day; Rec.Day) { } } } }
    trigger OnFindRecord(Which: Text): Boolean var L: Codeunit "RY Log"; begin L.Add('find(' + Which + ')'); exit(Rec.Find(Which)); end;
    trigger OnNextRecord(Steps: Integer): Integer var L: Codeunit "RY Log"; begin L.Add('next(' + Format(Steps) + ')'); exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() var L: Codeunit "RY Log"; begin L.Add('agr' + Format(Rec."Entry No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "RY Log"; begin L.Add('agcr' + Format(Rec."Entry No.")); end;
}
page 70411 "RY Find Entries Desc"
{
    PageType = ListPart; SourceTable = "RY Entry"; ApplicationArea = All; UsageCategory = None;
    SourceTableView = sorting("Item No.", "Entry No.") order(descending);
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } field(Day; Rec.Day) { } } } }
    trigger OnFindRecord(Which: Text): Boolean var L: Codeunit "RY Log"; begin L.Add('find(' + Which + ')'); exit(Rec.Find(Which)); end;
    trigger OnNextRecord(Steps: Integer): Integer var L: Codeunit "RY Log"; begin L.Add('next(' + Format(Steps) + ')'); exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() var L: Codeunit "RY Log"; begin L.Add('agr' + Format(Rec."Entry No.")); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "RY Log"; begin L.Add('agcr' + Format(Rec."Entry No.")); end;
}
page 70412 "RY LP Linked"
{
    PageType = ListPlus; SourceTable = "RY Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(E; "RY Find Entries") { SubPageLink = "Item No." = field("No."); } } }
}
page 70413 "RY LP Linked ViewDesc"
{
    PageType = ListPlus; SourceTable = "RY Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(E; "RY Find Entries") { SubPageLink = "Item No." = field("No."); SubPageView = sorting("Item No.", "Entry No.") order(descending); } } }
}
page 70414 "RY LP Linked OwnDesc"
{
    PageType = ListPlus; SourceTable = "RY Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(E; "RY Find Entries Desc") { SubPageLink = "Item No." = field("No."); } } }
}
page 70415 "RY LP Linked ViewWhere"
{
    PageType = ListPlus; SourceTable = "RY Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } part(E; "RY Find Entries") { SubPageLink = "Item No." = field("No."); SubPageView = where("Entry No." = filter(> 1)); } } }
}
codeunit 70490 "RY F"
{
    Subtype = Test;
    var Log: Codeunit "RY Log"; Lib: Codeunit "RY Lib";

    [Test] procedure F0_Pinned() var P: TestPage "RY LP Linked"; T: Text; B: Boolean; begin Lib.Seed(); P.OpenView(); P.GoToKey('I1'); Log.Take(); P.E.Filter.SetFilter("Entry No.", '2..3'); T := Log.Take(); B := P.E.First(); Error('%1 first=%2 %3', T, B, Log.Take()); end;
    [Test] procedure F3_Filter_Next_Last() var P: TestPage "RY LP Linked"; T: Text; B: Boolean; begin Lib.Seed(); P.OpenView(); P.GoToKey('I1'); P.E.Filter.SetFilter("Entry No.", '2..3'); P.E.Next(); Log.Take(); B := P.E.Last(); Error('last=%1 %2', B, Log.Take()); end;
    [Test] procedure F4_Filter_Next_Previous_First() var P: TestPage "RY LP Linked"; T: Text; B: Boolean; begin Lib.Seed(); P.OpenView(); P.GoToKey('I1'); P.E.Filter.SetFilter("Entry No.", '2..4'); P.E.Next(); P.E.Previous(); Log.Take(); B := P.E.First(); Error('first=%1 %2', B, Log.Take()); end;
    [Test] procedure F5_SubPageViewWhere_Filter() var P: TestPage "RY LP Linked ViewWhere"; T: Text; B: Boolean; begin Lib.Seed(); P.OpenView(); P.GoToKey('I1'); Log.Take(); P.E.Filter.SetFilter("Entry No.", '..3'); T := Log.Take(); B := P.E.First(); Error('%1 row=%4 first=%2 %3', T, B, Log.Take(), P.E."Entry No.".Value); end;
    [Test] procedure F6_Filter_RowInsertedBefore_First() var P: TestPage "RY LP Linked"; T: Text; B: Boolean; E: Record "RY Entry"; begin Lib.Seed(); E.Get('I1', 2); E.Delete(); P.OpenView(); P.GoToKey('I1'); P.E.Filter.SetFilter("Entry No.", '2..4'); E.Init(); E."Item No." := 'I1'; E."Entry No." := 2; E.Insert(); Log.Take(); B := P.E.First(); Error('first=%1 %2 row=%3', B, Log.Take(), P.E."Entry No.".Value); end;
    [Test] procedure F7_Filter_Last_Filter_Last() var P: TestPage "RY LP Linked"; T: Text; B: Boolean; begin Lib.Seed(); P.OpenView(); P.GoToKey('I1'); P.E.Filter.SetFilter("Entry No.", '2..3'); P.E.Last(); Log.Take(); P.E.Filter.SetFilter(Day, Format(DMY2Date(2, 1, 2025)) + '..'); T := Log.Take(); B := P.E.First(); Error('%1 row=%4 first=%2 %3', T, B, Log.Take(), P.E."Entry No.".Value); end;
    [Test] procedure F8_HostGoToKeyBack_First() var P: TestPage "RY LP Linked"; T: Text; B: Boolean; begin Lib.Seed(); P.OpenView(); P.GoToKey('I1'); P.E.Filter.SetFilter("Entry No.", '2..7'); P.GoToKey('I22'); T := Log.Take(); Log.Take(); B := P.E.First(); Error('first=%1 %2 row=%3', B, Log.Take(), P.E."Entry No.".Value); end;
}
"#;

fn outcome<'a>(outcomes: &'a [(String, Outcome)], name: &str) -> &'a Outcome {
    &outcomes
        .iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
}

fn failure(outcomes: &[(String, Outcome)], name: &str) -> String {
    match outcome(outcomes, name) {
        Outcome::Fail(why) => why.lines().next().unwrap_or_default().trim_end().to_string(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn split_key_in_another_order_runs_only_without_any_other_filter() {
    let o = al_testkit::outcomes_in(SPLIT, "RX S");
    let f = failure(&o, "S0_Pinned");
    assert!(
        f.starts_with("rows=10000:1:r1:old 20000:2:r2:old 30000:3:r3:old 40000:0:x:"),
        "{f}"
    );
    for name in [
        "S1_SubPageViewWhereHidesNextRow",
        "S2_SubPageViewWhereHidesAll",
        "S3_PartOwnFilterGroup2HidesNextRow",
        "S6_HostMovedBetweenNews",
    ] {
        match outcome(&o, name) {
            Outcome::NotRun(why) => assert!(why.contains("not read in primary key order"), "{name}: {why}"),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn unsettled_part_update_runs_only_on_the_pinned_pages_and_calls() {
    let o = al_testkit::outcomes_in(UNSETTLED, "RX U");
    let before = "AGR 20000 | AGR 10000 | AGR 20000 | AGR 20000";
    let pinned = "upd=[MOD 20000  | V-UPD 20000 | MOD 20000 1 | AGR 20000 | AGR 10000]";
    let mut wrong = Vec::new();
    let mut check = |name: &str, want: String| {
        let f = failure(&o, name);
        if f != want {
            wrong.push(format!("{name}: {f}\n  want: {want}"));
        }
    };
    check("U1_Pinned", format!("{pinned} closed=[AGR 20000]"));
    check(
        "U4_RowDeletedBeforeClose",
        format!("{pinned} closed=[] rows=10000:1:r1:old"),
    );
    check(
        "U5_SecondUpdate",
        format!("{pinned} upd2=[V-UPD 20000 | MOD 20000 2 | {before}] closed=[]"),
    );
    check(
        "U6_RecordFieldControlUpdate",
        format!("upd=[INS 20000  | V-REC 20000 | MOD 20000 1 | {before}] closed=[]"),
    );
    check(
        "U7_PlainPart_TypedKey",
        format!("upd=[MOD 20000  | V-UPD 20000 | MOD 20000 1 | {before}] closed=[]"),
    );
    check("U8_HostFieldReadThenClose", format!("{pinned} closed=[]"));
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn find_part_end_moves_read_nothing_only_after_the_pinned_calls() {
    let o = al_testkit::outcomes_in(FIND, "RY F");
    let mut wrong = Vec::new();
    let mut check = |name: &str, want: &str| {
        let f = failure(&o, name);
        if f != want {
            wrong.push(format!("{name}: {f}\n  want: {want}"));
        }
    };
    // The pinned calls (PartFilterExpressionControl.al:676 EntriesFind_FilterOneDay_First_Read).
    check(
        "F0_Pinned",
        "[find(-) agr2 next(1) agr3 next(1) agr2 find(=) agr2 agcr2 ] first=Yes []",
    );
    // Not pinned, the reads before the path existed: Last() after a Next() that moved; First() after Next() and Previous();
    // a SubPageView with `where`; a row the test inserted before the first one; a second filter after Last(); the host's
    // GoToKey between the filter and First().
    check("F3_Filter_Next_Last", "last=Yes [agr3 find(=) agr3 agcr3 ]");
    check("F4_Filter_Next_Previous_First", "first=Yes [agr2 find(=) agr2 agcr2 ]");
    check(
        "F5_SubPageViewWhere_Filter",
        "[find(=><) agr2 next(1) agr3 next(1) agr2 next(-1) find(=) agr2 agcr2 ] row=2 first=Yes [agr2 find(=) agr2 agcr2 ]",
    );
    check(
        "F6_Filter_RowInsertedBefore_First",
        "first=Yes [agr3 find(=) agr3 agcr3 ] row=3",
    );
    check(
        "F7_Filter_Last_Filter_Last",
        "[find(=><) agr3 next(1) next(-1) find(=) agr3 agcr3 ] row=3 first=Yes [agr3 find(=) agr3 agcr3 ]",
    );
    check("F8_HostGoToKeyBack_First", "first=Yes [agr6 find(=) agr6 agcr6 ] row=6");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
