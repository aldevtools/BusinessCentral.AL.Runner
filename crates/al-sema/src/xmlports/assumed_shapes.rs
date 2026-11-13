//! XmlPort layouts that run under an assumption of corpus PR 1366 (not pinned yet). Each predicate requires, by equality, the
//! facts every port of its probe file has; the result is stored on the IR node and read by al-vm. Every other layout keeps
//! main's result.

use super::{Context, INERT_OBJECT_PROPERTIES, Layout, bool_prop, split_source_at_dot, table_object};
use crate::members::FieldClass;
use crate::types::Ty;
use al_ir as ir;
use ir::XmlNodeKind as K;

/// The port's own properties are `required` (lower-case name and value), each once, and besides them only `Format = Xml`,
/// `UseRequestPage = false` (as every probe port declares them) and the properties that change nothing
/// (`INERT_OBJECT_PROPERTIES`).
fn port_properties(layout: &Layout, required: &[(&str, &str)]) -> bool {
    let props = &layout.object_properties;
    let once = |name: &str, value: &str| {
        props
            .iter()
            .filter(|(n, v)| n == name && v.eq_ignore_ascii_case(value))
            .count()
            == 1
    };
    required.iter().all(|(n, v)| once(n, v))
        && props.iter().all(|(n, v)| {
            required.iter().any(|(r, _)| r == n)
                || (n == "format" && v.eq_ignore_ascii_case("xml"))
                || (n == "userequestpage" && bool_prop(v) == Some(false))
                || INERT_OBJECT_PROPERTIES.contains(&n.as_str())
        })
}

/// The properties of node `i` are exactly `want` (name and value, compared ignoring case), in any order.
fn node_properties(layout: &Layout, i: usize, want: &[(&str, &str)]) -> bool {
    let props: Vec<(String, String)> = layout.nodes[i]
        .section
        .properties()
        .map(|p| (p.name(), p.value_text().trim().to_string()))
        .collect();
    props.len() == want.len()
        && want.iter().all(|(n, v)| {
            props
                .iter()
                .any(|(pn, pv)| pn.eq_ignore_ascii_case(n) && pv.eq_ignore_ascii_case(v))
        })
}

/// The lower-case names of the triggers of node `i`.
fn triggers(layout: &Layout, i: usize) -> Vec<String> {
    layout.nodes[i]
        .section
        .triggers()
        .map(|t| t.name().to_ascii_lowercase())
        .collect()
}

fn kids(layout: &Layout, i: usize) -> Vec<usize> {
    (0..layout.nodes.len())
        .filter(|&k| layout.nodes[k].parent == Some(i))
        .collect()
}

/// Node `t` is a table element right below the root text element.
fn table_below_root(layout: &Layout, t: usize) -> bool {
    layout.nodes[t].kind == K::Table
        && layout.nodes[t]
            .parent
            .is_some_and(|r| layout.nodes[r].parent.is_none() && layout.nodes[r].kind == K::Text)
}

/// The field node `f` names the table element `t` as its source.
fn of_table(layout: &Layout, f: usize, t: usize) -> bool {
    layout.nodes[f]
        .args
        .get(1)
        .and_then(|s| split_source_at_dot(s))
        .is_some_and(|(table, _)| crate::scan::unquote_xmlport(table).eq_ignore_ascii_case(&layout.nodes[t].name))
}

/// Field node `i` in the import layout of xmlports/import/XpAfterAssignTableRelation.al (corpus PR 1366, ports 77702-77711,
/// :83-465). Each fact by equality, as every port there has it:
/// - the port declares `Direction = Import`, `Encoding = UTF8`, `FormatEvaluate = Xml` (:86-90);
/// - its table element is right below the root text element and declares nothing or only `UseTemporary = true` (:96-98,
///   :290-292); its triggers are OnBeforeInsertRecord, OnAfterInsertRecord or neither (:114-122);
/// - every node of the table element is a field attribute or field element of it without nodes below it, and exactly one has a
///   trigger: node `i`, with OnAfterAssignField only, declaring nothing or only `FieldValidate = No` (:99-112, :196-198).
///
/// What al-vm `xmlports/after_assign.rs` checks on top: the field, its relation and validate code, and the record.
pub(super) fn after_assign_layout(layout: &Layout, i: usize) -> bool {
    let n = &layout.nodes[i];
    let Some(t) = n.parent else {
        return false;
    };
    let siblings = kids(layout, t);
    let field_leaves = siblings.iter().all(|&k| {
        matches!(layout.nodes[k].kind, K::Field | K::FieldAttr) && of_table(layout, k, t) && !layout.has_children(k)
    });
    let triggered = siblings.iter().filter(|&&k| !triggers(layout, k).is_empty()).count();
    let table_triggers = triggers(layout, t)
        .iter()
        .all(|name| name == "onbeforeinsertrecord" || name == "onafterinsertrecord");
    port_properties(
        layout,
        &[("direction", "import"), ("encoding", "utf8"), ("formatevaluate", "xml")],
    ) && layout.format == ir::XmlFormat::Xml
        && table_below_root(layout, t)
        && (node_properties(layout, t, &[]) || node_properties(layout, t, &[("UseTemporary", "true")]))
        && table_triggers
        && field_leaves
        && triggered == 1
        && triggers(layout, i) == ["onafterassignfield"]
        && (node_properties(layout, i, &[]) || node_properties(layout, i, &[("FieldValidate", "No")]))
}

/// Text element `i` in the export shape of xmlports/shapes-properties/XpTextElemFieldAttrExport.al (corpus PR 1366, ports
/// 77721-77727, :43-361). Each fact by equality, as every port there with a field attribute has it:
/// - the port declares `Direction = Export` and `FormatEvaluate = Xml` (:46-49), no Encoding and no namespace;
/// - its table element is right below the root text element and declares exactly `MinOccurs = Zero`, `UseTemporary = true`
///   (:57-60);
/// - `i` declares `MaxOccurs = Once`, with or without `MinOccurs = Zero` (:64-65, :204), and has no trigger (:109) or only
///   OnBeforePassVariable (:68); it is the only text element of the table element with a field attribute;
/// - its one node is a field attribute of a Normal Boolean field (:33), or with the trigger a Text field (:34, :300), of that
///   table element, without properties or triggers (:66).
pub(super) fn text_attr_export(cx: &Context, layout: &Layout, i: usize) -> bool {
    let n = &layout.nodes[i];
    let Some(t) = n.parent else {
        return false;
    };
    let [attr] = kids(layout, i)[..] else {
        return false;
    };
    let with_attr = |k: usize| {
        layout.nodes[k].kind == K::Text && kids(layout, k).iter().any(|&a| layout.nodes[a].kind == K::FieldAttr)
    };
    let one_such = kids(layout, t).into_iter().filter(|&k| with_attr(k)).count() == 1;
    let element_triggers = triggers(layout, i);
    port_properties(layout, &[("direction", "export"), ("formatevaluate", "xml")])
        && layout.format == ir::XmlFormat::Xml
        && n.kind == K::Text
        && table_below_root(layout, t)
        && node_properties(layout, t, &[("MinOccurs", "Zero"), ("UseTemporary", "true")])
        && (node_properties(layout, i, &[("MaxOccurs", "Once")])
            || node_properties(layout, i, &[("MaxOccurs", "Once"), ("MinOccurs", "Zero")]))
        && (element_triggers.is_empty() || element_triggers == ["onbeforepassvariable"])
        && one_such
        && layout.nodes[attr].kind == K::FieldAttr
        && of_table(layout, attr, t)
        && node_properties(layout, attr, &[])
        && triggers(layout, attr).is_empty()
        && measured_attr_field(cx, layout, attr, t, !element_triggers.is_empty())
}

/// The field of attribute `attr` is a Normal field of the table of table element `t`: Boolean (:33, with and without the
/// element's trigger, :66 and :113) or, with the trigger, Text (:34, :300).
fn measured_attr_field(cx: &Context, layout: &Layout, attr: usize, t: usize, with_trigger: bool) -> bool {
    let Some(table) = layout.nodes[t].args.get(1).and_then(|name| table_object(cx, name)) else {
        return false;
    };
    let Some((_, field)) = layout.nodes[attr].args.get(1).and_then(|s| split_source_at_dot(s)) else {
        return false;
    };
    let field = crate::scan::unquote_xmlport(field);
    cx.members[table]
        .fields
        .iter()
        .find(|f| f.name.eq_ignore_ascii_case(&field))
        .is_some_and(|f| {
            f.class == FieldClass::Normal && (f.ty == Ty::Boolean || (with_trigger && matches!(f.ty, Ty::Text(_))))
        })
}
