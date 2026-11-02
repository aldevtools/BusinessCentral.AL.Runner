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
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{Instance, LibraryValue, RtType, Value};
use al_builtins::xmlport::{Elem, Node, ReadFailure};
use al_data::{Store, position};
use al_ir as ir;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

mod assigned;
mod container_tree;
mod export_xml;
mod import_record;
mod import_xml;
mod pass;
mod run_request;
mod sequence;
mod shape;
#[cfg(test)]
mod tests;
mod text;
mod text_columns;
mod value;
mod wraps;

use self::container_tree::*;
use self::pass::*;
pub(crate) use self::pass::{PassView, UnlinkedPass};
use self::sequence::*;
use self::shape::*;
use self::text::*;
use self::text_columns::*;
use self::value::*;
use self::wraps::*;

type Inst = Rc<RefCell<Instance>>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn xerr(message: impl Into<String>, code: &str) -> Signal {
    Signal::Error(Box::new(AlError {
        message: message.into(),
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_SchemaMismatch_ThrowsXmlPortInvalidXml
fn schema_error(why: impl AsRef<str>) -> Signal {
    xerr(
        format!("Imported XML cannot validate with the schema: {}", why.as_ref()),
        "XmlPortInvalidXml",
    )
}

/// xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_BadInteger_ThrowsEvaluate
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
            // The separators of a text xmlport: a value a program set reads back as it was given; a declared one reads as the
            // characters it stands for (`<CR/LF>` is `\r\n`, `<None>` is empty); an undeclared one reads as the default
            // (platform/lt-xmlport-session-navapp-cookie/P94XpDelim.al:Getters_*, Setters_*, *_Variants).
            ids::XMLPORT_FIELDDELIMITER_1
            | ids::XMLPORT_FIELDSEPARATOR_1
            | ids::XMLPORT_RECORDSEPARATOR_1
            | ids::XMLPORT_TABLESEPARATOR_1 => {
                let inst = self.xp_instance(receiver)?;
                let kind: u8 = match id {
                    ids::XMLPORT_FIELDDELIMITER_1 => 0,
                    ids::XMLPORT_FIELDSEPARATOR_1 => 1,
                    ids::XMLPORT_RECORDSEPARATOR_1 => 2,
                    _ => 3,
                };
                match args.first() {
                    Some(v) => {
                        let Some(text) = al_builtins::native::text_of(v) else {
                            return unsupported(format!("{id} with {}", v.describe()));
                        };
                        let mut i = inst.borrow_mut();
                        i.xp_text.retain(|(k, _)| *k != kind);
                        i.xp_text.push((kind, text.to_string()));
                        Ok(Value::Null)
                    }
                    None => {
                        if let Some((_, v)) = inst.borrow().xp_text.iter().find(|(k, _)| *k == kind) {
                            return Ok(Value::Text(v.clone().into()));
                        }
                        let object = inst.borrow().object as usize;
                        let Some(xp) = self.engine.program.objects[object].xmlport.as_ref() else {
                            return unsupported(format!("object {object} is not an xmlport"));
                        };
                        let (declared, default) = match kind {
                            0 => (&xp.field_delimiter, "\""),
                            1 => (&xp.field_separator, ","),
                            2 => (&xp.record_separator, "\\r\\n"),
                            _ => (&xp.table_separator, "\\r\\n\\r\\n"),
                        };
                        Ok(Value::Text(
                            match declared {
                                Some(d) => declared_separator(d)?,
                                None => default.to_string(),
                            }
                            .into(),
                        ))
                    }
                }
            }
            // Break, BreakUnbound and Quit end the trigger that calls them (platform/lt-xmlport-session-navapp-cookie/P94XpPathFlow.al:
            // Export_Break, Export_BreakUnbound, Export_Quit); what the run does next is `xp_flow`. Called from a procedure the
            // trigger called, the Exit leaves only that procedure: pinned only where the trigger ends with the call.
            ids::XMLPORT_BREAK_1 | ids::XMLPORT_BREAKUNBOUND_1 | ids::XMLPORT_QUIT_1 | ids::XMLPORT_SKIP_1 => {
                let inst = self.xp_instance(receiver)?;
                let trigger_depth = inst
                    .borrow()
                    .xp_text
                    .iter()
                    .find(|(k, _)| *k == 9)
                    .and_then(|(_, v)| v.parse::<usize>().ok());
                // From the trigger itself or from any procedure it calls: the whole trigger ends (corpus xml/lt-xml-port-flow/
                // XmlPortFlowFromTrigger.al:P170_Flow_*ViaProcedure, the code after the call in the trigger does not run).
                match trigger_depth {
                    Some(d) if self.depth > d => {
                        self.report.flow_unwind = Some(d + 1);
                    }
                    _ => return unsupported(format!("{id} outside the triggers where it is pinned")),
                }
                let kind = match id {
                    ids::XMLPORT_BREAK_1 => "1",
                    ids::XMLPORT_BREAKUNBOUND_1 => "2",
                    ids::XMLPORT_SKIP_1 => "4",
                    _ => "3",
                };
                let mut s = inst.borrow_mut();
                s.xp_text.retain(|(k, _)| *k != 8);
                s.xp_text.push((8, kind.to_string()));
                Err(Signal::Exit(None))
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
            // (xmlport/lt-xmlport-query-datatransfer/XmlPortFixtures.al:265 and :275, xmlport 73421: XpTExportBasic.al:211
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
            // Filename(): empty until set, per instance (P94XpFilename.al: Fresh_Empty, Set_ReadBack, Set_TwoInstancesIndependent).
            ids::XMLPORT_FILENAME_1 => {
                let inst = self.xp_instance(receiver)?;
                match args.first() {
                    Some(v) => {
                        let Some(text) = al_builtins::native::text_of(v) else {
                            return unsupported(format!("{id} with {}", v.describe()));
                        };
                        let text = text.to_string();
                        let mut s = inst.borrow_mut();
                        s.xp_text.retain(|(k, _)| *k != 5);
                        s.xp_text.push((5, text));
                        Ok(Value::Null)
                    }
                    None => {
                        let s = inst.borrow();
                        let name = s
                            .xp_text
                            .iter()
                            .find(|(k, _)| *k == 5)
                            .map(|(_, v)| v.clone())
                            .unwrap_or_default();
                        Ok(Value::Text(name.into()))
                    }
                }
            }
            // TextEncoding(): MSDos until declared or set (UTF8 when declared so); the setter takes the member (P94XpEncoding.al:
            // Getter_Defaults, Setter_ReadBack). Which bytes an export then writes is separate.
            ids::XMLPORT_TEXTENCODING_1 => {
                const MEMBERS: [&str; 4] = ["MSDos", "UTF8", "UTF16", "Windows"];
                let inst = self.xp_instance(receiver)?;
                let members = || -> std::sync::Arc<[String]> {
                    MEMBERS.iter().map(|m| (*m).to_string()).collect::<Vec<_>>().into()
                };
                match args.first() {
                    Some(Value::Option(i, _)) if (0..4).contains(i) => {
                        let mut s = inst.borrow_mut();
                        s.xp_text.retain(|(k, _)| *k != 4);
                        s.xp_text.push((4, i.to_string()));
                        Ok(Value::Null)
                    }
                    Some(other) => unsupported(format!("{id} with {}", other.describe())),
                    None => {
                        if let Some((_, v)) = inst.borrow().xp_text.iter().find(|(k, _)| *k == 4) {
                            return Ok(Value::Option(v.parse().unwrap_or(0), members()));
                        }
                        let object = inst.borrow().object as usize;
                        match self.engine.program.objects[object].xmlport.as_ref() {
                            Some(xp) if xp.utf8 => Ok(Value::Option(1, members())),
                            Some(xp) if xp.text_encoding_declared => {
                                unsupported("TextEncoding() of an xmlport that declares another encoding (not pinned)")
                            }
                            Some(_) => Ok(Value::Option(0, members())),
                            None => unsupported(format!("object {object} is not an xmlport")),
                        }
                    }
                }
            }
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
            // The static forms run a fresh instance of the xmlport with that id.
            ids::XMLPORT_EXPORT_2 | ids::XMLPORT_IMPORT_2 => {
                let dir = if matches!(id, ids::XMLPORT_EXPORT_1 | ids::XMLPORT_EXPORT_2) {
                    Dir::Export
                } else {
                    Dir::Import
                };
                let Some(Value::Integer(n)) = args.first() else {
                    return unsupported(format!("{id} with an id that is not an integer"));
                };
                let Some(object) = self
                    .engine
                    .program
                    .objects
                    .heads()
                    .iter()
                    .position(|o| o.is_kind(al_ir::ObjKind::XmlPort) && o.id == u32::try_from(*n).ok())
                else {
                    return unsupported(format!(
                        "{id} of id {n}, which no xmlport declares (error text not pinned)"
                    ));
                };
                let inst = self.instance(object);
                let schema = self.xp_schema(&inst)?;
                let (slot, want) = match dir {
                    Dir::Export => (schema.xp().destination_slot, "OutStream"),
                    Dir::Import => (schema.xp().source_slot, "InStream"),
                };
                match args.get(1) {
                    Some(v @ Value::Library(n, _)) if n.eq_ignore_ascii_case(want) => {
                        inst.borrow_mut().globals[slot] = v.clone();
                    }
                    other => return unsupported(format!("{id} with the stream {}", other.describe())),
                }
                // The record only seeds the table view
                // (xmlport/TestXmlPortObject.al:XmlPort_Import_StaticWithRecord_GivenVariableUnchangedWithoutExplicitGet,
                // :XmlPort_Export_StaticWithRecord_RespectsFilters).
                if let Some(rec) = args.get(2) {
                    self.xp_set_view(&inst, Some(rec))?;
                }
                self.xp_run(&inst, dir, ret_used, true)
            }
            // The static `Xmlport.Run(Number, RequestWindow, Import, Record)`. Pinned: an id no xmlport declares raises the missing-object
            // error (platform/lt-xmlport-session-navapp-cookie/P94XpPathFlow.al:RunStatic_UnknownNumbers; misc/W5MissingObject.al:
            // MissingReport_Page_XmlPort), and an import of an export-only xmlport raises its direction error (same file:
            // RunStatic_ImportWithoutSource). What a run that passes those checks reads or writes is in no corpus test and is refused.
            ids::XMLPORT_RUN_2 => {
                let Some(Value::Integer(n)) = args.first() else {
                    return unsupported(format!("{id} with an id that is not an integer"));
                };
                let Some(object) = self
                    .engine
                    .program
                    .objects
                    .heads()
                    .iter()
                    .position(|o| o.is_kind(al_ir::ObjKind::XmlPort) && o.id == u32::try_from(*n).ok())
                else {
                    if self
                        .engine
                        .program
                        .objects
                        .heads()
                        .iter()
                        .any(|o| o.is_shell_of(al_ir::ObjKind::XmlPort) && o.id == u32::try_from(*n).ok())
                    {
                        return unsupported(format!(
                            "Xmlport.Run of xmlport {n}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)"
                        ));
                    }
                    return Err(self.missing_object("XmlPort", i64::from(*n)));
                };
                let import = matches!(args.get(2), Some(Value::Boolean(true)));
                let inst = self.instance(object);
                let schema = self.xp_schema(&inst)?;
                if import && schema.xp().direction == ir::XmlDirection::Export {
                    return Err(xerr(
                        format!("The XMLPort '{}' is designed for export only.", schema.name()),
                        "XmlPortExportDirection",
                    ));
                }
                self.xp_run_static_existing(args, import, &inst, &schema)
            }
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

    /// `SetTableView`: the filters of the given record go to the first table element
    /// (xmlport/TestXmlPortObject.al:XmlPort_SetTableView_RestrictsExportedRows).
    fn xp_set_view(&mut self, inst: &Inst, view: Option<&Value>) -> Result<(), Signal> {
        let schema = self.xp_schema(inst)?;
        let Some(first) = schema.xp().nodes.iter().find(|n| n.kind == ir::XmlNodeKind::Table) else {
            return unsupported(format!(
                "SetTableView on XmlPort \"{}\", which has no table element",
                schema.name()
            ));
        };
        let Some(Value::Record(view)) = view else {
            return unsupported(format!("SetTableView with {}", view.describe()));
        };
        let target = self.xp_record(inst, first)?;
        let v = view.borrow();
        if v.table != target.borrow().table {
            return unsupported("SetTableView with a record of another table (error text not pinned)");
        }
        if v.temporary || v.marked_only {
            return unsupported("SetTableView with a temporary or marked record (not pinned)");
        }
        // The view's key and direction order the export, ties included
        // (xmlport/TestXmlPortTriggersReplaceViewContracts.al:Export_OnBeforePassField_Runs_AndSetTableViewFiltersAndOrders,
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
        let r = self.xp_run_tx(inst, dir, ret_used, static_form);
        // The direction ImportFile() reads holds only while the port runs (see XMLPORT_IMPORTFILE_1).
        inst.borrow_mut().xp_text.retain(|(k, _)| *k != 13);
        r
    }

    fn xp_run_tx(&mut self, inst: &Inst, dir: Dir, ret_used: bool, static_form: bool) -> Result<Value, Signal> {
        // An export started inside a TryFunction whose result is used: its triggers' Insert and Modify are the export's, not
        // writes of the try (xmlport/lt-y-xmlport-props/YXmlPortProps.al:861, RS_XmlPortRequestPageRec_Export: in
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
        // xmlport/TestXmlPortExportImportContracts.al:XmlPort_WrongDirection_Throws
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
        for a in &schema.xp().assumes {
            al_builtins::assumptions::assume(*a);
        }
        // OnPreXmlPort runs when the port starts, in both directions
        // (handlers/TestPageActionRunObjectKinds_Tests.al:ProbeControlEachTargetRecordsItsOwnExecution pins
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
        if dir == Dir::Import && schema.xp().format != ir::XmlFormat::Xml {
            let text = self.xp_read_source(inst, schema.xp())?;
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
            // the import succeeds (xmlport/lt-ay-xmlport/AyXmlPortFlowTriggers.al:352-389, Import_pre_*: `pre1;`, no rows).
            // An export writes the declaration alone (:584-624); whether a line break follows it is not pinned. Skip there,
            // and the value of a guarded import, are not pinned either.
            if let Some(flow) = Self::xp_flow(inst) {
                Self::xp_clear_flow(inst);
                if dir == Dir::Export || ret_used || flow == Flow::Skip {
                    return unsupported(
                        "Break, BreakUnbound, Quit or Skip in OnPreXmlPort of an export, a guarded import, or Skip there (not pinned)",
                    );
                }
                inst.borrow_mut().xp_text.retain(|(k, _)| *k != 12);
                return Ok(Value::Null);
            }
        }
        Self::xp_clear_flow(inst);
        let run = match dir {
            Dir::Export => self.xp_export(inst, &schema, ret_used),
            Dir::Import => self.xp_import(inst, &schema, ret_used, static_form),
        };
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
        // OnPostXmlPort runs after the data was passed, in both directions (platform/lt-xmlport-session-navapp-cookie/
        // P94XpPathFlow.al and P94XpFilename.al log it last); after an error it does not run. After a guarded run that
        // returned false it is not pinned.
        if let Some(r) = post {
            if matches!(result, Value::Boolean(false)) {
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
/// declared last in the table element: the shape xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al:91-104 pins
/// (`<lines>` in `<head>`, exported once per row, also when empty: QR_Export_OneHead_NoLines, QR_Export_TwoHeads_LinesOnSecond).
/// The filter group of a table element's SourceTableView: its own, as a report's DataItemTableView has.
const XP_VIEW_GROUP: i32 = crate::reports::ITEM_VIEW_GROUP;

/// The `Instance::xp_text` kind that says an export ran on the instance (its SourceTableView keys are not set again).
const XP_EXPORTED: u8 = 14;

/// The `Instance::xp_text` kind that says an Import read the source of the instance.
const XP_IMPORTED: u8 = 15;

/// The `Instance::xp_text` kind that says an Import of the instance ended in an error.
const XP_IMPORT_FAILED: u8 = 16;
