//! A journal page whose Post action emptied its view and left Rec, with `CurrPage.Update(false)`, on a primary key no row has
//! (the Base Application's codeunit 241 "Item Jnl.-Post" leaves `"Line No." := 1` there): what the list stands on afterwards,
//! with and without the new-row line (al-runner issues 6027, 6292, 7197, 7821).

use super::pinned_body::{Callees, Names, callees_are, page_reads_as, routine_text, table_is};
use super::pinned_shape::{NextEnd, PinnedEnd, Reads, TestCall};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The page of the reporter's repro (al-runner issue 7821, `Journal Post New Row Wks`), line for line (`pinned_body.rs`), under
/// any object and variable names: `«T»` its source table, `«L»` the log codeunit, `«V»` the log variable. The repro, the page
/// of open corpus PR 1333 (testpage/R7821JournalPostNewLine.al, page 67140) and page 67206 of the pinned
/// testpage/R7821JournalPostVariants.al:44 differ only in those names and give the same answer.
const NEW_LINE_PAGE: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "AutoSplitKey = true",
    "DelayedInsert = true",
    "repeater Rows in - {}",
    "field Line No. in Rows = Line No. {applicationarea=All}",
    "field Quantity in Rows = Quantity {applicationarea=All}",
    POST_ACTION,
    POST_TRIGGER,
    "area Processing: Post",
    ON_OPEN_PAGE,
    ON_AFTER_GET_RECORD,
    ON_NEW_RECORD,
    ON_AFTER_GET_CURR_RECORD,
];

/// The same page with the field "Batch Name" first in the repeater (testpage/R7821JournalPostVariants.al:124, page 67207
/// "TP R7821V Field Wks").
const BATCH_FIELD_PAGE: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "AutoSplitKey = true",
    "DelayedInsert = true",
    "repeater Rows in - {}",
    "field Batch Name in Rows = Batch Name {applicationarea=All}",
    "field Line No. in Rows = Line No. {applicationarea=All}",
    "field Quantity in Rows = Quantity {applicationarea=All}",
    POST_ACTION,
    POST_TRIGGER,
    "area Processing: Post",
    ON_OPEN_PAGE,
    ON_AFTER_GET_RECORD,
    ON_NEW_RECORD,
    ON_AFTER_GET_CURR_RECORD,
];

const POST_ACTION: &str = "action Post in - {applicationarea=All; caption='Post'}";
const POST_TRIGGER: &str = "  trigger onaction: var Posted: Record \"«T»\"; begin Posted.Copy(Rec); Rec.DeleteAll(); if (not \
     Posted.Find('=><')) then begin Posted.Reset(); Posted.FilterGroup(2); Posted.SetRange(field#0, 'B1'); \
     Posted.FilterGroup(0); Posted.\"Line No.\" := 1; end; Rec.Copy(Posted); CurrPage.Update(false); end";
const ON_OPEN_PAGE: &str =
    "trigger onopenpage: begin Rec.FilterGroup(2); Rec.SetRange(field#0, 'B1'); Rec.FilterGroup(0); end";
const ON_AFTER_GET_RECORD: &str =
    "trigger onaftergetrecord: var «V»: Codeunit \"«L»\"; begin «V».Add(('R:' + Format(Rec.\"Line No.\"))); end";
const ON_NEW_RECORD: &str =
    "trigger onnewrecord: var «V»: Codeunit \"«L»\"; begin «V».Add(('N:' + Format(Rec.\"Line No.\"))); end";
const ON_AFTER_GET_CURR_RECORD: &str =
    "trigger onaftergetcurrrecord: var «V»: Codeunit \"«L»\"; begin «V».Add(('C:' + Format(Rec.\"Line No.\"))); end";

/// The repro's table `Journal Post Line`.
const LINE_TABLE: &[&str] = &[
    "field 1 \"Batch Name\": Code[10]",
    "field 2 \"Line No.\": Integer",
    "field 3 \"Quantity\": Decimal",
    "key \"Batch Name\", \"Line No.\" {Clustered=true}",
];

/// The one procedure the page's triggers (and the logging subscribers) call: the log codeunit's Add.
const LOG_ADD: &[&str] = &["\"«L»\".Add: begin Entries := (Entries + (Entry + ';')); end"];

/// The bodies of the subscribers to OnBeforeActionEvent and OnAfterActionEvent of Post that the corpus file declares
/// (testpage/R7821JournalPostVariants.al:224 codeunit 67208 "TP R7821V Subs", counting; :250 codeunit 67209 "TP R7821V Log
/// Subs", logging), each in a codeunit with `EventSubscriberInstance = Manual`.
const COUNTING_SUBSCRIBER: &str = "begin Runs := (Runs + 1); end";
const LOGGING_BEFORE: &str = "var «V»: Codeunit \"«L»\"; begin «V».Add('BA'); end";
const LOGGING_AFTER: &str = "var «V»: Codeunit \"«L»\"; begin «V».Add('AA'); end";

/// The page text of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PageText {
    /// `NEW_LINE_PAGE`.
    Base,
    /// `BATCH_FIELD_PAGE`.
    BatchField,
}

/// The one row the table holds next to the page's batch when Post ran, besides none: Line No. 10000, Quantity 100, in batch
/// B2 or A1 (testpage/R7821JournalPostVariants.al:276 `InsertLines`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OtherRow {
    None,
    B2,
    A1,
}

/// The subscribers of the page's events and which of them are bound when Post runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Subscribers {
    /// No subscriber of any event of the page.
    None,
    /// The subscribers of the corpus file are declared (`COUNTING_SUBSCRIBER`, `LOGGING_BEFORE`, `LOGGING_AFTER`), none bound.
    Unbound,
    /// The same, and one instance of the counting codeunit is bound, no logging one.
    Counting,
    /// The same, and one instance of the logging codeunit is bound, no counting one.
    Logging,
}

/// How the page opened: the test's own OpenEdit (`PageState::test_open_edit`), or the static, non-modal `Page.Run(Page::X, Rec)`
/// caught by the TestPage's Trap (`PageRunCall::StaticWithRecord`). The record it was given is `PostStart`'s to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opened {
    OpenEdit,
    PageRun,
    Other,
}

/// One measured cell of testpage/R7821JournalPostVariants.al: the page, the rows of the view when Post starts (`PostStart`),
/// the other row, the subscribers and the open.
type Cell = (PageText, usize, OtherRow, Subscribers, Opened);

/// The cells the pinned testpage/R7821JournalPostVariants.al measures on BC 27.5, 28.5 and 29.0, each with the test that
/// measures it. Every one runs the blank line three times after OnAfterGetRecord of the deleted row, shows line 0 (and batch
/// B1), runs nothing on Close and leaves no row of batch B1. The cells of the base page need both subscriber codeunits declared
/// (`Subscribers::Unbound` when none is bound), as the corpus app declares them; the cells of the Batch Name page need none.
const VARIANT_CELLS: &[Cell] = &[
    // :372 TPV7821_Base_OneLine_PageRun
    (PageText::Base, 1, OtherRow::None, Subscribers::Unbound, Opened::PageRun),
    // :386 TPV7821_ExtraField_OneLine_PageRun, :401 _OpenEdit
    (
        PageText::BatchField,
        1,
        OtherRow::None,
        Subscribers::None,
        Opened::PageRun,
    ),
    (
        PageText::BatchField,
        1,
        OtherRow::None,
        Subscribers::None,
        Opened::OpenEdit,
    ),
    // :416 TPV7821_TwoLines_PageRun, :434 TPV7821_ThreeLines_PageRun (`extra_blank_pairs`)
    (PageText::Base, 2, OtherRow::None, Subscribers::Unbound, Opened::PageRun),
    (PageText::Base, 3, OtherRow::None, Subscribers::Unbound, Opened::PageRun),
    // :448 TPV7821_OtherBatchAfter_OneLine_PageRun, :462 TPV7821_OtherBatchBefore_OneLine_PageRun, :476 OtherBatchAfter _OpenEdit
    (PageText::Base, 1, OtherRow::B2, Subscribers::Unbound, Opened::PageRun),
    (PageText::Base, 1, OtherRow::A1, Subscribers::Unbound, Opened::PageRun),
    (PageText::Base, 1, OtherRow::B2, Subscribers::Unbound, Opened::OpenEdit),
    // :490 TPV7821_ActionSubs_OneLine_PageRun, :505 TPV7821_ActionSubsLogged_OneLine_PageRun (`R:10000;BA;AA;N:0;C:0;...`), :519
    // TPV7821_ActionSubs_OneLine_OpenEdit
    (
        PageText::Base,
        1,
        OtherRow::None,
        Subscribers::Counting,
        Opened::PageRun,
    ),
    (PageText::Base, 1, OtherRow::None, Subscribers::Logging, Opened::PageRun),
    (
        PageText::Base,
        1,
        OtherRow::None,
        Subscribers::Counting,
        Opened::OpenEdit,
    ),
];

/// Which pinned case the new-row line settle is (`new_line_cell`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewLineCell {
    /// The reporter's repro: `NEW_LINE_PAGE`, one row, no other row, no subscriber (assumption `post-left-other-key-on-new-line`).
    Repro,
    /// A cell of `VARIANT_CELLS` (pinned).
    Variant(Cell),
}

/// The page's Rec and its view when Post starts, before its body resets the filters (`new_line_post_start`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PostStart {
    /// The rows of the view: lines 10000, 20000, ... of batch B1, Quantity 300 (testpage/R7821JournalPostVariants.al:282-288).
    rows: usize,
    /// Rec holds the record's filter of Page.Run, batch B1 in filter group 0 (:296, :328), next to OnOpenPage's.
    run_filter: bool,
}

impl PageText {
    /// The fields of Rec, by position, whose `Value()` the corpus file reads after the Post: Line No. (:334, :364) and, on the
    /// Batch Name page, Batch Name (:365). `LINE_TABLE` fixes the positions.
    fn read_fields(self) -> &'static [usize] {
        match self {
            PageText::Base => &[1],
            PageText::BatchField => &[0, 1],
        }
    }
}

impl Session {
    /// Whether the list settled after an action that emptied its view and left another primary key, no stored row's, in Rec
    /// with `CurrPage.Update(false)` (`rec_moved_other` in `action_invoke_body`); `held` are the keys of the rows the list
    /// held before the action, `start` what `new_line_post_start` found then. `Some(n)`: the caller then runs OnNewRecord and
    /// OnAfterGetCurrRecord `n` times more.
    ///
    /// Without the new-row line the list stands on the blank Rec as when Rec keeps the deleted row's key
    /// (`settle_on_blank_rec`; pages/RtFilteredGoneRow.al:640 TD_LeavesKey_Line1WithUpdate_OneLine `R:10000;N:B1:0;C:B1:0;`,
    /// :700 _OtherBatchRowRemains; al-runner issue 6027). With two to six rows shown before the action BC 28 runs OnNewRecord
    /// and OnAfterGetCurrRecord once more, BC 27 and BC 29 twice (:656 TD_LeavesKey_Line1WithUpdate_ThreeLines, :679
    /// _TwoLines, :701 _FourLines, :724 _FiveLines, :747 _SixLines, `#if BC28PLUS and not BC29PLUS`; the reporter's page
    /// testpage/Y1WorksheetPostLog.al:161 Y1_Post_TwoLines, :177 Y1_Post_ThreeLines; al-runner issues 6292, 7197): the settle
    /// runs the first, the caller the second.
    ///
    /// With the new-row line, filtered to one batch in filter group 2, the list runs its blank line three times as when the
    /// action only deleted the row (`settle_after_action`, misc/lt-probes-needed-7/PnGRefresh.al:702): `R:10000;N:0;C:0;N:0;C:0;
    /// N:0;C:0;` after one row (al-runner issue 7821). Only in the cells of `new_line_cell`: the reporter's repro (assumption
    /// `post-left-other-key-on-new-line`; the repro then only closes the page, so every other later call is refused) and the
    /// cells of `VARIANT_CELLS` (testpage/R7821JournalPostVariants.al; the corpus file then reads the shown line, and the batch
    /// on the Batch Name page, by `Value()` and closes the page).
    ///
    /// Rows left in the view, seven or more rows shown before, or any other shape with the new-row line are not pinned
    /// (`None`: the caller refuses).
    pub(super) fn post_left_other_key_settled(
        &mut self,
        st: &St,
        held: Option<&[Vec<Value>]>,
        start: Option<PostStart>,
    ) -> Result<Option<usize>, Signal> {
        let rows_before = held.map_or(0, <[_]>::len);
        let on_blank = (1..=6).contains(&rows_before) && {
            let s = st.borrow();
            s.gone_blank_no_new_line && s.open && s.cursor == Cursor::None && !s.draft
        };
        if on_blank {
            return Ok(Some(usize::from(rows_before > 1 && !self.blank_pair_once_on_28())));
        }
        let Some(cell) = self.new_line_cell(st, rows_before, start)? else {
            return Ok(None);
        };
        let (reads, reason) = match cell {
            NewLineCell::Repro => {
                assume(Assumption::PostLeftOtherKeyOnNewLine);
                (
                    Reads::No,
                    "a call of the test after the Post of a journal that left another primary key in Rec on its new-row line, \
                     other than Close (not pinned)",
                )
            }
            NewLineCell::Variant((page, ..)) => (
                Reads::Values(page.read_fields()),
                "a call of the test after the Post of a journal that left another primary key in Rec on its new-row line, other \
                 than a value read of the line or the batch and Close (not pinned)",
            ),
        };
        st.borrow_mut().pinned_end = Some(PinnedEnd {
            reason,
            reads,
            caption: false,
            part_reads: false,
            one_action: false,
            first_ends: false,
            part_visible: false,
            part_moves: false,
            no_writes: false,
            next: NextEnd::Refused,
            close: true,
            ok: false,
            walk: false,
        });
        Ok(Some(match cell {
            NewLineCell::Variant((_, rows, ..)) => self.extra_blank_pairs(rows),
            NewLineCell::Repro => 0,
        }))
    }

    /// BC 28 runs the blank line once more where BC 27 and BC 29 run it twice (`Gate::ThreeLineLeavesKeyOnceFrom28`).
    fn blank_pair_once_on_28(&self) -> bool {
        self.has(al_version::Gate::ThreeLineLeavesKeyOnceFrom28)
            && !self.has(al_version::Gate::ThreeLineLeavesKeyTwiceAgainFrom29)
    }

    /// The blank-line pairs the new-row line runs after its three when the view held `rows` rows when Post started
    /// (testpage/R7821JournalPostVariants.al:416 TPV7821_TwoLines_PageRun: five pairs in all on BC 27.5 and 29.0, four on BC
    /// 28.5, `#if BC28PLUS and not BC29PLUS`; :434 TPV7821_ThreeLines_PageRun: five on all three; one row: three).
    fn extra_blank_pairs(&self, rows: usize) -> usize {
        match rows {
            2 if self.blank_pair_once_on_28() => 1,
            2 | 3 => 2,
            _ => 0,
        }
    }

    /// The page's Rec and its view as every variant cell has them when an action of list `st` starts, `None` otherwise; `held`
    /// are the keys of the rows the list holds then. Required, each by equality:
    /// - Rec is a stored record (:313 `Line: Record`, not temporary), not restricted to marks;
    /// - its filters: batch B1 in filter group 2 (OnOpenPage, :94-99), and for a Page.Run cell batch B1 in filter group 0 (the
    ///   record's `SetRange`, :296), nothing else;
    /// - the view holds one to three rows, the lines i * 10000 of batch B1 with Quantity 300 (:282-288);
    /// - the list holds them all and Rec stands on the first (:297 `FindFirst`; OpenEdit opens on it).
    pub(super) fn new_line_post_start(
        &mut self,
        st: &St,
        held: Option<&[Vec<Value>]>,
    ) -> Result<Option<PostStart>, Signal> {
        let Some(held) = held else { return Ok(None) };
        let rec = self.page_rec(st);
        let run_filter = {
            let r = rec.borrow();
            match (r.temporary, r.marked_only, r.filters.as_slice()) {
                (false, false, [f]) if batch_b1_in(f, 2) => false,
                (false, false, [a, b]) if [(a, 0), (b, 2)].iter().all(|(f, g)| batch_b1_in(f, *g)) => true,
                (false, false, [a, b]) if [(a, 2), (b, 0)].iter().all(|(f, g)| batch_b1_in(f, *g)) => true,
                _ => return Ok(None),
            }
        };
        let rows = self.view_rows(&rec, 4)?;
        let lines = (1..=3).contains(&rows.len()) && rows.iter().zip(1..).all(|(row, i)| line_row(row, i * 10000));
        let on_first = rows.first().is_some_and(|row| same_key(&rec.borrow().fields, row));
        let held_all = held.len() == rows.len() && held.iter().zip(&rows).all(|(key, row)| same_key(key, row));
        Ok((lines && held_all && on_first).then_some(PostStart {
            rows: rows.len(),
            run_filter,
        }))
    }

    /// The pinned case of the new-row line, `None` for any other shape. Facts every case requires:
    /// - the page, its table and the procedure its triggers call read as `NEW_LINE_PAGE` or `BATCH_FIELD_PAGE`, `LINE_TABLE`
    ///   and `LOG_ADD`, with no page or table extension, and nothing subscribes to the table or the log codeunit;
    /// - the page: top-level, opened for edit, not modal, not a lookup;
    /// - the test's calls since the open: only this Invoke of Post;
    /// - the refresh just ran the blank line three times (`settle_after_action`) and the page stands on it;
    /// - the table holds no row, or one row of `OtherRow`.
    ///
    /// The repro: `NEW_LINE_PAGE`, one row shown before, no other row, no subscriber of the page (with main's open: by OpenEdit,
    /// or by Page.Run on the trapped TestPage). Any other case must be a cell of `VARIANT_CELLS`, with its open by equality, and
    /// Post must have started as `start` says: its rows are the cell's, and Rec holds the record's filter exactly when the
    /// cell's open is Page.Run.
    fn new_line_cell(
        &mut self,
        st: &St,
        rows_before: usize,
        start: Option<PostStart>,
    ) -> Result<Option<NewLineCell>, Signal> {
        let open = {
            let s = st.borrow();
            let on_new_line = s.gone_new_line_settled
                && s.open
                && s.cursor == Cursor::NewLine
                && s.placeholder_line
                && s.mode == PageMode::Edit
                && !s.modal
                && !s.lookup
                && s.host.is_none();
            if !on_new_line {
                return Ok(None);
            }
            if s.test_open_edit() {
                Opened::OpenEdit
            } else if s.caught_by_trap && s.trapped_run == Some(super::page_run_close::PageRunCall::StaticWithRecord) {
                Opened::PageRun
            } else {
                Opened::Other
            }
        };
        let calls_ok = matches!(
            Self::test_calls(st).as_deref(),
            Some([TestCall::Other(PageMethod::Invoke)])
        );
        if !calls_ok {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        let object = st.borrow().object as usize;
        let Some((page, subscribers)) = self.new_line_code(object, table) else {
            return Ok(None);
        };
        let all_rows = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let mut a = all_rows.borrow_mut();
            a.filters.clear();
            a.marked_only = false;
        }
        let other = match self.view_rows(&all_rows, 2)?.as_slice() {
            [] => OtherRow::None,
            [row] => match other_row(row) {
                Some(other) => other,
                None => return Ok(None),
            },
            _ => return Ok(None),
        };
        if (page, rows_before, other, subscribers) == (PageText::Base, 1, OtherRow::None, Subscribers::None) {
            return Ok(Some(NewLineCell::Repro));
        }
        let Some(start) = start else { return Ok(None) };
        let cell = (page, start.rows, other, subscribers, open);
        let start_ok = start.run_filter == (open == Opened::PageRun);
        Ok((start_ok && VARIANT_CELLS.contains(&cell)).then_some(NewLineCell::Variant(cell)))
    }

    /// The page text and the subscriber state of page `object` over table `table`, `None` when the code is not a pinned one.
    fn new_line_code(&mut self, object: usize, table: usize) -> Option<(PageText, Subscribers)> {
        let engine = self.engine();
        let mut callees = Callees::new();
        let mut names = Names::default();
        let page = if page_reads_as(&engine.program, object, NEW_LINE_PAGE, &mut callees, &mut names) {
            PageText::Base
        } else if page_reads_as(&engine.program, object, BATCH_FIELD_PAGE, &mut callees, &mut names) {
            PageText::BatchField
        } else {
            return None;
        };
        let declared = if engine.events.any_subscriber_of(object) {
            Some(declared_subscribers(&engine, object, &mut callees, &mut names)?)
        } else {
            None
        };
        let code_ok = table_is(&engine, table, LINE_TABLE, &mut callees, &mut names)
            && callees_are(&engine, &callees, LOG_ADD, &mut names);
        drop(engine);
        if !code_ok {
            return None;
        }
        let Some((counting, logging)) = declared else {
            return Some((page, Subscribers::None));
        };
        let subscribers = match (
            self.bound_instances(counting).len(),
            self.bound_instances(logging).len(),
        ) {
            (0, 0) => Subscribers::Unbound,
            (1, 0) => Subscribers::Counting,
            (0, 1) => Subscribers::Logging,
            _ => return None,
        };
        Some((page, subscribers))
    }
}

/// The two codeunits (counting, logging) when the subscribers of page `object` are exactly the ones corpus PR 1384 declares:
/// OnBeforeActionEvent and OnAfterActionEvent of Post, each with one subscriber of a counting codeunit
/// (`COUNTING_SUBSCRIBER`) and one of another, logging codeunit (`LOGGING_BEFORE`, `LOGGING_AFTER`), both manual, no
/// parameter, no other property on either codeunit (`only_manual_instance`), and no subscriber declared but not compiled
/// (testpage/R7821JournalPostVariants.al:224-269).
fn declared_subscribers(
    engine: &crate::Engine,
    object: usize,
    callees: &mut Callees,
    names: &mut Names,
) -> Option<(usize, usize)> {
    let reg = &engine.events;
    if reg.missing.keys().any(|(o, _)| *o == object) {
        return None;
    }
    let mut events: Vec<&str> = reg
        .subscribers
        .iter()
        .filter(|((o, _), v)| *o == object && !v.is_empty())
        .map(|((_, e), _)| e.as_str())
        .collect();
    events.sort_unstable();
    if events != ["onafteractionevent", "onbeforeactionevent"] {
        return None;
    }
    let mut found: Option<(usize, usize)> = None;
    for (event, logging_body) in [
        ("onbeforeactionevent", LOGGING_BEFORE),
        ("onafteractionevent", LOGGING_AFTER),
    ] {
        let subs = reg.subscribers.get(&(object, event.to_string()))?;
        let [a, b] = subs.as_slice() else { return None };
        let mut counting = None;
        let mut logging = None;
        for sub in [a, b] {
            if !sub.manual
                || !only_manual_instance(&engine.program.objects[sub.object])
                || sub.element != "Post"
                || engine.program.objects[sub.object].routines[sub.routine].params != 0
            {
                return None;
            }
            let mut inner = Callees::new();
            let body = routine_text(&engine.program, sub.object, sub.routine, &mut inner)?;
            if body == COUNTING_SUBSCRIBER && inner.is_empty() && counting.is_none() {
                counting = Some(sub.object);
            } else if logging.is_none() && names.line_is(&body, logging_body) {
                callees.extend(inner);
                logging = Some(sub.object);
            } else {
                return None;
            }
        }
        let pair = (counting?, logging?);
        if pair.0 == pair.1 || found.is_some_and(|f| f != pair) {
            return None;
        }
        found = Some(pair);
    }
    found
}

/// Whether codeunit `o` declares one property, `EventSubscriberInstance = Manual`, as both subscriber codeunits of the corpus
/// file do (testpage/R7821JournalPostVariants.al:226, :252).
fn only_manual_instance(o: &ir::Object) -> bool {
    matches!(o.properties.as_slice(), [(k, v)] if k.eq_ignore_ascii_case("EventSubscriberInstance") && v.eq_ignore_ascii_case("Manual"))
}

/// The other row of a cell: batch B2 or A1, Line No. 10000, Quantity 100 (testpage/R7821JournalPostVariants.al:276).
fn other_row(row: &[Value]) -> Option<OtherRow> {
    let [Value::Code(batch), Value::Integer(10000), Value::Decimal(quantity), ..] = row else {
        return None;
    };
    if *quantity != rust_decimal::Decimal::from(100) {
        return None;
    }
    match batch.as_str() {
        "B2" => Some(OtherRow::B2),
        "A1" => Some(OtherRow::A1),
        _ => None,
    }
}

/// Whether `f` is `SetRange("Batch Name", 'B1')` in filter group `group` (testpage/R7821JournalPostVariants.al:97, :296).
fn batch_b1_in(f: &al_builtins::filter::FieldFilter, group: i32) -> bool {
    use al_builtins::filter::FilterExpr;
    let b1 = |v: &Value| matches!(v, Value::Code(c) if c.as_str() == "B1");
    (f.field, f.group) == (0, group) && matches!(&f.expr, FilterExpr::Range(Some(a), Some(b)) if b1(a) && b1(b))
}

/// Whether `row` is line `line` of batch B1 with Quantity 300 (testpage/R7821JournalPostVariants.al:284-286).
fn line_row(row: &[Value], line: i32) -> bool {
    matches!(row, [Value::Code(batch), Value::Integer(l), Value::Decimal(q), ..]
        if batch.as_str() == "B1" && *l == line && *q == rust_decimal::Decimal::from(300))
}

/// Whether `a` and `b` start with the same primary key ("Batch Name", "Line No.", `LINE_TABLE`), values by `Value::identical`.
fn same_key(a: &[Value], b: &[Value]) -> bool {
    matches!((a.get(..2), b.get(..2)), (Some(a), Some(b)) if a.iter().zip(b).all(|(x, y)| x.identical(y)))
}
