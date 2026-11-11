//! The import of a root table element that holds a nested table element of headers (al-runner issue 6924; pinned by
//! xmlports/shapes-properties/XpRootTableNested.al, 11 tests :475-701, and the variants of XpRootTableNestedVariants.al :1415-1842:
//! declared or undeclared MinOccurs and MaxOccurs, a line table element with MinOccurs = Zero, the wrapper of the lines first,
//! in the middle or last, empty and self-closing elements). The rows are read by `xp_import_record` as any table
//! element's; this module only says which ports take that path and what a failed import may leave behind.

use super::*;

/// Whether the port's only top-level element is a `root_holding_table`.
pub(super) fn port_with_root_holding_table(schema: &Schema) -> bool {
    let mut tops = (0..schema.xp().nodes.len()).filter(|&i| schema.xp().nodes[i].parent.is_none());
    matches!((tops.next(), tops.next()), (Some(top), None) if root_holding_table(schema, top))
}

/// An Xml port with Direction not set (Both), FormatEvaluate = Xml, no namespaces and no PreserveWhiteSpace whose root is a table element without triggers, LinkTable, fields of its
/// own or MinOccurs = Zero, with MaxOccurs = Once or no MaxOccurs, that holds exactly one `header_table` (XpRootTableNested.al:81-186 xmlport
/// "XP Root Table Full" with a temporary root, :267-313 "XP Root Table Real" with a real one, :336-352 "XP Root Table Min";
/// XpRootTableNestedVariants.al:249-403 "XP RTV Undeclared" without MinOccurs or MaxOccurs on the root, pinned by :1443, :1457).
/// Its row is stored after every header (the root's key stays blank: :482 `roots 1:0;`, :671-672 `real root rows 1`, `id 0`).
pub(super) fn root_holding_table(schema: &Schema, top: usize) -> bool {
    let xp = schema.xp();
    let n = schema.node(top);
    let [head] = schema.children(top)[..] else {
        return false;
    };
    xp.format == ir::XmlFormat::Xml
        && xp.direction == ir::XmlDirection::Both
        && xp.format_evaluate_xml
        && !xp.preserve_whitespace
        && xp.namespaces.is_empty()
        && xp.default_ns.is_none()
        && !xp.inline_schema
        && !xp.use_lax
        && !xp.udn_root_prefix
        && n.kind == ir::XmlNodeKind::Table
        && n.parent.is_none()
        && n.max_unbounded != Some(true)
        && plain_table(n, &[])
        && header_table(schema, head)
}

/// A table element with its default storing (AutoSave, no AutoUpdate or AutoReplace), no LinkTable, SourceTableView or
/// MinOccurs = Zero, whose triggers are among `allowed`.
fn plain_table(n: &ir::XmlNode, allowed: &[&str]) -> bool {
    !n.min_zero && stores_plainly(n, allowed)
}

/// `plain_table` whatever its MinOccurs.
fn stores_plainly(n: &ir::XmlNode, allowed: &[&str]) -> bool {
    n.kind == ir::XmlNodeKind::Table
        && n.link_node.is_none()
        && n.source_view.is_none()
        && n.auto_save
        && !n.auto_update
        && !n.auto_replace
        && n.calc_fields.is_empty()
        && n.triggers.iter().all(|(name, _)| allowed.contains(&name.as_str()))
}

/// The header table element: temporary, MaxOccurs = Unbounded declared or not, OnAfterInitRecord and OnAfterInsertRecord its only
/// triggers, its nodes leaf field elements of its own record and leaf text elements without triggers (some MinOccurs = Zero), and
/// at most one text element that wraps a `line_table` (XpRootTableNested.al:88-185 with the wrapper last). The wrapper may stand
/// anywhere among the header's nodes: first (XpRootTableNestedVariants.al:593 "XP RTV Wrap First", pinned by :1499, :1513, :1601),
/// in the middle (:781 "XP RTV Wrap Middle", :1527, :1541) or last (:59 "XP RTV Wrap Last", :1415); the header's fields and text
/// elements are read in document order, so a line sees only those read before the wrapper (:1499 `kind=[]/cust=[]`).
fn header_table(schema: &Schema, head: usize) -> bool {
    let h = schema.node(head);
    let kids = schema.children(head);
    let wraps = kids.iter().filter(|&&k| !header_leaf(schema, head, k)).count();
    h.use_temporary
        && h.max_unbounded != Some(false)
        && plain_table(h, &["onafterinitrecord", "onafterinsertrecord"])
        && !kids.is_empty()
        && wraps <= 1
        && kids
            .iter()
            .all(|&k| header_leaf(schema, head, k) || line_wrap(schema, k))
}

/// A leaf field element of `table`'s own record, or a leaf text element, without triggers (XpRootTableNested.al:94-128).
fn header_leaf(schema: &Schema, table: usize, k: usize) -> bool {
    let m = schema.node(k);
    schema.children(k).is_empty()
        && m.triggers.is_empty()
        && !m.unbound
        && match m.kind {
            ir::XmlNodeKind::Field => m.table_node == Some(table),
            ir::XmlNodeKind::Text => !m.big_text,
            _ => false,
        }
}

/// A text element without triggers or MinOccurs = Zero whose only node is a `line_table` (XpRootTableNested.al:129-172 `Lines`).
fn line_wrap(schema: &Schema, w: usize) -> bool {
    let m = schema.node(w);
    let [line] = schema.children(w)[..] else {
        return false;
    };
    m.kind == ir::XmlNodeKind::Text
        && m.triggers.is_empty()
        && !m.min_zero
        && m.max_unbounded != Some(true)
        && line_table(schema, line)
}

/// The line table element: temporary, OnBeforeInsertRecord its only trigger, its nodes leaf field elements of its own record
/// without triggers (XpRootTableNested.al:134-171, MinOccurs and MaxOccurs not declared). MaxOccurs = Unbounded may be declared
/// (XpRootTableNestedVariants.al:128-131, pinned by :1415), and MinOccurs = Once or Zero: with Zero a wrapper without lines stores
/// the header without lines (:466-470, pinned by :1471, :1485, :1587, :1601); with Once it fails the schema check (:1555, :1571).
fn line_table(schema: &Schema, line: usize) -> bool {
    let l = schema.node(line);
    let kids = schema.children(line);
    l.use_temporary
        && l.max_unbounded != Some(false)
        && stores_plainly(l, &["onbeforeinsertrecord"])
        && !kids.is_empty()
        && kids
            .iter()
            .all(|&k| schema.node(k).kind == ir::XmlNodeKind::Field && header_leaf(schema, line, k))
}

impl Session {
    /// Imports document `root` into a `root_holding_table` port through `xp_import_record`. In BC a failed import leaves every
    /// temporary table of the port empty (XpRootTableNested.al:593 RT_Full_DecimalWithComma_Import, :612, :630: the failures there
    /// come before any row is stored); a failure after a row was stored in one is not pinned and is refused.
    pub(super) fn xp_import_root_holding_table(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        top: usize,
        root: &Elem,
    ) -> Result<(), Signal> {
        let result = self.xp_import_record(inst, schema, top, root, true, false);
        if let Err(Signal::Error(_)) = &result {
            for node in schema.xp().nodes.iter().filter(|n| n.kind == ir::XmlNodeKind::Table) {
                let rec = self.xp_record(inst, node)?;
                if !rec.borrow().temporary {
                    continue;
                }
                let t = self.table(&rec)?;
                let cx = self.cx.clone();
                let rows = self
                    .store_for(&rec)
                    .count(&cx, &t, &[])
                    .map_err(|e| Signal::unsupported(format!("{e}")))?;
                if rows != 0 {
                    return unsupported(
                        "XmlPort import of a root table element holding a table element that fails after a row was stored in a temporary table (not pinned)",
                    );
                }
            }
        }
        result
    }
}
