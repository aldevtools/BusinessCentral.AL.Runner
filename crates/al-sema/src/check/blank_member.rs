//! `O::""` and `O::" "` on an Option or an Enum: AL0132 where no member has that name.
use super::*;

impl<'a> Checker<'a> {
    /// `scope_member` for `left::rhs`, after the blank member names (`O::""`) were judged.
    pub(crate) fn scope_member_named(
        &mut self,
        sc: &mut Scope,
        node: &SyntaxNode,
        left: &SyntaxNode,
        t: &Ty,
        rhs: &str,
    ) {
        if !self.al_blank_member_name(sc, node, left, t, rhs) {
            self.scope_member(sc, node, t, rhs);
        }
    }

    /// AL0132 (corpus must-not-compile 3126-3131): a blank member name after `::`. The nameless member (`""` or an unquoted empty
    /// member of an Option, a value named `""` of an Enum) has no name to write, so `O::""` is never defined; `O::" "` names the
    /// member `" "` only where one exists. The message names the left side as written (`O`, `Opt` for `Rec.Opt`, `E`). Reported for
    /// the workspace only. Returns whether it reported, so the caller records no member.
    pub(crate) fn al_blank_member_name(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        left: &SyntaxNode,
        t: &Ty,
        rhs: &str,
    ) -> bool {
        if !rhs.is_empty() && !rhs.trim().is_empty() || self.ws.is_dependency(sc.oi) {
            return false;
        }
        let named = match t {
            Ty::Option(members) => !rhs.is_empty() && members.iter().any(|m| m == rhs),
            Ty::Enum(name) => match self.resolve(sc, ObjKind::Enum, name) {
                Some(ei) if !self.ws.is_dependency(ei) => {
                    !rhs.is_empty() && self.members[ei].enum_values.iter().any(|(_, n)| n == rhs)
                }
                _ => return false,
            },
            _ => return false,
        };
        if named {
            return false;
        }
        // `Rec.Opt::""` names `Opt`: the member name node is the last child of the member access.
        let named_node = if left.kind() == SyntaxKind::MemberExpr {
            left.children().last().unwrap_or_else(|| left.clone())
        } else {
            left.clone()
        };
        let shown = name_of(&named_node);
        let shown = crate::alcodes::quoted_if_needed(&shown);
        self.diag(
            sc,
            node,
            "AL0132",
            format!("'{shown}' does not contain a definition for '{rhs}'"),
        );
        true
    }
}
