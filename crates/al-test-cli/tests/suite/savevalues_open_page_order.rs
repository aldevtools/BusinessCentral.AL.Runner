//! A SaveValues request page restores its saved values before its OnOpenPage, so OnOpenPage overwrites an Editable = false
//! global it recomputes from a caller's procedure value, and a caller's procedure value wins over a saved one without an
//! OnOpenPage (al-runner issue 6924). Pinned on BC 27.5, 28.5 and 29.0 by corpus tests/al-language/reports/request-page/
//! SaveValuesOpenPageOrder.al (corpus PR 1162; the objects below are that file, only the observer codeunit is local).

use al_test_cli::report::Outcome;

const SRC: &str = r####"
table 96100 "LT SVO Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Name; Text[30]) { }
        field(3; Qty; Integer) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 96100 "LT SVO Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Entries;
        Entries := '';
        exit(Result);
    end;
}

// OnOpenPage recomputes the Editable = false global from the value the caller passed.
report 96100 "LT SVO With OpenPage"
{
    ProcessingOnly = true;
    UsageCategory = None;

    requestpage
    {
        SaveValues = true;

        layout
        {
            area(Content)
            {
                field(Loc; LocTxt)
                {
                    ApplicationArea = All;
                    Editable = false;
                }
                field(Other; OtherTxt)
                {
                    ApplicationArea = All;
                }
            }
        }

        trigger OnOpenPage()
        var
            Log: Codeunit "LT SVO Log";
        begin
            Log.Add('open.before loc=[' + LocTxt + '] other=[' + OtherTxt + ']');
            LocTxt := 'computed(' + InputTxt + ')';
            Log.Add('open.after loc=[' + LocTxt + ']');
        end;
    }

    procedure SetX(NewInput: Text[30])
    begin
        InputTxt := NewInput;
    end;

    trigger OnPreReport()
    var
        Log: Codeunit "LT SVO Log";
    begin
        Log.Add('pre loc=[' + LocTxt + '] other=[' + OtherTxt + '] input=[' + InputTxt + ']');
    end;

    var
        LocTxt: Text[60];
        OtherTxt: Text[30];
        InputTxt: Text[30];
}

// No OnOpenPage: the caller's procedure sets the Editable = false global directly.
report 96101 "LT SVO Without OpenPage"
{
    ProcessingOnly = true;
    UsageCategory = None;

    requestpage
    {
        SaveValues = true;

        layout
        {
            area(Content)
            {
                field(Loc; LocTxt)
                {
                    ApplicationArea = All;
                    Editable = false;
                }
                field(Other; OtherTxt)
                {
                    ApplicationArea = All;
                }
            }
        }
    }

    procedure SetLoc(NewLoc: Text[60])
    begin
        LocTxt := NewLoc;
    end;

    trigger OnPreReport()
    var
        Log: Codeunit "LT SVO Log";
    begin
        Log.Add('pre loc=[' + LocTxt + '] other=[' + OtherTxt + ']');
    end;

    var
        LocTxt: Text[60];
        OtherTxt: Text[30];
}

// Data items with a DataItemTableView, with and without request filter fields.
report 96102 "LT SVO View Report"
{
    ProcessingOnly = true;
    UsageCategory = None;

    dataset
    {
        dataitem(DiView; "LT SVO Row")
        {
            DataItemTableView = where(Qty = filter(> 2));

            trigger OnPreDataItem()
            var
                Log: Codeunit "LT SVO Log";
            begin
                Log.Add('preDi DiView filters=[' + DiView.GetFilters() + '] view=[' + DiView.GetView() + '] view(false)=[' + DiView.GetView(false) + ']');
            end;
        }
        dataitem(DiBoth; "LT SVO Row")
        {
            DataItemTableView = sorting(Name) where(Qty = filter(< 10));
            RequestFilterFields = Code;

            trigger OnPreDataItem()
            var
                Log: Codeunit "LT SVO Log";
            begin
                Log.Add('preDi DiBoth filters=[' + DiBoth.GetFilters() + '] view=[' + DiBoth.GetView() + '] view(false)=[' + DiBoth.GetView(false) + ']');
            end;
        }
        dataitem(DiPlain; "LT SVO Row")
        {
            RequestFilterFields = Code;

            trigger OnPreDataItem()
            var
                Log: Codeunit "LT SVO Log";
            begin
                Log.Add('preDi DiPlain filters=[' + DiPlain.GetFilters() + '] view=[' + DiPlain.GetView() + '] view(false)=[' + DiPlain.GetView(false) + ']');
            end;
        }
    }

    requestpage
    {
        layout
        {
            area(Content)
            {
                field(Marker; MarkerTxt)
                {
                    ApplicationArea = All;
                }
            }
        }
    }

    trigger OnPreReport()
    var
        Log: Codeunit "LT SVO Log";
    begin
        Log.Add('pre DiView filters=[' + DiView.GetFilters() + '] view=[' + DiView.GetView() + '] view(false)=[' + DiView.GetView(false) + ']');
        Log.Add('pre DiBoth filters=[' + DiBoth.GetFilters() + '] view=[' + DiBoth.GetView() + '] view(false)=[' + DiBoth.GetView(false) + ']');
        Log.Add('pre DiPlain filters=[' + DiPlain.GetFilters() + '] view=[' + DiPlain.GetView() + '] view(false)=[' + DiPlain.GetView(false) + ']');
    end;

    var
        MarkerTxt: Text[10];
}

codeunit 96199 "LT SVO Observer"
{
    SingleInstance = true;
    var Seen: Dictionary of [Text, Text]; Want: Dictionary of [Text, Text];
    procedure Observe(K: Text; V: Text) begin Seen.Set(K, V); end;
    procedure Expect(K: Text; V: Text) begin Want.Set(K, V); end;
    procedure Verify()
    var K: Text;
    begin
        foreach K in Want.Keys() do
            if Seen.Get(K) <> Want.Get(K) then Error('%1: got [%2] want [%3]', K, Seen.Get(K), Want.Get(K));
        Clear(Seen);
        Clear(Want);
    end;
}

codeunit 96101 "LT SVO Tests"
{
    Subtype = Test;

    var
        HandlerRun: Integer;
        UserFilterCode: Text;

    local procedure TakeLog(): Text
    var
        Log: Codeunit "LT SVO Log";
    begin
        exit(Log.Take());
    end;

    local procedure RunWithOpenPage(Input: Text[30]; UseInput: Boolean)
    var
        Rep: Report "LT SVO With OpenPage";
    begin
        if UseInput then
            Rep.SetX(Input);
        Rep.RunModal();
    end;

    local procedure RunWithoutOpenPage(Loc: Text[60]; UseLoc: Boolean)
    var
        Rep: Report "LT SVO Without OpenPage";
    begin
        if UseLoc then
            Rep.SetLoc(Loc);
        Rep.RunModal();
    end;

    [Test]
    [HandlerFunctions('OpenPageHandler')]
    procedure SaveValues_OpenPageRecompute_SecondRunOrder()
    var
        Observer: Codeunit "LT SVO Observer";
    begin
        HandlerRun := 0;
        TakeLog();
        RunWithOpenPage('A', true);
        Observer.Observe('run1', TakeLog());
        RunWithOpenPage('B', true);
        Observer.Observe('run2', TakeLog());
        RunWithOpenPage('', false);
        Observer.Observe('run3 no SetX', TakeLog());
        Observer.Expect('run1', 'open.before loc=[] other=[];open.after loc=[computed(A)];handler loc=[computed(A)] other=[];pre loc=[computed(A)] other=[o1] input=[A];');
        Observer.Expect('run2', 'open.before loc=[computed(A)] other=[o1];open.after loc=[computed(B)];handler loc=[computed(B)] other=[o1];pre loc=[computed(B)] other=[o2] input=[B];');
        Observer.Expect('run3 no SetX', 'open.before loc=[computed(B)] other=[o2];open.after loc=[computed()];handler loc=[computed()] other=[o2];pre loc=[computed()] other=[o3] input=[];');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('NoOpenPageHandler')]
    procedure SaveValues_CallerProcedureOnly_EditableFalse_SecondRun()
    var
        Observer: Codeunit "LT SVO Observer";
    begin
        HandlerRun := 0;
        TakeLog();
        RunWithoutOpenPage('first', true);
        Observer.Observe('run1', TakeLog());
        RunWithoutOpenPage('second', true);
        Observer.Observe('run2', TakeLog());
        RunWithoutOpenPage('', false);
        Observer.Observe('run3 no SetLoc', TakeLog());
        Observer.Expect('run1', 'handler loc=[first] other=[];pre loc=[first] other=[o1];');
        Observer.Expect('run2', 'handler loc=[second] other=[o1];pre loc=[second] other=[o2];');
        Observer.Expect('run3 no SetLoc', 'handler loc=[second] other=[o2];pre loc=[second] other=[o3];');
        Observer.Verify();
    end;

    [Test]
    [HandlerFunctions('ViewHandler')]
    procedure DataItemTableView_GetFiltersAndGetView_InPreReport()
    var
        Observer: Codeunit "LT SVO Observer";
        Rep: Report "LT SVO View Report";
        Rep2: Report "LT SVO View Report";
    begin
        TakeLog();
        UserFilterCode := '';
        Rep.RunModal();
        Observer.Observe('no user filter', TakeLog());
        UserFilterCode := 'A*';
        Rep2.RunModal();
        Observer.Observe('user filter Code A*', TakeLog());
        Observer.Expect('no user filter', 'pre DiView filters=[] view=[VERSION(1) SORTING(Code)] view(false)=[VERSION(1) SORTING(Field1)];pre DiBoth filters=[] view=[VERSION(1) SORTING(Name)] view(false)=[VERSION(1) SORTING(Field2)];pre DiPlain filters=[] view=[VERSION(1) SORTING(Code)] view(false)=[VERSION(1) SORTING(Field1)];preDi DiView filters=[] view=[VERSION(1) SORTING(Code)] view(false)=[VERSION(1) SORTING(Field1)];preDi DiBoth filters=[] view=[VERSION(1) SORTING(Name)] view(false)=[VERSION(1) SORTING(Field2)];preDi DiPlain filters=[] view=[VERSION(1) SORTING(Code)] view(false)=[VERSION(1) SORTING(Field1)];');
        Observer.Expect('user filter Code A*', 'pre DiView filters=[] view=[VERSION(1) SORTING(Code)] view(false)=[VERSION(1) SORTING(Field1)];pre DiBoth filters=[Code: A*] view=[VERSION(1) SORTING(Name) WHERE(Code=FILTER(A*))] view(false)=[VERSION(1) SORTING(Field2) WHERE(Field1=1(A*))];pre DiPlain filters=[Code: A*] view=[VERSION(1) SORTING(Code) WHERE(Code=FILTER(A*))] view(false)=[VERSION(1) SORTING(Field1) WHERE(Field1=1(A*))];preDi DiView filters=[] view=[VERSION(1) SORTING(Code)] view(false)=[VERSION(1) SORTING(Field1)];preDi DiBoth filters=[Code: A*] view=[VERSION(1) SORTING(Name) WHERE(Code=FILTER(A*))] view(false)=[VERSION(1) SORTING(Field2) WHERE(Field1=1(A*))];preDi DiPlain filters=[Code: A*] view=[VERSION(1) SORTING(Code) WHERE(Code=FILTER(A*))] view(false)=[VERSION(1) SORTING(Field1) WHERE(Field1=1(A*))];');
        Observer.Verify();
    end;

    [RequestPageHandler]
    procedure OpenPageHandler(var RP: TestRequestPage "LT SVO With OpenPage")
    var
        Log: Codeunit "LT SVO Log";
    begin
        HandlerRun += 1;
        Log.Add('handler loc=[' + RP.Loc.Value() + '] other=[' + RP.Other.Value() + ']');
        RP.Other.SetValue('o' + Format(HandlerRun));
        RP.OK().Invoke();
    end;

    [RequestPageHandler]
    procedure NoOpenPageHandler(var RP: TestRequestPage "LT SVO Without OpenPage")
    var
        Log: Codeunit "LT SVO Log";
    begin
        HandlerRun += 1;
        Log.Add('handler loc=[' + RP.Loc.Value() + '] other=[' + RP.Other.Value() + ']');
        RP.Other.SetValue('o' + Format(HandlerRun));
        RP.OK().Invoke();
    end;

    [RequestPageHandler]
    procedure ViewHandler(var RP: TestRequestPage "LT SVO View Report")
    begin
        if UserFilterCode <> '' then begin
            RP.DiBoth.SetFilter(Code, UserFilterCode);
            RP.DiPlain.SetFilter(Code, UserFilterCode);
        end;
        RP.OK().Invoke();
    end;
}
"####;

fn outcome_of(name: &str) -> Outcome {
    al_testkit::outcomes_in(SRC, "LT SVO Tests")
        .into_iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}"))
        .1
}

#[test]
fn open_page_recompute_sees_the_value_of_the_run_it_belongs_to() {
    assert_eq!(outcome_of("SaveValues_OpenPageRecompute_SecondRunOrder"), Outcome::Pass);
}

#[test]
fn caller_procedure_value_wins_over_the_saved_one_without_open_page() {
    assert_eq!(
        outcome_of("SaveValues_CallerProcedureOnly_EditableFalse_SecondRun"),
        Outcome::Pass
    );
}

#[test]
fn data_item_table_view_where_is_not_part_of_get_filters_or_get_view() {
    assert_eq!(
        outcome_of("DataItemTableView_GetFiltersAndGetView_InPreReport"),
        Outcome::Pass
    );
}

/// The caller runs `SetX` only: the variable `SetLoc` writes was not set by the caller, so its saved value restores
/// (SaveValuesOpenPageOrder.al case 2, `run3 no SetLoc`, which shows the saved value when the caller did not set it).
#[test]
fn a_variable_a_procedure_the_caller_did_not_run_writes_restores_the_saved_value() {
    let src = r#"
report 97200 "SV Two Setters"
{
    ProcessingOnly = true;
    requestpage
    {
        SaveValues = true;
        layout { area(Content) { field(Loc; LocTxt) { ApplicationArea = All; } field(Inp; InputTxt) { ApplicationArea = All; } } }
    }
    procedure SetX(V: Text[30]) begin InputTxt := V; end;
    procedure SetLoc(V: Text[30]) begin LocTxt := V; end;
    trigger OnPreReport() begin Seen := 'loc=[' + LocTxt + '] inp=[' + InputTxt + ']'; end;
    var LocTxt: Text[30]; InputTxt: Text[30];
    procedure GetSeen(): Text begin exit(Seen); end;
    var Seen: Text;
}
codeunit 97201 "SV Two Setters Tests"
{
    Subtype = Test;
    var Mode: Integer;
    [Test] [HandlerFunctions('H')] procedure Run()
    var R1: Report "SV Two Setters"; R2: Report "SV Two Setters"; R3: Report "SV Two Setters";
    begin
        Mode := 1;
        R1.SetLoc('saved'); R1.SetX('one'); R1.RunModal();
        Mode := 2;
        R2.SetX('two'); R2.RunModal();
        Mode := 3;
        R3.RunModal();
    end;
    [RequestPageHandler] procedure H(var P: TestRequestPage "SV Two Setters")
    begin
        case Mode of
            2: if (P.Loc.Value <> 'saved') or (P.Inp.Value <> 'two') then Error('run2 loc=%1 inp=%2', P.Loc.Value, P.Inp.Value);
            3: if (P.Loc.Value <> 'saved') or (P.Inp.Value <> 'two') then Error('run3 loc=%1 inp=%2', P.Loc.Value, P.Inp.Value);
        end;
        P.OK().Invoke();
    end;
}"#;
    assert_eq!(al_testkit::outcomes_in(src, "SV Two Setters Tests")[0].1, Outcome::Pass);
}
