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
fn source_table<'a>(c: &'a Catalog, o: &Object) -> Option<&'a al_ir::ObjectHead> {
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
/// integer range `a .. b`, and on a Date field `filter('')`, `filter(0D)`, `filter(<> '')` and `const(0D)`
/// (pages/lt-cd-testpage-leads/CdTestPageLeads.al:890), `order(ascending)`, a Code const and a Decimal `filter(> 0)`
/// (platform/lt-aw-fresh-tenant/AwTests.al:116-117), a Decimal const of a whole number, and the Option, Enum and Code `filter()`
/// shapes of [`member_filter_text`]. Anything else (a const on another type, another filter) is not pinned.
/// The second value is the assumption the text rests on, if any.
fn view_text(
    o: &Object,
    table: Option<&al_ir::ObjectHead>,
    table_objects: &al_ir::Objects,
) -> Result<(String, Option<al_builtins::assumptions::Assumption>), String> {
    let Some(v) = prop(o, "SourceTableView") else {
        return Ok((String::new(), None));
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
        // `order(ascending)` adds nothing (platform/lt-aw-fresh-tenant/AwTests.al:116 `SORTING(Field2,Field3) WHERE(...)`).
        if body.trim().eq_ignore_ascii_case("descending") {
            out.push_str("ORDER(1) ");
        } else if !body.trim().eq_ignore_ascii_case("ascending") {
            return Err(not_pinned());
        }
    }
    let assumed: Option<al_builtins::assumptions::Assumption> = None;
    if let Some(body) = filters {
        let mut conds = Vec::new();
        for cond in split_top(body).ok_or_else(not_pinned)? {
            let (name, rest) = cond.split_once('=').ok_or_else(not_pinned)?;
            let f = field(name)?;
            let rest = rest.trim();
            if let Some(value) = rest
                .strip_prefix_ci("const(")
                .and_then(|r| r.strip_suffix(')'))
                .filter(|_| matches!(f.ty, RtType::Decimal))
            {
                // A Decimal const of a whole number reads as written (pages/lt-cd-testpage-leads/CdTestPageLeads.al
                // TD_SourceTableView_Const_Decimal `WHERE(Field2=CONST(0))`, `CONST(-2)`; al-runner issue 6030). A fraction
                // (`CONST(1.5)` there) is not built yet.
                let value = value.trim();
                let digits = value.strip_prefix('-').unwrap_or(value);
                if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
                    return Err(not_pinned());
                }
                conds.push(format!("Field{}=CONST({value})", f.id));
            } else if let Some(value) = rest.strip_prefix_ci("const(").and_then(|r| r.strip_suffix(')')) {
                conds.push(format!(
                    "Field{}=CONST({})",
                    f.id,
                    const_text(f, value.trim(), table_objects).ok_or_else(not_pinned)?
                ));
            } else if let Some(text) = rest
                .strip_prefix_ci("filter(")
                .and_then(|r| r.strip_suffix(')'))
                .filter(|_| matches!(f.ty, RtType::Date))
                .and_then(
                    |v| match v.split_whitespace().collect::<String>().to_ascii_lowercase().as_str() {
                        "<>''" => Some("<>''"),
                        "''" | "0d" => Some("''"),
                        _ => None,
                    },
                )
            {
                // A Date field blank or not blank: `filter('')` and `filter(0D)` read `Field7=FILTER('')`, `filter(<> '')` reads
                // `Field7=FILTER(<>'')` (pages/lt-cd-testpage-leads/CdTestPageLeads.al:890-905
                // SH_SourceTableView_DateFilterBlank_PageMetadata).
                conds.push(format!("Field{}=FILTER({text})", f.id));
            } else if let Some(n) = rest
                .strip_prefix_ci("filter(")
                .and_then(|r| r.strip_suffix(')'))
                .filter(|_| matches!(f.ty, RtType::Decimal))
                .and_then(|v| v.trim().strip_prefix('>'))
                .map(str::trim)
                .filter(|n| n.parse::<u32>().is_ok())
            {
                // A Decimal `filter(> 0)` reads `FILTER(>0)` (platform/lt-aw-fresh-tenant/AwTests.al:117). Another operator
                // or a fraction is not pinned.
                conds.push(format!("Field{}=FILTER(>{n})", f.id));
            } else if let Some(text) = rest
                .strip_prefix_ci("filter(")
                .and_then(|r| r.strip_suffix(')'))
                .and_then(|v| member_filter_text(f, v.trim(), table_objects))
            {
                conds.push(format!("Field{}=FILTER({text})", f.id));
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
    Ok((out, assumed))
}

/// The value of a `const(...)` as the view text shows it: an option or enum member by its ordinal
/// (TestPageMetadataSourceObject.al:61-80 declares `Status = const(Active)` and expects `CONST(2)`), a number as written,
/// a Text value without its quotes (W2PageMetadata.al:394 `CONST(x)`, :397 `CONST(a b)`).
/// The text of a `filter()` on an Option, Enum or Code field, the shapes pinned by lt-ss1030/SsPageMetaFilterView.al:97-113
/// (PageMetadata_SourceTableView_FilterShapes; al-runner issue 6251): an Option member reads as its ordinal (`filter(Quote)`
/// `FILTER(0)`), two Option members joined by `|` as their ordinals (`filter(Quote | Invoice)` `FILTER(0|2)`), `<>` and one Option
/// member as `<>` and its ordinal (`filter(<> "Order")` `FILTER(<>1)`), one Enum member as its ordinal (`filter(Beta)`
/// `FILTER(1)`), and a Code literal of two values joined by `|` without its quotes (`filter('A|B')` `FILTER(A|B)`). Any other
/// shape (more members, `<>` or `|` on an Enum, another operator, a Code literal of another form) is not pinned.
fn member_filter_text(f: &al_ir::Field, value: &str, table_objects: &al_ir::Objects) -> Option<String> {
    let member = |v: &str| {
        let v = v.trim();
        let v = v.strip_prefix('"').and_then(|x| x.strip_suffix('"')).unwrap_or(v);
        const_text(f, v, table_objects)
    };
    match &f.ty {
        RtType::Option(_) => {
            if let Some(rest) = value.strip_prefix("<>") {
                return Some(format!("<>{}", member(rest)?));
            }
            match value.split('|').collect::<Vec<_>>().as_slice() {
                [one] => member(one),
                [a, b] => Some(format!("{}|{}", member(a)?, member(b)?)),
                _ => None,
            }
        }
        RtType::Enum(_) if !value.contains(['|', '<', '>', '&', '.']) => member(value),
        RtType::Code(_) => {
            let text = string_lit(value)?;
            let parts: Vec<&str> = text.split('|').collect();
            let plain = |p: &&str| !p.is_empty() && p.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
            (parts.len() == 2 && parts.iter().all(plain)).then_some(text)
        }
        _ => None,
    }
}

fn const_text(f: &al_ir::Field, value: &str, table_objects: &al_ir::Objects) -> Option<String> {
    let plain = |v: &str| !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    match &f.ty {
        RtType::Text(_) => {
            let text = string_lit(value)?;
            (!text.is_empty() && text.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ')).then_some(text)
        }
        // A Code value without its quotes (platform/lt-aw-fresh-tenant/AwTests.al:116 `const('A')` reads `CONST(A)`). Pinned
        // for upper-case letters and digits; a lower-case letter (whether BC raises it) or another character is not.
        RtType::Code(_) => {
            let text = string_lit(value)?;
            (!text.is_empty() && text.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())).then_some(text)
        }
        RtType::Option(m) if plain(value) => m
            .iter()
            .position(|x| x.eq_ignore_ascii_case(value))
            .map(|p| p.to_string()),
        RtType::Enum(e) if plain(value) => objects_enum_ordinal(table_objects, *e, value).map(|p| p.to_string()),
        RtType::Integer | RtType::BigInteger if value.parse::<i64>().is_ok() => Some(value.to_string()),
        // The blank date as a DateTime text (pages/lt-cd-testpage-leads/CdTestPageLeads.al:890-905, `const 0D`).
        RtType::Date if value.eq_ignore_ascii_case("0d") => Some("0001-01-01T00:00:00.0000000".into()),
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

fn objects_enum_ordinal(objects: &al_ir::Objects, enum_object: u32, member: &str) -> Option<i32> {
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
fn caption_fields(o: &Object, table: Option<&al_ir::ObjectHead>) -> Result<String, String> {
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
    for (i, h) in c.objects.iter().enumerate() {
        let (Some(id), al_ir::ObjKind::Page) = (h.id, h.kind) else {
            continue;
        };
        // A row reads the page's controls: only the pages are made.
        let o = &c.full[i];
        let mut key = blank.row();
        key[pos(t, 1)] = Value::Integer(id as i32);
        match page_metadata_row(&c, o, t, key.clone()) {
            Ok((r, None)) => out.push(Entry::ok(r)),
            Ok((r, Some(a))) => out.push(Entry::ok(r).assuming(a)),
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
        .filter_map(|e| prop(*e, name))
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

fn page_metadata_row(
    c: &Catalog,
    o: &Object,
    t: &TableDef,
    mut r: Row,
) -> Result<(Row, Option<al_builtins::assumptions::Assumption>), String> {
    // A page type that is not pinned leaves only this column unknown: a read of it, or a filter on it the row's other
    // columns do not already rule out, is refused (al-runner issue 6251: System Application's "Power BI Element Addin Host"
    // is a `UserControlHost`).
    let page_type: Result<i32, String> = match prop(o, "PageType") {
        None => Ok(0),
        Some(v) => member(&PAGE_TYPES.iter().map(|(n, _)| *n).collect::<Vec<_>>(), v)
            .and_then(|name| PAGE_TYPES.iter().find(|(n, _)| *n == name).map(|(_, i)| *i))
            .ok_or_else(|| format!("Page Metadata: page '{}': PageType `{v}` is not pinned", o.name)),
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
    // A column whose value is not pinned is left unknown alone: a read of it, or a filter on it the row's other columns do
    // not rule out, is refused, and the rest of the row is served (al-runner issue 6251).
    let col = |v: Result<Value, String>| v.unwrap_or_else(|why| Value::Unknown(format!("Page Metadata: {why}").into()));
    r[pos(t, 3)] = col(
        super::objects::declared_caption(prop(o, "Caption"), prop(o, "CaptionML"), &o.name)
            .map_err(|e| format!("page '{}': {e}", o.name))
            .map(|c| Value::Text(c.chars().take(80).collect())),
    );
    r[pos(t, 6)] = col(effective_prop(c, o, "CardPageID").and_then(|p| match p {
        None => Ok(Value::Integer(0)),
        Some(v) => c
            .id_of(al_ir::ObjKind::Page, v)
            .map_err(|e| format!("page '{}': CardPageID {e}", o.name))
            .map(|id| Value::Integer(id as i32)),
    }));
    // Editable: the declared literal, Yes when undeclared (misc/W2PageMetadata.al:461 RP_LookupPageSortedBySecondaryKey_CardPageId:
    // a List with Editable = false reads No, a Card that declares none Yes). Other page types without the property are not
    // pinned (docs/probes-needed.md).
    r[pos(t, 4)] = col(allowed(c, o, "Editable").map(Value::Boolean));
    r[pos(t, 5)] = match page_type {
        Ok(n) => option(t, 5, n),
        Err(why) => Value::Unknown(why.into()),
    };
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
    // A view of a shape that is not pinned leaves only this column unknown: a read of it, or a filter on it, is refused,
    // and the rest of the row is served (al-runner issue 6251: Base App's Sales Quote has `filter(Quote)`).
    let view_assumed = match view_text(o, src, c.full) {
        Ok((view, assumed)) => {
            r[pos(t, 15)] = Value::Text(view.into());
            assumed
        }
        Err(why) => {
            r[pos(t, 15)] = Value::Unknown(format!("Page Metadata: {why}").into());
            None
        }
    };
    for (id, name) in [(16, "InsertAllowed"), (17, "ModifyAllowed"), (18, "DeleteAllowed")] {
        r[pos(t, id)] = col(allowed(c, o, name).map(Value::Boolean));
    }
    r[pos(t, 19)] = Value::Boolean(has_source && lit_bool(o, "DelayedInsert", false));
    r[pos(t, 20)] = Value::Boolean(has_source && lit_bool(o, "ShowFilter", true));
    r[pos(t, 21)] = Value::Boolean(has_source && lit_bool(o, "MultipleNewLines", false));
    r[pos(t, 22)] = Value::Boolean(has_source && lit_bool(o, "SaveValues", false));
    r[pos(t, 23)] = Value::Boolean(has_source && lit_bool(o, "AutoSplitKey", false));
    // DataCaptionFields that cannot be resolved leave only this column unknown, as an unpinned view does (al-runner issue
    // 6251: lt-ss1030/SsPageMetaFilterView.al:116 PageMetadata_FilteredRead_ApiPageOfTable reads the table beside such a page).
    r[pos(t, 24)] = if has_source {
        match caption_fields(o, src) {
            Ok(text) => Value::Text(text.into()),
            Err(why) => col(Err(why)),
        }
    } else {
        Value::Text(String::new().into())
    };
    r[pos(t, 25)] = super::page_temporary::source_table_temporary(o, has_source, src);
    r[pos(t, 26)] = Value::Boolean(has_source && lit_bool(o, "LinksAllowed", true));
    r[pos(t, 27)] = Value::Boolean(lit_bool(o, "ChangeTrackingAllowed", false));
    r[pos(t, 28)] = Value::Boolean(has_source && lit_bool(o, "PopulateAllFields", false));
    r[pos(t, 29)] = super::page_temporary::app_id(c, o);
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
    Ok((r, view_assumed))
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
    for (i, h) in c.objects.iter().enumerate() {
        let (Some(id), al_ir::ObjKind::Page) = (h.id, h.kind) else {
            continue;
        };
        // The controls are in the page itself: only the pages are made.
        let o = &c.full[i];
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
                Ok(false) => out.push(Entry::ok(r)),
                // A row of an expression-bound control (`Control::expr_source`), pinned by
                // record/TestPageControlFieldArraySource.al (codeunit 89002).
                Ok(true) => out.push(Entry::ok(r)),
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
    src: Option<&al_ir::ObjectHead>,
    ctl: &al_ir::Control,
    t: &TableDef,
    r: &mut Row,
) -> Result<bool, String> {
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
    let owner = ctl.owner.map_or(o, |e| &c.full[e]);
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
        // A control bound to an expression of a served form (`Control::expr_source`) has no source field and names the
        // expression as written; an undeclared Editable is `True` for an array element or a record variable's field and
        // `False` for an arithmetic expression (al-runner issue 5842, corpus writer's probe
        // record/TestPageControlFieldArraySource.al; not on the pinned corpus yet). A call with an argument list and no space
        // or text constant (`Format(Rec.Kind)`, `Rec.DisplayCode()`, `Names.NameOf(Rec."No.")`) names the call as written, `Rec.`
        // kept, and an undeclared Editable is `False` (record/lt-x3-6368/X3PageControlCalls.al:142-147 and :162-167, al-runner
        // issue 6368; corpus PR 874, branch al-runtime-rs-staging-x3 at 83d0857d, not yet on master).
        Binding::Expr(_) if let Some((text, editable)) = &ctl.expr_source => {
            r[pos(t, 9)] = Value::Text(text.clone().into());
            r[pos(t, 7)] = bool_prop(o, ctl, "editable", if *editable { "True" } else { "False" })?;
            return Ok(true);
        }
        // A single call of another form (a built-in such as `field(companyName; CompanyName)`, a method call written with a space,
        // `Control::expr_call`): the row is served with no
        // source field, FieldNo 0 (record/lt-wb-6284/WbPageControlBuiltIn.al:46-70, al-runner issue 6284; corpus branch
        // al-runtime-rs-staging-wb at a07787b2, not yet on master). What BC stores as its SourceExpression, and an undeclared
        // Editable, are not pinned: a read of either is refused.
        Binding::Expr(_) if ctl.expr_call => {
            let unpinned = |col: &str| {
                Value::Unknown(
                    format!(
                        "Page Control Field: page '{}' control '{}': an expression-bound control (its {col} is not pinned)",
                        o.name, ctl.name
                    )
                    .into(),
                )
            };
            r[pos(t, 9)] = unpinned("SourceExpression");
            if !ctl.props.iter().any(|(k, _)| k == "editable") {
                r[pos(t, 7)] = unpinned("Editable");
            }
            return Ok(true);
        }
        _ => {
            return Err(format!(
                "page '{}' control '{}': an expression-bound control (its SourceExpression is not pinned)",
                o.name, ctl.name
            ));
        }
    }
    Ok(false)
}
