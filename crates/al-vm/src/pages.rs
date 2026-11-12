//! The test client for pages (ADR-012): the part of BC's test framework that
//! opens a page, moves on its rows, reads and writes its controls and closes
//! it again. The page itself runs as ordinary AL (triggers, procedures,
//! property routines) on an `Instance` in this session.
//!
//! Every rule below cites the corpus test that pins it; behaviour no test
//! pins raises `Signal::Unsupported`.

use crate::{Engine, Session, UiKind};
use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal, unsupported};
use al_builtins::ops;
use al_builtins::pagefmt::{self, Blank, ControlFormat};
use al_builtins::value::{RecordState, RtType, Value};
use al_data::Store as _;
use al_ir as ir;
use rust_decimal::prelude::ToPrimitive;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

type St = Rc<RefCell<PageState>>;
type Rec = Rc<RefCell<RecordState>>;

/// corpus testpage/lifecycle/TestPageTeardown.al:227, testpage/lifecycle/TestPageClosingActions.al:338
const NOT_OPEN: &str = "The TestPage is not open.";
/// corpus testpage/lifecycle/TestPageClosingActions.al:339
const ALREADY_OPEN: &str = "The TestPage is already open.";

enum Left {
    Go,
    Stay,
}

fn error(message: impl Into<String>) -> Signal {
    Signal::Error(Box::new(AlError {
        message: message.into(),
        code: String::new(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// Whether `name` (a page's name or id as a table property writes it) is a page of a dependency app
/// that was not compiled.
fn names_uncompiled_page(engine: &Engine, name: &str) -> bool {
    engine.program.objects.heads().iter().any(|o| {
        o.is_shell_of(al_ir::ObjKind::Page)
            && (o.name.eq_ignore_ascii_case(name) || o.id.is_some_and(|i| i.to_string() == name))
    })
}

fn error_code(message: impl Into<String>, code: &str) -> Signal {
    Signal::Error(Box::new(AlError {
        message: message.into(),
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// The id a TestPage error names a control by: FNV-1a over the UTF-16 text of the declaring
/// object's id followed by the control name, read as a signed 32-bit number and made positive.
/// It reproduces every id the corpus pins: part `Hidden` of page 60344 (1318487454), `Name` of
/// page 329 (301776846), `User Defined` of page 5361 (104384132) and the extension control
/// `Ignore Status on Service Docs.` of page extension 6478 (1790009997).
pub(crate) fn control_id(object_id: u32, name: &str) -> i64 {
    request::action_id(object_id, name)
}

/// A host move whose parts are reading again (`Session::host_move_running`, set in `pageops.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HostMove {
    pub(crate) method: al_builtins::page_op::PageMethod,
    /// The move stayed on the row and its parts read again through `sync_parts_stayed` (a GoToKey or GoToRecord to the row the
    /// host stands on). False for every other move, a GoToRecord the `pageops.rs` last branch syncs included.
    pub(crate) stayed: bool,
}

impl HostMove {
    /// A move whose parts read again outside `sync_parts_stayed`.
    pub(crate) fn moved(method: al_builtins::page_op::PageMethod) -> Self {
        HostMove { method, stayed: false }
    }

    /// A GoToKey or GoToRecord that stayed on the row; its parts read again through `sync_parts_stayed`.
    pub(crate) fn stayed(method: al_builtins::page_op::PageMethod) -> Self {
        HostMove { method, stayed: true }
    }
}

fn page_of<'a>(engine: &'a Engine, st: &St) -> &'a ir::Page {
    let object = st.borrow().object as usize;
    engine.program.objects[object]
        .page
        .as_ref()
        .expect("a TestPage is typed by a page object")
}

/// A property of a page as the page and its extensions leave it: an extension's value wins over the base
/// page's (corpus pages/extensions/ZPageExtProps.al:629, 685-1110). Two extensions that set the
/// same property are not pinned. The value is trimmed.
fn effective_prop(engine: &Engine, object: usize, name: &str) -> Result<Option<String>, Signal> {
    let prop = |o: usize| {
        engine.program.objects[o]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_string())
    };
    let exts: Vec<String> = engine.program.objects[object]
        .page
        .as_ref()
        .map(|p| p.extensions.iter().filter_map(|&e| prop(e)).collect())
        .unwrap_or_default();
    match exts.len() {
        0 => Ok(prop(object)),
        1 => Ok(exts.into_iter().next()),
        _ => unsupported(format!(
            "several page extensions set {name} (which one wins is not pinned)"
        )),
    }
}

/// Whether the page declares InsertAllowed or ModifyAllowed as false, extensions counted
/// (ZInsertAllowed_1 to _4, ZModifyAllowed_1 to _4 in the same file). An extension's value other than a
/// literal true or false is not pinned.
fn flag_denied(engine: &Engine, object: usize, name: &str) -> Result<bool, Signal> {
    let in_extension = engine.program.objects[object].page.as_ref().is_some_and(|p| {
        p.extensions.iter().any(|&e| {
            engine.program.objects[e]
                .properties
                .iter()
                .any(|(k, _)| k.eq_ignore_ascii_case(name))
        })
    });
    match effective_prop(engine, object, name)?.map(|v| v.to_ascii_lowercase()) {
        None => Ok(false),
        Some(v) if v == "false" => Ok(true),
        Some(v) if v == "true" || !in_extension => Ok(false),
        Some(_) => unsupported(format!(
            "a page extension sets {name} to something other than a literal true or false (not pinned)"
        )),
    }
}

/// `'text'` → `text`; anything else as written.
pub(crate) fn unquote(s: &str) -> String {
    let t = s.trim();
    match t.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
        Some(inner) => inner.replace("''", "'"),
        None => t.to_string(),
    }
}

/// Whether a property value as written is one Text literal (`'..'`, with `''` for a quote inside), not an expression.
pub(crate) fn is_text_literal(s: &str) -> bool {
    let t = s.trim();
    t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') && !t[1..t.len() - 1].replace("''", "").contains('\'')
}

fn is_true(s: &str) -> bool {
    s.trim().eq_ignore_ascii_case("true")
}

/// A boolean control property as written: `true` or `false`. Anything else is an expression the compile keeps as text
/// (only Visible, Editable, Enabled and HideValue are evaluated, `al_sema::pages::dynamic_type`); reading it as false would
/// be silently wrong (pages/properties/P119LabelHideValue.al:74: `HideValue = <variable>` hides the
/// value), so it is refused.
fn lit_bool(name: &str, s: &str) -> Result<bool, Signal> {
    let t = unquote(s);
    if t.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if t.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        unsupported(format!(
            "control property {name} = {}: an expression the test client does not evaluate",
            s.trim()
        ))
    }
}

type Handler = (usize, usize, Rc<RefCell<al_builtins::value::Instance>>);

mod areas;
mod background;
mod end_move_no_read;
mod flowfield_hidden;
pub(crate) use background::BgWorker;
mod choice_entry;
mod document_handler_error;
mod filter;
mod filter_enum;
mod filter_link_numbers;
mod modal_handler_error;
mod page_run_close;
pub(crate) use page_run_close::PageRunCall;
mod param_read;
mod part_tree_typed;
mod part_unsettled;
mod parts;
mod qualified_name;
mod read_backwards;
mod relation_lookup;
pub(crate) mod relation_lookup_close;
mod row_var_outside_call;
mod split_key_view;
mod text_control_value;
use text_control_value::TextConv;
pub(crate) mod request;
mod request_expr_params;
pub(crate) mod request_options;
pub(crate) use request::{ParamsCall, RequestClosed};
mod access_by_permission;
mod action_blob_change;
mod actions;
mod blank_key_retyped;
mod caption_class;
mod caption_expr_list;
mod caption_relation;
mod caption_runs;
mod caption_translation;
mod clear_test_page;
mod close_save_failed;
mod code_close_veto;
mod modal_close_save_error;
mod triggerless_action;
pub(crate) use code_close_veto::{CodeCloseVeto, TriggerKind};
mod controls;
mod currpage_run_typed_key;
mod data_caption_run;
mod dev_endpoint;
mod dirty_step;
mod draft;
mod empty_after_action;
mod empty_view_update;
mod ext_column_visible;
mod fn_controls;
mod format;
mod format_type11;
mod gone_after_action;
mod gone_new_line;
mod gone_read_only_list;
mod gotokey_find;
mod held_page_part;
mod host_follows;
mod inserted_rows;
mod leave_new_line;
mod list_refresh;
mod list_view_switch;
mod long_view;
mod lookup_editable;
mod new_document_insert;
mod rendered_field;
use lookup_editable::LookupLock;
mod lookup_next;
mod lookup_refresh;
mod lookup_start;
mod lookup_trigger;
mod marked_refill_goto;
mod modal_answer;
mod modify_denied;
pub(crate) mod modify_record_xrec;
mod moved_onto_deleted;
mod never_run_update;
mod next_held_row_read;
mod no_source_events;
use no_source_events::NoSourceNew;
mod open;
pub(crate) mod pinned_shape;
mod ui_caption_translate;
mod update_refresh_card;
pub(crate) mod validate_xrec;
mod validation_text;
pub(crate) use open::written_by_routine;
mod action_body;
mod card_part_new_host;
mod empty_part_rec;
mod empty_part_rec_list;
mod list_card_action;
mod other_record_no_row;
mod page_get_record;
mod page_var_modeless_temp;
mod page_var_view;
mod pageops;
mod part_filter_find;
mod part_filter_unsaved_host;
mod part_host_update;
mod part_insert_swallowed;
mod part_refresh_find_forward;
mod part_visible_gate;
mod pinned_body;
mod post_left_new_line;
mod provider_hidden;
mod refill;
mod refresh_new_line;
mod rows;
mod rows_above;
mod rows_client;
mod runobject;
mod server_row_lag;
mod shown;
pub(crate) mod state;
mod table_values_allowed;
mod temp_list_refill;
mod test_end;
mod typed_handler_temp;
pub(crate) use shown::ShownNote;
pub(crate) use state::{
    ClosedBy, ControlRef, ControlTarget, Cursor, LookupKind, PageMode, PageState, ReadBackwards, WindowRow, control_of,
    into_st, page_value, st_of, test_control,
};

impl Session {
    /// `TP.Ctl`: a handle on control `control` of the test page `page`.
    pub(crate) fn page_control(&mut self, page: Value, control: usize, runtime: bool) -> Result<Value, Signal> {
        match page {
            Value::TestPage(p) => {
                let p = into_st(p);
                // A part control reads as the page it shows (a `TestPart`).
                let is_part = {
                    let engine = self.engine();
                    page_of(&engine, &p).controls[control].kind == ir::ControlKind::Part
                };
                if is_part {
                    return self.part_page(&p, control, runtime).map(|part| page_value(&part));
                }
                Ok(test_control(p, ControlTarget::Control(control)))
            }
            other => Err(Signal::unsupported(format!("control of {}", other.describe()))),
        }
    }

    fn engine(&self) -> Arc<Engine> {
        self.engine.clone()
    }

    /// The page's `Rec`.
    fn page_rec(&self, st: &St) -> Rec {
        let object = st.borrow().object as usize;
        let slot = self.engine.program.objects[object]
            .rec_slot
            .expect("page with a source table");
        let inst = st.borrow().instance.clone().expect("open page");
        let v = inst.borrow().globals[slot].clone();
        match v {
            Value::Record(r) => r,
            other => panic!("Rec is {}", other.describe()),
        }
    }

    fn require_open(&self, st: &St) -> Result<(), Signal> {
        if st.borrow().open {
            Ok(())
        } else {
            Err(error_code(NOT_OPEN, "TestPageNotOpened"))
        }
    }

    /// Runs `f` and names the trigger of `st`'s page (lower case) whose AL error `f` failed with, if it was one.
    fn traced<T>(
        &mut self,
        st: &St,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> (Result<T, Signal>, Option<String>) {
        self.pages.failed_trigger = None;
        let r = f(self);
        let object = st.borrow().object;
        let origin = match (&r, self.pages.failed_trigger.take()) {
            (Err(Signal::Error(_)), Some((o, name))) if o == object => Some(name),
            _ => None,
        };
        (r, origin)
    }

    /// A mark that changes with every insert, modify and delete the store takes (SystemRowVersion and the delete count).
    fn write_mark(&self) -> (i64, u64) {
        (self.next_row_version, self.deletes)
    }

    /// Whether the page is of type Card.
    fn is_card(&self, st: &St) -> bool {
        matches!(
            self.engine.object_facts[st.borrow().object as usize].page_type,
            Some(al_ir::PageType::Card)
        )
    }

    /// The error the client raises when a page trigger's AL error forces the page to close: "Page <caption> has to close."
    /// with the code FormAbort:Dialog (corpus testpage/CcPageLeads.al:525, :538, :551 on open, :564 with the
    /// key on Next). Pinned for an AL `Error` (code Dialog) on a page with no caption of its own, where the caption is the
    /// page's name; any other error, or a page with a caption, raises an error whose text and code are not pinned.
    fn has_to_close(&self, st: &St, e: &AlError, caption: String) -> Signal {
        let object = st.borrow().object as usize;
        let named = self.page_caption(st) == self.engine.program.objects[object].name;
        if e.code != "Dialog" || !named {
            return Signal::Error(Box::new(al_builtins::native::unpinned_error()));
        }
        Signal::Error(Box::new(AlError {
            message: format!("Page {caption} has to close."),
            code: "FormAbort:Dialog".into(),
            call_stack: e.call_stack.clone(),
            ..Default::default()
        }))
    }

    /// Runs page trigger `name` (lower case) if the page declares it, then the same trigger of
    /// each page extension that declares it. The page's own trigger runs first (assumed, see
    /// docs/probes-needed.md "Page extensions"); the lowering refuses two extensions that
    /// declare one trigger and extension triggers that answer a Boolean. The answer returned
    /// is the page's own.
    fn page_trigger(&mut self, st: &St, name: &str, args: Vec<Value>) -> Result<Option<Value>, Signal> {
        self.data_caption_before_trigger(st, name)?;
        let r = self.in_page_trigger(st, name, |s| s.page_trigger_run(st, name, args))?;
        self.data_caption_after_trigger(st, name);
        self.show_ext_columns_after_first_open_row(st, name)?;
        Ok(r)
    }

    fn page_trigger_run(&mut self, st: &St, name: &str, args: Vec<Value>) -> Result<Option<Value>, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        // The request page of a report never runs OnAfterGetRecord, whatever the report's data items do
        // (reports/request-page/RequestPageOnAfterGetRecord.al: no `rp OnAfterGetRecord` in any of the three runs).
        // With a source table it runs, before OnAfterGetCurrRecord (testpage/ABTests.al:1195 ReqPageBaseReportOk,
        // reports/request-page/Y80ReportExtRequestPage.al Y80_RX_ReadOnly_OK `handler start`).
        if name == "onaftergetrecord"
            && engine.program.objects[object].is_kind(al_ir::ObjKind::Report)
            && !self.has_source(st)
        {
            return Ok(None);
        }
        let mut answer = None;
        let mut result = Ok(());
        let declared =
            page.triggers.iter().any(|(n, _)| n == name) || page.ext_triggers.iter().any(|(n, _, _)| n == name);
        if declared {
            st.borrow_mut().triggers_run += 1;
        }
        // In OnAfterGetRecord xRec is the row the page fetched before (a blank row first); in OnAfterGetCurrRecord on a stored
        // row it is the row OnAfterGetCurrRecord ran with before (a blank row with the part link first). Every other trigger
        // keeps the xRec the page holds (testpage/parts/QiHostSaveParts.al:361-362, :411-412, :580).
        // On a new row OnAfterGetCurrRecord sees the xRec the page holds, the new row as it was set up (misc/lt-probes-needed-7/
        // PnGNewRecord.al:94 HandedRecordNoFilters `List.OnNewRecord;List.AGCR(,)` after `List.AGCR(N1,)`; PnGShowRow.al
        // ListLosesOnlyRowDeleteNoUpdate `List.OnNewRecord;List.AGCR(,)` after `List.AGR(U1,Upd 1)`), and so does the next one on a
        // stored row (`agcr_after_new`).
        let stored_row = {
            let s = st.borrow();
            !s.draft && s.cursor == Cursor::Row
        };
        let fetch =
            (name == "onaftergetrecord" || (name == "onaftergetcurrrecord" && stored_row)) && self.has_source(st);
        let after_new = if name == "onaftergetcurrrecord" && self.has_source(st) {
            let mut s = st.borrow_mut();
            let was = s.agcr_after_new;
            s.agcr_after_new = !stored_row;
            was
        } else {
            false
        };
        let held_xrec = if fetch && declared && !after_new {
            let before = {
                let s = st.borrow();
                if name == "onaftergetrecord" {
                    s.agr_xrec.clone()
                } else {
                    s.agcr_xrec.clone()
                }
            };
            let before = match before {
                Some(f) => f,
                None => self.blank_fetch_xrec(st, name == "onaftergetcurrrecord")?,
            };
            let held = self.page_xrec_fields(st);
            self.set_xrec_fields(st, before);
            Some(held)
        } else {
            None
        };
        if declared && name == "onaftergetcurrrecord" && stored_row && self.has_source(st) {
            let started = self.page_rec(st).borrow().fields.clone();
            st.borrow_mut().agcr_started = Some(started);
        }
        // OnAfterGetCurrRecord changes what the current row shows only for the variables it sets.
        let vars_before = (declared && name == "onaftergetcurrrecord" && stored_row).then(|| self.row_var_values(st));
        if let Some(r) = page.triggers.iter().find(|(n, _)| n == name).map(|(_, r)| *r) {
            let inst = st.borrow().instance.clone().expect("open page");
            match self.page_call(st, object, r, inst, args.clone()) {
                Ok((v, _)) => answer = Some(v),
                Err(e) => {
                    if matches!(e, Signal::Error(_)) {
                        self.pages.failed_trigger = Some((object as u32, name.to_string()));
                    }
                    result = Err(e)
                }
            }
        }
        self.pages.ext_answers.clear();
        if result.is_ok() {
            for (n, e, r) in &page.ext_triggers {
                if n == name {
                    let inst = self.owner_instance(st, Some(*e));
                    match self.page_call(st, *e, *r, inst, args.clone()) {
                        Ok((Value::Boolean(b), _)) if has_value_exit(&engine.program.objects[*e].routines[*r].body) => {
                            self.pages.ext_answers.push(b)
                        }
                        Ok(_) => {}
                        Err(err) => {
                            self.pages.failed_trigger = None;
                            result = Err(err);
                            break;
                        }
                    }
                }
            }
        }
        if fetch {
            if let Some(held) = held_xrec {
                self.set_xrec_fields(st, held);
            }
            let now = self.page_rec(st).borrow().fields.clone();
            let mut s = st.borrow_mut();
            if name == "onaftergetrecord" {
                s.agr_xrec = Some(now);
            } else {
                s.agcr_xrec = Some(now);
            }
        }
        self.end_page_call(st);
        if name == "onaftergetrecord" && result.is_ok() && self.has_source(st) {
            let rec = self.page_rec(st);
            let key = self.row_key(&rec)?;
            let mut s = st.borrow_mut();
            if !s.agr_rows.iter().any(|k| Self::same_key(k, &key)) {
                s.agr_rows.push(key.clone());
            }
            s.agr_row = Some(key);
        }
        // What the repeater's page-variable controls show for the row: what the variables hold after its OnAfterGetRecord, and for
        // the current row after OnAfterGetCurrRecord (pages/row-page-variable/QiRowPageVar.al:208 QI_RV_List_Navigation, :312
        // QI_RV_Part_Navigation: every row shows its own `agr N | cur N` after every move).
        if declared && result.is_ok() && (name == "onaftergetrecord" || (name == "onaftergetcurrrecord" && stored_row))
        {
            self.note_row_vars(
                st,
                if name == "onaftergetcurrrecord" {
                    vars_before.as_deref()
                } else {
                    None
                },
            )?;
        }
        if result.is_ok() && declared && (name == "onaftergetrecord" || (name == "onaftergetcurrrecord" && stored_row))
        {
            self.run_fn_controls(st, name == "onaftergetrecord")?;
        }
        // What the Editable and Enabled expressions of the repeater's controls read for the row: the variables as its
        // OnAfterGetRecord left them (`row_prop_vars`).
        if name == "onaftergetrecord" && declared && result.is_ok() {
            self.note_row_prop_vars(st)?;
        }
        // The platform's OnAfterGetRecordEvent follows the page's trigger and the extensions' (events/ClEventLeads.al:530,
        // PageTriggers_ExtensionsAndSubscribers_Order: `page.OnAfterGetRecord;extA..;extB..;sub.OnAfterGetRecordEvent;`). Raised
        // only where the page declares the trigger: when the client fetches a row of a page without it is not pinned
        // (`check_page_events` refuses such a page).
        if name == "onaftergetrecord" && result.is_ok() && self.declares(st, "onaftergetrecord") {
            self.page_event(st, "onaftergetrecordevent", None, None)?;
        }
        result.map(|()| answer)
    }

    /// The global slots the property `name` (an Editable or Enabled expression) of control `idx` reads, when the control is the
    /// page's own and the expression is made of the page's variables, constants and operators only (`Editable =
    /// SequenceEditable`, `Enabled = not Locked`); `None` for any other expression.
    pub(crate) fn prop_global_slots(&self, st: &St, idx: usize, name: &str) -> Option<Vec<usize>> {
        fn slots(e: &ir::Expr, out: &mut Vec<usize>) -> bool {
            match e {
                ir::Expr::Const(_) => true,
                ir::Expr::Load(ir::Place::Global(n)) => {
                    if !out.contains(n) {
                        out.push(*n);
                    }
                    true
                }
                ir::Expr::Unary(_, a) | ir::Expr::Convert(_, a) => slots(a, out),
                ir::Expr::Binary(_, a, b) => slots(a, out) && slots(b, out),
                _ => false,
            }
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let c = &page.controls[idx];
        if c.owner.is_some() {
            return None;
        }
        let Some(ir::PropVal::Routine(r)) = c.prop(name) else {
            return None;
        };
        let object = st.borrow().object as usize;
        let body: &[ir::Stmt] = &engine.program.objects[object].routines[*r].body;
        let [ir::Stmt::Exit(Some(e))] = body else {
            return None;
        };
        let mut out = Vec::new();
        slots(e, &mut out).then_some(out)
    }

    /// Notes, for the row the page's record holds, the page variables the Editable and Enabled expressions of the repeater's
    /// controls read (see `row_prop_vars`).
    fn note_row_prop_vars(&mut self, st: &St) -> Result<(), Signal> {
        if !self.has_source(st) || st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return Ok(());
        }
        let engine = self.engine();
        let n = page_of(&engine, st).controls.len();
        let mut wanted: Vec<usize> = Vec::new();
        for i in 0..n {
            for name in ["editable", "enabled"] {
                if matches!(page_of(&engine, st).controls[i].prop(name), Some(p) if !matches!(p, ir::PropVal::Lit(_)))
                    && self.in_repeater(st, i)
                {
                    for slot in self.prop_global_slots(st, i, name).unwrap_or_default() {
                        if !wanted.contains(&slot) {
                            wanted.push(slot);
                        }
                    }
                }
            }
        }
        if wanted.is_empty() {
            return Ok(());
        }
        let inst = self.owner_instance(st, None);
        let values: Vec<(usize, Value)> = wanted
            .into_iter()
            .map(|slot| (slot, inst.borrow().globals[slot].clone()))
            .collect();
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let mut s = st.borrow_mut();
        match s.row_prop_vars.iter_mut().find(|(k, _)| Self::same_key(k, &key)) {
            Some((_, held)) => *held = values,
            None => s.row_prop_vars.push((key, values)),
        }
        Ok(())
    }

    /// What the page variables bound to controls of the repeater hold now: (control index, value).
    fn row_var_values(&self, st: &St) -> Vec<(usize, Value)> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let mut values = Vec::new();
        for i in 0..page.controls.len() {
            let Some((slot, owner, element)) = Self::control_var(page, i) else {
                continue;
            };
            if !self.in_repeater(st, i) {
                continue;
            }
            let inst = self.owner_instance(st, owner);
            values.push((i, Self::var_or_element(&inst.borrow().globals[slot], element)));
        }
        values
    }

    /// Notes what the page-variable controls of the repeater show for the row the page's record holds (see `row_vars`). After
    /// OnAfterGetCurrRecord (`before` holds the values before it ran) only the variables it changed take their new value.
    fn note_row_vars(&mut self, st: &St, before: Option<&[(usize, Value)]>) -> Result<(), Signal> {
        if !self.has_source(st) || st.borrow().cursor != Cursor::Row || st.borrow().draft {
            return Ok(());
        }
        let values = self.row_var_values(st);
        if values.is_empty() {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let mut s = st.borrow_mut();
        s.row_vars_behind_outside_call = false;
        match (s.row_vars.iter_mut().find(|(k, _)| Self::same_key(k, &key)), before) {
            (Some((_, held)), Some(before)) => {
                for ((i, now), (_, was)) in values.into_iter().zip(before) {
                    if !Self::same_values(std::slice::from_ref(&now), std::slice::from_ref(was)) {
                        match held.iter_mut().find(|(c, _)| *c == i) {
                            Some((_, v)) => *v = now,
                            None => held.push((i, now)),
                        }
                    }
                }
            }
            (Some((_, held)), None) => *held = values,
            (None, _) => s.row_vars.push((key, values)),
        }
        Ok(())
    }

    /// xRec of the first OnAfterGetRecord (a blank row) or OnAfterGetCurrRecord (a blank row with each primary-key field
    /// whose filter is a single value on it: the part link) of an open page (testpage/parts/QiHostSaveParts.al:361
    /// `L.AGR(D1/10000|d1 l1 x=/0)`, `L.AGCR(D1/10000|d1 l1 x=D1/0)`, `U.AGCR(D1/10000 x=/0)`, `H.AGCR(D1 x=)`).
    fn blank_fetch_xrec(&mut self, st: &St, with_filters: bool) -> Result<Vec<Value>, Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let mut fields = self.initial_fields(rec.borrow().table as usize);
        if with_filters {
            let r = rec.borrow();
            for &pos in &table.keys[0] {
                let mut single = r.filters.iter().filter(|f| f.field == pos).map(|f| match &f.expr {
                    al_builtins::filter::FilterExpr::Eq(v) => Some(v.clone()),
                    al_builtins::filter::FilterExpr::Range(Some(a), Some(b))
                        if ops::compare(a, b).is_ok_and(|o| o.is_eq()) =>
                    {
                        Some(a.clone())
                    }
                    _ => None,
                });
                if let (Some(Some(v)), None) = (single.next(), single.next()) {
                    fields[pos] = v;
                }
            }
        }
        Ok(fields)
    }

    /// The instance that runs the code of `owner`: the page itself, or one of its extensions.
    fn owner_instance(&self, st: &St, owner: Option<usize>) -> Rc<RefCell<al_builtins::value::Instance>> {
        let s = st.borrow();
        match owner {
            None => s.instance.clone().expect("open page"),
            Some(e) => s
                .ext_instances
                .iter()
                .find(|(o, _)| *o == e)
                .map(|(_, i)| i.clone())
                .expect("an extension of the page has an instance"),
        }
    }

    /// A page-driven call ends: the client takes the page's view back, which carries the key and
    /// `Ascending` but not a per-field `SetAscending` (corpus testpage/TestPageViewRoundTripRoutes.al,
    /// Action_SetAscending_ReadInTheSameTrigger_IsDescending against ..._ReadByALaterAction).
    fn end_page_call(&mut self, st: &St) {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        if engine.program.objects[object].rec_slot.is_none() {
            return;
        }
        if st.borrow().instance.is_some() {
            self.page_rec(st).borrow_mut().field_descending = 0;
        }
    }

    /// A page trigger that answers Boolean: `Some(answer)` when the trigger exists and can state one,
    /// `None` when it is absent or never executes `exit(value)`. A trigger that falls off its end
    /// without an exit does not veto (corpus pages/currpage/PageUpdateNewRowCard.Page.al: OnInsertRecord
    /// assigns the key and exits nothing, and the row is saved; testpage/TrtCardNoInsert.Page.al
    /// exits false and it is not).
    fn page_trigger_answer(&mut self, st: &St, name: &str, args: Vec<Value>) -> Result<Option<bool>, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let routine = page_of(&engine, st)
            .triggers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, r)| *r);
        self.pages.ext_vetoed = false;
        let states_answer = routine.is_some_and(|r| has_value_exit(&engine.program.objects[object].routines[r].body));
        let v = self.page_trigger(st, name, args)?;
        let own = match v {
            Some(Value::Boolean(b)) if states_answer => Some(b),
            _ => None,
        };
        // OnInsertRecord and OnModifyRecord of the page extensions: all of them run, the page's first, whatever the others
        // answered, and one false answer of an extension vetoes the insert or the modify (testpage/
        // UeTriggerTests.al NewRow_FirstExtensionAnswersFalse, ModifyRow_SecondExtensionAnswersFalse; awaiting the corpus pin).
        let exts = std::mem::take(&mut self.pages.ext_answers);
        if exts.is_empty() {
            return Ok(own);
        }
        // Pinned by testpage/UeTriggerTests.al (was assumption `pageext-record-trigger-answers`).
        if exts.contains(&false) {
            self.pages.ext_vetoed = true;
            return Ok(Some(false));
        }
        // The page's own false with every extension answering true is not pinned with extensions present.
        if own == Some(false) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageExtRecordTriggerPageFalse);
        }
        Ok(Some(own.unwrap_or(true)))
    }

    /// `OnQueryClosePage` of a page and of the extensions that declare it: all of them run, the page's first
    /// and then each extension's in application order, whatever the earlier ones answered. The page closes
    /// only when every trigger that states an answer says true (corpus
    /// testpage/TestPageCloseTriggersSelectionCardPageContracts.al:
    /// OnQueryClosePage_PageAndExtension_BothRun_AndBothMustAgreeToClose). `Some(false)` vetoes; `None` when
    /// none states an answer.
    fn query_close_answer(&mut self, st: &St, args: Vec<Value>) -> Result<Option<bool>, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let mut answers = Vec::new();
        let mut calls: Vec<(usize, usize, Option<usize>)> = Vec::new();
        if let Some(r) = page
            .triggers
            .iter()
            .find(|(n, _)| n == "onqueryclosepage")
            .map(|(_, r)| *r)
        {
            calls.push((object, r, None));
        }
        for (n, e, r) in &page.ext_triggers {
            if n == "onqueryclosepage" {
                calls.push((*e, *r, Some(*e)));
            }
        }
        let mut failure = None;
        for (obj, routine, owner) in calls {
            let r = &engine.program.objects[obj].routines[routine];
            // A named return value the page's own trigger assigned and did not `exit` with is its answer: `Result := false`
            // and the end of the trigger refuse the close (pages/triggers/QcrNamedResult.al:114-119, :288-331; the lowering
            // makes an unassigned one Yes). In a page extension's trigger that is not pinned.
            let named = r.ret_local.is_some();
            let states = has_value_exit(&r.body) || (named && owner.is_none());
            let inst = self.owner_instance(st, owner);
            match self.page_call(st, obj, routine, inst, args.clone()) {
                Ok((Value::Boolean(false), _)) if named && !states => {
                    failure = Some(Signal::Unsupported(
                        "a page extension's OnQueryClosePage that ends with its named return value set to false (not pinned)".into(),
                    ));
                    break;
                }
                Ok((Value::Boolean(b), _)) if states => answers.push(b),
                Ok(_) => {}
                Err(e) => {
                    failure = Some(e);
                    break;
                }
            }
        }
        self.end_page_call(st);
        if let Some(e) = failure {
            return Err(e);
        }
        Ok(if answers.contains(&false) {
            Some(false)
        } else if answers.is_empty() {
            None
        } else {
            Some(true)
        })
    }

    /// Raises the platform event `event` that BC publishes after the page's own trigger of that name
    /// (corpus testpage/TestPageTriggerEvents.al:1-20): the page's own trigger runs first, the event is
    /// the platform's last word. `allow` is the `var Allow...` parameter some of them carry; the
    /// returned flag is what the subscribers left in it.
    fn page_event(&mut self, st: &St, event: &str, allow: Option<bool>, below: Option<bool>) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        self.note_event(object, event, None);
        if !engine.events.is_subscribed(object, event) {
            return Ok(allow.unwrap_or(true));
        }
        if engine.program.objects[object].rec_slot.is_none() {
            return self.no_source_page_event(st, event, allow);
        }
        let rec = Value::Record(self.page_rec(st));
        let xobj = &engine.program.objects[object];
        let xrec = {
            let inst = st.borrow().instance.clone().expect("open page");
            let v = xobj.xrec_slot.map(|x| inst.borrow().globals[x].clone());
            v.unwrap_or(Value::Null)
        };
        let mut names = vec!["Rec".to_string()];
        let mut args = vec![rec];
        match event {
            "onopenpageevent" | "onclosepageevent" | "onaftergetcurrrecordevent" | "onaftergetrecordevent" => {}
            "onqueryclosepageevent" => {
                names.push("AllowClose".into());
                args.push(Value::Boolean(true));
            }
            "onnewrecordevent" => {
                names.extend(["BelowxRec".into(), "xRec".into()]);
                args.extend([Value::Boolean(below.unwrap_or(false)), xrec]);
            }
            "oninsertrecordevent" => {
                names.extend(["BelowxRec".into(), "xRec".into(), "AllowInsert".into()]);
                args.extend([Value::Boolean(below.unwrap_or(false)), xrec, Value::Boolean(true)]);
            }
            "onmodifyrecordevent" => {
                names.extend(["xRec".into(), "AllowModify".into()]);
                args.extend([xrec, Value::Boolean(true)]);
            }
            other => return unsupported(format!("page event {other}")),
        }
        let call = crate::dispatch::EventCall {
            object,
            event,
            element: None,
            isolated: false,
            names: &names,
            sender: None,
        };
        self.raise(&call, &mut args)?;
        Ok(match (allow, args.last()) {
            (Some(_), Some(Value::Boolean(b))) => *b,
            _ => true,
        })
    }

    /// `OnBeforeValidateEvent` / `OnAfterValidateEvent` of the control `element` of the page: raised around the control's own
    /// OnValidate after the table field's events and trigger have run (corpus
    /// record/load-fields/P81PageValidateEvents.al: `T.Name.After > P.Name.Before > P.Name.OnValidate >
    /// P.Name.After`). The parameters are `Rec` and `xRec` (the corpus subscribers take `Rec` only).
    pub(crate) fn page_validate_event(&mut self, st: &St, event: &str, element: &str) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        self.note_event(object, event, Some(element));
        if !engine.events.is_subscribed(object, event) {
            return Ok(());
        }
        let rec = Value::Record(self.page_rec(st));
        let xrec = {
            let inst = st.borrow().instance.clone().expect("open page");
            let v = engine.program.objects[object]
                .xrec_slot
                .map(|x| inst.borrow().globals[x].clone());
            v.unwrap_or(Value::Null)
        };
        let names = vec!["Rec".to_string(), "xRec".to_string()];
        let mut args = vec![rec, xrec];
        let call = crate::dispatch::EventCall {
            object,
            event,
            element: Some(element),
            isolated: false,
            names: &names,
            sender: None,
        };
        self.raise(&call, &mut args)
    }

    /// `OnBeforeActionEvent` / `OnAfterActionEvent` of the action `element` of the page: raised by `TestAction.Invoke` before
    /// the action's OnAction trigger and after it when it did not fail (corpus
    /// events/platform-triggers/P114PageActionEvents.al: P114_ActionOrder, P114_ActionRaisesError,
    /// P114_BeforeEventChangesRecord). The subscribers take `Rec` or nothing. A page without a source table,
    /// and a subscriber that is declared and not compiled are refused: none is pinned.
    pub(crate) fn page_action_event(&mut self, st: &St, event: &str, element: &str) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        self.note_event(object, event, Some(element));
        if engine.events.missing_for(object, event, Some(element)).is_some() {
            return unsupported(format!(
                "page event {event} of action {element}: a subscriber listens, and its app is not compiled"
            ));
        }
        if !engine.events.is_subscribed(object, event) {
            return Ok(());
        }
        if engine.program.objects[object].rec_slot.is_none() {
            return unsupported(format!("page event {event} on a page without a source table"));
        }
        let names = vec!["Rec".to_string()];
        let mut args = vec![Value::Record(self.page_rec(st))];
        let call = crate::dispatch::EventCall {
            object,
            event,
            element: Some(element),
            isolated: false,
            names: &names,
            sender: None,
        };
        // An error a subscriber raises stops the action (before) or comes after its OnAction (after) and reaches Invoke
        // with its own text (events/page-events/PageActionSubscriberErrors.al: Act_BeforeSubscriberRaises,
        // Act_AfterSubscriberRaises, Act_BothSubscribersRaise).
        self.raise(&call, &mut args)
    }

    /// Refuses a page some subscriber listens to for an event the client does not raise yet.
    fn check_page_events(&self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        // Subscribers that are compiled and subscribers that are declared and not (both would run
        // in BC).
        let declared = engine
            .events
            .subscribers
            .iter()
            .filter(|(_, subs)| !subs.is_empty())
            .map(|((o, ev), _)| (o, ev))
            .chain(engine.events.missing.keys().map(|(o, ev)| (o, ev)))
            .filter(|(o, _)| **o == object);
        // Hash order must not pick which refusal reason is reported (`outcomes_do_not_depend_on_threads_or_home`).
        let mut declared: Vec<_> = declared.collect();
        declared.sort();
        for (o, ev) in &declared {
            if let Some(why) = self.no_source_event_refusal(&engine, st, **o, ev) {
                return unsupported(why);
            }
        }
        for (o, ev) in declared {
            // The action events are checked when an action is invoked (`refuse_unpinned_action_subscribers`): a page that
            // nothing invokes opens (events/page-events/PageActionEventsWithoutTrigger.al:
            // ActionEvents_OpenWithoutInvoke_SubscribersOfTriggerlessActions).
            // A page over no table of a pinned page type never raises OnAfterGetRecordEvent, nor OnDeleteRecordEvent
            // (`no_source_events`).
            let no_source_pinned = !self.has_source(st) && self.no_source_type_pinned(st);
            if *o == object
                && ev == "onaftergetrecordevent"
                && !self.declares(st, "onaftergetrecord")
                && !no_source_pinned
            {
                return unsupported(
                    "page event onaftergetrecordevent on a page that does not declare OnAfterGetRecord: when the test client \
                     raises it is not pinned"
                        .to_string(),
                );
            }
            if *o == object
                && ev.starts_with("on")
                && ev.ends_with("event")
                && !al_ir::RAISED_PAGE_EVENTS.contains(&ev.as_str())
                // The field events of a page are raised when the client writes the field, so they are refused there
                // (`control_set_value`), not for a page that is merely opened.
                && !matches!(ev.as_str(), "onaftervalidateevent" | "onbeforevalidateevent")
                // The client has no operation that deletes the page's row (an action without code of ours is not on the
                // runtime page, `action_without_behaviour`); a page that is merely
                // opened never raises this event. Base
                // Application subscribes to it on General Journal and the other journals (88 tests of
                // Tests-General Journal were refused at the open).
                && !(ev == "ondeleterecordevent"
                    && (no_source_pinned || al_builtins::assumptions::hit(al_builtins::assumptions::Assumption::PageRowDeleteEventNotRaised)))
            {
                // Events a page publishes itself (IntegrationEvent) are ordinary routines.
                let published = engine
                    .events
                    .publishers
                    .values()
                    .any(|p| p.object == object && p.event == *ev);
                if !published {
                    return unsupported(format!("page event {ev} is not raised by the test client yet"));
                }
            }
        }
        Ok(())
    }

    /// Calls routine `r` of `object` (the page or one of its extensions) on `inst`. A tree page's cut is off Rec while the code runs:
    /// the page's AL code sees every row (`apply_tree_cut`).
    fn page_call(
        &mut self,
        st: &St,
        object: usize,
        r: usize,
        inst: Rc<RefCell<al_builtins::value::Instance>>,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Signal> {
        let hidden = self.hide_tree_cut(st);
        // A routine the page runs reads its row from the server again (`server_row_lag`).
        st.borrow_mut().server_lag = None;
        let result = self.call(object, r, inst, args);
        if let Some(saved) = hidden
            && st.borrow().instance.is_some()
        {
            self.show_tree_cut(st, saved)?;
        }
        result
    }

    /// Calls routine `r` of `owner` (the page, or one of its extensions) on its instance.
    fn owner_routine(&mut self, st: &St, owner: Option<usize>, r: usize) -> Result<Value, Signal> {
        let object = owner.unwrap_or(st.borrow().object as usize);
        let inst = self.owner_instance(st, owner);
        let result = self.page_call(st, object, r, inst, vec![]).map(|(v, _)| v);
        self.end_page_call(st);
        result
    }

    /// Calls routine `r` of `owner` with arguments.
    fn owner_routine_with(
        &mut self,
        st: &St,
        owner: Option<usize>,
        r: usize,
        args: Vec<Value>,
    ) -> Result<Value, Signal> {
        let object = owner.unwrap_or(st.borrow().object as usize);
        let inst = self.owner_instance(st, owner);
        let result = self.page_call(st, object, r, inst, args).map(|(v, _)| v);
        self.end_page_call(st);
        result
    }

    /// Calls routine `r` written in control `idx`: in the extension that declares the control, else the page.
    fn control_routine(&mut self, st: &St, idx: usize, r: usize) -> Result<Value, Signal> {
        let engine = self.engine();
        let owner = page_of(&engine, st).controls[idx].owner;
        self.owner_routine(st, owner, r)
    }

    /// The value of the routine behind property value `v` of control `idx`; `None` for a literal.
    fn prop_routine_value(&mut self, st: &St, idx: usize, v: &ir::PropVal) -> Result<Option<Value>, Signal> {
        match v {
            ir::PropVal::Lit(_) => Ok(None),
            ir::PropVal::Routine(r) => self.control_routine(st, idx, *r).map(Some),
            ir::PropVal::Ext(e, r) => self.owner_routine(st, Some(*e), *r).map(Some),
        }
    }
}

/// Whether any statement of `body` executes `exit(value)`.
fn has_value_exit(body: &[ir::Stmt]) -> bool {
    use ir::Stmt;
    body.iter().any(|s| match s {
        Stmt::Exit(Some(_)) => true,
        Stmt::If(_, a, b) => has_value_exit(a) || has_value_exit(b),
        Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => has_value_exit(b),
        Stmt::For { body, .. } | Stmt::Foreach { body, .. } => has_value_exit(body),
        Stmt::Case {
            branches, otherwise, ..
        } => branches.iter().any(|(_, b)| has_value_exit(b)) || has_value_exit(otherwise),
        _ => false,
    })
}
