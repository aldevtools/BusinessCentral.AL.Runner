//! The head of an object (ADR-054 decision 2): what a pass over all objects of a program reads, and nothing more.
//!
//! A pass that runs over every object at start (the engine's table definitions, the event registry, the lists of the
//! metadata tables, the selection of test codeunits) reads heads ([`crate::Objects::heads`]), so it does not make the
//! objects of a list whose objects are made on first read. The field names are those of [`Object`], so a pass moves
//! from objects to heads by its loop alone. A head is computed from its object ([`ObjectHead::of`]); a test compares
//! the two.

use crate::{Attribute, ObjKind, Object, Table};
use al_builtins::value::RtType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ObjectHead {
    pub kind: ObjKind,
    pub uncompiled: bool,
    pub id: Option<u32>,
    pub name: String,
    pub app: u32,
    pub properties: Vec<(String, String)>,
    pub namespace: Option<String>,
    pub extends_id: Option<u32>,
    pub enum_values: Vec<(i32, String)>,
    /// The table of a table object, as linked (extension fields included).
    pub table: Option<Table>,
    /// The page extensions of a page (`Page::extensions`).
    pub page_extensions: Vec<usize>,
    /// The report a report extension changes the run of (`ReportExt::base`).
    pub report_ext_base: Option<usize>,
    /// How many routines the object has.
    pub routine_count: u32,
    /// The routines that publish or subscribe to an event, in routine order.
    pub events: Vec<EventRoutine>,
}

/// A routine that declares an event attribute (`IntegrationEvent`, `BusinessEvent`, `InternalEvent` or
/// `EventSubscriber`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EventRoutine {
    /// The routine's index in its object.
    pub routine: u32,
    pub name: String,
    /// Every attribute of the routine, as declared.
    pub attributes: Vec<Attribute>,
    /// The codeunit objects of the variables the routine sees (the object's globals, then the routine's locals), in
    /// that order.
    pub codeunit_slots: Vec<u32>,
}

/// Whether an attribute named `name` declares an event or a subscriber.
pub fn is_event_attribute(name: &str) -> bool {
    ["integrationevent", "businessevent", "internalevent", "eventsubscriber"]
        .iter()
        .any(|n| name.eq_ignore_ascii_case(n))
}

impl ObjectHead {
    /// The head of `o`.
    pub fn of(o: &Object) -> ObjectHead {
        let globals: Vec<u32> = o.globals.iter().filter_map(|s| codeunit_of(&s.ty)).collect();
        let events = o
            .routines
            .iter()
            .enumerate()
            .filter(|(_, r)| r.attributes.iter().any(|a| is_event_attribute(&a.name)))
            .map(|(i, r)| EventRoutine {
                routine: i as u32,
                name: r.name.clone(),
                attributes: r.attributes.clone(),
                codeunit_slots: globals
                    .iter()
                    .copied()
                    .chain(r.locals.iter().filter_map(|s| codeunit_of(&s.ty)))
                    .collect(),
            })
            .collect();
        ObjectHead {
            kind: o.kind,
            uncompiled: o.uncompiled,
            id: o.id,
            name: o.name.clone(),
            app: o.app,
            properties: o.properties.clone(),
            namespace: o.namespace.clone(),
            extends_id: o.extends_id,
            enum_values: o.enum_values.clone(),
            table: o.table.clone(),
            page_extensions: o.page.as_ref().map_or_else(Vec::new, |p| p.extensions.clone()),
            report_ext_base: o.report_ext.as_ref().map(|x| x.base),
            routine_count: o.routines.len() as u32,
            events,
        }
    }

    /// Whether the object is a compiled object of `kind` (`Object::is_kind`).
    pub fn is_kind(&self, kind: ObjKind) -> bool {
        self.kind == kind && !self.uncompiled
    }

    /// Whether the object is a shell of `kind` (`Object::is_shell_of`).
    pub fn is_shell_of(&self, kind: ObjKind) -> bool {
        self.kind == kind && self.uncompiled
    }
}

fn codeunit_of(ty: &RtType) -> Option<u32> {
    match ty {
        RtType::Codeunit(c) => Some(*c),
        _ => None,
    }
}
