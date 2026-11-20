//! The filters in filter group 1 (the global group) a session holds, per table, what other records of the table see of
//! them, and when they end.
//!
//! In-session: BC keeps a group 1 filter on every record of the table for the rest of the session
//! ([`Session::set_global_group_filter`], [`Session::group1_use`]).
//!
//! Across sessions (pinned by tests/al-language/install-upgrade/G1sInstallSessionTests.al with
//! tests/al-language-xapp-a/G1sXaInstall.al and tests/al-language-xapp-b/G1sXbInstall.al, BC 27 to 29): a filter that a
//! Record variable set with SetFilter or SetRange in filter group 1, in the per-company part of an app's install session
//! in the session company, in the body of one of two routines ([`Group1InstallShape`]; the innermost AL frame is that
//! routine): the install codeunit's OnInstallAppPerCompany trigger (G1sXaInstall.al:131-155, the filters at :142-144
//! and :151-153), or a "System Initialization".OnAfterLogin subscriber that the session's Company Triggers raise
//! (:116-124); and the install trigger, or the Company Triggers that raised the subscriber, ran to its end (the probe's
//! routines all do). It is not there in the next app's install session (G1sInstallSessionTests.al:55-63, B after A;
//! :65-73, the main app after B) nor in the test session after the install sessions (:75-85); the same for a per-company
//! table (:107-123, table 98485). An OnAfterLogin subscriber sets it again in each session (:87-105). Only an entry set
//! in that shape ends; every other entry stays, and a record without it reads it only for the calls
//! [`Session::group1_use`] allows.

use super::*;
use crate::globaltriggers::GlobalOp;
use crate::records::Rec;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::unsupported;
use al_builtins::record_op::RecordOp;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Weak;

/// The place in an app's install session that the session runs now, of the two the probe measured; set by the runner
/// around the per-company part in the session company.
#[derive(Clone, Default)]
pub enum Group1InstallShape {
    /// Neither: a group 1 filter set now stays for the session.
    #[default]
    Off,
    /// The install codeunit's OnInstallAppPerCompany trigger, as (object, routine): only a filter its body sets ends
    /// (G1sXaInstall.al:131-155).
    InstallTrigger((usize, usize)),
    /// The session's Company Triggers: only a filter the body of one of these routines, the "System Initialization".
    /// OnAfterLogin subscribers, sets ends (G1sXaInstall.al:116-124).
    Login(Rc<[(usize, usize)]>),
}

/// Whether a group 1 filter ends with its install session.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ends {
    /// Set outside the measured shape: it stays for the session.
    Never,
    /// Set in a [`Group1InstallShape`] other than `Off`, by a routine that has not returned yet: it ends only when the
    /// routine runs to its end ([`Session::end_group1_install_routine`]).
    AwaitsRoutine,
    /// Set in the measured shape by a routine that ran to its end: it ends with its install session.
    WithInstallSession,
}

/// One table's group 1 filters. An entry with no filter is left by a record that took back the filter another record set
/// ([`Session::set_global_group_filter`]): the records that still hold filters in group 1 are refused there.
struct Entry {
    /// The filters, as text to compare ([`group1_signature`]).
    signature: String,
    /// The filters themselves, for the records that see them ([`Session::with_group1`]).
    filters: Vec<FieldFilter>,
    ends: Ends,
    /// The kind of variable that set the filters; `None` for a temporary RecordRef, which no test measured.
    setter: Option<VarKind>,
    /// The readers whose FindSet read the filters and found a row, and whose Next calls since then found one: only such a
    /// reader's Next reads the filters (`Fresh*`, FilterGroupOtherVariable.al:169-172).
    find_set_readers: Vec<Weak<RefCell<RecordState>>>,
}

/// The kinds of variable FilterGroupOtherVariable.al set a group 1 filter with and read it through, each with its own
/// readers ([`measured_read`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum VarKind {
    /// A Record variable of a database table.
    Record,
    /// A temporary Record variable.
    TemporaryRecord,
    /// A RecordRef of a database table.
    RecordRef,
}

impl VarKind {
    /// The kind of a Record variable (`record`) or a RecordRef, of a temporary table or not; `None` for a temporary
    /// RecordRef.
    fn of(record: bool, temporary: bool) -> Option<VarKind> {
        match (record, temporary) {
            (true, false) => Some(VarKind::Record),
            (true, true) => Some(VarKind::TemporaryRecord),
            (false, false) => Some(VarKind::RecordRef),
            (false, true) => None,
        }
    }
}

/// The group 1 filters of one session; StartSession parks them (bgsession.rs).
#[derive(Default)]
pub(crate) struct Filters {
    by_table: HashMap<u32, Entry>,
}

/// The session's group 1 filters (`Session::global_group1`) and the install shape they are set in.
#[derive(Default)]
pub(crate) struct Group1Filters {
    pub(crate) filters: Filters,
    shape: Group1InstallShape,
}

impl Group1Filters {
    /// No table holds a group 1 filter.
    pub(crate) fn is_empty(&self) -> bool {
        self.filters.by_table.values().all(|e| e.signature.is_empty())
    }

    /// Some table holds a group 1 filter, or had one another record took back.
    pub(crate) fn has_tables(&self) -> bool {
        !self.filters.by_table.is_empty()
    }

    /// The group 1 filters of every table that holds some, as text to compare (al-vm `state_stamp.rs`).
    pub(crate) fn signatures(&self) -> std::collections::BTreeMap<u32, String> {
        (self.filters.by_table.iter())
            .filter(|(_, e)| !e.signature.is_empty())
            .map(|(&t, e)| (t, e.signature.clone()))
            .collect()
    }
}

impl Session {
    /// Notes what group 1 of table `table` holds now, from `rec`; `by_record`: a Record variable of AL code set it;
    /// `by_another`: a record that held no group 1 filter took the filter back.
    fn note_group1(&mut self, table: u32, rec: &Rec, by_record: bool, by_another: bool) {
        let signature = group1_signature(&rec.borrow());
        if signature.is_empty() && !by_another {
            self.global_group1.filters.by_table.remove(&table);
            return;
        }
        let filters = rec.borrow().filters.iter().filter(|f| f.group == 1).cloned().collect();
        let setter = VarKind::of(by_record, rec.borrow().temporary);
        // The innermost AL frame: the routine whose body sets the filter.
        let innermost = (self.frames.last())
            .zip(self.call_stack.last())
            .map(|(&o, f)| (o, f.routine));
        let in_shape = by_record
            && innermost.is_some_and(|r| match &self.global_group1.shape {
                Group1InstallShape::Off => false,
                Group1InstallShape::InstallTrigger(trigger) => r == *trigger,
                Group1InstallShape::Login(subscribers) => subscribers.contains(&r),
            });
        let entry = Entry {
            signature,
            filters,
            ends: if in_shape { Ends::AwaitsRoutine } else { Ends::Never },
            setter,
            find_set_readers: Vec::new(),
        };
        self.global_group1.filters.by_table.insert(table, entry);
    }

    /// The place in the install session the session runs now (module documentation). The runner ends each routine it
    /// runs with [`Self::end_group1_install_routine`]; an entry still waiting for its routine stays for the session.
    pub fn set_group1_install_shape(&mut self, shape: Group1InstallShape) {
        self.settle_group1_entries(false);
        self.global_group1.shape = shape;
    }

    /// A routine the runner ran returned (`ran_to_end`: it returned Ok), whether it ran in a [`Group1InstallShape`] or
    /// not: the filters it set in the measured shape end with their install session only if it ran to its end (the probe
    /// measured routines that do, G1sXaInstall.al:116-124, :131-155); otherwise they stay for the session. The shape is
    /// `Off` again.
    pub fn end_group1_install_routine(&mut self, ran_to_end: bool) {
        self.settle_group1_entries(ran_to_end);
        self.global_group1.shape = Group1InstallShape::Off;
    }

    /// Settles the entries waiting for their routine: they end with their install session when it ran to its end.
    fn settle_group1_entries(&mut self, ran_to_end: bool) {
        let settled = if ran_to_end {
            Ends::WithInstallSession
        } else {
            Ends::Never
        };
        for e in self.global_group1.filters.by_table.values_mut() {
            if e.ends == Ends::AwaitsRoutine {
                e.ends = settled;
            }
        }
    }

    /// An app's install session begins, or the test session follows the install sessions: the group 1 filters set in
    /// the shape of an earlier install session end (G1sInstallSessionTests.al:55-63, :65-73, :75-85; module
    /// documentation).
    pub fn end_group1_install_session(&mut self) {
        let g = &mut self.global_group1;
        g.shape = Group1InstallShape::Off;
        g.filters.by_table.retain(|_, e| e.ends != Ends::WithInstallSession);
    }
}

/// The filters of a record in filter group 1, as text to compare.
fn group1_signature(r: &RecordState) -> String {
    let mut v: Vec<String> = (r.filters.iter())
        .filter(|f| f.group == 1)
        .map(|f| format!("{}={:?}", f.field, f.expr))
        .collect();
    v.sort();
    v.join(";")
}

/// Who asks for the records of a table whose group 1 holds an entry ([`Session::group1_use`]).
#[derive(Clone, Copy)]
pub(crate) enum Group1Op<'a> {
    /// A Record method AL code calls, on a Record or through a FieldRef.
    Record(RecordOp),
    /// A RecordRef method AL code calls.
    RecordRef(&'a str),
    /// A page reading its rows.
    Page,
}

impl Group1Op<'_> {
    fn name(self) -> String {
        match self {
            Group1Op::Record(m) => m.name().to_string(),
            Group1Op::RecordRef(m) => format!("RecordRef.{m}"),
            Group1Op::Page => "A page reading its rows".to_string(),
        }
    }

    fn is_filter_group(self) -> bool {
        matches!(
            self,
            Group1Op::Record(RecordOp::FilterGroup) | Group1Op::RecordRef("FilterGroup")
        )
    }
}

/// The calls tests/al-language/record/filters/FilterGroupOtherVariable.al (codeunit 97452) measured on a variable that holds
/// no filter of its own while another variable's filter is in group 1, by the kind of variable that set the filter and the
/// kind that reads it; `group`: the reader's current filter group; `args`: the number of arguments of the call (each
/// measured call passes no optional argument: no RunTrigger, no Steps, no UpdateKey).
fn measured_read(setter: VarKind, reader: VarKind, op: Group1Op, group: i32, args: usize) -> bool {
    use RecordOp::*;
    use VarKind::*;
    match (reader, op, group, args) {
        // `FreshGroup1Filter*` (:177-186) after each kind of setter: :614 and :730, :828, :861.
        (Record, Group1Op::Record(GetFilters), 1, 0) => true,
        // `Fresh*` (:163-174) after each kind of setter: :605-611 and :729, :827, :860.
        (Record, Group1Op::Record(Count | FindSet | Next), 0, 0) => true,
        // A Record setter, a database Record reader: :612-622, :731-746.
        (Record, Group1Op::Record(IsEmpty | GetFilters | Reset | Clear | DeleteAll | Init | Insert | Modify), 0, 0)
        | (Record, Group1Op::Record(Get), 0, 1)
        | (Record, Group1Op::Record(ModifyAll), 0, 2) => setter == Record,
        // A temporary Record reader's Count after a temporary setter (:826) or a Record setter (:900).
        (TemporaryRecord, Group1Op::Record(Count), 0, 0) => setter == TemporaryRecord || setter == Record,
        // A RecordRef reader: Count after a RecordRef setter (:859) or a Record setter (:896); GetFilters in its group 1
        // after a Record setter (:897-898).
        (RecordRef, Group1Op::RecordRef("Count"), 0, 0) => setter == RecordRef || setter == Record,
        (RecordRef, Group1Op::RecordRef("GetFilters"), 1, 0) => setter == Record,
        _ => false,
    }
}

/// The view every measured reader had: no filter in any group, the primary key ascending, no marks, the session company
/// (each reader of FilterGroupOtherVariable.al is a fresh variable, or one after Reset or Clear: :611-621, :733, :745).
fn reader_view_measured(r: &RecordState) -> bool {
    r.filters.is_empty()
        && r.key == 0
        && r.adhoc.is_none()
        && r.ascending
        && r.field_descending == 0
        && r.marks.is_empty()
        && !r.marked_only
        && r.company.is_none()
}

/// The step of a FindSet loop a shared call is ([`Entry::find_set_readers`]).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopStep {
    FindSet,
    Next,
    Other,
}

/// What a record of a table whose group 1 holds an entry reads ([`Session::group1_use`]).
pub(crate) enum Group1Use {
    /// Its own filters.
    Own,
    /// Its own filters and the group 1 filters of table `table`'s entry, for this one call, which is `step` of a FindSet
    /// loop ([`Session::with_group1`]).
    Shared {
        table: u32,
        filters: Vec<FieldFilter>,
        step: LoopStep,
    },
}

/// How a SetFilter or SetRange in filter group 1 changes the session's entry ([`Session::set_global_group_filter`]).
enum Group1Set {
    /// A database record with no entry for its table, or that holds the entry's filters: main's path, P145PageRunView.al.
    Setter,
    /// A record that holds no group 1 filter takes back the entry's one filter: SetRange(Field) on the field the entry
    /// filters (FilterGroupOtherVariable.al:197-204 `ClearGroup1In3` at :748, `ClearGroup1In4` at :793; then a fresh
    /// variable reads every row, :762 and :797, and an empty group 1, :763).
    ClearByAnother,
}

impl Session {
    /// SetFilter or SetRange in filter group 1 (the global group): BC keeps the filter on every record of the table for the
    /// rest of the session, through Reset and Clear (types/text-code-char/P145PageRunView.al:232-235, SfSeed :236-268,
    /// seed_before = 4 at :283), the temporary records of the table and its RecordRefs included
    /// (FilterGroupOtherVariable.al:802-874). The session notes what group 1 holds now.
    #[cold]
    #[inline(never)]
    pub(crate) fn set_global_group_filter(
        &mut self,
        rec: &Rec,
        table: u32,
        method: RecordOp,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let set = self.group1_set(rec, table, method, args)?;
        let by_record = self.is_al_record_call(method, rec);
        let r = self.record_op(method, rec, args)?;
        self.note_group1(table, rec, by_record, matches!(set, Group1Set::ClearByAnother));
        Ok(r)
    }

    /// Which change [`Self::set_global_group_filter`] makes, or the refusal of a shape no test measured.
    fn group1_set(&self, rec: &Rec, table: u32, method: RecordOp, args: &[Value]) -> Result<Group1Set, Signal> {
        let entry = self.global_group1.filters.by_table.get(&table);
        let (temporary, own) = {
            let r = rec.borrow();
            (r.temporary, group1_signature(&r))
        };
        let set_range = method == RecordOp::SetRange;
        let set = match entry {
            // No entry (or one taken back, by a record that holds no group 1 filter).
            None => (!temporary || set_range && self.integer_value(rec, args)).then_some(Group1Set::Setter),
            Some(e) if e.signature.is_empty() && own.is_empty() => {
                (!temporary || set_range && self.integer_value(rec, args)).then_some(Group1Set::Setter)
            }
            Some(e) if !e.signature.is_empty() && own == e.signature => {
                (!temporary || set_range && args.len() == 1).then_some(Group1Set::Setter)
            }
            Some(e) if !temporary && own.is_empty() && set_range && args.len() == 1 && self.entry_pinned(rec, e) => {
                let field = args.first().and_then(|a| match a {
                    Value::Integer(p) => usize::try_from(*p).ok(),
                    _ => None,
                });
                (field == e.filters.first().map(|f| f.field)).then_some(Group1Set::ClearByAnother)
            }
            _ => None,
        };
        match set {
            Some(s) => Ok(s),
            None if temporary => unsupported(format!(
                "{} in filter group 1 (the global group) on a temporary record (how BC scopes it is not pinned)",
                method.name()
            )),
            None => self.group1_refusal(entry.is_some_and(|e| e.signature.is_empty()), Group1Op::Record(method)),
        }
    }

    /// SetRange(Field, Value) of one Integer value on an Integer field: the form a temporary record set in group 1
    /// (FilterGroupOtherVariable.al:822-824, SetRange(Tag, 1)).
    fn integer_value(&self, rec: &Rec, args: &[Value]) -> bool {
        let Ok(t) = self.table(rec) else {
            return false;
        };
        matches!(args, [Value::Integer(p), Value::Integer(_)]
            if usize::try_from(*p).ok().and_then(|p| t.fields.get(p)).is_some_and(|f| f.ty == RtType::Integer))
    }

    /// The entry form FilterGroupOtherVariable.al measured: one filter, SetRange of one Integer value on an Integer field
    /// outside the primary key (SetRange(Tag, 1) at :193, :604, :823, :855, :893; Tag is no key field of the tables).
    fn entry_pinned(&self, rec: &Rec, e: &Entry) -> bool {
        let Ok(t) = self.table(rec) else {
            return false;
        };
        let [f] = &e.filters[..] else {
            return false;
        };
        let one_integer = match &f.expr {
            FilterExpr::Range(Some(Value::Integer(a)), Some(Value::Integer(b))) => a == b,
            FilterExpr::Eq(Value::Integer(_)) => true,
            _ => false,
        };
        let outside_primary_key = t.keys.first().is_some_and(|pk| !pk.contains(&f.field));
        one_integer && outside_primary_key && t.fields.get(f.field).is_some_and(|d| d.ty == RtType::Integer)
    }

    /// A shared write runs no code: no event subscriber of the table, and its global trigger known to be off (the tables of
    /// FilterGroupOtherVariable.al have no subscriber, :622, :736-746; RunTrigger is not passed, [`measured_read`]).
    fn shared_write_runs_no_code(&self, rec: &Rec, op: Group1Op) -> bool {
        let global = match op {
            Group1Op::Record(RecordOp::DeleteAll) => GlobalOp::Delete,
            Group1Op::Record(RecordOp::Insert) => GlobalOp::Insert,
            Group1Op::Record(RecordOp::Modify | RecordOp::ModifyAll) => GlobalOp::Modify,
            _ => return true,
        };
        let Ok(t) = self.table(rec) else {
            return false;
        };
        let subscribed = self.engine.events.any_subscriber_of(t.object as usize);
        !subscribed && self.global_known_off(&t, global)
    }

    /// What `rec`, of table `table`, reads for the call `op` while group 1 of the table holds an entry. A record that holds
    /// the entry's filters (the setter, or a copy of it such as the record of Page.Run(Page, Rec), P145PageRunView.al:232-235)
    /// reads its own, on a database table; on a temporary one only Count is measured (FilterGroupOtherVariable.al:825). A
    /// record that holds no group 1 filter reads the entry's filters as well for the calls [`measured_read`] lists, with the
    /// view every measured reader had ([`reader_view_measured`]), a Next only inside the reader's FindSet loop, a write only
    /// when it runs no code ([`Self::shared_write_runs_no_code`]), and the entry in the measured form
    /// ([`Self::entry_pinned`]). After a record that held none took the filter back,
    /// a record with no group 1 filter reads its own (:762-763, :797); one that still holds some is refused. Every other
    /// shape is refused.
    pub(crate) fn group1_use(&self, rec: &Rec, table: u32, op: Group1Op, args: &[Value]) -> Result<Group1Use, Signal> {
        let Some(entry) = self.global_group1.filters.by_table.get(&table) else {
            return Ok(Group1Use::Own);
        };
        if op.is_filter_group() {
            return Ok(Group1Use::Own);
        }
        let (temporary, own, group, view_measured) = {
            let r = rec.borrow();
            (
                r.temporary,
                group1_signature(&r),
                r.filter_group,
                reader_view_measured(&r),
            )
        };
        let taken_back = entry.signature.is_empty();
        if taken_back && own.is_empty() {
            return Ok(Group1Use::Own);
        }
        if !taken_back && own == entry.signature && (!temporary || matches!(op, Group1Op::Record(RecordOp::Count))) {
            return Ok(Group1Use::Own);
        }
        let reader = VarKind::of(matches!(op, Group1Op::Record(_)), temporary);
        let step = match op {
            Group1Op::Record(RecordOp::FindSet) => LoopStep::FindSet,
            Group1Op::Record(RecordOp::Next) => LoopStep::Next,
            _ => LoopStep::Other,
        };
        let in_loop = || (entry.find_set_readers.iter()).any(|w| std::ptr::eq(w.as_ptr(), Rc::as_ptr(rec)));
        let shared = !taken_back
            && view_measured
            && (entry.setter.zip(reader)).is_some_and(|(s, r)| measured_read(s, r, op, group, args.len()))
            && (step != LoopStep::Next || in_loop())
            && self.entry_pinned(rec, entry)
            && self.shared_write_runs_no_code(rec, op);
        if shared {
            return Ok(Group1Use::Shared {
                table,
                filters: entry.filters.clone(),
                step,
            });
        }
        self.group1_refusal(taken_back, op)
    }

    fn group1_refusal<T>(&self, taken_back: bool, op: Group1Op) -> Result<T, Signal> {
        let what = op.name();
        if taken_back {
            return unsupported(format!(
                "{what} on a record that holds filters in filter group 1 (the global group) after another record took \
                 the global group's filter back (not pinned)"
            ));
        }
        unsupported(format!(
            "{what} on a record of a table with a filter in filter group 1 (the global group) that another record set: BC \
             applies it to every record of the table for the session, the runtime does not"
        ))
    }

    /// Runs `f` with `use_` applied to `rec`: the entry's group 1 filters of a [`Group1Use::Shared`] are on the record
    /// (which holds no group 1 filter of its own) only for the call. The reader stays in its FindSet loop while FindSet
    /// and Next find a row ([`Entry::find_set_readers`]).
    pub(crate) fn with_group1(
        &mut self,
        rec: &Rec,
        use_: Group1Use,
        f: impl FnOnce(&mut Self) -> Result<Value, Signal>,
    ) -> Result<Value, Signal> {
        let Group1Use::Shared { table, filters, step } = use_ else {
            return f(self);
        };
        rec.borrow_mut().filters.extend(filters);
        let r = f(self);
        rec.borrow_mut().filters.retain(|f| f.group != 1);
        let found = match (step, &r) {
            (LoopStep::FindSet, Ok(Value::Boolean(true))) => true,
            (LoopStep::Next, Ok(Value::Integer(n))) => *n != 0,
            _ => false,
        };
        if let Some(e) = self.global_group1.filters.by_table.get_mut(&table) {
            let here = Rc::as_ptr(rec);
            e.find_set_readers
                .retain(|w| w.strong_count() > 0 && !std::ptr::eq(w.as_ptr(), here));
            if found {
                e.find_set_readers.push(Rc::downgrade(rec));
            }
        }
        r
    }
}
