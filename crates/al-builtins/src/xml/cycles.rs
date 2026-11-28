//! Frees the XML nodes that only the cycle between a node and its parent keeps alive (al-runner issue 8560).
//!
//! A node holds its parent strongly, as BC does: AL that keeps only a node of a document still reaches the document
//! (`GetDocument` answers Yes and writes it, xml/XmlElement.al:349 Navigation_Parent_Document, xml/XmlNodes.al:363
//! Node_Navigation), and a root element whose document variable went out of scope still has its document as parent
//! (xml/XmlElement.al:521 AddAfterSelf_Root_Error, :601 ReplaceWith_Root_Unparented). The parent holds its children
//! strongly too, so a node with a parent is in a reference cycle that `Rc` never frees; BC's .NET garbage collector does.
//!
//! [`note`] registers every node that gets a parent, on the thread that made it (`Rc` never leaves that thread). When
//! the registry reaches twice the entries left by the last sweep (at least [`SWEEP_FLOOR`]), [`note`] sweeps: for each
//! registered node that is alive, and its parent, it counts the strong references the other swept nodes hold to it
//! (children, attributes, the declaration and the parent link, the only handles an [`XData`] holds). A node with more
//! references than those is held from outside (a variable, a list, a kept value), and so is every node it reaches;
//! the rest is garbage, and its parent link is cleared, which ends the cycle. Nothing here is observable to AL: a node
//! this module frees is one no variable can reach.

use super::dom::{XData, XmlCell};
use super::*;
use std::cell::Cell;
use std::collections::HashMap;

/// The registry length below which [`note`] never sweeps.
const SWEEP_FLOOR: usize = 1024;

thread_local! {
    /// The nodes that got a parent on this thread and were alive at the last sweep or got one since.
    static NOTED: RefCell<Vec<Weak<RefCell<LibraryValue>>>> = const { RefCell::new(Vec::new()) };
    /// The registry length at which [`note`] next sweeps.
    static SWEEP_AT: Cell<usize> = const { Cell::new(SWEEP_FLOOR) };
}

/// Registers `child`, which just got a parent, and sweeps when the registry has grown enough.
pub(super) fn note(child: &Handle) {
    let len = NOTED.with(|n| {
        let mut n = n.borrow_mut();
        n.push(Rc::downgrade(child));
        n.len()
    });
    if len >= SWEEP_AT.with(Cell::get) {
        sweep();
    }
}

/// The handles `cell` holds strongly: its children, attributes and declaration, and its parent.
fn edges(cell: &XmlCell, out: &mut Vec<Handle>) {
    match &cell.data {
        XData::Document { decl, children } => out.extend(decl.iter().chain(children).cloned()),
        XData::Element { attrs, children, .. } => out.extend(attrs.iter().chain(children).cloned()),
        _ => {}
    }
    out.extend(cell.parent.iter().cloned());
}

/// Clears the parent link of every registered node (and parent) no outside reference reaches. A node some caller
/// holds borrowed cannot be read, so the sweep then frees nothing and waits for the next threshold.
pub(super) fn sweep() {
    let noted: Vec<Handle> = NOTED.with(|n| n.borrow().iter().filter_map(Weak::upgrade).collect());
    let mut nodes: Vec<Handle> = Vec::new();
    let mut index: HashMap<*const RefCell<LibraryValue>, usize> = HashMap::new();
    let mut add = |h: &Handle, nodes: &mut Vec<Handle>| {
        index.entry(Rc::as_ptr(h)).or_insert_with(|| {
            nodes.push(h.clone());
            nodes.len() - 1
        });
    };
    for h in &noted {
        add(h, &mut nodes);
    }
    drop(noted);
    // The parents of noted nodes: a document or a root element never gets a parent itself.
    let mut i = 0;
    while i < nodes.len() {
        let parent = match nodes[i].try_borrow() {
            Ok(b) => match &*b {
                LibraryValue::Xml(c) => c.parent.clone(),
                _ => None,
            },
            Err(_) => return finish(),
        };
        if let Some(p) = parent {
            add(&p, &mut nodes);
        }
        i += 1;
    }
    let mut out: Vec<Vec<usize>> = Vec::with_capacity(nodes.len());
    let mut inner = vec![0usize; nodes.len()];
    let mut held = Vec::new();
    for h in &nodes {
        held.clear();
        match h.try_borrow() {
            Ok(b) => {
                if let LibraryValue::Xml(c) = &*b {
                    edges(c, &mut held);
                }
            }
            Err(_) => return finish(),
        }
        let targets: Vec<usize> = held.iter().filter_map(|t| index.get(&Rc::as_ptr(t)).copied()).collect();
        for &t in &targets {
            inner[t] += 1;
        }
        out.push(targets);
    }
    drop(held);
    // `nodes` holds one reference to each; `held` is gone.
    let mut reached: Vec<bool> = nodes
        .iter()
        .zip(&inner)
        .map(|(h, &n)| Rc::strong_count(h) > n + 1)
        .collect();
    let mut todo: Vec<usize> = (0..nodes.len()).filter(|&i| reached[i]).collect();
    while let Some(i) = todo.pop() {
        for &t in &out[i] {
            if !reached[t] {
                reached[t] = true;
                todo.push(t);
            }
        }
    }
    for (h, _) in nodes.iter().zip(&reached).filter(|(_, r)| !**r) {
        if let LibraryValue::Xml(c) = &mut *h.borrow_mut() {
            c.parent = None;
        }
    }
    drop(nodes);
    finish();
}

/// Drops the entries of freed nodes and sets the next threshold.
fn finish() {
    let left = NOTED.with(|n| {
        let mut n = n.borrow_mut();
        n.retain(|w| w.strong_count() > 0);
        n.len()
    });
    SWEEP_AT.with(|s| s.set((2 * left).max(SWEEP_FLOOR)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::dom::{new_node, set_parent};

    fn element(name: &str) -> Handle {
        new_node(XData::Element {
            prefix: String::new(),
            local: name.into(),
            ns: String::new(),
            attrs: Vec::new(),
            children: Vec::new(),
            open: false,
            filled: false,
        })
    }

    fn attach(parent: &Handle, child: &Handle) {
        if let LibraryValue::Xml(c) = &mut *parent.borrow_mut() {
            if let XData::Element { children, .. } = &mut c.data {
                children.push(child.clone());
            }
        }
        set_parent(child, Some(parent));
    }

    #[test]
    fn a_held_child_keeps_its_parent_and_a_dropped_pair_is_freed() {
        let root = element("a");
        let child = element("b");
        attach(&root, &child);
        let weak_root = Rc::downgrade(&root);
        drop(root);
        sweep();
        assert!(
            weak_root.upgrade().is_some(),
            "the child a variable holds keeps its parent"
        );
        let weak_child = Rc::downgrade(&child);
        drop(child);
        sweep();
        assert!(weak_root.upgrade().is_none(), "nothing outside holds the pair");
        assert!(weak_child.upgrade().is_none());
    }

    #[test]
    fn a_borrowed_node_stops_the_sweep_and_frees_nothing() {
        let root = element("a");
        let child = element("b");
        attach(&root, &child);
        let weak_root = Rc::downgrade(&root);
        let weak_child = Rc::downgrade(&child);
        let keep = child.clone();
        drop((root, child));
        {
            let _b = keep.borrow_mut();
            sweep();
        }
        assert!(weak_root.upgrade().is_some());
        drop(keep);
        sweep();
        assert!(weak_child.upgrade().is_none() && weak_root.upgrade().is_none());
    }
}
