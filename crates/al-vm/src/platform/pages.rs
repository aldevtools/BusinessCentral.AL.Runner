//! The page tables: Page Metadata and Page Control Field.
//!
//! Pinned by corpus record/TestPageMetadataVirtualTable.al, TestPageMetadataProperties.al,
//! TestPageMetadataSourceObject.al, TestPageControlFieldVirtualTable.al,
//! TestPageControlFieldDefaults.al, TestPageControlFieldControlTree.al,
//! TestPageControlFieldPageExtension.al and pageextensioncontrol/
//! TestPageExtensionControlPrecompiledRows_Tests.al.

use super::mat::Entry;
use super::objects::{Catalog, ident, member, prop, string_lit, truthy};
use super::{Blank, Providers, option, pos};
use al_builtins::value::{RtType, Value};
use al_data::{Row, TableDef};
use al_ir::{Binding, ControlKind, Object, PropVal};

/// The Page Metadata "PageType" ordinals (the column names 0 to 12; PromptDialog is 20,
/// record/TestPageMetadataVirtualTable.al:33-60). Other page types are not pinned.
const PAGE_TYPES: &[(&str, i32)] = &[
    ("Card", 0),
    ("List", 1),
    ("RoleCenter", 2),
    ("CardPart", 3),
    ("ListPart", 4),
    ("Document", 5),
    ("Worksheet", 6),
    ("ListPlus", 7),
    ("ConfirmationDialog", 8),
    ("NavigatePage", 9),
    ("StandardDialog", 10),
    ("API", 11),
    ("HeadlinePart", 12),
    ("PromptDialog", 20),
];

/// The table a page shows: the table of its `Rec`.
fn source_table<'a>(c: &'a Catalog, o: &Object) -> Option<&'a Object> {
    let slot = o.rec_slot?;
    match o.globals.get(slot).map(|g| &g.ty) {
        Some(RtType::Record(t, _)) => Some(&c.objects[*t as usize]),
        _ => None,
    }
}

/// The id of the source table of a page: from its `Rec` when it compiled, else from the name its
/// `SourceTable` property gives. 0 when it declares none.
fn source_table_id(c: &Catalog, o: &Object) -> Result<u32, String> {
    if let Some(t) = source_table(c, o) {
        return Ok(t.id.unwrap_or(0));
    }
    match prop(o, "SourceTable") {
        None => Ok(0),
        Some(v) => c
            .id_of(al_ir::ObjKind::Table, v)
            .map_err(|why| format!("page '{}': SourceTable {why}", o.name)),
    }
}

fn lit_bool(o: &Object, name: &str, default: bool) -> bool {
    prop(o, name).map_or(default, truthy)
}

fn text_prop(o: &Object, name: &str, max: usize) -> String {
    prop(o, name)
        .and_then(string_lit)
        .unwrap_or_default()
        .chars()
        .take(max)
        .collect()
}

/// A source table view as BC formats it: `SORTING(Field2,Field1) ORDER(1) WHERE(Field2=CONST(1),Field3=FILTER(1..5))`.
/// Pinned shapes (misc/W2PageMetadata.al:384 SourceTableView_Shapes, :461 RP_LookupPageSortedBySecondaryKey_CardPageId,
/// TestPageMetadataSourceObject.al:61-80): `sorting` over one or more fields and `order(descending)`, each followed by a
/// space; `where` with one or more conditions, each `const` of an integer, option, enum or Text value or `filter` of an
/// integer range `a .. b`. Anything else (`order(ascending)`, a const on another type, another filter) is not pinned.
fn view_text(o: &Object, table: Option<&Object>, table_objects: &[Object]) -> Result<String, String> {
    let Some(v) = prop(o, "SourceTableView") else {
        return Ok(String::new());
    };
    let not_pinned = || {
        format!(
            "page '{}': SourceTableView `{v}` (only sorting, order(descending) and where with const or an integer range filter are pinned)",
            o.name
        )
    };
    let table = table.ok_or_else(not_pinned)?;
    let fields = &table.table.as_ref().ok_or_else(not_pinned)?.fields;
    let field = |name: &str| {
        let name = ident(name);
        fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(&name))
            .ok_or_else(not_pinned)
    };
    let clauses = view_clauses(v).ok_or_else(not_pinned)?;
    let (mut sorting, mut order, mut filters) = (None, None, None);
    for (name, body) in clauses {
        let slot = match name.to_ascii_lowercase().as_str() {
            "sorting" => &mut sorting,
            "order" => &mut order,
            "where" => &mut filters,
            _ => return Err(not_pinned()),
        };
        if slot.replace(body).is_some() {
            return Err(not_pinned());
        }
    }
    let mut out = String::new();
    if let Some(body) = sorting {
        let ids = split_top(body)
            .ok_or_else(not_pinned)?
            .iter()
            .map(|n| field(n).map(|f| format!("Field{}", f.id)))
            .collect::<Result<Vec<_>, _>>()?;
        out.push_str(&format!("SORTING({}) ", ids.join(",")));
    }
    if let Some(body) = order {
        if !body.trim().eq_ignore_ascii_case("descending") {
            return Err(not_pinned());
        }
        out.push_str("ORDER(1) ");
    }
    if let Some(body) = filters {
        let mut conds = Vec::new();
        for cond in split_top(body).ok_or_else(not_pinned)? {
            let (name, rest) = cond.split_once('=').ok_or_else(not_pinned)?;
            let f = field(name)?;
            let rest = rest.trim();
            if let Some(value) = rest.strip_prefix_ci("const(").and_then(|r| r.strip_suffix(')')) {
                conds.push(format!(
                    "Field{}=CONST({})",
                    f.id,
                    const_text(f, value.trim(), table_objects).ok_or_else(not_pinned)?
                ));
            } else if let Some(value) = rest.strip_prefix_ci("filter(").and_then(|r| r.strip_suffix(')')) {
                // An integer range, written `1 .. 5`, reads `1..5` (W2PageMetadata.al:395 E_filter_range).
                let (a, b) = value.split_once("..").ok_or_else(not_pinned)?;
                let (a, b) = (a.trim(), b.trim());
                if !matches!(f.ty, RtType::Integer) || a.parse::<i32>().is_err() || b.parse::<i32>().is_err() {
                    return Err(not_pinned());
                }
                conds.push(format!("Field{}=FILTER({a}..{b})", f.id));
            } else {
                return Err(not_pinned());
            }
        }
        out.push_str(&format!("WHERE({})", conds.join(",")));
    }
    Ok(out)
}

/// The value of a `const(...)` as the view text shows it: an option or enum member by its ordinal
/// (TestPageMetadataSourceObject.al:61-80 declares `Status = const(Active)` and expects `CONST(2)`), a number as written,
/// a Text value without its quotes (W2PageMetadata.al:394 `CONST(x)`, :397 `CONST(a b)`).
fn const_text(f: &al_ir::Field, value: &str, table_objects: &[Object]) -> Option<String> {
    let plain = |v: &str| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    match &f.ty {
        RtType::Text(_) => {
            let text = string_lit(value)?;
            (!text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ')).then_some(text)
        }
        RtType::Option(m) if plain(value) => m
            .iter()
            .position(|x| x.eq_ignore_ascii_case(value))
            .map(|p| p.to_string()),
        RtType::Enum(e) if plain(value) => objects_enum_ordinal(table_objects, *e, value).map(|p| p.to_string()),
        RtType::Integer | RtType::BigInteger if value.parse::<i64>().is_ok() => Some(value.to_string()),
        _ => None,
    }
}

/// The clauses of a view, `name(body)` separated by white space, with their bodies. `None` when it does not read so.
fn view_clauses(v: &str) -> Option<Vec<(&str, &str)>> {
    let mut out = Vec::new();
    let mut rest = v.trim();
    while !rest.is_empty() {
        let open = rest.find('(')?;
        let name = rest[..open].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()) {
            return None;
        }
        let close = matching_paren(rest, open)?;
        out.push((name, &rest[open + 1..close]));
        rest = rest[close + 1..].trim_start();
    }
    Some(out)
}

/// The index of the `)` that closes the `(` at `open`, outside quotes.
fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let (mut depth, mut quote) = (0usize, None::<char>);
    for (i, c) in s.char_indices().skip_while(|(i, _)| *i < open) {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// `body` split at its top-level commas (outside parentheses and quotes), each part trimmed. `None` for an empty part.
fn split_top(body: &str) -> Option<Vec<&str>> {
    let (mut parts, mut start, mut depth, mut quote) = (Vec::new(), 0, 0usize, None::<char>);
    for (i, c) in body.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') => depth = depth.checked_sub(1)?,
            (None, ',') if depth == 0 => {
                parts.push(body[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(body[start..].trim());
    (!parts.iter().any(|p| p.is_empty())).then_some(parts)
}

fn objects_enum_ordinal(objects: &[Object], enum_object: u32, member: &str) -> Option<i32> {
    objects[enum_object as usize]
        .enum_values
        .iter()
        .find(|(_, n)| n.eq_ignore_ascii_case(member))
        .map(|(v, _)| *v)
}

trait StripCi {
    fn strip_prefix_ci(&self, p: &str) -> Option<&str>;
}
impl StripCi for str {
    fn strip_prefix_ci(&self, p: &str) -> Option<&str> {
        (self.len() >= p.len() && self.is_char_boundary(p.len()) && self[..p.len()].eq_ignore_ascii_case(p))
            .then(|| &self[p.len()..])
    }
}

/// The ids of the fields `DataCaptionFields` names, comma-joined.
fn caption_fields(o: &Object, table: Option<&Object>) -> Result<String, String> {
    let Some(v) = prop(o, "DataCaptionFields") else {
        return Ok(String::new());
    };
    let fields = table.and_then(|t| t.table.as_ref()).map(|t| &t.fields).ok_or_else(|| {
        format!(
            "page '{}': DataCaptionFields need the source table, which was not compiled",
            o.name
        )
    })?;
    let mut ids = Vec::new();
    for name in v.split(',') {
        let name = ident(name);
        let f = fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(&name))
            .ok_or_else(|| {
                format!(
                    "page '{}': DataCaptionFields names `{name}`, not a field of the source table",
                    o.name
                )
            })?;
        ids.push(f.id.to_string());
    }
    Ok(ids.join(","))
}

pub(super) fn page_metadata_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    let blank = Blank::new(t);
    for o in c.objects {
        let (Some(id), al_ir::ObjKind::Page) = (o.id, o.kind) else {
            continue;
        };
        let mut key = blank.row();
        key[pos(t, 1)] = Value::Integer(id as i32);
        match page_metadata_row(&c, o, t, key.clone()) {
            Ok(r) => out.push(Entry::ok(r)),
            Err(why) => out.push(Entry::poisoned(key, why)),
        }
    }
    out
}

/// A property of a page as the page and its extensions leave it: an extension's value wins over the
/// base page's (corpus pageext/lt-z-page-ext-props/ZPageExtProps.al:629, 685-1110). Two extensions that set
/// the same property are not pinned.
fn effective_prop<'a>(c: &'a Catalog, o: &'a Object, name: &str) -> Result<Option<&'a str>, String> {
    let own = o.id.filter(|_| o.kind == al_ir::ObjKind::Page);
    // From the catalog's index: this runs four times for every page of the program when Page Metadata is first read.
    let from_extensions: Vec<&str> = (c.page_extensions_of(own).iter())
        .filter_map(|e| prop(e, name))
        .collect();
    match from_extensions.as_slice() {
        [] => Ok(prop(o, name)),
        [one] => Ok(Some(one)),
        _ => Err(format!(
            "page '{}': {} page extensions set {name} (which one wins is not pinned)",
            o.name,
            from_extensions.len()
        )),
    }
}

/// InsertAllowed, ModifyAllowed or DeleteAllowed as Page Metadata reports it: true unless the page or its
/// extension declares false (ZPageExtProps.al:685-1110). A value that is not a literal is not pinned.
fn allowed(c: &Catalog, o: &Object, name: &str) -> Result<bool, String> {
    match effective_prop(c, o, name)?.map(str::to_ascii_lowercase).as_deref() {
        None | Some("true") => Ok(true),
        Some("false") => Ok(false),
        Some(v) => Err(format!(
            "page '{}': {name} `{v}` is not a literal true or false (what the column reports is not pinned)",
            o.name
        )),
    }
}

fn page_metadata_row(c: &Catalog, o: &Object, t: &TableDef, mut r: Row) -> Result<Row, String> {
    let page_type = match prop(o, "PageType") {
        None => 0,
        Some(v) => {
            let name = member(&PAGE_TYPES.iter().map(|(n, _)| *n).collect::<Vec<_>>(), v)
                .ok_or_else(|| format!("page '{}': PageType `{v}` is not pinned", o.name))?;
            PAGE_TYPES
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, i)| *i)
                .unwrap_or(0)
        }
    };
    let src_id = source_table_id(c, o)?;
    let src = c.table_by_id(src_id).filter(|_| src_id != 0);
    // A page with no source table has no source object: its flags are all false and its texts empty
    // (AL Runner, measured on BC: docs of RecordPatches.PageMetadataSourceObject).
    let has_source = src_id != 0;
    r[pos(t, 2)] = Value::Text(o.name.clone().into());
    // Caption and CardPageID come from the declaration (docs/probes-needed.md, probe batch 5): the
    // `Caption` (else the English `CaptionML`, else the name), and the id of the page `CardPageID`
    // names, 0 when none. Page Management reads the card page off the lookup page of a table.
    r[pos(t, 3)] = Value::Text(
        super::objects::declared_caption(prop(o, "Caption"), prop(o, "CaptionML"), &o.name)
            .map_err(|e| format!("page '{}': {e}", o.name))?
            .chars()
            .take(80)
            .collect(),
    );
    r[pos(t, 6)] = Value::Integer(match effective_prop(c, o, "CardPageID")? {
        None => 0,
        Some(v) => c
            .id_of(al_ir::ObjKind::Page, v)
            .map_err(|e| format!("page '{}': CardPageID {e}", o.name))? as i32,
    });
    // Editable: the declared literal, Yes when undeclared (misc/W2PageMetadata.al:461 RP_LookupPageSortedBySecondaryKey_CardPageId:
    // a List with Editable = false reads No, a Card that declares none Yes). Other page types without the property are not
    // pinned (docs/probes-needed.md).
    r[pos(t, 4)] = Value::Boolean(allowed(c, o, "Editable")?);
    r[pos(t, 5)] = option(t, 5, page_type);
    // BC's own placeholder for a declared DataCaptionExpression (TestPageMetadataProperties.al:23-26).
    r[pos(t, 7)] = Value::Text(
        if prop(o, "DataCaptionExpression").is_some() {
            "DataCaptionExprCode"
        } else {
            ""
        }
        .into(),
    );
    r[pos(t, 8)] = Value::Boolean(lit_bool(o, "RefreshOnActivate", false));
    r[pos(t, 9)] = Value::Text(text_prop(o, "APIPublisher", 40).into());
    r[pos(t, 10)] = Value::Text(text_prop(o, "APIGroup", 40).into());
    // APIVersion reads BC's own default `beta` when undeclared (TestPageMetadataProperties.al:94-97).
    r[pos(t, 11)] = Value::Text(match prop(o, "APIVersion") {
        None => "beta".into(),
        Some(v) => string_lit(v).unwrap_or_default().chars().take(250).collect(),
    });
    r[pos(t, 12)] = Value::Text(text_prop(o, "EntitySetName", 250).into());
    r[pos(t, 13)] = Value::Text(text_prop(o, "EntityName", 250).into());
    r[pos(t, 14)] = Value::Integer(src_id as i32);
    r[pos(t, 15)] = Value::Text(view_text(o, src, c.objects)?.into());
    for (id, name) in [(16, "InsertAllowed"), (17, "ModifyAllowed"), (18, "DeleteAllowed")] {
        r[pos(t, id)] = Value::Boolean(allowed(c, o, name)?);
    }
    r[pos(t, 19)] = Value::Boolean(has_source && lit_bool(o, "DelayedInsert", false));
    r[pos(t, 20)] = Value::Boolean(has_source && lit_bool(o, "ShowFilter", true));
    r[pos(t, 21)] = Value::Boolean(has_source && lit_bool(o, "MultipleNewLines", false));
    r[pos(t, 22)] = Value::Boolean(has_source && lit_bool(o, "SaveValues", false));
    r[pos(t, 23)] = Value::Boolean(has_source && lit_bool(o, "AutoSplitKey", false));
    r[pos(t, 24)] = Value::Text(if has_source {
        caption_fields(o, src)?.into()
    } else {
        String::new().into()
    });
    r[pos(t, 26)] = Value::Boolean(has_source && lit_bool(o, "LinksAllowed", true));
    r[pos(t, 27)] = Value::Boolean(lit_bool(o, "ChangeTrackingAllowed", false));
    r[pos(t, 28)] = Value::Boolean(has_source && lit_bool(o, "PopulateAllFields", false));
    r[pos(t, 30)] = Value::Text(
        ident(prop(o, "InherentPermissions").unwrap_or(""))
            .chars()
            .take(5)
            .collect(),
    );
    r[pos(t, 31)] = Value::Text(
        ident(prop(o, "InherentEntitlements").unwrap_or(""))
            .chars()
            .take(5)
            .collect(),
    );
    r[pos(t, 32)] = Value::Text(o.namespace.clone().unwrap_or_default().into());
    Ok(r)
}

// ---- Page Control Field -----------------------------------------------------------------------------

/// A boolean control property as the column text: undeclared is `default`; a literal is its own
/// text in lower case; an expression of a form the corpus pins, the text lowering wrote for it (`Control::prop_texts`). What BC
/// reports for another expression is not pinned (docs/probes-needed.md): that column alone is left unknown, so a read of it is
/// refused and the rest of the row is served (al-runner issue 5786).
fn bool_prop(o: &Object, ctl: &al_ir::Control, name: &str, default: &str) -> Result<Value, String> {
    match ctl.props.iter().find(|(k, _)| k == name).map(|(_, v)| v) {
        None => Ok(Value::Text(default.into())),
        Some(PropVal::Lit(v)) => {
            let t = v.trim().to_ascii_lowercase();
            if t != "true" && t != "false" {
                return Err(format!(
                    "page '{}' control '{}': {name} `{v}` is not a literal",
                    o.name, ctl.name
                ));
            }
            // How BC spells a declared value is not pinned; Evaluate reads this form
            // (TestPageControlFieldDefaults.al:52-80).
            Ok(Value::Text(t.into()))
        }
        // An expression of a pinned form reads as BC writes it (P132PageControlVisible.al:134-264, AL Runner issue 5786).
        Some(_) if let Some((_, text)) = ctl.prop_texts.iter().find(|(k, _)| k == name) => Ok(Value::Text(text.clone().into())),
        Some(_) => Ok(Value::Unknown(
            format!(
                "Page Control Field: page '{}' control '{}': {name} is an expression (what the column reports is not pinned)",
                o.name, ctl.name
            )
            .into(),
        )),
    }
}

pub(super) fn page_control_field_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let mut out = Vec::new();
    let blank = Blank::new(t);
    for o in c.objects {
        let (Some(id), al_ir::ObjKind::Page) = (o.id, o.kind) else {
            continue;
        };
        let Some(page) = &o.page else {
            let mut k = blank.row();
            k[pos(t, 1)] = Value::Integer(id as i32);
            out.push(Entry::poisoned_prefix(
                k,
                1,
                format!("page '{}' was not compiled, so its controls are not known", o.name),
            ));
            continue;
        };
        let src = source_table(&c, o);
        let mut sequence = 0;
        for (n, ctl) in page.controls.iter().enumerate() {
            if ctl.kind != ControlKind::Field {
                continue;
            }
            let mut r = blank.row();
            r[pos(t, 1)] = Value::Integer(id as i32);
            r[pos(t, 2)] = Value::Integer(n as i32 + 1);
            // The field controls number from 0 in declaration order, those inside a group
            // included (record/TestTableRelationsSequenceMetadataContracts.al:107-122).
            r[pos(t, 11)] = Value::Integer(sequence);
            sequence += 1;
            match control_row(&c, o, src, ctl, t, &mut r) {
                Ok(()) => out.push(Entry::ok(r)),
                Err(why) => {
                    let mut k = blank.row();
                    k[pos(t, 1)] = Value::Integer(id as i32);
                    k[pos(t, 2)] = Value::Integer(n as i32 + 1);
                    out.push(Entry::poisoned(k, why));
                }
            }
        }
    }
    out
}

fn control_row(
    c: &Catalog,
    o: &Object,
    src: Option<&Object>,
    ctl: &al_ir::Control,
    t: &TableDef,
    r: &mut Row,
) -> Result<(), String> {
    r[pos(t, 3)] = Value::Text(ctl.name.clone().into());
    // TableNo is the page's source table, stated even for a control with no source field
    // (TestPageControlFieldControlTree.al:59-61).
    r[pos(t, 4)] = Value::Integer(src.and_then(|s| s.id).unwrap_or(0) as i32);
    // Enabled and Visible report a lower-case `true` when undeclared, Editable a capitalised `True`
    // (TestPageControlFieldDefaults.al:15-50).
    r[pos(t, 6)] = bool_prop(o, ctl, "enabled", "true")?;
    r[pos(t, 7)] = bool_prop(o, ctl, "editable", "True")?;
    r[pos(t, 8)] = bool_prop(o, ctl, "visible", "true")?;
    // The variables a control can name are the globals of the object that declares it.
    let owner = ctl.owner.map_or(o, |e| &c.objects[e]);
    match &ctl.binding {
        Binding::Field(fp) => {
            let table = src.and_then(|s| s.table.as_ref()).ok_or_else(|| {
                format!(
                    "page '{}' control '{}': the source table was not compiled",
                    o.name, ctl.name
                )
            })?;
            let f = table
                .fields
                .get(*fp)
                .ok_or_else(|| format!("page '{}' control '{}': no such source field", o.name, ctl.name))?;
            r[pos(t, 5)] = Value::Integer(f.id as i32);
            r[pos(t, 9)] = Value::Text(f.name.clone().into());
            // The members of an option or enum field (TestPageControlFieldDefaults.al:60-72).
            r[pos(t, 10)] = Value::Text(match &f.ty {
                RtType::Option(m) => m.join(",").into(),
                RtType::Enum(e) => c.objects[*e as usize]
                    .enum_values
                    .iter()
                    .map(|(_, n)| n.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
                    .into(),
                _ => String::new().into(),
            });
        }
        // A control bound to a page variable names the variable and has no source field
        // (TestPageControlFieldControlTree.al:42-57).
        Binding::Var(slot) => {
            let g = owner
                .globals
                .get(*slot)
                .ok_or_else(|| format!("page '{}' control '{}': no such variable", o.name, ctl.name))?;
            r[pos(t, 9)] = Value::Text(g.name.clone().into());
        }
        _ => {
            return Err(format!(
                "page '{}' control '{}': an expression-bound control (its SourceExpression is not pinned)",
                o.name, ctl.name
            ));
        }
    }
    Ok(())
}
