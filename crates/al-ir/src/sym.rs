//! The symbolic form of an object (ADR-033): what lowering produces and the link step turns into
//! [`crate::Program`].
//!
//! A symbolic object is an [`Object`] whose references to other objects, routines and fields are
//! not positions in the program but indices into the object's own [`Refs`] tables. Each table
//! entry is a stable key (decision D1 and D2 of the plan: an object by kind and id or name, a
//! routine by name and parameter-type signature, a field by table and field number). The object
//! therefore does not depend on which other objects are in the program or in which order they
//! come. Only link knows positions.

use crate::{Object, SrcPos};
use serde::{Deserialize, Serialize};

pub use crate::ObjKind;

/// The name part of an [`ObjKey`] (owner decision D1: the id where the object has one).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ObjName {
    Id(u32),
    /// The namespace-qualified name in lower case, for a kind without an id (an interface).
    Name(String),
}

impl Default for ObjName {
    fn default() -> ObjName {
        ObjName::Id(0)
    }
}

/// The identity of an object: the app that declares it, its kind, and its id or name.
///
/// The app is part of the key because two apps of one program can declare the same kind and id
/// (the Microsoft apps of one BC version do: a few tables and many permission sets exist twice).
/// A reference resolved to one of them must not resolve to the other when the set of apps changes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct ObjKey {
    /// The id of the declaring app (`AppInfo::id`).
    pub app: std::sync::Arc<str>,
    pub kind: ObjKind,
    pub name: ObjName,
}

// Hashed without the app: it is long, two objects with one kind and id in different apps are rare, and equality
// still tells them apart.
impl std::hash::Hash for ObjKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        self.name.hash(state);
    }
}

impl ObjKey {
    /// The key of an object with this app, kind, id, namespace and name.
    pub fn new(app: &str, kind: ObjKind, id: Option<u32>, namespace: Option<&str>, name: &str) -> ObjKey {
        let name = match id {
            Some(id) => ObjName::Id(id),
            None => ObjName::Name(match namespace {
                Some(ns) if !ns.is_empty() => format!("{}.{}", ns.to_lowercase(), name.to_lowercase()),
                _ => name.to_lowercase(),
            }),
        };
        ObjKey {
            app: app.into(),
            kind,
            name,
        }
    }

    /// The key of an object of a lowered program; `apps` is `Program::apps`.
    pub fn of(o: &Object, apps: &[crate::AppInfo]) -> ObjKey {
        let app = apps.get(o.app as usize).map_or("", |a| a.id.as_str());
        ObjKey::new(app, o.kind, o.id, o.namespace.as_deref(), &o.name)
    }
}

impl std::fmt::Display for ObjKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            ObjName::Id(id) => write!(f, "{} {id}", self.kind)?,
            ObjName::Name(n) => write!(f, "{} \"{n}\"", self.kind)?,
        }
        write!(f, " of app {}", self.app)
    }
}

/// The identity of a routine inside its object (owner decision D2): its name in lower case, the
/// section it belongs to when it is a trigger of a field or a control, and the parameter types.
/// `ordinal` separates routines that are equal in everything else, in declaration order (zero for
/// nearly every routine).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalKey {
    pub name: String,
    /// `field 7`, `control Name`, `action Name`, ...: the owning section of a trigger. Empty for
    /// a procedure and for a trigger of the object itself.
    pub section: String,
    /// The parameters, one text per parameter: the type as written in lower case without spaces or quotes,
    /// after `var ` for a parameter passed by reference.
    pub params: Vec<String>,
    /// A procedure declared `local`.
    pub local: bool,
    /// A trigger.
    pub trigger: bool,
    pub ordinal: u32,
}

impl std::fmt::Display for LocalKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name, self.params.join(","))?;
        if !self.section.is_empty() {
            write!(f, " in {}", self.section)?;
        }
        if self.ordinal > 0 {
            write!(f, " number {}", self.ordinal + 1)?;
        }
        Ok(())
    }
}

/// The identity of a routine of any object.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RoutineKey {
    pub obj: ObjKey,
    pub local: LocalKey,
}

/// The identity of a field: the table and the field number, extensions included.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FieldKey {
    pub table: ObjKey,
    pub no: u32,
}

/// The identity of a control of a page: the page and the control name (unique in a page,
/// extensions included).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ControlKey {
    pub page: ObjKey,
    pub name: String,
}

/// Where an enum declares that a codeunit implements an interface (`Implementation`, `DefaultImplementation`,
/// `UnknownValueImplementation`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplSlot {
    /// On the value with this ordinal.
    Value(u32),
    Default,
    Unknown,
}

/// One `Interface = Codeunit` pair of an enum or enum extension, as lowering resolved the names (`None`: not found).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumImplDecl {
    pub slot: ImplSlot,
    pub interface: Option<ObjKey>,
    pub codeunit: Option<ObjKey>,
}

/// Whose event a subscriber subscribes to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriberTarget {
    Object(ObjKey),
    GlobalTriggers,
    CaptionClass,
    ReportingTriggers,
    FilterHelperTriggers,
}

/// An event subscriber of an object that was not compiled (a dependency object nothing reached): the runtime
/// refuses to raise the event it subscribes to, because it has no code to run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriberDecl {
    pub target: SubscriberTarget,
    /// The place of the subscriber among those the object declares.
    pub index: u32,
    pub event: String,
    pub element: String,
    /// `codeunit "Name"`, for the refusal text.
    pub subscriber: String,
}

/// A reference that stands for a set of objects or routines that only the whole program knows. Lowering writes
/// a marker (`walk::EXPAND`) at the site and the index of the entry; link replaces the marker with the set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expansion {
    /// `Expr::InterfaceCall`: the procedure of this shape in every codeunit that implements the interface.
    InterfaceCall {
        interface: ObjKey,
        name: String,
        params: Vec<String>,
    },
    /// `Expr::InterfaceTest`: every codeunit that implements the interface.
    InterfaceTest { interface: ObjKey },
    /// `Expr::EnumImpl`: the codeunit that implements the interface for each value of the enum.
    EnumImpl { enum_: ObjKey, interface: ObjKey },
}

/// A routine of another object, as an object holds it: the object is a place in [`Refs::objects`] (the keys of the
/// objects are not repeated), the routine its key in that object.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoutineRef {
    pub obj: u32,
    pub local: LocalKey,
}

/// A field of a table in [`Refs::objects`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldRef {
    pub table: u32,
    pub no: u32,
}

/// A control of a page in [`Refs::objects`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlRef {
    pub page: u32,
    pub name: String,
}

/// What a symbolic object refers to. A reference inside the object's IR is an index into one of
/// these lists.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Refs {
    pub objects: Vec<ObjKey>,
    pub routines: Vec<RoutineRef>,
    pub fields: Vec<FieldRef>,
    pub controls: Vec<ControlRef>,
    pub expansions: Vec<Expansion>,
}

impl Refs {
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
            && self.routines.is_empty()
            && self.fields.is_empty()
            && self.controls.is_empty()
            && self.expansions.is_empty()
    }
}

/// The lists of extensions that a base object holds in the linked program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ListKind {
    /// `Table::extensions`.
    TableExtensions,
    /// `Page::extensions`: the page extensions (and request page extensions of a report) merged into a page.
    PageExtensions,
    /// `Report::extensions`: the report extensions that change a run.
    ReportExtensions,
}

/// An extension object joins the list of its base (`SymObject::extends`). Link puts the joined objects
/// in the list in the order of `order`, then of their place in the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Join {
    pub list: ListKind,
    pub order: (u32, u32, u32),
}

/// A key a table extension declares: fields as references, as the table holds them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyDecl {
    pub fields: Vec<usize>,
    pub unique: bool,
    pub props: Vec<(String, String)>,
}

/// `modify(Field)` of a table extension: it changes properties of the field the name says and adds
/// triggers to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldModify {
    pub field: String,
    pub properties: Vec<(String, String)>,
    /// (object, routine) of the `OnBeforeValidate` and `OnAfterValidate` triggers it declares.
    pub before_validate: Vec<(usize, usize)>,
    pub after_validate: Vec<(usize, usize)>,
}

/// The property name a [`TableContribution`] uses for a reason that refuses its table: link adds the value to
/// `Table::unsupported`.
pub const UNSUPPORTED_TABLE_PROPERTY: &str = "\0unsupported";

/// What a table extension adds to its table.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct TableContribution {
    pub fields: Vec<crate::Field>,
    pub keys: Vec<KeyDecl>,
    pub modifies: Vec<FieldModify>,
    /// The object-level properties the extension sets on its table (`DataCaptionFields`, `LookupPageId`,
    /// `DrillDownPageId`), as (name, value) in declared order.
    #[serde(default)]
    pub properties: Vec<(String, String)>,
}

/// A control of a page among the controls of the page and of its extensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlId {
    /// The place among the controls of the page itself.
    Base(usize),
    /// The place among the controls an extension adds.
    Ext { ext: ObjKey, index: usize },
}

/// A control a page extension adds, and where it goes: after `pred` in the list of all controls (`None`: first),
/// in the control `parent` (`None`: at the top of an area).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtControl {
    /// The control. Its `parent`, `action_ref` target and `part_link` provider are replaced by the ids below.
    pub control: crate::Control,
    pub pred: Option<ControlId>,
    pub parent: Option<ControlId>,
    pub target: Option<ControlId>,
    pub provider: Option<ControlId>,
    /// For a control at the top of an area of `actions`: the name of the area.
    pub area: Option<String>,
    /// While the object is made symbolic: the page the part named by `provider` shows, as a position in the
    /// workspace (the host fields of the part's link are fields of its source table). Cleared after.
    pub provider_page: Option<usize>,
}

/// What one `modify` block of a page extension changes in a control.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlMod {
    pub target: ControlId,
    pub props: Vec<(String, crate::PropVal)>,
    /// (trigger name in lower case, routine of the extension).
    pub triggers: Vec<(String, usize)>,
}

/// What a page extension adds to its page.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PageContribution {
    pub controls: Vec<ExtControl>,
    pub mods: Vec<ControlMod>,
    /// The page triggers it declares: (name in lower case, routine of the extension).
    pub triggers: Vec<(String, usize)>,
}

/// What an extension object adds to its base, which link merges into the base in the order of the
/// extensions in the program (ADR-033 decision 3).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Contribution {
    /// The values and captions an enum extension declares.
    pub enum_values: Vec<(i32, String)>,
    pub enum_captions: Vec<(i32, String)>,
    /// The fields, keys and field changes a table extension declares.
    pub table: Option<TableContribution>,
    /// The controls, changes and triggers a page extension (or a report extension's request page) adds.
    pub page: Option<PageContribution>,
}

/// An object as lowering produces it.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SymObject {
    pub key: ObjKey,
    /// The key of each routine of `object`, in the order of `object.routines`. Lowering leaves it
    /// empty (link asks for the keys of the objects that are called); a stored object fills it.
    pub routines: Vec<LocalKey>,
    pub refs: Refs,
    /// For a codeunit: the interfaces it implements, those it lists and the ones they extend.
    pub implements: Vec<ObjKey>,
    /// For an object that was not compiled: its event subscribers.
    pub subscribers: Vec<SubscriberDecl>,
    /// For an enum or enum extension: the implementations it declares and the ordinals of its values.
    pub enum_impls: Vec<EnumImplDecl>,
    pub enum_ordinals: Vec<u32>,
    /// For an extension object: the object it extends, when it is in the program.
    pub extends: Option<ObjKey>,
    /// The lists of its base that the extension is in.
    pub joins: Vec<Join>,
    /// What it adds to its base.
    pub contribution: Contribution,
    /// Only the facts other objects ask of it are here: a layer build marks the objects of the layers below as
    /// supplied and never reads their lowered form.
    pub placeholder: bool,
    /// For a page of a placeholder: the name of each control in lower case, in order.
    pub control_names: Vec<String>,
    /// For a table whose IR carries no field list (a dependency table that was not compiled) but
    /// whose fields other objects use: the number of each field, in the order of the table's field
    /// list. Empty otherwise.
    pub field_numbers: Vec<u32>,
    /// The object. Every reference class that has moved into `refs` holds an index into it.
    pub object: Object,
    /// The source positions of the object's statements and routines, numbered from 0 (a statement
    /// marker and a routine's `pos` hold an index into this list). Link moves them to their place
    /// in `Program::positions`. Their `object` and `routine` are the object's own.
    pub positions: Vec<SrcPos>,
    /// For a placeholder: how many source positions the object has. Link keeps their ids free, as a hole of the
    /// position table that the program's source fills (ADR-054), so the ids of every object are those of a complete
    /// compile. Not part of the stored form.
    #[serde(skip)]
    pub reserved_positions: u32,
}

/// A fast hash for the interning tables of lowering and link, whose keys are small integers and short
/// names (the std hash is built to resist attacks, which these keys do not need).
#[derive(Default, Clone, Copy)]
pub struct Fx(u64);

impl std::hash::Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut chunks = bytes.chunks_exact(8);
        for c in &mut chunks {
            self.write_u64(u64::from_le_bytes(c.try_into().expect("eight bytes")));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut last = [0u8; 8];
            last[..rest.len()].copy_from_slice(rest);
            self.write_u64(u64::from_le_bytes(last) ^ ((rest.len() as u64) << 56));
        }
    }
    fn write_u8(&mut self, i: u8) {
        self.write_u64(u64::from(i));
    }
    fn write_u32(&mut self, i: u32) {
        self.write_u64(u64::from(i));
    }
    fn write_usize(&mut self, i: usize) {
        self.write_u64(i as u64);
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

/// A `HashMap` with the [`Fx`] hash.
pub type FxMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<Fx>>;
