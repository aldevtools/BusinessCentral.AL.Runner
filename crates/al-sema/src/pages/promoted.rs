//! Promoted parts of a page that a page extension may anchor on although the page does not declare them (al-runner issues
//! 6646 and 6647). alc compiles these forms (corpus writer Y64's compile facts, alc 30 with BC 29 symbols): every page has
//! the groups `Category_Category4` to `Category_Category20` (1 to 3 and 21 on are AL0271), and an action promoted the legacy
//! way (`Promoted = true`) has the actionref `<action>_Promoted` (an action that is not promoted has none, AL0271).

use super::{Op, Tree};
use al_syntax::ast;

/// Whether `target` is one of the groups `Category_Category4` to `Category_Category20` every page has.
pub(super) fn numbered_category(target: &str) -> bool {
    let lower = target.to_ascii_lowercase();
    lower
        .strip_prefix("category_category")
        .and_then(|n| n.parse::<u32>().ok())
        .is_some_and(|n| (4..=20).contains(&n))
}

/// Whether a `modify` of an implicit group sets only what AL code cannot see: the group is an anchor and no control of the
/// page, so its Caption and ShowAs reach no TestPage. Visible is left out: whether it hides the actionrefs in the group on
/// real BC is not pinned.
pub(super) fn inert_group_modify(section: &ast::Section) -> bool {
    section.sections().next().is_none()
        && section.triggers().next().is_none()
        && section
            .properties()
            .all(|p| ["caption", "captionml", "showas"].contains(&p.name().to_ascii_lowercase().as_str()))
}

impl Tree {
    /// Whether `target` is the implicit actionref `<action>_Promoted` of an action of the page promoted the legacy way, which
    /// the page does not declare itself.
    pub(super) fn legacy_promoted_ref(&self, target: &str) -> bool {
        let Some(cut) = target.len().checked_sub("_Promoted".len()) else {
            return false;
        };
        if !target.is_char_boundary(cut) || !target[cut..].eq_ignore_ascii_case("_Promoted") {
            return false;
        }
        let action = &target[..cut];
        self.find_node(target).is_none()
            && self.nodes.iter().any(|n| {
                n.kind == al_ir::ControlKind::Action
                    && n.name.eq_ignore_ascii_case(action)
                    && n.section.properties().any(|p| {
                        p.name().eq_ignore_ascii_case("Promoted") && p.value_text().trim().eq_ignore_ascii_case("true")
                    })
            })
    }

    /// Why a `modify` of an implicit group or actionref (an anchor this file allows) is refused: one that sets Visible or
    /// another property AL code may see. A group some action of the page is promoted to the legacy way is left as it was.
    pub(super) fn unpinned_anchor_modify(&self, op: &Op) -> Option<String> {
        let target = op.section.args().first().cloned().unwrap_or_default();
        if !op.in_actions || !op.section.name().eq_ignore_ascii_case("modify") {
            return None;
        }
        let n = self.find_node(&target)?;
        let legacy =
            target.get(..9).is_some_and(|p| p.eq_ignore_ascii_case("Category_")) && self.promotes_to(&target[9..]);
        let category = numbered_category(&target) && !inert_group_modify(&op.section) && !legacy;
        let promoted_ref = target.to_ascii_lowercase().ends_with("_promoted");
        if !self.nodes[n].anchor_only || !(category || promoted_ref) {
            return None;
        }
        Some(format!(
            "page extension: modify({target}): `{target}` is not declared on the page; changing it this way is not pinned"
        ))
    }
}
