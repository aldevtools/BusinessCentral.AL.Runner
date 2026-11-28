//! XmlPort layouts that run only as their corpus file measures them. Each predicate requires, by equality, the facts every
//! port of that file has; the result is stored on the IR node (or, for a probe not pinned yet, as the port's assumption) and
//! read by al-vm. Every other layout keeps main's result.

use super::{Context, INERT_OBJECT_PROPERTIES, Layout, bool_prop, split_source_at_dot, table_object};
use crate::members::FieldClass;
use crate::types::Ty;
use al_ir as ir;
use ir::XmlNodeKind as K;

/// The port's own properties are `required` (lower-case name and value), each once, the names of `free` each once with any
/// value, and besides them only `Format = Xml`, `UseRequestPage = false` (as every probe port declares them) and the
/// properties that change nothing (`INERT_OBJECT_PROPERTIES`).
fn port_properties(layout: &Layout, required: &[(&str, &str)], free: &[&str]) -> bool {
    let props = &layout.object_properties;
    let once = |name: &str, value: &str| {
        props
            .iter()
            .filter(|(n, v)| n == name && v.eq_ignore_ascii_case(value))
            .count()
            == 1
    };
    required.iter().all(|(n, v)| once(n, v))
        && free.iter().all(|f| props.iter().filter(|(n, _)| n == f).count() == 1)
        && props.iter().all(|(n, v)| {
            required.iter().any(|(r, _)| r == n)
                || free.contains(&n.as_str())
                || (n == "format" && v.eq_ignore_ascii_case("xml"))
                || (n == "userequestpage" && bool_prop(v) == Some(false))
                || INERT_OBJECT_PROPERTIES.contains(&n.as_str())
        })
}

/// The properties of node `i` are exactly `want` (name and value, compared ignoring case), in any order. An `XmlName` that
/// names the node's own name exactly is left out: the IR node is then the same as without it (`xml_name`).
fn node_properties(layout: &Layout, i: usize, want: &[(&str, &str)]) -> bool {
    let node = &layout.nodes[i];
    let props: Vec<(String, String)> = node
        .section
        .properties()
        .map(|p| (p.name(), p.value_text().trim().to_string()))
        .filter(|(n, v)| !(n.eq_ignore_ascii_case("XmlName") && crate::scan::unquote_xmlport(v) == node.name))
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

/// The facts of the import layout of xmlports/import/XpAfterAssignTableRelation.al (ports 77702-77740,
/// :138-1324) that field node `i` shares with every port there, each by equality:
/// - the port declares `Direction = Import`, `Encoding = UTF8`, `FormatEvaluate = Xml` (:141-145);
/// - its table element is right below the root text element and declares nothing or only `UseTemporary = true` (:151-153,
///   :345); its triggers are OnBeforeInsertRecord, OnAfterInsertRecord or neither (:169-177);
/// - every node of the table element is a field attribute or field element of it without nodes below it (:154-155).
///
/// The result is the number of those nodes that have a trigger.
fn import_layout_triggered(layout: &Layout, i: usize) -> Option<usize> {
    let t = layout.nodes[i].parent?;
    let siblings = kids(layout, t);
    let field_leaves = siblings.iter().all(|&k| {
        matches!(layout.nodes[k].kind, K::Field | K::FieldAttr) && of_table(layout, k, t) && !layout.has_children(k)
    });
    let table_triggers = triggers(layout, t)
        .iter()
        .all(|name| name == "onbeforeinsertrecord" || name == "onafterinsertrecord");
    let shared = port_properties(
        layout,
        &[("direction", "import"), ("encoding", "utf8"), ("formatevaluate", "xml")],
        &[],
    ) && layout.format == ir::XmlFormat::Xml
        && table_below_root(layout, t)
        && (node_properties(layout, t, &[]) || node_properties(layout, t, &[("UseTemporary", "true")]))
        && table_triggers
        && field_leaves;
    shared.then(|| siblings.iter().filter(|&&k| !triggers(layout, k).is_empty()).count())
}

/// Field node `i` in the import layout with OnAfterAssignField (XpAfterAssignTableRelation.al ports 77702-77706, 77709-77711,
/// 77713, 77715, 77730-77731, 77734-77735, 77737-77738 and 77740): the facts of `import_layout_triggered`, and exactly one node of the table
/// element has a trigger: node `i`, with OnAfterAssignField only, declaring nothing (:155), only `FieldValidate = No` (:253,
/// :1031) or only `FieldValidate = Yes` (:855).
///
/// What al-vm `xmlports/after_assign.rs` checks on top: the field, its relation and validate code, and the record.
pub(super) fn after_assign_layout(layout: &Layout, i: usize) -> bool {
    import_layout_triggered(layout, i) == Some(1)
        && triggers(layout, i) == ["onafterassignfield"]
        && (node_properties(layout, i, &[])
            || node_properties(layout, i, &[("FieldValidate", "No")])
            || node_properties(layout, i, &[("FieldValidate", "Yes")]))
}

/// Field node `i` in the import layout without triggers of the four temporary ports that change main's result
/// (XpAfterAssignTableRelation.al ports 77714 :576, 77717 :694, 77719 :765 and 77732 :935). Each fact by equality, as all
/// four have it:
/// - the facts of `import_layout_triggered`, and no node of the table element has a trigger;
/// - the port declares no trigger;
/// - the root text element declares nothing, has no trigger and holds only the table element;
/// - the table element holds two nodes: first a field attribute that declares nothing, then `i` (:592-593, :710-711,
///   :781-782, :951-952);
/// - node `i` declares nothing (:711, :782) or only `FieldValidate = Yes` (:954).
///
/// What al-vm `xmlports/after_assign.rs` checks on top: the field of the first node (the table's Integer key field), the
/// field of `i`, its relation and validate code, and the record; only a temporary table element changes main's result there.
pub(super) fn plain_import_layout(layout: &Layout, i: usize) -> bool {
    let Some(t) = layout.nodes[i].parent else {
        return false;
    };
    let Some(root) = layout.nodes[t].parent else {
        return false;
    };
    import_layout_triggered(layout, i) == Some(0)
        && layout.object_triggers.is_empty()
        && node_properties(layout, root, &[])
        && triggers(layout, root).is_empty()
        && kids(layout, root) == [t]
        && matches!(kids(layout, t)[..], [first, second] if second == i
            && layout.nodes[first].kind == K::FieldAttr
            && node_properties(layout, first, &[]))
        && (node_properties(layout, i, &[]) || node_properties(layout, i, &[("FieldValidate", "Yes")]))
}

/// Text element `i` in the export shape of xmlports/shapes-properties/XpTextElemFieldAttrExport.al (ports
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
    port_properties(layout, &[("direction", "export"), ("formatevaluate", "xml")], &[])
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

/// The schema layouts of xmlports/shapes-properties/XpUnusedNsPrefix.al (corpus PR 1663, not pinned yet). Every node
/// declares nothing unless named here.
#[derive(Clone, Copy, PartialEq)]
enum NsLayout {
    /// A root text element over one text leaf whose only trigger is OnBeforePassVariable (:40-49).
    TextLeaf,
    /// A root text element over a text attribute (NamespacePrefix naming a declared prefix, OnBeforePassVariable), then the
    /// text leaf (:221-238).
    TextAttrLeaf,
    /// A root text element over one table element (an XmlName at most, no trigger) with a field attribute and then a field
    /// element of it, neither with properties or triggers (:278-286).
    TableAttrField,
}

/// The default namespace properties of a port of that file.
#[derive(Clone, Copy, PartialEq)]
enum NsDefault {
    /// DefaultNamespace and UseDefaultNamespace = true (:34-35).
    DnUdn,
    /// Neither property (:134-140).
    Neither,
    /// DefaultNamespace and UseDefaultNamespace = false (:164-165).
    DnNoUdn,
    /// UseDefaultNamespace = true alone (:190).
    UdnNoDn,
}

/// The measured cells: layout, default namespace, Encoding, Direction and the number of Namespaces prefixes.
const UNUSED_PREFIX_CELLS: &[(NsLayout, NsDefault, &str, &str, usize)] = &[
    // Port :28, XUP_OneUnused_Export :351.
    (NsLayout::TextLeaf, NsDefault::DnUdn, "utf8", "export", 1),
    // Port :54, XUP_TwoUnused_Export :359, XUP_TwoUnused_ReadByLine :367, XUP_TwoUnused_InstanceExport :389.
    (NsLayout::TextLeaf, NsDefault::DnUdn, "utf8", "export", 2),
    // Port :134, XUP_NoDefault_Export :428.
    (NsLayout::TextLeaf, NsDefault::Neither, "utf8", "export", 2),
    // Port :158, XUP_DnNoUdn_Export :436.
    (NsLayout::TextLeaf, NsDefault::DnNoUdn, "utf8", "export", 2),
    // Port :184, XUP_UdnNoDn_Export :444.
    (NsLayout::TextLeaf, NsDefault::UdnNoDn, "utf8", "export", 2),
    // Port :209, XUP_TextAttr_Export :452.
    (NsLayout::TextAttrLeaf, NsDefault::DnUdn, "utf8", "export", 2),
    // Port :266, XUP_Table_Export :469.
    (NsLayout::TableAttrField, NsDefault::DnUdn, "utf8", "export", 2),
    // Port :291, XUP_TwoUnusedUtf16_Export :478.
    (NsLayout::TextLeaf, NsDefault::DnUdn, "utf16", "export", 2),
    // Port :317, XUP_Both_ExportImport :486.
    (NsLayout::TableAttrField, NsDefault::DnUdn, "utf8", "both", 2),
];

/// The port is a cell of xmlports/shapes-properties/XpUnusedNsPrefix.al (corpus PR 1663, assumption
/// `xmlport-unused-namespace-prefix`): Format = Xml, Namespaces of named prefixes only, no element and no field attribute
/// with NamespacePrefix (a text attribute in `NsLayout::TextAttrLeaf` only), no port trigger and no request page, and its
/// layout, default namespace, Encoding, Direction and prefix count are one row of `UNUSED_PREFIX_CELLS`. Besides those the
/// port declares only what `port_properties` lets through.
pub(super) fn unused_prefixes(layout: &Layout) -> bool {
    let named = !layout.namespaces.is_empty() && layout.namespaces.iter().all(|(p, _)| !p.is_empty());
    let Some(shape) = ns_layout(layout) else {
        return false;
    };
    named
        && layout.format == ir::XmlFormat::Xml
        && layout.object_triggers.is_empty()
        && !layout.request_page
        && UNUSED_PREFIX_CELLS
            .iter()
            .any(|&(cell, default, encoding, direction, count)| {
                cell == shape
                    && count == layout.namespaces.len()
                    && ns_port_properties(layout, default, encoding, direction)
            })
}

/// The port's assumption when it is such a cell (`ir::XmlPort::assumes`; al-vm reads it to know the port's import).
pub(super) fn unused_prefixes_assumes(layout: &Layout) -> Vec<al_builtins::assumptions::Assumption> {
    layout
        .unused_prefixes
        .then_some(al_builtins::assumptions::Assumption::XmlportUnusedNamespacePrefix)
        .into_iter()
        .collect()
}

fn ns_port_properties(layout: &Layout, default: NsDefault, encoding: &str, direction: &str) -> bool {
    let mut required = vec![("direction", direction), ("encoding", encoding)];
    let (udn, dn) = match default {
        NsDefault::DnUdn => (Some("true"), true),
        NsDefault::Neither => (None, false),
        NsDefault::DnNoUdn => (Some("false"), true),
        NsDefault::UdnNoDn => (Some("true"), false),
    };
    if let Some(value) = udn {
        required.push(("usedefaultnamespace", value));
    }
    let free: &[&str] = if dn {
        &["namespaces", "defaultnamespace"]
    } else {
        &["namespaces"]
    };
    port_properties(layout, &required, free)
}

/// The `NsLayout` of the port's schema, all of it, or `None`.
fn ns_layout(layout: &Layout) -> Option<NsLayout> {
    let n = &layout.nodes;
    let root = n
        .first()
        .filter(|r| r.parent.is_none() && r.kind == K::Text)
        .map(|_| 0)?;
    let bare = |i: usize| node_properties(layout, i, &[]) && !layout.has_children(i);
    let passes = |i: usize| triggers(layout, i) == ["onbeforepassvariable"];
    let leaf = |i: usize| n[i].kind == K::Text && bare(i) && passes(i);
    let attr = |i: usize| {
        let prefix = layout
            .property(i, "NamespacePrefix")
            .map(|p| crate::scan::unquote_xmlport(&p));
        n[i].kind == K::TextAttr
            && n[i].section.properties().count() == 1
            && prefix.is_some_and(|p| layout.namespaces.iter().any(|(q, _)| *q == p))
            && !layout.has_children(i)
            && passes(i)
    };
    let field = |i: usize, t: usize, kind: K| {
        n[i].kind == kind && of_table(layout, i, t) && bare(i) && triggers(layout, i).is_empty()
    };
    let table = |t: usize| {
        n[t].kind == K::Table
            && n[t]
                .section
                .properties()
                .all(|p| p.name().eq_ignore_ascii_case("XmlName"))
            && triggers(layout, t).is_empty()
            && matches!(kids(layout, t)[..], [a, f] if field(a, t, K::FieldAttr) && field(f, t, K::Field))
    };
    if !node_properties(layout, root, &[]) || !triggers(layout, root).is_empty() {
        return None;
    }
    match (kids(layout, root).as_slice(), n.len()) {
        (&[l], 2) if leaf(l) => Some(NsLayout::TextLeaf),
        (&[a, l], 3) if attr(a) && leaf(l) => Some(NsLayout::TextAttrLeaf),
        (&[t], 4) if table(t) => Some(NsLayout::TableAttrField),
        _ => None,
    }
}
