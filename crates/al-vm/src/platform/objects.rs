//! The tables that list the compiled objects: AllObj, AllObjWithCaption, CodeUnit Metadata and
//! Table Metadata. Their rows come from the program, which holds every object of every loaded
//! app: the ones the checker reached are compiled, the others are shells that keep their id,
//! name, namespace, owning app and the properties written in the object body (ADR-013).
//!
//! The corpus pins these in record/TestAllObjVirtualTable.al, TestAllObjOwningAppPerKind.al,
//! TestCodeunitMetadataVirtualTable.al, TestCodeunitInventoryOrder.al and
//! TestTableMetadataVirtualTable.al. AL Runner measured the rest by decompiling the platform's
//! providers (`docs/virtual-tables-allobj.md` there); where a column is not pinned by the corpus
//! the rule used is stated at the column.

use super::mat::Entry;
use super::{Blank, Providers, ids, option, pos};
use crate::Engine;
use al_builtins::value::Value;
use al_data::{Row, TableDef};
use al_ir::{Object, ObjectHead, ObjectInfo};
use std::collections::HashMap;

/// The option ordinal of the AllObj "Object Type" member for an object kind, if the kind is listed.
/// Members: TableData=0, Table=1, Report=3, Codeunit=5, XMLport=6, MenuSuite=7, Page=8, Query=9,
/// System=10, FieldNumber=11, PageExtension=14, TableExtension=15, Enum=16, EnumExtension=17,
/// Profile=18, ProfileExtension=19, PermissionSet=20, PermissionSetExtension=21,
/// ReportExtension=22 (the declaration of the AllObj table in the platform app).
pub(super) fn object_type(kind: al_ir::ObjKind) -> Option<i32> {
    Some(match kind {
        al_ir::ObjKind::Table => 1,
        al_ir::ObjKind::Report => 3,
        al_ir::ObjKind::Codeunit => 5,
        al_ir::ObjKind::XmlPort => 6,
        al_ir::ObjKind::Page => 8,
        al_ir::ObjKind::Query => 9,
        al_ir::ObjKind::PageExtension => 14,
        al_ir::ObjKind::TableExtension => 15,
        al_ir::ObjKind::Enum => 16,
        al_ir::ObjKind::EnumExtension => 17,
        al_ir::ObjKind::Profile => 18,
        al_ir::ObjKind::PermissionSet => 20,
        al_ir::ObjKind::PermissionSetExtension => 21,
        al_ir::ObjKind::ReportExtension => 22,
        _ => return None,
    })
}

/// Whether AllObjWithCaption fills "App ID" for the kind: tables, reports, xmlports, pages, queries,
/// codeunits and the extension kinds, not enums, permission sets or profiles (corpus
/// record/TestAllObjVirtualTable.al `AllObjWithCaption_FindEnum_BaseApplicationEnum_AppIdIsEmpty`
/// pins the enum; the rest is AL Runner's reading of the platform provider).
fn carries_app_id(kind: al_ir::ObjKind) -> bool {
    matches!(
        kind,
        al_ir::ObjKind::Table
            | al_ir::ObjKind::Report
            | al_ir::ObjKind::XmlPort
            | al_ir::ObjKind::Page
            | al_ir::ObjKind::Query
            | al_ir::ObjKind::Codeunit
            | al_ir::ObjKind::PageExtension
            | al_ir::ObjKind::TableExtension
            | al_ir::ObjKind::EnumExtension
            | al_ir::ObjKind::PermissionSetExtension
            | al_ir::ObjKind::ReportExtension
    )
}

/// The value of property `name` as written in the object body.
pub(super) fn prop<'a, O: ObjectInfo + ?Sized>(o: &'a O, name: &str) -> Option<&'a str> {
    o.properties()
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.trim())
}

/// `"Quoted Name"` or `Name` as the name it spells.
pub(super) fn ident(v: &str) -> String {
    let v = v.trim();
    v.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(v)
        .to_string()
}

/// The first string literal of a property value: `'It''s'` is `It's`. A value that does not start
/// with a literal has none.
pub(super) fn string_lit(v: &str) -> Option<String> {
    let rest = v.trim().strip_prefix('\'')?;
    let mut out = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                out.push('\'');
                chars.next();
            } else {
                return Some(out);
            }
        } else {
            out.push(c);
        }
    }
    None
}

pub(super) fn truthy(v: &str) -> bool {
    v.trim().eq_ignore_ascii_case("true")
}

/// The member of `members` that `text` spells, case-insensitively, as the member is written.
pub(super) fn member<'a>(members: &[&'a str], text: &str) -> Option<&'a str> {
    let t = ident(text);
    members.iter().find(|m| m.eq_ignore_ascii_case(&t)).copied()
}

const PAGE_TYPES: &[&str] = &[
    "Card",
    "List",
    "RoleCenter",
    "CardPart",
    "ListPart",
    "Document",
    "Worksheet",
    "ListPlus",
    "ConfirmationDialog",
    "NavigatePage",
    "StandardDialog",
    "API",
    "HeadlinePart",
    "XmlPort",
    "PromptDialog",
    "ReportPreview",
    "ReportProcessingOnly",
    "UserControlHost",
];
const TABLE_TYPES: &[&str] = &[
    "Normal",
    "CRM",
    "ExternalSQL",
    "Exchange",
    "MicrosoftGraph",
    "Query",
    "Temporary",
];
const QUERY_TYPES: &[&str] = &["Normal", "API"];
const CODEUNIT_SUBTYPES: &[&str] = &["Normal", "Test", "TestRunner", "Upgrade", "Install"];

/// The members of CodeUnit Metadata TestType and RequiredTestIsolation in ordinal order
/// (codeunits/procedures/XObjectProps.al:255-256, :267-268).
const TEST_TYPES: &[&str] = &["None", "UnitTest", "IntegrationTest", "Uncategorized", "AITest"];
const TEST_ISOLATIONS: &[&str] = &["None", "Disabled", "Codeunit", "Function"];

/// What the program says about each object, by name and by owner.
pub(super) struct Catalog<'a> {
    /// The heads of the objects, by object index: what a pass over all of them reads, so a run over a compile cache makes
    /// no object for it (ADR-054). A listing walks [`Catalog::installed`], not this.
    pub objects: &'a [ObjectHead],
    /// The engine, whose tenant has the apps a listing lists ([`al_vm::Engine::app_installed`]).
    engine: &'a Engine,
    /// The objects themselves, for what a head does not hold (made on first touch).
    pub full: &'a al_ir::Objects,
    pub apps: &'a [al_ir::AppInfo],
    /// The lookups by name and id, built on first use and kept on the engine (the program does not change under it, and
    /// a permission guard asks once per control of a page): a provider that reads only `objects` and `apps` (a Field row by
    /// its key) does not pay for them.
    maps: &'a std::sync::OnceLock<Maps>,
}

#[derive(Default)]
pub(crate) struct Maps {
    /// (base kind, lower-case name) to the ids of the objects of that name, each with whether the object is a stub that
    /// gives way ([`gives_way`]).
    names: HashMap<(al_ir::ObjKind, String), Vec<(u32, bool)>>,
    /// The same, with the index of the app that declares each object (for [`Catalog::id_of_from`]).
    named_in: HashMap<(al_ir::ObjKind, String), Vec<u32>>,
    /// Per app index: the app indexes it sees (itself and its dependencies, transitively).
    sees: Vec<Vec<bool>>,
    /// Table id to the first table object of that id that is not declared `ObsoleteState = Moved`, else the first.
    tables: HashMap<u32, usize>,
    /// Page id to the page extensions that extend it, in program order.
    page_extensions: HashMap<u32, Vec<usize>>,
}

impl<'a> Catalog<'a> {
    pub(super) fn new(engine: &'a Engine) -> Catalog<'a> {
        let program = &engine.program;
        Catalog {
            objects: program.objects.heads(),
            engine,
            full: &program.objects,
            apps: &program.apps,
            maps: &engine.catalog_maps,
        }
    }

    fn maps(&self) -> &Maps {
        self.maps.get_or_init(|| {
            let mut m = Maps::default();
            for (i, o) in self.objects.iter().enumerate() {
                if let (al_ir::ObjKind::PageExtension, Some(base)) = (o.kind, o.extends_id) {
                    m.page_extensions.entry(base).or_default().push(i);
                }
                if let Some(id) = o.id {
                    m.names
                        .entry((o.kind, o.name.to_lowercase()))
                        .or_default()
                        .push((id, gives_way(o)));
                    m.named_in
                        .entry((o.kind, o.name.to_lowercase()))
                        .or_default()
                        .push(o.app);
                    if o.kind == al_ir::ObjKind::Table {
                        // A moved table is not in the runtime app (metadata/platform-tables/AwTests.al:426-442): the
                        // table it moved to holds the id (al-runner issue 6716).
                        let first = m.tables.entry(id).or_insert(i);
                        if moved(&self.objects[*first]) && !moved(o) {
                            *first = i;
                        }
                    }
                }
            }
            m.sees = sees(self.apps);
            m
        })
    }

    /// The objects of the apps on the engine's tenant, in program order (ADR-073 Phase 2: an app the engine leaves
    /// uninstalled has no row in any listing).
    pub(super) fn installed(&self) -> impl Iterator<Item = &'a ObjectHead> + use<'a> {
        let engine = self.engine;
        self.objects.iter().filter(move |o| engine.app_installed(o.app))
    }

    /// [`Catalog::installed`] with each object's index.
    pub(super) fn installed_indexed(&self) -> impl Iterator<Item = (usize, &'a ObjectHead)> + use<'a> {
        let engine = self.engine;
        self.objects
            .iter()
            .enumerate()
            .filter(move |(_, o)| engine.app_installed(o.app))
    }

    /// The page extensions of page `id` on the engine's tenant, in program order (none for `None`).
    pub(super) fn page_extensions_of(&self, id: Option<u32>) -> Vec<&'a Object> {
        id.and_then(|id| self.maps().page_extensions.get(&id))
            .map_or_else(Vec::new, |v| {
                (v.iter().filter(|&&i| self.engine.object_installed(i)))
                    .map(|&i| &self.full[i])
                    .collect()
            })
    }

    /// The first table object with this id, on the engine's tenant.
    pub(super) fn table_by_id(&self, id: u32) -> Option<&'a ObjectHead> {
        (self.maps().tables.get(&id))
            .filter(|&&i| self.engine.object_installed(i))
            .map(|&i| &self.objects[i])
    }

    /// The manifest id of the app that owns `o`; empty for the platform's own objects, which no
    /// app owns (AL Runner leaves them without an owner).
    pub(super) fn owner<O: ObjectInfo + ?Sized>(&self, o: &O) -> [u8; 16] {
        let Some(app) = self.apps.get(o.app() as usize) else {
            return [0; 16];
        };
        if app.id.eq_ignore_ascii_case(al_sema_platform_app_id()) {
            return [0; 16];
        }
        al_builtins::ops::parse_guid(&app.id).unwrap_or([0; 16])
    }

    /// The id of the object of `kind` that a property value names: a number, `Page::"X"` or a name.
    pub(super) fn id_of(&self, kind: al_ir::ObjKind, value: &str) -> Result<u32, String> {
        self.id_of_from(kind, value, None)
    }

    /// [`id_of`](Self::id_of) as the code of app `from` resolves the name: only objects of that app, of the apps it depends
    /// on (transitively) and of the platform count, as the AL compiler resolves a name from the declaring app. Corpus
    /// Y94AbpActionRef.al Y94_Ref_GrantsB: D365 BASIC of the Base Application names table 8613 "Config. Package Table" although
    /// the corpus app declares a table of that name too (_fixtures/ALTRelationNameCollisionTables.al:28); BC composes the set.
    pub(super) fn id_of_from(&self, kind: al_ir::ObjKind, value: &str, from: Option<u32>) -> Result<u32, String> {
        let v = value.trim();
        let v = v.split_once("::").map_or(v, |(_, rest)| rest);
        if let Ok(n) = v.parse::<u32>() {
            return Ok(n);
        }
        let name = ident(v);
        let key = (kind, name.to_lowercase());
        let maps = self.maps();
        let all = maps.names.get(&key).map_or(&[][..], Vec::as_slice);
        let seen: Vec<(u32, bool)>;
        let found = match (from.and_then(|a| maps.sees.get(a as usize)), maps.named_in.get(&key)) {
            (Some(sees), Some(apps)) => {
                seen = all
                    .iter()
                    .zip(apps)
                    .filter(|(_, a)| sees.get(**a as usize).copied().unwrap_or(true))
                    .map(|(f, _)| *f)
                    .collect();
                // The app's code compiled, so a name it sees in none of its apps means the dependency list read here is
                // short (an app known under another id): the name is looked up in every app, as before.
                if seen.is_empty() { all } else { &seen[..] }
            }
            _ => all,
        };
        match one_id(found) {
            Some(Ok(id)) => Ok(id),
            None => Err(format!("no {kind} named `{name}`")),
            Some(Err(())) => Err(format!("more than one {kind} is named `{name}`")),
        }
    }
}

/// An object marked `ObsoleteState = Moved` or `Removed`: a stub that gives way to a live object of the same name, as
/// the compiler resolves a name (`al_sema::workspace::Workspace::pick_real`; must-not-compile
/// 0970-obsolete-removed-table.al for a removed table).
fn gives_way(o: &ObjectHead) -> bool {
    o.properties.iter().any(|(k, v)| {
        k.eq_ignore_ascii_case("ObsoleteState")
            && (v.trim().eq_ignore_ascii_case("Moved") || v.trim().eq_ignore_ascii_case("Removed"))
    })
}

/// The one id the objects of a name stand for: the stubs give way to the live objects, and objects of one id are one
/// object to a grant, which is keyed on (object type, id) (al-runner issue 6716: Base Application 29's moved table 231
/// "Reason Code" and Business Foundation's). `None` for no object, `Err` for two ids.
/// Per app index: the app indexes it sees, itself and its dependencies (transitively, by app id), and the platform's own app.
fn sees(apps: &[al_ir::AppInfo]) -> Vec<Vec<bool>> {
    let index = |id: &str| apps.iter().position(|a| a.id.eq_ignore_ascii_case(id));
    let platform = index(al_sema_platform_app_id());
    (0..apps.len())
        .map(|from| {
            let mut seen = vec![false; apps.len()];
            if let Some(p) = platform {
                seen[p] = true;
            }
            let mut todo = vec![from];
            while let Some(a) = todo.pop() {
                if std::mem::replace(&mut seen[a], true) && a != from {
                    continue;
                }
                todo.extend(
                    apps[a]
                        .dependencies
                        .iter()
                        .filter_map(|d| index(d))
                        .filter(|&d| !seen[d]),
                );
            }
            seen
        })
        .collect()
}

fn one_id(found: &[(u32, bool)]) -> Option<Result<u32, ()>> {
    let live = found.iter().any(|&(_, stub)| !stub);
    let mut ids = found.iter().filter(|&&(_, stub)| !live || !stub).map(|&(id, _)| id);
    let first = ids.next()?;
    Some(if ids.all(|id| id == first) { Ok(first) } else { Err(()) })
}

pub(crate) fn al_sema_platform_app_id() -> &'static str {
    "8874ed3a-0643-4247-9ced-7a7002f7135d"
}

/// The caption a declaration states: the `Caption` literal, else the `ENU` entry of `CaptionML`, else the
/// name (AL's default caption). A caption that is not a literal is not known.
pub(crate) fn declared_caption(caption: Option<&str>, caption_ml: Option<&str>, name: &str) -> Result<String, String> {
    if let Some(c) = caption {
        return string_lit(c).ok_or_else(|| format!("Caption `{c}` is not a literal"));
    }
    if let Some(ml) = caption_ml {
        let enu = split_outside_quotes(ml)
            .iter()
            .find_map(|part| {
                let part = part.trim();
                let (lang, text) = part.split_once('=')?;
                lang.trim().eq_ignore_ascii_case("ENU").then(|| text.trim())
            })
            .and_then(string_lit);
        return enu.ok_or_else(|| format!("CaptionML `{ml}` has no ENU literal"));
    }
    Ok(name.to_string())
}

/// The entry for language code `lang` (`ENU`, `DEU`) of an ML property such as `SummaryML = ENU = 'x', DEU = 'y'`: `Ok(None)` when it has
/// none, `Err` when the property is not a list of literals.
pub(crate) fn ml_entry(ml: &str, lang: &str) -> Result<Option<String>, String> {
    for part in split_outside_quotes(ml) {
        let Some((code, text)) = part.trim().split_once('=') else {
            return Err(format!("`{ml}` is not a list of language = text entries"));
        };
        if code.trim().eq_ignore_ascii_case(lang) {
            return string_lit(text.trim())
                .map(Some)
                .ok_or_else(|| format!("`{ml}`: the {lang} entry is not a literal"));
        }
    }
    Ok(None)
}

/// `a='x,y',b='z'` split at the commas outside single quotes.
fn split_outside_quotes(s: &str) -> Vec<&str> {
    let (mut out, mut start, mut quoted) = (Vec::new(), 0, false);
    for (i, c) in s.char_indices() {
        match c {
            '\'' => quoted = !quoted,
            ',' if !quoted => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

pub(super) fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// The object caption: the `Caption` property's literal, else the object name. A tableextension's row names it by its name:
/// its Caption is the caption of the table it extends (table-objects/extensions/TestTableExtFieldModify.al:174-179).
pub(super) fn caption<O: ObjectInfo + ?Sized>(o: &O) -> String {
    if o.kind() == al_ir::ObjKind::TableExtension {
        return o.name().to_string();
    }
    prop(o, "Caption")
        .and_then(string_lit)
        .unwrap_or_else(|| o.name().to_string())
}

/// [`caption`] as a column value, cut to `max` characters: unknown where a tableextension sets the table's caption over another
/// (`al_ir::CAPTION_REFUSED`).
fn caption_value<O: ObjectInfo + ?Sized>(o: &O, max: usize) -> Value {
    match o.properties().iter().find(|(k, _)| k == al_ir::CAPTION_REFUSED) {
        Some((_, why)) => Value::Unknown(why.clone().into()),
        None => Value::Text(truncate(&caption(o), max).into()),
    }
}

/// `is_platform`, for the Field table's package columns.
pub(super) fn platform_object<O: ObjectInfo + ?Sized>(o: &O) -> bool {
    is_platform(o)
}

/// The App Package ID and App Runtime Package ID AllObj reports for `o`.
pub(super) fn package_ids<O: ObjectInfo + ?Sized>(c: &Catalog, o: &O) -> ([u8; 16], [u8; 16]) {
    (package_of(c, o), runtime_package_id(c, o))
}

fn namespace(o: &ObjectHead) -> String {
    o.namespace.clone().unwrap_or_default()
}

// ---- AllObj and AllObjWithCaption -----------------------------------------------------------------

/// The text of "Object Subtype" for an object (AllObjWithCaption).
fn subtype_text(c: &Catalog, o: &ObjectHead) -> Result<String, String> {
    let kind = o.kind;
    let unknown = |what: &str, v: &str| format!("{kind} '{}': {what} `{v}` is not understood", o.name);
    Ok(match kind {
        al_ir::ObjKind::Table => match prop(o, "TableType") {
            None => "Normal".into(),
            Some(v) => member(TABLE_TYPES, v).ok_or_else(|| unknown("TableType", v))?.into(),
        },
        al_ir::ObjKind::Page => match prop(o, "PageType") {
            None => "Card".into(),
            Some(v) => member(PAGE_TYPES, v).ok_or_else(|| unknown("PageType", v))?.into(),
        },
        al_ir::ObjKind::Query => match prop(o, "QueryType") {
            None => "Normal".into(),
            Some(v) => member(QUERY_TYPES, v).ok_or_else(|| unknown("QueryType", v))?.into(),
        },
        // A codeunit's Normal subtype is the empty string, and so is Install, which the compiler
        // writes into the metadata as Normal (TestAllObjVirtualTable.al, the codeunit and install cases).
        al_ir::ObjKind::Codeunit => match prop(o, "Subtype") {
            None => String::new(),
            Some(v) => match member(CODEUNIT_SUBTYPES, v).ok_or_else(|| unknown("Subtype", v))? {
                "Normal" | "Install" => String::new(),
                other => other.into(),
            },
        },
        // The id of the object the extension extends, as a decimal string.
        al_ir::ObjKind::PageExtension
        | al_ir::ObjKind::TableExtension
        | al_ir::ObjKind::EnumExtension
        | al_ir::ObjKind::PermissionSetExtension
        | al_ir::ObjKind::ReportExtension => {
            let _ = c;
            o.extends_id.map(|id| id.to_string()).unwrap_or_default()
        }
        _ => String::new(),
    })
}

/// Builds one `Entry` per object: its row, or a poisoned entry (key only) with the reason.
pub(super) fn entries(
    c: &Catalog,
    t: &TableDef,
    wanted: impl Fn(&ObjectHead) -> Option<Vec<(u32, Value)>>,
    build: impl Fn(&ObjectHead, &mut Row) -> Result<(), String>,
) -> Vec<Entry> {
    let mut out = Vec::new();
    let blank = Blank::new(t);
    for o in c.installed() {
        let Some(key) = wanted(o) else { continue };
        let mut r = blank.row();
        for (id, v) in &key {
            r[pos(t, *id)] = v.clone();
        }
        match build(o, &mut r) {
            Ok(()) => out.push(Entry::ok(r)),
            Err(why) => {
                let mut k = blank.row();
                for (id, v) in key {
                    k[pos(t, id)] = v;
                }
                out.push(Entry::poisoned(k, why));
            }
        }
    }
    out
}

/// A platform object (a system table): it has a package id, shared by all of them, and no runtime package id.
fn is_platform<O: ObjectInfo + ?Sized>(o: &O) -> bool {
    o.id().is_some_and(|id| id >= 2_000_000_000)
}

/// The package id AllObj reports for `o`: the owning app's, or the platform's own for a platform object.
fn package_of<O: ObjectInfo + ?Sized>(c: &Catalog, o: &O) -> [u8; 16] {
    if is_platform(o) {
        ids::package_id([0x70; 16])
    } else {
        ids::package_id(c.owner(o))
    }
}

/// "App Runtime Package ID": the package id for an object of the app under test, a different one for an object of a
/// dependency app (Base Application), and none for a platform object (metadata/W6AllObj.al:
/// AllObj_PackageIds_ThisApp_BaseApp_Platform, BC 27.5 and 29.0).
fn runtime_package_id<O: ObjectInfo + ?Sized>(c: &Catalog, o: &O) -> [u8; 16] {
    if is_platform(o) {
        return [0; 16];
    }
    let dependency = c.apps.get(o.app() as usize).is_some_and(|a| a.dependency);
    ids::runtime_package_id(c.owner(o), dependency)
}

/// The codeunits of Microsoft's Test Runner app that BC's test environment has (BaTpEnv.al:28).
const TEST_RUNNER_CODEUNITS: std::ops::RangeInclusive<u32> = 130_450..=130_455;

/// A table declared `ObsoleteState = Moved`.
fn moved(o: &ObjectHead) -> bool {
    o.kind == al_ir::ObjKind::Table && al_ir::ObjectFacts::of_properties(&o.properties).moved
}

pub(super) fn all_obj_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = entries(
        &c,
        t,
        |o| {
            let (Some(id), Some(ty)) = (o.id, object_type(o.kind)) else {
                return None;
            };
            // A table declared ObsoleteState = Moved is not in the runtime app and has no row
            // (metadata/platform-tables/AwTests.al:413-423).
            if moved(o) {
                return None;
            }
            Some(vec![(1, option(t, 1, ty)), (3, Value::Integer(id as i32))])
        },
        |o, r| {
            let pkg = package_of(&c, o);
            r[pos(t, 4)] = Value::Text(o.name.clone().into());
            r[pos(t, 60)] = Value::Guid(pkg);
            r[pos(t, 61)] = Value::Guid(runtime_package_id(&c, o));
            r[pos(t, 62)] = Value::Text(namespace(o).into());
            Ok(())
        },
    );
    // BC's test environment has the codeunits of Microsoft's Test Runner app, 130450-130455 (permissions/
    // lt-ba-testpermissions/BaTpEnv.al:26-28). Every run loads that app from source (ADR-013, "The test environment's
    // apps"), and its objects are rows like any other app's. A run that found no package for it (offline, no feed) has
    // no such codeunit: a read that may land on one of them is refused.
    for id in TEST_RUNNER_CODEUNITS {
        let present = c
            .objects
            .iter()
            .any(|o| o.id == Some(id) && object_type(o.kind) == Some(5));
        if !present {
            let mut k = Blank::new(t).row();
            k[pos(t, 1)] = option(t, 1, 5);
            k[pos(t, 3)] = Value::Integer(id as i32);
            out.push(Entry::poisoned(
                k,
                format!(
                    "AllObj row of codeunit {id}: BC's test environment has the Test Runner app's codeunits 130450-130455, and this run found no package of Microsoft's Test Runner app"
                ),
            ));
        }
    }
    // Every table has a TableData row (ordinal 0) next to its Table row, with the Table row's name, namespace, package id and
    // runtime package id: a table of the app under test, a temporary one, a dependency's (Base Application's Customer and
    // Contact) and the platform's (Company, the virtual Integer and AllObj) (metadata/session-system/AllObjNamespace.al:
    // 143-218 PW_AllObj_*, :220-262 UG_AllObj_TableDataRow_OfContact_AllColumns). A table declared Moved has no Table row, and
    // so none here.
    for o in c.installed() {
        if let (Some(id), al_ir::ObjKind::Table) = (o.id, o.kind)
            && !moved(o)
        {
            let mut k = Blank::new(t).row();
            k[pos(t, 1)] = option(t, 1, 0);
            k[pos(t, 3)] = Value::Integer(id as i32);
            k[pos(t, 4)] = Value::Text(o.name.clone().into());
            k[pos(t, 60)] = Value::Guid(package_of(&c, o));
            k[pos(t, 61)] = Value::Guid(runtime_package_id(&c, o));
            k[pos(t, 62)] = Value::Text(namespace(o).into());
            out.push(Entry::ok(k));
        }
    }
    out
}

pub(super) fn all_obj_with_caption_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let key = |o: &ObjectHead, ty: i32| Some(vec![(1, option(t, 1, ty)), (3, Value::Integer(o.id? as i32))]);
    let mut out = entries(&c, t, |o| key(o, object_type(o.kind)?), |o, r| caption_row(&c, t, o, r));
    // Every table has a TableData row next to its Table row, with its name, caption and subtype (metadata/session-system/
    // AllObjNamespace.al:156-218 PW_AllObj_TableDataRowPerTableKind; al-runner issue 6643). Its package ids, app id and namespace
    // are taken to be the Table row's, as AllObj's TableData row has them (:220-262): assumption
    // allobjwithcaption-tabledata-columns until corpus writer Y64's probe lands. A table declared Moved has none.
    let tables = |o: &ObjectHead| {
        (o.kind == al_ir::ObjKind::Table && !moved(o))
            .then(|| key(o, 0))
            .flatten()
    };
    let table_data = entries(&c, t, tables, |o, r| caption_row(&c, t, o, r));
    if !table_data.is_empty() {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::AllobjwithcaptionTabledataColumns);
    }
    out.extend(table_data);
    out
}

/// The columns of the AllObjWithCaption row of object `o` after its key.
fn caption_row(c: &Catalog, t: &TableDef, o: &ObjectHead, r: &mut Row) -> Result<(), String> {
    let owner = c.owner(o);
    r[pos(t, 4)] = Value::Text(o.name.clone().into());
    // "Name" (5, BC 29 on, Text[100]): the object name, as Metadata Permission's "AL Object Name" looks it up
    // (permissions/permission-sets/Y59MetadataPermissionNames.al:257-266, corpus master 9cf26907).
    if let Some(p) = t.fields.iter().position(|f| f.id == 5) {
        r[p] = Value::Text(o.name.chars().take(100).collect());
    }
    r[pos(t, 20)] = caption_value(o, 249);
    // A subtype the runtime has no pinned text for (a CDS table) leaves only that column unknown: the other
    // columns still answer a filter or a count (metadata/CmInstallObjects.al:73-79).
    r[pos(t, 30)] = match subtype_text(c, o) {
        Ok(s) => Value::Text(s.into()),
        Err(why) => Value::Unknown(format!("AllObjWithCaption: {why}").into()),
    };
    r[pos(t, 60)] = Value::Guid(package_of(c, o));
    r[pos(t, 61)] = Value::Guid(runtime_package_id(c, o));
    r[pos(t, 62)] = Value::Guid(if carries_app_id(o.kind) { owner } else { [0; 16] });
    r[pos(t, 63)] = Value::Text(namespace(o).into());
    Ok(())
}

// ---- Application Object Metadata -------------------------------------------------------------------

/// One row per object, keyed as AllObj names it (Runtime Package ID, Object Type, Object ID) and Emit Version 0: Microsoft's
/// ODataUtility.GetTenantWebServiceMetadata finds a query's row with `Get(AllObj."App Runtime Package ID", type, id)`, the
/// missing Emit Version taking its blank value (tests/al-language-onprem-dotnet/dotnet/dotnet/QxDotNetDemoShapes.al:173-181,
/// dotnet/dotnet/ReQueryMetadataShapes.al:116-140: the reader it builds is not null). Only a query's Metadata is pinned, and
/// only as the input of QueryMetadataReader.FromStream: the Blob holds the query's id under the mark
/// `stream::QUERY_METADATA`, which refuses any other look. The other columns are unknown (`mark_unpinned`), and the row of
/// any other object type is refused: nothing pins what it holds.
pub(super) fn application_object_metadata_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    entries(
        &c,
        t,
        |o| {
            let (Some(id), Some(ty)) = (o.id, object_type(o.kind)) else {
                return None;
            };
            // Profiles are not a member of this table's Object Type; a moved table has no AllObj row either.
            if ty == 18 || moved(o) {
                return None;
            }
            Some(vec![
                (1, Value::Guid(runtime_package_id(&c, o))),
                (3, option(t, 3, ty)),
                (4, Value::Integer(id as i32)),
            ])
        },
        |o, r| {
            if o.kind != al_ir::ObjKind::Query {
                return Err(format!(
                    "the Application Object Metadata row of {} '{}' (only a query's Metadata is pinned)",
                    o.kind, o.name
                ));
            }
            let blob = al_builtins::stream::new_blob();
            al_builtins::stream::write_bytes(&blob, &o.id.unwrap_or(0).to_le_bytes());
            blob.borrow_mut().render_pending = Some(al_builtins::stream::QUERY_METADATA.into());
            r[pos(t, 6)] = Value::Blob(blob);
            Ok(())
        },
    )
}

// ---- CodeUnit Metadata ----------------------------------------------------------------------------

pub(super) fn codeunit_metadata_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    entries(
        &c,
        t,
        |o| match (o.id, o.kind) {
            (Some(id), al_ir::ObjKind::Codeunit) => Some(vec![(1, Value::Integer(id as i32))]),
            _ => None,
        },
        |o, r| {
            let subtype = match prop(o, "Subtype") {
                None => "Normal",
                Some(v) => member(CODEUNIT_SUBTYPES, v)
                    .ok_or_else(|| format!("codeunit '{}': Subtype `{v}` is not understood", o.name))?,
            };
            // Install does not reach the metadata: the compiler writes Normal (TestCodeunitMetadataVirtualTable.al,
            // `InstallCodeunit_ReportsSubtypeNormal`, `QuotedSubtypeInstall_ReportsSubtypeNormal`).
            let ordinal = match subtype {
                "Test" => 1,
                "TestRunner" => 2,
                "Upgrade" => 3,
                _ => 0,
            };
            r[pos(t, 2)] = Value::Text(o.name.clone().into());
            r[pos(t, 3)] = Value::Integer(table_no(&c, o)? as i32);
            r[pos(t, 4)] = Value::Boolean(prop(o, "SingleInstance").is_some_and(truthy));
            r[pos(t, 5)] = option(t, 5, ordinal);
            r[pos(t, 6)] = Value::Guid(c.owner(o));
            r[pos(t, 7)] = Value::Text(truncate(&ident(prop(o, "InherentPermissions").unwrap_or("")), 5).into());
            r[pos(t, 8)] = Value::Text(truncate(&ident(prop(o, "InherentEntitlements").unwrap_or("")), 5).into());
            // TestType and RequiredTestIsolation read the declared member; a test codeunit without them reads
            // UnitTest and None (codeunits/procedures/XObjectProps.al:255-256, :267-268, :279), any other
            // codeunit None and None (record/TestCodeunitMetadataVirtualTable.al:415-419, :456-459). A non-test
            // codeunit that declares either is not pinned: refused.
            let declared = |name: &str, members: &[&str]| -> Result<Option<i32>, String> {
                match prop(o, name) {
                    None => Ok(None),
                    Some(_) if subtype != "Test" => Err(format!(
                        "codeunit '{}': {name} on a codeunit that is not Subtype = Test is not pinned",
                        o.name
                    )),
                    Some(v) => members
                        .iter()
                        .position(|m| m.eq_ignore_ascii_case(&ident(v)))
                        .map(|i| Some(i as i32))
                        .ok_or_else(|| format!("codeunit '{}': {name} `{v}` is not understood", o.name)),
                }
            };
            let test_type = declared("TestType", TEST_TYPES)?.unwrap_or(i32::from(subtype == "Test"));
            r[pos(t, 9)] = option(t, 9, test_type);
            let isolation = declared("RequiredTestIsolation", TEST_ISOLATIONS)?.unwrap_or(0);
            r[pos(t, 10)] = option(t, 10, isolation);
            r[pos(t, 11)] = Value::Text(namespace(o).into());
            Ok(())
        },
    )
}

/// The id of the table a codeunit's `TableNo` names; 0 for none.
fn table_no(c: &Catalog, o: &ObjectHead) -> Result<u32, String> {
    // A compiled codeunit has `Rec`: a global of the table's record type.
    if let Some(t) = o.rec_table {
        return Ok(c.objects[t as usize].id.unwrap_or(0));
    }
    match prop(o, "TableNo") {
        None => Ok(0),
        Some(v) => c
            .id_of(al_ir::ObjKind::Table, v)
            .map_err(|why| format!("codeunit '{}': TableNo {why}", o.name)),
    }
}

// ---- Table Metadata -------------------------------------------------------------------------------

const DATA_CLASSIFICATIONS: &[&str] = &[
    "CustomerContent",
    "ToBeClassified",
    "EndUserIdentifiableInformation",
    "AccountData",
    "EndUserPseudonymousIdentifiers",
    "OrganizationIdentifiableInformation",
    "SystemMetadata",
];
const OBSOLETE_STATES: &[&str] = &["No", "Pending", "Removed"];
const COMPRESSION_TYPES: &[&str] = &["Unspecified", "None", "Row", "Page"];
const SCOPES: &[&str] = &["Cloud", "OnPrem"];
const ACCESS: &[&str] = &["Public", "Internal"];

fn ordinal_of(members: &[&str], text: &str) -> Option<i32> {
    members
        .iter()
        .position(|m| m.eq_ignore_ascii_case(&ident(text)))
        .map(|i| i as i32)
}

/// The ids of the fields a table's `DataCaptionFields` names, comma-joined (empty without the property).
fn data_caption_field_ids(o: &ObjectHead) -> Result<String, String> {
    let Some(v) = prop(o, "DataCaptionFields") else {
        return Ok(String::new());
    };
    let fields = o
        .table
        .as_ref()
        .map(|t| &t.fields)
        .ok_or_else(|| format!("table '{}' has no fields", o.name))?;
    let mut ids = Vec::new();
    for name in v.split(',') {
        let name = ident(name);
        let f = fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| format!("table '{}': DataCaptionFields names `{name}`, not a field", o.name))?;
        ids.push(f.id.to_string());
    }
    Ok(ids.join(","))
}

pub(super) fn table_metadata_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    entries(
        &c,
        t,
        |o| match (o.id, o.kind) {
            (Some(id), al_ir::ObjKind::Table) => Some(vec![(1, Value::Integer(id as i32))]),
            _ => None,
        },
        |o, r| {
            let bad = |name: &str, v: &str| format!("table '{}': {name} `{v}` is not understood", o.name);
            let opt = |name: &str, members: &[&str], default: i32| -> Result<i32, String> {
                match prop(o, name) {
                    None => Ok(default),
                    Some(v) => ordinal_of(members, v).ok_or_else(|| bad(name, v)),
                }
            };
            let page = |name: &str| -> Result<i32, String> {
                match prop(o, name) {
                    None => Ok(0),
                    Some(v) => c
                        .id_of(al_ir::ObjKind::Page, v)
                        .map(|i| i as i32)
                        .map_err(|why| format!("table '{}': {name} {why}", o.name)),
                }
            };
            r[pos(t, 2)] = Value::Text(o.name.clone().into());
            // Caption is a Text[80] field that holds a 120-character caption whole (metadata/session-system/CyLongCaption.al:
            // 47-56); a longer one is not pinned.
            r[pos(t, 3)] = if caption(o).chars().count() > 120 {
                Value::Unknown(
                    format!(
                        "Table Metadata.Caption of table '{}', longer than 120 characters (not pinned)",
                        o.name
                    )
                    .into(),
                )
            } else {
                caption_value(o, 120)
            };
            // DataPerCompany defaults to true (TestTableMetadataVirtualTable.al, `DeclaredTable_ReturnsMatchingRow`).
            r[pos(t, 4)] = Value::Boolean(prop(o, "DataPerCompany").is_none_or(truthy));
            r[pos(t, 5)] = Value::Integer(page("LookupPageId")?);
            r[pos(t, 6)] = Value::Integer(page("DrillDownPageId")?);
            // DataCaptionFields: the ids of the fields it names, comma-joined; a table extension's counts (link merges it,
            // record/table-properties/XTablePropsTests.al:578, :586 `1,4`).
            r[pos(t, 7)] = Value::Text(data_caption_field_ids(o)?.into());
            // LinkedObject (XTablePropsTests.al:579-591: `Yes` on a table that declares it, `No` without).
            r[pos(t, 9)] = Value::Boolean(prop(o, "LinkedObject").is_some_and(truthy));
            // PasteIsValid defaults to true in AL; no corpus test reads it.
            r[pos(t, 8)] = Value::Boolean(prop(o, "PasteIsValid").is_none_or(truthy));
            r[pos(t, 11)] = option(t, 11, opt("TableType", TABLE_TYPES, 0)?);
            r[pos(t, 12)] =
                Value::Text(truncate(&prop(o, "ExternalName").and_then(string_lit).unwrap_or_default(), 248).into());
            r[pos(t, 13)] = option(t, 13, opt("ObsoleteState", OBSOLETE_STATES, 0)?);
            r[pos(t, 14)] =
                Value::Text(truncate(&prop(o, "ObsoleteReason").and_then(string_lit).unwrap_or_default(), 248).into());
            // No DataClassification reads CustomerContent (same file, `TableDeclaringNoDataClassification_ReportsTheDefault`).
            r[pos(t, 15)] = option(t, 15, opt("DataClassification", DATA_CLASSIFICATIONS, 0)?);
            r[pos(t, 16)] = Value::Boolean(prop(o, "ReplicateData").is_none_or(truthy));
            r[pos(t, 17)] = option(t, 17, opt("CompressionType", COMPRESSION_TYPES, 0)?);
            r[pos(t, 18)] = Value::Guid(c.owner(o));
            // InherentPermissions: `X` as declared, blank when not (permissions/W6PermissionColumns.al
            // Y17_TableMetadata_InherentPermissions, corpus master 97c29f86); another value is not pinned.
            r[pos(t, 19)] = match ident(prop(o, "InherentPermissions").unwrap_or("")) {
                v if v.is_empty() || v == "X" => Value::Text(v.into()),
                v => super::unknown(
                    t,
                    19,
                    &format!("table '{}', which declares InherentPermissions = {v}", o.name),
                ),
            };
            r[pos(t, 20)] = Value::Text(truncate(&ident(prop(o, "InherentEntitlements").unwrap_or("")), 5).into());
            r[pos(t, 21)] = option(t, 21, opt("Scope", SCOPES, 0)?);
            r[pos(t, 22)] = option(t, 22, opt("Access", ACCESS, 0)?);
            r[pos(t, 23)] = Value::Text(namespace(o).into());
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two catalogs of one engine share one set of lookups: a page with guarded controls asks for a guard's object
    /// id once per control, and building the lookups walks every object of the program.
    #[test]
    fn catalogs_of_one_engine_share_their_lookups() {
        let engine = crate::Engine::new(al_ir::Program::default(), Vec::new());
        let (a, b) = (Catalog::new(&engine), Catalog::new(&engine));
        assert!(std::ptr::eq(a.maps(), b.maps()));
        let sibling = engine.sibling();
        assert!(std::ptr::eq(a.maps(), Catalog::new(&sibling).maps()));
    }
}
