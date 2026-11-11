//! `FieldRef.SetFilter` with a placeholder argument and `FieldRef.SetRange` on a Media or MediaSet field: BC cannot convert the
//! argument to the field type and raises an error, for a Guid argument and for the text of a Guid alike
//! (recordref-fieldref/fieldref/FieldRefFilterMediaGuidText.al:258-296 and :397-434, al-runner issue 7461). A filter text without
//! an argument is read by `al_builtins::filter`.

use crate::Session;
use al_builtins::library_kind::LibraryKind;
use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};

/// The filter forms with one argument the corpus pins (`ArgForms`: `%1`, `<>%1`, `>=%1`).
const PINNED_FORMS: [&str; 3] = ["%1", "<>%1", ">=%1"];

impl Session {
    /// The error of FieldRef `method` (SetFilter or SetRange) with `args` on a field of type `ty`, before any filter is set;
    /// `Ok(())` when the field is no Media or MediaSet or the call passes no value to convert.
    /// From BC 28: `The value "<value>" can't be evaluated into type MediaSet.` (Evaluate:Argument), the value as Format writes a
    /// Guid; BC 27: `Invalid Conversion. Source: NavVariant, Destination: NavMediaSet.` (Argument); `Media` and `NavMedia` on a
    /// Media field (`#if BC28PLUS` at :258, :296, :336, :397, :434). Pinned arguments: a Guid, the braced upper-case text of a Guid
    /// (the only text form the corpus passes), and for SetRange a second Guid; the error names the first value. Every other
    /// argument and form is refused.
    pub(crate) fn media_fieldref_filter_argument(
        &self,
        method: &str,
        ty: &RtType,
        args: &[Value],
    ) -> Result<(), Signal> {
        let type_name = match ty {
            RtType::Library(LibraryKind::Media) => "Media",
            RtType::Library(LibraryKind::MediaSet) => "MediaSet",
            _ => return Ok(()),
        };
        let first = match (method, args) {
            ("SetFilter", [_]) | ("SetRange", []) => return Ok(()),
            ("SetFilter", [Value::Text(form), v]) if PINNED_FORMS.contains(&&**form) => v,
            ("SetRange", [v]) | ("SetRange", [v, Value::Guid(_)]) => v,
            _ => {
                return Err(Signal::unsupported(format!(
                    "FieldRef.{method} on a {type_name} field with these arguments (not pinned)"
                )));
            }
        };
        let shown = match first {
            Value::Guid(g) => al_builtins::format::format_guid(g),
            Value::Text(t) if is_braced_upper_guid(t) => t.to_string(),
            other => {
                return Err(Signal::unsupported(format!(
                    "FieldRef.{method} on a {type_name} field with the argument {} (not pinned)",
                    al_builtins::describe::Describe::describe(other)
                )));
            }
        };
        if self.has(al_version::Gate::MediaFieldRefFilterArgumentEvaluateErrorFrom28) {
            al_builtins::native::raise(
                format!("The value \"{shown}\" can't be evaluated into type {type_name}."),
                "Evaluate:Argument",
            )
        } else {
            al_builtins::native::raise(
                format!("Invalid Conversion. Source: NavVariant, Destination: Nav{type_name}."),
                "Argument",
            )
        }
    }
}

/// Whether `t` is a Guid as Format writes it: braces, upper-case hexadecimal digits and hyphens.
fn is_braced_upper_guid(t: &str) -> bool {
    match al_builtins::evaluate::read_text(&RtType::Guid, t) {
        Some(Value::Guid(g)) => al_builtins::format::format_guid(&g) == t,
        _ => false,
    }
}
