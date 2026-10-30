//! The link step (ADR-033): symbolic objects in, the dense [`Program`] out.
//!
//! A symbolic object ([`SymObject`]) refers to other objects and routines by stable key through
//! its own [`Refs`](al_ir::sym::Refs) tables. [`link`] gives every object its position, resolves
//! each key to the position it now has, and moves the source positions of every object to their
//! place in `Program::positions`. The result is what lowering used to write directly: the VM reads
//! it unchanged.
//!
//! Core crate: no OS access (ADR-006).
use al_ir::sym::{
    Contribution, ControlKey, EnumImplDecl, Expansion, FieldKey, FxMap, ImplSlot, ListKind, LocalKey, ObjKey,
    RoutineKey, SubscriberTarget, SymObject,
};
use al_ir::walk::{
    ExpansionSite, RefVisitor, TableOf, walk_body, walk_object, walk_page_contribution, walk_table_contribution,
};
use al_ir::{Body, BodyFixup, Stmt};
use al_ir::{Object, Program, SrcPos};
use std::sync::Arc;
use std::sync::OnceLock;

/// One object to link. (An object a layer build only marks as supplied, whose lowered form is never read,
/// is a [`SymObject`] with `placeholder` set and the facts the other objects ask of it.)
pub type Unit = SymObject;

/// One object to link for [`link_items`]: a symbolic unit, or the facts of an object a compile cache supplies (ADR-054),
/// read in place. A placeholder is linked as a [`Unit`] with `placeholder` set and these facts would be, without the
/// unit: a run on Base Application has about ten thousand of them, and each unit is a few KB.
pub enum Item<'p> {
    Unit(Box<Unit>),
    Placeholder(Placeholder<'p>),
}

/// What the other objects ask of an object a compile cache supplies (the fields of a placeholder [`Unit`]).
#[derive(Clone, Copy)]
pub struct Placeholder<'p> {
    pub key: &'p ObjKey,
    pub field_numbers: &'p [u32],
    pub control_names: &'p [String],
    pub implements: &'p [ObjKey],
    pub enum_impls: &'p [EnumImplDecl],
    pub enum_ordinals: &'p [u32],
    pub reserved_positions: u32,
}

/// The object of a placeholder: the default object, as a placeholder [`Unit`] holds.
static NO_OBJECT: std::sync::LazyLock<Object> = std::sync::LazyLock::new(Object::default);

impl Item<'_> {
    pub fn key(&self) -> &ObjKey {
        match self {
            Item::Unit(u) => &u.key,
            Item::Placeholder(p) => p.key,
        }
    }
    /// The symbolic unit, `None` for a placeholder item.
    fn unit(&self) -> Option<&Unit> {
        match self {
            Item::Unit(u) => Some(u),
            Item::Placeholder(_) => None,
        }
    }
    fn placeholder(&self) -> bool {
        self.unit().is_none_or(|u| u.placeholder)
    }
    fn object(&self) -> &Object {
        self.unit().map_or(&NO_OBJECT, |u| &u.object)
    }
    fn field_numbers(&self) -> &[u32] {
        match self {
            Item::Unit(u) => &u.field_numbers,
            Item::Placeholder(p) => p.field_numbers,
        }
    }
    fn control_names(&self) -> &[String] {
        match self {
            Item::Unit(u) => &u.control_names,
            Item::Placeholder(p) => p.control_names,
        }
    }
    fn implements(&self) -> &[ObjKey] {
        match self {
            Item::Unit(u) => &u.implements,
            Item::Placeholder(p) => p.implements,
        }
    }
    fn enum_impls(&self) -> &[EnumImplDecl] {
        match self {
            Item::Unit(u) => &u.enum_impls,
            Item::Placeholder(p) => p.enum_impls,
        }
    }
    fn enum_ordinals(&self) -> &[u32] {
        match self {
            Item::Unit(u) => &u.enum_ordinals,
            Item::Placeholder(p) => p.enum_ordinals,
        }
    }
}

/// Why a set of objects cannot be linked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkError {
    /// Two objects have the same key.
    DuplicateObject(ObjKey),
    /// An object refers to an object that is not among the ones linked.
    UnknownObject { from: ObjKey, to: ObjKey },
    /// An object refers to a routine that the target object does not have.
    UnknownRoutine { from: ObjKey, to: RoutineKey },
    /// A codeunit that implements an interface has no procedure of the shape of the one called.
    NoImplementation {
        from: ObjKey,
        codeunit: ObjKey,
        name: String,
    },
    /// An object refers to a control that the target page does not have.
    UnknownControl { from: ObjKey, to: ControlKey },
    /// An object refers to a field that the target table does not have.
    UnknownField { from: ObjKey, to: FieldKey },
    /// Two routines of one object have the same key.
    DuplicateRoutine { object: ObjKey, key: LocalKey },
    /// A reference in the IR is not an index into the object's table (a site that lowering did not
    /// move into `refs`, or a table that is too short).
    BadReference {
        object: ObjKey,
        class: &'static str,
        index: usize,
    },
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::DuplicateObject(k) => write!(f, "two objects have the key {k}"),
            LinkError::UnknownObject { from, to } => write!(f, "{from} refers to {to}, which is not linked"),
            LinkError::UnknownRoutine { from, to } => {
                write!(f, "{from} refers to routine {} of {}, which has none", to.local, to.obj)
            }
            LinkError::NoImplementation { from, codeunit, name } => {
                write!(
                    f,
                    "{from} calls `{name}` through an interface, which {codeunit} does not implement"
                )
            }
            LinkError::UnknownControl { from, to } => {
                write!(f, "{from} refers to control {} of {}, which has none", to.name, to.page)
            }
            LinkError::UnknownField { from, to } => {
                write!(f, "{from} refers to field {} of {}, which has none", to.no, to.table)
            }
            LinkError::DuplicateRoutine { object, key } => {
                write!(f, "{object} has two routines with the key {key}")
            }
            LinkError::BadReference { object, class, index } => {
                write!(f, "{object} has {class} reference {index}, outside its table")
            }
        }
    }
}

impl std::error::Error for LinkError {}

/// The linked program and where each object's source positions went.
pub struct Linked {
    pub program: Program,
    /// Per object: where its source positions are in `Program::positions` (start, count).
    pub pos_ranges: Vec<(u32, u32)>,
}

/// What keys can be resolved against: a set of objects, each with its key, the keys of its
/// routines and its object. [`link`] resolves against the objects it is given, [`relink`] against
/// a program that is already linked.
trait Space {
    fn len(&self) -> usize;
    fn key(&self, i: usize) -> &ObjKey;
    fn routines(&self, i: usize) -> &[LocalKey];
    fn object(&self, i: usize) -> &Object;
    /// A table's field numbers, in order, when `object` is only a placeholder.
    fn field_numbers(&self, _i: usize) -> Option<&[u32]> {
        None
    }
    /// A page's control names, in order, when `object` is only a placeholder.
    fn control_names(&self, _i: usize) -> Option<&[String]> {
        None
    }
    /// The interfaces codeunit `i` implements.
    fn implemented(&self, _i: usize) -> &[ObjKey] {
        &[]
    }
    /// The implementations enum (or enum extension) `i` declares, the ordinals of its values, and the
    /// enum extensions of an enum, in program order.
    fn enum_impls(&self, _i: usize) -> &[EnumImplDecl] {
        &[]
    }
    fn enum_ordinals(&self, _i: usize) -> &[u32] {
        &[]
    }
    fn enum_extensions(&self, _i: usize) -> &[usize] {
        &[]
    }
    /// The page extensions of page `i` that add controls, in the order they apply, and what they add.
    fn page_extensions(&self, _i: usize) -> Vec<(usize, &al_ir::sym::PageContribution)> {
        Vec::new()
    }
    /// The numbers of the fields that table extensions add to table `i`, in order.
    fn extra_field_numbers(&self, _i: usize) -> &[u32] {
        &[]
    }
}

/// The units to link, with the keys of their routines.
struct UnitSpace<'u, 'k> {
    units: &'u [Item<'u>],
    /// Per table: the numbers of the fields its extensions add.
    extras: FxMap<usize, Vec<u32>>,
    /// Per enum: its extensions, in program order.
    enum_exts: FxMap<usize, Vec<usize>>,
    /// Per page: its extensions, in the order they apply.
    page_exts: FxMap<usize, Vec<usize>>,
    routines: &'k (dyn Fn(usize) -> &'k [LocalKey] + Sync),
}

impl Space for UnitSpace<'_, '_> {
    fn len(&self) -> usize {
        self.units.len()
    }
    fn key(&self, i: usize) -> &ObjKey {
        self.units[i].key()
    }
    fn routines(&self, i: usize) -> &[LocalKey] {
        (self.routines)(i)
    }
    fn object(&self, i: usize) -> &Object {
        self.units[i].object()
    }
    fn field_numbers(&self, i: usize) -> Option<&[u32]> {
        Some(self.units[i].field_numbers()).filter(|f| !f.is_empty())
    }
    fn control_names(&self, i: usize) -> Option<&[String]> {
        Some(self.units[i].control_names()).filter(|c| !c.is_empty())
    }
    fn extra_field_numbers(&self, i: usize) -> &[u32] {
        self.extras.get(&i).map_or(&[], |v| v.as_slice())
    }
    fn implemented(&self, i: usize) -> &[ObjKey] {
        self.units[i].implements()
    }
    fn enum_impls(&self, i: usize) -> &[EnumImplDecl] {
        self.units[i].enum_impls()
    }
    fn enum_ordinals(&self, i: usize) -> &[u32] {
        self.units[i].enum_ordinals()
    }
    fn enum_extensions(&self, i: usize) -> &[usize] {
        self.enum_exts.get(&i).map_or(&[], |v| v.as_slice())
    }
    fn page_extensions(&self, i: usize) -> Vec<(usize, &al_ir::sym::PageContribution)> {
        self.page_exts
            .get(&i)
            .into_iter()
            .flatten()
            .filter_map(|&e| self.units[e].unit()?.contribution.page.as_ref().map(|p| (e, p)))
            .collect()
    }
}

/// The page extensions of each page (and report with a request page), in the order they apply.
fn page_extensions(units: &[Item<'_>]) -> FxMap<usize, Vec<usize>> {
    let at: FxMap<&ObjKey, usize> = units.iter().enumerate().map(|(i, u)| (u.key(), i)).collect();
    let mut found: Vec<(usize, (u32, u32, u32), usize)> = Vec::new();
    for (i, u) in units.iter().enumerate() {
        let Some(s) = u.unit() else { continue };
        let Some(base) = s.extends.as_ref().and_then(|k| at.get(k)).copied() else {
            continue;
        };
        for j in s.joins.iter().filter(|j| j.list == ListKind::PageExtensions) {
            found.push((base, j.order, i));
        }
    }
    found.sort();
    let mut out: FxMap<usize, Vec<usize>> = FxMap::default();
    for (base, _, e) in found {
        out.entry(base).or_default().push(e);
    }
    out
}

/// The enum extensions of each enum, in program order.
fn enum_extensions(units: &[Item<'_>]) -> FxMap<usize, Vec<usize>> {
    let at: FxMap<&ObjKey, usize> = units.iter().enumerate().map(|(i, u)| (u.key(), i)).collect();
    let mut out: FxMap<usize, Vec<usize>> = FxMap::default();
    for (i, u) in units.iter().enumerate() {
        if let Some(s) = u.unit() {
            if s.object.kind == al_ir::ObjKind::EnumExtension {
                if let Some(base) = s.extends.as_ref().and_then(|k| at.get(k)).copied() {
                    out.entry(base).or_default().push(i);
                }
            }
        }
    }
    out
}

/// The numbers of the fields the table extensions of each table add, in program order.
fn extra_fields(units: &[Item<'_>]) -> FxMap<usize, Vec<u32>> {
    let at: FxMap<&ObjKey, usize> = units.iter().enumerate().map(|(i, u)| (u.key(), i)).collect();
    let mut out: FxMap<usize, Vec<u32>> = FxMap::default();
    for s in units.iter().filter_map(Item::unit) {
        let (Some(base), Some(t)) = (
            s.extends.as_ref().and_then(|k| at.get(k)).copied(),
            s.contribution.table.as_ref(),
        ) else {
            continue;
        };
        out.entry(base).or_default().extend(t.fields.iter().map(|f| f.id));
    }
    out
}

/// The positions of keys in a [`Space`], found when first asked for.
struct Tables<'a, S: Space + ?Sized> {
    space: &'a S,
    at: FxMap<&'a ObjKey, usize>,
    routine_at: Vec<OnceLock<FxMap<&'a LocalKey, usize>>>,
    field_at: Vec<OnceLock<FxMap<u32, usize>>>,
    control_at: Vec<OnceLock<FxMap<String, usize>>>,
    /// Per interface: the codeunits that implement it, in program order.
    implementers: OnceLock<FxMap<usize, Vec<usize>>>,
}

impl<'a, S: Space + ?Sized> Tables<'a, S> {
    fn new(space: &'a S) -> Result<Tables<'a, S>, Box<LinkError>> {
        let n = space.len();
        let mut at: FxMap<&ObjKey, usize> = FxMap::with_capacity_and_hasher(n, Default::default());
        for i in 0..n {
            if at.insert(space.key(i), i).is_some() {
                return Err(Box::new(LinkError::DuplicateObject(space.key(i).clone())));
            }
        }
        Ok(Tables {
            space,
            at,
            routine_at: (0..n).map(|_| OnceLock::new()).collect(),
            field_at: (0..n).map(|_| OnceLock::new()).collect(),
            control_at: (0..n).map(|_| OnceLock::new()).collect(),
            implementers: OnceLock::new(),
        })
    }

    fn object(&self, from: &ObjKey, key: &ObjKey) -> Result<usize, Box<LinkError>> {
        self.at.get(key).copied().ok_or_else(|| {
            Box::new(LinkError::UnknownObject {
                from: from.clone(),
                to: key.clone(),
            })
        })
    }

    fn routine_table(&self, i: usize) -> Result<&FxMap<&'a LocalKey, usize>, Box<LinkError>> {
        if let Some(t) = self.routine_at[i].get() {
            return Ok(t);
        }
        let keys = self.space.routines(i);
        let mut t = FxMap::with_capacity_and_hasher(keys.len(), Default::default());
        for (r, k) in keys.iter().enumerate() {
            if t.insert(k, r).is_some() {
                return Err(Box::new(LinkError::DuplicateRoutine {
                    object: self.space.key(i).clone(),
                    key: k.clone(),
                }));
            }
        }
        Ok(self.routine_at[i].get_or_init(|| t))
    }

    /// The position of each field number in a table: its place in the table's merged field list.
    fn field_table(&self, i: usize) -> &FxMap<u32, usize> {
        self.field_at[i].get_or_init(|| {
            if let Some(f) = self.space.field_numbers(i) {
                return first_of(f.iter().copied());
            }
            match &self.space.object(i).table {
                Some(t) => first_of(
                    t.fields
                        .iter()
                        .map(|f| f.id)
                        .chain(self.space.extra_field_numbers(i).iter().copied()),
                ),
                None => FxMap::default(),
            }
        })
    }

    /// The position of each control name in a page's control list: the first of a name, as
    /// lowering took it, compared without case.
    fn control_table(&self, i: usize) -> &FxMap<String, usize> {
        self.control_at[i].get_or_init(|| {
            let mut at = FxMap::default();
            if let Some(names) = self.space.control_names(i) {
                for (pos, c) in names.iter().enumerate() {
                    at.entry(c.clone()).or_insert(pos);
                }
            } else if let Some(p) = &self.space.object(i).page {
                let exts = self.space.page_extensions(i);
                if exts.is_empty() {
                    for (pos, c) in p.controls.iter().enumerate() {
                        at.entry(c.name.to_ascii_lowercase()).or_insert(pos);
                    }
                } else if let Ok(order) = place_controls(p.controls.len(), &exts, &|k: &ObjKey| self.at.get(k).copied())
                {
                    // The page with the controls its extensions add: their names in order.
                    for (pos, id) in order.iter().enumerate() {
                        let name = if id.0 == PAGE_BASE {
                            &p.controls[id.1].name
                        } else {
                            let c = exts.iter().find(|(e, _)| *e == id.0).map(|(_, c)| c);
                            &c.expect("placed from this list").controls[id.1].control.name
                        };
                        at.entry(name.to_ascii_lowercase()).or_insert(pos);
                    }
                }
            }
            at
        })
    }

    /// The codeunits that implement interface `iface`, in program order.
    fn implementers(&self, iface: usize) -> &[usize] {
        let all = self.implementers.get_or_init(|| {
            let mut m: FxMap<usize, Vec<usize>> = FxMap::default();
            for i in 0..self.space.len() {
                for k in self.space.implemented(i) {
                    if let Some(&j) = self.at.get(k) {
                        m.entry(j).or_default().push(i);
                    }
                }
            }
            m
        });
        all.get(&iface).map_or(&[], |v| v.as_slice())
    }

    /// The set an expansion stands for.
    fn expand(&self, from: &ObjKey, e: &Expansion) -> Result<Expanded, Box<LinkError>> {
        match e {
            Expansion::InterfaceTest { interface } => {
                let i = self.object(from, interface)?;
                Ok(Expanded::Test(self.implementers(i).iter().map(|&c| c as u32).collect()))
            }
            Expansion::InterfaceCall {
                interface,
                name,
                params,
            } => {
                let i = self.object(from, interface)?;
                let mut targets = Vec::new();
                for &cu in self.implementers(i) {
                    let shape = |k: &LocalKey| {
                        !k.local
                            && !k.trigger
                            && k.name == *name
                            && k.params.len() == params.len()
                            && k.params
                                .iter()
                                .zip(params)
                                .all(|(a, b)| a.starts_with("var ") == b.starts_with("var "))
                    };
                    let keys = self.space.routines(cu);
                    let same: Vec<usize> = (0..keys.len()).filter(|&r| shape(&keys[r])).collect();
                    let exact = same.iter().copied().find(|&r| keys[r].params == *params);
                    let Some(r) = exact.or(same.first().copied()) else {
                        return Err(Box::new(LinkError::NoImplementation {
                            from: from.clone(),
                            codeunit: self.space.key(cu).clone(),
                            name: name.clone(),
                        }));
                    };
                    targets.push((cu, r));
                }
                Ok(Expanded::Call(targets))
            }
            Expansion::EnumImpl { enum_, interface } => {
                let e = self.object(from, enum_)?;
                let order: Vec<usize> = std::iter::once(e)
                    .chain(self.space.enum_extensions(e).iter().copied())
                    .collect();
                let decls: Vec<&EnumImplDecl> = order.iter().flat_map(|&o| self.space.enum_impls(o)).collect();
                let missing = |_: &str| {
                    Box::new(LinkError::BadReference {
                        object: from.clone(),
                        class: "enum implementation",
                        index: 0,
                    })
                };
                let find = |slot: ImplSlot| -> Result<Option<usize>, Box<LinkError>> {
                    for d in decls.iter().filter(|d| d.slot == slot) {
                        if d.interface.as_ref() != Some(interface) {
                            continue;
                        }
                        return match d.codeunit.as_ref().and_then(|k| self.at.get(k)) {
                            Some(&c) => Ok(Some(c)),
                            None => Err(missing("codeunit")),
                        };
                    }
                    Ok(None)
                };
                let default = find(ImplSlot::Default)?;
                let unknown = find(ImplSlot::Unknown)?;
                let mut by_ordinal = Vec::new();
                for &o in &order {
                    for &ordinal in self.space.enum_ordinals(o) {
                        match find(ImplSlot::Value(ordinal))?.or(default) {
                            Some(c) => by_ordinal.push((ordinal as i32, c)),
                            None => return Err(missing("value")),
                        }
                    }
                }
                Ok(Expanded::Enum { by_ordinal, unknown })
            }
        }
    }

    /// What the tables of symbolic object `s` mean here: the position of every key.
    fn resolve(&self, s: &SymObject) -> Result<Resolved, Box<LinkError>> {
        let mut objects = Vec::with_capacity(s.refs.objects.len());
        for k in &s.refs.objects {
            objects.push(self.object(&s.key, k)?);
        }
        // An index into the object's list of objects: out of range is a defect of whoever made the object.
        let place = |ix: u32, class: &'static str| -> Result<usize, Box<LinkError>> {
            objects.get(ix as usize).copied().ok_or_else(|| {
                Box::new(LinkError::BadReference {
                    object: s.key.clone(),
                    class,
                    index: ix as usize,
                })
            })
        };
        let key_of = |ix: u32| s.refs.objects.get(ix as usize).cloned().unwrap_or_default();
        let mut routines = Vec::with_capacity(s.refs.routines.len());
        for k in &s.refs.routines {
            let o = place(k.obj, "routine")?;
            let r = *self.routine_table(o)?.get(&k.local).ok_or_else(|| {
                Box::new(LinkError::UnknownRoutine {
                    from: s.key.clone(),
                    to: al_ir::sym::RoutineKey {
                        obj: key_of(k.obj),
                        local: k.local.clone(),
                    },
                })
            })?;
            routines.push(r);
        }
        let mut fields = Vec::with_capacity(s.refs.fields.len());
        for k in &s.refs.fields {
            let o = place(k.table, "field")?;
            // A query's column is named by its position, which is the number.
            let pos = if key_of(k.table).kind == al_ir::ObjKind::Query {
                Some(k.no as usize)
            } else {
                self.field_table(o).get(&k.no).copied()
            };
            fields.push(pos.ok_or_else(|| {
                Box::new(LinkError::UnknownField {
                    from: s.key.clone(),
                    to: al_ir::sym::FieldKey {
                        table: key_of(k.table),
                        no: k.no,
                    },
                })
            })?);
        }
        let mut controls = Vec::with_capacity(s.refs.controls.len());
        for k in &s.refs.controls {
            let o = place(k.page, "control")?;
            controls.push(self.control_table(o).get(&k.name).copied().ok_or_else(|| {
                Box::new(LinkError::UnknownControl {
                    from: s.key.clone(),
                    to: al_ir::sym::ControlKey {
                        page: key_of(k.page),
                        name: k.name.clone(),
                    },
                })
            })?);
        }
        let mut expanded = Vec::with_capacity(s.refs.expansions.len());
        for e in &s.refs.expansions {
            expanded.push(self.expand(&s.key, e)?);
        }
        Ok(Resolved {
            objects,
            routines,
            fields,
            controls,
            expanded,
        })
    }
}

/// Rewrites `object`, whose references are indices into the tables of a symbolic object, with
/// what they resolved to.
fn rewrite(object: &mut Object, res: &Arc<Resolved>, key: &ObjKey) -> Result<(), Box<LinkError>> {
    let mut w = Rewrite::new(res, key);
    walk_object(object, &mut w);
    match w.bad {
        Some((class, index)) => Err(Box::new(LinkError::BadReference {
            object: key.clone(),
            class,
            index,
        })),
        None => Ok(()),
    }
}

/// Links `units` into `base`, which holds everything that is not an object (`apps`, `dotnet_types`,
/// `sources`, `uncompiled_subscribers`; its `objects` and `positions` are empty). The objects come
/// out in the order of `units`.
///
/// `routines` gives the keys of the routines of object `i`, asked for the objects that other objects
/// call.
pub fn link<'k>(
    units: Vec<Unit>,
    base: Program,
    workers: usize,
    routines: &'k (dyn Fn(usize) -> &'k [LocalKey] + Sync),
) -> Result<Linked, Box<LinkError>> {
    let items = units.into_iter().map(|u| Item::Unit(Box::new(u))).collect();
    let (mut program, objects, pos_ranges) = link_items(items, base, workers, routines)?;
    program.objects = objects
        .into_iter()
        .map(|o| o.expect("every unit is linked"))
        .collect::<Vec<_>>()
        .into();
    Ok(Linked { program, pos_ranges })
}

/// What [`link_items`] gives: the program without its objects, the objects (`None` for a placeholder item) and
/// [`Linked::pos_ranges`].
pub type LinkedItems = (Program, Vec<Option<Object>>, Vec<(u32, u32)>);

/// [`link`] of [`Item`]s. The program comes back without its objects: they are the list beside it, `None` for a
/// [`Item::Placeholder`] (no object is made for it). The third value is [`Linked::pos_ranges`].
pub fn link_items<'k>(
    units: Vec<Item<'_>>,
    mut base: Program,
    workers: usize,
    routines: &'k (dyn Fn(usize) -> &'k [LocalKey] + Sync),
) -> Result<LinkedItems, Box<LinkError>> {
    debug_assert!(base.objects.is_empty() && base.positions.is_empty());
    // What each symbolic object's tables mean in the program: the position of every key.
    let merges;
    let subscribers;
    let resolved: Vec<Option<Arc<Resolved>>> = {
        let space = UnitSpace {
            units: &units,
            extras: extra_fields(&units),
            enum_exts: enum_extensions(&units),
            page_exts: page_extensions(&units),
            routines,
        };
        let tables = Tables::new(&space)?;
        merges = Merges::collect(&units, &tables);
        subscribers = uncompiled_subscribers(&units, &tables)?;
        par_map(units.iter().collect(), workers, |_, u| match u.unit() {
            Some(s) if !s.placeholder => tables.resolve(s).map(|r| Some(Arc::new(r))),
            _ => Ok(None),
        })
        .into_iter()
        .collect::<Result<_, _>>()?
    };
    // Where each object's source positions go.
    let mut pos_ranges = Vec::with_capacity(units.len());
    let mut next = 0u32;
    for u in &units {
        let len = match u {
            Item::Unit(u) if !u.placeholder => u.positions.len() as u32,
            Item::Unit(u) => u.reserved_positions,
            Item::Placeholder(p) => p.reserved_positions,
        };
        pos_ranges.push((next, len));
        next += len;
    }
    // The units, apart: a placeholder item has nothing to rewrite.
    let n = units.len();
    let mut work: Vec<(usize, Unit, Option<Arc<Resolved>>)> = Vec::new();
    for (i, (u, r)) in units.into_iter().zip(resolved).enumerate() {
        if let Item::Unit(u) = u {
            work.push((i, *u, r));
        }
    }
    let done = par_map(
        work,
        workers,
        |_, (i, u, res)| -> Result<(usize, Done), Box<LinkError>> {
            let mut contribution = u.contribution;
            let (mut object, mut positions, key) = (u.object, u.positions, u.key);
            if let Some(res) = res {
                rewrite(&mut object, &res, &key)?;
                let mut w = Rewrite::new(&res, &key);
                if let Some(t) = contribution.table.as_mut() {
                    walk_table_contribution(t, &mut w);
                }
                if let Some(p) = contribution.page.as_mut() {
                    walk_page_contribution(p, &mut w);
                }
                if let Some((class, index)) = w.bad {
                    return Err(Box::new(LinkError::BadReference {
                        object: key,
                        class,
                        index,
                    }));
                }
            }
            if !positions.is_empty() {
                for r in &mut object.routines {
                    r.shift_positions(pos_ranges[i].0);
                }
                for p in &mut positions {
                    p.object = i as u32;
                }
            }
            Ok((i, (object, positions, contribution)))
        },
    );
    let mut objects: Vec<Option<Object>> = (0..n).map(|_| None).collect();
    let mut contributions: Vec<Contribution> = (0..n).map(|_| Contribution::default()).collect();
    let mut positions_of: Vec<Vec<SrcPos>> = (0..n).map(|_| Vec::new()).collect();
    for d in done {
        let (i, (object, positions, contribution)) = d?;
        objects[i] = Some(object);
        contributions[i] = contribution;
        positions_of[i] = positions;
    }
    base.positions.reserve(next as usize);
    for (i, mut positions) in positions_of.into_iter().enumerate() {
        if positions.is_empty() {
            // A placeholder's ids (ADR-054); nothing for an object without positions.
            base.positions.push_hole(pos_ranges[i].1, i);
        }
        base.positions.append(&mut positions);
    }
    merges.apply(&mut objects, &mut contributions)?;
    if !subscribers.is_empty() {
        base.uncompiled_subscribers = subscribers;
    }
    Ok((base, objects, pos_ranges))
}

/// An object linked, the source positions it holds, and what it adds to its base.
type Done = (Object, Vec<SrcPos>, Contribution);

/// How many objects a worker takes at a time.
const CHUNK: usize = 8;
/// The stack of a worker: the walk recurses on the nesting of the code, as lowering does.
const STACK: usize = 64 << 20;

/// `f` applied to every item, on `workers` threads (the calling thread waits), results in the order of the items.
pub fn par_map<T: Send, R: Send>(items: Vec<T>, workers: usize, f: impl Fn(usize, T) -> R + Sync) -> Vec<R> {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let n = items.len();
    let workers = workers.min(n / CHUNK).max(1);
    if workers == 1 {
        return items.into_iter().enumerate().map(|(i, x)| f(i, x)).collect();
    }
    let slots: Vec<Mutex<Option<T>>> = items.into_iter().map(|x| Mutex::new(Some(x))).collect();
    let out: Vec<Mutex<Option<R>>> = (0..n).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                std::thread::Builder::new()
                    .stack_size(STACK)
                    .spawn_scoped(scope, || {
                        loop {
                            let k = next.fetch_add(CHUNK, Ordering::Relaxed);
                            if k >= n {
                                return;
                            }
                            for i in k..(k + CHUNK).min(n) {
                                let item = slots[i].lock().expect("a worker panicked").take().expect("taken once");
                                let r = f(i, item);
                                *out[i].lock().expect("a worker panicked") = Some(r);
                            }
                        }
                    })
                    .expect("spawn a link worker")
            })
            .collect();
        for h in handles {
            h.join().expect("a link worker panicked");
        }
    });
    out.into_iter()
        .map(|m| m.into_inner().expect("a worker panicked").expect("every item is done"))
        .collect()
}

/// The event subscribers of the objects that were not compiled, as the program lists them: those of an object's
/// events in the order of the objects, then those of the platform's codeunits; each group in the order of the
/// subscribing objects.
fn uncompiled_subscribers<S: Space + ?Sized>(
    units: &[Item<'_>],
    tables: &Tables<'_, S>,
) -> Result<Vec<al_ir::UncompiledSubscriber>, Box<LinkError>> {
    use al_ir::PlatformPublisher as P;
    let mut found: Vec<(u8, usize, usize, u32, al_ir::UncompiledSubscriber)> = Vec::new();
    for (i, u) in units.iter().enumerate() {
        let Some(s) = u.unit() else { continue };
        for d in &s.subscribers {
            let (group, publisher, platform) = match &d.target {
                SubscriberTarget::Object(k) => {
                    let p = tables.object(&s.key, k)?;
                    (0, p, P::None)
                }
                SubscriberTarget::GlobalTriggers => (1, 0, P::GlobalTriggers),
                SubscriberTarget::CaptionClass => (2, 0, P::CaptionClass),
                SubscriberTarget::ReportingTriggers => (3, 0, P::ReportingTriggers),
                SubscriberTarget::FilterHelperTriggers => (4, 0, P::FilterHelperTriggers),
            };
            found.push((
                group,
                publisher,
                i,
                d.index,
                al_ir::UncompiledSubscriber {
                    publisher: (group == 0).then_some(publisher as u32),
                    platform,
                    event: d.event.clone(),
                    element: d.element.clone(),
                    subscriber: d.subscriber.clone(),
                },
            ));
        }
    }
    found.sort_by_key(|f| (f.0, f.1, f.2, f.3));
    Ok(found.into_iter().map(|f| f.4).collect())
}

/// The object `i` of a list being linked, which holds it: a symbolic object (`Merges::symbolic`).
fn obj_at(objects: &mut [Option<Object>], i: usize) -> &mut Object {
    objects[i].as_mut().expect("a symbolic object is linked")
}

/// (extension or `PAGE_BASE`, place in its list): the identity of a control while a page is put together.
type PageId = (usize, usize);
const PAGE_BASE: usize = usize::MAX;

/// The order of all controls of a page: its own `base_n` controls keep their places, and each control a page
/// extension adds goes after the control it names (`None`: first). `contribs` are the extensions that add
/// controls, in the order they apply, with the position of the extension; `at` finds an object by key.
fn place_controls(
    base_n: usize,
    contribs: &[(usize, &al_ir::sym::PageContribution)],
    at: &dyn Fn(&ObjKey) -> Option<usize>,
) -> Result<Vec<PageId>, &'static str> {
    use al_ir::sym::ControlId;
    let id = |c: &ControlId| -> Result<PageId, &'static str> {
        match c {
            ControlId::Base(i) => Ok((PAGE_BASE, *i)),
            ControlId::Ext { ext, index } => at(ext).map(|e| (e, *index)).ok_or("control of an extension"),
        }
    };
    let mut order: Vec<PageId> = (0..base_n).map(|i| (PAGE_BASE, i)).collect();
    let mut pending: Vec<(usize, usize)> = Vec::new();
    for (ci, (_, p)) in contribs.iter().enumerate() {
        for k in 0..p.controls.len() {
            pending.push((ci, k));
        }
    }
    loop {
        let before = pending.len();
        let mut later = Vec::new();
        for (ci, k) in std::mem::take(&mut pending) {
            let (e, p) = &contribs[ci];
            let place = match &p.controls[k].pred {
                None => Some(0),
                Some(pred) => {
                    let pid = id(pred)?;
                    order.iter().position(|x| *x == pid).map(|i| i + 1)
                }
            };
            match place {
                Some(place) => order.insert(place, (*e, k)),
                None => later.push((ci, k)),
            }
        }
        pending = later;
        if pending.is_empty() || pending.len() == before {
            break;
        }
    }
    if pending.is_empty() {
        Ok(order)
    } else {
        Err("control order")
    }
}

/// What the extension objects add to their bases (ADR-033 decision 3), found before the objects are
/// consumed and applied after they are linked. Only a base that is symbolic takes it: an object a
/// compile cache supplied already holds its lists.
/// (base, list, order, extension)
type ListEntry = (usize, ListKind, (u32, u32, u32), usize);

#[derive(Default)]
struct Merges {
    /// (base, list, order, extension)
    lists: Vec<ListEntry>,
    /// (base, extension) of the extensions with values, in program order.
    enums: Vec<(usize, usize)>,
    /// (base, extension) of the table extensions that add fields, keys or changes.
    tables: Vec<(usize, usize)>,
    symbolic: Vec<bool>,
    /// Where each object is (to read a control id of an extension).
    at: FxMap<ObjKey, usize>,
}

impl Merges {
    fn collect<S: Space + ?Sized>(units: &[Item<'_>], tables: &Tables<'_, S>) -> Merges {
        let mut m = Merges {
            symbolic: units.iter().map(|u| !u.placeholder()).collect(),
            at: tables.at.iter().map(|(k, &i)| ((*k).clone(), i)).collect(),
            ..Default::default()
        };
        for (i, u) in units.iter().enumerate() {
            let Some(s) = u.unit() else { continue };
            let Some(base) = s.extends.as_ref().and_then(|k| tables.at.get(k)).copied() else {
                continue;
            };
            for j in &s.joins {
                m.lists.push((base, j.list, j.order, i));
            }
            if !s.contribution.enum_values.is_empty() || !s.contribution.enum_captions.is_empty() {
                m.enums.push((base, i));
            }
            if s.contribution.table.is_some() {
                m.tables.push((base, i));
            }
        }
        m.lists.sort();
        m
    }

    fn apply(&self, objects: &mut [Option<Object>], contributions: &mut [Contribution]) -> Result<(), Box<LinkError>> {
        for &(base, list, _, ext) in &self.lists {
            if !self.symbolic[base] || obj_at(objects, base).uncompiled {
                continue;
            }
            let o = obj_at(objects, base);
            match list {
                ListKind::TableExtensions => {
                    if let Some(t) = o.table.as_mut() {
                        t.extensions.push(ext);
                    }
                }
                ListKind::PageExtensions => {
                    if let Some(p) = o.page.as_mut() {
                        p.extensions.push(ext);
                    }
                }
                ListKind::ReportExtensions => {
                    if let Some(r) = o.report.as_mut() {
                        r.extensions.push(ext);
                    }
                }
            }
        }
        self.apply_tables(objects, contributions);
        self.apply_pages(objects, contributions)?;
        for &(base, ext) in &self.enums {
            if self.symbolic[base] && !obj_at(objects, base).uncompiled {
                let c = &contributions[ext];
                obj_at(objects, base).enum_values.extend(c.enum_values.iter().cloned());
                obj_at(objects, base)
                    .enum_captions
                    .extend(c.enum_captions.iter().cloned());
            }
        }
        Ok(())
    }
}

impl Merges {
    /// Puts the controls the page extensions add into their pages, applies their `modify` blocks and adds their
    /// page triggers. An added control goes after the control `pred` names in the list of all controls (the
    /// controls of the page keep their places), and every index inside the page is made again.
    fn apply_pages(
        &self,
        objects: &mut [Option<Object>],
        contributions: &mut [Contribution],
    ) -> Result<(), Box<LinkError>> {
        use al_ir::sym::ControlId;
        type Id = PageId;
        const BASE: usize = PAGE_BASE;
        #[allow(clippy::needless_range_loop)] // `b` is the object, which `symbolic` and the contributions index too
        for b in 0..objects.len() {
            if !self.symbolic[b] || obj_at(objects, b).uncompiled || obj_at(objects, b).page.is_none() {
                continue;
            }
            let exts: Vec<usize> = obj_at(objects, b).page.as_ref().expect("checked").extensions.clone();
            if exts.iter().all(|&e| contributions[e].page.is_none()) {
                continue;
            }
            let whose = ObjKey::new(
                "",
                obj_at(objects, b).kind,
                obj_at(objects, b).id,
                None,
                &obj_at(objects, b).name,
            );
            let bad = |class: &'static str| {
                Box::new(LinkError::BadReference {
                    object: whose.clone(),
                    class,
                    index: 0,
                })
            };
            let id = |c: &ControlId| -> Result<Id, Box<LinkError>> {
                match c {
                    ControlId::Base(i) => Ok((BASE, *i)),
                    ControlId::Ext { ext, index } => self
                        .at
                        .get(ext)
                        .map(|&e| (e, *index))
                        .ok_or_else(|| bad("control of an extension")),
                }
            };
            let contribs: Vec<(usize, al_ir::sym::PageContribution)> = exts
                .iter()
                .filter_map(|&e| contributions[e].page.take().map(|p| (e, p)))
                .collect();
            let page = obj_at(objects, b).page.as_mut().expect("checked");
            let base_controls = std::mem::take(&mut page.controls);
            let base_n = base_controls.len();
            // The order of all controls.
            let refs: Vec<(usize, &al_ir::sym::PageContribution)> = contribs.iter().map(|(e, p)| (*e, p)).collect();
            let order = place_controls(base_n, &refs, &|k: &ObjKey| self.at.get(k).copied()).map_err(bad)?;
            let place: FxMap<Id, usize> = order.iter().enumerate().map(|(i, x)| (*x, i)).collect();
            let at = |c: &ControlId| -> Result<usize, Box<LinkError>> {
                place.get(&id(c)?).copied().ok_or_else(|| bad("control"))
            };
            let pure = |i: usize| place.get(&(BASE, i)).copied().ok_or_else(|| bad("control"));
            // The controls in their new order, with the indices made again.
            let mut base_slots: Vec<Option<al_ir::Control>> = base_controls.into_iter().map(Some).collect();
            let mut ext_slots: FxMap<Id, al_ir::sym::ExtControl> = FxMap::default();
            for (e, p) in contribs.iter() {
                for (k, c) in p.controls.iter().enumerate() {
                    ext_slots.insert((*e, k), c.clone());
                }
            }
            let mut controls = Vec::with_capacity(order.len());
            let mut areas: FxMap<String, Vec<usize>> = FxMap::default();
            for (pos, x) in order.iter().enumerate() {
                if x.0 == BASE {
                    let mut c = base_slots[x.1].take().expect("a control is placed once");
                    c.parent = c.parent.map(&pure).transpose()?;
                    if let Some(al_ir::ActionRef::Target(t)) = &mut c.action_ref {
                        *t = pure(*t)?;
                    }
                    if let Some(l) = c.part_link.as_mut() {
                        l.provider = l.provider.map(&pure).transpose()?;
                    }
                    if let Some(l) = c.run_object.as_mut().and_then(|r| r.link.as_mut()) {
                        l.provider = l.provider.map(&pure).transpose()?;
                    }
                    controls.push(c);
                } else {
                    let ec = ext_slots.remove(x).expect("an added control is placed once");
                    let mut c = ec.control;
                    c.parent = ec.parent.as_ref().map(at).transpose()?;
                    if let (Some(al_ir::ActionRef::Target(t)), Some(target)) = (&mut c.action_ref, &ec.target) {
                        *t = at(target)?;
                    }
                    if let (Some(l), Some(p)) = (c.part_link.as_mut(), &ec.provider) {
                        l.provider = Some(at(p)?);
                    }
                    if let Some(a) = ec.area {
                        areas.entry(a).or_default().push(pos);
                    }
                    controls.push(c);
                }
            }
            // The `modify` blocks, the extensions in order.
            for (e, p) in &contribs {
                for m in &p.mods {
                    let c = &mut controls[at(&m.target)?];
                    for (name, val) in &m.props {
                        match c.props.iter_mut().find(|(n, _)| n == name) {
                            Some(slot) => slot.1 = val.clone(),
                            None => c.props.push((name.clone(), val.clone())),
                        }
                    }
                    for (name, r) in &m.triggers {
                        c.ext_triggers.push((name.clone(), *e, *r));
                    }
                }
                for (name, r) in &p.triggers {
                    page.ext_triggers.push((name.clone(), *e, *r));
                }
            }
            for a in &mut page.action_areas {
                let mut roots = a.roots.iter().map(|&r| pure(r)).collect::<Result<Vec<_>, _>>()?;
                roots.extend(areas.remove(&a.name).unwrap_or_default());
                roots.sort_unstable();
                a.roots = roots;
            }
            page.controls = controls;
        }
        Ok(())
    }

    /// Merges what table extensions add into their tables the way the members of a table are merged:
    /// the fields of each extension, then its `modify` blocks (the first field of the name takes the
    /// properties), then the triggers every field of the name takes, and the keys.
    fn apply_tables(&self, objects: &mut [Option<Object>], contributions: &mut [Contribution]) {
        let mut at = 0;
        while at < self.tables.len() {
            let base = self.tables[at].0;
            let mut end = at;
            while end < self.tables.len() && self.tables[end].0 == base {
                end += 1;
            }
            let exts: Vec<usize> = self.tables[at..end].iter().map(|&(_, e)| e).collect();
            at = end;
            if !self.symbolic[base] || obj_at(objects, base).uncompiled {
                continue;
            }
            let obj = obj_at(objects, base);
            let Some(table) = obj.table.as_mut() else {
                continue;
            };
            let adds: Vec<al_ir::sym::TableContribution> =
                exts.iter().filter_map(|&e| contributions[e].table.take()).collect();
            for c in &adds {
                for (k, v) in &c.properties {
                    if k == al_ir::sym::UNSUPPORTED_TABLE_PROPERTY {
                        table.unsupported.push(v.clone());
                        continue;
                    }
                    match obj.properties.iter_mut().find(|(pk, _)| pk.eq_ignore_ascii_case(k)) {
                        Some(slot) => slot.1 = v.clone(),
                        None => obj.properties.push((k.clone(), v.clone())),
                    }
                }
                table.fields.extend(c.fields.iter().cloned());
                for m in &c.modifies {
                    if let Some(f) = table.fields.iter_mut().find(|f| f.name.eq_ignore_ascii_case(&m.field)) {
                        for (k, v) in &m.properties {
                            match f.properties.iter_mut().find(|(pk, _)| pk.eq_ignore_ascii_case(k)) {
                                Some(slot) => slot.1 = v.clone(),
                                None => f.properties.push((k.clone(), v.clone())),
                            }
                        }
                    }
                }
            }
            for c in &adds {
                for m in &c.modifies {
                    for f in table
                        .fields
                        .iter_mut()
                        .filter(|f| f.name.eq_ignore_ascii_case(&m.field))
                    {
                        f.ext_before_validate.extend(m.before_validate.iter().copied());
                        f.ext_after_validate.extend(m.after_validate.iter().copied());
                    }
                }
                for k in &c.keys {
                    table.keys.push(k.fields.clone());
                    table.unique_keys.push(k.unique);
                    table.key_props.push(k.props.clone());
                }
            }
        }
    }
}

/// The objects of a linked program, as a [`Space`]: `keys` and `routines` say what the program
/// does not (the key of each object, the keys of its routines).
struct ProgramSpace<'p, 'k> {
    program: &'p Program,
    keys: &'k [ObjKey],
    routines: &'k dyn Fn(usize) -> &'k [LocalKey],
}

impl Space for ProgramSpace<'_, '_> {
    fn len(&self) -> usize {
        self.program.objects.len()
    }
    fn key(&self, i: usize) -> &ObjKey {
        &self.keys[i]
    }
    fn routines(&self, i: usize) -> &[LocalKey] {
        (self.routines)(i)
    }
    fn object(&self, i: usize) -> &Object {
        &self.program.objects[i]
    }
}

/// Puts new versions of some objects into a program that is already linked: the incremental
/// compile (ADR-029). `replace` holds the symbolic objects, by their position. Their keys are
/// resolved against the program as it is, so the declarations of every object (its key, its
/// routines, its fields and controls) must be as they were. The source positions of the program
/// are renumbered in object order, the unchanged objects moved to their new place, as [`link`] would
/// number them. `pos_ranges` is where each object's positions are (start, count), as in [`Linked`].
pub fn relink<'k>(
    program: &mut Program,
    pos_ranges: &mut [(u32, u32)],
    keys: &'k [ObjKey],
    routines: &'k dyn Fn(usize) -> &'k [LocalKey],
    replace: Vec<(usize, SymObject)>,
) -> Result<(), Box<LinkError>> {
    let resolved: Vec<Arc<Resolved>> = {
        let space = ProgramSpace {
            program,
            keys,
            routines,
        };
        let tables = Tables::new(&space)?;
        replace
            .iter()
            .map(|(_, s)| tables.resolve(s).map(Arc::new))
            .collect::<Result<_, _>>()?
    };
    let mut renewed: Vec<Option<(SymObject, Arc<Resolved>)>> = (0..program.objects.len()).map(|_| None).collect();
    for ((oi, s), res) in replace.into_iter().zip(resolved) {
        renewed[oi] = Some((s, res));
    }
    // A lazy table (ADR-054) is read whole: the ranges move.
    let old_positions: Vec<SrcPos> = std::mem::take(&mut program.positions).iter().copied().collect();
    let mut positions: Vec<SrcPos> = Vec::with_capacity(old_positions.len());
    for (oi, slot) in renewed.into_iter().enumerate() {
        let cursor = positions.len() as u32;
        match slot {
            Some((s, res)) => {
                let SymObject {
                    key,
                    mut object,
                    positions: mut own,
                    ..
                } = s;
                rewrite(&mut object, &res, &key)?;
                // What the extensions add to the object is as it was: the declarations did not change.
                let old = &program.objects[oi];
                object.enum_values = old.enum_values.clone();
                object.enum_captions = old.enum_captions.clone();
                if let (Some(n), Some(o)) = (object.table.as_mut(), old.table.as_ref()) {
                    n.extensions = o.extensions.clone();
                }
                if let (Some(n), Some(o)) = (object.page.as_mut(), old.page.as_ref()) {
                    n.extensions = o.extensions.clone();
                }
                if let (Some(n), Some(o)) = (object.report.as_mut(), old.report.as_ref()) {
                    n.extensions = o.extensions.clone();
                }
                let len = own.len() as u32;
                for r in &mut object.routines {
                    r.shift_positions(cursor);
                }
                for p in &mut own {
                    p.object = oi as u32;
                }
                positions.append(&mut own);
                program.objects[oi] = object;
                pos_ranges[oi] = (cursor, len);
            }
            None => {
                let (start, len) = pos_ranges[oi];
                if len > 0 {
                    positions.extend_from_slice(&old_positions[start as usize..(start + len) as usize]);
                    if start != cursor {
                        let by = cursor.wrapping_sub(start);
                        for r in &mut program.objects[oi].routines {
                            r.shift_positions(by);
                        }
                    }
                }
                pos_ranges[oi] = (cursor, len);
            }
        }
    }
    program.positions = positions.into();
    Ok(())
}

/// The positions the keys of one symbolic object's tables resolved to.
struct Resolved {
    objects: Vec<usize>,
    routines: Vec<usize>,
    fields: Vec<usize>,
    controls: Vec<usize>,
    expanded: Vec<Expanded>,
}

/// A set that an expansion stood for.
enum Expanded {
    Call(Vec<(usize, usize)>),
    Test(Vec<u32>),
    Enum {
        by_ordinal: Vec<(i32, usize)>,
        unknown: Option<usize>,
    },
}

struct Rewrite<'a> {
    res: &'a Resolved,
    /// The same rewrite for a routine body that is not decoded yet: it runs when the body is decoded (ADR-022,
    /// "Measured 2026-10-21"). `None` inside that deferred rewrite.
    lazy: Option<Arc<dyn BodyFixup>>,
    bad: Option<(&'static str, usize)>,
}

impl<'a> Rewrite<'a> {
    fn new(res: &'a Arc<Resolved>, key: &ObjKey) -> Rewrite<'a> {
        Rewrite {
            res,
            lazy: Some(Arc::new(LinkBody {
                res: res.clone(),
                key: key.clone(),
            })),
            bad: None,
        }
    }
}

/// The rewrite of one routine body of an object, run when the body is decoded.
struct LinkBody {
    res: Arc<Resolved>,
    key: ObjKey,
}

impl BodyFixup for LinkBody {
    fn apply(&self, body: &mut Vec<Stmt>) -> Result<(), String> {
        let mut w = Rewrite {
            res: &self.res,
            lazy: None,
            bad: None,
        };
        walk_body(body, &mut w);
        match w.bad {
            Some((class, index)) => Err(LinkError::BadReference {
                object: self.key.clone(),
                class,
                index,
            }
            .to_string()),
            None => Ok(()),
        }
    }
}

impl RefVisitor for Rewrite<'_> {
    fn lazy_body(&mut self, body: &mut Body) -> bool {
        match &self.lazy {
            Some(f) => body.fix(f.clone()).is_ok(),
            None => false,
        }
    }

    fn object(&mut self, r: &mut usize) {
        match self.res.objects.get(*r) {
            Some(&p) => *r = p,
            None => self.bad = self.bad.or(Some(("object", *r))),
        }
    }

    fn routine(&mut self, r: &mut usize, _object: usize) {
        match self.res.routines.get(*r) {
            Some(&p) => *r = p,
            None => self.bad = self.bad.or(Some(("routine", *r))),
        }
    }

    fn field(&mut self, r: &mut usize, _table: TableOf) {
        match self.res.fields.get(*r) {
            Some(&p) => *r = p,
            None => self.bad = self.bad.or(Some(("field", *r))),
        }
    }

    fn expansion(&mut self, index: usize, site: ExpansionSite<'_>) {
        match (self.res.expanded.get(index), site) {
            (Some(Expanded::Call(t)), ExpansionSite::InterfaceCall { targets }) => *targets = t.clone(),
            (Some(Expanded::Test(o)), ExpansionSite::InterfaceTest { objects }) => *objects = o.clone(),
            (
                Some(Expanded::Enum { by_ordinal, unknown }),
                ExpansionSite::EnumImpl {
                    by_ordinal: b,
                    unknown: u,
                },
            ) => {
                *b = by_ordinal.clone();
                *u = *unknown;
            }
            _ => self.bad = self.bad.or(Some(("expansion", index))),
        }
    }

    fn control(&mut self, r: &mut usize) {
        match self.res.controls.get(*r) {
            Some(&p) => *r = p,
            None => self.bad = self.bad.or(Some(("control", *r))),
        }
    }
}

/// The position of each field number. A merged field list can hold a number twice (an app that
/// extends a table redeclares a field); the lowering took the first, so a key does too.
fn first_of(ids: impl Iterator<Item = u32>) -> FxMap<u32, usize> {
    let mut at = FxMap::default();
    for (p, id) in ids.enumerate() {
        at.entry(id).or_insert(p);
    }
    at
}

/// The first place where two programs differ, readable; `None` when they are equal. The proof of
/// ADR-033: what link makes must be exactly what lowering used to write.
pub fn first_difference(a: &Program, b: &Program) -> Option<String> {
    if a == b {
        return None;
    }
    if a.objects.len() != b.objects.len() {
        return Some(format!("{} objects against {}", a.objects.len(), b.objects.len()));
    }
    for (i, (x, y)) in a.objects.iter().zip(&b.objects).enumerate() {
        if x != y {
            return Some(format!("object {i} `{}`: {}", x.name, text_difference(x, y)));
        }
    }
    if a.positions != b.positions {
        let at = a
            .positions
            .iter()
            .zip(b.positions.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(a.positions.len().min(b.positions.len()));
        return Some(format!(
            "source position {at}: {:?} against {:?} ({} and {} positions)",
            a.positions.get(at),
            b.positions.get(at),
            a.positions.len(),
            b.positions.len()
        ));
    }
    if a.apps != b.apps {
        return Some("apps".into());
    }
    if a.dotnet_types != b.dotnet_types {
        return Some("dotnet_types".into());
    }
    if a.sources != b.sources {
        return Some("sources".into());
    }
    if a.uncompiled_subscribers != b.uncompiled_subscribers {
        return Some("uncompiled_subscribers".into());
    }
    Some("programs differ in a part this report does not know".into())
}

fn text_difference<T: std::fmt::Debug>(a: &T, b: &T) -> String {
    let (x, y) = (format!("{a:?}"), format!("{b:?}"));
    let at = x
        .bytes()
        .zip(y.bytes())
        .position(|(p, q)| p != q)
        .unwrap_or(x.len().min(y.len()));
    let from = (0..=at.saturating_sub(120))
        .rev()
        .find(|&i| x.is_char_boundary(i) && y.is_char_boundary(i))
        .unwrap_or(0);
    let cut = |t: &str| {
        let end = (at + 120).min(t.len());
        let end = (end..=t.len()).find(|&i| t.is_char_boundary(i)).unwrap_or(t.len());
        t[from..end].to_string()
    };
    format!(
        "first difference at byte {at}:\n  expected ...{}\n  linked   ...{}",
        cut(&x),
        cut(&y)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_ir::sym::{ObjName, Refs};
    use al_ir::{Arg, Expr, ObjKind, Routine, Stmt};

    fn key(id: u32) -> ObjKey {
        ObjKey {
            app: "app".into(),
            kind: ObjKind::Codeunit,
            name: ObjName::Id(id),
        }
    }

    fn keys_of(units: &[Unit]) -> Vec<Vec<LocalKey>> {
        units
            .iter()
            .map(|u| {
                if u.placeholder {
                    vec![local("first", 0), local("second", 0)]
                } else {
                    u.routines.clone()
                }
            })
            .collect()
    }

    /// `link` with the routine keys the symbolic objects carry (a dense unit in these tests has `first` and `second`).
    fn link_all(units: Vec<Unit>, workers: usize) -> Result<Linked, Box<LinkError>> {
        let keys = keys_of(&units);
        let routines = |i: usize| keys[i].as_slice();
        link(units, Program::default(), workers, &routines)
    }

    fn local(name: &str, ordinal: u32) -> LocalKey {
        LocalKey {
            name: name.into(),
            section: String::new(),
            params: Vec::new(),
            local: false,
            trigger: false,
            ordinal,
        }
    }

    fn routine(name: &str, body: Vec<Stmt>) -> Routine {
        Routine {
            name: name.into(),
            field: None,
            container: None,
            is_trigger: false,
            attributes: vec![],
            params: 0,
            by_var: vec![],
            locals: vec![],
            ret: None,
            ret_local: None,
            body: body.into(),
            pos: al_ir::NO_POS,
            end_pos: al_ir::NO_POS,
            begin_pos: al_ir::NO_POS,
            expr_tys: Vec::new(),
            reads_var_max_len: false,
        }
    }

    fn object(id: u32, routines: Vec<Routine>) -> Object {
        Object {
            kind: ObjKind::Codeunit,
            id: Some(id),
            name: format!("C{id}"),
            routines,
            ..Default::default()
        }
    }

    /// Codeunit `from` whose routine `Run` calls routine `Second` of codeunit `to`.
    fn caller(from: u32, to: u32) -> SymObject {
        let call = Stmt::Expr(Expr::Call {
            object: 0,
            routine: 0,
            instance: None,
            args: Vec::<Arg>::new(),
            try_call: false,
        });
        SymObject {
            key: key(from),
            routines: vec![local("run", 0)],
            field_numbers: vec![],
            refs: Refs {
                objects: vec![key(to)],
                routines: vec![al_ir::sym::RoutineRef {
                    obj: 0,
                    local: local("second", 0),
                }],
                ..Default::default()
            },
            object: object(from, vec![routine("Run", vec![call])]),
            positions: vec![],
            ..Default::default()
        }
    }

    fn callee(id: u32) -> SymObject {
        SymObject {
            key: key(id),
            routines: vec![local("first", 0), local("second", 0)],
            refs: Refs::default(),
            field_numbers: vec![],
            object: object(id, vec![routine("First", vec![]), routine("Second", vec![])]),
            positions: vec![],
            ..Default::default()
        }
    }

    fn call_of(p: &Program, at: usize) -> (usize, usize) {
        match &p.objects[at].routines[0].body[0] {
            Stmt::Expr(Expr::Call { object, routine, .. }) => (*object, *routine),
            other => panic!("not a call: {other:?}"),
        }
    }

    #[test]
    fn a_call_finds_its_target_wherever_the_target_is() {
        let a = link_all(vec![caller(1, 2), callee(2)], 1).unwrap();
        assert_eq!(call_of(&a.program, 0), (1, 1));
        let b = link_all(vec![callee(2), callee(3), caller(1, 2)], 1).unwrap();
        assert_eq!(call_of(&b.program, 2), (0, 1));
    }

    /// A body read from the compact form (a cache entry) is linked when it is first read: the call is resolved and the
    /// source positions are shifted as in an eager link, and linking decodes nothing.
    #[test]
    fn a_stored_body_is_linked_on_first_read_as_an_eager_body_is_linked_at_once() {
        let pos = |n: u32| SrcPos {
            file: 0,
            line: n,
            col: 1,
            end_line: n,
            object: 0,
            routine: 0,
        };
        let with_pos = |mut c: SymObject| {
            c.object.routines[0].body.insert(0, Stmt::Pos(0));
            c.positions = vec![pos(7)];
            c
        };
        let mut first = callee(2);
        first.positions = vec![pos(1), pos(2)];
        let eager = link_all(vec![first.clone(), callee(3), with_pos(caller(1, 3))], 1).unwrap();
        let stored = with_pos(caller(1, 3));
        let read: SymObject = postcard::from_bytes(&postcard::to_stdvec(&stored).unwrap()).unwrap();
        assert!(!read.object.routines[0].body.is_loaded());
        let lazy = link_all(vec![first, callee(3), read], 1).unwrap();
        assert!(
            !lazy.program.objects[2].routines[0].body.is_loaded(),
            "link leaves a stored body alone"
        );
        assert_eq!(lazy.program.objects[2].routines[0].body[0], Stmt::Pos(2));
        assert_eq!(call_of_at(&lazy.program, 2, 1), (1, 1));
        assert_eq!(first_difference(&eager.program, &lazy.program), None);
    }

    fn call_of_at(p: &Program, at: usize, stmt: usize) -> (usize, usize) {
        match &p.objects[at].routines[0].body[stmt] {
            Stmt::Expr(Expr::Call { object, routine, .. }) => (*object, *routine),
            other => panic!("not a call: {other:?}"),
        }
    }

    #[test]
    fn a_dependency_object_placed_as_it_is_can_still_be_called_by_key() {
        let dense = SymObject {
            key: key(2),
            placeholder: true,
            object: object(2, vec![routine("First", vec![]), routine("Second", vec![])]),
            ..Default::default()
        };
        let linked = link_all(vec![dense, caller(1, 2)], 1).unwrap();
        assert_eq!(call_of(&linked.program, 1), (0, 1));
    }

    #[test]
    fn a_missing_target_is_an_error_that_names_it() {
        let e = link_all(vec![caller(1, 2)], 1).err().unwrap();
        assert_eq!(
            *e,
            LinkError::UnknownObject {
                from: key(1),
                to: key(2)
            }
        );
        let mut c = callee(2);
        c.routines.truncate(1);
        c.object.routines.truncate(1);
        let e = link_all(vec![caller(1, 2), c], 1).err().unwrap();
        assert!(matches!(*e, LinkError::UnknownRoutine { .. }), "{e:?}");
    }

    #[test]
    fn two_objects_with_one_key_do_not_link() {
        let e = link_all(vec![callee(2), callee(2)], 1).err().unwrap();
        assert_eq!(*e, LinkError::DuplicateObject(key(2)));
    }

    #[test]
    fn a_reference_outside_the_table_is_an_error() {
        let mut c = caller(1, 2);
        c.refs.objects.clear();
        let e = link_all(vec![c, callee(2)], 1).err().unwrap();
        assert!(matches!(*e, LinkError::BadReference { .. }), "{e:?}");
    }

    fn enum_unit(id: u32, kind: ObjKind, values: &[(i32, &str)], extends: Option<u32>) -> Unit {
        let vals: Vec<(i32, String)> = values.iter().map(|(v, n)| (*v, n.to_string())).collect();
        let mut s = SymObject {
            key: ObjKey {
                app: "app".into(),
                kind,
                name: ObjName::Id(id),
            },
            object: Object {
                kind,
                id: Some(id),
                ..Default::default()
            },
            extends: extends.map(|b| ObjKey {
                app: "app".into(),
                kind: ObjKind::Enum,
                name: ObjName::Id(b),
            }),
            ..Default::default()
        };
        if extends.is_some() {
            s.contribution.enum_values = vals;
        } else {
            s.object.enum_values = vals;
        }
        s
    }

    #[test]
    fn extensions_add_their_enum_values_to_the_base_in_program_order() {
        let units = vec![
            enum_unit(10, ObjKind::EnumExtension, &[(5, "Five")], Some(1)),
            enum_unit(1, ObjKind::Enum, &[(0, "Zero"), (1, "One")], None),
            enum_unit(11, ObjKind::EnumExtension, &[(6, "Six")], Some(1)),
        ];
        let linked = link_all(units, 1).unwrap();
        let names: Vec<&str> = linked.program.objects[1]
            .enum_values
            .iter()
            .map(|(_, n)| n.as_str())
            .collect();
        assert_eq!(names, ["Zero", "One", "Five", "Six"]);
        assert!(linked.program.objects[0].enum_values.is_empty());
    }

    fn field(id: u32, name: &str) -> al_ir::Field {
        serde_json::from_value(serde_json::json!({
            "id": id, "name": name, "ty": "Integer", "stored": true, "init": null, "init_text": null,
            "relation_table": null, "relation_tables": [], "auto_increment": false, "relation": false,
            "on_validate": null, "ext_before_validate": [], "ext_after_validate": [], "on_lookup": null,
            "properties": [["Caption", "'Old'"]], "relation_arms": [], "relation_meta": [], "relation_runtime": [],
            "flow_filter": false, "calc": null, "calc_refusal": null, "declared_in": null
        }))
        .unwrap()
    }

    #[test]
    fn a_table_extension_adds_fields_keys_and_changes_in_program_order() {
        use al_ir::sym::{FieldModify, KeyDecl, TableContribution};
        let key = |id, kind| ObjKey {
            app: "app".into(),
            kind,
            name: ObjName::Id(id),
        };
        let table = SymObject {
            key: key(1, ObjKind::Table),
            object: Object {
                kind: ObjKind::Table,
                id: Some(1),
                table: Some(al_ir::Table {
                    fields: vec![field(1, "A")],
                    keys: vec![vec![0]],
                    unique_keys: vec![true],
                    key_props: vec![vec![]],
                    per_company: true,
                    extensions: vec![],
                    unsupported: vec![],
                }),
                ..Default::default()
            },
            refs: Refs {
                objects: vec![key(1, ObjKind::Table)],
                fields: vec![al_ir::sym::FieldRef { table: 0, no: 1 }],
                ..Default::default()
            },
            ..Default::default()
        };
        let ext = |id: u32, field_id: u32, name: &str, key_of: Option<u32>| {
            let mut s = SymObject {
                key: key(id, ObjKind::TableExtension),
                extends: Some(key(1, ObjKind::Table)),
                object: Object {
                    kind: ObjKind::TableExtension,
                    id: Some(id),
                    ..Default::default()
                },
                refs: Refs {
                    objects: vec![key(1, ObjKind::Table)],
                    fields: vec![
                        al_ir::sym::FieldRef { table: 0, no: field_id },
                        al_ir::sym::FieldRef { table: 0, no: 1 },
                    ],
                    ..Default::default()
                },
                ..Default::default()
            };
            s.contribution.table = Some(TableContribution {
                fields: vec![field(field_id, name)],
                keys: key_of
                    .map(|_| KeyDecl {
                        fields: vec![0, 1],
                        unique: false,
                        props: vec![],
                    })
                    .into_iter()
                    .collect(),
                modifies: vec![FieldModify {
                    field: "a".into(),
                    properties: vec![("Caption".into(), "'New'".into())],
                    before_validate: vec![],
                    after_validate: vec![],
                }],
                properties: vec![],
            });
            s
        };
        let units = vec![ext(11, 20, "B", None), table, ext(12, 30, "C", Some(1))];
        let linked = link_all(units, 1).unwrap();
        let t = linked.program.objects[1].table.as_ref().unwrap();
        let ids: Vec<u32> = t.fields.iter().map(|f| f.id).collect();
        assert_eq!(ids, [1, 20, 30]);
        assert_eq!(t.fields[0].properties, [("Caption".to_string(), "'New'".to_string())]);
        assert_eq!(t.keys, [vec![0], vec![2, 0]]);
        assert_eq!(t.unique_keys, [true, false]);
    }

    #[test]
    fn controls_go_after_the_control_they_name_in_any_order_of_the_list() {
        use al_ir::sym::{ControlId, ExtControl, PageContribution};
        let ext = |k: usize, pred: Option<ControlId>| ExtControl {
            control: al_ir::Control {
                name: format!("X{k}"),
                kind: al_ir::ControlKind::Field,
                parent: None,
                binding: al_ir::Binding::None,
                props: vec![],
                triggers: vec![],
                part_page: None,
                part_link: None,
                owner: None,
                ext_triggers: vec![],
                run_object: None,
                action_ref: None,
                prop_texts: vec![],
                var_field: None,
                var_slot: None,
                expr_source: None,
                expr_call: false,
                array_element: None,
            },
            pred,
            parent: None,
            target: None,
            provider: None,
            area: None,
            provider_page: None,
        };
        let me = ObjKey {
            app: "a".into(),
            kind: ObjKind::PageExtension,
            name: ObjName::Id(9),
        };
        // X1 after X0, X0 after base control 1: listed in the wrong order on purpose.
        let page = PageContribution {
            controls: vec![
                ext(
                    1,
                    Some(ControlId::Ext {
                        ext: me.clone(),
                        index: 1,
                    }),
                ),
                ext(0, Some(ControlId::Base(1))),
            ],
            ..Default::default()
        };
        let order = place_controls(3, &[(7, &page)], &|k| (k == &me).then_some(7)).unwrap();
        assert_eq!(order, [(PAGE_BASE, 0), (PAGE_BASE, 1), (7, 1), (7, 0), (PAGE_BASE, 2)]);
        let lost = PageContribution {
            controls: vec![ext(0, Some(ControlId::Base(9)))],
            ..Default::default()
        };
        assert!(place_controls(3, &[(7, &lost)], &|_| None).is_err());
    }

    #[test]
    fn par_map_keeps_the_order_of_the_items_on_any_number_of_workers() {
        let items: Vec<usize> = (0..1000).collect();
        for workers in [1, 2, 7] {
            let out = par_map(items.clone(), workers, |i, x| (i, x * 2));
            assert!(
                out.iter().enumerate().all(|(i, &(j, v))| i == j && v == i * 2),
                "{workers}"
            );
        }
        assert!(par_map(Vec::<u8>::new(), 4, |_, x| x).is_empty());
    }

    #[test]
    fn source_positions_move_to_their_place_in_the_program() {
        let pos = |line| SrcPos {
            file: 0,
            line,
            col: 1,
            end_line: line,
            object: 0,
            routine: 0,
        };
        let mut a = callee(1);
        a.positions = vec![pos(1), pos(2)];
        a.object.routines[0].body = vec![Stmt::Pos(1)].into();
        a.object.routines[0].pos = 0;
        let mut b = callee(2);
        b.positions = vec![pos(7)];
        b.object.routines[1].pos = 0;
        let linked = link_all(vec![a, b], 1).unwrap();
        assert_eq!(linked.pos_ranges, vec![(0, 2), (2, 1)]);
        assert_eq!(linked.program.objects[1].routines[1].pos, 2);
        assert_eq!(linked.program.positions[2].line, 7);
        assert_eq!(linked.program.positions[2].object, 1);
    }
}
