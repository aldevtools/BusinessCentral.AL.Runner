//! Questions about the shape of the schema (which container is which kind), and GUID text.

use super::*;

/// The two shapes of a text element right below a table element that the corpus pins with its triggers and text
/// (
/// xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TO_AttrItem_*, TO_TextContent_*, objects
/// ToXpTextInTableObjects.al xmlports 90001-90003; the same test as al-sema xmlports.rs `text_in_table_shape`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TextInTable {
    /// Its nodes are only text attributes (xmlports 90001, 90002): `<item option_value="1">REQUIRED</item>`.
    Attrs,
    /// Text attributes and leaf text and field elements, at least one element (xmlport 90003).
    Content,
    /// Field and text attributes and exactly one text element whose nodes are only text attributes (
    /// QrXpContainerOccurs.al TP_Nested_*, xmlport
    /// 90022 in TpXpNestedObjects.al): `<entry no="R1" status="open"><admission option_value="1">A1</admission></entry>`.
    Nested,
}

pub(super) fn text_in_table(schema: &Schema, c: usize) -> Option<TextInTable> {
    let n = schema.node(c);
    let kids = schema.children(c);
    let below_table = n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table);
    // One text element further down, only text attributes below it: `<a:Party><b:EndpointID schemeID="0088">E1</b:EndpointID>`
    // with an OnBeforePassVariable on both (xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespacesTests.al:60, the PEPPOL shape).
    let below_text_in_table = n.parent.is_some_and(|p| {
        let p = schema.node(p);
        p.kind == ir::XmlNodeKind::Text && p.parent.is_some_and(|g| schema.node(g).kind == ir::XmlNodeKind::Table)
    });
    if n.kind == ir::XmlNodeKind::Text && below_text_in_table && attrs_only(schema, c) {
        return Some(TextInTable::Attrs);
    }
    if n.kind != ir::XmlNodeKind::Text || !below_table || kids.is_empty() {
        return None;
    }
    let mut elements = false;
    let mut field_attrs = false;
    let mut inner = 0;
    for &k in &kids {
        match schema.node(k).kind {
            ir::XmlNodeKind::TextAttr => {}
            ir::XmlNodeKind::FieldAttr => field_attrs = true,
            ir::XmlNodeKind::Text | ir::XmlNodeKind::Field if schema.children(k).is_empty() => elements = true,
            ir::XmlNodeKind::Text if attrs_only(schema, k) => inner += 1,
            _ => return None,
        }
    }
    match (inner, elements, field_attrs) {
        (1, false, _) => Some(TextInTable::Nested),
        (0, true, false) => Some(TextInTable::Content),
        (0, false, false) => Some(TextInTable::Attrs),
        _ => None,
    }
}

/// A text element right below a table element with no triggers and no attributes, whose elements are leaf text and field
/// elements and text elements with only text attributes, at least one of those: written as any container
/// (xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespacesTests.al:60, `<a:Party><b:EndpointID schemeID="0088">E1</b:EndpointID>
/// <Plain>p</Plain></a:Party>`).
pub(super) fn plain_container(schema: &Schema, c: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    n.kind == ir::XmlNodeKind::Text
        && n.triggers.is_empty()
        && kids.iter().any(|&k| attrs_only(schema, k))
        && kids.iter().all(|&k| {
            let m = schema.node(k);
            match m.kind {
                ir::XmlNodeKind::Text | ir::XmlNodeKind::Field if schema.children(k).is_empty() => true,
                ir::XmlNodeKind::Text => attrs_only(schema, k),
                _ => false,
            }
        })
}

/// A text element whose nodes are only text attributes (at least one).
pub(super) fn attrs_only(schema: &Schema, c: usize) -> bool {
    let kids = schema.children(c);
    !kids.is_empty() && kids.iter().all(|&a| schema.node(a).kind == ir::XmlNodeKind::TextAttr)
}

/// The inner element of a `TextInTable::Nested` text element.
pub(super) fn nested_inner(schema: &Schema, c: usize) -> bool {
    schema.node(c).kind == ir::XmlNodeKind::Text
        && schema
            .node(c)
            .parent
            .is_some_and(|p| text_in_table(schema, p) == Some(TextInTable::Nested))
}

/// A text element without triggers whose nodes are all attributes (at least one).
pub(super) fn attr_container(schema: &Schema, c: usize) -> bool {
    let kids = schema.children(c);
    schema.node(c).kind == ir::XmlNodeKind::Text
        && schema.node(c).triggers.is_empty()
        && !kids.is_empty()
        && kids.iter().all(|&k| {
            matches!(
                schema.node(k).kind,
                ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr
            )
        })
}

/// A text element right below a temporary table element, without triggers, with `MinOccurs = Zero` and `MaxOccurs = Once`,
/// holding only one temporary table element without LinkTable and with `MinOccurs = Zero` whose nodes are only field
/// attributes (
/// xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TP_Unlinked_*, xmlport 90023 in TpXpNestedObjects.al).
pub(super) fn unlinked_container(schema: &Schema, c: usize, table: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    let [inner] = kids[..] else {
        return false;
    };
    let t = schema.node(inner);
    let fields = schema.children(inner);
    n.kind == ir::XmlNodeKind::Text
        && n.parent == Some(table)
        && schema.node(table).kind == ir::XmlNodeKind::Table
        && schema.node(table).use_temporary
        && n.triggers.is_empty()
        && n.min_zero
        && n.max_unbounded == Some(false)
        && t.kind == ir::XmlNodeKind::Table
        && t.link_node.is_none()
        && t.use_temporary
        && t.min_zero
        && !fields.is_empty()
        && fields
            .iter()
            .all(|&f| schema.node(f).kind == ir::XmlNodeKind::FieldAttr)
}

/// For export only: a text element right below a temporary table element, without triggers, with `MinOccurs = Zero` and
/// `MaxOccurs = Once`, holding only one temporary table element without LinkTable that is optional (`MinOccurs = Zero`) or
/// required and repeating (`MinOccurs = Once`, `MaxOccurs = Unbounded`), with any nodes the export supports below it
/// (QrXpContainerOccurs.al TR_Booking_*, xmlport
/// 90064 in TrXpReplyObjects.al: `details` > `line` with text attributes and `parts` > `Part`). Leaf field elements of the
/// outer table without triggers may come before the inner table (TR_Member_*, xmlport 90065: `member` > `membernumber`,
/// `email`, `card`).
pub(super) fn unlinked_reply_container(schema: &Schema, c: usize, table: usize) -> bool {
    let n = schema.node(c);
    let kids = schema.children(c);
    let Some((&inner, fields)) = kids.split_last() else {
        return false;
    };
    let plain_field = |f: usize| {
        let k = schema.node(f);
        k.kind == ir::XmlNodeKind::Field
            && k.table_node == Some(table)
            && k.triggers.is_empty()
            && schema.children(f).is_empty()
    };
    if !fields.iter().all(|&f| plain_field(f)) {
        return false;
    }
    let t = schema.node(inner);
    n.kind == ir::XmlNodeKind::Text
        && n.parent == Some(table)
        && schema.node(table).kind == ir::XmlNodeKind::Table
        && schema.node(table).use_temporary
        && n.triggers.is_empty()
        && n.min_zero
        && n.max_unbounded == Some(false)
        && t.kind == ir::XmlNodeKind::Table
        && t.link_node.is_none()
        && t.use_temporary
        && (t.min_zero || t.max_unbounded == Some(true))
        && !schema.children(inner).is_empty()
}

/// For export only: a text element with the default MinOccurs and MaxOccurs, without triggers, right below a temporary table
/// element, holding only one temporary table element without LinkTable with `MaxOccurs = Once` and `MinOccurs = Zero`: written
/// empty without rows, with the first row only otherwise (former assumption `xmlport-default-container-unlinked` (pinned on corpus master since bump 1030); corpus branch
/// al-runtime-rs-staging-pub-ts, QrXpContainerOccurs.al TS_Cards_DefaultOccurs_Export, xmlport 90084 in TsXpObjects.al).
pub(super) fn default_unlinked_container(schema: &Schema, c: usize, table: usize) -> bool {
    let n = schema.node(c);
    let [inner] = schema.children(c)[..] else {
        return false;
    };
    let t = schema.node(inner);
    n.kind == ir::XmlNodeKind::Text
        && n.parent == Some(table)
        && schema.node(table).kind == ir::XmlNodeKind::Table
        && schema.node(table).use_temporary
        && n.triggers.is_empty()
        && !n.min_zero
        && n.max_unbounded.is_none()
        && t.kind == ir::XmlNodeKind::Table
        && t.link_node.is_none()
        && t.use_temporary
        && t.min_zero
        && t.max_unbounded == Some(false)
        && !schema.children(inner).is_empty()
}

/// For export only: a text element without triggers, with `MinOccurs = Zero` and `MaxOccurs = Once`, right below a temporary table
/// element with `MaxOccurs = Once`, holding one or more text attributes and then one text element without triggers, with
/// `MinOccurs = Zero` and `MaxOccurs = Once`, that holds only one temporary table element without LinkTable with `MinOccurs = Once`
/// and `MaxOccurs = Unbounded` (former assumption `xmlport-attrs-over-unlinked` (pinned on corpus master since bump 1030); corpus writer TT's branch al-runtime-rs-staging-tt,
/// QrXpContainerOccurs.al TT_Change_Export_*, xmlport 90104 in TtXpObjects.al: `Ticket` > `Admissions` > `Admission`).
pub(super) fn attrs_over_unlinked(schema: &Schema, c: usize, table: usize) -> bool {
    let n = schema.node(c);
    let tn = schema.node(table);
    let kids = schema.children(c);
    let Some((&inner, attrs)) = kids.split_last() else {
        return false;
    };
    let optional_once = |k: &ir::XmlNode| k.triggers.is_empty() && k.min_zero && k.max_unbounded == Some(false);
    let i = schema.node(inner);
    let [t] = schema.children(inner)[..] else {
        return false;
    };
    let t = schema.node(t);
    n.kind == ir::XmlNodeKind::Text
        && n.parent == Some(table)
        && tn.kind == ir::XmlNodeKind::Table
        && tn.use_temporary
        && tn.max_unbounded == Some(false)
        && optional_once(n)
        && !attrs.is_empty()
        && attrs.iter().all(|&a| schema.node(a).kind == ir::XmlNodeKind::TextAttr)
        && i.kind == ir::XmlNodeKind::Text
        && optional_once(i)
        && t.kind == ir::XmlNodeKind::Table
        && t.link_node.is_none()
        && t.use_temporary
        && !t.min_zero
        && t.max_unbounded == Some(true)
}

/// What the import of `text` into a Guid field with `FormatEvaluate = Xml` gives.
pub(super) enum XmlGuid {
    Value([u8; 16]),
    Invalid,
    Unpinned,
}

/// The XML forms of a Guid an import takes (QrXpContainerOccurs.al TS_Guid_Import_Element, TS_Guid_Import_Attribute on branch
/// al-runtime-rs-staging-pub-ts): the 8-4-4-4-12 form bare, in braces or in parentheses, or the 32 digits bare, in any case, with
/// spaces around; empty is the null Guid. A text with a character that is no hex digit, dash, bracket or space (`not-a-guid`),
/// or the 8-4-4-4-12 form one digit short, is invalid. Other forms (mismatched brackets, digits in brackets, other lengths) are
/// not pinned.
pub(super) fn xml_guid(text: &str) -> XmlGuid {
    if text.is_empty() {
        return XmlGuid::Value([0; 16]);
    }
    let t = text.trim_matches(' ');
    if t.chars()
        .any(|c| !(c.is_ascii_hexdigit() || matches!(c, '-' | '{' | '}' | '(' | ')')))
    {
        return XmlGuid::Invalid;
    }
    let inner = match (t.chars().next(), t.chars().last()) {
        (Some('{'), Some('}')) | (Some('('), Some(')')) => &t[1..t.len() - 1],
        _ => t,
    };
    if inner.contains(['{', '}', '(', ')']) {
        return XmlGuid::Unpinned;
    }
    let groups: Vec<&str> = inner.split('-').collect();
    let lens: Vec<usize> = groups.iter().map(|g| g.len()).collect();
    let digits: String = match lens.as_slice() {
        [8, 4, 4, 4, 12] => groups.concat(),
        [32] if inner.len() == t.len() => inner.to_string(),
        [8, 4, 4, 4, 11] => return XmlGuid::Invalid,
        _ => return XmlGuid::Unpinned,
    };
    let mut g = [0u8; 16];
    for (i, b) in g.iter_mut().enumerate() {
        *b = u8::from_str_radix(&digits[2 * i..2 * i + 2], 16).expect("hex digits checked above");
    }
    XmlGuid::Value(g)
}

pub(super) fn linked_container(schema: &Schema, c: usize, table: usize) -> bool {
    let kids = schema.children(c);
    let last = schema.children(table).into_iter().rfind(|&k| {
        !matches!(
            schema.node(k).kind,
            ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr
        )
    });
    schema.node(c).kind == ir::XmlNodeKind::Text
        && schema.node(c).triggers.is_empty()
        && last == Some(c)
        && !kids.is_empty()
        && kids.iter().all(|&k| {
            let n = schema.node(k);
            n.kind == ir::XmlNodeKind::Table && n.link_node == Some(table) && !n.link_fields.is_empty()
        })
}

/// Whether node `c` may come more than once in a row: its `MaxOccurs`, else a table element and a leaf text element may
/// (a leaf text element without MaxOccurs is read twice: xmlport/W7XmlPortTests.al:141,149, `required twice`), a field element
/// and a container text element may not (as before the MaxOccurs pins), except one right below a table element that holds only
/// field elements and has no triggers: it is read again into the same record, the last one wins (misc/lt-y13-triage/
/// Y13XmlPortContainer.al:120, :134, :156, corpus master: `twice` gives one row with the second container's values).
pub(super) fn repeats(schema: &Schema, c: usize) -> bool {
    let n = schema.node(c);
    n.max_unbounded.unwrap_or(match n.kind {
        ir::XmlNodeKind::Table => true,
        ir::XmlNodeKind::Text => {
            let kids = schema.children(c);
            kids.is_empty()
                || (n.triggers.is_empty()
                    && n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table)
                    && kids.iter().all(|&k| schema.node(k).kind == ir::XmlNodeKind::Field))
        }
        _ => false,
    })
}
