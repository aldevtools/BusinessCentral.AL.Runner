//! The thaw of an [`Arena`] into a session (design section 2.3), on the session's own thread. Every node gets a new handle
//! first (an instance through `instance_cycles::shared`, so the thread's registry notes it; a temporary table a new id of
//! the session and its store), then each is filled with handles of the same thaw: so cycles need no special case, and
//! no thawn value is shared with the image or another thaw (except `Txt` and `Arc<[String]>`, which are immutable).
//!
//! The order: shells (and Blobs, which name nothing); xRec sets from the set they belong to; temporary tables' stores and
//! fetched rows (they name only Blobs); records (they name tables, rows, Blobs and sets); the temporary tables' empty-Blob
//! Insert notes (they name records); then the content of every shell.

use super::arena::{Arena, Frozen, Node, NodeId};
use crate::Session;
use al_builtins::value::{Instance, LibraryValue, ListVec, QueryState, RecordState, TableGlobals, TempId, Value};
use std::cell::RefCell;
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
}

/// The handles of one thaw, by node.
pub(crate) struct Maker<'a> {
    arena: &'a Arena,
    made: Vec<Option<Made>>,
    /// The thawn database store's change count, and each thawn temporary table's (by its new id).
    pub(crate) store_counter: u64,
    temp_counters: HashMap<u64, u64>,
}

impl Maker<'_> {
    fn made(&self, id: NodeId) -> &Made {
        self.made[id.0 as usize]
            .as_ref()
            .expect("the thaw makes a node before anything names it")
    }

    /// The value of `f`.
    pub(crate) fn value(&self, f: &Frozen) -> Value {
        match f {
            Frozen::Plain(p) => p.value(),
            Frozen::Node(id) => match self.made(*id) {
                Made::Instance(r) => Value::Codeunit(r.clone()),
                Made::Array(r) => Value::Array(r.clone()),
                Made::List(r) => Value::List(r.clone()),
                Made::Dictionary(r) => Value::Dictionary(r.clone()),
                Made::Record(r) => Value::Record(r.clone()),
                Made::Library(kind, r) => Value::Library(kind.clone(), r.clone()),
                Made::Blob(b) => Value::Blob(b.clone()),
                Made::Query(q) => Value::Query(q.clone()),
                Made::Globals(_) | Made::TempTable(_) | Made::Row(_) => {
                    unreachable!(
                        "the freeze names a table globals set, a temporary table or a row only where it is one"
                    )
                }
            },
        }
    }

    pub(crate) fn values(&self, f: &[Frozen]) -> Vec<Value> {
        f.iter().map(|v| self.value(v)).collect()
    }

    fn instance(&self, id: NodeId) -> Rc<RefCell<Instance>> {
        match self.made(id) {
            Made::Instance(r) => r.clone(),
            _ => unreachable!("an instance node"),
        }
    }

    pub(crate) fn record_handle(&self, id: NodeId) -> Rc<RefCell<RecordState>> {
        match self.made(id) {
            Made::Record(r) => r.clone(),
            _ => unreachable!("a record node"),
        }
    }

    pub(crate) fn globals(&self, id: NodeId) -> &TableGlobals {
        match self.made(id) {
            Made::Globals(g) => g,
            _ => unreachable!("a table globals node"),
        }
    }

    pub(crate) fn temp_id(&self, id: NodeId) -> TempId {
        match self.made(id) {
            Made::TempTable(t) => t.clone(),
            _ => unreachable!("a temporary table node"),
        }
    }

    pub(crate) fn row(&self, id: NodeId) -> Rc<Vec<Value>> {
        match self.made(id) {
            Made::Row(r) => r.clone(),
            _ => unreachable!("a row node"),
        }
    }

    /// The change count of the thawn temporary table `id`.
    pub(crate) fn session_temp_counter(&self, id: u64) -> u64 {
        self.temp_counters.get(&id).copied().unwrap_or(0)
    }
}

/// Makes `session` hold the graph of `image`: its handles, its temporary tables, `singletons` (object index, node), the
/// manual bindings (instance nodes in order) and the live Temp Blob records.
pub(crate) fn thaw(session: &mut Session, image: &super::LoginImage) {
    let (arena, singletons, bindings) = (&image.arena, &image.singletons[..], &image.bindings[..]);
    let mut m = Maker {
        arena,
        made: arena.nodes.iter().map(|_| None).collect(),
        store_counter: session.store.change_counter(),
        temp_counters: HashMap::new(),
    };
    make_shells(&mut m, session);
    make_xrecs(&mut m);
    make_tables_and_rows(&mut m, session);
    for (i, n) in arena.nodes.iter().enumerate() {
        if let Node::Record(r) = n {
            m.made[i] = Some(Made::Record(Rc::new(RefCell::new(m.record(r)))));
        }
    }
    for (i, n) in arena.nodes.iter().enumerate() {
        if let Node::TempTable(t) = n {
            let table = NodeId(i as u32);
            let holder = (arena.nodes.iter().enumerate())
                .find(|(_, n)| matches!(n, Node::Record(r) if r.temp() == Some(table)))
                .map(|(r, _)| NodeId(r as u32));
            m.note_empty_blob_insert(t, &m.temp_id(table), holder);
        }
    }
    fill(&m);
    session.singletons = (singletons.iter()).map(|&(o, id)| (o, m.instance(id))).collect();
    session.events.bindings = (bindings.iter())
        .map(|&id| crate::dispatch::Binding {
            instance: Rc::downgrade(&m.instance(id)),
        })
        .collect();
    let fields: Vec<_> = (image.temp_blob_fields.iter())
        .map(|&(r, pos)| (m.record_handle(r), pos))
        .collect();
    session.temp_blob_impl_fields.note_all(&fields);
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
            Node::Record(_) | Node::Row(_) => None,
            Node::Pending => unreachable!("the walk fills every node"),
        };
    }
}

/// Pass 1b: a set's xRec set is made only by the set it belongs to (`TableGlobals::xrec`): each is taken from there.
fn make_xrecs(m: &mut Maker<'_>) {
    // A set is the xRec set of at most one set; a chain of xRec sets is taken from its root down.
    let parents: Vec<(usize, NodeId)> = (m.arena.nodes.iter().enumerate())
        .filter_map(|(i, n)| match n {
            Node::Globals { xrec: Some(x), .. } => Some((x.0 as usize, NodeId(i as u32))),
            _ => None,
        })
        .collect();
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
        assert!(left < before, "xRec sets form no cycle: each is made by its own set");
    }
}

/// Pass 2: each temporary table's store, with the new id, and each fetched row; they name only Blobs.
fn make_tables_and_rows(m: &mut Maker<'_>, session: &mut Session) {
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
                let store = al_data::memory::MemStore::from_exported(tables);
                let id = m.temp_id(NodeId(i as u32)).id();
                m.temp_counters.insert(id, store.change_counter());
                session.temps.insert(id, store);
            }
            Node::Row(r) => m.made[i] = Some(Made::Row(Rc::new(m.values(r)))),
            _ => {}
        }
    }
}

/// Pass 4: the content of every shell.
fn fill(m: &Maker<'_>) {
    for (i, n) in m.arena.nodes.iter().enumerate() {
        let id = NodeId(i as u32);
        match (n, m.made(id)) {
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
            (Node::Library(_, n), Made::Library(_, r)) => *r.borrow_mut() = m.library(n),
            (Node::Query(q), Made::Query(r)) => {
                let mut s = r.borrow_mut();
                s.filters = (q.filters.iter())
                    .map(|(field, expr)| {
                        let f = al_builtins::plain::PlainFilter {
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
            _ => {}
        }
    }
}
