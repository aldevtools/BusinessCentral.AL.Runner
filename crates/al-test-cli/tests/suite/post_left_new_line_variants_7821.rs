//! al-runner issue 7821, the reporter's variants of the journal whose Post left another primary key in Rec on its new-row line:
//! the page with one more field, two and three rows, one more row in another batch, and subscribers to the Post action. The
//! objects and tests are corpus pull request 1384 (testpage/R7821JournalPostVariants.al, merged, not yet in the pinned corpus),
//! unchanged, with the corpus's Probe Observer; all 11 tests pass on BC 27.5, 28.5 and 29.0. The cells run through the assumption
//! `post-left-other-key-new-line-variants` until the file is pinned (crates/al-vm/src/pages/post_left_new_line.rs). Each
//! neighbor below changes one fact of a measured cell and keeps main's refusal.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

/// The corpus's Probe Observer (codeunits 69160 and 69163).
const OBSERVER: &str = include_str!("../../../../corpus/tests/al-language/_fixtures/ProbeObserver.Codeunit.al");

/// Each test of codeunit `codeunit`, its outcome and the assumptions it went through.
fn run(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let l = compile(&format!("{src}\n{OBSERVER}"));
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.name == codeunit)
        .expect("the codeunit");
    let mut hits = Vec::new();
    let outcomes = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    outcomes
        .into_iter()
        .map(|(name, outcome)| {
            let marks = hits
                .iter()
                .find(|(n, _): &&(String, Vec<Assumption>)| *n == name)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (name, outcome, marks)
        })
        .collect()
}

const TESTS: &str = "TP R7821V Tests";

/// The outcome of test `test` of `src`.
fn outcome(src: &str, test: &str) -> (Outcome, Vec<Assumption>) {
    run(src, TESTS)
        .into_iter()
        .find(|(n, ..)| n == test)
        .map(|(_, o, m)| (o, m))
        .unwrap_or_else(|| panic!("no test {test}"))
}

/// `src` with each `(from, to)` of `edits` applied once; each `from` must be there.
fn variant(edits: &[(&str, &str)]) -> String {
    edits.iter().fold(PROBE.to_string(), |s, (from, to)| {
        assert!(s.contains(from), "not in the probe: {from}");
        s.replacen(from, to, 1)
    })
}

#[test]
fn the_probe_passes_as_a_flagged_pass() {
    let all = run(PROBE, TESTS);
    assert_eq!(all.len(), 11);
    for (name, o, marks) in all {
        assert!(matches!(o, Outcome::Pass), "{name}: {o:?}");
        assert!(
            marks.contains(&Assumption::PostLeftOtherKeyNewLineVariants),
            "{name}: {marks:?}"
        );
        assert!(
            !marks.contains(&Assumption::PostLeftOtherKeyOnNewLine),
            "{name}: {marks:?}"
        );
    }
}

/// The refusal of main for an action that left another key in Rec outside the pinned cells.
fn refused_at_post(o: &Outcome) -> bool {
    matches!(o, Outcome::NotRun(r) if r.contains("another primary key"))
}

/// One fact of a measured cell changed: the Post keeps main's refusal and takes no assumption.
#[test]
fn neighbors_of_the_cells_keep_mains_refusal() {
    let run_wks = "        RunWks(Observer, 1, '', true, 0);";
    let open_edit = |from: &'static str, to: &'static str| (from, to);
    let cases: Vec<(&str, String, &str)> = vec![
        (
            "two rows, OpenEdit",
            variant(&[open_edit(
                "        RunWks(Observer, 2, '', true, 0);",
                "        RunWks(Observer, 2, '', false, 0);",
            )]),
            "TPV7821_TwoLines_PageRun",
        ),
        (
            "four rows",
            variant(&[(run_wks, "        RunWks(Observer, 4, '', true, 0);")]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "other row in A1, OpenEdit",
            variant(&[(
                "        RunWks(Observer, 1, 'A1', true, 0);",
                "        RunWks(Observer, 1, 'A1', false, 0);",
            )]),
            "TPV7821_OtherBatchBefore_OneLine_PageRun",
        ),
        (
            "logging subscribers, OpenEdit",
            variant(&[(
                "        RunWks(Observer, 1, '', true, 2);",
                "        RunWks(Observer, 1, '', false, 2);",
            )]),
            "TPV7821_ActionSubsLogged_OneLine_PageRun",
        ),
        (
            "other row in B3",
            variant(&[(
                "        RunWks(Observer, 1, 'B2', true, 0);",
                "        RunWks(Observer, 1, 'B3', true, 0);",
            )]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "other row on line 20000",
            variant(&[(
                "            Line.\"Line No.\" := 10000;",
                "            Line.\"Line No.\" := 20000;",
            )]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "other row with Quantity 5",
            variant(&[("            Line.Quantity := 100;", "            Line.Quantity := 5;")]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "two other rows",
            variant(&[(
                "            Line.Quantity := 100;\n            Line.Insert();",
                "            Line.Quantity := 100;\n            Line.Insert();\n            Line.\"Line No.\" := 20000;\n            \
                 Line.Insert();",
            )]),
            "TPV7821_OtherBatchAfter_OneLine_PageRun",
        ),
        (
            "both subscriber codeunits bound",
            variant(&[(
                "            1:\n                BindSubscription(Subs);",
                "            1:\n                begin\n                    BindSubscription(Subs);\n                    \
                 BindSubscription(LogSubs);\n                end;",
            )]),
            "TPV7821_ActionSubs_OneLine_PageRun",
        ),
        (
            "a counting subscriber that counts two",
            variant(&[("        Runs += 1;\n    end;", "        Runs += 2;\n    end;")]),
            "TPV7821_ActionSubs_OneLine_PageRun",
        ),
        (
            "counting subscribers bound statically",
            variant(&[(
                "codeunit 67208 \"TP R7821V Subs\"\n{\n    EventSubscriberInstance = Manual;\n",
                "codeunit 67208 \"TP R7821V Subs\"\n{\n",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "a third subscriber, to OnAfterGetCurrRecordEvent",
            variant(&[(
                "    procedure Count(): Integer",
                "    [EventSubscriber(ObjectType::Page, Page::\"TP R7821V Wks\", 'OnAfterGetCurrRecordEvent', '', false, false)]\n    \
                 local procedure OnCurr(var Rec: Record \"TP R7821V Line\")\n    begin\n    end;\n\n    procedure Count(): Integer",
            )]),
            "TPV7821_Base_OneLine_PageRun",
        ),
        (
            "the Batch Name page with two rows",
            variant(&[(
                "        InsertLines(Line, 1, '');\n        if WithPageRun then begin\n            Worksheet.Trap();\n            \
                 Page.Run(Page::\"TP R7821V Field Wks\", Line);",
                "        InsertLines(Line, 2, '');\n        if WithPageRun then begin\n            Worksheet.Trap();\n            \
                 Page.Run(Page::\"TP R7821V Field Wks\", Line);",
            )]),
            "TPV7821_ExtraField_OneLine_PageRun",
        ),
        (
            "the Batch Name page with a log variable of another name in one trigger",
            variant(&[(
                "    trigger OnNewRecord(BelowxRec: Boolean)\n    var\n        Log: Codeunit \"TP R7821V Log\";\n    begin\n        \
                 Log.Add('N:' + Format(Rec.\"Line No.\"));\n    end;\n\n    trigger OnAfterGetCurrRecord()\n    var\n        Log: \
                 Codeunit \"TP R7821V Log\";\n    begin\n        Log.Add('C:' + Format(Rec.\"Line No.\"));\n    end;\n}\n\ncodeunit",
                "    trigger OnNewRecord(BelowxRec: Boolean)\n    var\n        Log: Codeunit \"TP R7821V Log\";\n    begin\n        \
                 Log.Add('N:' + Format(Rec.\"Line No.\"));\n    end;\n\n    trigger OnAfterGetCurrRecord()\n    var\n        Other: \
                 Codeunit \"TP R7821V Log\";\n    begin\n        Other.Add('C:' + Format(Rec.\"Line No.\"));\n    end;\n}\n\ncodeunit",
            )]),
            "TPV7821_ExtraField_OneLine_PageRun",
        ),
    ];
    for (what, src, test) in cases {
        let (o, marks) = outcome(&src, test);
        assert!(refused_at_post(&o), "{what}: {o:?}");
        assert!(
            !marks.contains(&Assumption::PostLeftOtherKeyNewLineVariants),
            "{what}: {marks:?}"
        );
    }
}

/// The repro's cell (no subscriber of the page, one row, no other row) keeps main's end: after the Post only Close, so the
/// probe's read of the shown line is refused.
#[test]
fn the_repro_cell_keeps_only_close_after_the_post() {
    let start = PROBE.find("// V4: counts the runs").expect("the subscribers");
    let end = PROBE.find("codeunit 67210").expect("the tests");
    let src = format!(
        "{}codeunit 67208 \"TP R7821V Subs\" {{ procedure Count(): Integer begin end; }}\ncodeunit 67209 \"TP R7821V Log Subs\" {{ }}\n{}",
        &PROBE[..start],
        &PROBE[end..]
    );
    let (o, marks) = outcome(&src, "TPV7821_Base_OneLine_PageRun");
    assert!(
        matches!(&o, Outcome::NotRun(r) if r.contains("other than Close")),
        "{o:?}"
    );
    assert!(marks.contains(&Assumption::PostLeftOtherKeyOnNewLine), "{marks:?}");
}

/// testpage/R7821JournalPostVariants.al of corpus pull request 1384 (merge commit 2a2e2e4e), unchanged.
const PROBE: &str = r##"
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/page/page-update-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); own objects 67204-67210 (TP R7821V *)
// local object: the page's Post action is what the test checks
// Probe: from al-runner issue 7821 (follow-up of 6027, 6292, 7197; the repro itself is testpage/R7821JournalPostNewLine.al). The reporter
//   changed one fact of the repro at a time and ran each variant on BC 29.0. This file has the same variants with own objects. The base
//   case: a Worksheet with the new-row line (AutoSplitKey, DelayedInsert), filtered to batch B1 in FilterGroup(2) in OnOpenPage, with
//   OnAfterGetRecord, OnNewRecord and OnAfterGetCurrRecord logging the line number. Its Post action copies Rec, deletes every line of
//   the batch and, when Find('=><') answers false, resets the copy, sets the batch filter in FilterGroup(2), assigns "Line No." := 1 (a
//   key no row has), copies it back into Rec and calls CurrPage.Update(false), as codeunit 241 "Item Jnl.-Post" leaves page 99000823
//   "Output Journal". The test holds one B1 line, opens the page with Page.Run on the filtered record on a trapped TestPage, invokes
//   Post and reads the page triggers that ran after the action, the line shown, the triggers of Close and the rows left.
//   Each variant changes exactly one fact of that base case:
//   - V1b: the page has one more field ("Batch Name") in the repeater.
//   - V2: two (or three) B1 lines before Post.
//   - V3: one more line in another batch (B2 after B1 in key order, or A1 before it), which Post does not delete.
//   - V4: subscribers to OnBeforeActionEvent and OnAfterActionEvent of Post, bound manually in the test (counting their runs, or
//     writing to the trigger log to show their order).
//   - Open: OpenEdit instead of Page.Run (for V1b, V3 and V4; the base case with OpenEdit is in R7821JournalPostNewLine.al).
//   The reporter saw on BC 29.0 for V1b, V3 and V4 the base case's `R:10000;N:0;C:0;N:0;C:0;N:0;C:0;`, for V2 five N:0;C:0 pairs.
// Result (measured on Windows BC 27.5, 28.5 and 29.0 containers, before the corpus CI run): the base case, V1b (both opens), V3 (B2 and A1,
//   both opens) and V4 (both opens) all give the base case's `R:10000;N:0;C:0;N:0;C:0;N:0;C:0;`, show line 0, run no trigger on Close and
//   leave no B1 line (V3 keeps its other-batch line). V1b shows batch B1. The counting subscribers ran twice; the logging ones run after
//   OnAfterGetRecord of the deleted line and before the blank line: `R:10000;BA;AA;N:0;C:0;...`. V2 with two lines gives five N:0;C:0
//   pairs on BC 27.5 and 29.0 and four on BC 28.5 (as the OpenEdit case of R7821JournalPostNewLine.al); with three lines five pairs on all.
// Not here: the reporter's V1a (the page under another id and name, same body: every page of this file already is one) and V5 (the real
//   Output Journal posting a production order: it writes to Microsoft tables, which the corpus does not do).
// BC versions: 27+ (tested by the corpus CI matrix)

table 67204 "TP R7821V Line"
{
    fields
    {
        field(1; "Batch Name"; Code[10]) { }
        field(2; "Line No."; Integer) { }
        field(3; Quantity; Decimal) { }
    }
    keys
    {
        key(PK; "Batch Name", "Line No.") { Clustered = true; }
    }
}

page 67206 "TP R7821V Wks"
{
    PageType = Worksheet;
    SourceTable = "TP R7821V Line";
    ApplicationArea = All;
    UsageCategory = None;
    AutoSplitKey = true;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Quantity; Rec.Quantity) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Post)
            {
                ApplicationArea = All;
                Caption = 'Post';

                trigger OnAction()
                var
                    Posted: Record "TP R7821V Line";
                begin
                    Posted.Copy(Rec);
                    Rec.DeleteAll();
                    if not Posted.Find('=><') then begin
                        Posted.Reset();
                        Posted.FilterGroup(2);
                        Posted.SetRange("Batch Name", 'B1');
                        Posted.FilterGroup(0);
                        Posted."Line No." := 1;
                    end;
                    Rec.Copy(Posted);
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange("Batch Name", 'B1');
        Rec.FilterGroup(0);
    end;

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('R:' + Format(Rec."Line No."));
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('N:' + Format(Rec."Line No."));
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('C:' + Format(Rec."Line No."));
    end;
}

// V1b: the page above with one more field, "Batch Name", in the repeater; nothing else changed.
page 67207 "TP R7821V Field Wks"
{
    PageType = Worksheet;
    SourceTable = "TP R7821V Line";
    ApplicationArea = All;
    UsageCategory = None;
    AutoSplitKey = true;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Batch Name"; Rec."Batch Name") { ApplicationArea = All; }
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Quantity; Rec.Quantity) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Post)
            {
                ApplicationArea = All;
                Caption = 'Post';

                trigger OnAction()
                var
                    Posted: Record "TP R7821V Line";
                begin
                    Posted.Copy(Rec);
                    Rec.DeleteAll();
                    if not Posted.Find('=><') then begin
                        Posted.Reset();
                        Posted.FilterGroup(2);
                        Posted.SetRange("Batch Name", 'B1');
                        Posted.FilterGroup(0);
                        Posted."Line No." := 1;
                    end;
                    Rec.Copy(Posted);
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange("Batch Name", 'B1');
        Rec.FilterGroup(0);
    end;

    trigger OnAfterGetRecord()
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('R:' + Format(Rec."Line No."));
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('N:' + Format(Rec."Line No."));
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('C:' + Format(Rec."Line No."));
    end;
}

codeunit 67205 "TP R7821V Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;
}

// V4: counts the runs of its subscribers to the Post action; bound manually by the test (the reporter's shape).
codeunit 67208 "TP R7821V Subs"
{
    EventSubscriberInstance = Manual;

    var
        Runs: Integer;

    [EventSubscriber(ObjectType::Page, Page::"TP R7821V Wks", 'OnBeforeActionEvent', 'Post', false, false)]
    local procedure OnBeforePost()
    begin
        Runs += 1;
    end;

    [EventSubscriber(ObjectType::Page, Page::"TP R7821V Wks", 'OnAfterActionEvent', 'Post', false, false)]
    local procedure OnAfterPost()
    begin
        Runs += 1;
    end;

    procedure Count(): Integer
    begin
        exit(Runs);
    end;
}

// V4, logged: the same subscribers write BA (before action) and AA (after action) into the trigger log, to show where they run.
codeunit 67209 "TP R7821V Log Subs"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Page, Page::"TP R7821V Wks", 'OnBeforeActionEvent', 'Post', false, false)]
    local procedure OnBeforePost()
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('BA');
    end;

    [EventSubscriber(ObjectType::Page, Page::"TP R7821V Wks", 'OnAfterActionEvent', 'Post', false, false)]
    local procedure OnAfterPost()
    var
        Log: Codeunit "TP R7821V Log";
    begin
        Log.Add('AA');
    end;
}

codeunit 67210 "TP R7821V Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure InsertLines(var Line: Record "TP R7821V Line"; Rows: Integer; OtherBatch: Code[10])
    var
        i: Integer;
    begin
        Line.Reset();
        Line.DeleteAll();
        for i := 1 to Rows do begin
            Line.Init();
            Line."Batch Name" := 'B1';
            Line."Line No." := i * 10000;
            Line.Quantity := 300;
            Line.Insert();
        end;
        if OtherBatch <> '' then begin
            Line.Init();
            Line."Batch Name" := OtherBatch;
            Line."Line No." := 10000;
            Line.Quantity := 100;
            Line.Insert();
        end;
        Line.SetRange("Batch Name", 'B1');
        Line.FindFirst();
        Commit();
    end;

    local procedure ObserveRows(var Observer: Codeunit "Probe Observer"; var Line: Record "TP R7821V Line")
    var
        Other: Record "TP R7821V Line";
    begin
        Observer.Observe('lines left', Format(Line.Count()));
        Other.SetFilter("Batch Name", '<>%1', 'B1');
        Observer.Observe('other batch lines left', Format(Other.Count()));
    end;

    // Subscribers: 0 none, 1 TP R7821V Subs (counting), 2 TP R7821V Log Subs (logging).
    local procedure RunWks(var Observer: Codeunit "Probe Observer"; Rows: Integer; OtherBatch: Code[10]; WithPageRun: Boolean; Subscribers: Integer)
    var
        Line: Record "TP R7821V Line";
        Log: Codeunit "TP R7821V Log";
        Subs: Codeunit "TP R7821V Subs";
        LogSubs: Codeunit "TP R7821V Log Subs";
        Worksheet: TestPage "TP R7821V Wks";
    begin
        InsertLines(Line, Rows, OtherBatch);
        case Subscribers of
            1:
                BindSubscription(Subs);
            2:
                BindSubscription(LogSubs);
        end;
        if WithPageRun then begin
            Worksheet.Trap();
            Page.Run(Page::"TP R7821V Wks", Line);
        end else
            Worksheet.OpenEdit();
        Log.Take();
        Worksheet.Post.Invoke();
        Observer.Observe('after post', Log.Take());
        Observer.Observe('shown line', Worksheet."Line No.".Value());
        Worksheet.Close();
        Observer.Observe('after close', Log.Take());
        case Subscribers of
            1:
                begin
                    UnbindSubscription(Subs);
                    Observer.Observe('subscriber runs', Format(Subs.Count()));
                end;
            2:
                UnbindSubscription(LogSubs);
        end;
        ObserveRows(Observer, Line);
    end;

    local procedure RunFieldWks(var Observer: Codeunit "Probe Observer"; WithPageRun: Boolean)
    var
        Line: Record "TP R7821V Line";
        Log: Codeunit "TP R7821V Log";
        Worksheet: TestPage "TP R7821V Field Wks";
    begin
        InsertLines(Line, 1, '');
        if WithPageRun then begin
            Worksheet.Trap();
            Page.Run(Page::"TP R7821V Field Wks", Line);
        end else
            Worksheet.OpenEdit();
        Log.Take();
        Worksheet.Post.Invoke();
        Observer.Observe('after post', Log.Take());
        Observer.Observe('shown line', Worksheet."Line No.".Value());
        Observer.Observe('shown batch', Worksheet."Batch Name".Value());
        Worksheet.Close();
        Observer.Observe('after close', Log.Take());
        ObserveRows(Observer, Line);
    end;

    [Test]
    procedure TPV7821_Base_OneLine_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 1, '', true, 0);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_ExtraField_OneLine_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunFieldWks(Observer, true);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('shown batch', 'B1');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_ExtraField_OneLine_OpenEdit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunFieldWks(Observer, false);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('shown batch', 'B1');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_TwoLines_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 2, '', true, 0);
#if BC28PLUS and not BC29PLUS
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;N:0;C:0;');
#else
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;N:0;C:0;N:0;C:0;');
#endif
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_ThreeLines_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 3, '', true, 0);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_OtherBatchAfter_OneLine_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 1, 'B2', true, 0);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '1');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_OtherBatchBefore_OneLine_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 1, 'A1', true, 0);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '1');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_OtherBatchAfter_OneLine_OpenEdit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 1, 'B2', false, 0);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '1');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_ActionSubs_OneLine_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 1, '', true, 1);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('subscriber runs', '2');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_ActionSubsLogged_OneLine_PageRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 1, '', true, 2);
        Observer.Expect('after post', 'R:10000;BA;AA;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;

    [Test]
    procedure TPV7821_ActionSubs_OneLine_OpenEdit()
    var
        Observer: Codeunit "Probe Observer";
    begin
        RunWks(Observer, 1, '', false, 1);
        Observer.Expect('after post', 'R:10000;N:0;C:0;N:0;C:0;N:0;C:0;');
        Observer.Expect('shown line', '0');
        Observer.Expect('after close', '');
        Observer.Expect('subscriber runs', '2');
        Observer.Expect('lines left', '0');
        Observer.Expect('other batch lines left', '0');
        Observer.Verify();
    end;
}
"##;
