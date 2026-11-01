//! The rows of the platform Windows Language table (2000000045) that the corpus pins, derived from the corpus so that
//! the runtime's table cannot drift from it and a new row never needs code.
//!
//! The pin is the folder [`FOLDER`] (al-runner issue 6558): every row of the table as the text of each field joined by
//! `|`, in the order of [`FIELDS`] (`1025|1025|Arabic (Saudi Arabia)|ARA|Yes|...|ar-SA`), in an `AddRow(<id>, <text>)`
//! call of an active branch for a major (`#if BC29PLUS` ... `#endif` gives one table per major). A major for which the
//! folder holds no row is not pinned, and the runtime keeps its older rows for it. The generated file is
//! `crates/al-vm/src/platform/windows_language_rows.rs`; the test `windows_language_rows_are_current`
//! (tests/windows_language_rows.rs) checks it, and `UPDATE_GOLDEN=1` rewrites it.

use std::collections::BTreeMap;

/// The corpus folder that pins the whole table, under the corpus checkout.
pub const FOLDER: &str = "tests/al-language/record/lt-y39-windows-language-table";

/// Why the rows of [`FOLDER`] are not served yet, or `None` once they are. The folder holds the 248 rows of the Windows
/// containers of BC 27.5 and 28.5 and of a BC 29 SaaS sandbox; the Linux CI containers give 208 or 212 rows built from ICU
/// data (Y39WindowsLanguageTable.al:9-17). Since corpus pin 7f7cdc2a the older tests that pinned the Linux values
/// (platform/lt-ag-contoso/AgWindowsLanguage.al) hold them back as `Suspect_` procedures, so the folder's rows are served.
pub const HELD: Option<&str> = None;

/// The field numbers of a row text, in order: every field of the table below the system fields.
pub const FIELDS: [u32; 16] = [1, 2, 10, 11, 20, 21, 22, 23, 24, 25, 30, 31, 32, 33, 34, 35];

/// The Language ID of a row text: `text` has one part per field of [`FIELDS`], numbers in the two Integer fields and
/// `Yes` or `No` in every Boolean field (20-25, 31-34).
fn row_text(text: &str) -> Option<i32> {
    let parts: Vec<&str> = text.split('|').collect();
    if parts.len() != FIELDS.len() {
        return None;
    }
    let id = parts[0].parse().ok()?;
    parts[1].parse::<i32>().ok()?;
    let boolean = |i: usize| matches!(parts[i], "Yes" | "No");
    ((4..10).all(boolean) && (11..15).all(boolean)).then_some(id)
}

/// The value of an AL string literal token.
fn literal(text: &str) -> String {
    text[1..text.len() - 1].replace("''", "'")
}

/// The rows of `src` active for `major`, by Language ID: each `AddRow(<id>, <text>)` call, where the text is string
/// literals and calls of a procedure without parameters whose body is `exit('<literal>')`, joined by `+`. A call that
/// cannot be read, a text that is no row of the id, or two different texts for one id are an error.
fn rows_of(file: &str, src: &str, major: u16, out: &mut BTreeMap<i32, String>) -> Result<(), String> {
    use al_syntax::SyntaxKind as K;
    let symbols = al_version::corpus_symbols(major);
    let tokens = al_syntax::lex(src);
    let tokens = al_syntax::preprocessor::preprocess(src, &tokens, &symbols).tokens;
    let mut sig: Vec<(K, &str)> = Vec::new();
    let mut pos = 0;
    for t in &tokens {
        let end = pos + t.len as usize;
        if !t.kind.is_trivia() {
            sig.push((t.kind, &src[pos..end]));
        }
        pos = end;
    }
    let is = |i: usize, k: K| sig.get(i).is_some_and(|t| t.0 == k);
    let word = |i: usize, w: &str| {
        sig.get(i)
            .is_some_and(|t| t.0 == K::Ident && t.1.eq_ignore_ascii_case(w))
    };
    // `procedure Name(): Text begin exit('x'); end`
    let mut texts: BTreeMap<String, String> = BTreeMap::new();
    for i in 0..sig.len() {
        if word(i, "procedure")
            && is(i + 1, K::Ident)
            && is(i + 2, K::LParen)
            && is(i + 3, K::RParen)
            && is(i + 4, K::Colon)
            && word(i + 5, "Text")
            && word(i + 6, "begin")
            && word(i + 7, "exit")
            && is(i + 8, K::LParen)
            && is(i + 9, K::StringLit)
            && is(i + 10, K::RParen)
        {
            texts.insert(sig[i + 1].1.to_ascii_lowercase(), literal(sig[i + 9].1));
        }
    }
    let mut i = 0;
    while i < sig.len() {
        if !(word(i, "AddRow") && is(i + 1, K::LParen) && !word(i.wrapping_sub(1), "procedure")) {
            i += 1;
            continue;
        }
        let at = format!("{file}: BC {major}: AddRow({}", sig.get(i + 2).map_or("", |t| t.1));
        let id: i32 = match (sig.get(i + 2), is(i + 3, K::Comma)) {
            (Some((K::Integer, n)), true) => n.parse().map_err(|_| format!("{at}: not a Language ID"))?,
            _ => return Err(format!("{at}: not `AddRow(<id>, <text>)`")),
        };
        let mut j = i + 4;
        let mut text = String::new();
        loop {
            match sig.get(j) {
                Some((K::StringLit, l)) => {
                    text.push_str(&literal(l));
                    j += 1;
                }
                Some((K::Ident, name)) if is(j + 1, K::LParen) && is(j + 2, K::RParen) => {
                    let v = texts
                        .get(&name.to_ascii_lowercase())
                        .ok_or_else(|| format!("{at}: `{name}()` is no procedure that exits with a literal"))?;
                    text.push_str(v);
                    j += 3;
                }
                _ => return Err(format!("{at}: the text is not literals and calls joined by `+`")),
            }
            if is(j, K::Plus) {
                j += 1;
            } else if is(j, K::RParen) {
                break;
            } else {
                return Err(format!("{at}: the text is not literals and calls joined by `+`"));
            }
        }
        if row_text(&text) != Some(id) {
            return Err(format!("{at}: `{text}` is no row of Language ID {id}"));
        }
        match out.get(&id) {
            Some(old) if *old != text => {
                return Err(format!(
                    "{file}: BC {major}: two rows for Language ID {id}: `{old}` and `{text}`"
                ));
            }
            _ => {
                out.insert(id, text);
            }
        }
        i = j + 1;
    }
    Ok(())
}

/// The pinned table of each major of `majors` that `files` (path, source) pin a row for: Language ID to row text.
pub fn tables(files: &[(String, String)], majors: &[u16]) -> Result<BTreeMap<u16, BTreeMap<i32, String>>, String> {
    let mut out = BTreeMap::new();
    for &major in majors {
        let mut rows = BTreeMap::new();
        for (file, src) in files {
            rows_of(file, src, major, &mut rows)?;
        }
        if !rows.is_empty() {
            out.insert(major, rows);
        }
    }
    Ok(out)
}

/// The generated Rust source of `crates/al-vm/src/platform/windows_language_rows.rs`: each row with the run of
/// consecutive majors that give it the same text.
pub fn to_rust(tables: &BTreeMap<u16, BTreeMap<i32, String>>, files: &[String]) -> String {
    let mut runs: Vec<(i32, u16, u16, &str)> = Vec::new();
    for (&major, rows) in tables {
        for (&id, text) in rows {
            match runs.iter_mut().find(|r| r.0 == id && r.2 + 1 == major && r.3 == text) {
                Some(r) => r.2 = major,
                None => runs.push((id, major, major, text)),
            }
        }
    }
    runs.sort_by_key(|r| (r.0, r.1));
    let mut s = String::new();
    s.push_str("// Generated by `al_test_cli::windows_language` from the corpus (");
    if files.is_empty() {
        s.push_str("no pin yet");
    } else {
        s.push_str(&files.join(", "));
    }
    s.push_str(").\n// Do not edit: UPDATE_GOLDEN=1 cargo test -p al-test-cli --test windows_language_rows\n\n");
    s.push_str("/// The field numbers of each row text of [`ROWS`], in order.\n");
    let fields = FIELDS.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
    s.push_str(&format!("pub const FIELDS: [u32; {}] = [{fields}];\n", FIELDS.len()));
    s.push_str("/// The majors whose whole table the corpus pins.\n");
    let majors = tables.keys().map(u16::to_string).collect::<Vec<_>>().join(", ");
    s.push_str(&format!("pub const MAJORS: &[u16] = &[{majors}];\n"));
    s.push_str(
        "/// Every pinned row, by Language ID: (first major, last major, the `Format` of each field of [`FIELDS`] joined by `|`).\n",
    );
    s.push_str("#[rustfmt::skip]\npub const ROWS: &[(u16, u16, &str)] = &[\n");
    for (_, a, b, text) in runs {
        s.push_str(&format!("    ({a}, {b}, {text:?}),\n"));
    }
    s.push_str("];\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW_1025: &str = "1025|1025|Arabic (Saudi Arabia)|ARA|Yes|Yes|Yes|Yes|No|Yes|720|No|No|No|No|ar-SA";

    fn file(body: &str) -> Vec<(String, String)> {
        let src = format!(
            "codeunit 1 X {{\n    local procedure Utf8CodePage(): Text\n    begin\n#if BC29PLUS\n        exit('1');\n#else\n        exit('65001');\n#endif\n    end;\n\n    local procedure Load()\n    begin\n{body}\n    end;\n}}\n"
        );
        vec![("a.al".to_string(), src)]
    }

    #[test]
    fn a_row_has_one_part_per_field_and_yes_or_no_in_the_boolean_fields() {
        assert_eq!(row_text(ROW_1025), Some(1025));
        assert_eq!(row_text("1025|Arabic (Saudi Arabia)|ARA|720|ar-SA"), None);
        assert_eq!(
            row_text(&ROW_1025.replace("|Yes|Yes|Yes|Yes|No|", "|Yes|x|Yes|Yes|No|")),
            None
        );
    }

    #[test]
    fn a_row_is_an_add_row_call_whose_text_may_call_a_procedure_that_exits_with_a_literal_per_major() {
        let files = file(&format!(
            "        AddRow(1025, '{ROW_1025}');\n        AddRow(1067, '1067|1067|Armenian (Armenia)|HYE|Yes|Yes|Yes|Yes|No|Yes|' + Utf8CodePage() + '|No|No|No|No|hy-AM');\n#if BC29PLUS\n        AddRow(1079, '1079|1079|Georgian (Georgia)|KAT|Yes|Yes|Yes|Yes|No|Yes|1|No|No|No|No|ka-GE');\n#endif"
        ));
        let t = tables(&files, &[27, 28, 29]).unwrap();
        assert_eq!(t.len(), 3);
        assert!(t[&29][&1067].contains("|1|"), "{:?}", t[&29]);
        assert!(t[&28][&1067].contains("|65001|"), "{:?}", t[&28]);
        assert!(!t[&28].contains_key(&1079) && t[&29].contains_key(&1079));
        let rust = to_rust(&t, &["a.al".to_string()]);
        assert!(rust.contains("pub const MAJORS: &[u16] = &[27, 28, 29];"), "{rust}");
        assert!(rust.contains(&format!("(27, 29, {ROW_1025:?})")), "{rust}");
        assert!(rust.contains("(27, 28, \"1067|"), "{rust}");
        assert!(rust.contains("(29, 29, \"1067|"), "{rust}");
        assert!(rust.contains("(29, 29, \"1079|"), "{rust}");
    }

    #[test]
    fn a_row_whose_id_or_text_cannot_be_read_is_an_error() {
        let two = file(&format!(
            "        AddRow(1025, '{ROW_1025}');\n        AddRow(1025, '{}');",
            ROW_1025.replace("ARA", "ARX")
        ));
        assert!(
            tables(&two, &[29])
                .unwrap_err()
                .contains("two rows for Language ID 1025")
        );
        let other = file(&format!("        AddRow(1026, '{ROW_1025}');"));
        assert!(tables(&other, &[29]).unwrap_err().contains("AddRow(1026"));
        let call = file("        AddRow(1025, '1025|' + Unknown() + '|x');");
        assert!(tables(&call, &[29]).unwrap_err().contains("Unknown"));
    }

    #[test]
    fn no_row_leaves_the_major_unpinned() {
        assert!(tables(&file(""), &[29]).unwrap().is_empty());
        assert!(to_rust(&BTreeMap::new(), &[]).contains("pub const MAJORS: &[u16] = &[];"));
    }
}
