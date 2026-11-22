//! `Record.SetFilter(Field, '<>%1', Value)` in the measured shape (`MeasuredSetFilter` with one Text argument) where the value
//! is a wildcard text with a blank at one end, or longer than the Code field (corpus PR 1503, open: record/filters/
//! PlaceholderWildcardSuffix.al, al-runner issue 8642; assumption `filter-placeholder-wildcard-edges`). The parser refuses these
//! values in every other caller and shape (`filter::Parser::not_pattern`).

use super::*;
use al_builtins::assumptions::{Assumption, assume};

/// The measured wildcard text: capital letters, digits and `-` (at least one), then one `*` (`XYZ-*`,
/// `XYZ-ABCDEFGHIJKLMNOPQRST*`).
fn measured_pattern(s: &str) -> bool {
    s.strip_suffix('*').is_some_and(|p| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
    })
}

/// The pattern of `s` when it is a measured pattern with one blank before or after it (`XYZ-* `, ` XYZ-*`).
fn with_edge_blank(s: &str) -> Option<&str> {
    [s.strip_suffix(' '), s.strip_prefix(' ')]
        .into_iter()
        .flatten()
        .find(|p| measured_pattern(p))
}

impl Session {
    /// The filter of the measured cells, before the parser; `Ok(None)`: the parser reads the text. GlobalLanguage 1033 (the
    /// probe's session, `Settings_OfTheSession`), field `pos` a Code[n] or Text[n]:
    /// - a measured pattern with one blank at one end, the whole value (blank included) of n characters or fewer (a longer one
    ///   is not measured: BC may raise the length error first): on a Code field the pattern without the blank
    ///   is the filter (`<>XYZ-*`, Code_NotEqualPlaceholder_StarSuffixValues `ne XYZ-* blank after`, `blank before`); on a Text
    ///   field the pattern keeps the blank, reads back in quotes (`<>'XYZ-* '`, views.rs) and keeps every measured row
    ///   (Text_NotEqualPlaceholder_StarSuffixValues; the row `XYZ-*` is kept, so it is no equality, which ignores trailing
    ///   blanks). A row with a blank at that end is not measured; a read meets it and refuses (`FilterExpr::compare_error`).
    /// - a measured pattern longer than the n of a Code field: the length error of the value (Code_OtherForms_StarSuffixValue
    ///   `ne long 21`, `ne long 25`, `ne long 35` on a Code[20]).
    pub(super) fn placeholder_wildcard_edge(
        &self,
        arg: &Value,
        t: &TableDef,
        pos: usize,
    ) -> Result<Option<FilterExpr>, Signal> {
        let Value::Text(v) = arg else {
            return Ok(None);
        };
        let (code, n) = match t.fields[pos].ty {
            RtType::Code(Some(n)) => (true, n),
            RtType::Text(Some(n)) => (false, n),
            _ => return Ok(None),
        };
        let s = v.as_str();
        let measured_session = self.global_language() == 1033;
        if let Some(p) = with_edge_blank(s).filter(|_| measured_session && s.len() <= n as usize) {
            assume(Assumption::FilterPlaceholderWildcardEdges);
            let pattern = if code { p } else { s };
            return Ok(Some(FilterExpr::NotLike(pattern.to_string(), false)));
        }
        if measured_session && code && measured_pattern(s) && s.len() > n as usize {
            assume(Assumption::FilterPlaceholderWildcardEdges);
            return al_builtins::ops::string_length_error(s, n);
        }
        Ok(None)
    }
}
