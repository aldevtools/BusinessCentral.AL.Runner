//! Page Metadata columns 25 "SourceTableTemporary" and 29 "App ID".
//!
//! Pinned by corpus platform/lt-y59-page-metadata/Y59PageMetaSourceTemp.al:192-197 (corpus master 9cf26907): the column is Yes
//! for a page that declares `SourceTableTemporary = true` (a List and a Card) and for a page whose source table is
//! `TableType = Temporary` and that does not declare the property; No for an explicit `false`, for no property and for a page
//! without a SourceTable. "App ID" is the id of the app that declares the page (:192, `own_app=Yes` for all six pages).

use super::objects::{Catalog, ident, prop};
use al_builtins::value::Value;
use al_ir::{Object, ObjectHead};

/// Column 25 of page `o`, whose source table is `src` (`has_source` false for a page without one). A declaration that is not a
/// literal, an explicit `false` on a temporary table and a declaration on a page without a source table are not pinned: the
/// column is unknown.
pub(super) fn source_table_temporary(o: &Object, has_source: bool, src: Option<&ObjectHead>) -> Value {
    let table_temporary = src
        .and_then(|t| prop(t, "TableType"))
        .is_some_and(|v| ident(v.trim()).eq_ignore_ascii_case("Temporary"));
    let declared = prop(o, "SourceTableTemporary").map(|v| v.trim().to_ascii_lowercase());
    match (declared.as_deref(), table_temporary) {
        (None, _) if !has_source => Value::Boolean(false),
        (Some("true"), _) if has_source => Value::Boolean(true),
        (None, temporary) => Value::Boolean(temporary),
        (Some("false"), false) if has_source => Value::Boolean(false),
        (Some(v), _) => Value::Unknown(
            format!(
                "Page Metadata: page '{}': SourceTableTemporary `{v}` (on a table of TableType {}) is not pinned",
                o.name,
                if table_temporary { "Temporary" } else { "Normal" }
            )
            .into(),
        ),
    }
}

/// Column 29: the id of the app that declares page `o`.
pub(super) fn app_id(c: &Catalog, o: &Object) -> Value {
    Value::Guid(c.owner(o))
}
