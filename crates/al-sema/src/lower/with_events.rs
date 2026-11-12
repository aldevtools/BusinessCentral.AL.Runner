//! Global `[WithEvents]` DotNet variables whose uses run (al-runner issue 7832): which ones, and for a Timer with an Elapsed
//! trigger, which uses. Every other WithEvents variable keeps the refusal of `Lowerer::var_attr_refusal`.

use super::*;
use al_syntax::ast::NodeText;

/// The name a global `[WithEvents]` variable's attribute gets when its uses run, or `None` (the refusal of
/// `Lowerer::var_attr_refusal` stays):
/// - [`WITH_EVENTS_NO_TRIGGER`]: a DotNet variable of a codeunit that declares no `trigger name::Event` for it. No AL code then
///   runs on its events, and its uses run as a plain DotNet variable's, marked with assumption `with-events-no-trigger`
///   (Microsoft's Sales and Inventory Forecast Tests, codeunit 139540 TestMissingApiUriOpenSetup; corpus PR 1365,
///   onprem-dotnet dotnet/ac/N4WithEventsNoRaise.al:320).
/// - [`WITH_EVENTS_TIMER_ELAPSED`]: a `System.Timers.Timer` variable of a codeunit, a page, a table or a report whose one trigger
///   is `name::Elapsed` (N4WithEventsNoRaise.al:291 codeunit, :372 page, :387 table, :405 report). The timer never starts, so the
///   trigger never runs; the uses that may run are those the probe made (`Lowerer::with_events_timer_use`, `Lowerer::with_events_timer_member`), marked with
///   assumption `with-events-timer-elapsed-not-started`.
pub(super) fn marker(lw: &Lowerer<'_>, kind: ObjKind, ty: &Ty, obj: &ast::Object, name: &str) -> Option<&'static str> {
    let Ty::DotNet(alias) = ty else { return None };
    let prefix = format!("{}::", name.to_lowercase());
    let events: Vec<String> = obj
        .triggers()
        .filter_map(|t| t.name().to_lowercase().strip_prefix(&prefix).map(str::to_string))
        .collect();
    if events.is_empty() && kind == ObjKind::Codeunit {
        return Some(WITH_EVENTS_NO_TRIGGER);
    }
    let timer = lw
        .ws
        .resolve_dotnet(lw.file, alias)
        .is_some_and(|i| lw.ws.dotnet[i].decl.type_name == "System.Timers.Timer");
    let measured_kind = matches!(
        kind,
        ObjKind::Codeunit | ObjKind::Page | ObjKind::Table | ObjKind::Report
    );
    (timer && measured_kind && events == ["elapsed"]).then_some(WITH_EVENTS_TIMER_ELAPSED)
}

/// The name a `[WithEvents]` attribute gets for a variable [`marker`] accepts with no trigger.
pub(crate) const WITH_EVENTS_NO_TRIGGER: &str = "withevents-no-trigger";

/// The name a `[WithEvents]` attribute gets for a Timer variable [`marker`] accepts with an Elapsed trigger.
pub(crate) const WITH_EVENTS_TIMER_ELAPSED: &str = "withevents-timer-elapsed";

/// The builtin `Clear` of a [`WITH_EVENTS_TIMER_ELAPSED`] variable lowers to (al-builtins `native/values.rs`).
const CLEAR_WITH_EVENTS_TIMER: &str = "System.ClearWithEventsTimer#1";


impl Lowerer<'_> {
    /// A use of the WithEvents Timer `name` ([`WITH_EVENTS_TIMER_ELAPSED`]) at `n`: the receiver of a member
    /// ([`Self::with_events_timer_member`] checks which), the target of `name := name.<constructor>()`, or the one argument of
    /// `IsNull` or `Clear` (N4WithEventsNoRaise.al:178-201, :61-65, :93-97, :140-144). Any other use is refused: the object could
    /// reach code that starts it.
    pub(crate) fn with_events_timer_use(&mut self, n: &SyntaxNode, name: &str) -> L<()> {
        let parent = n.parent();
        let same_var = |e: &SyntaxNode| e.kind() == SyntaxKind::NameRef && e.text_string().trim().eq_ignore_ascii_case(name);
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
                        .and_then(|c| c.children().next())
                        .is_some_and(|r| same_var(&r))
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
            self.assume(al_builtins::assumptions::Assumption::WithEventsTimerElapsedNotStarted);
            return Ok(());
        }
        self.timer_refusal(name)
    }

    /// A member of the WithEvents Timer `name` ([`WITH_EVENTS_TIMER_ELAPSED`]): the constructor without arguments, a read or an
    /// assignment of `Interval` and a read of `Enabled` (N4WithEventsNoRaise.al:178-201). Any other member is refused: `Start`,
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

    /// `Clear(name)` of the WithEvents Timer `name`: the variable becomes null (N4WithEventsNoRaise.al:197-198, :313-314
    /// `null_cleared` Yes). The object it held is disposed then (a Disposed trigger runs, :347), which nothing can see here: the
    /// variable has no Disposed trigger and the object never left it ([`Self::with_events_timer_use`]).
    pub(crate) fn with_events_timer_clear_id(&self, id: &str, args: &[SyntaxNode]) -> Option<&'static str> {
        match args {
            [a] if id == "System.Clear#1" && self.with_events_timer(a).is_some() => Some(CLEAR_WITH_EVENTS_TIMER),
            _ => None,
        }
    }

    fn timer_refusal(&mut self, name: &str) -> L<()> {
        let why = format!(
            "variable `{name}` has the attribute WithEvents and an Elapsed trigger: only its constructor, Interval, Enabled, IsNull and Clear are measured"
        );
        self.run_refusals.insert(why.clone());
        Err(why)
    }

    /// The name of the WithEvents Timer ([`WITH_EVENTS_TIMER_ELAPSED`]) `recv` names, if it is one.
    pub(crate) fn with_events_timer(&self, recv: &SyntaxNode) -> Option<String> {
        if recv.kind() != SyntaxKind::NameRef || self.in_request_page {
            return None;
        }
        let Some(Res::Global(name)) = self.res(recv) else { return None };
        self.global_var_attrs
            .get(name)
            .is_some_and(|a| a.iter().any(|a| a == WITH_EVENTS_TIMER_ELAPSED))
            .then(|| name.clone())
    }
}
