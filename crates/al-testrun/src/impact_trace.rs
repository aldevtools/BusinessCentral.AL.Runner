//! The impact index's side of a run (ADR-043 step 2): what each test raised and touched, as keys that stay the same
//! from one compile to the next, and the impact points of an extension in the same keys.
//!
//! A key names an object by kind and id, never by its index in a program: the index is recorded on a compile of
//! Microsoft's apps alone and matched against a compile that also holds the extension.
//!
//! - `event <kind> <id> <event>` and `event <kind> <id> <event>/<element>`: an event raised (`al_vm::impact`);
//! - `table <id>`: a record operation on the table; `write <id>`: one that wrote it;
//! - `page <id>`: a page whose instance was made; `report <id>`: a report that was used;
//! - `enum <id>`: an enum whose values were read (names, ordinals, captions, a value formatted or evaluated, an
//!   interface dispatched on a value).

use al_ir::{ObjKind, Program};
use al_vm::impact::Touches;
use std::collections::{BTreeSet, HashMap};

/// The name of an event's publisher object in a key: `<kind> <id>`. `None`: an object without an id.
pub(crate) fn publisher(program: &Program, object: usize) -> Option<String> {
    match object {
        al_vm::events::CAPTION_CLASS => Some("platform captionclass".into()),
        al_vm::events::GLOBAL_TRIGGERS => Some("codeunit 2000000002".into()),
        al_vm::events::REPORTING_TRIGGERS => Some("platform reportingtriggers".into()),
        al_vm::events::COMPANY_TRIGGERS => Some("codeunit 2000000003".into()),
        _ => {
            let o = program.objects.get(object)?;
            Some(format!("{} {}", o.kind.label(), o.id?))
        }
    }
}

/// The impact point of install code (`label`) that can write `writes`, or a table the program does not name (`any`): it
/// selects the tests that use one of those tables (ADR-043 section 1).
fn install_point(program: &Program, label: &str, writes: BTreeSet<u32>, any: bool) -> ImpactPoint {
    if any {
        return unsupported(label, "its install code writes a table the program does not name");
    }
    let tables: Vec<usize> = writes.iter().map(|&t| t as usize).collect();
    ImpactPoint {
        what: format!("{label} (install code)"),
        matcher: Matcher::AnyKey(
            tables
                .iter()
                .filter_map(|&t| object_key(program, "table", t as u32))
                .collect(),
        ),
        site: Site::Tables(tables),
        runs_code: false,
    }
}

fn object_key(program: &Program, what: &str, object: u32) -> Option<String> {
    let id = program.objects.get(object as usize)?.id?;
    Some(format!("{what} {id}"))
}

/// The keys of what a test raised and touched, sorted.
pub fn keys(program: &Program, t: &Touches) -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    for (object, event) in &t.events {
        if let Some(p) = publisher(program, *object) {
            out.insert(format!("event {p} {event}"));
        }
    }
    for (set, what) in [
        (&t.tables, "table"),
        (&t.writes, "write"),
        (&t.pages, "page"),
        (&t.reports, "report"),
        (&t.enums, "enum"),
    ] {
        out.extend(set.iter().filter_map(|&o| object_key(program, what, o)));
    }
    out.into_iter().collect()
}

/// Numbers keys in the order they are first seen: the index stores each test's keys as numbers.
#[derive(Debug, Default)]
pub struct Interner {
    pub keys: Vec<String>,
    ids: HashMap<String, u32>,
}

impl Interner {
    pub fn id(&mut self, key: String) -> u32 {
        if let Some(&i) = self.ids.get(&key) {
            return i;
        }
        let i = self.keys.len() as u32;
        self.keys.push(key.clone());
        self.ids.insert(key, i);
        i
    }

    /// The numbers of `keys`, sorted.
    pub fn ids(&mut self, keys: Vec<String>) -> Vec<u32> {
        let mut v: Vec<u32> = keys.into_iter().map(|k| self.id(k)).collect();
        v.sort_unstable();
        v
    }
}

/// How the index finds the tests an impact point can change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Matcher {
    /// A test that has this key.
    Key(String),
    /// A test that wrote any table (a `Global Triggers` subscriber to the database events: BC raises them for
    /// every table whose setup asks for them).
    AnyWrite,
    /// A test that has one of these keys (the tables install code can write).
    AnyKey(Vec<String>),
    /// The index cannot select for it: the run takes every test codeunit (the reason says what is missing).
    Unsupported(String),
}

/// What stands for an impact point in the static graph (`impact_static`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Site {
    /// An event, keyed as `al_vm::events::Registry` keys it: (publisher object, lower-case event name).
    Event(usize, String),
    /// Any use of the object: a table, page, report or enum (an index into `program.objects`).
    Object(usize),
    /// Any use of one of these tables (the tables install code can write).
    Tables(Vec<usize>),
    /// Nothing in the program stands for it; the text says why.
    None(String),
}

/// One place where Microsoft's code runs, shows or reads something the extension adds (ADR-043 section 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactPoint {
    /// What it is, for the report: `subscriber "Ext"."Proc" to codeunit 22 OnBeforeX`.
    pub what: String,
    pub matcher: Matcher,
    /// What stands for it in the static graph.
    pub site: Site,
    /// It runs extension code when it is reached (a subscriber, a trigger), so reaching it while the install
    /// baseline runs changes the data every test starts from.
    pub runs_code: bool,
}

impl ImpactPoint {
    /// Whether a test with the keys `has` reaches it.
    pub fn reached_by(&self, keys: &[String], has: impl Fn(&str) -> bool) -> bool {
        match &self.matcher {
            Matcher::Key(k) => has(k),
            Matcher::AnyWrite => keys.iter().any(|k| k.starts_with("write ")),
            Matcher::AnyKey(list) => list.iter().any(|k| has(k)),
            Matcher::Unsupported(_) => true,
        }
    }
}

fn routine_label(program: &Program, object: usize, routine: usize) -> String {
    let o = &program.objects[object];
    format!("\"{}\".{}", o.name, o.routines[routine].name)
}

use crate::impact_static::GLOBAL_WRITE_EVENTS;

/// The table, page and report events whose raises `al_vm::impact` notes.
pub(crate) const TABLE_EVENTS: [&str; 10] = [
    "onbeforeinsertevent",
    "onafterinsertevent",
    "onbeforemodifyevent",
    "onaftermodifyevent",
    "onbeforedeleteevent",
    "onafterdeleteevent",
    "onbeforerenameevent",
    "onafterrenameevent",
    "onbeforevalidateevent",
    "onaftervalidateevent",
];
pub(crate) const PAGE_EVENTS: [&str; 11] = [
    "onopenpageevent",
    "onclosepageevent",
    "onaftergetcurrrecordevent",
    "onqueryclosepageevent",
    "onnewrecordevent",
    "oninsertrecordevent",
    "onmodifyrecordevent",
    "onbeforeactionevent",
    "onafteractionevent",
    "onbeforevalidateevent",
    "onaftervalidateevent",
];

/// The impact points of the objects `is_extension` accepts (an index into `program.objects`), in program order.
pub fn impact_points(lowered: &al_sema::lower::Lowered, is_extension: impl Fn(usize) -> bool) -> Vec<ImpactPoint> {
    let program = &lowered.program;
    let registry = al_vm::events::Registry::build(program);
    let mut out = Vec::new();
    // Event subscribers, in the order of their codeunits.
    let mut subs: Vec<(usize, usize, usize, String, String)> = Vec::new();
    let install_subscriber = |o: usize, r: usize| {
        registry
            .install_subscribers
            .iter()
            .any(|(so, sr, _)| (*so, *sr) == (o, r))
    };
    for ((object, event), list) in &registry.subscribers {
        // `Company Triggers` OnCompanyOpen and OnCompanyOpenCompleted are listed once, under the key the install session
        // raises them by (`events::COMPANY_TRIGGERS`), also when a loaded app declares the codeunit.
        if program.objects.get(*object).is_some_and(|o| {
            o.kind == ObjKind::Codeunit
                && o.id == Some(2_000_000_003)
                && (event == "oncompanyopen" || event == "oncompanyopencompleted")
        }) {
            continue;
        }
        // The install events of `Extension Triggers` run the subscriber as install code (listed below, ADR-046).
        for s in list
            .iter()
            .filter(|s| is_extension(s.object) && !s.manual && !install_subscriber(s.object, s.routine))
        {
            subs.push((s.object, s.routine, *object, event.clone(), s.element.to_lowercase()));
        }
    }
    subs.sort();
    for (so, sr, object, event, element) in subs {
        let who = routine_label(program, so, sr);
        let Some(name) = publisher(program, object) else {
            continue;
        };
        // A publisher of the extension's own objects is raised only from code the extension adds, which one of
        // its other impact points reaches.
        if object < program.objects.len() && is_extension(object) {
            continue;
        }
        let what = format!("subscriber {who} to {name} {event}");
        let key = |e: &str| {
            if element.is_empty() {
                format!("event {name} {e}")
            } else {
                format!("event {name} {e}/{element}")
            }
        };
        let kind = program.objects.get(object).map(|o| o.kind);
        let routine_publisher = registry
            .publishers
            .values()
            .any(|p| p.object == object && p.event == event);
        let global_write = object == registry.global_key && GLOBAL_WRITE_EVENTS.contains(&event.as_str());
        let matcher = if object == registry.global_key {
            if global_write {
                Matcher::AnyWrite
            } else {
                Matcher::Unsupported(format!("{what}: the index does not note this Global Triggers event"))
            }
        } else if object == al_vm::events::CAPTION_CLASS
            || routine_publisher
            || (kind == Some(ObjKind::Table) && TABLE_EVENTS.contains(&event.as_str()))
            || (kind == Some(ObjKind::Page) && PAGE_EVENTS.contains(&event.as_str()))
        {
            Matcher::Key(key(&event))
        } else {
            Matcher::Unsupported(format!("{what}: the index does not note raises of this event"))
        };
        // Static reach sees an event raised by a publisher routine, by a use of its table, page or report, and the
        // Global Triggers write events by a record write. The platform raises the others where no code shows it.
        let site = if routine_publisher
            || global_write
            || matches!(kind, Some(ObjKind::Table | ObjKind::Page | ObjKind::Report))
        {
            Site::Event(object, event.clone())
        } else {
            Site::None("the platform raises this event where no code in the program shows it".into())
        };
        out.push(ImpactPoint {
            what,
            matcher,
            site,
            runs_code: true,
        });
    }
    // A subscriber to the install events of `Extension Triggers` runs at the extension's install (ADR-046 decision 1):
    // install code, like an install codeunit's triggers below.
    let mut index = None;
    for &(so, sr, _) in registry
        .install_subscribers
        .iter()
        .filter(|(so, _, _)| is_extension(*so))
    {
        let label = routine_label(program, so, sr);
        let index = index.get_or_insert_with(|| crate::runtime::written::ReachIndex::new(&registry, lowered));
        let f = crate::runtime::written::reach_facts(&registry, lowered, index, so, sr);
        out.push(install_point(program, &label, f.writes, f.writes_any));
    }
    // Extension objects: which base object they change.
    let mut base: HashMap<usize, usize> = HashMap::new();
    for (oi, o) in program.objects.iter().enumerate() {
        for e in o.table.iter().flat_map(|t| t.extensions.iter()) {
            base.insert(*e, oi);
        }
        for e in o.page.iter().flat_map(|p| p.extensions.iter()) {
            base.insert(*e, oi);
        }
        if let Some(x) = &o.report_ext {
            base.insert(oi, x.base);
        }
    }
    for (oi, o) in program.objects.iter().enumerate().filter(|(oi, _)| is_extension(*oi)) {
        let label = format!("{} \"{}\"", o.kind.label(), o.name);
        let triggers = o.routines.iter().any(|r| r.is_trigger);
        let of_base = |what: &str| -> Option<(String, String, usize)> {
            let b = *base.get(&oi)?;
            let id = program.objects[b].id?;
            Some((format!("{what} {id}"), program.objects[b].name.clone(), b))
        };
        let point = match o.kind {
            ObjKind::TableExtension | ObjKind::PageExtension | ObjKind::ReportExtension => {
                let what = match o.kind {
                    ObjKind::TableExtension => "table",
                    ObjKind::PageExtension => "page",
                    _ => "report",
                };
                match of_base(what) {
                    // A table extension's fields, keys and triggers are reached through any record operation on the
                    // table; a page or report extension through any use of its page or report.
                    Some((key, name, b)) => ImpactPoint {
                        what: format!("{label} of {what} \"{name}\""),
                        matcher: Matcher::Key(key),
                        site: Site::Object(b),
                        runs_code: triggers || o.kind != ObjKind::TableExtension,
                    },
                    None => unsupported(&label, "its base object is not in the program"),
                }
            }
            // A new value shows wherever the enum's values are read: its names, ordinals or captions, a value
            // formatted or evaluated, an interface dispatched on a value (`al_vm::impact`, the `enum` key).
            ObjKind::EnumExtension => {
                let b = o.extends_id.and_then(|id| {
                    program
                        .objects
                        .iter()
                        .position(|e| e.kind == ObjKind::Enum && e.id == Some(id))
                });
                match b {
                    Some(b) => ImpactPoint {
                        what: format!("{label} of enum \"{}\"", program.objects[b].name),
                        matcher: Matcher::Key(format!("enum {}", program.objects[b].id.unwrap_or_default())),
                        site: Site::Object(b),
                        runs_code: true,
                    },
                    None => unsupported(&label, "its base enum is not in the program"),
                }
            }
            // Install code changes the data tests start from: it selects the tests that use a table its static reach
            // can write (ADR-043 section 1).
            ObjKind::Codeunit
                if o.properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Install")) =>
            {
                let mut writes = std::collections::BTreeSet::new();
                let mut any = false;
                let index = crate::runtime::written::ReachIndex::new(&registry, lowered);
                for (ri, _) in o.routines.iter().enumerate().filter(|(_, r)| r.is_trigger) {
                    let f = crate::runtime::written::reach_facts(&registry, lowered, &index, oi, ri);
                    writes.extend(f.writes);
                    any |= f.writes_any;
                }
                install_point(program, &label, writes, any)
            }
            // New objects are reached only through one of the points above; permission sets do nothing here (the
            // runtime checks no permission); a codeunit's subscribers are listed above.
            ObjKind::Table
            | ObjKind::Page
            | ObjKind::Codeunit
            | ObjKind::Report
            | ObjKind::Query
            | ObjKind::XmlPort
            | ObjKind::Enum
            | ObjKind::Interface
            | ObjKind::PermissionSet
            | ObjKind::PermissionSetExtension => continue,
            ObjKind::Profile | ObjKind::Other => unsupported(&label, "the index does not cover this kind of object"),
        };
        out.push(point);
    }
    out
}

/// An impact point neither the index nor static reach can select for.
fn unsupported(label: &str, why: &str) -> ImpactPoint {
    ImpactPoint {
        what: label.to_string(),
        matcher: Matcher::Unsupported(format!("{label}: {why}")),
        site: Site::None(why.to_string()),
        runs_code: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interned_keys_keep_their_number() {
        let mut i = Interner::default();
        assert_eq!(i.ids(vec!["b".into(), "a".into()]), [0, 1]);
        assert_eq!(i.ids(vec!["a".into(), "c".into()]), [1, 2]);
        assert_eq!(i.keys, ["b", "a", "c"]);
    }
}
