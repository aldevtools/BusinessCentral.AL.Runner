//! Record filters as data (shared by the VM and the store).

use crate::describe::Describe;
use crate::native::Signal;
use crate::ops::compare;
use crate::value::Value;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub enum FilterExpr {
    /// `SetRange(F, A, B)` and `SetRange(F, A)` (as A..A); open ends are None.
    Range(Option<Value>, Option<Value>),
    Eq(Value),
    /// A range typed as text whose two ends are the same value (`SetFilter(D, '010126..010126')`): it finds what `Eq` finds, and
    /// `GetFilter` writes it back as `a..a`, where `SetRange(D, a, a)` gives `a` (record/load-fields/
    /// P81DateFilters.al:SetFilter_Date_RangeForms `r15`, SetFilter_Date_Placeholders_TwoDates `range_same`).
    RangeSame(Value),
    Ne(Value),
    Lt(Value),
    Le(Value),
    Gt(Value),
    Ge(Value),
    Or(Vec<FilterExpr>),
    And(Vec<FilterExpr>),
    /// `*` / `?` wildcards on text; `@` makes it case-insensitive.
    Like(String, bool),
    /// `<>` before a wildcard pattern: the texts the pattern does not match. `<>*` keeps no row, not even the blank one
    /// (record/filters/WildcardGetFilter.al:106-173, Forms_CodeField and Forms_TextField).
    NotLike(String, bool),
    /// `@` alone keeps every row; `<>@` alone keeps none (`true`). Pinned on a Text field (record/filters/WcInvalidFilterExpr.al:
    /// 84-90 `@`, :93-111 `<>@`).
    AtSignAlone(bool),
    /// An empty value in a view's `1()` or `FILTER()`: BC keeps every row and writes the filter back as `==`
    /// (record/views/Y51SetViewShapes.al:141-154, SetView_EmptyFilterValue, on Text, Date and Integer fields).
    EmptyViewValue,
    /// Built by a store for a Code field the database stores as a number (`SQLDataType` Integer, BigInteger or Variant), never
    /// by the language: `inner` holds BigInteger bounds and is applied to the number the field's text is (a text that is no
    /// number stays text, which no bound matches).
    NumCode(Box<FilterExpr>),
    /// A filter that is set and reads back as `inner` does, but whose matching is not pinned (an unclosed quote after `@`, corpus
    /// master record/crud/RcAFilters.al:996-1027 pins only SetFilter and GetFilter). The text is the filter as given. A store
    /// refuses a read that selects rows through it ([`FilterExpr::unpinned_match`]).
    UnpinnedMatch(Box<FilterExpr>, String),
    /// A filter that selects rows as `inner` does, but whose text (GetFilter, GetFilters, a view) is not pinned: a wildcard text
    /// written as a literal value (`=A*B`, `>A*B`, `A*B..AXB`, `..A*`). Only the placeholder and SetRange forms of such a value
    /// read back pinned (record/filters/PlaceholderWildcardValue.al:249-262, :300-325, :413).
    UnpinnedText(Box<FilterExpr>),
}

/// Characters whose presence in a text value makes `GetFilter` quote the whole value (record/
/// TestSetFilterSpecialTextContracts.al, record/filters/W4Filters.al:174-191), and `..`.
pub fn needs_quotes(s: &str) -> bool {
    s.contains(['|', '&', '<', '>', '=', '(', ')']) || s.contains("..")
}

/// The number a Code text of plain digits is.
pub fn code_number(v: &Value) -> Option<i64> {
    match v {
        Value::Code(s) | Value::Text(s) => {
            let s = s.trim();
            if s.is_empty() {
                Some(0)
            } else if s.bytes().all(|c| c.is_ascii_digit()) {
                s.parse().ok()
            } else {
                None
            }
        }
        _ => None,
    }
}

impl FilterExpr {
    /// Whether a field value passes; a filter whose matching is not pinned ([`FilterExpr::UnpinnedMatch`]) refuses, whoever
    /// asks (hard rule 4).
    pub fn matches(&self, v: &Value) -> Result<bool, Signal> {
        if let Some(why) = self.unpinned_match() {
            return Err(Signal::unsupported(why));
        }
        Ok(self.matches_checked(v))
    }

    /// [`FilterExpr::matches`] for a caller that refused a filter with [`FilterExpr::unpinned_match`] first (a store refuses
    /// the read once, then checks each row). Values that cannot be compared do not pass.
    pub fn matches_checked(&self, v: &Value) -> bool {
        // A text of spaces only is blank to a filter: `SetRange(Txt, ' ')` finds the rows holding '', ' ' and '  ', and
        // `<> ' '` finds the others (record/filters/W4Filters.al:BlankOnlyFilterValues_MatchOnlyBlankRows, BC 27.5 and 29.0).
        // Other trailing spaces are not pinned.
        if let Value::Blob(b) = v {
            return crate::filter_blob::matches(self, b);
        }
        let v = &*blank_if_spaces(v);
        let cmp = |x: &Value| {
            let x = blank_if_spaces(x);
            crate::ops::hyphen_filter_order(v, &x).or_else(|| compare(v, &x).ok())
        };
        // A Text field's value against a bound of a range or a comparison: the collation order of the keys, letters without
        // regard to case first (record/filters/UcQuotedValues.al SetView_FilterGreaterB `view_txt_plain_ids`,
        // PageAndQuery_FilterGreaterB `page_txt_ids`: `>B` keeps neither a>b nor a>). Equality stays exact.
        let ord = |x: &Value| match (v, x) {
            (Value::Text(a), Value::Text(b) | Value::Code(b)) if !a.has_marker() && !b.has_marker() => {
                Some(crate::ops::text_key_order(a, b))
            }
            _ => cmp(x),
        };
        match self {
            FilterExpr::Range(lo, hi) => {
                lo.as_ref()
                    .is_none_or(|l| matches!(ord(l), Some(Ordering::Equal | Ordering::Greater)))
                    && hi
                        .as_ref()
                        .is_none_or(|h| matches!(ord(h), Some(Ordering::Equal | Ordering::Less)))
            }
            FilterExpr::Eq(x) | FilterExpr::RangeSame(x) => cmp(x) == Some(Ordering::Equal),
            FilterExpr::Ne(x) => matches!(cmp(x), Some(Ordering::Less | Ordering::Greater)),
            FilterExpr::Lt(x) => ord(x) == Some(Ordering::Less),
            FilterExpr::Le(x) => matches!(ord(x), Some(Ordering::Less | Ordering::Equal)),
            FilterExpr::Gt(x) => ord(x) == Some(Ordering::Greater),
            FilterExpr::Ge(x) => matches!(ord(x), Some(Ordering::Greater | Ordering::Equal)),
            FilterExpr::Like(p, ci) => crate::native::text_of(v).is_some_and(|t| like(&t, p, *ci)),
            FilterExpr::NotLike(p, ci) => crate::native::text_of(v).is_some_and(|t| !like(&t, p, *ci)),
            FilterExpr::AtSignAlone(negated) => !negated,
            FilterExpr::EmptyViewValue => true,
            FilterExpr::Or(xs) => xs.iter().any(|x| x.matches_checked(v)),
            FilterExpr::And(xs) => xs.iter().all(|x| x.matches_checked(v)),
            FilterExpr::NumCode(inner) => match code_number(v) {
                Some(n) => inner.matches_checked(&Value::BigInteger(n)),
                None => inner.matches_checked(v),
            },
            // Refused by `matches` and by a store before it asks (see `matches_checked`).
            FilterExpr::UnpinnedMatch(..) => false,
            FilterExpr::UnpinnedText(inner) => inner.matches_checked(v),
        }
    }

    /// The refusal of a read that selects rows through this filter, when its matching is not pinned.
    pub fn unpinned_match(&self) -> Option<String> {
        match self {
            FilterExpr::UnpinnedMatch(_, text) => Some(format!(
                "a read through the filter `{text}`, an unclosed quote after `@`: its matching not pinned"
            )),
            FilterExpr::Or(xs) | FilterExpr::And(xs) => xs.iter().find_map(FilterExpr::unpinned_match),
            FilterExpr::NumCode(inner) | FilterExpr::UnpinnedText(inner) => inner.unpinned_match(),
            _ => None,
        }
    }
}

impl FilterExpr {
    /// The first comparison of `v` with a value of the filter that cannot be answered. [`FilterExpr::matches`] counts such a
    /// value as not passing; a caller that must not guess (a TableRelation condition, hard rule 4) refuses on it instead.
    pub fn compare_error(&self, v: &Value) -> Option<Signal> {
        let v = &*blank_if_spaces(v);
        let cmp = |x: &Value| {
            let x = blank_if_spaces(x);
            if crate::ops::hyphen_filter_order(v, &x).is_some() {
                return None;
            }
            compare(v, &x).err()
        };
        match self {
            FilterExpr::Range(lo, hi) => lo.iter().chain(hi.iter()).find_map(cmp),
            FilterExpr::Eq(x)
            | FilterExpr::RangeSame(x)
            | FilterExpr::Ne(x)
            | FilterExpr::Lt(x)
            | FilterExpr::Le(x)
            | FilterExpr::Gt(x)
            | FilterExpr::Ge(x) => cmp(x),
            FilterExpr::Like(..) | FilterExpr::NotLike(..) => match v {
                Value::Unknown(why) => Some(Signal::unsupported(why.to_string())),
                Value::Text(t) | Value::Code(t) if t.has_marker() => Some(Signal::Unsupported(
                    "a wildcard filter on a text that is not known".into(),
                )),
                _ => None,
            },
            FilterExpr::AtSignAlone(_) | FilterExpr::EmptyViewValue => None,
            FilterExpr::UnpinnedMatch(..) => self.unpinned_match().map(Signal::unsupported),
            FilterExpr::Or(xs) | FilterExpr::And(xs) => xs.iter().find_map(|x| x.compare_error(v)),
            FilterExpr::UnpinnedText(inner) => inner.compare_error(v),
            FilterExpr::NumCode(inner) => match code_number(v) {
                Some(n) => inner.compare_error(&Value::BigInteger(n)),
                None => inner.compare_error(v),
            },
        }
    }
}

/// The value, or the blank text when it is spaces only; borrowed (no clone) in the usual case.
fn blank_if_spaces(v: &Value) -> std::borrow::Cow<'_, Value> {
    use std::borrow::Cow;
    match v {
        Value::Text(s) if !s.is_empty() && s.bytes().all(|c| c == b' ') => {
            Cow::Owned(Value::Text(String::new().into()))
        }
        Value::Code(s) if !s.is_empty() && s.bytes().all(|c| c == b' ') => {
            Cow::Owned(Value::Code(String::new().into()))
        }
        // Trailing blanks do not count in a filter comparison: `SetRange(K, 'abc  ')` finds the row 'abc' (record/keys-sorting/
        // TableKeyCaseTrailingBlank.al:P184_TextKey_Get_And_Filter_OtherCase_Blanks `range blanks count = 1`).
        Value::Text(s) if s.ends_with(' ') => Cow::Owned(Value::Text(s.trim_end_matches(' ').to_string().into())),
        Value::Code(s) if s.ends_with(' ') => Cow::Owned(Value::Code(s.trim_end_matches(' ').to_string().into())),
        other => Cow::Borrowed(other),
    }
}

fn like(text: &str, pattern: &str, ci: bool) -> bool {
    let (t, p): (Vec<char>, Vec<char>) = if ci {
        (
            text.to_lowercase().chars().collect(),
            pattern.to_lowercase().chars().collect(),
        )
    } else {
        (text.chars().collect(), pattern.chars().collect())
    };
    fn go(t: &[char], p: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => (0..=t.len()).any(|i| go(&t[i..], &p[1..])),
            Some('?') => !t.is_empty() && go(&t[1..], &p[1..]),
            Some(c) => t.first() == Some(c) && go(&t[1..], &p[1..]),
        }
    }
    go(&t, &p)
}

/// Parses a `SetFilter` expression for a field of type `ty`. `%1`.. take the
/// typed `args`. Forms the corpus does not exercise yet return an error.
pub fn parse_filter(text: &str, args: &[crate::value::Value], ty: &crate::value::RtType) -> Result<FilterExpr, String> {
    parse_filter_with(text, args, ty, &|_| Ok(Vec::new()))
}

/// Like [`parse_filter`], with the (ordinal, name) members of an enum object
/// so that filters on enum fields can name members.
/// Prefix of the parse error for a word a Boolean filter does not accept; the rest is the word.
pub const INVALID_BOOLEAN: &str = "invalid Boolean filter value: ";
/// The prefix of the error for a text no Guid filter reads (types/date-time/P143Guid_Filters.al:P143_GuidInvalid).
pub const INVALID_GUID: &str = "invalid Guid filter value: ";
/// The prefix of the error for a date text no Date filter reads; the failing value follows (types/date-time/
/// P143Date_DePlain.al: `Das Format Date „17.01.27“ ...`).
pub const INVALID_DATE: &str = "invalid Date filter value: ";
/// The prefix of the error for a Date filter text with an empty side; the English message follows (record/load-fields/
/// P81DateFilters.al:SetFilter_Date_RangeForms `..`, SetFilter_Date_InvalidTexts `|`, `&`, `||`).
pub const FILTER_SYNTAX: &str = "invalid Date filter syntax: ";
/// Follows [`FILTER_SYNTAX`] in the errors of an Option filter: the filter text is shown as written, blanks kept
/// (FMOptionBlankField.al `flt_neg`); the Date errors keep the trimmed text.
pub const AS_WRITTEN: &str = "\u{1}";

/// The prefix of the error for a filter value BC cannot evaluate into the field's type: the type's name, a tab, the value
/// (`The value "abc" can't be evaluated into type Integer.`, record/crud/RcAFilters.al:SetFilter_Integer_Invalid,
/// events/platform-triggers/PtObserve.al:Bad_Int, Bad_Decimal, Bad_Time).
pub const NOT_EVALUATED: &str = "filter value not evaluated: ";

/// The prefix of the error for a number filter that starts with a placeholder kept as written; the filter text follows
/// (`The variable "%1*" was not understood.`, record/filters/W4Filters.al:694 `num_star_after`).
pub const VARIABLE_NOT_UNDERSTOOD: &str = "filter variable not understood: ";

/// The parse error of a Text filter with an unclosed `(` (record/filters/WcInvalidFilterExpr.al:66-90 `((*`, `(`, `((x`, `(c-a`).
const MISSING_PARENTHESIS: &str = "A ')' is missing.";

/// The parse error of a Text filter with `)` or `<>` where it cannot stand (WcInvalidFilterExpr.al:71-87 `c-a)`, `@<>C-A`, `@<>`).
fn did_not_expect(what: &str) -> String {
    format!(
        "{FILTER_SYNTAX}An error occurred while interpreting the filter: Did not expect '{what}'.\\\\For more information, see https://go.microsoft.com/fwlink/?linkid=2158058."
    )
}

/// The upper case of a Code filter value. Under Turkish or Azerbaijani a value with `i` is not pinned (only an assignment to a Code
/// is, text/text-methods/CaseConv.Codeunit.al:280-310) and is refused.
fn filter_upper(s: &str) -> Result<String, String> {
    if crate::ops::turkic_code() && s.contains('i') {
        return Err("a Code filter value with `i` under Turkish or Azerbaijani (not pinned)".into());
    }
    Ok(s.to_uppercase())
}

/// A Duration filter value (al-runner issue 7388; recordref-fieldref/fieldref/TestFieldRefFilter.al FieldRef_SetFilter_Zero_Duration,
/// _DurationTexts, corpus PR 1248): a whole number, with blanks around it and a minus, counts hours as Evaluate reads it (`0`,
/// ` 0 `, `1000` is 41 days 16 hours, `-1` is -1 hour); a number with `S` is seconds and with `M` minutes (`1S`, `0S`, `1M`);
/// a number, a blank and a unit word is what Evaluate reads (`1 minute`). The milliseconds, or `None` for another text.
fn duration_filter_value(t: &str) -> Option<i64> {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let n = |s: &str| s.parse::<i64>().ok();
    if digits(t.strip_prefix('-').unwrap_or(t)) {
        return n(t)?.checked_mul(3_600_000);
    }
    if let Some(s) = t.strip_suffix('S').filter(|s| digits(s)) {
        return n(s)?.checked_mul(1000);
    }
    if let Some(m) = t.strip_suffix('M').filter(|m| digits(m)) {
        return n(m)?.checked_mul(60_000);
    }
    let (num, word) = t.split_once(' ')?;
    let per = match word {
        "minute" | "minutes" => 60_000,
        "second" | "seconds" => 1000,
        "hour" | "hours" => 3_600_000,
        "day" | "days" => 86_400_000,
        _ => return None,
    };
    if !digits(num) {
        return None;
    }
    n(num)?.checked_mul(per)
}

fn not_evaluated(ty: &str, text: &str) -> String {
    format!("{NOT_EVALUATED}{ty}\t{text}")
}

/// The value text of an error `not_evaluated` made, `None` for another error.
pub fn not_evaluated_value(e: &str) -> Option<&str> {
    e.strip_prefix(NOT_EVALUATED)?.split_once('\t').map(|(_, v)| v)
}

/// Whether `text` is one word of ASCII letters, the only text a Decimal, Time or DateTime filter is pinned to refuse with the
/// evaluate error (PtObserve.al: `abc`).
pub fn letters_only(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphabetic())
}

/// Whether `text` is one word of ASCII letters and hyphens (`NOT-A-NUMBER`, `-`, `abc`): the texts a Decimal, BigInteger, Duration
/// and Time filter are pinned to refuse with the evaluate error (record/filters/InvalidDecimalFilter.al:71, :96,
/// :110, :134-137).
pub fn letters_or_hyphens(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphabetic() || c == '-')
}

/// Whether `text` is `n` groups of ASCII digits joined by `sep`, each group not empty.
fn digit_groups(text: &str, sep: char, n: usize) -> bool {
    let parts: Vec<&str> = text.split(sep).collect();
    parts.len() == n
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// How a Decimal filter value reads: `Some(Ok)` the value, `Some(Err)` the evaluate error, `None` a text no corpus test pins.
/// record/filters/InvalidDecimalFilter.al:92-126 (Decimal_Forms): letters and hyphens (`abc`, `x`, `-`,
/// `NOT-A-NUMBER`), `*`, `1;5` and `1.2.3` raise the error; `1,5` reads as 15 (the comma is dropped) and `1e3` as 1000.
fn decimal_filter_text(text: &str) -> Option<Result<rust_decimal::Decimal, ()>> {
    if let Ok(d) = text.parse() {
        return Some(Ok(d));
    }
    if letters_or_hyphens(text)
        || text == "*"
        || digit_groups(text, ';', 2)
        || digit_groups(text, '.', 3)
        || spaced_number(text)
    {
        return Some(Err(()));
    }
    if digit_groups(text, ',', 2) {
        return text.replace(',', "").parse().ok().map(Ok);
    }
    // A thousands comma before a fraction: `1,234.5` is 1234.5 (events/table-events/BdFilterEvents.al:BdFilter_Decimal_8).
    if let Some((int, frac)) = text.split_once('.') {
        let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
        if let Some((th, rest)) = int.split_once(',') {
            if digits(th) && th.len() <= 3 && rest.len() == 3 && digits(rest) && digits(frac) {
                return text.replace(',', "").parse().ok().map(Ok);
            }
        }
    }
    // One exponent digit, the pinned `1e3`; a larger exponent is not pinned.
    if digit_groups(text, 'e', 2) && text.split('e').nth(1).is_some_and(|e| e.len() == 1) {
        return rust_decimal::Decimal::from_scientific(text).ok().map(Ok);
    }
    None
}

/// `12:mm:ss`: the only Time text a filter is pinned to read (PtObserve.al:Bad_Time, `12:00:00`).
fn is_noon_text(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 8
        && t.starts_with("12:")
        && b[5] == b':'
        && t[3..5].parse::<u32>().is_ok_and(|m| m < 60)
        && t[6..8].parse::<u32>().is_ok_and(|s| s < 60)
        && t[3..5].bytes().chain(t[6..8].bytes()).all(|c| c.is_ascii_digit())
}

/// Two digit groups split by one blank (`1 000`, `1 2`) or a minus sign, a blank and digits (`- 1`): texts an Integer or
/// Decimal filter is pinned to refuse with the evaluate error (events/table-events/BdFilterEvents.al:BdFilter_Int_4,
/// BdFilter_Decimal_6, BdFilter_Decimal_7). Other texts with blank space in them are not pinned.
pub fn spaced_number(text: &str) -> bool {
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    match text.split_once(' ') {
        Some((a, b)) => (digits(a) || a == "-") && digits(b),
        None => false,
    }
}

/// The Time a filter text names in an English session, for the forms the corpus pins (events/table-events/
/// BdFilterEvents.al:BdFilter_Time_1 to _10): an hour of one digit (`1`), a digit, a dot and a digit for the hour and the
/// minutes (`1.5` is 1:05), `H:MM` (`9:00`), `HH:MM:SS` (`13:00:00`, `09:05:07`), `12:MM:SS PM`, four digits for the hour and the
/// minutes (`1300`) and six with the seconds (`120000`). `None` for any other text, and for an hour, minute or second out of range.
fn time_filter_text(t: &str) -> Option<Value> {
    let ctx = DATE_CONTEXT.with(std::cell::Cell::get)?;
    if ctx.language != 1033 {
        return None;
    }
    let num = |p: &str, len: usize| -> Option<i64> {
        (p.len() == len && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse().ok())
            .flatten()
    };
    let (h, m, s) = if let Some(rest) = t.strip_suffix(" PM") {
        // Only noon is pinned with PM (BdFilter_Time_7).
        match rest.split(':').collect::<Vec<_>>()[..] {
            ["12", m, s] => (12, num(m, 2)?, num(s, 2)?),
            _ => return None,
        }
    } else if t.contains(':') {
        match t.split(':').collect::<Vec<_>>()[..] {
            [h, m] => (num(h, 1)?, num(m, 2)?, 0),
            [h, m, s] => (num(h, 2)?, num(m, 2)?, num(s, 2)?),
            _ => return None,
        }
    } else if let Some((h, m)) = t.split_once('.') {
        (num(h, 1)?, num(m, 1)?, 0)
    } else {
        match t.len() {
            // `0` is not pinned: it may name the undefined time.
            1 if t != "0" => (num(t, 1)?, 0, 0),
            4 => (num(&t[0..2], 2)?, num(&t[2..4], 2)?, 0),
            6 => (num(&t[0..2], 2)?, num(&t[2..4], 2)?, num(&t[4..6], 2)?),
            _ => return None,
        }
    };
    (h < 24 && m < 60 && s < 60).then_some(Value::Time(((h * 60 + m) * 60 + s) * 1000 + 1))
}

/// The DateTime a filter text names in an English session, for the forms the corpus pins (types/P136Filter.al:
/// 326-380 on corpus master, TV_SetView_DateTime_GetViewRoundTrip and TV_SetView_DateTime_LiteralTexts): `MM/DD/YY` and a clock of
/// `HH:MM`, `HH:MM:SS` or `HH:MM:SS.f` with one to three digits as tenths, hundredths or milliseconds (`.8`, `.08`, `.033`), then
/// ` PM` or ` AM` with the hour 12 (`12:00 AM` is midnight, from GetView of a range), or no suffix with the hour 12
/// (`12:00:00.033` is noon). The value is the clock of that day as the session reads it (Evaluate also reads `m/d/y h:mm` as
/// UTC). `None` for any other text: a date alone (`01/15/24` keeps the row at noon of that day, so it is no point at midnight; how
/// BC reads it is not pinned), another hour with AM or PM, another date spelling, another language.
fn datetime_filter_text(t: &str) -> Option<Value> {
    let ctx = DATE_CONTEXT.with(std::cell::Cell::get)?;
    if ctx.language != 1033 {
        return None;
    }
    let num = |p: &str, len: usize| -> Option<i64> {
        (p.len() == len && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse().ok())
            .flatten()
    };
    // The form a view writes, `yyyy-mm-ddThh:mm:ss[.f]Z` in UTC, reads back (P136Filter.al TV_SetView_DateTime_GetViewRoundTrip
    // `same view=Yes`, with the view text of UN_DateTimeFilter_* types/P136Filter.al:447-586).
    if let Some((d, c)) = t.strip_suffix('Z').and_then(|r| r.split_once('T')) {
        let p: Vec<&str> = d.split('-').collect();
        if !(digit_groups(d, '-', 3) && p[0].len() == 4 && p[1].len() == 2 && p[2].len() == 2) {
            return None;
        }
        let (y, m, dd) = (num(p[0], 4)?, num(p[1], 2)?, num(p[2], 2)?);
        if !(1..=12).contains(&m) || dd < 1 || dd > i64::from(crate::datefns::days_in_month(y as i32, m as u32)) {
            return None;
        }
        let day = crate::format::day_from_civil(y as i32, m as u32, dd as u32);
        let (hms, frac) = match c.split_once('.') {
            Some((h, f)) if (1..=3).contains(&f.len()) => (h, Some(f)),
            Some(_) => return None,
            None => (c, None),
        };
        let (h, mi, sec) = match hms.split(':').collect::<Vec<_>>()[..] {
            [h, mi, sec] => (num(h, 2)?, num(mi, 2)?, num(sec, 2)?),
            _ => return None,
        };
        let ms = match frac {
            Some(f) => num(f, f.len())? * 10_i64.pow(3 - f.len() as u32),
            None => 0,
        };
        return (h < 24 && mi < 60 && sec < 60)
            .then(|| Value::DateTime((i64::from(day) - 1) * 86_400_000 + ((h * 60 + mi) * 60 + sec) * 1000 + ms + 1));
    }
    let parts: Vec<&str> = t.split(' ').collect();
    let (date, clock, suffix) = match parts[..] {
        [d, c] => (d, c, None),
        [d, c, s @ ("AM" | "PM")] => (d, c, Some(s)),
        _ => return None,
    };
    if !digit_groups(date, '/', 3) || date.split('/').any(|p| p.len() != 2) {
        return None;
    }
    let Some(Value::Date(day, false)) = crate::evaluate::read_text(&crate::value::RtType::Date, date) else {
        return None;
    };
    if day <= 0 {
        return None;
    }
    let (hms, frac) = match clock.split_once('.') {
        Some((c, f)) if (1..=3).contains(&f.len()) => (c, Some(f)),
        Some(_) => return None,
        None => (clock, None),
    };
    let (h, m, s) = match hms.split(':').collect::<Vec<_>>()[..] {
        [h, m] if frac.is_none() => (num(h, 2)?, num(m, 2)?, 0),
        [h, m, s] => (num(h, 2)?, num(m, 2)?, num(s, 2)?),
        _ => return None,
    };
    let ms = match frac {
        Some(f) => num(f, f.len())? * 10_i64.pow(3 - f.len() as u32),
        None => 0,
    };
    // Other hours (types/P136Filter.al:497-586, UN_DateTimeFilter_HourNotTwelve*, _MoreHourForms*): `08:30 AM`, `08:30 PM`, `05:45:10.5 PM`, and without a suffix a
    // 24-hour clock (`08:30`, `17:45`, `00:00`). An hour past 12 with AM or PM is an error whose filter texts are not built here.
    let h = match suffix {
        Some("PM") if h == 12 => 12,
        Some("AM") if h == 12 => 0,
        Some("PM") if (1..12).contains(&h) => h + 12,
        Some("AM") if (1..12).contains(&h) => h,
        Some(_) => return None,
        None if h == 12 => 12,
        None if h < 24 => h,
        None => return None,
    };
    (m < 60 && s < 60)
        .then(|| Value::DateTime((i64::from(day) - 1) * 86_400_000 + ((h * 60 + m) * 60 + s) * 1000 + ms + 1))
}

/// The day a DateTime filter text names when it is a date alone, in an English session: `MM/DD/YY`, `M/DD/YY` or `YYYY-MM-DD`
/// (P136Filter.al UN_DateTimeFilter_DateAlone* types/P136Filter.al:447-586). A date alone stands for the
/// whole day, from 00:00:00 to 23:59:59 (a row at 23:59:59.999 is not matched). `None` for any other text.
fn datetime_filter_day(t: &str) -> Option<i32> {
    let ctx = DATE_CONTEXT.with(std::cell::Cell::get)?;
    if ctx.language != 1033 {
        return None;
    }
    let p: Vec<&str> = t.split(['/', '-']).collect();
    let shape = if t.contains('/') {
        digit_groups(t, '/', 3) && (1..=2).contains(&p[0].len()) && p[1].len() == 2 && p[2].len() == 2
    } else {
        digit_groups(t, '-', 3) && p[0].len() == 4 && p[1].len() == 2 && p[2].len() == 2
    };
    if !shape {
        return None;
    }
    let text = if t.contains('-') {
        format!("{}/{}/{}", p[1], p[2], &p[0][2..])
    } else {
        t.to_string()
    };
    if t.contains('-') && !p[0].starts_with("20") {
        return None;
    }
    match crate::evaluate::read_text(&crate::value::RtType::Date, &text) {
        Some(Value::Date(day, false)) if day > 0 => Some(day),
        _ => None,
    }
}

/// `DD.MM.YY` alone or with ` HH:MM`, in an English session.
fn dotted_datetime_text(t: &str) -> bool {
    if DATE_CONTEXT
        .with(std::cell::Cell::get)
        .is_none_or(|c| c.language != 1033)
    {
        return false;
    }
    let (date, clock) = match t.split_once(' ') {
        Some((d, c)) => (d, Some(c)),
        None => (t, None),
    };
    digit_groups(date, '.', 3)
        && date.split('.').all(|p| p.len() == 2)
        && clock.is_none_or(|c| digit_groups(c, ':', 2) && c.split(':').all(|p| p.len() == 2))
}

/// Whether the Date filter context is an English session.
fn english_session() -> bool {
    DATE_CONTEXT
        .with(std::cell::Cell::get)
        .is_some_and(|c| c.language == 1033)
}

/// The first and the last instant of `day` a date-alone DateTime filter covers.
fn day_start(day: i32) -> Value {
    Value::DateTime((i64::from(day) - 1) * 86_400_000 + 1)
}
fn day_end(day: i32) -> Value {
    Value::DateTime((i64::from(day) - 1) * 86_400_000 + 86_399_000 + 1)
}

/// What the text of a Date filter needs from the session: today, the work date and the language of the messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct DateFilterContext {
    pub today: i32,
    pub work_date: i32,
    pub language: i32,
    /// The fiscal year of the work date, read from the Accounting Period table when the filter text holds a `Y` (the session
    /// fills it; `None`: not looked up or not decidable from the rows the runtime holds).
    pub fiscal_year: Option<(i32, i32)>,
}

/// The last day of the demo data the corpus company holds in Accounting Period (36 months from 2025-01-01, ADR-047 rejected
/// seeding them): a fiscal year that starts after it is decided by the rows the runtime holds alone.
pub const ACCOUNTING_PERIOD_DEMO_END: (i32, u32, u32) = (2027, 12, 31);

/// The fiscal year `Y` stands for, from the Accounting Period rows: `rows` are the starting dates of the rows with New Fiscal
/// Year. The year runs from the latest such row on or before `work_date` to the day before the next one, or to 9999-12-31 after
/// the last (types/Y81DateFilterLetters.al:126 Y81_Y_FiscalYearOfWorkDate; needs corpus bump). `None` when the
/// answer could depend on the demo rows, or when no row is on or before the work date (BC raises an error whose text depends on
/// the demo data).
pub fn fiscal_year_of(rows: &[i32], work_date: i32) -> Option<(i32, i32)> {
    let start = rows.iter().copied().filter(|&d| d <= work_date).max()?;
    let (dy, dm, dd) = ACCOUNTING_PERIOD_DEMO_END;
    if start <= crate::format::day_from_civil(dy, dm, dd) {
        return None;
    }
    let end = rows
        .iter()
        .copied()
        .filter(|&d| d > work_date)
        .min()
        .map_or_else(|| crate::format::day_from_civil(9999, 12, 31), |next| next - 1);
    Some((start, end))
}

/// The Tuesday of ISO week `week` (1 to 7) of the ISO year of `work_date`, which `T-<week>D` stands for in a Date filter; `T-D`
/// (`week` 0) is the Tuesday of the week of the work date itself (types/Y81DateFilterLetters.al:274
/// Y81_Letters_TFamily; needs corpus bump).
fn tuesday_of_iso_week(work_date: i32, week: i32) -> i32 {
    // 2024-01-01 was a Monday.
    let monday_of = |d: i32| d - (d - crate::format::day_from_civil(2024, 1, 1)).rem_euclid(7);
    let this_monday = monday_of(work_date);
    if week == 0 {
        return this_monday + 1;
    }
    let iso_year = crate::format::civil_from_day(this_monday + 3).0;
    monday_of(crate::format::day_from_civil(iso_year, 1, 4)) + (week - 1) * 7 + 1
}

thread_local! {
    static DATE_CONTEXT: std::cell::Cell<Option<DateFilterContext>> = const { std::cell::Cell::new(None) };
}

/// Runs `f` with the date context of the session in force for the Date filters it parses.
pub fn with_date_context<R>(ctx: DateFilterContext, f: impl FnOnce() -> R) -> R {
    let old = DATE_CONTEXT.with(|c| c.replace(Some(ctx)));
    let r = f();
    DATE_CONTEXT.with(|c| c.set(old));
    r
}

pub fn parse_filter_with(
    text: &str,
    args: &[crate::value::Value],
    ty: &crate::value::RtType,
    enums: &dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
) -> Result<FilterExpr, String> {
    if matches!(ty, crate::value::RtType::Date) {
        if let Some(e) = date_letter_filter(text.trim()) {
            return e;
        }
    }
    // `@` (case-insensitive) makes no sense for a Guid: `@{1A2B...}` raises the Guid format error (record/filters/
    // Y77GuidFilter.al Record_Shapes5To6 `shape5`; pinned for a braced Guid after the `@`).
    if matches!(ty, crate::value::RtType::Guid)
        && text.trim().strip_prefix('@').is_some_and(|rest| {
            rest.starts_with('{') && crate::evaluate::read_text(&crate::value::RtType::Guid, rest.trim()).is_some()
        })
    {
        return Err(format!("{INVALID_GUID}{text}"));
    }
    // A Blob field: see `filter_blob`. With placeholders the text falls through and is refused.
    if crate::filter_blob::is_blob(ty) && args.is_empty() && !text.trim().is_empty() {
        return crate::filter_blob::parse(text);
    }
    // A DateFormula field takes no formula in angle brackets (`<1M>`, `<-CM+2D>`: the lexer does not expect the closing `>`), a text
    // of letters with one that is no unit needs a time unit, and `*M` needs a number (types/P136Filter.al:241-258
    // `oneMAngles`, `oneDAngles`, `garbage`, `wildcardM`; record/filters/Y77GuidFilter.al DateFormula_TextFilter_*). The
    // formulas that are valid are refused below.
    if matches!(ty, crate::value::RtType::DateFormula) {
        let t = text.trim();
        let formula_chars = |s: &str| s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'-');
        if t.len() > 2 && t.starts_with('<') && t.ends_with('>') && formula_chars(&t[1..t.len() - 1]) {
            return Err(did_not_expect(">"));
        }
        if !t.is_empty()
            && t.bytes().all(|b| b.is_ascii_alphabetic())
            && t.bytes().any(|b| !b"CDWMQYcdwmqy".contains(&b))
        {
            return Err(format!(
                "{FILTER_SYNTAX}The date formula '{}' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit.",
                t.to_uppercase()
            ));
        }
        if t == "*M" {
            return Err(format!("{FILTER_SYNTAX}The date formula '*M' should include a number."));
        }
    }
    // Three dots in a Date filter read as two (P81DateFilters.al:SetFilter_Date_InvalidTexts, `010126...020126`).
    let dots;
    let text = if matches!(ty, crate::value::RtType::Date)
        && DATE_CONTEXT
            .with(std::cell::Cell::get)
            .is_some_and(|c| c.language == 1033)
        && text.contains("...")
        && !text.contains('\'')
        && !text.contains("....")
    {
        dots = text.replace("...", "..");
        dots.as_str()
    } else {
        text
    };
    // `<>` as the whole filter on a Code or Text field raises the filter error (recordref-fieldref/fieldref/
    // FieldRefFilterMatrix.al:1140 `bare.Code`, :1310 `bare.Text`). With blank space around it the filter is not pinned.
    if matches!(ty, crate::value::RtType::Text(_) | crate::value::RtType::Code(_)) && text.trim() == "<>" {
        return Err(if text == "<>" {
            format!("{FILTER_SYNTAX}A value is expected after the '<>'.")
        } else {
            "`<>` with blank space around it as the whole filter (not pinned)".into()
        });
    }
    let mut kept_placeholder = false;
    let mut at_placeholder = false;
    let mut missing_placeholder = None;
    let mut unclosed_after_at = false;
    // The text `?` alone in a placeholder is the literal text only where the corpus pins it: `%1`, `=%1`, `<>%1`, `>%1` and
    // `<=%1` as the whole filter on a Text or Code field (record/filters/PlaceholderWildcardValue.al:225, :242, :259, :276,
    // :293, :434-502; FieldRef :650). Elsewhere it stays a wildcard, as before.
    let question_literal = matches!(ty, crate::value::RtType::Text(_) | crate::value::RtType::Code(_))
        && matches!(text, "%1" | "=%1" | "<>%1" | ">%1" | "<=%1");
    let toks = tokenize(
        text,
        args,
        question_literal,
        &mut kept_placeholder,
        &mut at_placeholder,
        &mut missing_placeholder,
        &mut unclosed_after_at,
    )?;
    // A placeholder without an argument: on a Code and a Text field it is the text `%n` (`<>%1` keeps the blank row and leaves out
    // the row `%1`; record/filters/PlaceholderWithoutArgument.al:95-132, :155-168); on an Integer and a Decimal field BC raises
    // `The variable "%1" was not understood.` (record/crud/RcAFilters.al:446,464; PlaceholderWithoutArgument.al:135-145). Other
    // types are not pinned (on a Date field it reads back as a date of the session, :8-9).
    if let Some(var) = missing_placeholder {
        match ty {
            crate::value::RtType::Text(_) | crate::value::RtType::Code(_) => {}
            crate::value::RtType::Integer | crate::value::RtType::Decimal => {
                return Err(format!("{VARIABLE_NOT_UNDERSTOOD}{var}"));
            }
            _ => return Err("filter placeholder without an argument".into()),
        }
    }
    if at_placeholder && !matches!(ty, crate::value::RtType::Text(_)) {
        return Err(format!(
            "`@` right before a placeholder on a {} field (only Text is pinned)",
            ty.describe()
        ));
    }
    // A placeholder kept as written next to a wildcard (see `tokenize`) is pinned on a Text, a Code and an Integer field only
    // (record/filters/W4Filters.al:661-695): on an Integer field `%1*` raises `The variable "%1*" was not understood.` and `*%1` the
    // evaluate error.
    if kept_placeholder {
        match ty {
            crate::value::RtType::Text(_) | crate::value::RtType::Code(_) => {}
            crate::value::RtType::Integer => {
                let t = text.trim();
                return Err(if t.starts_with('%') {
                    format!("{VARIABLE_NOT_UNDERSTOOD}{t}")
                } else {
                    not_evaluated("Integer", t)
                });
            }
            other => {
                return Err(format!(
                    "placeholder next to a wildcard on a {} field (only Text, Code and Integer are pinned)",
                    other.describe()
                ));
            }
        }
    }
    let mut p = Parser {
        toks,
        i: 0,
        written_wildcard: false,
        ty,
        enums,
    };
    let mut e = p.or()?;
    if p.written_wildcard {
        e = FilterExpr::UnpinnedText(Box::new(e));
    }
    if p.i != p.toks.len() {
        if matches!(ty, crate::value::RtType::Text(_)) && matches!(p.toks[p.i], Tok::Op(")")) {
            return Err(did_not_expect(")"));
        }
        return Err(format!("unexpected {} in filter `{text}`", p.toks[p.i].describe()));
    }
    // Which rows a filter with an unclosed quote after `@` keeps is not pinned (see `unclosed_quote`): the filter is set and reads
    // back, a read that selects rows through it is refused.
    if unclosed_after_at {
        return Ok(FilterExpr::UnpinnedMatch(Box::new(e), text.trim().to_string()));
    }
    if media_guid_order(ty, &e) {
        return Err(
            "a range or `<`, `>` filter of a Guid on a Media or MediaSet field (the order of Guids is not pinned)"
                .into(),
        );
    }
    Ok(e)
}

/// Whether `e` on a Media or MediaSet field orders Guids: a range or `<`, `<=`, `>`, `>=`. BC orders them otherwise than by
/// their text: on a MediaSet field the range `{11F319D9-...-6E725A266949}..{F1F319D9-...-6E725A266949}` keeps no row that
/// holds a new media id (recordref-fieldref/fieldref/FieldRefFilterMediaGuidText.al:244, :383), which the text order would keep
/// most of the time, and `<` and `>` against such an id change from run to run (`gt` and `lt` are ignored there). That order is
/// not pinned, so it is refused on both types until it is probed (owner decision 2026-11-11; Record.SetRange with two values
/// is refused for the same reason, al-vm records/op_inner.rs).
pub fn media_guid_order(ty: &crate::value::RtType, e: &FilterExpr) -> bool {
    use crate::library_kind::LibraryKind as K;
    if !matches!(ty, crate::value::RtType::Library(K::Media | K::MediaSet)) {
        return false;
    }
    fn orders(e: &FilterExpr) -> bool {
        match e {
            FilterExpr::Range(a, b) => [a, b].iter().any(|v| matches!(v, Some(Value::Guid(_)))),
            FilterExpr::Lt(v) | FilterExpr::Le(v) | FilterExpr::Gt(v) | FilterExpr::Ge(v) => {
                matches!(v, Value::Guid(_))
            }
            FilterExpr::Or(xs) | FilterExpr::And(xs) => xs.iter().any(orders),
            _ => false,
        }
    }
    orders(e)
}

#[derive(Debug, Clone)]
enum Tok {
    /// Unquoted text (may hold wildcards).
    Raw(String),
    /// Quoted expression: no filter operators inside (`@` and wildcards stay active).
    Quoted(String),
    /// A `%n` placeholder's typed value.
    Typed(Value),
    /// The text of a `%n` placeholder that holds a text, put in as unquoted text (it may hold wildcards).
    Arg(String),
    Op(&'static str),
}

impl Describe for Tok {
    fn describe(&self) -> String {
        match self {
            Tok::Raw(s) | Tok::Quoted(s) | Tok::Arg(s) => format!("`{s}`"),
            Tok::Typed(v) => v.describe(),
            Tok::Op(o) => format!("`{o}`"),
        }
    }
}

/// Whether `text` has a `%n` placeholder right before or after a wildcard. A record's SetFilter keeps it as written (see `tokenize`);
/// elsewhere (a Query column) that is not pinned.
pub fn placeholder_next_to_wildcard(text: &str) -> bool {
    let c: Vec<char> = text.chars().collect();
    (0..c.len().saturating_sub(1)).any(|i| {
        c[i] == '%'
            && c[i + 1].is_ascii_digit()
            && ((i > 0 && matches!(c[i - 1], '*' | '?')) || matches!(c.get(i + 2), Some('*' | '?')))
    })
}

/// A quote that opens a literal and is not closed (`rest` starts at the quote). Only a quote that starts the filter raises
/// "An single quote (') is missing." (`'`, `'abc`, `'''`); after `@` alone the filter is accepted as written (`@'`, `@'abc`,
/// `@'''`) (corpus master record/crud/RcAFilters.al:996-1027, SetFilter_Text_UnterminatedQuote_ErrorsOnlyWhenTheQuoteStartsTheFilter).
/// Returns the text that is the value in that case; `None` when the quote is closed. Anywhere else it is not pinned.
fn unclosed_quote(rest: &[char], out: &[Tok]) -> Result<Option<String>, String> {
    let mut j = 1;
    loop {
        match rest.get(j) {
            None => break,
            Some('\'') if rest.get(j + 1) == Some(&'\'') => j += 2,
            Some('\'') => return Ok(None),
            Some(_) => j += 1,
        }
    }
    match out {
        [] => Err(format!("{FILTER_SYNTAX}An single quote (') is missing.")),
        [Tok::Op("@")] => Ok(Some(rest.iter().collect())),
        _ => Err("an unclosed quote after an operator in a filter (not pinned)".into()),
    }
}

fn tokenize(
    text: &str,
    args: &[Value],
    question_literal: bool,
    kept_placeholder: &mut bool,
    at_placeholder: &mut bool,
    missing_placeholder: &mut Option<String>,
    unclosed_after_at: &mut bool,
) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut raw = String::new();
    let flush = |raw: &mut String, out: &mut Vec<Tok>| {
        let t = raw.trim();
        if !t.is_empty() {
            out.push(Tok::Raw(t.to_string()));
        }
        raw.clear();
    };
    let mut i = 0;
    while i < c.len() {
        let rest: String = c[i..].iter().take(2).collect();
        let op = ["..", "<>", "<=", ">="]
            .iter()
            .find(|o| rest.starts_with(**o))
            .copied()
            .or_else(|| match c[i] {
                '|' => Some("|"),
                '&' => Some("&"),
                '<' => Some("<"),
                '>' => Some(">"),
                '=' => Some("="),
                '(' => Some("("),
                ')' => Some(")"),
                // `@` makes the value case-insensitive only where a value starts; after the first character of a value it is a
                // normal character (record/filters/W4Filters.al UN_SetView_AtSignInValue_Text, _Text_2, _TextMore_2, _TextMore_3 on
                // writer UN's corpus branch, commit d3bcc580: `a@b.dk`, `@a@b.dk`, `*@b*`, `ab@`, Code `AT1@`; assumption
                // `filter-at-sign-in-value`).
                '@' if raw.trim().is_empty() => Some("@"),
                _ => None,
            });
        if let Some(op) = op {
            flush(&mut raw, &mut out);
            out.push(Tok::Op(op));
            i += op.len();
        // A quote opens a literal only where a value starts; inside a word it is a character (record/crud/RcAMisc.al:
        // GetView_Field_Types `quote`: `it's`).
        } else if c[i] == '\'' && raw.trim().is_empty() {
            flush(&mut raw, &mut out);
            if let Some(unclosed) = unclosed_quote(&c[i..], &out)? {
                // After `@` alone an unclosed quote is not a literal: the rest of the text is the value as written.
                raw.push_str(&unclosed);
                *unclosed_after_at = true;
                break;
            }
            let mut lit = String::new();
            i += 1;
            // A placeholder is replaced inside quotes only when it is the whole quoted text: `'%1'` takes the argument, `'x%1y'` and
            // `'%1%2'` stay as written (record/filters/FilterPlaceholderInQuotes.al:P169_QuotedPlaceholder_InsideLongerQuotedText;
            // the filter text of Gen. Jnl.-Show CT Entries filters a blank currency as a quoted blank). A text or an Integer argument is put
            // in as it reads; any other type is not pinned.
            loop {
                match c.get(i) {
                    None => unreachable!("unclosed_quote checked that the quote is closed"),
                    Some('\'') if c.get(i + 1) == Some(&'\'') => {
                        lit.push('\'');
                        i += 2;
                    }
                    Some('\'') => {
                        i += 1;
                        break;
                    }
                    Some(ch) => {
                        lit.push(*ch);
                        i += 1;
                    }
                }
            }
            if lit.len() == 2 && lit.starts_with('%') && lit.as_bytes()[1].is_ascii_digit() {
                let n = (lit.as_bytes()[1] - b'0') as usize;
                let arg = args
                    .get(n.wrapping_sub(1))
                    .ok_or("filter placeholder without an argument")?;
                match arg {
                    Value::Text(t) | Value::Code(t) => lit = t.to_string(),
                    Value::Integer(v) => lit = v.to_string(),
                    other => {
                        return Err(format!(
                            "placeholder %{n} inside quotes holds {}: only a text or an Integer is put in (not pinned)",
                            other.describe()
                        ));
                    }
                }
            }
            out.push(Tok::Quoted(lit));
        } else if c[i] == '%' && c.get(i + 1).is_some_and(|d| d.is_ascii_digit()) {
            let n = c[i + 1].to_digit(10).expect("digit") as usize;
            // A placeholder right next to a wildcard is not put in: the filter keeps `%1` as written. Pinned: `<>%1*` with `A` reads
            // back as `<>%1*` and keeps every row (record/filters/WildcardGetFilter.al:188, :198); `%1*`, `*%1*`,
            // `@*%1*`, `*%1` and `?%1` on a Text and a Code field read back as written and keep no row (record/filters/W4Filters.al:661-695,
            // SetFilterPercent1_WildcardsAroundThePlaceholder_BQ). A placeholder after other text and a wildcard (`x%1*`, `**%1`) is
            // not pinned.
            let wild = |w: &char| matches!(w, '*' | '?');
            let before = raw.trim();
            let after_wildcard = before.chars().count() == 1
                && before.chars().all(|w| wild(&w))
                && (out.is_empty() || matches!(out.last(), Some(Tok::Op("@"))));
            let start = before.is_empty() && (out.is_empty() || matches!(out.last(), Some(Tok::Op("<>"))));
            // `@` right before a placeholder: the placeholder is not put in either. Pinned for the whole filter `@%1` on a Text
            // field: a value that differs from the row only in case keeps no row (onprem metadata/platform-tables/
            // Y35StoredTables.al:194-195, :210). The read-back `@%1` follows `@*%1*` (record/filters/W4Filters.al:662, :682; a
            // probes-needed row). Inside a longer filter or on a field other than a Text it is refused.
            if before.is_empty() && matches!(out.last(), Some(Tok::Op("@"))) {
                if out.len() != 1 || i + 2 != c.len() {
                    return Err(format!(
                        "`@` right before placeholder %{n} inside a longer filter (not pinned)"
                    ));
                }
                *at_placeholder = true;
                raw.push('%');
                raw.push(c[i + 1]);
                i += 2;
                continue;
            }
            if after_wildcard || c.get(i + 2).is_some_and(wild) {
                if !(after_wildcard || start) {
                    return Err(format!("placeholder %{n} right before a wildcard (not pinned)"));
                }
                *kept_placeholder = true;
                raw.push('%');
                raw.push(c[i + 1]);
                i += 2;
                continue;
            }
            flush(&mut raw, &mut out);
            // A placeholder with no argument stays as written: on a Code and a Text field `<>%1` is the text `%1`
            // (record/filters/PlaceholderWithoutArgument.al:95-132, staging r6980); the caller decides per field type.
            let Some(arg) = args.get(n.wrapping_sub(1)).cloned() else {
                missing_placeholder.get_or_insert_with(|| format!("%{n}"));
                out.push(Tok::Raw(format!("%{n}")));
                i += 2;
                continue;
            };
            // A text substituted for a placeholder is TEXT, not a filter, except that `*` and `?` stay
            // wildcards and a leading `@` ignores case. `&`, `|`, `(`, `)`, `<`, `=` and `..` in it make
            // BC quote the whole value (record/TestSetFilterSpecialTextContracts.al, BC 27.5);
            // `>` is also quoted (record/filters/W4Filters.al:174-191), with a leading `@` too: `'@a>b@x.com'`
            // ignores case (format-evaluate/Y14Small.al:309, corpus master). A text with both a wildcard and one of those is
            // quoted and keeps the wildcard (record/filters/W4Filters.al SetFilterPercent1_WildcardTogetherWithOperators: `a*&b` reads back
            // `'a*&b'` and matches `a&b`); the text `*` alone is the literal text, not a wildcard (`star_alone`: `='*'`, no row).
            if let Value::Text(t) | Value::Code(t) = &arg {
                // A lone `@` is the text `@`, quoted: GetFilters reads `'@'` and it matches only that value
                // (format-evaluate/Y14Small.al:309 `@`, corpus master).
                if t == "@" {
                    out.push(Tok::Typed(arg.clone()));
                    i += 2;
                    continue;
                }
                let (at, body) = match t.strip_prefix('@') {
                    Some(rest) => (true, rest),
                    None => (false, t.as_str()),
                };
                let special = body.contains(['&', '|', '(', ')', '<', '>', '=']) || body.contains("..");
                if body.starts_with('@') {
                    return Err(format!(
                        "placeholder %{n} holds a text with the filter character `@`: how BC quotes it is not pinned"
                    ));
                }
                // The text `?` alone is the literal text too in the pinned shapes (`question_literal`): `%1` reads back `='?'`
                // and keeps only the row `?`, `<>%1` reads back `<>'?'` (record/filters/PlaceholderWildcardValue.al:225, :242).
                if t == "*" || (t == "?" && question_literal) {
                    out.push(Tok::Typed(arg.clone()));
                    i += 2;
                    continue;
                }
                if at {
                    out.push(Tok::Op("@"));
                }
                if special {
                    out.push(Tok::Quoted(body.to_string()));
                } else if !body.is_empty() {
                    out.push(Tok::Arg(body.to_string()));
                } else {
                    out.push(Tok::Typed(arg.clone()));
                }
                i += 2;
                continue;
            }
            out.push(Tok::Typed(arg));
            i += 2;
        } else {
            // Pinned by record/filters/W4Filters.al UN_SetView_AtSignInValue_Text, UN_SetView_AtSignInValue_Text_2 (was assumption `filter-at-sign-in-value`).
            raw.push(c[i]);
            i += 1;
        }
    }
    flush(&mut raw, &mut out);
    Ok(out)
}

struct Parser<'a> {
    toks: Vec<Tok>,
    i: usize,
    /// A wildcard text written as a literal value was read (see [`FilterExpr::UnpinnedText`]).
    written_wildcard: bool,
    ty: &'a crate::value::RtType,
    enums: &'a dyn Fn(u32) -> Result<Vec<(i32, String)>, String>,
}

impl Parser<'_> {
    fn peek_op(&self) -> Option<&'static str> {
        match self.toks.get(self.i) {
            Some(Tok::Op(o)) => Some(o),
            _ => None,
        }
    }

    fn date(&self) -> bool {
        matches!(self.ty, crate::value::RtType::Date)
    }

    fn or(&mut self) -> Result<FilterExpr, String> {
        if self.date() && self.peek_op() == Some("|") {
            return Err(format!(
                "{FILTER_SYNTAX}The left side of '|' operators cannot be empty."
            ));
        }
        let mut xs = vec![self.and()?];
        while self.peek_op() == Some("|") {
            self.i += 1;
            if self.date() && self.peek_op() == Some("|") {
                return Err(format!(
                    "{FILTER_SYNTAX}The right side of '|' operators cannot be empty."
                ));
            }
            xs.push(self.and()?);
        }
        Ok(if xs.len() == 1 {
            xs.pop().expect("one")
        } else {
            FilterExpr::Or(xs)
        })
    }

    fn and(&mut self) -> Result<FilterExpr, String> {
        if self.date() && self.peek_op() == Some("&") {
            return Err(format!(
                "{FILTER_SYNTAX}The left side of '&' operators cannot be empty."
            ));
        }
        let mut xs = vec![self.term()?];
        while self.peek_op() == Some("&") {
            self.i += 1;
            xs.push(self.term()?);
        }
        Ok(if xs.len() == 1 {
            xs.pop().expect("one")
        } else {
            FilterExpr::And(xs)
        })
    }

    fn value(&mut self) -> Option<Tok> {
        match self.toks.get(self.i) {
            Some(Tok::Op(_)) | None => None,
            Some(t) => {
                self.i += 1;
                Some(t.clone())
            }
        }
    }

    fn typed(&mut self, t: Tok) -> Result<Value, String> {
        use crate::value::RtType as T;
        let from_arg = matches!(t, Tok::Arg(_));
        let text = match t {
            // An Enum value given for an Option column through SetFilter's %1 filters by its ordinal and shows under the Enum's own name
            // (types/duration/B_EnumOptionVariant.al:Filter_OptionColumn_SetRange_EnumValue, `filter_green_filter`
            // is `Green`; SetRange shows the Option's member, `range_green_filter` is `One`).
            Tok::Typed(v @ Value::Enum(..)) if matches!(self.ty, T::Option(_)) => return Ok(v),
            Tok::Typed(v) => return crate::ops::convert(v, self.ty).map_err(|e| e.describe()),
            // A quoted empty text is the blank date (corpus record/load-fields/P81DateFilters.al:193-194,
            // :308). The unquoted empty text stays refused.
            Tok::Quoted(s) if s.is_empty() && matches!(self.ty, T::Date) => return Ok(Value::Date(0, false)),
            // A quoted empty text on a DateTime field is the blank DateTime (types/P136Filter.al:361, corpus
            // master, TV_SetView_DateTime_LiteralTexts `empty` keeps only the row with no DateTime).
            Tok::Quoted(s) if s.is_empty() && matches!(self.ty, T::DateTime) => return Ok(Value::DateTime(0)),
            // A quoted empty text on a Time field is the undefined Time, after every operator, as a range end and in an OR list
            // (corpus master 3eb812ed, record/filters/TimeFilterEmptyText.al:104-128, al-runner
            // issue 6900).
            Tok::Quoted(s) if s.is_empty() && matches!(self.ty, T::Time) => return Ok(Value::Time(0)),
            // On a Media field a quoted empty text, alone or after `<>`, raises the Guid format error
            // (streams/media/SoMediaFilter.al:143-147, SO_MediaFilter_EmptyAndQuoted).
            Tok::Quoted(s)
                if s.is_empty() && matches!(self.ty, T::Library(crate::library_kind::LibraryKind::Media)) =>
            {
                return Err(INVALID_GUID.to_string());
            }
            Tok::Raw(s) | Tok::Quoted(s) | Tok::Arg(s) => s,
            Tok::Op(o) => return Err(format!("operator `{o}` where a value was expected")),
        };
        // A written wildcard text as a literal value reads back as it did before the placeholder forms were pinned: refused,
        // unless it is quoted for a filter operator in it or holds a double quote (see `FilterExpr::UnpinnedText`).
        if !from_arg
            && matches!(self.ty, T::Text(_) | T::Code(_))
            && text.contains(['*', '?'])
            && !text.contains('"')
            && !needs_quotes(&text)
        {
            self.written_wildcard = true;
        }
        Ok(match self.ty {
            T::Text(_) => Value::Text(text.into()),
            T::Code(_) => Value::Code(filter_upper(text.trim())?.into()),
            // Any text that is no Integer: letters, a range end `x`, `1.5`, a number past the range (RcAFilters.al:392-414, PtObserve.al:277).
            // A text with blank space in it is not pinned.
            T::Integer => Value::Integer(text.parse().map_err(|_| {
                if text.contains(char::is_whitespace) && !spaced_number(&text) {
                    format!("`{text}` is not an Integer")
                } else {
                    not_evaluated("Integer", &text)
                }
            })?),
            // A word of letters and hyphens raises the evaluate error (record/filters/InvalidDecimalFilter.al:134,
            // Other_Types `NOT-A-NUMBER`); other texts that are no BigInteger are not pinned.
            T::BigInteger => Value::BigInteger(text.parse().map_err(|_| {
                if letters_or_hyphens(&text) {
                    not_evaluated("BigInteger", &text)
                } else {
                    format!("`{text}` is not a BigInteger")
                }
            })?),
            T::Decimal => match decimal_filter_text(&text) {
                Some(Ok(d)) => Value::Decimal(d),
                Some(Err(())) => return Err(not_evaluated("Decimal", &text)),
                None => return Err(format!("`{text}` is not a Decimal")),
            },
            // InvalidDecimalFilter.al:135 (Other_Types): `NOT-A-NUMBER` on a Duration field. Other Duration texts are not pinned.
            T::Duration if letters_or_hyphens(text.trim()) => return Err(not_evaluated("Duration", text.trim())),
            // `<>''` is every Duration but zero (TestFieldRefFilter.al FieldRef_SetFilter_Zero_Duration `dur_ne_empty`).
            T::Duration if text.trim().is_empty() => Value::Duration(0),
            T::Duration if duration_filter_value(text.trim()).is_some() => {
                Value::Duration(duration_filter_value(text.trim()).expect("checked"))
            }
            // A Time filter of letters raises the evaluate error (PtObserve.al:262), also with hyphens (InvalidDecimalFilter.al:137,
            // `NOT-A-NUMBER`); `12:00:00` is noon (PtObserve.al:268, the text the MakeTimeFilter subscriber writes). Other times are not pinned.
            T::Time if letters_or_hyphens(text.trim()) => return Err(not_evaluated("Time", text.trim())),
            T::Time if is_noon_text(text.trim()) => {
                let t = text.trim();
                let (m, s) = (t[3..5].parse::<i64>().unwrap_or(0), t[6..8].parse::<i64>().unwrap_or(0));
                Value::Time(12 * 3_600_000 + m * 60_000 + s * 1000 + 1)
            }
            T::Time if time_filter_text(text.trim()).is_some() => {
                time_filter_text(text.trim()).unwrap_or(Value::Time(0))
            }
            T::DateTime if letters_only(text.trim()) => return Err(not_evaluated("DateTime", text.trim())),
            // A dotted date in an English session, alone or with a clock, is no DateTime (P136Filter.al UN_DateTimeFilter_DateAlone,
            // _MoreHourForms_3 `15.01.25`, `15.01.25 08:30` types/P136Filter.al:447-586).
            T::DateTime if dotted_datetime_text(text.trim()) => {
                return Err(not_evaluated("DateTime", text.trim()));
            }
            T::DateTime if datetime_filter_text(text.trim()).is_some() => {
                datetime_filter_text(text.trim()).unwrap_or(Value::DateTime(0))
            }
            // A four-digit year, and texts that are no DateTime (`date_filter_forms`).
            T::DateTime if english_session() && crate::date_filter_forms::english_datetime(text.trim()).is_some() => {
                match crate::date_filter_forms::english_datetime(text.trim()) {
                    Some(Ok(v)) => v,
                    _ => return Err(not_evaluated("DateTime", text.trim())),
                }
            }
            // Format(false) = 'No' is used as a filter value in the corpus. The words BC accepts,
            // in any case, are Yes, True, 1 and No, False, 0
            // (record/TestFilterTextBooleanFindContracts.al:103, SetFilter_BooleanField_AcceptedWords).
            T::Boolean => match crate::filter_boolean_words::boolean_filter_word(
                DATE_CONTEXT.with(std::cell::Cell::get).map(|c| c.language),
                &text,
            )? {
                Some(b) => Value::Boolean(b),
                None => return Err(format!("{INVALID_BOOLEAN}{text}")),
            },
            // The matcher trims both sides and skips zero-length member names (corpus
            // types/TestEnumBlankOptionFilterTests.Codeunit.al:122,188).
            T::Option(m) => match m
                .iter()
                .position(|x| !x.is_empty() && x.trim().eq_ignore_ascii_case(text.trim()))
            {
                Some(i) => Value::Option(i as i32, m.clone()),
                None => Value::Option(
                    text.parse().map_err(|_| {
                        if matches!(text.trim(), "\" \"" | "\"\"") {
                            format!(
                                "{FILTER_SYNTAX}'{}' is not an option. The existing options are: {}",
                                text.trim(),
                                m.join(",")
                            )
                        } else {
                            format!("`{text}` is not an option member")
                        }
                    })?,
                    m.clone(),
                ),
            },
            T::Enum(id) => {
                // The list holds the captions first, then the names, so a caption wins over a name that spells the same
                // (corpus record/filters/EnumFilterByCaption.al: SetFilter_CaptionEqualsOtherName_Result).
                let named = (self.enums)(*id)?
                    .into_iter()
                    .find(|(_, n)| !n.is_empty() && n.trim().eq_ignore_ascii_case(text.trim()))
                    .map(|(v, _)| v);
                match named.or_else(|| text.trim().parse().ok()) {
                    Some(v) => Value::Enum(*id, v),
                    None => return Err(format!("`{text}` is not a member of the enum")),
                }
            }
            // A Guid filter value is braced or plain, with dashes, or the 32 digits alone, in any case; it shows as the braced upper
            // case form. A braced text without dashes, a missing brace, a wrong length, a non-hex digit, a number and a wildcard raise
            // the Guid format error (P143Guid_Filters.al:P143_GuidBraced, P143_GuidInvalid, P143_GuidEmpty).
            T::Guid if !text.trim().is_empty() => match crate::evaluate::read_text(self.ty, text.trim()) {
                Some(v) if !text.trim().starts_with('(') => v,
                _ => return Err(format!("{INVALID_GUID}{text}")),
            },
            // A date in a filter (types/date-time/P143Date_EnPlain.al, _EnRanges.al, _EnFormulas.al, _DePlain.al,
            // _DeRanges.al, _DeFormulas.al): month, day and year in that order with `/` (and `.` when the language is German), a closing date
            // as `C` and a date, `T` and `W` (any case) for today and the work date, and in English `CD` the closing date of today, `cd` and
            // `0D` today. The empty text is no filter. A text that is none of these is an error in German (its text is pinned there);
            // in any other language it is refused.
            T::Date if !text.trim().is_empty() && !text.contains(['<', '>']) => date_filter_value(text.trim())?,
            // The empty text is the empty formula: `''` keeps the row without a formula, `<>''` the rows with one
            // (record/Y70Probes.al:218-262, FilterDateFormula_EmptyText `filter_two_quotes`, `filter_ne_two_quotes`).
            T::DateFormula if text.trim().is_empty() => Value::DateFormula(String::new()),
            // The empty text on a RecordId field is the empty RecordId: `''` keeps the rows without one, `<>''` the rows with one
            // (recordref-fieldref/fieldref/FieldRefFilterEmptyText.al:153-190, `neq_text_same_field`, `eq_text_same_field`;
            // al-runner issue 7437).
            T::Library(crate::library_kind::LibraryKind::RecordId) if text.trim().is_empty() => {
                Value::default_for(self.ty)
            }
            // A formula without brackets is the stored formula: `1M` keeps the row of `<1M>`, `<=1M` and `<>1M` compare the
            // stored texts (record/filters/DateFormulaOperatorFilterText.al:116-134, staging r6980). Pinned for a number and an
            // upper-case unit.
            T::DateFormula if date_formula_step(text.trim()) => Value::DateFormula(format!("<{}>", text.trim())),
            // A date (a formatted Date, `'<=' + Format(D)`) is no formula: BC raises the time-unit error of the value without
            // its operator (DateFormulaOperatorFilterText.al:95-113, al-runner issue 6998).
            T::DateFormula
                if text.trim().contains('/') && text.trim().bytes().all(|c| c.is_ascii_digit() || c == b'/') =>
            {
                return Err(format!(
                    "{FILTER_SYNTAX}The date formula '{}' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning current time unit based on date, can be specified as a prefix to the time unit.",
                    text.trim()
                ));
            }
            // A Guid text on a Media field compares with the field's media id, braced or plain, with `=` or `<>` before it: an empty
            // field equals the null Guid (al-runner issue 5991: streams/media/SoMediaFilter.al SO_MediaFilter_NullGuidForms,
            // SO_MediaFilter_MediaIdOfFilledRow, SO_MediaFilter_FieldRefReproShape). Letters, `*` and a number raise the Guid format
            // error, as on a Guid field (SO_MediaFilter_InvalidText `abc`, `star`, `0`).
            T::Library(crate::library_kind::LibraryKind::Media) if !text.trim().is_empty() => {
                match crate::evaluate::read_text(&T::Guid, text.trim()) {
                    Some(v @ Value::Guid(_)) if !text.trim().starts_with('(') => v,
                    _ if text.trim() == "*" || text.trim().chars().all(|ch| ch.is_ascii_alphanumeric()) => {
                        return Err(format!("{INVALID_GUID}{text}"));
                    }
                    _ => return Err(format!("text filter value `{text}` for Media: not pinned")),
                }
            }
            // A Guid text on a MediaSet field compares with the field's set id: an empty set equals the null Guid, so `<>` keeps no
            // empty row (streams/media/SoMediaFilter.al:246-250, SO_MediaSetFilter_NullGuidForms). Any other Guid, braced, bare,
            // in lower case or quoted, with `=`, `<>`, `|`, `..` and `&`, the id of the stored set and that of the item inside it,
            // which matches no row (recordref-fieldref/fieldref/FieldRefFilterMediaGuidText.al:227-251, :288-365;
            // al-runner issue 7461). GetFilter writes the Guid braced in upper case. A text that is no Guid, and a Guid other than
            // the null Guid in a text placeholder argument (`SetFilter(MS, '%1', Format(G))`; the FieldRef form raises the
            // conversion error, FieldRefFilterMediaGuidText.al:258-296), are not pinned on a MediaSet field: refused.
            T::Library(crate::library_kind::LibraryKind::MediaSet) if !text.trim().starts_with('(') => {
                match crate::evaluate::read_text(&T::Guid, text.trim()) {
                    Some(Value::Guid(g)) if from_arg && g != [0; 16] => {
                        return Err(format!(
                            "a text placeholder argument `{text}` on a MediaSet field (not pinned)"
                        ));
                    }
                    Some(v @ Value::Guid(_)) => v,
                    _ => {
                        return Err(format!("text filter value `{text}` for MediaSet: not pinned"));
                    }
                }
            }
            other => {
                return Err(format!(
                    "text filter value `{text}` for {other}: how BC reads the text (an empty `''`, a formula such as `1M`, a region's date format) is not pinned (docs/probes-needed.md)",
                    other = other.describe()
                ));
            }
        })
    }

    /// A single value, which may be a wildcard pattern on text fields.
    fn single(&mut self, t: Tok, case_insensitive: bool) -> Result<FilterExpr, String> {
        use crate::value::RtType as T;
        // `@` and wildcards stay active inside a quoted expression; only the filter operators
        // are literal there (record/TestFilterQuotedLiterals.al).
        if let (Tok::Quoted(q), T::Text(_) | T::Code(_)) = (&t, self.ty) {
            let (body, ci) = match q.strip_prefix('@') {
                Some(b) => (b, true),
                None => (q.as_str(), case_insensitive),
            };
            if body.contains(['*', '?']) || ci {
                // A Code filter shows upper case with `@` too (types/TestProbeScalarContracts.al:SetFilter_CodeFieldUpperCases).
                let body = if matches!(self.ty, T::Code(_)) {
                    filter_upper(body)?
                } else {
                    body.to_string()
                };
                return Ok(FilterExpr::Like(body, ci));
            }
        }
        if let (Tok::Raw(s) | Tok::Arg(s), T::Text(_) | T::Code(_)) = (&t, self.ty) {
            let s = if matches!(self.ty, T::Code(_)) {
                filter_upper(s)?
            } else {
                s.clone()
            };
            if s.contains(['*', '?']) || case_insensitive {
                return Ok(FilterExpr::Like(s, case_insensitive));
            }
        }
        Ok(FilterExpr::Eq(self.typed(t)?))
    }

    /// `<>` before a wildcard pattern on a Text or Code field (record/filters/WildcardGetFilter.al:106-202):
    /// `<>A*`, `<>*`, `<>?` and `<>@a*` keep the texts the pattern does not match; a placeholder that holds exactly `*` or `?` is
    /// the literal text (`<>%1` with `*` reads back as `<>'*'` and keeps every row; substituted as a typed value). A placeholder
    /// holding another wildcard text, and the pattern written in quotes, are the pattern too and read back without quotes
    /// (record/filters/PlaceholderWildcardValue.al:215-228 `<>%1` with `A*B`, `A?B`, `A*`, `*B`, `A*B?` and the reporter's
    /// value, :363-377 `<>'A*B'`, :424-436 on a Code field, :633-657 through a FieldRef; al-runner issue 7460). A quoted pattern
    /// that holds a filter operator, an apostrophe or a double quote or starts with `@`, and `<>@` before a text without a
    /// wildcard, are not pinned. `None`: not a pattern, read as before.
    fn not_pattern(&mut self) -> Result<Option<FilterExpr>, String> {
        use crate::value::RtType as T;
        if !matches!(self.ty, T::Text(_) | T::Code(_)) {
            return Ok(None);
        }
        let ci = self.peek_op() == Some("@");
        let at = self.i + usize::from(ci);
        let quoted = matches!(self.toks.get(at), Some(Tok::Quoted(_)));
        let wild = |s: &str| s.contains(['*', '?']);
        match self.toks.get(at) {
            Some(Tok::Raw(s)) if wild(s) => {
                let p = if matches!(self.ty, T::Code(_)) {
                    filter_upper(s)?
                } else {
                    s.clone()
                };
                self.i = at + 1;
                Ok(Some(FilterExpr::NotLike(p, ci)))
            }
            // On a Text field `<>@` before a value without a wildcard keeps the rows that do not equal it without regard to case,
            // and `<>@` alone keeps none (WcInvalidFilterExpr.al:93-111 `<>@C-A`, `<>@c-b`, `<>@`). A placeholder value that starts
            // with `@` (`'<>%1', '@NO PRINT'`) is the same filter, and on a Code field the value is upper-cased first
            // (record/filters/PlaceholderAtSignValue.al, staging r6980; al-runner issue 7034).
            Some(Tok::Raw(s) | Tok::Arg(s)) if ci && matches!(self.ty, T::Text(_) | T::Code(_)) => {
                let p = if matches!(self.ty, T::Code(_)) {
                    filter_upper(s)?
                } else {
                    s.clone()
                };
                self.i = at + 1;
                Ok(Some(FilterExpr::NotLike(p, true)))
            }
            None if ci && matches!(self.ty, T::Text(_)) && self.toks.len() == 2 => {
                self.i = at;
                Ok(Some(FilterExpr::AtSignAlone(true)))
            }
            _ if ci => Err("`<>@` before a value without a wildcard (not pinned)".into()),
            // A substituted text with a filter operator arrives quoted (`tokenize`), so a plain `Arg` holds none.
            // A quoted pattern with blank space in it (`<>'A *'`) is pinned by no case: refused below, as before.
            Some(Tok::Arg(s) | Tok::Quoted(s))
                if wild(s) && plain_pattern(s) && !(quoted && s.contains(char::is_whitespace)) =>
            {
                let p = if matches!(self.ty, T::Code(_)) {
                    filter_upper(s)?
                } else {
                    s.clone()
                };
                self.i = at + 1;
                Ok(Some(FilterExpr::NotLike(p, false)))
            }
            Some(Tok::Arg(s)) if wild(s) => Err(format!("`<>%n` with the wildcard text `{s}` (not pinned)")),
            Some(Tok::Quoted(s)) if wild(s) || s.starts_with('@') => {
                Err(format!("`<>` before the quoted pattern `{s}` (not pinned)"))
            }
            _ => Ok(None),
        }
    }

    /// The day of a date-alone value on a DateTime field ([`datetime_filter_day`]); marks the assumption.
    fn day_alone(&self, t: &Tok) -> Option<i32> {
        match (t, self.ty) {
            (Tok::Raw(s), crate::value::RtType::DateTime) => {
                let day = datetime_filter_day(s.trim())?;
                Some(day)
            }
            _ => None,
        }
    }

    /// A range end on a DateTime field: a date alone is the start of its day as the low end, its end as the high end.
    /// A date alone next to a value with a clock is not pinned.
    fn datetime_range(&self, lo: Option<Tok>, hi: Option<Tok>) -> Result<Option<FilterExpr>, String> {
        let (dl, dh) = (
            lo.as_ref().and_then(|t| self.day_alone(t)),
            hi.as_ref().and_then(|t| self.day_alone(t)),
        );
        if dl.is_none() && dh.is_none() {
            return Ok(None);
        }
        if (lo.is_some() && dl.is_none()) || (hi.is_some() && dh.is_none()) {
            return Err("a DateTime filter range of a date alone and a value with a clock (not pinned)".into());
        }
        Ok(Some(FilterExpr::Range(dl.map(day_start), dh.map(day_end))))
    }

    fn term(&mut self) -> Result<FilterExpr, String> {
        match self.peek_op() {
            Some("(") => {
                self.i += 1;
                let text_field = matches!(self.ty, crate::value::RtType::Text(_));
                if text_field && self.i == self.toks.len() {
                    return Err(format!("{FILTER_SYNTAX}{MISSING_PARENTHESIS}"));
                }
                let e = self.or()?;
                if self.peek_op() != Some(")") {
                    if text_field && self.i == self.toks.len() {
                        return Err(format!("{FILTER_SYNTAX}{MISSING_PARENTHESIS}"));
                    }
                    return Err("missing `)` in filter".into());
                }
                self.i += 1;
                Ok(e)
            }
            Some("@") => {
                self.i += 1;
                // On a Text field (WcInvalidFilterExpr.al:66-90): `@` alone keeps every row, `@<>` raises, and `@@C-A` reads the
                // second `@` as the value's first character (no row holds `@C-A`; GetFilter writes `@@C-A`).
                if matches!(self.ty, crate::value::RtType::Text(_)) {
                    match (self.toks.get(self.i), self.toks.get(self.i + 1)) {
                        (None, _) if self.toks.len() == 1 => return Ok(FilterExpr::AtSignAlone(false)),
                        (Some(Tok::Op("<>")), _) => return Err(did_not_expect("<>")),
                        (Some(Tok::Op("@")), Some(Tok::Raw(s))) if self.toks.len() == 3 && !s.contains(['*', '?']) => {
                            let s = format!("@{s}");
                            self.i += 2;
                            return Ok(FilterExpr::Like(s, true));
                        }
                        _ => {}
                    }
                }
                let v = self.value().ok_or("value expected after @")?;
                self.single(v, true)
            }
            Some(op @ ("<>" | "<=" | ">=" | "<" | ">" | "=")) => {
                self.i += 1;
                if op == "<>" {
                    if let Some(e) = self.not_pattern()? {
                        return Ok(e);
                    }
                }
                // `<` before a date alone on a DateTime field is before the start of that day (P136Filter.al
                // UN_DateTimeFilter_MoreHourForms_3 `<01/15/25`); the other operators before a date alone are not pinned.
                if let Some(day) = self.toks.get(self.i).and_then(|t| self.day_alone(t)) {
                    if op != "<" {
                        return Err(format!("`{op}` before a date alone on a DateTime field (not pinned)"));
                    }
                    self.i += 1;
                    return Ok(FilterExpr::Lt(day_start(day)));
                }
                let before = self.written_wildcard;
                let v = match self.value() {
                    Some(v) => self.typed(v)?,
                    None if matches!(self.ty, crate::value::RtType::Text(_) | crate::value::RtType::Code(_)) => {
                        Value::Text(String::new().into())
                    }
                    // `<>` alone on a Time, Date or DateTime field raises the filter error (corpus
                    // master 3eb812ed, record/filters/TimeFilterEmptyText.al:124, :148, :171 `ne alone`).
                    None if op == "<>"
                        && matches!(
                            self.ty,
                            crate::value::RtType::Time | crate::value::RtType::Date | crate::value::RtType::DateTime
                        ) =>
                    {
                        return Err(format!("{FILTER_SYNTAX}A value is expected after the '<>'."));
                    }
                    // The same on an Option field (FMOptionBlankField.al `flt_neg`, all eleven tables).
                    None if op == "<>" && matches!(self.ty, crate::value::RtType::Option(_)) => {
                        return Err(format!(
                            "{FILTER_SYNTAX}{AS_WRITTEN}A value is expected after the '<>'."
                        ));
                    }
                    None => return Err(format!("value expected after `{op}`")),
                };
                // `=*` keeps the read-back `='*'` it had, the text of `%1` with `*` (record/filters/W4Filters.al `star_alone`).
                if op == "=" && matches!(&v, Value::Text(t) | Value::Code(t) if &**t == "*") {
                    self.written_wildcard = before;
                }
                Ok(match op {
                    "<>" => FilterExpr::Ne(v),
                    "<=" => FilterExpr::Le(v),
                    ">=" => FilterExpr::Ge(v),
                    "<" => FilterExpr::Lt(v),
                    ">" => FilterExpr::Gt(v),
                    _ => FilterExpr::Eq(v),
                })
            }
            Some("..") => {
                self.i += 1;
                if self.date() && self.i == self.toks.len() && self.i == 1 {
                    return Err(format!(
                        "{FILTER_SYNTAX}Both sides of the range ('..') cannot be empty."
                    ));
                }
                let hi = self.value().ok_or("value expected after ..")?;
                if let Some(e) = self.datetime_range(None, Some(hi.clone()))? {
                    return Ok(e);
                }
                Ok(FilterExpr::Range(None, Some(self.typed(hi)?)))
            }
            _ => {
                let Some(lo) = self.value() else {
                    // An empty filter value (`''` was a quoted empty literal and is a value).
                    return Err("empty filter term".into());
                };
                if self.peek_op() == Some("..") {
                    self.i += 1;
                    let at = self.i;
                    let hi = self.value();
                    if let Some(e) = self.datetime_range(Some(lo.clone()), hi)? {
                        return Ok(e);
                    }
                    self.i = at;
                    let lo = self.typed(lo)?;
                    let hi = match self.value() {
                        Some(h) => Some(self.typed(h)?),
                        None => None,
                    };
                    // An Integer range with the same two ends shows as `5..5` too (events/table-events/
                    // BdFilterEvents.al:BdFilter_Int_8 m3, the text a MakeIntFilter subscriber repaired to `5..5`).
                    if (self.date() || matches!(self.ty, crate::value::RtType::Integer))
                        && hi
                            .as_ref()
                            .is_some_and(|h| compare(h, &lo).ok() == Some(Ordering::Equal))
                    {
                        return Ok(FilterExpr::RangeSame(lo));
                    }
                    // GetFilter writes a range of one wildcard text as SetRange does (`='A*B'`, record/filters/
                    // PlaceholderWildcardValue.al:413); whether a written range with two equal ends reads back the same is not
                    // pinned.
                    if matches!(&lo, Value::Text(t) | Value::Code(t) if t.contains(['*', '?']))
                        && hi
                            .as_ref()
                            .is_some_and(|h| compare(h, &lo).ok() == Some(Ordering::Equal))
                    {
                        return Err("a filter range whose two ends are the same wildcard text (not pinned)".into());
                    }
                    return Ok(FilterExpr::Range(Some(lo), hi));
                }
                if let Some(day) = self.day_alone(&lo) {
                    return Ok(FilterExpr::Range(Some(day_start(day)), Some(day_end(day))));
                }
                // A quoted empty text alone on a DateTime field is the blank DateTime as a range of one value: GetFilter(s) shows
                // `''..''`, also next to `|`, while `=''`, a range end and the blank DateTime given as %1 show `''` (corpus master
                // 3eb812ed, record/filters/TimeFilterEmptyText.al:160-172 `empty`, `eq empty`, `range from empty`, `empty or arg`;
                // record/views/Y71SetViewDateTime.al:125-127 dt_empty_or).
                if matches!(&lo, Tok::Quoted(q) if q.is_empty()) && matches!(self.ty, crate::value::RtType::DateTime) {
                    return Ok(FilterExpr::RangeSame(Value::DateTime(0)));
                }
                self.single(lo, false)
            }
        }
    }
}

/// Whether the wildcard text `s` holds no filter operator, quote or leading `@`: the pattern forms that
/// record/filters/PlaceholderWildcardValue.al pins after `<>` (`Parser::not_pattern`).
fn plain_pattern(s: &str) -> bool {
    !s.contains(['&', '|', '(', ')', '<', '>', '=', '\'', '"', '@']) && !s.contains("..")
}

/// A filter on the field at position `field` of the table's field list.
#[derive(Debug, Clone)]
pub struct FieldFilter {
    /// The GlobalLanguage in force on this thread when the filter was set ([`filter_language`]): a filter text shows an enum
    /// value's CaptionML caption only under that language (al-vm enum_caption_ml.rs).
    pub lang: i32,
    pub field: usize,
    pub expr: FilterExpr,
    /// The filter group it was set in (`FilterGroup(n)`); every group applies at once.
    pub group: i32,
}

impl FieldFilter {
    /// A filter set now: it records the language of this thread ([`filter_language`]).
    pub fn new(field: usize, expr: FilterExpr, group: i32) -> Self {
        FieldFilter {
            lang: filter_language(),
            field,
            expr,
            group,
        }
    }
}

thread_local! {
    /// The session language of this thread, which a new [`FieldFilter`] records; the VM sets it with the GlobalLanguage.
    static FILTER_LANGUAGE: std::cell::Cell<i32> = const { std::cell::Cell::new(1033) };
}

/// Sets the language a new [`FieldFilter`] records; the VM calls it when the session language changes.
pub fn set_filter_language(language: i32) {
    FILTER_LANGUAGE.with(|c| c.set(language));
}

/// The language a new [`FieldFilter`] records.
pub fn filter_language() -> i32 {
    FILTER_LANGUAGE.with(|c| c.get())
}

/// The filter group `FilterGroup(-1)` writes to: its filters on different fields are alternatives
/// (a row passes when it matches any of them), while the filters of every other group all have to
/// match (corpus record/PWFilterGroupOr.al).
pub const OR_GROUP: i32 = -1;

/// The refusal of a read that selects rows through `filters`, when one of them has a matching that is not pinned.
pub fn unpinned_match(filters: &[FieldFilter]) -> Option<String> {
    filters.iter().find_map(|f| f.expr.unpinned_match())
}

/// Whether `row` passes `filters`: every filter outside the OR group matches, and, when the OR
/// group holds filters, at least one of them does. A filter whose matching is not pinned refuses.
pub fn row_passes(row: &[Value], filters: &[FieldFilter]) -> Result<bool, Signal> {
    if let Some(why) = unpinned_match(filters) {
        return Err(Signal::unsupported(why));
    }
    Ok(row_passes_checked(row, filters))
}

/// Whether every filter matches its field of `row`, whatever its group; a filter whose matching is not pinned refuses.
pub fn all_match(filters: &[FieldFilter], row: &[Value]) -> Result<bool, Signal> {
    for f in filters {
        if !f.expr.matches(&row[f.field])? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `row` when every filter matches it ([`all_match`]), else `None`.
pub fn passing(filters: &[FieldFilter], row: Option<Vec<Value>>) -> Result<Option<Vec<Value>>, Signal> {
    match row {
        Some(r) if all_match(filters, &r)? => Ok(Some(r)),
        _ => Ok(None),
    }
}

/// [`row_passes`] for a caller that refused the filters with [`unpinned_match`] first.
pub fn row_passes_checked(row: &[Value], filters: &[FieldFilter]) -> bool {
    let (mut or_seen, mut or_matched) = (false, false);
    for f in filters {
        let matched = f.expr.matches_checked(&row[f.field]);
        if f.group == OR_GROUP {
            or_seen = true;
            or_matched |= matched;
        } else if !matched {
            return false;
        }
    }
    !or_seen || or_matched
}

/// Total order on key tuples (field-wise `compare`; incomparable counts as equal).
pub fn key_cmp(a: &[Value], b: &[Value]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        match crate::ops::compare_key_value(x, y).unwrap_or(Ordering::Equal) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    a.len().cmp(&b.len())
}

/// A Date filter text that is one of the letters `M`, `D`, `CM` or `Y` alone, in an English session
/// (record/filters/DateFilterLettersWorkDates.al, five work dates): `M` is the Monday of the week of the
/// work date and `D` the work date; `CM` is the last day of the month of Today; `Y` runs from the work date to the end of its year
/// when that is the year of Today (the work date 2026-01-01). `Y` in another year is refused: it gave `01/01/27..12/31/9999` for
/// the work date 2030-05-15, which no rule the corpus pins explains.
fn date_letter_filter(t: &str) -> Option<Result<FilterExpr, String>> {
    if !matches!(t, "M" | "D" | "CM" | "Y") {
        return None;
    }
    let c = DATE_CONTEXT.with(std::cell::Cell::get).filter(|c| c.language == 1033)?;
    let date = |d: i32| Value::Date(d, false);
    let (wy, _, _) = crate::format::civil_from_day(c.work_date);
    Some(Ok(match t {
        // 2024-01-01 was a Monday.
        "M" => FilterExpr::Eq(date(
            c.work_date - (c.work_date - crate::format::day_from_civil(2024, 1, 1)).rem_euclid(7),
        )),
        "D" => FilterExpr::Eq(date(c.work_date)),
        "CM" => {
            let (y, m, _) = crate::format::civil_from_day(c.today);
            FilterExpr::Eq(date(crate::format::day_from_civil(
                y,
                m,
                crate::iso::days_in_month(y, m),
            )))
        }
        _ if let Some((start, end)) = c.fiscal_year => FilterExpr::Range(Some(date(start)), Some(date(end))),
        _ if crate::format::civil_from_day(c.today).0 == wy => FilterExpr::Range(
            Some(date(c.work_date)),
            Some(date(crate::format::day_from_civil(wy, 12, 31))),
        ),
        _ => {
            return Some(Err(
                "the Date filter letter `Y` with a work date outside the year of Today (not pinned)".to_string(),
            ));
        }
    }))
}

/// `T-D` (0) and `T-1D` to `T-7D` (n): the texts that stand for a Tuesday of an ISO week ([`tuesday_of_iso_week`]).
fn t_weeks(t: &str) -> Option<i32> {
    let n = t.strip_prefix("T-")?.strip_suffix('D')?;
    match n {
        "" => Some(0),
        _ if n.len() == 1 => n.parse().ok().filter(|w| (1..=7).contains(w)),
        _ => None,
    }
}

/// The value of one Date filter text, or the error: `INVALID_DATE` and the text for a German session or an English text of a
/// shape the corpus pins as invalid, a refusal otherwise.
fn date_filter_value(t: &str) -> Result<Value, String> {
    let ctx = DATE_CONTEXT.with(std::cell::Cell::get);
    let german = ctx.is_some_and(|c| c.language == 1031);
    let english = ctx.is_some_and(|c| c.language == 1033);
    let needs_session =
        || format!("text filter value `{t}` for Date: the session's dates are not known here (not pinned)");
    match t {
        "T" | "t" => return ctx.map(|c| Value::Date(c.today, false)).ok_or_else(needs_session),
        "W" | "w" => return ctx.map(|c| Value::Date(c.work_date, false)).ok_or_else(needs_session),
        _ if english && ctx.is_some() && t_weeks(t).is_some() => {
            let week = t_weeks(t).unwrap_or(0);
            return Ok(Value::Date(
                tuesday_of_iso_week(ctx.map_or(0, |c| c.work_date), week),
                false,
            ));
        }
        "CD" if english => return ctx.map(|c| Value::Date(c.today, true)).ok_or_else(needs_session),
        "cd" | "0D" if english => return ctx.map(|c| Value::Date(c.today, false)).ok_or_else(needs_session),
        _ => {}
    }
    // The words for today and the work date, in any case, and `*` (P81DateFilters.al:SetFilter_Date_OtherKeywordLetters
    // `TODAY`, `Today`, `WORKDATE`; SetFilter_Date_InvalidTexts `*`).
    if english {
        if t.eq_ignore_ascii_case("today") || t == "*" {
            return ctx.map(|c| Value::Date(c.today, false)).ok_or_else(needs_session);
        }
        if t.eq_ignore_ascii_case("workdate") {
            return ctx.map(|c| Value::Date(c.work_date, false)).ok_or_else(needs_session);
        }
    }
    // The dotted spelling reads like the slashed one in German: `1.1.2027` is the first of January, `17.01.27` has no month 17.
    let slashed;
    // In an English session `03.01.2020` reads as `03/01/2020` too (record/crud/RcAFilters.al:689, SetFilter_Decimal_Date_
    // Boolean_Option_Enum `dateDotted`); pinned for two digits, two digits and four digits only.
    let english_dotted = english && {
        let p: Vec<&str> = t.split('.').collect();
        p.len() == 3
            && [2, 2, 4]
                .iter()
                .zip(&p)
                .all(|(n, x)| x.len() == *n && x.bytes().all(|b| b.is_ascii_digit()))
    };
    let text = if (german && !t.contains('/') && t.matches('.').count() == 2) || english_dotted {
        slashed = t.replace('.', "/");
        slashed.as_str()
    } else {
        t
    };
    if let Some(v) = crate::evaluate::read_text(&crate::value::RtType::Date, text) {
        return Ok(v);
    }
    // Dashes read like slashes in an English session when the year has four digits (P81DateFilters.al:SetFilter_Date_InvalidTexts
    // `01-01-2026`), and with two-digit numbers throughout (record/lt-date-text-forms/DateFilterTextForms.al:
    // GetRange_DateTextForms_English `separators_short_year` `02-01-26`, GetRange_DayAboveTwelve_English `mm_dd_yy_separators`).
    let dashed_short = t.split('-').count() == 3
        && t.split('-')
            .all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit()));
    if english {
        let b = text.as_bytes();
        if (b.len() == 10
            && b[2] == b'-'
            && b[5] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit()))
            || dashed_short
        {
            if let Some(v) = crate::evaluate::read_text(&crate::value::RtType::Date, &text.replace('-', "/")) {
                return Ok(v);
            }
        }
    }
    // Six digits read as month, day, year in an English or a German session: `020126` is 2026-02-01 and `040126` is 2026-04-01
    // (recordref-fieldref/fieldref/RangeMinMaxTypes.al:389-565, filters of a Date field; DateFilterTextForms.al:
    // GetRange_SixDigits_German). Eight digits read the same way with a four-digit year in both (DateFilterTextForms.al:
    // GetRange_SixDigits_German `eight_digits`); seven digits in an English session (P81DateFilters.al:SetFilter_Date_RangeForms
    // `01012026`, SetFilter_Date_InvalidTexts `0101202`).
    let digits_only = text.bytes().all(|b| b.is_ascii_digit());
    if (english || german) && digits_only {
        match text.len() {
            6 => {
                let spelled = format!("{}/{}/{}", &text[0..2], &text[2..4], &text[4..6]);
                if let Some(v) = crate::evaluate::read_text(&crate::value::RtType::Date, &spelled) {
                    return Ok(v);
                }
            }
            7 | 8 if english || text.len() == 8 => {
                let (m, d, y): (u32, u32, i32) = (
                    text[0..2].parse().unwrap_or(0),
                    text[2..4].parse().unwrap_or(0),
                    text[4..].parse().unwrap_or(0),
                );
                if (1..=12).contains(&m) && y >= 1 && d >= 1 && d <= crate::iso::days_in_month(y, m) {
                    return Ok(Value::Date(crate::format::day_from_civil(y, m, d), false));
                }
            }
            _ => {}
        }
    }
    // Four digits are month and day, the year is the work date's (DateFilterTextForms.al:GetRange_DateTextForms_English
    // `four_digits` 0201 with work date 2031 gives 2031-02-01, GetRange_FourDigits_OtherWorkDate with 2038 gives 2038; German
    // :GetRange_SixDigits_German `four_digits`; `0113` reads, `1301` does not).
    if (english || german) && text.len() == 4 && digits_only {
        if let Some(c) = ctx {
            let year = crate::format::civil_from_day(c.work_date).0;
            let spelled = format!("{}/{}/{year:04}", &text[0..2], &text[2..4]);
            if let Some(v) = crate::evaluate::read_text(&crate::value::RtType::Date, &spelled) {
                return Ok(v);
            }
        } else {
            return Err(needs_session());
        }
    }
    // A month and a day without a year, and slash forms with impossible numbers (`date_filter_forms`).
    if let Some(c) = ctx.filter(|_| english)
        && let Some(read) = crate::date_filter_forms::english_date(text, c.work_date)
    {
        return read.map_err(|()| format!("{INVALID_DATE}{t}"));
    }
    // The texts the corpus pins as invalid: a number of 4 to 8 digits that is no date, `0`, a text with a letter no
    // date formula uses (P81DateFilters.al:SetFilter_Date_InvalidTexts, EmptyAndZeroForms `0`, `000000`), and three two-digit
    // numbers split by dashes (DateFilterTextForms.al:GetRange_DayAboveTwelve_English `dd_mm_yy_separators`).
    let invalid_shape = (digits_only && matches!(text.len(), 4..=8))
        || dashed_short
        || text == "0"
        || text
            .chars()
            .any(|c| c.is_ascii_alphabetic() && !"CDMQTWY".contains(c.to_ascii_uppercase()));
    if german || (english && invalid_shape) {
        return Err(format!("{INVALID_DATE}{t}"));
    }
    Err(format!(
        "text filter value `{t}` for Date: no form pinned for this language (formulas and other region formats are not pinned, docs/probes-needed.md)"
    ))
}

/// A DateFormula filter value of one step without brackets: digits and an upper-case unit (`1M`, `10D`).
fn date_formula_step(t: &str) -> bool {
    let digits = t.trim_end_matches(['D', 'W', 'M', 'Q', 'Y']);
    t.len() == digits.len() + 1 && !digits.is_empty() && digits.bytes().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// record/filters/W4Filters.al UN_SetView_AtSignInValue_* (writer UN's corpus branch, commit d3bcc580): an `@` after the first
    /// character of a value is a normal character; `@` where a value starts still ignores case.
    #[test]
    fn an_at_sign_inside_a_value_is_a_character() {
        use crate::value::RtType as T;
        let txt = T::Text(Some(30));
        let p = |t: &str, ty: &T| parse_filter(t, &[], ty).unwrap_or_else(|e| panic!("{t}: {e}"));
        let m = |e: &FilterExpr, v: &str| e.matches_checked(&Value::Text(v.to_string().into()));
        let e = p("a@b.dk", &txt);
        assert!(m(&e, "a@b.dk") && !m(&e, "A@B.DK"));
        let e = p("@a@b.dk", &txt);
        assert!(m(&e, "a@b.dk") && m(&e, "A@B.DK") && !m(&e, "b@a.dk"));
        let e = p("*@b*", &txt);
        assert!(m(&e, "a@b.dk") && !m(&e, "A@B.DK") && !m(&e, "b@a.dk"));
        let e = p("ab@", &txt);
        assert!(m(&e, "ab@") && !m(&e, "ab"));
        let e = p("@abc|@xabc", &txt);
        assert!(m(&e, "ABC") && m(&e, "xAbc") && !m(&e, "@abc"));
        let e = p("AT1@", &T::Code(Some(20)));
        assert!(m(&e, "AT1@") && !m(&e, "AT1"));
    }

    /// record/crud/RcAFilters.al:SetFilter_Integer_Invalid and events/platform-triggers/PtObserve.al:Bad_Int, Bad_Decimal,
    /// Bad_Time: the value that fails is named with its type; texts the corpus does not pin stay refused.
    #[test]
    fn a_value_that_does_not_evaluate_names_its_type() {
        use crate::value::RtType as T;
        let err = |t: &str, ty: &T| parse_filter(t, &[], ty).err().unwrap_or_default();
        assert_eq!(err("abc", &T::Integer), format!("{NOT_EVALUATED}Integer\tabc"));
        assert_eq!(err("1..x", &T::Integer), format!("{NOT_EVALUATED}Integer\tx"));
        assert_eq!(
            err("3000000000", &T::Integer),
            format!("{NOT_EVALUATED}Integer\t3000000000")
        );
        assert_eq!(err("abc", &T::Decimal), format!("{NOT_EVALUATED}Decimal\tabc"));
        assert_eq!(err("abc", &T::Time), format!("{NOT_EVALUATED}Time\tabc"));
        // InvalidDecimalFilter.al:118: `1,5` reads as 15.
        assert!(parse_filter("1,5", &[], &T::Decimal).is_ok());
        assert!(!err("1,5,6", &T::Decimal).starts_with(NOT_EVALUATED));
        assert!(!err("13:00:00", &T::Time).starts_with(NOT_EVALUATED));
        let noon = parse_filter("12:00:00", &[], &T::Time).expect("noon");
        assert!(noon.matches_checked(&Value::Time(12 * 3_600_000 + 1)));
    }

    /// Microsoft's Tests-SCM and Tests-ERM filter Date and Guid fields with text they built with `Format`
    /// (`SetFilter("Posting Date", '..' + Format(D))`, `SetFilter(SystemId, Format(G))`): the text reads as `Evaluate`
    /// reads it (pinned by types/date-time/P143Date_EnPlain.al and the Guid tests); a
    /// formula or an empty text stays refused.
    #[test]
    fn a_date_or_guid_filter_reads_formatted_text_as_evaluate_does() {
        use crate::value::RtType as T;
        let date = parse_filter("01/17/27", &[], &T::Date).expect("a date in the region's short format");
        let d = Value::Date(crate::format::day_from_civil(2027, 1, 17), false);
        assert!(date.matches_checked(&d));
        assert!(!date.matches_checked(&Value::Date(0, false)));
        let range = parse_filter("..01/17/27", &[], &T::Date).expect("a range up to a date");
        assert!(range.matches_checked(&d));
        let guid = parse_filter("{261C0957-2EC9-802F-F3FB-808A6781620E}", &[], &T::Guid).expect("a Guid with braces");
        let g = Value::Guid(crate::ops::parse_guid("261C0957-2EC9-802F-F3FB-808A6781620E").unwrap());
        assert!(guid.matches_checked(&g));
        let closing = parse_filter("C01/17/27", &[], &T::Date).expect("a closing date as Format writes it");
        assert!(closing.matches_checked(&Value::Date(crate::format::day_from_civil(2027, 1, 17), true)));
        assert!(!closing.matches_checked(&d), "the closing date is not the date");
        assert!(parse_filter("1M", &[], &T::Date).is_err(), "a formula stays refused");
        // T and W need the session's dates (P143Date_EnFormulas.al).
        assert!(parse_filter("T", &[], &T::Date).is_err());
        let today = crate::format::day_from_civil(2026, 10, 15);
        let ctx = DateFilterContext {
            today,
            work_date: today + 1,
            language: 1033,
            ..Default::default()
        };
        with_date_context(ctx, || {
            let t = parse_filter("..t", &[], &T::Date).expect("a range up to today");
            assert!(
                t.matches_checked(&Value::Date(today, false)) && !t.matches_checked(&Value::Date(today + 1, false))
            );
            let w = parse_filter("W", &[], &T::Date).expect("the work date");
            assert!(w.matches_checked(&Value::Date(today + 1, false)));
            let cd = parse_filter("CD", &[], &T::Date).expect("the closing date of today");
            assert!(cd.matches_checked(&Value::Date(today, true)) && !cd.matches_checked(&Value::Date(today, false)));
        });
    }

    /// events/table-events/BdFilterEvents.al: the Time texts an English session reads, the Integer and Decimal texts with a
    /// types/P136Filter.al:354-380 (corpus master, TV_SetView_DateTime_LiteralTexts): a DateTime filter text in
    /// an English session reads `MM/DD/YY HH:MM[:SS[.f]] [AM|PM]`, `''` is the blank DateTime; a date alone stays refused.
    #[test]
    fn datetime_filter_texts() {
        use crate::value::RtType as T;
        let day = |d: &str| match crate::evaluate::read_text(&T::Date, d) {
            Some(Value::Date(n, _)) => i64::from(n),
            other => panic!("{d}: {other:?}"),
        };
        let at = |d: &str, h: i64, m: i64, s: i64, ms: i64| {
            Value::DateTime((day(d) - 1) * 86_400_000 + ((h * 60 + m) * 60 + s) * 1000 + ms + 1)
        };
        assert!(
            parse_filter("11/07/26 12:00 PM", &[], &T::DateTime).is_err(),
            "no session language: refused"
        );
        let ctx = DateFilterContext {
            today: 1,
            work_date: 1,
            language: 1033,
            ..Default::default()
        };
        with_date_context(ctx, || {
            for (text, v) in [
                ("11/07/26 12:00:00.033 PM", at("11/07/26", 12, 0, 0, 33)),
                ("11/03/26 12:00:00.08 PM", at("11/03/26", 12, 0, 0, 80)),
                ("11/03/26 12:00:00.8 PM", at("11/03/26", 12, 0, 0, 800)),
                ("01/15/24 12:00:00 PM", at("01/15/24", 12, 0, 0, 0)),
                ("01/15/24 12:00 PM", at("01/15/24", 12, 0, 0, 0)),
                ("11/03/26 12:00 AM", at("11/03/26", 0, 0, 0, 0)),
                ("11/03/26 12:00:11.767 PM", at("11/03/26", 12, 0, 11, 767)),
                ("11/07/26 12:00:00.033", at("11/07/26", 12, 0, 0, 33)),
                ("''", Value::DateTime(0)),
            ] {
                let f = parse_filter(text, &[], &T::DateTime).unwrap_or_else(|e| panic!("{text}: {e}"));
                assert!(f.matches_checked(&v), "{text}");
            }
            for text in [
                "11/07/26 1:00 PM",
                "11/07/26 13:00 PM",
                "11/07/26 12:00:00.0331 PM",
                "2026-11-07 12:00",
                "11/07/26 12:60 PM",
            ] {
                let e = parse_filter(text, &[], &T::DateTime).expect_err(text);
                assert!(!e.starts_with(NOT_EVALUATED), "{text} stays refused: {e}");
            }
        });
    }

    /// types/P136Filter.al:447-586 UN_DateTimeFilter_*: a date alone covers its whole day, other
    /// hours read on a 24-hour clock or with AM/PM, `15.01.25` raises the evaluate error.
    #[test]
    fn datetime_filter_date_alone_and_other_hours() {
        use crate::value::RtType as T;
        let ctx = DateFilterContext {
            today: 1,
            work_date: 1,
            language: 1033,
            ..Default::default()
        };
        with_date_context(ctx, || {
            let day = match crate::evaluate::read_text(&T::Date, "01/15/25") {
                Some(Value::Date(n, _)) => i64::from(n),
                other => panic!("{other:?}"),
            };
            let at = |d: i64, h: i64, m: i64, s: i64, ms: i64| {
                Value::DateTime((day + d - 1) * 86_400_000 + ((h * 60 + m) * 60 + s) * 1000 + ms + 1)
            };
            let rows = [
                (11, at(0, 0, 0, 0, 0)),
                (12, at(0, 8, 30, 0, 0)),
                (13, at(0, 8, 30, 30, 0)),
                (15, at(0, 17, 45, 10, 500)),
                (16, at(0, 17, 45, 0, 0)),
                (18, at(1, 0, 0, 0, 0)),
                (19, at(-1, 20, 0, 0, 0)),
            ];
            let ids = |text: &str| -> Vec<i32> {
                let f = parse_filter(text, &[], &T::DateTime).unwrap_or_else(|e| panic!("{text}: {e}"));
                rows.iter()
                    .filter(|(_, v)| f.matches_checked(v))
                    .map(|(i, _)| *i)
                    .collect()
            };
            for text in ["01/15/25", "1/15/25", "2025-01-15"] {
                assert_eq!(ids(text), [11, 12, 13, 15, 16], "{text}");
            }
            assert_eq!(ids("..01/15/25"), [11, 12, 13, 15, 16, 19]);
            assert_eq!(ids("01/15/25.."), [11, 12, 13, 15, 16, 18]);
            assert_eq!(ids("01/15/25..01/16/25"), [11, 12, 13, 15, 16, 18]);
            assert_eq!(ids("<01/15/25"), [19]);
            assert_eq!(ids("01/15/25 08:30"), [12]);
            assert_eq!(ids("01/15/25 08:30 AM"), [12]);
            assert_eq!(ids("01/15/25 08:30 PM"), Vec::<i32>::new());
            assert_eq!(ids("01/15/25 17:45:10.5"), [15]);
            assert_eq!(ids("01/15/25 05:45:10.5 PM"), [15]);
            assert_eq!(ids("01/15/25 00:00"), [11]);
            assert_eq!(ids(">01/15/25 08:30"), [13, 15, 16, 18]);
            // The text a view writes reads back.
            assert_eq!(ids("2025-01-15T17:45:10.5Z"), [15]);
            assert_eq!(ids("2025-01-15T00:00:00Z..2025-01-15T23:59:59Z"), [11, 12, 13, 15, 16]);
            for text in ["15.01.25", "15.01.25 08:30"] {
                let e = parse_filter(text, &[], &T::DateTime).expect_err(text);
                assert!(e.starts_with(NOT_EVALUATED), "{text}: {e}");
            }
            for text in [">01/15/25", "01/15/25..01/16/25 08:00", "01/15/25 17:45:10.500 PM"] {
                let e = parse_filter(text, &[], &T::DateTime).expect_err(text);
                assert!(!e.starts_with(NOT_EVALUATED), "{text} stays refused: {e}");
            }
        });
    }

    /// blank that raise the evaluate error, `1,234.5`, and `5..5` on an Integer showing both ends.
    #[test]
    fn bd_filter_texts_for_time_integer_and_decimal() {
        use crate::value::RtType as T;
        let at = |h: i64, m: i64, s: i64| Value::Time(((h * 60 + m) * 60 + s) * 1000 + 1);
        assert!(
            parse_filter("1", &[], &T::Time).is_err(),
            "no session language: refused"
        );
        let ctx = DateFilterContext {
            today: 1,
            work_date: 1,
            language: 1033,
            ..Default::default()
        };
        with_date_context(ctx, || {
            for (text, v) in [
                ("1", at(1, 0, 0)),
                ("1.5", at(1, 5, 0)),
                ("9:00", at(9, 0, 0)),
                ("13:00:00", at(13, 0, 0)),
                ("09:05:07", at(9, 5, 7)),
                ("12:00:00 PM", at(12, 0, 0)),
                ("1300", at(13, 0, 0)),
                ("120000", at(12, 0, 0)),
            ] {
                let f = parse_filter(text, &[], &T::Time).unwrap_or_else(|e| panic!("{text}: {e}"));
                assert!(f.matches_checked(&v), "{text}");
            }
            for text in ["0", "25:00:00", "1:00:00 PM", "12345"] {
                let e = parse_filter(text, &[], &T::Time).expect_err(text);
                assert!(!e.starts_with(NOT_EVALUATED), "{text} stays refused: {e}");
            }
        });
        for (text, ty) in [("1 2", T::Integer), ("1 000", T::Decimal), ("- 1", T::Decimal)] {
            let e = parse_filter(text, &[], &ty).expect_err(text);
            assert_eq!(not_evaluated_value(&e), Some(text), "{text}");
        }
        let d = parse_filter("1,234.5", &[], &T::Decimal).expect("a thousands comma");
        assert!(d.matches_checked(&Value::Decimal(rust_decimal::Decimal::new(12345, 1))));
        assert!(matches!(
            parse_filter("5..5", &[], &T::Integer),
            Ok(FilterExpr::RangeSame(Value::Integer(5)))
        ));
    }

    /// `@` before a braced Guid is the Guid format error (record/filters/Y77GuidFilter.al Record_Shapes5To6 `shape5`).
    #[test]
    fn a_guid_filter_with_an_at_sign_before_the_guid_is_invalid() {
        use crate::value::RtType as T;
        let e = parse_filter("@{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}", &[], &T::Guid).unwrap_err();
        assert!(e.starts_with(INVALID_GUID), "{e}");
        assert!(parse_filter("{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}", &[], &T::Guid).is_ok());
    }

    /// Microsoft's Library - ERM `GetDiscountPaymentTerm` filters a DateFormula field with `<>` and the empty text: there the
    /// empty text is the empty formula (record/Y70Probes.al:218-262, FilterDateFormula_EmptyText).
    #[test]
    fn a_date_formula_filter_reads_the_empty_text_as_the_empty_formula() {
        use crate::value::RtType as T;
        let not_blank = parse_filter("<>''", &[], &T::DateFormula).expect("<> the empty formula");
        assert!(!not_blank.matches_checked(&Value::DateFormula(String::new())));
        assert!(not_blank.matches_checked(&Value::DateFormula("<1M>".into())));
        // A formula without brackets is the stored formula (record/filters/DateFormulaOperatorFilterText.al:116-134, staging r6980).
        let one_month = parse_filter("1M", &[], &T::DateFormula).expect("a formula without brackets");
        assert!(one_month.matches_checked(&Value::DateFormula("<1M>".into())));
        assert!(!one_month.matches_checked(&Value::DateFormula("<1D>".into())));
    }

    /// An angle-bracketed formula is no DateFormula filter: the closing `>` is not expected (errors/lt-ms-refusals-136/
    /// P136Filter.al:P136_SetFilter_DateFormula `oneMAngles`, `oneDAngles`; record/Y70Probes.al:249-258).
    #[test]
    fn a_date_formula_filter_in_angle_brackets_does_not_expect_the_closing_bracket() {
        use crate::value::RtType as T;
        for text in ["<1M>", "<1D>", "<-CM+2D>"] {
            let e = parse_filter(text, &[], &T::DateFormula).expect_err(text);
            assert_eq!(e, did_not_expect(">"), "{text}");
        }
        // An open bracket is a less-than filter on a formula, as `<=1M` is (DateFormulaOperatorFilterText.al:130).
        assert!(parse_filter("<1M", &[], &T::DateFormula).is_ok());
        // A text of letters that is no unit needs a time unit (Y77GuidFilter.al DateFormula_TextFilter_SetFilterOther `sf_plain`).
        let e = parse_filter("abc", &[], &T::DateFormula).unwrap_err();
        assert!(
            e.contains(
                "The date formula 'ABC' should include a time unit.\\\\Time units can be: D,WD,W,M,Q, or Y.\\C, meaning"
            ),
            "{e}"
        );
    }

    /// A quoted empty text in a Date filter is the blank date: `''` finds the blank rows, `<>''` the others
    /// (corpus record/load-fields/P81DateFilters.al:193-194, z9 and z10), and `''..01/01/26` is the range
    /// from the blank date (same file, :294 and :308, where %1 holds 0D). Item's GetFilter("Date Filter") gives that text back
    /// to SetFilter in Transfer Line.FilterLinesWithItemToPlan.
    #[test]
    fn a_date_filter_reads_a_quoted_empty_text_as_the_blank_date() {
        use crate::value::RtType as T;
        let blank = Value::Date(0, false);
        let day = Value::Date(crate::format::day_from_civil(2026, 1, 1), false);
        let eq = parse_filter("''", &[], &T::Date).expect("''");
        assert!(eq.matches_checked(&blank) && !eq.matches_checked(&day));
        let ne = parse_filter("<>''", &[], &T::Date).expect("<>''");
        assert!(!ne.matches_checked(&blank) && ne.matches_checked(&day));
        let upto = parse_filter("''..01/01/26", &[], &T::Date).expect("''..01/01/26");
        assert!(upto.matches_checked(&blank) && upto.matches_checked(&day));
    }

    #[test]
    fn range_and_comparisons() {
        let r = FilterExpr::Range(Some(Value::Integer(2)), Some(Value::Integer(4)));
        assert!(!r.matches_checked(&Value::Integer(1)));
        assert!(r.matches_checked(&Value::Integer(2)));
        assert!(r.matches_checked(&Value::Integer(4)));
        assert!(!r.matches_checked(&Value::Integer(5)));
        assert!(FilterExpr::Range(None, Some(Value::Integer(3))).matches_checked(&Value::Integer(-9)));
        assert!(
            FilterExpr::Or(vec![
                FilterExpr::Eq(Value::Integer(1)),
                FilterExpr::Gt(Value::Integer(8))
            ])
            .matches_checked(&Value::Integer(9))
        );
        assert!(!FilterExpr::Ne(Value::Text("a".into())).matches_checked(&Value::Text("a".into())));
    }

    // record/filters/UcQuotedValues.al SetView_FilterGreaterB `view_txt_plain_ids` and PageAndQuery_FilterGreaterB
    // `page_txt_ids`: `>B` on a Text field keeps C, zzz and Diana's (Meals), not a>b or a> (the collation order, letters
    // without regard to case first).
    #[test]
    fn a_text_comparison_filter_uses_the_collation_order() {
        let gt = FilterExpr::Gt(Value::Text("B".into()));
        for (v, want) in [
            ("a>b", false),
            ("a>", false),
            ("C", true),
            ("zzz", true),
            ("Diana's (Meals)", true),
            (">b", false),
            ("B", false),
        ] {
            assert_eq!(gt.matches_checked(&Value::Text(v.into())), want, "{v}");
        }
        assert!(
            FilterExpr::Range(Some(Value::Text("a".into())), Some(Value::Text("B".into())))
                .matches_checked(&Value::Text("b".into()))
        );
        // Equality stays exact.
        assert!(!FilterExpr::Eq(Value::Text("b".into())).matches_checked(&Value::Text("B".into())));
    }

    #[test]
    fn the_or_group_is_alternatives_and_the_other_groups_all_apply() {
        let ff = |field, v: i32, group| FieldFilter {
            lang: crate::filter::filter_language(),
            field,
            expr: FilterExpr::Eq(Value::Integer(v)),
            group,
        };
        let row = [Value::Integer(1), Value::Integer(2), Value::Integer(3)];
        // Either of two fields matches: the row passes.
        assert!(row_passes_checked(&row, &[ff(0, 9, OR_GROUP), ff(1, 2, OR_GROUP)]));
        assert!(!row_passes_checked(&row, &[ff(0, 9, OR_GROUP), ff(1, 9, OR_GROUP)]));
        // A filter of group 0 still has to match next to the OR group.
        assert!(!row_passes_checked(&row, &[ff(2, 9, 0), ff(1, 2, OR_GROUP)]));
        assert!(row_passes_checked(&row, &[ff(2, 3, 0), ff(1, 2, OR_GROUP)]));
        assert!(row_passes_checked(&row, &[]));
    }

    use crate::value::RtType;

    fn pf(text: &str, args: &[Value], ty: RtType) -> FilterExpr {
        parse_filter(text, args, &ty).unwrap_or_else(|e| panic!("{text}: {e}"))
    }
    fn ints(f: &FilterExpr, vals: &[i32]) -> Vec<bool> {
        vals.iter().map(|v| f.matches_checked(&Value::Integer(*v))).collect()
    }
    fn texts(f: &FilterExpr, vals: &[&str]) -> Vec<bool> {
        vals.iter()
            .map(|v| f.matches_checked(&Value::Code(v.to_string().into())))
            .collect()
    }

    fn opt(text: &str, members: &[&str]) -> Result<FilterExpr, String> {
        let m: Vec<String> = members.iter().map(|x| x.to_string()).collect();
        parse_filter(text, &[], &RtType::Option(m.into()))
    }

    // types/TestEnumBlankOptionFilterTests.Codeunit.al: option member names match with both
    // sides trimmed and case ignored, zero-length member names are skipped, and only then
    // is the text read as an ordinal.
    #[test]
    fn option_filters_match_member_names_trimmed_and_skip_zero_length_names() {
        let m = ["", "Draft", "Active"];
        let ord = |f: &FilterExpr, o: i32| f.matches_checked(&Value::Option(o, Vec::<String>::new().into()));
        let f = opt(" draft ", &m).unwrap();
        assert!(ord(&f, 1) && !ord(&f, 2));
        // A space-named member (`OptionMembers = " ",Draft`) is not zero length: '' trims to it.
        let f = opt("''", &[" ", "Draft"]).unwrap();
        assert!(ord(&f, 0) && !ord(&f, 1));
        // A zero-length member name is skipped, so '' names no member and is no ordinal.
        assert!(opt("''", &m).is_err());
        // The ordinal fallback.
        let f = opt("2", &m).unwrap();
        assert!(ord(&f, 2) && !ord(&f, 1));
        assert!(opt("Nope", &m).is_err());
    }

    #[test]
    fn filter_ranges_and_comparisons() {
        assert_eq!(
            ints(&pf("20..30", &[], RtType::Integer), &[19, 20, 30, 31]),
            [false, true, true, false]
        );
        assert_eq!(ints(&pf("..5", &[], RtType::Integer), &[-1, 5, 6]), [true, true, false]);
        assert_eq!(ints(&pf("5..", &[], RtType::Integer), &[4, 5, 99]), [false, true, true]);
        assert_eq!(ints(&pf(">=1", &[], RtType::Integer), &[0, 1]), [false, true]);
        assert_eq!(ints(&pf(">0", &[], RtType::Integer), &[0, 1]), [false, true]);
        assert_eq!(ints(&pf("<>3", &[], RtType::Integer), &[2, 3]), [true, false]);
        assert_eq!(ints(&pf("1|3", &[], RtType::Integer), &[1, 2, 3]), [true, false, true]);
    }

    #[test]
    fn filter_placeholders_take_typed_values() {
        assert_eq!(
            ints(&pf("<%1", &[Value::Integer(10)], RtType::Integer), &[9, 10]),
            [true, false]
        );
        assert_eq!(
            ints(
                &pf("%1|%2", &[Value::Integer(4), Value::Integer(7)], RtType::Integer),
                &[4, 5, 7]
            ),
            [true, false, true]
        );
        let f = pf(
            "%1..%2",
            &[Value::Date(10, false), Value::Date(20, false)],
            RtType::Date,
        );
        assert!(f.matches_checked(&Value::Date(15, false)) && !f.matches_checked(&Value::Date(21, false)));
    }

    #[test]
    fn filter_wildcards_on_code_follow_code_normalisation() {
        assert_eq!(
            texts(&pf("AP*", &[], RtType::Code(Some(20))), &["APPLE", "XAP"]),
            [true, false]
        );
        // Values convert to the field type: on a Code field 'ap*' means 'AP*'.
        assert_eq!(texts(&pf("ap*", &[], RtType::Code(Some(20))), &["APPLE"]), [true]);
        assert_eq!(
            texts(&pf("A", &[], RtType::Code(Some(20))), &["A", "AB"]),
            [true, false]
        );
    }

    #[test]
    fn filter_empty_and_quoted_literals() {
        let f = pf("''", &[], RtType::Text(None));
        assert!(f.matches_checked(&Value::Text(String::new().into())) && !f.matches_checked(&Value::Text("x".into())));
        let f = pf("<>''", &[], RtType::Text(None));
        assert!(!f.matches_checked(&Value::Text(String::new().into())) && f.matches_checked(&Value::Text("x".into())));
        let f = pf("'Smith & Sons'", &[], RtType::Text(None));
        assert!(f.matches_checked(&Value::Text("Smith & Sons".into())));
        let f = pf("'(Travel|Meals)'", &[], RtType::Text(None));
        assert!(f.matches_checked(&Value::Text("(Travel|Meals)".into())));
    }

    fn enum_members(_: u32) -> Vec<(i32, String)> {
        vec![
            (0, " ".into()),
            (1, "Initial Entry".into()),
            (2, "Payment Discount (VAT Excl.)".into()),
            (3, "Application".into()),
        ]
    }

    fn enum_hits(text: &str) -> Vec<i32> {
        let f = parse_filter_with(text, &[], &RtType::Enum(7), &|id| Ok(enum_members(id)))
            .unwrap_or_else(|e| panic!("{text}: {e}"));
        (0..4).filter(|v| f.matches_checked(&Value::Enum(7, *v))).collect()
    }

    // corpus record/TestCalcFormulaQuotedFilterHeader.Table.al:21-52: member names with
    // spaces, parentheses and a blank member, once the quotes are single quotes.
    #[test]
    fn enum_filters_take_member_names_and_ordinals() {
        assert_eq!(enum_hits("'Initial Entry'"), [1]);
        assert_eq!(enum_hits("'Payment Discount (VAT Excl.)'"), [2]);
        assert_eq!(enum_hits("Application | 'Initial Entry'"), [1, 3]);
        assert_eq!(enum_hits("<> ' '"), [1, 2, 3]);
        assert_eq!(enum_hits("' '"), [0]);
        assert_eq!(enum_hits("application"), [3]);
        assert_eq!(enum_hits("2"), [2]);
        assert!(parse_filter_with("Nope", &[], &RtType::Enum(7), &|id| Ok(enum_members(id))).is_err());
    }

    // corpus types/TestEnumBlankOptionFilterTests.Codeunit.al:122,160,188: the option matcher
    // trims both sides, skips zero-length member names, and falls back to an ordinal.
    #[test]
    fn option_filters_trim_both_sides_and_skip_zero_length_member_names() {
        let opt = |members: &[&str]| RtType::Option(members.iter().map(|m| m.to_string()).collect());
        let hits = |ty: &RtType, text: &str| -> Result<Vec<i32>, String> {
            let f = parse_filter(text, &[], ty)?;
            let names: Vec<String> = match ty {
                RtType::Option(m) => m.to_vec(),
                _ => unreachable!(),
            };
            Ok((0..names.len() as i32)
                .filter(|i| f.matches_checked(&Value::Option(*i, names.clone().into())))
                .collect())
        };
        // A member named with one space is matched by the empty string.
        let space = opt(&[" ", "Draft", "Active"]);
        assert_eq!(hits(&space, "''").unwrap(), [0]);
        assert_eq!(hits(&space, "' Draft '").unwrap(), [1]);
        assert_eq!(hits(&space, "active").unwrap(), [2]);
        // A member with no name at all is skipped, so the empty string names nothing.
        let empty = opt(&["", "Draft", "Active"]);
        assert!(hits(&empty, "''").is_err());
        assert_eq!(hits(&empty, "2").unwrap(), [2]);
        assert!(hits(&empty, "Purple").is_err());
    }

    #[test]
    fn option_filter_matches_member_names_trimmed_case_blind_and_skips_zero_length_members() {
        // corpus types/TestEnumBlankOptionFilterTests.Codeunit.al:1-12, :187-205
        let members: Vec<String> = vec!["".into(), "Draft".into(), " ".into()];
        let ty = RtType::Option(members.into());
        let ord = |f: &FilterExpr| {
            (0..3)
                .map(|i| f.matches_checked(&Value::Option(i, Default::default())))
                .collect::<Vec<_>>()
        };
        assert_eq!(ord(&pf(" draft ", &[], ty.clone())), [false, true, false]);
        assert_eq!(ord(&pf("'DRAFT'", &[], ty.clone())), [false, true, false]);
        // The empty string names the blank member " " (trimmed), never the zero-length one.
        assert_eq!(ord(&pf("''", &[], ty.clone())), [false, false, true]);
        let zero_length_only = RtType::Option(vec!["".into(), "Draft".into()].into());
        assert!(
            parse_filter("''", &[], &zero_length_only).is_err(),
            "'' names no member here"
        );
    }

    #[test]
    fn at_and_wildcards_stay_active_inside_a_quoted_expression() {
        // record/TestFilterQuotedLiterals.al: `'@*Smith & Sons*'` ("quote so &, (, ), =, | are literal,
        // and keep @ and * active"): a case-insensitive "contains" filter on a value with operators in it.
        let f = pf("'@*Smith & Sons*'", &[], RtType::Text(None));
        assert!(f.matches_checked(&Value::Text("Mr smith & sons LLC".into())));
        assert!(!f.matches_checked(&Value::Text("Acme Co".into())));
        let f = pf("'@*(2025)*'", &[], RtType::Text(None));
        assert!(f.matches_checked(&Value::Text("Expenses (2025) - Diana".into())));
        assert!(!f.matches_checked(&Value::Text("Expenses 2025".into())));
        // Without @ the quoted wildcard still matches case-sensitively.
        let f = pf("'*A|B*'", &[], RtType::Text(None));
        assert!(f.matches_checked(&Value::Text("xA|By".into())) && !f.matches_checked(&Value::Text("xa|by".into())));
        // A quoted expression without wildcards is a plain literal.
        let f = pf("'Smith & Sons'", &[], RtType::Text(None));
        assert!(
            f.matches_checked(&Value::Text("Smith & Sons".into()))
                && !f.matches_checked(&Value::Text("smith & sons".into()))
        );
    }

    #[test]
    fn filter_forms_not_pinned_yet_are_errors() {
        assert!(
            parse_filter("01-15-24", &[], &RtType::Date).is_err(),
            "text dates depend on the region"
        );
    }

    #[test]
    fn key_order() {
        assert_eq!(
            key_cmp(
                &[Value::Integer(1), Value::Text("b".into())],
                &[Value::Integer(1), Value::Text("a".into())]
            ),
            Ordering::Greater
        );
        assert_eq!(
            key_cmp(&[Value::Code("A".into())], &[Value::Code("A".into())]),
            Ordering::Equal
        );
    }
}

#[cfg(test)]
mod placeholder_wildcard_tests {
    use super::placeholder_next_to_wildcard as p;

    #[test]
    fn finds_a_placeholder_next_to_a_wildcard() {
        for t in ["%1*", "*%1*", "@*%1*", "*%1", "?%1", "<>%1*"] {
            assert!(p(t), "{t}");
        }
        for t in ["%1", "%1&*a*", "a*|%1", "*", "%"] {
            assert!(!p(t), "{t}");
        }
    }
}

#[cfg(test)]
mod compare_error_tests {
    use super::FilterExpr;
    use crate::value::Value;

    #[test]
    fn a_value_that_cannot_be_compared_is_reported() {
        let unknown = Value::Unknown("not known".into());
        let f = FilterExpr::Or(vec![
            FilterExpr::Eq(Value::Text("A".into())),
            FilterExpr::Like("B*".into(), false),
        ]);
        assert!(f.compare_error(&unknown).is_some());
        assert!(!f.matches_checked(&unknown));
    }

    #[test]
    fn a_value_that_compares_is_not_reported() {
        let f = FilterExpr::Or(vec![
            FilterExpr::Eq(Value::Code("A".into())),
            FilterExpr::Like("B*".into(), false),
        ]);
        assert!(f.compare_error(&Value::Code("C".into())).is_none());
        assert!(f.compare_error(&Value::Code("".into())).is_none());
    }
}
