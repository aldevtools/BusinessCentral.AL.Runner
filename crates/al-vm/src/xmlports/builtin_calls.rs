//! The XmlPort builtins that need more than a few lines: Break, BreakUnbound, Quit and Skip, `Filename`, `TextEncoding`, and the
//! static `Export`, `Import` and `Run`. `xmlport_builtin` (mod.rs) dispatches to them.

use super::*;

impl Session {
    /// Break, BreakUnbound, Quit and Skip end the trigger that calls them (metadata/session-system/P94XpPathFlow.al:
    /// Export_Break, Export_BreakUnbound, Export_Quit); what the run does next is `xp_flow`.
    pub(super) fn xp_flow_builtin(&mut self, id: al_ir::BuiltinId, receiver: Option<Value>) -> Result<Value, Signal> {
        let inst = self.xp_instance(receiver)?;
        let trigger_depth = inst
            .borrow()
            .xp_text
            .iter()
            .find(|(k, _)| *k == 9)
            .and_then(|(_, v)| v.parse::<usize>().ok());
        // From the trigger itself or from any procedure it calls: the whole trigger ends (corpus xml/xml/
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

    /// Filename(): empty until set, per instance (P94XpFilename.al: Fresh_Empty, Set_ReadBack, Set_TwoInstancesIndependent).
    pub(super) fn xp_filename_builtin(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
    ) -> Result<Value, Signal> {
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

    /// TextEncoding(): MSDos until declared or set (UTF8 when declared so); the setter takes the member (P94XpEncoding.al:
    /// Getter_Defaults, Setter_ReadBack). Which bytes an export then writes is separate.
    pub(super) fn xp_text_encoding_builtin(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
    ) -> Result<Value, Signal> {
        const MEMBERS: [&str; 4] = ["MSDos", "UTF8", "UTF16", "Windows"];
        let inst = self.xp_instance(receiver)?;
        let members =
            || -> std::sync::Arc<[String]> { MEMBERS.iter().map(|m| (*m).to_string()).collect::<Vec<_>>().into() };
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

    /// The static forms run a fresh instance of the xmlport with that id.
    pub(super) fn xp_static_export_import(
        &mut self,
        id: al_ir::BuiltinId,
        args: &[Value],
        ret_used: bool,
    ) -> Result<Value, Signal> {
        let dir = if matches!(id, ids::XMLPORT_EXPORT_1 | ids::XMLPORT_EXPORT_2) {
            Dir::Export
        } else {
            Dir::Import
        };
        let Some(Value::Integer(n)) = args.first() else {
            return unsupported(format!("{id} with an id that is not an integer"));
        };
        let object = self.xp_static_object(id, *n)?;
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
        // (xmlports/TestXmlPortObject.al:XmlPort_Import_StaticWithRecord_GivenVariableUnchangedWithoutExplicitGet,
        // :XmlPort_Export_StaticWithRecord_RespectsFilters).
        if let Some(rec) = args.get(2) {
            self.xp_set_view(&inst, Some(rec))?;
        }
        self.xp_run(&inst, dir, ret_used, true)
    }

    /// The static `Xmlport.Run(Number, RequestWindow, Import, Record)`. Pinned: an id no xmlport declares raises the missing-object
    /// error (metadata/session-system/P94XpPathFlow.al:RunStatic_UnknownNumbers; errors/W5MissingObject.al:
    /// MissingReport_Page_XmlPort), and an import of an export-only xmlport raises its direction error (same file:
    /// RunStatic_ImportWithoutSource). What a run that passes those checks reads or writes is in no corpus test and is refused.
    pub(super) fn xp_static_run(&mut self, id: al_ir::BuiltinId, args: &[Value]) -> Result<Value, Signal> {
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
}
