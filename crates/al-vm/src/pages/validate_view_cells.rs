//! The cells of corpus pull request 1518, tests/al-language/pages/currpage/ListValidateFilterRefresh.al (merged), with the
//! read cells of corpus pull requests 1529 (the same file) and 1535 (ListValidateViewReads.al, open), and of corpus pull
//! request 1539, TreeValidateFilterRefresh.al (open), and of corpus pull request 1554, TreeValidateFilterShapes.al (open), whose
//! tree pages are those of 1539 line for line, that `validate_view_refresh.rs` runs: the code of each measured page line for line
//! (`pinned_body.rs`), and the test's steps on it, each cell with the line of its test.
//!
//! The probe objects of corpus pull request 1518 (ListValidateFilterRefresh.al) line for line, one
//! `CellPage` per page shape (the List without OnAfterGetCurrRecord, the issue's List, the List with
//! both row triggers, the List with OnAfterGetCurrRecord only, the Card, the Card of issue 8717; and
//! the trees of corpus pull request 1539, TreeValidateFilterRefresh.al: the issue's tree of 8833 and
//! the trees with OnAfterGetRecord, both row triggers, OnAfterGetCurrRecord and CollapseAll; and of
//! corpus pull request 1554, TreeValidateFilterShapes.al: the tree of 8846 with CollapseAll and with
//! OnAfterGetCurrRecord; a tree cell's rows by equality in `Cell::rows`), and the
//! cells' test calls on each (`Cell`, `Step`; the test's read-only calls at the points corpus pull
//! requests 1529 and 1535 measure are one `Step::Reads`, the calls of each page in
//! `CellPage::read_calls`). Page 97498 of ListValidateViewReads.al (`CellPage::Wide`) is the issue's
//! page with controls of every type. Data only; `validate_view_refresh.rs` reads it.

use al_builtins::assumptions::Assumption;

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

/// The read probe's table 97496 "LT RVR Row" (corpus pull request 1535, ListValidateViewReads.al:15-49): page 97583's table
/// with fields of every type; `«L»` is its log codeunit.
const WIDE_TABLE: [&str; 14] = [
    "DataClassification = CustomerContent",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer",
    "field 3 \"Name\": Text[30]",
    "field 4 \"Value\": Text[30]",
    "field 5 \"Kind\": Option Alpha,Beta,Gamma",
    "field 6 \"Qty\": Decimal",
    "field 7 \"Day\": Date",
    "field 8 \"At\": Time",
    "field 9 \"Stamp\": DateTime",
    "field 10 \"Flag\": Boolean",
    "field 11 \"Indentation\": Integer",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
    "trigger \"Name\".OnValidate: var Log: Codeunit \"«L»\"; begin Log.Add((('T.V(' + Rec.\"Name\") + ')')); end",
];

/// Its page 97498 "LT RVR Lines" (:73-144): the issue's page with a control for each other field.
const WIDE_PAGE: [&str; 17] = [
    "PageType = List",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Name in Lines = Name {applicationarea=All}",
    "  trigger onvalidate: begin Rec.SetRange(field#0, 'G1'); CurrPage.Update(false); end",
    "field Value in Lines = Value {applicationarea=All}",
    "  trigger onvalidate: begin CurrPage.Update(false); end",
    "field Entry No. in Lines = Entry No. {applicationarea=All}",
    "field Kind in Lines = Kind {applicationarea=All}",
    "field Qty in Lines = Qty {applicationarea=All}",
    "field Day in Lines = Day {applicationarea=All}",
    "field At in Lines = At {applicationarea=All}",
    "field Stamp in Lines = Stamp {applicationarea=All}",
    "field Flag in Lines = Flag {applicationarea=All}",
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

/// The tree pages of corpus pull request 1539, tests/al-language/pages/currpage/TreeValidateFilterRefresh.al (open): the issue's
/// table 97716 "LT TVF Row" (:19-35), the table of al-runner issue 8833's repro reads the same.
const TREE_ISSUE_TABLE: [&str; 6] = [
    "DataClassification = SystemMetadata",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer",
    "field 3 \"Name\": Text[30]",
    "field 4 \"Indentation\": Integer",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
];

/// The repeater of the tree pages: C indented under B, TreeInitialState ExpandAll; and the same with CollapseAll.
const TREE_REPEATER: &str = "repeater Lines in - {indentationcolumn=Rec.Indentation; indentationcontrols=Name; showastree=true; \
                             treeinitialstate=ExpandAll}";
const TREE_COLLAPSED_REPEATER: &str = "repeater Lines in - {indentationcolumn=Rec.Indentation; indentationcontrols=Name; \
                                       showastree=true; treeinitialstate=CollapseAll}";

/// The issue's page 97718 "LT TVF Issue Tree" (:82-119); the page of issue 8833's repro reads the same, and its log
/// codeunit's Add (`EDIT_CALLEES`).
const TREE_ISSUE_PAGE: [&str; 8] = [
    "PageType = List",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = Lists",
    TREE_REPEATER,
    "field Name in Lines = Name {}",
    "  trigger onvalidate: begin Rec.SetRange(field#0, 'G1'); CurrPage.Update(false); end",
    "trigger onaftergetrecord: begin Log.Add((('AGR(' + Rec.\"Name\") + ')')); end",
];

/// Page 97714 "LT TVF Drop Tree" (:1585-1614): the issue's tree page without OnAfterGetRecord; the page of al-runner issue
/// 8846's repro reads the same.
const TREE_DROP_PAGE: [&str; 7] = [
    TREE_ISSUE_PAGE[0],
    TREE_ISSUE_PAGE[1],
    TREE_ISSUE_PAGE[2],
    TREE_ISSUE_PAGE[3],
    TREE_ISSUE_PAGE[4],
    TREE_ISSUE_PAGE[5],
    TREE_ISSUE_PAGE[6],
];

/// Pages 97678 "LT TVS Drop Tree Collapsed" (TreeValidateFilterShapes.al:833-862, corpus pull request 1554): page 97714 with
/// TreeInitialState CollapseAll; and 97679 "LT TVS Drop Tree Curr" (:865-902): page 97714 with OnAfterGetCurrRecord, whose log
/// codeunit 97681's Add reads as `EDIT_CALLEES`. Their table 97677 reads as `TREE_ISSUE_TABLE`.
const TREE_DROP_COLLAPSED_PAGE: [&str; 7] = [
    TREE_ISSUE_PAGE[0],
    TREE_ISSUE_PAGE[1],
    TREE_ISSUE_PAGE[2],
    TREE_ISSUE_PAGE[3],
    TREE_COLLAPSED_REPEATER,
    TREE_ISSUE_PAGE[5],
    TREE_ISSUE_PAGE[6],
];
const TREE_DROP_CURR_PAGE: [&str; 8] = [
    TREE_ISSUE_PAGE[0],
    TREE_ISSUE_PAGE[1],
    TREE_ISSUE_PAGE[2],
    TREE_ISSUE_PAGE[3],
    TREE_ISSUE_PAGE[4],
    TREE_ISSUE_PAGE[5],
    TREE_ISSUE_PAGE[6],
    "trigger onaftergetcurrrecord: begin Log.Add((('AGCR(' + Rec.\"Name\") + ')')); end",
];

/// Table 97719 "LT TVF Line" (:121-138) of the configurable tree pages, whose ChangeView and log codeunit 97717 read as
/// `CALLEES`.
const TREE_TABLE: [&str; 7] = [
    "DataClassification = CustomerContent",
    "field 1 \"Group Code\": Code[10]",
    "field 2 \"Entry No.\": Integer",
    "field 3 \"Name\": Text[30]",
    "field 4 \"Value\": Text[30]",
    "field 5 \"Indentation\": Integer",
    "key \"Group Code\", \"Entry No.\" {Clustered=true}",
];

/// Page 97720 "LT TVF Tree" (:141-226): the tree with OnAfterGetRecord only. 97721 adds OnAfterGetCurrRecord (:229-319), 97722
/// has OnAfterGetCurrRecord only (:322-407), 97723 is 97720 with TreeInitialState CollapseAll (:410-495).
const TREE_PAGE: [&str; 13] = [
    LINES_PAGE[0],
    LINES_PAGE[1],
    LINES_PAGE[2],
    LINES_PAGE[3],
    TREE_REPEATER,
    LINES_PAGE[5],
    NAME_VALIDATE,
    LINES_PAGE[7],
    VALUE_VALIDATE,
    ACTION,
    ON_ACTION,
    ACTION_AREA,
    AGR,
];
const TREE_BOTH_PAGE: [&str; 14] = [
    LINES_PAGE[0],
    LINES_PAGE[1],
    LINES_PAGE[2],
    LINES_PAGE[3],
    TREE_REPEATER,
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
const TREE_CURR_PAGE: [&str; 13] = [
    LINES_PAGE[0],
    LINES_PAGE[1],
    LINES_PAGE[2],
    LINES_PAGE[3],
    TREE_REPEATER,
    LINES_PAGE[5],
    NAME_VALIDATE,
    LINES_PAGE[7],
    VALUE_VALIDATE,
    ACTION,
    ON_ACTION,
    ACTION_AREA,
    AGCR,
];
const TREE_COLLAPSED_PAGE: [&str; 13] = [
    LINES_PAGE[0],
    LINES_PAGE[1],
    LINES_PAGE[2],
    LINES_PAGE[3],
    TREE_COLLAPSED_REPEATER,
    LINES_PAGE[5],
    NAME_VALIDATE,
    LINES_PAGE[7],
    VALUE_VALIDATE,
    ACTION,
    ON_ACTION,
    ACTION_AREA,
    AGR,
];

/// Which measured page a page is (the line numbers of TreeValidateFilterRefresh.al are those of corpus pull request 1539's head
/// 4e9aaba6, whose lines are those of 7eab6eba).
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
    /// Page 97498 of ListValidateViewReads.al: the issue's page with controls of every type.
    Wide,
    /// Page 97718 of TreeValidateFilterRefresh.al, the issue's tree of 8833: OnAfterGetRecord only.
    TreeIssue,
    /// Page 97720: a tree with OnAfterGetRecord only.
    Tree,
    /// Page 97721: a tree with OnAfterGetRecord and OnAfterGetCurrRecord.
    TreeBoth,
    /// Page 97722: a tree with OnAfterGetCurrRecord only.
    TreeCurr,
    /// Page 97723: the tree of 97720 with TreeInitialState CollapseAll.
    TreeCollapsed,
    /// Page 97714, the tree of issue 8846: the issue's tree of 8833 without OnAfterGetRecord.
    TreeDrop,
    /// Page 97727: the List over the tree's table, 97720 without ShowAsTree.
    TreeLines,
    /// Page 97678 of TreeValidateFilterShapes.al: page 97714 with TreeInitialState CollapseAll.
    TreeDropCollapsed,
    /// Page 97679 of TreeValidateFilterShapes.al: page 97714 with OnAfterGetCurrRecord.
    TreeDropCurr,
}

impl CellPage {
    pub(crate) const ALL: [CellPage; 16] = [
        CellPage::Issue,
        CellPage::Lines,
        CellPage::Both,
        CellPage::Curr,
        CellPage::Card,
        CellPage::EditCard,
        CellPage::Wide,
        CellPage::TreeIssue,
        CellPage::Tree,
        CellPage::TreeBoth,
        CellPage::TreeCurr,
        CellPage::TreeCollapsed,
        CellPage::TreeDrop,
        CellPage::TreeLines,
        CellPage::TreeDropCollapsed,
        CellPage::TreeDropCurr,
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
            CellPage::Wide => (&WIDE_PAGE, &WIDE_TABLE, &EDIT_CALLEES),
            CellPage::TreeIssue => (&TREE_ISSUE_PAGE, &TREE_ISSUE_TABLE, &EDIT_CALLEES),
            CellPage::Tree => (&TREE_PAGE, &TREE_TABLE, &CALLEES),
            CellPage::TreeBoth => (&TREE_BOTH_PAGE, &TREE_TABLE, &CALLEES),
            CellPage::TreeCurr => (&TREE_CURR_PAGE, &TREE_TABLE, &CALLEES),
            CellPage::TreeCollapsed => (&TREE_COLLAPSED_PAGE, &TREE_TABLE, &CALLEES),
            CellPage::TreeDrop => (&TREE_DROP_PAGE, &TREE_ISSUE_TABLE, &[]),
            CellPage::TreeLines => (&LINES_PAGE, &TREE_TABLE, &CALLEES),
            CellPage::TreeDropCollapsed => (&TREE_DROP_COLLAPSED_PAGE, &TREE_ISSUE_TABLE, &[]),
            CellPage::TreeDropCurr => (&TREE_DROP_CURR_PAGE, &TREE_ISSUE_TABLE, &EDIT_CALLEES),
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
            CellPage::Wide => &WIDE_CELLS,
            CellPage::TreeIssue => &TREE_ISSUE_CELLS,
            CellPage::Tree => &TREE_CELLS,
            CellPage::TreeBoth => &TREE_BOTH_CELLS,
            CellPage::TreeCurr => &TREE_CURR_CELLS,
            CellPage::TreeCollapsed => &TREE_COLLAPSED_CELLS,
            CellPage::TreeDrop | CellPage::TreeDropCollapsed | CellPage::TreeDropCurr => &TREE_DROP_CELLS,
            CellPage::TreeLines => &TREE_LINES_CELLS,
        }
    }

    /// The rows the table held in every cell on a page other than the tree pages and their List (:563-565 three; :1622 one).
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
        matches!(
            self,
            CellPage::Issue
                | CellPage::Wide
                | CellPage::Lines
                | CellPage::TreeIssue
                | CellPage::Tree
                | CellPage::TreeCollapsed
                | CellPage::TreeLines
        )
    }

    /// The rows of the table, in the key, that `cell` on the page had (`Cell::rows`): on a tree page and on the List over the
    /// tree's table; `None` elsewhere.
    pub(crate) fn tree_rows(self, cell: &Cell) -> Option<&'static [Row]> {
        if self.tree() || self == CellPage::TreeLines {
            // Every cell of these pages names its rows; one that does not would hold none.
            return Some(cell.rows.unwrap_or(&[]));
        }
        None
    }

    /// A tree page of TreeValidateFilterRefresh.al (assumption `tree-validate-view-refresh`).
    pub(crate) fn tree(self) -> bool {
        matches!(
            self,
            CellPage::TreeIssue
                | CellPage::Tree
                | CellPage::TreeBoth
                | CellPage::TreeCurr
                | CellPage::TreeCollapsed
                | CellPage::TreeDrop
                | CellPage::TreeDropCollapsed
                | CellPage::TreeDropCurr
        )
    }

    /// The entry the rule runs under on the page: the pages of TreeValidateFilterRefresh.al, the List 97727 too, run under
    /// `tree-validate-view-refresh`; the Lists without OnAfterGetCurrRecord of ListValidateFilterRefresh.al under
    /// `list-validate-view-refresh`; the other pages under `validate-update-false-rereads-row`.
    pub(crate) fn assumption(self) -> Assumption {
        match self {
            CellPage::Issue | CellPage::Wide | CellPage::Lines => Assumption::ListValidateViewRefresh,
            CellPage::Both | CellPage::Curr | CellPage::Card | CellPage::EditCard => {
                Assumption::ValidateUpdateFalseRereadsRow
            }
            CellPage::TreeIssue
            | CellPage::Tree
            | CellPage::TreeBoth
            | CellPage::TreeCurr
            | CellPage::TreeCollapsed
            | CellPage::TreeDrop
            | CellPage::TreeLines
            | CellPage::TreeDropCollapsed
            | CellPage::TreeDropCurr => Assumption::TreeValidateViewRefresh,
        }
    }

    /// Whether control read `m` with `args` on a field of type `ty` is one the read cells of the page measure. On page 97498
    /// each As* read is measured on its own type only, OptionCount and GetOption on the Option field only, and GetOption with
    /// index 1 and 3 only (ListValidateViewReads.al :496-:581); on another type BC raises `Invalid Conversion. Source: String,
    /// Destination: NavInteger.` (AsInteger on Text) or a CLR exception (GetOption on Decimal, GetOption(4)), which the
    /// neighbors Wide_AsIntegerOnText_BetweenSteps and the others measure. The reads of #1529 on the other pages take any
    /// field.
    pub(crate) fn read_fits(self, m: PageMethod, ty: &RtType, args: &[Value]) -> bool {
        if self != CellPage::Wide {
            return true;
        }
        match m {
            PageMethod::AsBoolean => matches!(ty, RtType::Boolean),
            PageMethod::AsDate => matches!(ty, RtType::Date),
            PageMethod::AsDateTime => matches!(ty, RtType::DateTime),
            PageMethod::AsDecimal => matches!(ty, RtType::Decimal),
            PageMethod::AsInteger => matches!(ty, RtType::Integer),
            PageMethod::AsTime => matches!(ty, RtType::Time),
            PageMethod::OptionCount => matches!(ty, RtType::Option(_)),
            PageMethod::GetOption => matches!(ty, RtType::Option(_)) && matches!(args, [Value::Integer(1 | 3)]),
            _ => false,
        }
    }

    /// The read-only calls the read cells measure on the page (`Step::Reads`): on a control bound to a field, and on the page.
    pub(crate) fn read_calls(self) -> (&'static [PageMethod], &'static [PageMethod]) {
        match self {
            CellPage::Wide => (&WIDE_CONTROL_READS, &WIDE_PAGE_READS),
            _ => (&CONTROL_READS, &PAGE_READS),
        }
    }
}

/// The read-only calls of corpus pull request 1529 (ListValidateFilterRefresh.al `ReadsLines` and the others), on a control
/// and on the page.
const CONTROL_READS: [PageMethod; 8] = [
    PageMethod::AssertEquals,
    PageMethod::Caption,
    PageMethod::Editable,
    PageMethod::Enabled,
    PageMethod::HideValue,
    PageMethod::ShowMandatory,
    PageMethod::ValidationErrorCount,
    PageMethod::Visible,
];
const PAGE_READS: [PageMethod; 3] = [
    PageMethod::Caption,
    PageMethod::Editable,
    PageMethod::ValidationErrorCount,
];

/// The read-only calls of corpus pull request 1535 on page 97498 (ListValidateViewReads.al `ReadsLines` blocks 1 and 2):
/// the As* reads, OptionCount and GetOption on a control, IsExpanded on the page. GetField by the Name control's id (block 3)
/// is checked by its id (`validate_view_reads`).
const WIDE_CONTROL_READS: [PageMethod; 8] = [
    PageMethod::AsBoolean,
    PageMethod::AsDate,
    PageMethod::AsDateTime,
    PageMethod::AsDecimal,
    PageMethod::AsInteger,
    PageMethod::AsTime,
    PageMethod::OptionCount,
    PageMethod::GetOption,
];
const WIDE_PAGE_READS: [PageMethod; 1] = [PageMethod::IsExpanded];

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
    /// `Expand(true)` or `Expand(false)` on the tree's row.
    Expand(bool),
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
    /// No call: the last SetValue raised the filtered-view error, and every later call on the page refuses.
    End,
}

/// One row of the table a cell on a tree page had, in the key: its Group Code and its Indentation.
pub(crate) type Row = (&'static str, i32);

/// The rows of the tree cells: A (G0) and B (G1) at the top, C (G1) under B (TreeValidateFilterRefresh.al:507 `Initialize`,
/// :514-:519).
const TREE_ROWS: [Row; 3] = [("G0", 0), ("G1", 0), ("G1", 1)];

/// The rows of the flat cells: A (G0), B (G1) and C (G2), none indented (:1167 `InitializeFlat`); the same groups in the key
/// over other entry numbers (:1436, :1448 `InitializeShape`).
const FLAT_ROWS: [Row; 3] = [("G0", 0), ("G1", 0), ("G2", 0)];

/// The steps of one cell and the calls after them.
#[derive(Clone, Copy)]
pub(crate) struct Cell {
    pub(crate) steps: &'static [Step],
    pub(crate) tail: Tail,
    /// On a tree page and on the List over the tree's table, the rows the table held (`CellPage::tree_rows`); `None` on the
    /// other pages, whose cells hold the number of rows only (`CellPage::table_rows`).
    pub(crate) rows: Option<&'static [Row]>,
}

use Change::{Drop, Keep, KeepText, Reset, Same};
use Ctl::{Name, Value as Val};
use Step::{Action, Expand, Fail, GoToKey, Last, Previous, Reads, Set};
use al_builtins::page_op::PageMethod;
use al_builtins::value::{RtType, Value};

const fn walk(steps: &'static [Step]) -> Cell {
    Cell {
        steps,
        tail: Tail::Walk,
        rows: None,
    }
}

const fn close(steps: &'static [Step]) -> Cell {
    Cell {
        steps,
        tail: Tail::Close,
        rows: None,
    }
}

const fn end(steps: &'static [Step]) -> Cell {
    Cell {
        steps,
        tail: Tail::End,
        rows: None,
    }
}

/// `cell` over rows none indented.
const fn flat(cell: Cell) -> Cell {
    over(&FLAT_ROWS, cell)
}

/// `cell` over `rows`.
const fn over(rows: &'static [Row], cell: Cell) -> Cell {
    Cell {
        rows: Some(rows),
        ..cell
    }
}

/// `walk(steps)` and `end(steps)` over the tree cells' rows.
const fn tree_walk(steps: &'static [Step]) -> Cell {
    over(&TREE_ROWS, walk(steps))
}

const fn tree_end(steps: &'static [Step]) -> Cell {
    over(&TREE_ROWS, end(steps))
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
        rows: None,
    },
    walk(&[Last, Set(Name, Upd::False, Keep, Some((2, 1)))]),
];

/// The cells on page 97498 of ListValidateViewReads.al (corpus pull request 1535): the issue's cell (:465) and its read
/// cells at each point (blocks 1 and 2, :496-:581).
const WIDE_CELLS: [Cell; 4] = [
    walk(&KEEP_F),
    walk(&KEEP_F_READS[0]),
    walk(&KEEP_F_READS[1]),
    walk(&KEEP_F_READS[2]),
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

/// The tree cells (TreeValidateFilterRefresh.al of corpus pull request 1539). The rows of the view are those of the tree, its
/// hidden child C too: SetRange on G1 keeps B (renamed C2) and C, the row first; no change keeps A, C2 and C, the row second.
const TREE_KEEP_F: [Step; 3] = name_then_value(Upd::False, Keep, 2, 0);
const TREE_KEEP_TEXT_F: [Step; 3] = name_then_value(Upd::False, KeepText, 2, 0);
const TREE_KEEP_T: [Step; 3] = name_then_value(Upd::True, Keep, 2, 0);
const TREE_SAME_F: [Step; 3] = name_then_value(Upd::False, Same, 3, 1);
const TREE_EXPANDED_F: [Step; 4] = [Last, Expand(true), TREE_KEEP_F[1], TREE_KEEP_F[2]];

/// The cells on the issue's tree page 97718 and the repro's test: Last, the SetValue on Name, then First, the walk and Close
/// (:761); the same with Expand(true) on B after Last (:777); the SetValue on the first row A, whose filter leaves it out of
/// the view of B and C (:1347).
const TREE_ISSUE_CELLS: [Cell; 3] = [
    tree_walk(&[Last, Set(Name, Upd::False, Keep, Some((2, 0)))]),
    tree_walk(&[Last, Expand(true), Set(Name, Upd::False, Keep, Some((2, 0)))]),
    tree_end(&[DROP_FROM_FIRST]),
];

/// The cell on page 97714 and the repro's test of issue 8846: the SetValue on the first row A, in a TryFunction (:1370) or
/// under asserterror (:1390); the same on pages 97678 and 97679 of TreeValidateFilterShapes.al (:968, :996).
const TREE_DROP_CELLS: [Cell; 1] = [tree_end(&[DROP_FROM_FIRST])];

/// The cells on page 97727, over the flat rows (`FLAT_ROWS`): the SetValue on Name from the first, the middle and
/// the last row, the filter leaving the row out; then the SetValue on Value, First, the walk and Close (:1246, :1264, :1282).
const TREE_LINES_CELLS: [Cell; 3] = [
    flat(walk(&[DROP_TO_FIRST, Set(Val, Upd::False, Same, Some((1, 0)))])),
    flat(walk(&[
        Last,
        Previous,
        DROP_TO_FIRST,
        Set(Val, Upd::False, Same, Some((1, 0))),
    ])),
    flat(walk(&DROP_F)), // :1282, and :1484 SetRange on G0 with B between
];

/// A SetValue on Name, Update(false), whose filter leaves the row out of a view of `rows`, the refresh settling on the first.
const DROP_TO_FIRST: Step = Set(Name, Upd::False, Drop, Some((1, 0)));

/// The cells on page 97720 (the line of each test). Then a filter that leaves the row out over the rows A (G0), B (G1) and C
/// (G2), none indented, from the first, the middle and the last row, the same groups over other entries in the key from the
/// first and the last row; from the middle row over the groups G0, G1 and G5; over the tree's rows, where Last stands on B and
/// SetRange on G0 leaves it out (:1146); and over a child in G0 before two top-level rows in G1, where Last stands on the
/// last of them and SetRange on G0 keeps only the child (:1460). Then the cells of TreeValidateFilterShapes.al (`SHAPE_CELLS`)
/// and the tree's first row dropped.
const TREE_CELLS: [Cell; 21] = [
    tree_walk(&TREE_KEEP_F),      // :794
    tree_walk(&TREE_KEEP_TEXT_F), // :812
    tree_walk(&TREE_KEEP_T),      // :830
    tree_walk(&[
        Last,
        Set(Name, Upd::None, Keep, None),
        Set(Val, Upd::False, Same, Some((2, 0))),
    ]), // :848
    tree_walk(&TREE_SAME_F),      // :866, and :884 (Reset of no filter)
    tree_walk(&TREE_EXPANDED_F),  // :902
    tree_walk(&[Last, Action(Upd::False, Keep), Set(Val, Upd::False, Same, Some((2, 0)))]), // :921
    tree_walk(&[Last, Set(Val, Upd::False, Same, Some((3, 1)))]), // :1129
    flat(tree_end(&[DROP_TO_FIRST])), // :1300, :1436
    flat(tree_end(&[Last, Previous, DROP_TO_FIRST])), // :1312
    flat(tree_end(&[Last, DROP_TO_FIRST])), // :1324, :1448, and :1472 SetRange on G0 with B between
    tree_end(&[Last, DROP_TO_FIRST]), // :1146
    over(
        &[("G0", 0), ("G1", 0), ("G5", 0)],
        tree_end(&[Last, Previous, DROP_TO_FIRST]),
    ), // :1424
    over(&[("G0", 1), ("G1", 0), ("G1", 0)], tree_end(&[Last, DROP_TO_FIRST])), // :1460
    SHAPE_CELLS[0],
    SHAPE_CELLS[1],
    SHAPE_CELLS[2],
    SHAPE_CELLS[3],
    SHAPE_CELLS[4],
    SHAPE_CELLS[5],
    tree_end(&[DROP_FROM_FIRST]), // TreeValidateFilterShapes.al:663
];

/// The cells of corpus pull request 1554, tests/al-language/pages/currpage/TreeValidateFilterShapes.al (open, head 070e6275),
/// on its copy of page 97720: Last, the SetValue on Name keeping the row, First, the walk and Close, over other rows. Last stands
/// on the last top-level row: A over A (G1), B (G1) and C (G2) indented 0, 1, 1 (:532) or 0, 1, 2 (:546), the view A and B;
/// B over the tree's rows with C indented 2 (:561); D over the tree's rows and D (G1) at the top (:584), the view B, C and D.
/// Over the tree's rows, Expand(false) after Last (:608), and Expand(true) on the first row right after the open (:632), before
/// Last.
const SHAPE_CELLS: [Cell; 6] = [
    over(&[("G1", 0), ("G1", 1), ("G2", 1)], tree_walk(&LAST_KEEP)),
    over(&[("G1", 0), ("G1", 1), ("G2", 2)], tree_walk(&LAST_KEEP)),
    over(&[("G0", 0), ("G1", 0), ("G1", 2)], tree_walk(&LAST_KEEP)),
    over(
        &[("G0", 0), ("G1", 0), ("G1", 1), ("G1", 0)],
        tree_walk(&[Last, Set(Name, Upd::False, Keep, Some((3, 2)))]),
    ),
    tree_walk(&[Last, Expand(false), LAST_KEEP[1]]),
    tree_walk(&[Expand(true), LAST_KEEP[0], LAST_KEEP[1]]),
];

/// Last, then a SetValue on Name, Update(false), whose filter keeps the row, the first of a view of two rows.
const LAST_KEEP: [Step; 2] = [Last, Set(Name, Upd::False, Keep, Some((2, 0)))];

/// A SetValue on the tree's first row A, Update(false), whose filter leaves it out of the view of B and C.
const DROP_FROM_FIRST: Step = Set(Name, Upd::False, Drop, Some((2, 0)));

/// The cells on page 97721, both row triggers (:939, :957, :975, :993; TreeValidateFilterShapes.al:696 the first row dropped).
const TREE_BOTH_CELLS: [Cell; 5] = [
    tree_walk(&TREE_KEEP_F),
    tree_walk(&TREE_KEEP_T),
    tree_walk(&TREE_SAME_F),
    tree_walk(&TREE_EXPANDED_F),
    tree_end(&[DROP_FROM_FIRST]),
];

/// The cells on page 97722, OnAfterGetCurrRecord only (:1012, :1030; TreeValidateFilterShapes.al:729 the first row dropped).
const TREE_CURR_CELLS: [Cell; 3] = [
    tree_walk(&TREE_KEEP_F),
    tree_walk(&TREE_SAME_F),
    tree_end(&[DROP_FROM_FIRST]),
];

/// The cells on page 97723, CollapseAll (:1048, :1066, :1084; TreeValidateFilterShapes.al:762 the first row dropped).
const TREE_COLLAPSED_CELLS: [Cell; 4] = [
    tree_walk(&TREE_KEEP_F),
    tree_walk(&TREE_SAME_F),
    tree_walk(&TREE_EXPANDED_F),
    tree_end(&[DROP_FROM_FIRST]),
];
