//! Demand-driven checking of dependency apps (ADR-013 decision 6).
//!
//! Microsoft's apps hold ten thousand objects and a test reaches a few hundred of
//! them. Checking and lowering all of it costs tens of seconds, so the checker starts
//! from the workspace's own code and follows what that code uses:
//!
//! * A routine that is *reached* has its body checked, and the procedures it calls
//!   are reached in turn.
//! * An object is *full* when reached code runs it: a codeunit it calls, a table it
//!   writes to. Everything BC would run implicitly for it is reached too: the triggers
//!   of a table, the subscribers to its events, the codeunits that implement an
//!   interface or enum it is used through.
//! * A table is *read* when reached code only reads it (a record variable, `Get`,
//!   `FindSet`, field access, `CalcFields`). Reading fires no trigger and no event,
//!   so nothing runs implicitly; the tables it relates to are lowered with it.
//! * An object is *schema* when only the shape of another object depends on it (a
//!   `TableRelation` target, a FlowField's source table, a page named by
//!   `Page::"X"`): it is lowered, its routines are not reached.
//!
//! Anything the analysis does not reach is not compiled. The workspace then stops
//! resolving it (`Workspace::restrict_to`), so code that needs it after all is
//! refused with "outside the workspace" instead of running half-present (hard rule 4).

use super::posmap::FxSet as HashSet;
use super::*;
use crate::workspace::Resolution;
use al_ir as ir;
use std::rc::Rc;

/// Base Application's `Company-Initialize`, which a new company runs once.
pub const COMPANY_INITIALIZE: u32 = 2;

/// The codeunits BC's AL test runner runs by id around every test method when they exist (Test Runner app, BC 27.5):
/// 131006 "Permissions Mock" (started and stopped by `Test Runner - Mgt`, TestRunnerMgt.Codeunit.al:105, :127),
/// 130301 "Reset State Before Test Run" and 130302 "Set State Test Run" (ALTestRunnerResetEnvironment.Codeunit.al:42-63),
/// 132553 "Block Changes to System Tables" (:65-75). They are found by id only, as the runner's `AllObj` check does.
pub const TEST_RUNNER_CODEUNITS: [u32; 4] = [131006, 130301, 130302, 132553];

/// An object nothing in the compile refers to.
pub const NONE: u8 = 0;
/// Lowered, with its routines stubbed.
pub const SCHEMA: u8 = 1;
/// A table reached code reads.
pub const READ: u8 = 2;
/// Run by reached code.
pub const FULL: u8 = 3;

/// What the workspace's code reaches in its dependency apps.
#[derive(Debug, Clone, Default)]
pub struct Demand {
    /// Per object: `NONE`, `SCHEMA`, `READ` or `FULL`. Objects of workspace apps are not tracked.
    pub level: Vec<u8>,
    /// The routines of dependency objects whose bodies were checked: (object, routine offset).
    pub reached: HashSet<(usize, usize)>,
}

impl Demand {
    /// Whether dependency object `i` is part of the compile at all.
    pub fn active(&self, i: usize) -> bool {
        self.level[i] != NONE
    }

    pub fn reached(&self, object: usize, offset: usize) -> bool {
        self.reached.contains(&(object, offset))
    }
}

pub(super) struct State {
    level: Vec<u8>,
    reached: HashSet<(usize, usize)>,
    queue: Vec<(usize, usize)>,
    /// Objects whose level just rose, with the new level.
    fresh: Vec<(usize, u8)>,
    /// Per table: the write operations (`INSERT`, ...) reached code performs on it.
    ops: Vec<u8>,
    /// Tables with a write operation not processed yet.
    fresh_ops: Vec<(usize, u8)>,
    /// The write operations (any table, any app) whose `Global Triggers` subscribers are reached.
    global_ops: u8,
    /// Write operations not yet processed for `Global Triggers`.
    fresh_global: u8,
    /// The write operations a `RecordRef` or `FieldRef` performs on a table that is known only
    /// when the code runs: every table the compile holds runs their triggers and subscribers.
    recref_ops: u8,
    /// `recref_ops` not processed yet.
    fresh_recref: u8,
    ctxs: HashMap<usize, Rc<ObjCtx>>,
    routines: HashMap<usize, Rc<Vec<ast::Routine>>>,
    /// The `OnResolveCaptionClass` subscribers are reached (a page with a `CaptionClass` is run).
    caption_class: bool,
    /// The `CustomDocumentMergerEx` subscribers are reached (a report with a custom layout is compiled).
    reporting_triggers: bool,
    /// The `MakeDateFilter` subscribers are reached (code that sets filters is compiled).
    filter_helper: bool,
    /// The `SubstituteReport` and `OnDocumentReady` subscribers are reached (code that calls `SaveAs` is compiled).
    save_as_events: bool,
    /// Bits per object kind: the catalog tables that list the objects of that kind are used, and
    /// every object of the kind is compiled to its shape (`touch_object_catalog`).
    catalogs: u8,
}

impl State {
    pub(super) fn new(objects: usize) -> State {
        State {
            level: vec![NONE; objects],
            reached: HashSet::default(),
            queue: Vec::new(),
            fresh: Vec::new(),
            ops: vec![0; objects],
            fresh_ops: Vec::new(),
            global_ops: 0,
            fresh_global: 0,
            recref_ops: 0,
            fresh_recref: 0,
            ctxs: HashMap::default(),
            routines: HashMap::default(),
            caption_class: false,
            reporting_triggers: false,
            filter_helper: false,
            save_as_events: false,
            catalogs: 0,
        }
    }
}

/// Write operations on a table, each of which fires its own triggers and events.
pub const INSERT: u8 = 1;
pub const MODIFY: u8 = 2;
pub const DELETE: u8 = 4;
pub const RENAME: u8 = 8;
pub const VALIDATE: u8 = 16;
pub const ALL_OPS: u8 = INSERT | MODIFY | DELETE | RENAME | VALIDATE;

/// The table triggers an operation runs, and the events it raises.
const OP_TRIGGERS: [(u8, &str, [&str; 2]); 5] = [
    (INSERT, "OnInsert", ["OnBeforeInsertEvent", "OnAfterInsertEvent"]),
    (MODIFY, "OnModify", ["OnBeforeModifyEvent", "OnAfterModifyEvent"]),
    (DELETE, "OnDelete", ["OnBeforeDeleteEvent", "OnAfterDeleteEvent"]),
    (RENAME, "OnRename", ["OnBeforeRenameEvent", "OnAfterRenameEvent"]),
    (
        VALIDATE,
        "OnValidate",
        ["OnBeforeValidateEvent", "OnAfterValidateEvent"],
    ),
];

/// The operations a Record method performs, if it writes.
pub(super) fn record_ops(method: &str) -> u8 {
    match method.to_ascii_lowercase().as_str() {
        "insert" => INSERT,
        "modify" => MODIFY,
        "delete" | "deleteall" => DELETE,
        "rename" => RENAME,
        "validate" => VALIDATE,
        "modifyall" => MODIFY | VALIDATE,
        _ => 0,
    }
}

const EVENT_ATTRS: [&str; 3] = ["IntegrationEvent", "BusinessEvent", "InternalEvent"];

impl Checker<'_> {
    /// Raises dependency object `i` to `level`.
    fn raise(&self, i: usize, level: u8) {
        if !self.ws.is_dependency(i) {
            return;
        }
        if let Some(st) = self.demand.borrow_mut().as_mut() {
            if st.level[i] == NONE {
                if let Some(tx) = &self.prefetch {
                    let _ = tx.send(self.ws.objects[i].file);
                }
            }
            if st.level[i] < level {
                st.level[i] = level;
                st.fresh.push((i, level));
            }
        }
    }

    /// Reached code uses object `i` the way its kind is usually used: a table is read, a
    /// codeunit, enum, interface or query runs, a page or report is only named.
    pub(super) fn touch(&self, i: usize) {
        let level = match self.ws.objects[i].kind {
            ObjKind::Table => READ,
            ObjKind::Codeunit | ObjKind::Enum | ObjKind::EnumExtension | ObjKind::Interface | ObjKind::Query => FULL,
            _ => SCHEMA,
        };
        self.raise(i, level);
    }

    /// Reached code runs object `i`: drives a page through the test client.
    pub(super) fn touch_full(&self, i: usize) {
        self.raise(i, FULL);
    }

    /// Reached code writes (`ops`) to a table of any app, perhaps one it does not name (a
    /// `RecordRef`): that raises the events of the platform's `Global Triggers` codeunit, whose
    /// subscribers can sit in a dependency app.
    pub(super) fn touch_global_ops(&self, ops: u8) {
        if ops == 0 {
            return;
        }
        if let Some(st) = self.demand.borrow_mut().as_mut() {
            let new = ops & !st.global_ops;
            if new != 0 {
                st.global_ops |= new;
                st.fresh_global |= new;
            }
        }
    }

    /// Reached code writes through a `RecordRef` or `FieldRef` (`ops`): the table is known only when
    /// the code runs, and a table the compile holds may be the one. Every table that is compiled,
    /// now or later, runs the triggers and subscribers of those operations (so the complete and the
    /// cached compile, which hold every table, give the outcome the demand-driven one gives).
    pub(super) fn touch_recref_ops(&self, ops: u8) {
        if ops == 0 {
            return;
        }
        self.touch_global_ops(ops);
        if let Some(st) = self.demand.borrow_mut().as_mut() {
            let new = ops & !st.recref_ops;
            if new != 0 {
                st.recref_ops |= new;
                st.fresh_recref |= new;
            }
        }
    }

    /// Reached code writes to table `i` (`ops`): that runs its triggers and the subscribers
    /// to its events for those operations.
    pub(super) fn touch_ops(&self, i: usize, ops: u8) {
        if ops == 0 {
            return;
        }
        self.touch_global_ops(ops);
        if !self.ws.is_dependency(i) {
            return;
        }
        self.raise(i, FULL);
        if let Some(st) = self.demand.borrow_mut().as_mut() {
            let new = ops & !st.ops[i];
            if new != 0 {
                st.ops[i] |= new;
                st.fresh_ops.push((i, new));
            }
        }
    }

    /// The shape of reached code's objects depends on `i`.
    pub(super) fn touch_schema(&self, i: usize) {
        self.raise(i, SCHEMA);
    }

    /// Every object a variable of type `t` can reach: `t` is used by code that runs.
    pub(super) fn touch_ty(&self, file: usize, t: &Ty) {
        if self.demand.borrow().is_none() {
            return;
        }
        match t {
            Ty::Record { table, .. } | Ty::TestFilter(table) => {
                if let Some(t) = self.resolve_at(file, ObjKind::Table, table) {
                    self.touch_object_catalog(t);
                }
            }
            Ty::Object(kind, name) => {
                let i = self.resolve_at(file, *kind, name);
                // The call graph of a variable of a report or XML port holds its triggers
                // (`lower.rs`, `driven_edges`), so that they are compiled whenever the variable
                // exists, or the complete compile would refuse tests the demand-driven one
                // runs.
                if let (Some(i), ObjKind::Report | ObjKind::XmlPort) = (i, kind) {
                    self.touch_full(i);
                }
            }
            Ty::Enum(name) => {
                self.resolve_at(file, ObjKind::Enum, name);
            }
            Ty::TestPage(name) | Ty::TestPart(name) => {
                if let Some(i) = self.resolve_at(file, ObjKind::Page, name) {
                    self.touch_full(i);
                    self.reach_run_targets(i);
                }
            }
            Ty::TestRequestPage(name) => {
                if let Some(i) = self.resolve_at(file, ObjKind::Report, name) {
                    self.touch_full(i);
                }
            }
            Ty::List(t) | Ty::Array(_, t) => self.touch_ty(file, t),
            Ty::Dictionary(k, v) => {
                self.touch_ty(file, k);
                self.touch_ty(file, v);
            }
            _ => {}
        }
    }

    /// A read of a platform table that lists the objects of the installed apps (`Field`, `Key`,
    /// `Table Relations Metadata`, `Report Metadata`, `Page Metadata`, `Page Action`, `Event
    /// Subscription`, ...) answers for every one of them; an object that was not compiled would be
    /// a row the runtime must refuse. So using such a table compiles the shape of every object it
    /// lists, in every compile alike (the complete and the cached compile have them all).
    fn touch_object_catalog(&self, t: usize) {
        use al_builtins::platform::{Provider, Virtual as V};
        let o = &self.ws.objects[t];
        if !self.ws.platform_table(t) {
            return;
        }
        let Some(Provider::Virtual(v)) = o.id.and_then(al_builtins::platform::provider) else {
            return;
        };
        let kinds: &[ObjKind] = match v {
            V::Field | V::Key | V::TableRelations => &[ObjKind::Table],
            V::ReportMetadata | V::ReportDataItems => &[ObjKind::Report, ObjKind::Table],
            V::PageMetadata | V::PageControlField => &[ObjKind::Page, ObjKind::Table],
            V::PageAction => &[ObjKind::Page, ObjKind::Table, ObjKind::Codeunit],
            V::EventSubscription => &[ObjKind::Codeunit],
            _ => return,
        };
        for &kind in kinds {
            let first = self.demand.borrow_mut().as_mut().is_some_and(|st| {
                let bit = 1u8 << (kind as u8 % 8);
                let first = st.catalogs & bit == 0;
                st.catalogs |= bit;
                first
            });
            if first {
                for i in 0..self.ws.objects.len() {
                    if self.ws.objects[i].kind == kind && self.ws.is_dependency(i) && self.ws.usable(i) {
                        self.raise(i, SCHEMA);
                    }
                }
            }
        }
    }

    /// Queues routine `offset` of object `object` to be checked, if it is a dependency's.
    fn reach(&self, object: usize, offset: usize) {
        if !self.ws.is_dependency(object) {
            return;
        }
        self.touch(object);
        if let Some(st) = self.demand.borrow_mut().as_mut() {
            if st.reached.insert((object, offset)) {
                st.queue.push((object, offset));
            }
        }
    }

    /// The install baseline's roots (ADR-013 decision 5): every install codeunit of a dependency
    /// app, with its install triggers, Base Application's `Company-Initialize`, and the `OnRun` of every dependency
    /// codeunit in `company_data` (the codeunits that build the company's data after it).
    pub(super) fn reach_baseline(&self, company_data: &[u32]) {
        for i in 0..self.ws.objects.len() {
            let o = &self.ws.objects[i];
            if !self.ws.is_dependency(i) || o.kind != ObjKind::Codeunit {
                continue;
            }
            // Cheap facts first: `usable` parses the object's file, and most codeunits are no root.
            let company_initialize = (o.id == Some(COMPANY_INITIALIZE)
                && o.name.eq_ignore_ascii_case("Company-Initialize"))
                || o.id.is_some_and(|id| company_data.contains(&id));
            if !(o.install || company_initialize) || !self.ws.usable(i) {
                continue;
            }
            if o.install {
                self.touch_full(i);
                for r in self.routines_of(i).iter() {
                    if r.is_trigger()
                        && (r.name().eq_ignore_ascii_case("OnInstallAppPerCompany")
                            || r.name().eq_ignore_ascii_case("OnInstallAppPerDatabase"))
                    {
                        self.reach(i, usize::from(r.syntax().text_range().start()));
                    }
                }
            } else if company_initialize {
                self.reach_run(i);
            }
        }
    }

    /// The roots the install run adds (ADR-046 step A): with `baseline`, a dependency app's subscribers to the install
    /// events of `Extension Triggers`, which run with its install codeunits; with `install_code` (an app under test has
    /// install code that runs), the dependency apps' subscribers to `OnCompanyOpen` and `OnCompanyOpenCompleted` of
    /// `Company Triggers`, which the install session raises before the per-company install code.
    pub(super) fn reach_install_events(&self, baseline: bool, install_code: bool) {
        if baseline {
            for (o, k) in self.ws.extension_triggers_subscribers() {
                let event = &self.ws.objects[o].subscribers[k].event;
                if self.ws.is_dependency(o)
                    && (event.eq_ignore_ascii_case("OnInstallAppPerCompany")
                        || event.eq_ignore_ascii_case("OnInstallAppPerDatabase"))
                {
                    self.reach_subscriber(o, k);
                }
            }
        }
        if install_code {
            for (o, k) in self.ws.company_triggers_subscribers() {
                let event = &self.ws.objects[o].subscribers[k].event;
                if self.ws.is_dependency(o)
                    && (event.eq_ignore_ascii_case("OnCompanyOpen")
                        || event.eq_ignore_ascii_case("OnCompanyOpenCompleted"))
                {
                    self.reach_subscriber(o, k);
                }
            }
        }
    }

    /// The codeunits the test runner runs around every test method ([`TEST_RUNNER_CODEUNITS`]): their OnRun is a root
    /// whenever a dependency app holds them, as nothing in the apps names them.
    pub(super) fn reach_test_runner(&self) {
        for i in 0..self.ws.objects.len() {
            let o = &self.ws.objects[i];
            if o.kind == ObjKind::Codeunit
                && self.ws.is_dependency(i)
                && o.id.is_some_and(|id| TEST_RUNNER_CODEUNITS.contains(&id))
            {
                self.reach_run(i);
            }
        }
    }

    /// Reaches the procedures the routine just checked calls.
    pub(super) fn take_calls(&mut self) {
        let calls = std::mem::take(&mut self.calls);
        if self.demand.borrow().is_some() {
            for (object, offset) in calls {
                self.reach(object, offset);
            }
        }
    }

    /// `Codeunit.Run` on object `i`, or `Codeunit::"X"` written where it can be run: its
    /// `OnRun` trigger runs.
    pub(super) fn reach_run(&self, i: usize) {
        if self.demand.borrow().is_none() || !self.ws.is_dependency(i) || !self.ws.usable(i) {
            return;
        }
        self.touch_full(i);
        for r in self.routines_of(i).iter() {
            if r.is_trigger() && r.name().eq_ignore_ascii_case("OnRun") {
                self.reach(i, usize::from(r.syntax().text_range().start()));
            }
        }
    }

    fn routines_of(&self, i: usize) -> Rc<Vec<ast::Routine>> {
        if let Some(st) = self.demand.borrow().as_ref() {
            if let Some(r) = st.routines.get(&i) {
                return r.clone();
            }
        }
        let routines = Rc::new(ast::routines_in(self.ws.object_ast(i).syntax()));
        if let Some(st) = self.demand.borrow_mut().as_mut() {
            st.routines.insert(i, routines.clone());
        }
        routines
    }

    fn reach_all(&self, i: usize) {
        for r in self.routines_of(i).iter() {
            self.reach(i, usize::from(r.syntax().text_range().start()));
        }
    }

    fn reach_triggers(&self, i: usize) {
        for r in self.routines_of(i).iter() {
            if r.is_trigger() {
                self.reach(i, usize::from(r.syntax().text_range().start()));
            }
        }
    }

    /// Reaches the routines `pages::expand` generated for control properties (names start `__`).
    fn reach_synthetic(&self, i: usize) {
        for r in self.routines_of(i).iter() {
            if r.name().starts_with("__") {
                self.reach(i, usize::from(r.syntax().text_range().start()));
            }
        }
    }

    /// Reaches the `k`-th event subscriber routine of object `o`.
    fn reach_subscriber(&self, o: usize, k: usize) {
        if !self.ws.usable(o) {
            return;
        }
        self.touch(o);
        let routines = self.routines_of(o);
        let routine = routines
            .iter()
            .filter(|r| r.attributes().any(|a| a.name().eq_ignore_ascii_case("EventSubscriber")))
            .nth(k);
        if let Some(r) = routine {
            self.reach(o, usize::from(r.syntax().text_range().start()));
        }
    }

    /// What using object `i` at `level` sets off.
    fn activate(&mut self, i: usize, level: u8) {
        if !self.ws.usable(i) {
            return;
        }
        let kind = self.ws.objects[i].kind;
        // An extension in use needs what it extends; an extended object needs its extensions
        // (their fields, enum values and controls are part of it).
        if let Some(t) = self.ws.extension_target(i) {
            self.raise(t, level);
        }
        for &e in self.ws.extensions_of(i) {
            self.raise(e, level);
        }
        match kind {
            // A field's CaptionClass expression runs when code reads the field's caption (FieldCaption, a FieldRef's
            // Caption, a page control over the field): `pages::expand` made it a `__captionclass_N` routine of the table or
            // table extension that writes it (pageext/lt-page-ext-properties/TableExtCaptionClass.al:56-74).
            ObjKind::Table => {
                self.activate_table(i, level);
                if level >= READ {
                    self.reach_synthetic(i);
                }
            }
            ObjKind::TableExtension => {
                if level >= READ {
                    self.reach_synthetic(i);
                }
            }
            // The test client drives a page: any of its routines can run, and so can the
            // operations on its table, so all of them are reached (the call graph the lowering
            // builds for a TestPage assumes the same).
            ObjKind::Page | ObjKind::PageExtension => {
                self.activate_page_shape(i);
                if level == FULL {
                    self.reach_all(i);
                    self.activate_source_table(i);
                    self.activate_page_events(i);
                }
            }
            ObjKind::Report | ObjKind::ReportExtension | ObjKind::XmlPort if level == FULL => {
                self.reach_triggers(i);
                // The test client evaluates the request page's expression-valued properties
                // (`Visible = Flag;`), which `pages::expand` turned into `__visible_N` routines.
                self.reach_synthetic(i);
                self.activate_source_table(i);
                self.activate_page_events(i);
            }
            ObjKind::Query => {
                if level == FULL {
                    self.reach_triggers(i);
                }
            }
            ObjKind::Enum | ObjKind::EnumExtension => {
                if level == FULL {
                    self.activate_enum(i);
                }
            }
            ObjKind::Interface if level == FULL => self.activate_interface(i),
            _ => {}
        }
    }

    fn activate_table(&mut self, t: usize, level: u8) {
        let file = self.ws.objects[t].file;
        let fields = self.members[t].fields.clone();
        for f in &fields {
            let decl_file = f.declared_in.map_or(file, |d| self.ws.objects[d].file);
            // A field of an enum type lowers to the enum; without it the field would go untyped.
            if let Ty::Enum(name) = &f.ty {
                self.resolve_at(decl_file, ObjKind::Enum, name);
            }
            if level < READ {
                continue;
            }
            // Validating and calculating read the tables a field relates to.
            if let Some((_, text)) = f
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("TableRelation"))
            {
                for arm in crate::relation::parse(text).unwrap_or_default() {
                    for n in [arm.path.len(), arm.path.len().saturating_sub(1)] {
                        if n == 0 {
                            continue;
                        }
                        if let Resolution::Found(r) =
                            self.ws
                                .resolve_relation_target(decl_file, ObjKind::Table, &arm.path[..n])
                        {
                            self.touch_schema(r);
                            break;
                        }
                    }
                }
            }
            if let Some(text) = &f.calc_formula {
                if let Ok(formula) = crate::calcformula::parse(text) {
                    if let Resolution::Found(r) = self.ws.resolve_ref(decl_file, ObjKind::Table, &formula.table_path) {
                        self.touch_schema(r);
                    }
                }
            }
        }
        // The table's events can fire whatever reached code does (a rename reaches related
        // tables, a RecordRef writes without a typed variable). Subscribers not reached
        // by an operation are lowered as stubs, so that if one does fire it refuses.
        for (o, _) in self.ws.subscribers_of(t) {
            self.touch_schema(o);
        }
    }

    /// Table `t` is written to (`ops`): reach the triggers and the subscribers those operations run.
    fn activate_ops(&mut self, t: usize, ops: u8) {
        if !self.ws.usable(t) {
            return;
        }
        for (op, trigger, events) in OP_TRIGGERS {
            if ops & op == 0 {
                continue;
            }
            for o in std::iter::once(t).chain(self.ws.extensions_of(t).iter().copied()) {
                if !self.ws.usable(o) {
                    continue;
                }
                self.touch_ops_object(o);
                for r in self.routines_of(o).iter() {
                    // A table extension's `modify(Field)` triggers bracket a field's OnValidate.
                    let ext_field_trigger = op == VALIDATE
                        && (r.name().eq_ignore_ascii_case("OnBeforeValidate")
                            || r.name().eq_ignore_ascii_case("OnAfterValidate"));
                    // A table extension also has `OnBeforeInsert` and `OnAfterInsert` around the
                    // table's own trigger (and so for Modify, Delete and Rename).
                    let around = o != t
                        && op != VALIDATE
                        && ["OnBefore", "OnAfter"].iter().any(|p| {
                            let name = r.name();
                            name.get(..p.len()).is_some_and(|h| h.eq_ignore_ascii_case(p))
                                && name
                                    .get(p.len()..)
                                    .is_some_and(|t| t.eq_ignore_ascii_case(&trigger[2..]))
                        });
                    if r.is_trigger() && (r.name().eq_ignore_ascii_case(trigger) || ext_field_trigger || around) {
                        self.reach(o, usize::from(r.syntax().text_range().start()));
                    }
                }
            }
            for (o, k) in self.ws.subscribers_of(t) {
                let event = &self.ws.objects[o].subscribers[k].event;
                if events.iter().any(|e| event.eq_ignore_ascii_case(e)) {
                    self.reach_subscriber(o, k);
                }
            }
        }
    }

    /// Writes (`ops`) raise the events of the platform codeunit `Global Triggers`: reach the
    /// subscribers in dependency apps. The setup events are asked for every table; the others
    /// are those the runtime raises for the operation (the global events of Insert, Modify and
    /// Delete are not published by BC, record/TestTriggerDispatchOrder.al).
    fn activate_global(&mut self, ops: u8) {
        let mut events: Vec<&str> = vec!["getglobaltabletriggermask", "getdatabasetabletriggersetup"];
        for (op, names) in [
            (INSERT, &["ondatabaseinsert"][..]),
            (MODIFY, &["ondatabasemodify"][..]),
            (DELETE, &["ondatabasedelete"][..]),
            (RENAME, &["onglobalrename", "ondatabaserename"][..]),
        ] {
            if ops & op != 0 {
                events.extend(names.iter().copied());
            }
        }
        for (o, k) in self.ws.global_trigger_subscribers() {
            let event = &self.ws.objects[o].subscribers[k].event;
            if events.iter().any(|e| event.eq_ignore_ascii_case(e)) {
                self.reach_subscriber(o, k);
            }
        }
    }

    /// An extension whose triggers run is used.
    fn touch_ops_object(&self, o: usize) {
        self.raise(o, FULL);
    }

    /// A page (or the request page of a report) the test client opens raises the platform's page
    /// events, and resolves its `CaptionClass` properties through `OnResolveCaptionClass`. Reach
    /// the subscribers of the events the client raises (`al_ir::RAISED_PAGE_EVENTS`) and the
    /// caption class resolvers. A subscriber to any other event of the page is not reached: the
    /// runtime refuses to open the page (`check_page_events`).
    fn activate_page_events(&mut self, i: usize) {
        for (o, k) in self.ws.subscribers_of(i) {
            let event = &self.ws.objects[o].subscribers[k].event;
            if ir::RAISED_PAGE_EVENTS.iter().any(|e| event.eq_ignore_ascii_case(e)) {
                self.reach_subscriber(o, k);
            }
        }
        self.reach_caption_class_if_used(i);
    }

    /// The pages and codeunits that the `RunObject` actions of workspace page `i` run. The test
    /// client opens a page target and runs a codeunit's `OnRun` when the action is invoked.
    /// (Only workspace pages: a dependency page's actions would pull in the app along the links.)
    pub(super) fn reach_run_objects(&self, i: usize) {
        if self.ws.is_dependency(i) {
            return;
        }
        self.reach_run_targets(i);
    }

    /// `reach_run_objects` for any page that code names with a variable: the test drives its
    /// actions, so what they run is reached, and the pages it opens are driven through variables
    /// of their own. The call graph of such a variable holds the same targets (`lower.rs`,
    /// `driven_edges`).
    pub(super) fn reach_run_targets(&self, i: usize) {
        if self.demand.borrow().is_none() || !self.ws.usable(i) {
            return;
        }
        let file = self.ws.objects[i].file;
        for sec in ast::sections_in(self.ws.object_ast(i).syntax()) {
            let Some(text) = sec
                .properties()
                .find(|p| p.name().eq_ignore_ascii_case("RunObject"))
                .map(|p| p.value_text())
            else {
                continue;
            };
            let Ok((kind, parts)) = crate::pages::parse_run_object(&text) else {
                continue;
            };
            match kind {
                ir::RunKind::Codeunit => {
                    if let Some(t) = self.resolve_parts(file, ObjKind::Codeunit, &parts) {
                        self.reach_run(t);
                    }
                }
                ir::RunKind::Page => {
                    if let Some(t) = self.resolve_parts(file, ObjKind::Page, &parts) {
                        self.touch_full(t);
                    }
                }
                _ => {}
            }
        }
    }

    /// Object `i` (a page or report the client can open) declares a `CaptionClass`: the subscribers
    /// to `OnResolveCaptionClass` run when it opens.
    pub(super) fn reach_caption_class_if_used(&self, i: usize) {
        if self.demand.borrow().is_none() {
            return;
        }
        if !ast::text_contains_ignore_case(self.ws.object_ast(i).syntax(), "captionclass") {
            return;
        }
        let first = self.demand.borrow_mut().as_mut().is_some_and(|st| {
            let first = !st.caption_class;
            st.caption_class = true;
            first
        });
        if first {
            for (o, k) in self.ws.caption_class_subscribers() {
                if self.ws.objects[o].subscribers[k]
                    .event
                    .eq_ignore_ascii_case("OnResolveCaptionClass")
                {
                    self.reach_subscriber(o, k);
                }
            }
        }
    }

    /// Object `i` (a report) declares a layout of `Type = Custom`: a render of it raises the platform's
    /// `CustomDocumentMergerEx`, so its subscribers are reached.
    pub(super) fn reach_reporting_triggers_if_used(&self, i: usize) {
        if self.demand.borrow().is_none() || self.ws.objects[i].kind != ObjKind::Report {
            return;
        }
        let syntax = self.ws.object_ast(i).syntax().clone();
        if !ast::text_contains_ignore_case(&syntax, "rendering") || !ast::text_contains_ignore_case(&syntax, "custom") {
            return;
        }
        let first = self.demand.borrow_mut().as_mut().is_some_and(|st| {
            let first = !st.reporting_triggers;
            st.reporting_triggers = true;
            first
        });
        if first {
            for (o, k) in self.ws.reporting_triggers_subscribers() {
                if self.ws.objects[o].subscribers[k]
                    .event
                    .eq_ignore_ascii_case("CustomDocumentMergerEx")
                {
                    self.reach_subscriber(o, k);
                }
            }
        }
    }

    /// Object `i` calls `SaveAs`: an Xml `SaveAs` raises the platform's `SubstituteReport` and `OnDocumentReady`
    /// (events/lt-platform-events-v/PeVTests.al:432), so their subscribers are reached.
    pub(super) fn reach_save_as_events_if_used(&self, i: usize) {
        if self.demand.borrow().is_none() {
            return;
        }
        if !ast::text_contains_ignore_case(self.ws.object_ast(i).syntax(), "saveas") {
            return;
        }
        let first = self.demand.borrow_mut().as_mut().is_some_and(|st| {
            let first = !st.save_as_events;
            st.save_as_events = true;
            first
        });
        if first {
            for (o, k) in self.ws.reporting_triggers_subscribers() {
                let event = &self.ws.objects[o].subscribers[k].event;
                if event.eq_ignore_ascii_case("SubstituteReport") || event.eq_ignore_ascii_case("OnDocumentReady") {
                    self.reach_subscriber(o, k);
                }
            }
        }
    }

    /// Object `i` sets filters from text: a Date text with a piece that does not read raises the platform's
    /// `MakeDateFilter`, so its subscribers are reached (events/lt-platform-triggers/PtObserve.al:157).
    pub(super) fn reach_filter_helper_if_used(&self, i: usize) {
        if self.demand.borrow().is_none() {
            return;
        }
        if !ast::text_contains_ignore_case(self.ws.object_ast(i).syntax(), "setfilter") {
            return;
        }
        let first = self.demand.borrow_mut().as_mut().is_some_and(|st| {
            let first = !st.filter_helper;
            st.filter_helper = true;
            first
        });
        if first {
            for (o, k) in self.ws.filter_helper_triggers_subscribers() {
                if al_ir::RAISED_FILTER_EVENTS
                    .iter()
                    .any(|e| self.ws.objects[o].subscribers[k].event.eq_ignore_ascii_case(e))
                {
                    self.reach_subscriber(o, k);
                }
            }
        }
    }

    /// A page lowers with its source table and the pages it shows as parts; without them it
    /// would be refused as pointing outside the workspace.
    fn activate_page_shape(&mut self, i: usize) {
        let obj = self.ws.object_ast(i);
        let file = self.ws.objects[i].file;
        let source = obj
            .properties()
            .find(|p| p.name().eq_ignore_ascii_case("SourceTable"))
            .map(|p| p.value_text().trim().trim_matches('"').to_string());
        if let Some(t) = source.and_then(|name| self.resolve_quiet(file, ObjKind::Table, &name)) {
            self.touch_schema(t);
        }
        for sec in ast::sections_in(obj.syntax()) {
            if sec.name().eq_ignore_ascii_case("part") {
                if let Some(p) = sec
                    .args()
                    .get(1)
                    .and_then(|n| self.resolve_quiet(file, ObjKind::Page, n))
                {
                    self.touch_schema(p);
                }
            }
        }
    }

    /// A page, report or XML port runs against the table it is built on.
    pub(super) fn activate_source_table(&self, i: usize) {
        let obj = self.ws.object_ast(i);
        let file = self.ws.objects[i].file;
        let source = obj
            .properties()
            .find(|p| p.name().eq_ignore_ascii_case("SourceTable"))
            .map(|p| p.value_text().trim().trim_matches('"').to_string());
        if let Some(name) = source {
            if let Some(t) = self.resolve_at(file, ObjKind::Table, &name) {
                if matches!(self.ws.objects[i].kind, ObjKind::Page | ObjKind::PageExtension) {
                    self.touch_ops(t, ALL_OPS);
                    // A lookup on a control the page gives no OnLookup runs the table field's
                    // (corpus handlers/TestPageFieldTableLookup.al:55).
                    if self.ws.is_dependency(t) && self.ws.usable(t) {
                        for o in std::iter::once(t).chain(self.ws.extensions_of(t).iter().copied()) {
                            if !self.ws.usable(o) {
                                continue;
                            }
                            for r in self.routines_of(o).iter() {
                                if r.is_trigger() && r.name().eq_ignore_ascii_case("OnLookup") {
                                    self.reach(o, usize::from(r.syntax().text_range().start()));
                                }
                            }
                        }
                    }
                }
            }
        }
        for sec in ast::sections_in(obj.syntax()) {
            let n = sec.name().to_ascii_lowercase();
            if matches!(n.as_str(), "dataitem" | "tableelement") {
                if let Some(table) = crate::types::section_object_name(&sec, 1) {
                    self.resolve_at(file, ObjKind::Table, &table);
                }
            }
            // A part opens with its host.
            if n == "part" {
                if let Some(page) = crate::types::section_object_name(&sec, 1) {
                    if let Some(p) = self.resolve_at(file, ObjKind::Page, &page) {
                        self.touch_full(p);
                    }
                }
            }
        }
    }

    /// An enum used through an interface runs the codeunits its values implement.
    fn activate_enum(&mut self, e: usize) {
        let impls = self.members[e].enum_impls.clone();
        for im in impls {
            let file = self.ws.objects[im.declared_in].file;
            let iface = self.ws.resolve_ref(file, ObjKind::Interface, &im.interface);
            let codeunit = self.ws.resolve_ref(file, ObjKind::Codeunit, &im.codeunit);
            if let (Resolution::Found(iface), Resolution::Found(cu)) = (iface, codeunit) {
                self.touch(iface);
                self.touch(cu);
                self.reach_implementation(iface, cu);
            }
        }
    }

    /// Every codeunit that implements interface `iface`, or an interface that extends it, is a
    /// candidate for its calls.
    fn activate_interface(&mut self, iface: usize) {
        for family in self.ws.interface_family(iface) {
            let name = self.ws.objects[family].name.clone();
            let candidates: Vec<usize> = self.ws.implementers_of(&name).to_vec();
            for cu in candidates {
                let o = &self.ws.objects[cu];
                let implements_it = o.implements.iter().any(|parts| {
                    matches!(self.ws.resolve_ref(o.file, ObjKind::Interface, parts), Resolution::Found(i) if i == family)
                });
                if implements_it && o.kind == ObjKind::Codeunit {
                    self.touch(cu);
                    self.reach_implementation(iface, cu);
                }
            }
        }
    }

    /// Reaches the procedures of `cu` that implement procedures of `iface` or of an interface it
    /// extends.
    fn reach_implementation(&self, iface: usize, cu: usize) {
        if !self.ws.usable(cu) {
            return;
        }
        let wanted: Vec<String> = std::iter::once(iface)
            .chain(self.ws.interface_bases(iface))
            .flat_map(|i| self.members[i].procedures.iter().map(|p| p.name.to_lowercase()))
            .collect();
        for r in self.routines_of(cu).iter() {
            if wanted.contains(&r.name().to_lowercase()) {
                self.reach(cu, usize::from(r.syntax().text_range().start()));
            }
        }
    }

    /// Checks one reached routine, and queues what it uses.
    fn check_reached(&mut self, object: usize, offset: usize) {
        if !self.ws.usable(object) {
            return;
        }
        let cached = self.demand.borrow().as_ref().and_then(|s| s.ctxs.get(&object).cloned());
        let ctx = match cached {
            Some(c) => c,
            None => {
                let c = Rc::new(self.object_ctx(object));
                if let Some(st) = self.demand.borrow_mut().as_mut() {
                    st.ctxs.insert(object, c.clone());
                }
                c
            }
        };
        let routines = self.routines_of(object);
        let Some(r) = routines
            .iter()
            .find(|r| usize::from(r.syntax().text_range().start()) == offset)
        else {
            return;
        };
        self.guard((object, offset), |c| c.check_routine_in(&ctx, r));
        self.take_calls();
        // A reached event publisher reaches its subscribers.
        let is_publisher = r
            .attributes()
            .any(|a| EVENT_ATTRS.iter().any(|e| a.name().eq_ignore_ascii_case(e)));
        if is_publisher {
            let name = r.name();
            // Subscribers name the base object of an event an extension publishes.
            let publisher = self.ws.extension_target(object).unwrap_or(object);
            for (o, k) in self.ws.subscribers_of(publisher) {
                if self.ws.objects[o].subscribers[k].event.eq_ignore_ascii_case(&name) {
                    self.reach_subscriber(o, k);
                }
            }
        }
    }

    /// Runs the demand loop to its end and reports what it reached. With `workers > 1` the
    /// reached routines are checked on that many threads (see [`Pool`]); the result is the same.
    pub(super) fn run_demand(&mut self, workers: usize, objects: Vec<usize>) -> Option<Demand> {
        self.demand.borrow().as_ref()?;
        // The stored platform tables are part of every tenant (the Company, the User, the links of
        // a record): compile their shape so the platform can seed and write them.
        for i in 0..self.ws.objects.len() {
            let stored = self.ws.platform_table(i)
                && self.ws.objects[i].id.and_then(al_builtins::platform::provider)
                    == Some(al_builtins::platform::Provider::Stored);
            if stored && self.ws.usable(i) {
                self.raise(i, READ);
            }
        }
        if workers <= 1 {
            self.demand_loop(None, objects);
        } else {
            let (ws, members, catalog) = (self.ws, self.members, self.catalog);
            let enums = self.system_enums.clone();
            let tdd_shared = self.tdd_apps.clone();
            let outs = std::thread::scope(|scope| {
                let mut pool = Pool::default();
                let mut handles = Vec::new();
                for _ in 0..workers {
                    let (batch_tx, batch_rx) = std::sync::mpsc::channel::<std::sync::Arc<RoundWork>>();
                    let (delta_tx, delta_rx) = std::sync::mpsc::channel::<Delta>();
                    let enums = enums.clone();
                    let tdd_apps = tdd_shared.clone();
                    let handle = std::thread::Builder::new()
                        .stack_size(WORKER_STACK)
                        .spawn_scoped(scope, move || {
                            worker(ws, members, catalog, enums, tdd_apps, batch_rx, delta_tx)
                        })
                        .expect("spawn a check worker");
                    pool.senders.push(batch_tx);
                    pool.receivers.push(delta_rx);
                    handles.push(handle);
                }
                self.demand_loop(Some(&pool), objects);
                // Closing the channels ends the workers.
                drop(pool);
                handles
                    .into_iter()
                    .map(|h| h.join().expect("a check worker panicked"))
                    .collect::<Vec<WorkerOut>>()
            });
            let mut types = vec![std::mem::take(&mut self.types)];
            let mut resolutions = vec![std::mem::take(&mut self.resolutions)];
            for out in outs {
                self.panicked.extend(out.panicked);
                self.diags.extend(out.diags);
                types.push(out.types);
                resolutions.push(out.resolutions);
            }
            self.types = PosMap::union(types);
            self.resolutions = PosMap::union(resolutions);
        }
        let st = self.demand.borrow_mut().take()?;
        Some(Demand {
            level: st.level,
            reached: st.reached,
        })
    }

    /// Checks every routine of `objects` on `workers` threads, each with a checker of its own, and
    /// folds what they found into this one. Nothing is reached on demand: this is the complete
    /// compile of those objects (the eager compile, and the objects a cache does not supply). A
    /// routine's result does not depend on which thread checks it, so the result is the one a
    /// single thread gives.
    pub(super) fn check_objects_parallel(&mut self, workers: usize, objects: &[usize]) {
        let (ws, members, catalog) = (self.ws, self.members, self.catalog);
        let enums = self.system_enums.clone();
        let tdd_apps = &self.tdd_apps;
        let next = std::sync::atomic::AtomicUsize::new(0);
        let outs: Vec<WorkerOut> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..workers)
                .map(|_| {
                    let enums = enums.clone();
                    let next = &next;
                    std::thread::Builder::new()
                        .stack_size(WORKER_STACK)
                        .spawn_scoped(scope, move || {
                            let mut c = Checker::new(ws, members, catalog, enums, false);
                            c.tdd_apps = tdd_apps.clone();
                            loop {
                                let k = next.fetch_add(OBJECT_CHUNK, std::sync::atomic::Ordering::Relaxed);
                                if k >= objects.len() {
                                    break;
                                }
                                for &oi in &objects[k..(k + OBJECT_CHUNK).min(objects.len())] {
                                    c.guard((oi, ANY_ROUTINE), |c| c.check_object(oi));
                                    c.take_calls();
                                }
                            }
                            WorkerOut {
                                panicked: c.panicked,
                                diags: c.diags,
                                types: c.types,
                                resolutions: c.resolutions,
                            }
                        })
                        .expect("spawn a check worker")
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("a check worker panicked"))
                .collect()
        });
        let mut types = vec![std::mem::take(&mut self.types)];
        let mut resolutions = vec![std::mem::take(&mut self.resolutions)];
        for out in outs {
            self.panicked.extend(out.panicked);
            self.diags.extend(out.diags);
            types.push(out.types);
            resolutions.push(out.resolutions);
        }
        self.types = PosMap::union(types);
        self.resolutions = PosMap::union(resolutions);
    }

    /// The fixpoint, a round at a time: the first round checks the workspace's `objects`; each
    /// round after it activates what the one before raised and checks what it reached, until a
    /// round has nothing left to do. The order within a round does not matter (everything only
    /// ever grows), so a round is spread over the worker threads.
    fn demand_loop(&mut self, pool: Option<&Pool>, objects: Vec<usize>) {
        let mut round: Vec<Task> = objects.into_iter().map(Task::Object).collect();
        loop {
            {
                let mut guard = self.demand.borrow_mut();
                let Some(st) = guard.as_mut() else { return };
                st.apply_recref(self.ws);
                round.extend(
                    std::mem::take(&mut st.fresh)
                        .into_iter()
                        .map(|(i, l)| Task::Activate(i, l)),
                );
                round.extend(
                    std::mem::take(&mut st.fresh_ops)
                        .into_iter()
                        .map(|(t, o)| Task::Ops(t, o)),
                );
                let global = std::mem::take(&mut st.fresh_global);
                if global != 0 {
                    round.push(Task::Global(global));
                }
                round.extend(
                    std::mem::take(&mut st.queue)
                        .into_iter()
                        .map(|(o, off)| Task::Check(o, off)),
                );
            }
            if round.is_empty() {
                break;
            }
            self.run_round(std::mem::take(&mut round), pool);
        }
    }

    /// Does one task on this thread.
    fn run_task(&mut self, task: Task) {
        match task {
            Task::Object(oi) => self.guard((oi, ANY_ROUTINE), |c| {
                c.check_object(oi);
                c.take_calls();
            }),
            Task::Check(o, off) => self.guard((o, off), |c| c.check_reached(o, off)),
            Task::Activate(i, level) => self.guard((i, ANY_ROUTINE), |c| c.activate(i, level)),
            Task::Ops(t, ops) => self.guard((t, ANY_ROUTINE), |c| c.activate_ops(t, ops)),
            Task::Global(ops) => self.activate_global(ops),
        }
    }

    /// Does the tasks of one round. They are cut into chunks, the tasks about one object together
    /// (so its context is built once), and the workers take chunks until none is left.
    fn run_round(&mut self, mut tasks: Vec<Task>, pool: Option<&Pool>) {
        let Some(pool) = pool.filter(|p| tasks.len() >= PARALLEL_MIN && !p.senders.is_empty()) else {
            for task in tasks {
                self.run_task(task);
            }
            return;
        };
        tasks.sort_unstable_by_key(Task::order);
        let mut chunks: Vec<Vec<Task>> = Vec::new();
        let mut at = 0;
        while at < tasks.len() {
            let object = tasks[at].object();
            let mut end = at + 1;
            while end < tasks.len() && end - at < CHUNK_MAX && tasks[end].object() == object {
                end += 1;
            }
            chunks.push(tasks[at..end].to_vec());
            at = end;
        }
        let work = std::sync::Arc::new(RoundWork {
            chunks,
            next: std::sync::atomic::AtomicUsize::new(0),
        });
        for sender in &pool.senders {
            sender.send(work.clone()).expect("a check worker stopped");
        }
        // Applied in worker order, whatever order they finish in.
        for receiver in &pool.receivers {
            let delta = receiver.recv().expect("a check worker stopped");
            self.apply(delta);
        }
    }

    /// What a worker's checks reached and raised, applied to this checker's state.
    fn apply(&self, d: Delta) {
        for (i, level) in d.raised {
            self.raise(i, level);
        }
        for (i, ops) in d.ops {
            self.touch_ops(i, ops);
        }
        self.touch_global_ops(d.global);
        self.touch_recref_ops(d.recref);
        if let Some(st) = self.demand.borrow_mut().as_mut() {
            for r in d.reached {
                if st.reached.insert(r) {
                    st.queue.push(r);
                }
            }
        }
    }
}

/// One piece of work of a round of the demand loop.
#[derive(Clone, Copy, Debug)]
enum Task {
    /// Check the routines of a workspace object (its code is the root of the demand).
    Object(usize),
    /// Check the reached routine at this offset of this dependency object.
    Check(usize, usize),
    /// An object was raised to this level: reach what using it at that level sets off.
    Activate(usize, u8),
    /// A table is written to with these operations: reach the triggers and subscribers.
    Ops(usize, u8),
    /// These operations raise the events of `Global Triggers`.
    Global(u8),
}

impl Task {
    /// The object the task is about.
    fn object(&self) -> usize {
        match *self {
            Task::Object(o) | Task::Check(o, _) | Task::Activate(o, _) | Task::Ops(o, _) => o,
            Task::Global(_) => 0,
        }
    }

    /// A deterministic order for the tasks of a round, whatever order they were found in.
    fn order(&self) -> (usize, u8, usize, u8) {
        match *self {
            Task::Object(o) => (o, 0, 0, 0),
            Task::Activate(o, l) => (o, 1, 0, l),
            Task::Ops(o, ops) => (o, 2, 0, ops),
            Task::Check(o, off) => (o, 3, off, 0),
            Task::Global(g) => (0, 4, 0, g),
        }
    }
}

/// The chunks of one round, shared by the workers, which take them in turn.
struct RoundWork {
    chunks: Vec<Vec<Task>>,
    next: std::sync::atomic::AtomicUsize,
}

/// The most tasks in a chunk: a bigger object is split over workers.
const CHUNK_MAX: usize = 32;

/// A round with fewer tasks than this is done on the calling thread.
const PARALLEL_MIN: usize = 48;

/// Objects a thread of `check_objects_parallel` takes at a time.
const OBJECT_CHUNK: usize = 2;

/// Stack of a check worker: the checker recurses on the nesting of the code it reads.
const WORKER_STACK: usize = 64 << 20;

/// The channels to the check workers of a parallel demand run. Routines reached in one round
/// are sent out, each worker answers with the [`Delta`] its checks produced, and the calling
/// thread applies the deltas and activates what they raised (the fixpoint stays on one thread;
/// only the checking of routines, the bulk of the work, is spread out).
#[derive(Default)]
struct Pool {
    senders: Vec<std::sync::mpsc::Sender<std::sync::Arc<RoundWork>>>,
    receivers: Vec<std::sync::mpsc::Receiver<Delta>>,
}

/// What a worker's checks asked for since its last answer.
struct Delta {
    /// Objects raised to a level, in the order they were.
    raised: Vec<(usize, u8)>,
    /// Write operations reached on a table.
    ops: Vec<(usize, u8)>,
    /// Write operations whose `Global Triggers` subscribers are reached.
    global: u8,
    /// `RecordRef` write operations.
    recref: u8,
    /// Routines reached.
    reached: Vec<(usize, usize)>,
}

impl State {
    /// `RecordRef` writes (see `touch_recref_ops`): the tables that are compiled run their
    /// operations. Called with the state of the calling thread, before a round is cut.
    fn apply_recref(&mut self, ws: &Workspace) {
        self.fresh_recref = 0;
        if self.recref_ops == 0 {
            return;
        }
        for i in 0..self.level.len() {
            if self.level[i] < READ || ws.objects[i].kind != ObjKind::Table || !ws.is_dependency(i) {
                continue;
            }
            let new = self.recref_ops & !self.ops[i];
            if new != 0 {
                self.ops[i] |= new;
                self.fresh_ops.push((i, new));
                if self.level[i] < FULL {
                    self.level[i] = FULL;
                    self.fresh.push((i, FULL));
                }
            }
        }
    }

    fn take_delta(&mut self) -> Delta {
        Delta {
            raised: std::mem::take(&mut self.fresh),
            ops: std::mem::take(&mut self.fresh_ops),
            global: std::mem::take(&mut self.fresh_global),
            recref: std::mem::take(&mut self.fresh_recref),
            reached: std::mem::take(&mut self.queue),
        }
    }
}

/// What a worker leaves behind when its channel closes.
struct WorkerOut {
    panicked: HashMap<(usize, usize), String>,
    diags: Vec<crate::workspace::Diagnostic>,
    types: PosMap<crate::types::Ty>,
    resolutions: PosMap<Res>,
}

/// A check worker: its own checker, with its own caches and a state that only records what the
/// checks reach, so the calling thread can fold it into the real one.
fn worker(
    ws: &Workspace,
    members: &crate::members::MemberTable<'_>,
    catalog: &al_builtins::catalog::Catalog,
    enums: HashSet<String>,
    tdd_apps: Option<std::sync::Arc<Vec<bool>>>,
    batches: std::sync::mpsc::Receiver<std::sync::Arc<RoundWork>>,
    deltas: std::sync::mpsc::Sender<Delta>,
) -> WorkerOut {
    let mut c = Checker::new(ws, members, catalog, enums, true);
    c.tdd_apps = tdd_apps;
    while let Ok(work) = batches.recv() {
        loop {
            let k = work.next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let Some(chunk) = work.chunks.get(k) else { break };
            for &task in chunk {
                c.run_task(task);
            }
        }
        let delta = c
            .demand
            .borrow_mut()
            .as_mut()
            .map(State::take_delta)
            .expect("demand state");
        if deltas.send(delta).is_err() {
            break;
        }
    }
    WorkerOut {
        panicked: c.panicked,
        diags: c.diags,
        types: c.types,
        resolutions: c.resolutions,
    }
}

/// The event subscribers of dependency objects that are declared but not compiled, for events of
/// objects (and platform codeunits) that are. BC would run them when the event is raised; the
/// program has no code for them, so the runtime refuses to raise such an event (hard rule 4).
/// Manual subscribers are left out: they fire only on an instance somebody bound, and that
/// instance is a compiled codeunit.
pub fn uncompiled_subscribers(ws: &Workspace, active: &dyn Fn(usize) -> bool) -> Vec<ir::UncompiledSubscriber> {
    uncompiled_subscriber_sites(ws, active)
        .into_iter()
        .map(|s| {
            let o = &ws.objects[s.object];
            let sub = &o.subscribers[s.index];
            ir::UncompiledSubscriber {
                publisher: s.publisher.map(|p| p as u32),
                platform: s.platform,
                event: sub.event.to_lowercase(),
                element: sub.element.clone(),
                subscriber: format!("{} \"{}\"", o.kind.label(), o.name),
            }
        })
        .collect()
}

/// One uncompiled subscriber: object `object` declares subscriber `index`, for an event of `publisher` (or of a
/// platform codeunit).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubscriberSite {
    pub publisher: Option<usize>,
    pub platform: ir::PlatformPublisher,
    pub object: usize,
    pub index: usize,
}

/// The sites [`uncompiled_subscribers`] describes, in its order.
pub fn uncompiled_subscriber_sites(ws: &Workspace, active: &dyn Fn(usize) -> bool) -> Vec<SubscriberSite> {
    let mut out = Vec::new();
    let mut seen = HashSet::default();
    let mut add = |publisher: Option<usize>, platform: ir::PlatformPublisher, so: usize, si: usize| {
        let o = &ws.objects[so];
        if !ws.is_dependency(so) || active(so) || o.manual_subscribers {
            return;
        }
        if !seen.insert((publisher, platform as u8, so, si)) {
            return;
        }
        out.push(SubscriberSite {
            publisher,
            platform,
            object: so,
            index: si,
        });
    };
    for oi in 0..ws.objects.len() {
        if active(oi) {
            for (so, si) in ws.subscribers_of(oi) {
                add(Some(oi), ir::PlatformPublisher::None, so, si);
            }
        }
    }
    for (so, si) in ws.global_trigger_subscribers() {
        add(None, ir::PlatformPublisher::GlobalTriggers, so, si);
    }
    for (so, si) in ws.caption_class_subscribers() {
        if ws.objects[so].subscribers[si]
            .event
            .eq_ignore_ascii_case("OnResolveCaptionClass")
        {
            add(None, ir::PlatformPublisher::CaptionClass, so, si);
        }
    }
    for (so, si) in ws.reporting_triggers_subscribers() {
        if al_ir::RAISED_REPORTING_EVENTS
            .iter()
            .any(|e| ws.objects[so].subscribers[si].event.eq_ignore_ascii_case(e))
        {
            add(None, ir::PlatformPublisher::ReportingTriggers, so, si);
        }
    }
    for (so, si) in ws.filter_helper_triggers_subscribers() {
        if al_ir::RAISED_FILTER_EVENTS
            .iter()
            .any(|e| ws.objects[so].subscribers[si].event.eq_ignore_ascii_case(e))
        {
            add(None, ir::PlatformPublisher::FilterHelperTriggers, so, si);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::lower::{Lowered, compile_apps};
    use crate::workspace::{AppKind, AppSource, Workspace};

    /// A workspace app `main` over a dependency app `dep`, compiled with demand-driven dependencies.
    fn compile(main: &str, dep: &[(&str, &str)]) -> (Workspace, Lowered) {
        let app = |id: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files,
        };
        compile_apps(
            vec![
                app("main", &["dep"], vec![("main.al".into(), main.into())]),
                app(
                    "dep",
                    &[],
                    dep.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
                ),
            ],
            vec![AppKind::Workspace, AppKind::Dependency],
            &Default::default(),
        )
    }

    fn object(l: &Lowered, name: &str) -> usize {
        l.program
            .objects
            .iter()
            .position(|o| o.name == name)
            .unwrap_or_else(|| panic!("no object {name}"))
    }

    /// The reasons routine `name` of object `obj` cannot run (empty: it runs).
    fn reasons(l: &Lowered, obj: &str, name: &str) -> Vec<String> {
        let o = object(l, obj);
        let r = l.program.objects[o]
            .routines
            .iter()
            .position(|r| r.name == name)
            .unwrap_or_else(|| panic!("no routine {name}"));
        l.unsupported[o][r].clone()
    }

    fn compiled(l: &Lowered, obj: &str, name: &str) -> bool {
        reasons(l, obj, name).iter().all(|r| !r.contains("not compiled"))
    }

    /// The program, the refusals, the call graph and the diagnostics of a compile of `main` over
    /// `dep` with `workers` check threads.
    fn compile_with_workers(main: &str, dep: &[(String, String)], workers: usize) -> String {
        compile_with_both(main, dep, workers, workers)
    }

    /// Like `compile_with_workers`, with `check` threads checking and `lower` threads lowering.
    fn compile_with_both(main: &str, dep: &[(String, String)], check: usize, lower: usize) -> String {
        let app = |id: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files,
        };
        let roots = crate::check::Roots {
            workers: Some(check),
            ..Default::default()
        };
        let ws = Workspace::load_apps(
            vec![
                app("main", &["dep"], vec![("main.al".into(), main.into())]),
                app("dep", &[], dep.to_vec()),
            ],
            vec![AppKind::Workspace, AppKind::Dependency],
            &Default::default(),
        );
        let members = crate::members::build(&ws);
        let checked = crate::check::check_with(&ws, &members, &al_builtins::catalog::Catalog::load(), roots);
        let l = crate::lower::lower_with_workers(&ws, &members, &checked, true, lower);
        format!(
            "{}\n{:?}\n{:?}\n{:?}",
            serde_json::to_string(&*l.program).unwrap(),
            l.unsupported,
            l.calls,
            ws.diagnostics
        )
    }

    #[test]
    fn checking_on_several_threads_compiles_the_same_program() {
        // 80 libraries reached in one round (more than a round needs to be spread over threads),
        // each calling a helper in the next round, one of which cannot run.
        let n = 80;
        let main = format!(
            "codeunit 1 Main {{ procedure Go() var {} begin {} end; }}",
            (0..n).map(|i| format!("L{i}: Codeunit Lib{i};")).collect::<String>(),
            (0..n).map(|i| format!("L{i}.Entry();")).collect::<String>()
        );
        let dep: Vec<(String, String)> = (0..n)
            .map(|i| {
                let broken = if i % 7 == 0 { "T := NoSuchBuiltin(1);" } else { "" };
                (
                    format!("lib{i}.al"),
                    format!(
                        "codeunit {} Lib{i} {{ procedure Entry() begin Helper(); end; \
                         local procedure Helper() var T: Text; begin {broken} end; \
                         procedure Unused() begin end; }}",
                        100 + i
                    ),
                )
            })
            .collect();
        let one = compile_with_workers(&main, &dep, 1);
        assert!(one.contains("Helper"), "the fixture compiles something");
        assert_eq!(one, compile_with_workers(&main, &dep, 4));
        assert_eq!(one, compile_with_workers(&main, &dep, 7));
        // Lowering the dependency objects on several threads does not change the program either.
        assert_eq!(one, compile_with_both(&main, &dep, 1, 5));
    }

    #[test]
    fn only_the_dependency_routines_the_workspace_calls_are_compiled() {
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var D: Codeunit Lib; begin D.Used(); end; }",
            &[(
                "lib.al",
                "codeunit 10 Lib { procedure Used() begin Inner(); end; local procedure Inner() begin end; procedure Unused() begin Other(); end; local procedure Other() begin end; }",
            )],
        );
        assert!(compiled(&l, "Lib", "Used"));
        assert!(compiled(&l, "Lib", "Inner"), "what a reached routine calls is reached");
        assert!(!compiled(&l, "Lib", "Unused"));
        assert!(!compiled(&l, "Lib", "Other"));
        // The stub says why it cannot run.
        assert!(reasons(&l, "Lib", "Unused")[0].contains("not compiled"));
    }

    #[test]
    fn a_dependency_report_request_page_expression_is_reached_by_a_test_request_page() {
        // handlers/TestReqPageBoundVisibility.al:172-188: the handler reads `.Visible()` of a control
        // whose `Visible = FlagVisible;` is a synthetic `__visible_N` routine of Base App's report.
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var R: TestRequestPage \"Lib Rep\"; begin end; }",
            &[(
                "lib.al",
                "report 10 \"Lib Rep\" { requestpage { layout { area(content) { field(Name; Nm) { Visible = FlagVisible; } } } } var Nm: Text; FlagVisible: Boolean; }",
            )],
        );
        let o = object(&l, "Lib Rep");
        let synthetic: Vec<_> = l.program.objects[o]
            .routines
            .iter()
            .filter(|r| r.name.starts_with("__visible_"))
            .map(|r| r.name.clone())
            .collect();
        assert_eq!(
            synthetic.len(),
            1,
            "{:?}",
            l.program.objects[o]
                .routines
                .iter()
                .map(|r| &r.name)
                .collect::<Vec<_>>()
        );
        assert!(compiled(&l, "Lib Rep", &synthetic[0]));
    }

    #[test]
    fn an_unsupported_dependency_routine_nothing_reaches_does_not_block_anything() {
        // `Rare` uses a builtin that does not exist; nothing reaches it, so nothing is refused.
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var D: Codeunit Lib; begin D.Fine(); end; }",
            &[(
                "lib.al",
                "codeunit 10 Lib { procedure Fine() begin end; procedure Rare() var T: Text; begin T := NoSuchBuiltin(1); end; }",
            )],
        );
        assert!(reasons(&l, "Lib", "Fine").is_empty());
        assert!(!compiled(&l, "Lib", "Rare"));
        assert!(reasons(&l, "Main", "Go").is_empty(), "{:?}", reasons(&l, "Main", "Go"));
    }

    #[test]
    fn a_reached_routine_that_cannot_run_blocks_what_reaches_it() {
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var D: Codeunit Lib; begin D.Rare(); end; }",
            &[(
                "lib.al",
                "codeunit 10 Lib { procedure Rare() var T: Text; begin T := NoSuchBuiltin(1); end; }",
            )],
        );
        assert!(compiled(&l, "Lib", "Rare"));
        assert!(!reasons(&l, "Lib", "Rare").is_empty());
        let go = l.program.objects[object(&l, "Main")]
            .routines
            .iter()
            .position(|r| r.name == "Go")
            .unwrap();
        let lib = object(&l, "Lib");
        let rare = l.program.objects[lib]
            .routines
            .iter()
            .position(|r| r.name == "Rare")
            .unwrap();
        assert!(l.calls[object(&l, "Main")][go].contains(&(lib, rare)));
    }

    #[test]
    fn objects_nothing_refers_to_are_not_compiled_and_stay_out_of_reach() {
        let (ws, l) = compile(
            "codeunit 1 Main { procedure Go() var R: Record Used; begin R.FindFirst(); end; }",
            &[
                ("used.al", "table 10 Used { fields { field(1; K; Code[10]) { } } }"),
                (
                    "other.al",
                    "table 11 Other { fields { field(1; K; Code[10]) { } } trigger OnInsert() begin end; }",
                ),
            ],
        );
        let used = &l.program.objects[object(&l, "Used")];
        let other = &l.program.objects[object(&l, "Other")];
        assert!(used.table.is_some());
        assert!(
            other.table.is_none() && other.routines.is_empty(),
            "an object nothing refers to is a shell"
        );
        // The workspace no longer resolves it either: code that needs it after all is refused.
        let fi = ws.files.iter().position(|f| f.path == "main.al").unwrap();
        assert_eq!(
            ws.resolve_ref(fi, crate::workspace::ObjKind::Table, &["Other".to_string()]),
            crate::workspace::Resolution::Missing
        );
    }

    /// `main` over the platform app (the table `Field`) and an app with the table `Other`.
    fn compile_with_catalog(main: &str) -> (Workspace, Lowered) {
        let app = |id: &str, deps: &[&str], files: Vec<(&str, &str)>| AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: files.into_iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
        };
        compile_apps(
            vec![
                app(
                    "main",
                    &[crate::workspace::PLATFORM_APP_ID, "dep"],
                    vec![("main.al", main)],
                ),
                app(
                    crate::workspace::PLATFORM_APP_ID,
                    &[],
                    vec![(
                        "field.al",
                        "table 2000000041 \"Field\" { fields { field(1; TableNo; Integer) { } field(7; \"Type\"; Integer) { } } }",
                    )],
                ),
                app(
                    "dep",
                    &[],
                    vec![("other.al", "table 11 Other { fields { field(1; K; Code[10]) { } } }")],
                ),
            ],
            vec![AppKind::Workspace, AppKind::Dependency, AppKind::Dependency],
            &Default::default(),
        )
    }

    /// Using the platform table `Field` as a table answers for every table of the installed apps, so
    /// their shapes are compiled. A record variable that only has its fields assigned and read (Library
    /// Assert's TypeNameOf) reads no row and needs none of them.
    #[test]
    fn a_catalog_table_used_as_a_table_compiles_every_table_shape() {
        let (_, l) = compile_with_catalog(
            "codeunit 1 Main { procedure Go() var F: Record \"Field\"; begin F.SetRange(TableNo, 18); end; }",
        );
        assert!(l.program.objects[object(&l, "Other")].table.is_some());
    }

    #[test]
    fn a_catalog_table_variable_used_only_through_its_fields_compiles_no_other_table() {
        let (_, l) = compile_with_catalog(
            "codeunit 1 Main { procedure Go(): Integer var F: Record \"Field\"; begin F.\"Type\" := 3; exit(F.\"Type\"); end; }",
        );
        assert!(
            l.program.objects[object(&l, "Other")].table.is_none(),
            "no row of Field is read, so no table's shape is needed"
        );
    }

    /// A trigger of a page in a dependency app calls a procedure of the page its part shows
    /// (`CurrPage.PurchLines.Page.ForceTotalsCalculation()` in Base Application's Purchase Invoice): the part's
    /// page is reached with the page that names it. Pinned in the workspace by handlers/TestPageTempPart_Host.al
    /// (`CurrPage.Lines.Page.SetRows`). The part's page has a group named `Page` (Base Application's Purch.
    /// Invoice Subform does): `.Page` is still the part's page, not that control.
    #[test]
    fn a_page_part_s_procedure_called_from_a_dependency_page_is_compiled() {
        let dep = [
            (
                "t.al",
                "table 10 T { fields { field(1; K; Integer) { } field(2; V; Integer) { } } keys { key(PK; K) { } } }",
            ),
            (
                "sub.al",
                "namespace Ns.Docs; page 12 \"Sub\" { PageType = ListPart; SourceTable = T; layout { area(Content) { repeater(R) { field(K; Rec.K) { } } group(\"Page\") { } } } \
                 procedure Force() begin end; }",
            ),
            (
                "host.al",
                "namespace Ns.Docs; page 11 \"Host\" { PageType = Card; SourceTable = T; layout { area(Content) { field(V; Rec.V) { \
                 trigger OnValidate() begin AfterValid(); end; } \
                 part(Lines; \"Sub\") { ApplicationArea = Basic, Suite; Editable = IsEditable; Enabled = IsEditable; SubPageLink = K = field(K); UpdatePropagation = Both; } } } \
                 var IsEditable: Boolean; local procedure AfterValid() begin CurrPage.Lines.Page.Force(); end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var TP: TestPage \"Host\"; begin TP.OpenEdit(); TP.V.SetValue(1); end; }",
            &dep,
        );
        let why = reasons(&l, "Host", "AfterValid");
        assert!(why.is_empty(), "{why:?}");
    }

    #[test]
    fn reading_a_table_runs_no_trigger_and_writing_runs_the_trigger_of_that_operation() {
        let dep = [(
            "t.al",
            "table 10 T { fields { field(1; K; Code[10]) { } } \
             trigger OnInsert() begin end; trigger OnModify() begin end; trigger OnDelete() begin end; }",
        )];
        let (_, read) = compile(
            "codeunit 1 Main { procedure Go() var R: Record T; begin R.FindFirst(); end; }",
            &dep,
        );
        for t in ["OnInsert", "OnModify", "OnDelete"] {
            assert!(!compiled(&read, "T", t), "{t} must not run when the table is only read");
        }
        let (_, write) = compile(
            "codeunit 1 Main { procedure Go() var R: Record T; begin R.Insert(true); end; }",
            &dep,
        );
        assert!(compiled(&write, "T", "OnInsert"));
        assert!(
            !compiled(&write, "T", "OnModify"),
            "only the operation used runs its trigger"
        );
        assert!(!compiled(&write, "T", "OnDelete"));
    }

    #[test]
    fn writing_a_table_reaches_its_event_subscribers_for_that_operation_only() {
        let dep = [
            ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
            (
                "s.al",
                "codeunit 20 Subs {\n\
                 [EventSubscriber(ObjectType::Table, Database::T, 'OnAfterInsertEvent', '', false, false)]\n\
                 local procedure OnIns() begin end;\n\
                 [EventSubscriber(ObjectType::Table, Database::T, 'OnAfterModifyEvent', '', false, false)]\n\
                 local procedure OnMod() begin end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var R: Record T; begin R.Insert(); end; }",
            &dep,
        );
        assert!(compiled(&l, "Subs", "OnIns"));
        assert!(!compiled(&l, "Subs", "OnMod"));
        // Reading only: neither subscriber is reached.
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var R: Record T; begin R.FindFirst(); end; }",
            &dep,
        );
        assert!(!compiled(&l, "Subs", "OnIns"));
    }

    #[test]
    fn a_reached_event_publisher_reaches_the_subscribers_of_that_event() {
        let dep = [
            (
                "p.al",
                "codeunit 10 Pub { procedure Do() begin OnThing(); OnOther(); end;\n\
                 [IntegrationEvent(false, false)] local procedure OnThing() begin end;\n\
                 [IntegrationEvent(false, false)] local procedure OnOther() begin end; }",
            ),
            (
                "s.al",
                "codeunit 20 Subs {\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)]\n\
                 local procedure A() begin end;\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnNever', '', false, false)]\n\
                 local procedure B() begin end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var P: Codeunit Pub; begin P.Do(); end; }",
            &dep,
        );
        assert!(compiled(&l, "Subs", "A"));
        assert!(!compiled(&l, "Subs", "B"), "the publisher never raises OnNever");
    }

    #[test]
    fn a_write_anywhere_reaches_the_global_triggers_subscribers_of_that_operation() {
        // `Global Triggers` (2000000002) is a platform codeunit no app declares here; the runtime
        // raises its events around writes, so a dependency's subscribers to them are reached by a
        // write to any table, the workspace's own included.
        let dep = [(
            "s.al",
            "codeunit 20 Subs {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)]\n\
             local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseInsert', '', false, false)]\n\
             local procedure Ins(RecRef: RecordRef) begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseModify', '', false, false)]\n\
             local procedure Mod(RecRef: RecordRef) begin end; }",
        )];
        let (_, l) = compile(
            "table 5 Mine { fields { field(1; K; Code[10]) { } } }\n\
             codeunit 1 Main { procedure Go() var R: Record Mine; begin R.Insert(); end; }",
            &dep,
        );
        assert!(compiled(&l, "Subs", "Setup"));
        assert!(compiled(&l, "Subs", "Ins"));
        assert!(
            !compiled(&l, "Subs", "Mod"),
            "only the operations used raise their events"
        );
        let (_, l) = compile(
            "table 5 Mine { fields { field(1; K; Code[10]) { } } }\n\
             codeunit 1 Main { procedure Go() var R: Record Mine; begin R.FindFirst(); end; }",
            &dep,
        );
        assert!(
            l.program.objects[object(&l, "Subs")].routines.is_empty(),
            "reading raises nothing, so the subscribers are not compiled"
        );
    }

    #[test]
    fn an_interface_used_reaches_the_codeunits_that_implement_it() {
        let dep = [
            ("i.al", "interface IThing { procedure Run(); }"),
            (
                "c.al",
                "codeunit 10 Impl implements IThing { procedure Run() begin Helper(); end; local procedure Helper() begin end; procedure Extra() begin end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var I: Interface IThing; begin I.Run(); end; }",
            &dep,
        );
        assert!(compiled(&l, "Impl", "Run"));
        assert!(compiled(&l, "Impl", "Helper"));
        assert!(
            !compiled(&l, "Impl", "Extra"),
            "only the interface's procedures are called through it"
        );
    }

    #[test]
    fn a_table_related_to_a_used_one_is_lowered_with_it_but_runs_nothing() {
        let dep = [
            (
                "a.al",
                "table 10 A { fields { field(1; K; Code[10]) { } field(2; Ref; Code[10]) { TableRelation = B.K; } } }",
            ),
            (
                "b.al",
                "table 11 B { fields { field(1; K; Code[10]) { } } trigger OnInsert() begin end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var R: Record A; begin R.FindFirst(); end; }",
            &dep,
        );
        let b = &l.program.objects[object(&l, "B")];
        assert!(
            b.table.is_some(),
            "the related table is lowered so the relation resolves"
        );
        assert!(!compiled(&l, "B", "OnInsert"));
        let a = l.program.objects[object(&l, "A")].table.as_ref().unwrap();
        assert!(a.fields[1].relation_arms.iter().all(|arm| arm.table.is_some()));
    }

    #[test]
    fn a_dependency_app_without_a_reference_from_the_workspace_compiles_nothing() {
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() begin end; }",
            &[
                ("lib.al", "codeunit 10 Lib { procedure P() begin end; }"),
                ("t.al", "table 11 T { }"),
            ],
        );
        assert!(l.program.objects[object(&l, "Lib")].routines.is_empty());
        assert!(l.dependency[object(&l, "Lib")] && !l.dependency[object(&l, "Main")]);
    }

    const T_WITH_TRIGGERS: &str = "table 10 T { fields { field(1; K; Code[10]) { } field(2; V; Integer) { } } \
         trigger OnInsert() begin end; trigger OnModify() begin end; trigger OnDelete() begin end; \
         procedure Bump() begin V := 1; Modify(true); end; }";

    #[test]
    fn a_write_without_the_record_name_inside_table_code_runs_the_trigger_of_that_operation() {
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var R: Record T; begin R.Bump(); end; }",
            &[("t.al", T_WITH_TRIGGERS)],
        );
        assert!(
            compiled(&l, "T", "OnModify"),
            "`Modify(true)` inside Bump writes the table"
        );
        assert!(!compiled(&l, "T", "OnDelete"));
    }

    #[test]
    fn a_workspace_page_over_a_dependency_table_runs_what_the_client_writes() {
        let (_, l) = compile(
            "page 5 Mine { PageType = Card; SourceTable = T; layout { area(Content) { field(K; Rec.K) { } } } }",
            &[("t.al", T_WITH_TRIGGERS)],
        );
        for t in ["OnInsert", "OnModify", "OnDelete"] {
            assert!(compiled(&l, "T", t), "{t}");
        }
    }

    #[test]
    fn the_run_object_actions_of_a_workspace_page_run_their_targets() {
        let dep = [(
            "c.al",
            "codeunit 10 Lib { trigger OnRun() begin end; procedure Other() begin end; } \
             codeunit 11 Unused { trigger OnRun() begin end; }",
        )];
        let (_, l) = compile(
            "page 5 Mine { PageType = Card; layout { area(Content) { } } \
             actions { area(Processing) { action(A) { RunObject = Codeunit Lib; } } } }",
            &dep,
        );
        assert!(compiled(&l, "Lib", "OnRun"));
        assert!(!compiled(&l, "Lib", "Other"));
        assert!(l.program.objects[object(&l, "Unused")].routines.is_empty());
    }

    #[test]
    fn a_page_the_client_opens_reaches_the_subscribers_of_the_events_it_raises_and_no_others() {
        let dep = [
            ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
            (
                "p.al",
                "page 20 P { PageType = Card; SourceTable = T; layout { area(Content) { field(K; Rec.K) { } } } }",
            ),
            (
                "s.al",
                "codeunit 30 Subs {\n\
                 [EventSubscriber(ObjectType::Page, Page::P, 'OnOpenPageEvent', '', false, false)]\n\
                 local procedure Opened() begin end;\n\
                 [EventSubscriber(ObjectType::Page, Page::P, 'OnAfterGetRecordEvent', '', false, false)]\n\
                 local procedure Row() begin end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var P: TestPage P; begin P.OpenNew(); end; }",
            &dep,
        );
        assert!(compiled(&l, "Subs", "Opened"));
        assert!(
            !compiled(&l, "Subs", "Row"),
            "the client does not raise it; the page refuses to open"
        );
        // Both are declared: the one nobody compiled is on the program's list, the other is not.
        let events: Vec<&str> = l
            .program
            .uncompiled_subscribers
            .iter()
            .map(|u| u.event.as_str())
            .collect();
        assert!(events.is_empty(), "{events:?}: a stub is registered, not missing");
    }

    #[test]
    fn caption_class_resolvers_are_reached_by_a_workspace_page_that_has_a_caption_class() {
        let resolver = (
            "r.al",
            "codeunit 30 Resolver {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', false, false)]\n\
             local procedure R(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }",
        );
        let with = "table 5 Mine { fields { field(1; K; Code[10]) { } } }\n\
             page 5 Mine { PageType = Card; SourceTable = Mine; layout { area(Content) { \
             field(K; Rec.K) { CaptionClass = 'ZZ,x'; } } } }";
        let (_, l) = compile(with, &[resolver]);
        assert!(compiled(&l, "Resolver", "R"));
        let without = with.replace("CaptionClass = 'ZZ,x';", "");
        let (_, l) = compile(&without, &[resolver]);
        assert!(l.program.objects[object(&l, "Resolver")].routines.is_empty());
        // Not reached, yet declared: the page client refuses to raise the event.
        assert_eq!(l.program.uncompiled_subscribers.len(), 1);
        let u = &l.program.uncompiled_subscribers[0];
        assert_eq!(u.platform, al_ir::PlatformPublisher::CaptionClass);
        assert_eq!(u.event, "onresolvecaptionclass");
    }

    #[test]
    fn an_event_published_by_an_extension_reaches_the_subscribers_of_its_base_object() {
        let dep = [
            ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
            (
                "x.al",
                "tableextension 11 TX extends T { procedure Fire() begin OnThing(); end; \
                 [IntegrationEvent(false, false)] local procedure OnThing() begin end; }",
            ),
            (
                "s.al",
                "codeunit 30 Subs {\n\
                 [EventSubscriber(ObjectType::Table, Database::T, 'OnThing', '', false, false)]\n\
                 local procedure A() begin end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var R: Record T; begin R.Fire(); end; }",
            &dep,
        );
        assert!(compiled(&l, "Subs", "A"));
    }

    #[test]
    fn a_record_ref_write_raises_the_global_events_of_that_operation() {
        let dep = [(
            "s.al",
            "codeunit 20 Subs {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseModify', '', false, false)]\n\
             local procedure Mod(RecRef: RecordRef) begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseDelete', '', false, false)]\n\
             local procedure Del(RecRef: RecordRef) begin end; }",
        )];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var RR: RecordRef; begin RR.Modify(); end; }",
            &dep,
        );
        assert!(compiled(&l, "Subs", "Mod"));
        assert!(!compiled(&l, "Subs", "Del"));
        // The delete subscriber sits in a compiled object, so it is a stub that refuses if it is
        // asked, not a missing one.
        assert!(l.program.uncompiled_subscribers.is_empty());
        assert!(reasons(&l, "Subs", "Del")[0].contains("not compiled"));
    }

    #[test]
    fn running_a_report_or_xmlport_reaches_its_triggers_by_variable_or_by_constant() {
        let dep = [
            (
                "r.al",
                "report 40 Rep { trigger OnPreReport() begin end; procedure Other() begin end; }",
            ),
            (
                "x.al",
                "xmlport 41 Xp { schema { textelement(Root) { } } trigger OnPreXmlPort() begin end; }",
            ),
        ];
        let (_, by_var) = compile(
            "codeunit 1 Main { procedure Go() var R: Report Rep; begin R.Run(); end; }",
            &dep,
        );
        assert!(compiled(&by_var, "Rep", "OnPreReport"));
        assert!(!compiled(&by_var, "Rep", "Other"));
        let (_, by_constant) = compile(
            "codeunit 1 Main { procedure Go() begin Report.Run(Report::Rep); XmlPort.Import(XmlPort::Xp); end; }",
            &dep,
        );
        assert!(compiled(&by_constant, "Rep", "OnPreReport"));
        assert!(compiled(&by_constant, "Xp", "OnPreXmlPort"));
        // Only named, not run: nothing is reached.
        let (_, named) = compile(
            "codeunit 1 Main { procedure Go() var I: Integer; begin I := Report::Rep; end; }",
            &dep,
        );
        assert!(!compiled(&named, "Rep", "OnPreReport"));
        // Handed to a procedure as an argument, workspace code may run it there
        // (corpus handlers/TestReportDatasetHelpLink.al: `RenderDataset(Report::"G/L Register")`).
        let (_, passed) = compile(
            "codeunit 1 Main { procedure Go() begin Show(Report::Rep); end; procedure Show(Id: Integer) begin end; }",
            &dep,
        );
        assert!(compiled(&passed, "Rep", "OnPreReport"));
    }

    #[test]
    fn the_triggers_before_and_after_a_table_trigger_that_an_extension_declares_are_reached() {
        let dep = [
            ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
            (
                "x.al",
                "tableextension 11 TX extends T { trigger OnBeforeInsert() begin end; trigger OnAfterInsert() begin end; \
                 trigger OnBeforeDelete() begin end; }",
            ),
        ];
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var R: Record T; begin R.Insert(); end; }",
            &dep,
        );
        assert!(compiled(&l, "TX", "OnBeforeInsert") && compiled(&l, "TX", "OnAfterInsert"));
        assert!(!compiled(&l, "TX", "OnBeforeDelete"));
    }

    #[test]
    fn subscribers_nothing_compiled_are_listed_by_publisher_unless_they_are_manual() {
        let dep = [
            ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
            (
                "p.al",
                "codeunit 20 Pub { [IntegrationEvent(false, false)] procedure OnX() begin end; }",
            ),
            (
                "s.al",
                "codeunit 30 Auto {\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnX', '', false, false)]\n\
                 local procedure A() begin end; }",
            ),
            (
                "m.al",
                "codeunit 31 Manual { EventSubscriberInstance = Manual;\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnX', '', false, false)]\n\
                 local procedure A() begin end; }",
            ),
        ];
        // The workspace uses the publisher without calling the event routine, so its subscribers
        // are not reached; the manual one cannot fire without a bound instance.
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var P: Codeunit Pub; begin end; }",
            &dep,
        );
        let list: Vec<_> = l
            .program
            .uncompiled_subscribers
            .iter()
            .map(|u| {
                (
                    u.publisher.map(|p| l.program.objects[p as usize].name.clone()),
                    u.event.clone(),
                    u.subscriber.clone(),
                )
            })
            .collect();
        assert_eq!(
            list,
            [(
                Some("Pub".to_string()),
                "onx".to_string(),
                "codeunit \"Auto\"".to_string()
            )]
        );
    }
    // A panic while checking or lowering one routine is a bug in the compiler. It must not end the
    // run: the routine becomes a refusal, so the tests that reach it are not run and the rest are.
    // `InjectedCheckPanic` and `InjectedLowerPanic` panic in the compiler (test builds only).
    #[test]
    fn a_panic_checking_a_reached_routine_refuses_it_and_nothing_else() {
        for workers in [1, 4] {
            let dep = [(
                "lib.al".to_string(),
                "codeunit 10 Lib { procedure Fine() begin end; procedure InjectedCheckPanic() begin end; \
                 procedure Other() begin end; }"
                    .to_string(),
            )];
            let main = "codeunit 1 Main { procedure Go() var L: Codeunit Lib; begin L.Fine(); L.InjectedCheckPanic(); L.Other(); end; \
                        procedure Clean() var L: Codeunit Lib; begin L.Fine(); end; }";
            let out = compile_with_workers(main, &dep, workers);
            assert!(
                out.contains("internal error while checking"),
                "{workers} workers: {out}"
            );
        }
    }

    #[test]
    fn a_panic_lowering_a_routine_refuses_it_and_nothing_else() {
        let (_, l) = compile(
            "codeunit 1 Main { procedure Go() var L: Codeunit Lib; begin L.Fine(); L.InjectedLowerPanic(); end; \
             procedure Clean() var L: Codeunit Lib; begin L.Fine(); end; }",
            &[(
                "lib.al",
                "codeunit 10 Lib { procedure Fine() begin end; procedure InjectedLowerPanic() begin end; }",
            )],
        );
        assert!(
            reasons(&l, "Lib", "InjectedLowerPanic")
                .iter()
                .any(|r| r.contains("internal error while lowering")),
            "{:?}",
            reasons(&l, "Lib", "InjectedLowerPanic")
        );
        assert!(
            reasons(&l, "Lib", "Fine").is_empty(),
            "{:?}",
            reasons(&l, "Lib", "Fine")
        );
        assert!(reasons(&l, "Main", "Clean").is_empty());
    }

    #[test]
    fn a_panic_in_the_workspace_refuses_that_routine_only() {
        let (_, l) = compile(
            "codeunit 1 Main { procedure InjectedCheckPanic() begin end; procedure InjectedLowerPanic() begin end; \
             procedure Fine() begin end; }",
            &[("lib.al", "codeunit 10 Lib { }")],
        );
        for name in ["InjectedCheckPanic", "InjectedLowerPanic"] {
            assert!(
                reasons(&l, "Main", name).iter().any(|r| r.contains("internal error")),
                "{name}: {:?}",
                reasons(&l, "Main", name)
            );
        }
        assert!(reasons(&l, "Main", "Fine").is_empty());
    }
}
