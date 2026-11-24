//! A Record method written without parentheses as the source of a control of a page's own layout (`field(Total; Rec.Count)`).
//!
//! Measured in corpus pull request 1525 (open), tests/al-language/pages/controls/RecCountControl.al, assumption
//! `page-control-rec-method-source`: on a List or Card page over a table, in a repeater or outside it, `Rec.Count`,
//! `Rec.CountApprox`, `Rec.GetFilters`, `Rec.IsEmpty`, `Rec.TableCaption` and `Rec.TableName` compile and show the value of the
//! method for the page's rows (a TestPage filter and the SourceTableView count). A field of the table with that name wins.
//! Every other method, page type, object kind and `CountApprox` under a filter keep the refusal of main.
use super::assignable_path;
use al_syntax::ast;

/// The methods the probe measured, lower case.
const MEASURED: [&str; 6] = [
    "count",
    "countapprox",
    "getfilters",
    "isempty",
    "tablecaption",
    "tablename",
];

/// Whether `name` is one of the measured Record methods.
pub(crate) fn measured_name(name: &str) -> bool {
    MEASURED.iter().any(|m| m.eq_ignore_ascii_case(name))
}

/// Whether `obj` is a page of the measured page types (List or Card).
pub(crate) fn measured_page(obj: &ast::Object) -> bool {
    obj.properties()
        .find(|p| p.name().eq_ignore_ascii_case("PageType"))
        .map(|p| p.value_text())
        .is_some_and(|t| ["list", "card"].iter().any(|k| t.trim().eq_ignore_ascii_case(k)))
}

/// Whether `text` is `Rec.<name>` with an unquoted name: a field, or a parameterless method called without parentheses.
pub(super) fn unquoted_rec_name(text: &str) -> bool {
    let t = text.trim();
    t.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("rec."))
        && !t[4..].is_empty()
        && t[4..].chars().all(|c| c.is_alphanumeric() || c == '_')
        && !t[4..].starts_with(|c: char| c.is_ascii_digit())
}

/// Whether `src` is `Rec.<measured method>`, unquoted and without parentheses.
fn measured_source(src: &str) -> bool {
    unquoted_rec_name(src) && measured_name(&src.trim()[4..])
}

/// Whether a field's source `src` gets a synthetic routine: any `Rec.<name>` of an extension (main), and a measured method on
/// the measured page.
pub(super) fn needs_routine(src: &str, extension: bool, measured_page: bool) -> bool {
    (extension && unquoted_rec_name(src)) || (measured_page && measured_source(src))
}

/// Whether TestPage.SetValue can write a control source through a `sourceset` routine: not a Record method.
pub(super) fn settable(expr: &str, measured_page: bool) -> bool {
    assignable_path(expr) && !(measured_page && measured_source(expr))
}
