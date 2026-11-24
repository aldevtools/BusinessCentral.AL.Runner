//! Global `[WithEvents]` DotNet variables whose uses run (al-runner issue 7832): a System.Timers.Timer, and which of its uses,
//! pinned by tests/al-language-onprem-dotnet/dotnet/ac/N4WithEventsNoRaise.al; and the AzureML variables of a codeunit with
//! no trigger for them, in the uses Base Application's codeunit 2001 makes, on assumption
//! `with-events-no-trigger-azure-ml-shape`. Every other WithEvents variable keeps the refusal of `Lowerer::var_attr_refusal`.
//!
//! Global `[WithEvents]` DotNet variables whose uses run (al-runner issue 7832): a codeunit's
//! variable without a trigger, and a Timer with only an Elapsed trigger (codeunit, page, table,
//! report) with the uses the probe made (constructor, Interval, Enabled, IsNull, Clear; ClearAll
//! refused). Entry points: `marker`, `timer_elapsed_trigger` (the report trigger `reports::layout`
//! accepts), `Lowerer::with_events_timer_use`, `Lowerer::with_events_timer_member`,
//! `Lowerer::with_events_clear_all`.

use super::*;
use al_syntax::ast::NodeText;

/// The name a global `[WithEvents]` variable's attribute gets when its uses run, or `None` (the refusal of
/// `Lowerer::var_attr_refusal` stays): [`WITH_EVENTS_TIMER_ELAPSED`] for a `System.Timers.Timer` variable of a codeunit, a
/// page, a table or a report whose one trigger is `name::Elapsed` (N4WithEventsNoRaise.al:291 codeunit, :372 page, :387 table,
/// :405 report), or of a codeunit that declares no trigger for it (:320-345). The timer never starts, so no trigger runs; the
/// uses that may run are those the probe made (`Lowerer::with_events_timer_use`, `Lowerer::with_events_timer_member`).
/// [`WITH_EVENTS_NO_TRIGGER`] for a variable of one of [`AZURE_ML_TYPES`] of a codeunit that declares no trigger for it (the
/// variables of Base Application's codeunit 2001 "Azure ML Connector", System/AI/AzureMLConnector.Codeunit.al:17-22, which
/// Microsoft's test 139540 reaches). A variable of any other DotNet type with no trigger stays refused.
pub(super) fn marker(
    ws: &Workspace,
    file: usize,
    kind: ObjKind,
    ty: &Ty,
    obj: &ast::Object,
    name: &str,
) -> Option<&'static str> {
    let Ty::DotNet(alias) = ty else { return None };
    let prefix = format!("{}::", name.to_lowercase());
    let events: Vec<String> = obj
        .triggers()
        .filter_map(|t| t.name().to_lowercase().strip_prefix(&prefix).map(str::to_string))
        .collect();
    let type_name = ws
        .resolve_dotnet(file, alias)
        .map(|i| ws.dotnet[i].decl.type_name.as_str());
    if kind == ObjKind::Codeunit && events.is_empty() && type_name.is_some_and(|t| AZURE_ML_TYPES.contains(&t)) {
        return Some(WITH_EVENTS_NO_TRIGGER);
    }
    let timer = type_name == Some("System.Timers.Timer");
    let measured_kind = matches!(
        kind,
        ObjKind::Codeunit | ObjKind::Page | ObjKind::Table | ObjKind::Report
    );
    let measured_triggers = events == ["elapsed"] || (events.is_empty() && kind == ObjKind::Codeunit);
    (timer && measured_kind && measured_triggers).then_some(WITH_EVENTS_TIMER_ELAPSED)
}

/// The DotNet types of the WithEvents variables of Base Application's codeunit 2001 "Azure ML Connector"
/// (System/AI/AzureMLConnector.Codeunit.al:17-22; declared in the platform's `Microsoft.Dynamics.Nav.AzureMLWrapper` assembly).
const AZURE_ML_TYPES: &[&str] = &[
    "Microsoft.Dynamics.Nav.AzureMLWrapper.AzureMLRequest",
    "Microsoft.Dynamics.Nav.AzureMLWrapper.AzureMLInputBuilder",
    "Microsoft.Dynamics.Nav.AzureMLWrapper.AzureMLParametersBuilder",
];

/// The name of a global of object `oi` that [`marker`] accepts as `wanted` ([`WITH_EVENTS_TIMER_ELAPSED`] or
/// [`WITH_EVENTS_NO_TRIGGER`]), if it has one.
fn marked_global(ws: &Workspace, oi: usize, wanted: &str) -> Option<String> {
    let obj = ws.object_ast(oi);
    let (kind, file) = (ws.objects[oi].kind, ws.objects[oi].file);
    obj.var_decls()
        .filter(|v| super::shared::var_attribute_names(v).iter().any(|a| a == "withevents"))
        .find_map(|v| {
            let ty = v.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
            v.names()
                .into_iter()
                .find(|n| marker(ws, file, kind, &ty, &obj, n) == Some(wanted))
        })
}

/// The name of a global of object `oi` that [`marker`] accepts as a [`WITH_EVENTS_TIMER_ELAPSED`] Timer, if it has one.
fn timer_global(ws: &Workspace, oi: usize) -> Option<String> {
    marked_global(ws, oi, WITH_EVENTS_TIMER_ELAPSED)
}

/// The lowercase name of the Elapsed trigger of report `oi`'s WithEvents Timer ([`WITH_EVENTS_TIMER_ELAPSED`]): the report runs
/// with that trigger, which never runs (N4WithEventsNoRaise.al:405).
pub(super) fn timer_elapsed_trigger(ws: &Workspace, oi: usize) -> Option<String> {
    (ws.objects[oi].kind == ObjKind::Report)
        .then(|| timer_global(ws, oi))
        .flatten()
        .map(|n| format!("{}::elapsed", n.to_lowercase()))
}

/// The name a `[WithEvents]` attribute gets for a Timer variable [`marker`] accepts.
pub(crate) const WITH_EVENTS_TIMER_ELAPSED: &str = "withevents-timer-elapsed";

/// The name a `[WithEvents]` attribute gets for an AzureML variable [`marker`] accepts with no trigger.
pub(crate) const WITH_EVENTS_NO_TRIGGER: &str = "withevents-no-trigger";

/// The builtin `Clear` of a [`WITH_EVENTS_TIMER_ELAPSED`] variable lowers to (al-builtins `native/values.rs`).
const CLEAR_WITH_EVENTS_TIMER: &str = "System.ClearWithEventsTimer#1";

impl Lowerer<'_> {
    /// A use of the WithEvents Timer `name` ([`WITH_EVENTS_TIMER_ELAPSED`]) at `n`: the receiver of a member
    /// ([`Self::with_events_timer_member`] checks which), the target of `name := name.<constructor>()`, or the one argument of
    /// `IsNull` or `Clear` (N4WithEventsNoRaise.al:184-223, :62-68, :95-103, :135-141). Any other use is refused: the object could
    /// reach code that starts it.
    pub(crate) fn with_events_timer_use(&mut self, n: &SyntaxNode, name: &str) -> L<()> {
        let parent = n.parent();
        let same_var =
            |e: &SyntaxNode| e.kind() == SyntaxKind::NameRef && e.text_string().trim().eq_ignore_ascii_case(name);
        let allowed = match parent.as_ref().map(|p| (p.kind(), p)) {
            Some((SyntaxKind::MemberExpr, p)) => p.children().next().as_ref() == Some(n),
            Some((SyntaxKind::AssignStmt, p)) => {
                let kids: Vec<SyntaxNode> = p.children().collect();
                let plain = p.children_with_tokens().any(|t| t.kind() == SyntaxKind::Assign);
                plain
                    && kids.len() == 2
                    && &kids[0] == n
                    && kids[1].kind() == SyntaxKind::CallExpr
                    && kids[1]
                        .children()
                        .next()
                        .filter(|c| c.kind() == SyntaxKind::MemberExpr)
                        .is_some_and(|m| {
                            m.children().next().is_some_and(|r| same_var(&r)) && self.no_trigger_ctor_call(n, &m)
                        })
            }
            Some((SyntaxKind::ArgList, p)) => {
                p.children().count() == 1
                    && p.parent().is_some_and(|call| {
                        call.kind() == SyntaxKind::CallExpr
                            && matches!(self.res(&call), Some(Res::Builtin(id)) if id == "System.IsNull#1" || id == "System.Clear#1")
                    })
            }
            _ => false,
        };
        if allowed {
            return Ok(());
        }
        self.timer_refusal(name)
    }

    /// A member of the WithEvents Timer `name` ([`WITH_EVENTS_TIMER_ELAPSED`]): the constructor without arguments, a read or an
    /// assignment of `Interval` and a read of `Enabled` (N4WithEventsNoRaise.al:184-223). Any other member is refused: `Start`,
    /// `Enabled := true` and the rest could start the timer, and the runtime raises no event.
    pub(crate) fn with_events_timer_member(&mut self, name: &str, op: &ir::DotNetOp, ctor: bool, args: usize) -> L<()> {
        let measured = match op {
            ir::DotNetOp::Call(_) => ctor && args == 0,
            ir::DotNetOp::Get(m) => m.eq_ignore_ascii_case("Interval") || m.eq_ignore_ascii_case("Enabled"),
            ir::DotNetOp::Set(m) => m.eq_ignore_ascii_case("Interval"),
            _ => false,
        };
        if measured {
            return Ok(());
        }
        self.timer_refusal(name)
    }

    /// `Clear(name)` of the WithEvents Timer `name`: the variable becomes null (N4WithEventsNoRaise.al:209-212, :312
    /// `null_cleared` Yes). The object it held is disposed then (a Disposed trigger runs, :347), which nothing can see here: the
    /// variable has no Disposed trigger and the object never left it ([`Self::with_events_timer_use`]).
    pub(crate) fn with_events_timer_clear_id(&self, id: &str, args: &[SyntaxNode]) -> Option<&'static str> {
        match args {
            [a] if id == "System.Clear#1" && self.with_events_timer(a).is_some() => Some(CLEAR_WITH_EVENTS_TIMER),
            _ => None,
        }
    }

    /// `ClearAll` (`id`) in a routine of an object that holds a WithEvents Timer ([`WITH_EVENTS_TIMER_ELAPSED`]), or of an
    /// extension of one (in a page extension ClearAll resets the base page's globals too, CyClearAllExtensions.al:166-193): the
    /// probe measures `Clear` of the Timer only (N4WithEventsNoRaise.al:209-212), so ClearAll is refused. So is ClearAll in a
    /// codeunit with a [`WITH_EVENTS_NO_TRIGGER`] variable: codeunit 2001 makes no such call.
    pub(crate) fn with_events_clear_all(&mut self, id: &str) -> L<()> {
        if id != "System.ClearAll#1" {
            return Ok(());
        }
        let holder = [Some(self.oi), self.ws.extension_target(self.oi)]
            .into_iter()
            .flatten()
            .find_map(|o| timer_global(self.ws, o));
        if let Some(name) = holder {
            return self.timer_refusal(&name);
        }
        match marked_global(self.ws, self.oi, WITH_EVENTS_NO_TRIGGER) {
            Some(name) => self.no_trigger_refusal(&name),
            None => Ok(()),
        }
    }

    /// A use of the AzureML variable `name` ([`WITH_EVENTS_NO_TRIGGER`]) at `n`, in the forms codeunit 2001 makes
    /// (AzureMLConnector.Codeunit.al:42-61): the receiver of a member ([`Self::with_events_no_trigger_member`] checks which),
    /// the target of `name := name.<constructor>(...)`, the one argument of `IsNull`, or an argument of a member call whose
    /// receiver is such a variable of this codeunit (`AzureMLRequest.SetInput(InputName, AzureMLInputBuilder)`). It runs as a
    /// plain DotNet variable's use on assumption `with-events-no-trigger-azure-ml-shape`; any other use is refused.
    pub(crate) fn with_events_no_trigger_use(&mut self, n: &SyntaxNode, name: &str) -> L<()> {
        let parent = n.parent();
        let same_var =
            |e: &SyntaxNode| e.kind() == SyntaxKind::NameRef && e.text_string().trim().eq_ignore_ascii_case(name);
        let allowed = match parent.as_ref().map(|p| (p.kind(), p)) {
            Some((SyntaxKind::MemberExpr, p)) => p.children().next().as_ref() == Some(n),
            Some((SyntaxKind::AssignStmt, p)) => {
                let kids: Vec<SyntaxNode> = p.children().collect();
                let plain = p.children_with_tokens().any(|t| t.kind() == SyntaxKind::Assign);
                plain
                    && kids.len() == 2
                    && &kids[0] == n
                    && kids[1].kind() == SyntaxKind::CallExpr
                    && kids[1]
                        .children()
                        .next()
                        .filter(|c| c.kind() == SyntaxKind::MemberExpr)
                        .is_some_and(|m| {
                            m.children().next().is_some_and(|r| same_var(&r)) && self.no_trigger_ctor_call(n, &m)
                        })
            }
            Some((SyntaxKind::ArgList, p)) => p.parent().is_some_and(|call| {
                call.kind() == SyntaxKind::CallExpr
                    && match self.res(&call) {
                        Some(Res::Builtin(id)) => id == "System.IsNull#1" && p.children().count() == 1,
                        _ => call
                            .children()
                            .next()
                            .filter(|c| c.kind() == SyntaxKind::MemberExpr)
                            .and_then(|c| c.children().next())
                            .is_some_and(|r| self.with_events_no_trigger(&r).is_some()),
                    }
            }),
            _ => false,
        };
        if allowed {
            self.assume(al_builtins::assumptions::Assumption::WithEventsNoTriggerAzureMlShape);
            return Ok(());
        }
        self.no_trigger_refusal(name)
    }

    /// Whether the member expression `m` on the AzureML variable `var` names the constructor of the variable's DotNet type
    /// (AzureMLConnector.Codeunit.al:42, :47, :49 assign only from the constructor).
    fn no_trigger_ctor_call(&self, var: &SyntaxNode, m: &SyntaxNode) -> bool {
        let Ty::DotNet(alias) = self.ty_of(var) else {
            return false;
        };
        let member = m.text_string();
        let member = member.rsplit('.').next().unwrap_or("").trim().trim_matches('"');
        self.ws
            .resolve_dotnet(self.file, &alias)
            .is_some_and(|i| self.dotnet_ctor_name(i, member))
    }

    /// A member of the AzureML variable `name` ([`WITH_EVENTS_NO_TRIGGER`]): a method call, the only member form codeunit
    /// 2001 makes (AzureMLConnector.Codeunit.al:42-189). A property read or write is refused.
    pub(crate) fn with_events_no_trigger_member(&mut self, name: &str, op: &ir::DotNetOp) -> L<()> {
        match op {
            ir::DotNetOp::Call(_) => Ok(()),
            _ => self.no_trigger_refusal(name),
        }
    }

    fn no_trigger_refusal(&mut self, name: &str) -> L<()> {
        let why = format!(
            "variable `{name}` has the attribute WithEvents: only the uses Base Application's Azure ML Connector makes run"
        );
        self.run_refusals.insert(why.clone());
        Err(why)
    }

    /// The name of the AzureML variable ([`WITH_EVENTS_NO_TRIGGER`]) `recv` names, if it is one.
    pub(crate) fn with_events_no_trigger(&self, recv: &SyntaxNode) -> Option<String> {
        self.marked_var(recv, WITH_EVENTS_NO_TRIGGER)
    }

    fn timer_refusal(&mut self, name: &str) -> L<()> {
        let why = format!(
            "variable `{name}` has the attribute WithEvents: only its constructor without arguments, Interval, Enabled, IsNull and Clear are measured"
        );
        self.run_refusals.insert(why.clone());
        Err(why)
    }

    /// The name of the WithEvents Timer ([`WITH_EVENTS_TIMER_ELAPSED`]) `recv` names, if it is one.
    pub(crate) fn with_events_timer(&self, recv: &SyntaxNode) -> Option<String> {
        self.marked_var(recv, WITH_EVENTS_TIMER_ELAPSED)
    }

    /// The name of the global `recv` names when [`marker`] gave it `wanted`.
    fn marked_var(&self, recv: &SyntaxNode, wanted: &str) -> Option<String> {
        if recv.kind() != SyntaxKind::NameRef || self.in_request_page {
            return None;
        }
        let Some(Res::Global(name)) = self.res(recv) else {
            return None;
        };
        self.global_var_attrs
            .get(name)
            .is_some_and(|a| a.iter().any(|a| a == wanted))
            .then(|| name.clone())
    }
}
