//! Tree-walking interpreter over the typed IR (ADR-011). `Engine` is the
//! immutable program (ADR-001); `Session` holds all mutable state.

use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal};
use al_builtins::ops;
use al_builtins::value::{Instance, RecordState, RtType, Value};
use al_ir as ir;

/// A media object a Modify or Delete removed from Tenant Media, kept so a rollback can put it back.
type RemovedMedia = (al_builtins::media::Id, al_builtins::media::MediaObject);
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::sync::Arc;

mod autoinc;
mod bgsession;
mod blob_copy;
mod blob_transfer;
mod clientless;
mod get_key_value;
mod package_data;
mod partial_gone;
mod placeholder_files;
mod run_restore;
pub mod single_track;
mod state;
pub(crate) use state::*;
mod baseline;
mod sequence_start;
pub use sequence_start::SequenceStart;
mod capture;
mod ctx;
mod debug;
mod exec;
mod interp;
mod isolated_storage_reads;
pub use baseline::{Baseline, NotShared, RowDelta, SharedBaseline};
pub use capture::{CaptureConfig, Captured, ClosedBy, LoopRecord, MAX_RECORDS, MessageRecord, Rendered, ValueRecord};
pub(crate) use ctx::base_slot_missing;
pub use debug::{DebugFrame, DebugHook, DebugView, Resume};
pub use exec::{disassemble as exec_disassemble, report as exec_report};
pub use faultsite::FaultFrame;
pub(crate) use interp::constant;
#[doc(hidden)]
pub use platform::{FieldGetByKey, field_get_by_key_pairs};
pub use profile::frame_name as profile_frame_name;
pub use profile::{
    DbStat, PosStat, ProfFrame, ProfNode, ProfileMode, ProfileTree, reaches_database,
    routine_label as profile_routine_label,
};
pub use state_stamp::StateStamp;
pub use test_runner_host::TestCodeunitHost;
mod callstack;
mod caught_errors;
mod clear_all_page_ext;
mod code_storage_check;
mod column_formatter;
mod companies;
mod control_add_in;
mod crm;
mod data_file_types;
mod dataset;
mod datatransfer;
mod db_event_no_op;
mod decimal_column;
mod dev_endpoint_ui;
mod dispatch;
pub mod dotnet;
mod effects;
mod empty_reads;
mod enum_caption_ml;
mod order_trial;
mod page_held_bindings;
pub use empty_reads::{InstallRows, KnownValue};
mod assigned_audit;
mod caller_writes;
mod errorcollect;
pub mod events;
mod ext_validate_order;
mod faultsite;
mod fieldref_length;
mod fieldref_media_filter;
mod fieldref_validate;
mod fieldref_value_into;
mod filter_text_arg;
mod filterevents;
mod filterpage;
mod flowfields;
mod geturl;
mod globaltriggers;
mod hostvalues;
mod http;
pub mod impact;
mod interfaces;
mod isdirty;
mod license_ceiling;
mod load_fields_request;
mod media_orphans;
mod min_row_version;
mod native_rows;
pub mod observer_tap;
mod pages;
mod partial;
mod permission_shadow;
mod permission_triggers;
mod permissions;
mod platform;
mod profile;
mod queries;
mod query_date_filter;
mod query_save_as;
mod raise_to;
mod rdlc_fields;
mod read_only_tables;
mod recmeta;
mod recordid_bookmark;
mod recordid_code_text;
mod recordid_table_name;
mod recordref_copy;
mod recordref_load_fields;
mod recordref_variant;
mod records;
mod refs;
mod relations;
mod rename;
mod rename_blank_key;
mod report_handler_unshown;
mod report_layout_export;
mod report_layout_streams;
mod report_no_layout_file;
pub mod report_placeholder;
mod reports;
mod run_page_link_open;
mod sessionops;
mod setup_no_op;
mod slot_default;
mod sqlcount;
mod sqltime;
mod stale_buffer_origin;
mod stale_own_modify;
mod stale_rename;
mod state_stamp;
mod tasks;
mod tenant_layout_cfg;
mod tenant_layouts;
mod tenant_set_delete;
mod test_rows;
mod test_runner_host;
mod testdata;
mod transactions;
mod translations;
mod ui;
mod user_account_keys;
mod user_email;
mod user_password;
mod usertable;
mod view_empty;
mod view_unknown;
mod views;
mod views_cross_enum;
mod xmlports;
mod xrec_blobs;
pub use transactions::TxModel;
pub use ui::{UiHandler, UiKind};

static NEXT_ENGINE_UID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Program app index -> the resource files a run gives that app in place of the program's (`Engine::resource_overrides`).
pub type ResourceOverrides = std::collections::HashMap<usize, Arc<Vec<(String, Vec<u8>)>>>;

pub struct Engine {
    pub program: Arc<ir::Program>,
    /// Per object, per routine: why it cannot run, if it cannot.
    pub blocked: Vec<Vec<Option<String>>>,
    /// Store schema per table object.
    pub tables: Arc<[Option<Arc<al_data::TableDef>>]>,
    /// Event publishers and their subscribers, resolved by name.
    /// Shared with the engine's siblings ([`Engine::sibling`]): it does not change once the engine is set up.
    pub events: Arc<events::Registry>,
    /// The BC version this program runs as; `None` gets the newest behaviour (ADR-015 3.4).
    pub target: Option<al_version::Target>,
    /// The .NET host, the registry of runtime-provided classes and the policy (ADR-014).
    pub dotnet: dotnet::DotNetConfig,
    /// Table objects whose rows the install baseline should have created and did not (ADR-013
    /// decision 5): a `Get` that finds nothing in dependency code is not BC's answer for them.
    pub unseeded: std::collections::HashSet<usize>,
    /// Stored platform tables whose fresh-tenant rows nothing pins (`al_builtins::platform::UNSEEDED_ROWS`):
    /// unseeded for the tests, but not for install code, which writes them.
    pub platform_unseeded: std::collections::HashSet<usize>,
    /// Tables whose demo rows the corpus database holds and the runtime does not (ADR-047 rejected): a `Count` or `IsEmpty`
    /// of one that the session's own code has not written to is refused (`al_testrun::baseline::demo_row_tables`).
    pub demo_tables: std::collections::HashSet<usize>,
    /// Why the company the tests run in could not be built (`al_testrun::company_data`): every test codeunit is then not
    /// run, with this reason. `None`: the company is as the install baseline left it.
    pub company_refusal: Option<String>,
    /// The dependency apps the run skipped (no package with their source was found), each named as the log names it
    /// (`` `Name` by Publisher ``). With one, a lookup by id of an object no loaded app declares may land in a skipped
    /// app, where BC finds it: it is refused instead of raising the missing-object error (al-runner issue 5749).
    pub skipped_apps: Vec<String>,
    /// The resource files a run gives some apps in place of those of the program (an entry of a `serve` request, issue
    /// 5635): program app index -> resources. Set between the passes of a run, while no session runs.
    pub resource_overrides: std::sync::RwLock<ResourceOverrides>,
    /// A session read the resource files of an app (`NavApp.GetResourceAsText` and the like).
    pub resources_read: std::sync::atomic::AtomicBool,
    /// A session made a `DotNet` call that may answer from more than its arguments (`dotnet::pure`): files,
    /// environment variables, the machine's clock, the assemblies of `--dotnet-assembly-dir`. That is outside the
    /// program, so an install baseline whose code made one is not kept on disk (al-testrun `baseline_cache`).
    pub dotnet_called: std::sync::atomic::AtomicBool,
    /// Rows of a restored company, read when a session first touches a table (ADR-020). `None`: a
    /// run without test data.
    pub test_data: Option<al_host::testdata::TestData>,
    /// The BC license of the run (ADR-060): License Permission answers from it. `None`: no license was available, and
    /// only the pinned rows are served.
    pub license: Option<Arc<al_host::license::License>>,
    /// Whether the license gives some table of the program less than Yes on Read, Insert, Modify or Delete, so that
    /// record accesses check it also in an unrestricted session (license_ceiling.rs). Worked out on first use.
    pub(crate) license_caps: std::sync::OnceLock<bool>,
    /// The service-tier setting DisableWriteInsideTryFunctions is off (`al-runner test --allow-write-in-try-functions`, al-runner
    /// issue 7143), as on Business Central online. Default `false`: the setting at its default, as on a BC container and the
    /// corpus CI, where a write inside a TryFunction whose result is used raises TryFunctionWrite.
    pub write_in_try_allowed: bool,
    /// `al-runner test --dependencies-published-global`: "Published As" of "NAV App Installed App" is Global for a dependency app
    /// by another publisher than Microsoft (a documented setting, assumption `dependency-published-as-global`). Default
    /// `false`: Dev, as published through the development endpoint (al-runner issue 7169).
    pub dependencies_published_global: bool,
    /// (table object, field position) -> the error BC raises when it evaluates the field's
    /// `InitValue` (`0DT` on a DateTime, a closing-date literal on a Date).
    pub unevaluable_init: Arc<std::collections::HashMap<(u32, usize), String>>,
    /// What the VM asks of a table on every record operation, worked out once (ADR-035 decision 4).
    pub(crate) table_facts: Arc<[TableFacts]>,
    /// What the VM reads of each object's properties, worked out once (ADR-032 decision 5).
    pub(crate) object_facts: Arc<[ir::ObjectFacts]>,
    /// The same for each field of each table object, by field position.
    pub(crate) field_facts: Arc<[Vec<ir::FieldFacts>]>,
    /// Per table object, the positions of the fields `RecordRef.FieldCount` and `FieldIndex` enumerate, in their order
    /// (`refs.rs` `enumerable_fields`), worked out on first use: they depend on the program alone (al-runner issue 5911).
    pub(crate) enumerable_fields: Arc<[std::sync::OnceLock<Box<[usize]>>]>,
    /// Names this engine among all those the process built: state shared between sessions of one
    /// engine is keyed by it.
    pub uid: u64,
    /// The lookups of `platform::objects::Catalog` by object name and id, built on the first ask. The program is shared
    /// with the siblings and does not change under them, so they share the lookups too.
    pub(crate) catalog_maps: Arc<std::sync::OnceLock<platform::objects::Maps>>,
    /// The tables a non-manual subscriber listens to the delete events of (see
    /// `autoinc::tables_with_delete_subscribers`), found when `Truncate` first asks.
    /// The profiles that get an All Profile row (see `platform::profiles::profile_seeds`).
    pub(crate) profile_seeds: std::sync::OnceLock<Vec<(usize, i32, bool)>>,
    /// The tables `seed_platform_rows` visits (`platform::seed::seed_tables`), found on the first session; shared with
    /// sibling engines, which have the same program.
    pub(crate) seed_tables: Arc<std::sync::OnceLock<Box<[usize]>>>,
    /// The seed rows that depend on the program only (`platform::seed::program_seed_rows`), per object index, found on the
    /// first session.
    pub(crate) seed_rows: std::sync::OnceLock<Option<platform::seed::SeedRows>>,
    /// The rows of the test data, built once for every thread of this engine (ADR-069 decision 4, `test_rows.rs`).
    pub(crate) test_rows: Arc<test_rows::TestRows>,
    /// ADR-069 decision 11 (`AL_PRIVATE_BASELINE=1`): each thread builds its own rows of the test data, as before
    /// `test_rows`. The reference side of the differential check; only time and memory depend on it.
    pub private_test_rows: bool,
    pub(crate) delete_subscribed: std::sync::OnceLock<std::collections::HashSet<String>>,
    /// What a run of a routine on a transient instance can leave behind (`effects.rs`), per (object, routine).
    pub(crate) lasting_effects: std::sync::Mutex<std::collections::HashMap<(usize, usize), Option<effects::Effect>>>,
    /// Per object, per routine: what a call needs to know about the routine, found once (see [`RoutineFacts`]).
    facts: Arc<[std::sync::OnceLock<Box<[RoutineFacts]>>]>,
    /// The register bytecode of the routines, compiled on first call (`exec/`).
    pub(crate) exec: Arc<exec::ExecProgram>,
    /// Per object, the ids of its source positions in ascending order, found when the Code Coverage table is first built.
    pub(crate) positions_by_object: std::sync::OnceLock<Vec<Vec<u32>>>,
    /// Codeunit objects by id (see [`Engine::compiled_codeunit`] and [`Engine::codeunit_object`]).
    codeunits: Arc<std::collections::HashMap<Option<u32>, CodeunitsWithId>>,
    /// Per object, the provider of a virtual platform table, found when the first session is made (`platform/mod.rs`).
    pub(crate) served_tables: std::sync::OnceLock<Arc<[Option<al_builtins::platform::Virtual>]>>,
    /// What crates above the VM work out from this engine's program once, by type (see [`Engine::derived`]).
    derived: std::sync::Mutex<std::collections::HashMap<std::any::TypeId, Arc<dyn std::any::Any + Send + Sync>>>,
    /// What the install code of the program leaves that this runtime cannot know (ADR-046), worked out from the program
    /// once by the runner (`al-testrun` `install.rs`) before the first install routine runs.
    pub install_facts: std::sync::OnceLock<InstallFacts>,
}

/// The tables whose install rows a test may not read (ADR-046), found from the static closures of the install routines.
#[derive(Debug, Default, Clone)]
pub struct InstallFacts {
    /// Table object -> why its rows are not known: a table whose rows depend on the order of the install routines of one
    /// app and part (amendment 1, A2 items a to c). (Decision 4c, a shared table that per-company install code writes, is
    /// gone: the default mode models a database with one company, amendment 4.) A read whose answer depends on the rows
    /// is refused outside install code, as for `Engine::unseeded`.
    pub unknown: std::collections::HashMap<usize, String>,
    /// The tables of the apps under test (ADR-046 decision 5): under the opt-in, the only per-company tables answered in
    /// a declared company other than the session company.
    pub under_test_tables: std::collections::HashSet<usize>,
    /// Table object -> why the order of its install rows is not known: tables an app with two or more install routines
    /// in one part writes (A2 item d). A read of `SystemRowVersion`, `SystemCreatedAt` or `SystemModifiedAt` of a row
    /// install code wrote is refused.
    pub row_order: std::collections::HashMap<usize, String>,
    /// Table object -> (app under test, the assumption its install rows rely on): the install session of an app under test
    /// raises Company Triggers OnCompanyOpen(Completed) to the subscribers of every installed app (ADR-046 amendment 1, A4
    /// item 4, owner decision 2026-10-19), an assumption until probe Q3. A test that touches the table outside install code
    /// is a flagged pass.
    pub assumed: std::collections::HashMap<usize, (usize, al_builtins::assumptions::Assumption)>,
}

/// The codeunit objects with one id, first in program order.
#[derive(Clone, Copy, Default)]
struct CodeunitsWithId {
    /// The first compiled one (`Object::is_kind`).
    compiled: Option<usize>,
    /// The first one, compiled or a shell of a dependency app that was not compiled.
    any: Option<usize>,
}

/// What a call of a routine asks of the routine's declaration, answered once when the engine is built instead of by
/// scanning the attributes and the publisher table on every call.
#[derive(Clone, Copy, Default)]
pub(crate) struct RoutineFacts {
    /// The routine is an event publisher: a call raises the event (`dispatch.rs`).
    pub publisher: bool,
    /// `[TryFunction]`.
    pub is_try: bool,
    /// `[CommitBehavior(...)]`, when the routine declares one that is understood.
    pub commit_behavior: Option<transactions::CommitBehavior>,
    /// `[ErrorBehavior(ErrorBehavior::Collect)]`.
    pub collects: bool,
    /// An attribute is declared that the runtime does not understand (`CommitBehavior` other than Ignore and Error,
    /// `ErrorBehavior` other than Collect, or more than one of either): the call takes the slow path, which reports it.
    pub odd_attributes: bool,
    /// A call hands the routine the declared lengths of its `var` Text and Code arguments (`ir::Routine::reads_var_max_len`;
    /// never for a publisher, whose subscribers would see them).
    pub var_lens: bool,
}

fn routine_facts(r: &ir::Routine, publisher: bool) -> RoutineFacts {
    let mut f = RoutineFacts {
        publisher,
        ..Default::default()
    };
    let mut commits = 0;
    let mut collects = 0;
    for a in &r.attributes {
        if a.name.eq_ignore_ascii_case("TryFunction") {
            f.is_try = true;
        } else if a.name.eq_ignore_ascii_case("CommitBehavior") {
            commits += 1;
            f.commit_behavior = transactions::CommitBehavior::from_attribute(&a.text);
            f.odd_attributes |= f.commit_behavior.is_none();
        } else if a.name.eq_ignore_ascii_case("ErrorBehavior") {
            collects += 1;
            f.collects = errorcollect::declares_collect(std::slice::from_ref(a)).unwrap_or(false);
            f.odd_attributes |= !f.collects;
        }
    }
    f.odd_attributes |= commits > 1 || collects > 1;
    f.var_lens = r.reads_var_max_len && !publisher;
    f
}

impl Engine {
    /// Reads about how many bytes the test data rows this engine's threads share hold now (ADR-069 decisions 4 and
    /// 10), for the memory budget, which counts them once. Holds the rows' cache, not the engine.
    pub fn shared_test_rows_meter(&self) -> impl Fn() -> u64 + Send + Sync + 'static {
        let rows = self.test_rows.clone();
        move || rows.bytes()
    }

    /// Whether this engine and `other` read one cache of test data rows (an engine and its [`Engine::sibling`]): the
    /// memory budget then counts the rows once (ADR-069 decision 10).
    pub fn shares_test_rows_with(&self, other: &Engine) -> bool {
        Arc::ptr_eq(&self.test_rows, &other.test_rows)
    }

    pub fn new(program: impl Into<Arc<ir::Program>>, blocked: Vec<Vec<Option<String>>>) -> Arc<Engine> {
        Engine::with_config(program, blocked, None, dotnet::DotNetConfig::default())
    }

    /// An engine that runs as `target`: version-gated behaviour follows it (ADR-015 3.4).
    pub fn with_target(
        program: impl Into<Arc<ir::Program>>,
        blocked: Vec<Vec<Option<String>>>,
        target: Option<al_version::Target>,
    ) -> Arc<Engine> {
        Engine::with_config(program, blocked, target, dotnet::DotNetConfig::default())
    }

    /// An engine whose `DotNet` calls are served by `dotnet` (ADR-014). `new` uses no .NET.
    pub fn with_dotnet(
        program: impl Into<Arc<ir::Program>>,
        blocked: Vec<Vec<Option<String>>>,
        dotnet: dotnet::DotNetConfig,
    ) -> Arc<Engine> {
        Engine::with_config(program, blocked, None, dotnet)
    }

    /// Target and .NET host together.
    pub fn with_config(
        program: impl Into<Arc<ir::Program>>,
        blocked: Vec<Vec<Option<String>>>,
        target: Option<al_version::Target>,
        dotnet: dotnet::DotNetConfig,
    ) -> Arc<Engine> {
        // A declared TestPage or Page variable holds a page state of this crate (ADR-072 decision 3).
        pages::state::register_closed_page();
        let program: Arc<ir::Program> = program.into();
        // Every pass here reads heads, so a program whose objects are made on first read makes none (ADR-054).
        let heads = program.objects.heads();
        let tables: Vec<Option<Arc<al_data::TableDef>>> = heads
            .iter()
            .enumerate()
            .map(|(i, o)| {
                o.table.as_ref().map(|t| {
                    Arc::new(al_data::TableDef {
                        object: i as u32,
                        name: o.name.clone(),
                        fields: t
                            .fields
                            .iter()
                            .map(|f| al_data::FieldDef {
                                id: f.id,
                                name: f.name.clone(),
                                ty: f.ty.clone(),
                                stored: f.stored,
                                code_storage: code_storage(f),
                            })
                            .collect(),
                        keys: t.keys.clone(),
                        per_company: t.per_company,
                    })
                })
            })
            .collect();
        let events = events::Registry::build(&program);
        let facts = (0..heads.len()).map(|_| std::sync::OnceLock::new()).collect();
        let table_facts: Vec<TableFacts> = heads
            .iter()
            .map(|o| TableFacts {
                unresolved_init: o
                    .table
                    .as_ref()
                    .is_some_and(|t| t.fields.iter().any(|f| f.init_text.is_some() && f.init.is_none())),
                sql_timestamp: o.table.as_ref().map_or_else(Vec::new, |t| {
                    t.fields
                        .iter()
                        .enumerate()
                        .filter(|(_, f)| {
                            f.properties.iter().any(|(k, v)| {
                                k.eq_ignore_ascii_case("SQLTimestamp") && v.trim().eq_ignore_ascii_case("true")
                            })
                        })
                        .map(|(i, _)| i)
                        .collect()
                }),
                insert_unqueued: o.table.as_ref().is_none_or(|t| {
                    t.fields.iter().any(|f| {
                        f.auto_increment
                            || matches!(f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob))
                    })
                }),
                code_sql_type: o.table.as_ref().is_some_and(|t| {
                    t.fields.iter().any(|f| {
                        matches!(f.ty, RtType::Code(_))
                            && f.properties.iter().any(|(k, v)| {
                                k.eq_ignore_ascii_case("SQLDataType") && !v.trim().eq_ignore_ascii_case("Varchar")
                            })
                    })
                }),
            })
            .collect();
        let object_facts: Vec<ir::ObjectFacts> = heads
            .iter()
            .map(|h| ir::ObjectFacts::of_properties(&h.properties))
            .collect();
        let field_facts: Vec<Vec<ir::FieldFacts>> = heads
            .iter()
            .map(|o| {
                o.table
                    .iter()
                    .flat_map(|t| t.fields.iter().map(ir::FieldFacts::of))
                    .collect()
            })
            .collect();
        let enumerable_fields = (0..heads.len()).map(|_| Default::default()).collect();
        let mut unevaluable_init = std::collections::HashMap::new();
        for (i, o) in heads.iter().enumerate() {
            for (pos, f) in o.table.iter().flat_map(|t| t.fields.iter().enumerate()) {
                if let Some(message) = records::unevaluable_init_value(f) {
                    unevaluable_init.insert((i as u32, pos), message);
                }
            }
        }
        let unseeded = heads
            .iter()
            .enumerate()
            .filter(|(_, o)| {
                o.kind == ir::ObjKind::Table
                    && o.id
                        .is_some_and(|id| al_builtins::platform::UNSEEDED_ROWS.contains(&id))
            })
            .map(|(i, _)| i)
            .collect();
        let exec = exec::ExecProgram::new(&program);
        let mut codeunits: std::collections::HashMap<Option<u32>, CodeunitsWithId> = Default::default();
        for (i, o) in heads.iter().enumerate() {
            if o.kind == ir::ObjKind::Codeunit {
                let e = codeunits.entry(o.id).or_default();
                e.any.get_or_insert(i);
                if o.is_kind(ir::ObjKind::Codeunit) {
                    e.compiled.get_or_insert(i);
                }
            }
        }
        Arc::new(Engine {
            program,
            blocked,
            tables: tables.into(),
            events: Arc::new(events),
            target,
            dotnet,
            unseeded: Default::default(),
            platform_unseeded: unseeded,
            demo_tables: Default::default(),
            company_refusal: None,
            skipped_apps: Vec::new(),
            resource_overrides: Default::default(),
            resources_read: Default::default(),
            dotnet_called: Default::default(),
            test_data: None,
            license: None,
            license_caps: Default::default(),
            write_in_try_allowed: false,
            dependencies_published_global: false,
            unevaluable_init: Arc::new(unevaluable_init),
            table_facts: table_facts.into(),
            object_facts: object_facts.into(),
            field_facts: field_facts.into(),
            enumerable_fields,
            uid: NEXT_ENGINE_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            catalog_maps: Default::default(),
            delete_subscribed: Default::default(),
            lasting_effects: Default::default(),
            profile_seeds: Default::default(),
            seed_tables: Default::default(),
            seed_rows: Default::default(),
            test_rows: Default::default(),
            private_test_rows: false,
            facts,
            exec: Arc::new(exec),
            positions_by_object: Default::default(),
            codeunits: Arc::new(codeunits),
            served_tables: Default::default(),
            derived: Default::default(),
            install_facts: Default::default(),
        })
    }

    /// A new engine over this one's program, refusals, target and .NET host that shares what [`Engine::with_config`]
    /// works out from the program (table definitions, event registry, per-object and per-routine facts, bytecode), so
    /// it is not worked out again: a run's install baseline and its tests each have an engine. Everything a run sets is
    /// as `with_config` leaves it, and the engine has its own `uid`. The event registry is shared as it is, with what
    /// the runner set in it (`unavailable_global`), which reads only the compile. The cache of test data rows is shared
    /// too (`test_rows.rs`).
    pub fn sibling(&self) -> Arc<Engine> {
        Arc::new(Engine {
            program: self.program.clone(),
            blocked: self.blocked.clone(),
            tables: self.tables.clone(),
            events: self.events.clone(),
            target: self.target.clone(),
            dotnet: self.dotnet.clone(),
            unseeded: Default::default(),
            platform_unseeded: self.platform_unseeded.clone(),
            demo_tables: Default::default(),
            company_refusal: None,
            skipped_apps: Vec::new(),
            resource_overrides: Default::default(),
            resources_read: Default::default(),
            dotnet_called: Default::default(),
            test_data: None,
            license: None,
            license_caps: Default::default(),
            write_in_try_allowed: false,
            dependencies_published_global: false,
            unevaluable_init: self.unevaluable_init.clone(),
            table_facts: self.table_facts.clone(),
            object_facts: self.object_facts.clone(),
            field_facts: self.field_facts.clone(),
            enumerable_fields: self.enumerable_fields.clone(),
            uid: NEXT_ENGINE_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            catalog_maps: self.catalog_maps.clone(),
            delete_subscribed: Default::default(),
            lasting_effects: Default::default(),
            profile_seeds: Default::default(),
            seed_tables: self.seed_tables.clone(),
            seed_rows: Default::default(),
            // The rows are a function of the test data and the program, which the sibling shares: one cache for the
            // run's install engine and tests engine, so each table is built once per process (ADR-069 decision 4).
            test_rows: self.test_rows.clone(),
            private_test_rows: false,
            facts: self.facts.clone(),
            exec: self.exec.clone(),
            positions_by_object: Default::default(),
            codeunits: self.codeunits.clone(),
            served_tables: Default::default(),
            derived: Default::default(),
            install_facts: Default::default(),
        })
    }

    /// What a call of routine `routine` of object `object` needs to know about it, worked out for all routines of the
    /// object the first time one of them is asked about (the call makes the object anyway).
    #[inline]
    pub(crate) fn routine_facts(&self, object: usize, routine: usize) -> RoutineFacts {
        match self.facts[object].get() {
            Some(facts) => facts[routine],
            None => self.object_routine_facts(object)[routine],
        }
    }

    #[cold]
    fn object_routine_facts(&self, object: usize) -> &[RoutineFacts] {
        self.facts[object].get_or_init(|| {
            self.program.objects[object]
                .routines
                .iter()
                .enumerate()
                .map(|(r, routine)| routine_facts(routine, self.events.publishers.contains_key(&(object, r))))
                .collect()
        })
    }

    /// Puts back what a run sets on an engine after [`Engine::with_config`], so that a warm server can run its next
    /// request on the engine of the last one (al-runner issue 5856) as on a new engine. Every field is named here: a new
    /// field fails to compile until it is either reset or listed as worked out from the program and the config alone.
    /// The .NET host is the caller's to set again (`dotnet`).
    pub fn reset_run_state(&mut self) {
        let Engine {
            // Set by a run.
            unseeded,
            demo_tables,
            company_refusal,
            skipped_apps,
            resource_overrides,
            resources_read,
            dotnet_called,
            test_data,
            license,
            license_caps,
            write_in_try_allowed,
            dependencies_published_global,
            // From the program and the config of `with_config` (and al-testrun's `global_triggers_find_unavailable`,
            // which reads only the compile): the same for every run over this program.
            program: _,
            blocked: _,
            tables: _,
            events: _,
            target: _,
            dotnet: _,
            platform_unseeded: _,
            unevaluable_init: _,
            table_facts: _,
            object_facts: _,
            field_facts: _,
            facts: _,
            exec: _,
            codeunits: _,
            // Caches that are worked out from the program on first use.
            profile_seeds: _,
            seed_tables: _,
            seed_rows: _,
            test_rows,
            private_test_rows: _,
            delete_subscribed: _,
            lasting_effects: _,
            positions_by_object: _,
            enumerable_fields: _,
            served_tables: _,
            derived: _,
            install_facts: _,
            catalog_maps: _,
            // Kept: state shared between the sessions of this engine is keyed by it.
            uid: _,
        } = self;
        unseeded.clear();
        test_rows.clear();
        demo_tables.clear();
        *company_refusal = None;
        skipped_apps.clear();
        match resource_overrides.get_mut() {
            Ok(o) => o.clear(),
            Err(e) => e.into_inner().clear(),
        }
        *resources_read.get_mut() = false;
        *dotnet_called.get_mut() = false;
        *test_data = None;
        *license = None;
        *license_caps = Default::default();
        *write_in_try_allowed = false;
        *dependencies_published_global = false;
    }

    /// The first compiled codeunit with this id, in program order: the object `Codeunit.Run(Id)` runs. Found in an index
    /// built with the engine; the test runner runs codeunits by id around every test method (al-testrun runner_reset.rs).
    pub fn compiled_codeunit(&self, id: Option<u32>) -> Option<usize> {
        self.codeunits.get(&id).and_then(|c| c.compiled)
    }

    /// The first codeunit with this id in program order, compiled or a shell of a dependency app that was not compiled.
    pub fn codeunit_object(&self, id: Option<u32>) -> Option<usize> {
        self.codeunits.get(&id).and_then(|c| c.any)
    }

    /// The value of type `T` made from this engine's program, made by `make` the first time it is asked for and shared
    /// by every session and thread of the engine after that: for data that depends on the program alone (al-testrun's
    /// index of the static reach). `make` must not ask the engine for derived data itself.
    pub fn derived<T: std::any::Any + Send + Sync>(&self, make: impl FnOnce() -> T) -> Arc<T> {
        let mut slots = self.derived.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let slot = slots
            .entry(std::any::TypeId::of::<T>())
            .or_insert_with(|| Arc::new(make()) as Arc<dyn std::any::Any + Send + Sync>)
            .clone();
        drop(slots);
        slot.downcast::<T>()
            .expect("a derived value is stored under its own type")
    }
}

/// How many emptied vectors a session keeps for the next calls, and the largest capacity it keeps (`Session::spare_vec`):
/// as many as calls are nested in a deep stack of ordinary code, none of the large ones.
const SPARE_VECS: usize = 64;
const SPARE_CAPACITY: usize = 256;

/// The reason `Signal::Unsupported` carries when the runner stopped the test (`Session::stop_flag`).
pub const STOPPED: &str = "the runner stopped the test";

/// Whether the default value of a variable of this type holds no state of its own, so copies of one default are as good as
/// separate defaults.
fn plain_default(t: &RtType) -> bool {
    matches!(
        t,
        RtType::Boolean
            | RtType::Integer
            | RtType::BigInteger
            | RtType::Decimal
            | RtType::Byte
            | RtType::Char
            | RtType::Text(_)
            | RtType::Code(_)
            | RtType::Date
            | RtType::Time
            | RtType::DateTime
            | RtType::Duration
            | RtType::Guid
            | RtType::DateFormula
            | RtType::Option(_)
            | RtType::Enum(_)
            | RtType::Variant
    )
}

/// An Integer, BigInteger or Boolean for a parameter of its own type: `ops::convert_plain` hands it back unchanged, so a
/// call binds it without a look at the other kinds of argument.
#[inline]
fn passed_unchanged(a: &Value, t: &RtType) -> bool {
    matches!(
        (a, t),
        (Value::Integer(_), RtType::Integer)
            | (Value::BigInteger(_), RtType::BigInteger)
            | (Value::Boolean(_), RtType::Boolean)
    )
}

/// The session's encryption key.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum EncryptionKey {
    Present,
    Deleted,
    /// Created after a delete; whether `EncryptionEnabled` answers Yes again is not pinned.
    Recreated,
}

/// The session language and WorkDate at the start of a test ([`Session::carried_state`]).
#[derive(Debug, Clone, Copy)]
pub struct CarriedState {
    language: i32,
    work_date: Option<i32>,
}

/// The isolated storage and the number sequences of a session at a test runner's checkpoint
/// ([`Session::storage_checkpoint`]).
#[derive(Debug, Clone, Default)]
pub struct StorageCheckpoint {
    isolated: std::collections::HashMap<(String, String), (String, bool, bool)>,
    sequences: std::collections::HashMap<(String, String), sessionops::Sequence>,
}

pub struct Session {
    pub engine: Arc<Engine>,
    pub last_error: AlError,
    /// How many errors AL code caught in this session (`Session::catch_error`, caught_errors.rs).
    caught_errors: u64,
    depth: usize,
    /// Emptied vectors of call values (arguments, a frame's slots, a builtin's operands), taken again by the next call
    /// (`spare_vec`, `return_vec`): a call of a procedure or a builtin allocates nothing once there are some.
    spare_vecs: Vec<Vec<Value>>,
    /// The view each table element with LinkTable and no LinkFields below another table element had at its first pass of the running
    /// export, by (xmlport instance address, node): every later pass starts from it again (`xmlports.rs` `xp_pass_start`).
    pub(crate) xp_pass_views: Vec<(usize, usize, xmlports::PassView)>,
    /// The objects of the routines on the call stack, innermost last (builtins scoped to an app,
    /// `NavApp.GetCallerModuleInfo`).
    pub(crate) frames: Vec<usize>,
    /// The state of each table element without LinkTable right below another table element at the end of its last pass of the
    /// running export, by (xmlport instance address, node) (`xmlports.rs` `xp_unlinked_pass_start`).
    pub(crate) xp_unlinked_passes: Vec<(usize, usize, xmlports::UnlinkedPass)>,
    /// The table element records (by address) of linked elements with LinkFields whose link filter an export under
    /// SourceTableView moved into the view's group: their GetFilters reads '' afterwards (xmlports.rs, corpus PR 896).
    pub(crate) xp_link_hidden: Vec<usize>,
    /// The routine and current statement position of every frame, parallel to `frames`
    /// (`callstack.rs`).
    pub(crate) call_stack: Vec<CallFrame>,
    /// The object whose routine raised the last AL error that left a routine (the innermost frame of its call stack).
    pub(crate) error_raised_in: Option<usize>,
    /// The test runner's frames below a test, in `Callstack` form (`callstack.rs`); empty
    /// outside a test run.
    pub(crate) runner_frames: String,
    /// `faultsite.rs`: whether to record where errors and refusals leave from, and the last one.
    fault_trace: bool,
    fault_site: Vec<faultsite::FaultFrame>,
    /// A refusal was raised where `fault_site` cannot name its statement (`Session::refusal_site`).
    fault_site_lost: bool,
    /// `IsolatedStorage` by (scope, key): the value, whether it was stored encrypted, and whether it was stored from a
    /// SecretText (errors/CuTryTests.al:181-232: such a value cannot be read into a Text).
    pub(crate) isolated_storage: std::collections::HashMap<(String, String), (String, bool, bool)>,
    /// What the isolated storage held at the last commit: a rollback goes back to it (session/
    /// TestProbeSessionMiscContracts.al:IsolatedStorage_NotRolledBack, whose name is the opposite of its result).
    pub(crate) isolated_committed: std::collections::HashMap<(String, String), (String, bool, bool)>,
    /// The isolated storage keys the running test method set (a page background task reads them differently, sessionops.rs Get).
    pub(crate) isolated_set_in_test: std::collections::HashSet<(String, String)>,
    /// The isolated storage keys this test codeunit set, in this test or an earlier one (isolated_storage_reads.rs).
    pub(crate) isolated_written_in_codeunit: std::collections::HashSet<(String, String)>,
    /// The TaskScheduler tasks this session created (`tasks.rs`).
    pub(crate) tasks: tasks::Tasks,
    /// An install codeunit is running.
    install_scope: bool,
    /// The company data codeunit (Microsoft's DemoTool) builds the company (`set_company_build`).
    company_build: bool,
    /// The code runs in a session without a client (`set_no_client`).
    no_client: bool,
    /// The code runs in a session `StartSession` started (`bgsession.rs`).
    pub(crate) background_session: bool,
    /// How many sessions `StartSession` started in this session (`bgsession.rs`).
    pub(crate) sessions_started: i32,
    /// An install routine runs in BC's install session, which has no client (`set_install_without_client`).
    install_without_client: bool,
    /// The session's data comes from a baseline in which the company data codeunit built the company to its end
    /// (`Baseline::company_built`).
    pub(crate) company_built: bool,
    /// The `Engine::demo_tables` the running test's own code (not install code) wrote to (`records/op_inner.rs`): a read of
    /// one of the others is refused before the session's own code wrote it.
    demo_written: std::collections::HashSet<u32>,
    /// The running test's own code (not install code) wrote General Ledger Setup (table 98), before any caption of areas 1 or 101
    /// was resolved (`setup_caption_read_early`): such captions read the test's row (`refuse_setup_caption`).
    pub(crate) gl_setup_written: bool,
    /// A caption of areas 1 or 101 was resolved before the session's code wrote General Ledger Setup: Base Application's
    /// SingleInstance resolvers then keep the row install code wrote.
    pub(crate) setup_caption_read_early: bool,
    /// Registered table connections by (type member, lower-case name), and the default name per type.
    pub(crate) table_connections: std::collections::HashSet<(i32, String)>,
    pub(crate) default_table_connection: std::collections::HashMap<i32, String>,
    /// The rows of each registered CRM table connection, by lower-case name (`crm.rs`).
    pub(crate) crm_stores: std::collections::HashMap<String, crm::CrmStore>,
    /// Number sequences by (company, name); a global sequence has an empty company.
    pub(crate) sequences: std::collections::HashMap<(String, String), sessionops::Sequence>,
    /// Each sequence this session restarted or deleted, as it was just before the first of those (`numbers_taken.rs` in
    /// al-testrun: what a codeunit under runner 130450 leaves of it).
    pub(crate) sequences_changed: std::collections::HashMap<(String, String), sessionops::SequenceChange>,
    /// The one instance of each `SingleInstance = true` codeunit in this session.
    singletons: std::collections::HashMap<usize, Rc<RefCell<Instance>>>,
    /// The notes on the watched single instances' globals (`single_track.rs`).
    si_track: Option<Box<single_track::Track>>,
    /// Single instances moved in from an earlier test codeunit's session that this session has not used yet.
    singles_unclaimed: std::collections::HashSet<usize>,
    pub(crate) store: al_data::memory::MemStore,
    /// Private tables of temporary records, by temp id.
    pub(crate) temps: std::collections::HashMap<u64, al_data::memory::MemStore>,
    next_temp: u64,
    /// Ids of temporary tables whose last record variable has gone (`TempId`); freed on the next use of a temporary table.
    pub(crate) temp_release: Rc<RefCell<Vec<u64>>>,
    pub(crate) next_guid: u64,
    /// The key mixed into every Guid the session makes (see [`Session::fresh_guid`]): the host's seed
    /// (`al_host::seed::fixed_guid_key`), or the test method's (`Session::seed_test_method`, ADR-061).
    guid_key: u64,
    /// What `Randomize()` without a seed reseeds from, plus a count: the host's seed, or the test method's derived seed
    /// (ADR-061).
    pub(crate) unseeded_base: i32,
    /// AL code of this session called `Randomize(n)`: the sequence goes on into the next test of the codeunit, as in BC
    /// (types/TestRandomSeededContracts.al:111-131), so a test method's start no longer reseeds it (ADR-061).
    pub(crate) randomized_with_seed: bool,
    /// The session's `Random` (`System.Random`, `Randomize`); see `hostvalues.rs`.
    pub(crate) rng: al_host::random::NetRandom,
    /// How many numbers `Random` has drawn in this session (a test that draws after tests that did not run sees another
    /// sequence than BC's).
    pub(crate) random_draws: u64,
    /// How many AL routine calls the session has made: `Record.IsDirty` notes whether a record write ran triggers or
    /// subscribers (isdirty.rs).
    pub(crate) calls_made: u64,
    /// `SessionInformation.SqlStatementsExecuted` and `SqlRowsRead` (sqlcount.rs).
    pub(crate) sql: sqlcount::SqlCounters,
    /// How many record methods that Record.IsDirty tracks are running (isdirty.rs).
    pub(crate) dirty_ops_open: u32,
    /// The writes with triggers that are running, innermost last (isdirty.rs `DirtyFrame`).
    pub(crate) dirty_frames: Vec<crate::isdirty::DirtyFrame>,
    /// The xRec handles that triggers and table events were given, while they live (xrec_blobs.rs).
    pub(crate) live_xrecs: Vec<std::rc::Weak<std::cell::RefCell<al_builtins::value::RecordState>>>,
    /// Whether the test running has reseeded the `Random` sequence (`Randomize`); see [`Session::inherited_random_draws`].
    pub(crate) reseeded_in_test: bool,
    /// How many numbers `Random` has drawn from a sequence the test running did not seed itself.
    pub(crate) inherited_draws: u64,
    /// The run seed reseeded `Random` when the test running began (`Session::seed_test_method`, ADR-061).
    pub(crate) run_seeded_in_test: bool,
    /// Of `inherited_draws`, those from a sequence the run seed did not restart either: they go on with what the tests
    /// before drew (after a `Randomize(n)` of theirs).
    pub(crate) carried_draws: u64,
    /// How many `Randomize()` calls without a seed the session has seen.
    pub(crate) unseeded_randomizes: i32,
    pub(crate) next_row_version: i64,
    /// How many Delete calls and DeleteAll calls that found a row have reached the store, and every Truncate call (on an empty
    /// table too): with `next_row_version`, it tells whether code wrote.
    pub(crate) deletes: u64,
    /// How many times the store rolled back (`rollback_transaction`): a rollback takes rows back and moves neither count above.
    pub(crate) rollbacks: u64,
    /// The highest SystemRowVersion install code gave a row (`Session::mark_install_end`): a row at or below it was
    /// written by install code (ADR-046 amendment 1, A2 item d).
    pub(crate) install_row_version: i64,
    /// The last row version of the restored database (`TableProvider::last_row_version`), 0 without one: the session's
    /// counter starts there, and rows at or below it are restored rows, not install code's.
    pub(crate) restored_row_version: i64,
    /// `SessionId()` of an install session, which is not the test session (ADR-046 amendment 1, A4 item 1).
    session_id_override: Option<i32>,
    /// Last AutoIncrement value handed out per table (and company). It is not part of the
    /// store: a rollback undoes rows but not the counter (corpus
    /// record/TestAutoIncrementExplicitKey.al:154).
    pub(crate) autoinc: std::collections::HashMap<(String, u32), i64>,
    /// The counter each table had before the first Truncate of the open transaction; a
    /// rollback puts it back (corpus record/TestAutoIncrementEdges.al:36-170).
    pub(crate) truncate_undo: std::collections::HashMap<(String, u32), i64>,
    /// Records inside an OnValidate (by handle) with the field values from before the outermost
    /// Validate, which nested Validates of the same record see as xRec (corpus
    /// record/TestXRecContracts.al:133).
    pub(crate) validating: Vec<(usize, Vec<Value>)>,
    /// The records (by handle) whose `validating` entry a part's link stamp pushed (`stamp_from_filters_seen_as`): the
    /// platform validates the link value, so the entry holds whatever code runs at that moment.
    pub(crate) stamp_validating: Vec<usize>,
    /// The call site named for the Validate about to start, with its record's handle (`Session::validate_at`).
    pub(crate) validate_site: Option<(usize, crate::records::ValidateSite)>,
    /// The Validates running now, innermost last: the record's handle and the call site (`Session::take_validate_site`).
    pub(crate) validate_sites: Vec<(usize, Option<crate::records::ValidateSite>)>,
    /// The table procedures that run on a record by a plain call (no operation of the record called them), innermost
    /// last: the record's handle, the depth of `call_stack` inside the procedure and its blank xRec. A Validate of that
    /// record in the procedure's own body passes this xRec (`RecordOp::Validate`).
    pub(crate) plain_proc_xrec: Vec<(usize, usize, crate::records::Rec)>,
    pub(crate) cx: al_data::AccessContext,
    /// The tables that held a row when the install baseline was captured (none without one).
    pub(crate) baseline_rows: std::collections::BTreeSet<u32>,
    /// The origin of the baseline the session restored and the store's change counter right after
    /// (`Session::baseline_origin`).
    pub(crate) baseline_origin: Option<(u64, u64)>,
    /// Why a write of the platform's seed rows was refused (`platform::seed`); the session then refuses every test.
    pub(crate) seed_refusal: Option<String>,
    /// Whether the builtin being called has its result used (ADR-011 `ret_used`).
    pub(crate) ret_used: bool,
    /// The routine `call` returned from last ended by running `exit(value)` (al-vm pages/record_trigger_answer.rs).
    pub(crate) exited_with_value: bool,
    /// The form of the text of the `FieldError` being called, for the text `GetLastErrorText(true)` answers
    /// (`Ctx::field_error_with_form`); `None` outside such a call.
    pub(crate) field_error_form: Option<al_builtins::native::ErrorTextForm>,
    /// The parameters of the routine that just failed with an error, as it left them:
    /// the caller writes its `var` arguments back before it handles the error.
    pub(crate) error_finals: Option<Vec<Value>>,
    /// The scalar globals passed by `var` to a procedure call that is still running. BC passes the argument by reference,
    /// the runtime copies it in and back; the two differ only when code reaches the global by its own name while the call
    /// runs. To a procedure of the same instance the callee's parameter aliases the global (`interp::alias_access`,
    /// codeunits/procedures/VarGlobalAlias.al); to another instance such an access is refused. Empty almost always,
    /// so the check on a global access is one length test.
    pub(crate) var_alias_watch: Vec<AliasWatch>,
    /// The transaction model of the running test method (set by the test runner).
    pub tx_model: TxModel,
    /// Whether a test codeunit is executing (a test codeunit cannot be run from one).
    pub in_test_codeunit: bool,
    /// The test codeunit runs under its RequiredTestIsolation = Function or Disabled, as BC's TestRunnerHub runs it
    /// (al-testrun `RunOptions::required_isolation`), not under a test runner codeunit's TestIsolation.
    pub isolation_from_hub: bool,
    /// The test codeunit runs under Disabled isolation: from the hub (`isolation_from_hub` set) or from a test runner's
    /// TestIsolation = Disabled.
    pub hub_disabled: bool,
    /// `MinimumActiveRowVersion` while the tests of a test codeunit run (`min_row_version.rs`).
    pub(crate) iso_min: min_row_version::IsoMin,
    /// The test runner's step that assigns the permission sets of a test method runs (codeunit 130302 "Set State Test
    /// Run", al-testrun `runner_reset.rs`): `PermissionTestHelper.AddEffectivePermissionSet` runs there on the assumption
    /// `runner-permission-sets-grant-all` instead of refusing.
    pub runner_permission_step: bool,
    /// ADR-066 decision 1: the test codeunit run of a test runner codeunit (`test_runner_host.rs`).
    pub(crate) runner_host: test_runner_host::RunnerHost,
    /// `al-runner test --ignore-permission-mock`: a test that assigns a permission set itself
    /// (`PermissionTestHelper.AddEffectivePermissionSet`) runs on with every access granted, on the assumption
    /// `permissions-not-enforced`, instead of being refused.
    pub ignore_permission_mock: bool,
    /// `al-runner test --permissions off` (ADR-062 section 5): no access kind is enforced, whatever stage is on by
    /// default (`permissions::Stage`).
    pub permissions_off: bool,
    /// The Role IDs `PermissionTestHelper.AddEffectivePermissionSet` assigned since the last `Clear` (ADR-062).
    pub(crate) permission_roles: Vec<String>,
    /// What the assigned sets give; `None` when the session is unrestricted (no set assigned, or sets that grant
    /// everything such as SUPER). Every access point checks only `is_some()` (`permissions.rs`).
    pub(crate) permissions: Option<Arc<permissions::EffectivePermissions>>,
    /// The test runner's step assigned its sets since the last `Clear` (they are not composed: assumption
    /// `runner-permission-sets-grant-all`); a set the test assigns on top of them leaves a session that is not fully
    /// composed (ADR-062 probe P25).
    pub(crate) runner_permission_sets: bool,
    /// The shadow run (`permission_shadow.rs`): the roles of the runner's step, and whether the session evaluates them.
    pub(crate) shadow_roles: Vec<String>,
    pub(crate) shadow_active: bool,
    pub(crate) shadow_lower_bound: bool,
    /// Set by AL code's record method call right before `record_op` of a restricted session, taken there (ADR-062).
    pub(crate) permission_from_al: bool,
    /// True while the fields of `SetAutoCalcFields` are calculated after a read: that read is allowed (Y24PermReads.al
    /// Y24_P15_FlowFieldsNothingOnSource, `autocalc_findfirst`).
    pub(crate) permission_autocalc: bool,
    /// True while a page calculates the FlowFields of its controls: BC drops such a control when the session cannot read the
    /// source table (Y24PermReads.al Y24_P15, `testpage_field`), which the runtime does not model, so a denial refuses.
    pub(crate) permission_page_calc: bool,
    /// The source table (object index) of a page while it opens (`open_page_as`): a read the page makes of it, when the table is
    /// empty, raises nothing (`Session::page_open_read_allowed`).
    pub(crate) permission_opening_source: Option<usize>,
    /// Why a report the running test reaches cannot run (set by the test runner from the static reach, `None`: no such
    /// report). A permission denial the test would catch is refused when it is set: the test goes on to the report, is
    /// refused there, and by then has changed state a later test of the codeunit reads (ADR-062 stage 2).
    pub report_refusal_after_denial: Option<String>,
    /// The object whose `Permissions` property and app decide the accesses of the next checks, where no AL frame makes them
    /// (a query, xmlport or report reading its tables, ADR-062 stage 2). `None`: the object of the current frame.
    pub(crate) permission_caller: Option<usize>,
    /// The xmlport whose export runs: the reads the runtime makes for it (not AL code) are decided as that object's.
    pub(crate) permission_run_object: Option<usize>,
    /// The query whose method runs (the object index), for the read check of `query_rows`.
    pub(crate) query_object: usize,
    /// The object index of the test codeunit being run (set by the test runner).
    pub test_object: Option<usize>,
    /// The `TestHttpRequestPolicy` of the running test codeunit (`None`: the property is absent).
    pub(crate) http_policy: Option<&'static str>,
    /// A scratch run (`al-runner serve`'s `execute`): the text of every `Message` raised outside a test
    /// method, in order. `None`: such a call is refused, as BC's behaviour then is not pinned.
    pub(crate) scratch_messages: Option<Vec<String>>,
    /// `--tdd` (ADR-051 decision 5): the stubs that ran since the last [`Session::take_tdd_reached`].
    pub(crate) tdd_reached: std::collections::BTreeSet<String>,
    /// The filters in filter group 1 (the global group) last set on a database record, per table object: BC keeps them on
    /// every record of the table for the rest of the session (records.rs `global_group_check`).
    pub(crate) global_group1: std::collections::HashMap<u32, String>,
    /// Value capture and loop tracking (`capture.rs`); `None`: off, and the statements run as they always did.
    pub(crate) capture: Option<Box<capture::Capture>>,
    /// Which executable form the next routine runs (one byte): the plain one, the captured one (a capture is on and the
    /// routine is one of the run's apps), or the debug one (a debugger is attached, ADR-048).
    pub(crate) form: Form,
    /// The debugger attached to this session (ADR-048); `None`: no debugger, and the statements run as they always did.
    pub(crate) debug: Option<Box<debug::Debugger>>,
    /// The constants of the program that cost something to build (a Decimal parsed from its text, a shared text, an
    /// Option's member list), built the first time a node is evaluated, by the address of the node: the program does not
    /// change while the session runs.
    constants: std::collections::HashMap<usize, Value, PtrHash>,
    /// Set from another thread to stop the AL code of this session: every loop iteration checks it
    /// and ends the running test with `Signal::Unsupported` (the runner's per-test time limit).
    pub(crate) stop: Arc<std::sync::atomic::AtomicBool>,
    /// How many times the session made its changes durable (`commit_transaction`). A runner reads
    /// it around a test method to see whether the method committed (test isolation, `isolation.rs`
    /// in al-test-cli).
    pub commits: u64,
    /// The `[ErrorBehavior(Collect)]` scopes on the call stack and the errors they hold
    /// (see `errorcollect.rs`).
    pub(crate) error_scopes: errorcollect::Scopes,
    /// The field values of a fresh record, by table: see `Session::initial_fields`.
    pub(crate) initial_cache: std::collections::HashMap<usize, records::InitialFields>,
    /// Tables with an ad-hoc key appended, by (table object, key fields): see `Session::table`.
    pub(crate) adhoc_tables: RefCell<AdhocTables>,
    /// Imported media objects and media sets (the platform tables behind Media fields).
    pub(crate) media: al_builtins::media::MediaStore,
    /// Media objects a Modify or Delete removed from Tenant Media: `.0` since the last commit (a rollback puts them back),
    /// `.1` committed since the test runner's checkpoint (its restore puts them back). See `Session::release_media`.
    pub(crate) media_removed: (Vec<RemovedMedia>, Vec<RemovedMedia>),
    /// A `ModifyAll(Field, Value, false)` of a table without modify events is running: every row is written, also one whose value
    /// does not change (recmeta.rs `modify_all`).
    pub(crate) bulk_modify: bool,
    /// The RecordRefs that Variants hold, and whether the `RecordRef.GetTable` that runs was given a Variant
    /// (`recordref_variant.rs`).
    pub(crate) variant_refs: recordref_variant::VariantRefs,
    /// Set while a `TestField.SetValue` written with a Variant argument runs (`TestField.SetValueVariant#1`).
    pub(crate) set_value_from_variant: bool,
    /// Set while a host page's `TestPage.New()` runs: the parts it links afresh run no OnNewRecord.
    pub(crate) host_new_running: Option<bool>,
    /// Set while a host page's open links its parts (`sync_parts_at_open`).
    pub(crate) part_open_running: bool,
    /// The host move whose parts are reading again: Next and Previous, a GoToKey or GoToRecord that stayed on the row, and the
    /// moves `pageops.rs` syncs in its last branch (GoToRecord, GoToKey and the others) (`pages/empty_part_rec.rs`).
    pub(crate) host_move_running: Option<pages::HostMove>,
    /// A host move syncs a part that serves its rows from OnFindRecord or OnNextRecord: the part keeps its rows when its link
    /// values did not change (`pages/parts.rs` `sync_parts_with`).
    /// `Some(true)`: a part that shows no row or only its blank line stays as it is too (a part linked through a Provider that
    /// stayed on its row).
    pub(crate) part_sync_keeps_same_link: Option<bool>,
    /// The host session behind `DotNet` values; ends with the session (ADR-014 section 6).
    pub(crate) dotnet: dotnet::DotNetState,
    /// The session's application areas as `ApplicationArea()` reads them: `#Basic,#Suite`, or the
    /// empty text, which enables every area. The test runner empties it before each test method
    /// (corpus session-system/TestSessionApplicationAreaTestBoundary.al:67552).
    pub application_areas: String,
    /// The clock and the identity (ADR-006).
    pub(crate) host: al_host::Host,
    /// The companies the run declared, the session company first (ADR-046 decision 2, `--companies`); empty for the
    /// default of one company.
    pub(crate) companies: Vec<String>,
    /// (company, table) -> why the rows of a per-company table in a company other than the session company are not known
    /// (ADR-046 decision 6, `Session::mark_company_unknown`).
    pub(crate) company_unknown: std::collections::HashMap<(String, u32), String>,
    /// The companies made by `Database.CopyCompany` from the session company (or from another such company): they hold
    /// the session company's rows as they were, so the rows install code wrote there are known
    /// (`Session::copy_company`, `Session::cx_for`).
    pub(crate) full_copies: std::collections::BTreeSet<String>,
    /// While `Database.CopyCompany` inserts the Company row: why the `RunTrigger` of the Company's insert events is not
    /// known. The subscribers get it as an unknown value (`Session::table_event`).
    pub(crate) run_trigger_unknown: Option<&'static str>,
    /// While `NavApp.LoadPackageData` inserts the rows of a data file: no table event is raised (`package_data`).
    pub(crate) package_load: bool,
    /// The time `Sleep` has waited, added to every clock reading.
    pub(crate) waited_ms: i64,
    /// The running test's time limit in milliseconds, which its `Sleep` calls count toward (al-runner issue 5605), and
    /// the time they have asked for since the test started. `None` outside a test with a limit.
    pub(crate) sleep_budget: Option<i64>,
    pub(crate) slept_in_test: i64,
    /// Text this session encrypted: ciphertext to plain text (`Encrypt`, `Decrypt`).
    pub(crate) encrypted: std::collections::HashMap<String, String>,
    /// Where the session's encryption key stands: a key exists at the start (misc: session-system/isolated-storage/EncryptionKeyDelete.al
    /// State_AtStart), `DeleteEncryptionKey` removes it and `CreateEncryptionKey` makes a new one.
    pub(crate) encryption_key: EncryptionKey,
    /// Temporary files created so far (`File.CreateTempFile` names them in order).
    pub(crate) temp_files: u32,
    /// ADR-040: the files a report on the allow-list was saved to in a rendered format, and for which nothing was
    /// written (path in lower case with `/`, name of the report). Any look at one is refused; a write by AL over the
    /// path ends the mark.
    pub(crate) render_pending_files: std::collections::HashMap<String, Arc<str>>,
    /// How a placeholder file came to be and whether an AL File holds it open (`placeholder_files.rs`).
    pub(crate) placeholder_files: placeholder_files::PlaceholderFiles,
    /// The instant of the last system stamp (milliseconds since the Unix epoch).
    pub(crate) last_stamp: i64,
    /// The key and count of the draws of how far past a stamp the clock lands (`Session::stamp_millis`): the run seed's
    /// (`al_host::seed::clock_key`), per test method once `Session::seed_test_method` ran (ADR-045 amendment 2026-10-31).
    pub(crate) clock_key: u64,
    pub(crate) clock_draws: u64,
    /// The date `WorkDate(Date)` set; today until then.
    pub(crate) work_date: Option<i32>,
    /// The language `GlobalLanguage(Id)` set.
    pub(crate) global_language: i32,
    /// What `CaptionClassTranslate` translated in this session: (class text, GlobalLanguage then, caption). Cleared by
    /// `SelectLatestVersion` (types/PSCaptionClass.al:94-115).
    pub(crate) caption_translations: Vec<(String, i32, String)>,
    /// The (class text, GlobalLanguage) pairs the `CaptionClassTranslate` statement raised the UI Helper event for.
    pub(crate) ui_caption_translated: Vec<(String, i32)>,
    /// The row `load` is about to read came from `Get` (partial.rs: a `Get` of the row the buffer holds keeps what is loaded).
    pub(crate) get_load: bool,
    /// The row `load` is about to read came from `GetBySystemId` (al-runner issue 5913: a read in the transaction that wrote
    /// the row's version, record/crud/SbStaleOwnModify.al:467-640).
    pub(crate) sysid_load: bool,
    /// The row `load` is about to read came from `Find`, `FindFirst`, `FindLast` or `FindSet`: its buffer gets the origin a Get
    /// buffer gets (record/crud/Y76StaleBuffers.al Y76_Part1_FindFirst_FindLast, Y76_Part1_FindSet_Assign).
    pub(crate) find_load: bool,
    /// The find about to start runs through a RecordRef: its flagged error text has rules of its own
    /// (`records/empty_find_origin.rs`).
    pub(crate) find_through_recordref: bool,
    /// A RecordRef method that forwards to the record method of its name is running: that method, and those nested in it,
    /// are not a Record variable's (`assigned_audit.rs`).
    pub(crate) through_recordref: bool,
    /// Transactions: the depth of transaction-beginning constructs, and where the open transaction stands.
    pub(crate) tx: TxState,
    /// Handler functions and notifications of the running test method.
    pub(crate) ui_state: UiState,
    /// What the TestPage client keeps between calls.
    pub(crate) pages: PageClients,
    /// Event subscriptions and what the Event Subscription table reports.
    pub(crate) events: EventState,
    /// Code coverage recording.
    pub(crate) coverage: CoverageState,
    /// The report being run: language, trigger flow, dataset.
    pub(crate) report: ReportState,
}

/// Nested AL calls allowed before the session gives up. The corpus pins that a chain of 990 calls
/// completes and one of 5000 is refused (codeunits/TestCodeunitRecursionDepth.al).
/// The deepest call chain, the test method included: 991 recursive calls under a test run (codeunit/
/// TestCodeunitRecursionDepth.al:RecursionDepth_JustInsideTheCeiling_Completes); a [TryFunction] counts for two frames
/// (session-system/TestProbeSessionMiscContracts.al:CallDepth_990, CallDepth_1000, CallDepth_1001).
pub const MAX_CALL_DEPTH: usize = 992;

/// The number sequences of the restored database, as a new session starts with them: the company's own under its
/// name, the others under none. A sequence that is not known to have handed out a value (its current value is its
/// start) hands out the start first.
fn restored_sequences(
    engine: &Engine,
    company: &str,
) -> std::collections::HashMap<(String, String), sessionops::Sequence> {
    let mut out = std::collections::HashMap::new();
    if let Some(td) = &engine.test_data {
        for s in td.provider.sequences() {
            let scope = if s.company_specific {
                company.to_string()
            } else {
                String::new()
            };
            out.insert(
                (scope, s.name),
                sessionops::Sequence {
                    increment: s.increment,
                    seed: s.start,
                    last: (s.current != s.start).then_some(s.current),
                },
            );
        }
    }
    out
}

impl Session {
    /// How many `SetTableView` views the session still holds (tests only: an entry must not outlive its record).
    #[doc(hidden)]
    pub fn set_view_filter_entries(&self) -> usize {
        self.report.set_view_filters.len()
    }

    /// A session on the deterministic test host: a frozen clock and AL Runner's identity.
    pub fn new(engine: Arc<Engine>) -> Session {
        Session::with_host(engine, al_host::Host::default())
    }

    /// The files AL code wrote through the `File` type, for a test or a front end to inspect.
    pub fn files(&self) -> &dyn al_host::FileSystem {
        &*self.host.files
    }

    /// A session whose clock, identity and file system come from `host`.
    pub fn with_host(engine: Arc<Engine>, host: al_host::Host) -> Session {
        // The session of a run with test data works in the restored company.
        let host = match &engine.test_data {
            Some(td) => host.in_company(td.provider.company()),
            None => host,
        };
        let (company, user) = (host.identity.company_name(), host.identity.user_id());
        let (random_seed, guid_seed) = (host.random_seed, host.guid_seed);
        let clock_key = al_host::seed::clock_key(host.run_seed, 0, "");
        let mut ident = platform::Ident::of(&host);
        let declared = host.identity.companies();
        let restored = match declared {
            al_host::Companies::All => engine.test_data.as_ref().map(|td| td.provider.companies()),
            _ => None,
        };
        ident.resolve_companies(declared, restored);
        let companies = ident.companies.clone();
        let event_stats = Rc::new(platform::EventStats::default());
        let media = al_builtins::media::MediaStore::default();
        let mut store = al_data::memory::MemStore::with_virtual_tables(Rc::new(platform::Providers::new(
            &engine,
            ident.clone(),
            event_stats.clone(),
            media.clone(),
        )))
        .as_database();
        if let Some(td) = &engine.test_data {
            store = store.with_backing(Rc::new(testdata::TestBacking::new(&engine, td.clone())));
        }
        let sequences = restored_sequences(&engine, &company);
        // A restored database hands out row versions after its own: every row a session writes sorts after the restored
        // rows, as on BC, where the counter is database wide (`TableProvider::last_row_version`).
        let restored_row_version = engine
            .test_data
            .as_ref()
            .and_then(|td| td.provider.last_row_version())
            .map_or(0, |v| i64::try_from(v).unwrap_or(i64::MAX));
        al_builtins::ops::set_turkic_code(al_builtins::native::is_turkic_language(host.identity.global_language()));
        al_builtins::filter::set_filter_language(host.identity.global_language());
        let mut session = Session {
            global_language: host.identity.global_language(),
            caption_translations: Vec::new(),
            ui_caption_translated: Vec::new(),
            get_load: false,
            sysid_load: false,
            find_load: false,
            find_through_recordref: false,
            through_recordref: false,
            host,
            waited_ms: 0,
            companies,
            company_unknown: Default::default(),
            full_copies: Default::default(),
            run_trigger_unknown: None,
            package_load: false,
            sleep_budget: None,
            slept_in_test: 0,
            encrypted: Default::default(),
            encryption_key: EncryptionKey::Present,
            temp_files: 0,
            render_pending_files: std::collections::HashMap::new(),
            placeholder_files: Default::default(),
            last_stamp: i64::MIN / 2,
            clock_key,
            clock_draws: 0,
            work_date: None,
            engine,
            last_error: AlError::default(),
            caught_errors: 0,
            depth: 0,
            spare_vecs: Vec::new(),
            xp_pass_views: Vec::new(),
            singletons: Default::default(),
            si_track: None,
            singles_unclaimed: Default::default(),
            xp_unlinked_passes: Vec::new(),
            xp_link_hidden: Vec::new(),
            store,
            temps: Default::default(),
            next_temp: 0,
            temp_release: Default::default(),
            next_guid: 0,
            guid_key: al_host::seed::fixed_guid_key(guid_seed.unwrap_or(random_seed)),
            unseeded_base: random_seed,
            randomized_with_seed: false,
            rng: al_host::random::NetRandom::new(random_seed),
            random_draws: 0,
            calls_made: 0,
            sql: sqlcount::SqlCounters::new_session(),
            dirty_ops_open: 0,
            dirty_frames: Vec::new(),
            live_xrecs: Vec::new(),
            reseeded_in_test: false,
            inherited_draws: 0,
            run_seeded_in_test: false,
            carried_draws: 0,
            unseeded_randomizes: 0,
            next_row_version: restored_row_version,
            deletes: 0,
            rollbacks: 0,
            install_row_version: restored_row_version,
            restored_row_version,
            session_id_override: None,
            autoinc: Default::default(),
            truncate_undo: Default::default(),
            validating: Vec::new(),
            stamp_validating: Vec::new(),
            validate_site: None,
            validate_sites: Vec::new(),
            plain_proc_xrec: Vec::new(),
            cx: al_data::AccessContext {
                company: company.into(),
                user: user.into(),
            },
            baseline_rows: Default::default(),
            baseline_origin: None,
            seed_refusal: None,
            ret_used: true,
            exited_with_value: false,
            field_error_form: None,
            error_finals: None,
            var_alias_watch: Vec::new(),
            frames: Vec::new(),
            call_stack: Vec::new(),
            error_raised_in: None,
            runner_frames: String::new(),
            fault_trace: false,
            fault_site_lost: false,
            fault_site: Vec::new(),
            isolated_storage: Default::default(),
            isolated_committed: Default::default(),
            isolated_set_in_test: Default::default(),
            isolated_written_in_codeunit: Default::default(),
            tasks: Default::default(),
            install_scope: false,
            company_build: false,
            no_client: false,
            background_session: false,
            sessions_started: 0,
            install_without_client: false,
            company_built: false,
            demo_written: Default::default(),
            gl_setup_written: false,
            setup_caption_read_early: false,
            sequences,
            sequences_changed: Default::default(),
            table_connections: Default::default(),
            default_table_connection: Default::default(),
            crm_stores: Default::default(),
            tx_model: TxModel::default(),
            in_test_codeunit: false,
            isolation_from_hub: false,
            iso_min: Default::default(),
            hub_disabled: false,
            runner_permission_step: false,
            runner_host: Default::default(),
            ignore_permission_mock: false,
            permissions_off: false,
            permission_roles: Vec::new(),
            runner_permission_sets: false,
            shadow_roles: Vec::new(),
            shadow_active: false,
            shadow_lower_bound: false,
            permission_from_al: false,
            permission_autocalc: false,
            permission_page_calc: false,
            permission_opening_source: None,
            report_refusal_after_denial: None,
            permission_caller: None,
            permission_run_object: None,
            query_object: 0,
            permissions: None,
            test_object: None,
            http_policy: None,
            scratch_messages: None,
            tdd_reached: Default::default(),
            global_group1: Default::default(),
            capture: None,
            form: Form::Plain,
            debug: None,
            constants: Default::default(),
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            commits: 0,
            error_scopes: Default::default(),
            initial_cache: Default::default(),
            adhoc_tables: Default::default(),
            media,
            media_removed: Default::default(),
            bulk_modify: false,
            variant_refs: Default::default(),
            set_value_from_variant: false,
            host_new_running: None,
            part_open_running: false,
            host_move_running: None,
            part_sync_keeps_same_link: None,
            dotnet: Default::default(),
            application_areas: String::new(),
            tx: TxState {
                committed_row_version: restored_row_version,
                ..TxState::fresh()
            },
            ui_state: UiState::default(),
            pages: PageClients::default(),
            events: EventState {
                bindings: Vec::new(),
                page_held: Vec::new(),
                page_var_runs: Vec::new(),
                pages_with_instance: Vec::new(),
                event_stats,
                publish_frame: false,
                only_targets: None,
                sender_refused: None,
                var_origins: Vec::new(),
                next_aliases: Vec::new(),
                next_is_subscriber: false,
                subscriber_stack: Vec::new(),
                next_var_lens: Vec::new(),
                publisher_var_lens: Vec::new(),
                unpinned_event_depth: 0,
                skip_dependency_subscribers: false,
                not_installed_apps: None,
                order_unknown_apps: None,
                order_unknown_hits: Default::default(),
                install_written: None,
                install_rows: None,
                setup_no_ops: Default::default(),
                trial: Default::default(),
                global_setups: Default::default(),
            },
            coverage: CoverageState::default(),
            report: ReportState::default(),
        };
        session.seed_platform_rows(&ident);
        session
    }

    /// What the AL test runner does before each test method: every application area on, and the
    /// transaction type back at UpdateNoLocks. What an earlier test wrote to a demo table does not count for this one: the corpus
    /// tests that write setup rows put the demo values back afterwards (record/QkSetupRows.al), which leaves the
    /// runtime with the install-code row (record/captions/QdCaptionClassAreas.al:798 QF_GLSetup_CorpusRow).
    pub fn start_test_method(&mut self) {
        // A test method starts with no permission set of its own assigned: the runner's mock step (131006 OnRun) and
        // its set step clear the assignment first; a test whose lowering was refused leaves none behind. The runner's
        // sets, which its set step assigned just before, stay (`runner_permission_sets`; the test runner calls this
        // again after its steps).
        let runner = self.runner_permission_sets;
        let shadow_roles = std::mem::take(&mut self.shadow_roles);
        self.clear_permission_sets();
        self.runner_permission_sets = runner;
        self.shadow_roles = shadow_roles;
        self.shadow_begin();
        self.application_areas.clear();
        self.isolated_set_in_test.clear();
        self.demo_written.clear();
        self.gl_setup_written = false;
        self.tx.transaction_type = 0;
        self.tx.tx_phase = crate::transactions::TX_FRESH;
        self.tx.lock_tx = false;
        self.tx.isolation_lock = false;
        self.tx.lock_unpinned = None;
        self.tx.recref_lock_tx = false;
        self.tx.isolation_lock_tables.clear();
        self.tx.locking_reads.clear();
        self.tx.db_reads = 0;
        self.tx.locking_read_tx_unpinned = false;
    }

    /// Makes every change so far durable (and a Truncate of this transaction final).
    pub fn commit_transaction(&mut self) {
        self.tx.deferred_insert = None;
        self.guarded_body_commit();
        al_data::Store::commit(&mut self.store);
        self.crm_committed();
        self.commits += 1;
        self.sql.committed();
        self.truncate_undo.clear();
        // Rows at or below this rowversion are durable: a buffer older than one of them is stale.
        self.tx.committed_row_version = self.next_row_version;
        self.tx.row_versions = Default::default();
        self.tx.rows_touched.clear();
        self.tx.tables_inserted.clear();
        self.tx.locked_tables.clear();
        self.tx.writes.clear();
        self.tx.insert_buffer.clear();
        self.tx.commit_counter = self.store.change_counter();
        self.tx.rolled_back = false;
        self.tx.reads_after_rollback.clear();
        self.tx.rollback_unpinned = None;
        self.tx.rollback_kind = None;
        // A transaction a lock opened counts as one that wrote: MinimumActiveRowVersion after it is not pinned.
        self.tx.last_commit_wrote = self.tx.tx_phase == crate::transactions::TX_WRITE || self.tx.lock_tx;
        self.isolated_committed = self.isolated_storage.clone();
        self.pages.saved_values_committed = self.pages.saved_values.clone();
        self.tasks.commit();
        let removed = std::mem::take(&mut self.media_removed.0);
        self.media_removed.1.extend(removed);
        self.end_write_phase();
    }

    /// The test runner took a checkpoint of the committed data: what was removed before it stays removed.
    pub fn media_checkpoint(&mut self) {
        self.media_removed.1.clear();
    }

    /// Media a Modify or Delete removed come back into Tenant Media, with a value again.
    fn restore_media(&mut self, removed: Vec<RemovedMedia>) {
        let mut gone = self.media.removed.borrow_mut();
        for (id, _) in &removed {
            gone.remove(id);
        }
        self.media.objects.borrow_mut().extend(removed);
    }

    /// The test runner went back to its checkpoint: the media removed since come back, as the rows that held them do.
    pub fn restore_media_checkpoint(&mut self) {
        let removed = std::mem::take(&mut self.media_removed.1);
        self.restore_media(removed);
    }

    /// The isolated storage and the number sequences now, for [`Session::restore_storage_checkpoint`] (a test runner's
    /// TestIsolation = Function, al-testrun `isolation::Checkpoint`).
    pub fn storage_checkpoint(&self) -> StorageCheckpoint {
        StorageCheckpoint {
            isolated: self.isolated_storage.clone(),
            sequences: self.sequences.clone(),
        }
    }

    /// Puts the isolated storage and the number sequences back as they were at `cp`. A key or a sequence that `cp`
    /// already held and that changed or went away since is put back too, but the answer is `false`: only a new key's
    /// `IsolatedStorage.Set` and a new sequence's `NumberSequence.Insert` are measured to be rolled back. `None`: nothing
    /// changed since `cp`.
    pub fn restore_storage_checkpoint(&mut self, cp: &StorageCheckpoint) -> Option<bool> {
        if self.isolated_storage == cp.isolated && self.sequences == cp.sequences {
            return None;
        }
        let kept_storage = cp.isolated.iter().all(|(k, v)| self.isolated_storage.get(k) == Some(v));
        let kept_sequences = cp.sequences.iter().all(|(k, v)| self.sequences.get(k) == Some(v));
        self.isolated_storage = cp.isolated.clone();
        self.isolated_committed = cp.isolated.clone();
        self.sequences = cp.sequences.clone();
        Some(kept_storage && kept_sequences)
    }

    /// The transaction a write call started has ended (record/TestProbeRecordRefContracts.al:WriteTransaction_State starts
    /// it with a write that changes nothing; codeunits/TestWriteTxTestBoundary.al:Test10 to Test13d end it with the run or
    /// the page operation, and a rollback ends it too).
    /// The `Consistent(false)` marks end with the transaction too.
    fn end_write_phase(&mut self) {
        self.tx.inconsistent.clear();
        self.tx.lock_tx = false;
        self.tx.isolation_lock = false;
        self.tx.lock_unpinned = None;
        self.tx.recref_lock_tx = false;
        self.tx.isolation_lock_tables.clear();
        self.tx.locking_reads.clear();
        self.tx.db_reads = 0;
        self.tx.locking_read_tx_unpinned = false;
        if self.tx.tx_phase == crate::transactions::TX_WRITE {
            self.tx.tx_phase = crate::transactions::TX_FRESH;
        }
    }

    /// Discards every change since the last commit. The AutoIncrement counters are not
    /// part of the data and stay (corpus record/TestAutoIncrementExplicitKey.al:154), except
    /// those a Truncate of this transaction changed.
    /// A rollback inside the run (asserterror, a guarded `Codeunit.Run`): refused while a Feature Key change of this
    /// transaction is pending, since whether BC undoes it is not settled (metadata/platform-tables/AwTests.al:557-576
    /// says it does not, on the Linux CI only).
    pub(crate) fn rollback_in_run(&mut self) -> Result<(), Signal> {
        if self.store.virtual_write_pending() {
            return Err(Signal::Unsupported(
                "a rollback of an uncommitted Feature Key change (whether it is undone is a Linux suspect, not settled)".into(),
            ));
        }
        self.rollback_transaction();
        Ok(())
    }

    pub fn rollback_transaction(&mut self) {
        self.note_rollback_for_stale_rule();
        self.tx.deferred_insert = None;
        al_data::Store::rollback(&mut self.store);
        self.rollbacks += 1;
        self.crm_rolled_back();
        self.sql.rolled_back();
        self.tx.row_versions = Default::default();
        self.tx.rows_touched.clear();
        self.tx.tables_inserted.clear();
        self.tx.locked_tables.clear();
        self.tx.writes.clear();
        self.tx.insert_buffer.clear();
        self.tx.commit_counter = self.store.change_counter();
        self.tx.rolled_back = true;
        self.tx.reads_after_rollback.clear();
        self.isolated_storage = self.isolated_committed.clone();
        // What SaveValues pages stored is kept in the database: a rollback takes back what was stored since the last commit. An
        // asserterror under TestIsolation = Codeunit (one transaction for the codeunit) takes back what an earlier test stored
        // (pages/PnF02SaveValuesExt.al:207-234, SaveValues_Ext_CancelThenReopen after SaveValues_Ext_OkThenReopen),
        // while without one the next test reads it (session-system/session/SaveValuesAcrossCodeunits.al
        // Dialog_NextTestOfSameCodeunit_Reads, reports/W3ReportRequestPageState.al
        // SaveValues_2_RunLaterTest_ReadsWhatTheFirstTestConfirmed).
        self.pages.saved_values = self.pages.saved_values_committed.clone();
        self.tasks.rollback();
        // The Tenant Media rows a Modify or Delete of this transaction removed come back with the rows.
        let removed = std::mem::take(&mut self.media_removed.0);
        self.restore_media(removed);
        // The code coverage tables keep their rows through a rollback (codeunits/PWCodeCoverageRows.al:
        // Rollback_RowsReadBeforeAnAsserterror, Rollback_RowsFirstReadInsideFailingScope): what was
        // recorded is written into them again at the next read.
        self.coverage_rolled_back();
        for (slot, value) in std::mem::take(&mut self.truncate_undo) {
            self.autoinc.insert(slot, value);
        }
        self.end_write_phase();
    }

    /// The signatures of the `--tdd` stubs that ran since the last call, sorted (ADR-051 decision 5).
    pub fn take_tdd_reached(&mut self) -> Vec<String> {
        std::mem::take(&mut self.tdd_reached).into_iter().collect()
    }

    /// The session's database, for runners that manage transactions around tests.
    pub fn store_mut(&mut self) -> &mut al_data::memory::MemStore {
        &mut self.store
    }

    /// The initial value of a slot of type `t`.
    pub fn default_value(&mut self, t: &RtType) -> Value {
        match t {
            RtType::Record(table, temporary) => {
                // A table declared `TableType = Temporary` is temporary for every variable
                // (corpus record 60198 TempOnly_IsTemporary_AlwaysTrue).
                let temporary = &(*temporary || self.engine.object_facts[*table as usize].temporary());
                let temp_id = temporary.then(|| self.new_temp_id());
                Value::Record(Rc::new(RefCell::new(RecordState {
                    table: *table,
                    temporary: *temporary,
                    temp_id,
                    company: None,
                    crm: self.engine.object_facts[*table as usize].crm(),
                    fields: self.initial_fields(*table as usize),
                    filters: Vec::new(),
                    filter_group: 0,
                    marks: Vec::new(),
                    marked_only: false,
                    read_isolation: 0,
                    // A record in a test run is Validated (corpus record/TestSecurityFilteringContracts.al:161-175).
                    security_filtering: 0,
                    declared_security_filtering: 0,
                    declared_security_filtering_unpinned: false,
                    security_filtering_unpinned: false,
                    security_filtering_array: false,
                    key: 0,
                    adhoc: None,
                    ascending: true,
                    field_descending: 0,
                    position: None,
                    auto_calc: Vec::new(),
                    load: Default::default(),
                    touched: false,
                    assigned: false,
                    inserted: false,
                    dirty_base: None,
                    is_dirty_base: al_builtins::value::DirtyBase::Fresh,
                    last_modify: None,
                    failed_insert: None,
                    row_origin: None,
                    own_read: Default::default(),
                    globals: Default::default(),
                    unpinned_buffer: None,
                    run_page_link: None,
                })))
            }
            // Each element of an array is a variable of its own: an array of records holds records, not
            // nulls (Tests-ERM "Discount Setup Notifications": `A: array[2] of Record "General Posting Setup"`).
            RtType::Array(dims, el) => {
                let n = dims.iter().map(|d| *d as usize).product();
                // An element that is a plain value (no shared state behind it) is built once and copied.
                let cells: Vec<Value> = if plain_default(el) {
                    vec![Value::default_for(el); n]
                } else {
                    (0..n).map(|_| self.default_value(el)).collect()
                };
                // The elements of an array of temporary records share one temporary table: Inventory Profile
                // Offsetting keeps demand and supply in `InventoryProfile: array[2] of Record "Inventory Profile"
                // temporary`, fills element 1 and plans with element 2 (Base Application, Microsoft's Tests-SCM-Planning
                // need it). Not pinned by a corpus test (assumption temporary-record-array-shares-table).
                if let Some(Value::Record(first)) = cells.first() {
                    if let Some(id) = first.borrow().temp_id.clone() {
                        for c in cells.iter().skip(1) {
                            if let Value::Record(rc) = c {
                                rc.borrow_mut().temp_id = Some(id.clone());
                            }
                        }
                        // record/temporary/RecordTempArrayElements.al:P174_NonTempArray_SharesDatabaseTable, P174_Array_ThirdElement_SharesToo.
                    }
                }
                Value::Array(Rc::new(RefCell::new(cells)))
            }
            other => Value::default_for(other),
        }
    }

    /// A fresh instance of codeunit `object` with default globals.
    /// What `ModuleInfo` reports for app `index` of the program, from its manifest.
    fn module_of_app(&self, index: usize) -> Option<al_builtins::value::ModuleInfoState> {
        let app = self.engine.program.apps.get(index)?;
        let version = {
            let parts: Vec<Option<i32>> = app.version.split('.').map(|p| p.parse().ok()).collect();
            match parts.as_slice() {
                [Some(a), Some(b), Some(c), Some(d)] => Some([*a, *b, *c, *d]),
                _ => None,
            }
        };
        Some(al_builtins::value::ModuleInfoState {
            id: ops::parse_guid(&app.id)?,
            name: app.name.clone(),
            publisher: app.publisher.clone(),
            version,
            dependencies: {
                let mut deps: Vec<[u8; 16]> = app.dependencies.iter().filter_map(|d| ops::parse_guid(d)).collect();
                // The app under test also depends on the "Application" app, which its manifest states as `application`
                // (session-system/TestProbeSessionMiscContracts.al:ModuleInfo_CurrentModule: 6 dependencies, the Application one last).
                deps.sort();
                if !app.dependency {
                    if let Some(application) = ops::parse_guid(al_builtins::moduleinfo::APPLICATION_APP_ID) {
                        if !deps.contains(&application) {
                            deps.push(application);
                        }
                    }
                }
                // BC lists them by id.
                deps.sort();
                deps
            },
            synthesized: false,
        })
    }

    /// How many private tables of temporary records the session holds (a table is freed once no record
    /// variable uses it any more).
    pub fn temporary_tables_held(&mut self) -> usize {
        for dead in self.temp_release.borrow_mut().drain(..) {
            self.temps.remove(&dead);
        }
        self.temps.len()
    }

    /// The identity of a new private table for a temporary record.
    /// An empty vector with room for `n` values of a call, from the spares when there is one.
    #[inline]
    pub(crate) fn spare_vec(&mut self, n: usize) -> Vec<Value> {
        let mut v = self.spare_vecs.pop().unwrap_or_default();
        v.reserve(n);
        v
    }

    /// Gives `v` back to the spares, emptied (its values are dropped in order, as a drop of the vector does).
    #[inline]
    pub(crate) fn return_vec(&mut self, mut v: Vec<Value>) {
        v.clear();
        if v.capacity() != 0 && self.spare_vecs.len() < SPARE_VECS && v.capacity() <= SPARE_CAPACITY {
            self.spare_vecs.push(v);
        }
    }

    pub(crate) fn new_temp_id(&mut self) -> al_builtins::value::TempId {
        self.next_temp += 1;
        al_builtins::value::TempId::new(self.next_temp, self.temp_release.clone())
    }

    /// The runner marks the time install codeunits run: `DataTransfer` is usable in it. Changes
    /// install code makes to rows the restored company holds are allowed and written down
    /// (ADR-020 amendment).
    pub fn set_install_scope(&mut self, on: bool) {
        self.install_scope = on;
        self.store.record_restored_changes(on);
    }

    /// The session runs on a server of kind `environment` from now on (`Environment Information` and the tenant helper
    /// answer for it). The install baseline runs a company data codeunit on the server kind its recipe names
    /// (`al-testrun/src/company_data.rs`); the tests keep the run's own `--environment`.
    pub fn set_environment(&mut self, environment: al_host::Environment) {
        let host = std::mem::take(&mut self.host);
        self.host = host.in_environment(environment);
    }

    /// The server kind the session runs on.
    pub fn environment(&self) -> al_host::Environment {
        self.host.identity.environment()
    }

    /// The apps (indexes in `Program::apps`) that are not installed yet: their event subscribers do not run. `None`:
    /// every app is installed. Microsoft's Legacy lane installs only its base apps before the DemoTool runs, the rest
    /// after it (BCApps build/scripts/ImportTestDataInBcContainer.ps1:907-923), so the install baseline holds the others
    /// back while the company is built.
    pub fn set_apps_not_installed(&mut self, apps: Option<std::collections::BTreeSet<u32>>) {
        self.events.event_stats.install_view.borrow_mut().not_installed = apps.clone().unwrap_or_default();
        self.events.not_installed_apps = apps;
    }

    /// The app (index in `Program::apps`) whose install code runs: its own row of "NAV App Installed App" does not exist yet.
    /// `set_apps_install_order_unknown(None)`, which ends an install, clears it.
    pub fn set_installing_app(&mut self, app: Option<u32>) {
        self.events.event_stats.install_view.borrow_mut().installing = app;
    }

    /// The apps (indexes in `Program::apps`) with no dependency path to the app whose install code runs: whether BC installs
    /// them before or after it is not known, so an event the install code raises that one of their subscribers would
    /// receive, and `NavApp.GetModuleInfo` of one of them, are refused. `None`: no install runs.
    pub fn set_apps_install_order_unknown(&mut self, apps: Option<std::collections::BTreeSet<u32>>) {
        {
            let mut view = self.events.event_stats.install_view.borrow_mut();
            view.order_unknown = apps.clone().unwrap_or_default();
            if apps.is_none() {
                view.installing = None;
            }
        }
        self.events.order_unknown_apps = apps;
        // What a subscriber handed to its publisher lives in the install session of the app that installed (order_trial.rs).
        self.events.trial.guarded.clear();
    }

    /// The tables (object indexes) the install code of the apps under test may write. A subscriber whose install order is
    /// not known and that stops unless it finds a row of an empty table of its own app outside this set does nothing in
    /// either order (empty_reads.rs). `None`: not known.
    pub fn set_install_written_tables(&mut self, tables: Option<std::collections::BTreeSet<u32>>) {
        self.events.install_written = tables;
    }

    /// The company data codeunit runs (al-testrun baseline.rs): the rules that hold only while the company is built apply
    /// (owner decision 2026-10-22: `Report.SaveAsPdf` to a file runs the report and writes no file, reports.rs).
    pub fn set_company_build(&mut self, on: bool) {
        self.company_build = on;
    }

    /// The code runs in a session without a client, as a step of the Contoso company build does (al-testrun baseline.rs,
    /// `CompanyData::no_client`; owner decision 2026-10-23): `GuiAllowed` answers false and marks
    /// `contoso-build-no-client`. Nothing else changes: Message is captured and Confirm refused as in any company build.
    pub fn set_no_client(&mut self, on: bool) {
        self.no_client = on;
    }

    /// `--no-client` (al-runner issue 7564): the test session is a session without a client. `GuiAllowed` answers false,
    /// `Message` returns and reaches no handler, `Confirm` raises CallbackNotAllowed whatever its default (all three as a page
    /// background task worker does, pages/background-tasks/BjNoGuiDialogs.al:8-10; that a web-service session does the same is
    /// the assumption `no-client-session`), and the other dialogs answer as that worker does or are refused (`clientless.rs`).
    pub fn set_clientless_session(&mut self, on: bool) {
        self.ui_state.clientless = on;
    }

    /// An install routine of an app runs (or no longer runs): BC runs it in a session without a client. `GuiAllowed()` is
    /// false, `CurrentClientType()` is SOAP, a `Message` does not stop it, and StrMenu, Confirm and `Page.Run` raise
    /// CallbackNotAllowed (errors/RlInstallUi.al:210-241, metadata/CmInstallLeads.al:54-61).
    /// Not set while the company is built: Company-Initialize and the company data codeunit keep their own rules.
    pub fn set_install_without_client(&mut self, on: bool) {
        self.install_without_client = on;
    }

    /// The session state a test method can change for the next test besides the database and the globals: the language
    /// (`GlobalLanguage`) and WorkDate. The test runner puts it back after a test refused while it ran (`restore_carried`).
    pub fn carried_state(&self) -> CarriedState {
        CarriedState {
            language: self.global_language,
            work_date: self.work_date,
        }
    }

    /// Puts back what [`Session::carried_state`] took: a test refused while it ran leaves nothing of it behind, as its
    /// database writes are rolled back (al-testrun `run.rs`).
    pub fn restore_carried(&mut self, state: CarriedState) {
        self.global_language = state.language;
        self.events.event_stats.language.set(state.language);
        al_builtins::ops::set_turkic_code(al_builtins::native::is_turkic_language(state.language));
        al_builtins::filter::set_filter_language(state.language);
        self.work_date = state.work_date;
    }

    /// Sets the session's WorkDate to the day number `day`, as `WorkDate(d)` does.
    pub fn set_work_date(&mut self, day: i32) {
        self.work_date = Some(day);
    }

    /// The session's WorkDate as a day number: the date set, or today.
    pub fn work_date(&self) -> i32 {
        self.work_date_day().expect("always a date")
    }

    /// The milliseconds the session's clock moved on besides the real clock (`Sleep`, and each reading by AL code):
    /// a run of code that leaves it unchanged read no clock.
    pub fn clock_offset_ms(&self) -> i64 {
        self.waited_ms
    }

    /// Whether the session notes the code it runs: AL code coverage, `--coverage` counts or the impact trace.
    pub fn notes_code_run(&self) -> bool {
        self.coverage.code_coverage || self.coverage.coverage_counts.is_some() || self.coverage.impact_on
    }

    /// Whether the session holds the single instance of codeunit `object` (one of its routines or variables used it).
    pub fn has_single_instance(&self, object: usize) -> bool {
        self.singletons.contains_key(&object)
    }

    /// The company the session's record operations work in.
    pub fn access_company(&self) -> &str {
        &self.cx.company
    }

    /// The change stamp of table object `object` in the session's company ([`al_data::memory::MemStore::table_stamp`]:
    /// equal stamps mean the same rows; 0, the rows the backing offers). `None` for a table without a schema or whose
    /// rows are computed.
    pub fn table_stamp(&self, object: u32) -> Option<u64> {
        let t = self.engine.tables.get(object as usize)?.as_ref()?;
        if self.store.is_virtual(t) {
            return None;
        }
        Some(self.store.table_stamp(&self.cx, t))
    }

    /// Whether the session has a client that is not a background session: BC's login code runs only then
    /// (Base Application `LogInManagement.CompanyOpen`, guarded by GuiAllowed and a client type other than Background).
    pub fn has_interactive_client(&self) -> bool {
        !self.install_without_client
            && !self.no_client
            && !self.ui_state.clientless
            && self.host.identity.gui_allowed()
            && self.host.identity.current_client_type() != al_host::ClientType::Background
    }

    /// Whether an install routine runs (`set_install_scope`).
    pub fn in_install_scope(&self) -> bool {
        self.install_scope
    }

    /// Install code has ended: every row whose SystemRowVersion is at or below the current one was written by it, or
    /// before it (ADR-046 amendment 1, A2 item d).
    /// How many row versions the session has handed out so far.
    pub fn row_versions_handed_out(&self) -> i64 {
        self.next_row_version
    }

    pub fn mark_install_end(&mut self) {
        self.install_row_version = self.next_row_version;
    }

    /// The `SessionId()` this session answers: an install session's own id, else the host's.
    pub(crate) fn session_id(&self) -> i32 {
        self.session_id_override
            .unwrap_or_else(|| self.host.identity.session_id())
    }

    /// The session runs as an install session with id `id`, or again as the host's session (`None`). BC installs each
    /// app in a session of its own, which is not the test session (record/BfInstallTests.al:142-144,
    /// :168; ADR-046 amendment 1, A4 item 1).
    pub fn set_install_session_id(&mut self, id: Option<i32>) {
        self.session_id_override = id;
    }

    /// Drops the session's SingleInstance codeunit instances: the next use starts from the initial state. BC's install
    /// session does this at the start of each company and of the per-database part
    /// (record/BfInstallTests.al:125-140; ADR-046 amendment 1, A4 item 2).
    pub fn reset_single_instances(&mut self) {
        self.singletons.clear();
    }

    /// Runs one subscriber of a platform event that has no parameters (the install events of `Extension Triggers`) on
    /// the instance the event would give it: the session's one instance of a SingleInstance codeunit, else a fresh one.
    pub fn call_parameterless_subscriber(&mut self, object: usize, routine: usize) -> Result<(), Signal> {
        let r = &self.engine.program.objects[object].routines[routine];
        if r.params != 0 {
            return Err(Signal::unsupported(format!(
                "subscriber {}.{} declares parameters, and the event it subscribes to has none",
                self.engine.program.objects[object].name, r.name
            )));
        }
        let fresh = Rc::new(RefCell::new(Instance {
            object: object as u32,
            ..Default::default()
        }));
        let inst = self.shared_instance(fresh);
        self.call(object, routine, inst, Vec::new()).map(|_| ())
    }

    /// The changes made to restored rows since install scope was last entered.
    pub fn restored_changes(&self) -> Vec<al_data::memory::RestoredChange> {
        self.store.restored_changes()
    }

    /// How many numbers `Random` has drawn in this session.
    pub fn random_draws(&self) -> u64 {
        self.random_draws
    }

    /// How many numbers `Random` has drawn, in this session, before the test drawing them reseeded the sequence: those
    /// numbers depend on what the tests before it drew. A draw after `Randomize` in the same test does not (the
    /// sequence restarts from the seed), so it is not counted.
    pub fn inherited_random_draws(&self) -> u64 {
        self.inherited_draws
    }

    /// Of [`Session::inherited_random_draws`], the numbers drawn from a sequence that the run seed did not restart at the
    /// start of the test either (ADR-061): they go on with what the tests before drew, after a `Randomize(n)` of theirs
    /// (types/TestRandomSeededContracts.al:111-131).
    pub fn carried_random_draws(&self) -> u64 {
        self.carried_draws
    }

    /// A test method of codeunit `codeunit` begins, before the test runner's reset (ADR-061): with `random`, the
    /// session's `Random` starts from `al_host::seed::derive(run seed, codeunit, method)` (unless AL code of this session
    /// called `Randomize(n)`: that sequence goes on, types/TestRandomSeededContracts.al:111-131) and `Randomize()`
    /// reseeds from it; with `guids`, the method's Guids and SystemIds come from its own key and counter. So a test's
    /// values depend on the run seed and its own name, not on the tests before it.
    pub fn seed_test_method(&mut self, codeunit: u32, method: &str, random: bool, guids: bool) {
        let run_seed = self.host.run_seed;
        if random {
            let seed = al_host::seed::derive(run_seed, codeunit, method);
            self.unseeded_base = seed;
            self.unseeded_randomizes = 0;
            if !self.randomized_with_seed {
                // The draws still count as inherited (`Session::inherited_random_draws`): an earlier test that did not
                // finish could have called `Randomize(n)`, and then this test would have gone on with that sequence.
                // They are not carried (`Session::carried_random_draws`).
                self.rng = al_host::random::NetRandom::new(seed);
                self.run_seeded_in_test = true;
            }
        }
        if guids {
            self.guid_key = al_host::seed::guid_key(run_seed, codeunit, method);
            self.next_guid = 0;
        }
        self.clock_key = al_host::seed::clock_key(run_seed, codeunit, method);
        self.clock_draws = 0;
    }

    /// A test method begins, before the test runner's reset: until something reseeds the `Random` sequence (the reset's
    /// `LibraryRandom.SetSeed(1)` included), what `Random` draws depends on the tests before (see
    /// [`Session::inherited_random_draws`]).
    pub fn start_random_scope(&mut self) {
        self.reseeded_in_test = false;
        self.run_seeded_in_test = false;
    }

    /// The instance a codeunit runs on when the session starts it: the session's single instance for a SingleInstance
    /// codeunit (the one its variables and event subscribers use), else a fresh one. A test codeunit with SingleInstance =
    /// true runs its test methods on it (codeunits/QiSingleInstanceTestCodeunit.al:57, :98).
    pub fn codeunit_instance(&mut self, object: usize) -> Rc<RefCell<Instance>> {
        let fresh = self.instance(object);
        self.shared_instance(fresh)
    }

    pub fn instance(&mut self, object: usize) -> Rc<RefCell<Instance>> {
        let engine = self.engine.clone();
        let globals = self.slot_defaults(&engine.program.objects[object].globals);
        Rc::new(RefCell::new(Instance {
            object: object as u32,
            globals,
            initialized: true,
            rerun: false,
            base: None,
            exts: Vec::new(),
            xp_text: Vec::new(),
            born: Default::default(),
            single: false,
        }))
    }

    /// A fresh instance of page or report extension `object` that is linked to `base`, the
    /// instance of the page or report it extends (ADR-023).
    pub(crate) fn extension_instance(&mut self, object: usize, base: &Rc<RefCell<Instance>>) -> Rc<RefCell<Instance>> {
        let ext = self.instance(object);
        ext.borrow_mut().base = Some(Rc::downgrade(base));
        ext
    }

    /// A fresh instance of a table or table extension for one record variable (ADR-021). Its `Rec`
    /// and `xRec` slots hold `Null` until a call fills them (`Session::enter_record`), so no
    /// record state is built for them.
    pub(crate) fn table_instance(&mut self, object: usize) -> Rc<RefCell<Instance>> {
        let (rs, xs) = {
            let o = &self.engine.program.objects[object];
            (o.rec_slot, o.xrec_slot)
        };
        let engine = self.engine.clone();
        let globals = engine.program.objects[object]
            .globals
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if Some(i) == rs || Some(i) == xs {
                    Value::Null
                } else {
                    self.slot_default(s)
                }
            })
            .collect();
        Rc::new(RefCell::new(Instance {
            object: object as u32,
            globals,
            initialized: true,
            rerun: false,
            base: None,
            exts: Vec::new(),
            xp_text: Vec::new(),
            born: Default::default(),
            single: false,
        }))
    }

    /// A record type whose default value has no state of its own (no temporary table): a record of a stored table.
    fn record_without_state(&self, t: &RtType) -> bool {
        let RtType::Record(table, false) = t else { return false };
        if self.engine.object_facts[*table as usize].temporary() {
            return false;
        }
        true
    }

    /// A RecordRef (held by a Variant) handed by value to a Record parameter of table `table`: of the same table, the Record
    /// `ops::record_ref_into_record` makes (streams/ArRecordProbes.al:366 VariantRecordRef_PassedToRecordParameter).
    /// Of another table it raises ObjectDefinitionChanged, naming the first field of the ref's table: pinned for that field a Code
    /// where the parameter's table has an Integer (ArRecordProbes.al SG_VariantRecordRef_OtherTable_AssignAndParameter `param`,
    /// corpus bump PR 738); other pairs of types, and a ref that is not open, raise an error whose text is not pinned.
    pub(crate) fn record_ref_into_record_param(
        &mut self,
        h: &Rc<RefCell<al_builtins::value::LibraryValue>>,
        table: u32,
    ) -> Result<Value, Signal> {
        let other = match &*h.borrow() {
            al_builtins::value::LibraryValue::RecordRef(rec) if rec.borrow().table == table => {
                return al_builtins::ops::record_ref_into_record(&rec.borrow());
            }
            al_builtins::value::LibraryValue::RecordRef(rec) => rec.borrow().table,
            al_builtins::value::LibraryValue::Empty => {
                return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())));
            }
            _ => {
                return Err(Signal::Unsupported(
                    "a RecordRef value that is no record into a Record parameter".into(),
                ));
            }
        };
        let first = |t: u32| {
            self.engine.program.objects[t as usize]
                .table
                .as_ref()
                .and_then(|s| s.fields.first())
                .map(|f| (f.name.clone(), f.ty.clone()))
        };
        if let (Some((_, RtType::Integer)), Some((name, RtType::Code(_)))) = (first(table), first(other)) {
            return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                message: format!(
                    "The definition of the {name} field has changed; old type: Integer, new type: Code. Start your activity again."
                ),
                code: "ObjectDefinitionChanged".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        Err(Signal::Error(Box::new(al_builtins::native::unpinned_error())))
    }

    /// The copy a parameter declared `temporary` (or of a TableType = Temporary table) gets of a record handed by value: an
    /// empty table of its own, the current values, no filters, the primary key ascending, no marks (see `call_inner`).
    fn temporary_param_copy(&mut self, rc: &Rc<RefCell<RecordState>>) -> Value {
        let mut st = rc.borrow().copy_as_new_variable();
        st.key = 0;
        st.adhoc = None;
        st.ascending = true;
        st.field_descending = 0;
        st.marks.clear();
        st.marked_only = false;
        st.filters.clear();
        st.filter_group = 0;
        st.temporary = true;
        st.temp_id = Some(self.new_temp_id());
        Value::Record(Rc::new(RefCell::new(st)))
    }

    /// A record passed by value is a copy (ADR-010). A temporary record's copy
    /// gets an independent, empty table of its own (corpus
    /// record/TestTempTableContracts.al:TempVar_PassedByValue_CallerNotAffected).
    pub(crate) fn record_by_value(&mut self, rc: &Rc<RefCell<RecordState>>) -> Value {
        let mut st = rc.borrow().copy_as_new_variable();
        if st.temporary {
            st.temp_id = Some(self.new_temp_id());
        }
        // A by-value copy of a record with a key on a Media field answers the primary key as its key (record/W4SortKeys.al:471);
        // the rest stays refused (`records::MEDIA_KEY`).
        if st.unpinned_buffer == Some(crate::records::MEDIA_KEY) {
            st.key = 0;
            st.adhoc = None;
            st.field_descending = 0;
        }
        // A copy of Rec of a page an action opened through `RunPageLink` holds Rec's fields and not the link's filters
        // (testpage/TestActionRunPageLinkFilterGroup.al:292-369, `filters=` in OnOpenPage and OnAfterGetCurrRecord); what else it
        // answers is not pinned (`records::RUN_PAGE_LINK_COPY`). Filters Rec's own code added stay and refuse GetFilters.
        if let Some(link) = st.run_page_link.take() {
            if crate::records::same_filters(&st.filters, &link) {
                st.filters.clear();
            }
            if st
                .unpinned_buffer
                .is_none_or(|w| w == crate::records::RUN_PAGE_LINK_OPEN_FILLED)
            {
                st.unpinned_buffer = Some(if self.pages.run_page_link_opening > 0 {
                    crate::records::RUN_PAGE_LINK_OPEN_COPY
                } else {
                    crate::records::RUN_PAGE_LINK_COPY
                });
            }
        }
        Value::Record(Rc::new(RefCell::new(st)))
    }

    /// [`Session::record_by_value`] for a copy whose SecurityFiltering the corpus does not measure: an event argument, a
    /// record handed to StartSession, a Variant's record, an array element passed by value
    /// (`RecordState::security_filtering_unmeasured_copy`).
    pub(crate) fn record_by_value_unmeasured(&mut self, rc: &Rc<RefCell<RecordState>>) -> Value {
        let v = self.record_by_value(rc);
        if let Value::Record(copy) = &v {
            copy.borrow_mut().security_filtering_unmeasured_copy(&rc.borrow());
        }
        v
    }

    /// The by-value copy of `rc` for a parameter of type `param`: a Variant parameter's copy is one whose SecurityFiltering
    /// the corpus does not measure ([`Session::record_by_value_unmeasured`]).
    fn record_by_value_into(&mut self, rc: &Rc<RefCell<RecordState>>, param: &RtType) -> Value {
        if matches!(param, RtType::Variant) {
            self.record_by_value_unmeasured(rc)
        } else {
            self.record_by_value(rc)
        }
    }

    /// An array passed by value is a copy: the callee's writes and Clear do not reach the caller
    /// (format-evaluate/W5Clear.al:Clear_ArrayPassedByValue). The SecurityFiltering of an array declared with the attribute is
    /// not measured there (SFSecurityFilteringFlow.al:429-459 reads its elements only directly).
    fn array_by_value(&mut self, a: &Rc<RefCell<Vec<Value>>>) -> Value {
        let items: Vec<Value> = a
            .borrow()
            .iter()
            .map(|v| match v {
                Value::Record(rc) => {
                    let copy = self.record_by_value_unmeasured(rc);
                    if let (Value::Record(c), true) = (&copy, rc.borrow().security_filtering_array) {
                        c.borrow_mut().security_filtering_not_measured();
                    }
                    copy
                }
                other => other.clone(),
            })
            .collect();
        Value::Array(Rc::new(RefCell::new(items)))
    }

    pub(crate) fn ensure_initialized(&mut self, inst: &Rc<RefCell<Instance>>) {
        if inst.borrow().initialized {
            return;
        }
        let object = inst.borrow().object as usize;
        let fresh = self.instance(object);
        let globals = std::mem::take(&mut fresh.borrow_mut().globals);
        let mut i = inst.borrow_mut();
        i.globals = globals;
        i.initialized = true;
    }

    /// Calls routine `routine` of `object` on `instance`. Returns the return
    /// value and the final values of all parameters (for `var` write-back).
    pub fn call(
        &mut self,
        object: usize,
        routine: usize,
        instance: Rc<RefCell<Instance>>,
        args: Vec<Value>,
    ) -> Result<(Value, Vec<Value>), Signal> {
        let facts = self.engine.routine_facts(object, routine);
        // ADR-068 item 3: a call from outside starts with no read recorded and refuses on one left at its end (an `exit` of an
        // expression that read a text with half of a surrogate pair runs no further statement).
        let outer = self.depth == 0;
        if outer {
            al_builtins::text::take_unaudited_read();
        }
        let r = self.call_with_facts(object, routine, instance, args, facts);
        if outer && r.is_ok() {
            al_builtins::text::check_unaudited()?;
        }
        r
    }

    /// [`Session::call`] with the routine's facts already looked up by the caller (one lookup per call, not one per step).
    pub(crate) fn call_with_facts(
        &mut self,
        object: usize,
        routine: usize,
        instance: Rc<RefCell<Instance>>,
        args: Vec<Value>,
        facts: RoutineFacts,
    ) -> Result<(Value, Vec<Value>), Signal> {
        // ADR-062: a call is an Execute access; an unrestricted session pays one branch.
        if self.permissions.is_some() {
            self.permission_gate_call(object, routine, &instance)?;
        }
        if let Some(Some(reason)) = self.engine.blocked.get(object).and_then(|o| o.get(routine)) {
            return Err(Signal::unsupported(reason.clone()));
        }
        // A report procedure that names the request page's `Rec`, called while OnInitReport runs: in OnInitReport itself a read
        // of Rec raises ReportRequestPageNotInitialized (reports/request-page/YRequestPageTests.al:551); through a procedure
        // it is not pinned.
        if self.report.report_init_depth.is_some()
            && self.engine.program.objects[object]
                .report
                .as_ref()
                .is_some_and(|r| r.request_rec_routines.contains(&routine))
        {
            return Err(Signal::Unsupported(
                "a report procedure that uses the request page's Rec, called while OnInitReport runs (not pinned)"
                    .into(),
            ));
        }
        if self.depth >= MAX_CALL_DEPTH {
            // With an event subscriber on the stack BC names event recursion: code `RecursiveEventCall:StackOverflow`, and a text that
            // names the innermost subscriber, the event and its publisher (codeunits/call-depth/CallDepthDirectAndEvent.al:
            // 101-137) for an integration event of a codeunit and OnAfterModifyEvent of a table; another event's text is not pinned.
            if let Some(top) = self.events.subscriber_stack.last() {
                let objects = &self.engine.program.objects;
                let text = top.map(|(publisher, so, sr)| {
                    let p = &objects[publisher];
                    let (event, kind) = if p.kind == al_ir::ObjKind::Table {
                        ("OnAfterModifyEvent", "Table")
                    } else {
                        ("None", "CodeUnit")
                    };
                    format!(
                        "Excessive recursion has been detected in event subscriber function {} for event {event} that is raised in {kind} {}.",
                        objects[so].routines[sr].name,
                        p.id.unwrap_or(0)
                    )
                });
                return Err(Signal::Error(Box::new(match text {
                    Some(message) => al_builtins::native::AlError {
                        message,
                        code: "RecursiveEventCall:StackOverflow".into(),
                        ..Default::default()
                    },
                    None => al_builtins::native::unpinned_text_error(
                        "RecursiveEventCall:StackOverflow",
                        "event-recursion-text-of-another-event",
                    ),
                })));
            }
            return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                message: "There is insufficient memory to execute this function. This can be caused by recursive function calls. Contact your system administrator.".into(),
                code: "StackOverflow".into(),
                call_stack: String::new(), ..Default::default() })));
        }
        self.depth += 1;
        self.calls_made += 1;
        // The builtin being run (a record write that runs triggers and events) keeps its own
        // `ret_used`: the callee's builtins must not change it.
        let ret_used = self.ret_used;
        let r = if facts.publisher {
            // [CommitBehavior] on the event declaration governs the subscribers the raise runs: Ignore drops their Commit, Error
            // raises the commit error (errors/CuCommitTests.al SP_CommitBehaviorIgnore_OnIntegrationEvent_*,
            // SP_CommitBehaviorError_OnIntegrationEvent_*, SP_CommitBehavior{Ignore,Error}_OnBusinessEvent_*). Any other
            // attribute text on a publisher is refused.
            if facts.odd_attributes {
                self.depth -= 1;
                return Err(Signal::Unsupported(
                    "an event publisher with a CommitBehavior or ErrorBehavior attribute other than one CommitBehavior Ignore or Error"
                        .into(),
                ));
            }
            let saved = self.tx.commit_behavior;
            if let Some(b) = facts.commit_behavior {
                self.tx.commit_behavior = self.tx.commit_behavior.nested(b);
            }
            let r = self.publish(object, routine, instance, args);
            self.tx.commit_behavior = saved;
            r
        } else {
            self.call_inner(object, routine, instance, args, facts)
        };
        self.ret_used = ret_used;
        self.depth -= 1;
        // ADR-040: a look at the output of a report that was never produced, made where the builtin could not refuse it
        // (a parser or a hash reading a stream, `Format` of a Blob), ends the run when the outermost call returns:
        // the result, or the error the empty bytes caused, is replaced by the refusal. One branch per outermost call.
        if self.depth == 0 {
            if let Some(refusal) = al_builtins::stream::take_pending_observation() {
                if !matches!(r, Err(Signal::Unsupported(_))) {
                    self.fault_site_lost = true;
                    return Err(refusal);
                }
            }
        }
        r
    }

    fn call_inner(
        &mut self,
        object: usize,
        routine: usize,
        instance: Rc<RefCell<Instance>>,
        args: Vec<Value>,
        facts: RoutineFacts,
    ) -> Result<(Value, Vec<Value>), Signal> {
        let aliases = std::mem::take(&mut self.events.next_aliases);
        self.events.next_is_subscriber = false;
        self.note_routine(object, routine);
        if self.coverage.cells.is_some() {
            self.tap_cells(object, routine, &instance);
        }
        self.first_use(&instance)?;
        self.note_caller_writes(&instance, object, routine)?;
        let engine = self.engine.clone();
        let r = &engine.program.objects[object].routines[routine];
        // The bytecode, if the mode asks for it. Whether a capture watches the routine is only known once its frame is
        // entered (`cap_enter_frame`); a capture watches the captured form of the bytecode.
        let chunk = engine.exec.chunk(&engine.program, object, routine);
        let mut locals = self.spare_vec(r.locals.len() + chunk.map_or(0, |c| usize::from(c.temps)));
        let passed = args.len().min(r.params);
        for (i, s) in r.locals.iter().enumerate() {
            // The default of a parameter that is passed in is never seen: the argument replaces it. One with state of
            // its own (a record takes the number of a temporary table) is still made, as the values are made in order.
            if i < passed && (plain_default(&s.ty) || matches!(&s.ty, RtType::Array(_, el) if plain_default(el))) {
                locals.push(Value::Null);
            } else if i < passed && self.record_without_state(&s.ty) {
                // A record of a stored table takes no temporary table: its fresh buffer would only be dropped (issue 6133).
                locals.push(Value::Null);
            } else {
                locals.push(self.slot_default(s));
            }
        }
        let mut args = args;
        for (i, a) in args.drain(..).enumerate().take(r.params) {
            // No arm below is for these pairs, and the last one hands them back unchanged.
            if passed_unchanged(&a, &r.locals[i].ty) {
                locals[i] = a;
                continue;
            }
            locals[i] = match (a, r.by_var.get(i).copied().unwrap_or(false)) {
                // A temporary record handed to a Variant parameter keeps its table: the callee counts the
                // caller's rows through the Variant, though a by-value copy is an empty buffer
                // (recordref-fieldref/recordref/PWRecordMisc.al:TempRecord_PassedToVariantParameter_ByValue, record/TestTempTableContracts.al:141-148).
                // Assigning the record to a Variant variable is a by-value copy
                // (:TempRecord_AssignedToVariantVariable).
                (Value::Record(rc), false) if matches!(r.locals[i].ty, RtType::Variant) && rc.borrow().temporary => {
                    let st = rc.borrow().copy_as_unmeasured_variable();
                    Value::Record(Rc::new(RefCell::new(st)))
                }
                // A temporary record handed by value to a parameter that is NOT declared temporary gives the callee a record of the
                // stored table, not a temporary one: IsTemporary is No, Count sees the stored rows, Insert, Modify and DeleteAll reach the
                // stored table and the caller's temporary rows do not change
                // (record/temporary/TempRecordByValue.al:TempRecord_ByValue_ToPersistentParameter, :TempRecord_ByValue_ModifyAndDeleteAll).
                // The copy keeps the record's current values and filters like any by-value copy. A parameter declared `temporary` gets
                // an independent empty buffer (record/TestTempTableContracts.al:141-148). Base Application's To-do.InsertTask relies on it.
                (Value::Record(rc), false)
                    if rc.borrow().temporary
                        && matches!(r.locals[i].ty, RtType::Record(t, false) if !self.engine.object_facts[t as usize].temporary()) =>
                {
                    let mut st = rc.borrow().copy_as_new_variable();
                    st.temporary = false;
                    st.temp_id = None;
                    Value::Record(Rc::new(RefCell::new(st)))
                }
                // A record of the stored table handed by value to a parameter declared `temporary` gives the callee a temporary
                // record with the record's values over an empty table of its own (events/ClEventLeads.al:70
                // `ins.temporary temp=Yes count=0 id=103`, an OnAfterInsertEvent subscriber; record/temporary/
                // QdStoredToTempParam.al:90-119 for a plain procedure: `callee.count = 0`, `callee.current = 2=two`), without the
                // caller's filters (`callee.filters` empty; the caller keeps `Id: 2..3`). It has the primary key, ascending, no marks
                // and MarkedOnly No, whatever the caller's key, sort order and marks (QdStoredToTempParam.al:149-240
                // QF_StoredRecord_KeyAndOrder_ToTemporaryParameter `callee.view = VERSION(1) SORTING(Field1)`, `callee.order = 1,2,3,`;
                // QF_StoredRecord_Marks_ToTemporaryParameter and _MarksNotMarkedOnly_: `callee.markedOnly = No`, `callee.marks =
                // 1=No,2=No,3=No,`; corpus bump 37).
                // A temporary record handed by value to a parameter declared `temporary`, or to one of a TableType = Temporary
                // table, is the same: an empty table of its own, the current values, no filters in any filter group, the primary
                // key ascending, no marks (QdStoredToTempParam.al:283-317 RN_TempRecord_ToTemporaryParameter, :319-344
                // _FilterGroups_, :346-381 _KeyAndOrder_, :383-416 _Marks_; :427-469 RN_TableTypeTemporary_ByValueParameter for
                // the parameter with and without `temporary`; al-runner issue 5768).
                (Value::Record(rc), false)
                    if matches!(r.locals[i].ty, RtType::Record(_, true))
                        || (rc.borrow().temporary
                            && matches!(r.locals[i].ty, RtType::Record(t, false) if self.engine.object_facts[t as usize].temporary())) =>
                {
                    self.temporary_param_copy(&rc)
                }
                // Rec of a page an action opened through RunPageLink handed to a Variant parameter keeps the link filter: a
                // RecordRef.GetTable of the Variant carries it (testpage/R7138RunPageLinkCopyGet.al:117-134 `rr[Project Code: P2]
                // n=2`); see `records::RUN_PAGE_LINK_VARIANT`.
                (Value::Record(rc), false)
                    if matches!(r.locals[i].ty, RtType::Variant) && rc.borrow().run_page_link.is_some() =>
                {
                    let mut st = rc.borrow().copy_as_unmeasured_variable();
                    st.run_page_link = None;
                    st.unpinned_buffer = Some(crate::records::RUN_PAGE_LINK_VARIANT);
                    Value::Record(Rc::new(RefCell::new(st)))
                }
                // The copy keeps what Record.IsDirty compares with (record/Y8RecordIsDirtyShapes.al:321-324 `by_value_dirty` Yes,
                // `by_value_clean` No).
                (Value::Record(rc), false) => {
                    let v = self.record_by_value_into(&rc, &r.locals[i].ty);
                    if let Value::Record(copy) = &v {
                        copy.borrow_mut().is_dirty_base = rc.borrow().is_dirty_base.copy();
                    }
                    v
                }
                // A RecordRef handed by value to a RecordRef or Variant parameter: a RecordRef of its own on the same buffer, so the
                // callee's Close leaves the caller's open (`recordref_variant.rs`).
                (Value::Library(al_builtins::library_kind::LibraryKind::RecordRef, h), false)
                    if matches!(
                        r.locals[i].ty,
                        RtType::Variant | RtType::Library(al_builtins::library_kind::LibraryKind::RecordRef)
                    ) =>
                {
                    crate::recordref_variant::own_copy(&h)
                }
                // A Variant holding a RecordRef handed by value to a Record parameter: see `record_ref_into_record_param`.
                (Value::Library(name, h), false)
                    if name.eq_ignore_ascii_case("RecordRef") && matches!(r.locals[i].ty, RtType::Record(..)) =>
                {
                    let RtType::Record(table, _) = &r.locals[i].ty else {
                        unreachable!()
                    };
                    self.record_ref_into_record_param(&h, *table)?
                }
                // An array passed by value is a copy (`array_by_value`).
                (Value::Array(a), false) => self.array_by_value(&a),
                // A Notification passed by value is the caller's own notification: the callee's SetData and Message reach the caller
                // (events/binding-senders/NotificationByValue.al:ByValue_CalleeSetsDataAndMessage, ByValue_CalleeSetsData_CallerHadData,
                // ByValue_CalleeReads; an event argument: Event_ByValueArgument_SubscriberChanges). An assignment copies
                // (:Assignment_Copies, errors/errorinfo/EnTNotification.al:Copy_Semantics). Microsoft's page "Check Credit Limit"
                // relies on it (CheckCreditLimit.Page.al:543).
                (a @ Value::Library(al_builtins::library_kind::LibraryKind::Notification, _), _) => a,
                // An HttpContent passed by reference is the caller's own object; only a by-value one is a copy (see `ops::convert`).
                (a @ Value::Library(..), true) if matches!(&a, Value::Library(n, _) if n == "HttpContent") => a,
                // An HttpRequestMessage passed by reference is not handed on (`ops::convert`, `http::request_handed_on`).
                (a @ Value::Library(..), true) if matches!(&a, Value::Library(n, _) if n == "HttpRequestMessage") => a,
                // A BigText passed by reference too; a by-value one is a copy (`ops::convert`, al-runner issue 5940).
                (a @ Value::Library(al_builtins::library_kind::LibraryKind::BigText, _), true) => a,
                // A Variant holding an Integer handed to a `var` Text parameter is passed as it is: no conversion, the callee's
                // assignment replaces it and the Variant then holds that text (types/variant/
                // VariantNumberToVarParam.al:VarTextParam_VariantArgument, `integer ok = Yes`, `variant after = filled`). Another
                // number is not pinned and keeps the conversion.
                (a @ Value::Integer(_), true) if matches!(r.locals[i].ty, RtType::Text(_)) => a,
                // A Variant holding a number, a Boolean, a Date, an Enum or an Option handed to a by-value Text or Code parameter
                // raises TypeConversion (types/variant/VariantNumberToTextParam.al:TextParam_ByValue_VariantNumbers,
                // CodeParam_ByValue_VariantNumbers, TextParam_ByValue_VariantTextAndBoolean; reports/request-page/
                // P147DateToText.al:P147_DateVariant_DefaultLanguage for a Date; QO_TimeVariant_EachConversion and
                // QO_DateTimeVariant_EachConversion for a Time and a DateTime).
                (
                    Value::Integer(_)
                    | Value::BigInteger(_)
                    | Value::Decimal(_)
                    | Value::Boolean(_)
                    | Value::Date(..)
                    | Value::Time(_)
                    | Value::DateTime(_)
                    | Value::Enum(..)
                    | Value::Option(..),
                    false,
                ) if matches!(r.locals[i].ty, RtType::Text(_) | RtType::Code(_)) => {
                    return Err(self.variant_to_text_error(matches!(r.locals[i].ty, RtType::Code(_))));
                }
                // An event argument the platform leaves unknown (RequestPageXml of SubstituteReport for an Html SaveAs, reports.rs)
                // is passed on by value as it is: a routine that never reads it runs, a read is refused where it happens.
                (a @ Value::Unknown(_), false) => a,
                (a, _) => ops::convert_plain(a, &r.locals[i].ty)?,
            };
        }
        self.return_vec(args);
        let var_lens = if facts.var_lens {
            std::mem::take(&mut self.events.next_var_lens)
        } else {
            Vec::new()
        };
        let mut frame = Frame {
            object,
            routine,
            instance,
            locals,
            aliases,
            var_lens,
        };
        let is_try = facts.is_try;
        // [CommitBehavior] governs the whole dynamic extent of the procedure and ends when it
        // returns (errors/TestCommitBehaviorAttribute.al:CommitBehavior_Ignore_ScopeIsPoppedOnReturn).
        let saved_commit_behavior = self.tx.commit_behavior;
        let collecting = if facts.odd_attributes {
            // An attribute the runtime does not understand: the scan that reports it.
            for a in r
                .attributes
                .iter()
                .filter(|a| a.name.eq_ignore_ascii_case("CommitBehavior"))
            {
                match transactions::CommitBehavior::from_attribute(&a.text) {
                    Some(b) => self.tx.commit_behavior = self.tx.commit_behavior.nested(b),
                    None => {
                        return Err(Signal::unsupported(format!(
                            "{} (only Ignore and Error are pinned)",
                            a.text
                        )));
                    }
                }
            }
            errorcollect::declares_collect(&r.attributes)?
        } else {
            if let Some(b) = facts.commit_behavior {
                self.tx.commit_behavior = self.tx.commit_behavior.nested(b);
            }
            facts.collects
        };
        if collecting {
            self.error_scopes.open();
        }
        self.tx.try_depth += usize::from(is_try);
        // A [TryFunction] takes a platform scope of its own on top of its frame (session/
        // TestProbeSessionMiscContracts.al:CallDepth_1000 stops at 988 frames below test, TryDepth and TryDepthF, while
        // codeunits/TestCodeunitRecursionDepth.al:RecursionDepth_JustInsideTheCeiling_Completes runs 991 frames under a test).
        self.depth += usize::from(is_try);
        let test_scope = if self.coverage.code_coverage {
            self.coverage_enter(object, routine)
        } else {
            None
        };
        self.frames.push(object);
        self.call_stack.push(CallFrame {
            event: false,
            routine,
            pos: ir::NO_POS,
        });
        self.profile_enter(object, routine, false);
        if self.capture.is_some() {
            self.cap_enter_frame(object, r, &frame.locals);
        }
        // A capture watches a tracked routine through its captured form (ADR-030).
        // A debugger runs every routine in its debug form (ADR-048).
        let chunk = match self.form {
            Form::Plain => chunk,
            Form::Captured => engine.exec.captured_chunk(&engine.program, object, routine),
            Form::Debug => engine.exec.debug_chunk(&engine.program, object, routine),
        };
        let mut outcome = if let Some(chunk) = chunk {
            if exec::counting() {
                exec::STATS
                    .bytecode_calls
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
            frame
                .locals
                .resize_with(r.locals.len() + usize::from(chunk.temps), || Value::Null);
            self.run_chunk(chunk, &mut frame)
        } else {
            // The bytecode names a register with 16 bits; a routine that needs more is refused, never run in part.
            Err(Signal::unsupported(format!(
                "routine `{}` needs more than 65,535 registers (locals and temporaries); the executable form cannot name them",
                r.name
            )))
        };
        if self.capture.is_some() {
            self.cap_leave_frame(&outcome);
        }
        // A refusal is noted with tracing off too: it ends the test, and `run.rs` reads where (al-runner issue 5961).
        if matches!(outcome, Err(Signal::Unsupported(_)))
            || self.fault_trace && matches!(outcome, Err(Signal::Error(_)))
        {
            self.note_fault();
        }
        // An error that leaves the routine takes the routine's frame into its call stack.
        if let (Err(Signal::Error(e)), Some(&top)) = (&mut outcome, self.call_stack.last()) {
            if e.call_stack.is_empty() {
                self.error_raised_in = Some(object);
            }
            self.add_frame(e, object, top);
        }
        self.profile_leave();
        self.call_stack.pop();
        if let Some(scope) = test_scope {
            self.coverage_leave(scope);
        }
        self.frames.pop();
        if collecting {
            outcome = self.close_error_scope(outcome);
        }
        self.tx.try_depth -= usize::from(is_try);
        self.depth -= usize::from(is_try);
        self.tx.commit_behavior = saved_commit_behavior;
        self.end_trap_scope(&frame, r.params, &r.by_var);
        if let Some(why) = self.end_unpinned_page_scope(&frame, r.params, &r.by_var) {
            self.fault_site_lost = true;
            outcome = Err(Signal::unsupported(why));
        }
        outcome = self.close_test_pages_at_test_end(&frame, r, outcome);
        let ret = match outcome {
            Ok(()) => None,
            // A report flow call (Skip, Break, Quit) leaves every procedure between the report trigger and the call.
            Err(Signal::Exit(_)) if self.report.flow_unwind.is_some_and(|d| self.depth > d) => {
                return Err(Signal::Exit(None));
            }
            Err(Signal::Exit(v)) => v,
            Err(Signal::Break | Signal::Continue) => None,
            Err(e) => {
                // Variables set before the error stay set for the caller that catches it
                // (errors/TestTryFunctionContracts.al, codeunits/TestScopeIsolationContracts.al).
                if matches!(e, Signal::Error(_)) {
                    self.error_finals = Some(frame.locals[..r.params].to_vec());
                }
                return Err(e);
            }
        };
        self.exited_with_value = ret.is_some();
        let value = match (&r.ret, ret, r.ret_local) {
            (Some(t), Some(v), _) => ops::convert_plain(v, t)?,
            (Some(_), None, Some(l)) => frame.locals[l].clone(),
            // A Record left by a bare `exit;` or by the end of the body is a fresh record of its table, with the InitValue
            // values (codeunits/procedures/P69ProcResultField.al:424-480 RN_BareExit_ReturnsRecord, RnNoExit; al-runner
            // issue 5757).
            (Some(t @ RtType::Record(..)), None, None) => self.default_value(t),
            (Some(t), None, None) => Value::default_for(t),
            (None, _, _) => Value::Null,
        };
        frame.locals.truncate(r.params);
        Ok((value, frame.locals))
    }

    /// The TypeConversion a Variant raises when it holds something that is no text and meets a Text or Code. The text follows
    /// the session language: English, and German for Text (reports/request-page/P147DateToText.al:
    /// P147_DateVariant_German, `Der Ausdruck ... kann nicht in einen ...NavText-Wert typkonvertiert werden.`); another language and the
    /// German text for Code are not pinned and are refused.
    pub(crate) fn variant_to_text_error(&self, code: bool) -> Signal {
        let kind = if code { "NavCode" } else { "NavText" };
        match (self.global_language, code) {
            (1033, _) => crate::transactions::al_error_code(
                &format!(
                    "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.{kind} value."
                ),
                "TypeConversion",
            ),
            (1031, false) => crate::transactions::al_error_code(
                "Der Ausdruck Microsoft.Dynamics.Nav.Runtime.NavIndirectValue kann nicht in einen Microsoft.Dynamics.Nav.Runtime.NavText-Wert typkonvertiert werden.",
                "TypeConversion",
            ),
            (lang, _) => Signal::unsupported(format!(
                "the TypeConversion text of a Variant into {kind} in language {lang} (not pinned)"
            )),
        }
    }

    /// The flag that stops this session's AL code (see `stop`). Setting it from another thread
    /// ends the running test at its next loop iteration; `clear_stop` rearms it.
    pub fn stop_flag(&self) -> Arc<std::sync::atomic::AtomicBool> {
        self.stop.clone()
    }

    /// Starts counting how often each statement runs (`al-runner test --coverage`).
    pub fn enable_coverage(&mut self) {
        self.coverage.coverage_counts = Some((Vec::new(), Vec::new()));
    }

    /// The statements that ran since the last call, as (position id, runs); counting starts again.
    pub fn take_coverage(&mut self) -> Vec<(u32, u32)> {
        let Some((counts, touched)) = &mut self.coverage.coverage_counts else {
            return Vec::new();
        };
        let mut out: Vec<(u32, u32)> = touched
            .drain(..)
            .map(|id| (id, std::mem::take(&mut counts[id as usize])))
            .collect();
        out.sort_unstable();
        out
    }

    /// Starts the count of the time `Sleep` asks for in one test (al-runner issue 5605): `Sleep` does not wait, but when
    /// the sum of what it asked for in the test goes past `limit`, the test is stopped as the CPU limit stops it. `None`:
    /// no limit; the count still runs, for [`Session::slept_in_test_ms`].
    pub fn start_sleep_budget(&mut self, limit: Option<std::time::Duration>) {
        self.sleep_budget = limit.map(|l| i64::try_from(l.as_millis()).unwrap_or(i64::MAX));
        self.slept_in_test = 0;
    }

    /// The time `Sleep` has asked for since [`Session::start_sleep_budget`], in milliseconds.
    pub fn slept_in_test_ms(&self) -> i64 {
        self.slept_in_test
    }

    /// Whether the `Sleep` calls of the test went past its limit (the test was stopped for it).
    pub fn sleep_budget_passed(&self) -> bool {
        self.sleep_budget.is_some_and(|l| self.slept_in_test > l)
    }

    /// Ends the count of [`Session::start_sleep_budget`]: `Sleep` outside a test counts toward no limit.
    pub fn end_sleep_budget(&mut self) {
        self.sleep_budget = None;
    }

    /// Rearms the stop flag for the next test.
    pub fn clear_stop(&self) {
        self.stop.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    #[inline]
    fn check_stop(&self) -> Result<(), Signal> {
        if self.stop.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(Signal::Unsupported(STOPPED.into()));
        }
        Ok(())
    }
}

/// Tables with an extra ad-hoc key, by (table object, key fields).
pub(crate) type AdhocTables = std::collections::HashMap<(u32, Vec<usize>), Arc<al_data::TableDef>>;

/// Facts about a table object that never change during a run.
#[derive(Debug, Clone, Default)]
pub(crate) struct TableFacts {
    /// Some field has an `InitValue` text that did not evaluate at compile time.
    pub(crate) unresolved_init: bool,
    /// The positions of the BigInteger fields with `SQLTimestamp = true`: they read the row version.
    pub(crate) sql_timestamp: Vec<usize>,
    /// Some Code field declares an `SQLDataType` other than `Varchar`: a write checks its value.
    pub(crate) code_sql_type: bool,
    /// The table has a Blob field or an AutoIncrement field: its Inserts are not queued (`records/insert_buffer.rs`).
    pub(crate) insert_unqueued: bool,
}

/// The shared storage of an array value.
type Cells = Rc<RefCell<Vec<Value>>>;

/// The executable form a routine runs in (`Session::form`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Form {
    Plain,
    /// Value capture and loop tracking (ADR-030).
    Captured,
    /// A debugger is attached (ADR-048): one `Op::DbgHit` per statement.
    Debug,
}

/// A routine on the call stack and the position of the statement it is running.
#[derive(Clone, Copy)]
pub(crate) struct CallFrame {
    /// An event publisher's frame (`dispatch::publish`): shown as `(Event)` at the line of the
    /// routine's `end`, whatever `pos` says.
    pub(crate) event: bool,
    pub(crate) routine: usize,
    /// `Program::positions` id of the current statement; `NO_POS` before the first one.
    pub(crate) pos: u32,
}

/// A global passed by `var` to a running procedure call (`Session::var_alias_watch`).
pub(crate) struct AliasWatch {
    pub(crate) instance: Rc<RefCell<Instance>>,
    pub(crate) global: usize,
    /// `None`: the callee is of another instance, and reaching the global by its name is refused. `Some(true)`: the
    /// callee's parameter aliases it and holds the value; `Some(false)`: a call the callee made runs, and the global holds it.
    pub(crate) live: Option<bool>,
}

struct Frame {
    object: usize,
    routine: usize,
    instance: Rc<RefCell<Instance>>,
    locals: Vec<Value>,
    /// `var` parameters of an event subscriber that are the publisher's global variables, or of a procedure that got a global
    /// of its own instance by `var`: (parameter slot, instance, global slot). Kept in step with the global around every call the frame makes (`finish_call`), so a change the sender makes to
    /// the global shows in the parameter and the other way round (corpus events/binding-senders).
    aliases: Vec<(usize, Rc<RefCell<Instance>>, usize)>,
    /// Per parameter: what `MaxStrLen` of a `var` Text or Code parameter answers, the declared length of the variable or
    /// field the caller passed (set by `finish_call`; `None`: not known, and `MaxStrLen` refuses).
    var_lens: Vec<Option<i32>>,
}

fn int(v: &Value) -> Result<i64, Signal> {
    match v {
        Value::Integer(i) => Ok(i64::from(*i)),
        Value::BigInteger(i) => Ok(*i),
        Value::Byte(b) => Ok(i64::from(*b)),
        Value::Char(c) => Ok(i64::from(*c)),
        // An Option member is its ordinal where an integer is wanted (an array index, a for-loop bound).
        Value::Option(n, _) => Ok(i64::from(*n)),
        // A Decimal with a whole value is a for-loop bound (`for x := 1 to ServLine."Qty. to Ship"`); a fractional one is not pinned.
        Value::Decimal(d) if d.fract() == rust_decimal::Decimal::ZERO => rust_decimal::prelude::ToPrimitive::to_i64(d)
            .ok_or_else(|| Signal::unsupported(format!("Decimal {d} used as an integer"))),
        other => Err(Signal::unsupported(format!("{} used as an integer", other.describe()))),
    }
}

/// The kind of a value for a refusal reason: the variant name, never its contents.
pub(crate) fn value_kind(v: &Value) -> String {
    let d = v.describe();
    d.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string()
}

/// Hashes a node address: a multiply, where the default hasher reads eight bytes through SipHash.
#[derive(Default, Clone, Copy)]
pub(crate) struct PtrHash(u64);

impl std::hash::BuildHasher for PtrHash {
    type Hasher = PtrHash;
    fn build_hasher(&self) -> PtrHash {
        PtrHash(0)
    }
}

impl std::hash::Hasher for PtrHash {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = (self.0.rotate_left(8) ^ u64::from(*b)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }
    fn write_usize(&mut self, n: usize) {
        self.0 = (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// How the database stores field `f`: a Code field's `SQLDataType` (record/table-properties/XTableObjects.al:127-129).
/// `Varchar` and no property store it as text; any other value that is no Integer or BigInteger is `Other`.
fn code_storage(f: &al_ir::Field) -> al_data::CodeStorage {
    if !matches!(f.ty, RtType::Code(_)) {
        return al_data::CodeStorage::Text;
    }
    match f
        .properties
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("SQLDataType"))
        .map(|(_, v)| v.trim())
    {
        Some(v) if v.eq_ignore_ascii_case("Integer") => al_data::CodeStorage::Integer,
        Some(v) if v.eq_ignore_ascii_case("BigInteger") => al_data::CodeStorage::BigInteger,
        Some(v) if v.eq_ignore_ascii_case("Variant") => al_data::CodeStorage::Variant,
        Some(v) if v.eq_ignore_ascii_case("Varchar") => al_data::CodeStorage::Text,
        Some(_) => al_data::CodeStorage::Other,
        None => al_data::CodeStorage::Text,
    }
}
