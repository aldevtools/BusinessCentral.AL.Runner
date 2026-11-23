//! The cells of corpus pull request 1518, tests/al-language/pages/currpage/ListValidateFilterRefresh.al (open), that
//! `validate_view_refresh.rs` runs: the code of each measured page line for line (`pinned_body.rs`), and the test's steps on
//! it, each cell with the line of its test.

/// The probe's table 97581 "LT RFU Line" (ListValidateFilterRefresh.al:17-41), and the table of al-runner issue 8698's repro:
/// `«L»` is the log codeunit.
const TABLE: [&str; 7] = [
    "DataClassification = CustomerContent",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer",
    "field 3 \"Name\": Text[30]",
    "field 4 \"Value\": Text[30]",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
    "trigger \"Name\".OnValidate: var Log: Codeunit \"«L»\"; begin Log.Add((('T.V(' + Rec.\"Name\") + ')')); end",
];

/// The issue's page: the probe's page 97583 "LT RFU Issue Lines" (:88-131) and the page of issue 8698's repro read the same;
/// `«T»` is the table.
const ISSUE_PAGE: [&str; 10] = [
    "PageType = List",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Name in Lines = Name {applicationarea=All}",
    "  trigger onvalidate: begin Rec.SetRange(field#0, 'G1'); CurrPage.Update(false); end",
    "field Value in Lines = Value {applicationarea=All}",
    "  trigger onvalidate: begin CurrPage.Update(false); end",
    "trigger onaftergetrecord: begin Log.Add((('P.AGR(' + Rec.\"Name\") + ')')); end",
];

/// The controls and the action of the probe's configurable pages (97584-97588): the Name control's OnValidate and the action
/// run ChangeView.
const NAME_VALIDATE: &str = "  trigger onvalidate: begin Log.Add((('P.V(' + Rec.\"Name\") + ')')); ChangeView(); end";
const VALUE_VALIDATE: &str = "  trigger onvalidate: begin CurrPage.Update(false); end";
const ACTION: &str = "action ChangeViewAction in - {applicationarea=All}";
const ON_ACTION: &str = "  trigger onaction: begin Log.Add((('P.A(' + Rec.\"Name\") + ')')); ChangeView(); end";
const ACTION_AREA: &str = "area Processing: ChangeViewAction";
const AGR: &str = "trigger onaftergetrecord: begin Log.Add((('P.AGR(' + Rec.\"Name\") + ')')); end";
const AGCR: &str = "trigger onaftergetcurrrecord: begin Log.Add((('P.AGCR(' + Rec.\"Name\") + ')')); end";

/// Page 97584 "LT RFU Lines" (:134-215): a List with OnAfterGetRecord only.
const LINES_PAGE: [&str; 13] = [
    "PageType = List",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Name in Lines = Name {applicationarea=All}",
    NAME_VALIDATE,
    "field Value in Lines = Value {applicationarea=All}",
    VALUE_VALIDATE,
    ACTION,
    ON_ACTION,
    ACTION_AREA,
    AGR,
];

/// Page 97585 "LT RFU Lines Both" (:217-303): the same with OnAfterGetRecord and OnAfterGetCurrRecord.
const BOTH_PAGE: [&str; 14] = [
    LINES_PAGE[0],
    LINES_PAGE[1],
    LINES_PAGE[2],
    LINES_PAGE[3],
    LINES_PAGE[4],
    LINES_PAGE[5],
    NAME_VALIDATE,
    LINES_PAGE[7],
    VALUE_VALIDATE,
    ACTION,
    ON_ACTION,
    ACTION_AREA,
    AGR,
    AGCR,
];

/// Page 97586 "LT RFU Lines Curr" (:305-386): the same with OnAfterGetCurrRecord only.
const CURR_PAGE: [&str; 13] = [
    LINES_PAGE[0],
    LINES_PAGE[1],
    LINES_PAGE[2],
    LINES_PAGE[3],
    LINES_PAGE[4],
    LINES_PAGE[5],
    NAME_VALIDATE,
    LINES_PAGE[7],
    VALUE_VALIDATE,
    ACTION,
    ON_ACTION,
    ACTION_AREA,
    AGCR,
];

/// Page 97588 "LT RFU Card" (:466-546): a Card with OnAfterGetRecord only.
const CARD_PAGE: [&str; 13] = [
    "PageType = Card",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "group General in - {}",
    "field Name in General = Name {applicationarea=All}",
    NAME_VALIDATE,
    "field Value in General = Value {applicationarea=All}",
    VALUE_VALIDATE,
    ACTION,
    ON_ACTION,
    ACTION_AREA,
    AGR,
];

/// The procedures the pages and the table call: codeunit 97582 "LT RFU Log" (`«L»`, :43-84 Add, FilterStep, UpdateStep) and
/// the page's ChangeView (`«P»`, :195-213 on page 97584, the same on 97585-97588).
const CALLEES: [&str; 4] = [
    "\"«L»\".Add: begin Buffer := (Buffer + (Entry + ';')); end",
    "\"«L»\".FilterStep: begin exit(Filtering); end",
    "\"«L»\".UpdateStep: begin exit(Updating); end",
    "\"«P»\".ChangeView: begin case Log.FilterStep() of 1: begin Rec.SetRange(field#0, 'G1'); end; 2: begin \
     Rec.SetFilter(field#0, 'G1'); end; 3: begin Rec.Reset(); end; 4: begin Rec.SetRange(field#0, 'G0'); end; end; case \
     Log.UpdateStep() of 1: begin CurrPage.Update(false); end; 2: begin CurrPage.Update(true); end; end; end",
];

/// The Card of al-runner issue 8717: the probe's table 97570 "LT RFU Edit Row" (:1532-1546) and the repro's table.
const EDIT_TABLE: [&str; 4] = [
    "DataClassification = CustomerContent",
    "field 1 \"No.\": Code[20]",
    "field 2 \"Name\": Text[30]",
    "key \"No.\" {Clustered=true}",
];

/// Its page: the probe's page 97571 "LT RFU Edit Card" (:1570-1607) and the repro's page read the same.
const EDIT_PAGE: [&str; 9] = [
    "PageType = Card",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "group General in - {}",
    "field No. in General = No. {applicationarea=All}",
    "field Name in General = Name {applicationarea=All}",
    VALUE_VALIDATE,
    "trigger onaftergetrecord: begin Log.Add((('AGR(' + Rec.\"Name\") + ')')); end",
];

/// Its log codeunit's Add: the probe's codeunit 97573 "LT RFU Edit Log" (:1548-1568) and the repro's.
const EDIT_CALLEES: [&str; 1] = [CALLEES[0]];

/// Which measured page a page is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellPage {
    /// Page 97583, the issue's page of 8698: a List with OnAfterGetRecord only.
    Issue,
    /// Page 97584: a List with OnAfterGetRecord only.
    Lines,
    /// Page 97585: a List with OnAfterGetRecord and OnAfterGetCurrRecord.
    Both,
    /// Page 97586: a List with OnAfterGetCurrRecord only.
    Curr,
    /// Page 97588: a Card with OnAfterGetRecord only.
    Card,
    /// Page 97571, the Card of issue 8717.
    EditCard,
}

impl CellPage {
    pub(crate) const ALL: [CellPage; 6] = [
        CellPage::Issue,
        CellPage::Lines,
        CellPage::Both,
        CellPage::Curr,
        CellPage::Card,
        CellPage::EditCard,
    ];

    /// The lines of the page, of its table and of the procedures they call.
    pub(crate) fn code(
        self,
    ) -> (
        &'static [&'static str],
        &'static [&'static str],
        &'static [&'static str],
    ) {
        match self {
            CellPage::Issue => (&ISSUE_PAGE, &TABLE, &CALLEES),
            CellPage::Lines => (&LINES_PAGE, &TABLE, &CALLEES),
            CellPage::Both => (&BOTH_PAGE, &TABLE, &CALLEES),
            CellPage::Curr => (&CURR_PAGE, &TABLE, &CALLEES),
            CellPage::Card => (&CARD_PAGE, &TABLE, &CALLEES),
            CellPage::EditCard => (&EDIT_PAGE, &EDIT_TABLE, &EDIT_CALLEES),
        }
    }

    /// The cells on the page.
    pub(crate) fn cells(self) -> &'static [Cell] {
        match self {
            CellPage::Issue => &ISSUE_CELLS,
            CellPage::Lines => &LINES_CELLS,
            CellPage::Both => &BOTH_CELLS,
            CellPage::Curr => &CURR_CELLS,
            CellPage::Card => &CARD_CELLS,
            CellPage::EditCard => &EDIT_CELLS,
        }
    }

    /// The rows the table held in every cell on the page (:563-565 three; :1622 one).
    pub(crate) fn table_rows(self) -> usize {
        match self {
            CellPage::EditCard => 1,
            _ => 3,
        }
    }

    /// A List with OnAfterGetRecord and without OnAfterGetCurrRecord: its moves read no row, so a SetValue, the action and
    /// Close read the row first when the page's last read was of another row. On the other pages every cell's last read
    /// before such a call was of the row itself.
    pub(crate) fn reads_first(self) -> bool {
        matches!(self, CellPage::Issue | CellPage::Lines)
    }
}

/// The control a SetValue typed into: the one bound to the field Name or to the field Value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ctl {
    Name,
    Value,
}

/// The `CurrPage.Update` a step ran: `Update(false)`, `Update(true)`, or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Upd {
    False,
    True,
    None,
}

/// What a step did to the page's view: nothing; a filter on one value of the first key field set where there was none, as
/// SetRange sets it, the row the step started on still in the view (`Keep`) or not (`Drop`); the same as SetFilter sets it,
/// the row still in the view (`KeepText`); a filter SetRange set taken off again (`Reset`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Change {
    Same,
    Keep,
    KeepText,
    Drop,
    Reset,
    Other,
}

/// One step of the test the cells take, reads left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    Last,
    Previous,
    GoToKey,
    /// A SetValue, with the rows of the view after it and the index of the row its refresh settled on (`None`: no refresh).
    Set(Ctl, Upd, Change, Option<(usize, usize)>),
    /// A SetValue that raised an error before its control's OnValidate (a value too long for the field).
    Fail(Ctl),
    /// The action ChangeViewAction (the refresh after it is main's).
    Action(Upd, Change),
    /// The test's read-only calls between two steps, any number of them (`validate_view_reads`).
    Reads,
    /// A step the cells do not take.
    Other,
}

/// The calls of a cell after its steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tail {
    /// First, Next until it answers No, and Close.
    Walk,
    /// First, then Close.
    FirstClose,
    /// Close.
    Close,
}

/// The steps of one cell and the calls after them.
pub(crate) struct Cell {
    pub(crate) steps: &'static [Step],
    pub(crate) tail: Tail,
}

use Change::{Drop, Keep, KeepText, Reset, Same};
use Ctl::{Name, Value as Val};
use Step::{Action, Fail, GoToKey, Last, Previous, Reads, Set};

const fn walk(steps: &'static [Step]) -> Cell {
    Cell {
        steps,
        tail: Tail::Walk,
    }
}

const fn close(steps: &'static [Step]) -> Cell {
    Cell {
        steps,
        tail: Tail::Close,
    }
}

/// Last, a SetValue on Name with `update` and `change` whose refresh settled on row `at` of a view of `rows`, then a SetValue on
/// Value (Update(false), no change) that settled there too.
const fn name_then_value(update: Upd, change: Change, rows: usize, at: usize) -> [Step; 3] {
    [
        Last,
        Set(Name, update, change, Some((rows, at))),
        Set(Val, Upd::False, Same, Some((rows, at))),
    ]
}

const KEEP_F: [Step; 3] = name_then_value(Upd::False, Keep, 2, 1);
const KEEP_T: [Step; 3] = name_then_value(Upd::True, Keep, 2, 1);
const DROP_F: [Step; 3] = name_then_value(Upd::False, Drop, 1, 0);
const SAME_F: [Step; 3] = name_then_value(Upd::False, Same, 3, 2);
const KEEP_TEXT_F: [Step; 3] = name_then_value(Upd::False, KeepText, 2, 1);
const KEEP_TEXT_T: [Step; 3] = name_then_value(Upd::True, KeepText, 2, 1);
const DROP_T: [Step; 3] = name_then_value(Upd::True, Drop, 1, 0);
const SAME_T: [Step; 3] = name_then_value(Upd::True, Same, 3, 2);
const ACTION_KEEP_F: [Step; 3] = [Last, Action(Upd::False, Keep), Set(Val, Upd::False, Same, Some((2, 1)))];

/// `steps` with the test's read-only calls at `at`: 1 after Last, 2 after the SetValue on Name or the action, 3 after the
/// SetValue on Value, before First.
const fn reads_at(steps: [Step; 3], at: usize) -> [Step; 4] {
    let mut out = [Reads; 4];
    let mut i = 0;
    while i < 3 {
        out[if i < at { i } else { i + 1 }] = steps[i];
        i += 1;
    }
    out
}

/// The cells' steps with the read-only calls at each point (corpus pull request 1529, ListValidateFilterRefresh.al
/// `RunReadsLines` and the others, two tests per point: block 1 and block 2 of `ReadsLines`).
const KEEP_F_READS: [[Step; 4]; 3] = [reads_at(KEEP_F, 1), reads_at(KEEP_F, 2), reads_at(KEEP_F, 3)];
const ACTION_READS: [[Step; 4]; 3] = [
    reads_at(ACTION_KEEP_F, 1),
    reads_at(ACTION_KEEP_F, 2),
    reads_at(ACTION_KEEP_F, 3),
];

/// The cells on page 97584 (the line of each test).
const LINES_CELLS: [Cell; 26] = [
    walk(&KEEP_F_READS[0]), // PR 1529 :1909-:2004
    walk(&KEEP_F_READS[1]),
    walk(&KEEP_F_READS[2]),
    walk(&ACTION_READS[0]), // :2023-:2118
    walk(&ACTION_READS[1]),
    walk(&ACTION_READS[2]),
    walk(&KEEP_F),      // :832
    walk(&KEEP_TEXT_F), // :883
    walk(&KEEP_T),      // :849
    walk(&KEEP_TEXT_T), // :1327
    walk(&[
        Last,
        Set(Name, Upd::None, Keep, None),
        Set(Val, Upd::False, Same, Some((2, 1))),
    ]), // :866
    walk(&DROP_F),      // :900
    walk(&DROP_T),      // :917
    walk(&SAME_F),      // :951, and :1345 (Reset of no filter)
    walk(&SAME_T),      // :968
    walk(&[
        Last,
        Set(Name, Upd::False, Keep, Some((2, 1))),
        Set(Name, Upd::False, Reset, Some((3, 2))),
    ]), // :985
    walk(&[
        Last,
        Previous,
        Set(Name, Upd::False, Keep, Some((2, 0))),
        Set(Val, Upd::False, Same, Some((2, 0))),
    ]), // :1003
    walk(&[
        Set(Name, Upd::False, Drop, Some((2, 0))),
        Set(Val, Upd::False, Same, Some((2, 0))),
    ]), // :1020
    walk(&[
        Set(Name, Upd::False, Same, Some((3, 0))),
        Set(Val, Upd::False, Same, Some((3, 0))),
    ]), // :1037
    walk(&ACTION_KEEP_F), // :1054
    walk(&[Last, Action(Upd::False, Drop), Set(Val, Upd::False, Same, Some((1, 0)))]), // :1071
    walk(&[Last, Action(Upd::None, Keep), Set(Val, Upd::False, Same, Some((2, 1)))]), // :1088
    walk(&[
        Last,
        Set(Name, Upd::None, Drop, None),
        Set(Val, Upd::False, Same, Some((1, 0))),
    ]), // :1363
    close(&[KEEP_F[0], KEEP_F[1], KEEP_F[2], Previous]), // :1381
    close(&[KEEP_F[0], KEEP_F[1], KEEP_F[2], KEEP_F[2]]), // :1397
    close(&[
        Last,
        Fail(Name),
        Set(Name, Upd::False, Keep, Some((2, 1))),
        Set(Val, Upd::False, Same, Some((2, 1))),
    ]), // :1413
];

/// The cells on the issue's page (:815; :1477 the steps of the repro of issue 8698, First then Close; :1444 First right after
/// the SetValue on Name).
const ISSUE_CELLS: [Cell; 6] = [
    walk(&KEEP_F),
    walk(&KEEP_F_READS[0]), // PR 1529 :1795-:1890
    walk(&KEEP_F_READS[1]),
    walk(&KEEP_F_READS[2]),
    Cell {
        steps: &KEEP_F,
        tail: Tail::FirstClose,
    },
    walk(&[Last, Set(Name, Upd::False, Keep, Some((2, 1)))]),
];

/// The cells on page 97585, both row triggers (:1105, :1122, :1139, :1156, :1173).
const BOTH_CELLS: [Cell; 11] = [
    walk(&KEEP_F),
    walk(&KEEP_F_READS[0]), // PR 1529 :2137-:2232
    walk(&KEEP_F_READS[1]),
    walk(&KEEP_F_READS[2]),
    walk(&ACTION_READS[0]), // :2251-:2346
    walk(&ACTION_READS[1]),
    walk(&ACTION_READS[2]),
    walk(&KEEP_T),
    walk(&DROP_F),
    walk(&SAME_F),
    walk(&ACTION_KEEP_F),
];

/// The cells on page 97586, OnAfterGetCurrRecord only (:1190, :1207).
const CURR_CELLS: [Cell; 5] = [
    walk(&KEEP_F),
    walk(&SAME_F),
    walk(&KEEP_F_READS[0]), // PR 1529 :2365-:2460
    walk(&KEEP_F_READS[1]),
    walk(&KEEP_F_READS[2]),
];

/// The cells on page 97588, the Card (:1258, :1275, :1292, :1309).
const CARD_CELLS: [Cell; 10] = [
    walk(&KEEP_F),
    walk(&KEEP_T),
    walk(&SAME_F),
    walk(&ACTION_KEEP_F),
    walk(&KEEP_F_READS[0]), // PR 1529 :2479-:2574
    walk(&KEEP_F_READS[1]),
    walk(&KEEP_F_READS[2]),
    walk(&ACTION_READS[0]), // :2593-:2688
    walk(&ACTION_READS[1]),
    walk(&ACTION_READS[2]),
];

/// The cell on page 97571, the Card of issue 8717, and the repro's test: GoToKey to its only row, the SetValue on Name, Close
/// (:1632).
const EDIT_CELLS: [Cell; 3] = [
    close(&[GoToKey, Set(Name, Upd::False, Same, Some((1, 0)))]),
    close(&[GoToKey, Reads, Set(Name, Upd::False, Same, Some((1, 0)))]), // PR 1529 :2846, :2869
    close(&[GoToKey, Set(Name, Upd::False, Same, Some((1, 0))), Reads]), // :2892, :2915
];
