//! Token-level preprocessor: evaluates `#if` / `#elif` / `#else` / `#endif`
//! against a symbol set and folds every run of tokens in an inactive branch
//! into one `DisabledText` trivia token, so the output still covers the source
//! byte for byte. `#define` / `#undef` change the symbol set from that point on.
//! `#region`, `#endregion` and `#pragma` are left as plain directives.

use crate::{SyntaxKind, Token};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Byte offset of the offending directive.
    pub offset: usize,
    pub message: String,
}

pub struct Preprocessed {
    pub tokens: Vec<Token>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Symbols are matched case-sensitively: corpus permissions/BmPreprocessorDefine.al:83
/// (`#if ltbmdefined` after `#define LTBMDEFINED` is false; BmPreprocessorScope.al:35).
pub fn preprocess(src: &str, tokens: &[Token], symbols: &BTreeSet<String>) -> Preprocessed {
    let mut syms: BTreeSet<String> = symbols.clone();
    let mut frames: Vec<Frame> = Vec::new();
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut diagnostics = Vec::new();
    let mut pos = 0usize;
    let mut after_directive = false;
    for &t in tokens {
        let text = &src[pos..pos + t.len as usize];
        let active = frames.last().is_none_or(|f| f.active);
        if t.kind == SyntaxKind::Directive {
            let mut diag = |m: String| {
                diagnostics.push(Diagnostic {
                    offset: pos,
                    message: m,
                })
            };
            let body = text[1..].split("//").next().unwrap_or("").trim();
            let (word, rest) = body.split_once(char::is_whitespace).unwrap_or((body, ""));
            match word.to_ascii_lowercase().as_str() {
                "if" => {
                    let cond = active
                        && eval(rest, &syms).unwrap_or_else(|e| {
                            diag(e);
                            false
                        });
                    frames.push(Frame {
                        parent_active: active,
                        active: cond,
                        taken: cond,
                        seen_else: false,
                    });
                }
                "elif" => match frames.last_mut() {
                    Some(f) if !f.seen_else => {
                        let cond = f.parent_active
                            && !f.taken
                            && eval(rest, &syms).unwrap_or_else(|e| {
                                diag(e);
                                false
                            });
                        f.active = cond;
                        f.taken |= cond;
                    }
                    Some(_) => diag("#elif after #else".into()),
                    None => diag("#elif without #if".into()),
                },
                "else" => match frames.last_mut() {
                    Some(f) if !f.seen_else => {
                        f.active = f.parent_active && !f.taken;
                        f.taken = true;
                        f.seen_else = true;
                    }
                    Some(_) => diag("second #else".into()),
                    None => diag("#else without #if".into()),
                },
                "endif" => {
                    if frames.pop().is_none() {
                        diag("#endif without #if".into());
                    }
                }
                "define" if active => {
                    syms.insert(rest.trim().to_string());
                }
                "undef" if active => {
                    let name = rest.trim();
                    // Whether `#undef x` removes a symbol defined only as `X` is not pinned (`#if` is case-sensitive,
                    // BmPreprocessorDefine.al:83): the file is refused (docs/probes-needed.md).
                    if !syms.contains(name) && syms.iter().any(|s| s.eq_ignore_ascii_case(name)) {
                        diag(format!(
                            "#undef {name} names a symbol that is defined only in another case (not pinned)"
                        ));
                    }
                    syms.remove(name);
                }
                _ => {}
            }
            out.push(t);
            after_directive = true;
        } else if active || (after_directive && t.kind == SyntaxKind::Newline) {
            out.push(t);
            after_directive = false;
        } else {
            match out.last_mut() {
                Some(last) if last.kind == SyntaxKind::DisabledText => last.len += t.len,
                _ => out.push(Token {
                    kind: SyntaxKind::DisabledText,
                    len: t.len,
                }),
            }
            after_directive = false;
        }
        pos += t.len as usize;
    }
    if !frames.is_empty() {
        diagnostics.push(Diagnostic {
            offset: src.len(),
            message: format!("{} unterminated #if", frames.len()),
        });
    }
    Preprocessed {
        tokens: out,
        diagnostics,
    }
}

struct Frame {
    parent_active: bool,
    active: bool,
    /// Some branch of this `#if` chain has already been taken.
    taken: bool,
    seen_else: bool,
}

/// `expr := or; or := and ('or' and)*; and := not ('and' not)*; not := 'not' not | atom;
/// atom := SYMBOL | '(' or ')'`
fn eval(expr: &str, syms: &BTreeSet<String>) -> Result<bool, String> {
    let spaced = expr.replace('(', " ( ").replace(')', " ) ");
    // The keywords `not`, `and` and `or` are matched in any case; symbols keep their case.
    let toks: Vec<String> = spaced
        .split_whitespace()
        .map(|w| match w.to_ascii_uppercase().as_str() {
            k @ ("NOT" | "AND" | "OR") => k.to_string(),
            _ => w.to_string(),
        })
        .collect();
    let mut i = 0;
    let v = or(&toks, &mut i, syms)?;
    if i != toks.len() {
        return Err(format!("unexpected `{}` in #if expression", toks[i]));
    }
    Ok(v)
}

fn or(t: &[String], i: &mut usize, s: &BTreeSet<String>) -> Result<bool, String> {
    let mut v = and(t, i, s)?;
    while t.get(*i).is_some_and(|w| w == "OR") {
        *i += 1;
        v |= and(t, i, s)?;
    }
    Ok(v)
}

fn and(t: &[String], i: &mut usize, s: &BTreeSet<String>) -> Result<bool, String> {
    let mut v = not(t, i, s)?;
    while t.get(*i).is_some_and(|w| w == "AND") {
        *i += 1;
        v &= not(t, i, s)?;
    }
    Ok(v)
}

fn not(t: &[String], i: &mut usize, s: &BTreeSet<String>) -> Result<bool, String> {
    match t.get(*i).map(String::as_str) {
        Some("NOT") => {
            *i += 1;
            Ok(!not(t, i, s)?)
        }
        Some("(") => {
            *i += 1;
            let v = or(t, i, s)?;
            if t.get(*i).map(String::as_str) != Some(")") {
                return Err("missing `)` in #if expression".into());
            }
            *i += 1;
            Ok(v)
        }
        Some(w) if w != ")" && w != "AND" && w != "OR" => {
            *i += 1;
            Ok(s.contains(w))
        }
        _ => Err("expected a symbol in #if expression".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex;
    use SyntaxKind::DisabledText;

    fn run(src: &str, syms: &[&str]) -> (Vec<(SyntaxKind, String)>, Vec<Diagnostic>) {
        let syms = syms.iter().map(|s| s.to_string()).collect();
        let p = preprocess(src, &lex(src), &syms);
        let mut pos = 0;
        let mut out = Vec::new();
        for t in &p.tokens {
            let end = pos + t.len as usize;
            out.push((t.kind, src[pos..end].to_string()));
            pos = end;
        }
        assert_eq!(pos, src.len(), "preprocessed tokens must still cover the source");
        (out, p.diagnostics)
    }

    fn code(src: &str, syms: &[&str]) -> Vec<String> {
        let (toks, diags) = run(src, syms);
        assert!(diags.is_empty(), "{diags:?}");
        toks.into_iter()
            .filter(|(k, _)| !k.is_trivia())
            .map(|(_, t)| t)
            .collect()
    }

    const SRC: &str = "a;\n#if BC29PLUS\nb;\n#else\nc;\n#endif\nd;\n";

    #[test]
    fn defined_symbol_keeps_the_if_branch() {
        assert_eq!(code(SRC, &["BC29PLUS"]), vec!["a", ";", "b", ";", "d", ";"]);
    }

    #[test]
    fn undefined_symbol_keeps_the_else_branch() {
        assert_eq!(code(SRC, &["BC27PLUS"]), vec!["a", ";", "c", ";", "d", ";"]);
    }

    #[test]
    fn undef_of_a_symbol_defined_only_in_another_case_is_diagnosed() {
        let (_, diags) = run("#define X\n#undef x\n", &[]);
        assert_eq!(diags.len(), 1, "{diags:?}");
        let (_, diags) = run("#undef bc29plus\n", &["BC29PLUS"]);
        assert_eq!(diags.len(), 1, "{diags:?}");
        let (_, diags) = run("#define X\n#undef X\n#undef Y\n", &[]);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn symbols_are_case_sensitive() {
        // corpus permissions/BmPreprocessorDefine.al:83 (`#if ltbmdefined` after
        // `#define LTBMDEFINED` takes the #else branch), read by BmPreprocessorScope.al:25, :35.
        assert_eq!(code(SRC, &["bc29plus"]), vec!["a", ";", "c", ";", "d", ";"]);
        let src = "#define LTBMDEFINED\n#if ltbmdefined\na\n#else\nb\n#endif\n";
        assert_eq!(code(src, &[]), vec!["b"]);
    }

    #[test]
    fn inactive_code_is_one_disabled_text_token_per_run() {
        let (toks, _) = run(SRC, &[]);
        let disabled: Vec<_> = toks
            .iter()
            .filter(|(k, _)| *k == DisabledText)
            .map(|(_, t)| t.as_str())
            .collect();
        assert_eq!(disabled, vec!["b;\n"]);
    }

    #[test]
    fn not_and_or_and_parentheses() {
        let src = "#if not A\nx\n#endif\n#if A and (B or C)\ny\n#endif\n";
        assert_eq!(code(src, &["A"]), Vec::<String>::new());
        assert_eq!(code(src, &["A", "C"]), vec!["y"]);
        assert_eq!(code(src, &[]), vec!["x"]);
    }

    #[test]
    fn elif_takes_the_first_true_branch_only() {
        let src = "#if A\na\n#elif B\nb\n#elif C\nc\n#else\nd\n#endif\n";
        assert_eq!(code(src, &["B", "C"]), vec!["b"]);
        assert_eq!(code(src, &[]), vec!["d"]);
    }

    #[test]
    fn nested_if_inside_an_inactive_branch_stays_inactive() {
        let src = "#if A\n#if B\nx\n#endif\n#else\ny\n#endif\n";
        assert_eq!(code(src, &["B"]), vec!["y"]);
    }

    #[test]
    fn define_and_undef_affect_later_directives() {
        let src = "#define X\n#if X\na\n#endif\n#undef X\n#if X\nb\n#endif\n";
        assert_eq!(code(src, &[]), vec!["a"]);
    }

    #[test]
    fn a_trailing_comment_on_a_directive_is_ignored() {
        assert_eq!(code("#if A // because\nx\n#endif\n", &["A"]), vec!["x"]);
    }

    #[test]
    fn region_and_pragma_are_plain_directives() {
        assert_eq!(
            code("#region R\n#pragma warning disable AA0001\nx\n#endregion\n", &[]),
            vec!["x"]
        );
    }

    #[test]
    fn unbalanced_directives_are_diagnosed() {
        assert_eq!(run("#if A\nx\n", &[]).1.len(), 1);
        assert_eq!(run("#endif\n", &[]).1.len(), 1);
        assert_eq!(run("#else\n", &[]).1.len(), 1);
    }
}
