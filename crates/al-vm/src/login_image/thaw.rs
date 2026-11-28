//! The thaw of an [`Arena`] into a session, on the session's own thread. Every node gets a new handle
//! first (an instance through `instance_cycles::shared`, so the thread's registry notes it; a temporary table a new id of
//! the session and its store), then each is filled with handles of the same thaw: so cycles need no special case, and
//! no thawn value is shared with the image or another thaw (except `Txt` and `Arc<[String]>`, which are immutable).
//!
//! The order: shells (and Blobs, which name nothing); xRec sets from the set they belong to; temporary tables' stores and
//! fetched rows (they name only Blobs); records (they name tables, rows, Blobs and sets); the temporary tables' empty-Blob
//! Insert notes (they name records); then the content of every shell.
//!
//! The thaw is made in two steps: [`make`] builds every handle from the image and changes nothing of the session but its
//! temporary table id counter (`own`, never observable: `session_fields.rs`); [`Thawn::install`] then gives them to the
//! session. Every lookup of a node checks its kind and that it is made already: an image whose own data is broken (a node
//! named where another kind is, a number with no node, an xRec chain that is a cycle or a set that is the xRec set of two
//! sets) makes [`make`] answer `Err`, before anything of the session changed (issue 9016).

use super::arena::{Arena, Frozen, Node, NodeId};
use crate::Session;
use al_builtins::value::{Instance, LibraryValue, ListVec, QueryState, RecordState, TableGlobals, TempId, Value};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// A node's new handle.
enum Made {
    Instance(Rc<RefCell<Instance>>),
    Array(Rc<RefCell<Vec<Value>>>),
    List(Rc<RefCell<ListVec>>),
    Dictionary(Rc<RefCell<Vec<(Value, Value)>>>),
    Record(Rc<RefCell<RecordState>>),
    Globals(TableGlobals),
    Library(al_builtins::library_kind::LibraryKind, Rc<RefCell<LibraryValue>>),
    Blob(al_builtins::stream::BlobRef),
    TempTable(TempId),
    Row(Rc<Vec<Value>>),
    Query(Rc<RefCell<QueryState>>),
    DotNet(Rc<al_builtins::value::DotNetObject>),
}

/// The handles of one thaw, by node.
pub(crate) struct Maker<'a> {
    arena: &'a Arena,
    made: Vec<Option<Made>>,
    /// The thawn database store's change count, and each thawn temporary table's (by its new id).
    pub(crate) store_counter: u64,
    temp_counters: HashMap<u64, u64>,
    /// The first broken part of the image a lookup found ([`Maker::broke`]); `None` while the image reads whole.
    broken: Cell<Option<&'static str>>,
    /// The thawn temporary tables' stores, by their new id, put in the session by [`Thawn::install`].
    temps: Vec<(u64, al_data::memory::MemStore)>,
    /// The collected errors' call stacks, by their thawn cell (al-builtins errorinfo.rs keys them by the cell's address).
    callstacks: Vec<(Value, String)>,
}

/// What [`make`] built from an image, for [`Thawn::install`].
pub(crate) struct Thawn {
    temps: Vec<(u64, al_data::memory::MemStore)>,
    callstacks: Vec<(Value, String)>,
    singletons: Vec<(usize, Rc<RefCell<Instance>>)>,
    bindings: Vec<Rc<RefCell<Instance>>>,
    temp_blob_fields: Vec<(Rc<RefCell<RecordState>>, usize)>,
}

impl Maker<'_> {
    /// Notes that the image's own data is broken (the first note is kept): [`make`] then answers `Err`.
    pub(crate) fn broke(&self, what: &'static str) {
        if self.broken.get().is_none() {
            self.broken.set(Some(what));
        }
    }

    /// The handle made for node `id`; `None`, noted as broken, when there is no such node or it is not made yet.
    fn made(&self, id: NodeId) -> Option<&Made> {
        let made = self.made.get(id.0 as usize).and_then(Option::as_ref);
        if made.is_none() {
            self.broke("a node named that is not there or not made yet");
        }
        made
    }

    /// The value of `f`; `Null`, noted as broken, where the node is not one a value holds.
    pub(crate) fn value(&self, f: &Frozen) -> Value {
        match f {
            Frozen::Plain(p) => p.value(),
            Frozen::Node(id) => match self.made(*id) {
                Some(Made::Instance(r)) => Value::Codeunit(r.clone()),
                Some(Made::Array(r)) => Value::Array(r.clone()),
                Some(Made::List(r)) => Value::List(r.clone()),
                Some(Made::Dictionary(r)) => Value::Dictionary(r.clone()),
                Some(Made::Record(r)) => Value::Record(r.clone()),
                Some(Made::Library(kind, r)) => Value::Library(kind.clone(), r.clone()),
                Some(Made::Blob(b)) => Value::Blob(b.clone()),
                Some(Made::Query(q)) => Value::Query(q.clone()),
                Some(Made::DotNet(d)) => Value::DotNet(d.clone()),
                Some(Made::Globals(_) | Made::TempTable(_) | Made::Row(_)) | None => {
                    self.broke("a value names a table globals set, a temporary table, a row or no node");
                    Value::Null
                }
            },
        }
    }

    pub(crate) fn values(&self, f: &[Frozen]) -> Vec<Value> {
        f.iter().map(|v| self.value(v)).collect()
    }

    // Each lookup below answers a stand-in of its kind, noted as broken, where node `id` is not of that kind: the thaw
    // goes on to its end and [`make`] drops what it built.

    fn instance(&self, id: NodeId) -> Rc<RefCell<Instance>> {
        match self.made(id) {
            Some(Made::Instance(r)) => r.clone(),
            _ => {
                self.broke("an instance names another kind of node");
                Rc::default()
            }
        }
    }

    /// `None`, noted as broken, where node `id` is not a record (a record has no stand-in).
    pub(crate) fn record_handle(&self, id: NodeId) -> Option<Rc<RefCell<RecordState>>> {
        match self.made(id) {
            Some(Made::Record(r)) => Some(r.clone()),
            _ => {
                self.broke("a record names another kind of node");
                None
            }
        }
    }

    pub(crate) fn globals(&self, id: NodeId) -> TableGlobals {
        match self.made(id) {
            Some(Made::Globals(g)) => g.share(),
            _ => {
                self.broke("a table globals set names another kind of node");
                TableGlobals::default()
            }
        }
    }

    /// `None`, noted as broken, where node `id` is not a temporary table (a token has no stand-in).
    pub(crate) fn temp_id(&self, id: NodeId) -> Option<TempId> {
        match self.made(id) {
            Some(Made::TempTable(t)) => Some(t.clone()),
            _ => {
                self.broke("a temporary table names another kind of node");
                None
            }
        }
    }

    pub(crate) fn row(&self, id: NodeId) -> Rc<Vec<Value>> {
        match self.made(id) {
            Some(Made::Row(r)) => r.clone(),
            _ => {
                self.broke("a fetched row names another kind of node");
                Rc::default()
            }
        }
    }

    /// The change count of the thawn temporary table `id`.
    pub(crate) fn session_temp_counter(&self, id: u64) -> u64 {
        self.temp_counters.get(&id).copied().unwrap_or(0)
    }
}

/// The graph of `image` as new handles: its temporary tables, `singletons` (object index, node), the manual bindings
/// (instance nodes in order) and the live Temp Blob records. `store_counter`: the change count of the store the session
/// holds once the login's rows are put on it. `Err`: the image's own data is broken (what is broken); nothing of
/// `session` changed then but its temporary table id counter.
pub(crate) fn make(
    session: &mut Session,
    image: &super::LoginImage,
    store_counter: u64,
) -> Result<Thawn, &'static str> {
    let (arena, singletons, bindings) = (&image.arena, &image.singletons[..], &image.bindings[..]);
    let mut m = Maker {
        arena,
        made: arena.nodes.iter().map(|_| None).collect(),
        store_counter,
        temp_counters: HashMap::new(),
        broken: Cell::new(None),
        temps: Vec::new(),
        callstacks: Vec::new(),
    };
    make_shells(&mut m, session);
    make_xrecs(&mut m);
    make_tables_and_rows(&mut m);
    for (i, n) in arena.nodes.iter().enumerate() {
        if let Node::Record(r) = n {
            m.made[i] = Some(Made::Record(Rc::new(RefCell::new(m.record(r)))));
        }
    }
    for (i, n) in arena.nodes.iter().enumerate() {
        let table = NodeId(i as u32);
        if let Node::TempTable(t) = n
            && let Some(token) = m.temp_id(table)
        {
            let holder = (arena.nodes.iter().enumerate())
                .find(|(_, n)| matches!(n, Node::Record(r) if r.temp() == Some(table)))
                .map(|(r, _)| NodeId(r as u32));
            m.note_empty_blob_insert(t, &token, holder);
            m.note_copy_targets(t, &token);
        }
    }
    fill(&mut m);
    let thawn = Thawn {
        singletons: (singletons.iter()).map(|&(o, id)| (o, m.instance(id))).collect(),
        bindings: bindings.iter().map(|&id| m.instance(id)).collect(),
        temp_blob_fields: (image.temp_blob_fields.iter())
            .filter_map(|&(r, pos)| Some((m.record_handle(r)?, pos)))
            .collect(),
        temps: std::mem::take(&mut m.temps),
        callstacks: std::mem::take(&mut m.callstacks),
    };
    match m.broken.get() {
        Some(what) => Err(what),
        None => Ok(thawn),
    }
}

impl Thawn {
    /// Makes `session` hold the graph [`make`] built.
    pub(crate) fn install(self, session: &mut Session) {
        session.temps.extend(self.temps);
        for (cell, stack) in &self.callstacks {
            al_builtins::errorinfo::set_callstack_of(cell, stack);
        }
        session.singletons = self.singletons.into_iter().collect();
        session.events.bindings = (self.bindings.iter())
            .map(|instance| crate::dispatch::Binding {
                instance: Rc::downgrade(instance),
                // The image does not carry whether the login bound it outside a subscriber; `false` keeps a carried
                // Option's origin away from its manual subscribers, so such a hand-on refuses instead of running.
                outside_subscriber: false,
            })
            .collect();
        session.temp_blob_impl_fields.note_all(&self.temp_blob_fields);
    }
}

/// Pass 1: a new empty handle for every node that is filled later, a whole Blob buffer, a new temporary table id.
fn make_shells(m: &mut Maker<'_>, session: &mut Session) {
    for (i, n) in m.arena.nodes.iter().enumerate() {
        m.made[i] = match n {
            Node::Instance(inst) => Some(Made::Instance(al_builtins::instance_cycles::shared(Instance {
                object: inst.object,
                ..Default::default()
            }))),
            Node::Array(_) => Some(Made::Array(Rc::default())),
            Node::List { .. } => Some(Made::List(Rc::default())),
            Node::Dictionary(_) => Some(Made::Dictionary(Rc::default())),
            Node::Library(kind, _) => Some(Made::Library(kind.clone(), Rc::default())),
            Node::Query(q) => Some(Made::Query(Rc::new(RefCell::new(QueryState::new(q.object))))),
            Node::Blob(buf) => Some(Made::Blob(std::sync::Arc::new(al_builtins::stream::BlobCell::new(
                buf.clone(),
            )))),
            Node::TempTable(_) => Some(Made::TempTable(session.new_temp_id())),
            Node::Globals { .. } => Some(Made::Globals(TableGlobals::new_shared())),
            Node::DotNet(d) => Some(Made::DotNet(Rc::new(al_builtins::value::DotNetObject::new(match d {
                super::arena::DotNetNode::Internal { type_name, .. } => al_builtins::value::DotNetKind::Internal {
                    type_name: type_name.clone(),
                    state: RefCell::new(Vec::new()),
                },
            })))),
            Node::Record(_) | Node::Row(_) => None,
            Node::Pending => {
                m.broke("a node the walk did not fill");
                None
            }
        };
    }
}

/// Pass 1b: a set's xRec set is made only by the set it belongs to (`TableGlobals::xrec`): each is taken from there. Noted
/// as broken: an xRec set that is not a set's node or is the xRec set of two sets, and a chain that is a cycle.
fn make_xrecs(m: &mut Maker<'_>) {
    // A set is the xRec set of at most one set; a chain of xRec sets is taken from its root down.
    let parents: Vec<(usize, NodeId)> = (m.arena.nodes.iter().enumerate())
        .filter_map(|(i, n)| match n {
            Node::Globals { xrec: Some(x), .. } => Some((x.0 as usize, NodeId(i as u32))),
            _ => None,
        })
        .collect();
    let mut children = std::collections::HashSet::new();
    for &(child, _) in &parents {
        if !matches!(m.arena.nodes.get(child), Some(Node::Globals { .. })) || !children.insert(child) {
            m.broke("an xRec set that is not a set, or the xRec set of two sets");
            return;
        }
    }
    let mut done = vec![false; m.made.len()];
    let mut left = parents.len();
    while left > 0 {
        let before = left;
        for &(child, parent) in &parents {
            let parent_done = !parents.iter().any(|(c, _)| *c == parent.0 as usize) || done[parent.0 as usize];
            if done[child] || !parent_done {
                continue;
            }
            let xrec = m.globals(parent).xrec();
            m.made[child] = Some(Made::Globals(xrec));
            done[child] = true;
            left -= 1;
        }
        if left == before {
            m.broke("a chain of xRec sets that is a cycle");
            return;
        }
    }
}

/// Pass 2: each temporary table's store, with the new id, and each fetched row; they name only Blobs.
fn make_tables_and_rows(m: &mut Maker<'_>) {
    for (i, n) in m.arena.nodes.iter().enumerate() {
        match n {
            Node::TempTable(t) => {
                let tables = (t.tables.iter())
                    .map(|t| al_data::memory::ExportedTable {
                        company: t.company.clone(),
                        object: t.object,
                        pk_storage: t.pk_storage.clone(),
                        rows: t.rows.iter().map(|(k, r)| (m.values(k), m.values(r))).collect(),
                    })
                    .collect();
                // No marks: only a DeleteAll of a database table sets an emptied one (records/delete_all.rs), and a
                // temporary table deletes no company.
                let store = al_data::memory::MemStore::from_exported(tables, &[], &[]);
                let Some(id) = m.temp_id(NodeId(i as u32)).map(|t| t.id()) else {
                    continue;
                };
                m.temp_counters.insert(id, store.change_counter());
                m.temps.push((id, store));
            }
            Node::Row(r) => m.made[i] = Some(Made::Row(Rc::new(m.values(r)))),
            _ => {}
        }
    }
}

/// Pass 4: the content of every shell.
fn fill(m: &mut Maker<'_>) {
    let mut callstacks = Vec::new();
    for (i, n) in m.arena.nodes.iter().enumerate() {
        let Some(made) = m.made(NodeId(i as u32)) else { continue };
        match (n, made) {
            (Node::Instance(n), Made::Instance(r)) => {
                let mut inst = r.borrow_mut();
                inst.globals = m.values(&n.globals);
                inst.initialized = n.initialized;
                inst.rerun = n.rerun;
                inst.base = n.base.map(|b| Rc::downgrade(&m.instance(b)));
                inst.exts = n.exts.iter().map(|&(o, e)| (o, m.instance(e))).collect();
                inst.xp_text = n.xp_text.clone();
                inst.born = al_builtins::value::Born(n.born);
                inst.single = n.single;
                inst.unmeasured = None;
            }
            (Node::Array(items), Made::Array(r)) => *r.borrow_mut() = m.values(items),
            (
                Node::List {
                    items,
                    version,
                    read_only,
                },
                Made::List(r),
            ) => {
                let mut l = ListVec::from(m.values(items));
                l.version = *version;
                l.read_only = *read_only;
                *r.borrow_mut() = l;
            }
            (Node::Dictionary(entries), Made::Dictionary(r)) => {
                *r.borrow_mut() = entries.iter().map(|(k, v)| (m.value(k), m.value(v))).collect();
            }
            (Node::Globals { instances, .. }, Made::Globals(g)) => {
                for &(o, inst) in instances {
                    g.set_instance(o, m.instance(inst));
                }
            }
            (Node::Library(_, n), Made::Library(kind, r)) => {
                if let Some(v) = m.library(n) {
                    *r.borrow_mut() = v;
                }
                // A collected error's `Callstack`, in this thread's side table by the new cell (al-builtins errorinfo.rs),
                // written at install: a cell of a thaw that is dropped must leave no entry under its address.
                if let super::library::LibraryNode::ErrorInfo(e) = n
                    && let Some(stack) = &e.callstack
                {
                    callstacks.push((Value::Library(kind.clone(), r.clone()), stack.clone()));
                }
            }
            (Node::Query(q), Made::Query(r)) => {
                let mut s = r.borrow_mut();
                s.filters = (q.filters.iter())
                    .map(|(field, expr)| {
                        let f = super::plain::PlainFilter {
                            lang: 0,
                            field: *field,
                            expr: expr.clone(),
                            group: 0,
                        };
                        (*field, f.filter().expr)
                    })
                    .collect();
                s.filter_replaced = q.filter_replaced;
                s.top = q.top;
                s.security_filtering = q.security_filtering;
                s.run = None;
                s.instance = q.instance.map(|i| m.instance(i));
                s.in_before_open = q.in_before_open;
                s.closed = q.closed;
                s.reopen = q.reopen.clone();
            }
            (Node::DotNet(super::arena::DotNetNode::Internal { state, .. }), Made::DotNet(d)) => {
                if let al_builtins::value::DotNetKind::Internal { state: cell, .. } = &d.kind {
                    *cell.borrow_mut() = m.values(state);
                }
            }
            _ => {}
        }
    }
    m.callstacks = callstacks;
}
