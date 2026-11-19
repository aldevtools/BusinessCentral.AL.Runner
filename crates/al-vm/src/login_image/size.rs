//! How much a [`LoginImage`] holds (design section 5): the rows it keeps, and an estimate of its state and graph.
//!
//! The row numbers are the shared baseline's own (`SharedBaseline::bytes`, `bytes_of`). The state number is an estimate
//! of memory: `size_of` of each node and of each value, plus the heap behind its strings, vectors and Blob buffers.
//! What this file cannot see is counted by its `size_of` only: a record node's own vectors (private to `record.rs`)
//! and the maps of the plain session fields. The graph's numbers (`nodes`, `instances`, `temp_tables`, `temp_rows`)
//! are exact.

use super::arena::{Frozen, Node};
use super::library::LibraryNode;
use super::temps::TempImage;
use super::{ImageSize, LoginImage};
use al_builtins::plain::Plain;
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
        let state = size_of::<super::fields::PlainFields>()
            + size_of::<super::session_parts::SessionParts>()
            + self.singletons.len() * size_of::<(usize, super::arena::NodeId)>()
            + self.bindings.len() * size_of::<super::arena::NodeId>()
            + nodes.iter().map(node_bytes).sum::<usize>();
        ImageSize {
            rows_bytes: self.rows.bytes(),
            rows_own_bytes: self.rows.bytes_of(&self.rows_own),
            rows: self.rows.rows(),
            state_bytes: state as u64,
            nodes: nodes.len(),
            instances: count(|n| matches!(n, Node::Instance(_))),
            temp_tables: count(|n| matches!(n, Node::TempTable(_))),
            temp_rows,
        }
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

fn frozen_bytes(v: &Frozen) -> usize {
    size_of::<Frozen>()
        + match v {
            Frozen::Plain(p) => plain_heap(p),
            Frozen::Node(_) => 0,
        }
}

fn all(v: &[Frozen]) -> usize {
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
            Node::Record(_) => size_of::<super::record::RecordNode>(),
            Node::Globals { instances, .. } => instances.len() * size_of::<(u32, super::arena::NodeId)>(),
            Node::Library(_, l) => library_heap(l),
            Node::Blob(b) => b.bytes.len(),
            Node::TempTable(t) => temp_bytes(t),
            Node::Query(_) => size_of::<super::arena::QueryNode>(),
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
