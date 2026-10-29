//! Page background tasks under the test client: `CurrPage.EnqueueBackgroundTask`,
//! `TestPage.RunPageBackgroundTask`, `Page.GetBackgroundParameters` and
//! `Page.SetBackgroundTaskResult`. BC's test framework runs the worker synchronously, so the task
//! has completed, and its trigger has run, when the TestPage call that caused it returns (corpus
//! handlers/TestPageBgTask_Tests.al, header). Every rule cites the corpus test that pins it.

use super::*;
use al_builtins::describe::Describe;
use al_builtins::page_op::PageOp;
use al_builtins::value::Value as V;

/// A worker codeunit that is running for a page background task.
pub(crate) struct BgWorker {
    params: V,
    result: Option<V>,
}

/// A new dictionary with the entries of `v` (the worker gets its own copy of the parameters).
fn copy_dictionary(v: &V) -> V {
    match v {
        V::Dictionary(d) => V::Dictionary(Rc::new(RefCell::new(d.borrow().clone()))),
        _ => V::Dictionary(Default::default()),
    }
}

impl Session {
    /// `CurrPage.EnqueueBackgroundTask(var TaskId; CodeunitId; [var Parameters]; [Timeout]; [ErrorLevel])`: the task
    /// waits in the queue until the client call that is running ends. A task that succeeds runs the same under every
    /// ErrorLevel (bgtasks/lt-bj-no-gui/BjNoGuiDialogs.al:86 with Error, EnqueueBackgroundTask_WithHandlers); what the
    /// level changes for a failing worker is in `bg_drain`.
    pub(super) fn bg_enqueue(&mut self, st: &St, args: &mut [Value]) -> Result<Value, Signal> {
        let level = match args.get(4) {
            None => None,
            Some(Value::Option(o, members)) if al_builtins::sysenum::is_page_background_task_error_level(members) => {
                Some(*o)
            }
            Some(other) => {
                return unsupported(format!(
                    "EnqueueBackgroundTask with the ErrorLevel {}",
                    other.describe()
                ));
            }
        };
        let Some(Value::Integer(codeunit)) = args.get(1).cloned() else {
            return unsupported("EnqueueBackgroundTask without a codeunit id");
        };
        // The Timeout (4th argument, or 3rd when the parameters are left out) does not matter to a
        // synchronous run.
        let params = match args.get(2) {
            Some(d @ V::Dictionary(_)) => copy_dictionary(d),
            _ => V::Dictionary(Default::default()),
        };
        let id = {
            let mut s = st.borrow_mut();
            let id = s.bg_next_id;
            s.bg_next_id += 1;
            s.bg_tasks.push_back((id, codeunit, params, level));
            id
        };
        args[0] = Value::Integer(id);
        // Under a TestPage the task runs at once: its completion trigger has run when this call returns, so a
        // cancel right after it cannot stop it (corpus handlers/TestPageCancelBackgroundTaskContracts.al:
        // Cancel_RightAfterEnqueue_ReturnsTrue_ButTheTaskHasCompleted: `completed` is logged before the
        // cancel's answer; bgtasks/W2BackgroundTasks.al:CancelPreviousTask_OnRowMove: `done` before `enqueued`).
        // The task runs after the call has stored the id in its `var TaskId` argument, so the completion trigger
        // sees the page variable already set: its TaskId equals the variable the enqueue filled
        // (bgtasks/W2BackgroundTasks.al:QP_CompletedTaskId_AgainstPageVariable_OpenView, page 87144 at :283-332).
        self.pages.bg_drain_after_call = Some(st.clone());
        Ok(Value::Null)
    }

    /// Runs the tasks an `EnqueueBackgroundTask` call queued, after the call wrote its arguments back.
    pub(crate) fn bg_drain_after_call(&mut self) -> Result<(), Signal> {
        let Some(st) = self.pages.bg_drain_after_call.take() else {
            return Ok(());
        };
        self.bg_drain(&st)
    }

    /// `CurrPage.CancelBackgroundTask(TaskId)`: true for the id of a task this page enqueued, also after it
    /// completed and again on a second call; false for an id nobody enqueued (9999, or 0 before any
    /// enqueue); it raises nothing (corpus handlers/TestPageCancelBackgroundTaskContracts.al).
    pub(super) fn bg_cancel(&mut self, st: &St, args: &mut [Value]) -> Result<Value, Signal> {
        let Some(Value::Integer(id)) = args.first().cloned() else {
            return unsupported("CancelBackgroundTask without a task id");
        };
        let enqueued = id >= 1 && id < st.borrow().bg_next_id;
        Ok(Value::Boolean(enqueued))
    }

    /// Runs the worker codeunit `codeunit` with `params`: the dictionary it set as its result (empty when it
    /// set none), or the error it raised. Its session may not write to a table of the database (corpus
    /// handlers/TestPageBgTask_Tests.al:EnqueueBackgroundTask_WorkerInsert_RefusedByReadOnlySession and
    /// the Modify and Rename arms). It reads the caller's rows, the uncommitted ones too, and is
    /// outside the caller's write transaction (handlers/TestPageBgTaskTx_Tests.al): see
    /// `in_write_transaction` and `codeunit_run`. A direct `Commit` and an unguarded `Codeunit.Run`
    /// are not pinned and end the run as unsupported.
    fn bg_run_worker(&mut self, codeunit: i32, params: V) -> Result<Result<V, AlError>, Signal> {
        let engine = self.engine();
        let Some(object) = engine
            .program
            .objects
            .heads()
            .iter()
            .position(|o| o.is_kind(al_ir::ObjKind::Codeunit) && o.id == Some(codeunit as u32))
        else {
            // A codeunit of a dependency app that nothing reached is not compiled (ADR-013); BC has it.
            if engine
                .program
                .objects
                .heads()
                .iter()
                .any(|o| o.is_shell_of(al_ir::ObjKind::Codeunit) && o.id == Some(codeunit as u32))
            {
                return unsupported(format!(
                    "background task of codeunit {codeunit}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)"
                ));
            }
            return unsupported(format!(
                "background task of codeunit {codeunit}, which no object declares"
            ));
        };
        let Some(on_run) = engine.program.objects[object]
            .routines
            .iter()
            .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"))
        else {
            return Ok(Ok(V::Dictionary(Default::default())));
        };
        if self.pages.bg_worker.is_some() {
            return unsupported("a background task started from a background task");
        }
        self.pages.bg_worker = Some(BgWorker { params, result: None });
        let inst = self.instance(object);
        let ran = self.call(object, on_run, inst, Vec::new());
        let worker = self.pages.bg_worker.take().expect("the worker is still set");
        match ran {
            Ok(_) => Ok(Ok(worker.result.unwrap_or_else(|| V::Dictionary(Default::default())))),
            Err(Signal::Error(e)) => Ok(Err(*e)),
            Err(other) => Err(other),
        }
    }

    /// `TestPage.RunPageBackgroundTask(CodeunitId; [var Parameters]; [RunCompletionTriggers])`: runs the
    /// worker now and returns its result dictionary; the worker's error reaches the caller as it is
    /// (corpus handlers/TestPageBgTask_Tests.al:RunPageBackgroundTask_ReturnsWorkerResult;
    /// handlers/TestPageBgTaskTx_Tests.al:RunPageBackgroundTask_FailingWorker_LeavesCallersWriteTransactionOpen).
    pub(super) fn bg_run_page_task(&mut self, st: &St, args: &mut [Value]) -> Result<Value, Signal> {
        let Some(Value::Integer(codeunit)) = args.first().cloned() else {
            return unsupported("RunPageBackgroundTask without a codeunit id");
        };
        let params = match args.get(1) {
            Some(d @ V::Dictionary(_)) => copy_dictionary(d),
            _ => V::Dictionary(Default::default()),
        };
        let completion = matches!(args.get(2), Some(V::Boolean(true)));
        let results = match self.bg_run_worker(codeunit, params)? {
            Ok(r) => r,
            // With the completion triggers, a failing worker runs OnPageBackgroundTaskError, and when that marks the error as
            // handled the call returns without an error (bgtasks/W2BackgroundTasks.al:508-537
            // RunPageBackgroundTask_FailingWorker_WithCompletionTriggers, `with_completion|ok` = Yes and the error trigger's log).
            // An error the trigger leaves unhandled, and what the call returns, are not pinned (docs/probes-needed.md): the result
            // is an empty dictionary.
            Err(e) if completion => {
                let id = {
                    let mut s = st.borrow_mut();
                    let id = s.bg_next_id;
                    s.bg_next_id += 1;
                    id
                };
                if !self.bg_error(st, id, &e)? {
                    return unsupported(
                        "RunPageBackgroundTask with completion triggers whose failing worker's error OnPageBackgroundTaskError leaves unhandled (not pinned)",
                    );
                }
                return Ok(V::Dictionary(Default::default()));
            }
            Err(e) => return Err(Signal::Error(Box::new(e))),
        };
        if completion {
            // The task id the completion trigger receives is not pinned: a fresh one (docs/probes-needed.md).
            let id = {
                let mut s = st.borrow_mut();
                let id = s.bg_next_id;
                s.bg_next_id += 1;
                id
            };
            self.bg_completed(st, id, &results)?;
        }
        Ok(copy_dictionary(&results))
    }

    /// `Page.GetBackgroundParameters()` and `Page.SetBackgroundTaskResult(Results)`, called by a worker.
    pub(super) fn bg_static(&mut self, method: PageOp, args: &mut [Value]) -> Result<Value, Signal> {
        let Some(worker) = self.pages.bg_worker.as_mut() else {
            return unsupported(format!("{method} outside a page background task (not pinned)"));
        };
        if method == PageOp::PageGetBackgroundParameters {
            return Ok(copy_dictionary(&worker.params));
        }
        match args.first() {
            Some(d @ V::Dictionary(_)) => {
                worker.result = Some(copy_dictionary(d));
                Ok(Value::Null)
            }
            other => unsupported(format!("Page.SetBackgroundTaskResult with {}", other.describe())),
        }
    }

    /// Runs the page's `OnPageBackgroundTaskCompleted(TaskId, Results)`, if it has one, then the same trigger of each page
    /// extension that declares it, for every task the page or an extension enqueued (pages/lt-uj-outside-leads/UjPageExtBgTask.al
    /// UJ_PageExt_BgTask_Completed, corpus branch al-runtime-rs-staging-uj 1dc037a8).
    fn bg_completed(&mut self, st: &St, id: i32, results: &V) -> Result<(), Signal> {
        if self.bg_ext_declares(st, "onpagebackgroundtaskcompleted") {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageExtBackgroundTaskTriggers);
        }
        if self.declares(st, "onpagebackgroundtaskcompleted")
            || self.bg_ext_declares(st, "onpagebackgroundtaskcompleted")
        {
            self.page_trigger(
                st,
                "onpagebackgroundtaskcompleted",
                vec![Value::Integer(id), copy_dictionary(results)],
            )?;
        }
        Ok(())
    }

    /// Runs the tasks the page enqueued, oldest first, each to its end: the completion trigger gets the
    /// worker's result, the error trigger its error. An error nobody marks as handled in
    /// `OnPageBackgroundTaskError` tears the page down, and what the caller sees is the closed page,
    /// not the worker's text (corpus handlers/TestPageBgTask_Tests.al:EnqueueBackgroundTask_HandledErrorDoesNotPropagate,
    /// EnqueueBackgroundTask_UnhandledErrorPropagates). Whether a task still waiting is cancelled when
    /// the page moves to another row is not pinned (docs/probes-needed.md): all of them run.
    pub(super) fn bg_drain(&mut self, st: &St) -> Result<(), Signal> {
        loop {
            let next = st.borrow_mut().bg_tasks.pop_front();
            let Some((id, codeunit, params, level)) = next else {
                return Ok(());
            };
            if !st.borrow().open {
                return unsupported("a background task queued on a page that closed before it ran (not pinned)");
            }
            match self.bg_run_worker(codeunit, params)? {
                Ok(results) => {
                    let r = self.bg_completed(st, id, &results);
                    self.torn_down_on_error(st, r)?;
                }
                // A failing worker under ErrorLevel Ignore: the error trigger runs, and the page stays open even though the
                // trigger did not mark the error as handled (bgtasks/W2BackgroundTasks.al:455 ErrorLevel_Ignore, `open|ok` = Yes,
                // `log` = `error(id=n,row=L1,handled_in=false);`).
                Err(e) if level == Some(2) => {
                    self.bg_error(st, id, &e)?;
                }
                // Under Error or Warning given as an argument, what a failing worker does to the page is not pinned
                // (docs/probes-needed.md).
                Err(_) if level.is_some() => {
                    return unsupported(
                        "a failing worker of a background task enqueued with ErrorLevel Error or Warning (not pinned)",
                    );
                }
                Err(e) => {
                    let handled = self.bg_error(st, id, &e)?;
                    // A task a card enqueued while OpenView reads its first row (from OnAfterGetCurrRecord) whose error nobody handles: OpenView
                    // raises the worker's own text, its code with `:TargetInvocation` added (pages/lt-uj-outside-leads/
                    // UjPageExtBgTask.al UJ_PageExt_BgTask_Error_Unhandled_Base and _Ext, corpus branch al-runtime-rs-staging-uj
                    // 1dc037a8: `worker failed for base-fail`, `Dialog:TargetInvocation`).
                    let opening_view_card = {
                        let s = st.borrow();
                        s.open_read && s.mode == PageMode::View
                    } && self.is_card(st);
                    if !handled && opening_view_card {
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::PageExtBackgroundTaskTriggers,
                        );
                        st.borrow_mut().bg_tasks.clear();
                        let mut e = e;
                        e.code = format!("{}:TargetInvocation", e.code);
                        return Err(Signal::Error(Box::new(e)));
                    }
                    if !handled {
                        st.borrow_mut().bg_tasks.clear();
                        self.mark_closed(st);
                        return Err(error(NOT_OPEN));
                    }
                }
            }
        }
    }

    /// Whether a page extension of the page declares trigger `name` (lower case).
    fn bg_ext_declares(&self, st: &St, name: &str) -> bool {
        let engine = self.engine();
        page_of(&engine, st).ext_triggers.iter().any(|(n, _, _)| n == name)
    }

    /// Runs `OnPageBackgroundTaskError(TaskId, ErrorCode, ErrorText, ErrorCallStack, var IsHandled)` of the page, then of each
    /// page extension that declares it, each one getting the IsHandled the one before left (pages/lt-uj-outside-leads/
    /// UjPageExtBgTask.al UJ_PageExt_BgTask_Error_Handled: `ext error ... handled=Yes` after the page marked it handled), and
    /// answers what the last one left in `IsHandled`.
    fn bg_error(&mut self, st: &St, id: i32, e: &AlError) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let own = page
            .triggers
            .iter()
            .find(|(n, _)| n == "onpagebackgroundtaskerror")
            .map(|(_, r)| (object, *r, None));
        let exts = page
            .ext_triggers
            .iter()
            .filter(|(n, _, _)| n == "onpagebackgroundtaskerror")
            .map(|(_, ext, r)| (*ext, *r, Some(*ext)));
        let routines: Vec<(usize, usize, Option<usize>)> = own.into_iter().chain(exts).collect();
        if routines.is_empty() {
            return Ok(false);
        }
        if routines.iter().any(|(_, _, ext)| ext.is_some()) {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PageExtBackgroundTaskTriggers);
        }
        al_builtins::native::refuse_unpinned(e, "OnPageBackgroundTaskError")?;
        let mut handled = false;
        for (obj, routine, ext) in routines {
            let inst = match ext {
                None => st.borrow().instance.clone().expect("open page"),
                Some(_) => self.owner_instance(st, ext),
            };
            let args = vec![
                Value::Integer(id),
                Value::Text(e.code.clone().into()),
                Value::Text(e.message.clone().into()),
                Value::Text(e.call_stack.clone().into()),
                Value::Boolean(handled),
            ];
            let (_, finals) = self.call(obj, routine, inst, args)?;
            self.end_page_call(st);
            handled = matches!(finals.get(4), Some(Value::Boolean(true)));
        }
        Ok(handled)
    }

    /// The error a write by a background task worker raises: the session is read-only. Only the
    /// pinned writes have a text (corpus handlers/TestPageBgTask_Tests.al, the three
    /// `..._RefusedByReadOnlySession` arms; bgtasks/W2BackgroundTasks.al:487-496 Worker_Delete).
    pub(crate) fn bg_refuse_write(&self, table: usize, method: &str) -> Result<(), Signal> {
        if self.pages.bg_worker.is_none() {
            return Ok(());
        }
        if !matches!(method, "Insert" | "Modify" | "Rename" | "Delete") {
            return unsupported(format!(
                "{method} in a page background task (the refusal text is not pinned)"
            ));
        }
        let o = &self.engine.program.objects[table];
        Err(self.bg_read_only_error(o.id.unwrap_or(0), &o.name, o.app, method))
    }

    /// `Sorry, the current permissions prevented the action. (TableData <id> <name> <method>: <app>)`, code
    /// `DB:ReadOnlySession` for Insert (session/lt-cn-session-leads/CnIsolatedStorageLeads.al:136-150), Modify
    /// (pages/lt-ao-testpage/AoTests.al:118-123) and Delete (bgtasks/W2BackgroundTasks.al:487-496). The code of a
    /// refused Rename is not pinned: it carries the unpinned mark.
    pub(crate) fn bg_read_only_error(&self, id: u32, name: &str, app: u32, method: &str) -> Signal {
        let app = self
            .engine
            .program
            .apps
            .get(app as usize)
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let code = match method {
            "Insert" | "Modify" | "Delete" => "DB:ReadOnlySession",
            _ => al_builtins::native::UNPINNED_ERROR,
        };
        crate::transactions::al_error_code(
            &format!("Sorry, the current permissions prevented the action. (TableData {id} {name} {method}: {app})"),
            code,
        )
    }

    /// The calls inside a worker whose transaction effect is not pinned: a direct `Commit` and an
    /// unguarded `Codeunit.Run` (the corpus pins the guarded run only).
    pub(crate) fn bg_refuse_transaction_call(&self, what: &str) -> Result<(), Signal> {
        if self.pages.bg_worker.is_some() {
            return unsupported(format!(
                "{what} in a page background task (not pinned: handlers/TestPageBgTaskTx_Tests.al pins only the guarded Codeunit.Run)"
            ));
        }
        Ok(())
    }
}
