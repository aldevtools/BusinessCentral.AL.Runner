//! An empty value in a WHERE condition of a view given to `SetView` (the Retention Policy filter views of the System
//! Application, gap G5-14). Pinned by record/lt-y51-setview-shapes/Y51SetViewShapes.al:141-154 (SetView_EmptyFilterValue):
//! `1()` and `FILTER()` on a Text, Date or Integer field set a filter BC writes back as `==` and that keeps every row;
//! `CONST()` on a Text field selects the blank value. `CONST()` on an Integer is an Evaluate error, read by the parser
//! (types/lt-evaluate-format-lasterror-setview/P86SetView.al:139). Other field types keep the parser's refusal.

use crate::Session;
use al_builtins::filter::FilterExpr;
use al_builtins::native::Signal;
use al_builtins::value::RtType;

impl Session {
    /// The filter an empty `value` of a `kind(...)` condition sets on a field of type `ty`, or `None` when the value is not
    /// empty or the shape is not pinned (the caller parses it then).
    pub(crate) fn empty_view_value(
        &self,
        kind: &str,
        ty: &RtType,
        value: &str,
        runtime: bool,
    ) -> Result<Option<FilterExpr>, Signal> {
        if !runtime || !value.trim().is_empty() {
            return Ok(None);
        }
        Ok(match (kind, ty) {
            ("1" | "FILTER", RtType::Text(_) | RtType::Date | RtType::Integer) => Some(FilterExpr::EmptyViewValue),
            ("CONST", RtType::Text(_)) => Some(
                self.parse_filter_text("''", ty)
                    .map_err(|e| Signal::Unsupported(format!("SetView: {e}")))?,
            ),
            _ => None,
        })
    }
}
