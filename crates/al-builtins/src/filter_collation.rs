//! Where a filter's answer for a text value rests on an order or a match of letters no measurement covers (al-runner issues
//! 8195 and 8196, review of al-runner PR 8220): the letters `Æ` and `æ` in an order or an equality, accented letters under `@`.
//! A reader refuses such a value; every other value keeps the answer of [`FilterExpr::matches_checked`].

use crate::filter::{FieldFilter, FilterExpr, OR_GROUP, blank_if_spaces, like};
use crate::value::Value;

impl FilterExpr {
    /// Why the answer of this filter for the text value `v` rests on an order or a match of letters no measurement covers: an
    /// ordered comparison whose order rests on `Æ` or `æ` ([`crate::collation::ae_decides_order`]), an equality of two texts that
    /// differ only in these letters ([`crate::collation::ae_equality_open`]; a range with two equal ends is an equality), or an
    /// `@` filter whose answer changes when the accents are ignored ([`at_refusal`]). `normal_record`: the value is of a normal
    /// record, the only kind either measurement covers.
    pub fn collation_refusal(&self, v: &Value, normal_record: bool) -> Option<String> {
        let v = &*blank_if_spaces(v);
        let (Value::Text(t) | Value::Code(t)) = v else {
            return None;
        };
        let ordered = |x: &Value| {
            let (Value::Text(b) | Value::Code(b)) = x else {
                return None;
            };
            let both_text = matches!((v, x), (Value::Text(_), Value::Text(_)));
            crate::collation::ae_decides_order(t, b, both_text, normal_record).then(|| {
                format!(
                    "the order of '{}' and '{}' in a filter (the letters Æ and æ are measured only in two Text values of a normal record, the capital Æ)",
                    t.as_str(),
                    b.as_str()
                )
            })
        };
        let equal = |x: &Value| {
            let (Value::Text(b) | Value::Code(b)) = x else {
                return None;
            };
            crate::collation::ae_equality_open(t, b).then(|| {
                format!(
                    "whether '{}' and '{}' are equal in a filter (the equality of the letters Æ and æ with AE and ae is not measured)",
                    t.as_str(),
                    b.as_str()
                )
            })
        };
        match self {
            FilterExpr::Range(Some(lo), Some(hi)) if same_text(lo, hi) => equal(lo),
            FilterExpr::Range(lo, hi) => lo.iter().chain(hi.iter()).find_map(ordered),
            FilterExpr::Eq(x) | FilterExpr::RangeSame(x) | FilterExpr::Ne(x) => equal(x),
            FilterExpr::Lt(x) | FilterExpr::Le(x) | FilterExpr::Gt(x) | FilterExpr::Ge(x) => ordered(x),
            FilterExpr::Like(p, true) | FilterExpr::NotLike(p, true) => {
                at_refusal(t, p, matches!(v, Value::Text(_)), normal_record)
            }
            FilterExpr::Or(xs) | FilterExpr::And(xs) => xs.iter().find_map(|x| x.collation_refusal(v, normal_record)),
            FilterExpr::NumCode(inner) | FilterExpr::UnpinnedText(inner) => inner.collation_refusal(v, normal_record),
            _ => None,
        }
    }

    /// Whether [`FilterExpr::collation_refusal`] can refuse some value: an equality only when its value can tie with another text
    /// ([`crate::collation::ae_equality_reachable`]), an ordered comparison and an `@` filter always. A store checks no row for the
    /// other filters.
    pub fn collation_open(&self) -> bool {
        let equal =
            |x: &Value| matches!(x, Value::Text(b) | Value::Code(b) if crate::collation::ae_equality_reachable(b));
        match self {
            FilterExpr::Range(Some(lo), Some(hi)) if same_text(lo, hi) => equal(lo),
            FilterExpr::Eq(x) | FilterExpr::RangeSame(x) | FilterExpr::Ne(x) => equal(x),
            FilterExpr::Range(..)
            | FilterExpr::Lt(_)
            | FilterExpr::Le(_)
            | FilterExpr::Gt(_)
            | FilterExpr::Ge(_)
            | FilterExpr::Like(_, true)
            | FilterExpr::NotLike(_, true) => true,
            FilterExpr::Or(xs) | FilterExpr::And(xs) => xs.iter().any(FilterExpr::collation_open),
            FilterExpr::NumCode(inner) | FilterExpr::UnpinnedText(inner) => inner.collation_open(),
            _ => false,
        }
    }

    /// Whether a value or a pattern of this filter holds a letter outside ASCII: a store checks its rows with
    /// [`FilterExpr::collation_refusal`] then, whatever letters they hold.
    pub fn holds_non_ascii(&self) -> bool {
        let non_ascii = |x: &Value| matches!(x, Value::Text(s) | Value::Code(s) if !s.is_ascii());
        match self {
            FilterExpr::Range(lo, hi) => lo.iter().chain(hi.iter()).any(non_ascii),
            FilterExpr::Eq(x)
            | FilterExpr::RangeSame(x)
            | FilterExpr::Ne(x)
            | FilterExpr::Lt(x)
            | FilterExpr::Le(x)
            | FilterExpr::Gt(x)
            | FilterExpr::Ge(x) => non_ascii(x),
            FilterExpr::Like(p, _) | FilterExpr::NotLike(p, _) => !p.is_ascii(),
            FilterExpr::Or(xs) | FilterExpr::And(xs) => xs.iter().any(FilterExpr::holds_non_ascii),
            FilterExpr::NumCode(inner) | FilterExpr::UnpinnedText(inner) => inner.holds_non_ascii(),
            _ => false,
        }
    }

    /// Whether this filter holds an `@` comparison, the only kind other accents than `Æ` and `æ` can leave open.
    pub fn holds_at(&self) -> bool {
        match self {
            FilterExpr::Like(_, true) | FilterExpr::NotLike(_, true) => true,
            FilterExpr::Or(xs) | FilterExpr::And(xs) => xs.iter().any(FilterExpr::holds_at),
            FilterExpr::NumCode(inner) | FilterExpr::UnpinnedText(inner) => inner.holds_at(),
            _ => false,
        }
    }

    /// The text value an equality filter holds its field to: `Eq`, `RangeSame` and a range with two equal ends of a text.
    pub fn text_equality(&self) -> Option<&str> {
        match self {
            FilterExpr::Ne(_) => None,
            _ => self.tie_text(),
        }
    }

    /// The text value of an equality or of `<>`: the only texts whose answer [`FilterExpr::collation_refusal`] can leave open
    /// are those that tie with it ([`crate::collation::ae_equality_open`]).
    pub fn tie_text(&self) -> Option<&str> {
        match self {
            FilterExpr::Ne(Value::Text(x) | Value::Code(x)) => Some(x.as_str()),
            FilterExpr::Eq(Value::Text(x) | Value::Code(x))
            | FilterExpr::RangeSame(Value::Text(x) | Value::Code(x)) => Some(x.as_str()),
            FilterExpr::Range(Some(lo @ (Value::Text(x) | Value::Code(x))), Some(hi)) if same_text(lo, hi) => {
                Some(x.as_str())
            }
            _ => None,
        }
    }
}

/// Two text values of the same characters.
fn same_text(a: &Value, b: &Value) -> bool {
    matches!((a, b), (Value::Text(x) | Value::Code(x), Value::Text(y) | Value::Code(y)) if x.as_str() == y.as_str())
}

/// Why an `@` filter's answer for `text` is not measured: on a temporary record, a Text value with the capital `É` the
/// assumption `at-filter-e-matches-e-acute` reads (measured on a normal table, al-runner issue 8196); on any record, an
/// accented letter in the text or the pattern where the answer with the accents ignored differs (issue 8196: BC ignores
/// accents; only `É` in a Text value against `e` is measured).
fn at_refusal(text: &str, pattern: &str, text_value: bool, normal_record: bool) -> Option<String> {
    if text.is_ascii() && pattern.is_ascii() {
        return None;
    }
    if text_value && !normal_record && text.contains('É') {
        return Some(format!(
            "the filter '@{pattern}' on '{text}' of a temporary record (É under @ is measured on a normal record only)"
        ));
    }
    let folded = |s: &str| crate::collation::fold_accents(s);
    (like(text, pattern, true, text_value) != like(&folded(text), &folded(pattern), true, false))
        .then(|| format!("the filter '@{pattern}' on '{text}': whether @ ignores these accents is not measured"))
}

/// Whether `row` passes `filters`, every group as [`crate::filter::row_passes_checked`] reads them, or why that answer is not
/// measured. A row that a filter with a measured answer keeps out is out, whatever the other filters leave open.
pub fn row_passes_collation(row: &[Value], filters: &[FieldFilter], normal_record: bool) -> Result<bool, String> {
    let (mut or_seen, mut or_matched) = (false, false);
    let (mut open, mut or_open): (Option<String>, Option<String>) = (None, None);
    for f in filters {
        let why = f.expr.collation_refusal(&row[f.field], normal_record);
        let matched = why.is_none() && f.expr.matches_checked(&row[f.field]);
        if f.group == OR_GROUP {
            or_seen = true;
            or_matched |= matched;
            or_open = or_open.or(why);
        } else if why.is_some() {
            open = open.or(why);
        } else if !matched {
            return Ok(false);
        }
    }
    if or_seen && !or_matched && or_open.is_none() {
        return Ok(false);
    }
    match open.or(if or_matched { None } else { or_open }) {
        Some(why) => Err(why),
        None => Ok(true),
    }
}

/// Whether every filter matches its field of `row`, whatever its group, for a reader that knows whether the record is a normal
/// one: a filter whose matching is not pinned refuses; a row that a filter with a measured answer keeps out does not match;
/// else a filter whose answer is not measured ([`FilterExpr::collation_refusal`]) refuses.
pub fn all_match_on(
    filters: &[FieldFilter],
    row: &[Value],
    normal_record: bool,
) -> Result<bool, crate::native::Signal> {
    let mut open = None;
    for f in filters {
        if let Some(why) = f.expr.unpinned_match() {
            return Err(crate::native::Signal::unsupported(why));
        }
        let v = &row[f.field];
        match f.expr.collation_refusal(v, normal_record) {
            Some(why) => open = open.or(Some(why)),
            None if !f.expr.matches_checked(v) => return Ok(false),
            None => {}
        }
    }
    match open {
        Some(why) => Err(crate::native::Signal::unsupported(why)),
        None => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Value {
        Value::Text(s.to_string().into())
    }

    fn on(field: usize, expr: FilterExpr, group: i32) -> FieldFilter {
        FieldFilter {
            lang: crate::filter::filter_language(),
            field,
            expr,
            group,
        }
    }

    /// Review of al-runner PR 8220 round 2: an order rests on `Æ` or `æ` only where one stands against another letter at or before
    /// the place that decides it; texts that agree there keep main's order.
    #[test]
    fn an_order_rests_on_ae_only_where_it_decides() {
        use crate::collation::ae_decides_order as decides;
        assert!(decides("Bæk", "Bak", true, true));
        assert!(decides("ærø", "Bravo", true, true));
        assert!(!decides("Bæk 1", "Bæk 2", true, true));
        assert!(!decides("Bæk", "Alpha", true, true));
        assert!(!decides("Bæk", "Bæ", true, true), "a text and its start");
        assert!(!decides("Bæk-1", "Bæk2", true, true), "the hyphen does not count first");
        // The capital Æ of the measured shape, and the same on a temporary record or a Code value.
        assert!(!decides("Ærø", "Alpha", true, true));
        assert!(decides("Ærø", "Alpha", true, false));
        assert!(decides("ÆRØ", "ALPHA", false, true));
        // The capital Æ read as AE against a small æ: refused where they meet.
        assert!(decides("Ærø", "æble", true, true));
        assert!(!decides("Ærø", "Bæk", true, true));
        // Case of Æ against æ.
        assert!(decides("Æble", "æble", false, true));
        assert!(!decides("Alpha", "Bravo", true, false));
    }

    #[test]
    fn an_equality_is_open_only_where_the_texts_tie_without_ae() {
        use crate::collation::{ae_equality_open as open, ae_equality_reachable as reachable};
        assert!(open("Bæk", "Baek"));
        assert!(open("Ærø", "aerø"));
        assert!(!open("Bæk", "Bæk "));
        assert!(!open("Bæk", "Bak"));
        assert!(!open("Adatum", "adatum"), "no Æ or æ: main's exact equality");
        assert!(reachable("Baek") && reachable("Bæk") && !reachable("Adatum A/S"));
        let same = |v: &str| FilterExpr::Range(Some(t(v)), Some(t(v)));
        assert!(same("Baek").collation_open() && !same("Adatum A/S").collation_open());
        assert!(!FilterExpr::Like("a*".into(), false).collation_open());
        assert!(same("Baek").collation_refusal(&t("Bæk"), true).is_some());
        assert!(same("Bak").collation_refusal(&t("Bæk"), true).is_none());
    }

    /// A row that a measured filter keeps out is out; a row only an unmeasured answer could let in refuses; groups as
    /// `row_passes_checked` reads them.
    #[test]
    fn a_measured_filter_that_keeps_a_row_out_decides() {
        let range = FilterExpr::Range(Some(t("A")), Some(t("B")));
        let row = [t("ærø"), t("X")];
        assert_eq!(
            row_passes_collation(&row, &[on(0, range.clone(), 0), on(1, FilterExpr::Eq(t("Y")), 0)], true),
            Ok(false)
        );
        assert!(row_passes_collation(&row, &[on(0, range.clone(), 0)], true).is_err());
        assert_eq!(
            row_passes_collation(&[t("Alpha"), t("X")], &[on(0, range.clone(), 0)], true),
            Ok(true)
        );
        // OR group: one member that matches for certain is enough; else the open member refuses.
        let or = [on(0, range.clone(), OR_GROUP), on(1, FilterExpr::Eq(t("X")), OR_GROUP)];
        assert_eq!(row_passes_collation(&row, &or, true), Ok(true));
        let or = [on(0, range.clone(), OR_GROUP), on(1, FilterExpr::Eq(t("Y")), OR_GROUP)];
        assert!(row_passes_collation(&row, &or, true).is_err());
        // all_match_on, as a record's own filters read a loaded row (al-runner issue 8195: `Ærø` in A..B of a normal record).
        assert!(all_match_on(&[on(0, range.clone(), 0)], &[t("Ærø")], true).unwrap());
        assert!(all_match_on(&[on(0, range.clone(), 0)], &[t("Ærø")], false).is_err());
        assert!(!all_match_on(&[on(0, range, 0), on(1, FilterExpr::Eq(t("Y")), 0)], &row, false).unwrap());
    }
}
