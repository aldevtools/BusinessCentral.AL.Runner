//! `Database::"X"` in a WHERE condition of a view written as an AL property, on an Integer field: the table's number.
//! A page's `SourceTableView` `const(Database::"X")` keeps the rows whose field holds the table's number and its
//! `Filter.GetFilter` reads that number; `filter(Database::A | Database::B)` is both numbers joined by `|` (al-runner issue
//! 7371; pages/view-filters/ViewConstBlankCode.al:734 SourceTableView_TableConst_RowsAndFilters, :769
//! SourceTableView_TableList_RowsAndFilters). A part's `SubPageView` `const(Database::"X")` keeps the same rows (:1003
//! SubPage_PartTable_Rows). A view given to `SetView` at run time is text, not AL: no `Database::` there.

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::RtType;

impl Session {
    /// The filter text of `value` when every `|` term of it is `Database::<table>` and `ty` is Integer, in a view written as
    /// an AL property (`runtime` false). `None` for any other text. A table name that names no table, or more than one, is
    /// refused.
    pub(crate) fn view_database_const(&self, value: &str, ty: &RtType, runtime: bool) -> Result<Option<String>, Signal> {
        if runtime || !matches!(ty, RtType::Integer) {
            return Ok(None);
        }
        let mut numbers = Vec::new();
        for term in value.split('|') {
            let Some((qualifier, name)) = term.trim().split_once("::") else {
                return Ok(None);
            };
            if !qualifier.trim().eq_ignore_ascii_case("Database") {
                return Ok(None);
            }
            let name = name.trim();
            let name = name
                .strip_prefix('\'')
                .and_then(|n| n.strip_suffix('\''))
                .map(|n| n.replace("''", "'"))
                .unwrap_or_else(|| name.to_string());
            let ids: Vec<u32> = self
                .engine
                .program
                .objects
                .heads()
                .iter()
                .filter(|o| o.table.is_some() && o.name.eq_ignore_ascii_case(&name))
                .filter_map(|o| o.id)
                .collect();
            match ids.as_slice() {
                [id] => numbers.push(id.to_string()),
                _ => return al_builtins::native::unsupported(format!("view condition `Database::{name}`")),
            }
        }
        Ok(Some(numbers.join("|")))
    }
}
