//! al-runner issue 8188: a List page's CardPart FactBox, linked to the row's key, around the new line. The corpus probe (corpus pull
//! request 1426, merged, not pinned yet: tests/al-language/pages/parts/ListHostFactBoxNewLine.al, green on every supported major in its CI) with a
//! stand-in for the corpus's Assert, and the reporter's repro (green on Business Central 29.1); the neighbors of the probe's page
//! keep what main (al-runner 1bd640629) reads. When the probe is pinned, delete the probe's test and read the objects from the
//! corpus file.

use al_builtins::assumptions::Assumption;
use al_testrun::Outcome;

/// Each test of `codeunit` in `src` (with the stand-in Assert), its outcome and the assumptions it ran through.
fn outcomes(src: &str, codeunit: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!("{src}\n{}", include_str!("../fixtures/outside-sc/Assert.al"));
    let l = al_testkit::compile_apps(vec![al_testkit::app(&src)]);
    let engine = al_vm::Engine::new(l.program.clone(), al_testkit::blocked(&l));
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
        al_testrun::isolation::TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    assert!(!outcomes.is_empty());
    outcomes
        .into_iter()
        .map(|(n, o)| {
            let marks = hits
                .iter()
                .find(|(h, _)| *h == n)
                .map(|(_, m)| m.clone())
                .unwrap_or_default();
            (n, o, marks)
        })
        .collect()
}

const PROBE: &str = include_str!("../fixtures/outside-fb/Issue8188Probe.al");
const REPRO: &str = include_str!("../fixtures/outside-fb/Issue8188.al");

/// The probe's two tests and the reporter's two pass, each flagged by the assumption list-factbox-new-line.
#[test]
fn issue_8188_list_factbox_follows_the_new_line() {
    for (src, codeunit) in [(PROBE, "LT FB Tests"), (REPRO, "Repro FactBox Tests")] {
        let bad: Vec<_> = outcomes(src, codeunit)
            .into_iter()
            .filter(|(_, o, marks)| *o != Outcome::Pass || !marks.contains(&Assumption::ListFactboxNewLine))
            .collect();
        assert!(bad.is_empty(), "{codeunit}: {bad:#?}");
    }
}

/// Each step of the probe's two tests and of the neighbors below, logged as the test goes (`O`) and raised at the end, so one
/// outcome holds every step.
const OBSERVE: &str = r#"
codeunit 97059 "LT FB Observe"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT FB Log";
        Obs: Text;

    local procedure O(Name: Text; Value: Text)
    begin
        Obs += Name + '=' + Value + ' | ';
    end;

    local procedure Seed(Rows: Integer)
    var
        Row: Record "LT FB Row";
    begin
        Row.DeleteAll();
        if Rows >= 1 then begin
            Row."No." := 'A';
            Row.Name := 'Alpha';
            Row.Insert();
        end;
        if Rows >= 2 then begin
            Row."No." := 'C';
            Row.Name := 'Gamma';
            Row.Insert();
        end;
        Log.Take();
        Obs := '';
    end;

    [Test]
    procedure EmptyView()
    var
        Rows: TestPage "LT FB Rows";
    begin
        Seed(0);
        Rows.OpenEdit();
        O('open', Log.Take());
        O('part', Rows.Peek.Name.Value());
        Rows."No.".SetValue('B');
        O('setno', Log.Take());
        O('part2', Rows.Peek.Name.Value());
        Rows.Name.SetValue('N');
        O('setname', Log.Take());
        O('part3', Rows.Peek.Name.Value());
        Rows.Close();
        O('close', Log.Take());
        Error('OBSERVE ' + Obs);
    end;

    [Test]
    procedure EmptyViewOpenView()
    var
        Rows: TestPage "LT FB Rows";
    begin
        Seed(0);
        Rows.OpenView();
        O('open', Log.Take());
        O('part', Rows.Peek.Name.Value());
        Rows.Close();
        O('close', Log.Take());
        Error('OBSERVE ' + Obs);
    end;

    [Test]
    procedure RowBefore()
    begin
        Walk(1);
    end;

    [Test]
    procedure TwoRowsBefore()
    begin
        Walk(2);
    end;

    local procedure Walk(Rows: Integer)
    var
        P: TestPage "LT FB Rows";
    begin
        Seed(Rows);
        P.OpenEdit();
        O('open', Log.Take());
        O('part', P.Peek.Name.Value());
        P.New();
        O('new', Log.Take());
        O('partnew', P.Peek.Name.Value());
        P."No.".SetValue('B');
        O('setno', Log.Take());
        O('part2', P.Peek.Name.Value());
        P.Name.SetValue('N');
        O('setname', Log.Take());
        O('part3', P.Peek.Name.Value());
        O('prev', Format(P.Previous()) + '/' + P."No.".Value());
        O('prevlog', Log.Take());
        O('part4', P.Peek.Name.Value());
        O('next', Format(P.Next()) + '/' + P."No.".Value());
        O('nextlog', Log.Take());
        O('part5', P.Peek.Name.Value());
        O('next2', Format(P.Next()) + '/' + P."No.".Value());
        O('next2log', Log.Take());
        O('part6', P.Peek.Name.Value());
        P.Close();
        O('close', Log.Take());
        Error('OBSERVE ' + Obs);
    end;

    [Test]
    procedure NewThenClose()
    var
        Rows: TestPage "LT FB Rows";
    begin
        Seed(1);
        Rows.OpenEdit();
        Rows.New();
        O('partnew', Rows.Peek.Name.Value());
        Rows.Close();
        O('close', Log.Take());
        Error('OBSERVE ' + Obs);
    end;

    [Test]
    procedure NewThenHostReads()
    var
        Rows: TestPage "LT FB Rows";
    begin
        Seed(1);
        Rows.OpenEdit();
        Rows.New();
        O('no', Rows."No.".Value());
        O('caption', Rows.Caption());
        O('namecaption', Rows.Name.Caption());
        O('editable', Format(Rows.Name.Editable()));
        Rows.Close();
        O('close', Log.Take());
        Error('OBSERVE ' + Obs);
    end;

    [Test]
    procedure NewThenCaptionThenKey()
    begin
        KeyAfterNew('B', true);
    end;

    [Test]
    procedure NewThenDuplicateKey()
    begin
        DuplicateKeyAfterNew('A');
    end;

    [Test]
    procedure NewThenDuplicateKeyLowerCase()
    begin
        DuplicateKeyAfterNew('a');
    end;

    local procedure DuplicateKeyAfterNew(NewKey: Text)
    var
        Rows: TestPage "LT FB Rows";
    begin
        Seed(1);
        Rows.OpenEdit();
        Rows.New();
        Rows."No.".SetValue(NewKey);
        O('setno', Log.Take());
        O('part', Rows.Peek.Name.Value());
        Error('OBSERVE ' + Obs);
    end;

    [Test]
    procedure NewThenBlankKey()
    begin
        KeyAfterNew('', false);
    end;

    local procedure KeyAfterNew(NewKey: Text; CaptionFirst: Boolean)
    var
        Rows: TestPage "LT FB Rows";
    begin
        Seed(1);
        Rows.OpenEdit();
        Rows.New();
        if CaptionFirst then
            O('caption', Rows.Caption());
        Rows."No.".SetValue(NewKey);
        O('setno', Log.Take());
        O('part', Rows.Peek.Name.Value());
        Rows.Close();
        O('close', Log.Take());
        Error('OBSERVE ' + Obs);
    end;
}

// One codeunit for each call the new path refuses after the New() that kept the part: a test refused while it runs leaves the
// later tests of its codeunit not run.
codeunit 97060 "LT FB Observe Next"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure NewThenNext()
    var
        Row: Record "LT FB Row";
        Rows: TestPage "LT FB Rows";
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
        Rows.OpenEdit();
        Rows.New();
        Error('OBSERVE next=' + Format(Rows.Next()) + ' | ');
    end;
}

codeunit 97061 "LT FB Observe SetName"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure NewThenSetName()
    var
        Row: Record "LT FB Row";
        Rows: TestPage "LT FB Rows";
    begin
        Row.DeleteAll();
        Row."No." := 'A';
        Row.Insert();
        Rows.OpenEdit();
        Rows.New();
        Rows.Name.SetValue('N');
        Error('OBSERVE ');
    end;
}
"#;

/// The outcome of each test of `OBSERVE` on the probe's objects changed by `edit`.
fn observe(edit: impl Fn(&str) -> String) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!("{}\n{OBSERVE}", edit(PROBE));
    ["LT FB Observe", "LT FB Observe Next", "LT FB Observe SetName"]
        .into_iter()
        .flat_map(|c| outcomes(&src, c))
        .collect()
}

/// A page that leaves the probe's shape in one fact, made from the probe's objects.
type Edit = fn(&str) -> String;

fn host_with_agcr(s: &str) -> String {
    s.replacen(
        "    var\n        Log: Codeunit \"LT FB Log\";\n}\n\npage 97053",
        "    trigger OnAfterGetCurrRecord()\n    begin\n        Log.Add('P.AGCR(' + Rec.\"No.\" + ')');\n    end;\n\n    var\n        Log: Codeunit \"LT FB Log\";\n}\n\npage 97053",
        1,
    )
}

fn host_trigger_writes_rec(s: &str) -> String {
    s.replacen(
        "        Log.Add('P.AGR(' + Rec.\"No.\" + ')');",
        "        Log.Add('P.AGR(' + Rec.\"No.\" + ')');\n        Rec.Name := Rec.Name;",
        1,
    )
}

fn part_update_propagation(s: &str) -> String {
    s.replacen(
        "                SubPageLink = \"No.\" = field(\"No.\");",
        "                SubPageLink = \"No.\" = field(\"No.\");\n                UpdatePropagation = Both;",
        1,
    )
}

fn second_part(s: &str) -> String {
    s.replacen(
        "            part(Peek; \"LT FB Peek\")",
        "            part(Other; \"LT FB Peek\")\n            {\n                ApplicationArea = All;\n            }\n            part(Peek; \"LT FB Peek\")",
        1,
    )
}

fn text_key(s: &str) -> String {
    s.replacen("field(1; \"No.\"; Code[10])", "field(1; \"No.\"; Text[10])", 1)
}

fn page_extension(s: &str) -> String {
    format!("{s}\npageextension 97055 \"LT FB Rows Ext\" extends \"LT FB Rows\"\n{{\n}}\n")
}

/// What main (al-runner 1bd640629) reads in `OBSERVE`'s EmptyView and RowBefore on a page that leaves the probe's shape in one
/// fact other than its host triggers and its parts.
const MAIN_EMPTY: &str = "OBSERVE open= | part= | setno= | part2= | setname=F.AGR(B);F.AGCR(B); | part3= | close= | ";
const MAIN_ROW: &str = "OBSERVE open=P.AGR(A);F.AGR(A);F.AGCR(A); | part=Alpha | new= | partnew= | setno= | part2= | \
    setname=F.AGR(B);F.AGCR(B); | part3= | prev=Yes/A | prevlog=F.AGR(A);F.AGCR(A); | part4=Alpha | next=Yes/B | \
    nextlog=P.AGR(B);F.AGR(B);F.AGCR(B); | part5=N | next2=Yes/ | next2log= | part6= | close= | ";

/// The outcome `name` of `o` is the failure `text` (an observation), and no test went through list-factbox-new-line.
fn reads(o: &[(String, Outcome, Vec<Assumption>)], name: &str, text: &str) -> bool {
    o.iter().all(|(_, _, m)| !m.contains(&Assumption::ListFactboxNewLine))
        && o.iter()
            .any(|(n, r, _)| n == name && matches!(r, Outcome::Fail(t) if t.as_str() == text))
}

/// A page that leaves the probe's shape in one fact keeps main's reads at every step, and runs every call after `New()`: a
/// trigger more on the host, a host trigger that does more than log, a part control property, a second part, a key field that
/// is no Code field, a page extension.
#[test]
fn issue_8188_neighbors_keep_main() {
    let main_host_agcr_empty =
        "OBSERVE open=P.AGCR();P.AGCR(); | part= | setno= | part2= | setname=F.AGR(B);F.AGCR(B); | part3= | close= | ";
    let main_host_agcr_row = "OBSERVE open=P.AGR(A);P.AGR(A);P.AGCR(A);F.AGR(A);F.AGCR(A); | part=Alpha | new=P.AGCR(); | partnew= | \
        setno= | part2= | setname=F.AGR(B);F.AGCR(B); | part3= | prev=Yes/A | prevlog=P.AGCR(A);F.AGR(A);F.AGCR(A); | part4=Alpha | \
        next=Yes/B | nextlog=P.AGR(B);P.AGCR(B);F.AGR(B);F.AGCR(B); | part5=N | next2=Yes/ | next2log=P.AGCR(); | part6= | close= | ";
    let main_second_empty =
        "OBSERVE open= | part= | setno= | part2= | setname=F.AGR(B);F.AGCR(B);F.AGR(B);F.AGCR(B); | part3= | close= | ";
    let main_second_row = "OBSERVE open=P.AGR(A);F.AGR(A);F.AGCR(A);F.AGR(A);F.AGCR(A); | part=Alpha | new=F.AGR(A);F.AGCR(A); | \
        partnew= | setno= | part2= | setname=F.AGR(A);F.AGCR(A);F.AGR(B);F.AGCR(B); | part3= | prev=Yes/A | \
        prevlog=F.AGR(A);F.AGCR(A); | part4=Alpha | next=Yes/B | nextlog=P.AGR(B);F.AGR(B);F.AGCR(B); | part5=N | next2=Yes/ | \
        next2log= | part6= | close= | ";
    let cells: [(&str, Edit, &str, &str); 6] = [
        (
            "host OnAfterGetCurrRecord",
            host_with_agcr,
            main_host_agcr_empty,
            main_host_agcr_row,
        ),
        ("host trigger writes Rec", host_trigger_writes_rec, MAIN_EMPTY, MAIN_ROW),
        ("part UpdatePropagation", part_update_propagation, MAIN_EMPTY, MAIN_ROW),
        ("second part", second_part, main_second_empty, main_second_row),
        ("key Text", text_key, MAIN_EMPTY, MAIN_ROW),
        ("page extension", page_extension, MAIN_EMPTY, MAIN_ROW),
    ];
    for (what, edit, empty, row) in cells {
        let o = observe(edit);
        let kept = reads(&o, "EmptyView", empty)
            && reads(&o, "RowBefore", row)
            && reads(&o, "NewThenNext", "OBSERVE next=No | ")
            && reads(&o, "NewThenSetName", "OBSERVE ");
        assert!(kept, "{what}: {o:#?}");
    }
}

/// On the probe's pages, the opens and rows the probe does not measure keep main's reads: OpenView of the empty view, and
/// `New()` from the first of two rows.
#[test]
fn issue_8188_unmeasured_open_and_rows_keep_main() {
    let o = observe(|s| s.to_string());
    let two_rows = "OBSERVE open=P.AGR(A);P.AGR(C);P.AGR(A);F.AGR(A);F.AGCR(A); | part=Alpha | new= | partnew= | setno= | part2= | \
        setname=F.AGR(B);F.AGCR(B); | part3= | prev=Yes/A | prevlog=F.AGR(A);F.AGCR(A); | part4=Alpha | next=Yes/C | \
        nextlog=F.AGR(C);F.AGCR(C); | part5=Gamma | next2=Yes/ | next2log= | part6= | close= | ";
    let only = |name: &str| o.iter().filter(|(n, _, _)| n == name).cloned().collect::<Vec<_>>();
    assert!(
        reads(
            &only("EmptyViewOpenView"),
            "EmptyViewOpenView",
            "OBSERVE open= | part= | close= | "
        ),
        "{o:#?}"
    );
    assert!(reads(&only("TwoRowsBefore"), "TwoRowsBefore", two_rows), "{o:#?}");
}

/// After the `New()` that kept the part on the row before, Close runs what it runs on main (the part shows that row until then);
/// a call the probe does not make there is refused: `Next()`, the SetValue of a field outside the key.
#[test]
fn issue_8188_calls_after_the_held_new_line() {
    let o = observe(|s| s.to_string());
    let outcome = |name: &str| o.iter().find(|(n, _, _)| n == name).map(|(_, r, _)| r.clone());
    assert_eq!(
        outcome("NewThenClose"),
        Some(Outcome::Fail(
            "OBSERVE partnew=Alpha | close=P.AGR(A);F.AGR(A);F.AGCR(A); | ".into()
        ))
    );
    for name in ["NewThenNext", "NewThenSetName"] {
        assert!(
            matches!(outcome(name), Some(Outcome::NotRun(r)) if r.contains("after New() on a List page whose CardPart FactBox still shows the row before")),
            "{name}: {o:#?}"
        );
    }
}

/// After the `New()` that kept the part on the row before, the calls whose answer does not depend on the part keep main's
/// answers (al-runner 1bd640629): the host's reads and property calls, and the key's SetValue the probe does not measure (a key a
/// row holds, also written in lower case; a blank key; a key after another call than a read), which lets the part read the new
/// line as main does at `New()`.
#[test]
fn issue_8188_main_reads_after_the_held_new_line() {
    let o = observe(|s| s.to_string());
    let outcome = |name: &str| o.iter().find(|(n, _, _)| n == name).map(|(_, r, _)| r.clone());
    let main = [
        (
            "NewThenHostReads",
            "OBSERVE no= | caption=LT FB Rows | namecaption=Name | editable=Yes | close=P.AGR(A);F.AGR(A);F.AGCR(A); | ",
        ),
        (
            "NewThenCaptionThenKey",
            "OBSERVE caption=LT FB Rows | setno=P.AGR(A);F.AGR(A);F.AGCR(A); | part= | close=F.AGR(B);F.AGCR(B); | ",
        ),
        (
            "NewThenDuplicateKey",
            "OBSERVE setno=P.AGR(A);F.AGR(A);F.AGCR(A); | part= | ",
        ),
        (
            "NewThenDuplicateKeyLowerCase",
            "OBSERVE setno=P.AGR(A);F.AGR(A);F.AGCR(A); | part= | ",
        ),
        (
            "NewThenBlankKey",
            "OBSERVE setno=P.AGR(A);F.AGR(A);F.AGCR(A); | part= | close=F.AGR();F.AGCR(); | ",
        ),
    ];
    for (name, read) in main {
        assert_eq!(outcome(name), Some(Outcome::Fail(read.into())), "{name}");
    }
}
