//! A .NET call given a text that holds rendered content (ADR-040 amendment 2026-11-12, A5 "The .NET sidecar"): refused unless
//! it is an entry of `al_builtins::rendered_text::DOTNET_ALLOWED` (P12). The rule and the table are `rendered_text`'s; this
//! names the call for it and checks the answer.

use al_builtins::rendered_text;
use al_builtins::value::Value;

use crate::{Session, Signal, ir};

impl Session {
    /// Before a .NET call while rendered content may exist: the marked input of an allowed call (its answer is then checked
    /// by [`Session::rendered_dotnet_answer`]), `None` when no argument holds rendered content; refused otherwise.
    pub(crate) fn rendered_dotnet_guard(
        &self,
        ty: Option<usize>,
        recv: &Value,
        op: &ir::DotNetOp,
        values: &[Value],
    ) -> Result<Option<String>, Signal> {
        // Only a static call names its type here; an instance member, a constructor, a property and a set match no entry.
        let static_type = match (recv, ty) {
            (Value::Null, Some(i)) => self.engine.program.dotnet_types[i].type_name.as_str(),
            _ => "",
        };
        let member = match op {
            ir::DotNetOp::Call(m) => m.as_str(),
            _ => "",
        };
        if !rendered_text::dotnet_guard(static_type, member, values)? {
            return Ok(None);
        }
        match values.first() {
            Some(Value::Text(t) | Value::Code(t)) => Ok(Some(t.as_str().to_string())),
            _ => unreachable!("an allowed entry has the marked text as its first argument"),
        }
    }

    /// The answer of an allowed call on `input`: its text with every mark in the changed form, refused when a mark was lost.
    pub(crate) fn rendered_dotnet_answer(input: &str, answer: Result<Value, Signal>) -> Result<Value, Signal> {
        match answer? {
            Value::Text(t) => Ok(Value::Text(rendered_text::dotnet_result(input, t.as_str())?.into())),
            _ => Err(rendered_text::refusal(input, "a .NET answer that is not a text")),
        }
    }
}
