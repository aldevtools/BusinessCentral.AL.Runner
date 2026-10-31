//! Import of table elements: record assignment, text inside a table, linked containers, save.

use super::*;

impl Session {
    /// One element of a table element: a fresh record, its link fields, `OnAfterInitRecord`,
    /// the attributes and children, and the save. `first_of_node` is false for the second and
    /// later elements of a table element under the same parent: only the first gets the link
    /// (xmlport/TestXmlPortTriggersReplaceViewContracts.al:Nested_Export_GroupsTheChildrenUnderTheirParent_ImportLinksOnlyTheFirstChild,
    /// corpus: .
    pub(super) fn xp_import_record(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
        first_of_node: bool,
        keep_key: bool,
    ) -> Result<(), Signal> {
        let n = schema.node(node);
        if n.max_unbounded == Some(false) && unlinked_below_table(schema, node) {
            return unsupported(
                "XmlPort import of a table element with MaxOccurs = Once and no LinkTable below another table element (not pinned)",
            );
        }
        if n.max_unbounded == Some(false) && nested_table(schema, node) {
            return unsupported(
                "XmlPort import of a table element with MaxOccurs = Once below another table element (not pinned)",
            );
        }
        if n.link_node.is_some()
            && n.source_view
                .as_deref()
                .is_some_and(|v| v.to_ascii_lowercase().contains("where"))
        {
            return unsupported(format!(
                "XmlPort import: sourcetableview with a filter on the linked tableelement {} is not pinned",
                n.name
            ));
        }
        let rec = self.xp_record(inst, n)?;
        let table = rec.borrow().table as usize;
        let mut fresh = self.initial_fields(table);
        if keep_key {
            // As Init: the primary key of the row before stays (see `xp_import_linked_container`).
            let t = self.table(&rec)?;
            let r = rec.borrow();
            for &k in &t.keys[0] {
                fresh[k] = r.fields[k].clone();
            }
        }
        if !first_of_node && n.link_node.is_some() {
            // A later row under the same parent keeps a link field of its primary key, as Init keeps the key, already in
            // OnBeforeInsertRecord (xmlport/TestXmlPortTriggersReplaceViewContracts.al:648, attributes over 3 levels and
            // elements over 2); a link field outside the key is 0 there (same file :414).
            let t = self.table(&rec)?;
            let r = rec.borrow();
            for &(own, _) in &n.link_fields {
                if t.keys[0].contains(&own) {
                    fresh[own] = r.fields[own].clone();
                }
            }
        }
        rec.borrow_mut().fields = fresh;
        let mut assigned: Vec<usize> = Vec::new();
        let mut links: Vec<(usize, Value)> = Vec::new();
        // Nested table elements take their link fields from the record above
        // (xmlport/TestXmlPortAdvanced.al:XmlPort_NestedTableElements_ImportCreatesParentAndChildren).
        if let Some(link) = n.link_node.filter(|_| first_of_node) {
            let parent = self.xp_record(inst, schema.node(link))?;
            for &(own, other) in &n.link_fields {
                let v = parent.borrow().fields[other].clone();
                let ty = self.engine.tables[table]
                    .as_ref()
                    .expect("a table element has a schema")
                    .fields[own]
                    .ty
                    .clone();
                let v = al_builtins::ops::convert(v, &ty)?;
                links.push((own, v.clone()));
                rec.borrow_mut().fields[own] = v;
                assigned.push(own);
            }
        }
        self.xp_run_trigger(inst, schema, node, "onafterinitrecord")?;
        // Break in OnAfterInitRecord of a table element right below the root ends the import before that row: the rows before
        // it stay, OnPostXmlPort runs (AyXmlPortFlowTriggers.al:428 Import_init_Break, `...init2;post;`, rows `1=a,`). Deeper it
        // is not pinned.
        if Self::xp_flow(inst) == Some(Flow::Break) {
            let below_root = n
                .parent
                .is_some_and(|p| schema.node(p).parent.is_none() && schema.node(p).kind == ir::XmlNodeKind::Text);
            if !below_root {
                return unsupported(
                    "Break in OnAfterInitRecord of a table element that is not right below the root (not pinned)",
                );
            }
            Self::xp_clear_flow(inst);
            Self::xp_stop_import(inst);
            return Ok(());
        }
        self.xp_import_attrs(inst, schema, node, elem, Some(&mut assigned))?;
        let mut saved = false;
        let mut saved_before_tables = false;
        let table_children: Vec<usize> = schema
            .children(node)
            .into_iter()
            .filter(|&c| schema.node(c).kind == ir::XmlNodeKind::Table)
            .collect();
        let linked_children = table_children
            .iter()
            .filter(|&&c| schema.node(c).link_node.is_some())
            .count();
        // Some linked and some not: pinned with the linked ones first in the schema and no triggers (AhXmlPort.al:325-360,
        // xmlport 78633): the record is stored before its first linked row, the rows below keep their own values.
        let mixed = linked_children != 0 && linked_children != table_children.len();
        if mixed {
            let linked_first = table_children
                .iter()
                .skip_while(|&&c| schema.node(c).link_node.is_some())
                .all(|&c| schema.node(c).link_node.is_none());
            let quiet = n.triggers.is_empty() && table_children.iter().all(|&c| schema.node(c).triggers.is_empty());
            if !linked_first || !quiet {
                return unsupported(
                    "XmlPort import of a table element with table elements below it of which only some name a LinkTable (not pinned)",
                );
            }
        }
        let save_first = linked_children == table_children.len() || mixed;
        let mut seq = Sequence::new(schema, node);
        let mut linked: Vec<usize> = Vec::new();
        for ce in elem.elements() {
            let Some(child) = seq.accept(schema, ce)? else {
                continue;
            };
            match schema.node(child).kind {
                ir::XmlNodeKind::Field => {
                    let pos = self.xp_assign_field(inst, schema, child, &ce.text())?;
                    assigned.push(pos);
                    // A link field the document gives another value than the record above fails the import there, before the
                    // elements below (xmlport/lt-ah-xmlport/AhXmlPort.al:325 Mixed_Import and :386 Grand_Import, `ParentId` 99
                    // under `Id` 1: `P1=p1 ` stored, nothing after). Pinned for an element and fields without triggers.
                    if let Some((_, want)) = links.iter().find(|(own, _)| *own == pos) {
                        let same = ops::compare(&rec.borrow().fields[pos], want)? == std::cmp::Ordering::Equal;
                        if !same {
                            let quiet = n.triggers.is_empty()
                                && schema
                                    .children(node)
                                    .iter()
                                    .all(|&c| schema.node(c).triggers.is_empty());
                            if !quiet || n.name != n.xml_name {
                                return unsupported(
                                    "XmlPort import of a link field with another value than the record above, with triggers or an XmlName (not pinned)",
                                );
                            }
                            return Err(xerr(
                                format!(
                                    "The data conflicts with the LinkTable and LinkFields setup of Table {}.",
                                    n.name
                                ),
                                "XmlPortData",
                            ));
                        }
                    }
                    if !schema.children(child).is_empty() {
                        self.xp_import_field_children(inst, schema, child, ce, &mut assigned)?;
                    }
                }
                ir::XmlNodeKind::Text if schema.children(child).is_empty() => {
                    self.xp_assign_leaf_text(inst, schema, child, ce)?
                }
                ir::XmlNodeKind::Table => {
                    // The record is stored before the elements below it are read when they are linked to it by LinkTable (assumption:
                    // docs/probes-needed.md; xmlport/lt-xmlport-linktable/XmlPortLinkTableNoLinkFields.al:BothTemporary_Import_*,
                    // PersistentChild_Import_*, with LinkTable and no LinkFields). Without LinkTable the elements below are imported first and
                    // the record is stored after them (same file, NoLinkTable_Import_OneParentOneChild, _TwoParents).
                    if mixed && !saved && schema.node(child).link_node.is_none() {
                        return unsupported(
                            "XmlPort import of a row of an unlinked table element before any linked row, beside linked ones (when the record above is stored is not pinned)",
                        );
                    }
                    if save_first && !saved {
                        self.xp_save_record(inst, schema, node, &assigned, false)?;
                        saved = true;
                        saved_before_tables = true;
                    }
                    let first = !linked.contains(&child);
                    linked.push(child);
                    self.xp_import_record(inst, schema, child, ce, first, false)?;
                    if Self::xp_import_stopped(inst) {
                        return Ok(());
                    }
                }
                ir::XmlNodeKind::Text if text_in_table(schema, child).is_some() => {
                    self.xp_import_text_in_table(inst, schema, child, ce, &mut assigned)?;
                }
                // A text element holding only attributes, of the row's fields or its own (xmlport/lt-sg-xmlport-occurs/
                // SgXmlPortOccurs.al:114-175 SG_AttrBelowText_Import*): its attributes are read into the row as the row's own.
                // Its own text and elements below it, beside a field attribute, are not pinned.
                ir::XmlNodeKind::Text if attr_container(schema, child) => {
                    if !ce.text().is_empty() || ce.elements().next().is_some() {
                        return unsupported(
                            "XmlPort import of a text element with a field attribute and its own text or elements below a table element (not pinned)",
                        );
                    }
                    self.xp_import_attrs(inst, schema, child, ce, Some(&mut assigned))?;
                }
                // `unlinked_container`: the inner rows are stored before the outer row, whose OnBeforeInsertRecord runs after them;
                // they stay in the temporary table across outer rows (QrXpContainerOccurs.al TP_Unlinked_Import,
                // TP_Unlinked_Import_Shapes on branch al-runtime-rs-staging-pub-tp).
                ir::XmlNodeKind::Text if unlinked_container(schema, child, node) => {
                    let mut seq = Sequence::new(schema, child);
                    let mut seen = false;
                    for ie in ce.elements() {
                        let Some(inner) = seq.accept(schema, ie)? else {
                            continue;
                        };
                        self.xp_import_record(inst, schema, inner, ie, !seen, false)?;
                        seen = true;
                        if Self::xp_import_stopped(inst) {
                            return Ok(());
                        }
                    }
                    seq.finish(schema)?;
                }
                ir::XmlNodeKind::Text if linked_container(schema, child, node) => {
                    // The rows below are linked to this record, which is stored first, as for a table element right below it.
                    if !saved {
                        self.xp_save_record(inst, schema, node, &assigned, false)?;
                        saved = true;
                    }
                    self.xp_import_linked_container(inst, schema, child, ce)?;
                    if Self::xp_import_stopped(inst) {
                        return Ok(());
                    }
                }
                _ => return unsupported("XmlPort import of a text element with content inside a table element"),
            }
        }
        seq.finish(schema)?;
        if !saved {
            self.xp_save_record(inst, schema, node, &assigned, false)?;
        } else if saved_before_tables {
            // A parent stored before its linked child rows is modified again after them, with its OnBefore/OnAfterModifyRecord and
            // the table's OnModify: with AutoReplace (XmlPortTriggerFlow.al:374 Import_TriggerOrder_NewRows `Par.BeforeModifyRecord=1/one`
            // after the child rows, :399 Import_TriggerOrder_ExistingRows) and without it (:452 NoAutoReplace_NewParentTwoChildren,
            // trace :462; on three levels each row after its own children, :535 ThreeLevels_NoAutoReplace). A parent without child
            // rows is not modified again (same traces). With AutoSave = false or AutoUpdate it is not pinned.
            if !n.auto_save || n.auto_update {
                return unsupported(
                    "XmlPort import of a table element with AutoUpdate or AutoSave = false after its linked child rows (whether it is stored again is not pinned)",
                );
            }
            self.xp_save_record(inst, schema, node, &assigned, true)?;
        }
        Ok(())
    }

    /// A text element right below a table element in a pinned shape (`text_in_table`): its attributes, then the elements below
    /// it, then its own text with its OnAfterAssignVariable (QrXpContainerOccurs.al TO_AttrItem_Trigger_Import log
    /// `init;no=R1;attr=1;item=REQ;before insert ...`, TO_TextContent_Import `kind=k1;name=N1;sub=s1;info=;`, and
    /// TO_TextContent_Import_Shapes `info=T` for `<info kind="k1">T<name>..`, on branch al-runtime-rs-staging-pub-to). An element
    /// below a text-only one fails the schema check (TO_AttrItem_Trigger_Import_Missing `child element`), pinned with every
    /// attribute present.
    pub(super) fn xp_import_text_in_table(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
        assigned: &mut Vec<usize>,
    ) -> Result<(), Signal> {
        let shape = text_in_table(schema, node).expect("checked by the caller");
        if shape == TextInTable::Attrs
            && let Some(x) = elem.elements().next()
        {
            let declared: Vec<&str> = schema
                .children(node)
                .into_iter()
                .map(|a| schema.node(a).xml_name.as_str())
                .collect();
            let exact = elem.attrs.len() == declared.len()
                && declared.iter().all(|d| elem.attrs.iter().any(|(name, _)| name == d));
            if !exact {
                return unsupported(
                    "XmlPort import of an element below a text-only element that also lacks or adds an attribute (which error wins is not pinned)",
                );
            }
            return Err(schema_error(format!(
                "The element '{}' cannot contain child element '{}' because the parent element's content model is text only.",
                schema.node(node).xml_name,
                x.name
            )));
        }
        self.xp_import_attrs(inst, schema, node, elem, Some(assigned))?;
        // The nested shape (TP_Nested_Import, TP_Nested_Import_Shapes on branch al-runtime-rs-staging-pub-tp): the outer element's
        // attributes, then the inner element's attributes and its text with its OnAfterAssignVariable (`status=open;admission=A1
        // admcode=1;`), then the outer element's own text (`entry=T` for `<entry ..>T<admission ..`). A missing inner element or a
        // missing Required attribute of it fails the schema check. An inner attribute without Occurrence = Required that the
        // document lacks, and elements inside the inner element, are not pinned.
        if shape == TextInTable::Nested {
            let mut seq = Sequence::new(schema, node);
            for ce in elem.elements() {
                let Some(child) = seq.accept(schema, ce)? else {
                    continue;
                };
                let lacks_unpinned = schema.children(child).into_iter().any(|a| {
                    let n = schema.node(a);
                    n.required != Some(true) && !ce.attrs.iter().any(|(name, _)| *name == n.xml_name)
                });
                if lacks_unpinned || ce.elements().next().is_some() {
                    return unsupported(
                        "XmlPort import of a text element inside a text element below a table element without an attribute that is not Required, or with elements inside it (not pinned)",
                    );
                }
                self.xp_import_attrs(inst, schema, child, ce, None)?;
                // `<admission .. />` with attributes runs the trigger once (TP_Nested_Import, branch al-runtime-rs-staging-pub-tp).
                self.xp_assign_variable(inst, schema, child, ce.text())?;
            }
            seq.finish(schema)?;
        }
        if shape == TextInTable::Content {
            let mut seq = Sequence::new(schema, node);
            for ce in elem.elements() {
                let Some(child) = seq.accept(schema, ce)? else {
                    continue;
                };
                // An empty `<tier />` runs its OnAfterAssignVariable twice here (QrXpContainerOccurs.al TQ_Response_Import_Shapes
                // `optional leaves absent`, `tier=;tier=;`; TR_EmptyLeaf_Import `inner=[];inner=[];`, `<inner></inner>` once).
                match schema.node(child).kind {
                    ir::XmlNodeKind::Field => assigned.push(self.xp_assign_field(inst, schema, child, &ce.text())?),
                    _ => self.xp_assign_leaf_text(inst, schema, child, ce)?,
                }
            }
            seq.finish(schema)?;
        }
        self.xp_assign_variable(inst, schema, node, elem.text())
    }

    /// A container text element inside a table element that holds only table elements linked to it
    /// (xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al:91-104, QR_Import_Present: `<lines>` in `<head>`).
    /// Every row of the container is linked to the record above: `H1/10=5;H1/20=7;H2/10=1` (QR_Import_Present). Right below a
    /// table element only the first row gets the link and a later one has 0 in a link field outside the primary key
    /// (xmlport/TestXmlPortTriggersReplaceViewContracts.al:414). Both fit a link set once and kept by Init for the fields of the
    /// primary key, which is what this does; a link field outside the primary key in a container is not pinned and is refused.
    pub(super) fn xp_import_linked_container(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
    ) -> Result<(), Signal> {
        for c in schema.children(node) {
            let n = schema.node(c);
            let t = self.table(&self.xp_record(inst, n)?)?;
            if n.link_fields.iter().any(|(own, _)| !t.keys[0].contains(own)) {
                return unsupported(format!(
                    "XmlPort import of the table element {} in a container text element with a link field outside its primary key (not pinned)",
                    n.xml_name
                ));
            }
        }
        self.xp_import_attrs(inst, schema, node, elem, None)?;
        let mut seq = Sequence::new(schema, node);
        let mut linked: Vec<usize> = Vec::new();
        for ce in elem.elements() {
            let Some(child) = seq.accept(schema, ce)? else {
                continue;
            };
            let first = !linked.contains(&child);
            linked.push(child);
            self.xp_import_record(inst, schema, child, ce, first, !first)?;
            if Self::xp_import_stopped(inst) {
                return Ok(());
            }
        }
        seq.finish(schema)
    }

    /// The attributes and elements below field element `node`: its own text is the field's value, the attributes and the leaf
    /// text and field elements after it fill their variables and fields (xmlport/lt-af-xmlport/AfXmlPortAttrChildren.al:247-257,
    /// 277-288, 306-315: `<Name Lang="de" Kind="K9">Beta</Name>`, `<Name>Gamma<Child>cc</Child></Name>`, `<Name>Delta<Lang>fr</Lang></Name>`).
    pub(super) fn xp_import_field_children(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
        assigned: &mut Vec<usize>,
    ) -> Result<(), Signal> {
        // A field attribute of a field element that the document lacks: the table element's "required attribute" error is not
        // pinned for this place (probe wanted: import a field element without its fieldattribute).
        for a in schema.children(node) {
            let n = schema.node(a);
            if n.kind == ir::XmlNodeKind::FieldAttr && !elem.attrs.iter().any(|(name, _)| *name == n.xml_name) {
                return unsupported(format!(
                    "XmlPort import of the field element {} without its field attribute {}: the error is not pinned (probe wanted)",
                    schema.node(node).xml_name,
                    n.xml_name
                ));
            }
        }
        self.xp_import_attrs(inst, schema, node, elem, Some(assigned))?;
        let mut seq = Sequence::new(schema, node);
        for ce in elem.elements() {
            let Some(child) = seq.accept(schema, ce)? else {
                continue;
            };
            match schema.node(child).kind {
                ir::XmlNodeKind::Field => assigned.push(self.xp_assign_field(inst, schema, child, &ce.text())?),
                ir::XmlNodeKind::Text => {
                    Self::xp_refuse_self_closed_leaf(schema, child, ce)?;
                    self.xp_assign_variable(inst, schema, child, ce.text())?
                }
                _ => {
                    return unsupported(
                        "XmlPort import of a node below a field element other than an attribute or a leaf element",
                    );
                }
            }
        }
        seq.finish(schema)
    }

    /// Stores the buffer of table element `node`: it is inserted with its triggers and events, as
    /// `Insert(true)` does (xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_RunsTheTableInsertTriggersAndEvents).
    /// A row with the same key is modified when `AutoUpdate` or `AutoReplace` is on: `AutoReplace`
    /// replaces the whole buffer and `AutoUpdate` changes only the fields the document carries, and
    /// both keep the row's SystemId
    /// (xmlport/TestXmlPortTriggersReplaceViewContracts.al:Import_AutoReplace_ReplacesTheWholeBuffer_KeepsSystemId,
    /// :Import_AutoUpdate_ChangesOnlyTheCarriedFields_KeepsSystemId, corpus: .
    /// The `OnBefore...Record` and `OnAfter...Record` triggers bracket it, and `AutoSave = false`
    /// leaves the storing to them (xmlport/TestXmlPortAdvanced.al:XmlPort_AutoSaveFalse_...).
    /// `again`: the second save of a row stored before its linked child rows, a Modify (see `xp_import_record`).
    pub(super) fn xp_save_record(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        assigned: &[usize],
        again: bool,
    ) -> Result<(), Signal> {
        let n = schema.node(node);
        let rec = self.xp_record(inst, n)?;
        let t = self.table(&rec)?;
        let temporary = rec.borrow().temporary;
        if again
            && !temporary
            && self
                .tx
                .deferred_insert
                .as_ref()
                .is_some_and(|(o, _)| *o == t.object as usize)
        {
            // The insert of a row whose key already existed fails after the row's child rows and before its second
            // OnBeforeModifyRecord, with the table's "already exists" error (XmlPortTriggerFlow.al:468
            // NoAutoReplace_ExistingParentWithChild: trace ends at `C.ai=10`, then DB:RecordExists).
            let (_, e) = self.tx.deferred_insert.take().expect("checked");
            self.tx.deferred_raised += 1;
            return Err(Signal::Error(Box::new(e)));
        }
        let cx = self.cx.clone();
        let pk = t.primary_key(&rec.borrow().fields);
        let stored = self
            .store_for(&rec)
            .get(&cx, &t, &pk)
            .map_err(|e| Signal::Unsupported(format!("{e}")))?;
        if again && stored.is_none() {
            return unsupported(format!(
                "XmlPort import: the row of table element {} stored before its child rows is gone after them (not pinned)",
                n.xml_name
            ));
        }
        let modify = stored.is_some() && (again || n.auto_update || n.auto_replace);
        // A temporary table element saves without the table's triggers: pinned for a nested one with AutoReplace
        // (XmlPortTriggerFlow.al:551 TempChild_AutoReplace, trace :561: no `tC+` or `tC~`) and for the insert of a new row
        // (misc/lt-y14-triage/Y14Small.al:419, a top-level element: no OnInsert, corpus master). The modify of a row a
        // temporary table element already holds is not pinned: refused where the table has a trigger.
        let nested = n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table);
        let pinned_temp = nested && n.auto_replace;
        let run_triggers = !(temporary && (pinned_temp || !modify));
        if temporary
            && n.auto_save
            && modify
            && !pinned_temp
            && (self.table_trigger(&t, "OnInsert").is_some()
                || self.table_trigger(&t, "OnModify").is_some()
                || !self.ext_triggers(&t, "OnInsert").is_empty()
                || !self.ext_triggers(&t, "OnModify").is_empty())
        {
            return unsupported(format!(
                "XmlPort import into the temporary table element {} of a row it already holds, whose table has an insert or modify trigger (whether it runs is not pinned)",
                n.xml_name
            ));
        }
        let (before, after) = if modify {
            ("onbeforemodifyrecord", "onaftermodifyrecord")
        } else {
            ("onbeforeinsertrecord", "onafterinsertrecord")
        };
        // Skip and BreakUnbound leave this record out (no insert, no after trigger), Break leaves it out and ends the import
        // (P112_Import_Skip, _Break, _BreakUnbound at beforeIns2).
        match self.xp_site(inst, schema, node, before)? {
            Some(Flow::Skip | Flow::Unbound) => return Ok(()),
            Some(Flow::Break) => {
                Self::xp_stop_import(inst);
                return Ok(());
            }
            _ => {}
        }
        if n.auto_save {
            if modify && !n.auto_replace {
                // AutoUpdate changes only the fields the document carries.
                let has_assign_trigger = n.trigger("onafterinitrecord").is_some()
                    || schema.children(node).iter().any(|&c| {
                        let child = schema.node(c);
                        child.trigger("onafterassignfield").is_some()
                            || child.trigger("onafterassignvariable").is_some()
                    });
                if has_assign_trigger {
                    return unsupported("AutoUpdate with triggers that assign fields (which values win is not pinned)");
                }
                let imported = rec.borrow().fields.clone();
                let key_args: Vec<Value> = pk.clone();
                self.ret_used = true;
                self.record_op(RecordOp::Get, &rec, &mut key_args.clone())?;
                {
                    let mut r = rec.borrow_mut();
                    for &p in assigned {
                        r.fields[p] = imported[p].clone();
                    }
                }
            } else if modify {
                // The row keeps its system fields.
                let old = stored.expect("modify implies a stored row");
                let mut r = rec.borrow_mut();
                for (i, f) in t.fields.iter().enumerate() {
                    if f.id >= 2_000_000_000 {
                        r.fields[i] = old[i].clone();
                    }
                }
            }
            // A duplicate key raises the table's own error, as an AL Insert does.
            self.ret_used = false;
            let method = if modify { RecordOp::Modify } else { RecordOp::Insert };
            self.record_op(method, &rec, &mut [Value::Boolean(run_triggers)])?;
        }
        // A Break after the insert ends the import with this record in (P112_Import_Break, afterIns2).
        if self.xp_site(inst, schema, node, after)? == Some(Flow::Break) {
            Self::xp_stop_import(inst);
        }
        Ok(())
    }

    // ------------------------------------------------- VariableText, FixedText
}
