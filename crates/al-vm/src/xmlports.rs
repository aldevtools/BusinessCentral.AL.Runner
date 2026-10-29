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
                unsupported(
                    "Xmlport.Run of an existing xmlport: where a run without a stream writes or reads (a file on the service tier a test cannot read) and what the request window does are not pinned",
                )
            }
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

    fn xp_import(&mut self, inst: &Inst, schema: &Schema, guarded: bool, static_form: bool) -> Result<Value, Signal> {
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
        // branch al-runtime-rs-staging-tt; assumption `xmlport-import-again`). Pinned for an unguarded XML import after one that
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
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportImportAgain);
            return Ok(Value::Null);
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

    fn xp_import_document(&mut self, inst: &Inst, schema: &Schema) -> Result<(), Signal> {
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
        // With PreserveWhiteSpace, white space between elements is not pinned.
        if xp.preserve_whitespace {
            fn spaced(e: &Elem) -> bool {
                (e.elements().next().is_some()
                    && e.children.iter().any(|c| matches!(c, Node::Text(t) if !t.is_empty())))
                    || e.elements().any(spaced)
            }
            if spaced(&root) {
                return unsupported(
                    "XmlPort import with PreserveWhiteSpace of white space between elements (not pinned)",
                );
            }
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
    fn xp_parse_in_namespace(
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
    fn xp_read_source(&mut self, inst: &Inst, xp: &ir::XmlPort) -> Result<String, Signal> {
        let Value::Library(_, h) = inst.borrow().globals[xp.source_slot].clone() else {
            return unsupported("XmlPort import without SetSource (a file dialog is not supported)");
        };
        let mut g = h.borrow_mut();
        let LibraryValue::InStream(st) = &mut *g else {
            return unsupported("XmlPort import without SetSource (a file dialog is not supported)");
        };
        // An XML import reads the whole stream, also one read to its end before SetSource, and leaves its read position where it
        // was (QrXpContainerOccurs.al TT_Import_SourceReadBefore, TT_ImportTwice_SameSource `source at end` No, on corpus writer TT's
        // branch al-runtime-rs-staging-tt; assumption `xmlport-import-whole-source`). From a position inside the stream it is not
        // pinned. A text-format import takes the bytes from the read position on (as before those pins).
        let bytes = if xp.format == ir::XmlFormat::Xml {
            let (pos, len) = (st.pos, st.len());
            if pos != 0 && pos < len {
                return unsupported("XmlPort import of a source stream read partly before (not pinned)");
            }
            if pos != 0 {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportImportWholeSource);
            }
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
    fn xp_run_object_trigger(&mut self, inst: &Inst, routine: usize) -> Result<(), Signal> {
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

    fn xp_run_trigger(&mut self, inst: &Inst, schema: &Schema, node: usize, name: &str) -> Result<(), Signal> {
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
    fn xp_flow(inst: &Inst) -> Option<Flow> {
        let s = inst.borrow();
        match s.xp_text.iter().find(|(k, _)| *k == 8).map(|(_, v)| v.as_str()) {
            Some("1") => Some(Flow::Break),
            Some("2") => Some(Flow::Unbound),
            Some("3") => Some(Flow::Quit),
            Some("4") => Some(Flow::Skip),
            _ => None,
        }
    }

    fn xp_clear_flow(inst: &Inst) {
        inst.borrow_mut().xp_text.retain(|(k, _)| *k != 8 && *k != 10);
    }

    /// Runs the trigger `name` of `node` and answers what it asked for with Skip, Break or BreakUnbound. Quit ends the run: the
    /// request stays set and the error unwinds to `xp_run` (corpus pages/lt-pages-xmlport-upload-members/P112XpXml.al:
    /// P112_Export_Quit, P112_Import_Quit).
    fn xp_site(&mut self, inst: &Inst, schema: &Schema, node: usize, name: &str) -> Result<Option<Flow>, Signal> {
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
    fn xp_quit(inst: &Inst) -> bool {
        Self::xp_flow(inst) == Some(Flow::Quit)
    }

    /// An import stops after the record it is on: a Break in a trigger of that record (P112_Import_Break).
    fn xp_stop_import(inst: &Inst) {
        inst.borrow_mut().xp_text.push((10, String::new()));
    }

    fn xp_import_stopped(inst: &Inst) -> bool {
        inst.borrow().xp_text.iter().any(|(k, _)| *k == 10)
    }

    /// `/Root/Item`, `/Root/Item/Id` and `/Root/Item[@Qty]`: the names of the elements above the node, an attribute last in brackets.
    fn xp_node_path(schema: &Schema, node: usize) -> String {
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
    fn xp_child_named(schema: &Schema, parent: usize, name: &str, attr: bool) -> Option<usize> {
        schema.children(parent).into_iter().find(|&c| {
            let n = schema.node(c);
            let is_attr = matches!(n.kind, ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr);
            is_attr == attr && n.xml_name == name
        })
    }

    fn xp_import_container(&mut self, inst: &Inst, schema: &Schema, node: usize, elem: &Elem) -> Result<(), Signal> {
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
    fn xp_import_attrs(
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
    fn xp_assign_variable(&mut self, inst: &Inst, schema: &Schema, node: usize, text: String) -> Result<(), Signal> {
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
    /// table element (QrXpContainerOccurs.al TR_EmptyLeaf_Import on writer TR's branch al-runtime-rs-staging-tr, real BC 29; a
    /// registered assumption until the bump). The variable is set to the empty text again before the second run (the corpus
    /// triggers only read it). A Break or Quit in the first run ends the import before the second run would come: not pinned.
    fn xp_assign_leaf_text(&mut self, inst: &Inst, schema: &Schema, node: usize, ce: &Elem) -> Result<(), Signal> {
        // `<x> </x>` gives the empty text (TR_EmptyLeaf_Import `space`: `top=[];`).
        let text = ce.text();
        let text = if !text.is_empty() && text.trim().is_empty() {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportSelfClosedLeafAssignTwice);
            String::new()
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
    fn xp_refuse_self_closed_leaf(schema: &Schema, node: usize, ce: &Elem) -> Result<(), Signal> {
        if ce.self_closed && schema.node(node).trigger("onafterassignvariable").is_some() {
            return unsupported(
                "XmlPort import of `<x />` for a text element with an OnAfterAssignVariable trigger below a field element (how often the trigger runs is not pinned)",
            );
        }
        Ok(())
    }

    /// Field element or attribute: the text becomes the field's value, then `OnAfterAssignField`.
    /// Returns the field's position.
    fn xp_assign_field(&mut self, inst: &Inst, schema: &Schema, node: usize, text: &str) -> Result<usize, Signal> {
        Ok(self.xp_assign_field_flow(inst, schema, node, text)?.0)
    }

    /// `xp_assign_field`, and what `OnAfterAssignField` asked for.
    fn xp_assign_field_flow(
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
        let text = if schema.xp().preserve_whitespace && text != raw {
            if n.kind != ir::XmlNodeKind::Field || !matches!(ty, RtType::Text(_)) {
                return unsupported(
                    "XmlPort import with PreserveWhiteSpace of white space around a value that is no Text field element (not pinned)",
                );
            }
            raw
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
    fn xp_validate_field(
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

    /// The value an imported text becomes in a field of type `ty`. An empty text is the type's
    /// default (xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_EmptyIntegerElement_ImportsZero,
    /// and an export writes a blank date as an empty element).
    fn xp_value(&mut self, text: &str, ty: &RtType, xml: bool) -> Result<Value, Signal> {
        if xml {
            return self.xp_value_xml(text, ty, None);
        }
        // A Legacy port reads the display text only: the XML date is refused and nothing is stored
        // (xmlport/TestXmlPortFormatEvaluateContracts.al:Import_LegacyXmlPort_TakesTheDisplayText_AndRefusesTheXmlDate).
        if matches!(ty, RtType::Date)
            && al_builtins::iso::parse_date(text).is_some()
            && text.as_bytes().get(4) == Some(&b'-')
        {
            return Err(xerr("The date is not valid.", "DateInvalid"));
        }
        match ty {
            RtType::Text(len) | RtType::Code(len) => {
                let n = text.encode_utf16().count();
                if let Some(m) = len.filter(|m| n > *m as usize) {
                    // xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_ValueTooLongForField_Throws
                    return Err(xerr(
                        format!(
                            "The length of the string is {n}, but it must be less than or equal to {m} characters. Value: {}",
                            ops::shown_in_length_error(text)
                        ),
                        "StringLengthExceeded",
                    ));
                }
                Ok(ops::convert(Value::Text(text.to_string().into()), ty)?)
            }
            RtType::Integer | RtType::BigInteger => {
                if text.is_empty() {
                    return Ok(self.default_value(ty));
                }
                let digits = text.strip_prefix('-').unwrap_or(text);
                let ok = !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit());
                match (ty, ok.then(|| text.parse::<i64>().ok()).flatten()) {
                    (RtType::Integer, Some(n)) if i32::try_from(n).is_ok() => Ok(Value::Integer(n as i32)),
                    (RtType::BigInteger, Some(n)) => Ok(Value::BigInteger(n)),
                    _ => Err(evaluate_error(
                        text,
                        if *ty == RtType::Integer {
                            "Integer"
                        } else {
                            "BigInteger"
                        },
                    )),
                }
            }
            RtType::DateTime if text.ends_with(" AM") || text.ends_with(" PM") => {
                // The display text an export writes: `03/05/24 02:30 PM`
                // (xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_AcceptsTheTextItExports).
                let Some((date, clock)) = text.split_once(' ') else {
                    return Err(evaluate_error(text, "DateTime"));
                };
                let Value::Date(day, _) = self.xp_value(date, &RtType::Date, false)? else {
                    return Err(evaluate_error(text, "DateTime"));
                };
                let (hm, ampm) = clock.rsplit_once(' ').unwrap_or((clock, ""));
                // The display text of a DateTime with or without seconds (`03/05/24 12:34:56 PM` is what the
                // Legacy probe imports, xmlport/TestXmlPortFormatEvaluateContracts.al:167).
                let clock = if hm.matches(':').count() == 2 {
                    format!("{hm} {ampm}")
                } else {
                    format!("{hm}:00 {ampm}")
                };
                let ms = parse_time_12h(&clock).ok_or_else(|| evaluate_error(text, "DateTime"))?;
                Ok(Value::DateTime((i64::from(day) - 1) * 86_400_000 + ms + 1))
            }
            RtType::Decimal | RtType::Date | RtType::DateTime | RtType::Boolean => {
                if text.is_empty() && *ty != RtType::Boolean {
                    return Ok(self.default_value(ty));
                }
                let name = match ty {
                    RtType::Decimal => "Decimal",
                    RtType::Date => "Date",
                    RtType::DateTime => "DateTime",
                    _ => "Boolean",
                };
                let mut args = [self.default_value(ty), Value::Text(text.to_string().into())];
                self.ret_used = true;
                let ok = al_builtins::evaluate::evaluate(self, None, &mut args)?;
                if matches!(ok, Value::Boolean(true)) {
                    Ok(args[0].clone())
                } else {
                    Err(evaluate_error(text, name))
                }
            }
            RtType::Time => {
                if text.trim().is_empty() {
                    return Ok(Value::Time(0));
                }
                parse_time_12h(text.trim())
                    .map(|ms| Value::Time(ms + 1))
                    .ok_or_else(|| evaluate_error(text, "Time"))
            }
            RtType::Duration => {
                if text.trim().is_empty() {
                    return Ok(Value::Duration(0));
                }
                parse_duration_text(text)
                    .map(Value::Duration)
                    .ok_or_else(|| evaluate_error(text, "Duration"))
            }
            RtType::Option(members) => {
                let t = text.trim();
                let found = members.iter().position(|m| m.eq_ignore_ascii_case(t));
                match found {
                    Some(i) => Ok(Value::Option(i as i32, members.clone().into())),
                    None if t.is_empty() => Ok(Value::Option(0, members.clone().into())),
                    None => Err(evaluate_error(text, "Option")),
                }
            }
            RtType::Enum(id) => {
                self.note_enum(*id);
                let members = self.engine.program.objects[*id as usize].enum_values.clone();
                let t = text.trim();
                match members.iter().find(|(_, n)| n.eq_ignore_ascii_case(t)) {
                    Some((o, _)) => Ok(Value::Enum(*id, *o)),
                    None if t.is_empty() => Ok(Value::Enum(*id, members.first().map_or(0, |m| m.0))),
                    None => Err(evaluate_error(text, "Enum")),
                }
            }
            other => unsupported(format!(
                "XmlPort import into a field of type {} (not pinned)",
                other.describe()
            )),
        }
    }

    /// The value an imported text becomes under `FormatEvaluate = Xml`: the XML forms only
    /// (xmlport/TestXmlPortFormatEvaluateContracts.al:Import_XmlPort_TakesTheXmlForms_AndRefusesTheDisplayText).
    /// A Date that is not `yyyy-mm-dd` fails with the beginning of BC's text; the rest of that text is not
    /// pinned (docs/probes-needed.md). Display text for another type, and the types the probe does not
    /// cover, are refused.
    fn xp_value_xml(
        &mut self,
        text: &str,
        ty: &RtType,
        members: Option<Vec<(i32, String, String)>>,
    ) -> Result<Value, Signal> {
        match ty {
            // corpus xmlport/PWXmlPortFormatEvaluate.al:Import_Xml_Time: `hh:mm:ss` with optional
            // milliseconds; the 12-hour display text is refused.
            RtType::Time if text.is_empty() => Ok(Value::Time(0)),
            RtType::Time => al_builtins::iso::parse_time(text)
                .map(Value::Time)
                .ok_or_else(|| evaluate_error(text, "Time")),
            // Import_Xml_Duration: the ISO 8601 form (`PT1H`); milliseconds and words are refused.
            RtType::Duration if text.is_empty() => Ok(Value::Duration(0)),
            RtType::Duration => parse_iso_duration(text)
                .map(Value::Duration)
                .ok_or_else(|| evaluate_error(text, "Duration")),
            // Import_Xml_Option, Import_Xml_Enum: the member name, its ordinal or its caption.
            RtType::Option(names) => {
                let t = text.trim();
                if t.is_empty() {
                    return Ok(Value::Option(0, names.clone().into()));
                }
                let by_name = names.iter().position(|m| m.eq_ignore_ascii_case(t));
                let by_caption = members
                    .as_ref()
                    .and_then(|m| m.iter().position(|(_, _, c)| c.eq_ignore_ascii_case(t)));
                let by_ordinal = t.parse::<usize>().ok().filter(|i| *i < names.len());
                match by_name.or(by_ordinal).or(by_caption) {
                    Some(i) => Ok(Value::Option(i as i32, names.clone().into())),
                    None => Err(evaluate_error(text, "Option")),
                }
            }
            RtType::Enum(id) => {
                self.note_enum(*id);
                let t = text.trim();
                let list = match members {
                    Some(m) => m,
                    None => self.engine.program.objects[*id as usize]
                        .enum_values
                        .iter()
                        .map(|(o, n)| (*o, n.clone(), n.clone()))
                        .collect(),
                };
                if t.is_empty() {
                    return Ok(Value::Enum(*id, list.first().map_or(0, |m| m.0)));
                }
                let by_name = list.iter().find(|(_, n, _)| n.eq_ignore_ascii_case(t));
                let by_ordinal = t
                    .parse::<i32>()
                    .ok()
                    .and_then(|o| list.iter().find(|(v, _, _)| *v == o));
                let by_caption = list.iter().find(|(_, _, c)| c.eq_ignore_ascii_case(t));
                match by_name.or(by_ordinal).or(by_caption) {
                    Some((o, _, _)) => Ok(Value::Enum(*id, *o)),
                    // An ordinal the enum does not declare is imported as it is; a text that names no member raises with the
                    // captions of the members (xmlport/PWXmlPortFormatEvaluate.al:Import_Xml_Enum_Invalid, `7` and `Two`).
                    None if !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) => match t.parse::<i32>() {
                        Ok(o) => Ok(Value::Enum(*id, o)),
                        Err(_) => unsupported(format!("XmlPort import of the ordinal {t} into an Enum (not pinned)")),
                    },
                    None if t == text => Err(xerr(
                        format!(
                            "'{t}' is not an option. The existing options are: {}",
                            list.iter().map(|(_, _, c)| c.as_str()).collect::<Vec<_>>().join(",")
                        ),
                        "InvalidOptionString",
                    )),
                    None => unsupported(
                        "XmlPort import of a text with surrounding spaces that names no Enum member (not pinned)",
                    ),
                }
            }
            RtType::Text(_) | RtType::Code(_) | RtType::Integer | RtType::BigInteger => self.xp_value(text, ty, false),
            // An empty element is the type's default, a Boolean's too (XpTImportBasic.al:Import_EmptyValuesExplicit).
            RtType::Date | RtType::DateTime | RtType::Decimal | RtType::Boolean if text.is_empty() => {
                Ok(self.default_value(ty))
            }
            RtType::Date => {
                match al_builtins::iso::parse_date(text).filter(|_| text.as_bytes().get(4) == Some(&b'-')) {
                    Some(d) => Ok(Value::Date(d, false)),
                    None => Err(xerr(
                        format!("The format of the Date '{text}' does not match your device's date settings."),
                        "DateInvalid",
                    )),
                }
            }
            RtType::DateTime => {
                match al_builtins::iso::parse_datetime(text).filter(|_| text.as_bytes().get(4) == Some(&b'-')) {
                    Some(v) => Ok(Value::DateTime(v)),
                    None => Err(evaluate_error(text, "DateTime")),
                }
            }
            RtType::Decimal => {
                let plain = {
                    let digits = text.strip_prefix('-').unwrap_or(text);
                    !digits.is_empty()
                        && digits.split('.').count() <= 2
                        && digits.split('.').all(|p| p.bytes().all(|b| b.is_ascii_digit()))
                        && !digits.starts_with('.')
                };
                match text.parse::<rust_decimal::Decimal>() {
                    Ok(d) if plain => Ok(Value::Decimal(d)),
                    _ => Err(evaluate_error(text, "Decimal")),
                }
            }
            // Import_Xml_DisplayTextOfDecimalBooleanDateTime: `true`, `false` and `1`, `0`; `Yes` is refused.
            RtType::Boolean => match text {
                "true" | "1" => Ok(Value::Boolean(true)),
                "false" | "0" => Ok(Value::Boolean(false)),
                _ => Err(evaluate_error(text, "Boolean")),
            },
            // QrXpContainerOccurs.al TS_Guid_Import_* on branch al-runtime-rs-staging-pub-ts (assumption `xmlport-guid-xml`).
            RtType::Guid => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportGuidXml);
                match xml_guid(text) {
                    XmlGuid::Value(g) => Ok(Value::Guid(g)),
                    XmlGuid::Invalid => Err(xerr(
                        "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.",
                        "InvalidGuidFormat",
                    )),
                    XmlGuid::Unpinned => unsupported(format!(
                        "XmlPort import of `{text}` into a Guid field with FormatEvaluate = Xml (this form is not pinned)"
                    )),
                }
            }
            other => unsupported(format!(
                "XmlPort import into a field of type {} with FormatEvaluate = Xml (the XML form is not pinned)",
                other.describe()
            )),
        }
    }

    /// One element of a table element: a fresh record, its link fields, `OnAfterInitRecord`,
    /// the attributes and children, and the save. `first_of_node` is false for the second and
    /// later elements of a table element under the same parent: only the first gets the link
    /// (xmlport/TestXmlPortTriggersReplaceViewContracts.al:Nested_Export_GroupsTheChildrenUnderTheirParent_ImportLinksOnlyTheFirstChild,
    /// corpus: .
    fn xp_import_record(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        elem: &Elem,
        first_of_node: bool,
        keep_key: bool,
    ) -> Result<(), Signal> {
        let n = schema.node(node);
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
                    if text_in_table(schema, child) == Some(TextInTable::Nested) {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::XmlportNestedTextInTable,
                        );
                    } else {
                        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportTextInTable);
                    }
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
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportUnlinkedTempInText);
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
    fn xp_import_text_in_table(
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
    fn xp_import_linked_container(
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
    fn xp_import_field_children(
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
    fn xp_save_record(
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
        // A temporary table element saves without the table's triggers: pinned for a nested one with AutoReplace
        // (XmlPortTriggerFlow.al:551 TempChild_AutoReplace, trace :561: no `tC+` or `tC~`). Elsewhere the triggers of a temporary
        // table element are not pinned: refused where the table has one.
        let nested = n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table);
        let run_triggers = !(temporary && nested && n.auto_replace);
        if temporary
            && n.auto_save
            && !(nested && n.auto_replace)
            && (self.table_trigger(&t, "OnInsert").is_some()
                || self.table_trigger(&t, "OnModify").is_some()
                || !self.ext_triggers(&t, "OnInsert").is_empty()
                || !self.ext_triggers(&t, "OnModify").is_empty())
        {
            return unsupported(format!(
                "XmlPort import into the temporary table element {} whose table has an insert or modify trigger (whether it runs is not pinned)",
                n.xml_name
            ));
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

    /// The root and table element of a text-format port: `Root > tableelement > fieldelements`.
    /// `text_columns`: a leaf text element may be a column too (the import of a VariableText port, xmlport/lt-aj-xmlport/
    /// AjRunAndXmlPort.al:QS_VariableText_TempInteger_AutoSaveFalse: its value goes into the variable).
    fn xp_text_layout(&self, schema: &Schema, top: usize, text_columns: bool) -> Result<(usize, Vec<usize>), Signal> {
        // A text-format port may have its table element at the top, with no root element
        // (P94Fixtures.al: "LT P94 Csv Plain", exported by P94XpDelim.al:Export_NothingDeclared).
        let direct = schema.node(top).kind == ir::XmlNodeKind::Table;
        let kids = if direct { vec![top] } else { schema.children(top) };
        let [table] = kids[..] else {
            return unsupported(
                "a VariableText or FixedText port whose root has other than one table element (not pinned)",
            );
        };
        if schema.node(table).kind != ir::XmlNodeKind::Table {
            return unsupported("a VariableText or FixedText port whose root has no table element (not pinned)");
        }
        let fields = schema.children(table);
        // A field attribute of a FixedText port is a column in schema order like a field element, cut and padded to its Width
        // (xmlport/lt-ay-xmlport/AyXmlPortAttributeWidth.al:163 export, :181 attributes only, :244 import).
        let fixed = schema.xp().format == ir::XmlFormat::FixedText;
        let column = |f: usize| {
            let n = schema.node(f);
            n.kind == ir::XmlNodeKind::Field
                || (fixed && n.kind == ir::XmlNodeKind::FieldAttr)
                || (text_columns && n.kind == ir::XmlNodeKind::Text && schema.children(f).is_empty())
        };
        if fields.iter().any(|&f| !column(f)) || fields.is_empty() {
            return unsupported(
                "a VariableText or FixedText port whose table element holds more than field elements (not pinned)",
            );
        }
        if schema.xp().format == ir::XmlFormat::FixedText && fields.iter().any(|&f| schema.node(f).width.is_none()) {
            return unsupported("a FixedText field element without a Width");
        }
        Ok((table, fields))
    }

    fn xp_import_text(&mut self, inst: &Inst, schema: &Schema, top: usize, text: &str) -> Result<(), Signal> {
        let xp = schema.xp();
        let (table, fields) = self.xp_text_layout(schema, top, xp.format == ir::XmlFormat::VariableText)?;
        let record_sep = non_empty(text_separator(
            set_or_declared(inst, 2, xp.record_separator.as_deref()).as_deref(),
            "\r\n",
        )?)?;
        let variable = xp.format == ir::XmlFormat::VariableText;
        let (fsep, fdelim) = (
            non_empty(text_separator(
                set_or_declared(inst, 1, xp.field_separator.as_deref()).as_deref(),
                ",",
            )?)?,
            text_separator(set_or_declared(inst, 0, xp.field_delimiter.as_deref()).as_deref(), "\"")?,
        );
        for line in text.split(record_sep.as_str()).filter(|l| !l.is_empty()) {
            let values: Vec<String> = if variable {
                split_variable(line, &fsep, &fdelim)
            } else {
                let chars: Vec<char> = line.chars().collect();
                let mut at = 0;
                fields
                    .iter()
                    .map(|&f| {
                        let w = schema.node(f).width.expect("checked by xp_text_layout");
                        let end = (at + w).min(chars.len());
                        let v: String = chars[at.min(end)..end].iter().collect();
                        at = end;
                        v
                    })
                    .collect()
            };
            let n = schema.node(table);
            let rec = self.xp_record(inst, n)?;
            let tbl = rec.borrow().table as usize;
            let fresh = self.initial_fields(tbl);
            rec.borrow_mut().fields = fresh;
            self.xp_run_trigger(inst, schema, table, "onafterinitrecord")?;
            // Fields are read as the values come. Pinned for VariableText (AgXmlPortImport.al:678, xmlport 87567): a record that
            // ends early leaves the field elements it lacks as initialized when each is `MinOccurs = Zero` ('crlf': `AUD;036;1;0.01`
            // of 8 columns), and one value too many is left out ('too many columns'). Any other length is not pinned.
            let lacking_optional = values.len() < fields.len()
                && fields[values.len()..].iter().all(|&f| {
                    let n = schema.node(f);
                    n.kind == ir::XmlNodeKind::Field && n.min_zero
                });
            let length_pinned =
                values.len() == fields.len() || (variable && (lacking_optional || values.len() == fields.len() + 1));
            let mut assigned = Vec::new();
            let mut broke = false;
            for (&f, v) in fields.iter().zip(&values) {
                if schema.node(f).kind == ir::XmlNodeKind::Text {
                    self.xp_assign_variable(inst, schema, f, v.clone())?;
                    continue;
                }
                let is_text = {
                    let pos = schema.node(f).field.expect("field nodes have a field");
                    matches!(
                        self.engine.tables[tbl].as_ref().map(|t| &t.fields[pos].ty),
                        Some(RtType::Text(_) | RtType::Code(_))
                    )
                };
                // A fixed-width field is padded with spaces.
                let v = if variable {
                    v.as_str()
                } else if is_text {
                    v.trim_end()
                } else {
                    v.trim()
                };
                let (pos, flow) = self.xp_assign_field_flow(inst, schema, f, v)?;
                assigned.push(pos);
                // A Break after a field is assigned leaves the other fields of the record unassigned (P112XpCsv.al:
                // P112_Import_Break, idAfter2: `2=/0`); the record is still stored.
                if flow == Some(Flow::Break) {
                    broke = true;
                    break;
                }
            }
            if !broke && !length_pinned {
                return unsupported(format!(
                    "a record with {} values for a port of {} fields (not pinned)",
                    values.len(),
                    fields.len()
                ));
            }
            self.xp_save_record(inst, schema, table, &assigned, false)?;
            if Self::xp_import_stopped(inst) {
                break;
            }
        }
        Ok(())
    }

    fn xp_export_text(&mut self, inst: &Inst, schema: &Schema, top: usize) -> Result<Vec<u8>, Signal> {
        let xp = schema.xp();
        let (table, fields) = self.xp_text_layout(schema, top, false)?;
        let record_sep = text_separator(
            set_or_declared(inst, 2, xp.record_separator.as_deref()).as_deref(),
            "\r\n",
        )?;
        let variable = xp.format == ir::XmlFormat::VariableText;
        let (fsep, fdelim) = (
            text_separator(set_or_declared(inst, 1, xp.field_separator.as_deref()).as_deref(), ",")?,
            text_separator(set_or_declared(inst, 0, xp.field_delimiter.as_deref()).as_deref(), "\"")?,
        );
        let mut lines = Vec::new();
        let rows = for_each_row(self, inst, schema, table, |me, inst, schema| {
            let mut cells = Vec::new();
            for &f in &fields {
                let v = match me.xp_field_pass(inst, schema, f) {
                    Ok(FieldPass::Value(v)) => v,
                    Ok(FieldPass::Omit) => continue,
                    Ok(FieldPass::Stop) => break,
                    // A Quit in a field leaves what was written, the record cut short (P112_Export_Quit, nameBefore2).
                    Err(Signal::Exit(None)) if Session::xp_flow(inst) == Some(Flow::Quit) => {
                        if !cells.is_empty() {
                            lines.push(cells.join(if variable { fsep.as_str() } else { "" }));
                        }
                        return Err(Signal::Exit(None));
                    }
                    Err(e) => return Err(e),
                };
                cells.push(if variable {
                    format!("{fdelim}{v}{fdelim}")
                } else {
                    let w = schema.node(f).width.expect("checked by xp_text_layout");
                    let mut cut: String = v.chars().take(w).collect();
                    while cut.chars().count() < w {
                        cut.push(' ');
                    }
                    cut
                });
            }
            lines.push(cells.join(if variable { fsep.as_str() } else { "" }));
            Ok(())
        });
        // A Quit ends the export with what was written so far; the request stays set for `xp_run`.
        match rows {
            Err(Signal::Exit(None)) if Self::xp_flow(inst) == Some(Flow::Quit) => {}
            other => other?,
        }
        let text = lines.join(&record_sep);
        use al_builtins::stream::Encoding;
        if xp.utf8 {
            let mut out = vec![0xEF, 0xBB, 0xBF];
            out.extend_from_slice(text.as_bytes());
            Ok(out)
        } else if variable && xp.text_encoding.as_deref() == Some("utf16") {
            // UTF-16 little endian after FF FE (xmlport/W7XmlPortTests.al:368; XpTEncoding.al:135, 31 characters in 64 bytes).
            let mut out = vec![0xFF, 0xFE];
            out.extend(Encoding::Utf16.encode(&text));
            Ok(out)
        } else if variable && xp.text_encoding.as_deref() == Some("windows") {
            // Code page 1252, `?` for a character it lacks (W7XmlPortTests.al:366 `E9`; XpTEncoding.al:90 `ü` and `ß`, :105
            // `€` as 80, :120 `✓` as `?`). A character outside the basic plane is not pinned.
            if text.chars().any(|c| (c as u32) > 0xFFFF) {
                return unsupported(
                    "a VariableText export with TextEncoding = WINDOWS of a character outside the basic plane (not pinned)",
                );
            }
            Ok(Encoding::Windows.encode(&text))
        } else if text.is_ascii() {
            Ok(text.into_bytes())
        } else if variable
            && matches!(xp.text_encoding.as_deref(), None | Some("msdos"))
            && !text.contains('€')
            && Encoding::MsDos.encode_pinned(&text)
        {
            // Code page 437, also without a TextEncoding (W7XmlPortTests.al:365 and :369, `é` as 82; XpTEncoding.al:150
            // `ü` and `ß`). The euro sign and a character the code page lacks are not pinned for an xmlport.
            Ok(Encoding::MsDos.encode(&text))
        } else {
            unsupported(
                "a VariableText or FixedText export with characters beyond ASCII and no TextEncoding (not pinned)",
            )
        }
    }

    // ---------------------------------------------------------------- export

    fn xp_export(&mut self, inst: &Inst, schema: &Schema, guarded: bool) -> Result<Value, Signal> {
        inst.borrow_mut().xp_text.push((6, String::new()));
        let addr = Rc::as_ptr(inst) as usize;
        self.xp_pass_views.retain(|(a, _, _)| *a != addr);
        let result = self.xp_export_inner(inst, schema, guarded);
        self.xp_pass_views.retain(|(a, _, _)| *a != addr);
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
        // refuses (records.rs `XP_VIEW_AFTER_EXPORT`, assumption `xmlport-export-again`).
        for (i, n) in schema.xp().nodes.iter().enumerate() {
            if n.source_view.is_some() {
                if let Ok(rec) = self.xp_record(inst, schema.node(i)) {
                    if rec.borrow().unpinned_buffer != Some(crate::records::XP_ROW_AFTER_ONCE) {
                        rec.borrow_mut().unpinned_buffer = Some(crate::records::XP_VIEW_AFTER_EXPORT);
                    }
                }
            }
        }
        result
    }

    fn xp_export_inner(&mut self, inst: &Inst, schema: &Schema, guarded: bool) -> Result<Value, Signal> {
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
        let table_root = root_table_element(schema, top);
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
                if xp.inline_schema || !xp.namespaces.is_empty() || xp.default_ns.is_some() {
                    return unsupported(
                        "XmlPort export of a table element root with InlineSchema or namespaces (not pinned)",
                    );
                }
                let rows = match self.xp_export_table(inst, schema, top) {
                    Err(Signal::Error(_)) if guarded => {
                        return unsupported("an error inside an Export whose result is used (not pinned)");
                    }
                    other => other?,
                };
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
                for (i, (prefix, uri)) in xp.namespaces.iter().enumerate() {
                    root.attrs.insert(i, (format!("xmlns:{prefix}"), uri.clone()));
                }
                // UseDefaultNamespace = true without DefaultNamespace: the root declares BC's namespace of the port as the default
                // and the elements below inherit it (xmlport/lt-bd-xmlport/BdXmlPort.al:511, DgXmlPortLeads.al:290).
                // It follows the root's own attributes (writer TF's branch al-runtime-rs-staging-pub-tf,
                // xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al TF_NsAttr_Export: `<Root Ver="v1" xmlns="...">`).
                if let Some(ns) = &xp.default_ns {
                    root.attrs.push(("xmlns".into(), ns.clone()));
                }
                root
            }) {
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
    fn xp_apply_source_views(&mut self, inst: &Inst, schema: &Schema) -> Result<Vec<(usize, ViewKey)>, Signal> {
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
            if n.link_node.is_none() && lower.contains("where") {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportViewFilterUnlinked);
            }
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
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportExportAgain);
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
            self.set_view_in(&rec, &t, &view, Some(XP_VIEW_GROUP), false, false)?;
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
    fn xp_export_inline(&mut self, inst: &Inst, schema: &Schema, top: usize) -> Result<String, Signal> {
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
    fn xp_export_container(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<Elem, Signal> {
        let mut out = Elem::new(&schema.node(node).xml_name);
        match self.xp_export_content(inst, schema, node, &mut out) {
            Err(Signal::Exit(None)) if Self::xp_quit(inst) => {}
            other => other?,
        }
        Ok(out)
    }

    fn xp_export_content(&mut self, inst: &Inst, schema: &Schema, node: usize, out: &mut Elem) -> Result<(), Signal> {
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
                    let mut e = Elem::new(&c.xml_name);
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
                    let nested =
                        text_in_table(schema, child) == Some(TextInTable::Nested) || nested_inner(schema, child);
                    if nested {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::XmlportNestedTextInTable,
                        );
                    } else {
                        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportTextInTable);
                    }
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
                    let mut e = Elem::new(&c.xml_name);
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
                    if default_unlinked_container(schema, child, node) {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::XmlportDefaultContainerUnlinked,
                        );
                    } else if unlinked_container(schema, child, node) {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::XmlportUnlinkedTempInText,
                        );
                    } else {
                        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportUnlinkedReply);
                    }
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
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportAttrsOverUnlinked);
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
                        && !attr_container(schema, child) =>
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
    fn xp_pass_variable(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<String, Signal> {
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
    fn xp_field_text(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<String, Signal> {
        self.xp_site(inst, schema, node, "onbeforepassfield")?;
        self.xp_field_value_text(inst, schema, node)
    }

    /// The field of a text-format export: Skip leaves it out of the record, Break leaves it out and ends the record
    /// (corpus P112XpCsv.al:P112_Export_Skip and _Break, idBefore2 and nameBefore2).
    fn xp_field_pass(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<FieldPass, Signal> {
        match self.xp_site(inst, schema, node, "onbeforepassfield")? {
            Some(Flow::Skip) => return Ok(FieldPass::Omit),
            Some(Flow::Break) => return Ok(FieldPass::Stop),
            _ => {}
        }
        Ok(FieldPass::Value(self.xp_field_value_text(inst, schema, node)?))
    }

    fn xp_field_value_text(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<String, Signal> {
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
    fn xp_field_text_xml(&mut self, v: &Value) -> Result<String, Signal> {
        match v {
            Value::Integer(i) => Ok(i.to_string()),
            Value::Text(s) | Value::Code(s) => Ok(s.to_string()),
            Value::Decimal(_) => al_builtins::xmlport::field_text(v)
                .map(|t| t.replace(',', ""))
                .or_else(|why| unsupported(format!("XmlPort export of {why} (the text form is not pinned)"))),
            Value::Boolean(b) => Ok(if *b { "true" } else { "false" }.to_string()),
            Value::Date(..) | Value::DateTime(_) => al_builtins::format::try_format(v, 0, 9, self)
                .or_else(|why| unsupported(format!("XmlPort export of a value: {why}"))),
            // corpus xmlport/PWXmlPortFormatEvaluate.al:Export_Xml_TimeDurationOptionEnum: `12:34:56`,
            // `P1DT1H1M1.001S`, and the ordinal of an Option or Enum.
            Value::Time(0) | Value::Duration(0) => Ok(String::new()),
            Value::Time(t) => {
                let ms = t - 1;
                if ms % 1000 != 0 {
                    return unsupported(
                        "XmlPort export of a Time with milliseconds with FormatEvaluate = Xml (not pinned)",
                    );
                }
                let s = ms / 1000;
                Ok(format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60))
            }
            Value::Duration(d) => Ok(iso_duration(*d)),
            Value::Option(i, _) => Ok(i.to_string()),
            Value::Enum(_, o) => Ok(o.to_string()),
            // `{UPPER-CASE}`, the null Guid too (QrXpContainerOccurs.al TS_Guid_Export on branch al-runtime-rs-staging-pub-ts).
            Value::Guid(g) => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportGuidXml);
                Ok(al_builtins::format::format_guid(g))
            }
            other => unsupported(format!(
                "XmlPort export of {} with FormatEvaluate = Xml (the XML form is not pinned)",
                other.describe()
            )),
        }
    }

    /// One element per row of the table element, in the order of the table view (primary key
    /// order by default), after the table view and the link to the record above
    /// (xmlport/TestXmlPortAdvanced.al:XmlPort_NestedTableElements_ExportIncludesChildRecordsAndAttribute).
    fn xp_export_table(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<Vec<Elem>, Signal> {
        let n = schema.node(node);
        let mut out = Vec::new();
        let mut rows = Vec::new();
        self.xp_pass_start(inst, schema, node)?;
        // A linked element is filtered to its parent's row before OnPreXmlItem: it counts the parent's rows only
        // (xmlport/lt-xmlport-trigger-flow/XmlPortTriggerFlow.al:325 `Chi.PreXmlItem count=2`, then `count=1`).
        self.xp_link_filter(inst, schema, node)?;
        // A Break in OnPreXmlItem leaves the element without rows and without the empty pass (P112_Export_Break, preItem1); a Quit
        // leaves what was written so far.
        match self.xp_site(inst, schema, node, "onprexmlitem") {
            Ok(Some(Flow::Break)) => return Ok(out),
            Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
            other => {
                other?;
            }
        }
        self.xp_rows(inst, schema, node, &mut rows)?;
        // MaxOccurs = Once writes the first row only (xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al:259 SG_OnceTemp_Export_TwoRowsEach,
        // :390 SG_OnceReal_Export `two`). Pinned without export triggers; whether a trigger sees the rows after the first is not.
        // With export triggers and two rows (QrXpContainerOccurs.al TR_Booking_TwoReplyRows on branch al-runtime-rs-staging-tr):
        // only the first row's OnAfterGetRecord runs and the record holds the second row afterwards. With three rows only the
        // first is written, with one OnAfterGetRecord (TS_Again_TrMember on branch al-runtime-rs-staging-pub-ts); the row the
        // record holds afterwards is not pinned, so its buffer is marked ([`crate::records::XP_ROW_AFTER_ONCE`]).
        let mut ends_on = None;
        let mut ends_unpinned = false;
        let mut nested_after = false;
        let nested_once = n.max_unbounded == Some(false) && nested_table(schema, node);
        if nested_once {
            // A linked one below another table element (writer TT's branch al-runtime-rs-staging-tt, QrXpContainerOccurs.al
            // TT_Confirm_Export_NoPreFilter: five rows, `get detail 1` only): the first row of each pass, one OnAfterGetRecord; the row
            // the record holds afterwards is not pinned.
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportLinkedOnce);
            if rows.len() > 1 {
                rows.truncate(1);
                nested_after = true;
            }
        } else if n.max_unbounded == Some(false) && rows.len() > 1 {
            if export_triggers_below(schema, node) {
                if rows.len() > 2 {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportExportAgain);
                    ends_unpinned = true;
                } else {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportUnlinkedReply);
                    ends_on = rows.pop();
                }
            }
            rows.truncate(1);
        }
        let rec = self.xp_record(inst, n)?;
        if rows.is_empty() && !n.min_zero && rec.borrow().unpinned_buffer == Some(crate::records::XP_ROW_AFTER_ONCE) {
            return unsupported(format!(
                "XmlPort export of a table element without rows that writes its record as it is: {}",
                crate::records::XP_ROW_AFTER_ONCE
            ));
        }
        // A table element without rows still passes its fields once, without a row: OnBeforePassField runs and no
        // OnAfterGetRecord (P94XpPathFlow.al:Export_PathsNoRows). Unless it is optional, the element is written once with the
        // values of the record as it stands (xmlport/lt-xmlport-query-datatransfer/XpTExportBasic.al:Export_StaticWithRecordNoRows,
        // Export_EmptyTable: 462 bytes, a row of `0`, empty text, `0.00`, `No`).
        if rows.is_empty() && !n.min_zero {
            let mut e = Elem::new(&n.xml_name);
            let r = self.xp_export_content(inst, schema, node, &mut e);
            out.push(e);
            match r {
                Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
                other => other?,
            }
        } else if rows.is_empty() && n.link_node.is_some() {
            // An optional linked element without rows passes no field (XmlPortTriggerFlow.al:353 Export_TriggerOrder_EmptyTables:
            // `Chi.PreXmlItem count=0` and no `Chi.Id.BeforePassField`).
        } else if rows.is_empty() && !n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table) {
            // An optional element that is not linked and not below another table element runs no trigger of its nodes without rows:
            // only its OnPreXmlItem ran (corpus writer TT's branch al-runtime-rs-staging-tt, QrXpContainerOccurs.al
            // TT_Optional_Unlinked_NoRows: `pre opt;` and no `pass opt no;`; assumption `xmlport-optional-no-rows`).
            if export_triggers_below(schema, node) {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportOptionalNoRows);
            }
        } else if rows.is_empty() {
            // P94XpPathFlow.al:Export_PathsNoRows pins the empty pass for an element that is not optional only; for an optional
            // element below another table element that is not linked, whether its fields are passed is not pinned.
            let passes = schema.children(node).into_iter().any(|c| {
                let k = schema.node(c);
                matches!(k.kind, ir::XmlNodeKind::Field | ir::XmlNodeKind::FieldAttr)
                    && k.trigger("onbeforepassfield").is_some()
            });
            if passes {
                return unsupported(
                    "XmlPort export of an optional table element without rows whose fields have OnBeforePassField (not pinned)",
                );
            }
        }
        for row in rows {
            self.load(&rec, row)?;
            if matches!(
                rec.borrow().unpinned_buffer,
                Some(crate::records::XP_ROW_AFTER_ONCE | crate::records::XP_NESTED_ROW_AFTER_ONCE)
            ) {
                rec.borrow_mut().unpinned_buffer = None;
            }
            // Skip leaves the row out, Break ends the rows; BreakUnbound ends the trigger and the row goes on (P112_Export_Skip,
            // _Break, _BreakUnbound at getRec2).
            match self.xp_site(inst, schema, node, "onaftergetrecord") {
                Ok(Some(Flow::Break | Flow::Skip)) if ends_on.is_some() || nested_after => {
                    return unsupported(
                        "XmlPort export of a table element with MaxOccurs = Once and two rows whose first OnAfterGetRecord skips or breaks (not pinned)",
                    );
                }
                Ok(Some(Flow::Break)) => break,
                Ok(Some(Flow::Skip)) => continue,
                Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
                Err(e) => return Err(e),
                Ok(_) => {}
            }
            let mut e = Elem::new(&n.xml_name);
            let r = self.xp_export_content(inst, schema, node, &mut e);
            out.push(e);
            match r {
                Err(Signal::Exit(None)) if Self::xp_quit(inst) => return Ok(out),
                other => other?,
            }
            if Self::xp_quit(inst) {
                return Ok(out);
            }
        }
        if let Some(row) = ends_on {
            self.load(&rec, row)?;
        }
        if ends_unpinned {
            rec.borrow_mut().unpinned_buffer = Some(crate::records::XP_ROW_AFTER_ONCE);
        }
        if nested_after {
            rec.borrow_mut().unpinned_buffer = Some(crate::records::XP_NESTED_ROW_AFTER_ONCE);
        }
        Ok(out)
    }

    /// The key each table element with LinkTable and no LinkFields below another table element has once the SourceTableViews are
    /// applied, before the first row of the export is read (`xp_pass_start`). Only the view's sorting (ascending; a later export on
    /// the same instance keeps the record's key, pinned only while it is the view's) or the primary key ascending without a
    /// sorting in the view is pinned (writer TU's branch
    /// al-runtime-rs-staging-tu, QrXpContainerOccurs.al TU_LinkedKey_NoChange `pre c1 [VERSION(1) SORTING(Field2)]` with
    /// `sorting(A)`, `pre d1 [VERSION(1) SORTING(Field1)]` without a view).
    fn xp_pass_key_snapshots(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        view_keys: &[(usize, ViewKey)],
    ) -> Result<(), Signal> {
        let addr = Rc::as_ptr(inst) as usize;
        for node in 0..schema.xp().nodes.len() {
            let n = schema.node(node);
            if !nested_table(schema, node) || n.link_node.is_none() || !n.link_fields.is_empty() {
                continue;
            }
            let view = n
                .source_view
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .replace(' ', "");
            let rec = self.xp_record(inst, n)?;
            let r = rec.borrow();
            // A later export on the same instance keeps the record's key (`xp_apply_source_views`): pinned only while it is still the
            // view's own.
            let ascending = r.ascending && r.field_descending == 0;
            let by_view = view.contains("sorting(")
                && ascending
                && view_keys
                    .iter()
                    .any(|(i, (k, a))| *i == node && *k == r.key && *a == r.adhoc);
            let primary = !view.contains("sorting(") && r.key == 0 && r.adhoc.is_none() && ascending;
            let v = PassView {
                filters: Vec::new(),
                filter_group: r.filter_group,
                key: r.key,
                adhoc: r.adhoc.clone(),
                ascending: r.ascending,
                field_descending: r.field_descending,
                pinned_key: !view.contains("order(") && (by_view || primary),
                started: false,
            };
            drop(r);
            self.xp_pass_views.push((addr, node, v));
        }
        Ok(())
    }

    /// The start of a pass of table element `node` with LinkTable and no LinkFields below another table element: LinkTable adds no
    /// filter, and every pass after the first starts from the filters the record had at the first, its SourceTableView included,
    /// whatever OnPreXmlItem of an earlier pass set or reset (writer TT's branch al-runtime-rs-staging-tt, QrXpContainerOccurs.al
    /// TT_Confirm_Export_NpFilters `pre detail a2 filters= count=5` after a1's Reset and filters, TT_Confirm_Export_TwoReplies
    /// `pre ticket filters=` at reply 2; assumption `xmlport-link-pass-reset`), and from the SourceTableView's key, or the primary
    /// key without a sorting in the view, ascending, whatever key or direction OnPreXmlItem of an earlier pass set (writer TU's
    /// branch al-runtime-rs-staging-tu, QrXpContainerOccurs.al TU_LinkedKey_KeyAtFirstPass, _KeyDescendingAtFirstPass,
    /// _DescendingAtFirstPass, _ResetAtFirstPass, _KeyAtSecondPass, _KeyAtEveryPass). Filters, marks, a key or a direction of the
    /// code's own on the record at the first pass are not pinned.
    fn xp_pass_start(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<(), Signal> {
        let n = schema.node(node);
        if !nested_table(schema, node) || n.link_node.is_none() || !n.link_fields.is_empty() {
            return Ok(());
        }
        let rec = self.xp_record(inst, n)?;
        let addr = Rc::as_ptr(inst) as usize;
        let Some(pos) = self.xp_pass_views.iter().position(|(a, i, _)| *a == addr && *i == node) else {
            return unsupported(format!(
                "XmlPort export of table element {} (LinkTable without LinkFields below another table element) outside an XML export (not pinned)",
                n.name
            ));
        };
        let v = self.xp_pass_views[pos].2.clone();
        if v.started {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportLinkPassReset);
            let mut r = rec.borrow_mut();
            r.filters = v.filters;
            r.filter_group = v.filter_group;
            r.key = v.key;
            r.adhoc = v.adhoc;
            r.ascending = v.ascending;
            r.field_descending = v.field_descending;
            r.marked_only = false;
            return Ok(());
        }
        let r = rec.borrow();
        if !v.pinned_key
            || r.key != v.key
            || r.adhoc != v.adhoc
            || r.ascending != v.ascending
            || r.field_descending != v.field_descending
        {
            return unsupported(format!(
                "XmlPort export of table element {} (LinkTable without LinkFields below another table element) whose record has a key or direction other than its SourceTableView's sorting or the primary key ascending at the first pass (not pinned)",
                n.name
            ));
        }
        if r.marked_only || r.filters.iter().any(|f| f.group != XP_VIEW_GROUP) {
            return unsupported(format!(
                "XmlPort export of table element {} (LinkTable without LinkFields below another table element) whose record has filters or marks of the code's own (not pinned)",
                n.name
            ));
        }
        let filters = r.filters.clone();
        let filter_group = r.filter_group;
        drop(r);
        let e = &mut self.xp_pass_views[pos].2;
        e.filters = filters;
        e.filter_group = filter_group;
        e.started = true;
        Ok(())
    }

    /// Filters linked table element `node` to the row its LinkTable holds.
    fn xp_link_filter(&mut self, inst: &Inst, schema: &Schema, node: usize) -> Result<(), Signal> {
        let n = schema.node(node);
        let Some(link) = n.link_node else { return Ok(()) };
        let rec = self.xp_record(inst, n)?;
        let parent = self.xp_record(inst, schema.node(link))?;
        for &(own, other) in &n.link_fields {
            let v = parent.borrow().fields[other].clone();
            self.ret_used = false;
            self.record_op(RecordOp::SetRange, &rec, &mut [Value::Integer(own as i32), v])?;
        }
        Ok(())
    }

    /// The rows of table element `node` in export order.
    fn xp_rows(&mut self, inst: &Inst, schema: &Schema, node: usize, rows: &mut Vec<Vec<Value>>) -> Result<(), Signal> {
        let n = schema.node(node);
        let rec = self.xp_record(inst, n)?;
        let t = self.table(&rec)?;
        self.xp_link_filter(inst, schema, node)?;
        let (key, asc) = {
            let r = rec.borrow();
            (r.key, r.ascending)
        };
        let mut after = None;
        while let Some(row) = self.scan(&rec, &t, asc, after.take())? {
            after = Some(position(&t, key, &row));
            rows.push(row);
        }
        Ok(())
    }
}

/// Runs `body` once for every row of table element `node`, with the row loaded.
/// The key and ad hoc key a SourceTableView's sorting gives a table element record.
type ViewKey = (usize, Option<(usize, Vec<usize>)>);

/// The view of a table element record at its first pass of an export (`Session::xp_pass_start`).
#[derive(Clone)]
pub(crate) struct PassView {
    filters: Vec<al_builtins::filter::FieldFilter>,
    filter_group: i32,
    key: usize,
    adhoc: Option<(usize, Vec<usize>)>,
    ascending: bool,
    field_descending: u32,
    /// Whether the key at the export's start is one TU pinned (the view's sorting or the primary key, ascending).
    pinned_key: bool,
    /// Whether the first pass ran (its filters are recorded).
    started: bool,
}

/// Whether node `node` is a table element right below another table element.
fn nested_table(schema: &Schema, node: usize) -> bool {
    let n = schema.node(node);
    n.kind == ir::XmlNodeKind::Table && n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table)
}

/// What the field of a text export gives.
enum FieldPass {
    Value(String),
    Omit,
    Stop,
}

/// What a trigger asked for: leave the table element's rows, skip the rest of the row, or end the run.
#[derive(Clone, Copy, PartialEq)]
enum Flow {
    Break,
    Unbound,
    Quit,
    Skip,
}

fn for_each_row(
    me: &mut Session,
    inst: &Inst,
    schema: &Schema,
    node: usize,
    mut body: impl FnMut(&mut Session, &Inst, &Schema) -> Result<(), Signal>,
) -> Result<(), Signal> {
    let mut rows = Vec::new();
    if me.xp_site(inst, schema, node, "onprexmlitem")? == Some(Flow::Break) {
        return Ok(());
    }
    me.xp_rows(inst, schema, node, &mut rows)?;
    let rec = me.xp_record(inst, schema.node(node))?;
    for row in rows {
        me.load(&rec, row)?;
        match me.xp_site(inst, schema, node, "onaftergetrecord")? {
            Some(Flow::Break) => break,
            Some(Flow::Skip) => continue,
            _ => {}
        }
        body(me, inst, schema)?;
    }
    Ok(())
}

/// A separator property: `<LF>`, `<CR>` and `<TAB>` stand for the control characters, other text
/// is itself. Other `<...>` names are not pinned.
/// A declared separator as the getter shows it: `<None>` is empty, `<CR/LF>`, `<LF>`, `<CR>` and `<Tab>` are their escapes, anything
/// else is as written (P94XpDelim.al:Getters_Declared, Getters_DeclaredTwoTables). Other tokens are not pinned.
/// The name of node `c` in the document: a field element with a `NamespacePrefix` is `prefix:Name`.
/// A `Format = Xml` port whose root is a table element of plain field elements, without triggers, attributes, a link or
/// a MinOccurs (xmlports 87562 and 87563 of platform/lt-ag-contoso/AgXmlPortImport.al:784-833, the DemoTool's xmlport
/// 101899). Other shapes of a table element root are not pinned.
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
        (Some(p), ir::XmlNodeKind::Field) => format!("{p}:{}", n.xml_name),
        _ => n.xml_name.clone(),
    }
}

/// A document read as UTF-16 whose declaration says `utf-8`: the reader switches to UTF-8 after the declaration and meets the
/// zero byte that follows the next `<`: `Name cannot begin with the '.' character, hexadecimal value 0x00. Line 1, position N.`
/// with N two past the end of the declaration (xmlport/lt-af-xmlport/AfXmlPortEncoding.al:
/// AF_Encoding_Import_Utf8Property_Utf16DocDeclaredUtf8, a declaration followed at once by the root element). Any other
/// shape after the declaration is not pinned.
fn utf16_declared_utf8(text: String) -> Result<String, Signal> {
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

fn declared_separator(v: &str) -> Result<String, Signal> {
    Ok(match v.to_ascii_uppercase().as_str() {
        "<NONE>" => String::new(),
        "<CR/LF>" => "\\r\\n".into(),
        "<LF>" => "\\n".into(),
        "<CR>" => "\\r".into(),
        "<TAB>" => "\\t".into(),
        u if u.starts_with('<') && u.ends_with('>') => {
            return unsupported(format!("the separator token {v} read back (not pinned)"));
        }
        _ => v.to_string(),
    })
}

/// The text a setter stored for separator `kind` (0 delimiter, 1 field, 2 record, 3 table), else the declared one.
fn set_or_declared(inst: &Inst, kind: u8, declared: Option<&str>) -> Option<String> {
    let set = inst
        .borrow()
        .xp_text
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, v)| v.clone());
    set.or_else(|| declared.map(str::to_string))
}

/// A field or record separator has to hold text; an empty one is not pinned.
fn non_empty(s: String) -> Result<String, Signal> {
    if s.is_empty() {
        unsupported("an empty field or record separator (not pinned)")
    } else {
        Ok(s)
    }
}

fn text_separator(value: Option<&str>, default: &str) -> Result<String, Signal> {
    let Some(v) = value else { return Ok(default.to_string()) };
    let mut out = String::new();
    let mut rest = v;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        match rest[start + 1..start + end].to_ascii_uppercase().as_str() {
            "LF" => out.push('\n'),
            "CR" => out.push('\r'),
            "TAB" => out.push('\t'),
            // <NewLine> writes CR LF (XpTVariableText.al:Csv_ExportThreeRows reads the lines back as <NL>-separated).
            "NEWLINE" | "CR/LF" => out.push_str("\r\n"),
            // <None> is the empty text (XmlPortFixtures.al "LT XpT Csv Semi" has no field delimiter).
            "NONE" => {}
            other => return unsupported(format!("the separator token <{other}> (not pinned)")),
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Splits a VariableText record at `sep`. A value that starts with the delimiter runs to the next
/// delimiter and may hold the separator; a doubled delimiter is not an escape, so the text after
/// the closing delimiter starts the next value
/// (xmlport/TestXmlPortTextFormatsContracts.al:VariableText_Import_DoubledQuoteInsideAQuotedValue_IsNotAnEscape,
/// corpus: .
fn split_variable(line: &str, sep: &str, delim: &str) -> Vec<String> {
    if delim.is_empty() {
        return line.split(sep).map(str::to_string).collect();
    }
    let mut out = Vec::new();
    let mut rest = line;
    loop {
        if let Some(after) = rest.strip_prefix(delim) {
            match after.find(delim) {
                Some(end) => {
                    out.push(after[..end].to_string());
                    rest = &after[end + delim.len()..];
                    match rest.strip_prefix(sep) {
                        Some(r) => rest = r,
                        None if rest.is_empty() => return out,
                        None => {}
                    }
                    if rest.is_empty() && !line.ends_with(sep) {
                        return out;
                    }
                }
                None => {
                    out.push(after.to_string());
                    return out;
                }
            }
        } else {
            match rest.find(sep) {
                Some(end) => {
                    out.push(rest[..end].to_string());
                    rest = &rest[end + sep.len()..];
                }
                None => {
                    out.push(rest.to_string());
                    return out;
                }
            }
        }
    }
}

/// `h:mm:ss AM` as milliseconds since midnight.
fn parse_time_12h(t: &str) -> Option<i64> {
    let (clock, ampm) = t.rsplit_once(' ')?;
    let mut it = clock.split(':');
    let (h, m, s): (i64, i64, i64) = (
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
        it.next()?.parse().ok()?,
    );
    if it.next().is_some() || !(1..=12).contains(&h) || m > 59 || s > 59 {
        return None;
    }
    let pm = match ampm {
        "AM" => false,
        "PM" => true,
        _ => return None,
    };
    Some((((h % 12) + if pm { 12 } else { 0 }) * 3600 + m * 60 + s) * 1000)
}

/// `1 day 1 hour 1 minute 1 second 1 millisecond` (the text an export writes) as milliseconds.
/// An ISO 8601 duration (`P1DT1H1M1.001S`, `PT1H`) in milliseconds.
fn parse_iso_duration(t: &str) -> Option<i64> {
    let rest = t.strip_prefix('P')?;
    let (days, time) = match rest.split_once('T') {
        Some((d, tm)) => (d, tm),
        None => (rest, ""),
    };
    if days.is_empty() && time.is_empty() {
        return None;
    }
    let mut total = 0i64;
    let mut take = |part: &str, units: &[(char, f64)]| -> Option<()> {
        let mut num = String::new();
        for c in part.chars() {
            if c.is_ascii_digit() || c == '.' {
                num.push(c);
                continue;
            }
            let mult = units.iter().find(|(u, _)| *u == c)?.1;
            let n: f64 = num.parse().ok()?;
            total += (n * mult).round() as i64;
            num.clear();
        }
        num.is_empty().then_some(())
    };
    take(days, &[('D', 86_400_000.0)])?;
    take(time, &[('H', 3_600_000.0), ('M', 60_000.0), ('S', 1000.0)])?;
    Some(total)
}

/// `P1DT1H1M1.001S` for 90061001 ms; zero parts are left out.
fn iso_duration(ms: i64) -> String {
    let (days, rest) = (ms / 86_400_000, ms % 86_400_000);
    let (h, rest) = (rest / 3_600_000, rest % 3_600_000);
    let (m, rest) = (rest / 60_000, rest % 60_000);
    let (sec, milli) = (rest / 1000, rest % 1000);
    let mut out = String::from("P");
    if days != 0 {
        out.push_str(&format!("{days}D"));
    }
    if h != 0 || m != 0 || rest != 0 {
        out.push('T');
        if h != 0 {
            out.push_str(&format!("{h}H"));
        }
        if m != 0 {
            out.push_str(&format!("{m}M"));
        }
        if rest != 0 {
            out.push_str(&if milli == 0 {
                format!("{sec}S")
            } else {
                format!("{sec}.{milli:03}S")
            });
        }
    }
    out
}

fn parse_duration_text(t: &str) -> Option<i64> {
    let mut total = 0i64;
    let mut words = t.split_whitespace();
    let mut any = false;
    while let Some(n) = words.next() {
        let n: i64 = n.parse().ok()?;
        let unit = words.next()?.trim_end_matches('s');
        total += n * match unit {
            "day" => 86_400_000,
            "hour" => 3_600_000,
            "minute" => 60_000,
            "second" => 1000,
            "millisecond" => 1,
            _ => return None,
        };
        any = true;
    }
    any.then_some(total)
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

/// Whether node `node` or a node below it has a trigger an export runs.
fn export_triggers_below(schema: &Schema, node: usize) -> bool {
    let n = schema.node(node);
    ["onaftergetrecord", "onbeforepassfield", "onbeforepassvariable"]
        .iter()
        .any(|t| n.trigger(t).is_some())
        || schema
            .children(node)
            .into_iter()
            .any(|c| export_triggers_below(schema, c))
}

/// The two shapes of a text element right below a table element that the corpus pins with its triggers and text
/// (assumption `xmlport-text-in-table`; corpus branch al-runtime-rs-staging-pub-to,
/// xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TO_AttrItem_*, TO_TextContent_*, objects
/// ToXpTextInTableObjects.al xmlports 90001-90003; the same test as al-sema xmlports.rs `text_in_table_shape`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextInTable {
    /// Its nodes are only text attributes (xmlports 90001, 90002): `<item option_value="1">REQUIRED</item>`.
    Attrs,
    /// Text attributes and leaf text and field elements, at least one element (xmlport 90003).
    Content,
    /// Field and text attributes and exactly one text element whose nodes are only text attributes (assumption
    /// `xmlport-nested-text-in-table`; corpus branch al-runtime-rs-staging-pub-tp, QrXpContainerOccurs.al TP_Nested_*, xmlport
    /// 90022 in TpXpNestedObjects.al): `<entry no="R1" status="open"><admission option_value="1">A1</admission></entry>`.
    Nested,
}

fn text_in_table(schema: &Schema, c: usize) -> Option<TextInTable> {
    let n = schema.node(c);
    let kids = schema.children(c);
    let below_table = n.parent.is_some_and(|p| schema.node(p).kind == ir::XmlNodeKind::Table);
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

/// A text element whose nodes are only text attributes (at least one).
fn attrs_only(schema: &Schema, c: usize) -> bool {
    let kids = schema.children(c);
    !kids.is_empty() && kids.iter().all(|&a| schema.node(a).kind == ir::XmlNodeKind::TextAttr)
}

/// The inner element of a `TextInTable::Nested` text element.
fn nested_inner(schema: &Schema, c: usize) -> bool {
    schema.node(c).kind == ir::XmlNodeKind::Text
        && schema
            .node(c)
            .parent
            .is_some_and(|p| text_in_table(schema, p) == Some(TextInTable::Nested))
}

/// A text element without triggers whose nodes are all attributes (at least one).
fn attr_container(schema: &Schema, c: usize) -> bool {
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
/// attributes (assumption `xmlport-unlinked-temp-in-text`; corpus branch al-runtime-rs-staging-pub-tp,
/// xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TP_Unlinked_*, xmlport 90023 in TpXpNestedObjects.al).
fn unlinked_container(schema: &Schema, c: usize, table: usize) -> bool {
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
/// (assumption `xmlport-unlinked-reply`; corpus branch al-runtime-rs-staging-tr, QrXpContainerOccurs.al TR_Booking_*, xmlport
/// 90064 in TrXpReplyObjects.al: `details` > `line` with text attributes and `parts` > `Part`). Leaf field elements of the
/// outer table without triggers may come before the inner table (TR_Member_*, xmlport 90065: `member` > `membernumber`,
/// `email`, `card`).
fn unlinked_reply_container(schema: &Schema, c: usize, table: usize) -> bool {
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
/// empty without rows, with the first row only otherwise (assumption `xmlport-default-container-unlinked`; corpus branch
/// al-runtime-rs-staging-pub-ts, QrXpContainerOccurs.al TS_Cards_DefaultOccurs_Export, xmlport 90084 in TsXpObjects.al).
fn default_unlinked_container(schema: &Schema, c: usize, table: usize) -> bool {
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
/// and `MaxOccurs = Unbounded` (assumption `xmlport-attrs-over-unlinked`; corpus writer TT's branch al-runtime-rs-staging-tt,
/// QrXpContainerOccurs.al TT_Change_Export_*, xmlport 90104 in TtXpObjects.al: `Ticket` > `Admissions` > `Admission`).
fn attrs_over_unlinked(schema: &Schema, c: usize, table: usize) -> bool {
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
enum XmlGuid {
    Value([u8; 16]),
    Invalid,
    Unpinned,
}

/// The XML forms of a Guid an import takes (QrXpContainerOccurs.al TS_Guid_Import_Element, TS_Guid_Import_Attribute on branch
/// al-runtime-rs-staging-pub-ts): the 8-4-4-4-12 form bare, in braces or in parentheses, or the 32 digits bare, in any case, with
/// spaces around; empty is the null Guid. A text with a character that is no hex digit, dash, bracket or space (`not-a-guid`),
/// or the 8-4-4-4-12 form one digit short, is invalid. Other forms (mismatched brackets, digits in brackets, other lengths) are
/// not pinned.
fn xml_guid(text: &str) -> XmlGuid {
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

fn linked_container(schema: &Schema, c: usize, table: usize) -> bool {
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
/// and a container text element may not (as before the MaxOccurs pins).
fn repeats(schema: &Schema, c: usize) -> bool {
    let n = schema.node(c);
    n.max_unbounded.unwrap_or(match n.kind {
        ir::XmlNodeKind::Table => true,
        ir::XmlNodeKind::Text => schema.children(c).is_empty(),
        _ => false,
    })
}

/// Checks the child elements of an imported element against the schema's sequence: the children
/// in declaration order, each once (a table element any number of times) unless it is optional
/// (`MinOccurs = Zero`). The messages are .NET's schema validation messages
/// (xmlport/TestXmlPortExportImportContracts.al:XmlPortImport_SchemaMismatch_ThrowsXmlPortInvalidXml).
struct Sequence {
    /// `UseLax = true` on the port.
    lax: bool,
    decl: Vec<usize>,
    parent: String,
    pos: usize,
    seen: usize,
    optional: Vec<bool>,
    repeat: Vec<bool>,
    /// The name an element has in the document: `prefix:Name` for a prefixed field element.
    names: Vec<String>,
    /// The name as an error lists it, quoted: `'Name'`, or `'Name' in namespace 'uri'` for a prefixed element.
    shown: Vec<String>,
}

impl Sequence {
    fn new(schema: &Schema, node: usize) -> Sequence {
        let decl: Vec<usize> = schema
            .children(node)
            .into_iter()
            .filter(|&c| {
                !matches!(
                    schema.node(c).kind,
                    ir::XmlNodeKind::TextAttr | ir::XmlNodeKind::FieldAttr
                )
            })
            .collect();
        Sequence {
            lax: schema.xp().use_lax,
            parent: schema.node(node).xml_name.clone(),
            pos: 0,
            seen: 0,
            optional: decl.iter().map(|&c| schema.node(c).min_zero).collect(),
            repeat: decl.iter().map(|&c| repeats(schema, c)).collect(),
            names: decl.iter().map(|&c| doc_name(schema, c)).collect(),
            shown: decl
                .iter()
                .map(|&c| {
                    let n = schema.node(c);
                    match n
                        .ns_prefix
                        .as_deref()
                        .and_then(|p| schema.xp().namespaces.iter().find(|(q, _)| q == p))
                    {
                        Some((_, uri)) => format!("'{}' in namespace '{uri}'", n.xml_name),
                        None => format!("'{}'", n.xml_name),
                    }
                })
                .collect(),
            decl,
        }
    }

    /// The names that may come next.
    fn expected(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.pos >= self.decl.len() {
            return out;
        }
        let done = self.seen >= 1 || self.optional[self.pos];
        if self.seen == 0 || self.repeat[self.pos] {
            out.push(self.shown[self.pos].clone());
        }
        if done {
            for j in self.pos + 1..self.decl.len() {
                out.push(self.shown[j].clone());
                if !self.optional[j] {
                    break;
                }
            }
        }
        out
    }

    /// Several possible elements are listed in one pair of quotes, `'Opt, Once'` (xmlport/W7XmlPortTests.al:129); with a
    /// namespace among them the form is not pinned.
    fn list(&self, head: String) -> Result<String, Signal> {
        let expected = self.expected();
        match expected.len() {
            0 => Ok(head),
            1 => Ok(format!("{head} List of possible elements expected: {}.", expected[0])),
            _ if expected.iter().any(|e| e.contains(" in namespace ")) => unsupported(
                "a schema error that lists several possible elements, one of them in a namespace (the list form is not pinned)",
            ),
            _ => {
                let names: Vec<&str> = expected.iter().map(|e| e.trim_matches('\'')).collect();
                Ok(format!(
                    "{head} List of possible elements expected: '{}'.",
                    names.join(", ")
                ))
            }
        }
    }

    /// The schema node that reads child element `ce`, `None` for an element a UseLax port skips.
    /// An element that declares a default namespace (`xmlns="uri"`) is in that namespace, which no node of the port is: a strict
    /// port names it in the error, a UseLax port skips it (YXmlPortProps.al:UseLax_Import_AcceptsUnknownElements). A UseLax port's
    /// schema ends every sequence with "any element in namespace '##other'", which its error lists. Both are pinned only for an
    /// element after the last element of the sequence; elsewhere the list is not pinned and is refused.
    fn accept(&mut self, schema: &Schema, ce: &Elem) -> Result<Option<usize>, Signal> {
        let name = ce.name.as_str();
        if let Some(ns) = ce.attr("xmlns").filter(|ns| !ns.is_empty()) {
            if !self.expected().is_empty() {
                return unsupported(
                    "XmlPort import of an element of another namespace where the sequence still expects elements (not pinned)",
                );
            }
            if self.lax {
                return Ok(None);
            }
            return Err(schema_error(format!(
                "The element '{}' has invalid child element '{name}' in namespace '{ns}'.",
                self.parent
            )));
        }
        if let Some(found) = self.accept_name(schema, name)? {
            return Ok(Some(found));
        }
        if self.lax {
            if !self.expected().is_empty() {
                return unsupported(
                    "XmlPort import into a UseLax port of an unknown element where the sequence still expects elements (not pinned)",
                );
            }
            return Err(schema_error(format!(
                "The element '{}' has invalid child element '{name}'. List of possible elements expected: any element in namespace '##other'.",
                self.parent
            )));
        }
        Err(schema_error(self.list(format!(
            "The element '{}' has invalid child element '{name}'.",
            self.parent
        ))?))
    }

    /// The schema node of the sequence named `name`, `None` when none may come here.
    fn accept_name(&mut self, _schema: &Schema, name: &str) -> Result<Option<usize>, Signal> {
        if self.pos < self.decl.len() {
            let cur = self.pos;
            if self.names[cur] == name && (self.seen == 0 || self.repeat[cur]) {
                self.seen += 1;
                return Ok(Some(self.decl[cur]));
            }
            if self.seen >= 1 || self.optional[cur] {
                for j in cur + 1..self.decl.len() {
                    if self.names[j] == name {
                        self.pos = j;
                        self.seen = 1;
                        return Ok(Some(self.decl[j]));
                    }
                    if !self.optional[j] {
                        break;
                    }
                }
            }
        }
        Ok(None)
    }

    fn finish(&self, _schema: &Schema) -> Result<(), Signal> {
        if self.decl.is_empty() {
            return Ok(());
        }
        let done = self.seen >= 1 || self.optional[self.pos];
        if done && (self.pos + 1..self.decl.len()).all(|j| self.optional[j]) {
            return Ok(());
        }
        Err(schema_error(self.list(format!(
            "The element '{}' has incomplete content.",
            self.parent
        ))?))
    }
}

#[cfg(test)]
mod tests {
    use super::{declared_separator, split_variable};

    // xmlport/TestXmlPortTextFormatsContracts.al:VariableText_Import_QuotedUnquotedAndBlankValues
    #[test]
    fn a_variable_text_record_splits_into_its_values() {
        let split = |l: &str| split_variable(l, ",", "\"");
        assert_eq!(
            split("\"1\",\"a,b\",\"1,234.50\",\"03/05/24\""),
            ["1", "a,b", "1,234.50", "03/05/24"]
        );
        assert_eq!(split("\"2\",\"plain\",\"0\",\"\""), ["2", "plain", "0", ""]);
        assert_eq!(split("3,unquoted,5.5,01/02/24"), ["3", "unquoted", "5.5", "01/02/24"]);
        // A doubled delimiter is no escape: the text after the closing one starts the next value.
        assert_eq!(
            split("\"2\",\"say \"\"hi\"\"\",\"0\",\"\""),
            ["2", "say ", "hi", "", "0", ""]
        );
    }

    // platform/lt-xmlport-session-navapp-cookie/P94XpDelim.al:Getters_Declared, Getters_DeclaredTwoTables
    #[test]
    fn a_declared_separator_reads_back_as_backslash_text() {
        assert_eq!(declared_separator("<CR/LF>").unwrap(), "\\r\\n");
        assert_eq!(declared_separator("<LF>").unwrap(), "\\n");
        assert_eq!(declared_separator("<None>").unwrap(), "");
        assert_eq!(declared_separator(";").unwrap(), ";");
        assert!(declared_separator("<Space>").is_err());
    }
}
