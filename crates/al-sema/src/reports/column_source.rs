//! Report column sources: the one parser of a column's source ([`ColumnSource`]: `Field`, `Item.Field`, `Global.Field`,
//! `Parent.Field`), and the gates of the column shapes a probe measures beyond the general column rules of `reports.rs`. Each
//! gate is one allowlist predicate over the facts of its cells; it sets a field of `ir::ReportColumn` that al-vm reads:
//! - a qualifier that holds a dot (`"Dq. Row".Qty`): [`dotted_columns_on`], [`dotted_column_measured`],
//!   [`dotted_qualifier_measured`] (assumption `report-column-dotted-item-qualifier`);
//! - [`measured_field`] of a column over a field of its own item, and [`parent_fields`] for a field of the direct parent
//!   (`ir::MeasuredField`).
//!
//! Every gate cites corpus PR 1652 (tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al at cf6884b9, merged,
//! not yet pinned) or the pinned corpus.

use super::*;
use crate::members::FieldInfo;

/// A column's source split at each dot outside double quotes (`"Dq. Row".Name.ToUpper()` is `Dq. Row`, `Name`, `ToUpper()`),
/// each part trimmed and unquoted as the lexer unquotes a name (`ast::unquote_ident`). The one reader of a column source:
/// `Owner::columns`, `caption_sources` and the gates of this module.
pub(super) struct ColumnSource {
    pub parts: Vec<String>,
    /// Every part is a name: an identifier or a quoted identifier (the tokens `ast::Section::arg_name_parts` reads).
    names: bool,
}

impl ColumnSource {
    pub fn of(column: &ast::Section) -> ColumnSource {
        let raw = column.args_raw().get(1).cloned().unwrap_or_default();
        let raw_parts = crate::scan::split_names(raw.trim(), '.');
        ColumnSource {
            names: raw_parts.iter().all(|p| is_name(p.trim())),
            parts: raw_parts.iter().map(|p| ast::unquote_ident(p.trim())).collect(),
        }
    }

    /// `Qualifier.Member`: exactly two parts, each a name.
    pub fn qualified(&self) -> Option<(&str, &str)> {
        match self.parts.as_slice() {
            [q, m] if self.names => Some((q, m)),
            _ => None,
        }
    }

    /// The source has a qualifier that holds a dot.
    pub fn dotted(&self) -> bool {
        self.parts.len() > 1 && self.parts[0].contains('.')
    }
}

/// An identifier (a letter or `_`, then letters, digits or `_`: al-syntax `lexer.rs`) or a quoted identifier.
fn is_name(p: &str) -> bool {
    if p.len() >= 2 && p.starts_with('"') && p.ends_with('"') {
        return true;
    }
    let mut chars = p.chars();
    chars.next().is_some_and(|c| c == '_' || c.is_alphabetic()) && chars.all(|c| c == '_' || c.is_alphanumeric())
}

/// A name that holds no dot.
fn undotted(name: &str) -> bool {
    name.chars().all(|c| c != '.')
}

/// How a column's source names a field of its own data item, with the field's name as declared.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Written {
    /// `Field`.
    Bare,
    /// `Item.Field` with the item's name as declared; `dotted`: the name holds a dot.
    Qualified { dotted: bool },
}

/// The rule of column `column` of the report itself over field `f` of its own data item's table `t`, written `written`.
pub(super) fn measured_field(f: &FieldInfo, t: usize, column: &ast::Section, written: Written) -> ir::MeasuredField {
    use ir::MeasuredField as M;
    match written {
        Written::Bare if amount_places_pinned(f, t, column) => M::AmountOverPlaces { assumed: false },
        Written::Qualified { .. } if field_unit_amount_measured(f, t, column) => M::UnitAmount,
        Written::Qualified { dotted: false } if amount_places_measured(f, t, column) => {
            M::AmountOverPlaces { assumed: true }
        }
        Written::Qualified { dotted: false } if option_caption_measured(f, t, column) => M::OptionCaption,
        _ => M::None,
    }
}

/// The shape of the properties column `column` states: none, exactly `IncludeCaption = true`, exactly `DecimalPlaces = 4 : 4`
/// (the one value probe :82 measures), or anything else (`None`).
#[derive(Clone, Copy, PartialEq)]
enum ColumnShape {
    NoProperty,
    IncludeCaption,
    DecimalPlaces,
}

fn column_shape(column: &ast::Section) -> Option<ColumnShape> {
    let stated: Vec<(String, String)> = column
        .properties()
        .map(|p| {
            (
                p.name().to_ascii_lowercase(),
                p.value_text().trim().to_ascii_lowercase(),
            )
        })
        .collect();
    match stated.as_slice() {
        [] => Some(ColumnShape::NoProperty),
        [(name, value)] if name == "includecaption" && value == "true" => Some(ColumnShape::IncludeCaption),
        [(name, value)] if name == "decimalplaces" && value.split_whitespace().collect::<String>() == "4:4" => {
            Some(ColumnShape::DecimalPlaces)
        }
        _ => None,
    }
}

/// Property `name` of field `f` as written, trimmed; DecimalPlaces without blanks (`0 : 5` is `0:5`).
fn field_prop(f: &FieldInfo, name: &str) -> Option<String> {
    let v = f
        .properties
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))?
        .1
        .trim();
    Some(if name.eq_ignore_ascii_case("DecimalPlaces") {
        v.split_whitespace().collect()
    } else {
        v.to_string()
    })
}

/// A normal field table `t` declares itself, whose caption no tableextension changes and that states no CaptionML: the
/// field facts every cell of DataItemNamesDottedQualifier.al shares (:30-37, :348-359 at cf6884b9), and of the field of
/// reports/dataset/ReportColumnFormatsHeaderRest.al:25.
fn plain_own_field(f: &FieldInfo, t: usize) -> bool {
    f.class == crate::members::FieldClass::Normal
        && f.declared_in == Some(t)
        && !f.caption_modified
        && field_prop(f, "CaptionML").is_none()
}

/// The AutoFormatType, AutoFormatExpression and DecimalPlaces field `f` states ([`field_prop`]).
fn field_formats(f: &FieldInfo) -> (Option<String>, Option<String>, Option<String>) {
    let p = |name: &str| field_prop(f, name);
    (p("AutoFormatType"), p("AutoFormatExpression"), p("DecimalPlaces"))
}

/// A Decimal field with AutoFormatType 1, no AutoFormatExpression and DecimalPlaces 0:3, a Caption as `captioned` says.
fn amount_places_field(f: &FieldInfo, t: usize, captioned: bool) -> bool {
    let formats = field_formats(f);
    plain_own_field(f, t)
        && f.ty == crate::types::Ty::Decimal
        && field_prop(f, "Caption").is_some() == captioned
        && (formats.0.as_deref(), formats.1.as_deref(), formats.2.as_deref()) == (Some("1"), None, Some("0:3"))
}

/// The pinned bare column over an AutoFormatType 1 field with DecimalPlaces 0:3 (reports/dataset/
/// ReportColumnFormatsHeaderRest.al: field :25 without Caption, column :46 without property; `#,##0.00` from BC 28 at :123,
/// `#,##0.###` on BC 27 at :125).
fn amount_places_pinned(f: &FieldInfo, t: usize, column: &ast::Section) -> bool {
    amount_places_field(f, t, false) && column_shape(column) == Some(ColumnShape::NoProperty)
}

/// Assumption `report-column-field-amount-places-format` (corpus PR 1652 at cf6884b9): `Item.Field` over the field :35
/// (AutoFormatType 1, DecimalPlaces 0:3, a Caption) under exactly IncludeCaption = true (:115), lines :286 and :288.
fn amount_places_measured(f: &FieldInfo, t: usize, column: &ast::Section) -> bool {
    amount_places_field(f, t, true) && column_shape(column) == Some(ColumnShape::IncludeCaption)
}

/// Assumption `report-column-qualified-option-caption` (corpus PR 1652 at cf6884b9): `Item.Field` over the Option field :356
/// (a Caption, `OptionMembers = " ",Open,Closed`, `OptionCaption = ' ,DQ Open cap,DQ Closed cap'`) under exactly
/// IncludeCaption = true (:428), line :678: a caption that is not empty for each member, no CaptionML or format property.
fn option_caption_measured(f: &FieldInfo, t: usize, column: &ast::Section) -> bool {
    let (crate::types::Ty::Option(members), Some(captions)) = (&f.ty, field_prop(f, "OptionCaption")) else {
        return false;
    };
    let captions = captions
        .strip_prefix('\'')
        .and_then(|c| c.strip_suffix('\''))
        .unwrap_or_default();
    let captions: Vec<&str> = captions.split(',').collect();
    plain_own_field(f, t)
        && field_prop(f, "Caption").is_some()
        && field_prop(f, "OptionCaptionML").is_none()
        && field_formats(f) == (None, None, None)
        && members.len() == captions.len()
        && captions.iter().all(|c| c.chars().count() > 0)
        && column_shape(column) == Some(ColumnShape::IncludeCaption)
}

/// Whether column `column` over field `f` of its own data item's table `t`, written `Item.Field` with the item's and the field's
/// names as declared, takes the `decimalformatter` of the field's AutoFormatType 2 (`#,##0.00###`, as a column's own
/// AutoFormatType 2 does: al-vm `column_formatter.rs`): assumption `report-column-field-unit-amount-format`, corpus PR 1652
/// (DataItemNamesDottedQualifier.al at cf6884b9): [`plain_own_field`] with a Caption, Decimal, AutoFormatType 2, no
/// AutoFormatExpression (:34, columns :78 and :114, lines :236 and :286) or AutoFormatExpression '' (:354, columns :389 and
/// :426, lines :630 and :692), no DecimalPlaces; a column of the report itself that states exactly IncludeCaption = true.
/// Every other column over such a field keeps the formatter of DecimalPlaces.
fn field_unit_amount_measured(f: &FieldInfo, t: usize, column: &ast::Section) -> bool {
    let formats = field_formats(f);
    plain_own_field(f, t)
        && f.ty == crate::types::Ty::Decimal
        && field_prop(f, "Caption").is_some()
        && matches!(
            (formats.0.as_deref(), formats.1.as_deref(), formats.2.as_deref()),
            (Some("2"), None | Some("''"), None)
        )
        && column_shape(column) == Some(ColumnShape::IncludeCaption)
}

/// Whether column `column` over field `f` of table `t`, written with its data item's dotted name (`"Dq. Row".Qty`, the name
/// in the item's own case), is a shape corpus PR 1652 measures (assumption `report-column-dotted-item-qualifier`;
/// tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al at cf6884b9; labels :224, :565-567, values :236,
/// :596-633). [`plain_own_field`], and by type:
/// - Text with a Caption (:31, :351), Date or Enum with a Caption (:352, :348), Code or Integer with or without one (:349,
///   :357, :350, :358), no DecimalPlaces, AutoFormatType or AutoFormatExpression, a column that states
///   exactly IncludeCaption = true (:75, :383-393); the Date and the Enum field also under a column without property
///   (:395, :398, values :600, :614);
/// - Decimal, with or without a Caption, by (AutoFormatType, AutoFormatExpression, DecimalPlaces): (none, none, any) under
///   IncludeCaption or a column DecimalPlaces = 4 : 4 (:32, :36, :37, :76, :80-82); (1, none, none) (:33, :77) and (0, none, none)
///   (:359, :394) under IncludeCaption; (0, none, 0:5) and (1, '', none) under IncludeCaption or no property (:353, :355,
///   :388, :390, :396, :397); AutoFormatType 2 only where [`field_unit_amount_measured`] holds.
///
/// Every other dotted shape keeps main's answer (no field); an Option field too (:616 is served undotted only, by
/// [`option_caption_measured`]). [`dotted_column_measured`] reads it for each own-item column of the report.
fn dotted_qualifier_measured(f: &FieldInfo, t: usize, column: &ast::Section) -> bool {
    use crate::types::Ty;
    use ColumnShape::*;
    let Some(shape) = column_shape(column) else {
        return false;
    };
    let p = |name: &str| field_prop(f, name);
    let captioned = p("Caption").is_some();
    let formats = field_formats(f);
    let no_format = formats == (None, None, None);
    let typed = match &f.ty {
        Ty::Text(_) => return dotted_text_measured(f, t, column, IncludeCaption),
        Ty::Date | Ty::Enum(_) => captioned && no_format && matches!(shape, IncludeCaption | NoProperty),
        Ty::Code(_) | Ty::Integer => no_format && shape == IncludeCaption,
        Ty::Decimal => match (formats.0.as_deref(), formats.1.as_deref(), formats.2.as_deref()) {
            (None, None, _) => matches!(shape, IncludeCaption | DecimalPlaces),
            // Only where the field formatter answers AutoFormatType 2 as BC does.
            (Some("2"), _, _) => field_unit_amount_measured(f, t, column),
            (Some("1" | "0"), None, None) => shape == IncludeCaption,
            (Some("0"), None, Some("0:5")) | (Some("1"), Some("''"), None) => {
                matches!(shape, IncludeCaption | NoProperty)
            }
            _ => false,
        },
        _ => false,
    };
    plain_own_field(f, t) && typed
}

/// Whether field `f` of table `t` is a Text field with a Caption that [`dotted_qualifier_measured`] admits (:31, :351: no
/// DecimalPlaces, AutoFormatType or AutoFormatExpression) and `column` states exactly the properties of `shape`.
fn dotted_text_measured(f: &FieldInfo, t: usize, column: &ast::Section, shape: ColumnShape) -> bool {
    plain_own_field(f, t)
        && matches!(f.ty, crate::types::Ty::Text(_))
        && field_prop(f, "Caption").is_some()
        && field_formats(f) == (None, None, None)
        && column_shape(column) == Some(shape)
}

/// The columns declared directly in data item `item`.
fn column_sections(item: &RawItem) -> impl Iterator<Item = ast::Section> {
    item.section
        .sections()
        .filter(|c| c.name().eq_ignore_ascii_case("column"))
}

/// Whether report `cx.oi` takes the field for its columns qualified with their own data item's dotted name (`"Dq. Row".Qty`),
/// assumption `report-column-dotted-item-qualifier`, all or none: only when a column of the report names its own item's
/// dotted name, every column whose source has a qualifier with a dot is a shape corpus PR 1652 measures
/// ([`dotted_column_measured`]), and no reportextension that changes the run declares such a column. Otherwise every dotted
/// column keeps main's answer and the run is not flagged.
pub(super) fn dotted_columns_on(cx: &Context) -> bool {
    let items = &cx.layout.items;
    let tables: Vec<Option<usize>> = items.iter().map(|r| table_object(cx.ws, cx.file, &r.table)).collect();
    let mut own = false;
    for (i, raw) in items.iter().enumerate() {
        for c in column_sections(raw) {
            let source = ColumnSource::of(&c);
            if !source.dotted() {
                continue;
            }
            match dotted_column_measured(cx.members, items, &tables, i, &c, &source.parts) {
                Some(j) => own |= j == i,
                None => return false,
            }
        }
    }
    own && !cx
        .extensions
        .iter()
        .flat_map(|&e| crate::pages::column_sections(&cx.ws.object_ast(e)))
        .any(|c| ColumnSource::of(&c).dotted())
}

/// Whether the child item `child` states a DataItemLink of exactly one `Field = field(ParentField)` pair: the link of the
/// probe's child items (`DataItemLink = RowId = field(Id)`, :88 and :124 at cf6884b9).
fn linked_child(child: &RawItem) -> bool {
    let Some(link) = child
        .section
        .properties()
        .find(|p| p.name().eq_ignore_ascii_case("DataItemLink"))
    else {
        return false;
    };
    let text = link.value_text();
    let pairs = crate::scan::split_double(&text, ',');
    let [pair] = pairs.as_slice() else {
        return false;
    };
    let parts = crate::scan::split_double(pair, '=');
    let [_, other] = parts.as_slice() else {
        return false;
    };
    let other = other.trim();
    other.to_ascii_lowercase().starts_with("field(") && other.ends_with(')')
}

/// The data item that column `column` of item `i`, with source `parts` ([`ColumnSource`]), reads through a dotted qualifier,
/// when the column is a shape DataItemNamesDottedQualifier.al (corpus PR 1652 at cf6884b9) measures. The qualifier and the
/// field are written as declared, and:
/// - `Item.Field` of its own item where [`dotted_qualifier_measured`] holds;
/// - `Item.Field.ToUpper()` of its own item, a Text field with a Caption, a column without property (:84, value :236);
/// - `Parent.Field` of the item it is nested in directly, when the child's name holds a dot (`"Dq. Child"` :86) and it is a
///   [`linked_child`] (:88), a Text field with a Caption under exactly IncludeCaption = true (`CParentName` :90, labels :224,
///   values :250). This column keeps main's rule for another item's field (`caption_sources`).
///
/// Every other column with a dotted qualifier is no measured shape (`None`): a Decimal field of the parent (:91-92; served
/// with undotted names only, [`parent_fields`]), another type or column property, another case, an item that is not the
/// parent, a global variable.
fn dotted_column_measured(
    members: &crate::members::MemberTable,
    items: &[RawItem],
    tables: &[Option<usize>],
    i: usize,
    column: &ast::Section,
    parts: &[String],
) -> Option<usize> {
    let (qualifier, member, call) = match parts {
        [q, m] => (q, m, None),
        [q, m, call] => (q, m, Some(call.as_str())),
        _ => return None,
    };
    let j = items.iter().position(|r| r.name == *qualifier)?;
    let t = tables[j]?;
    let f = members[t].fields.iter().find(|f| f.name == *member)?;
    let measured = match call {
        None if j == i => dotted_qualifier_measured(f, t, column),
        None => {
            items[i].parent == Some(j)
                && items[i].name.contains('.')
                && linked_child(&items[i])
                && dotted_text_measured(f, t, column, ColumnShape::IncludeCaption)
        }
        Some("ToUpper()") => j == i && dotted_text_measured(f, t, column, ColumnShape::NoProperty),
        Some(_) => false,
    };
    measured.then_some(j)
}

/// Assumption `report-column-parent-field-format` (corpus PR 1652 at cf6884b9): parent fields :32 (DecimalPlaces 0:3) and :33
/// (AutoFormatType 1), both with a Caption; the child :122-124 a [`linked_child`] of [`probe_child_table`]; columns :127-128
/// under exactly IncludeCaption = true; line :300.
fn parent_field_measured(f: &FieldInfo, t: usize, child: &RawItem, column: &ast::Section) -> bool {
    let formats = field_formats(f);
    plain_own_field(f, t)
        && f.ty == crate::types::Ty::Decimal
        && field_prop(f, "Caption").is_some()
        && matches!(
            (formats.0.as_deref(), formats.1.as_deref(), formats.2.as_deref()),
            (None, None, Some("0:3")) | (Some("1"), None, None)
        )
        && linked_child(child)
        && column_shape(column) == Some(ColumnShape::IncludeCaption)
}

/// Whether child item `child` over table `ct` has the facts of the probe's child (table :46-55, item :122-130 at cf6884b9) that
/// may decide the parent column's formatter (`#,##0.####` for AutoFormatType 1 is the DecimalPlaces 0:4 of :55): exactly one
/// Decimal field, [`plain_own_field`], with DecimalPlaces 0:4 and no AutoFormatType or AutoFormatExpression, and a column
/// `Child.Field` over it under exactly IncludeCaption = true (`CPrice` :130).
fn probe_child_table(members: &crate::members::MemberTable, ct: usize, child: &RawItem) -> bool {
    let decimals: Vec<&FieldInfo> = members[ct]
        .fields
        .iter()
        .filter(|f| f.ty == crate::types::Ty::Decimal)
        .collect();
    let [price] = decimals.as_slice() else {
        return false;
    };
    plain_own_field(price, ct)
        && field_formats(price) == (None, None, Some("0:4".to_string()))
        && column_sections(child).any(|c| {
            ColumnSource::of(&c).qualified() == Some((child.name.as_str(), price.name.as_str()))
                && column_shape(&c) == Some(ColumnShape::IncludeCaption)
        })
}

/// Marks each column of report `raw` whose source is `Parent.Field` of its direct parent, a top-level item, the parent's name
/// without a dot and both names as declared, in a [`probe_child_table`] child, where [`parent_field_measured`] holds: the parent field decides its `decimalformatter`
/// (`ir::MeasuredField::ParentField`). A dotted parent name stays with [`dotted_columns_on`], which turns the report's dotted
/// columns off for this shape.
pub(super) fn parent_fields(members: &crate::members::MemberTable, raw: &[RawItem], items: &mut [ir::ReportItem]) {
    for i in 0..items.len() {
        let Some(p) = raw[i].parent else {
            continue;
        };
        let (Some(t), Some(ct)) = (items[p].table, items[i].table) else {
            continue;
        };
        // The probe's parent is a top-level item (:111).
        if raw[p].parent.is_some() || !probe_child_table(members, ct, &raw[i]) {
            continue;
        }
        for (k, c) in column_sections(&raw[i]).enumerate().take(items[i].columns.len()) {
            let source = ColumnSource::of(&c);
            let Some((qualifier, member)) = source.qualified() else {
                continue;
            };
            if qualifier != raw[p].name || !undotted(qualifier) {
                continue;
            }
            let Some(pos) = members[t].fields.iter().position(|f| f.name == member) else {
                continue;
            };
            if parent_field_measured(&members[t].fields[pos], t, &raw[i], &c) {
                let column = &mut items[i].columns[k];
                column.places_table = Some(t);
                column.places_field = Some(pos);
                column.measured_field = ir::MeasuredField::ParentField;
            }
        }
    }
}
