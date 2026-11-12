//! Rendered content in a Text value (ADR-040 amendment 2026-11-12, Part A "the opaque value").
//!
//! A Text can hold the Html a Word layout report renders: text BC produced whose characters this runtime does not know. It is
//! known to be not empty (reports/layouts-saveas/AMReportOutputTests.al:205-217, `has_output` Yes, `kind` markup) and nothing
//! else. It stands in the text as [`RENDERED_TEXT_MARK`] followed by one character that numbers the render ([`Form`]): the
//! fourth mark of `native::TEXT_MARKS`, so every text function, filter and key that refuses a marked text refuses it too.
//!
//! This module owns the mark, the process-wide list of renders, the refusal text, the one place a marked text starts
//! ([`read_whole`]), the allow-list of builtin uses ([`guard`], table [`ALLOWED`]) and of .NET calls ([`DOTNET_ALLOWED`]). A use
//! that is not a row refuses. The blob side (a buffer that holds rendered content) is `placeholder_output`'s.
//!
//! The paths it lets through run under assumptions (CLAUDE.md rule 3, second exception: al-runner issue 6924, the reporter's
//! tests pass on BC and need the body to move): `report-html-rendered-text` when a read gives a marked text, and
//! `email-body-rgba-replace-keeps-content` for the one `Regex.Replace` of [`DOTNET_ALLOWED`].

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::library_kind::LibraryKind;
use crate::native::Signal;
use crate::value::{LibraryValue, Value};

/// The mark of rendered content in a text: a noncharacter, as the other marks (`native::CALL_STACK_MARK`).
pub const RENDERED_TEXT_MARK: char = '\u{FDD4}';

/// The number character after the mark: `WHOLE + n` for the unchanged content of render `n`, `CHANGED + n` for content an
/// allowed step derived from it (ADR-040 amendment 2026-11-12, A1). Read only directly after the mark.
const WHOLE: u32 = 0xF0000;
const CHANGED: u32 = 0x10_0000;
/// Renders a number character can name (plane 15 and 16 end at U+xFFFD).
const MAX_RENDERS: u32 = 0xFFFE;

/// The two forms of a mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// The complete rendered content stands here, unchanged.
    Whole,
    /// Content derived from it by an allowed step that can change characters.
    Changed,
}

/// One render: what a refusal names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Render {
    pub report: String,
    pub format: String,
    pub layout: String,
}

/// The renders of the process. A marked text can outlive the session that read it (a SingleInstance global, a stored row, the
/// strict order mode), so the numbers are never per session and an entry is never reused (loss mode L15).
static RENDERS: Mutex<Vec<Render>> = Mutex::new(Vec::new());

/// Set by the first [`read_whole`] that gives a marked text and never cleared, for the same reason: from then on the VM runs
/// [`guard`] on every builtin call.
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Whether a marked text may exist in this process (see [`ACTIVE`]).
pub fn active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Registers a render and gives its number, `None` once the numbers are used up (the read then refuses).
pub fn register(report: &str, format: &str, layout: &str) -> Option<u32> {
    let mut list = RENDERS.lock().unwrap_or_else(|e| e.into_inner());
    let n = u32::try_from(list.len()).ok().filter(|n| *n < MAX_RENDERS)?;
    list.push(Render {
        report: report.to_string(),
        format: format.to_string(),
        layout: layout.to_string(),
    });
    Some(n)
}

/// The render with number `n`.
pub fn render(n: u32) -> Option<Render> {
    RENDERS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(n as usize)
        .cloned()
}

/// The text of the mark of render `n` in `form`.
pub fn mark_text(n: u32, form: Form) -> String {
    let base = match form {
        Form::Whole => WHOLE,
        Form::Changed => CHANGED,
    };
    let number = char::from_u32(base + n).expect("a render number names a character");
    format!("{RENDERED_TEXT_MARK}{number}")
}

/// The marks in `s`, in order: the render number and the form. `None` when a mark is not followed by a number character.
pub fn marks(s: &str) -> Option<Vec<(u32, Form)>> {
    let mut out = Vec::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != RENDERED_TEXT_MARK {
            continue;
        }
        let n = it.next()? as u32;
        out.push(match n {
            n if (WHOLE..WHOLE + MAX_RENDERS).contains(&n) => (n - WHOLE, Form::Whole),
            n if (CHANGED..CHANGED + MAX_RENDERS).contains(&n) => (n - CHANGED, Form::Changed),
            _ => return None,
        });
    }
    Some(out)
}

/// Whether `s` holds rendered content.
pub fn holds(s: &str) -> bool {
    s.contains(RENDERED_TEXT_MARK)
}

/// `s` with every mark in the changed form (`Format(BigText)`, P12).
pub fn to_changed(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        out.push(c);
        if c == RENDERED_TEXT_MARK
            && let Some(n) = it.next()
        {
            let n = n as u32;
            let changed = if (WHOLE..WHOLE + MAX_RENDERS).contains(&n) {
                n - WHOLE + CHANGED
            } else {
                n
            };
            out.push(char::from_u32(changed).unwrap_or(char::REPLACEMENT_CHARACTER));
        }
    }
    out
}

/// `s` as the runner prints it: each mark and its number as `<rendered content of report X>` (A6, loss mode L11).
pub fn describe(s: &str) -> String {
    if !holds(s) {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != RENDERED_TEXT_MARK {
            out.push(c);
            continue;
        }
        let name = it
            .next()
            .and_then(|n| marks(&format!("{RENDERED_TEXT_MARK}{n}")))
            .and_then(|m| m.first().and_then(|(n, _)| render(*n)))
            .map_or_else(|| "a report".to_string(), |r| format!("report {}", r.report));
        out.push_str(&format!("<rendered content of {name}>"));
    }
    out
}

/// The refusal of `what` on a text that holds rendered content (A3b). Names the first render of `s`.
pub fn refusal(s: &str, what: &str) -> Signal {
    let named = marks(s)
        .and_then(|m| m.first().and_then(|(n, _)| render(*n)))
        .map_or_else(
            || "a report".to_string(),
            |r| format!("report {} ({}, layout {})", r.report, r.format, r.layout),
        );
    Signal::unsupported(format!(
        "a use of the rendered content of {named}: {what}; the runtime has no layout renderer, so the text is not known"
    ))
}

/// The text of the first value of `vals` that holds rendered content, looking into a TextBuilder and a BigText (A5).
fn held<'a>(vals: impl IntoIterator<Item = &'a Value>) -> Option<String> {
    vals.into_iter().find_map(value_text)
}

/// The rendered text a value holds: a Text or Code (also inside a Variant, which holds the value itself), a TextBuilder or a
/// BigText, a List or Dictionary that holds one as a value.
fn value_text(v: &Value) -> Option<String> {
    match v {
        // `audited_str` only looks for the mark here; a text with half of a surrogate pair goes on to its own reader.
        Value::Text(t) | Value::Code(t) | Value::TextConst(t) if t.has_marker() && holds(t.audited_str()) => {
            Some(t.audited_str().to_string())
        }
        Value::Library(_, h) => match &*h.borrow() {
            LibraryValue::Text(s) | LibraryValue::TextCap(s, _) if holds(s) => Some(s.clone()),
            _ => None,
        },
        Value::List(l) => l.borrow().iter().find_map(value_text),
        Value::Dictionary(d) => d.borrow().iter().find_map(|(_, v)| value_text(v)),
        _ => None,
    }
}

/// Where a row of [`ALLOWED`] lets a marked value stand.
#[derive(Debug, Clone, Copy)]
enum At {
    /// The receiver (a TextBuilder, BigText, List or Dictionary holding it, or a Variant).
    Recv,
    /// Argument `i`.
    Arg(usize),
    /// Every argument from `i` on (the values of `StrSubstNo`, `Session.LogMessage`).
    From(usize),
}

/// A further condition of a row.
#[derive(Debug, Clone, Copy, PartialEq)]
enum When {
    Always,
    /// Exactly this many arguments (no length, no format number).
    Args(usize),
    /// `StrSubstNo`: the template is known and uses the marked value only as `%n`.
    PercentOnly,
    /// A stream write: one argument, the return value unused, a UTF-8 stream.
    Utf8Write,
    /// `BigText.Write(OutStream)`: the return value unused, the stream argument UTF-8.
    StreamArgUtf8,
    /// `Format(Value)`: one argument, a Text (a Variant that holds one arrives as the Text) or a TextBuilder.
    FormatOfText,
    /// `Format(BigText)`: one argument, a BigText.
    FormatOfBigText,
    /// `BigText.GetSubText(Text, 1)`: a Text target, the Integer position 1, no length, the return value unused (it is a
    /// length).
    SubTextFromOne,
}

/// One allowed use: the builtin id name, where the marked value may stand, the condition, the form of the result and why BC's
/// answer does not depend on the characters.
struct Row {
    name: &'static str,
    at: At,
    when: When,
    /// The form of the mark in the result, `None` when the use moves the value unchanged.
    form: Option<Form>,
    cite: &'static str,
}

/// Table A3 of the amendment. A use that is not a row refuses. A new row is an exact match of a pinned or reported shape.
static ALLOWED: &[Row] = &[
    // P2: joining does not look at the characters.
    Row {
        name: "TextBuilder.Append#1",
        at: At::Arg(0),
        when: When::Args(1),
        form: None,
        cite: "P2",
    },
    Row {
        name: "TextBuilder.Append#1",
        at: At::Recv,
        when: When::Args(1),
        form: None,
        cite: "P2",
    },
    Row {
        name: "TextBuilder.AppendLine#1",
        at: At::Arg(0),
        when: When::Args(1),
        form: None,
        cite: "P2",
    },
    Row {
        name: "TextBuilder.AppendLine#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P2",
    },
    Row {
        name: "BigText.AddText",
        at: At::Arg(0),
        when: When::Args(1),
        form: None,
        cite: "P2",
    },
    Row {
        name: "BigText.AddText",
        at: At::Recv,
        when: When::Args(1),
        form: None,
        cite: "P2",
    },
    // P1: a value out of a Variant goes to the conversion after it unread (`ops::convert` takes only an unbounded Text).
    Row {
        name: "Variant.Into#1",
        at: At::From(0),
        when: When::Always,
        form: None,
        cite: "P1",
    },
    Row {
        name: "Variant.Into#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P1",
    },
    // P3: the identity on a Text.
    Row {
        name: "TextBuilder.ToText#1",
        at: At::Recv,
        when: When::Args(0),
        form: None,
        cite: "P3",
    },
    Row {
        name: "System.Format#1",
        at: At::Arg(0),
        when: When::FormatOfText,
        form: None,
        cite: "P3",
    },
    // `Format(BigText)` gives the changed form (owner decision 2026-11-12; `format::bigtext_text`).
    Row {
        name: "System.Format#1",
        at: At::Arg(0),
        when: When::FormatOfBigText,
        form: Some(Form::Changed),
        cite: "P3",
    },
    // The Text target overload only (`#1` is the BigText target, which the amendment does not list).
    Row {
        name: "BigText.GetSubText#2",
        at: At::Recv,
        when: When::SubTextFromOne,
        form: None,
        cite: "P3",
    },
    // P4: a value of StrSubstNo goes in through Format (format-evaluate/P86StrSubstNo.al Sub_Percent).
    Row {
        name: "Text.StrSubstNo#1",
        at: At::From(1),
        when: When::PercentOnly,
        form: None,
        cite: "P4",
    },
    // P7: writing does not look at the characters.
    Row {
        name: "OutStream.WriteText#1",
        at: At::Arg(0),
        when: When::Utf8Write,
        form: None,
        cite: "P7",
    },
    Row {
        name: "OutStream.Write",
        at: At::Arg(0),
        when: When::Utf8Write,
        form: None,
        cite: "P7",
    },
    Row {
        name: "BigText.Write#1",
        at: At::Recv,
        when: When::StreamArgUtf8,
        form: None,
        cite: "P7",
    },
    // P8: a collection stores a value unread (as for a SQL counter value, `sqlcount::guard`).
    Row {
        name: "List.Add#1",
        at: At::Arg(0),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Set#1",
        at: At::Arg(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Set#2",
        at: At::From(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Insert#1",
        at: At::Arg(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Get#2",
        at: At::Arg(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Add#1",
        at: At::Arg(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Set#1",
        at: At::Arg(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Set#2",
        at: At::From(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Get#2",
        at: At::Arg(1),
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Add#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Set#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Set#2",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Insert#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Get#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "List.Get#2",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Add#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Set#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Set#2",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Get#1",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    Row {
        name: "Dictionary.Get#2",
        at: At::Recv,
        when: When::Always,
        form: None,
        cite: "P8",
    },
    // A2: a whole read replaces the value of its variable without reading it (the second read itself refuses: the stream is at
    // the end of the placeholder).
    Row {
        name: "InStream.Read",
        at: At::Arg(0),
        when: When::Args(1),
        form: None,
        cite: "A2",
    },
    Row {
        name: "BigText.Read#1",
        at: At::Recv,
        when: When::Args(1),
        form: None,
        cite: "A2",
    },
    // P10: telemetry is not readable by the test (session-system/session/SqlCounters.al:516).
    Row {
        name: "Session.LogMessage#1",
        at: At::From(0),
        when: When::Always,
        form: None,
        cite: "P10",
    },
    Row {
        name: "Session.LogMessage#2",
        at: At::From(0),
        when: When::Always,
        form: None,
        cite: "P10",
    },
];

/// The rows of [`ALLOWED`] as (builtin, the amendment's row, the form of the result: `None` moves the value unchanged), for
/// the table test (loss modes L12, L16).
pub fn allowed_rows() -> impl Iterator<Item = (&'static str, &'static str, Option<Form>)> {
    ALLOWED.iter().map(|r| (r.name, r.cite, r.form))
}

/// `Variant.Is*` asks only the type (P9).
fn asks_type(name: &str) -> bool {
    name.starts_with("Variant.Is")
}

/// The base name of a builtin id name: `OutStream.Write#7` is `OutStream.Write` (one row covers the overloads of a write; the
/// row's condition requires a Text argument).
fn matches_name(row: &str, name: &str) -> bool {
    row == name || (!row.contains('#') && name.split('#').next() == Some(row))
}

/// The builtin call guard (A5): refuses builtin `name` given a value that holds rendered content at a position no row of
/// [`ALLOWED`] lists. Runs while [`active`]; `ret_used` is whether the call's return value is used.
pub fn guard(name: &str, recv: Option<&Value>, args: &[Value], ret_used: bool) -> Result<(), Signal> {
    let allowed = |at_recv: bool, i: usize| {
        ALLOWED.iter().any(|row| {
            matches_name(row.name, name)
                && match row.at {
                    At::Recv => at_recv,
                    At::Arg(j) => !at_recv && i == j,
                    At::From(j) => !at_recv && i >= j,
                }
                && condition(row.when, recv, args, ret_used, i)
        })
    };
    // The row is looked up first: a List or Dictionary a row allows is not walked on every call.
    if let Some(r) = recv
        && !asks_type(name)
        && !allowed(true, 0)
        && let Some(s) = value_text(r)
    {
        return Err(refusal(&s, &format!("{name} on the value")));
    }
    for (i, a) in args.iter().enumerate() {
        if !allowed(false, i)
            && let Some(s) = value_text(a)
        {
            return Err(refusal(&s, &format!("{name} given the value")));
        }
    }
    Ok(())
}

fn condition(when: When, recv: Option<&Value>, args: &[Value], ret_used: bool, i: usize) -> bool {
    match when {
        When::Always => true,
        When::Args(n) => args.len() == n,
        When::PercentOnly => percent_only(args, i),
        When::Utf8Write => {
            let text_arg = args
                .first()
                .is_none_or(|a| matches!(a, Value::Text(_) | Value::Code(_)));
            args.len() <= 1 && text_arg && !ret_used && recv.is_some_and(utf8_out)
        }
        When::StreamArgUtf8 => args.len() == 1 && !ret_used && utf8_out(&args[0]),
        When::FormatOfText => {
            matches!(args, [Value::Text(_)]) || matches!(args, [v] if is_library(v, LibraryKind::TextBuilder))
        }
        When::FormatOfBigText => matches!(args, [v] if is_library(v, LibraryKind::BigText)),
        When::SubTextFromOne => matches!(args, [Value::Text(_), Value::Integer(1)]) && !ret_used,
    }
}

/// Whether `v` is a library value of `kind`.
fn is_library(v: &Value, kind: LibraryKind) -> bool {
    matches!(v, Value::Library(k, _) if *k == kind)
}

/// Whether `v` is an OutStream on a UTF-8 stream.
fn utf8_out(v: &Value) -> bool {
    let Value::Library(_, h) = v else {
        return false;
    };
    matches!(&*h.borrow(), LibraryValue::OutStream(_, enc) if *enc == crate::stream::Encoding::Utf8)
}

/// `StrSubstNo(Template, ..)`: the template is known (it holds no mark) and uses value `i` (argument `i`, `%i`) only as `%i`,
/// never as `#i` (a `#n` field pads or cuts to a width and shows `*` on overflow, which depends on the length).
fn percent_only(args: &[Value], i: usize) -> bool {
    let Some(Value::Text(t) | Value::Code(t)) = args.first() else {
        return false;
    };
    if t.has_marker() {
        return false;
    }
    let (mut percent, mut field) = (false, false);
    let s = t.as_str().as_bytes();
    let mut k = 0;
    while k < s.len() {
        if s[k] == b'%' || s[k] == b'#' {
            let start = k + 1;
            let mut end = start;
            while end < s.len() && s[end].is_ascii_digit() {
                end += 1;
            }
            if end > start
                && let Ok(n) = std::str::from_utf8(&s[start..end]).unwrap_or("").parse::<usize>()
                && n == i
            {
                if s[k] == b'%' {
                    percent = true;
                } else {
                    field = true;
                }
            }
            k = end.max(k + 1);
        } else {
            k += 1;
        }
    }
    percent && !field
}

/// The .NET calls a marked argument may reach (A5): (type, member). One entry, P12: `Email Message Impl.`
/// `ReplaceRgbaColorsWithRgb` (System Application) calls `Regex.Replace(Body, Pattern, Replacement)` on every body
/// `Email Message.Create` is given, with exactly this pattern and replacement.
pub static DOTNET_ALLOWED: &[DotNetEntry] = &[DotNetEntry {
    ty: "System.Text.RegularExpressions.Regex",
    member: "Replace",
    pattern: RGBA_PATTERN,
    replacement: RGBA_REPLACEMENT,
}];

/// One entry of [`DOTNET_ALLOWED`]: a static `Replace(Input, Pattern, Replacement)` with these known arguments.
pub struct DotNetEntry {
    pub ty: &'static str,
    pub member: &'static str,
    pub pattern: &'static str,
    pub replacement: &'static str,
}

/// The pattern and replacement of `Email Message Impl.`.`ReplaceRgbaColorsWithRgb`. The replacement is never empty, so a text
/// that is not empty stays not empty.
const RGBA_PATTERN: &str = r"rgba\((\d+), ?(\d+), ?(\d+), ?\d+\)";
const RGBA_REPLACEMENT: &str = "rgb($1, $2, $3)";

/// Refuses a .NET call given a marked argument unless it is an entry of [`DOTNET_ALLOWED`]; `Ok(true)` when it is one (the
/// caller then runs it with [`dotnet_result`] on the answer).
pub fn dotnet_guard(ty: &str, member: &str, args: &[Value]) -> Result<bool, Signal> {
    let Some(s) = held(args) else {
        return Ok(false);
    };
    let known = |i: usize| match args.get(i) {
        Some(Value::Text(t) | Value::Code(t)) if !t.has_marker() => Some(t.as_str().to_string()),
        _ => None,
    };
    let entry = DOTNET_ALLOWED.iter().any(|e| {
        e.ty.eq_ignore_ascii_case(ty)
            && e.member.eq_ignore_ascii_case(member)
            && args.len() == 3
            && value_text(&args[0]).is_some()
            && known(1).as_deref() == Some(e.pattern)
            && known(2).as_deref() == Some(e.replacement)
    });
    if !entry {
        return Err(refusal(&s, &format!("the DotNet member {ty}.{member} given the value")));
    }
    crate::assumptions::assume(crate::assumptions::Assumption::EmailBodyRgbaReplaceKeepsContent);
    Ok(true)
}

/// The text a [`DOTNET_ALLOWED`] call answered for `input`: every mark in the changed form; refused when the answer lost or
/// gained a mark (loss mode L6).
pub fn dotnet_result(input: &str, answer: &str) -> Result<String, Signal> {
    let count = |s: &str| marks(s).map(|m| m.len());
    match (count(input), count(answer)) {
        (Some(a), Some(b)) if a == b && a > 0 => Ok(to_changed(answer)),
        _ => Err(refusal(input, "a .NET answer that does not keep every mark")),
    }
}

/// What [`read_whole`] reads from.
pub enum Source<'a> {
    /// The Html placeholder of render `render` (`None`: the run that wrote it gives no render, see [`html_mark`]).
    Html { render: Option<u32>, bytes: &'a [u8] },
    /// A buffer a marked text was written to: its bytes.
    Written { bytes: &'a [u8] },
}

/// A2: the one place a marked text starts. `InStream.Read(Text)` without a length with its return value unused, or
/// `BigText.Read(InStream)`, at position 0 of a buffer that holds rendered content. The Html placeholder gives the mark alone
/// (its own markup never reaches AL, loss mode L8), and only while the buffer holds exactly the placeholder's bytes; a buffer a
/// marked text was written to gives that text back as the pinned read of known text does: up to its first zero byte or the
/// end (streams/ProbeJsonAndInStreamBytes.al InStreamRead_Text_OnWriteTextContent_NoTerminator), on a UTF-8 stream, the bytes
/// UTF-8 and every mark whole. Such a buffer only ever took appends (`mark_written`, `stream::write_bytes`), so the bytes
/// before a mark, and a text that ends at a zero byte before the first mark, are the known text AL wrote. The answer is the
/// text and the bytes the read takes.
pub fn read_whole(
    src: Source,
    at_start: bool,
    ret_used: bool,
    utf8: bool,
    what: &str,
) -> Result<(String, usize), Signal> {
    let refuse = |why: &str| {
        Err(Signal::unsupported(format!(
            "{what} of a buffer that holds a rendered report output {why} (the runtime has no layout renderer)"
        )))
    };
    if !at_start {
        return refuse("from a position after its start");
    }
    if ret_used {
        return refuse("with its return value used (the number of characters is not known)");
    }
    let text = match src {
        Source::Html { render: None, .. } => {
            return refuse(
                "from a report run that has no render number: it produced no dataset row (what BC renders then is not \
                 pinned, ADR-040 amendment 2026-11-12, A2), or the process used up its render numbers",
            );
        }
        Source::Html { render: Some(n), bytes } => {
            if bytes != crate::placeholder_output::HTML {
                return refuse("after AL wrote over it");
            }
            (mark_text(n, Form::Whole), bytes.len())
        }
        Source::Written { bytes } => {
            if !utf8 {
                return refuse("on a stream that is not UTF-8");
            }
            if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                return refuse("that starts with a byte order mark");
            }
            let body = bytes.split(|b| *b == 0).next().unwrap_or_default();
            let taken = (body.len() + 1).min(bytes.len());
            let Ok(s) = std::str::from_utf8(body) else {
                return refuse("whose bytes are not UTF-8");
            };
            if !marks(s).is_some_and(|m| m.iter().all(|(n, _)| render(*n).is_some())) {
                return refuse("whose bytes hold a mark that is not whole");
            }
            (s.to_string(), taken)
        }
    };
    ACTIVE.store(true, Ordering::Relaxed);
    crate::assumptions::assume(crate::assumptions::Assumption::ReportHtmlRenderedText);
    Ok(text)
}

/// The placeholder mark of the Html a save of `report` wrote, with its render number (`None`: the run produced no dataset row).
pub fn html_mark(report: &str, render: Option<u32>) -> std::sync::Arc<str> {
    let base = crate::placeholder_output::mark("Html", report);
    match render {
        Some(n) => format!("{base}, render {n}").into(),
        None => base,
    }
}

/// The render number of an Html placeholder mark, `None` for any other mark or one without a number.
pub fn html_render(mark: &str) -> Option<Option<u32>> {
    let rest = mark.strip_prefix("a placeholder Html of report ")?;
    Some(rest.rsplit_once(", render ").and_then(|(_, n)| n.parse().ok()))
}

/// The placeholder kind of a buffer a marked text was written to (A4): the placeholder rules hold for it (presence and a size
/// known only to be greater than 0, byte copies keep it, every other read refuses).
pub const WRITTEN_KIND: &str = "text with rendered content";

/// The placeholder kind of such a buffer once its bytes no longer stand for what BC holds: a write that was not an append
/// at its end touched it, or rendered content was written over bytes that were there. BC's rendering is longer than its
/// mark, so which bytes BC keeps after such a write is not known. No read answers for this kind.
pub const WRITTEN_OVER_KIND: &str = "text with rendered content that was written over";

/// The [`WRITTEN_KIND`] mark for the marked text `s` (P7).
pub fn written_mark(s: &str) -> std::sync::Arc<str> {
    let report = marks(s)
        .and_then(|m| m.first().and_then(|(n, _)| render(*n)))
        .map_or_else(|| "a report".to_string(), |r| r.report);
    crate::placeholder_output::mark(WRITTEN_KIND, &report)
}

/// The [`WRITTEN_OVER_KIND`] mark of the same report as the [`WRITTEN_KIND`] mark `written`.
pub fn written_over(written: &str) -> std::sync::Arc<str> {
    let report = written.strip_prefix(&written_prefix()).unwrap_or("a report");
    crate::placeholder_output::mark(WRITTEN_OVER_KIND, report)
}

/// Marks `buf` after rendered content was written to it: a marked text (P7) or a byte copy of a [`WRITTEN_KIND`] buffer,
/// `written` being that content's mark. `appended`: the write started at the end of what the buffer held. A buffer without a
/// mark gets `written` after an append and [`WRITTEN_OVER_KIND`] otherwise. A buffer with a mark keeps it: this kind's is
/// already changed by `stream::write_bytes` when the write was no append, and any other mark refuses more reads than this one.
pub fn mark_written(buf: &mut crate::stream::BlobBuf, written: std::sync::Arc<str>, appended: bool) {
    if buf.render_pending.is_none() {
        buf.render_pending = Some(if appended { written } else { written_over(&written) });
    }
}

fn written_prefix() -> String {
    format!("a placeholder {WRITTEN_KIND} of report ")
}

/// Whether a placeholder mark is [`WRITTEN_KIND`]'s.
pub fn is_written(mark: &str) -> bool {
    mark.starts_with(&written_prefix())
}

/// A2 on an InStream value: `Some` when the stream reads a buffer that holds rendered content (the Html placeholder or a
/// buffer a marked text was written to), with the text [`read_whole`] gives or its refusal; the stream is then at its end.
/// `None` for any other stream: the caller reads as before.
pub fn read_in(stream: Option<&Value>, ret_used: bool, what: &str) -> Option<Result<String, Signal>> {
    let Some(Value::Library(_, h)) = stream else {
        return None;
    };
    let mut g = h.borrow_mut();
    let LibraryValue::InStream(s) = &mut *g else {
        return None;
    };
    let blob = s.live_blob()?.clone();
    let buf = blob.borrow();
    let mark = crate::placeholder_output::of(&buf)?.clone();
    let src = match html_render(&mark) {
        Some(render) => Source::Html {
            render,
            bytes: &buf.bytes,
        },
        None if is_written(&mark) => Source::Written { bytes: &buf.bytes },
        None => return None,
    };
    let utf8 = s.enc == crate::stream::Encoding::Utf8;
    let read = read_whole(src, s.pos == 0, ret_used, utf8, what);
    Some(read.map(|(text, taken)| {
        s.pos = taken;
        text
    }))
}

/// Whether a native that only stores or writes a text argument carries `s` on (`native::text_of_carrying_call_stack`, the
/// TextBuilder's argument): it holds rendered content and no other mark but a call stack's. The guard decided before the native
/// ran that the use is a row of [`ALLOWED`].
pub fn carried(s: &str) -> bool {
    holds(s) && !s.contains([crate::native::CARRIED_TEXT_MARK, crate::native::SQL_COUNT_TEXT_MARK])
}
