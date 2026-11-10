//! The refusal key lists of every crate that refuses, together (ADR-071; owner decision 2026-11-10: one list per crate,
//! checked together by one test, `tests/suite/refusal_keys.rs`). Each list is the `ALL` its crate's
//! `refusal_keys!` call makes from `src/refusals/<key>.rs`. A crate that starts a list adds a row to [`LISTS`].
//! `docs/probes-needed.md` holds a table generated from these lists (`--refusals-doc` writes it; the test fails while
//! it is stale).

use al_builtins::refusal::RefusalKey;

/// Per crate that refuses: its folder under `crates/` and its list.
pub const LISTS: &[(&str, &[RefusalKey])] = &[("al-builtins", al_builtins::refusal::ALL)];

/// Every key with the folder of its crate, sorted by key.
pub fn all() -> Vec<(&'static str, RefusalKey)> {
    let mut v: Vec<_> = LISTS
        .iter()
        .flat_map(|(c, list)| list.iter().map(move |k| (*c, *k)))
        .collect();
    v.sort_by_key(|(_, k)| k.key());
    v
}

/// The table of `docs/probes-needed.md`.
pub fn markdown_table() -> String {
    let mut s = String::from("| Key | Area | Probe | Registered in |\n|---|---|---|---|\n");
    for (krate, k) in all() {
        let key = k.key();
        let area = key.split_once('.').map_or(key, |(a, _)| a);
        let probe = k.info().probe.text().replace('|', "\\|");
        s.push_str(&format!(
            "| `{key}` | {area} | {probe} | `crates/{krate}/src/refusals/{key}.rs` |\n"
        ));
    }
    s
}

/// The line before the generated table in `docs/probes-needed.md`.
pub const TABLE_BEGIN: &str = "<!-- refusal-keys:begin (generated from the refusal key lists; al-test-cli --refusals-doc or UPDATE_REFUSALS_DOC=1 on the test refusal_keys rewrites it) -->";
/// The line after it.
pub const TABLE_END: &str = "<!-- refusal-keys:end -->";

/// `docs/probes-needed.md` (`text`) with the table between its two markers replaced by [`markdown_table`]; the rest
/// of the file is kept.
pub fn probes_needed_doc(text: &str) -> Result<String, String> {
    let (Some(a), Some(b)) = (text.find("<!-- refusal-keys:begin"), text.find(TABLE_END)) else {
        return Err("docs/probes-needed.md has no refusal-keys block: add the two marker lines".into());
    };
    if b < a {
        return Err("docs/probes-needed.md: the refusal-keys end marker comes before the begin marker".into());
    }
    Ok(format!(
        "{}{TABLE_BEGIN}\n{}{TABLE_END}{}",
        &text[..a],
        markdown_table(),
        &text[b + TABLE_END.len()..]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_needed_doc_replaces_only_the_table_between_the_markers() {
        let text = format!("# Probes\n\nintro\n{TABLE_BEGIN}\n| old |\n{TABLE_END}\n\nrest\n");
        let new = probes_needed_doc(&text).unwrap();
        assert_eq!(
            new,
            format!(
                "# Probes\n\nintro\n{TABLE_BEGIN}\n{}{TABLE_END}\n\nrest\n",
                markdown_table()
            )
        );
        assert_eq!(probes_needed_doc(&new).unwrap(), new);
        assert!(probes_needed_doc("no markers here").is_err());
    }
}
