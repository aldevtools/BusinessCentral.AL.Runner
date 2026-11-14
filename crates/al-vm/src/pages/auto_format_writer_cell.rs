//! The page side of `auto_format_writer`: a control whose AutoFormatType a subscriber could resolve, and the two cells where BC was
//! measured with a writer the test can see.

use super::*;

/// Which read of a control's text asks for its format.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlRead {
    /// `TestField.Value()` (`field_value_text`).
    Value,
    /// Any other read or entry (SetValue, AssertEquals, a filter text, ...).
    Other,
}

impl Session {
    /// Refuses the format of control `idx` of AutoFormatType `kind` (the control's, else its field's) while a subscriber could
    /// write its result (`Session::auto_format_writer_refusal`).
    pub(super) fn refuse_auto_format_writer_on_control(
        &self,
        st: &St,
        idx: usize,
        kind: &str,
        read: ControlRead,
    ) -> Result<(), Signal> {
        let measured = read == ControlRead::Value && self.auto_format_writer_cell(st, idx);
        match self.auto_format_writer_refusal(kind, measured) {
            Some(why) => unsupported(why),
            None => Ok(()),
        }
    }

    /// One of the two cells measured with a writer the test can see (the corpus fixture PtFixtures.al:148-158, log mode 2), each fact
    /// by equality: a List opened by the test's OpenView with the control's own AutoFormatType 1 (events/platform-triggers/
    /// PtObserve.al:463-465, :482-497 with PtObjects.al:31, :45), or a Card opened by the test's OpenEdit with the control's own
    /// AutoFormatType 2 (PtObserve.al:546-549, :558-570 with PtObjects.al:57, :67); both with the control's literal blank
    /// AutoFormatExpression, no DecimalPlaces on the control, and bound to a Decimal field of Rec that declares no AutoFormatType,
    /// AutoFormatExpression or DecimalPlaces (PtObjects.al:15). The read is `Value()` (the caller).
    fn auto_format_writer_cell(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let ctl = &page_of(&engine, st).controls[idx];
        let lit = |n: &str| match ctl.prop(n) {
            Some(ir::PropVal::Lit(v)) => Some(v.trim().to_string()),
            _ => None,
        };
        let page = engine.object_facts[object].page_type.clone();
        let open = {
            let s = st.borrow();
            match (page, lit("autoformattype").as_deref()) {
                (Some(ir::PageType::List), Some("1")) => s.test_open_view(),
                (Some(ir::PageType::Card), Some("2")) => s.test_open_edit(),
                _ => false,
            }
        };
        let plain_field = self.control_field(st, idx).is_some_and(|f| {
            matches!(f.ty, RtType::Decimal)
                && ["AutoFormatType", "AutoFormatExpression", "DecimalPlaces"]
                    .iter()
                    .all(|n| !f.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(n)))
        });
        open && lit("autoformatexpression").as_deref() == Some("''")
            && ctl.prop("decimalplaces").is_none()
            && plain_field
    }
}
