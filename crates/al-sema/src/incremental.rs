//! Incremental compile of a warm workspace: what is needed to take a changed file without compiling
//! everything again (`docs/adr/029-incremental-compile.md`).

use al_syntax::{SyntaxKind, SyntaxNode};
use std::hash::{Hash, Hasher};

/// A fingerprint of the declarations of a parsed file: every token outside the bodies of its
/// procedures and triggers (the local `var` sections and the `begin ... end` blocks), and the kinds of
/// the nodes around them. Trivia (white space, comments, directives) is left out. Two files with the
/// same skeleton declare the same objects, fields, procedures, signatures, attributes, properties and
/// variables; they differ only in the code of routines.
pub fn skeleton(green: &rowan::GreenNode) -> u64 {
    let root = SyntaxNode::new_root(green.clone());
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut it = root.preorder_with_tokens();
    while let Some(ev) = it.next() {
        match ev {
            rowan::WalkEvent::Enter(rowan::NodeOrToken::Node(n)) => {
                let body = matches!(n.kind(), SyntaxKind::VarSection | SyntaxKind::Block)
                    && n.parent()
                        .is_some_and(|p| matches!(p.kind(), SyntaxKind::Procedure | SyntaxKind::Trigger));
                if body {
                    // A marker where the code was (a routine with a body and one without differ); the
                    // local variables leave no mark: adding one changes nothing outside the routine.
                    if n.kind() == SyntaxKind::Block {
                        (n.kind() as u16).hash(&mut h);
                    }
                    it.skip_subtree();
                } else {
                    (n.kind() as u16).hash(&mut h);
                }
            }
            rowan::WalkEvent::Enter(rowan::NodeOrToken::Token(t)) => {
                if !t.kind().is_trivia() {
                    (t.kind() as u16).hash(&mut h);
                    t.text().hash(&mut h);
                }
            }
            rowan::WalkEvent::Leave(_) => {}
        }
    }
    h.finish()
}

/// Whether the code of these objects can change without the lowering of any other object changing
/// (`lower::relower_objects`). Codeunits that implement no interface, tables and table extensions qualify.
/// Other kinds are read by the lowering of other objects through their code (a page by its extensions'
/// controls, a report by its extensions' triggers, and so on), and are compiled again with the complete
/// compile.
pub fn eligible(ws: &crate::Workspace, objects: &[usize]) -> Result<(), String> {
    for &oi in objects {
        let o = &ws.objects[oi];
        if ws.is_dependency(oi) {
            return Err("an object of a dependency app".into());
        }
        if !matches!(
            o.kind,
            crate::ObjKind::Codeunit | crate::ObjKind::Table | crate::ObjKind::TableExtension
        ) {
            return Err(format!("{} {} is not a codeunit or table", o.kind.label(), o.name));
        }
        if !o.implements.is_empty() {
            return Err(format!("codeunit {} implements an interface", o.name));
        }
    }
    Ok(())
}

/// Whether the declarations of these objects can change and the program still be renewed by lowering the
/// objects and their readers again (`readers`, `lower::relower_objects`): codeunits that implement nothing.
/// Everything else is read by the lowering of other objects without a name (the fields of a table by every
/// extension, the controls of a page, the implementers of an interface) and is compiled again with the complete
/// compile.
pub fn eligible_for_declarations(ws: &crate::Workspace, objects: &[usize]) -> Result<(), String> {
    for &oi in objects {
        let o = &ws.objects[oi];
        if ws.is_dependency(oi) {
            return Err("an object of a dependency app".into());
        }
        if o.kind != crate::ObjKind::Codeunit {
            return Err(format!("{} {} is not a codeunit", o.kind.label(), o.name));
        }
        if !o.implements.is_empty() {
            return Err(format!("codeunit {} implements an interface", o.name));
        }
        if !o.name.is_ascii() {
            return Err(format!("the name of codeunit {} is not ASCII", o.name));
        }
    }
    Ok(())
}

/// The objects of the workspace's own apps that can read the declarations of `changed`: those whose code calls a routine
/// of a changed object in the program before the change (`Lowered::calls`), and those whose file mentions the name of a
/// changed object or its id. A call finds a reader that reaches the object without naming it (through a procedure of
/// another codeunit that returns it, al-runner issue 7625). A name or number finds the rest: a variable type,
/// `Codeunit::"Name"`, an attribute, `Codeunit.Run(50100)`.
/// A call or member that did not resolve before the change is in no call list, and may resolve to a changed object after
/// it. So an object that is neither kind of reader is an error when the program before the change holds one for it: a
/// call of its code with no callee the checker resolved (`Lowered::unresolved_calls`; a `--tdd` procedure stub site is
/// one), or a compile error in its file (`Lowered::al_diagnostics`: an AL0132 member, an AL0130 `var` argument, any
/// other error that may read a declaration).
/// More objects than `limit` is an error: lowering them again costs as much as the complete compile.
/// The caller compiles everything again on an error.
pub fn readers(
    ws: &crate::Workspace,
    lowered: &crate::lower::Lowered,
    changed: &[usize],
    limit: usize,
) -> Result<Vec<usize>, String> {
    let names: Vec<String> = changed
        .iter()
        .map(|&o| ws.objects[o].name.to_ascii_lowercase())
        .collect();
    let ids: Vec<String> = changed
        .iter()
        .filter_map(|&o| ws.objects[o].id.map(|i| i.to_string()))
        .collect();
    let calls = &lowered.calls;
    let calls_changed = |oi: usize| oi < calls.len() && calls.of(oi).iter().flatten().any(|(o, _)| changed.contains(o));
    let mut mentioned = vec![false; ws.files.len()];
    let mut had_error = vec![false; ws.files.len()];
    for (fi, f) in ws.files.iter().enumerate() {
        if f.lazy || ws.kinds[f.app] == crate::AppKind::Dependency {
            continue;
        }
        let text = ws.text(fi);
        mentioned[fi] =
            names.iter().any(|n| contains_ignore_case(text, n)) || ids.iter().any(|i| contains_number(text, i));
        had_error[fi] = lowered
            .al_diagnostics
            .iter()
            .any(|d| d.app == f.app && d.file == f.path);
    }
    let mut out = Vec::new();
    for oi in 0..ws.objects.len() {
        if changed.contains(&oi) || ws.is_dependency(oi) {
            continue;
        }
        let file = ws.objects[oi].file;
        if mentioned[file] || calls_changed(oi) {
            out.push(oi);
        } else if had_error[file] || lowered.unresolved_calls.get(oi).copied().unwrap_or(true) {
            let o = &ws.objects[oi];
            return Err(format!(
                "{} {} has a call or member the compile before the change did not resolve",
                o.kind.label(),
                o.name
            ));
        }
        if out.len() > limit {
            return Err(format!("{} objects or more read the changed declarations", out.len()));
        }
    }
    Ok(out)
}

/// Whether `text` holds `needle` (lower case ASCII), whatever the case.
fn contains_ignore_case(text: &str, needle: &str) -> bool {
    let (t, n) = (text.as_bytes(), needle.as_bytes());
    if n.is_empty() || n.len() > t.len() {
        return false;
    }
    let first = n[0];
    let (lo, hi) = (first, first.to_ascii_uppercase());
    let mut at = 0;
    while at + n.len() <= t.len() {
        match t[at..=t.len() - n.len()].iter().position(|&b| b == lo || b == hi) {
            None => return false,
            Some(i) => {
                let start = at + i;
                if t[start..start + n.len()].eq_ignore_ascii_case(n) {
                    return true;
                }
                at = start + 1;
            }
        }
    }
    false
}

/// Whether `text` holds the number `digits` on its own (not as part of a longer number).
fn contains_number(text: &str, digits: &str) -> bool {
    let (t, n) = (text.as_bytes(), digits.as_bytes());
    let mut from = 0;
    while let Some(i) = text[from..].find(digits) {
        let (a, b) = (from + i, from + i + n.len());
        let before = a == 0 || !t[a - 1].is_ascii_alphanumeric();
        let after = b >= t.len() || !t[b].is_ascii_alphanumeric();
        if before && after {
            return true;
        }
        from = a + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{AppSource, Workspace};

    fn fingerprint(src: &str) -> u64 {
        let parse = al_syntax::parse(src, &Default::default());
        assert!(parse.errors.is_empty(), "{:?}", parse.errors);
        skeleton(&parse.green)
    }

    const BASE: &str = r#"
codeunit 50100 C
{
    Subtype = Test;
    var
        Global: Integer;

    [Test]
    procedure One(N: Integer): Integer
    var
        Loc: Integer;
    begin
        Loc := N;
        exit(Loc);
    end;

    trigger OnRun()
    begin
    end;
}
"#;

    #[test]
    fn the_code_of_routines_is_not_part_of_the_skeleton() {
        let same = |from: &str, to: &str| {
            assert!(BASE.contains(from), "{from}");
            assert_eq!(
                fingerprint(BASE),
                fingerprint(&BASE.replace(from, to)),
                "{from} -> {to}"
            );
        };
        same("Loc := N;", "Loc := N + 1; Loc := Loc * 2;");
        same("exit(Loc);", "// a comment\n        exit(Loc);");
        same("Loc: Integer;", "Loc: Integer;\n        More: Text;");
        same("Loc: Integer;", "Loc: Integer; // gone\n        More: Record Customer;");
        same("    begin\n    end;", "    begin\n        Clear(Global);\n    end;");
        // White space and comments anywhere do not matter.
        assert_eq!(fingerprint(BASE), fingerprint(&format!("// head\n\n{BASE}\n\n")));
    }

    #[test]
    fn declarations_are_part_of_the_skeleton() {
        let differs = |from: &str, to: &str| {
            assert!(BASE.contains(from), "{from}");
            assert_ne!(
                fingerprint(BASE),
                fingerprint(&BASE.replace(from, to)),
                "{from} -> {to}"
            );
        };
        differs("procedure One(N: Integer)", "procedure One(N: Integer; M: Integer)");
        differs("procedure One(N: Integer): Integer", "procedure One(N: Integer): Text");
        differs("procedure One", "local procedure One");
        differs("procedure One", "procedure Two");
        differs("[Test]", "[Test]\n    [HandlerFunctions('H')]");
        differs("Global: Integer;", "Global: Text;");
        differs("Subtype = Test;", "Subtype = Normal;");
        differs("codeunit 50100 C", "codeunit 50101 C");
        differs("trigger OnRun()", "trigger OnRun(Z: Integer)");
        // A routine more.
        differs(
            "    trigger OnRun()",
            "    procedure Extra() begin end;\n    trigger OnRun()",
        );
    }

    fn app(text: &str) -> Vec<AppSource> {
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![
                (
                    "First.al".into(),
                    "codeunit 50001 First { procedure P() begin end; }".into(),
                ),
                ("Second.al".into(), text.into()),
            ],
        }]
    }

    fn load(text: &str) -> Workspace {
        Workspace::load(app(text), &Default::default())
    }

    /// A workspace that took a text with `replace_file` is the workspace that loads that text.
    fn assert_same(a: &Workspace, b: &Workspace) {
        let key = |w: &Workspace| {
            w.objects
                .iter()
                .map(|o| (o.file, o.kind, o.id, o.name.clone(), o.offset, o.ordinal))
                .collect::<Vec<_>>()
        };
        assert_eq!(key(a), key(b));
        assert_eq!(a.diagnostics, b.diagnostics);
        for fi in 0..a.files.len() {
            assert_eq!(a.text(fi), b.text(fi));
            assert_eq!(a.apps[0].files[fi], b.apps[0].files[fi]);
            assert_eq!(
                a.green(fi).to_string(),
                b.green(fi).to_string(),
                "the tree of file {fi}"
            );
        }
        for oi in 0..a.objects.len() {
            assert_eq!(
                a.object_ast(oi).syntax().to_string(),
                b.object_ast(oi).syntax().to_string()
            );
        }
    }

    #[test]
    fn a_workspace_that_takes_a_changed_text_is_the_one_that_loads_it() {
        let old =
            "// head\ncodeunit 50002 Second\n{\n    procedure Q(): Integer\n    begin\n        exit(1);\n    end;\n}\n";
        let new = "// head\n// more head\ncodeunit 50002 Second\n{\n    procedure Q(): Integer\n    var\n        Z: Record Missing;\n    begin\n        exit(2);\n    end;\n}\n";
        let mut ws = load(old);
        let objects = ws.replace_file(1, new.into()).unwrap();
        assert_eq!(objects, vec![1]);
        assert_same(&ws, &load(new));
        // The unresolved record type in the new body is a diagnostic, as in a load.
        assert!(
            ws.diagnostics.iter().any(|d| d.code == "AR0002"),
            "{:?}",
            ws.diagnostics
        );
        // And back: the diagnostic goes.
        ws.replace_file(1, old.into()).unwrap();
        assert_same(&ws, &load(old));
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    }

    #[test]
    fn a_text_that_changes_a_declaration_or_does_not_parse_is_refused_and_changes_nothing() {
        let old = "codeunit 50002 Second\n{\n    procedure Q(): Integer\n    begin\n        exit(1);\n    end;\n}\n";
        let mut ws = load(old);
        for (why, new) in [
            ("a declaration", old.replace("Q(): Integer", "Q(): Text")),
            ("the name", old.replace("Second", "Third")),
            ("the id", old.replace("50002", "50009")),
            ("an object more", format!("{old}\ncodeunit 50003 More {{ }}")),
            ("a parse error", old.replace("exit(1);", "exit(;")),
        ] {
            assert!(ws.replace_file(1, new).is_err(), "{why}");
            assert_same(&ws, &load(old));
        }
        assert!(ws.replace_file(7, "x".into()).is_err());
    }
}
