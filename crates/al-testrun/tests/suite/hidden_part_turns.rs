//! Parts a Visible expression hid (al-runner issue 7722): which turns to visible run as pinned, which are flagged and which
//! are refused, and the new-row shapes that keep main's behavior. Pins: testpage/parts/HiddenPartExtensionVariable.al:632 (a
//! Document host whose action body turns a hidden CardPart visible once: one extra read before the refresh),
//! testpage/parts/HiddenPartOutsideChain.al:319-363 (OpenNew, linked CardPart, Close of the new row). Every other shape is
//! refused or flagged, and these tests keep it so (the not-passing lists cannot tell a refusal from a failure).

use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};

const FIXTURES: &str = r#"
table 71000 "TF Doc"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; Name; Text[30]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}

codeunit 71001 "TF Log"
{
    SingleInstance = true;
    var
        Entries: Text;
        ShowPart: Boolean;
        HostArmed: Boolean;
        HostHides: Boolean;

    procedure Reset(Show: Boolean)
    begin
        Entries := '';
        ShowPart := Show;
        HostArmed := false;
        HostHides := false;
    end;

    procedure Show(): Boolean begin exit(ShowPart); end;
    procedure Arm(On: Boolean) begin HostArmed := On; end;
    procedure Armed(): Boolean begin exit(HostArmed); end;
    procedure ArmHide(On: Boolean) begin HostArmed := false; HostHides := On; end;
    procedure Hides(): Boolean begin exit(HostHides); end;

    procedure Add(Line: Text)
    begin
        if Entries <> '' then
            Entries += ' ';
        Entries += Line;
    end;

    procedure Take() Result: Text
    begin
        Result := Entries;
        Entries := '';
    end;

    procedure Expect(Expected: Text)
    var
        Actual: Text;
    begin
        Actual := Take();
        if Actual <> Expected then
            Error('expected <%1> got <%2>', Expected, Actual);
    end;
}

page 71002 "TF Fb"
{
    PageType = CardPart;
    SourceTable = "TF Doc";
    layout { area(Content) { field("No."; Rec."No.") { } } }
    var
        Log: Codeunit "TF Log";
    trigger OnAfterGetRecord() begin Log.Add('fb-agr(' + Rec."No." + ')'); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('fb-agcr(' + Rec."No." + ')'); end;
}

page 71003 "TF Doc Page"
{
    PageType = Document;
    SourceTable = "TF Doc";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(Name; Rec.Name)
            {
                trigger OnValidate()
                begin
                    if Rec.Name = 'show' then
                        FbVisible := true;
                end;
            }
        }
        area(FactBoxes)
        {
            part(Fb; "TF Fb") { SubPageLink = SystemId = field(SystemId); Visible = FbVisible; }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Plain) { trigger OnAction() begin Log.Add('act-plain'); end; }
            action(Flip) { trigger OnAction() begin FbVisible := not FbVisible; Log.Add('act-flip(' + Format(FbVisible) + ')'); end; }
            action(FlipUpd) { trigger OnAction() begin FbVisible := not FbVisible; CurrPage.Update(false); end; }
            action(FlipSave) { trigger OnAction() begin FbVisible := not FbVisible; Rec.Name := 'saved'; CurrPage.Update(true); end; }
            action(FlipWrite) { trigger OnAction() begin FbVisible := not FbVisible; Rec.Name := 'written'; end; }
            action(ArmHost) { trigger OnAction() begin Log.Arm(true); Log.Add('act-arm'); end; }
            action(DisarmHide) { trigger OnAction() begin Log.Arm(false); FbVisible := false; Log.Add('act-hide'); end; }
        }
    }
    var
        Log: Codeunit "TF Log";
        FbVisible: Boolean;
    trigger OnOpenPage() begin FbVisible := Log.Show(); end;
    trigger OnAfterGetCurrRecord()
    begin
        if Log.Armed() then
            FbVisible := true;
        if Log.Hides() then
            FbVisible := false;
        Log.Add('host-agcr(' + Rec."No." + ')');
    end;
}

page 71004 "TF Card Page"
{
    PageType = Card;
    SourceTable = "TF Doc";
    layout
    {
        area(Content) { field("No."; Rec."No.") { } }
        area(FactBoxes)
        {
            part(Fb; "TF Fb") { SubPageLink = "No." = field("No."); Visible = FbVisible; }
        }
    }
    actions
    {
        area(Processing)
        {
            action(Flip) { trigger OnAction() begin FbVisible := not FbVisible; Log.Add('act-flip(' + Format(FbVisible) + ')'); end; }
        }
    }
    var
        Log: Codeunit "TF Log";
        FbVisible: Boolean;
    trigger OnOpenPage() begin FbVisible := Log.Show(); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('host-agcr(' + Rec."No." + ')'); end;
}

page 71005 "TF Card Unlinked"
{
    PageType = Card;
    SourceTable = "TF Doc";
    layout
    {
        area(Content) { field("No."; Rec."No.") { } }
        area(FactBoxes) { part(Fb; "TF Fb") { Visible = FbVisible; } }
    }
    var
        FbVisible: Boolean;
}

page 71007 "TF Doc Unlinked"
{
    PageType = Document;
    SourceTable = "TF Doc";
    layout
    {
        area(Content) { field("No."; Rec."No.") { } }
        area(FactBoxes) { part(Fb; "TF Fb") { Visible = FbVisible; } }
    }
    actions
    {
        area(Processing)
        {
            action(Flip) { trigger OnAction() begin FbVisible := not FbVisible; end; }
        }
    }
    var
        FbVisible: Boolean;
}

codeunit 71006 "TF Seed"
{
    procedure Seed()
    var
        Doc: Record "TF Doc";
    begin
        Doc.DeleteAll();
        Doc."No." := 'H1';
        Doc.Insert();
        Doc."No." := 'H2';
        Doc.Insert();
    end;
}
"#;

/// One test codeunit per shape (id, test name, body), so a refusal in one leaves the others alone.
const TESTS: &[(u32, &str, &str)] = &[
    // HiddenPartExtensionVariable.al:632: the pinned turn.
    (
        71100,
        "DocFirstTurnByAction",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1'); Log.Take();
         Doc.Flip.Invoke(); Log.Expect('act-flip(Yes) fb-agr(H1) fb-agcr(H1) host-agcr(H1) fb-agr(H1) fb-agcr(H1)');",
    ),
    // A second turn by an action body.
    (
        71101,
        "DocSecondTurnByAction",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1');
         Doc.Flip.Invoke(); Doc.Flip.Invoke(); Doc.Flip.Invoke();",
    ),
    // A part shown at open, hidden and turned visible again by an action body.
    (
        71102,
        "DocShownAtOpenTurnedAgain",
        "Seed.Seed(); Log.Reset(true); Doc.OpenEdit(); Doc.GoToKey('H1'); Doc.Flip.Invoke(); Doc.Flip.Invoke();",
    ),
    // The host's OnAfterGetCurrRecord turns the part visible during the refresh: flagged.
    (
        71103,
        "DocTurnByHostTrigger",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1'); Log.Take();
         Doc.ArmHost.Invoke(); Log.Expect('act-arm host-agcr(H1) fb-agr(H1) fb-agcr(H1)');",
    ),
    // After a pinned turn and an action that hides the part, the host's trigger turns it visible again: flagged again.
    (
        71104,
        "DocHostTriggerTurnAfterActionTurns",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1');
         Doc.Flip.Invoke(); Doc.Flip.Invoke(); Log.Take();
         Doc.ArmHost.Invoke(); Log.Expect('act-arm host-agcr(H1) fb-agr(H1) fb-agcr(H1)');",
    ),
    // After a host-trigger turn, an action body that turns the part visible again is not a first turn: refused.
    (
        71105,
        "DocActionTurnAfterHostTriggerTurn",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1');
         Doc.ArmHost.Invoke(); Doc.DisarmHide.Invoke(); Doc.Flip.Invoke();",
    ),
    // A field's OnValidate turns the part visible: flagged (Microsoft's tests need it).
    (
        71106,
        "DocTurnByOnValidate",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1'); Doc.Name.SetValue('show'); Doc.Plain.Invoke();",
    ),
    // The pin is a Document host; a Card host is refused.
    (
        71107,
        "CardTurnByAction",
        "Seed.Seed(); Log.Reset(false); Card.OpenEdit(); Card.GoToKey('H1'); Card.Flip.Invoke();",
    ),
    // New() on an open host: the shown linked CardPart runs no trigger (as main; every pin is OpenNew).
    (
        71108,
        "CardNewOnOpenHostAsMain",
        "Seed.Seed(); Log.Reset(true); Card.OpenEdit(); Card.GoToKey('H1'); Log.Take();
         Card.New(); Log.Expect('host-agcr()');",
    ),
    // New() on an open host, key typed, Close: main's refusal stays (the quiet Close is pinned for OpenNew only).
    (
        71109,
        "CardNewOnOpenHostKeyClose",
        "Seed.Seed(); Log.Reset(true); Card.OpenEdit(); Card.GoToKey('H1'); Card.New(); Card.\"No.\".SetValue('H3'); Card.Close();",
    ),
    // OpenNew with a hidden CardPart without SubPageLink: every pin is a linked part, refused.
    (
        71110,
        "UnlinkedHiddenPartOpenNew",
        "Seed.Seed(); Unlinked.OpenNew(); Unlinked.Close();",
    ),
    // The host's OnAfterGetCurrRecord shows the part while the page opens, an action hides it, an action shows it again: the
    // part showed since the open, so this is not a first turn.
    (
        71111,
        "DocOpenTurnHideActionShow",
        "Seed.Seed(); Log.Reset(false); Log.Arm(true); Doc.OpenEdit(); Doc.DisarmHide.Invoke(); Doc.Flip.Invoke();",
    ),
    // The same turn while the page opens, hidden again by the host's trigger at a Next: only the refill of the open saw the
    // part visible.
    (
        71118,
        "DocOpenTurnHostHidesActionShow",
        "Seed.Seed(); Log.Reset(false); Log.Arm(true); Doc.OpenEdit(); Log.ArmHide(true); Doc.Next(); Log.ArmHide(false);
         Doc.Flip.Invoke();",
    ),
    // The pin opens the page with OpenEdit; OpenView is refused.
    (
        71112,
        "DocOpenViewTurnByAction",
        "Seed.Seed(); Log.Reset(false); Doc.OpenView(); Doc.GoToKey('H1'); Doc.Flip.Invoke();",
    ),
    // The pinned part has a SubPageLink; a part without one is refused.
    (
        71113,
        "DocUnlinkedTurnByAction",
        "Seed.Seed(); DocUnlinked.OpenEdit(); DocUnlinked.GoToKey('H1'); DocUnlinked.Flip.Invoke();",
    ),
    // The pinned action body only sets the variable: a body that also runs CurrPage.Update(false), writes Rec and runs
    // CurrPage.Update(true), or writes Rec is refused.
    (
        71114,
        "DocTurnByActionWithUpdateFalse",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1'); Doc.FlipUpd.Invoke();",
    ),
    (
        71115,
        "DocTurnByActionWithSave",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1'); Doc.FlipSave.Invoke();",
    ),
    (
        71116,
        "DocTurnByActionWithRecWrite",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1'); Doc.FlipWrite.Invoke();",
    ),
    // The pinned host holds no unsaved edit when the action is invoked; after a SetValue it is refused.
    (
        71117,
        "DocDirtyHostTurnByAction",
        "Seed.Seed(); Log.Reset(false); Doc.OpenEdit(); Doc.GoToKey('H1'); Doc.Name.SetValue('x'); Doc.Flip.Invoke();",
    ),
];

fn source() -> String {
    let mut src = FIXTURES.to_string();
    for (id, name, body) in TESTS {
        src.push_str(&format!(
            "codeunit {id} \"TF T{id}\"\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n    [Test]\n    procedure {name}()\n    var\n        Seed: Codeunit \"TF Seed\";\n        Log: Codeunit \"TF Log\";\n        Doc: TestPage \"TF Doc Page\";\n        Card: TestPage \"TF Card Page\";\n        Unlinked: TestPage \"TF Card Unlinked\";\n        DocUnlinked: TestPage \"TF Doc Unlinked\";\n    begin\n        {body}\n    end;\n}}\n"
        ));
    }
    src
}

fn run() -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), source())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    )
}

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a TestResult {
    rt.results()
        .iter()
        .find(|t| t.test == test)
        .unwrap_or_else(|| panic!("no result for {test}"))
}

fn flagged(rt: &CorpusRuntime, test: &str) -> bool {
    result(rt, test)
        .assumptions
        .contains(&Assumption::HiddenPartShownOutsideAction)
}

/// What `test` should end as: a pass (flagged or not), or a refusal whose reason holds the text.
enum Want {
    Pass { flagged: bool },
    Refused(&'static str),
}

fn check(rt: &CorpusRuntime, test: &str, want: Want) -> Option<String> {
    let r = result(rt, test);
    match (&want, &r.outcome) {
        (Want::Pass { flagged: f }, Outcome::Pass) if flagged(rt, test) == *f => None,
        (Want::Refused(why), Outcome::NotRun(reason)) if reason.contains(why) => None,
        (Want::Pass { flagged: f }, other) => Some(format!(
            "{test}: want pass (flagged {f}), got {other:?}, flagged {}",
            flagged(rt, test)
        )),
        (Want::Refused(why), other) => Some(format!("{test}: want a refusal with <{why}>, got {other:?}")),
    }
}

#[test]
fn hidden_part_turns_and_new_row_shapes() {
    let rt = run();
    let turn = "an action that makes a part visible";
    let wants = [
        // Pinned turn: passes, not flagged.
        ("DocFirstTurnByAction", Want::Pass { flagged: false }),
        // Turns by the host's trigger: pass, flagged, also after earlier action turns.
        ("DocTurnByHostTrigger", Want::Pass { flagged: true }),
        ("DocHostTriggerTurnAfterActionTurns", Want::Pass { flagged: true }),
        // Turns no pin covers: refused.
        ("DocSecondTurnByAction", Want::Refused("the part showed before")),
        ("DocShownAtOpenTurnedAgain", Want::Refused(turn)),
        ("DocActionTurnAfterHostTriggerTurn", Want::Refused(turn)),
        ("CardTurnByAction", Want::Refused("the host page is not a Document")),
        ("DocOpenTurnHideActionShow", Want::Refused(turn)),
        ("DocOpenTurnHostHidesActionShow", Want::Refused(turn)),
        (
            "DocOpenViewTurnByAction",
            Want::Refused("the host page was not opened with OpenEdit"),
        ),
        (
            "DocUnlinkedTurnByAction",
            Want::Refused("the part has no SubPageLink of field terms"),
        ),
        (
            "DocTurnByActionWithUpdateFalse",
            Want::Refused("the action body did more than set the variable"),
        ),
        ("DocTurnByActionWithSave", Want::Refused(turn)),
        ("DocTurnByActionWithRecWrite", Want::Refused(turn)),
        (
            "DocDirtyHostTurnByAction",
            Want::Refused("the host page or a part held an unsaved edit"),
        ),
        ("DocTurnByOnValidate", Want::Pass { flagged: true }),
        // New() on an open host keeps main's behavior: no trigger of the part, and main's refusal at Close.
        ("CardNewOnOpenHostAsMain", Want::Pass { flagged: false }),
        (
            "CardNewOnOpenHostKeyClose",
            Want::Refused("closing a new host row while a part that shows no row"),
        ),
        // An unlinked hidden part on OpenNew.
        ("UnlinkedHiddenPartOpenNew", Want::Refused("without a SubPageLink")),
    ];
    let wrong: Vec<String> = wants
        .into_iter()
        .filter_map(|(test, want)| check(&rt, test, want))
        .collect();
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
