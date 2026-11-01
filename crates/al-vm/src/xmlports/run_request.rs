//! `Xmlport.Run` of an existing port: the static form after its id and direction checks, and the instance `Run()`
//! (xmlport/lt-y40-xmlport-request-page/Y40XmlPortRequestPage.al, corpus master ac3499d6).

use super::*;

impl Session {
    /// The static `Xmlport.Run(Number, RequestWindow, Import, Record)` of a port that passed the id and direction checks.
    pub(super) fn xp_run_static_existing(
        &mut self,
        args: &[Value],
        import: bool,
        inst: &Inst,
        schema: &Schema,
    ) -> Result<Value, Signal> {
        // Pinned for an export port with a request page with content (xmlport/lt-y40-xmlport-request-page/
        // Y40XmlPortRequestPage.al:383-423, corpus master ac3499d6): with the request window the page opens after
        // OnInitXmlPort and no handler can take it; without it the port runs (`init;pre only=0;row 1;...;post;`), and
        // what it writes goes to a file on the service tier no test reads. The rest is not pinned.
        let window = match args.get(1) {
            Some(Value::Boolean(b)) => Some(*b),
            _ => None,
        };
        match window {
            Some(true) if !import && args.get(3).is_none() && schema.xp().request_page => {
                self.xp_run_request_window(inst, schema)
            }
            Some(false) if !import && args.get(3).is_none() && schema.xp().request_page => {
                let sink = Value::Library(
                    al_builtins::library_kind::LibraryKind::OutStream,
                    Rc::new(std::cell::RefCell::new(LibraryValue::OutStream(
                        al_builtins::stream::new_blob(),
                        al_builtins::stream::Encoding::default(),
                    ))),
                );
                Self::xp_run_use_request_page_refused(schema)?;
                inst.borrow_mut().globals[schema.xp().destination_slot] = sink;
                self.xp_run(inst, Dir::Export, false, true).map(|_| Value::Null)
            }
            _ => unsupported(
                "Xmlport.Run of an existing xmlport: where a run without a stream writes or reads (a file on the service tier a test cannot read) and what the request window does are not pinned",
            ),
        }
    }

    /// The instance `Run()` of an export port with a request page with content opens the page after OnInitXmlPort and no
    /// handler can take it (Y40XmlPortRequestPage.al:398, Y40_Run_Instance_DefaultRequestPage_NoHandler). Without a
    /// request page it is not pinned.
    pub(super) fn xp_run_instance(&mut self, receiver: Option<Value>) -> Result<Value, Signal> {
        let inst = self.xp_instance(receiver)?;
        let schema = self.xp_schema(&inst)?;
        if schema.xp().direction == ir::XmlDirection::Export && schema.xp().request_page {
            return self.xp_run_request_window(&inst, &schema);
        }
        unsupported(
            "Xmlport.Run on an xmlport variable without a request page, or not an export port: where it writes or reads and what the request window does are not pinned",
        )
    }

    /// A run with the request window of a port with a request page: OnInitXmlPort runs (once per instance, as for an export),
    /// then the page opens and, as a RequestPageHandler cannot be typed on an xmlport (must-not-compile case 2513), the test
    /// ends in `Unhandled UI: RequestPage <id>` (MissingUIHandler), inside a TryFunction too (Y40XmlPortRequestPage.al:383-423,
    /// log `init;`). A Quit in OnInitXmlPort there is not pinned.
    fn xp_run_request_window(&mut self, inst: &Inst, schema: &Schema) -> Result<Value, Signal> {
        Self::xp_run_use_request_page_refused(schema)?;
        if let Some(why) = schema.xp().run_unsupported.first() {
            return unsupported(format!("XmlPort \"{}\": {why}", schema.name()));
        }
        let routines = &schema.engine.program.objects[schema.object].routines;
        let init = routines
            .iter()
            .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnInitXmlPort"));
        Self::xp_clear_flow(inst);
        if let Some(r) = init {
            if !inst.borrow().xp_text.iter().any(|(k, _)| *k == 11) {
                inst.borrow_mut().xp_text.push((11, String::new()));
                self.xp_run_object_trigger(inst, r)?;
                if Self::xp_flow(inst).is_some() {
                    Self::xp_clear_flow(inst);
                    return unsupported(
                        "Break, Skip or Quit in OnInitXmlPort of a run with the request window (not pinned)",
                    );
                }
            }
        }
        match self.bind_page_handler(crate::ui::UiKind::RequestPage, schema.object as u32) {
            Ok(_) => unsupported("a request page handler bound to an xmlport's request page (not pinned)"),
            Err(e) => Err(e),
        }
    }

    /// What the property UseRequestPage does to Xmlport.Run is not pinned (Y40XmlPortRequestPage.al runs a port without it).
    fn xp_run_use_request_page_refused(schema: &Schema) -> Result<(), Signal> {
        let o = &schema.engine.program.objects[schema.object];
        if o.properties
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("UseRequestPage"))
        {
            return unsupported(format!(
                "Xmlport.Run of \"{}\", which declares UseRequestPage (not pinned)",
                schema.name()
            ));
        }
        Ok(())
    }
}
