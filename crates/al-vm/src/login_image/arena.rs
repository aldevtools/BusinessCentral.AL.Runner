//! The SingleInstance object graph of a session as an arena with no `Rc` (design section 2): every handle a global
//! reaches (an instance, an array, a List, a Dictionary, a record, a table globals set, a library cell, a Blob buffer, a
//! temporary table, a fetched row, a query) is one node, numbered by first sight; a value is plain or a node number. So
//! two places that hold one handle hold one node, a cycle ends at a node already numbered, and the arena is `Send + Sync`
//! by its types.

use al_builtins::plain::{Plain, PlainFilterExpr};

/// The number of a node: its index in [`Arena::nodes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NodeId(pub(crate) u32);

/// A value of the graph: plain, or the number of the node that is its handle.
#[derive(Debug, Clone)]
pub(crate) enum Frozen {
    Plain(Plain),
    Node(NodeId),
}

/// The graph: every handle once, in the order the walk first saw it.
#[derive(Debug, Default)]
pub(crate) struct Arena {
    pub(crate) nodes: Vec<Node>,
}

impl Arena {
    #[cfg(test)]
    pub(crate) fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0 as usize]
    }
}

/// One handle and what it holds.
#[derive(Debug)]
pub(crate) enum Node {
    /// `Rc<RefCell<Instance>>`.
    Instance(InstanceNode),
    /// `Value::Array`.
    Array(Vec<Frozen>),
    /// `Value::List` (`ListVec`: its items, its change count, whether it is read-only).
    List {
        items: Vec<Frozen>,
        version: u64,
        read_only: bool,
    },
    /// `Value::Dictionary`: the entries in the order they are stored, removed ones included.
    Dictionary(Vec<(Frozen, Frozen)>),
    /// `Rc<RefCell<RecordState>>`.
    Record(Box<super::record::RecordNode>),
    /// A shared `TableGlobals` set: the instances of the table's globals, and its xRec set when one was made.
    Globals {
        instances: Vec<(u32, NodeId)>,
        xrec: Option<NodeId>,
    },
    /// `Rc<RefCell<LibraryValue>>`.
    Library(al_builtins::library_kind::LibraryKind, super::library::LibraryNode),
    /// `Arc<BlobCell>`: a deep copy of the buffer.
    Blob(al_builtins::stream::BlobBuf),
    /// A temporary table, by its id (`TempId`): every record that shares it names this node.
    TempTable(super::temps::TempImage),
    /// The row a record's load buffer fetched (`FetchedRow`'s `Rc`).
    Row(Vec<Frozen>),
    /// `Rc<RefCell<QueryState>>` without a result set.
    Query(Box<QueryNode>),
    /// `Rc<DotNetObject>` of a kind the runtime serves itself (an enum value, an internal object such as
    /// `ALConfigSettings.Instance`): the ones the check renders exactly. An object of the .NET process is never imaged.
    DotNet(DotNetNode),
    /// Seen, not walked yet (only while the walk runs).
    Pending,
}

/// A codeunit instance (al-builtins `Instance`), every field but `live` (a new count goes with the new instance) and
/// `unmeasured` (an image requires `None`).
#[derive(Debug, Clone)]
pub(crate) struct InstanceNode {
    pub(crate) object: u32,
    pub(crate) globals: Vec<Frozen>,
    pub(crate) initialized: bool,
    pub(crate) rerun: bool,
    /// The instance it extends, while that lives (a `Weak`).
    pub(crate) base: Option<NodeId>,
    pub(crate) exts: Vec<(u32, NodeId)>,
    pub(crate) xp_text: Vec<(u8, String)>,
    /// `Born`, written explicitly at thaw: its default reads a thread-local.
    pub(crate) born: bool,
    pub(crate) single: bool,
}

/// A .NET object the runtime serves itself (al-builtins `DotNetKind::Enum`, `DotNetKind::Internal`).
#[derive(Debug)]
pub(crate) enum DotNetNode {
    Enum {
        clr_type: String,
        name: String,
        ordinal: i64,
    },
    Internal {
        type_name: String,
        state: Vec<Frozen>,
    },
}

/// A query variable that has not run (al-builtins `QueryState` with `run: None`).
#[derive(Debug)]
pub(crate) struct QueryNode {
    pub(crate) object: u32,
    pub(crate) filters: Vec<(usize, PlainFilterExpr)>,
    pub(crate) filter_replaced: bool,
    pub(crate) top: Option<i32>,
    pub(crate) security_filtering: Option<i32>,
    pub(crate) instance: Option<NodeId>,
    pub(crate) in_before_open: bool,
    pub(crate) closed: bool,
    pub(crate) reopen: al_builtins::query_reopen_step::ReopenStep,
}

// The arena is part of the image any thread may keep.
const _: fn() = || {
    fn both<T: Send + Sync>() {}
    both::<Arena>();
};
