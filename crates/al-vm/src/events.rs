//! Event publishers and subscribers: the registry built once per program
//! (ADR-001: it is part of the immutable `Engine`) and the pure parsing of
//! the attributes that feed it. Dispatch lives in `Session` (`dispatch.rs`).
//!
//! A subscriber is bound to the object the compile resolved its publisher name
//! to in the subscriber's file (`ir::Attribute::publisher`). A subscriber whose
//! publisher is not in the workspace (Base App, platform codeunits) attaches
//! to nothing: the publisher cannot run here, so it cannot fire. The platform
//! codeunits the runtime raises events of are found by the name as written.

use al_ir as ir;
use al_ir::ObjKind;
use std::collections::HashMap;

/// One `[EventSubscriber(...)]` routine bound to a publisher in the workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subscriber {
    pub object: usize,
    pub routine: usize,
    /// The subscriber codeunit declares `EventSubscriberInstance = Manual`:
    /// it only receives events while an instance is bound.
    pub manual: bool,
    /// `ElementName`, as written (a field name for field events, else empty).
    pub element: String,
}

/// An `[IntegrationEvent]` / `[BusinessEvent]` / `[InternalEvent]` routine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publisher {
    /// Object that subscribers name: the routine's own object, or the base
    /// table for an event declared in a table extension.
    pub object: usize,
    /// Lower-case event name.
    pub event: String,
    /// The attribute that declares the event.
    pub kind: EventKind,
    pub include_sender: bool,
    /// `IntegrationEvent(IncludeSender, GlobalVarAccess)`: GlobalVarAccess (`false` for the other kinds).
    pub global_var_access: bool,
    pub isolated: bool,
}

/// The attribute that declares an event publisher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Integration,
    Business,
    Internal,
}

#[derive(Debug)]
pub struct Registry {
    /// (publisher object, lower-case event name) → subscribers, in program order.
    pub subscribers: HashMap<(usize, String), Vec<Subscriber>>,
    /// (object, routine) → the event the routine publishes.
    pub publishers: HashMap<(usize, usize), Publisher>,
    /// The object key under which subscribers to the platform codeunit `Global Triggers`
    /// (2000000002) are registered: the codeunit itself when a loaded app declares it
    /// (the platform `System.app`), else [`GLOBAL_TRIGGERS`].
    pub global_key: usize,
    /// Whether any subscriber of `Global Triggers` exists.
    pub has_global_triggers: bool,
    /// Subscribers (object, routine) of `Global Triggers` in a dependency app that cannot run
    /// (they reach something refused, such as a platform table with no provider), and what they
    /// read. Raising an event one of them would receive is refused (`dispatch.rs`), except for the
    /// two setup events while the tables the subscriber reads hold no row.
    pub unavailable_global: std::collections::HashMap<(usize, usize), UnavailableGlobal>,
    /// (publisher object, lower-case event name) → subscribers BC would run that this program has
    /// no code for (`Program::uncompiled_subscribers`, and subscribers whose publisher cannot be
    /// told apart from another object of the same name). Raising such an event is refused.
    pub missing: HashMap<(usize, String), Vec<Missing>>,
    /// (publisher object, lower-case event name) of the events whose subscribers depend on a publisher name several objects have,
    /// which the subscriber's file picks (`ir::PublisherRef::Picked`): every one of those objects, for each event a picked
    /// subscriber names. Asking for the subscribers of such an event marks the assumption `subscriber-publisher-namespace-scope`
    /// ([`Registry::subscribers_at`], [`Registry::is_subscribed`]).
    scope_assumed: std::collections::HashSet<(usize, String)>,
    /// (publisher object, lower-case event name) of the events a subscriber written with a bare publisher id is bound to
    /// ([`by_id_measured`]): asking for their subscribers marks the assumption `subscriber-publisher-by-id`.
    by_id_assumed: std::collections::HashSet<(usize, String)>,
    /// (object, routine) of the `install_subscribers` written with a bare publisher id ([`Registry::is_by_id_install`]).
    by_id_install: std::collections::HashSet<(usize, usize)>,
    /// (subscriber, bare id) of each dependency app's subscriber to a platform codeunit event of [`platform_by_id_events`] written
    /// with the bare id, in program order ([`Registry::dependency_platform_by_id`]).
    dependency_platform_by_id: Vec<(String, String)>,
    /// (lower-case codeunit, lower-case event) -> subscribers of events the runtime does not raise: every event of `UI Helper
    /// Triggers`, of `Reporting Triggers` and Base Application's `ReportManagement`, and OnResolveAutoFormat and
    /// OnAfterResolveAutoFormat of `Auto Format`. Where BC raises such an event, the runtime refuses
    /// (`Registry::unraised_subscriber`, `auto_format_writer`).
    pub unraised: HashMap<(String, String), Vec<Unraised>>,
    /// Subscribers whose publisher is no object of the program, not even a shell, and is none the runtime raises events
    /// of without it: the publisher's app is not loaded (a `--no-deps` run of an app that depends on Microsoft's apps).
    /// Real BC raises its events (record/table-relation/RenameBlankKeyRelation.al:722-736 subscribes to Base
    /// Application's GlobalTriggerManagement, whose `OnAfterOnDatabaseRename` the log shows, :2104-2112); the runtime
    /// cannot. Each with the publisher's event as written, for the refusal (`Registry::absent_publisher_subscriber`).
    pub absent: Vec<(String, Unraised)>,
    /// Subscribers (object, routine, lower-case event) to `OnInstallAppPerCompany` and `OnInstallAppPerDatabase` of the
    /// platform codeunit `Extension Triggers` (2000000010), in program order. BC raises them when the subscriber's own app
    /// is installed (events/platform-triggers/PeVFixtures.al:395-407, PeVTests.al:284;
    /// record/BfInstallTests.al:148-170), so the install run calls them with the app's install
    /// codeunits (`al-testrun` `install.rs`, ADR-046 decision 1).
    pub install_subscribers: Vec<(usize, usize, String)>,
    /// Publisher object -> (object, routine) of every subscriber of any of its events: `subscribers` grouped by
    /// object, built at the first [`Registry::subscribers_of`]. The registry does not change once the engine holds it
    /// (ADR-001).
    by_publisher: std::sync::OnceLock<HashMap<usize, Vec<(usize, usize)>>>,
    /// `depends[i][j]`: app `i` of the program depends on app `j`, directly or through other apps (by app id). The
    /// subscriber order across apps reads it (`dispatch.rs`).
    pub depends: Vec<Vec<bool>>,
}

/// A subscriber of a platform event the runtime does not raise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unraised {
    /// The subscriber's object.
    pub object: usize,
    /// Where it is declared, for the refusal text.
    pub subscriber: String,
    /// The objects through which a test can see that it ran: its own codeunit and the SingleInstance codeunits its codeunit
    /// or routine holds a variable of (the corpus fixtures log there, events/platform-triggers/PeVFixtures.al:133,
    /// events/platform-triggers/P114GlobalTriggerEvents.al:31).
    pub witnesses: Vec<usize>,
    /// The subscriber routine, in `object`.
    pub routine: usize,
    /// The subscriber codeunit declares `EventSubscriberInstance = Manual`: it would receive the event only while bound.
    pub manual: bool,
}

impl Unraised {
    /// The entry for subscriber `r` of codeunit `oi`.
    fn of(heads: &[ir::ObjectHead], oi: usize, o: &ir::ObjectHead, r: &ir::EventRoutine) -> Unraised {
        Unraised {
            object: oi,
            subscriber: format!("{} \"{}\".{}", o.kind, o.name, r.name),
            witnesses: witnesses(heads, oi, r),
            routine: r.routine as usize,
            manual: is_manual(o),
        }
    }

    /// Whether test codeunit `test` (object `test_index`) can see that this subscriber ran: it is the subscriber's codeunit or
    /// holds a variable of one of its witnesses. Outside a test run (a unit test of the runtime) every subscriber counts.
    pub fn seen_by(&self, test: Option<&ir::Object>, test_index: Option<usize>) -> bool {
        match (test, test_index) {
            (Some(t), Some(ti)) => {
                self.witnesses.contains(&ti)
                    || t.globals
                        .iter()
                        .chain(t.routines.iter().flat_map(|r| r.locals.iter()))
                        .any(|s| matches!(s.ty, al_builtins::value::RtType::Codeunit(c) if self.witnesses.contains(&(c as usize))))
            }
            _ => true,
        }
    }
}

/// A subscriber BC would run that the program cannot run: one that is declared and not compiled, or one whose publisher name
/// several objects have and its file names none of them by a measured rule (`ir::PublisherRef::Unpicked`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    /// `ElementName`; empty when the event has no elements.
    pub element: String,
    /// Where it is declared, for the refusal text.
    pub subscriber: String,
    /// The subscribing object, when it was compiled (`MissingWhy::NotCompiled`: `None`).
    pub from: Option<usize>,
    pub why: MissingWhy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissingWhy {
    /// The compile left the subscriber out (`Program::uncompiled_subscribers`).
    NotCompiled,
    /// The publisher name as written: several objects have it, and the subscriber's file does not pick one.
    PublisherNotPicked(String),
    /// The publisher argument as written: neither `<Kind>::<name>` nor an object id (`ir::PublisherRef::Unread`).
    PublisherNotRead(String),
    /// The publisher written as the id (the second field) of a platform codeunit (the first field) whose events the runtime
    /// raises to the subscribers that name it ([`Registry::add_by_id_subscriber`]). The third field: the subscriber codeunit
    /// when it declares `EventSubscriberInstance = Manual` ([`Missing::bound_only`]).
    PlatformPublisherById(&'static str, String, Option<usize>),
    /// The publisher written as a bare id (the first field) in a shape the assumption `subscriber-publisher-by-id` does not cover
    /// ([`by_id_measured`]). The second field: the subscriber codeunit when it declares `EventSubscriberInstance = Manual`
    /// ([`Missing::bound_only`]).
    PublisherByIdNotMeasured(String, Option<usize>),
}

impl Missing {
    /// The manual subscriber codeunit this refusal holds for only while an instance of it is bound: a by-id subscriber
    /// (`MissingWhy::PlatformPublisherById`, `MissingWhy::PublisherByIdNotMeasured`). Unbound, it receives the event under
    /// neither reading of the id, as any manual subscriber (`dispatch.rs` `event_targets`). `None`: the refusal always holds.
    pub fn bound_only(&self) -> Option<usize> {
        match self.why {
            MissingWhy::PlatformPublisherById(_, _, manual) | MissingWhy::PublisherByIdNotMeasured(_, manual) => manual,
            _ => None,
        }
    }

    /// Whether an event for `element` (any element when `None`) reaches this subscriber.
    pub fn reaches(&self, element: Option<&str>) -> bool {
        element.is_none_or(|e| self.element.is_empty() || self.element.eq_ignore_ascii_case(e))
    }

    /// Why raising `event` is refused for this subscriber.
    pub fn refusal(&self, event: &str) -> String {
        match &self.why {
            MissingWhy::NotCompiled => format!(
                "event {event}: {} subscribes to it and was not compiled (no code the workspace reaches uses it)",
                self.subscriber
            ),
            MissingWhy::PublisherNotPicked(name) => format!(
                "event {event}: {} subscribes to {name}, a name more than one object has, and its file's namespace and `using`s \
                 do not pick one by a measured rule (al-runner issue 8171)",
                self.subscriber
            ),
            MissingWhy::PublisherNotRead(written) => format!(
                "event {event}: {} subscribes to the publisher {written}, an argument that is neither `<Kind>::<name>` nor an \
                 object id, so which object it names is not known",
                self.subscriber
            ),
            MissingWhy::PlatformPublisherById(name, written, _) => format!(
                "event {event}: {} subscribes to the platform codeunit {name} by its id {written}; the runtime raises that \
                 codeunit's events only to subscribers that name it, and whether BC treats the id the same is not pinned \
                 (al-runner issue 7644)",
                self.subscriber
            ),
            MissingWhy::PublisherByIdNotMeasured(written, _) => format!(
                "event {event}: {} subscribes to its publisher by the id {written}; that BC binds it as it binds the name is \
                 measured for other publisher kinds and events only (al-runner issue 8470)",
                self.subscriber
            ),
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Registry {
            subscribers: HashMap::new(),
            publishers: HashMap::new(),
            global_key: GLOBAL_TRIGGERS,
            has_global_triggers: false,
            unavailable_global: Default::default(),
            missing: HashMap::new(),
            scope_assumed: Default::default(),
            by_id_assumed: Default::default(),
            by_id_install: Default::default(),
            dependency_platform_by_id: Vec::new(),
            unraised: HashMap::new(),
            absent: Vec::new(),
            install_subscribers: Vec::new(),
            by_publisher: Default::default(),
            depends: Vec::new(),
        }
    }
}

fn is_manual(o: &ir::ObjectHead) -> bool {
    ir::ObjectFacts::of_properties(&o.properties).manual_subscriber
}

impl Registry {
    /// A subscriber of `UI Helper Triggers`: `CaptionClassTranslate` is raised by the runtime (`UI_HELPER_TRIGGERS`), every
    /// event of the codeunit is also listed as one the runtime does not raise everywhere (`Registry::unraised_subscriber`).
    fn add_ui_helper_subscriber(
        &mut self,
        heads: &[ir::ObjectHead],
        oi: usize,
        o: &ir::ObjectHead,
        r: &ir::EventRoutine,
        target: &str,
        event: &str,
    ) {
        if event.eq_ignore_ascii_case(al_ir::CAPTION_CLASS_TRANSLATE) {
            self.subscribers
                .entry((UI_HELPER_TRIGGERS, event.to_string()))
                .or_default()
                .push(Subscriber {
                    object: oi,
                    routine: r.routine as usize,
                    manual: is_manual(o),
                    element: String::new(),
                });
        }
        self.unraised
            .entry((target.to_ascii_lowercase(), event.to_string()))
            .or_default()
            .push(Unraised::of(heads, oi, o, r));
    }
}

/// A platform codeunit whose events the runtime raises without the codeunit object, under a key of its own. Its subscribers are
/// found by the name as written (`Registry::build`); one written with the id refuses ([`Registry::add_by_id_subscriber`]).
struct RaisedPlatformCodeunit {
    /// Its id and name (`al_ir::FILTER_HELPER_CODEUNIT` and the others).
    codeunit: (u32, &'static str),
    /// The object key the runtime raises its events under.
    key: usize,
    /// Every event of the codeunit is listed in `Registry::unraised`.
    unraised: bool,
}

const FILTER_HELPER: RaisedPlatformCodeunit = RaisedPlatformCodeunit {
    codeunit: al_ir::FILTER_HELPER_CODEUNIT,
    key: FILTER_HELPER_TRIGGERS,
    unraised: false,
};

const UI_HELPER: RaisedPlatformCodeunit = RaisedPlatformCodeunit {
    codeunit: al_ir::UI_HELPER_CODEUNIT,
    key: UI_HELPER_TRIGGERS,
    unraised: true,
};

const REPORTING: RaisedPlatformCodeunit = RaisedPlatformCodeunit {
    codeunit: al_ir::REPORTING_CODEUNIT,
    key: REPORTING_TRIGGERS,
    unraised: true,
};

impl Registry {
    /// A codeunit subscriber whose publisher is written as an id (`written`), before the publisher is resolved: the id of
    /// [`FILTER_HELPER`], [`UI_HELPER`] or [`REPORTING`] by equality, or 44 resolved to Base Application's ReportManagement
    /// (`al_ir::is_report_management`). Raising the event it names under the platform codeunit's key refuses
    /// (`MissingWhy::PlatformPublisherById`; a manual subscriber only while an instance is bound), and it is listed in
    /// `unraised` where a subscriber written with the name is. Whether the runtime stands in for the publisher.
    fn add_by_id_subscriber(
        &mut self,
        program: &ir::Program,
        (oi, o, r): (usize, &ir::ObjectHead, &ir::EventRoutine),
        a: &ir::Attribute,
        written: &str,
        event: &str,
    ) -> bool {
        let heads = program.objects.heads();
        if let Some(p) = [&FILTER_HELPER, &UI_HELPER, &REPORTING]
            .into_iter()
            .find(|p| p.codeunit.0.to_string() == written)
        {
            self.missing
                .entry((p.key, event.to_string()))
                .or_default()
                .push(Missing {
                    element: String::new(),
                    subscriber: format!("{} \"{}\".{}", o.kind, o.name, r.name),
                    from: Some(oi),
                    why: MissingWhy::PlatformPublisherById(
                        p.codeunit.1,
                        written.to_string(),
                        is_manual(o).then_some(oi),
                    ),
                });
            if p.unraised {
                self.unraised
                    .entry((p.codeunit.1.to_ascii_lowercase(), event.to_string()))
                    .or_default()
                    .push(Unraised::of(heads, oi, o, r));
            }
            return true;
        }
        let report_management = match a.publisher {
            ir::PublisherRef::Only(t) => heads.get(t as usize).is_some_and(|h| {
                h.kind == ObjKind::Codeunit
                    && program
                        .apps
                        .get(h.app as usize)
                        .is_some_and(|app| al_ir::is_report_management(h.id, &h.name, &app.id))
            }),
            _ => false,
        };
        if report_management && written == al_ir::REPORT_MANAGEMENT.0.to_string() {
            self.unraised
                .entry((al_ir::REPORT_MANAGEMENT.1.to_ascii_lowercase(), event.to_string()))
                .or_default()
                .push(Unraised::of(heads, oi, o, r));
            return true;
        }
        false
    }
}

/// The objects through which a test can see that subscriber `r` of codeunit `oi` ran: the codeunit itself and the
/// SingleInstance codeunits the routine holds a variable of (`Unraised::witnesses`).
fn witnesses(heads: &[ir::ObjectHead], oi: usize, r: &ir::EventRoutine) -> Vec<usize> {
    let mut witnesses = vec![oi];
    for &c in &r.codeunit_slots {
        let c = c as usize;
        if heads
            .get(c)
            .is_some_and(|x| ir::ObjectFacts::of_properties(&x.properties).single_instance)
            && !witnesses.contains(&c)
        {
            witnesses.push(c);
        }
    }
    witnesses
}

/// A subscriber of `Global Triggers` in a dependency app that cannot run.
#[derive(Debug, Clone, Default)]
pub struct UnavailableGlobal {
    /// Why it cannot run.
    pub why: String,
    /// The configuration tables its decision reads: the record variables of the routines it calls
    /// within two levels. While none holds a row, the setup events find nothing configured and a
    /// subscriber that cannot run would have answered "no" (see `dispatch.rs`).
    pub config: Vec<u32>,
}

fn flag(args: &[String], i: usize) -> bool {
    args.get(i).is_some_and(|a| a.eq_ignore_ascii_case("true"))
}

/// The object key under which subscribers to the Caption Class codeunit's OnResolveCaptionClass
/// are registered (the codeunit itself is not in the workspace).
pub const CAPTION_CLASS: usize = usize::MAX;

/// The object key under which subscribers to the platform codeunit `Global Triggers` are
/// registered while no loaded app declares that codeunit. Its events have no body: the runtime
/// raises them itself (`globaltriggers.rs`).
pub const GLOBAL_TRIGGERS: usize = usize::MAX - 1;

/// The object key under which subscribers to the platform codeunit `Reporting Triggers` are registered: the runtime
/// raises its `CustomDocumentMergerEx` when a report renders with a custom layout.
pub const REPORTING_TRIGGERS: usize = usize::MAX - 2;

/// The object key under which subscribers to the platform codeunit `Filter Helper Triggers` are registered: the runtime
/// raises its `MakeDateFilter` when a Date filter text has a piece it cannot read (events/platform-triggers/PtObserve.al:157).
pub const FILTER_HELPER_TRIGGERS: usize = usize::MAX - 3;

/// The object key under which subscribers to `OnCompanyOpen` and `OnCompanyOpenCompleted` of the platform codeunit
/// `Company Triggers` (2000000003) are registered: the install session of an app runs the app's own ones in each company
/// before the install routines run there (record/BfInstallTests.al:125-140, `company open calls`;
/// ADR-046 amendment 1, A4 item 4; `al-testrun` `install.rs`).
pub const COMPANY_TRIGGERS: usize = usize::MAX - 4;

/// The object key under which subscribers to `CaptionClassTranslate` of the platform codeunit `UI Helper Triggers` are
/// registered: the `CaptionClassTranslate` statement raises it (events/platform-triggers/P114GlobalTriggerEvents.al:309-330,
/// `pages/ui_caption_translate.rs`). The other events of that codeunit stay in `Registry::unraised`.
pub const UI_HELPER_TRIGGERS: usize = usize::MAX - 5;

/// `out[i][j]`: app `i` depends on app `j`, directly or through other apps of `apps` (dependencies named by app id).
fn app_dependencies(apps: &[ir::AppInfo]) -> Vec<Vec<bool>> {
    let n = apps.len();
    let mut out = vec![vec![false; n]; n];
    for (i, a) in apps.iter().enumerate() {
        for d in &a.dependencies {
            for (j, b) in apps.iter().enumerate() {
                if j != i && b.id.eq_ignore_ascii_case(d) {
                    out[i][j] = true;
                }
            }
        }
    }
    // Transitive closure (Floyd-Warshall); a program holds tens of apps.
    for k in 0..n {
        let via = out[k].clone();
        for row in out.iter_mut() {
            if row[k] {
                for (cell, &v) in row.iter_mut().zip(&via) {
                    *cell |= v;
                }
            }
        }
    }
    out
}

/// The publisher argument `written` of a subscriber when it is a bare object id (`50100`). `Codeunit::50100` does not compile
/// (AL0104, corpus PR 1466 must-not-compile 3216-3218).
fn bare_id(written: &str) -> Option<&str> {
    let w = written.trim();
    (!w.is_empty() && w.bytes().all(|b| b.is_ascii_digit())).then_some(w)
}

/// Whether a subscriber of object `o` whose publisher is written as the bare id `id` (kind `kind`, bound to `found`) is in a shape
/// the assumption `subscriber-publisher-by-id` covers: BC binds it as it binds the name form (corpus PR 1466,
/// tests/al-language/events/subscriber-by-id/SbiSubscriberByIdTests.al, local runs on every verified BC major). The measured
/// shapes, each fact by equality; every other one refuses (`MissingWhy::PublisherByIdNotMeasured`):
/// for a subscriber of the app under test only (the probe's subscribers are all in its one app, the app under test; a
/// dependency app's subscriber is not measured):
/// - the platform codeunits of [`platform_by_id_events`];
/// - a publisher object of the subscriber's own app, being a
///   codeunit's own IntegrationEvent or BusinessEvent (:23, :36, :47, :65), a report's own IntegrationEvent (:133), a
///   table's OnBeforeInsertEvent, OnAfterInsertEvent, OnAfterModifyEvent, OnAfterDeleteEvent and OnAfterValidateEvent
///   (:78), a page's OnOpenPageEvent and OnAfterValidateEvent (:109);
/// - the System Application's codeunit 45 "Auto Format" (`al_ir::is_system_application_codeunit`) OnResolveAutoFormat and
///   OnAfterResolveAutoFormat (:201).
fn by_id_measured(
    (heads, apps): (&[ir::ObjectHead], &[ir::AppInfo]),
    o: &ir::ObjectHead,
    kind: ObjKind,
    id: &str,
    event: &str,
    found: Option<usize>,
) -> bool {
    // AppInfo::dependency has two states: a dependency app or the app under test.
    let of_app_under_test = apps.get(o.app as usize).is_some_and(|a| !a.dependency);
    if platform_by_id_events(kind, id).contains(&event) {
        return of_app_under_test;
    }
    let found = found.and_then(|t| heads.get(t)).filter(|_| of_app_under_test);
    let system_application = |p: &ir::ObjectHead| {
        (apps.get(p.app as usize)).is_some_and(|a| al_ir::is_system_application_codeunit("Auto Format", &p.name, &a.id))
    };
    if (kind, id) == (ObjKind::Codeunit, "45") && found.is_some_and(|p| p.id == Some(45) && system_application(p)) {
        return ["onresolveautoformat", "onafterresolveautoformat"].contains(&event);
    }
    let Some(p) = found.filter(|p| p.app == o.app && p.kind == kind) else {
        return false;
    };
    let own_event = |attributes: &[&str]| {
        (p.events.iter()).any(|e| {
            e.name.eq_ignore_ascii_case(event)
                && (e.attributes.iter()).any(|a| attributes.iter().any(|n| a.name.eq_ignore_ascii_case(n)))
        })
    };
    match kind {
        ObjKind::Codeunit => own_event(&["IntegrationEvent", "BusinessEvent"]),
        ObjKind::Report => own_event(&["IntegrationEvent"]),
        ObjKind::Table => [
            "onbeforeinsertevent",
            "onafterinsertevent",
            "onaftermodifyevent",
            "onafterdeleteevent",
            "onaftervalidateevent",
        ]
        .contains(&event),
        ObjKind::Page => ["onopenpageevent", "onaftervalidateevent"].contains(&event),
        _ => false,
    }
}

/// The events of the platform codeunit written as the bare id `id` (kind `kind`) that corpus PR 1466 measured a bare-id
/// subscriber of the app under test to (SbiSubscriberByIdTests.al): Global Triggers 2000000002 GetDatabaseTableTriggerSetup and
/// OnDatabaseInsert (:144), Caption Class 42 OnResolveCaptionClass and OnAfterCaptionClassResolve (:186), Company Triggers
/// 2000000003 OnCompanyOpen and OnCompanyOpenCompleted (:265), Extension Triggers 2000000010 OnInstallAppPerDatabase and
/// OnInstallAppPerCompany (:276).
fn platform_by_id_events(kind: ObjKind, id: &str) -> &'static [&'static str] {
    match (kind, id) {
        (ObjKind::Codeunit, "2000000002") => &["getdatabasetabletriggersetup", "ondatabaseinsert"],
        (ObjKind::Codeunit, "42") => &["onresolvecaptionclass", "onaftercaptionclassresolve"],
        (ObjKind::Codeunit, "2000000003") => &["oncompanyopen", "oncompanyopencompleted"],
        (ObjKind::Codeunit, "2000000010") => &["oninstallappperdatabase", "oninstallapppercompany"],
        _ => &[],
    }
}

/// The `Registry::unraised` key of platform codeunit `(id, name)` (`al_ir::UI_HELPER_CODEUNIT`, `al_ir::REPORTING_CODEUNIT`).
pub fn unraised_key((_, name): (u32, &str)) -> String {
    name.to_ascii_lowercase()
}

fn is_global_triggers(target: &str) -> bool {
    target.eq_ignore_ascii_case("Global Triggers") || target == "2000000002"
}

impl Registry {
    pub fn build(program: &ir::Program) -> Registry {
        let mut reg = Registry::default();
        // Heads only: building the registry makes no object of a program whose objects are made on first read (ADR-054).
        let heads = program.objects.heads();
        let base_of = base_objects(heads);
        let any_head = object_keys(heads);
        let present = |kind: ObjKind, target: &str| any_head.contains(&(kind, target.to_lowercase()));
        // A shell (an uncompiled object) is never a publisher.
        let compiled_codeunit = |o: &&ir::ObjectHead| !o.uncompiled && o.kind == ObjKind::Codeunit;
        if let Some(gt) = heads
            .iter()
            .position(|o| compiled_codeunit(&o) && o.name.eq_ignore_ascii_case("Global Triggers"))
            .or_else(|| {
                heads
                    .iter()
                    .position(|o| compiled_codeunit(&o) && o.id == Some(2_000_000_002))
            })
        {
            reg.global_key = gt;
        }
        for (oi, o) in heads.iter().enumerate() {
            for r in &o.events {
                let ri = r.routine as usize;
                for a in &r.attributes {
                    let name = a.name.to_ascii_lowercase();
                    let args = split_args(&a.text);
                    match name.as_str() {
                        "integrationevent" | "businessevent" | "internalevent" => {
                            // IntegrationEvent(IncludeSender, GlobalVarAccess[, Isolated]);
                            // BusinessEvent / InternalEvent(IncludeSender[, Isolated]).
                            let kind = match name.as_str() {
                                "integrationevent" => EventKind::Integration,
                                "businessevent" => EventKind::Business,
                                _ => EventKind::Internal,
                            };
                            let integration = kind == EventKind::Integration;
                            let isolated = if integration { flag(&args, 2) } else { flag(&args, 1) };
                            reg.publishers.insert(
                                (oi, ri),
                                Publisher {
                                    object: base_of.get(&oi).copied().unwrap_or(oi),
                                    event: r.name.to_lowercase(),
                                    kind,
                                    include_sender: flag(&args, 0),
                                    global_var_access: integration && flag(&args, 1),
                                    isolated,
                                },
                            );
                        }
                        "eventsubscriber" if args.len() >= 3 => {
                            let kind = match unquote(args[0].rsplit("::").next().unwrap_or(""))
                                .to_lowercase()
                                .as_str()
                            {
                                "codeunit" => ObjKind::Codeunit,
                                "table" => ObjKind::Table,
                                "page" => ObjKind::Page,
                                "report" => ObjKind::Report,
                                "query" => ObjKind::Query,
                                "xmlport" => ObjKind::XmlPort,
                                _ => continue,
                            };
                            // The platform codeunits below are found by the name as written, namespace left out.
                            let (target, _) = publisher_name(args[1].rsplit("::").next().unwrap_or(""));
                            let event = unquote(&args[2]).to_lowercase();
                            // The runtime raises (or refuses where BC raises) this event without the publisher object.
                            let mut native = kind == ObjKind::Codeunit
                                && reg.add_by_id_subscriber(program, (oi, o, r), a, &target, &event);
                            let Some(found) = reg.publisher_of(heads, (oi, kind), r, a, &event, args.get(3)) else {
                                continue;
                            };
                            let by_id = bare_id(&args[1]);
                            let measured = by_id
                                .is_some_and(|id| by_id_measured((heads, &program.apps), o, kind, id, &event, found));
                            if reg.list_dependency_platform_by_id(&program.apps, (o, &r.name), (kind, by_id), &event) {
                                continue;
                            }
                            // The page client raises the caption class event under a key of its own,
                            // whether or not the codeunit is compiled.
                            if kind == ObjKind::Codeunit
                                && (target.eq_ignore_ascii_case("Caption Class") || target == "42")
                                && al_ir::is_raised_caption_class_event(&event)
                            {
                                native = true;
                                reg.subscribers
                                    .entry((CAPTION_CLASS, event.clone()))
                                    .or_default()
                                    .push(Subscriber {
                                        object: oi,
                                        routine: ri,
                                        manual: is_manual(o),
                                        element: String::new(),
                                    });
                            }
                            if kind == ObjKind::Codeunit
                                && (target.eq_ignore_ascii_case("Extension Triggers") || target == "2000000010")
                                && (event == "oninstallapppercompany" || event == "oninstallappperdatabase")
                            {
                                native = true;
                                reg.install_subscribers.push((oi, ri, event.clone()));
                            }
                            if kind == ObjKind::Codeunit
                                && (target.eq_ignore_ascii_case("Company Triggers") || target == "2000000003")
                                && (event == "oncompanyopen" || event == "oncompanyopencompleted")
                            {
                                native = true;
                                reg.subscribers
                                    .entry((COMPANY_TRIGGERS, event.clone()))
                                    .or_default()
                                    .push(Subscriber {
                                        object: oi,
                                        routine: ri,
                                        manual: is_manual(o),
                                        element: String::new(),
                                    });
                            }
                            if kind == ObjKind::Codeunit && target.eq_ignore_ascii_case(UI_HELPER.codeunit.1) {
                                native = true;
                                reg.add_ui_helper_subscriber(heads, oi, o, r, &target, &event);
                            }
                            // The events of Reporting Triggers, and those Base Application's ReportManagement raises from its
                            // subscribers to them, are listed too: a Pdf save that writes a placeholder raises none of them and
                            // refuses while a test can see a subscriber (`report_placeholder`).
                            if kind == ObjKind::Codeunit
                                && (target.eq_ignore_ascii_case(REPORTING.codeunit.1)
                                    || target.eq_ignore_ascii_case(al_ir::REPORT_MANAGEMENT.1))
                            {
                                native = true;
                                reg.unraised
                                    .entry((target.to_ascii_lowercase(), event.clone()))
                                    .or_default()
                                    .push(Unraised::of(heads, oi, o, r));
                            }
                            // The events of the System Application's "Auto Format" codeunit that its AutoFormatTranslate subscriber
                            // raises: listed too, the runtime raises neither (`auto_format_writer`).
                            if kind == ObjKind::Codeunit
                                && (target.eq_ignore_ascii_case("Auto Format") || target == "45")
                                && (event == "onresolveautoformat" || event == "onafterresolveautoformat")
                            {
                                native = true;
                                reg.unraised
                                    .entry(("auto format".to_string(), event.clone()))
                                    .or_default()
                                    .push(Unraised::of(heads, oi, o, r));
                            }
                            // The layout selection events go to Base Application's ReportManagement only (ADR-040 amendment
                            // 2026-11-12, A7; `report_save::fact_selection`).
                            if kind == ObjKind::Codeunit
                                && target.eq_ignore_ascii_case(REPORTING.codeunit.1)
                                && raised_reporting_event(program, o, &event)
                            {
                                native = true;
                                reg.subscribers
                                    .entry((REPORTING_TRIGGERS, event.clone()))
                                    .or_default()
                                    .push(Subscriber {
                                        object: oi,
                                        routine: ri,
                                        manual: is_manual(o),
                                        element: String::new(),
                                    });
                            }
                            if kind == ObjKind::Codeunit
                                && target.eq_ignore_ascii_case(FILTER_HELPER.codeunit.1)
                                && al_ir::RAISED_FILTER_EVENTS
                                    .iter()
                                    .any(|e| e.eq_ignore_ascii_case(event.as_str()))
                            {
                                native = true;
                                reg.subscribers
                                    .entry((FILTER_HELPER_TRIGGERS, event.clone()))
                                    .or_default()
                                    .push(Subscriber {
                                        object: oi,
                                        routine: ri,
                                        manual: is_manual(o),
                                        element: String::new(),
                                    });
                            }
                            if let Some(id) = by_id {
                                let global = kind == ObjKind::Codeunit && is_global_triggers(&target);
                                if measured {
                                    reg.note_by_id((oi, ri), id, found, &event);
                                } else if let Some(key) = found.or(global.then_some(reg.global_key)) {
                                    // Bound to no publisher: raising the event it names refuses.
                                    reg.has_global_triggers |= global;
                                    let element = args.get(3).map(|e| unquote(e)).unwrap_or_default();
                                    reg.refuse_by_id((key, event), element, (oi, o, &r.name), id);
                                    continue;
                                }
                            }
                            match found {
                                Some(t) => {
                                    if kind == ObjKind::Codeunit && is_global_triggers(&target) {
                                        reg.has_global_triggers = true;
                                    }
                                    reg.subscribers.entry((t, event)).or_default().push(Subscriber {
                                        object: oi,
                                        routine: ri,
                                        manual: is_manual(o),
                                        element: args.get(3).map(|e| unquote(e)).unwrap_or_default(),
                                    })
                                }
                                None if kind == ObjKind::Codeunit && is_global_triggers(&target) => {
                                    reg.has_global_triggers = true;
                                    reg.subscribers
                                        .entry((reg.global_key, event))
                                        .or_default()
                                        .push(Subscriber {
                                            object: oi,
                                            routine: ri,
                                            manual: is_manual(o),
                                            element: String::new(),
                                        })
                                }
                                None if !native && !present(kind, &target) => reg.absent.push((
                                    format!("{kind} \"{target}\".{}", unquote(&args[2])),
                                    Unraised::of(heads, oi, o, r),
                                )),
                                None => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // The order subscribers receive an event in: the static ones first, then the manual ones;
        // by codeunit; within a codeunit by the name of the procedure, not by where it is declared
        // (corpus events/ProbeEventSubscriberOrder.al:StaticSubscribers_OneCodeunit_OrderAndSharedInstance,
        // Zeta, Alpha and Mid run as Alpha, Mid, Zeta; ManualSubscribers_BoundHighThenLow_OrderOfCalls,
        // the manual codeunits run in the order of their ids whatever the order of the binds).
        for list in reg.subscribers.values_mut() {
            list.sort_by_cached_key(|s| {
                (
                    s.manual,
                    s.object,
                    heads[s.object]
                        .events
                        .iter()
                        .find(|e| e.routine as usize == s.routine)
                        .map_or_else(String::new, |e| e.name.to_lowercase()),
                )
            });
        }
        // Subscribers the compile left out (ADR-013 decision 6).
        for u in &program.uncompiled_subscribers {
            let key = match u.platform {
                ir::PlatformPublisher::None => match u.publisher {
                    Some(p) => p as usize,
                    None => continue,
                },
                ir::PlatformPublisher::GlobalTriggers => {
                    reg.has_global_triggers = true;
                    reg.global_key
                }
                ir::PlatformPublisher::CaptionClass => CAPTION_CLASS,
                ir::PlatformPublisher::ReportingTriggers => REPORTING_TRIGGERS,
                ir::PlatformPublisher::FilterHelperTriggers => FILTER_HELPER_TRIGGERS,
                ir::PlatformPublisher::UiHelperTriggers => UI_HELPER_TRIGGERS,
            };
            reg.missing.entry((key, u.event.clone())).or_default().push(Missing {
                element: u.element.clone(),
                subscriber: u.subscriber.clone(),
                from: None,
                why: MissingWhy::NotCompiled,
            });
        }
        reg.depends = app_dependencies(&program.apps);
        reg
    }

    /// (object, routine) of every subscriber (manual or not, any element) of every event of `publisher`: what a scan of
    /// all keys of `subscribers` for that object finds, without the scan. Static reach (`al-testrun` blocked.rs) asks
    /// this for each table a reached routine has a record of.
    pub fn subscribers_of(&self, publisher: usize) -> &[(usize, usize)] {
        let index = self.by_publisher.get_or_init(|| {
            let mut index: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
            for ((p, _), list) in &self.subscribers {
                index
                    .entry(*p)
                    .or_default()
                    .extend(list.iter().map(|s| (s.object, s.routine)));
            }
            index
        });
        index.get(&publisher).map_or(&[], Vec::as_slice)
    }

    /// Whether any subscriber (manual or not) listens to `event` of `object`, or one that is
    /// declared but not compiled would.
    /// A subscriber of platform event `event` (lower case, `None`: any event) of codeunit `publisher` (lower case) that the
    /// runtime does not raise and that test codeunit `test` can see: the test codeunit is the subscriber's codeunit or holds a
    /// variable of one of its witnesses (`Unraised::witnesses`). A test that names none of them cannot tell whether the event
    /// ran, unless the subscriber changes what the raising statement does (decided without owner review, 2026-10-19:
    /// refusing every caption translation of the corpus for one logging fixture would stop corpus cases that cannot see it).
    pub fn unraised_subscriber(
        &self,
        publisher: &str,
        event: Option<&str>,
        test: Option<&ir::Object>,
        test_index: Option<usize>,
    ) -> Option<(&str, &str)> {
        self.unraised_subscriber_but(publisher, event, test, test_index, None)
    }

    /// [`Registry::unraised_subscriber`], leaving out the subscribers of object `but` (one the runtime raises the event to).
    pub fn unraised_subscriber_but(
        &self,
        publisher: &str,
        event: Option<&str>,
        test: Option<&ir::Object>,
        test_index: Option<usize>,
        but: Option<usize>,
    ) -> Option<(&str, &str)> {
        let mut found: Vec<(&str, &str)> = self
            .unraised
            .iter()
            .filter(|((p, e), _)| p == publisher && event.is_none_or(|want| e == want))
            .flat_map(|((_, e), subs)| {
                subs.iter()
                    .filter(|u| Some(u.object) != but && u.seen_by(test, test_index))
                    .map(move |u| (e.as_str(), u.subscriber.as_str()))
            })
            .collect();
        found.sort();
        found.into_iter().next()
    }

    /// Why test codeunit `test` (object `test_index`) is not run: it can see a subscriber whose publisher is not in the
    /// program ([`Registry::absent`], [`Unraised::seen_by`]), so whether BC ran that subscriber is not known here.
    pub fn absent_publisher_subscriber(&self, test: &ir::Object, test_index: usize) -> Option<String> {
        let mut seen: Vec<_> = (self.absent.iter())
            .filter(|(_, u)| u.seen_by(Some(test), Some(test_index)))
            .map(|(publisher, u)| (u.subscriber.as_str(), publisher.as_str()))
            .collect();
        seen.sort_unstable();
        let (subscriber, publisher) = seen.first()?;
        Some(format!(
            "the test codeunit can see subscriber {subscriber} of {publisher}, whose publisher is in no loaded app: the \
             runtime cannot raise its events"
        ))
    }

    /// Whether any subscriber (compiled or not) listens to an event of object `object`.
    pub fn any_subscriber_of(&self, object: usize) -> bool {
        self.subscribers.iter().any(|((o, _), v)| *o == object && !v.is_empty())
            || self.missing.keys().any(|(o, _)| *o == object)
    }

    /// The events (publisher, lower-case event name) routine `routine` of object `object` is a compiled subscriber to.
    pub fn subscriptions_of(&self, object: usize, routine: usize) -> Vec<(usize, &str)> {
        let mut found: Vec<(usize, &str)> = self
            .subscribers
            .iter()
            .filter(|(_, v)| v.iter().any(|s| s.object == object && s.routine == routine))
            .map(|((p, e), _)| (*p, e.as_str()))
            .collect();
        found.sort_unstable();
        found
    }

    pub fn is_subscribed(&self, object: usize, event: &str) -> bool {
        self.note_scope(object, event);
        self.subscribers
            .get(&(object, event.to_string()))
            .is_some_and(|v| !v.is_empty())
            || self.missing.contains_key(&(object, event.to_string()))
    }

    /// The subscribers of `event` (lower case) of `object`, in the order they receive it.
    pub fn subscribers_at(&self, object: usize, event: &str) -> Option<&Vec<Subscriber>> {
        self.note_scope(object, event);
        self.subscribers.get(&(object, event.to_string()))
    }

    /// Marks the assumption `subscriber-publisher-namespace-scope` when the subscribers of `event` of `object` depend on a
    /// publisher name the subscriber's file picked among several objects (`scope_assumed`), and `subscriber-publisher-by-id`
    /// when one of them is written with a bare publisher id (`by_id_assumed`).
    fn note_scope(&self, object: usize, event: &str) {
        if !self.scope_assumed.is_empty() && self.scope_assumed.contains(&(object, event.to_string())) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::SubscriberPublisherNamespaceScope);
        }
        if !self.by_id_assumed.is_empty() && self.by_id_assumed.contains(&(object, event.to_string())) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::SubscriberPublisherById);
        }
    }

    /// Lists subscriber routine `name` of object `(oi, o)`, written with the bare publisher id `id` in a shape
    /// [`by_id_measured`] does not cover, as missing for `(publisher key, event)` and `element`: raising the event refuses.
    fn refuse_by_id(
        &mut self,
        key: (usize, String),
        element: String,
        (oi, o, name): (usize, &ir::ObjectHead, &str),
        id: &str,
    ) {
        self.missing.entry(key).or_default().push(Missing {
            element,
            subscriber: format!("{} \"{}\".{}", o.kind, o.name, name),
            from: Some(oi),
            why: MissingWhy::PublisherByIdNotMeasured(id.to_string(), is_manual(o).then_some(oi)),
        });
    }

    /// Subscriber `(object, routine)`, written with the bare publisher id `id` in a shape [`by_id_measured`] covers and bound to
    /// `found`: every key it receives `event` under marks the assumption (`note_scope`, [`Registry::is_by_id_install`]).
    fn note_by_id(&mut self, (object, routine): (usize, usize), id: &str, found: Option<usize>, event: &str) {
        let platform = match id {
            "2000000002" => Some(self.global_key),
            "42" => Some(CAPTION_CLASS),
            "2000000003" => Some(COMPANY_TRIGGERS),
            "2000000010" => {
                self.by_id_install.insert((object, routine));
                None
            }
            _ => None,
        };
        for key in found.into_iter().chain(platform) {
            self.by_id_assumed.insert((key, event.to_string()));
        }
    }

    /// Lists subscriber routine `name` of object `o` when it is a dependency app's subscriber to an event of
    /// [`platform_by_id_events`] written with the bare id `by_id` (kind `kind`): not measured, so it is registered nowhere
    /// and every test refuses ([`Registry::dependency_platform_by_id`]). Whether it listed it.
    fn list_dependency_platform_by_id(
        &mut self,
        apps: &[ir::AppInfo],
        (o, name): (&ir::ObjectHead, &str),
        (kind, by_id): (ObjKind, Option<&str>),
        event: &str,
    ) -> bool {
        let Some(id) = by_id.filter(|&id| {
            platform_by_id_events(kind, id).contains(&event) && apps.get(o.app as usize).is_some_and(|a| a.dependency)
        }) else {
            return false;
        };
        let subscriber = format!("{} \"{}\".{}", o.kind, o.name, name);
        self.dependency_platform_by_id.push((subscriber, id.to_string()));
        true
    }

    /// Why every test of the run is not run: a dependency app subscribes to a platform codeunit event by its bare id, which corpus
    /// PR 1466 measured for a subscriber of the app under test only (SbiSubscriberByIdTests.al). Such a subscriber is
    /// registered nowhere: it would act before or outside any one test (an install or company-open subscriber, Global Triggers,
    /// Caption Class).
    pub fn dependency_platform_by_id(&self) -> Option<String> {
        let (subscriber, id) = self.dependency_platform_by_id.first()?;
        Some(format!(
            "{subscriber} of a dependency app subscribes to the platform codeunit {id} by its bare id; that BC binds it as it \
             binds the name is measured for a subscriber of the app under test only (al-runner issue 8470)"
        ))
    }

    /// Whether install subscriber `(object, routine)` (`install_subscribers`) is written with the bare id of Extension
    /// Triggers: the tables it writes mark `subscriber-publisher-by-id` (al-testrun `install::assume_by_id_install_tables`).
    pub fn is_by_id_install(&self, object: usize, routine: usize) -> bool {
        self.by_id_install.contains(&(object, routine))
    }

    /// The publisher object that subscriber attribute `a` of routine `r` of object `oi` (kind `kind` named in its first argument)
    /// is bound to, for `event`: the object the compile resolved its publisher name to in the subscriber's file (`ir::PublisherRef`,
    /// al-sema `Workspace::subscriber_publisher`), a shell never. `Some(None)`: no object of the program. `None`: the subscriber is
    /// bound to no object and `event` is refused for each object it may name: the twins of a name the file picks none of
    /// (`MissingWhy::PublisherNotPicked`), every object of `kind` for a publisher argument the compile did not read
    /// (`MissingWhy::PublisherNotRead`).
    fn publisher_of(
        &mut self,
        heads: &[ir::ObjectHead],
        (oi, kind): (usize, ObjKind),
        r: &ir::EventRoutine,
        a: &ir::Attribute,
        event: &str,
        element: Option<&String>,
    ) -> Option<Option<usize>> {
        let compiled = |t: u32| Some(t as usize).filter(|&t| heads.get(t).is_some_and(|h| !h.uncompiled));
        let written = || split_args(&a.text).get(1).cloned().unwrap_or_default();
        let (targets, why): (Vec<usize>, MissingWhy) = match &a.publisher {
            ir::PublisherRef::NotSubscriber | ir::PublisherRef::NotVisible => return Some(None),
            ir::PublisherRef::Only(t) => return Some(compiled(*t)),
            ir::PublisherRef::Picked { object, twins } => {
                for &t in twins {
                    self.scope_assumed.insert((t as usize, event.to_string()));
                }
                return Some(compiled(*object));
            }
            ir::PublisherRef::Unpicked(twins) => {
                let w = written();
                let name = w
                    .split_once("::")
                    .map_or(w.as_str(), |(_, name)| name)
                    .trim()
                    .to_string();
                (
                    twins.iter().map(|&t| t as usize).collect(),
                    MissingWhy::PublisherNotPicked(name),
                )
            }
            ir::PublisherRef::Unread => (
                (0..heads.len()).filter(|&t| heads[t].kind == kind).collect(),
                MissingWhy::PublisherNotRead(written()),
            ),
        };
        let o = &heads[oi];
        for t in targets {
            self.missing.entry((t, event.to_string())).or_default().push(Missing {
                element: element.map(|e| unquote(e)).unwrap_or_default(),
                subscriber: format!("{} \"{}\".{}", o.kind, o.name, r.name),
                from: Some(oi),
                why: why.clone(),
            });
        }
        None
    }

    /// The first subscriber of `event` of `object` that is declared but not compiled and that an
    /// event for `element` (any element when `None`) would reach.
    pub fn missing_for(&self, object: usize, event: &str, element: Option<&str>) -> Option<&Missing> {
        self.missing_at(object, event).iter().find(|m| m.reaches(element))
    }

    /// The subscribers of `event` of `object` that are declared but cannot run, in program order, for every element.
    pub fn missing_at(&self, object: usize, event: &str) -> &[Missing] {
        self.missing
            .get(&(object, event.to_string()))
            .map_or(&[], Vec::as_slice)
    }
}

/// Whether the runtime raises `event` of Reporting Triggers to subscriber object `o`: the events of
/// `al_ir::RAISED_REPORTING_EVENTS` to every subscriber, the layout selection events to Base Application's ReportManagement only
/// (ADR-040 amendment 2026-11-12, A7; `report_save::fact_selection`).
fn raised_reporting_event(program: &ir::Program, o: &ir::ObjectHead, event: &str) -> bool {
    let named = |list: &[&str]| list.iter().any(|e| e.eq_ignore_ascii_case(event));
    named(&al_ir::RAISED_REPORTING_EVENTS)
        || (named(&al_ir::REPORT_MANAGEMENT_LAYOUT_EVENTS)
            && program
                .apps
                .get(o.app as usize)
                .is_some_and(|a| al_ir::is_report_management(o.id, &o.name, &a.id)))
}

/// Splits the argument list of an attribute written as `[Name(a, 'b,c', "d")]`
/// at top-level commas. Quotes (`'…'` with `''` escapes, `"…"`) and brackets
/// protect their commas; each piece is trimmed.
pub fn split_args(attribute: &str) -> Vec<String> {
    let Some(open) = attribute.find('(') else {
        return Vec::new();
    };
    let close = attribute.rfind(')').unwrap_or(attribute.len());
    let inner = &attribute[open + 1..close.max(open + 1)];
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    for c in inner.chars() {
        match quote {
            Some(q) => {
                cur.push(c);
                // A doubled quote re-enters the quoted state on the next character.
                if c == q {
                    quote = None;
                }
            }
            None => match c {
                '\'' | '"' => {
                    quote = Some(c);
                    cur.push(c);
                }
                '(' | '[' => {
                    depth += 1;
                    cur.push(c);
                }
                ')' | ']' => {
                    depth = depth.saturating_sub(1);
                    cur.push(c);
                }
                ',' if depth == 0 => out.push(std::mem::take(&mut cur).trim().to_string()),
                _ => cur.push(c),
            },
        }
    }
    let last = cur.trim().to_string();
    if !last.is_empty() || !out.is_empty() {
        out.push(last);
    }
    out
}

/// The object name written after `Kind::`: the last part of a name that carries its namespace
/// (`Microsoft.Foundation.Company."Company-Initialize"` is `Company-Initialize`), and whether
/// there was a namespace. Dots inside quotes belong to the name.
pub(crate) fn publisher_name(written: &str) -> (String, bool) {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in written.trim().chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            '.' if !quoted => parts.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    let qualified = parts.len() > 1;
    (unquote(parts.last().map_or("", |p| p.as_str())), qualified)
}

/// The text of a name argument: `'On Foo'` → `On Foo` (`''` is an
/// apostrophe), `"Name"` → `Name`, a bare identifier as it is.
pub fn unquote(arg: &str) -> String {
    let a = arg.trim();
    if let Some(inner) = a.strip_prefix('\'').and_then(|x| x.strip_suffix('\'')) {
        return inner.replace("''", "'");
    }
    if let Some(inner) = a.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        return inner.to_string();
    }
    a.to_string()
}

/// The base object of each extension object: events declared in a table or page extension are subscribed to on the base
/// object (corpus events/extension-events/TestExtensionDeclaredEvents.al:92 PageExtEvent_SubscriberBoundToBasePage_...).
fn base_objects(heads: &[ir::ObjectHead]) -> HashMap<usize, usize> {
    let mut base_of: HashMap<usize, usize> = HashMap::new();
    for (oi, o) in heads.iter().enumerate() {
        if let Some(base) = o.report_ext_base {
            base_of.insert(oi, base);
        }
        if let Some(t) = &o.table {
            for &e in &t.extensions {
                base_of.insert(e, oi);
            }
        }
        for &e in &o.page_extensions {
            base_of.insert(e, oi);
        }
    }
    base_of
}

/// Every object of the program by kind and lower-case name or id, shells included: a publisher among them is in a loaded
/// app ([`Registry::absent`]).
fn object_keys(heads: &[ir::ObjectHead]) -> std::collections::HashSet<(ObjKind, String)> {
    let mut keys = std::collections::HashSet::new();
    for o in heads {
        keys.insert((o.kind, o.name.to_lowercase()));
        if let Some(id) = o.id {
            keys.insert((o.kind, id.to_string()));
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_at_top_level_commas_only() {
        assert_eq!(
            split_args(
                "[EventSubscriber(ObjectType::Codeunit, Codeunit::\"A, B\", 'On-Check.Value (Qty) & Amt', '', false, false)]"
            ),
            vec![
                "ObjectType::Codeunit",
                "Codeunit::\"A, B\"",
                "'On-Check.Value (Qty) & Amt'",
                "''",
                "false",
                "false"
            ]
        );
        assert_eq!(
            split_args("[IntegrationEvent(false, 'it''s, ok')]"),
            vec!["false", "'it''s, ok'"]
        );
        assert_eq!(split_args("[Test]"), Vec::<String>::new());
        assert_eq!(split_args("[Foo()]"), Vec::<String>::new());
    }
    #[test]
    fn nested_brackets_protect_their_commas_and_single_arguments_survive() {
        assert_eq!(split_args("[X(f(a, b), c)]"), vec!["f(a, b)", "c"]);
        assert_eq!(split_args("[X(f(a), b)]"), vec!["f(a)", "b"]);
        assert_eq!(split_args("[X(g[1, 2], h)]"), vec!["g[1, 2]", "h"]);
        assert_eq!(split_args("[X(a)]"), vec!["a"]);
        assert_eq!(split_args("[X(a,)]"), vec!["a", ""]);
        // A closing bracket with nothing open does not underflow.
        assert_eq!(split_args("[X(a), b)]"), vec!["a)", "b"]);
    }

    #[test]
    fn a_close_before_the_open_yields_no_arguments_instead_of_panicking() {
        assert_eq!(split_args("[Foo)("), Vec::<String>::new());
    }

    fn registry(src: &str) -> Registry {
        registry_of_app("a", src)
    }

    fn registry_of_app(app_id: &str, src: &str) -> Registry {
        let (_ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: app_id.into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src.into())],
            }],
            &Default::default(),
        );
        Registry::build(&l.program)
    }

    /// `Registry::unraised` for a subscriber written with an id: UI Helper Triggers (2000000004) and Reporting Triggers
    /// (2000000005) by the written id, Base Application's ReportManagement by the object id 44 resolves to. Not Filter Helper
    /// Triggers (2000000007), not an app's own codeunit, not a codeunit 44 ReportManagement of another app.
    #[test]
    fn by_id_subscribers_are_listed_as_unraised_only_for_the_codeunits_listed_by_name() {
        let sub = |publisher: &str, event: &str| {
            format!(
                "codeunit 50310 Sub
                 {{ [EventSubscriber(ObjectType::Codeunit, {publisher}, '{event}', '', false, false)] local procedure H() begin end; }}"
            )
        };
        let listed = |reg: &Registry| {
            let mut keys: Vec<String> = reg.unraised.keys().map(|(c, e)| format!("{c}.{e}")).collect();
            keys.sort();
            keys
        };
        let a = |src: String| listed(&registry(&src));
        assert_eq!(
            a(sub("2000000004", "CaptionClassTranslate")),
            ["ui helper triggers.captionclasstranslate"]
        );
        assert_eq!(
            a(sub("2000000005", "SubstituteReport")),
            ["reporting triggers.substitutereport"]
        );
        assert!(a(sub("2000000007", "MakeIntFilter")).is_empty());
        let own =
            "codeunit 50300 \"UI Helper Triggers\" { [IntegrationEvent(false, false)] procedure OnX() begin end; }";
        assert!(a(format!("{own}{}", sub("50300", "OnX"))).is_empty());
        let report_management = "codeunit 44 ReportManagement { [IntegrationEvent(false, false)] procedure OnAfterSubstituteReport() begin end; }";
        let src = format!("{report_management}{}", sub("44", "OnAfterSubstituteReport"));
        assert_eq!(
            listed(&registry_of_app(al_ir::BASE_APPLICATION_APP_ID, &src)),
            ["reportmanagement.onaftersubstitutereport"]
        );
        assert!(listed(&registry(&src)).is_empty());
    }

    #[test]
    fn a_subscriber_attribute_with_fewer_than_three_arguments_is_ignored() {
        let reg = registry(
            "codeunit 50200 Pub { [IntegrationEvent(false, false)] procedure OnX() begin end; }
             codeunit 50201 Sub { [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub)] local procedure H() begin end; }",
        );
        assert!(reg.subscribers.is_empty());
        assert_eq!(reg.publishers.len(), 1);
    }

    #[test]
    fn subscribers_attach_to_every_object_kind_they_name() {
        // The registry resolves each target by (kind, name); a kind it does not know is skipped.
        let reg = registry(
            "report 50300 Rep { }
             query 50301 Qry { elements { dataitem(Dummy; Integer) { } } }
             xmlport 50302 Xml { schema { textelement(Root) { } } }
             page 50303 Pg { }
             table 50304 Tbl { fields { field(1; \"No.\"; Integer) { } } keys { key(PK; \"No.\") { } } }
             codeunit 50305 Cu { }
             codeunit 50310 Sub
             {
                 [EventSubscriber(ObjectType::Report, Report::Rep, 'OnR', '', false, false)] local procedure R() begin end;
                 [EventSubscriber(ObjectType::Query, Query::Qry, 'OnQ', '', false, false)] local procedure Q() begin end;
                 [EventSubscriber(ObjectType::XmlPort, XmlPort::Xml, 'OnX', '', false, false)] local procedure X() begin end;
                 [EventSubscriber(ObjectType::Page, Page::Pg, 'OnP', '', false, false)] local procedure P() begin end;
                 [EventSubscriber(ObjectType::Table, Database::Tbl, 'OnT', '', false, false)] local procedure T() begin end;
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Cu, 'OnC', '', false, false)] local procedure C() begin end;
             }",
        );
        for (name, event) in [
            ("Rep", "onr"),
            ("Qry", "onq"),
            ("Xml", "onx"),
            ("Pg", "onp"),
            ("Tbl", "ont"),
            ("Cu", "onc"),
        ] {
            let target = reg.subscribers.keys().find(|(_, e)| e == event).unwrap_or_else(|| {
                panic!(
                    "no subscriber for {name}: {:?}",
                    reg.subscribers.keys().collect::<Vec<_>>()
                )
            });
            assert_eq!(reg.subscribers[target].len(), 1, "{name}");
        }
    }

    #[test]
    fn only_a_missing_global_triggers_codeunit_sets_the_global_triggers_flag() {
        let other = registry(
            "codeunit 50310 Sub
             { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Not There\", 'OnX', '', false, false)] local procedure H() begin end; }",
        );
        assert!(!other.has_global_triggers);
        let global = registry(
            "codeunit 50310 Sub
             { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnX', '', false, false)] local procedure H() begin end; }",
        );
        assert!(global.has_global_triggers);
        // A different object kind named like the platform codeunit is not it.
        let table = registry(
            "codeunit 50310 Sub
             { [EventSubscriber(ObjectType::Table, Database::\"Global Triggers\", 'OnX', '', false, false)] local procedure H() begin end; }",
        );
        assert!(!table.has_global_triggers);
    }

    /// `Registry::absent`: a publisher that is no object of the program, and none whose events the runtime raises or refuses
    /// without the object. A publisher in the program, also a table, is not absent.
    #[test]
    fn only_a_publisher_no_object_is_and_the_runtime_does_not_stand_in_for_is_absent() {
        let sub = |kind: &str, publisher: &str, event: &str| {
            registry(&format!(
                "table 50300 T {{ fields {{ field(1; K; Integer) {{ }} }} }}
                 codeunit 50301 Pub {{ [IntegrationEvent(false, false)] procedure OnX() begin end; }}
                 codeunit 50310 Sub
                 {{ [EventSubscriber(ObjectType::{kind}, {publisher}, '{event}', '', false, false)] local procedure H() begin end; }}"
            ))
            .absent
            .iter()
            .map(|(p, u)| format!("{} {p}", u.subscriber))
            .collect::<Vec<_>>()
        };
        assert_eq!(
            sub("Codeunit", "Codeunit::\"Not There\"", "OnX"),
            ["codeunit \"Sub\".H codeunit \"Not There\".OnX"]
        );
        for (kind, publisher, event) in [
            ("Codeunit", "Codeunit::Pub", "OnX"),
            ("Table", "Database::T", "OnAfterInsertEvent"),
            ("Codeunit", "Codeunit::\"Global Triggers\"", "OnDatabaseInsert"),
            ("Codeunit", "Codeunit::\"Company Triggers\"", "OnCompanyOpenCompleted"),
            ("Codeunit", "Codeunit::\"Reporting Triggers\"", "OnX"),
        ] {
            assert_eq!(sub(kind, publisher, event), Vec::<String>::new(), "{publisher}");
        }
    }

    #[test]
    fn a_publisher_written_with_its_namespace_is_found_by_its_name() {
        let reg = registry(
            "namespace A.B;\n\
             codeunit 50200 Pub { [IntegrationEvent(false, false)] procedure OnX() begin end; }\n\
             codeunit 50201 Sub { [EventSubscriber(ObjectType::Codeunit, Codeunit::A.B.Pub, 'OnX', '', false, false)] local procedure H() begin end; \
             [EventSubscriber(ObjectType::Codeunit, Codeunit::A.B.\"Pub\", 'OnY', '', false, false)] local procedure G() begin end; }",
        );
        let keys: Vec<_> = reg.subscribers.keys().map(|(_, e)| e.as_str()).collect();
        assert!(keys.contains(&"onx") && keys.contains(&"ony"), "{keys:?}");
        assert!(reg.missing.is_empty());
    }

    #[test]
    fn a_qualified_name_two_objects_share_attaches_to_the_one_of_that_namespace() {
        // Two codeunits `Pub` in two namespaces of two apps (one app may not declare both, AL0197): the qualified name names
        // the one of namespace A, from a file with a namespace (al-runner issue 8171; corpus PR 1423, SnUsingTests.al).
        let app = |id: &str, deps: &[&str], src: &str| al_sema::workspace::AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: vec![(format!("{id}.al"), src.into())],
        };
        let (_ws, l) = al_sema::lower::compile(
            vec![
                app("a", &[], "namespace A;\ncodeunit 50200 Pub { }"),
                app("b", &["a"], "namespace B;\ncodeunit 50201 Pub { }"),
                app(
                    "s",
                    &["a", "b"],
                    "namespace S;\ncodeunit 50202 Sub { [EventSubscriber(ObjectType::Codeunit, Codeunit::A.Pub, 'OnX', '', false, false)] local procedure H() begin end; }",
                ),
            ],
            &Default::default(),
        );
        let reg = Registry::build(&l.program);
        let a = l.program.objects.iter().position(|o| o.id == Some(50200)).unwrap();
        let b = l.program.objects.iter().position(|o| o.id == Some(50201)).unwrap();
        assert!(reg.missing.is_empty());
        assert!(reg.is_subscribed(a, "onx"));
        assert!(!reg.is_subscribed(b, "onx"));
    }

    #[test]
    fn the_caption_class_subscribers_are_found_under_their_own_key_whether_or_not_the_codeunit_is_compiled() {
        let sub = "codeunit 50310 Sub { [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', false, false)] local procedure H() begin end; }";
        let absent = registry(sub);
        assert!(absent.is_subscribed(CAPTION_CLASS, "onresolvecaptionclass"));
        let present = registry(&format!("codeunit 42 \"Caption Class\" {{ }}\n{sub}"));
        assert!(present.is_subscribed(CAPTION_CLASS, "onresolvecaptionclass"));
    }

    /// `subscribers_of` is the scan of every key of `subscribers` for one publisher object, done once per object.
    #[test]
    fn the_subscribers_of_an_object_are_those_of_every_event_it_publishes() {
        let (_ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![(
                    "f.al".into(),
                    "table 50300 T { fields { field(1; K; Code[10]) { } } }
                    codeunit 50301 Pub {
                        [IntegrationEvent(false, false)] procedure OnA() begin end;
                        [IntegrationEvent(false, false)] procedure OnB() begin end; }
                    codeunit 50302 Sub {
                        [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnA', '', false, false)]
                        local procedure A() begin end;
                        [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnB', '', false, false)]
                        local procedure B() begin end;
                        [EventSubscriber(ObjectType::Table, Database::T, 'OnAfterInsertEvent', '', false, false)]
                        local procedure C(var Rec: Record T) begin end; }
                    codeunit 50303 Manual { EventSubscriberInstance = Manual;
                        [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnA', '', false, false)]
                        local procedure D() begin end; }"
                        .into(),
                )],
            }],
            &Default::default(),
        );
        let reg = Registry::build(&l.program);
        let index = |name: &str| l.program.objects.heads().iter().position(|o| o.name == name).unwrap();
        let scanned = |publisher: usize| {
            let mut all: Vec<(usize, usize)> = (reg.subscribers.iter())
                .filter(|((p, _), _)| *p == publisher)
                .flat_map(|(_, list)| list.iter().map(|s| (s.object, s.routine)))
                .collect();
            all.sort();
            all
        };
        for (publisher, count) in [(index("Pub"), 3), (index("T"), 1)] {
            let mut got = reg.subscribers_of(publisher).to_vec();
            got.sort();
            assert_eq!(got.len(), count, "{got:?}");
            assert_eq!(got, scanned(publisher));
        }
        assert!(reg.subscribers_of(index("Sub")).is_empty());
        assert!(reg.subscribers_of(usize::MAX).is_empty());
    }

    #[test]
    fn an_uncompiled_subscriber_counts_as_a_listener_for_its_element_only() {
        let (_ws, mut l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![(
                    "f.al".into(),
                    "table 50200 T { fields { field(1; K; Code[10]) { } } }".into(),
                )],
            }],
            &Default::default(),
        );
        let uncompiled = |platform, publisher, event: &str, element: &str| al_ir::UncompiledSubscriber {
            publisher,
            platform,
            event: event.into(),
            element: element.into(),
            subscriber: "codeunit \"S\"".into(),
        };
        std::sync::Arc::make_mut(&mut l.program).uncompiled_subscribers = vec![
            uncompiled(
                al_ir::PlatformPublisher::None,
                Some(0),
                "onaftervalidateevent",
                "Status",
            ),
            uncompiled(al_ir::PlatformPublisher::GlobalTriggers, None, "ondatabasemodify", ""),
        ];
        let reg = Registry::build(&l.program);
        assert!(reg.is_subscribed(0, "onaftervalidateevent"));
        assert!(reg.missing_for(0, "onaftervalidateevent", Some("status")).is_some());
        assert!(reg.missing_for(0, "onaftervalidateevent", None).is_some());
        assert!(reg.missing_for(0, "onaftervalidateevent", Some("Other")).is_none());
        assert!(reg.missing_for(0, "onafterinsertevent", None).is_none());
        // The platform codeunits are found under the keys the runtime raises them with.
        assert!(reg.has_global_triggers);
        assert!(reg.missing_for(reg.global_key, "ondatabasemodify", None).is_some());
    }
}
