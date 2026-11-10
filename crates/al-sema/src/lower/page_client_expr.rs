//! What the lowering of a page does with a Boolean control expression BC compiles on the client when the page opens
//! (`crate::pages::client_expression`): refuse what the client compile fails on, set aside a BigInteger literal, and check
//! the types behind a qualified value through a member (`E.Mode::Beta`).
use super::*;
use al_builtins::assumptions::Assumption;
use std::cell::RefCell;

impl Lowerer<'_> {
    /// Checks Boolean control expression `value` of property `name` (written at `off` in `owner`, the page or an extension)
    /// of control `control`. What the client compile fails on goes to `unsupported`, a BigInteger literal to `big_integer`.
    /// A control expression of an extension that runs through an Enum field of a global Record marks its assumption in
    /// `assumes`.
    pub(super) fn client_expr_reasons(
        &self,
        (owner, off): (usize, usize),
        kind: Option<&ir::ControlKind>,
        (name, value, control): (&str, &str, &str),
        (unsupported, big_integer, assumes): (&mut Vec<String>, &mut Vec<String>, &RefCell<Vec<Assumption>>),
    ) {
        match crate::pages::client_expression_of(kind, name, value) {
            Some(crate::pages::ClientExpr::Refused(why)) => {
                unsupported.push(format!("property {name} of control {control}: {why}"))
            }
            Some(crate::pages::ClientExpr::BigIntegerLiteral(why)) => {
                big_integer.push(format!("property {name} of control {control}: {why}"))
            }
            Some(crate::pages::ClientExpr::QualifiedThroughMember(why)) => {
                let synthetic = crate::pages::synthetic_name(name, off);
                let routine = self.routine_nodes.get(owner).iter().find(|r| r.name() == synthetic);
                if !routine.is_some_and(|r| self.qualified_values_pinned(owner, r)) {
                    unsupported.push(format!("property {name} of control {control}: {why}"));
                } else if self.ws.objects[owner].kind == ObjKind::PageExtension {
                    // Only a page's own controls are pinned (testpage/CcPageLeads.al:1051, :1118, :1185, :1386). A control an
                    // extension declares or changes, through the extension's own global, takes the same rule: an outside
                    // reporter's tests run it on real BC (al-runner issue 6924); the probe wins.
                    let mut a = assumes.borrow_mut();
                    if !a.contains(&Assumption::ControlExprMemberEnumInExtension) {
                        a.push(Assumption::ControlExprMemberEnumInExtension);
                    }
                }
            }
            None => {}
        }
    }

    /// Whether every qualified value through a member in synthetic routine `routine` of `owner` is the form BC compiles and
    /// runs: an Enum value through an Enum field of a global Record variable of `owner`, `E.Mode::Beta` and
    /// `E."Mode Two"::"Gamma Delta"` (testpage/CcPageLeads.al:1051 VisibleExpr_V1, :1118 V2, :1185 V3, :1386 V6). BC replaces the
    /// value by its constant before the client compile; the lowering does too (a `Res::EnumValue` lowers to a constant), so
    /// the record is never read. Not pinned, so refused: an Option field (`Res::OptionValue`), a namespace-qualified enum
    /// (`Ns."Kind"::Beta`, its base is no variable), a variable of the base page in an extension (`Res::BaseGlobal`), a
    /// local, and a member of anything but a name (`this.E.Mode::Beta`, `E[1].Mode::Beta`). A missing resolution refuses.
    fn qualified_values_pinned(&self, owner: usize, routine: &ast::Routine) -> bool {
        let file = self.ws.objects[owner].file;
        let res = |n: &SyntaxNode| self.checked.resolutions.get(&key(file, n)).map(Res::inner);
        routine
            .syntax()
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::ScopeExpr)
            .all(|scope| {
                let Some(qualifier) = scope.first_child() else {
                    return false;
                };
                if qualifier.kind() != SyntaxKind::MemberExpr {
                    // `"Enum"::Value`, `EnumVar::Value`: accepted by the text scan already (CcPageLeads.al:1252, :1319).
                    return true;
                }
                let Some(base) = qualifier.first_child() else {
                    return false;
                };
                if base.kind() != SyntaxKind::NameRef {
                    return false;
                }
                let text = base.text().to_string();
                if ["rec", "xrec", "this"].contains(&text.trim().trim_matches('"').to_ascii_lowercase().as_str()) {
                    // `Rec.Status::Open`: accepted by the text scan already.
                    return true;
                }
                matches!(res(&scope), Some(Res::EnumValue { .. }))
                    && matches!(res(&base), Some(Res::Global(_)))
                    && matches!(self.checked.types.get(&key(file, &base)), Some(Ty::Record { .. }))
            })
    }
}
