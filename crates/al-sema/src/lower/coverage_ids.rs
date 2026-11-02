//! The `Test ID` of the per-test code coverage tables (Code Coverage Tests Run, Code Coverage Test Lookup).
//!
//! The corpus pins only shapes of the value, never the number (BC computes it per run, codeunit/
//! TestCodeCoveragePerTest.al:14): it is the same for the tests of a recording and for two recordings of one test
//! (`a - b` is 0, `a = b`, `a <> b`: coverage/W6CodeCoverageIds.al:111-112, :167), it is not 1 (:168), and the lookup
//! rows of a test carry it (`Lookup.SetRange("Test ID", Run."Test ID")`: codeunit/TestCodeCoveragePerTest.al:116).
//! A read of the value is lowered only in one of those shapes, directly or through an Integer local that only ever
//! holds a Test ID; any other read would show the runtime's own number and is refused.
use super::*;
use al_builtins::platform::{COVERAGE_TEST_ID, coverage_test_id_refusal};
use al_syntax::ast::NodeText;

impl Lowerer<'_> {
    /// Whether field `id` of table object `table` is a coverage `Test ID`.
    pub(crate) fn is_coverage_test_id(&self, table: usize, id: u32) -> bool {
        self.ws.objects[table]
            .id
            .is_some_and(|t| COVERAGE_TEST_ID.contains(&(t, id)))
    }

    /// `n` reads the coverage Test ID of a stored record (field `id` of `table`): refused unless it is a pinned shape.
    pub(crate) fn coverage_test_id_read(&self, n: &SyntaxNode, table: usize, id: u32) -> L<()> {
        if !self.is_coverage_test_id(table, id) {
            return Ok(());
        }
        let (child, parent) = up(n);
        if let Some(p) = &parent {
            // The target of an assignment is a write.
            if p.kind() == SyntaxKind::AssignStmt && p.children().next().as_ref() == Some(&child) {
                return Ok(());
            }
            // `Holder := Run."Test ID"`: the local's every other use must be a pinned shape.
            if let Some(name) = self.holder_target(p, &child) {
                if self.is_holder(p, &name) && self.holder_uses_pinned(p, &name) {
                    return Ok(());
                }
                return Err(coverage_test_id_refusal(&format!(
                    "`{}` keeps the Test ID in a variable that is used in another way",
                    p.text_string().trim()
                )));
            }
        }
        if self.pinned_use(n) {
            return Ok(());
        }
        Err(coverage_test_id_refusal(&format!(
            "`{}`",
            parent.as_ref().unwrap_or(n).text_string().trim()
        )))
    }

    /// `n` reads the "Method ID" of a stored Code Coverage Test Lookup row (field 3 of 2000000288) or Tests Run row (field 3 of
    /// 2000000289). BC's numbers are compiler-assigned ids (Test Lookup 31658110 for `RecordedInner`, Tests Run 469735107 for
    /// `RecordedOuter`, codeunit/PWCodeCoverageRows.al:308-309; per name and signature and per test, coverage/
    /// PxCodeCoverageIds.al:7-8, :186) whose rule is not known; the runtime's own numbers would show. Kept, because refusing them
    /// loses baseline cases: an argument of `AreNotEqual`/`AreEqual` (codeunit/TestCodeCoveragePerTest.al:121,
    /// TestLookup_TestStartedWhileRecording_HasTheMethodItRan, pinned: the two differ) and a comparison, directly or through an
    /// Integer local that is only compared (coverage/W6CodeCoverageIds.al:83-98 MethodId_SameBody_DifferentNames_TestsRunRows,
    /// :129-139 MethodId_TestsRunVersusTestLookup, :156-164 MethodId_SameProcedure_TwoRecordings). Any other read is refused.
    pub(crate) fn coverage_lookup_method_id_read(&self, n: &SyntaxNode, table: usize, id: u32) -> L<()> {
        if id != 3 || !matches!(self.ws.objects[table].id, Some(2_000_000_288 | 2_000_000_289)) {
            return Ok(());
        }
        let (child, parent) = up(n);
        if let Some(p) = &parent {
            if p.kind() == SyntaxKind::AssignStmt && p.children().next().as_ref() == Some(&child) {
                return Ok(());
            }
            if let Some(name) = self.holder_target(p, &child) {
                let only_compared = self
                    .local_refs(p, &name)
                    .iter()
                    .all(|r| Self::assigned_from(r).is_some() || Self::compared(r));
                if only_compared {
                    return Ok(());
                }
            } else if Self::compared(n) || Self::assert_argument(n) {
                return Ok(());
            }
        }
        Err(format!(
            "`{}`: the value of the Code Coverage Method ID is not pinned (a compiler-assigned id, codeunit/PWCodeCoverageRows.al:308-309)",
            parent.as_ref().unwrap_or(n).text_string().trim()
        ))
    }

    /// `x` is an operand of a comparison.
    fn compared(x: &SyntaxNode) -> bool {
        let (_, Some(p)) = up(x) else { return false };
        p.kind() == SyntaxKind::BinExpr
            && matches!(
                sig_tokens(&p).first().map(|t| t.text().to_string()).as_deref(),
                Some("=" | "<>" | "<" | ">" | "<=" | ">=")
            )
    }

    /// `x` is an argument of a call to a method named AreNotEqual or AreEqual.
    fn assert_argument(x: &SyntaxNode) -> bool {
        let (_, Some(p)) = up(x) else { return false };
        p.kind() == SyntaxKind::ArgList
            && p.parent()
                .filter(|c| c.kind() == SyntaxKind::CallExpr)
                .and_then(|c| c.children().next())
                .is_some_and(|f| {
                    let f = f.text_string();
                    let name = f.rsplit('.').next().unwrap_or("").trim().to_ascii_lowercase();
                    name == "arenotequal" || name == "areequal"
                })
    }

    /// A builtin call on a coverage record that would show the Test ID's value: a filter or a key given as a number or a
    /// text, a filter read back, `Get`.
    pub(crate) fn coverage_test_id_call(&self, id: &str, receiver: Option<&SyntaxNode>, args: &[SyntaxNode]) -> L<()> {
        let names_test_id = |a: &SyntaxNode| matches!(self.res(a), Some(Res::Field { table, id }) if self.is_coverage_test_id(*table, *id));
        let on_test_id = args.first().is_some_and(names_test_id);
        let refuse = |what: &str| Err(coverage_test_id_refusal(what));
        if id.starts_with("Record.SetRange#") && on_test_id {
            return match args.len() {
                1 => Ok(()),
                2 if self.test_id_like(&args[1]) => Ok(()),
                _ => refuse("SetRange of the Test ID to a value that is not a Test ID"),
            };
        }
        if (id.starts_with("Record.SetFilter#") || id.starts_with("Record.GetFilter#")) && on_test_id {
            return refuse("a filter text on the Test ID");
        }
        let stored_coverage = receiver.is_some_and(|r| {
            matches!(self.ty_ref(r), Ty::Record { table, temporary: false }
                if table.eq_ignore_ascii_case("Code Coverage Tests Run") || table.eq_ignore_ascii_case("Code Coverage Test Lookup"))
        });
        if stored_coverage && (id.starts_with("Record.Get#") || id.starts_with("Record.GetFilters#")) {
            return refuse(&format!(
                "{} on a code coverage table",
                id.split('#').next().unwrap_or(id)
            ));
        }
        Ok(())
    }

    /// The local an assignment `Local := <child>` writes, by name, when `child` is its source.
    fn holder_target(&self, p: &SyntaxNode, child: &SyntaxNode) -> Option<String> {
        if p.kind() != SyntaxKind::AssignStmt || !sig_tokens(p).iter().any(|t| t.kind() == SyntaxKind::Assign) {
            return None;
        }
        let mut kids = p.children();
        let (lhs, rhs) = (kids.next()?, kids.next()?);
        if &rhs != child {
            return None;
        }
        match self.res(&lhs) {
            Some(Res::Local(name)) if matches!(self.ty_ref(&lhs), Ty::Integer) => Some(name.clone()),
            _ => None,
        }
    }

    /// The references to local `name` in the routine around `at`.
    fn local_refs(&self, at: &SyntaxNode, name: &str) -> Vec<SyntaxNode> {
        let Some(routine) = at.ancestors().find_map(ast::Routine::cast) else {
            return Vec::new();
        };
        routine
            .syntax()
            .descendants()
            .filter(|d| d.kind() == SyntaxKind::NameRef && matches!(self.res(d), Some(Res::Local(l)) if l == name))
            .collect()
    }

    /// Whether `r` is the target of a plain assignment, and what is assigned.
    fn assigned_from(r: &SyntaxNode) -> Option<SyntaxNode> {
        let p = r.parent()?;
        let mut kids = p.children();
        (p.kind() == SyntaxKind::AssignStmt && kids.next().as_ref() == Some(r)).then(|| kids.next())?
    }

    /// Local `name` (an Integer) is only ever assigned a stored record's Test ID, with `:=`.
    fn is_holder(&self, at: &SyntaxNode, name: &str) -> bool {
        let refs = self.local_refs(at, name);
        let mut assigned = false;
        for r in &refs {
            let Some(p) = r.parent() else { continue };
            if p.kind() == SyntaxKind::AssignStmt && p.children().next().as_ref() == Some(r) {
                let plain = sig_tokens(&p).iter().any(|t| t.kind() == SyntaxKind::Assign);
                match Self::assigned_from(r) {
                    Some(rhs) if plain && self.direct_read(&rhs) => assigned = true,
                    _ => return false,
                }
            }
        }
        assigned
    }

    /// Every use of holder `name` that is not its assignment is a pinned shape.
    fn holder_uses_pinned(&self, at: &SyntaxNode, name: &str) -> bool {
        self.local_refs(at, name)
            .iter()
            .all(|r| Self::assigned_from(r).is_some() || self.pinned_use(r))
    }

    /// A stored record's Test ID, read as a field.
    fn direct_read(&self, e: &SyntaxNode) -> bool {
        let e = strip(e);
        if !matches!(e.kind(), SyntaxKind::MemberExpr | SyntaxKind::NameRef) {
            return false;
        }
        let temporary = e.kind() == SyntaxKind::MemberExpr
            && e.children()
                .next()
                .is_some_and(|b| matches!(self.ty_ref(&b), Ty::Record { temporary: true, .. }));
        !temporary && matches!(self.res(&e), Some(Res::Field { table, id }) if self.is_coverage_test_id(*table, *id))
    }

    /// A Test ID: read directly or held in an Integer local that only ever holds one.
    fn test_id_like(&self, e: &SyntaxNode) -> bool {
        let e = strip(e);
        if self.direct_read(&e) {
            return true;
        }
        match self.res(&e) {
            Some(Res::Local(name)) if e.kind() == SyntaxKind::NameRef => self.is_holder(&e, &name.clone()),
            _ => false,
        }
    }

    /// `x` (a Test ID) is used in a pinned shape: subtracted from or compared for equality with another Test ID, compared
    /// for equality with 1, or the value of `SetRange("Test ID", x)`.
    fn pinned_use(&self, x: &SyntaxNode) -> bool {
        let (child, Some(p)) = up(x) else { return false };
        match p.kind() {
            SyntaxKind::BinExpr => {
                let op = sig_tokens(&p).first().map(|t| t.text().to_string()).unwrap_or_default();
                let Some(other) = p.children().find(|k| k != &child) else {
                    return false;
                };
                match op.as_str() {
                    "-" => self.test_id_like(&other),
                    "=" | "<>" => self.test_id_like(&other) || is_literal_one(&other),
                    _ => false,
                }
            }
            SyntaxKind::ArgList => {
                let args: Vec<SyntaxNode> = p.children().collect();
                let Some(call) = p.parent().filter(|c| c.kind() == SyntaxKind::CallExpr) else {
                    return false;
                };
                let set_range = matches!(self.res(&call), Some(Res::Builtin(id)) if id.starts_with("Record.SetRange#"));
                set_range
                    && args.len() == 2
                    && args[1] == child
                    && matches!(self.res(&args[0]), Some(Res::Field { table, id }) if self.is_coverage_test_id(*table, *id))
            }
            _ => false,
        }
    }
}

/// `n` without the parentheses around it.
fn strip(n: &SyntaxNode) -> SyntaxNode {
    let mut n = n.clone();
    while n.kind() == SyntaxKind::ParenExpr {
        match n.children().next() {
            Some(k) => n = k,
            None => break,
        }
    }
    n
}

/// The outermost parenthesized form of `n` and the node that holds it.
fn up(n: &SyntaxNode) -> (SyntaxNode, Option<SyntaxNode>) {
    let mut child = n.clone();
    let mut parent = n.parent();
    while let Some(p) = parent.clone().filter(|p| p.kind() == SyntaxKind::ParenExpr) {
        child = p;
        parent = child.parent();
    }
    (child, parent)
}

fn is_literal_one(n: &SyntaxNode) -> bool {
    let n = strip(n);
    n.kind() == SyntaxKind::Literal && n.text_string().trim() == "1"
}
