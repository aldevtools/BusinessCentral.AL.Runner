//! Event publishers and subscribers: the registry built once per program
//! (ADR-001: it is part of the immutable `Engine`) and the pure parsing of
//! the attributes that feed it. Dispatch lives in `Session` (`dispatch.rs`).
//!
//! Resolution is by name against the workspace only. A subscriber whose
//! publisher is not in the workspace (Base App, platform codeunits) attaches
//! to nothing: the publisher cannot run here, so it cannot fire.

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
    pub include_sender: bool,
    pub isolated: bool,
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
    /// (lower-case platform codeunit, lower-case event) -> subscribers of platform events the runtime does not raise:
    /// `UI Helper Triggers`. Where BC raises such an event, the runtime refuses (`Registry::unraised_subscriber`).
    pub unraised: HashMap<(String, String), Vec<Unraised>>,
    /// Subscribers (object, routine, lower-case event) to `OnInstallAppPerCompany` and `OnInstallAppPerDatabase` of the
    /// platform codeunit `Extension Triggers` (2000000010), in program order. BC raises them when the subscriber's own app
    /// is installed (events/lt-platform-events-v/PeVFixtures.al:395-407, PeVTests.al:284;
    /// install/lt-bf-install-probes/BfInstallTests.al:148-170), so the install run calls them with the app's install
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
    /// Where it is declared, for the refusal text.
    pub subscriber: String,
    /// The objects through which a test can see that it ran: its own codeunit and the SingleInstance codeunits its codeunit
    /// or routine holds a variable of (the corpus fixtures log there, events/lt-platform-events-v/PeVFixtures.al:133,
    /// events/lt-global-triggers-statements/P114GlobalTriggerEvents.al:31).
    pub witnesses: Vec<usize>,
}

/// A subscriber that is declared and not compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Missing {
    /// `ElementName`; empty when the event has no elements.
    pub element: String,
    /// Where it is declared, for the refusal text.
    pub subscriber: String,
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
            unraised: HashMap::new(),
            install_subscribers: Vec::new(),
            by_publisher: Default::default(),
            depends: Vec::new(),
        }
    }
}

fn is_manual(o: &ir::Object) -> bool {
    ir::ObjectFacts::of(o).manual_subscriber
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
/// raises its `MakeDateFilter` when a Date filter text has a piece it cannot read (events/lt-platform-triggers/PtObserve.al:157).
pub const FILTER_HELPER_TRIGGERS: usize = usize::MAX - 3;

/// The object key under which subscribers to `OnCompanyOpen` and `OnCompanyOpenCompleted` of the platform codeunit
/// `Company Triggers` (2000000003) are registered: the install session of an app runs the app's own ones in each company
/// before the install routines run there (install/lt-bf-install-probes/BfInstallTests.al:125-140, `company open calls`;
/// ADR-046 amendment 1, A4 item 4; `al-testrun` `install.rs`).
pub const COMPANY_TRIGGERS: usize = usize::MAX - 4;

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
        for i in 0..n {
            if out[i][k] {
                for j in 0..n {
                    if out[k][j] {
                        out[i][j] = true;
                    }
                }
            }
        }
    }
    out
}

fn is_global_triggers(target: &str) -> bool {
    target.eq_ignore_ascii_case("Global Triggers") || target == "2000000002"
}

impl Registry {
    pub fn build(program: &ir::Program) -> Registry {
        let mut reg = Registry::default();
        // Events declared in a table or page extension are subscribed to on the base object
        // (corpus extensionevents/TestExtensionDeclaredEvents.al:92 PageExtEvent_SubscriberBoundToBasePage_...).
        let mut base_of: HashMap<usize, usize> = HashMap::new();
        for (oi, o) in program.objects.iter().enumerate() {
            if let Some(x) = &o.report_ext {
                base_of.insert(oi, x.base);
            }
            if let Some(t) = &o.table {
                for &e in &t.extensions {
                    base_of.insert(e, oi);
                }
            }
            if let Some(p) = &o.page {
                for &e in &p.extensions {
                    base_of.insert(e, oi);
                }
            }
        }
        let mut by_name: HashMap<(ObjKind, String), usize> = HashMap::new();
        // Objects that share a kind and a name (they sit in different namespaces).
        let mut same_name: HashMap<(ObjKind, String), Vec<usize>> = HashMap::new();
        let mut by_id: HashMap<(ObjKind, u32), usize> = HashMap::new();
        for (oi, o) in program.objects.iter().enumerate() {
            // A shell (an uncompiled object) is never found by kind and name.
            if o.uncompiled {
                continue;
            }
            by_name.entry((o.kind, o.name.to_lowercase())).or_insert(oi);
            same_name.entry((o.kind, o.name.to_lowercase())).or_default().push(oi);
            if let Some(id) = o.id {
                by_id.entry((o.kind, id)).or_insert(oi);
            }
        }
        if let Some(&gt) = by_name
            .get(&(ObjKind::Codeunit, "global triggers".to_string()))
            .or_else(|| by_id.get(&(ObjKind::Codeunit, 2_000_000_002)))
        {
            reg.global_key = gt;
        }
        for (oi, o) in program.objects.iter().enumerate() {
            for (ri, r) in o.routines.iter().enumerate() {
                for a in &r.attributes {
                    let name = a.name.to_ascii_lowercase();
                    let args = split_args(&a.text);
                    match name.as_str() {
                        "integrationevent" | "businessevent" | "internalevent" => {
                            // IntegrationEvent(IncludeSender, GlobalVarAccess[, Isolated]);
                            // BusinessEvent / InternalEvent(IncludeSender[, Isolated]).
                            let isolated = if name == "integrationevent" {
                                flag(&args, 2)
                            } else {
                                flag(&args, 1)
                            };
                            reg.publishers.insert(
                                (oi, ri),
                                Publisher {
                                    object: base_of.get(&oi).copied().unwrap_or(oi),
                                    event: r.name.to_lowercase(),
                                    include_sender: flag(&args, 0),
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
                            // `Codeunit::Microsoft.Foundation.Company."Company-Initialize"` writes the
                            // namespace; objects are found by their name.
                            let (target, qualified) = publisher_name(args[1].rsplit("::").next().unwrap_or(""));
                            let event = unquote(&args[2]).to_lowercase();
                            // A qualified name of which several objects of the program share the last
                            // part cannot be told apart here (the program keeps no namespaces): the
                            // subscriber attaches to none of them and every one refuses its event.
                            if qualified {
                                let twins = same_name.get(&(kind, target.to_lowercase())).map_or(&[][..], |v| v);
                                if twins.len() > 1 {
                                    for &t in twins {
                                        reg.missing.entry((t, event.clone())).or_default().push(Missing {
                                            element: args.get(3).map(|e| unquote(e)).unwrap_or_default(),
                                            subscriber: format!("{} \"{}\".{}", o.kind, o.name, r.name),
                                        });
                                    }
                                    continue;
                                }
                            }
                            let found = by_name.get(&(kind, target.to_lowercase())).copied().or_else(|| {
                                target
                                    .parse::<u32>()
                                    .ok()
                                    .and_then(|id| by_id.get(&(kind, id)).copied())
                            });
                            // The page client raises the caption class event under a key of its own,
                            // whether or not the codeunit is compiled.
                            if kind == ObjKind::Codeunit
                                && (target.eq_ignore_ascii_case("Caption Class") || target == "42")
                                && event == "onresolvecaptionclass"
                            {
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
                                reg.install_subscribers.push((oi, ri, event.clone()));
                            }
                            if kind == ObjKind::Codeunit
                                && (target.eq_ignore_ascii_case("Company Triggers") || target == "2000000003")
                                && (event == "oncompanyopen" || event == "oncompanyopencompleted")
                            {
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
                            if kind == ObjKind::Codeunit && target.eq_ignore_ascii_case("UI Helper Triggers") {
                                let mut witnesses = vec![oi];
                                for slot in o.globals.iter().chain(&r.locals) {
                                    if let al_builtins::value::RtType::Codeunit(c) = slot.ty {
                                        let c = c as usize;
                                        if program.objects.get(c).is_some_and(|x| {
                                            x.properties.iter().any(|(k, v)| {
                                                k.eq_ignore_ascii_case("SingleInstance")
                                                    && v.trim().eq_ignore_ascii_case("true")
                                            })
                                        }) && !witnesses.contains(&c)
                                        {
                                            witnesses.push(c);
                                        }
                                    }
                                }
                                reg.unraised
                                    .entry((target.to_ascii_lowercase(), event.clone()))
                                    .or_default()
                                    .push(Unraised {
                                        subscriber: format!("{} \"{}\".{}", o.kind, o.name, r.name),
                                        witnesses,
                                    });
                            }
                            if kind == ObjKind::Codeunit
                                && target.eq_ignore_ascii_case("Reporting Triggers")
                                && al_ir::RAISED_REPORTING_EVENTS
                                    .iter()
                                    .any(|e| e.eq_ignore_ascii_case(event.as_str()))
                            {
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
                                && target.eq_ignore_ascii_case("Filter Helper Triggers")
                                && al_ir::RAISED_FILTER_EVENTS
                                    .iter()
                                    .any(|e| e.eq_ignore_ascii_case(event.as_str()))
                            {
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
        // (corpus handlers/ProbeEventSubscriberOrder.al:StaticSubscribers_OneCodeunit_OrderAndSharedInstance,
        // Zeta, Alpha and Mid run as Alpha, Mid, Zeta; ManualSubscribers_BoundHighThenLow_OrderOfCalls,
        // the manual codeunits run in the order of their ids whatever the order of the binds).
        for list in reg.subscribers.values_mut() {
            list.sort_by_cached_key(|s| {
                (
                    s.manual,
                    s.object,
                    program.objects[s.object].routines[s.routine].name.to_lowercase(),
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
            };
            reg.missing.entry((key, u.event.clone())).or_default().push(Missing {
                element: u.element.clone(),
                subscriber: u.subscriber.clone(),
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
        let sees = |u: &Unraised| {
            match (test, test_index) {
            (Some(t), Some(ti)) => {
                u.witnesses.contains(&ti)
                    || t.globals
                        .iter()
                        .chain(t.routines.iter().flat_map(|r| r.locals.iter()))
                        .any(|s| matches!(s.ty, al_builtins::value::RtType::Codeunit(c) if u.witnesses.contains(&(c as usize))))
            }
            // Outside a test run (a unit test of the runtime) every subscriber counts.
            _ => true,
        }
        };
        let mut found: Vec<(&str, &str)> = self
            .unraised
            .iter()
            .filter(|((p, e), _)| p == publisher && event.is_none_or(|want| e == want))
            .flat_map(|((_, e), subs)| {
                subs.iter()
                    .filter(|u| sees(u))
                    .map(move |u| (e.as_str(), u.subscriber.as_str()))
            })
            .collect();
        found.sort();
        found.into_iter().next()
    }

    pub fn is_subscribed(&self, object: usize, event: &str) -> bool {
        self.subscribers
            .get(&(object, event.to_string()))
            .is_some_and(|v| !v.is_empty())
            || self.missing.contains_key(&(object, event.to_string()))
    }

    /// The first subscriber of `event` of `object` that is declared but not compiled and that an
    /// event for `element` (any element when `None`) would reach.
    pub fn missing_for(&self, object: usize, event: &str, element: Option<&str>) -> Option<&Missing> {
        self.missing
            .get(&(object, event.to_string()))?
            .iter()
            .find(|m| element.is_none_or(|e| m.element.is_empty() || m.element.eq_ignore_ascii_case(e)))
    }
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
        let (_ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src.into())],
            }],
            &Default::default(),
        );
        Registry::build(&l.program)
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
    fn a_qualified_name_two_objects_share_attaches_to_neither_and_both_refuse() {
        // Two codeunits `Pub` in different namespaces: the program keeps no namespaces, so the
        // subscriber cannot be told which it means.
        let (_ws, l) = al_sema::lower::compile(
            vec![al_sema::workspace::AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![
                    ("a.al".into(), "namespace A;\ncodeunit 50200 Pub { }".into()),
                    ("b.al".into(), "namespace B;\ncodeunit 50201 Pub { }".into()),
                    (
                        "s.al".into(),
                        "codeunit 50202 Sub { [EventSubscriber(ObjectType::Codeunit, Codeunit::A.Pub, 'OnX', '', false, false)] local procedure H() begin end; }"
                            .into(),
                    ),
                ],
            }],
            &Default::default(),
        );
        let reg = Registry::build(&l.program);
        assert!(reg.subscribers.is_empty());
        assert_eq!(reg.missing.len(), 2);
        assert!(reg.is_subscribed(0, "onx") || reg.is_subscribed(1, "onx"));
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
        let index = |name: &str| l.program.objects.iter().position(|o| o.name == name).unwrap();
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
