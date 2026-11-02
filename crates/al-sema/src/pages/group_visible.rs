//! Visible of the controls that hold other controls (cuegroup, fixed, grid) bound to an expression (al-runner issue 6675).
//!
//! Measured by corpus writer Y65 on BC 29 (pages/lt-y65-cue-visible/Y65CueVisible.al:235, :262): a cuegroup's Visible bound to a page variable (set in OnInit, OnOpenPage or
//! OnAfterGetRecord), `not` a variable or `not Rec.IsEmpty()` reads the value it has after the page opened (an array element
//! reads Yes on BC 29 whatever it holds, :316-331, and is refused);
//! bound to a call of the page's own procedure with no arguments (`IsShown()`) it reads No. BC compiles any Boolean expression
//! there (must-not-compile 2630: an Integer does not compile). What else a call does there is not measured: refused.

use super::{ClientExpr, client_expression};
use al_ir::ControlKind;

/// Whether property `prop` of a control of `kind` reads No when it is bound to a call of one of the page's own procedures with
/// no arguments (the procedure never runs). An action's Visible or Enabled and an action group's Visible
/// (pages/lt-qq-procedure-bound-action/QqBoundProps.al:29, :77-211); a cuegroup's Visible (Y65CueVisible.al:117 GrpCall,
/// `cuegroup IsShown() (J)` reads `No 0` at :235 and :262).
pub fn bound_to_procedure_call(kind: Option<&ControlKind>, prop: &str) -> bool {
    match kind {
        Some(ControlKind::Action) => prop == "visible" || prop == "enabled",
        Some(ControlKind::ActionGroup | ControlKind::CueGroup) => prop == "visible",
        _ => false,
    }
}

/// The literal a property bound to a procedure call (see [`bound_to_procedure_call`]) reads. An action's is `false`. A cuegroup's
/// is `'false'`: it reads No as well, but the fields in it stay on the page and read their values (Y65CueVisible.al:235, :262
/// `No 0`), while a control under a Visible of the plain literal `false` is not on the runtime page (`control_eliminated`).
pub fn procedure_bound_value(kind: Option<&ControlKind>) -> String {
    match kind {
        Some(ControlKind::CueGroup) => "'false'".to_string(),
        _ => "false".to_string(),
    }
}

/// [`client_expression`] for property `prop` of a control of `kind`. A cuegroup's Visible may call `IsEmpty()` of Rec: `not
/// Rec.IsEmpty()` reads the value after open (Y65CueVisible.al:129 GrpEmpty, `Yes 0` at :235 and :262). Any other call stays
/// refused.
pub fn client_expression_of(kind: Option<&ControlKind>, prop: &str, value: &str) -> Option<ClientExpr> {
    if matches!(kind, Some(ControlKind::CueGroup)) && prop == "visible" {
        // An array element there reads Yes on BC 29 whatever it holds (Y65CueVisible.al:316-331 `ShowArr[1] false (A)` is
        // `Yes 0`); the runtime reads the element, and BC 27 and 28 are not measured: refused.
        if value.contains('[') {
            return Some(ClientExpr::Refused(format!(
                "a cuegroup's Visible bound to the array element expression `{}` (BC 29 shows the group whatever the element \
                 holds; not served)",
                value.trim()
            )));
        }
        return client_expression(&without_rec_is_empty(value));
    }
    client_expression(value)
}

/// `value` with each `Rec.IsEmpty()` (any case, spaces around the parentheses) put as a plain name, so that the check of
/// [`client_expression`] does not see it as a call.
fn without_rec_is_empty(value: &str) -> String {
    let lower = value.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while i < value.len() {
        let rest = &lower[i..];
        let at_word_start = i == 0 || !lower[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '.');
        if at_word_start && let Some(after) = rest.strip_prefix("rec.isempty") {
            let tail = after.trim_start();
            if let Some(tail) = tail.strip_prefix('(') {
                let tail = tail.trim_start();
                if let Some(tail) = tail.strip_prefix(')') {
                    out.push_str("RecIsEmpty");
                    i = value.len() - tail.len();
                    continue;
                }
            }
        }
        let c = value[i..].chars().next().unwrap_or(' ');
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Whether a `fixed` or `grid` section binds Visible to an expression: the shapes no corpus test measures (a cuegroup's are
/// pages/lt-ab-group-properties/ABCueProps.al and pages/lt-y65-cue-visible/Y65CueVisible.al).
pub fn holder_visible_awaits_pin(section: &al_syntax::ast::Section) -> bool {
    let name = section.name().to_ascii_lowercase();
    if !["fixed", "grid"].contains(&name.as_str()) {
        return false;
    }
    section.properties().any(|p| {
        let v = p.value_text();
        let v = v.trim();
        let literal = v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false");
        // A fixed or grid control's Visible is measured for no shape: a variable there rests on the assumption too.
        p.name().eq_ignore_ascii_case("visible") && !literal
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rec_is_empty_is_hidden_from_the_call_check() {
        assert_eq!(without_rec_is_empty("not Rec.IsEmpty()"), "not RecIsEmpty");
        assert_eq!(without_rec_is_empty("not rec.ISEMPTY ( )"), "not RecIsEmpty");
        assert_eq!(without_rec_is_empty("not xRec.IsEmpty()"), "not xRec.IsEmpty()");
        assert_eq!(without_rec_is_empty("Rec.IsEmptyX()"), "Rec.IsEmptyX()");
    }

    #[test]
    fn only_a_cuegroup_visible_takes_rec_is_empty() {
        assert_eq!(
            client_expression_of(Some(&ControlKind::CueGroup), "visible", "not Rec.IsEmpty()"),
            None
        );
        assert!(client_expression_of(Some(&ControlKind::Group), "visible", "not Rec.IsEmpty()").is_some());
        assert!(client_expression_of(Some(&ControlKind::CueGroup), "visible", "Rec.Count() > 0").is_some());
        assert!(client_expression_of(Some(&ControlKind::CueGroup), "visible", "IsShown() and X").is_some());
    }

    /// Y65CueVisible.al:316-331 (BC 29): a cuegroup's Visible bound to an array element reads Yes even when the element is
    /// false, which the runtime does not serve: refused. On BC 27 and 28 it is not measured.
    #[test]
    fn a_cuegroup_visible_bound_to_an_array_element_is_refused() {
        assert!(matches!(
            client_expression_of(Some(&ControlKind::CueGroup), "visible", "ShowArr[1]"),
            Some(ClientExpr::Refused(_))
        ));
        assert!(matches!(
            client_expression_of(Some(&ControlKind::CueGroup), "visible", "not ShowArr[2]"),
            Some(ClientExpr::Refused(_))
        ));
    }
}
