//! XmlPort import and export (Format = Xml): `SetSource`, `SetDestination`, `SetTableView`,
//! `Import` and `Export`, instance and static forms.
//!
//! Every rule is pinned by a corpus test, cited where it is applied. What no test pins is
//! refused (`Signal::Unsupported`) and listed in docs/probes-needed.md: the exact bytes of an
//! export (declaration, whitespace), the texts of import errors, field types other than
//! Integer, Decimal, Text and Code, tables with triggers or event subscribers, and so on.

use crate::records::Rec;
use crate::transactions::TxModel;
use crate::{AlError, Session, Signal};
use al_builtins::builtin_id::ids;
use al_builtins::describe::Describe;
use al_builtins::native::unsupported;
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{Instance, LibraryValue, RtType, Value};
use al_builtins::xmlport::{Elem, Node, ReadFailure};
use al_data::{Store, position};
use al_ir as ir;
use legacy_guid::Written;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

mod after_assign;
mod assigned;
mod builtin_calls;
mod container_tree;
mod einvoice_shapes;
mod export_xml;
mod import_encoding;
mod import_record;
mod import_view;
mod import_xml;
mod legacy_guid;
mod pass;
mod plain_tree;
mod root_nested;
mod run_request;
mod sequence;
mod shape;
#[cfg(test)]
mod tests;
mod text;
mod text_attr_export;
mod text_columns;
mod text_nested;
mod value;
mod value_forms;
mod wraps;

use self::container_tree::*;
use self::einvoice_shapes::*;
use self::pass::*;
pub(crate) use self::pass::{PassView, UnlinkedPass};
use self::plain_tree::*;
use self::root_nested::*;
use self::sequence::*;
use self::shape::*;
use self::text_columns::*;
use self::value::*;
use self::wraps::*;

type Inst = Rc<RefCell<Instance>>;

fn xerr(message: impl Into<String>, code: &str) -> Signal {
    Signal::Error(Box::new(AlError {
        message: message.into(),
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// xmlports/TestXmlPortExportImportContracts.al:XmlPortImport_SchemaMismatch_ThrowsXmlPortInvalidXml
fn schema_error(why: impl AsRef<str>) -> Signal {
    xerr(
        format!("Imported XML cannot validate with the schema: {}", why.as_ref()),
        "XmlPortInvalidXml",
    )
}

/// xmlports/TestXmlPortExportImportContracts.al:XmlPortImport_BadInteger_ThrowsEvaluate
fn evaluate_error(text: &str, type_name: &str) -> Signal {
    xerr(
        format!("The value \"{text}\" can't be evaluated into type {type_name}."),
        "Evaluate",
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Import,
    Export,
}

/// The schema of the xmlport instance `inst` is, kept alive independently of the session.
struct Schema {
    engine: Arc<crate::Engine>,
    object: usize,
}

impl Schema {
    fn xp(&self) -> &ir::XmlPort {
        self.engine.program.objects[self.object]
            .xmlport
            .as_ref()
            .expect("checked by Session::xp_schema")
    }
    fn name(&self) -> &str {
        &self.engine.program.objects[self.object].name
    }
    fn node(&self, i: usize) -> &ir::XmlNode {
        &self.xp().nodes[i]
    }
    fn children(&self, i: usize) -> Vec<usize> {
        (0..self.xp().nodes.len())
            .filter(|&c| self.xp().nodes[c].parent == Some(i))
            .collect()
    }
}

impl Session {
    /// The xmlport object of a static `Xmlport.Export` or `Xmlport.Import` with the id `n`: an id no xmlport declares is a
    /// missing-object error, also one of another object kind, 0 and 2000000000 (xmlports/shapes-properties/Y83XpMiscTests.al:
    /// Static_Undeclared_Id, needs the corpus bump); one of a dependency app that was not compiled is refused.
    fn xp_static_object(&self, id: al_ir::BuiltinId, n: i32) -> Result<usize, Signal> {
        let heads = self.engine.program.objects.heads();
        let want = u32::try_from(n).ok();
        if let Some(object) = heads
            .iter()
            .position(|o| o.is_kind(al_ir::ObjKind::XmlPort) && o.id == want)
        {
            return Ok(object);
        }
        if heads
            .iter()
            .any(|o| o.is_shell_of(al_ir::ObjKind::XmlPort) && o.id == want)
        {
            return unsupported(format!(
                "{id} of xmlport {n}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)"
            ));
        }
        Err(self.missing_object("XmlPort", i64::from(n)))
    }

    pub(crate) fn xmlport_builtin(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
        ret_used: bool,
    ) -> Result<Value, Signal> {
        match id {
            ids::XMLPORT_SETSOURCE_1 | ids::XMLPORT_SETDESTINATION_1 => {
                let inst = self.xp_instance(receiver)?;
                let schema = self.xp_schema(&inst)?;
                let (slot, want) = if id == ids::XMLPORT_SETSOURCE_1 {
                    (schema.xp().source_slot, "InStream")
                } else {
                    (schema.xp().destination_slot, "OutStream")
                };
                match args.first() {
                    Some(v @ Value::Library(n, _)) if n.eq_ignore_ascii_case(want) => {
                        inst.borrow_mut().globals[slot] = v.clone();
                        Ok(Value::Null)
                    }
                    other => unsupported(format!("{id} with {}", other.describe())),
                }
            }
            ids::XMLPORT_FIELDDELIMITER_1
            | ids::XMLPORT_FIELDSEPARATOR_1
            | ids::XMLPORT_RECORDSEPARATOR_1
            | ids::XMLPORT_TABLESEPARATOR_1 => self.xp_separator_builtin(id, receiver, args),
            ids::XMLPORT_BREAK_1 | ids::XMLPORT_BREAKUNBOUND_1 | ids::XMLPORT_QUIT_1 | ids::XMLPORT_SKIP_1 => {
                self.xp_flow_builtin(id, receiver)
            }
            // CurrentPath(): see `xp_run_trigger`.
            ids::XMLPORT_CURRENTPATH_1 => {
                let inst = self.xp_instance(receiver)?;
                let path = inst
                    .borrow()
                    .xp_text
                    .iter()
                    .find(|(k, _)| *k == 7)
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                Ok(Value::Text(path.into()))
            }
            // ImportFile(): true while the port imports, false while it exports, read in OnPreXmlPort and OnPostXmlPort
            // (xmlports/import/XmlPortFixtures.al:265 and :275, xmlport 73421: XpTExportBasic.al:211
            // Export_TextAndAttributeElements, XpTImportOptions.al:268-307). Outside a run and the setter are not pinned.
            ids::XMLPORT_IMPORTFILE_1 => {
                let inst = self.xp_instance(receiver)?;
                if !args.is_empty() {
                    return unsupported("Xmlport.ImportFile with an argument (not pinned)");
                }
                let dir = inst
                    .borrow()
                    .xp_text
                    .iter()
                    .find(|(k, _)| *k == 13)
                    .map(|(_, v)| v == "1");
                match dir {
                    Some(import) => Ok(Value::Boolean(import)),
                    None => unsupported("Xmlport.ImportFile outside OnPreXmlPort or OnPostXmlPort (not pinned)"),
                }
            }
            ids::XMLPORT_FILENAME_1 => self.xp_filename_builtin(id, receiver, args),
            ids::XMLPORT_TEXTENCODING_1 => self.xp_text_encoding_builtin(id, receiver, args),
            ids::XMLPORT_SETTABLEVIEW_1 => {
                let inst = self.xp_instance(receiver)?;
                self.xp_set_view(&inst, args.first())?;
                Ok(Value::Null)
            }
            ids::XMLPORT_EXPORT_1 | ids::XMLPORT_IMPORT_1 => {
                let inst = self.xp_instance(receiver)?;
                let dir = if matches!(id, ids::XMLPORT_EXPORT_1 | ids::XMLPORT_EXPORT_2) {
                    Dir::Export
                } else {
                    Dir::Import
                };
                self.xp_run(&inst, dir, ret_used, false)
            }
            ids::XMLPORT_EXPORT_2 | ids::XMLPORT_IMPORT_2 => self.xp_static_export_import(id, args, ret_used),
            ids::XMLPORT_RUN_2 => self.xp_static_run(id, args),
            ids::XMLPORT_RUN_1 => self.xp_run_instance(receiver),
            other => unsupported(format!("builtin {other}")),
        }
    }

    /// The xmlport instance behind a receiver, with its globals made.
    fn xp_instance(&mut self, receiver: Option<Value>) -> Result<Inst, Signal> {
        match receiver {
            Some(Value::Codeunit(rc)) => {
                self.ensure_initialized(&rc);
                Ok(rc)
            }
            other => unsupported(format!("an XmlPort method on {}", other.describe())),
        }
    }

    /// The schema of `inst`'s object; refuses an xmlport the runtime cannot run.
    fn xp_schema(&self, inst: &Inst) -> Result<Schema, Signal> {
        let object = inst.borrow().object as usize;
        let schema = Schema {
            engine: self.engine.clone(),
            object,
        };
        match &schema.engine.program.objects[object].xmlport {
            Some(xp) => match xp.unsupported.first() {
                None => Ok(schema),
                Some(why) => unsupported(format!("XmlPort \"{}\": {why}", schema.name())),
            },
            None => unsupported(format!("object {object} is not an xmlport")),
        }
    }

    fn xp_record(&self, inst: &Inst, node: &ir::XmlNode) -> Result<Rec, Signal> {
        match node.slot.map(|s| inst.borrow().globals[s].clone()) {
            Some(Value::Record(rc)) => Ok(rc),
            other => unsupported(format!(
                "table element {} holds {other}",
                node.name,
                other = other.describe()
            )),
        }
    }

    /// `SetTableView`: the filters of the given record go to the first table element, in schema order and nested ones included,
    /// whose table is the record's table; a second element of the same table keeps all its rows
    /// (xmlports/TestXmlPortObject.al:XmlPort_SetTableView_RestrictsExportedRows; xmlports/set-table-view/
    /// XmlPortSetTableViewElement.al:128 SetTableView_RecordOfLaterSiblingElement_Export, :186 _RecordOfNestedChildElement_Export,
    /// :248 _TwoElementsOfTheSameTable_Export). A record of a table no element uses
    /// raises XmlPortNodeNotFound (same file, :226 SetTableView_RecordOfTableNoElementUses).
    fn xp_set_view(&mut self, inst: &Inst, view: Option<&Value>) -> Result<(), Signal> {
        let schema = self.xp_schema(inst)?;
        if !schema.xp().nodes.iter().any(|n| n.kind == ir::XmlNodeKind::Table) {
            return unsupported(format!(
                "SetTableView on XmlPort \"{}\", which has no table element",
                schema.name()
            ));
        }
        let Some(Value::Record(view)) = view else {
            return unsupported(format!("SetTableView with {}", view.describe()));
        };
        let view_table = view.borrow().table;
        let mut target = None;
        for node in schema.xp().nodes.iter().filter(|n| n.kind == ir::XmlNodeKind::Table) {
            let rec = self.xp_record(inst, node)?;
            if rec.borrow().table == view_table {
                target = Some(rec);
                break;
            }
        }
        let Some(target) = target else {
            // Measured for a port and a table that have no Caption, where the text names both; whether it shows a caption
            // instead of the name is not pinned.
            let port = &self.engine.program.objects[schema.object];
            let table = &self.engine.program.objects[view_table as usize];
            let captioned = |o: &ir::Object| {
                o.properties
                    .iter()
                    .any(|(k, _)| k.to_ascii_lowercase().starts_with("caption"))
            };
            if captioned(port) || captioned(table) {
                return unsupported(
                    "SetTableView with a record of a table no element uses, on a port or table with a Caption (error text not pinned)",
                );
            }
            return Err(Signal::Error(Box::new(AlError {
                message: format!(
                    "The {} XmlPort does not have a XmlItem that uses the table (Table {} {}) specified in the function SetTableView.",
                    port.name,
                    table.id.unwrap_or(0),
                    table.name
                ),
                code: "XmlPortNodeNotFound".into(),
                ..Default::default()
            })));
        };
        let v = view.borrow();
        if v.temporary || v.marked_only {
            return unsupported("SetTableView with a temporary or marked record (not pinned)");
        }
        // The view's key and direction order the export, ties included
        // (xmlports/TestXmlPortTriggersReplaceViewContracts.al:Export_OnBeforePassField_Runs_AndSetTableViewFiltersAndOrders,
        // corpus: .
        let mut t = target.borrow_mut();
        t.filters = v.filters.clone();
        t.filter_group = v.filter_group;
        t.key = v.key;
        t.adhoc = v.adhoc.clone();
        t.ascending = v.ascending;
        t.field_descending = v.field_descending;
        Ok(())
    }

    fn xp_run(&mut self, inst: &Inst, dir: Dir, ret_used: bool, static_form: bool) -> Result<Value, Signal> {
        let saved = self.permission_run_object;
        if self.permission_checks() {
            self.permission_run_object = self.xp_schema(inst).ok().map(|s| s.object);
        }
        let r = self.xp_run_tx(inst, dir, ret_used, static_form);
        self.permission_run_object = saved;
        // The direction ImportFile() reads holds only while the port runs (see XMLPORT_IMPORTFILE_1).
        inst.borrow_mut().xp_text.retain(|(k, _)| *k != 13);
        r
    }

    fn xp_run_tx(&mut self, inst: &Inst, dir: Dir, ret_used: bool, static_form: bool) -> Result<Value, Signal> {
        // An export started inside a TryFunction whose result is used: its triggers' Insert and Modify are the export's, not
        // writes of the try (xmlports/shapes-properties/YXmlPortProps.al:861, RS_XmlPortRequestPageRec_Export: in
        // OnPreXmlPort Modify answers false and Insert writes the row the export then lists), as an import's are
        // (`used_try_nested_world`). Other writes there are refused (`refuse_used_try_write`).
        if dir == Dir::Export
            && self.tx_model != crate::transactions::TxModel::None
            && self.tx.xp_export_try_world.is_none()
            && self.used_try_write_applies()
        {
            self.tx.tx_world += 1;
            self.tx.xp_export_try_world = Some(self.tx.tx_world);
            let r = self.xp_run_body(inst, dir, ret_used, static_form);
            self.tx.xp_export_try_world = None;
            self.tx.tx_world -= 1;
            return r;
        }
        self.xp_run_body(inst, dir, ret_used, static_form)
    }

    fn xp_run_body(&mut self, inst: &Inst, dir: Dir, ret_used: bool, static_form: bool) -> Result<Value, Signal> {
        let schema = self.xp_schema(inst)?;
        // xmlports/TestXmlPortExportImportContracts.al:XmlPort_WrongDirection_Throws
        match (dir, schema.xp().direction) {
            (Dir::Import, ir::XmlDirection::Export) => {
                return Err(xerr(
                    format!("The XMLPort '{}' is designed for export only.", schema.name()),
                    "XmlPortExportDirection",
                ));
            }
            (Dir::Export, ir::XmlDirection::Import) => {
                return Err(xerr(
                    format!("The XMLPort '{}' is designed for import only.", schema.name()),
                    "XmlPortImportDirection",
                ));
            }
            _ => {}
        }
        // Constructs no corpus test pins are refused when the port runs, not when it is declared
        // (`ir::XmlPort::run_unsupported`).
        if let Some(why) = schema.xp().run_unsupported.first() {
            return unsupported(format!("XmlPort \"{}\": {why}", schema.name()));
        }
        if let (Dir::Export, Some(why)) = (dir, schema.xp().export_unsupported.first()) {
            return unsupported(format!("XmlPort \"{}\": {why}", schema.name()));
        }
        if let (Dir::Import, Some(why)) = (dir, schema.xp().import_unsupported.first()) {
            return unsupported(format!("XmlPort \"{}\": {why}", schema.name()));
        }
        for a in &schema.xp().assumes {
            al_builtins::assumptions::assume(*a);
        }
        // ADR-062: an export reads its table elements with the xmlport's own Permissions (Y24PermObjectReads.al Y24_P16_*).
        if dir == Dir::Export && self.permission_checks() {
            let mut tables = Vec::new();
            for node in &schema.xp().nodes {
                if let Ok(rec) = self.xp_record(inst, node) {
                    let r = rec.borrow();
                    if !r.temporary && !tables.contains(&(r.table as usize)) {
                        tables.push(r.table as usize);
                    }
                }
            }
            self.permission_gate_object_read(schema.object, &tables)?;
        }
        // OnPreXmlPort runs when the port starts, in both directions
        // (testpage/TestPageActionRunObjectKinds_Tests.al:ProbeControlEachTargetRecordsItsOwnExecution pins
        // it for an export); the other object triggers are not pinned and refused.
        let routines = &schema.engine.program.objects[schema.object].routines;
        let mut pre = None;
        let mut post = None;
        let mut init = None;
        for (i, r) in routines.iter().enumerate().filter(|(_, r)| r.is_trigger) {
            if r.name.eq_ignore_ascii_case("OnPreXmlPort") {
                pre = Some(i);
            } else if r.name.eq_ignore_ascii_case("OnPostXmlPort") {
                post = Some(i);
            } else if r.name.eq_ignore_ascii_case("OnInitXmlPort") {
                init = Some(i);
            }
        }
        // A guarded static Import of an Xml port that BC turns into false for a pending write runs no object trigger
        // (`xp_static_import_blocked`). With an OnInitXmlPort, and for a text port (whose source is read before OnPreXmlPort, and
        // an empty one fails first as before), it is not pinned and keeps the order below.
        if dir == Dir::Import
            && static_form
            && ret_used
            && init.is_none()
            && schema.xp().format == ir::XmlFormat::Xml
            && self.xp_static_import_blocked()?
        {
            return Ok(Value::Boolean(false));
        }
        Self::xp_clear_flow(inst);
        assigned::mark_run_start(inst);
        // OnInitXmlPort runs once, when the variable is first used: a second Export or Import of the same variable does not
        // run it again, a static call makes a new instance (corpus P112XpXml.al:P112_Export_RoundTrip_SameInstance,
        // P112_Static_Export_Import). A Quit in it ends the run with nothing written (P112_Export_Init_Quit); Skip and Break
        // only end the trigger (P112_Import_Init_Actions).
        if let Some(r) = init {
            let first = !inst.borrow().xp_text.iter().any(|(k, _)| *k == 11);
            if first {
                inst.borrow_mut().xp_text.push((11, String::new()));
                self.xp_run_object_trigger(inst, r)?;
                if Self::xp_flow(inst) == Some(Flow::Quit) {
                    Self::xp_clear_flow(inst);
                    return Ok(if ret_used { Value::Boolean(true) } else { Value::Null });
                }
                Self::xp_clear_flow(inst);
            }
        }
        // The source of a text-format import is read, and found empty, before OnPreXmlPort runs (P112XpCsv.al:
        // P112_Import_EmptySource, `init 1;init done;`); the text waits for the import.
        if dir == Dir::Import
            && import_encoding::defers_decoding(schema.xp())
            && !import_xml::text_port_with_table_root(schema.xp())
        {
            let bytes = self.xp_read_source_bytes(inst, schema.xp())?;
            if bytes.is_empty() {
                return Err(xerr(
                    "Cannot import the specified XML document because the file is empty.",
                    "XmlPortData",
                ));
            }
            import_encoding::stash_raw(inst, &bytes)?;
        } else if dir == Dir::Import && schema.xp().format != ir::XmlFormat::Xml {
            let text = match self.xp_read_source(inst, schema.xp()) {
                // A table-rooted text port fails with the root error whatever the bytes are, also ones that are no UTF-8 or
                // UTF-16 text (XpTEncoding.al:165 Encoding_ImportCsvWindows, :179 Encoding_ImportCsvUtf16; no trigger runs there).
                Err(Signal::Unsupported(_)) if import_xml::text_port_with_table_root(schema.xp()) => {
                    return Err(import_xml::root_type_error());
                }
                other => other?,
            };
            if text.is_empty() {
                return Err(xerr(
                    "Cannot import the specified XML document because the file is empty.",
                    "XmlPortData",
                ));
            }
            inst.borrow_mut().xp_text.retain(|(k, _)| *k != 12);
            inst.borrow_mut().xp_text.push((12, text));
        }
        let mark = |inst: &Inst| {
            inst.borrow_mut()
                .xp_text
                .push((13, if dir == Dir::Import { "1" } else { "0" }.to_string()));
        };
        let unmark = |inst: &Inst| inst.borrow_mut().xp_text.retain(|(k, _)| *k != 13);
        if let Some(r) = pre {
            mark(inst);
            self.xp_run_object_trigger(inst, r)?;
            unmark(inst);
            // Break, BreakUnbound and Quit in OnPreXmlPort end an import there: nothing is read, OnPostXmlPort does not run and
            // the import succeeds (xmlports/shapes-properties/AyXmlPortFlowTriggers.al:352-389, Import_pre_*: `pre1;`, no rows).
            // An export writes the declaration alone (:584-624); whether a line break follows it is not pinned. Skip there,
            // and the value of a guarded import, are not pinned either.
            if let Some(flow) = Self::xp_flow(inst) {
                Self::xp_clear_flow(inst);
                // An export whose result is used writes the declaration alone and returns true, for an Xml port with Encoding = UTF8
                // (xmlports/shapes-properties/AyXmlPortFlowTriggers.al:586-625 Export_pre_Break, _BreakUnbound, _Quit, and :790-835 _Bytes:
                // 54 bytes, no line break, no OnPostXmlPort). Other ports and encodings are not pinned.
                if dir == Dir::Export
                    && ret_used
                    && flow != Flow::Skip
                    && schema.xp().format == ir::XmlFormat::Xml
                    && schema.xp().xml_utf8
                    && !schema.xp().utf8
                    && !schema.xp().inline_schema
                {
                    return self.xp_export_declaration_only(inst, &schema);
                }
                if dir == Dir::Export || ret_used || flow == Flow::Skip {
                    return unsupported(
                        "Break, BreakUnbound, Quit or Skip in OnPreXmlPort of an export, a guarded import, or Skip there (not pinned)",
                    );
                }
                inst.borrow_mut().xp_text.retain(|(k, _)| *k != 12);
                import_encoding::drop_raw(inst);
                return Ok(Value::Null);
            }
        }
        Self::xp_clear_flow(inst);
        let run = match dir {
            Dir::Export => self.xp_export(inst, &schema, ret_used),
            Dir::Import => self.xp_import(inst, &schema, ret_used, static_form),
        };
        inst.borrow_mut().xp_text.retain(|(k, _)| *k != XP_STOPPED);
        // Quit ends the run at once: no OnPostXmlPort, and the run succeeded (Export_Quit).
        if Self::xp_flow(inst) == Some(Flow::Quit) && matches!(run, Err(Signal::Exit(None)) | Ok(_)) {
            Self::xp_clear_flow(inst);
            return Ok(if ret_used { Value::Boolean(true) } else { Value::Null });
        }
        let result = run?;
        // A Break, BreakUnbound, Skip or Quit anywhere else is not pinned.
        if Self::xp_flow(inst).is_some() {
            Self::xp_clear_flow(inst);
            return unsupported("Break, BreakUnbound, Skip or Quit in a trigger the corpus does not pin (not pinned)");
        }
        // OnPostXmlPort runs after the data was passed, in both directions (metadata/session-system/
        // P94XpPathFlow.al and P94XpFilename.al log it last); after an error it does not run. After a guarded run that
        // returned false it is not pinned.
        if let Some(r) = post {
            if matches!(result, Value::Boolean(false)) {
                // A guarded import of a `root_holding_table` port whose read of the document failed does not run it (al-runner
                // issue 6924; xmlports/shapes-properties/XpRootTableNested.al:604, :622, :640, the log ends before `post`). A false
                // without a read (the static form with a pending write, `xp_import`) is not pinned.
                let read_failed = inst.borrow().xp_text.iter().any(|(k, _)| *k == XP_IMPORT_FAILED);
                if dir == Dir::Import && read_failed && port_with_root_holding_table(&schema) {
                    return Ok(result);
                }
                return unsupported("OnPostXmlPort after a guarded run that returned false (not pinned)");
            }
            mark(inst);
            self.call(schema.object, r, inst.clone(), Vec::new())?;
            unmark(inst);
        }
        Ok(result)
    }

    // ---------------------------------------------------------------- import
}

/// What a trigger asked for: leave the table element's rows, skip the rest of the row, or end the run.
#[derive(Clone, Copy, PartialEq)]
enum Flow {
    Break,
    Unbound,
    Quit,
    Skip,
}

fn root_table_element(schema: &Schema, top: usize) -> bool {
    let n = schema.node(top);
    schema.xp().format == ir::XmlFormat::Xml
        && n.kind == ir::XmlNodeKind::Table
        && n.triggers.is_empty()
        && n.link_node.is_none()
        && !n.min_zero
        && !schema.children(top).is_empty()
        && schema.children(top).into_iter().all(|c| {
            let f = schema.node(c);
            f.kind == ir::XmlNodeKind::Field && f.triggers.is_empty() && !f.min_zero && schema.children(c).is_empty()
        })
}

fn doc_name(schema: &Schema, c: usize) -> String {
    let n = schema.node(c);
    match (&n.ns_prefix, n.kind) {
        (Some(p), ir::XmlNodeKind::Field | ir::XmlNodeKind::Text | ir::XmlNodeKind::Table) => {
            format!("{p}:{}", n.xml_name)
        }
        _ => n.xml_name.clone(),
    }
}

/// Whether text element `c` inside table element `table` is a container of table elements linked to it and nothing else,
/// declared last in the table element: the shape xmlports/shapes-properties/QrXpContainerOccurs.al:91-104 pins
/// (`<lines>` in `<head>`, exported once per row, also when empty: QR_Export_OneHead_NoLines, QR_Export_TwoHeads_LinesOnSecond).
/// The filter group of a table element's SourceTableView: its own, as a report's DataItemTableView has.
const XP_VIEW_GROUP: i32 = crate::reports::ITEM_VIEW_GROUP;

/// The `Instance::xp_text` kind that says an export ran on the instance (its SourceTableView keys are not set again).
const XP_EXPORTED: u8 = 14;

/// The `Instance::xp_text` kind that says an Import read the source of the instance.
const XP_IMPORTED: u8 = 15;

/// The `Instance::xp_text` kind that says an Import of the instance ended in an error.
const XP_IMPORT_FAILED: u8 = 16;

/// The `Instance::xp_text` kind that says an export ended in a Break of a leaf text element below the root: nothing else is passed.
const XP_STOPPED: u8 = 30;
