//! From the object lowering wrote (references are positions in the program) to the symbolic
//! object link takes (references are indices into the object's own tables of stable keys,
//! ADR-033).
//!
//! The keys of an object's routines come from its syntax, not from its lowered form: a routine
//! that nothing reached is lowered to a stub without parameters, and its key must not change
//! because of that.

use al_ir as ir;
use al_ir::sym::{
    Contribution, ControlRef, FieldRef, Join, ListKind, LocalKey, ObjKey, Refs, RoutineRef, SymObject,
    TableContribution,
};
use al_ir::walk::{RefVisitor, TableOf, walk_object};
use al_syntax::ast;
use std::collections::HashMap;

/// The routine keys of an object, one per routine node in the order the object numbers them.
pub(crate) fn routine_keys(nodes: &[ast::Routine]) -> Vec<LocalKey> {
    let mut seen: HashMap<(String, String, Vec<String>), u32> = HashMap::new();
    nodes
        .iter()
        .map(|r| {
            let name = r.name().to_lowercase();
            let section = section_of(r);
            let params = parameters_of(r);
            let slot = seen.entry((name.clone(), section.clone(), params.clone())).or_insert(0);
            let ordinal = *slot;
            *slot += 1;
            LocalKey {
                name,
                section,
                params,
                local: r.is_local(),
                trigger: r.is_trigger(),
                ordinal,
            }
        })
        .collect()
}

/// The nearest enclosing section of a trigger (`field 7`, `action Post`), as
/// `ir::Routine::container` holds it, in lower case. Empty for a procedure or a trigger of the
/// object itself.
fn section_of(r: &ast::Routine) -> String {
    match r.syntax().ancestors().find_map(ast::Section::cast) {
        Some(sec) => {
            let arg = sec.args().first().cloned().unwrap_or_default();
            format!(
                "{} {}",
                sec.name().to_ascii_lowercase(),
                arg.trim().trim_matches('"').to_lowercase()
            )
        }
        None => String::new(),
    }
}

/// The parameters as written: the type in lower case without spaces or quotes, after `var ` for a
/// parameter passed by reference.
pub(crate) fn parameters_of(r: &ast::Routine) -> Vec<String> {
    r.params()
        .map(|p| {
            let mut out = String::new();
            if p.is_var() {
                out.push_str("var ");
            }
            if let Some(t) = p.type_ref() {
                t.syntax().text().for_each_chunk(|chunk| {
                    for c in chunk.chars() {
                        if !c.is_whitespace() && c != '"' {
                            if c.is_ascii() {
                                out.push(c.to_ascii_lowercase());
                            } else {
                                out.extend(c.to_lowercase());
                            }
                        }
                    }
                });
            }
            out
        })
        .collect()
}

/// The routine keys of the objects of a program, made from the syntax of an object when another
/// object first asks for them. An object a compile cache supplies has its keys in the cache.
pub(crate) struct RoutineKeys<'a> {
    ws: &'a crate::workspace::Workspace,
    /// Per object: the keys the cache supplies.
    supplied: Vec<Option<&'a [LocalKey]>>,
    /// Per object: whether it is compiled (a shell has no routines).
    compiled: Vec<bool>,
    cells: Vec<std::sync::OnceLock<Vec<LocalKey>>>,
}

impl<'a> RoutineKeys<'a> {
    pub(crate) fn new(
        ws: &'a crate::workspace::Workspace,
        supplied: Vec<Option<&'a [LocalKey]>>,
        compiled: Vec<bool>,
    ) -> RoutineKeys<'a> {
        let n = compiled.len();
        RoutineKeys {
            ws,
            supplied,
            compiled,
            cells: (0..n).map(|_| std::sync::OnceLock::new()).collect(),
        }
    }

    /// The keys of the routines of object `i`, in the order the object numbers them.
    pub(crate) fn of(&self, i: usize) -> &[LocalKey] {
        if let Some(k) = self.supplied[i] {
            return k;
        }
        self.cells[i].get_or_init(|| {
            if self.compiled[i] || self.ws.usable(i) {
                routine_keys(&ast::routines_in(self.ws.object_ast(i).syntax()))
            } else {
                Vec::new()
            }
        })
    }
}

/// The fields the expressions of an object name, as (table, field number), in the order lowering
/// first named them. An expression holds the index into this list.
#[derive(Default)]
pub(crate) struct FieldRefs {
    pub(crate) list: Vec<(usize, u32)>,
    ix: FxMap<(usize, u32), usize>,
}

impl FieldRefs {
    pub(crate) fn intern(&mut self, table: usize, id: u32) -> usize {
        let list = &mut self.list;
        *self.ix.entry((table, id)).or_insert_with(|| {
            list.push((table, id));
            list.len() - 1
        })
    }
}

/// The controls the expressions of an object name: (page, control name in lower case, the control's
/// position in the page's layout, which the check against the old numbering reads). An expression
/// holds the index into this list.
#[derive(Default)]
pub(crate) struct ControlRefs {
    pub(crate) list: Vec<(usize, String, usize)>,
    ix: FxMap<(usize, String), usize>,
}

impl ControlRefs {
    pub(crate) fn intern(&mut self, page: usize, name: &str, pos: usize) -> usize {
        let name = name.to_ascii_lowercase();
        let list = &mut self.list;
        *self.ix.entry((page, name.clone())).or_insert_with(|| {
            list.push((page, name, pos));
            list.len() - 1
        })
    }
}

use al_ir::ObjKind;
use al_ir::sym::FxMap;

/// A set that lowering worked out for a site and the symbolic form leaves to link: the answer it found
/// (positions, for the check and for the cache) and what it asked.
pub(crate) enum Expanded {
    InterfaceCall {
        interface: usize,
        name: String,
        params: Vec<String>,
        targets: Vec<(usize, usize)>,
    },
    InterfaceTest {
        interface: usize,
        objects: Vec<u32>,
    },
    EnumImpl {
        enum_: usize,
        interface: usize,
        by_ordinal: Vec<(i32, usize)>,
        unknown: Option<usize>,
    },
}

impl Expanded {
    /// The entry of `Refs::expansions` for it.
    pub(crate) fn symbolic(&self, keys: &[ObjKey]) -> al_ir::sym::Expansion {
        use al_ir::sym::Expansion;
        match self {
            Expanded::InterfaceCall {
                interface,
                name,
                params,
                ..
            } => Expansion::InterfaceCall {
                interface: keys[*interface].clone(),
                name: name.clone(),
                params: params.clone(),
            },
            Expanded::InterfaceTest { interface, .. } => Expansion::InterfaceTest {
                interface: keys[*interface].clone(),
            },
            Expanded::EnumImpl { enum_, interface, .. } => Expansion::EnumImpl {
                enum_: keys[*enum_].clone(),
                interface: keys[*interface].clone(),
            },
        }
    }

    /// The answer, written where the marker is (what lowering used to write).
    pub(crate) fn fill(&self, site: al_ir::walk::ExpansionSite<'_>) {
        use al_ir::walk::ExpansionSite as S;
        match (self, site) {
            (Expanded::InterfaceCall { targets, .. }, S::InterfaceCall { targets: t }) => *t = targets.clone(),
            (Expanded::InterfaceTest { objects, .. }, S::InterfaceTest { objects: o }) => *o = objects.clone(),
            (
                Expanded::EnumImpl {
                    by_ordinal, unknown, ..
                },
                S::EnumImpl {
                    by_ordinal: b,
                    unknown: u,
                },
            ) => {
                *b = by_ordinal.clone();
                *u = *unknown;
            }
            _ => unreachable!("the marker and its entry are of one kind"),
        }
    }
}

/// What one `modify` block of a page extension changes in a control (an index of the page's control list): the
/// properties it sets and the triggers it adds, in the order it wrote them.
#[derive(Clone, Debug)]
pub(crate) struct PageMod {
    pub(crate) control: usize,
    pub(crate) ext: usize,
    pub(crate) props: Vec<(String, ir::PropVal)>,
    pub(crate) triggers: Vec<(String, usize)>,
}

/// The `modify` blocks of the page being lowered, and the properties of the controls they change before any applies.
#[derive(Default)]
pub(crate) struct PageMods {
    pub(crate) mods: Vec<PageMod>,
    pub(crate) before: Vec<(usize, Vec<(String, ir::PropVal)>)>,
}

/// The sites that stand for sets: lowering writes a marker and the index of the entry.
#[derive(Default)]
pub(crate) struct Expansions {
    pub(crate) list: Vec<Expanded>,
}

/// The implementations an enum or enum extension declares (slot, interface, codeunit, in workspace numbering) and
/// the ordinals of its values.
pub(crate) type EnumInfo = (Vec<(al_ir::sym::ImplSlot, Option<usize>, Option<usize>)>, Vec<u32>);

/// The values and the captions an enum declares.
pub(crate) type EnumOwn = (Vec<(i32, String)>, Vec<(i32, String)>);

/// The `modify` blocks of a table extension: field name and the properties it sets.
pub(crate) type Modifies = Vec<(String, Vec<(String, String)>)>;

/// What the symbolic form of each object is made from: the key of every object and of its routines,
/// in program order, and what the lowering knew about tables.
pub(crate) struct Keys<'a> {
    pub objects: &'a [ObjKey],
    /// The key of each routine of object `i`.
    pub routines: &'a (dyn Fn(usize) -> &'a [LocalKey] + Sync),
    /// Per object: the table whose record it works on (`Shared::table_of`).
    pub table_of: &'a [Option<usize>],
    /// The number of field `pos` of table `table`.
    pub field_no: &'a (dyn Fn(usize, usize) -> Option<u32> + Sync),
    /// The implementations and value ordinals an enum or enum extension declares itself.
    pub enum_info: &'a (dyn Fn(usize) -> EnumInfo + Sync),
    /// The interfaces codeunit `i` implements (those it lists and the ones they extend), in workspace numbering.
    pub implements: &'a (dyn Fn(usize) -> Vec<usize> + Sync),
    /// The object an extension object extends, when it is in the program.
    pub extends: &'a (dyn Fn(usize) -> Option<usize> + Sync),
    /// Where an extension object is in the order of the list of its base.
    pub join_order: &'a (dyn Fn(usize, ListKind) -> (u32, u32, u32) + Sync),
    /// The values and captions an enum or enum extension declares itself, as the IR holds them.
    pub own_enum: &'a (dyn Fn(usize) -> EnumOwn + Sync),
    /// The table (object) of data item `item` of report `report`.
    pub report_item_table: &'a (dyn Fn(usize, usize) -> Option<usize> + Sync),
}

/// What other objects hold of an object that the symbolic form takes out of them.
pub(crate) struct Extras<'a> {
    /// The lists of its base that the object is in: (list, base).
    pub joins: &'a [(ListKind, usize)],
    pub table_contribution: Option<TableContribution>,
    pub page_contribution: Option<al_ir::sym::PageContribution>,
    pub expansions: &'a [Expanded],
    /// The uncompiled subscribers this object declares.
    pub subscribers: Vec<al_ir::sym::SubscriberDecl>,
}

/// Makes the symbolic object of a lowered one. `Err` names a reference that points outside the
/// program (a bug of the lowering).
pub(crate) fn localize(
    keys: &Keys<'_>,
    own: usize,
    mut object: ir::Object,
    positions: Vec<ir::SrcPos>,
    field_refs: &[(usize, u32)],
    control_refs: &[(usize, String, usize)],
    extras: Extras<'_>,
) -> Result<SymObject, String> {
    let Extras {
        joins,
        table_contribution,
        page_contribution,
        expansions,
        subscribers,
    } = extras;
    let object_kind = object.kind;
    let (enum_impls, enum_ordinals) = if matches!(object_kind, ObjKind::Enum | ObjKind::EnumExtension) {
        let (decls, ordinals) = (keys.enum_info)(own);
        let key = |i: Option<usize>| i.map(|i| keys.objects[i].clone());
        (
            decls
                .into_iter()
                .map(|(slot, interface, codeunit)| al_ir::sym::EnumImplDecl {
                    slot,
                    interface: key(interface),
                    codeunit: key(codeunit),
                })
                .collect(),
            ordinals,
        )
    } else {
        (Vec::new(), Vec::new())
    };
    // What other objects add to this one is not part of it: link merges it again.
    match object.kind {
        ObjKind::Enum if !object.uncompiled => {
            (object.enum_values, object.enum_captions) = (keys.own_enum)(own);
        }
        _ => {}
    }
    if let Some(t) = object.table.as_mut() {
        t.extensions.clear();
    }
    if let Some(p) = object.page.as_mut() {
        p.extensions.clear();
    }
    if let Some(r) = object.report.as_mut() {
        r.extensions.clear();
    }
    let mut contribution = if object.kind == ObjKind::EnumExtension {
        let (enum_values, enum_captions) = (keys.own_enum)(own);
        Contribution {
            enum_values,
            enum_captions,
            table: None,
            page: None,
        }
    } else {
        Contribution::default()
    };
    contribution.table = table_contribution;
    contribution.page = page_contribution;
    let extends = (keys.extends)(own).map(|t| keys.objects[t].clone());
    let refs = Refs {
        expansions: expansions.iter().map(|e| e.symbolic(keys.objects)).collect(),
        ..Default::default()
    };
    let mut v = Localize {
        keys,
        own,
        refs,
        objects: FxMap::default(),
        routines: FxMap::default(),
        fields: FxMap::default(),
        bad: None,
    };
    // The expressions' own references come first: they are already indices into these lists.
    for &(table, id) in field_refs {
        let table_ix = v.object_index(table);
        v.fields.entry((table, id)).or_insert(v.refs.fields.len());
        v.refs.fields.push(FieldRef {
            table: table_ix,
            no: id,
        });
    }
    for (page, name, _) in control_refs {
        let page_ix = v.object_index(*page);
        v.refs.controls.push(ControlRef {
            page: page_ix,
            name: name.clone(),
        });
    }
    walk_object(&mut object, &mut v);
    if let Some(t) = contribution.table.as_mut() {
        al_ir::walk::walk_table_contribution(t, &mut v);
    }
    if let Some(p) = contribution.page.as_mut() {
        al_ir::walk::walk_page_contribution(p, &mut v);
        for c in &mut p.controls {
            c.provider_page = None;
        }
    }
    if let Some(b) = v.bad {
        return Err(format!("{}: {b}", keys.objects[own]));
    }
    Ok(SymObject {
        key: keys.objects[own].clone(),
        routines: Vec::new(),
        refs: v.refs,
        implements: if object_kind == ObjKind::Codeunit {
            (keys.implements)(own)
                .into_iter()
                .map(|i| keys.objects[i].clone())
                .collect()
        } else {
            Vec::new()
        },
        subscribers,
        enum_impls,
        enum_ordinals,
        extends,
        joins: joins
            .iter()
            .map(|&(list, _)| Join {
                list,
                order: (keys.join_order)(own, list),
            })
            .collect(),
        contribution,
        placeholder: false,
        control_names: Vec::new(),
        field_numbers: Vec::new(),
        object,
        positions,
        reserved_positions: 0,
    })
}

struct Localize<'a, 'k> {
    keys: &'a Keys<'k>,
    own: usize,
    refs: Refs,
    objects: FxMap<usize, usize>,
    routines: FxMap<(usize, usize), usize>,
    fields: FxMap<(usize, u32), usize>,
    bad: Option<String>,
}

impl Localize<'_, '_> {
    /// The place of object `at` (a position in the workspace) in the object's list of objects.
    fn object_index(&mut self, at: usize) -> u32 {
        let refs = &mut self.refs;
        let keys = self.keys;
        *self.objects.entry(at).or_insert_with(|| {
            refs.objects.push(keys.objects[at].clone());
            refs.objects.len() - 1
        }) as u32
    }

    /// The table object a field reference is a field of, in workspace numbering.
    fn table_object(&self, table: TableOf) -> Option<usize> {
        match table {
            // The table itself, or the table an extension adds to.
            TableOf::Own => self.keys.table_of.get(self.own).copied().flatten().or(Some(self.own)),
            TableOf::Trigger | TableOf::PageSource => self.keys.table_of.get(self.own).copied().flatten(),
            TableOf::Object(t) => Some(t),
            TableOf::SourceOf(p) => self.keys.table_of.get(p).copied().flatten(),
            TableOf::BaseItem { report, item } => (self.keys.report_item_table)(report, item),
            TableOf::Embedded | TableOf::Unknown => None,
        }
    }
}

impl RefVisitor for Localize<'_, '_> {
    fn object(&mut self, r: &mut usize) {
        let at = *r;
        let Some(key) = self.keys.objects.get(at) else {
            self.bad
                .get_or_insert_with(|| format!("object reference {at} is outside the program"));
            return;
        };
        let refs = &mut self.refs;
        *r = *self.objects.entry(at).or_insert_with(|| {
            refs.objects.push(key.clone());
            refs.objects.len() - 1
        });
    }

    fn routine(&mut self, r: &mut usize, object: usize) {
        let at = (object, *r);
        let known = object < self.keys.objects.len();
        let Some(local) = known.then(|| (self.keys.routines)(object)).and_then(|t| t.get(*r)) else {
            self.bad
                .get_or_insert_with(|| format!("routine reference {} of object {object} does not exist", at.1));
            return;
        };
        let local = local.clone();
        if let Some(&known) = self.routines.get(&at) {
            *r = known;
            return;
        }
        let obj = self.object_index(object);
        self.refs.routines.push(RoutineRef { obj, local });
        let ix = self.refs.routines.len() - 1;
        self.routines.insert(at, ix);
        *r = ix;
    }

    fn expansion(&mut self, index: usize, _: al_ir::walk::ExpansionSite<'_>) {
        if index >= self.refs.expansions.len() {
            self.bad
                .get_or_insert_with(|| format!("expansion {index} is outside the object's list"));
        }
    }

    fn control(&mut self, r: &mut usize) {
        if *r >= self.refs.controls.len() {
            self.bad
                .get_or_insert_with(|| format!("control reference {} is outside the object's list", *r));
        }
    }

    fn field(&mut self, r: &mut usize, table: TableOf) {
        if table == TableOf::Embedded {
            if *r >= self.refs.fields.len() {
                self.bad
                    .get_or_insert_with(|| format!("field reference {} is outside the object's list", *r));
            }
            return;
        }
        let Some(t) = self.table_object(table) else {
            self.bad
                .get_or_insert_with(|| format!("field position {} has no table ({})", *r, table.describe()));
            return;
        };
        let Some(no) = (self.keys.field_no)(t, *r) else {
            self.bad
                .get_or_insert_with(|| format!("field position {} is not a field of object {t}", *r));
            return;
        };
        if let Some(&known) = self.fields.get(&(t, no)) {
            *r = known;
            return;
        }
        let table = self.object_index(t);
        self.refs.fields.push(FieldRef { table, no });
        let ix = self.refs.fields.len() - 1;
        self.fields.insert((t, no), ix);
        *r = ix;
    }
}
