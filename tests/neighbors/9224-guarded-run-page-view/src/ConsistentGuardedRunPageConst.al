// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-consistent-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/codeunit/codeunit-run-method
//   https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-sourcetableview-property
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); own objects 99551-99564 (GPC Kind, GPC Z, GPC M, GPC Quotes, GPC Orders,
//   GPC Invoices, GPC Credit Memos, GPC Kind Orders, GPC Log, GPC Inner, GPC Run, GPC Tests, GPC Order Card, GPC Invoice Card)
// Probe: from al-runner issue 9224 (follow-up of al-runner issue 9182 and corpus probe ConsistentGuardedRunRecTwoKey.al). There the
//   group-2 equality on the Enum key field came from SetRange. Here it comes from a list page's SourceTableView
//   where("Doc Kind" = const(...)), like the Purchase document list pages that codeunit 134762 opens: the page action copies Rec
//   and hands the copy to a guarded Codeunit.Run whose codeunit marks a table with Consistent(false).
// Area: the cells (the page action runs the guarded run; the test then raises Error('END') under asserterror; then a Commit in the
//   caller; then the durable rows):
//   - Argument table GPC Z: Doc Kind (Enum GPC Kind, key 1), No. (Code, key 2), Name, Kind (Enum GPC Kind, not in the key). Rows:
//     (Quote, 1) q1, (Quote, 2) q2, (Order, 1) o1, (Order, 2) o2 (Kind Order), (Invoice, 1) i1 (Kind Order), (Invoice, 2) i2,
//     (Credit Memo, 1) c1, (Credit Memo, 2) c2. The marked table is GPC M (never the argument).
//   - Pages (the action works on the first row the TestPage shows): GPC Quotes (const Quote, ordinal 0), GPC Orders (const Order),
//     GPC Invoices (const Invoice), GPC Credit Memos (const Credit Memo), GPC Kind Orders (Kind = const(Order), a field outside
//     the key). Shapes on GPC Orders: OrderTpFilter (the TestPage filter No. = 1), OrderSetRange (the action adds
//     SetRange("No.", '1') to the copy), OrderSetFilter (the action adds SetFilter("No.", '1') to the copy), OrderDirect (no copy: the action runs the guarded Codeunit.Run on the page's
//     Rec itself).
//   - Document pages with a filter view, like the Purchase Order / Invoice card pages of the base app
//     (SourceTableView = where("Document Type" = filter(Order))): GPC Order Card (filter(Order)), GPC Invoice Card
//     (filter(Invoice)); OrderCardDirect = GPC Order Card with the run on the page's Rec itself.
//   - Marks (steps of the inner codeunit): Fails = mark,err; Ends = mark; PlainCommit = mark,commit (mark = insert a row into GPC M
//     and Consistent(false) on GPC M).
//   Each cell reads: the page Rec's filter state and name, the inner codeunit's view of the argument (FilterGroup, GetFilters of
//   groups 0, 1, 2, 4, 6, filtered count), the run's return value, last error text and code, the copy's name and filter state after
//   the run, the error the asserterror sees, the error of a later Commit, and the durable rows.
// What BC did (the same on 27.5, 28.5 and 29.0, measured on corpus CI):
//   - Every page view (const on the key field, const on a field outside the key, filter(...) on a document card page) reaches
//     the inner codeunit as a filter in FilterGroup 2 (GetFilters of group 2, e.g. 'Doc Kind: Order'); the current group is 0.
//     The copy and the page's own Rec give the same view. A TestPage filter or a SetRange/SetFilter in the action lands in group 0.
//   - Fails (mark,err): Codeunit.Run returns false, last error 'inner failed' (code Dialog); the caller's asserterror sees END.
//   - PlainCommit (mark,commit): Codeunit.Run returns false with the Consistent error on GPC M (code DB:TableInConsistent);
//     the asserterror sees END.
//   - Ends (mark): Codeunit.Run does not return (no 'run=' line in the log): the Consistent error on GPC M leaves the run
//     and the page action, and the test's asserterror sees it (code TestWrapped:CSide).
//   - A later Commit in the caller succeeds (the asserterror around Commit then Error('END') sees END); only the seed rows
//     are durable (m=[m1;]).
// The values are what BC produced.
// local object: the tests must commit (a guarded Codeunit.Run commits), and the page action and inner codeunit are the code under test.
// BC versions: 27.5, 28.5, 29.0

enum 99551 "GPC Kind"
{
    Extensible = false;

    value(0; Quote) { }
    value(1; Order) { }
    value(2; Invoice) { }
    value(3; "Credit Memo") { }
}

table 99552 "GPC Z"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Doc Kind"; Enum "GPC Kind") { }
        field(2; "No."; Code[20]) { }
        field(3; Name; Text[30]) { }
        field(4; Kind; Enum "GPC Kind") { }
    }

    keys
    {
        key(PK; "Doc Kind", "No.") { Clustered = true; }
    }
}

table 99553 "GPC M"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

page 99554 "GPC Quotes"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Quote));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

page 99555 "GPC Orders"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

page 99556 "GPC Invoices"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const(Invoice));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

page 99557 "GPC Credit Memos"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = const("Credit Memo"));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

page 99558 "GPC Kind Orders"
{
    PageType = List;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where(Kind = const(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

page 99563 "GPC Order Card"
{
    PageType = Document;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = filter(Order));
    Editable = false;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

page 99564 "GPC Invoice Card"
{
    PageType = Document;
    ApplicationArea = All;
    UsageCategory = None;
    SourceTable = "GPC Z";
    SourceTableView = where("Doc Kind" = filter(Invoice));
    Editable = false;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(RunGuarded)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Runner: Codeunit "GPC Run";
                begin
                    Runner.RunCopy(Rec);
                end;
            }
        }
    }
}

// The steps the inner codeunit runs, the extra filter the action adds, and a log (a SingleInstance codeunit is not rolled back).
codeunit 99559 "GPC Log"
{
    SingleInstance = true;

    var
        Steps: Text;
        Extra: Text;
        LogText: Text;

    procedure Set(NewSteps: Text; NewExtra: Text)
    begin
        Steps := NewSteps;
        Extra := NewExtra;
        LogText := '';
    end;

    procedure GetSteps(): Text
    begin
        exit(Steps);
    end;

    procedure GetExtra(): Text
    begin
        exit(Extra);
    end;

    procedure Log(Line: Text)
    begin
        LogText += Line + ';';
    end;

    procedure GetLog(): Text
    begin
        exit(LogText);
    end;

    // The filter state of a record: the FilterGroup it reports, the filters of groups 0, 1, 2, 4, 6 and the filtered count.
    procedure Describe(var RecRef: RecordRef; Fg: Integer): Text
    var
        Groups: List of [Integer];
        G: Integer;
        Original: Integer;
        Result: Text;
    begin
        Groups.Add(0);
        Groups.Add(1);
        Groups.Add(2);
        Groups.Add(4);
        Groups.Add(6);
        Original := RecRef.FilterGroup();
        Result := 'fg=' + Format(Fg);
        foreach G in Groups do begin
            RecRef.FilterGroup(G);
            Result += ' g' + Format(G) + '=[' + RecRef.GetFilters() + ']';
        end;
        RecRef.FilterGroup(Original);
        Result += ' count=' + Format(RecRef.Count());
        exit(Result);
    end;
}

// The inner codeunit: logs the filter state of its argument, then runs the steps of "GPC Log" (comma separated).
//   mark: insert a row into GPC M and Consistent(false) on GPC M. commit: Commit. err: Error('inner failed').
codeunit 99560 "GPC Inner"
{
    TableNo = "GPC Z";

    trigger OnRun()
    var
        LogCu: Codeunit "GPC Log";
        M: Record "GPC M";
        RecRef: RecordRef;
        Step: Text;
    begin
        RecRef.GetTable(Rec);
        LogCu.Log('inner ' + LogCu.Describe(RecRef, Rec.FilterGroup()) + ' name=' + Rec.Name);
        foreach Step in LogCu.GetSteps().Split(',') do
            case Step of
                'mark':
                    begin
                        M.Init();
                        M.Id := 50;
                        M.Name := 'state';
                        M.Insert();
                        M.Consistent(false);
                    end;
                'commit':
                    Commit();
                'err':
                    Error('inner failed');
            end;
    end;
}

// What the page action calls: copies the page's Rec, adds the extra filter, runs the inner codeunit guarded and logs the outcome.
codeunit 99561 "GPC Run"
{
    procedure RunCopy(var PageRec: Record "GPC Z")
    var
        LogCu: Codeunit "GPC Log";
        R: Record "GPC Z";
        RecRef: RecordRef;
        Ok: Boolean;
    begin
        RecRef.GetTable(PageRec);
        LogCu.Log('page ' + LogCu.Describe(RecRef, PageRec.FilterGroup()) + ' name=' + PageRec.Name);
        R.Copy(PageRec);
        case LogCu.GetExtra() of
            'setrange':
                R.SetRange("No.", '1');
            'setfilter':
                R.SetFilter("No.", '1');
        end;
        ClearLastError();
        if LogCu.GetExtra() = 'direct' then
            Ok := Codeunit.Run(Codeunit::"GPC Inner", PageRec)
        else
            Ok := Codeunit.Run(Codeunit::"GPC Inner", R);
        LogCu.Log('run=' + Format(Ok));
        LogCu.Log('text=' + GetLastErrorText());
        LogCu.Log('code=' + GetLastErrorCode());
        Clear(RecRef);
        if LogCu.GetExtra() = 'direct' then begin
            RecRef.GetTable(PageRec);
            LogCu.Log('arg ' + LogCu.Describe(RecRef, PageRec.FilterGroup()) + ' name=' + PageRec.Name);
        end else begin
            RecRef.GetTable(R);
            LogCu.Log('arg ' + LogCu.Describe(RecRef, R.FilterGroup()) + ' name=' + R.Name);
        end;
        ClearLastError();
    end;
}

codeunit 99562 "GPC Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        LogCu: Codeunit "GPC Log";

    local procedure Initialize(Steps: Text; Extra: Text)
    var
        Z: Record "GPC Z";
        M: Record "GPC M";
    begin
        Z.Consistent(true);
        M.Consistent(true);
        Z.DeleteAll();
        M.DeleteAll();
        InsertRow(Z."Doc Kind"::Quote, '1', 'q1', Z.Kind::Quote);
        InsertRow(Z."Doc Kind"::Quote, '2', 'q2', Z.Kind::Quote);
        InsertRow(Z."Doc Kind"::Order, '1', 'o1', Z.Kind::Quote);
        InsertRow(Z."Doc Kind"::Order, '2', 'o2', Z.Kind::Order);
        InsertRow(Z."Doc Kind"::Invoice, '1', 'i1', Z.Kind::Order);
        InsertRow(Z."Doc Kind"::Invoice, '2', 'i2', Z.Kind::Quote);
        InsertRow(Z."Doc Kind"::"Credit Memo", '1', 'c1', Z.Kind::Quote);
        InsertRow(Z."Doc Kind"::"Credit Memo", '2', 'c2', Z.Kind::Quote);
        M.Init();
        M.Id := 1;
        M.Name := 'm1';
        M.Insert();
        Commit();
        ClearLastError();
        LogCu.Set(Steps, Extra);
    end;

    local procedure InsertRow(DocKind: Enum "GPC Kind"; No: Code[20]; NewName: Text[30]; NewKind: Enum "GPC Kind")
    var
        Z: Record "GPC Z";
    begin
        Z.Init();
        Z."Doc Kind" := DocKind;
        Z."No." := No;
        Z.Name := NewName;
        Z.Kind := NewKind;
        Z.Insert();
    end;

    local procedure Rows(): Text
    var
        Z: Record "GPC Z";
        M: Record "GPC M";
        ZNames: Text;
        MNames: Text;
    begin
        if Z.FindSet() then
            repeat
                ZNames += Z.Name + ';';
            until Z.Next() = 0;
        if M.FindSet() then
            repeat
                MNames += M.Name + ';';
            until M.Next() = 0;
        exit('z=[' + ZNames + '] m=[' + MNames + ']');
    end;

    // The rows a commit made durable: an unrelated error undoes every write since the last commit.
    local procedure DurableRows(): Text
    begin
        asserterror Error('unrelated');
        ClearLastError();
        exit(Rows());
    end;

    // Opens the page of the shape, invokes the action on the first row it shows, then raises Error('END').
    local procedure PageThenEnd(Shape: Text)
    var
        Quotes: TestPage "GPC Quotes";
        Orders: TestPage "GPC Orders";
        Invoices: TestPage "GPC Invoices";
        CreditMemos: TestPage "GPC Credit Memos";
        KindOrders: TestPage "GPC Kind Orders";
        OrderCard: TestPage "GPC Order Card";
        InvoiceCard: TestPage "GPC Invoice Card";
    begin
        case Shape of
            'Quote':
                begin
                    Quotes.OpenView();
                    Quotes.First();
                    Quotes.RunGuarded.Invoke();
                    Quotes.Close();
                end;
            'Order', 'OrderSetRange', 'OrderSetFilter', 'OrderDirect':
                begin
                    Orders.OpenView();
                    Orders.First();
                    Orders.RunGuarded.Invoke();
                    Orders.Close();
                end;
            'OrderTpFilter':
                begin
                    Orders.OpenView();
                    Orders.Filter.SetFilter("No.", '1');
                    Orders.First();
                    Orders.RunGuarded.Invoke();
                    Orders.Close();
                end;
            'Invoice':
                begin
                    Invoices.OpenView();
                    Invoices.First();
                    Invoices.RunGuarded.Invoke();
                    Invoices.Close();
                end;
            'CreditMemo':
                begin
                    CreditMemos.OpenView();
                    CreditMemos.First();
                    CreditMemos.RunGuarded.Invoke();
                    CreditMemos.Close();
                end;
            'OrderCard', 'OrderCardDirect':
                begin
                    OrderCard.OpenView();
                    OrderCard.First();
                    OrderCard.RunGuarded.Invoke();
                    OrderCard.Close();
                end;
            'InvoiceCard':
                begin
                    InvoiceCard.OpenView();
                    InvoiceCard.First();
                    InvoiceCard.RunGuarded.Invoke();
                    InvoiceCard.Close();
                end;
            'KindOrder':
                begin
                    KindOrders.OpenView();
                    KindOrders.First();
                    KindOrders.RunGuarded.Invoke();
                    KindOrders.Close();
                end;
        end;
        Error('END');
    end;

    local procedure CommitThenEnd()
    begin
        Commit();
        Error('END');
    end;

    local procedure PageCell(var Observer: Codeunit "Probe Observer"; Shape: Text; Steps: Text)
    var
        Extra: Text;
    begin
        case Shape of
            'OrderSetRange':
                Extra := 'setrange';
            'OrderSetFilter':
                Extra := 'setfilter';
            'OrderDirect', 'OrderCardDirect':
                Extra := 'direct';
        end;
        Initialize(Steps, Extra);
        asserterror PageThenEnd(Shape);
        Observer.ObserveError('run end');
        Observer.Observe('log', LogCu.GetLog());
        asserterror CommitThenEnd();
        Observer.ObserveError('commit');
        Observer.Observe('durable rows', DurableRows());
    end;

    [Test]
    procedure Page_Quote_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Quote', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Quote_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Quote', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Quote_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Quote', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Quote] g4=[] g6=[] count=2 name=q1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Order_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Order', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Order_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Order', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Order_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Order', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Invoice_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Invoice', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Invoice_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Invoice', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_Invoice_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'Invoice', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_CreditMemo_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'CreditMemo', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_CreditMemo_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'CreditMemo', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_CreditMemo_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'CreditMemo', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Credit Memo] g4=[] g6=[] count=2 name=c1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_KindOrder_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'KindOrder', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;inner fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_KindOrder_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'KindOrder', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;inner fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_KindOrder_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'KindOrder', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;inner fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Kind: Order] g4=[] g6=[] count=2 name=o2;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderTpFilter_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderTpFilter', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderTpFilter_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderTpFilter', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderTpFilter_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderTpFilter', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderSetRange_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderSetRange', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderSetRange_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderSetRange', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderSetRange_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderSetRange', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderSetFilter_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderSetFilter', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderSetFilter_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderSetFilter', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderSetFilter_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderSetFilter', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[No.: 1] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=1 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderCard_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderCard', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderCard_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderCard', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderCard_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderCard', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_InvoiceCard_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'InvoiceCard', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_InvoiceCard_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'InvoiceCard', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_InvoiceCard_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'InvoiceCard', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Invoice] g4=[] g6=[] count=2 name=i1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderDirect_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderDirect', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderDirect_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderDirect', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderDirect_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderDirect', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderCardDirect_Fails()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderCardDirect', 'mark,err');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=inner failed;code=Dialog;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderCardDirect_Ends()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderCardDirect', 'mark');
        Observer.Expect('run end|text', 'The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.');
        Observer.Expect('run end|code', 'TestWrapped:CSide');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;

    [Test]
    procedure Page_OrderCardDirect_PlainCommit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        PageCell(Observer, 'OrderCardDirect', 'mark,commit');
        Observer.Expect('run end|text', 'END');
        Observer.Expect('run end|code', 'Dialog');
        Observer.Expect('log', 'page fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;inner fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;run=No;text=The transaction cannot be completed because it will cause inconsistencies in the GPC M table. Check where and how the CONSISTENT function is used in the transaction to find the reason for the error.\\Contact your system administrator.\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This prevents data from being updated incorrectly.;code=DB:TableInConsistent;arg fg=0 g0=[] g1=[] g2=[Doc Kind: Order] g4=[] g6=[] count=2 name=o1;');
        Observer.Expect('commit|text', 'END');
        Observer.Expect('commit|code', 'Dialog');
        Observer.Expect('durable rows', 'z=[q1;q2;o1;o2;i1;i2;c1;c2;] m=[m1;]');
        Observer.Verify();
    end;
}
