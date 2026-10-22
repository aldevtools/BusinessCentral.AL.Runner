//! Row providers for the platform's system tables (ADR-013 decision 7, ADR-019).
//!
//! The schema of a platform table is the real declaration in the platform `System` app, compiled
//! like any other table. Its rows are not AL: the platform computes them. A [`Providers`] is
//! handed to the session's store, which asks it for every table it `serves`. Which tables those
//! are is the registry `al_builtins::platform::REGISTRY`; a table with no entry is refused at
//! compile time, so nothing here is ever asked for an unregistered table.
//!
//! A provider reads the compiled program, never the OS (ADR-006). Every observable rule carries
//! the corpus line that pins it; a column or a shape nothing pins is refused, not guessed.

mod apps;
pub(crate) mod coverage;
mod date;
mod fields;
mod host_tables;
pub(crate) use host_tables::time_zone_get_refusal;
mod identity;
mod ids;
mod integer;
mod links;
mod mat;
mod metadata;
pub(crate) mod objects;
mod pages;
mod permissions;
pub(crate) mod profiles;
pub(crate) mod ranges;
mod reports;
mod seed;

use crate::Engine;
use al_builtins::describe::Describe;
use al_builtins::filter::FieldFilter;
use al_builtins::platform::Virtual;
use al_builtins::value::Value;
use al_data::memory::MemStore;
use al_data::{AccessContext, Row, Scan, StoreError, TableDef, VirtualTables};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

/// (subscriber codeunit id, lower-case function) to (object, routine).
type EventIndex = HashMap<(u32, String), (usize, usize)>;

/// What the Event Subscription table reports about the running session: how many events each
/// subscriber handled and how many instances of a manual subscriber are bound (record/
/// TestEventSubscriptionColumnsReportLayoutListContracts.al). The session writes it, the provider reads it.
#[derive(Default)]
pub(crate) struct EventStats {
    /// (subscriber object, subscriber routine) to the events it handled.
    pub calls: RefCell<HashMap<(usize, usize), i64>>,
    /// Subscriber object to the number of bound instances.
    pub active: RefCell<HashMap<usize, i64>>,
    /// The session's `GlobalLanguage` when it is not 1033 (0: 1033). The Report Layout List's Description follows it.
    pub language: std::cell::Cell<i32>,
}

/// What the host says about the session, read once when the session starts.
#[derive(Clone)]
pub(crate) struct Ident {
    pub user_id: String,
    pub user_sid: [u8; 16],
    pub company: String,
    pub company_id: [u8; 16],
    pub session_id: i32,
    pub instance_id: i32,
    pub host_name: String,
    pub client_type: al_host::ClientType,
    /// When the session started: milliseconds since the Unix epoch.
    pub login_millis: i64,
    /// The host's time zones, in `No.` order.
    pub time_zones: Vec<al_host::TimeZoneInfo>,
    /// The Windows name of the session's time zone.
    pub time_zone: String,
}

impl Ident {
    pub(crate) fn of(host: &al_host::Host) -> Ident {
        let id = &host.identity;
        Ident {
            user_id: id.user_id(),
            user_sid: id.user_security_id(),
            company: id.company_name(),
            company_id: id.company_id(),
            time_zone: id.time_zone(),
            session_id: id.session_id(),
            instance_id: id.service_instance_id(),
            host_name: id.host_name(),
            client_type: id.current_client_type(),
            login_millis: host.clock.now_utc_millis(),
            time_zones: id.time_zones(),
        }
    }
}

/// Listings that sessions of one engine share. A listed table's rows depend on the program (and,
/// for three tables, on the host's identity: see [`Providers::shares`]), never on a session's
/// data, so a session that needs a table another session of the same engine listed takes that
/// listing. The Field table alone is 37,000 rows, listed in 0.2 s; a corpus run's test codeunits
/// each have a session of their own and several of them read it.
///
/// Shared by all threads: the first thread that needs a table builds it, the others wait for it
/// instead of building it again (a builder reads the program and the host, never another listing,
/// so two builders cannot wait for each other). The two engines used last keep their listings,
/// which bounds the memory: a run has the engine the tests run on and the one the install
/// baseline ran on. The engine is named by its `uid`, not held: a held `Arc` or `Weak` would stop
/// `Arc::get_mut` on the engine, which the runner uses.
#[derive(Default)]
struct SharedListings {
    engines: Mutex<Vec<EngineListings>>,
}

struct EngineListings {
    engine: u64,
    tables: HashMap<Virtual, Arc<OnceLock<Option<Arc<mat::SharedMat>>>>>,
}

/// The most engines whose listings are kept.
const ENGINES_KEPT: usize = 2;

static SHARED: LazyLock<SharedListings> = LazyLock::new(SharedListings::default);

impl SharedListings {
    /// The listing of `kind` for `engine`, built by `build` if no thread has built it yet. `None`
    /// when the listing holds a handle and cannot be shared: each session then lists it itself.
    fn listing(&self, engine: u64, kind: Virtual, build: impl FnOnce() -> mat::Mat) -> Option<Arc<mat::SharedMat>> {
        let slot = {
            let mut engines = self.engines.lock().unwrap_or_else(|e| e.into_inner());
            let at = match engines.iter().position(|e| e.engine == engine) {
                Some(at) => at,
                None => {
                    if engines.len() == ENGINES_KEPT {
                        engines.remove(0);
                    }
                    engines.push(EngineListings {
                        engine,
                        tables: HashMap::new(),
                    });
                    engines.len() - 1
                }
            };
            // The engine asked for last is the last to go.
            let last = engines.len() - 1;
            engines.swap(at, last);
            engines[last].tables.entry(kind).or_default().clone()
        };
        slot.get_or_init(|| mat::SharedMat::try_new(build()).map(Arc::new))
            .clone()
    }
}

/// A listing a session reads: shared with the other threads, or its own when it cannot be shared.
#[derive(Clone)]
pub(crate) enum Listed {
    Shared(Arc<mat::SharedMat>),
    Own(Rc<mat::Mat>),
}

impl std::ops::Deref for Listed {
    type Target = mat::Mat;
    fn deref(&self) -> &mat::Mat {
        match self {
            Listed::Shared(m) => m,
            Listed::Own(m) => m,
        }
    }
}

/// The rows of every virtual platform table, computed on read.
pub(crate) struct Providers {
    /// Weak: the engine owns nothing of the session, and a baseline snapshot of the store (which
    /// holds this provider) must not keep the engine shared.
    engine: std::sync::Weak<Engine>,
    /// `Engine::uid` of that engine.
    engine_uid: u64,
    ident: Ident,
    /// Per object index: the provider of that table, if it is a virtual platform table ([`served_tables`], once per
    /// engine).
    served: Arc<[Option<Virtual>]>,
    /// Tables listed in full, once (they depend on the program and the host only).
    listed: RefCell<HashMap<Virtual, Listed>>,
    /// The Aggregate Permission Set rows with the tenant's own sets in them, and the stamp of the
    /// stored Tenant Permission Set table they were composed at.
    aggregate: permissions::Cache,
    grants: permissions::Grants,
    events: Rc<EventStats>,
    /// The session's media: the rows of `Tenant Media`.
    media: al_builtins::media::MediaStore,
    /// (subscriber codeunit id, lower-case function) to (object, routine); built on first use.
    event_index: RefCell<Option<EventIndex>>,
}

/// Per object index of the engine's program: the provider of that table, if it is a virtual platform table. It depends on
/// the program only, so it is worked out once per engine (`Engine::served_tables`), not for every session.
fn served_tables(engine: &Engine) -> Arc<[Option<Virtual>]> {
    engine
        .program
        .objects
        .iter()
        .map(|o| {
            if !o.is_kind(al_ir::ObjKind::Table) {
                return None;
            }
            let id = o.id?;
            match al_builtins::platform::provider(id) {
                Some(_) => al_builtins::platform::virtual_table(id),
                // A table of the platform app that nothing provides: its rows are refused.
                None if engine
                    .program
                    .apps
                    .get(o.app as usize)
                    .is_some_and(|a| a.id.eq_ignore_ascii_case(objects::al_sema_platform_app_id())) =>
                {
                    Some(Virtual::Unprovided)
                }
                None => None,
            }
        })
        .collect()
}

impl Providers {
    pub(crate) fn new(
        engine: &Arc<Engine>,
        ident: Ident,
        events: Rc<EventStats>,
        media: al_builtins::media::MediaStore,
    ) -> Providers {
        let served = engine.served_tables.get_or_init(|| served_tables(engine)).clone();
        Providers {
            engine: Arc::downgrade(engine),
            engine_uid: engine.uid,
            ident,
            served,
            listed: RefCell::default(),
            aggregate: RefCell::default(),
            grants: Default::default(),
            events,
            media,
            event_index: RefCell::new(None),
        }
    }

    /// An Event Subscription row with "Number of Calls" (8) and "Active Manual Instances" (13) as
    /// the session has them now.
    /// The session's `GlobalLanguage`.
    pub(super) fn language(&self) -> i32 {
        match self.events.language.get() {
            0 => 1033,
            n => n,
        }
    }

    fn with_event_stats(&self, t: &TableDef, row: Option<Row>) -> Option<Row> {
        let mut row = row?;
        if self.events.calls.borrow().is_empty() && self.events.active.borrow().is_empty() {
            return Some(row);
        }
        let (Value::Integer(cu), Value::Text(f)) = (&row[pos(t, 1)], &row[pos(t, 2)]) else {
            return Some(row);
        };
        let key = (*cu as u32, f.to_ascii_lowercase());
        let mut index = self.event_index.borrow_mut();
        let index = index.get_or_insert_with(|| {
            let engine = self.engine();
            let mut m = HashMap::new();
            for (oi, o) in engine.program.objects.iter().enumerate() {
                let (Some(id), al_ir::ObjKind::Codeunit) =
                    (o.id, if o.uncompiled { al_ir::ObjKind::Other } else { o.kind })
                else {
                    continue;
                };
                for (ri, r) in o.routines.iter().enumerate() {
                    m.insert((id, r.name.to_ascii_lowercase()), (oi, ri));
                }
            }
            m
        });
        if let Some(&(oi, ri)) = index.get(&key) {
            row[pos(t, 8)] = Value::BigInteger(self.events.calls.borrow().get(&(oi, ri)).copied().unwrap_or(0));
            row[pos(t, 13)] = Value::Integer(self.events.active.borrow().get(&oi).copied().unwrap_or(0) as i32);
        }
        Some(row)
    }

    /// A filter on a column the session changes cannot be answered from the shared listing: the rows
    /// are read with the session's counts first (record/ProbePlatformTables.al: `EventSubscription_FilterOnNumberOfCalls`).
    fn event_filtered_listing(&self, t: &TableDef, filters: &[FieldFilter]) -> Result<Option<mat::Mat>, StoreError> {
        if !filters.iter().any(|f| matches!(t.fields[f.field].id, 8 | 13)) {
            return Ok(None);
        }
        let listed = self.listed(t, Virtual::EventSubscription)?;
        Ok(Some(listed.mapped(|row| {
            self.with_event_stats(t, Some(row.clone()))
                .unwrap_or_else(|| row.clone())
        })))
    }

    pub(super) fn engine(&self) -> Arc<Engine> {
        self.engine.upgrade().expect("a session outlives its provider's reads")
    }

    /// The Aggregate Permission Set rows as of now: composed again when the stored tenant sets changed.
    fn aggregate(&self, store: &MemStore, cx: &AccessContext, t: &TableDef) -> Result<Rc<mat::Mat>, StoreError> {
        let stamp = self
            .tenant_permission_set_def()
            .map_or(0, |tenant| store.table_stamp(cx, &tenant));
        if let Some((at, m)) = &*self.aggregate.borrow() {
            if *at == stamp {
                return Ok(m.clone());
            }
        }
        let m = Rc::new(mat::Mat::new(t, permissions::aggregate_rows(self, store, cx, t)?));
        *self.aggregate.borrow_mut() = Some((stamp, m.clone()));
        Ok(m)
    }

    /// Whether the listing of `kind` is the same for every session of the engine. Session, Active
    /// Session and Time Zone list the host's identity of this session.
    fn shares(kind: Virtual) -> bool {
        !matches!(
            kind,
            Virtual::Session | Virtual::ActiveSession | Virtual::TimeZone | Virtual::TenantMedia
        )
    }

    fn kind(&self, t: &TableDef) -> Virtual {
        self.served[t.object as usize].expect("serves() was true")
    }

    /// One `Tenant Media` row per imported media object, in id order: ID, Description, Content, Mime
    /// Type, Height, Width and File Name (empty), as the importing methods leave them (corpus
    /// media/PXMediaFileName.al, media/TestMediaSetPersistence.al:180-207).
    fn tenant_media_rows(&self, t: &TableDef) -> Vec<mat::Entry> {
        let objects = self.media.objects.borrow();
        let mut ids: Vec<_> = objects.keys().copied().collect();
        ids.sort();
        ids.into_iter()
            .map(|id| {
                let o = &objects[&id];
                let mut row = blank(t);
                row[pos(t, 1)] = Value::Guid(id);
                // The four argument import: what the file name changes in this row (its File Name, and
                // the Description and Mime Type the platform derives from it when they are empty) is not
                // pinned (docs/probes-needed.md, `Media.ImportStream(.., FileName)`).
                if !o.file_name.is_empty() {
                    return mat::Entry::poisoned(
                        row,
                        "a Tenant Media row of a media imported with a file name: its File Name, Description and Mime Type are not pinned".into(),
                    );
                }
                // ADR-044: the picture was never produced; its content, MIME type and size are unknown.
                if let Some(what) = &o.pending {
                    let al_builtins::native::Signal::Unsupported(why) = al_builtins::stream::render_pending_error(what) else {
                        unreachable!("render_pending_error is a refusal")
                    };
                    return mat::Entry::poisoned(row, why);
                }
                row[pos(t, 2)] = Value::Text(o.description.clone().into());
                let blob = al_builtins::stream::new_blob();
                al_builtins::stream::write_bytes(&blob, &o.bytes);
                row[pos(t, 3)] = Value::Blob(blob);
                row[pos(t, 4)] = Value::Text(o.mime.clone().into());
                row[pos(t, 5)] = Value::Integer(o.height);
                row[pos(t, 6)] = Value::Integer(o.width);
                // The row names the session company (platform/W6TenantMedia.al:TenantMedia_RowAfterImportStream).
                if let Some(p) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case("Company Name")) {
                    row[p] = Value::Text(self.ident.company.clone().into());
                }
                mat::Entry::ok(row)
            })
            .collect()
    }

    /// The Feature Key rows: the served ones, with the rows a Modify stored (`Enabled`) over them.
    fn feature_key_listing(&self, store: &MemStore, cx: &AccessContext, t: &TableDef) -> Result<Listed, StoreError> {
        let stored = store.stored_rows(cx, t);
        if stored.is_empty() {
            return self.listed(t, Virtual::FeatureKey);
        }
        let mut entries = self.list_rows(t, Virtual::FeatureKey);
        for e in &mut entries {
            let key = t.primary_key(&e.row);
            let same_key = |r: &&Row| {
                t.primary_key(r)
                    .iter()
                    .zip(&key)
                    .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
            };
            if let Some(over) = stored.iter().find(same_key) {
                e.row = over.clone();
            }
        }
        Ok(Listed::Own(Rc::new(mat::Mat::new(t, entries))))
    }

    /// The rows of a table that is listed in full.
    fn listed(&self, t: &TableDef, kind: Virtual) -> Result<Listed, StoreError> {
        // In another language than 1033 the layouts' descriptions are that language's: listed per read, not kept.
        if kind == Virtual::ReportLayoutList && self.language() != 1033 {
            return Ok(Listed::Own(Rc::new(mat::Mat::new(
                t,
                reports::layout_list_rows(self, t),
            ))));
        }
        if let Some(m) = self.listed.borrow().get(&kind) {
            return Ok(m.clone());
        }
        // Refused before anything is built.
        match kind {
            Virtual::ApiWebhookEntity => {
                return Err(refused(
                    "API Webhook Entity: the entities of the API pages are listed by the platform and no corpus test pins them",
                ));
            }
            Virtual::Unprovided => {
                return Err(refused(format!(
                    "platform table '{}' ({}) has no provider",
                    t.name,
                    self.engine().program.objects[t.object as usize].id.unwrap_or(0)
                )));
            }
            // The media the session imported, read fresh each time (they change between reads).
            Virtual::TenantMedia => return Ok(Listed::Own(Rc::new(mat::Mat::new(t, self.tenant_media_rows(t))))),
            _ => {}
        }
        let build = || mat::Mat::new(t, self.list_rows(t, kind));
        let shared = if Self::shares(kind) {
            SHARED.listing(self.engine_uid, kind, build)
        } else {
            mat::SharedMat::try_new(build()).map(Arc::new)
        };
        let m = match shared {
            Some(m) => Listed::Shared(m),
            None => Listed::Own(Rc::new(build())),
        };
        self.listed.borrow_mut().insert(kind, m.clone());
        Ok(m)
    }

    /// The rows of the table of `kind`, worked out from the program and the host, with the
    /// columns nothing pins left unknown ([`mark_unpinned`]).
    fn list_rows(&self, t: &TableDef, kind: Virtual) -> Vec<mat::Entry> {
        let mut rows = self.provided_rows(t, kind);
        mark_unpinned(t, &mut rows);
        rows
    }

    /// The rows a provider fills for the table of `kind`.
    fn provided_rows(&self, t: &TableDef, kind: Virtual) -> Vec<mat::Entry> {
        match kind {
            Virtual::ApiWebhookEntity | Virtual::TenantMedia | Virtual::Unprovided => {
                unreachable!("refused or listed before")
            }
            Virtual::XmlPortMetadata => metadata::xmlport_metadata_rows(self, t),
            Virtual::EventSubscription => metadata::event_subscription_rows(self, t),
            Virtual::PageAction => metadata::page_action_rows(self, t),
            Virtual::TableRelations => metadata::table_relations_rows(self, t),
            Virtual::QueryMetadata => metadata::query_metadata_rows(self, t),
            Virtual::ReportLayoutList => reports::layout_list_rows(self, t),
            Virtual::ReportMetadata => reports::report_metadata_rows(self, t),
            Virtual::ReportDataItems => reports::report_data_item_rows(self, t),
            Virtual::MetadataPermissionSet => permissions::metadata_permission_set_rows(self, t),
            Virtual::PermissionSet => permissions::permission_set_rows(self, t, false),
            Virtual::PermissionSetAssignable => permissions::permission_set_rows(self, t, true),
            Virtual::AggregatePermissionSet => unreachable!("composed per read"),
            Virtual::MetadataPermission | Virtual::ExpandedPermission | Virtual::Permission => {
                unreachable!("composed per read")
            }
            Virtual::NavAppInstalledApp => apps::nav_app_installed_app_rows(self, t),
            Virtual::NavAppExtra => apps::nav_app_extra_rows(self, t),
            Virtual::InstalledApplication => apps::installed_application_rows(self, t),
            Virtual::PublishedApplication => apps::published_application_rows(self, t),
            Virtual::TimeZone => host_tables::time_zone_rows(t, &self.ident),
            Virtual::Session => vec![mat::Entry::ok(identity::session_row(t, &self.ident))],
            Virtual::ActiveSession => vec![mat::Entry::ok(identity::active_session_row(t, &self.ident))],
            Virtual::AllObj => objects::all_obj_rows(self, t),
            Virtual::AllObjWithCaption => objects::all_obj_with_caption_rows(self, t),
            Virtual::CodeunitMetadata => objects::codeunit_metadata_rows(self, t),
            Virtual::TableMetadata => objects::table_metadata_rows(self, t),
            Virtual::PageMetadata => pages::page_metadata_rows(self, t),
            Virtual::PageControlField => pages::page_control_field_rows(self, t),
            Virtual::Field => fields::field_rows(self, t),
            Virtual::FeatureKey => host_tables::feature_key_rows(t, self.engine().target.as_ref()),
            Virtual::Key => fields::key_rows(self, t),
            Virtual::IntelligentCloud => Vec::new(),
            Virtual::Integer | Virtual::Date | Virtual::WindowsLanguage | Virtual::LicensePermission => {
                unreachable!("generated, never listed")
            }
        }
    }
}

/// Leaves every column of a table of `PINNED_COLUMNS` that the list does not name, outside the
/// primary key, the FlowFields and the system fields, `Value::Unknown` in every row: a read of it is refused
/// (ADR-013 decision 7; hard rule 3), a copy keeps the marker. There are no exceptions: a
/// column a corpus test reads but does not assert is not pinned either.
pub(crate) fn mark_unpinned(t: &TableDef, rows: &mut [mat::Entry]) {
    let Some(id) = al_builtins::platform::REGISTRY
        .iter()
        .chain(al_builtins::platform::LATER_MAJORS)
        .find(|(_, n, _)| n.eq_ignore_ascii_case(&t.name))
        .map(|(i, _, _)| *i)
    else {
        return;
    };
    let Some((_, pinned)) = al_builtins::platform::PINNED_COLUMNS.iter().find(|(i, _)| *i == id) else {
        return;
    };
    for (pos, f) in t.fields.iter().enumerate() {
        if f.id >= al_builtins::platform::SYSTEM_OBJECT_BASE
            || !f.stored
            || pinned.contains(&f.id)
            || t.keys[0].contains(&pos)
        {
            continue;
        }
        let why = Value::Unknown(unknown_why(t, f.id, "any row"));
        for e in rows.iter_mut() {
            if !matches!(e.row[pos], Value::Unknown(_)) {
                e.row[pos] = why.clone();
            }
        }
    }
}

/// Position in `t.fields` of the field with id `id`.
pub(crate) fn pos(t: &TableDef, id: u32) -> usize {
    t.fields
        .iter()
        .position(|f| f.id == id)
        .unwrap_or_else(|| panic!("table '{}' has no field {id}: the platform declaration changed", t.name))
}

/// A row of default values; the providers set the columns they know.
pub(crate) fn blank(t: &TableDef) -> Row {
    t.fields.iter().map(|f| Value::default_for(&f.ty)).collect()
}

/// The value of column `id` of a row whose value there no corpus test pins: a read of it is refused, naming the table,
/// the field and `whose` (the row, or the kind of rows), while the other columns of the row read as usual
/// (`Value::Unknown`). Writing the field replaces it.
pub(crate) fn unknown(t: &TableDef, id: u32, whose: &str) -> Value {
    Value::Unknown(unknown_why(t, id, whose))
}

/// The reason [`unknown`] holds, for a provider that shares one among many rows.
pub(crate) fn unknown_why(t: &TableDef, id: u32, whose: &str) -> Arc<str> {
    format!(
        "the value of field '{}' of platform table '{}' is not known for {whose}: no corpus test pins it (docs/probes-needed.md)",
        t.fields[pos(t, id)].name,
        t.name
    )
    .into()
}

/// A row of default values made once, for a listing that starts every row from one: copying it
/// shares the member lists of the option columns instead of building each again.
pub(crate) struct Blank {
    row: Row,
}

impl Blank {
    pub(crate) fn new(t: &TableDef) -> Blank {
        Blank { row: blank(t) }
    }

    pub(crate) fn row(&self) -> Row {
        self.row.clone()
    }

    /// An `Option` value of field `id` with ordinal `n`.
    pub(crate) fn option(&self, t: &TableDef, id: u32, n: i32) -> Value {
        match &self.row[pos(t, id)] {
            Value::Option(_, m) => Value::Option(n, m.clone()),
            // A column the row starts unknown in: its members come from the type.
            Value::Unknown(_) => option(t, id, n),
            other => panic!("field {id} of '{}' is not an option: {}", t.name, other.describe()),
        }
    }
}

/// An `Option` value of field `id` with ordinal `n`.
pub(crate) fn option(t: &TableDef, id: u32, n: i32) -> Value {
    match Value::default_for(&t.fields[pos(t, id)].ty) {
        Value::Option(_, m) => Value::Option(n, m),
        other => panic!("field {id} of '{}' is not an option: {}", t.name, other.describe()),
    }
}

pub(crate) fn refused(why: impl Into<String>) -> StoreError {
    StoreError::Refused(why.into())
}

impl VirtualTables for Providers {
    fn serves(&self, t: &TableDef) -> bool {
        self.served.get(t.object as usize).is_some_and(Option::is_some)
    }

    // Enabled is the only writable column of a Feature Key (record/TestFeatureKeyTable.al:
    // Record_FeatureKey_ModifyingAReadOnlyColumn_RaisesNamingTheField); that a change stays for the rest of the run is
    // assumed (docs/probes-needed.md, Feature Key).
    fn writable_columns(&self, t: &TableDef) -> &'static [u32] {
        if self.served.get(t.object as usize) == Some(&Some(Virtual::FeatureKey)) {
            &[2]
        } else {
            &[]
        }
    }

    fn write_refusal(&self, t: &TableDef) -> Option<String> {
        (self.served.get(t.object as usize) == Some(&Some(Virtual::Unprovided))).then(|| {
            format!(
                "platform table '{}' ({}) has no provider",
                t.name,
                self.engine().program.objects[t.object as usize].id.unwrap_or(0)
            )
        })
    }

    fn get(&self, store: &MemStore, cx: &AccessContext, t: &TableDef, pk: &[Value]) -> Result<Option<Row>, StoreError> {
        match self.kind(t) {
            Virtual::AggregatePermissionSet => self.aggregate(store, cx, t)?.get(t, pk),
            Virtual::MetadataPermission | Virtual::ExpandedPermission | Virtual::Permission => {
                let which = permissions::grant_table(self.kind(t));
                // A read by key names one set; the rest of the key is matched by the table.
                let filters = permissions::key_filters(t, which, pk);
                permissions::grants_mat(self, store, cx, t, which, &filters)?.get(t, pk)
            }
            // From BC 29 a read by key under any App ID finds the null-App-ID row of SUPER and
            // SECURITY (permissions/TestMetadataPermissionSet.al:85-128).
            Virtual::MetadataPermissionSet
                if al_version::gate_on(
                    self.engine().target.as_ref(),
                    al_version::Gate::SuperPermissionSetIgnoresAppId,
                ) && matches!(pk.get(1), Some(Value::Code(r) | Value::Text(r)) if r.eq_ignore_ascii_case("SUPER") || r.eq_ignore_ascii_case("SECURITY")) =>
            {
                let mut key = pk.to_vec();
                key[0] = Value::Guid([0; 16]);
                self.listed(t, Virtual::MetadataPermissionSet)?.get(t, &key)
            }
            Virtual::Integer => integer::get(t, pk),
            Virtual::Date => date::get(t, pk),
            Virtual::WindowsLanguage => host_tables::windows_language_get(t, pk),
            Virtual::LicensePermission => host_tables::license_permission_get(t, pk),
            Virtual::EventSubscription => {
                Ok(self.with_event_stats(t, self.listed(t, Virtual::EventSubscription)?.get(t, pk)?))
            }
            Virtual::FeatureKey => self.feature_key_listing(store, cx, t)?.get(t, pk),
            Virtual::Field => Ok(self
                .listed(t, Virtual::Field)?
                .get(t, pk)?
                .map(|r| fields::field_get(t, r))),
            k => self.listed(t, k)?.get(t, pk),
        }
    }

    fn next(&self, store: &MemStore, cx: &AccessContext, t: &TableDef, scan: &Scan) -> Result<Option<Row>, StoreError> {
        match self.kind(t) {
            Virtual::AggregatePermissionSet => self.aggregate(store, cx, t)?.next(t, scan),
            Virtual::MetadataPermission | Virtual::ExpandedPermission | Virtual::Permission => {
                let which = permissions::grant_table(self.kind(t));
                permissions::grants_mat(self, store, cx, t, which, scan.filters)?.next(t, scan)
            }
            // The legacy table lists only assignable sets when walked, all of them when the key is named.
            Virtual::PermissionSet if !permissions::pins_the_primary_key(t, scan.filters) => {
                self.listed(t, Virtual::PermissionSetAssignable)?.next(t, scan)
            }
            Virtual::Integer => integer::next(t, scan),
            Virtual::Date => date::next(t, scan),
            Virtual::WindowsLanguage => host_tables::windows_language_next(t, scan),
            Virtual::LicensePermission => host_tables::license_permission_next(t, scan),
            Virtual::EventSubscription => {
                if let Some(m) = self.event_filtered_listing(t, scan.filters)? {
                    return m.next(t, scan);
                }
                Ok(self.with_event_stats(t, self.listed(t, Virtual::EventSubscription)?.next(t, scan)?))
            }
            Virtual::FeatureKey => self.feature_key_listing(store, cx, t)?.next(t, scan),
            k => self.listed(t, k)?.next(t, scan),
        }
    }

    fn count(
        &self,
        store: &MemStore,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
    ) -> Result<usize, StoreError> {
        match self.kind(t) {
            Virtual::AggregatePermissionSet => self.aggregate(store, cx, t)?.count(t, filters),
            Virtual::MetadataPermission | Virtual::ExpandedPermission | Virtual::Permission => {
                let which = permissions::grant_table(self.kind(t));
                permissions::grants_mat(self, store, cx, t, which, filters)?.count(t, filters)
            }
            Virtual::PermissionSet if !permissions::pins_the_primary_key(t, filters) => {
                self.listed(t, Virtual::PermissionSetAssignable)?.count(t, filters)
            }
            Virtual::Integer => integer::count(t, filters),
            Virtual::Date => date::count(t, filters),
            Virtual::WindowsLanguage => host_tables::windows_language_count(t, filters),
            Virtual::LicensePermission => host_tables::license_permission_count(t, filters),
            Virtual::EventSubscription => {
                if let Some(m) = self.event_filtered_listing(t, filters)? {
                    return m.count(t, filters);
                }
                self.listed(t, Virtual::EventSubscription)?.count(t, filters)
            }
            Virtual::FeatureKey => self.feature_key_listing(store, cx, t)?.count(t, filters),
            k => self.listed(t, k)?.count(t, filters),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> Arc<Engine> {
        Engine::new(al_ir::Program::default(), Vec::new())
    }

    fn some_def() -> TableDef {
        TableDef {
            object: 0,
            name: "T".into(),
            fields: Vec::new(),
            keys: vec![Vec::new()],
            per_company: false,
        }
    }

    fn some_mat() -> mat::Mat {
        mat::Mat::new(&some_def(), Vec::new())
    }

    #[test]
    fn engines_have_their_own_uid() {
        assert_ne!(engine().uid, engine().uid);
    }

    #[test]
    fn sessions_of_one_engine_share_a_listing_and_other_engines_do_not() {
        let shared = SharedListings::default();
        let a = shared.listing(1, Virtual::Field, some_mat).unwrap();
        assert!(Arc::ptr_eq(
            &a,
            &shared.listing(1, Virtual::Field, || panic!("built twice")).unwrap()
        ));
        let other_kind = shared.listing(1, Virtual::AllObj, some_mat).unwrap();
        assert!(!Arc::ptr_eq(&a, &other_kind));
        let other_engine = shared.listing(2, Virtual::Field, some_mat).unwrap();
        assert!(!Arc::ptr_eq(&a, &other_engine));
    }

    #[test]
    fn only_the_two_engines_used_last_keep_their_listings() {
        let shared = SharedListings::default();
        let first = shared.listing(1, Virtual::Field, some_mat).unwrap();
        shared.listing(2, Virtual::Field, some_mat);
        // Engine 1 is still held: the baseline engine and the test engine of one run both list.
        assert!(Arc::ptr_eq(
            &first,
            &shared
                .listing(1, Virtual::Field, || panic!("dropped too early"))
                .unwrap()
        ));
        shared.listing(3, Virtual::Field, some_mat);
        // Engine 2 was used before engine 1 was asked again, so engine 2 is the one that went.
        let rebuilt = std::cell::Cell::new(false);
        shared.listing(2, Virtual::Field, || {
            rebuilt.set(true);
            some_mat()
        });
        assert!(rebuilt.get(), "engine 2 was replaced");
    }

    #[test]
    fn threads_asking_for_one_listing_build_it_once() {
        let shared = SharedListings::default();
        let builds = std::sync::atomic::AtomicUsize::new(0);
        let barrier = std::sync::Barrier::new(8);
        let got: Vec<Arc<mat::SharedMat>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        shared.listing(7, Virtual::Field, || {
                            builds.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                            std::thread::sleep(std::time::Duration::from_millis(20));
                            some_mat()
                        })
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap().unwrap()).collect()
        });
        assert_eq!(builds.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(got.iter().all(|m| Arc::ptr_eq(m, &got[0])));
    }

    #[test]
    fn a_listing_that_holds_a_handle_is_not_shared() {
        let shared = SharedListings::default();
        let mut t = some_def();
        t.fields.push(al_data::FieldDef {
            id: 1,
            name: "F".into(),
            ty: al_builtins::value::RtType::Variant,
            stored: true,
            code_storage: Default::default(),
        });
        let with_handle = || mat::Mat::new(&t, vec![mat::Entry::ok(vec![Value::List(Default::default())])]);
        assert!(shared.listing(1, Virtual::Field, with_handle).is_none());
    }

    #[test]
    fn a_listing_does_not_stop_arc_get_mut_on_its_engine() {
        // The runner sets `unseeded` on the engine after the install baseline listed tables.
        let mut a = engine();
        SHARED.listing(a.uid, Virtual::Field, some_mat);
        assert!(Arc::get_mut(&mut a).is_some());
    }

    #[test]
    fn what_depends_on_the_host_is_not_shared() {
        for kind in [Virtual::Session, Virtual::ActiveSession, Virtual::TimeZone] {
            assert!(!Providers::shares(kind));
        }
        for kind in [
            Virtual::Field,
            Virtual::AllObj,
            Virtual::PermissionSet,
            Virtual::PageMetadata,
        ] {
            assert!(Providers::shares(kind));
        }
    }
}
