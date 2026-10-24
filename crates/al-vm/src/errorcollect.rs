//! Error collection: `[ErrorBehavior(ErrorBehavior::Collect)]`, `Error(ErrorInfo)` of a collectible
//! error, `GetCollectedErrors`, `HasCollectedErrors`, `ClearCollectedErrors`, `IsCollectingErrors`.
//!
//! Every rule is pinned by corpus errorinfo/TestErrorInfoType.al (its header, findings A to D, and
//! the tests at :448-520); what it does not pin is refused or listed in docs/probes-needed.md.
//!
//! - The attribute opens a scope for the dynamic extent of the procedure it is on. It does not
//!   catch an `Error` raised in that procedure's own body, only in what it calls (finding B).
//! - A collectible `Error(ErrorInfo)` inside a scope is kept and the call returns normally; a
//!   non-collectible one unwinds as usual (:474-490).
//! - Closing the OUTERMOST scope rethrows what it holds: the message itself for one error, a
//!   summary for several; the set is gone afterwards (finding C). An inner scope hands its errors
//!   to the outer one.
//! - `HasCollectedErrors` is a within-scope predicate and answers true outside any (finding D).

use crate::{AlError, Session, Signal};
use al_builtins::builtin_id::ids;
use al_builtins::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// The open collecting scopes (each the number of errors held when it opened) and the errors held.
#[derive(Debug, Default)]
pub(crate) struct Scopes {
    starts: Vec<usize>,
    held: Vec<Value>,
}

impl Scopes {
    pub(crate) fn open(&mut self) {
        self.starts.push(self.held.len());
    }

    /// The errors held since the innermost scope opened.
    fn visible(&self) -> &[Value] {
        match self.starts.last() {
            Some(&s) => &self.held[s..],
            None => &[],
        }
    }
}

/// Whether the routine carries `[ErrorBehavior(ErrorBehavior::Collect)]`. `Default` is refused: the
/// corpus never uses it, so what it does inside an outer collecting scope is not pinned.
pub(crate) fn declares_collect(attrs: &[al_ir::Attribute]) -> Result<bool, Signal> {
    let mut collect = false;
    for a in attrs.iter().filter(|a| a.name.eq_ignore_ascii_case("ErrorBehavior")) {
        let t = a.text.to_ascii_lowercase();
        match t.split_once("::").map(|(_, v)| v.trim_end_matches([')', ']', ' '])) {
            Some("collect") => collect = true,
            _ => {
                return Err(Signal::Unsupported(format!(
                    "{} (only ErrorBehavior::Collect is pinned)",
                    a.text
                )));
            }
        }
    }
    Ok(collect)
}

/// The text a scope raises when it closes holding several errors. The corpus pins that it carries
/// the first message and calls it a "Multiple errors occurred..." summary (errorinfo/
/// TestErrorInfoType.al:129,500-503); the whole text is session/TestProbeSessionMiscContracts.al:ErrorCollection_ScopeExitText.
fn summary(first: &str) -> String {
    format!(
        "Multiple errors occurred during the operation the first of which is: {first}. All errors can be found under 'Detailed Information'."
    )
}

impl Session {
    /// `Error(ErrorInfo)` of a collectible error: kept when a scope is open.
    pub(crate) fn collect_error(&mut self, info: &Value) -> bool {
        if self.error_scopes.starts.is_empty() {
            return false;
        }
        match al_builtins::errorinfo::copy_of(info) {
            Some(copy) => {
                al_builtins::errorinfo::set_callstack_of(&copy, &self.collected_call_stack());
                self.error_scopes.held.push(copy);
                true
            }
            None => false,
        }
    }

    /// Ends the scope the procedure opened. A failure discards what the scope held and goes on
    /// unwinding; a normal end of the outermost scope rethrows what it held.
    pub(crate) fn close_error_scope(&mut self, outcome: Result<(), Signal>) -> Result<(), Signal> {
        let sc = &mut self.error_scopes;
        let start = sc.starts.pop().unwrap_or(0);
        if matches!(&outcome, Err(e) if !matches!(e, Signal::Exit(_) | Signal::Break | Signal::Continue)) {
            sc.held.truncate(start);
            return outcome;
        }
        if !sc.starts.is_empty() || sc.held.len() <= start {
            return outcome;
        }
        let held = std::mem::take(&mut sc.held);
        let mut first = al_builtins::errorinfo::message_of(&held[0]).unwrap_or_default();
        // An Internal error raises the generic text with a backslash before its correlation id, where its collected Message
        // has a line break (misc/lt-probes-needed-2/PnBErrorsCollections.al:360-375 RO_CollectedErrorInfo_Internal_NotCleared_
        // ScopeExit). As the first of several, the summary is not measured.
        if al_builtins::errorinfo::is_internal(&held[0]) {
            if held.len() > 1 {
                return Err(Signal::Unsupported(
                    "several collected errors at the end of the scope, the first an Internal one (the summary text is not pinned)"
                        .into(),
                ));
            }
            first = first.replace("\r\n", "\\");
        }
        let message = if held.len() == 1 { first } else { summary(&first) };
        // session/TestProbeSessionMiscContracts.al:ErrorCollection_ScopeExitText, ErrorCollection_SingleError: code Dialog.
        Err(Signal::Error(Box::new(AlError {
            message,
            code: "Dialog".into(),
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// The error-collection builtins; `None` when `id` is not one of them.
    pub(crate) fn error_collection_op(
        &mut self,
        id: al_ir::BuiltinId,
        args: &[Value],
    ) -> Option<Result<Value, Signal>> {
        Some(match id {
            ids::SYSTEM_ISCOLLECTINGERRORS_1 => Ok(Value::Boolean(!self.error_scopes.starts.is_empty())),
            // Outside a scope the platform computes 0 < 0 - (-1), which is true
            // (errorinfo/TestErrorInfoType.al:133-141, corpus docs/al-language-coverage-gaps.md:518).
            ids::SYSTEM_HASCOLLECTEDERRORS_1 => Ok(Value::Boolean(
                self.error_scopes.starts.is_empty() || !self.error_scopes.visible().is_empty(),
            )),
            ids::SYSTEM_CLEARCOLLECTEDERRORS_1 => {
                if let Some(&s) = self.error_scopes.starts.last() {
                    self.error_scopes.held.truncate(s);
                }
                Ok(Value::Null)
            }
            ids::SYSTEM_GETCOLLECTEDERRORS_1 => {
                // The list is what the scope holds; with `Clear` true the scope is emptied afterwards, as the
                // documentation says (handlers/PWRefusedBuiltins.al:FieldError_WithErrorInfo_InCollectScope pins the
                // list a call with true returns; that it empties the scope is the documented meaning only).
                let clear = match args.first() {
                    None | Some(Value::Null) => false,
                    Some(Value::Boolean(b)) => *b,
                    Some(other) => {
                        return Some(Err(Signal::Unsupported(format!(
                            "GetCollectedErrors with {}",
                            al_builtins::describe::Describe::describe(other)
                        ))));
                    }
                };
                let copies = self
                    .error_scopes
                    .visible()
                    .iter()
                    .filter_map(al_builtins::errorinfo::copy_of)
                    .collect();
                if clear {
                    if let Some(&s) = self.error_scopes.starts.last() {
                        self.error_scopes.held.truncate(s);
                    }
                }
                Ok(Value::List(Rc::new(RefCell::new(copies))))
            }
            _ => return None,
        })
    }
}
