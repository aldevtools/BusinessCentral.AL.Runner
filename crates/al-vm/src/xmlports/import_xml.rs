//! Import of a Format = Xml port: the document, containers, attributes and fields.

use super::*;

impl Session {
    pub(super) fn xp_import(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        guarded: bool,
        static_form: bool,
    ) -> Result<Value, Signal> {
        // An import ignores the SourceTableView of an element that is not linked (platform/lt-ag-contoso/AgXmlPortImport.al:62-80);
        // a filter on a linked one is not pinned on import. An XML import refuses it when it reaches an element of that table element
        // (`xp_import_record`): an import never touches the record of a table element the document has no element of.
        if let Some(n) = schema.xp().nodes.iter().find(|n| {
            schema.xp().format != ir::XmlFormat::Xml
                && n.link_node.is_some()
                && n.source_view
                    .as_deref()
                    .is_some_and(|v| v.to_ascii_lowercase().contains("where"))
        }) {
            return unsupported(format!(
                "XmlPort import: sourcetableview with a filter on the linked tableelement {} is not pinned",
                n.name
            ));
        }
        // A second Import on an instance that imported reads nothing and stores nothing: only OnPreXmlPort and OnPostXmlPort run
        // again, also after SetSource on the same or a new stream, with or without Codeunit.Run (xmlport/lt-xmlport-query-datatransfer/XpTStaticAndInstance.al:74 Instance_ImportTwiceSameSource:
        // nothing stored again; QrXpContainerOccurs.al
        // TT_ImportTwice_SameSource, _SameStreamAgain, _NewSourceOtherRows, _NewSourceSameRows, _Direct on corpus writer TT's
        // branch al-runtime-rs-staging-tt; former assumption `xmlport-import-again` (pinned on corpus master since bump 1030)). Pinned for an unguarded XML import after one that
        // succeeded; the rest is refused.
        if inst.borrow().xp_text.iter().any(|(k, _)| *k == XP_IMPORTED) {
            if guarded || schema.xp().format != ir::XmlFormat::Xml {
                return unsupported(
                    "a second Import on the same XmlPort instance, guarded or of a Format other than Xml (not pinned)",
                );
            }
            if inst.borrow().xp_text.iter().any(|(k, _)| *k == XP_IMPORT_FAILED) {
                return unsupported(
                    "a second Import on the same XmlPort instance after an Import that failed (not pinned)",
                );
            }
            // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_ImportTwice_SameSource, TT_ImportTwice_SameStreamAgain, TT_ImportTwice_NewSourceOtherRows, TT_ImportTwice_NewSourceSameRows, TT_ImportTwice_Direct (was assumption `xmlport-import-again`).
            // A schema without a table element reads the new source again, its globals kept (al-runner issue 6657; awaiting
            // xmlport/lt-y66-xmlport-var-members/Y66XmlPortVarMembers.al:235-277 `got = a;a;`).
            if schema.xp().nodes.iter().any(|n| n.table.is_some()) {
                return Ok(Value::Null);
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlPortImportAgainTextOnly);
        }
        if guarded {
            // A guarded import needs a transaction world of its own, which BC refuses to open
            // while the caller holds an uncommitted write; the static form turns the refusal
            // into false, the instance form raises it
            // (xmlport/TestXmlPortImportWriteTx.al:StaticGuardedImport_WithUncommittedWrite_ReturnsFalse_AndImportsNothing,
            // :InstanceGuardedImport_WithUncommittedWrite_IsRefused_AndImportsNothing).
            if self.in_write_transaction()? {
                if self.tx_model == TxModel::AutoRollback {
                    return unsupported("guarded XmlPort.Import with a pending write under AutoRollback (not pinned)");
                }
                return if static_form {
                    // Whether the trapped refusal becomes the last error, and with what text, is not
                    // pinned: reading it is refused.
                    self.last_error = al_builtins::native::unpinned_error();
                    Ok(Value::Boolean(false))
                } else {
                    Err(crate::transactions::transaction_stopped())
                };
            }
            if self.tx.try_depth > 0 {
                return unsupported("guarded XmlPort.Import inside a TryFunction (not pinned)");
            }
        }
        // Import begins a transaction of its own, so a write is allowed inside it even under
        // TransactionModel::None, and the transaction ends with the import
        // (codeunit/TestWriteTxTestBoundary.al:WriteTx_Test12_AnXmlPortImportMayWriteUnderNone).
        let began_world = self.no_transaction();
        self.tx.tx_world += 1;
        let result = self.xp_import_document(inst, schema);
        self.tx.tx_world -= 1;
        if result.is_err() && inst.borrow().xp_text.iter().any(|(k, _)| *k == XP_IMPORTED) {
            inst.borrow_mut().xp_text.push((XP_IMPORT_FAILED, String::new()));
        }
        // A Quit ends the import with the records stored so far kept (P112_Import_Quit); `xp_run` reads the request.
        let result = match result {
            Err(Signal::Exit(None)) if Self::xp_flow(inst) == Some(Flow::Quit) => Ok(()),
            other => other,
        };
        match result {
            Ok(()) if guarded => {
                // The import's own transaction ends with it, so a second guarded import is not
                // blocked by the first one's rows
                // (xmlport/TestXmlPortImportWriteTx.al:GuardedImport_LeavingImportedRows_DoesNotBlockTheNextGuardedImport).
                if self.tx_model != TxModel::AutoRollback {
                    self.commit_transaction();
                }
                Ok(Value::Boolean(true))
            }
            Ok(()) => {
                if began_world {
                    self.commit_transaction();
                }
                Ok(Value::Null)
            }
            // A guarded import that fails returns false and loses its writes; the error stays
            // readable (xmlport/TestXmlPortImportWriteTx.al:StaticGuardedImport_FailingSecondElement_ReturnsFalse_AndRollsBackTheFirstRow).
            Err(Signal::Error(e)) if guarded => {
                self.rollback_transaction();
                self.error_finals = None;
                self.last_error = *e;
                Ok(Value::Boolean(false))
            }
            Err(other) => Err(other),
        }
    }

    pub(super) fn xp_import_document(&mut self, inst: &Inst, schema: &Schema) -> Result<(), Signal> {
        let xp = schema.xp();
        let stashed = {
            let mut i = inst.borrow_mut();
            let at = i.xp_text.iter().position(|(k, _)| *k == 12);
            at.map(|a| i.xp_text.remove(a).1)
        };
        let text = match stashed {
            Some(t) => t,
            None => self.xp_read_source(inst, xp)?,
        };
        // `xp_import` answers a second Import before it gets here.
        inst.borrow_mut().xp_text.push((XP_IMPORTED, String::new()));
        // xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_MalformedXml_Throws
        if text.is_empty() {
            return Err(xerr(
                "Cannot import the specified XML document because the file is empty.",
                "XmlPortData",
            ));
        }
        let tops: Vec<usize> = (0..xp.nodes.len()).filter(|&i| xp.nodes[i].parent.is_none()).collect();
        let [top] = tops[..] else {
            return unsupported("XmlPort import with more than one top-level element (not pinned)");
        };
        if xp.text_encoding.as_deref() == Some("msdos") {
            return unsupported("XmlPort import into a VariableText port with TextEncoding = MSDOS (not pinned)");
        }
        // An import of a text port whose root is a table element is refused by BC, an export is not
        // (xmlport/lt-xmlport-query-datatransfer/XpTVariableText.al:Csv_ImportBasic and the other Csv_Import tests).
        if xp.format != ir::XmlFormat::Xml && xp.nodes[top].kind == ir::XmlNodeKind::Table {
            return Err(xerr(
                "If you choose XMLport format Fixed Text or Variable Text, then the root element must have source type Text.",
                "XmlPortInvalidNodeType",
            ));
        }
        // The root error above is pinned for WINDOWS and UTF16 (XpTEncoding.al:165-205); an import that gets further is not.
        if let Some(enc) = &xp.text_encoding {
            return unsupported(format!(
                "XmlPort import into a VariableText port with TextEncoding = {} (not pinned)",
                enc.to_ascii_uppercase()
            ));
        }
        let table_root = root_table_element(schema, top);
        if !table_root && (xp.nodes[top].kind != ir::XmlNodeKind::Text || schema.children(top).is_empty()) {
            return unsupported("XmlPort import whose root is not a text element with content (not pinned)");
        }
        // A table view filters the rows the xmlport reads; what it does to an import is not pinned.
        for n in xp.nodes.iter().filter(|n| n.kind == ir::XmlNodeKind::Table) {
            if !self.xp_record(inst, n)?.borrow().filters.is_empty() {
                return unsupported("XmlPort import into a table element with a table view filter (not pinned)");
            }
        }
        if xp.format != ir::XmlFormat::Xml {
            return self.xp_import_text(inst, schema, top, &text);
        }

        // PreserveWhiteSpace with a default namespace imports as without one (corpus PR 896, misc/lt-np4-probes/Np4XmlPort.al
        // Y9_XmlPort_Import_PreserveWhiteSpace_WithNamespace).
        let parsed = match &xp.default_ns {
            None => al_builtins::xmlport::parse(&text),
            Some(ns) => self.xp_parse_in_namespace(schema, top, table_root, &text, ns)?,
        };
        let root = match parsed {
            Ok(r) => r,
            Err(ReadFailure::Malformed(Some(message))) => return Err(xerr(message, "XmlPortImport:Xml")),
            Err(ReadFailure::Malformed(None)) => {
                return unsupported("XmlPort import of a malformed document whose error text is not pinned");
            }
            // A port without namespaces does not find a root element that is in one, by a default namespace or a prefix
            // (xmlport/lt-xmlport-query-datatransfer/XpTNamespaces.al:Import_NoNamespaceXmlportWithNamespacedInput and, for a
            // port whose DefaultNamespace is not used, Import_DefaultNsMatching, _DefaultNsWrong, _DefaultNsViaPrefixDeclaration;
            // xmlport/lt-bd-xmlport/BdXmlPort.al:315 BdDefaultNamespace_OnRoot, strict and lax; pages/lt-report-page-properties/
            // 06-xmlport-dnsfalse.al:45). Pinned for a text element root of the document's local name.
            Err(ReadFailure::Unsupported(why)) => {
                if let Some(local) = why.strip_prefix(al_builtins::xmlport::ROOT_IN_NAMESPACE) {
                    if xp.namespaces.is_empty()
                        && xp.nodes.iter().all(|n| n.ns_prefix.is_none())
                        && xp.nodes[top].kind == ir::XmlNodeKind::Text
                        && local == xp.nodes[top].xml_name
                    {
                        return Err(xerr(
                            format!("The element <{local}> cannot be found in the object description."),
                            "XmlPortElementNotFound",
                        ));
                    }
                    return unsupported(
                        "XmlPort import of a document whose root element is in a namespace (not pinned)",
                    );
                }
                return unsupported(format!("XmlPort import of {why}"));
            }
        };
        if xp.inline_schema {
            return unsupported(
                "XmlPort import into a port with InlineSchema = true of a document that parses (not pinned)",
            );
        }
        // A V11 port is measured only with a `1.1` declaration, which the reader refuses (YXmlPortProps.al:621).
        if xp.xml_v11 {
            return unsupported(
                "XmlPort import into a port with XmlVersionNo = V11 of a document that parses (not pinned)",
            );
        }
        // Only the prefixes the port declares are pinned (YXmlPortProps.al:NodeProperties_Import_NamesPrefixes).
        fn foreign_prefix(e: &Elem, declared: &[(String, String)]) -> bool {
            e.name
                .split_once(':')
                .is_some_and(|(p, _)| !declared.iter().any(|(q, _)| q == p))
                || e.attrs.iter().any(|(n, _)| {
                    n.strip_prefix("xmlns:")
                        .is_some_and(|p| !declared.iter().any(|(q, _)| q == p))
                })
                || e.elements().any(|c| foreign_prefix(c, declared))
        }
        // With PreserveWhiteSpace, white space between elements is passed over (corpus PR 896, misc/lt-np4-probes/Np4XmlPort.al
        // Y9_XmlPort_Import_PreserveWhiteSpace_NoNamespace: indented rows import as `rows=3`); other text between them is not
        // pinned.
        if xp.preserve_whitespace {
            fn spaced(e: &Elem) -> bool {
                (e.elements().next().is_some()
                    && e.children
                        .iter()
                        .any(|c| matches!(c, Node::Text(t) if !t.trim().is_empty())))
                    || e.elements().any(spaced)
            }
            if spaced(&root) {
                return unsupported(
                    "XmlPort import with PreserveWhiteSpace of white space between elements (not pinned)",
                );
            }
        }
        // An import port with the empty prefix `""` among its Namespaces fails on BC with a schema error that names an element
        // (xmlport/lt-y17-xmlport-ns/Y17XmlPortNamespacesTests.al:178, `The 'urn:lt-d:Name' element is not declared.`); which
        // element it names is not modelled.
        if xp.namespaces.iter().any(|(p, _)| p.is_empty()) {
            return unsupported("XmlPort import with the empty prefix \"\" among its Namespaces (not modelled)");
        }
        // A prefix on a text or table element is pinned for the export only (Y17XmlPortNamespacesTests.al:60).
        if xp
            .nodes
            .iter()
            .any(|n| n.ns_prefix.is_some() && matches!(n.kind, ir::XmlNodeKind::Text | ir::XmlNodeKind::Table))
        {
            return unsupported("XmlPort import with a NamespacePrefix on a text or table element (not pinned)");
        }
        // A port without namespaces ignores a prefix that the root declares and no element uses
        // (XpTNamespaces.al:Import_NoNamespaceXmlportWithUnusedPrefixDecl, `xmlns:z` on the root: imported).
        let mut root = root;
        if xp.namespaces.is_empty() {
            fn uses_prefix(e: &Elem, p: &str) -> bool {
                e.name.split_once(':').is_some_and(|(q, _)| q == p) || e.elements().any(|c| uses_prefix(c, p))
            }
            let unused: Vec<String> = root
                .attrs
                .iter()
                .filter_map(|(n, _)| n.strip_prefix("xmlns:"))
                .filter(|p| !uses_prefix(&root, p))
                .map(|p| format!("xmlns:{p}"))
                .collect();
            root.attrs.retain(|(n, _)| !unused.contains(n));
        }
        if foreign_prefix(&root, &xp.namespaces) {
            return unsupported("XmlPort import of a document with namespaces the port does not declare (not pinned)");
        }
        // An element that carries a prefix of the port needs the port's declaration on the root; other layouts are not pinned.
        for (prefix, uri) in &xp.namespaces {
            fn uses(e: &Elem, p: &str) -> bool {
                e.name.starts_with(p) || e.elements().any(|c| uses(c, p))
            }
            let marker = format!("{prefix}:");
            if uses(&root, &marker) && root.attr(&format!("xmlns:{prefix}")) != Some(uri.as_str()) {
                return unsupported(format!(
                    "XmlPort import of a document that uses the prefix {prefix} without declaring it on the root (not pinned)"
                ));
            }
        }
        if root.name != xp.nodes[top].xml_name {
            return Err(schema_error(format!("The '{}' element is not declared.", root.name)));
        }
        // A table element as the root is one row: its fields are validated and the row is inserted, an empty element
        // fails the schema check (platform/lt-ag-contoso/AgXmlPortImport.al:514 QX_RootTableElement_Import).
        if table_root {
            return self.xp_import_record(inst, schema, top, &root, true, false);
        }
        self.xp_import_container(inst, schema, top, &root)
    }

    /// The root element of an import into a port with `UseDefaultNamespace = true` and no `DefaultNamespace`, whose elements
    /// are all in BC's namespace of the port (xmlport/lt-bd-xmlport/BdXmlPort.al:499 SqUseDefaultNamespace_NoDefaultNamespace,
    /// xmlport 89580): a root in that namespace, by default or by a prefix, is read with local names; a root in another
    /// namespace or in none is not found; a first child in no namespace below a root with one child fails the schema check.
    /// Pinned for a text element root whose name the document's root has.
    pub(super) fn xp_parse_in_namespace(
        &self,
        schema: &Schema,
        top: usize,
        table_root: bool,
        text: &str,
        ns: &str,
    ) -> Result<Result<Elem, ReadFailure>, Signal> {
        use al_builtins::xmlport::InNamespace;
        let xp = schema.xp();
        let root_node = &xp.nodes[top];
        if table_root || root_node.kind != ir::XmlNodeKind::Text {
            return unsupported(
                "XmlPort import with UseDefaultNamespace into a port whose root is not a text element (not pinned)",
            );
        }
        match al_builtins::xmlport::parse_in_namespace(text, ns) {
            Ok(InNamespace::Tree(e)) => Ok(Ok(e)),
            Ok(InNamespace::RootElsewhere(local)) if local == root_node.xml_name => Err(xerr(
                format!("The element <{local}> cannot be found in the object description."),
                "XmlPortElementNotFound",
            )),
            Ok(InNamespace::FirstChildInNone { root, child })
                if root == root_node.xml_name
                    && matches!(schema.children(top)[..], [only] if xp.nodes[only].xml_name == child) =>
            {
                Err(schema_error(format!(
                    "The element '{root}' in namespace '{ns}' has invalid child element '{child}'. List of possible elements expected: '{child}' in namespace '{ns}'."
                )))
            }
            // An attribute of the root prefixed into the port's namespace is not the port's attribute, which is in no namespace:
            // the schema check fails before anything is imported (writer TF's branch al-runtime-rs-staging-pub-tf,
            // xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al TF_NsAttr_Import_Qualified `prefixed attributes`, a declared
            // text attribute of the root). Pinned for a port without DefaultNamespace (BC's own namespace), the case
            // `xmlport-udn-text-attribute` registers.
            Ok(InNamespace::RootAttributeInNamespace(attr))
                if xp
                    .default_ns
                    .as_deref()
                    .is_some_and(|d| d.starts_with("urn:microsoft-dynamics-nav/xmlports/x"))
                    && schema
                        .children(top)
                        .iter()
                        .any(|&c| xp.nodes[c].kind == ir::XmlNodeKind::TextAttr && xp.nodes[c].xml_name == attr) =>
            {
                Err(schema_error(format!("The '{ns}:{attr}' attribute is not declared.")))
            }
            Ok(_) => {
                unsupported("XmlPort import with UseDefaultNamespace of a document of a shape the corpus does not pin")
            }
            Err(e) => Ok(Err(e)),
        }
    }

    /// The text of the source stream from its read position, which moves to the end. A byte order
    /// mark decides the encoding, else the text is UTF-8; the stream's own encoding does not matter.
    pub(super) fn xp_read_source(&mut self, inst: &Inst, xp: &ir::XmlPort) -> Result<String, Signal> {
        let Value::Library(_, h) = inst.borrow().globals[xp.source_slot].clone() else {
            return unsupported("XmlPort import without SetSource (a file dialog is not supported)");
        };
        let mut g = h.borrow_mut();
        let LibraryValue::InStream(st) = &mut *g else {
            return unsupported("XmlPort import without SetSource (a file dialog is not supported)");
        };
        // An XML import reads the whole stream, also one read to its end before SetSource, and leaves its read position where it
        // was (QrXpContainerOccurs.al TT_Import_SourceReadBefore, TT_ImportTwice_SameSource `source at end` No, on corpus writer TT's
        // branch al-runtime-rs-staging-tt; former assumption `xmlport-import-whole-source` (pinned on corpus master since bump 1030)). From a position inside the stream it is not
        // pinned. A text-format import takes the bytes from the read position on (as before those pins).
        let bytes = if xp.format == ir::XmlFormat::Xml {
            let (pos, len) = (st.pos, st.len());
            if pos != 0 && pos < len {
                return unsupported("XmlPort import of a source stream read partly before (not pinned)");
            }
            // Pinned by xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Import_SourceReadBefore, TT_ImportTwice_SameSource (was assumption `xmlport-import-whole-source`).
            st.pos = 0;
            let all = st.rest();
            st.pos = pos;
            all
        } else {
            st.drain()
        };
        if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
            let units: Vec<u16> = rest.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
            return utf16_declared_utf8(String::from_utf16_lossy(&units));
        }
        // A UTF-16 little endian document without a byte order mark that starts with `<` is read as UTF-16 (a TextEncoding::UTF16
        // OutStream writes no byte order mark and the document declares encoding="UTF-16": xmlport/lt-xmlport-linktable/
        // XmlPortLinkTableNoLinkFields.al, Import runner at line 395 and BothTemporary_Import_OneParentOneChild).
        if bytes.len() >= 2 && bytes[0] == b'<' && bytes[1] == 0 {
            let units: Vec<u16> = bytes
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect();
            return utf16_declared_utf8(String::from_utf16_lossy(&units));
        }
        // A UTF-16 big endian document, with the byte order mark FE FF or without one when it starts with `<`, is read as UTF-16
        // when it declares utf-16 or (without a mark) nothing; one without a mark that declares utf-8 fails at the first NUL after
        // the declaration (xmlport/lt-ah-xmlport/AhXmlPort.al:543-597, BigEndian_*: `'.', hexadecimal value 0x00, is an invalid
        // character. Line 1, position 39.` after the 38 characters of the declaration). Other declarations are not pinned.
        let bom_be = bytes.starts_with(&[0xFE, 0xFF]);
        if bom_be || (bytes.len() >= 2 && bytes[0] == 0 && bytes[1] == b'<') {
            let body = if bom_be { &bytes[2..] } else { &bytes[..] };
            let units: Vec<u16> = body.chunks_exact(2).map(|p| u16::from_be_bytes([p[0], p[1]])).collect();
            let text = String::from_utf16_lossy(&units);
            let declared = text
                .strip_prefix("<?xml")
                .and_then(|_| text.split_once("?>"))
                .map(|(decl, rest)| {
                    (
                        decl.to_ascii_lowercase(),
                        decl.chars().count() + 2,
                        rest.starts_with('<'),
                    )
                });
            let says = |d: &str, enc: &str| {
                d.contains(&format!("encoding=\"{enc}\"")) || d.contains(&format!("encoding='{enc}'"))
            };
            return match declared {
                Some((d, _, _)) if says(&d, "utf-16") => Ok(text),
                None if !bom_be => Ok(text),
                Some((d, len, true)) if !bom_be && says(&d, "utf-8") && !d.contains('\n') && !d.contains('\r') => {
                    Err(xerr(
                        format!(
                            "'.', hexadecimal value 0x00, is an invalid character. Line 1, position {}.",
                            len + 1
                        ),
                        "XmlPortImport:Xml",
                    ))
                }
                _ => unsupported("XmlPort import of a UTF-16 big endian document with this declaration (not pinned)"),
            };
        }
        let rest = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
        match std::str::from_utf8(rest) {
            // A document that declares utf-16 in bytes without a UTF-16 byte order mark cannot be read
            // (xmlport/lt-xmlport-query-datatransfer/XpTImportBasic.al:Import_PrologUtf16Declared).
            Ok(t)
                if t.starts_with("<?xml")
                    && t.split_once("?>").is_some_and(|(decl, _)| {
                        let d = decl.to_ascii_lowercase();
                        d.contains("encoding=\"utf-16\"") || d.contains("encoding='utf-16'")
                    }) =>
            {
                Err(xerr(
                    "There is no Unicode byte order mark. Cannot switch to Unicode.",
                    "XmlPortImport:Xml",
                ))
            }
            Ok(t) => Ok(t.to_string()),
            Err(_) => unsupported("XmlPort import of a source that is not UTF-8 or UTF-16 (not pinned)"),
        }
    }

    /// Runs an object trigger (OnInitXmlPort) as a trigger the flow builtins may end.
    pub(super) fn xp_run_object_trigger(&mut self, inst: &Inst, routine: usize) -> Result<(), Signal> {
        let object = inst.borrow().object as usize;
        {
            let mut s = inst.borrow_mut();
            s.xp_text.retain(|(k, _)| *k != 9);
            s.xp_text.push((9, self.depth.to_string()));
        }
        let result = self.call(object, routine, inst.clone(), Vec::new()).map(|_| ());
        self.report.flow_unwind = None;
        inst.borrow_mut().xp_text.retain(|(k, _)| *k != 9);
        result
    }

    pub(super) fn xp_run_trigger(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        name: &str,
    ) -> Result<(), Signal> {
        match schema.node(node).trigger(name) {
            Some(r) => {
                // `CurrentPath` in a trigger of an export is the path of the element; in an import and outside the triggers it
                // is empty (platform/lt-xmlport-session-navapp-cookie/P94XpPathFlow.al: Export_PathsInTriggers,
                // Import_PathsInTriggers).
                let exporting = inst.borrow().xp_text.iter().any(|(k, _)| *k == 6);
                if exporting {
                    let path = Self::xp_node_path(schema, node);
                    let mut s = inst.borrow_mut();
                    s.xp_text.retain(|(k, _)| *k != 7);
                    s.xp_text.push((7, path));
                }
                {
                    let mut s = inst.borrow_mut();
                    s.xp_text.retain(|(k, _)| *k != 9);
                    s.xp_text.push((9, self.depth.to_string()));
                }
                let result = self.call(schema.object, r, inst.clone(), Vec::new()).map(|_| ());
                self.report.flow_unwind = None;
                inst.borrow_mut().xp_text.retain(|(k, _)| *k != 7 && *k != 9);
                result
            }
            None => Ok(()),
        }
    }

    /// What the trigger that just ended asked for with Break, BreakUnbound or Quit.
    pub(super) fn xp_flow(inst: &Inst) -> Option<Flow> {
        let s = inst.borrow();
        match s.xp_text.iter().find(|(k, _)| *k == 8).map(|(_, v)| v.as_str()) {
            Some("1") => Some(Flow::Break),
            Some("2") => Some(Flow::Unbound),
            Some("3") => Some(Flow::Quit),
            Some("4") => Some(Flow::Skip),
            _ => None,
        }
    }

    pub(super) fn xp_clear_flow(inst: &Inst) {
        inst.borrow_mut().xp_text.retain(|(k, _)| *k != 8 && *k != 10);
    }

    /// Runs the trigger `name` of `node` and answers what it asked for with Skip, Break or BreakUnbound. Quit ends the run: the
    /// request stays set and the error unwinds to `xp_run` (corpus pages/lt-pages-xmlport-upload-members/P112XpXml.al:
    /// P112_Export_Quit, P112_Import_Quit).
    pub(super) fn xp_site(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        name: &str,
    ) -> Result<Option<Flow>, Signal> {
        self.xp_run_trigger(inst, schema, node, name)?;
        match Self::xp_flow(inst) {
            Some(Flow::Quit) => Err(Signal::Exit(None)),
            Some(other) => {
                inst.borrow_mut().xp_text.retain(|(k, _)| *k != 8);
                Ok(Some(other))
            }
            None => Ok(None),
        }
    }

    /// Whether a Quit was requested: the export hands back the document written so far (P112_Export_Quit: the stream is not
    /// empty) and `xp_run` ends the run without OnPostXmlPort.
    pub(super) fn xp_quit(inst: &Inst) -> bool {
        Self::xp_flow(inst) == Some(Flow::Quit)
    }

    /// An import stops after the record it is on: a Break in a trigger of that record (P112_Import_Break).
    pub(super) fn xp_stop_import(inst: &Inst) {
        inst.borrow_mut().xp_text.push((10, String::new()));
    }

    pub(super) fn xp_import_stopped(inst: &Inst) -> bool {
        inst.borrow().xp_text.iter().any(|(k, _)| *k == 10)
    }

    /// `/Root/Item`, `/Root/Item/Id` and `/Root/Item[@Qty]`: the names of the elements above the node, an attribute last in brackets.
    pub(super) fn xp_node_path(schema: &Schema, node: usize) -> String {
        let mut names = Vec::new();
        let mut at = Some(node);
        while let Some(n) = at {
            let nd = schema.node(n);
            let attr = matches!(nd.kind, ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr);
            names.push((nd.xml_name.clone(), attr));
            at = nd.parent;
        }
        names.reverse();
        let mut out = String::new();
        for (name, attr) in names {
            if attr {
                out.push_str(&format!("[@{name}]"));
            } else {
                out.push('/');
                out.push_str(&name);
            }
        }
        out
    }

    /// The child of schema node `parent` that reads element `name` (`want_attr`: attribute).
    pub(super) fn xp_child_named(schema: &Schema, parent: usize, name: &str, attr: bool) -> Option<usize> {
        schema.children(parent).into_iter().find(|&c| {
            let n = schema.node(c);
            let is_attr = matches!(n.kind, ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr);
            is_attr == attr && n.xml_name == name
        })
    }

    pub(super) fn xp_import_container(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
    ) -> Result<(), Signal> {
        self.xp_import_attrs(inst, schema, node, elem, None)?;
        let mut seq = Sequence::new(schema, node);
        let mut linked: Vec<usize> = Vec::new();
        for ce in elem.elements() {
            let Some(child) = seq.accept(schema, ce)? else {
                continue;
            };
            match schema.node(child).kind {
                ir::XmlNodeKind::Table => {
                    let first = !linked.contains(&child);
                    linked.push(child);
                    self.xp_import_record(inst, schema, child, ce, first, false)?;
                    if Self::xp_import_stopped(inst) {
                        return Ok(());
                    }
                }
                ir::XmlNodeKind::Text if !schema.children(child).is_empty() => {
                    self.xp_import_container(inst, schema, child, ce)?
                }
                ir::XmlNodeKind::Text => {
                    self.xp_assign_leaf_text(inst, schema, child, ce)?;
                    // Break in OnAfterAssignVariable of a text element right below the root ends the import there and
                    // OnPostXmlPort runs; Quit ends it without OnPostXmlPort (xmlport/lt-ay-xmlport/AyXmlPortFlowTriggers.al:391
                    // Import_head_Break `pre1;head1;post;`, :417 Import_head_Quit `pre1;head1;`, no rows). Deeper it is not pinned.
                    if let Some(f @ (Flow::Break | Flow::Quit)) = Self::xp_flow(inst) {
                        if schema.node(node).parent.is_some() {
                            return unsupported(
                                "Break or Quit in OnAfterAssignVariable of a text element below the root's children (not pinned)",
                            );
                        }
                        if f == Flow::Break {
                            Self::xp_clear_flow(inst);
                        }
                        return Ok(());
                    }
                }
                _ => return unsupported("XmlPort import of a field element outside a table element"),
            }
        }
        seq.finish(schema)
    }

    /// Reads the attributes of `elem` into the schema's attribute nodes under `node`.
    /// `assigned` collects the table fields they set.
    pub(super) fn xp_import_attrs(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
        mut assigned: Option<&mut Vec<usize>>,
    ) -> Result<(), Signal> {
        // A field attribute of the schema has to be there (xmlport/lt-xmlport-query-datatransfer/XpTImportOptions.al:
        // Import_AttributesMissing, Import_AttributesAsElements); the first one the element lacks is named. So has a text
        // attribute declared `Occurrence = Required`, also when its value is empty (xmlport/lt-qr-xmlport-container-occurs/
        // QrXpContainerOccurs.al:QS_TextAttr_Import_Root, QS_TextAttr_Import_Rows; report/lt-u-report-props/UPropertyTests.al:336).
        // A text attribute without `Occurrence` may be missing, as before these pins. A field attribute declared
        // `Occurrence = Optional` may be missing and leaves its field as it is, one declared Required may not
        // (platform/lt-ag-contoso/AgXmlPortImport.al:248 FieldAttribute_Occurrence_RequiredAndOptional_Import).
        // A text attribute of the root text element without `Occurrence` is required too
        // (xmlport/lt-xmlport-query-datatransfer/XpTImportOptions.al:282 Import_TextElementsMissingAttribute, xmlport 73421).
        // So is a text attribute without `Occurrence` of a text element right below a table element in a pinned shape
        // (`text_in_table`; QrXpContainerOccurs.al TO_AttrItem_Trigger_Import `attribute missing`, TO_TextContent_Import
        // `no kind`, on branch al-runtime-rs-staging-pub-to); beside a field attribute it is pinned only in the nested shape
        // (`TextInTable::Nested`; TP_Nested_Import_Shapes `status missing`, on branch al-runtime-rs-staging-pub-tp).
        let root_text = schema.node(node).parent.is_none() && schema.node(node).kind == ir::XmlNodeKind::Text;
        let in_table = text_in_table(schema, node).is_some();
        let lacks = |n: &ir::XmlNode| !elem.attrs.iter().any(|(name, _)| *name == n.xml_name);
        let below_table = schema.node(node).kind == ir::XmlNodeKind::Text
            && schema
                .node(node)
                .parent
                .is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table);
        if below_table && !in_table {
            let unpinned = schema.children(node).into_iter().any(|a| {
                let n = schema.node(a);
                n.kind == ir::XmlNodeKind::TextAttr && n.required.is_none() && lacks(n)
            });
            if unpinned {
                return unsupported(
                    "XmlPort import of a text element below a table element without a text attribute that has no Occurrence, beside a field attribute (not pinned)",
                );
            }
        }
        let missing = schema.children(node).into_iter().find(|&a| {
            let n = schema.node(a);
            ((n.kind == ir::XmlNodeKind::FieldAttr && n.required != Some(false))
                || (n.kind == ir::XmlNodeKind::TextAttr
                    && (n.required == Some(true) || (n.required.is_none() && (root_text || in_table)))))
                && lacks(n)
        });
        // A declaration of a namespace of the port is no attribute (YXmlPortProps.al:NodeProperties_Import_NamesPrefixes).
        let declares_ns = |name: &str, value: &str| {
            name.strip_prefix("xmlns:")
                .is_some_and(|p| schema.xp().namespaces.iter().any(|(q, u)| q == p && u == value))
        };
        // An attribute the schema lacks on the root or on a table element fails the schema check (xmlport/
        // lt-xmlport-query-datatransfer/XpTImportBasic.al:668 Import_AttributeOnRootIgnored, :682 Import_AttributeOnItem;
        // QrXpContainerOccurs.al:QS_TextAttr_Import_Root, an element with text attributes of its own). Not pinned: a prefixed
        // name, below a field element, with UseLax, and an element that also lacks a required attribute (which error wins).
        let unknown = elem.attrs.iter().find(|(name, value)| {
            !declares_ns(name, value) && Self::xp_child_named(schema, node, name, true).is_none()
        });
        if let Some((name, _)) = unknown {
            let pinned = !name.contains(':')
                && name != "xmlns"
                && !schema.xp().use_lax
                && missing.is_none()
                && matches!(schema.node(node).kind, ir::XmlNodeKind::Text | ir::XmlNodeKind::Table);
            if !pinned {
                return unsupported(format!(
                    "XmlPort import of an attribute {name} the schema does not have (error not pinned)"
                ));
            }
            return Err(schema_error(format!("The '{name}' attribute is not declared.")));
        }
        if let Some(a) = missing {
            return Err(schema_error(format!(
                "The required attribute '{}' is missing.",
                schema.node(a).xml_name
            )));
        }
        for (name, value) in &elem.attrs {
            if declares_ns(name, value) {
                continue;
            }
            let Some(a) = Self::xp_child_named(schema, node, name, true) else {
                return unsupported(format!(
                    "XmlPort import of an attribute {name} the schema does not have (error not pinned)"
                ));
            };
            match schema.node(a).kind {
                ir::XmlNodeKind::TextAttr => self.xp_assign_variable(inst, schema, a, value.clone())?,
                _ => {
                    let pos = self.xp_assign_field(inst, schema, a, value)?;
                    if let Some(list) = assigned.as_deref_mut() {
                        list.push(pos);
                    }
                }
            }
        }
        Ok(())
    }

    /// Text element or attribute: the text goes into its variable, then `OnAfterAssignVariable`
    /// (xmlport/TestXmlPortAdvanced.al:XmlPort_TextVariableTriggers_ImportMutatesAssignedValues).
    pub(super) fn xp_assign_variable(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        text: String,
    ) -> Result<(), Signal> {
        let slot = schema.node(node).slot.expect("text nodes have a slot");
        // A BigText element gets the whole text, past 2048 characters (xmlport/lt-xmlport-bigtext/XmlPortBigText.al:218).
        inst.borrow_mut().globals[slot] = if schema.node(node).big_text {
            Value::Library(
                al_builtins::library_kind::LibraryKind::BigText,
                std::rc::Rc::new(std::cell::RefCell::new(al_builtins::value::LibraryValue::Text(text))),
            )
        } else {
            Value::Text(text.into())
        };
        self.xp_run_trigger(inst, schema, node, "onafterassignvariable")
    }

    /// A leaf text element (no attributes or elements in the schema) read from `ce`: `<x />` runs its OnAfterAssignVariable
    /// twice, `<x></x>`, `<x> </x>` and `<x>v</x>` once, below the root, below a table element and inside a text element below a
    /// table element (QrXpContainerOccurs.al TR_EmptyLeaf_Import; deeper text elements run on the assumption
    /// `xmlport-self-closed-leaf-assign-twice`). The variable is set to the empty text again before the second run (the corpus
    /// triggers only read it). A Break or Quit in the first run ends the import before the second run would come: not pinned.
    pub(super) fn xp_assign_leaf_text(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        ce: &Elem,
    ) -> Result<(), Signal> {
        // `<x> </x>` gives the empty text (TR_EmptyLeaf_Import `space`: `top=[];`).
        // Other white space: kept with PreserveWhiteSpace, each run made one space and the ends trimmed without it (corpus PR
        // 896, misc/lt-np4-probes/Np4XmlPort.al Y9_XmlPort_Import_PreserveWhiteSpace_NoNamespace `note=[  lead  inner\n gap\t ]`,
        // Y9_XmlPort_Import_DefaultWhiteSpace_NoNamespace `note=[lead inner gap]`, with a default namespace the same).
        let text = ce.text();
        let text = if !text.is_empty() && text.trim().is_empty() {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportSelfClosedLeafAssignTwice);
            String::new()
        } else if !schema.xp().preserve_whitespace && text.contains(XML_SPACE) {
            collapse_white_space(&text)
        } else {
            text
        };
        self.xp_assign_variable(inst, schema, node, text)?;
        if ce.self_closed && schema.node(node).trigger("onafterassignvariable").is_some() {
            if Self::xp_import_stopped(inst) {
                return unsupported(
                    "XmlPort import of `<x />` whose OnAfterAssignVariable ended the import on its first run (whether it runs again is not pinned)",
                );
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportSelfClosedLeafAssignTwice);
            self.xp_assign_variable(inst, schema, node, String::new())?;
        }
        Ok(())
    }

    /// `<x />` for a text element with an OnAfterAssignVariable below a field element, a place TR_EmptyLeaf_Import does not
    /// cover: how often the trigger runs is not pinned.
    pub(super) fn xp_refuse_self_closed_leaf(schema: &Schema, node: usize, ce: &Elem) -> Result<(), Signal> {
        if ce.self_closed && schema.node(node).trigger("onafterassignvariable").is_some() {
            return unsupported(
                "XmlPort import of `<x />` for a text element with an OnAfterAssignVariable trigger below a field element (how often the trigger runs is not pinned)",
            );
        }
        Ok(())
    }

    /// Field element or attribute: the text becomes the field's value, then `OnAfterAssignField`.
    /// Returns the field's position.
    pub(super) fn xp_assign_field(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        text: &str,
    ) -> Result<usize, Signal> {
        Ok(self.xp_assign_field_flow(inst, schema, node, text)?.0)
    }

    /// `xp_assign_field`, and what `OnAfterAssignField` asked for.
    pub(super) fn xp_assign_field_flow(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        text: &str,
    ) -> Result<(usize, Option<Flow>), Signal> {
        // White space around a value is dropped, for text too (xmlport/lt-xmlport-query-datatransfer/XpTImportBasic.al:
        // Import_WhitespaceAroundValues).
        let raw = text;
        let text = text.trim();
        let n = schema.node(node);
        let table_node = schema.node(n.table_node.expect("field nodes have a table node"));
        let pos = n.field.expect("field nodes have a field");
        let rec = self.xp_record(inst, table_node)?;
        let (ty, calc, flow_filter) = {
            let r = rec.borrow();
            let field = self.engine.program.objects[r.table as usize]
                .table
                .as_ref()
                .and_then(|s| s.fields.get(pos));
            let calc = field
                .filter(|f| !f.stored && !f.flow_filter)
                .map(|f| f.calc.as_ref().map(|c| c.method));
            let flow_filter = field.is_some_and(|f| f.flow_filter);
            match &self.engine.tables[r.table as usize] {
                Some(t) => (t.fields[pos].ty.clone(), calc, flow_filter),
                None => return unsupported("XmlPort import into a table without a schema"),
            }
        };
        // With PreserveWhiteSpace a Text field element keeps it (XpTImportOptions.al:406 Import_PreserveWhitespace, `  a  b  `);
        // around another value or in an attribute that is not pinned.
        // A value of white space only is the empty text with and without it (corpus PR 896, misc/lt-np4-probes/Np4XmlPort.al
        // Y9_XmlPort_Import_PreserveWhiteSpace_NoNamespace `[]`). Without it an Xml import makes each run of white space inside a
        // Text field element one space (Y9_XmlPort_Import_DefaultWhiteSpace_NoNamespace, _WithNamespace `[a b]`).
        let collapsed;
        let text = if schema.xp().preserve_whitespace && text != raw && !text.is_empty() {
            if n.kind != ir::XmlNodeKind::Field || !matches!(ty, RtType::Text(_)) {
                return unsupported(
                    "XmlPort import with PreserveWhiteSpace of white space around a value that is no Text field element (not pinned)",
                );
            }
            raw
        } else if !schema.xp().preserve_whitespace
            && schema.xp().format == ir::XmlFormat::Xml
            && n.kind == ir::XmlNodeKind::Field
            && matches!(ty, RtType::Text(_))
            && text.contains(XML_SPACE)
        {
            collapsed = collapse_white_space(text);
            collapsed.as_str()
        } else {
            text
        };
        // A field element bound to a FlowFilter is skipped by an Xml import: its OnAfterAssignField does not run and the import
        // goes on (xmlport/lt-xmlport-flowfield-element/XmlPortFlowFilterElement.al:224 Import_FlowFilterElement_AssignedWithoutSave,
        // `log` empty; xmlport/lt-xmlport-flowfield-sum-count/XmlPortFlowFieldSumCount.al:326 `no error`). The other formats and
        // a text that is no value of the type are not pinned.
        if flow_filter {
            if schema.xp().format != al_ir::XmlFormat::Xml {
                return unsupported(format!(
                    "XmlPort import of field element {} bound to a FlowFilter in a text format (not pinned)",
                    n.name
                ));
            }
            if !text.is_empty() && self.xp_value(text, &ty, false).is_err() {
                return unsupported(format!(
                    "XmlPort import of field element {} bound to a FlowFilter with a text that is no value of its type (not pinned)",
                    n.name
                ));
            }
            return Ok((pos, None));
        }
        // A field element bound to a FlowField is skipped by an import: the field keeps its value and OnAfterAssignField
        // does not run. Xml: every method, with the trigger or without (xmlport/lt-ay-xmlport/AyXmlPortLookupFlowField.al:
        // Import_AutoCalc and Import_NoAutoCalc for Lookup, `before-insert=;`; :364 PYD_Import_Xml_ExistMinMaxAverage for Exist,
        // Min, Max, Average, Lookup and Count with triggers; xmlport/lt-xmlport-flowfield-sum-count/XmlPortFlowFieldSumCount.al:
        // 326-372 for Sum and Count). VariableText: Lookup, Count and Min (:415); FixedText: Lookup and Count (:439). A text that
        // is no value of the type is skipped too for a Decimal or Boolean element (:389, :402); for an Integer one the schema
        // check fails (:377 `many`). The other methods of the text formats and the other wrong texts are not pinned.
        if let Some(method) = calc {
            use al_ir::CalcMethod;
            let pinned = match (schema.xp().format, method) {
                (_, None) => false,
                (al_ir::XmlFormat::Xml, Some(_)) => true,
                (al_ir::XmlFormat::VariableText, Some(m)) => {
                    matches!(m, CalcMethod::Lookup | CalcMethod::Count | CalcMethod::Min)
                }
                (al_ir::XmlFormat::FixedText, Some(m)) => matches!(m, CalcMethod::Lookup | CalcMethod::Count),
            };
            if !pinned {
                return unsupported(format!(
                    "XmlPort import of field element {} bound to a FlowField of this kind (not pinned)",
                    n.name
                ));
            }
            if !text.is_empty() && self.xp_value(text, &ty, false).is_err() {
                let xml = schema.xp().format == al_ir::XmlFormat::Xml;
                match ty {
                    RtType::Decimal | RtType::Boolean if xml => {}
                    RtType::Integer if xml && text.chars().all(|c| c.is_ascii_alphabetic()) => {
                        return Err(schema_error(format!(
                            "The '{}' element is invalid - The value '{text}' is invalid according to its datatype \
                             'http://www.w3.org/2001/XMLSchema:int' - The string '{text}' is not a valid Int32 value.",
                            n.xml_name
                        )));
                    }
                    _ => {
                        return unsupported(format!(
                            "XmlPort import of field element {} bound to a FlowField with a text that is no value of its type (not pinned)",
                            n.name
                        ));
                    }
                }
            }
            return Ok((pos, None));
        }
        let value = if schema.xp().format_evaluate_xml {
            let members = {
                let r = rec.borrow();
                match &self.engine.tables[r.table as usize] {
                    Some(t) => self.members_of_field(t, pos)?,
                    None => None,
                }
            };
            self.xp_value_xml(text, &ty, members.map(|(_, m)| m))?
        } else if !schema.xp().format_evaluate_declared {
            // Without a FormatEvaluate the display text is read and, when that is no value, the XML form.
            match self.xp_value(text, &ty, false) {
                Ok(v) => v,
                Err(Signal::Error(first)) => match self.xp_value_xml(text, &ty, None) {
                    Ok(v) => v,
                    // A Date that neither reads gives the long text with the code Evaluate (XpTImportBasic.al: Import_BadDate,
                    // Import_DateWrongFormat).
                    Err(Signal::Error(_)) if matches!(ty, RtType::Date) => {
                        return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                            message: format!(
                                "The format of the Date '{text}' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'."
                            ),
                            code: "Evaluate".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    Err(Signal::Error(_)) => return Err(Signal::Error(first)),
                    Err(other) => return Err(other),
                },
                Err(other) => return Err(other),
            }
        } else {
            match self.xp_value(text, &ty, false) {
                // FormatEvaluate = Legacy reads a Date in the form yyyy-mm-dd too (xmlport/lt-xmlport-query-datatransfer/
                // XpTImportOptions.al:337 Import_FormatEvaluateLegacy_XmlValues, `2024-01-15`). Other XML forms are not pinned
                // here (TestXmlPortFormatEvaluateContracts.al:192 refuses the document as a whole).
                Err(Signal::Error(e))
                    if matches!(ty, RtType::Date)
                        && text.len() == 10
                        && text
                            .char_indices()
                            .all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() }) =>
                {
                    match self.xp_value_xml(text, &ty, None) {
                        Ok(v) => v,
                        Err(_) => return Err(Signal::Error(e)),
                    }
                }
                // A DateTime in the XML form without a zone fails with this text (TestXmlPortFormatEvaluateContracts.al:192-194,
                // `2024-03-05T12:34:56` beside a Date the line above reads); its code is not pinned.
                Err(Signal::Unsupported(_) | Signal::Error(_))
                    if matches!(ty, RtType::DateTime)
                        && text.len() == 19
                        && text.char_indices().all(|(i, c)| match i {
                            4 | 7 => c == '-',
                            10 => c == 'T',
                            13 | 16 => c == ':',
                            _ => c.is_ascii_digit(),
                        }) =>
                {
                    return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                        message: "The date is not valid.".into(),
                        code: al_builtins::native::UNPINNED_ERROR.into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                other => other?,
            }
        };
        rec.borrow_mut().fields[pos] = value.clone();
        self.xp_validate_field(schema, node, &rec, pos, value, text.is_empty())?;
        let flow = self.xp_site(inst, schema, node, "onafterassignfield")?;
        Ok((pos, flow))
    }

    /// The validation of an imported field (xmlport/lt-bj-xmlport-default-validation/BjXmlPortDefaultValidation.al:255-334):
    /// on a Format = Xml port a field element is validated unless it says `FieldValidate = No`, or, with
    /// `DefaultFieldsValidation = false`, when it says `FieldValidate = Yes`. Validated, its OnValidate runs with xRec
    /// already holding the new value and a TableRelation is checked, per field in document order before the insert; not
    /// validated, the value is stored as it is. Where validation can be seen (an OnValidate, a table extension's validate
    /// trigger, a checked TableRelation, a subscriber to the validate events), these are refused as not pinned: another
    /// format, a blank field attribute into a field without a TableRelation, and a validated element with an
    /// OnAfterAssignField trigger (which runs first is not pinned). A field attribute follows the same rule as an element,
    /// except that a blank one is not validated.
    pub(super) fn xp_validate_field(
        &mut self,
        schema: &Schema,
        node: usize,
        rec: &Rec,
        pos: usize,
        value: Value,
        blank: bool,
    ) -> Result<(), Signal> {
        let n = schema.node(node);
        let t = self.table(rec)?;
        let engine = self.engine.clone();
        let Some(field) = engine.program.objects[t.object as usize]
            .table
            .as_ref()
            .and_then(|s| s.fields.get(pos))
        else {
            return Ok(());
        };
        let mut seen = field.on_validate.is_some()
            || !field.ext_before_validate.is_empty()
            || !field.ext_after_validate.is_empty()
            || (field.relation && engine.field_facts[t.object as usize][pos].validate_relation);
        if !seen {
            seen = self.listened(&t, "onbeforevalidateevent", Some(&field.name))?
                || self.listened(&t, "onaftervalidateevent", Some(&field.name))?;
        }
        if !seen {
            return Ok(());
        }
        let xp = schema.xp();
        // A VariableText port with `DefaultFieldsValidation = false` does not validate a field element that does not say
        // FieldValidate (platform/lt-ag-contoso/AgXmlPortImport.al:678, xmlport 87567: the Iso field's OnValidate never runs).
        if xp.format == al_ir::XmlFormat::VariableText
            && xp.no_default_fields_validation
            && n.kind == al_ir::XmlNodeKind::Field
            && n.field_validate.is_none()
        {
            return Ok(());
        }
        if xp.format != al_ir::XmlFormat::Xml {
            return unsupported(format!(
                "XmlPort import of {} into {}.{}, which has validation code: whether a port of a Format other than Xml validates it is not pinned",
                n.name, t.name, field.name
            ));
        }
        // A field attribute is validated as a field element is: FieldValidate = Yes always, No never, left out when
        // DefaultFieldsValidation is true or left out (pages/lt-report-page-properties/06-xmlport-fvat.al:118-119;
        // xmlport/lt-bj-xmlport-default-validation/BjXmlPortDefaultValidation.al:625, :657, :689, :721: OnValidate runs with
        // xRec already holding the new value, a value the related table lacks fails the import).
        if !matches!(n.kind, al_ir::XmlNodeKind::Field | al_ir::XmlNodeKind::FieldAttr) {
            return unsupported(format!(
                "XmlPort import of {} into {}.{}, which has validation code: whether it is validated is not pinned",
                n.name, t.name, field.name
            ));
        }
        if !n.field_validate.unwrap_or(!xp.no_default_fields_validation) {
            return Ok(());
        }
        // A blank field attribute is not validated, even with FieldValidate = Yes, where a blank field element is
        // (BjXmlPortDefaultValidation.al:625 'AttrDef blank'/'AttrYes blank' against :641 'ElDef blank'). Pinned for a field
        // with a TableRelation only.
        if n.kind == al_ir::XmlNodeKind::FieldAttr && blank {
            if field.relation {
                return Ok(());
            }
            return unsupported(format!(
                "XmlPort import of a blank field attribute {} into {}.{}, which has validation code and no TableRelation: whether it is validated is not pinned",
                n.name, t.name, field.name
            ));
        }
        if n.trigger("onafterassignfield").is_some() {
            return unsupported(format!(
                "XmlPort import of field element {} with OnAfterAssignField into a validated field: which runs first is not pinned",
                n.name
            ));
        }
        self.record_op(RecordOp::Validate, rec, &mut [Value::Integer(pos as i32), value])?;
        Ok(())
    }
}

/// A document read as UTF-16 whose declaration says `utf-8`: the reader switches to UTF-8 after the declaration and meets the
/// zero byte that follows the next `<`: `Name cannot begin with the '.' character, hexadecimal value 0x00. Line 1, position N.`
/// with N two past the end of the declaration (xmlport/lt-af-xmlport/AfXmlPortEncoding.al:
/// AF_Encoding_Import_Utf8Property_Utf16DocDeclaredUtf8, a declaration followed at once by the root element). Any other
/// shape after the declaration is not pinned.
pub(super) fn utf16_declared_utf8(text: String) -> Result<String, Signal> {
    let Some((decl, rest)) = text.strip_prefix('\u{feff}').unwrap_or(&text).split_once("?>") else {
        return Ok(text);
    };
    let lower = decl.to_ascii_lowercase();
    if !lower.starts_with("<?xml") || !(lower.contains("encoding=\"utf-8\"") || lower.contains("encoding='utf-8'")) {
        return Ok(text);
    }
    if decl.contains('\n') || decl.contains('\r') || !rest.starts_with('<') {
        return unsupported(
            "XmlPort import of a UTF-16 document that declares utf-8 with a different layout (error text not pinned)",
        );
    }
    let position = decl.chars().count() + 2 + 2;
    Err(xerr(
        format!("Name cannot begin with the '.' character, hexadecimal value 0x00. Line 1, position {position}."),
        "XmlPortImport:Xml",
    ))
}
