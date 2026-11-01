//! Export of a Format = Xml port: elements, containers, field text.

use super::*;

impl Session {
    pub(super) fn xp_export(&mut self, inst: &Inst, schema: &Schema, guarded: bool) -> Result<Value, Signal> {
        // PreserveWhiteSpace changes nothing an Xml export writes (al-sema xmlports.rs, Np4XmlPort.al on corpus master).
        inst.borrow_mut().xp_text.push((6, String::new()));
        let addr = Rc::as_ptr(inst) as usize;
        self.xp_pass_views.retain(|(a, _, _)| *a != addr);
        self.xp_unlinked_passes.retain(|(a, _, _)| *a != addr);
        let result = self.xp_export_inner(inst, schema, guarded);
        self.xp_pass_views.retain(|(a, _, _)| *a != addr);
        self.xp_unlinked_passes.retain(|(a, _, _)| *a != addr);
        // After the export the row a linked MaxOccurs = Once element was left on is as unpinned as its view.
        for n in schema.xp().nodes.iter().filter(|n| n.kind == ir::XmlNodeKind::Table) {
            if let Ok(rec) = self.xp_record(inst, n) {
                let mut r = rec.borrow_mut();
                if r.unpinned_buffer == Some(crate::records::XP_NESTED_ROW_AFTER_ONCE) {
                    r.unpinned_buffer = Some(crate::records::XP_ROW_AFTER_ONCE);
                }
            }
        }
        inst.borrow_mut()
            .xp_text
            .retain(|(k, _)| *k != 6 && *k != 7 && *k != XP_EXPORTED);
        inst.borrow_mut().xp_text.push((XP_EXPORTED, String::new()));
        // A table element record after an export under SourceTableView: what corpus writer TS pinned of it runs, the rest
        // refuses (records.rs `XP_VIEW_AFTER_EXPORT`, former assumption `xmlport-export-again` (pinned on corpus master since bump 1030)).
        for (i, n) in schema.xp().nodes.iter().enumerate() {
            if n.source_view.is_some() {
                if let Ok(rec) = self.xp_record(inst, schema.node(i)) {
                    // The link filter of a linked element with LinkFields does not show afterwards: GetFilters reads '' (corpus
                    // PR 896, misc/lt-np4-probes/Np4XmlPort.al Y9_XmlPort_LinkedOnceOnce_* `held ...|filters=`). It is kept
                    // with the view's own filters, which count but do not show.
                    if !n.link_fields.is_empty() {
                        for f in rec.borrow_mut().filters.iter_mut() {
                            if f.group == 0 && n.link_fields.iter().any(|(own, _)| *own == f.field) {
                                f.group = XP_VIEW_GROUP;
                            }
                        }
                        let addr = Rc::as_ptr(&rec) as usize;
                        if !self.xp_link_hidden.contains(&addr) {
                            self.xp_link_hidden.push(addr);
                        }
                    }
                    if rec.borrow().unpinned_buffer != Some(crate::records::XP_ROW_AFTER_ONCE) {
                        rec.borrow_mut().unpinned_buffer = Some(crate::records::XP_VIEW_AFTER_EXPORT);
                    }
                }
            }
        }
        result
    }

    pub(super) fn xp_export_inner(&mut self, inst: &Inst, schema: &Schema, guarded: bool) -> Result<Value, Signal> {
        let xp = schema.xp();
        // The document is written in its own encoding whatever the stream's own.
        let blob = match inst.borrow().globals[xp.destination_slot].clone() {
            Value::Library(_, h) => match &*h.borrow() {
                LibraryValue::OutStream(blob, _) => blob.clone(),
                _ => return unsupported("XmlPort export without SetDestination (a download is not supported)"),
            },
            _ => return unsupported("XmlPort export without SetDestination (a download is not supported)"),
        };
        // BC's export of a port with an unbound field element does not end (YXmlPortProps.al:715-716).
        if let Some(n) = xp.nodes.iter().find(|n| n.unbound) {
            return unsupported(format!(
                "XmlPort export of the unbound field element {} (BC's export of it does not end; not pinned)",
                n.xml_name
            ));
        }
        let tops: Vec<usize> = (0..xp.nodes.len()).filter(|&i| xp.nodes[i].parent.is_none()).collect();
        let [top] = tops[..] else {
            return unsupported("XmlPort export with more than one top-level element (not pinned)");
        };
        let table_top = xp.format != ir::XmlFormat::Xml && xp.nodes[top].kind == ir::XmlNodeKind::Table;
        // A table element root written once with any content below it, in a port that declares namespaces: the PEPPOL BIS 3.0
        // shape (xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespaces.al, an Integer root with MaxOccurs = Once, an OnAfterGetRecord,
        // text and table elements below; Y17XmlPortNamespacesTests.al:60-93). Without namespaces it is not pinned.
        let declares = !xp.namespaces.is_empty() || xp.default_ns.is_some();
        let table_root = root_table_element(schema, top) || (declares && once_root(schema, top));
        if !table_top && !table_root && (xp.nodes[top].kind != ir::XmlNodeKind::Text || schema.children(top).is_empty())
        {
            return unsupported("XmlPort export whose root is not a text element with content (not pinned)");
        }
        let view_keys = self.xp_apply_source_views(inst, schema)?;
        self.xp_pass_key_snapshots(inst, schema, &view_keys)?;
        let bytes = if xp.format == ir::XmlFormat::Xml {
            if xp.utf8 {
                return unsupported("XmlPort export with TextEncoding = UTF8 (not pinned)");
            }
            // A table element as the root is written once per row, one root element after the other; with no row, once with
            // the record as it stands (platform/lt-ag-contoso/AgXmlPortImport.al:557 QX_RootTableElement_Export, :584 _TwoRows).
            if table_root {
                // Namespaces on a table element root that is written once (MaxOccurs = Once over Integer, the PEPPOL shape):
                // xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespacesTests.al:60, :71, :82, :93 (UTF8). Several roots, UTF16 or
                // InlineSchema with them are not pinned.
                if xp.inline_schema || (declares && (!xp.xml_utf8 || xp.preserve_whitespace)) {
                    return unsupported(
                        "XmlPort export of a table element root with InlineSchema or namespaces (not pinned)",
                    );
                }
                let mut rows = match self.xp_export_table(inst, schema, top) {
                    Err(Signal::Error(_)) if guarded => {
                        return unsupported("an error inside an Export whose result is used (not pinned)");
                    }
                    other => other?,
                };
                if declares {
                    let [root] = rows.as_mut_slice() else {
                        return unsupported(
                            "XmlPort export of a table element root with namespaces written other than once (not pinned)",
                        );
                    };
                    if al_builtins::xmlport::has_line_end(root) {
                        return unsupported(
                            "XmlPort export of a text value with a line end under a table element root with namespaces (not pinned)",
                        );
                    }
                    declare_namespaces(root, &xp.namespaces, xp.default_ns.as_deref());
                }
                let encoding = if xp.xml_utf8 { "UTF-8" } else { "UTF-16" };
                let text = al_builtins::xmlport::serialize_roots(&rows, encoding);
                let bytes = if xp.xml_utf8 {
                    text.into_bytes()
                } else {
                    al_builtins::xmlport::utf16_bom_bytes(&text)
                };
                al_builtins::stream::write_bytes(&blob, &bytes);
                return Ok(if guarded { Value::Boolean(true) } else { Value::Null });
            }
            if xp.inline_schema {
                let text = self.xp_export_inline(inst, schema, top)?;
                al_builtins::stream::write_bytes(&blob, &al_builtins::xmlport::utf16_bom_bytes(&text));
                return Ok(if guarded { Value::Boolean(true) } else { Value::Null });
            }
            match self.xp_export_container(inst, schema, top).map(|mut root| {
                declare_namespaces(&mut root, &xp.namespaces, xp.default_ns.as_deref());
                root
            }) {
                // A line end in a text value (LF, CR LF, a lone CR) is written as CR LF: pinned on an UTF8 export, with and without
                // PreserveWhiteSpace and a default namespace (misc/lt-np4-probes/Np4XmlPort.al:280, :300, :320, corpus PR 871;
                // Y9_XmlPort_Export_LineFeed_Plain, corpus PR 896), and on an UTF16 export without either (FF FE, 302 bytes:
                // Y9_XmlPort_Export_LineFeed_Utf16_RawStream, _Utf16Stream). UTF16 with PreserveWhiteSpace or a namespace is not.
                Ok(root)
                    if al_builtins::xmlport::has_line_end(&root)
                        && !xp.xml_utf8
                        && (xp.preserve_whitespace || xp.default_ns.is_some()) =>
                {
                    return unsupported(
                        "XmlPort export of a text value with a line end on an UTF16 port with PreserveWhiteSpace or a default namespace (not pinned)",
                    );
                }
                Ok(mut root) if al_builtins::xmlport::has_line_end(&root) => {
                    al_builtins::xmlport::line_ends_to_lf(&mut root);
                    let enc = if xp.xml_utf8 { "UTF-8" } else { "UTF-16" };
                    let text =
                        al_builtins::xmlport::line_feeds_to_crlf(&al_builtins::xmlport::serialize_as(&root, enc));
                    if xp.xml_utf8 {
                        text.into_bytes()
                    } else {
                        al_builtins::xmlport::utf16_bom_bytes(&text)
                    }
                }
                Ok(root) if xp.xml_utf8 => al_builtins::xmlport::serialize_as(&root, "UTF-8").into_bytes(),
                Ok(root) => al_builtins::xmlport::utf16_bom_bytes(&al_builtins::xmlport::serialize(&root)),
                Err(Signal::Error(_)) if guarded => {
                    return unsupported("an error inside an Export whose result is used (not pinned)");
                }
                Err(e) => return Err(e),
            }
        } else {
            self.xp_export_text(inst, schema, top)?
        };
        al_builtins::stream::write_bytes(&blob, &bytes);
        Ok(if guarded { Value::Boolean(true) } else { Value::Null })
    }

    /// The `SourceTableView` of each table element, before an export reads a row (xmlport/lt-ra-xmlport/RaXmlPortExport.al): the
    /// view's sorting and order replace the key and direction a SetTableView gave the record (:654 `SetTableView sorting(Seq)`,
    /// also descending, exports in the view's order) and its filters are added to the record's (:654 `SetTableView filter
    /// only`; on a linked element to the link, :793 `where on the link field`: Def B has no line of Def A). A view without
    /// sorting keeps the record's key and direction. Pinned for Format = Xml without InlineSchema; the rest is refused.
    pub(super) fn xp_apply_source_views(
        &mut self,
        inst: &Inst,
        schema: &Schema,
    ) -> Result<Vec<(usize, ViewKey)>, Signal> {
        let xp = schema.xp();
        let mut keys = Vec::new();
        let again = inst.borrow().xp_text.iter().any(|(k, _)| *k == XP_EXPORTED);
        for (i, n) in xp.nodes.iter().enumerate() {
            let Some(raw) = n.source_view.as_deref() else { continue };
            if xp.format != ir::XmlFormat::Xml || xp.inline_schema {
                return unsupported(format!(
                    "XmlPort export with a SourceTableView on table element {} in a port of a Format other than Xml or with InlineSchema (not pinned)",
                    n.name
                ));
            }
            let view = crate::views::property_view(raw);
            let lower = view.to_ascii_lowercase();
            // A filter on an element without LinkTable (QrXpContainerOccurs.al TS_View_FirstExport on branch
            // al-runtime-rs-staging-pub-ts): the rows of the filter in the view's key, combined with the code's filters.
            // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_View_FirstExport, TS_View_SecondExport (was assumption `xmlport-view-filter-unlinked`).
            let rec = self.xp_record(inst, schema.node(i))?;
            let t = self.table(&rec)?;
            if again {
                // A later export on the same instance (QrXpContainerOccurs.al TS_View_SecondExport, TS_Again_KeyBetween on
                // branch al-runtime-rs-staging-pub-ts): the key is not set again (a key set in between, or the primary key
                // after Reset, decides the order), the view's filters are applied again, the code's filters stay. After
                // SetView, or with an ORDER in the view, it is not pinned.
                if rec.borrow().unpinned_buffer == Some(crate::records::XP_BUFFER_AFTER_EXPORT) {
                    return unsupported(
                        "XmlPort export after SetView on a table element record an earlier export on the same instance left (not pinned)",
                    );
                }
                if lower.contains("order") {
                    return unsupported(
                        "a later XmlPort export on the same instance of a table element whose SourceTableView has an ORDER (not pinned)",
                    );
                }
                // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Again_SameData, TS_Again_AssignBetween, TS_Again_TransferFieldsBetween, TS_Again_InitBetween, TS_Again_KeyBetween, TS_Again_LineFilter, TS_View_SecondExport, TS_Again_TrBooking, TS_Again_TrMember (was assumption `xmlport-export-again`).
                let mut r = rec.borrow_mut();
                if r.unpinned_buffer == Some(crate::records::XP_VIEW_AFTER_EXPORT) {
                    r.unpinned_buffer = None;
                }
                r.filters.retain(|f| f.group != XP_VIEW_GROUP);
            }
            let kept = rec.borrow().filters.clone();
            let kept_order = {
                let r = rec.borrow();
                (r.key, r.adhoc.clone(), r.ascending, r.field_descending)
            };
            self.set_view_in(&rec, &t, &view, Some(XP_VIEW_GROUP), false, false, false)?;
            let mut r = rec.borrow_mut();
            if lower.contains("sorting") {
                keys.push((i, (r.key, r.adhoc.clone())));
            }
            if again || !lower.contains("sorting") {
                (r.key, r.adhoc, r.ascending, r.field_descending) = kept_order;
            } else if !lower.contains("order") {
                // A sorting without order is ascending, also after a SetTableView with Ascending(false) (:654 `SetTableView
                // sorting(Seq) descending`).
                r.ascending = true;
                r.field_descending = 0;
            }
            let mut filters = kept;
            filters.extend(r.filters.iter().filter(|f| f.group == XP_VIEW_GROUP).cloned());
            r.filters = filters;
        }
        Ok(keys)
    }

    /// The export of a port with `InlineSchema = true`: the root, an XSD with a complex type for the table element and one
    /// for the root, then the rows on one line (xmlport/lt-y-xmlport-props/YXmlPortProps.al:563
    /// InlineSchema_Export_ContainsSchema). Pinned for a root text element holding one table element whose children are
    /// Integer and Text field elements, with at least one row; anything else is refused.
    pub(super) fn xp_export_inline(&mut self, inst: &Inst, schema: &Schema, top: usize) -> Result<String, Signal> {
        let xp = schema.xp();
        let shape = || "XmlPort export with InlineSchema of a shape the corpus does not pin (a root text element with one table element of Integer and Text field elements, no namespaces or Encoding)";
        let kids = schema.children(top);
        let [table] = kids[..] else {
            return unsupported(shape());
        };
        if schema.node(table).kind != ir::XmlNodeKind::Table
            || schema.node(table).link_node.is_some()
            || !xp.namespaces.is_empty()
            || xp.xml_utf8
            || !schema.node(top).triggers.is_empty()
            || !schema.node(table).triggers.is_empty()
        {
            return unsupported(shape());
        }
        let tn = schema.node(table);
        if tn.max_unbounded.is_some() {
            return unsupported(shape());
        }
        let rec = self.xp_record(inst, tn)?;
        let t = self.table(&rec)?;
        // The schema of the table element: its elements in a sequence, then its attributes, in schema order
        // (xmlport/lt-bd-xmlport/BdXmlPort.al:425 BdInlineSchema_FieldTypes, :473 BdInlineSchema_Attributes). An Integer
        // element is `xs:int` and a BigInteger one `xs:long`, both with `default="0"`; every other pinned type is `xs:string`.
        // A field attribute is `xs:string` whatever its type, `use="required"` unless Occurrence = Optional; a text attribute
        // is required; a text element with MinOccurs = Zero is `minOccurs="0" maxOccurs="unbounded"`.
        let (mut fields, mut attrs) = (String::new(), String::new());
        for f in schema.children(table) {
            let n = schema.node(f);
            if !n.triggers.is_empty() || n.max_unbounded.is_some() || n.ns_prefix.is_some() {
                return unsupported(shape());
            }
            let ty = n.field.map(|p| &t.fields[p].ty);
            let element = |min: &str, max: &str, default: bool, xs: &str| {
                format!(
                    "      <xs:element minOccurs=\"{min}\" maxOccurs=\"{max}\"{} name=\"{}\" type=\"xs:{xs}\" />\r\n",
                    if default { " default=\"0\"" } else { "" },
                    n.xml_name
                )
            };
            match (n.kind, ty) {
                (ir::XmlNodeKind::Field, Some(ty)) if !n.min_zero => {
                    let line = match ty {
                        RtType::Integer => element("1", "1", true, "int"),
                        RtType::BigInteger => element("1", "1", true, "long"),
                        RtType::Text(_)
                        | RtType::Code(_)
                        | RtType::Decimal
                        | RtType::Date
                        | RtType::Boolean
                        | RtType::Option(_)
                        | RtType::DateTime
                        | RtType::Time
                        | RtType::Duration
                        | RtType::Guid => element("1", "1", false, "string"),
                        _ => return unsupported(shape()),
                    };
                    fields.push_str(&line);
                }
                (ir::XmlNodeKind::Text, _) if n.min_zero && schema.children(f).is_empty() && !n.big_text => {
                    fields.push_str(&element("0", "unbounded", false, "string"));
                }
                (ir::XmlNodeKind::FieldAttr, Some(RtType::Integer | RtType::Decimal | RtType::Boolean))
                | (ir::XmlNodeKind::TextAttr, _) => {
                    let used = match (n.kind, n.required) {
                        (_, None) => "required",
                        (ir::XmlNodeKind::FieldAttr, Some(false)) => "optional",
                        _ => return unsupported(shape()),
                    };
                    attrs.push_str(&format!(
                        "    <xs:attribute name=\"{}\" type=\"xs:string\" use=\"{used}\" />\r\n",
                        n.xml_name
                    ));
                }
                _ => return unsupported(shape()),
            }
        }
        if fields.is_empty() {
            return unsupported(shape());
        }
        let root = self.xp_export_container(inst, schema, top)?;
        let mut rows = String::new();
        for c in &root.children {
            let al_builtins::xmlport::Node::Elem(e) = c else {
                return unsupported(shape());
            };
            match al_builtins::xmlport::compact(e) {
                Some(r) => rows.push_str(&r),
                None => {
                    return unsupported(
                        "XmlPort export with InlineSchema of an empty value (how it is written is not pinned)",
                    );
                }
            }
        }
        // An optional table element without rows writes none (BdXmlPort.al:460 BdInlineSchema_NoRowsAttributes); one that is
        // not optional writes the record as it stands (:447 BdInlineSchema_NoRows), as `xp_export_table` does.
        if rows.is_empty() && !tn.min_zero {
            return unsupported("XmlPort export with InlineSchema and no rows (not pinned)");
        }
        let (root_name, table_name) = (&schema.node(top).xml_name, &tn.xml_name);
        let table_min = if tn.min_zero { "0" } else { "1" };
        Ok(format!(
            "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>\r\n<{root_name}>\r\n<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\r\n  <xs:complexType name=\"{table_name}\">\r\n    <xs:sequence>\r\n{fields}    </xs:sequence>\r\n{attrs}  </xs:complexType>\r\n  <xs:complexType name=\"{root_name}\" mixed=\"true\">\r\n    <xs:sequence>\r\n      <xs:element minOccurs=\"{table_min}\" maxOccurs=\"unbounded\" name=\"{table_name}\" type=\"{table_name}\" />\r\n    </xs:sequence>\r\n  </xs:complexType>\r\n  <xs:element name=\"{root_name}\" type=\"{root_name}\" />\r\n</xs:schema>{rows}</{root_name}>"
        ))
    }

    /// A text element with content: its attributes, then its children in schema order.
    pub(super) fn xp_export_container(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<Elem, Signal> {
        let mut out = Elem::new(&doc_name(schema, node));
        match self.xp_export_content(inst, schema, node, &mut out) {
            Err(Signal::Exit(None)) if Self::xp_quit(inst) => {}
            other => other?,
        }
        Ok(out)
    }

    pub(super) fn xp_export_content(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        out: &mut Elem,
    ) -> Result<(), Signal> {
        for child in schema.children(node) {
            let c = schema.node(child);
            // A Quit in a trigger of the child leaves it out and ends the document here.
            macro_rules! got {
                ($e:expr) => {
                    match $e {
                        Ok(v) => v,
                        Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(()),
                        Err(e) => return Err(e),
                    }
                };
            }
            match c.kind {
                ir::XmlNodeKind::TextAttr => {
                    let v = got!(self.xp_pass_variable(inst, schema, child));
                    // An empty `Occurrence = Optional` text attribute is left out, an empty Required one is written as `attr=""`
                    // (QrXpContainerOccurs.al:QS_TextAttr_Export_AllEmpty, QS_TextAttr_Export_OptionalSetRequiredEmpty).
                    if !(v.is_empty() && c.required == Some(false)) {
                        out.attrs.push((c.xml_name.clone(), v));
                    }
                }
                ir::XmlNodeKind::FieldAttr => {
                    let v = got!(self.xp_field_text(inst, schema, child));
                    // An Optional field attribute whose text is empty is left out, a Required one is written as `attr=""`
                    // (xmlport/lt-ra-xmlport/RaXmlPortExport.al:553 RA_FieldAttribute_Occurrence_Export_EmptyValues: Text, Code
                    // and a blank Date left out or empty, `OptNum="0"` and `OptFlag="No"` written).
                    if v.is_empty() && c.required == Some(false) {
                        continue;
                    }
                    out.attrs.push((c.xml_name.clone(), v));
                }
                ir::XmlNodeKind::Field => {
                    let v = got!(self.xp_field_text(inst, schema, child));
                    let empty = v.is_empty();
                    let mut e = Elem::new(&doc_name(schema, child));
                    e.children.push(Node::Text(v));
                    // The attributes and elements below a field element follow its value (AfXmlPortAttrChildren.al:255, 285, 313:
                    // `<Name Lang="en" Kind="K1">Alpha</Name>`, `<Name>Alpha<Child>c1</Child></Name>`).
                    if !schema.children(child).is_empty() {
                        // How an empty value is written before an element below it is not pinned.
                        if empty
                            && schema
                                .children(child)
                                .iter()
                                .any(|&g| matches!(schema.node(g).kind, ir::XmlNodeKind::Text | ir::XmlNodeKind::Field))
                        {
                            return unsupported(format!(
                                "XmlPort export of the field element {} with an empty value and an element below it (not pinned)",
                                c.xml_name
                            ));
                        }
                        let r = self.xp_export_content(inst, schema, child, &mut e);
                        out.children.push(Node::Elem(e));
                        r?;
                        if Self::xp_quit(inst) {
                            return Ok(());
                        }
                        continue;
                    }
                    out.children.push(Node::Elem(e));
                }
                ir::XmlNodeKind::Text if schema.children(child).is_empty() => {
                    let v = got!(self.xp_pass_variable(inst, schema, child));
                    // currXMLport.Skip() in its OnBeforePassVariable leaves the element out (xmlport/lt-y17-xmlport-ns/
                    // Y17XmlPortNamespaces.al `Opt`, Y17XmlPortNamespacesTests.al:60: no `<b:Opt />`). Break, BreakUnbound and
                    // Quit are handled where the flow is read (lt-ay-xmlport/AyXmlPortFlowTriggers.al Export_head_*).
                    if Self::xp_flow(inst) == Some(Flow::Skip) {
                        Self::xp_clear_flow(inst);
                        continue;
                    }
                    let mut e = Elem::new(&doc_name(schema, child));
                    e.children.push(Node::Text(v));
                    out.children.push(Node::Elem(e));
                }
                // A text element right below a table element in a pinned shape (`text_in_table`): its OnBeforePassVariable runs
                // before its attributes', its text follows its attributes and an empty text leaves the element empty
                // (QrXpContainerOccurs.al TO_AttrItem_Trigger_Export, TO_AttrItem_Plain_Export on branch al-runtime-rs-staging-pub-to);
                // with elements below it, its OnBeforePassVariable runs first and then its attributes and elements
                // (TO_TextContent_Export). Its own text next to elements below it is not pinned.
                // The nested shape (`TextInTable::Nested`, TP_Nested_Export on branch al-runtime-rs-staging-pub-tp): the outer
                // element's OnBeforePassVariable, its attributes, then the inner element as the text-only shape above
                // (`pass entry;pass status;pass admission;pass admcode;`, `<admission option_value="2" />` for an empty text).
                ir::XmlNodeKind::Text if text_in_table(schema, child).is_some() || nested_inner(schema, child) => {
                    let text = got!(self.xp_pass_variable(inst, schema, child));
                    if !text.is_empty()
                        && matches!(
                            text_in_table(schema, child),
                            Some(TextInTable::Content | TextInTable::Nested)
                        )
                    {
                        return unsupported(
                            "XmlPort export of a text element below a table element with both its own text and elements below it (not pinned)",
                        );
                    }
                    let mut e = Elem::new(&doc_name(schema, child));
                    let r = self.xp_export_content(inst, schema, child, &mut e);
                    // Only a Required text attribute of the inner element is pinned empty (`option_value=""`, TP_Nested_Export R3).
                    if r.is_ok() && nested_inner(schema, child) {
                        let unpinned = schema.children(child).into_iter().any(|a| {
                            let n = schema.node(a);
                            n.required != Some(true)
                                && !e.attrs.iter().any(|(name, v)| *name == n.xml_name && !v.is_empty())
                        });
                        if unpinned {
                            return unsupported(
                                "XmlPort export of an empty text attribute without Occurrence = Required of a text element inside a text element below a table element (not pinned)",
                            );
                        }
                    }
                    if !text.is_empty() {
                        e.children.push(Node::Text(text));
                    }
                    out.children.push(Node::Elem(e));
                    r?;
                    if Self::xp_quit(inst) {
                        return Ok(());
                    }
                }
                // A text element holding only attributes below a table element is written as any container (xmlport/
                // lt-sg-xmlport-occurs/SgXmlPortOccurs.al:78-110 SG_AttrBelowText_Export_*). Its own text beside a field
                // attribute is not pinned.
                ir::XmlNodeKind::Text
                    if schema.node(node).kind == ir::XmlNodeKind::Table
                        && attr_container(schema, child)
                        && !self.xp_pass_variable(inst, schema, child)?.is_empty() =>
                {
                    return unsupported(
                        "XmlPort export of a text element with a field attribute and its own text below a table element (not pinned)",
                    );
                }
                // `unlinked_container`: every outer row writes ALL the inner records, `<attributes />` when there are none
                // (QrXpContainerOccurs.al TP_Unlinked_Export, TP_Unlinked_Export_NoAttrs on branch al-runtime-rs-staging-pub-tp).
                // `unlinked_reply_container`: the inner table writes the rows it holds when it is reached, so an outer row's
                // OnAfterGetRecord that empties and refills it gives each outer row its own rows; an optional inner table without rows
                // leaves the text element empty (`<details />`) (QrXpContainerOccurs.al TR_Booking_Reply_* on branch
                // al-runtime-rs-staging-tr).
                ir::XmlNodeKind::Text
                    if unlinked_container(schema, child, node)
                        || unlinked_reply_container(schema, child, node)
                        || default_unlinked_container(schema, child, node) =>
                {
                    // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Cards_DefaultOccurs_Export (was assumption `xmlport-default-container-unlinked`).
                    let e = self.xp_export_container(inst, schema, child)?;
                    out.children.push(Node::Elem(e));
                    if Self::xp_quit(inst) {
                        return Ok(());
                    }
                }
                // `attrs_over_unlinked`: always written, its text attributes (an empty Required one as `attr=""`) and then the inner
                // text element, which holds the rows of the inner table, or that table once with the record as it stands when it
                // has none (QrXpContainerOccurs.al TT_Change_Export_TwoRows, _OneRow, _TokenNoRows, _Error on corpus writer TT's
                // branch al-runtime-rs-staging-tt: `<Ticket ChangeRequestToken="" Kind="K"><Admissions><Admission Code="">...`).
                // Its own text is not pinned.
                ir::XmlNodeKind::Text if attrs_over_unlinked(schema, child, node) => {
                    // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Change_Export_TwoRows, TT_Change_Export_TokenNoRows, TT_Change_Export_Error (was assumption `xmlport-attrs-over-unlinked`).
                    if !got!(self.xp_pass_variable(inst, schema, child)).is_empty() {
                        return unsupported(
                            "XmlPort export of a text element with text attributes over a text element that holds a table element, with its own text (not pinned)",
                        );
                    }
                    let e = self.xp_export_container(inst, schema, child)?;
                    out.children.push(Node::Elem(e));
                    if Self::xp_quit(inst) {
                        return Ok(());
                    }
                }
                ir::XmlNodeKind::Text
                    if schema.node(node).kind == ir::XmlNodeKind::Table
                        && !linked_container(schema, child, node)
                        && !attr_container(schema, child)
                        && !plain_container(schema, child) =>
                {
                    return unsupported("XmlPort export of a text element with content inside a table element");
                }
                ir::XmlNodeKind::Text => {
                    let e = self.xp_export_container(inst, schema, child)?;
                    out.children.push(Node::Elem(e));
                    if Self::xp_quit(inst) {
                        return Ok(());
                    }
                }
                ir::XmlNodeKind::Table => {
                    for e in self.xp_export_table(inst, schema, child)? {
                        out.children.push(Node::Elem(e));
                    }
                    if Self::xp_quit(inst) {
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    /// `OnBeforePassVariable`, then the variable's text
    /// (xmlport/TestXmlPortAdvanced.al:XmlPort_TextVariableTriggers_ExportManipulatesTextAndAttributes).
    pub(super) fn xp_pass_variable(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<String, Signal> {
        self.xp_run_trigger(inst, schema, node, "onbeforepassvariable")?;
        let slot = schema.node(node).slot.expect("text nodes have a slot");
        match &inst.borrow().globals[slot] {
            Value::Text(s) | Value::Code(s) => Ok(s.to_string()),
            // A BigText element writes its whole text (xmlport/lt-xmlport-bigtext/XmlPortBigText.al:178, :198).
            Value::Library(al_builtins::library_kind::LibraryKind::BigText, h) => match &*h.borrow() {
                al_builtins::value::LibraryValue::Empty => Ok(String::new()),
                al_builtins::value::LibraryValue::Text(s) => Ok(s.clone()),
                _ => unsupported("XmlPort export of a BigText variable of this kind"),
            },
            other => unsupported(format!("XmlPort export of the variable {}", other.describe())),
        }
    }

    /// `OnBeforePassField` (it runs for every row: xmlport/TestXmlPortTriggersReplaceViewContracts.al:
    /// Export_OnBeforePassField_Runs_AndSetTableViewFiltersAndOrders), then the field's text: the
    /// display text of its type
    /// (xmlport/TestXmlPortExportImportContracts.al:XmlPortExport_Document_FullText). A blank option
    /// is a single space.
    pub(super) fn xp_field_text(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<String, Signal> {
        self.xp_site(inst, schema, node, "onbeforepassfield")?;
        self.xp_field_value_text(inst, schema, node)
    }

    /// The field of a text-format export: Skip leaves it out of the record, Break leaves it out and ends the record
    /// (corpus P112XpCsv.al:P112_Export_Skip and _Break, idBefore2 and nameBefore2).
    pub(super) fn xp_field_pass(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<FieldPass, Signal> {
        match self.xp_site(inst, schema, node, "onbeforepassfield")? {
            Some(Flow::Skip) => return Ok(FieldPass::Omit),
            Some(Flow::Break) => return Ok(FieldPass::Stop),
            _ => {}
        }
        Ok(FieldPass::Value(self.xp_field_value_text(inst, schema, node)?))
    }

    pub(super) fn xp_field_value_text(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<String, Signal> {
        let n = schema.node(node);
        let rec = self.xp_record(inst, schema.node(n.table_node.expect("field nodes have a table node")))?;
        let pos = n.field.expect("field nodes have a field");
        // A FlowField is calculated for the export (assumption xmlport-exports-flowfield-calculated).
        // AutoCalcField decides it when declared (xmlport/lt-y-xmlport-props/YXmlPortProps.al:
        // NodeProperties_Export_NamesPrefixesAutoCalc: true calculates, false writes the field as it is).
        if !self.table(&rec)?.fields[pos].stored && n.auto_calc != Some(false) {
            if n.auto_calc.is_none() {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::XmlportExportsFlowfieldCalculated,
                );
            }
            self.ret_used = false;
            self.record_op(RecordOp::CalcFields, &rec, &mut [Value::Integer(pos as i32)])?;
        }
        let v = rec.borrow().fields[pos].clone();
        // A column the platform row leaves unknown is not exported.
        if let Some(why) = v.unknown() {
            return Err(Signal::Unsupported(why.to_string()));
        }
        if schema.xp().format_evaluate_xml {
            return self.xp_field_text_xml(&v);
        }
        // An Option field with an OptionCaption writes the caption (corpus
        // xmlport/PWXmlPortFormatEvaluate.al:Export_Legacy_TimeDurationOptionEnum: `Beta cap`).
        if let Value::Option(o, _) = &v {
            let t = self.table(&rec)?;
            if let Some((_, members)) = self.members_of_field(&t, pos)? {
                if let Some((_, _, caption)) = members.get(*o as usize) {
                    return Ok(if caption.is_empty() {
                        " ".to_string()
                    } else {
                        caption.clone()
                    });
                }
            }
        }
        match &v {
            Value::Integer(i) => Ok(i.to_string()),
            // A BigInteger without digit grouping and a Guid in braces (xmlport/lt-bd-xmlport/BdXmlPort.al:425
            // BdInlineSchema_FieldTypes: `9876543210`, `{5A2F6C9E-...}`; :436 `0`, `{00000000-...}`). A negative BigInteger
            // is not pinned.
            Value::BigInteger(i) if *i >= 0 => Ok(i.to_string()),
            Value::Guid(g) => Ok(al_builtins::format::format_guid(g)),
            Value::Text(s) | Value::Code(s) => Ok(s.to_string()),
            Value::Decimal(_) => al_builtins::xmlport::field_text(&v)
                .or_else(|why| unsupported(format!("XmlPort export of {why} (the text form is not pinned)"))),
            Value::Boolean(_) | Value::Date(..) | Value::Time(_) | Value::DateTime(_) | Value::Duration(_) => {
                al_builtins::format::try_format(&v, 0, 0, self)
                    .or_else(|why| unsupported(format!("XmlPort export of a value: {why}")))
            }
            Value::Option(..) | Value::Enum(..) => {
                let t = al_builtins::format::try_format(&v, 0, 0, self)
                    .or_else(|why| unsupported(format!("XmlPort export of a value: {why}")))?;
                Ok(if t.is_empty() { " ".to_string() } else { t })
            }
            other => unsupported(format!(
                "XmlPort export of {} (the text form is not pinned)",
                other.describe()
            )),
        }
    }

    /// The XML forms `FormatEvaluate = Xml` writes (xmlport/TestXmlPortFormatEvaluateContracts.al:
    /// Export_Xml_WritesTheXmlForms): `2024-03-05`, `1234.50`, `true`, `2024-03-05T12:34:56Z`. Other
    /// types are not pinned.
    pub(super) fn xp_field_text_xml(&mut self, v: &Value) -> Result<String, Signal> {
        match v {
            Value::Integer(i) => Ok(i.to_string()),
            Value::Text(s) | Value::Code(s) => Ok(s.to_string()),
            Value::Decimal(_) => al_builtins::xmlport::field_text(v)
                .map(|t| t.replace(',', ""))
                .or_else(|why| unsupported(format!("XmlPort export of {why} (the text form is not pinned)"))),
            Value::Boolean(b) => Ok(if *b { "true" } else { "false" }.to_string()),
            // A Time or DateTime with milliseconds writes them without trailing zeros, the DateTime with `Z` (writer TU's branch
            // al-runtime-rs-staging-tu, xmlport/PWXmlPortFormatEvaluate.al TU_Export_TimeMilliseconds: `12:34:56.789`,
            // `12:34:56.5`, `12:34:56.007`, `2024-03-05T12:34:56.789Z`; former assumption `xmlport-time-ms-xml` (pinned on corpus master since bump 1030)).
            Value::Date(..) | Value::DateTime(_) => {
                // Pinned by xmlport/PWXmlPortFormatEvaluate.al TU_Export_TimeMilliseconds (was assumption `xmlport-time-ms-xml`).
                al_builtins::format::try_format(v, 0, 9, self)
                    .or_else(|why| unsupported(format!("XmlPort export of a value: {why}")))
            }
            // corpus xmlport/PWXmlPortFormatEvaluate.al:Export_Xml_TimeDurationOptionEnum: `12:34:56`,
            // `P1DT1H1M1.001S`, and the ordinal of an Option or Enum.
            Value::Time(0) => Ok(String::new()),
            // A zero Duration is `P0DT0H0M0.0S` (writer TU's branch al-runtime-rs-staging-tu, PWXmlPortFormatEvaluate.al
            // TU_Export_TimeMilliseconds `<Dur>P0DT0H0M0.0S</Dur>`; former assumption `xmlport-time-ms-xml` (pinned on corpus master since bump 1030)).
            Value::Duration(0) => {
                // Pinned by xmlport/PWXmlPortFormatEvaluate.al TU_Export_TimeMilliseconds (was assumption `xmlport-time-ms-xml`).
                Ok("P0DT0H0M0.0S".to_string())
            }
            Value::Time(_) => {
                // Pinned by xmlport/PWXmlPortFormatEvaluate.al TU_Export_TimeMilliseconds (was assumption `xmlport-time-ms-xml`).
                al_builtins::format::try_format(v, 0, 9, self)
                    .or_else(|why| unsupported(format!("XmlPort export of a value: {why}")))
            }
            Value::Duration(d) => Ok(iso_duration(*d)),
            Value::Option(i, _) => Ok(i.to_string()),
            Value::Enum(_, o) => Ok(o.to_string()),
            // `{UPPER-CASE}`, the null Guid too (QrXpContainerOccurs.al TS_Guid_Export on branch al-runtime-rs-staging-pub-ts).
            Value::Guid(g) => {
                // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Guid_Import_Element, TS_Guid_Import_Attribute, TS_Guid_Export (was assumption `xmlport-guid-xml`).
                Ok(al_builtins::format::format_guid(g))
            }
            other => unsupported(format!(
                "XmlPort export of {} with FormatEvaluate = Xml (the XML form is not pinned)",
                other.describe()
            )),
        }
    }
}

/// A separator property: `<LF>`, `<CR>` and `<TAB>` stand for the control characters, other text
/// is itself. Other `<...>` names are not pinned.
/// A declared separator as the getter shows it: `<None>` is empty, `<CR/LF>`, `<LF>`, `<CR>` and `<Tab>` are their escapes, anything
/// else is as written (P94XpDelim.al:Getters_Declared, Getters_DeclaredTwoTables). Other tokens are not pinned.
/// The name of node `c` in the document: a field element with a `NamespacePrefix` is `prefix:Name`.
/// A `Format = Xml` port whose root is a table element of plain field elements, without triggers, attributes, a link or
/// a MinOccurs (xmlports 87562 and 87563 of platform/lt-ag-contoso/AgXmlPortImport.al:784-833, the DemoTool's xmlport
/// 101899). Other shapes of a table element root are not pinned.
/// A table element root with MaxOccurs = Once, not linked and not optional (see `xp_export` for where it is pinned).
pub(super) fn once_root(schema: &Schema, top: usize) -> bool {
    let n = schema.node(top);
    schema.xp().format == ir::XmlFormat::Xml
        && n.kind == ir::XmlNodeKind::Table
        && n.max_unbounded == Some(false)
        && n.link_node.is_none()
        && !n.min_zero
        && !schema.children(top).is_empty()
}

/// The namespace declarations of an exported root. The prefixed entries come first, in declaration order, before the root's
/// own attributes (xmlport/lt-bd-xmlport/BdXmlPort.al:407). The default namespace follows the root's own attributes, last:
/// with UseDefaultNamespace the port's (BC's namespace of the port without DefaultNamespace: BdXmlPort.al:511,
/// DgXmlPortLeads.al:290; after the root's attributes: xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al TF_NsAttr_Export
/// `<Root Ver="v1" xmlns="...">`), else the uri of the empty prefix `""`, whether `""` is listed first or last
/// (xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespacesTests.al:60, :71); a used default namespace replaces that uri (:82).
pub(super) fn declare_namespaces(root: &mut Elem, namespaces: &[(String, String)], default_ns: Option<&str>) {
    for (at, (prefix, uri)) in namespaces.iter().filter(|(p, _)| !p.is_empty()).enumerate() {
        root.attrs.insert(at, (format!("xmlns:{prefix}"), uri.clone()));
    }
    let empty = namespaces.iter().find(|(p, _)| p.is_empty()).map(|(_, u)| u.as_str());
    if let Some(ns) = default_ns.or(empty) {
        root.attrs.push(("xmlns".into(), ns.to_string()));
    }
}
