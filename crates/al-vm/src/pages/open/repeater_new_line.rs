//! Whether a list whose repeater says `Editable` shows the blank new-row line.

use super::*;

/// What the `Editable` of the page's repeater says about the blank new-row line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::pages) enum RepeaterLine {
    /// No repeater says Editable, or it says the literal `true`: the page decides as before.
    Free,
    /// The repeater's Editable is false: no new-row line.
    Denied,
    /// The repeater's Editable is an expression the page's variables alone do not decide (a procedure call, a field): not pinned.
    Unknown,
}

impl Session {
    /// The value of the repeater's `Editable`, evaluated now, decides whether the new-row line shows (corpus branch
    /// al-runtime-rs-staging-y65 eefaf01b, pages/lt-y65-readonly-repeater/Y65ReadOnlyRepeater.al, al-runner issue 6674):
    /// - `Editable = false` (:255 List, :422 Worksheet, :443 ListPart in a Card) and a page variable that is false (:297): no
    ///   line, Next on the last row answers false and stays there;
    /// - a page variable that is true (:339): the line shows, as on a repeater without the property.
    ///
    /// An expression of the page's variables, constants and operators is taken by its value; any other expression (a procedure
    /// call, a field of Rec) is not pinned (`Unknown`, refused where the line would show).
    pub(in crate::pages) fn repeater_line(&mut self, st: &St) -> Result<RepeaterLine, Signal> {
        let engine = self.engine();
        let Some(idx) = page_of(&engine, st).controls.iter().position(|c| {
            c.kind == ir::ControlKind::Repeater
                && match c.prop("editable") {
                    Some(ir::PropVal::Lit(v)) => !v.trim().eq_ignore_ascii_case("true"),
                    Some(_) => true,
                    None => false,
                }
        }) else {
            return Ok(RepeaterLine::Free);
        };
        let literal = matches!(
            page_of(&engine, st).controls[idx].prop("editable"),
            Some(ir::PropVal::Lit(_))
        );
        drop(engine);
        if !literal && self.prop_global_slots(st, idx, "editable").is_none() {
            return Ok(RepeaterLine::Unknown);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::RepeaterEditableDecidesNewRowLine);
        Ok(if self.prop_bool_now(st, idx, "editable", true)? {
            RepeaterLine::Free
        } else {
            RepeaterLine::Denied
        })
    }

    /// TestPage.Editable(): the page's own editability, except that a Worksheet whose repeater says Editable = false answers
    /// Yes when it is opened with OpenView (pages/lt-y65-readonly-repeater/Y65ReadOnlyRepeater.al:423 `view two rows`
    /// `editable=No/Yes`). A Worksheet opened for view with an editable repeater is not pinned and keeps the earlier answer.
    pub(in crate::pages) fn test_page_editable(&mut self, st: &St) -> Result<bool, Signal> {
        if self.page_editable(st) {
            return Ok(true);
        }
        let object = st.borrow().object as usize;
        let worksheet = self.engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet);
        let only_view = {
            let s = st.borrow();
            s.mode == PageMode::View && !s.editable_off && !s.lookup && s.host.is_none()
        };
        let page_says_read_only = self.engine().program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Editable") && v.trim().eq_ignore_ascii_case("false"));
        Ok(worksheet && only_view && !page_says_read_only && self.repeater_line(st)? == RepeaterLine::Denied)
    }
}
