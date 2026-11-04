//! The filter text a `SetFilter` call carries in its second argument.

use al_builtins::value::Value;

/// A Guid as the filter text is its braced text, the blank one included (record/filters/Y77GuidFilter.al
/// Record_GuidField, Record_TextField, Record_CodeField `*_sf_g1`, `*_sf_blank`); any other value is its text, none an empty one.
pub(crate) fn filter_text(arg: Option<&Value>) -> al_builtins::text::Txt {
    match arg {
        Some(Value::Guid(g)) => al_builtins::format::format_guid(g).into(),
        other => al_builtins::native::text_of(other.unwrap_or(&Value::Null)).unwrap_or_default(),
    }
}
