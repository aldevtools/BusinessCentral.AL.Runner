//! Reading refusals: the ranking of not-run reasons, with names and numbers folded so that one
//! cause groups together (`--top-refusals` of both command lines).

use crate::CaseId;
use std::collections::BTreeMap;

/// One refusal reason with the cases that share it.
#[derive(Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The reason with names and numbers folded (`normalise_reason`).
    pub key: String,
    pub count: usize,
    /// The first case (by codeunit, then name) with this reason, and its reason as raised.
    pub example: CaseId,
    pub example_reason: String,
}

/// Folds a refusal reason so that reasons that differ only in a name or a number group together:
/// the routine the reason was found in (`Object.Routine: `, `handler X: `, `OnRun: `) goes, quoted
/// names become `<name>` and numbers become `N`.
pub fn normalise_reason(raw: &str) -> String {
    fold_numbers(&fold_quotes(strip_site(raw)))
}

/// The reason without the routine it was found in (`Object.Routine: `, `handler X: `, `OnRun: `) and without the routine it
/// stopped in (`explain::site_suffix`).
fn strip_site(raw: &str) -> &str {
    let mut rest = crate::explain::without_site_suffix(raw.trim()).trim_end();
    for _ in 0..4 {
        let Some((head, tail)) = rest.split_once(": ") else {
            break;
        };
        if tail.trim().is_empty() || !is_site_prefix(head) {
            break;
        }
        rest = tail.trim_start();
    }
    rest
}

/// Replaces the text inside every outermost pair of call parentheses (directly after a name) with `..`, so that calls that
/// differ only in their arguments group together.
fn fold_parens(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    let mut prev = ' ';
    for c in s.chars() {
        let call = prev.is_alphanumeric() || prev == '_' || prev == '"';
        prev = c;
        match c {
            '(' if depth == 0 && !call => out.push(c),
            '(' => {
                if depth == 0 {
                    out.push_str("(..");
                }
                depth += 1;
            }
            ')' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    out.push(')');
                }
            }
            _ if depth > 0 => {}
            _ => out.push(c),
        }
    }
    out
}

/// A finer view than `rank` for `--refusal-detail TEXT`: the not-run reasons that contain `needle`,
/// with the site and the call arguments folded but names and numbers kept, most frequent first.
pub fn detail(not_run: impl IntoIterator<Item = (CaseId, String)>, needle: &str) -> Vec<Refusal> {
    let mut groups: BTreeMap<String, Refusal> = BTreeMap::new();
    for (id, reason) in not_run {
        if !reason.contains(needle) {
            continue;
        }
        let key = fold_parens(strip_site(&reason));
        let g = groups.entry(key.clone()).or_insert_with(|| Refusal {
            key,
            count: 0,
            example: id.clone(),
            example_reason: reason.clone(),
        });
        g.count += 1;
        if id < g.example {
            g.example = id;
            g.example_reason = reason;
        }
    }
    let mut v: Vec<Refusal> = groups.into_values().collect();
    v.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    v
}

/// Whether `head` (the text before the first `: `) says where a reason was found, not what it is.
fn is_site_prefix(head: &str) -> bool {
    if head == "OnRun" {
        return true;
    }
    if head.starts_with("handler ") && !head.contains(['.', '`', '(']) {
        return true;
    }
    // A variable: `LayoutList`: platform table ...
    if head.len() > 2 && head.starts_with('`') && head.ends_with('`') && !head[1..head.len() - 1].contains('`') {
        return true;
    }
    // `Object Name.RoutineName`: an identifier after the last dot. DotNet and builtin texts that
    // name a member the same way are reasons, not sites.
    if head.starts_with("DotNet") || head.starts_with("builtin ") || head.contains(['`', '(', '\'']) {
        return false;
    }
    match head.rsplit_once('.') {
        Some((obj, routine)) => {
            !obj.is_empty()
                && !routine.is_empty()
                && routine.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !routine.starts_with(|c: char| c.is_ascii_digit())
        }
        None => false,
    }
}

/// `'x'`, `` `x` `` and `"x"` become `<name>`. An apostrophe inside a word is not a quote.
fn fold_quotes(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let opens = matches!(c, '\'' | '`' | '"') && (i == 0 || !chars[i - 1].is_alphanumeric());
        if opens {
            let close =
                (i + 1..chars.len()).find(|&j| chars[j] == c && chars.get(j + 1).is_none_or(|n| !n.is_alphanumeric()));
            if let Some(j) = close {
                out.push_str("<name>");
                i = j + 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// A run of digits that is not part of a word (`#4`, `(61)`, `format 2`) becomes `N`. A reference to a decision record keeps
/// its numbers (`ADR-013 decision 5`, `ADR-066 amendment 12, decision 3`): it names the rule, not a value.
fn fold_numbers(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let word = |c: char| c.is_alphabetic() || c == '_';
    let mut out = String::with_capacity(s.len());
    // Where the last number of an ADR reference ended in `out`.
    let mut adr_end: Option<usize> = None;
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len() && chars[j].is_ascii_digit() {
            j += 1;
        }
        let in_word = (i > 0 && word(chars[i - 1])) || (j < chars.len() && word(chars[j]));
        let adr = out.ends_with("ADR-")
            || adr_end.is_some_and(|e| matches!(out[e..].trim_start_matches(','), " decision " | " amendment "));
        if in_word || adr {
            out.extend(&chars[i..j]);
        } else {
            out.push('N');
        }
        adr_end = adr.then_some(out.len());
        i = j;
    }
    out
}

/// The not-run tests as `(test, reason as raised)`, grouped by normalised reason, most frequent
/// first (ties by reason).
pub fn rank(not_run: impl IntoIterator<Item = (CaseId, String)>) -> Vec<Refusal> {
    let mut groups: BTreeMap<String, Refusal> = BTreeMap::new();
    for (id, reason) in not_run {
        let key = normalise_reason(&reason);
        let g = groups.entry(key.clone()).or_insert_with(|| Refusal {
            key,
            count: 0,
            example: id.clone(),
            example_reason: reason.clone(),
        });
        g.count += 1;
        if id < g.example {
            g.example = id;
            g.example_reason = reason;
        }
    }
    let mut v: Vec<Refusal> = groups.into_values().collect();
    v.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.key.cmp(&b.key)));
    v
}

/// The ranking as text: `limit` entries, then what was left out.
pub fn render_top(title: &str, list: &[Refusal], limit: usize) -> String {
    render_ranked("Top refusals", title, "not run", "--top-refusals", list, limit)
}

/// [`render_top`] for failed tests (`--top-failures`): the same folding, another heading.
pub fn render_top_failures(title: &str, list: &[Refusal], limit: usize) -> String {
    render_ranked("Top failures", title, "failed", "--top-failures", list, limit)
}

fn render_ranked(heading: &str, title: &str, noun: &str, flag: &str, list: &[Refusal], limit: usize) -> String {
    let total: usize = list.iter().map(|r| r.count).sum();
    let mut s = format!("{heading}, {title}: {total} {noun}, {} distinct reasons\n", list.len());
    for r in list.iter().take(limit) {
        s.push_str(&format!("{:>6}  {}\n", r.count, r.key));
        let raw = if r.example_reason == r.key {
            String::new()
        } else {
            format!(": {}", clip(&r.example_reason, 200))
        };
        s.push_str(&format!(
            "        e.g. {} {}{raw}\n",
            r.example.codeunit, r.example.name
        ));
    }
    if list.len() > limit {
        let rest: usize = list[limit..].iter().map(|r| r.count).sum();
        s.push_str(&format!(
            "        ... {} more reasons, {rest} cases ({flag} {})\n",
            list.len() - limit,
            list.len()
        ));
    }
    s
}

pub fn clip(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let head: String = s.chars().take(max).collect();
        format!("{head}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reasons_that_differ_in_a_name_or_a_number_fold_together() {
        let n = normalise_reason;
        assert_eq!(
            n("Test Query Object.Query_SaveAsCsv: builtin Query.SaveAsCsv#4 is not implemented"),
            "builtin Query.SaveAsCsv#N is not implemented"
        );
        assert_eq!(
            n("builtin Query.SaveAsCsv#2 is not implemented"),
            "builtin Query.SaveAsCsv#N is not implemented"
        );
        assert_eq!(
            n("`LayoutList`: platform table 'Report Layout List' (2000000234) has no provider"),
            "platform table <name> (N) has no provider"
        );
        assert_eq!(
            n(
                "handler OkGetDemandToReserve: Request Page Parameters Helper.GetFiltersForTable: foreach over a DotNet value is refused"
            ),
            "foreach over a DotNet value is refused"
        );
        assert_eq!(
            n("OnRun: Some Codeunit.OnRun: division by zero (not pinned)"),
            "division by zero (not pinned)"
        );
        assert_eq!(
            n("Format of a Duration with standard format 2 (not pinned)"),
            "Format of a Duration with standard format N (not pinned)"
        );
    }

    #[test]
    fn detail_groups_matching_reasons_by_text_with_the_site_and_the_call_arguments_folded() {
        let id = |n: &str| CaseId {
            codeunit: 1,
            name: n.into(),
        };
        let rows = vec![
            (
                id("a"),
                "Lib.Gen: call `NoSeries.ReadIsolation(IsolationLevel::UpdLock)` (None) is not lowered yet".to_string(),
            ),
            (
                id("b"),
                "Other.Proc: call `NoSeries.ReadIsolation(IsolationLevel::ReadCommitted)` (None) is not lowered yet"
                    .to_string(),
            ),
            (
                id("c"),
                "Lib.Gen: call `X.Y(1, (2))` (None) is not lowered yet".to_string(),
            ),
            (id("d"), "builtin Z is not implemented".to_string()),
        ];
        let d = detail(rows, "is not lowered");
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].key, "call `NoSeries.ReadIsolation(..)` (None) is not lowered yet");
        assert_eq!(d[0].count, 2);
        assert_eq!(d[1].key, "call `X.Y(..)` (None) is not lowered yet");
    }

    #[test]
    fn a_dotnet_member_is_a_reason_not_a_site_and_a_name_with_digits_in_a_word_stays() {
        assert_eq!(
            normalise_reason("DotNet X.Y.IsSandbox: refused by policy: server class 'X.Y' is neither registered"),
            "DotNet X.Y.IsSandbox: refused by policy: server class <name> is neither registered"
        );
        assert_eq!(normalise_reason("Test01 does not fit"), "Test01 does not fit");
        assert_eq!(
            normalise_reason("it doesn't fit and don't try"),
            "it doesn't fit and don't try"
        );
    }

    // al-runner issue 8104: a reference to a decision record is part of the cause, not a number to fold.
    // al-runner issue 8142: the routine a refusal of the company open stopped in does not split its group.
    #[test]
    fn the_site_a_reason_stopped_in_does_not_split_its_group() {
        let plain =
            "OnAfterLogin raised to the login subscribers of the apps under test before the first test: Clear of a X";
        let at = format!("{plain} [in codeunit 50110 \"Repro\".Reset at LoginApp: Repro.Codeunit.al:9:9]");
        assert_eq!(normalise_reason(&at), normalise_reason(plain));
        // A bracket that is not the site stays.
        assert_ne!(
            normalise_reason("Get of a row [in a key]"),
            normalise_reason("Get of a row")
        );
    }

    #[test]
    fn an_adr_reference_keeps_its_numbers() {
        assert_eq!(
            normalise_reason("FindSet on 'X': the rows (ADR-013 decision 5) and 7 more"),
            "FindSet on <name>: the rows (ADR-013 decision 5) and N more"
        );
        assert_eq!(
            normalise_reason("on the Company table (ADR-046 decision 4a)"),
            "on the Company table (ADR-046 decision 4a)"
        );
        assert_eq!(
            normalise_reason("ADR-066 amendment 12, decision 3 of 4"),
            "ADR-066 amendment 12, decision 3 of N"
        );
        assert_eq!(normalise_reason("decision 5 alone"), "decision N alone");
    }
}
