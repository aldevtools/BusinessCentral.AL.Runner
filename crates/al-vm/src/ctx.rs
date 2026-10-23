use super::*;

impl al_builtins::format::FormatCtx for Session {
    fn format_gate(&self, gate: al_version::Gate) -> bool {
        self.has(gate)
    }
    /// The report's language while a report runs (set by a trigger, or by the variable a SaveAs ran: report/
    /// W3ReportObjectIdPreviewLanguage.al:893 `Ja`; report/lt-bd-format-region/BdRegionTests.al:1036 `mars` in a Run), else the
    /// session's.
    fn language(&self) -> i32 {
        self.report
            .curr_report_language
            .or(self.report.run_language)
            .unwrap_or(self.global_language)
    }
    fn region(&self) -> Option<String> {
        self.report.format_region.clone()
    }
    fn record_id_table(&self, table: u32) -> Option<(String, Option<String>)> {
        let t = self.engine.tables.get(table as usize)?.clone()?;
        let caption = self.caption_checked(&t).ok()?;
        Some((caption, self.engine.program.objects[table as usize].namespace.clone()))
    }
    /// `OnInitReport` of a report with the `FormatRegion` property: the corpus reports format their values only in
    /// `OnAfterGetRecord` (report/lt-y-report-props/YReports.al:32-48), so whether `Format` there already follows the
    /// region is not pinned (docs/probes-needed.md).
    ///
    /// Outside a report started by Run, `CurrReport.Language :=` another language changes what `Format` writes
    /// (report/W3ReportObjectIdPreviewLanguage.al:546-557 Language_Set1031_FormatAndColumns, a SaveAs: `Format changed=Ja`),
    /// which is not built.
    fn region_unpinned(&self) -> Option<String> {
        if let Some(r) = &self.report.region_in_init {
            return Some(format!(
                "in OnInitReport of a report with FormatRegion = '{r}' (whether it follows the region is not pinned)"
            ));
        }
        match self.report.curr_report_language {
            Some(n)
                if n != self.global_language
                    && self.report.report_preview_pinned.last() != Some(&crate::state::ReportStart::Run) =>
            {
                Some(format!(
                    "after CurrReport.Language := {n} in a report not started by Run (the formats in that language are not built)"
                ))
            }
            _ => None,
        }
    }
    /// The caption a value declares, else its name (types/TestEnumCaption.al).
    fn enum_name(&self, id: u32, value: i32) -> String {
        self.note_enum(id);
        let o = &self.engine.program.objects[id as usize];
        if let Some((_, c)) = o.enum_captions.iter().find(|(v, _)| *v == value) {
            return c.clone();
        }
        o.enum_values
            .iter()
            .find(|(v, _)| *v == value)
            .map(|(_, n)| n.clone())
            .unwrap_or_else(|| value.to_string())
    }
}

impl al_builtins::native::Ctx for Session {
    fn gate(&self, gate: al_version::Gate) -> bool {
        self.has(gate)
    }

    fn http_send(
        &mut self,
        request: al_builtins::native::HttpOutgoing,
    ) -> Result<al_builtins::value::TestHttpResponseState, Signal> {
        self.http_send_impl(request)
    }
    fn today_day(&self) -> Option<i32> {
        Some(self.today_day_number())
    }
    fn code_coverage_refresh(&mut self) -> Result<(), Signal> {
        self.coverage_refresh()
    }
    fn work_date_day(&self) -> Option<i32> {
        Session::work_date_day(self)
    }
    fn now_unix_millis(&self) -> Option<i64> {
        Some(self.now_millis())
    }
    fn format_composite(&mut self, v: &Value) -> Option<Result<String, Signal>> {
        self.format_composite_value(v)
    }
    fn copy_stream_dotnet(&mut self, out: &Value, src: &Value, count: Option<i64>) -> Option<Result<(), Signal>> {
        Some(self.copy_stream_with_dotnet(out, src, count))
    }
    fn format_dotnet(&mut self, v: &Value) -> Option<Result<String, Signal>> {
        self.format_dotnet_value(v)
    }
    fn auto_format_field_places(&mut self, record: &Value, field_no: usize) -> Result<String, Signal> {
        self.auto_format_field_places_of(record, field_no)
    }
    fn last_error(&self) -> AlError {
        self.last_error.clone()
    }
    fn set_last_error(&mut self, e: AlError) {
        self.last_error = e;
    }
    fn record(
        &mut self,
        method: al_builtins::record_op::RecordOp,
        rec: Option<&mut Value>,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        match rec {
            Some(Value::Record(rc)) => {
                let rc = rc.clone();
                self.record_op(method, &rc, args)
            }
            other => Err(Signal::Unsupported(format!(
                "record method {method} on {}",
                other.describe()
            ))),
        }
    }
    fn page(
        &mut self,
        method: al_builtins::page_op::PageOp,
        recv: Option<&mut Value>,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        self.page_op(method, recv, args)
    }

    fn query(&mut self, method: &str, recv: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
        self.query_op(method, recv, args)
    }

    fn ref_op(&mut self, id: &str, receiver: Option<&mut Value>, args: &mut [Value]) -> Result<Value, Signal> {
        Session::ref_op(self, id, receiver, args)
    }
    fn set_value_from_variant(&mut self, on: bool) {
        self.set_value_from_variant = on;
    }
    fn session_op(&mut self, id: al_ir::BuiltinId, args: &mut [Value]) -> Result<Value, Signal> {
        Session::session_op(self, id, args)
    }
    fn media_store(&mut self) -> Option<&mut al_builtins::media::MediaStore> {
        Some(&mut self.media)
    }
    fn collect_error(&mut self, info: &Value) -> bool {
        Session::collect_error(self, info)
    }
    fn fs_exists(&mut self, path: &str) -> Result<bool, Signal> {
        self.refuse_pending_file(path)?;
        Ok(self.host.files.exists(path))
    }
    fn fs_read(&mut self, path: &str) -> Result<Option<Vec<u8>>, Signal> {
        self.refuse_pending_file(path)?;
        Ok(self.host.files.read(path))
    }
    fn fs_write(&mut self, path: &str, bytes: &[u8]) -> Result<(), String> {
        // A write by AL over the path ends the mark: the file is what AL wrote.
        if !self.render_pending_files.is_empty() {
            self.render_pending_files.remove(&pending_key(path));
        }
        self.host.files.write(path, bytes)
    }
    fn fs_remove(&mut self, path: &str) -> Result<bool, Signal> {
        self.refuse_pending_file(path)?;
        Ok(self.host.files.remove(path))
    }
    fn fs_write_unproduced(&mut self, path: &str, mark: &str) -> Result<(), String> {
        self.host.files.write(path, &[])?;
        self.render_pending_files.insert(pending_key(path), mark.into());
        Ok(())
    }
    fn fs_serves_path(&self, path: &str) -> bool {
        self.host.files.serves_path(path)
    }
    fn fs_dir_exists(&self, path: &str) -> bool {
        self.host.files.dir_exists(path)
    }
    fn fs_temp_name(&mut self) -> String {
        self.temp_files += 1;
        format!("{}{}.tmp", al_builtins::file::TEMP_DIR, self.temp_files)
    }
    fn session_profile(&self) -> Option<(String, [u8; 16], String)> {
        self.personalization_of_session_user()
    }
    fn session_locale(&self) -> Option<(i32, i32, String)> {
        Some((
            self.global_language,
            self.host.identity.locale_id(),
            self.host.identity.time_zone(),
        ))
    }
    fn module_at_frame(&self, depth: usize) -> Option<al_builtins::value::ModuleInfoState> {
        let at = self.frames.len().checked_sub(1 + depth)?;
        self.module_of_app(self.engine.program.objects[self.frames[at]].app as usize)
    }
    fn app_resources(&self) -> Option<al_builtins::native::AppResources> {
        let obj = *self.frames.last()?;
        let index = self.engine.program.objects[obj].app as usize;
        let app = self.engine.program.apps.get(index)?;
        self.engine
            .resources_read
            .store(true, std::sync::atomic::Ordering::Relaxed);
        let resources = self
            .engine
            .resource_overrides
            .read()
            .ok()
            .and_then(|o| o.get(&index).map(|r| r.as_ref().clone()))
            .unwrap_or_else(|| app.resources.clone());
        // BC names the app `<name> by <publisher> <version>` (session/W5NavAppResources.al:GetResourceAsText_NameMatching).
        Some((format!("{} by {} {}", app.name, app.publisher, app.version), resources))
    }
    fn runner_call_stack(&self) -> String {
        self.runner_frames.clone()
    }
    fn module_callstack(&self, skip: usize) -> Vec<al_builtins::value::ModuleInfoState> {
        let mut seen = Vec::new();
        let mut out = Vec::new();
        for obj in self.frames.iter().rev().skip(skip) {
            let app = self.engine.program.objects[*obj].app as usize;
            if !seen.contains(&app) {
                seen.push(app);
                out.extend(self.module_of_app(app));
            }
        }
        out
    }
    fn application_module(&self) -> al_builtins::value::ModuleInfoState {
        let bc = self.engine.target.as_ref().map(|t| &t.bc);
        let (major, minor) = bc.map_or((29, 0), |b| (i32::from(b.major), i32::from(b.minor)));
        // Build and revision of the base application are not zero (session/W5ModuleInfo.al:
        // ModuleInfo_NullId_Properties); the artifact's own numbers when the run knows them, else 1.
        let (build, revision) = bc
            .and_then(|b| b.build.as_deref())
            .and_then(|s| {
                let n: Vec<i32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
                (n.len() >= 2).then(|| (n[n.len() - 2], n[n.len() - 1]))
            })
            .unwrap_or((1, 1));
        al_builtins::value::ModuleInfoState {
            // The module BC answers for the null app id: the base application W1 (session/
            // TestProbeSessionMiscContracts.al:ModuleInfo_NullAppId).
            id: [0; 16],
            name: "W1".into(),
            publisher: "Microsoft".into(),
            version: Some([major, minor, build, revision]),
            dependencies: Vec::new(),
            synthesized: true,
        }
    }
    fn module_by_id(&self, id: &[u8; 16]) -> al_builtins::native::ModuleLookup {
        use al_builtins::native::ModuleLookup;
        let apps = &self.engine.program.apps;
        let parse = |s: &str| ops::parse_guid(s);
        if let Some(i) = apps.iter().position(|a| parse(&a.id).as_ref() == Some(id)) {
            if let Some(m) = self.module_of_app(i) {
                return ModuleLookup::Found(m);
            }
        }
        if apps
            .iter()
            .any(|a| a.dependencies.iter().any(|d| parse(d).as_ref() == Some(id)))
        {
            return ModuleLookup::OutsideWorkspace;
        }
        ModuleLookup::NotInstalled
    }
    fn workspace_table(&self, number: i32) -> bool {
        self.engine
            .program
            .objects
            .iter()
            .any(|o| o.is_kind(al_ir::ObjKind::Table) && o.id.is_some_and(|id| i64::from(id) == i64::from(number)))
    }
    fn install_scope(&self) -> bool {
        self.install_scope
    }
    fn in_test_run(&self) -> bool {
        self.in_test_codeunit
    }
    fn isolation_from_hub(&self) -> bool {
        self.isolation_from_hub
    }
    fn enum_members(&self, id: u32) -> Option<Vec<(i32, String)>> {
        self.note_enum(id);
        Some(self.engine.program.objects[id as usize].enum_values.clone())
    }
    fn evaluate_record_id(
        &mut self,
        text: &str,
        number: i64,
    ) -> Option<Result<Option<al_builtins::value::LibraryValue>, Signal>> {
        Some(self.evaluate_record_id_text(text, number))
    }
    fn evaluate_field_target(&mut self, v: &Value) -> Option<Result<al_builtins::native::FieldTarget, Signal>> {
        let Value::Library(_, l) = v else { return None };
        let (rec, pos) = match &*l.borrow() {
            al_builtins::value::LibraryValue::FieldRef(r, p) | al_builtins::value::LibraryValue::KeyFieldRef(r, p) => {
                (r.clone(), *p)
            }
            _ => return None,
        };
        let t = match self.table(&rec) {
            Ok(t) => t,
            Err(e) => return Some(Err(e)),
        };
        Some(Ok(al_builtins::native::FieldTarget {
            ty: t.fields[pos].ty.clone(),
            option_caption: self.field_property(&t, pos, "OptionCaption").is_some(),
            rec,
            pos,
        }))
    }
    fn enum_captions(&self, id: u32) -> Vec<(i32, String)> {
        self.note_enum(id);
        self.engine.program.objects[id as usize].enum_captions.clone()
    }
    fn commit(&mut self) -> Result<(), Signal> {
        self.commit_builtin()
    }
    fn ret_used(&self) -> bool {
        self.ret_used
    }
    fn ui_message(&mut self, text: String) -> Result<(), Signal> {
        Session::ui_message(self, text)
    }
    fn ui_confirm(&mut self, question: String, default: bool) -> Result<bool, Signal> {
        Session::ui_confirm(self, question, default)
    }
    fn ui_str_menu(&mut self, options: String, default: i32, instruction: String) -> Result<i32, Signal> {
        Session::ui_str_menu(self, options, default, instruction)
    }
    fn ui_session_settings(&mut self, settings: Value) -> Result<Option<Value>, Signal> {
        Session::ui_session_settings(self, settings)
    }
    fn ui_send_notification(&mut self, notification: Value, id: [u8; 16]) -> Result<Option<Value>, Signal> {
        Session::ui_send_notification(self, notification, id)
    }
    fn ui_recall_notification(&mut self, notification: Value, id: [u8; 16]) -> Result<bool, Signal> {
        Session::ui_recall_notification(self, notification, id)
    }
    fn new_guid(&mut self) -> [u8; 16] {
        self.fresh_guid()
    }
}

/// A version 4 Guid (RFC 4122) that is a pure function of the host seed and a counter, so every
/// byte looks random and a run is still repeatable (ADR-006: no OS randomness in the core;
/// ADR-009: determinism is allowed where AL cannot tell the difference). The bytes are two
/// SplitMix64 outputs; the version nibble is 4 and the variant bits are `10`.
pub(crate) fn guid_for(seed: i32, n: u64) -> [u8; 16] {
    let mut g = raw_guid_for(seed, n);
    g[6] = 0x40 | (g[6] & 0x0F);
    g[8] = 0x80 | (g[8] & 0x3F);
    g
}

/// The bytes of [`guid_for`] before the version and variant bits are set: the form of the SystemId the
/// platform gives a new row, which is not RFC 4122 version 4 (types/TestGuidFormContracts.al:
/// SystemId_OfNewRows_IsNotVersion4).
pub(crate) fn raw_guid_for(seed: i32, n: u64) -> [u8; 16] {
    let mut state = ((seed as u32 as u64) << 32) ^ n.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut next = || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut g = [0u8; 16];
    g[..8].copy_from_slice(&next().to_be_bytes());
    g[8..].copy_from_slice(&next().to_be_bytes());
    g
}

impl Session {
    /// The next Guid of the session: `CreateGuid`, a row's SystemId, the ids of media and tasks.
    /// `CreateGuid` is a version 4 Guid (types/TestGuidFormContracts.al:CreateGuid_IsVersion4).
    pub(crate) fn fresh_guid(&mut self) -> [u8; 16] {
        self.next_guid += 1;
        guid_for(self.guid_seed, self.next_guid)
    }

    /// The SystemId of a new row: not a version 4 Guid.
    pub(crate) fn fresh_system_id(&mut self) -> [u8; 16] {
        self.next_guid += 1;
        raw_guid_for(self.guid_seed, self.next_guid)
    }
}

pub(crate) fn base_slot_missing(slot: usize) -> Signal {
    Signal::Unsupported(format!(
        "internal error: the base object's instance has no global slot {slot}"
    ))
}

/// How a path is compared with the pending ones: case folded and with `/` for `\`, so a different spelling of the
/// same file is still the file (a superset of what the file system would call equal, which only refuses more).
pub(crate) fn pending_key(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

impl Session {
    /// Refuses a look at a file whose rendered output was never produced (ADR-040).
    pub(crate) fn refuse_pending_file(&self, path: &str) -> Result<(), Signal> {
        if self.render_pending_files.is_empty() {
            return Ok(());
        }
        match self.render_pending_files.get(&pending_key(path)) {
            Some(report) => Err(al_builtins::stream::render_pending_error(report)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod conformance {
    /// ADR-001: the compiled program is shared across threads; sessions are not.
    #[test]
    fn engine_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<super::Engine>();
    }
}
