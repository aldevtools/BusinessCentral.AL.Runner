//! How much a [`LoginImage`] holds: the rows it keeps, and an estimate of its state and graph.
//!
//! The row number is the rows' own (`LoginRows::size`, the encoded rows the login wrote). The state number is an
//! estimate of memory: `size_of` of each node and of each value, plus the heap behind its strings, vectors, Blob
//! buffers and a record node's vectors (`RecordNode::heap_bytes`, their owner). The maps of the plain session fields
//! count by their `size_of` only. The graph's numbers (`nodes`, `instances`, `temp_tables`, `temp_rows`, the counts per
//! kind) are exact. `kept_bytes` is what the starts' memory caps count: the rows, the state and the base shape.

use super::arena::{Frozen, Node};
use super::library::LibraryNode;
use super::plain::Plain;
use super::temps::TempImage;
use super::{ImageSize, LoginImage};
use std::mem::size_of;

impl LoginImage {
    /// The image's size: its rows, its state and its graph ([`ImageSize`]).
    pub fn size(&self) -> ImageSize {
        let nodes = &self.arena.nodes;
        let count = |f: fn(&Node) -> bool| nodes.iter().filter(|n| f(n)).count();
        let temp_rows = nodes
            .iter()
            .map(|n| match n {
                Node::TempTable(t) => t.row_count(),
                _ => 0,
            })
            .sum();
        let mut kinds = [crate::KindSize::default(); super::NODE_KINDS.len()];
        for n in nodes {
            if let Some(k) = kind(n) {
                kinds[k].count += 1;
                kinds[k].bytes += node_bytes(n) as u64;
            }
        }
        ImageSize {
            rows_bytes: self.rows.size() as u64,
            state_bytes: self.state_bytes() as u64,
            nodes: nodes.len(),
            instances: count(|n| matches!(n, Node::Instance(_))),
            temp_tables: count(|n| matches!(n, Node::TempTable(_))),
            temp_rows,
            kept_bytes: self.kept_bytes(),
            kinds,
        }
    }

    /// What keeping this image costs in memory, the number the starts' caps count: its rows, its state
    /// and its base shape.
    pub fn kept_bytes(&self) -> u64 {
        (self.rows.size() + self.state_bytes() + self.base_shape.bytes()) as u64
    }

    /// The plain fields, the nested states, the graph's roots and every node with the heap behind it.
    fn state_bytes(&self) -> usize {
        size_of::<super::fields::PlainFields>()
            + size_of::<super::session_parts::SessionParts>()
            + self.singletons.len() * size_of::<(usize, super::arena::NodeId)>()
            + self.bindings.len() * size_of::<super::arena::NodeId>()
            + self.temp_blob_fields.len() * size_of::<(super::arena::NodeId, usize)>()
            + self.arena.nodes.iter().map(node_bytes).sum::<usize>()
    }

    /// The shape of the store the login started from (`Session::base_shape`).
    pub fn base_shape(&self) -> &super::BaseShape {
        &self.base_shape
    }
}

/// The heap behind a plain value.
fn plain_heap(p: &Plain) -> usize {
    match p {
        Plain::Text(t) | Plain::Code(t) | Plain::TextConst(t, _) => t.as_str().len(),
        Plain::DateFormula(s) => s.len(),
        Plain::Option(_, names) => names.iter().map(|n| size_of::<String>() + n.len()).sum(),
        Plain::Unknown(s) => s.len(),
        Plain::Null
        | Plain::Boolean(_)
        | Plain::Integer(_)
        | Plain::BigInteger(_)
        | Plain::Decimal(_)
        | Plain::Byte(_)
        | Plain::Char(_)
        | Plain::Date(..)
        | Plain::Time(_)
        | Plain::DateTime(_)
        | Plain::Duration(_)
        | Plain::Guid(_)
        | Plain::Enum(..) => 0,
    }
}

/// The index of `n`'s kind in [`super::NODE_KINDS`]; `None` for a node the walk had not finished (none after it).
fn kind(n: &Node) -> Option<usize> {
    Some(match n {
        Node::Instance(_) => 0,
        Node::Array(_) => 1,
        Node::List { .. } => 2,
        Node::Dictionary(_) => 3,
        Node::Record(_) => 4,
        Node::Globals { .. } => 5,
        Node::Library(..) => 6,
        Node::Blob(_) => 7,
        Node::TempTable(_) => 8,
        Node::Row(_) => 9,
        Node::Query(_) => 10,
        Node::DotNet(_) => 11,
        Node::Pending => return None,
    })
}

pub(super) fn frozen_bytes(v: &Frozen) -> usize {
    size_of::<Frozen>()
        + match v {
            Frozen::Plain(p) => plain_heap(p),
            Frozen::Node(_) => 0,
        }
}

pub(super) fn all(v: &[Frozen]) -> usize {
    v.iter().map(frozen_bytes).sum()
}

fn temp_bytes(t: &TempImage) -> usize {
    t.tables
        .iter()
        .map(|t| t.company.len() + t.rows.iter().map(|(key, row)| all(key) + all(row)).sum::<usize>())
        .sum()
}

/// The node itself and the heap behind it.
fn node_bytes(n: &Node) -> usize {
    size_of::<Node>()
        + match n {
            Node::Instance(i) => {
                all(&i.globals)
                    + i.exts.len() * size_of::<(u32, super::arena::NodeId)>()
                    + i.xp_text
                        .iter()
                        .map(|(_, s)| size_of::<(u8, String)>() + s.len())
                        .sum::<usize>()
            }
            Node::Array(v) | Node::Row(v) => all(v),
            Node::List { items, .. } => all(items),
            Node::Dictionary(entries) => entries.iter().map(|(k, v)| frozen_bytes(k) + frozen_bytes(v)).sum(),
            Node::Record(r) => size_of::<super::record::RecordNode>() + r.heap_bytes(),
            Node::Globals { instances, .. } => instances.len() * size_of::<(u32, super::arena::NodeId)>(),
            Node::Library(_, l) => library_heap(l),
            Node::Blob(b) => b.bytes.len(),
            Node::TempTable(t) => temp_bytes(t),
            Node::Query(_) => size_of::<super::arena::QueryNode>(),
            Node::DotNet(d) => match d {
                super::arena::DotNetNode::Internal { type_name, state } => type_name.len() + all(state),
            },
            Node::Pending => 0,
        }
}

fn library_heap(l: &LibraryNode) -> usize {
    match l {
        LibraryNode::Text(s) | LibraryNode::TextCap(s, _) => s.len(),
        LibraryNode::RecordId { name, key, .. } => name.len() + all(key),
        LibraryNode::UserNameFieldRef { value, .. } => value.as_ref().map_or(0, String::len),
        LibraryNode::Empty
        | LibraryNode::ClearedStream
        | LibraryNode::XmlNodeOfUnassignedElement
        | LibraryNode::ClosedRecordRef
        | LibraryNode::HandedContext
        | LibraryNode::XmlNameTable
        | LibraryNode::SystemTable(_)
        | LibraryNode::Version(_)
        | LibraryNode::RecordRef(_)
        | LibraryNode::FieldRef(..)
        | LibraryNode::KeyFieldRef(..)
        | LibraryNode::KeyRef(..)
        | LibraryNode::ErrorInfo(_)
        | LibraryNode::Owned(_) => 0,
    }
}
