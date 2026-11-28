//! The store cell of `ClearAll()` in a codeunit with a TableNo over a register tree that another variable shares and that
//! no call has made yet ([`super::Store::UncalledTree`], assumption clear-all-uncalled-register-tree, open corpus PR 1650,
//! tests/al-language/codeunits/clear-instance/ClearSharedCodeunitNeverCalledTree.al): the shape of "Sales-Post" clearing
//! the "Whse. Jnl.-Register Line" that "Whse.-Activity-Post" gave it through a setter (al-runner issue 8964). Which
//! codeunits form the tree is [`Session::uncalled_tree_node`]'s; the cell itself is [`super::measured`]'s. Entry point:
//! `Session::uncalled_tree_pair` (from `Session::store_kind_in`).

use super::*;

impl Session {
    /// Whether `ClearAll()` in `owner` of its global of codeunit `store` is the pair corpus PR 1650 measures
    /// (ClearSharedCodeunitNeverCalledTree.al): the owner a codeunit with a TableNo ([`Owner::TableNo`], "CNC Owner"
    /// :287-293) and the store the register of an [`Session::uncalled_tree_node`] tree ("CNC Reg" :215-229).
    pub(super) fn uncalled_tree_pair(&self, owner: usize, store: usize) -> bool {
        self.owner_kind(owner) == Some(Owner::TableNo) && self.uncalled_tree_node(store, 0)
    }

    /// Whether codeunit `object`, `depth` levels below the register (0: the register), is a node of the tree corpus PR 1650
    /// measures, by its role ([`Node`]) and the kinds of its globals ([`node_kind`]). The measured tree: the register
    /// (TableNo, not SingleInstance) "CNC Reg" holds the plain "CNC A" and "CNC B" one level below it (:224-225); "CNC B"
    /// holds "CNC A" and the plain "CNC W" two levels below it (:174-175); "CNC A" holds the SingleInstance "CNC C" and
    /// "CNC U" (:124-125), two or three levels below; "CNC C" holds the SingleInstance "CNC I" (:73), three or four levels
    /// below. No codeunit of the tree states a Subtype.
    pub(super) fn uncalled_tree_node(&self, object: usize, depth: u8) -> bool {
        let o = &self.engine.program.objects[object];
        let role = match (
            depth,
            self.engine.object_facts[object].single_instance,
            o.rec_slot.is_some(),
        ) {
            (0, false, true) => Node::Register,
            (1 | 2, false, false) => Node::Plain,
            (2..=4, true, false) => Node::SingleInstance,
            _ => return false,
        };
        o.kind == al_ir::ObjKind::Codeunit
            && self.declared_subtype(object).is_none()
            && o.globals.iter().enumerate().all(|(i, g)| {
                o.rec_slot == Some(i)
                    || match g.ty {
                        RtType::Codeunit(c) if !g.label => self.uncalled_tree_node(c as usize, depth + 1),
                        _ => node_kind(role, g),
                    }
            })
    }
}

/// The role of a codeunit in the tree of corpus PR 1650 ([`Session::uncalled_tree_node`]).
#[derive(Clone, Copy)]
enum Node {
    /// "CNC Reg" (:215-229): the register, with a TableNo, not SingleInstance.
    Register,
    /// "CNC A" (:116-133), "CNC B" (:165-181) and "CNC W" (:104-108): not SingleInstance, no TableNo.
    Plain,
    /// "CNC U" (:81-89), "CNC C" (:68-73) and "CNC I" (:50-55): SingleInstance, no TableNo.
    SingleInstance,
}

/// Whether global `g`, which is not a codeunit, is of a kind a node of role `role` declares in corpus PR 1650: records
/// that are not temporary, Integer, Boolean, `Label 'text', Comment = 'text'` and `Label 'text'` in the register
/// (:220-229); also temporary records and Date in a plain node (:119-133, :168-181, :107-108); records that are not
/// temporary, Boolean and the two Labels in a SingleInstance node (:86-89, :55).
fn node_kind(role: Node, g: &al_ir::Slot) -> bool {
    if g.label {
        return g.comment_label || g.plain_label;
    }
    match role {
        Node::Register => matches!(g.ty, RtType::Record(_, false) | RtType::Integer | RtType::Boolean),
        Node::Plain => matches!(
            g.ty,
            RtType::Record(..) | RtType::Integer | RtType::Date | RtType::Boolean
        ),
        Node::SingleInstance => matches!(g.ty, RtType::Record(_, false) | RtType::Boolean),
    }
}
