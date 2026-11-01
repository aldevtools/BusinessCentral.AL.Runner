//! The compile errors of a page control's `Visible` that corpus writer Y65 measured (al-runner issue 6675; must-not-compile
//! 2630-2632 on corpus branch al-runtime-rs-staging-y65, commit eefaf01b, alc 30.0 with BC 27, 28 and 29 symbols).
//!
//! A field's Visible is a client expression: an array element is AL0322 "Array access is not valid for client expressions."
//! (2631 `ShowArr[1]`), a method call AL0322 "Procedure calls is not valid for client expressions." (2632 `not
//! Rec.IsEmpty()`; a bare call of a page procedure is `field_control_procedure_calls`, case 2000). A group, cuegroup, fixed, grid,
//! repeater, part or action takes any Boolean expression there, and an Integer literal is AL0208 on a cuegroup (2630
//! `Visible = 5`).

use super::{offset_of, push};
use crate::workspace::{Diagnostic, Workspace};
use al_syntax::ast;

const CLIENT_TAIL: &str = " Client expressions can only use simple data types and field references.";

/// AL0322 and AL0208 for the `Visible` properties of the controls of page `oi` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, procedures: &[String], out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for s in crate::pages::control_sections(obj) {
        let kind = s.name().to_ascii_lowercase();
        for p in s.properties().filter(|p| p.name().eq_ignore_ascii_case("visible")) {
            let value = p.value_text();
            if kind == "cuegroup" && is_integer_literal(&value) {
                push(
                    out,
                    ws,
                    file,
                    offset_of(p.syntax()),
                    "AL0208",
                    "The expression must be of Boolean type. Use a comparison or boolean expression.".to_string(),
                );
                continue;
            }
            if kind != "field" {
                continue;
            }
            let shape = scan(&value);
            // A bare call of one of the page's procedures is reported by `field_control_procedure_calls`.
            let bare_page_call =
                crate::pages::bare_call_name(&value).is_some_and(|f| procedures.contains(&f.to_ascii_lowercase()));
            let message = if shape.call && !bare_page_call {
                "Procedure calls is not valid for client expressions."
            } else if shape.array && !shape.call {
                "Array access is not valid for client expressions."
            } else {
                continue;
            };
            push(
                out,
                ws,
                file,
                offset_of(p.syntax()),
                "AL0322",
                format!("{message}{CLIENT_TAIL}"),
            );
        }
    }
}

fn is_integer_literal(value: &str) -> bool {
    let v = value.trim();
    !v.is_empty() && v.chars().all(|c| c.is_ascii_digit())
}

/// What an expression holds outside its text literals and quoted names.
#[derive(Debug, Default, PartialEq, Eq)]
struct Shape {
    /// A name followed by `(` that is no operator word (a procedure call or a method call).
    call: bool,
    /// An index `[`.
    array: bool,
}

fn scan(value: &str) -> Shape {
    let mut shape = Shape::default();
    let b: Vec<char> = value.chars().collect();
    let mut i = 0;
    let mut prev_word: Option<String> = None;
    while i < b.len() {
        let c = b[i];
        if c == '\'' || c == '"' {
            let start = i;
            i += 1;
            while i < b.len() && b[i] != c {
                i += 1;
            }
            i += 1;
            // A quoted name can be called (`"Is Shown"()`); a text literal cannot.
            prev_word = (c == '"').then(|| b[start..i.min(b.len())].iter().collect());
            continue;
        }
        if c.is_alphanumeric() || c == '_' {
            let start = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_') {
                i += 1;
            }
            prev_word = Some(b[start..i].iter().collect());
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => {
                if prev_word.as_ref().is_some_and(|w| {
                    !w.starts_with(|d: char| d.is_ascii_digit())
                        && !["and", "or", "not", "xor", "div", "mod", "in"].contains(&w.to_ascii_lowercase().as_str())
                }) {
                    shape.call = true;
                }
            }
            '[' => shape.array = true,
            _ => {}
        }
        prev_word = None;
        i += 1;
    }
    shape
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_and_array_elements_are_found_outside_literals() {
        assert_eq!(
            scan("not Rec.IsEmpty()"),
            Shape {
                call: true,
                array: false
            }
        );
        assert_eq!(
            scan("UserId() = ''"),
            Shape {
                call: true,
                array: false
            }
        );
        assert_eq!(
            scan("ShowArr[1]"),
            Shape {
                call: false,
                array: true
            }
        );
        assert_eq!(scan("not (A and B)"), Shape::default());
        assert_eq!(scan("Rec.Name = 'x(1)[2]'"), Shape::default());
        assert_eq!(
            scan("\"Is Shown\"()"),
            Shape {
                call: true,
                array: false
            }
        );
        assert!(is_integer_literal(" 5 "));
        assert!(!is_integer_literal("ShowIt"));
    }
}
