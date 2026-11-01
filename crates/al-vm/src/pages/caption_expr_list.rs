//! The caption a TestPage shows for a List, Worksheet or ListPart with `DataCaptionExpression` (gap G5-18).

use super::*;

impl Session {
    /// The caption of a page with a repeater and `DataCaptionExpression`, pinned by
    /// pages/lt-y50-list-caption-expr/Y50ListCaptionExpr.al:
    /// - a List or Worksheet shows `<caption> - <value>`, the expression evaluated at each read: a global set before the run
    ///   (:322-324 `- X1` after open, Next and GoToKey), a procedure reading Rec's filter (:367-369, also after
    ///   TestPage.Filter.SetFilter), the current row (:409-411), a Worksheet in a modal handler (:384-386);
    /// - an empty value shows the caption alone (:341 the global left empty, :368 no filter);
    /// - on no row (an empty view) the caption alone (:412 `LT Y50 DC List Row|`);
    /// - the page's DataCaptionFields add nothing next to the expression (:436-437);
    /// - a ListPart shows the value alone, without its caption (:452 `Part Alpha`); empty or on no row it is not pinned.
    ///
    /// A new row is not pinned and is refused.
    pub(super) fn list_data_caption_expression_caption(&mut self, st: &St, base: String) -> Result<String, Signal> {
        let part = st.borrow().host.is_some();
        {
            let s = st.borrow();
            if s.draft || s.mode == PageMode::New {
                return unsupported("the caption of a list with DataCaptionExpression on a new row (not pinned)");
            }
            if s.cursor == Cursor::None {
                if part {
                    return unsupported("the caption of a list part with DataCaptionExpression and no row (not pinned)");
                }
                return Ok(base);
            }
        }
        let value = self.data_caption_expression_value(st)?;
        if value.is_empty() {
            if part {
                return unsupported("the caption of a list part with DataCaptionExpression whose value is empty (not pinned)");
            }
            return Ok(base);
        }
        if part {
            return Ok(value);
        }
        Ok(format!("{base} - {value}"))
    }

    /// The value of the page's `DataCaptionExpression`, evaluated now.
    pub(super) fn data_caption_expression_value(&mut self, st: &St) -> Result<String, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let Some(r) = engine.program.objects[object]
            .routines
            .iter()
            .position(|x| x.name == ir::DATA_CAPTION_ROUTINE)
        else {
            return unsupported("the caption of a page with DataCaptionExpression (the expression was not compiled)");
        };
        match self.owner_routine(st, None, r)? {
            Value::Text(t) | Value::Code(t) => Ok(t.to_string()),
            _ => unsupported("the caption of a page with DataCaptionExpression (the expression is no text)"),
        }
    }
}
