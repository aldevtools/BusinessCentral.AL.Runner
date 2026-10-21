//! Record filters as data (shared by the VM and the store).

use crate::describe::Describe;
use crate::ops::compare;
use crate::value::Value;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub enum FilterExpr {
    /// `SetRange(F, A, B)` and `SetRange(F, A)` (as A..A); open ends are None.
    Range(Option<Value>, Option<Value>),
    Eq(Value),
    /// A range typed as text whose two ends are the same value (`SetFilter(D, '010126..010126')`): it finds what `Eq` finds, and
    /// `GetFilter` writes it back as `a..a`, where `SetRange(D, a, a)` gives `a` (record/lt-loadfields-pagesource-validate/
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
    /// (record/lt-bl-wildcard-getfilter/WildcardGetFilter.al:106-173, Forms_CodeField and Forms_TextField).
    NotLike(String, bool),
}

impl FilterExpr {
    /// Whether a field value passes. Values that cannot be compared do not pass.
    pub fn matches(&self, v: &Value) -> bool {
        // A text of spaces only is blank to a filter: `SetRange(Txt, ' ')` finds the rows holding '', ' ' and '  ', and
        // `<> ' '` finds the others (filters/W4Filters.al:BlankOnlyFilterValues_MatchOnlyBlankRows, BC 27.5 and 29.0).
        // Other trailing spaces are not pinned.
        let v = &*blank_if_spaces(v);
        let cmp = |x: &Value| {
            let x = blank_if_spaces(x);
            crate::ops::hyphen_filter_order(v, &x).or_else(|| compare(v, &x).ok())
        };
        match self {
            FilterExpr::Range(lo, hi) => {
                lo.as_ref()
                    .is_none_or(|l| matches!(cmp(l), Some(Ordering::Equal | Ordering::Greater)))
                    && hi
                        .as_ref()
                        .is_none_or(|h| matches!(cmp(h), Some(Ordering::Equal | Ordering::Less)))
            }
            FilterExpr::Eq(x) | FilterExpr::RangeSame(x) => cmp(x) == Some(Ordering::Equal),
            FilterExpr::Ne(x) => matches!(cmp(x), Some(Ordering::Less | Ordering::Greater)),
            FilterExpr::Lt(x) => cmp(x) == Some(Ordering::Less),
            FilterExpr::Le(x) => matches!(cmp(x), Some(Ordering::Less | Ordering::Equal)),
            FilterExpr::Gt(x) => cmp(x) == Some(Ordering::Greater),
            FilterExpr::Ge(x) => matches!(cmp(x), Some(Ordering::Greater | Ordering::Equal)),
            FilterExpr::Like(p, ci) => crate::native::text_of(v).is_some_and(|t| like(&t, p, *ci)),
            FilterExpr::NotLike(p, ci) => crate::native::text_of(v).is_some_and(|t| !like(&t, p, *ci)),
            FilterExpr::Or(xs) => xs.iter().any(|x| x.matches(v)),
            FilterExpr::And(xs) => xs.iter().all(|x| x.matches(v)),
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
        // Trailing blanks do not count in a filter comparison: `SetRange(K, 'abc  ')` finds the row 'abc' (record/lt-key-case-trailing-blank/
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
    parse_filter_with(text, args, ty, &|_| Vec::new())
}

/// Like [`parse_filter`], with the (ordinal, name) members of an enum object
/// so that filters on enum fields can name members.
/// Prefix of the parse error for a word a Boolean filter does not accept; the rest is the word.
pub const INVALID_BOOLEAN: &str = "invalid Boolean filter value: ";
/// The prefix of the error for a text no Guid filter reads (types/lt-in-precedence-date-guid-filters/P143Guid_Filters.al:P143_GuidInvalid).
pub const INVALID_GUID: &str = "invalid Guid filter value: ";
/// The prefix of the error for a date text no Date filter reads; the failing value follows (types/lt-in-precedence-date-guid-filters/
/// P143Date_DePlain.al: `Das Format Date „17.01.27“ ...`).
pub const INVALID_DATE: &str = "invalid Date filter value: ";
/// The prefix of the error for a Date filter text with an empty side; the English message follows (record/lt-loadfields-pagesource-validate/
/// P81DateFilters.al:SetFilter_Date_RangeForms `..`, SetFilter_Date_InvalidTexts `|`, `&`, `||`).
pub const FILTER_SYNTAX: &str = "invalid Date filter syntax: ";

/// The prefix of the error for a filter value BC cannot evaluate into the field's type: the type's name, a tab, the value
/// (`The value "abc" can't be evaluated into type Integer.`, record/lt-record-read/RcAFilters.al:SetFilter_Integer_Invalid,
/// events/lt-platform-triggers/PtObserve.al:Bad_Int, Bad_Decimal, Bad_Time).
pub const NOT_EVALUATED: &str = "filter value not evaluated: ";

/// The upper case of a Code filter value. Under Turkish or Azerbaijani a value with `i` is not pinned (only an assignment to a Code
/// is, text/lt-text-functions/CaseConv.Codeunit.al:280-310) and is refused.
fn filter_upper(s: &str) -> Result<String, String> {
    if crate::ops::turkic_code() && s.contains('i') {
        return Err("a Code filter value with `i` under Turkish or Azerbaijani (not pinned)".into());
    }
    Ok(s.to_uppercase())
}

fn not_evaluated(ty: &str, text: &str) -> String {
    format!("{NOT_EVALUATED}{ty}\t{text}")
}

/// Whether `text` is one word of ASCII letters, the only text a Decimal, Time or DateTime filter is pinned to refuse with the
/// evaluate error (PtObserve.al: `abc`).
pub fn letters_only(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_alphabetic())
}

/// Whether `text` is one word of ASCII letters and hyphens (`NOT-A-NUMBER`, `-`, `abc`): the texts a Decimal, BigInteger, Duration
/// and Time filter are pinned to refuse with the evaluate error (record/lt-bl-invalid-decimal-filter/InvalidDecimalFilter.al:71, :96,
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
/// record/lt-bl-invalid-decimal-filter/InvalidDecimalFilter.al:92-126 (Decimal_Forms): letters and hyphens (`abc`, `x`, `-`,
/// `NOT-A-NUMBER`), `*`, `1;5` and `1.2.3` raise the error; `1,5` reads as 15 (the comma is dropped) and `1e3` as 1000.
fn decimal_filter_text(text: &str) -> Option<Result<rust_decimal::Decimal, ()>> {
    if let Ok(d) = text.parse() {
        return Some(Ok(d));
    }
    if letters_or_hyphens(text) || text == "*" || digit_groups(text, ';', 2) || digit_groups(text, '.', 3) {
        return Some(Err(()));
    }
    if digit_groups(text, ',', 2) {
        return text.replace(',', "").parse().ok().map(Ok);
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

/// What the text of a Date filter needs from the session: today, the work date and the language of the messages.
#[derive(Debug, Clone, Copy)]
pub struct DateFilterContext {
    pub today: i32,
    pub work_date: i32,
    pub language: i32,
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
    enums: &dyn Fn(u32) -> Vec<(i32, String)>,
) -> Result<FilterExpr, String> {
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
    let toks = tokenize(text, args)?;
    let mut p = Parser { toks, i: 0, ty, enums };
    let e = p.or()?;
    if p.i != p.toks.len() {
        return Err(format!("unexpected {} in filter `{text}`", p.toks[p.i].describe()));
    }
    Ok(e)
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

fn tokenize(text: &str, args: &[Value]) -> Result<Vec<Tok>, String> {
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
                '@' => Some("@"),
                _ => None,
            });
        if let Some(op) = op {
            flush(&mut raw, &mut out);
            out.push(Tok::Op(op));
            i += op.len();
        // A quote opens a literal only where a value starts; inside a word it is a character (record/lt-record-read/RcAMisc.al:
        // GetView_Field_Types `quote`: `it's`).
        } else if c[i] == '\'' && raw.trim().is_empty() {
            flush(&mut raw, &mut out);
            let mut lit = String::new();
            i += 1;
            // A placeholder is replaced inside quotes only when it is the whole quoted text: `'%1'` takes the argument, `'x%1y'` and
            // `'%1%2'` stay as written (record/lt-filter-placeholder-in/FilterPlaceholderInQuotes.al:P169_QuotedPlaceholder_InsideLongerQuotedText;
            // the filter text of Gen. Jnl.-Show CT Entries filters a blank currency as a quoted blank). A text or an Integer argument is put
            // in as it reads; any other type is not pinned.
            loop {
                match c.get(i) {
                    None => return Err("unterminated quote in filter".into()),
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
            flush(&mut raw, &mut out);
            let n = c[i + 1].to_digit(10).expect("digit") as usize;
            // A placeholder right before a wildcard is not put in after `<>`: `<>%1*` with `A` reads back as `<>%1*` and keeps every
            // row (record/lt-bl-wildcard-getfilter/WildcardGetFilter.al:188, :198). Elsewhere it is not pinned.
            if c.get(i + 2).is_some_and(|w| matches!(w, '*' | '?')) {
                if matches!(out.last(), Some(Tok::Op("<>"))) && raw.trim().is_empty() {
                    raw.push('%');
                    raw.push(c[i + 1]);
                    i += 2;
                    continue;
                }
                return Err(format!("placeholder %{n} right before a wildcard (not pinned)"));
            }
            let arg = args
                .get(n.wrapping_sub(1))
                .cloned()
                .ok_or("filter placeholder without an argument")?;
            // A text substituted for a placeholder is TEXT, not a filter, except that `*` and `?` stay
            // wildcards and a leading `@` ignores case. `&`, `|`, `(`, `)`, `<`, `=` and `..` in it make
            // BC quote the whole value (record/TestSetFilterSpecialTextContracts.al, BC 27.5); `>` is
            // not pinned, nor a text with both a wildcard and one of those.
            if let Value::Text(t) | Value::Code(t) = &arg {
                let (at, body) = match t.strip_prefix('@') {
                    Some(rest) => (true, rest),
                    None => (false, t.as_str()),
                };
                let special = body.contains(['&', '|', '(', ')', '<', '=']) || body.contains("..");
                let wildcard = body.contains(['*', '?']);
                if body.contains('>') || (special && wildcard) || body.starts_with('@') {
                    let ch = body.chars().find(|c| ">*?@".contains(*c)).unwrap_or('>');
                    return Err(format!(
                        "placeholder %{n} holds a text with the filter character `{ch}`: how BC quotes it is not pinned"
                    ));
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
    ty: &'a crate::value::RtType,
    enums: &'a dyn Fn(u32) -> Vec<(i32, String)>,
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

    fn typed(&self, t: Tok) -> Result<Value, String> {
        use crate::value::RtType as T;
        let text = match t {
            // An Enum value given for an Option column through SetFilter's %1 filters by its ordinal and shows under the Enum's own name
            // (types/lt-filter-enum-relation-duration-guid/B_EnumOptionVariant.al:Filter_OptionColumn_SetRange_EnumValue, `filter_green_filter`
            // is `Green`; SetRange shows the Option's member, `range_green_filter` is `One`).
            Tok::Typed(v @ Value::Enum(..)) if matches!(self.ty, T::Option(_)) => return Ok(v),
            Tok::Typed(v) => return crate::ops::convert(v, self.ty).map_err(|e| e.describe()),
            // A quoted empty text is the blank date (corpus record/lt-loadfields-pagesource-validate/P81DateFilters.al:193-194,
            // :308). The unquoted empty text stays refused.
            Tok::Quoted(s) if s.is_empty() && matches!(self.ty, T::Date) => return Ok(Value::Date(0, false)),
            Tok::Raw(s) | Tok::Quoted(s) | Tok::Arg(s) => s,
            Tok::Op(o) => return Err(format!("operator `{o}` where a value was expected")),
        };
        Ok(match self.ty {
            T::Text(_) => Value::Text(text.into()),
            T::Code(_) => Value::Code(filter_upper(text.trim())?.into()),
            // Any text that is no Integer: letters, a range end `x`, `1.5`, a number past the range (RcAFilters.al:392-414, PtObserve.al:277).
            // A text with blank space in it is not pinned.
            T::Integer => Value::Integer(text.parse().map_err(|_| {
                if text.contains(char::is_whitespace) {
                    format!("`{text}` is not an Integer")
                } else {
                    not_evaluated("Integer", &text)
                }
            })?),
            // A word of letters and hyphens raises the evaluate error (record/lt-bl-invalid-decimal-filter/InvalidDecimalFilter.al:134,
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
            // A Time filter of letters raises the evaluate error (PtObserve.al:262), also with hyphens (InvalidDecimalFilter.al:137,
            // `NOT-A-NUMBER`); `12:00:00` is noon (PtObserve.al:268, the text the MakeTimeFilter subscriber writes). Other times are not pinned.
            T::Time if letters_or_hyphens(text.trim()) => return Err(not_evaluated("Time", text.trim())),
            T::Time if is_noon_text(text.trim()) => {
                let t = text.trim();
                let (m, s) = (t[3..5].parse::<i64>().unwrap_or(0), t[6..8].parse::<i64>().unwrap_or(0));
                Value::Time(12 * 3_600_000 + m * 60_000 + s * 1000 + 1)
            }
            T::DateTime if letters_only(text.trim()) => return Err(not_evaluated("DateTime", text.trim())),
            // Format(false) = 'No' is used as a filter value in the corpus. The words BC accepts,
            // in any case, are Yes, True, 1 and No, False, 0
            // (record/TestFilterTextBooleanFindContracts.al:103, SetFilter_BooleanField_AcceptedWords).
            T::Boolean if ["yes", "true", "1"].contains(&text.to_lowercase().as_str()) => Value::Boolean(true),
            T::Boolean if ["no", "false", "0"].contains(&text.to_lowercase().as_str()) => Value::Boolean(false),
            T::Boolean => return Err(format!("{INVALID_BOOLEAN}{text}")),
            // The matcher trims both sides and skips zero-length member names (corpus
            // types/TestEnumBlankOptionFilterTests.Codeunit.al:122,188).
            T::Option(m) => match m
                .iter()
                .position(|x| !x.is_empty() && x.trim().eq_ignore_ascii_case(text.trim()))
            {
                Some(i) => Value::Option(i as i32, m.clone().into()),
                None => Value::Option(
                    text.parse().map_err(|_| format!("`{text}` is not an option member"))?,
                    m.clone().into(),
                ),
            },
            T::Enum(id) => {
                // The list holds the captions first, then the names, so a caption wins over a name that spells the same
                // (corpus record/lt-enum-filter-caption/EnumFilterByCaption.al: SetFilter_CaptionEqualsOtherName_Result).
                let named = (self.enums)(*id)
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
            // A date in a filter (types/lt-in-precedence-date-guid-filters/P143Date_EnPlain.al, _EnRanges.al, _EnFormulas.al, _DePlain.al,
            // _DeRanges.al, _DeFormulas.al): month, day and year in that order with `/` (and `.` when the language is German), a closing date
            // as `C` and a date, `T` and `W` (any case) for today and the work date, and in English `CD` the closing date of today, `cd` and
            // `0D` today. The empty text is no filter. A text that is none of these is an error in German (its text is pinned there);
            // in any other language it is refused.
            T::Date if !text.trim().is_empty() && !text.contains(['<', '>']) => date_filter_value(text.trim())?,
            // The empty text is the empty formula (Microsoft's Library - ERM `GetDiscountPaymentTerm`).
            T::DateFormula if text.trim().is_empty() => {
                crate::assumptions::assume(crate::assumptions::Assumption::FilterEmptyTextDateFormula);
                Value::DateFormula(String::new())
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

    /// `<>` before a wildcard pattern on a Text or Code field (record/lt-bl-wildcard-getfilter/WildcardGetFilter.al:106-202):
    /// `<>A*`, `<>*`, `<>?` and `<>@a*` keep the texts the pattern does not match; a placeholder that holds exactly `*` is the
    /// literal text (`<>%1` with `*` reads back as `<>'*'` and keeps every row). A quoted pattern, a placeholder holding another
    /// wildcard text and `<>@` before a text without a wildcard are not pinned. `None`: not a pattern, read as before.
    fn not_pattern(&mut self) -> Result<Option<FilterExpr>, String> {
        use crate::value::RtType as T;
        if !matches!(self.ty, T::Text(_) | T::Code(_)) {
            return Ok(None);
        }
        let ci = self.peek_op() == Some("@");
        let at = self.i + usize::from(ci);
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
            _ if ci => Err("`<>@` before a value without a wildcard (not pinned)".into()),
            Some(Tok::Arg(s)) if s == "*" => Ok(None),
            Some(Tok::Arg(s)) if wild(s) => Err(format!("`<>%n` with the wildcard text `{s}` (not pinned)")),
            Some(Tok::Quoted(s)) if wild(s) || s.starts_with('@') => {
                Err(format!("`<>` before the quoted pattern `{s}` (not pinned)"))
            }
            _ => Ok(None),
        }
    }

    fn term(&mut self) -> Result<FilterExpr, String> {
        match self.peek_op() {
            Some("(") => {
                self.i += 1;
                let e = self.or()?;
                if self.peek_op() != Some(")") {
                    return Err("missing `)` in filter".into());
                }
                self.i += 1;
                Ok(e)
            }
            Some("@") => {
                self.i += 1;
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
                let v = match self.value() {
                    Some(v) => self.typed(v)?,
                    None if matches!(self.ty, crate::value::RtType::Text(_) | crate::value::RtType::Code(_)) => {
                        Value::Text(String::new().into())
                    }
                    None => return Err(format!("value expected after `{op}`")),
                };
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
                Ok(FilterExpr::Range(None, Some(self.typed(hi)?)))
            }
            _ => {
                let Some(lo) = self.value() else {
                    // An empty filter value (`''` was a quoted empty literal and is a value).
                    return Err("empty filter term".into());
                };
                if self.peek_op() == Some("..") {
                    self.i += 1;
                    let lo = self.typed(lo)?;
                    let hi = match self.value() {
                        Some(h) => Some(self.typed(h)?),
                        None => None,
                    };
                    if self.date()
                        && hi
                            .as_ref()
                            .is_some_and(|h| compare(h, &lo).ok() == Some(Ordering::Equal))
                    {
                        return Ok(FilterExpr::RangeSame(lo));
                    }
                    return Ok(FilterExpr::Range(Some(lo), hi));
                }
                self.single(lo, false)
            }
        }
    }
}

/// A filter on the field at position `field` of the table's field list.
#[derive(Debug, Clone)]
pub struct FieldFilter {
    pub field: usize,
    pub expr: FilterExpr,
    /// The filter group it was set in (`FilterGroup(n)`); every group applies at once.
    pub group: i32,
}

/// The filter group `FilterGroup(-1)` writes to: its filters on different fields are alternatives
/// (a row passes when it matches any of them), while the filters of every other group all have to
/// match (corpus record/PWFilterGroupOr.al).
pub const OR_GROUP: i32 = -1;

/// Whether `row` passes `filters`: every filter outside the OR group matches, and, when the OR
/// group holds filters, at least one of them does.
pub fn row_passes(row: &[Value], filters: &[FieldFilter]) -> bool {
    let (mut or_seen, mut or_matched) = (false, false);
    for f in filters {
        let matched = f.expr.matches(&row[f.field]);
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
    let text = if german && !t.contains('/') && t.matches('.').count() == 2 {
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
    // (fieldref/lt-range-min-max-types/RangeMinMaxTypes.al:389-565, filters of a Date field; DateFilterTextForms.al:
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

#[cfg(test)]
mod tests {
    use super::*;

    /// record/lt-record-read/RcAFilters.al:SetFilter_Integer_Invalid and events/lt-platform-triggers/PtObserve.al:Bad_Int, Bad_Decimal,
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
        assert!(noon.matches(&Value::Time(12 * 3_600_000 + 1)));
    }

    /// Microsoft's Tests-SCM and Tests-ERM filter Date and Guid fields with text they built with `Format`
    /// (`SetFilter("Posting Date", '..' + Format(D))`, `SetFilter(SystemId, Format(G))`): the text reads as `Evaluate`
    /// reads it (pinned by types/lt-in-precedence-date-guid-filters/P143Date_EnPlain.al and the Guid tests); a
    /// formula or an empty text stays refused.
    #[test]
    fn a_date_or_guid_filter_reads_formatted_text_as_evaluate_does() {
        use crate::value::RtType as T;
        let date = parse_filter("01/17/27", &[], &T::Date).expect("a date in the region's short format");
        let d = Value::Date(crate::format::day_from_civil(2027, 1, 17), false);
        assert!(date.matches(&d));
        assert!(!date.matches(&Value::Date(0, false)));
        let range = parse_filter("..01/17/27", &[], &T::Date).expect("a range up to a date");
        assert!(range.matches(&d));
        let guid = parse_filter("{261C0957-2EC9-802F-F3FB-808A6781620E}", &[], &T::Guid).expect("a Guid with braces");
        let g = Value::Guid(crate::ops::parse_guid("261C0957-2EC9-802F-F3FB-808A6781620E").unwrap());
        assert!(guid.matches(&g));
        let closing = parse_filter("C01/17/27", &[], &T::Date).expect("a closing date as Format writes it");
        assert!(closing.matches(&Value::Date(crate::format::day_from_civil(2027, 1, 17), true)));
        assert!(!closing.matches(&d), "the closing date is not the date");
        assert!(parse_filter("1M", &[], &T::Date).is_err(), "a formula stays refused");
        // T and W need the session's dates (P143Date_EnFormulas.al).
        assert!(parse_filter("T", &[], &T::Date).is_err());
        let today = crate::format::day_from_civil(2026, 10, 15);
        let ctx = DateFilterContext {
            today,
            work_date: today + 1,
            language: 1033,
        };
        with_date_context(ctx, || {
            let t = parse_filter("..t", &[], &T::Date).expect("a range up to today");
            assert!(t.matches(&Value::Date(today, false)) && !t.matches(&Value::Date(today + 1, false)));
            let w = parse_filter("W", &[], &T::Date).expect("the work date");
            assert!(w.matches(&Value::Date(today + 1, false)));
            let cd = parse_filter("CD", &[], &T::Date).expect("the closing date of today");
            assert!(cd.matches(&Value::Date(today, true)) && !cd.matches(&Value::Date(today, false)));
        });
    }

    /// Microsoft's Library - ERM `GetDiscountPaymentTerm` filters a DateFormula field with `<>` and the empty text: there the
    /// empty text is the empty formula (assumption `filter-empty-text-dateformula`).
    #[test]
    fn a_date_formula_filter_reads_the_empty_text_as_the_empty_formula() {
        use crate::value::RtType as T;
        let not_blank = parse_filter("<>''", &[], &T::DateFormula).expect("<> the empty formula");
        assert!(!not_blank.matches(&Value::DateFormula(String::new())));
        assert!(not_blank.matches(&Value::DateFormula("<1M>".into())));
        assert!(
            parse_filter("1M", &[], &T::DateFormula).is_err(),
            "a formula stays refused"
        );
    }

    /// A quoted empty text in a Date filter is the blank date: `''` finds the blank rows, `<>''` the others
    /// (corpus record/lt-loadfields-pagesource-validate/P81DateFilters.al:193-194, z9 and z10), and `''..01/01/26` is the range
    /// from the blank date (same file, :294 and :308, where %1 holds 0D). Item's GetFilter("Date Filter") gives that text back
    /// to SetFilter in Transfer Line.FilterLinesWithItemToPlan.
    #[test]
    fn a_date_filter_reads_a_quoted_empty_text_as_the_blank_date() {
        use crate::value::RtType as T;
        let blank = Value::Date(0, false);
        let day = Value::Date(crate::format::day_from_civil(2026, 1, 1), false);
        let eq = parse_filter("''", &[], &T::Date).expect("''");
        assert!(eq.matches(&blank) && !eq.matches(&day));
        let ne = parse_filter("<>''", &[], &T::Date).expect("<>''");
        assert!(!ne.matches(&blank) && ne.matches(&day));
        let upto = parse_filter("''..01/01/26", &[], &T::Date).expect("''..01/01/26");
        assert!(upto.matches(&blank) && upto.matches(&day));
    }

    #[test]
    fn range_and_comparisons() {
        let r = FilterExpr::Range(Some(Value::Integer(2)), Some(Value::Integer(4)));
        assert!(!r.matches(&Value::Integer(1)));
        assert!(r.matches(&Value::Integer(2)));
        assert!(r.matches(&Value::Integer(4)));
        assert!(!r.matches(&Value::Integer(5)));
        assert!(FilterExpr::Range(None, Some(Value::Integer(3))).matches(&Value::Integer(-9)));
        assert!(
            FilterExpr::Or(vec![
                FilterExpr::Eq(Value::Integer(1)),
                FilterExpr::Gt(Value::Integer(8))
            ])
            .matches(&Value::Integer(9))
        );
        assert!(!FilterExpr::Ne(Value::Text("a".into())).matches(&Value::Text("a".into())));
    }

    #[test]
    fn the_or_group_is_alternatives_and_the_other_groups_all_apply() {
        let ff = |field, v: i32, group| FieldFilter {
            field,
            expr: FilterExpr::Eq(Value::Integer(v)),
            group,
        };
        let row = [Value::Integer(1), Value::Integer(2), Value::Integer(3)];
        // Either of two fields matches: the row passes.
        assert!(row_passes(&row, &[ff(0, 9, OR_GROUP), ff(1, 2, OR_GROUP)]));
        assert!(!row_passes(&row, &[ff(0, 9, OR_GROUP), ff(1, 9, OR_GROUP)]));
        // A filter of group 0 still has to match next to the OR group.
        assert!(!row_passes(&row, &[ff(2, 9, 0), ff(1, 2, OR_GROUP)]));
        assert!(row_passes(&row, &[ff(2, 3, 0), ff(1, 2, OR_GROUP)]));
        assert!(row_passes(&row, &[]));
    }

    use crate::value::RtType;

    fn pf(text: &str, args: &[Value], ty: RtType) -> FilterExpr {
        parse_filter(text, args, &ty).unwrap_or_else(|e| panic!("{text}: {e}"))
    }
    fn ints(f: &FilterExpr, vals: &[i32]) -> Vec<bool> {
        vals.iter().map(|v| f.matches(&Value::Integer(*v))).collect()
    }
    fn texts(f: &FilterExpr, vals: &[&str]) -> Vec<bool> {
        vals.iter()
            .map(|v| f.matches(&Value::Code(v.to_string().into())))
            .collect()
    }

    fn opt(text: &str, members: &[&str]) -> Result<FilterExpr, String> {
        let m: Vec<String> = members.iter().map(|x| x.to_string()).collect();
        parse_filter(text, &[], &RtType::Option(m))
    }

    // types/TestEnumBlankOptionFilterTests.Codeunit.al: option member names match with both
    // sides trimmed and case ignored, zero-length member names are skipped, and only then
    // is the text read as an ordinal.
    #[test]
    fn option_filters_match_member_names_trimmed_and_skip_zero_length_names() {
        let m = ["", "Draft", "Active"];
        let ord = |f: &FilterExpr, o: i32| f.matches(&Value::Option(o, Vec::<String>::new().into()));
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
        assert!(f.matches(&Value::Date(15, false)) && !f.matches(&Value::Date(21, false)));
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
        assert!(f.matches(&Value::Text(String::new().into())) && !f.matches(&Value::Text("x".into())));
        let f = pf("<>''", &[], RtType::Text(None));
        assert!(!f.matches(&Value::Text(String::new().into())) && f.matches(&Value::Text("x".into())));
        let f = pf("'Smith & Sons'", &[], RtType::Text(None));
        assert!(f.matches(&Value::Text("Smith & Sons".into())));
        let f = pf("'(Travel|Meals)'", &[], RtType::Text(None));
        assert!(f.matches(&Value::Text("(Travel|Meals)".into())));
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
        let f = parse_filter_with(text, &[], &RtType::Enum(7), &enum_members).unwrap_or_else(|e| panic!("{text}: {e}"));
        (0..4).filter(|v| f.matches(&Value::Enum(7, *v))).collect()
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
        assert!(parse_filter_with("Nope", &[], &RtType::Enum(7), &enum_members).is_err());
    }

    // corpus types/TestEnumBlankOptionFilterTests.Codeunit.al:122,160,188: the option matcher
    // trims both sides, skips zero-length member names, and falls back to an ordinal.
    #[test]
    fn option_filters_trim_both_sides_and_skip_zero_length_member_names() {
        let opt = |members: &[&str]| RtType::Option(members.iter().map(|m| m.to_string()).collect());
        let hits = |ty: &RtType, text: &str| -> Result<Vec<i32>, String> {
            let f = parse_filter(text, &[], ty)?;
            let names: Vec<String> = match ty {
                RtType::Option(m) => m.clone(),
                _ => unreachable!(),
            };
            Ok((0..names.len() as i32)
                .filter(|i| f.matches(&Value::Option(*i, names.clone().into())))
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
        let ty = RtType::Option(members);
        let ord = |f: &FilterExpr| {
            (0..3)
                .map(|i| f.matches(&Value::Option(i, Default::default())))
                .collect::<Vec<_>>()
        };
        assert_eq!(ord(&pf(" draft ", &[], ty.clone())), [false, true, false]);
        assert_eq!(ord(&pf("'DRAFT'", &[], ty.clone())), [false, true, false]);
        // The empty string names the blank member " " (trimmed), never the zero-length one.
        assert_eq!(ord(&pf("''", &[], ty.clone())), [false, false, true]);
        let zero_length_only = RtType::Option(vec!["".into(), "Draft".into()]);
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
        assert!(f.matches(&Value::Text("Mr smith & sons LLC".into())));
        assert!(!f.matches(&Value::Text("Acme Co".into())));
        let f = pf("'@*(2025)*'", &[], RtType::Text(None));
        assert!(f.matches(&Value::Text("Expenses (2025) - Diana".into())));
        assert!(!f.matches(&Value::Text("Expenses 2025".into())));
        // Without @ the quoted wildcard still matches case-sensitively.
        let f = pf("'*A|B*'", &[], RtType::Text(None));
        assert!(f.matches(&Value::Text("xA|By".into())) && !f.matches(&Value::Text("xa|by".into())));
        // A quoted expression without wildcards is a plain literal.
        let f = pf("'Smith & Sons'", &[], RtType::Text(None));
        assert!(f.matches(&Value::Text("Smith & Sons".into())) && !f.matches(&Value::Text("smith & sons".into())));
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
