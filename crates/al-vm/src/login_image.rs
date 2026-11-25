//! The session image (al-runner issue 8211): what a test session holds after the raised login, as owned
//! data any thread may keep ([`LoginImage`], [`Session::login_image`]), and a new session made to hold it
//! ([`Session::thaw_login_image`]). al-testrun `login_start.rs` keeps an image per login key and thaws it into a later
//! Session of the key in place of its login; `login_replay.rs` compares thaws with real logins, the
//! differential test.
//!
//! Every field of `Session` is in one class of the session field table (`session_fields.rs`). The image holds the `plain`
//! fields by value (`fields.rs`, made from the table), the sub-states and counters of `session_parts.rs`, the committed
//! rows the login wrote apart from the store it started from (`rows.rs`, over [`LoginBase`]: rows go through their
//! bytes, so no Blob buffer is shared with the session that goes on to run its tests), the shape of that store
//! ([`BaseShape`]), and the SingleInstance object graph as an arena (`arena.rs`). A field of the `refuse` or `rest` class
//! that does not hold a new session's value means no image (`Session::image_refused_field`, `Session::at_rest_why`).
//!
//! Of the session state the table lists outside `Session`'s fields, what is the session's own is imaged: the emptied
//! marks of the store (in `rows.rs`, test `a_table_the_login_emptied_stays_emptied_in_the_twin`) and an ErrorInfo's
//! call stack (`ErrorInfoNode::callstack`, test `an_error_infos_call_stack_follows_it_into_the_twin`).
//!
//! The thaw's target is a session on the image's engine with the run's session flags applied, that holds the store the
//! login started from (the login key proves the rows equal; [`Session::base_shape`] what the key's digest does not see)
//! and that [`Session::thaw_target_why`] accepts: every field the image does not write keeps the target's value, which is
//! a new session's for every `refuse` and `rest` field. It writes through the session's own setters where one exists, so
//! the thread-locals they keep in step follow:
//! - the filter language and the Turkic case rule follow GlobalLanguage: `Session::restore_carried`;
//! - the SQL counters re-read the store's operation count: `SqlCounters::restore_left`;
//! - an instance's `Born` reads a thread-local by default (al-builtins `value.rs` `BORN_RESTRICTED`): the thaw writes it;
//! - a new instance is noted in the thread's instance registry (`instance_cycles::shared`), so its `collect` can free it.

use crate::Session;

mod arena;
mod fields;
mod freeze;
mod library;
mod plain;
mod record;
mod rows;
mod session_parts;
mod size;
mod temps;
mod thaw;

/// What a session holds after the login, as an image any thread may keep (see the module comment).
pub struct LoginImage {
    /// `Engine::uid` of the engine the image was taken on: it is thawn only on that engine.
    engine: u64,
    plain: fields::PlainFields,
    parts: session_parts::SessionParts,
    /// The committed rows apart from the store the login started from, row by row, and the emptied marks
    /// (`rows::LoginRows::take`). The counters are plain fields.
    rows: rows::LoginRows,
    arena: arena::Arena,
    /// The instances `singletons` holds: (object index, node), in ascending object index.
    singletons: Vec<(usize, arena::NodeId)>,
    /// The live manual bindings in order, by instance node.
    bindings: Vec<arena::NodeId>,
    /// The live records of `Session::temp_blob_impl_fields`, by record node, with their field positions.
    temp_blob_fields: Vec<(arena::NodeId, usize)>,
    /// The shape of the store the login started from ([`Session::base_shape`]): a replay needs the target's to be equal.
    base_shape: BaseShape,
}

// The image is kept and given to sessions of other threads: `Send + Sync` by its types, no `unsafe`.
const _: fn() = || {
    fn both<T: Send + Sync>() {}
    both::<LoginImage>();
};

/// Why a session has no image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotImaged {
    /// A `Session` field (or `tx.x`, `events.x`) does not hold a new session's value.
    Field(&'static str),
    /// Code runs or a call left something behind: the first field of the `rest` class that does not hold
    /// (`Session::at_rest_why`).
    NotAtRest(&'static str),
    /// A value with no exact form: where (ids and indexes only, never a value) and what.
    Value { path: String, what: String },
    /// A handle something outside the imaged globals also holds.
    HeldOutside { path: String },
    /// The rows: an uncommitted change, or a row value with no bytes form.
    Rows(String),
}

impl NotImaged {
    /// One line for the report, with no value and no path in it: the reason that is counted.
    pub fn reason(&self) -> String {
        match self {
            NotImaged::Field(name) => format!("field {name}"),
            NotImaged::NotAtRest(field) => format!("not at rest: {field}"),
            NotImaged::Value { what, .. } => what.clone(),
            NotImaged::HeldOutside { .. } => "a handle held outside the image".to_string(),
            NotImaged::Rows(why) => format!("rows: {why}"),
        }
    }
}

/// Why an image is not thawn into a session ([`Session::thaw_login_image`]): the session is left as it was, so its caller
/// runs the login in it instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotThawed {
    /// The image was taken on another engine (`Engine::uid`).
    Engine,
    /// The session is not a thaw target: the first field of `Session::at_rest_why`, of `Session::image_refused_field` or
    /// of the facts the thaw does not write that does not hold, or a store watch (`Session::thaw_target_why`).
    Target(&'static str),
    /// The session's store does not take the rows the login wrote: it lacks a table the login changed row by row (a
    /// base of another shape than the image's).
    Rows,
}

impl NotThawed {
    /// One line for the report, with no value in it: the reason that is counted.
    pub fn reason(&self) -> String {
        match self {
            NotThawed::Engine => "an image of another engine".to_string(),
            NotThawed::Target(field) => format!("the target is not a new session: {field}"),
            NotThawed::Rows => "the target's store does not take the rows the login wrote".to_string(),
        }
    }
}

/// How much an image holds ([`LoginImage::size`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageSize {
    /// The encoded rows the login wrote (`LoginRows::size`).
    pub rows_bytes: u64,
    pub state_bytes: u64,
    pub nodes: usize,
    pub instances: usize,
    pub temp_tables: usize,
    pub temp_rows: usize,
    /// What keeping the image costs ([`LoginImage::kept_bytes`]): what the starts' memory cap counts.
    pub kept_bytes: u64,
    /// Per kind of node, in the order of [`NODE_KINDS`]: how many the arena holds and their bytes (names only, no value).
    pub kinds: [KindSize; NODE_KINDS.len()],
}

/// The kinds of arena node, as the image line names them ([`ImageSize::kinds`]).
pub const NODE_KINDS: [&str; 12] = [
    "instance",
    "array",
    "list",
    "dictionary",
    "record",
    "table globals",
    "library",
    "blob",
    "temporary table",
    "row",
    "query",
    "dotnet",
];

/// How many nodes of one kind an image holds, and their bytes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KindSize {
    pub count: usize,
    pub bytes: u64,
}

/// What a login's row delta assumes of the store it is put on, beyond the rows the key's digest sees: which tables have
/// an entry (with their row counts), and the committed marks the rows leave out (the tables a DeleteAll emptied, the
/// company copies, the deleted companies). A replay takes a start only onto a store of the same shape.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BaseShape {
    /// (company, table object, committed rows) of every committed table entry, sorted.
    pub tables: Vec<(String, u32, usize)>,
    /// al-data `MemStore::committed_marks`.
    pub marks: al_data::memory::CommittedMarks,
}

impl BaseShape {
    /// About how many bytes the shape holds.
    fn bytes(&self) -> usize {
        use std::mem::size_of;
        self.tables
            .iter()
            .map(|(c, ..)| size_of::<(String, u32, usize)>() + c.len())
            .sum::<usize>()
            + (self.marks.emptied.iter())
                .map(|(c, _)| size_of::<(String, u32)>() + c.len())
                .sum::<usize>()
            + (self.marks.copies.iter())
                .map(|(a, b)| size_of::<(String, String)>() + a.len() + b.len())
                .sum::<usize>()
            + (self.marks.deleted.iter())
                .map(|c| size_of::<String>() + c.len())
                .sum::<usize>()
    }
}

/// What a session was before its login ([`Session::login_base`]): the store the login starts from (a root clone, so it
/// stays on the session's thread), its shape, and whether the session could have taken a start then
/// ([`Session::thaw_target_why`]).
pub struct LoginBase {
    /// The committed store before the login (a detached root clone): the image holds the rows apart from it.
    store: al_data::memory::MemStore,
    /// The session's data before the login, for the check's twin ([`LoginBase::portable`]); `None` only for the test
    /// base [`LoginBase::before_everything`].
    data: Option<crate::Baseline>,
    shape: BaseShape,
    target: Option<&'static str>,
}

impl LoginBase {
    /// The shape of the store before the login.
    pub fn shape(&self) -> &BaseShape {
        &self.shape
    }

    /// [`Session::thaw_target_why`] before the login.
    pub fn target_why(&self) -> Option<&'static str> {
        self.target
    }

    /// The store before the login in a form another thread may take ([`Session::start_from`]): the baseline's bytes
    /// (`Baseline::to_bytes`) and the tables a DeleteAll emptied, which the bytes leave out. `None` when a row has no
    /// bytes form or the store holds company copies or deleted companies, which the bytes leave out too.
    pub fn portable(&self) -> Option<PortableBase> {
        let marks = &self.shape.marks;
        if !marks.copies.is_empty() || !marks.deleted.is_empty() {
            return None;
        }
        Some(PortableBase {
            bytes: self.data.as_ref()?.to_bytes()?,
            emptied: marks.emptied.clone(),
        })
    }

    /// A base before every change of the store: the image then holds every committed row.
    #[cfg(test)]
    pub(crate) fn before_everything() -> LoginBase {
        LoginBase {
            store: al_data::memory::MemStore::default(),
            data: None,
            shape: BaseShape::default(),
            target: None,
        }
    }
}

/// The store a login started from, as rows another thread may take ([`LoginBase::portable`]): the check thaws an image
/// into a twin on a thread of its own, which starts from it ([`Session::start_from`]).
pub struct PortableBase {
    bytes: Vec<u8>,
    emptied: Vec<(String, u32)>,
}

impl Session {
    /// What this session is before its login: take it before the login and give it to [`Session::login_image`].
    pub fn login_base(&self) -> LoginBase {
        LoginBase {
            store: self.store.clone().detached(),
            data: Some(self.data_now()),
            shape: self.base_shape(),
            target: self.thaw_target_why(),
        }
    }

    /// The shape of this session's committed store ([`BaseShape`]): what a row delta assumes of the store it is put on
    /// beyond the rows.
    pub fn base_shape(&self) -> BaseShape {
        BaseShape {
            tables: self.store.committed_entries(),
            marks: self.store.committed_marks(),
        }
    }

    /// Why this session cannot take a login start in place of its login: the first field of the `at_rest` rule or of the
    /// `refuse` class that does not hold a new session's value, then of the facts the thaw does not write (the transaction,
    /// event and UI facts and the Temp Blob fields), or `store watch` when an access watch is on (it would see the thaw's
    /// writes). `None`: it can. The thaw's own target rule.
    pub fn thaw_target_why(&self) -> Option<&'static str> {
        (self.at_rest_why())
            .or_else(|| self.image_refused_field())
            .or_else(|| self.unwritten_fact_why())
            .or_else(|| self.store.watching().then_some("store watch"))
    }

    /// Makes this session start from `base` ([`LoginBase::portable`]) over the engine's test data, as the session the
    /// base was taken from started its login: the check's twin, brought to its login by the runner's steps. SQL
    /// counters at rest stay at rest: they note the store operations of the thread since the last Commit, and putting
    /// the store back is none of the session's (they read the count again, as `restore_install_left` has them do).
    pub fn start_from(&mut self, base: &PortableBase) {
        let at_rest = self.sql.left_inactive();
        let b = (crate::Baseline::from_bytes(&base.bytes).expect("a base read back from its own bytes"))
            .backed_by(&self.engine);
        self.restore_baseline(&b);
        let marked = rows::marked_as(&self.store, &base.emptied, &Default::default())
            .expect("an emptied table has an entry, and the bytes keep an empty one");
        self.store = std::mem::take(&mut self.store).with_committed_tables(marked, &[]);
        if let Some(left) = at_rest {
            self.sql.restore_left(&left);
        }
    }

    /// What this session holds, as an image any thread may keep. `base`: what it was before the login
    /// ([`Session::login_base`]).
    pub fn login_image(&self, base: &LoginBase) -> Result<LoginImage, NotImaged> {
        if let Some(field) = self.at_rest_why() {
            return Err(NotImaged::NotAtRest(field));
        }
        if let Some(field) = self.image_refused_field() {
            return Err(NotImaged::Field(field));
        }
        if self.store.has_uncommitted() {
            return Err(NotImaged::Rows("an uncommitted change".into()));
        }
        let parts = session_parts::SessionParts::take(self)?;
        let walk = freeze::Walker::new(self).walk()?;
        let rows = rows::LoginRows::take(&self.store, &base.store, &base.shape.tables).ok_or_else(|| {
            NotImaged::Rows(
                "a row with no bytes form, or company copies or deleted companies apart from the store the login started \
                 from"
                    .into(),
            )
        })?;
        Ok(LoginImage {
            engine: self.engine.uid,
            plain: fields::PlainFields::take(self),
            parts,
            rows,
            arena: walk.arena,
            singletons: walk.singletons,
            bindings: walk.bindings,
            temp_blob_fields: walk.temp_blob_fields,
            base_shape: base.shape.clone(),
        })
    }

    /// Makes this session hold what the image holds, in place of running its login. `self`: a session of the image's
    /// engine with the run's session flags applied, that [`Session::thaw_target_why`] accepts and whose store holds what
    /// the login started from (equal rows by the login key, an equal [`Session::base_shape`]): the rows the login wrote go
    /// on top of its own store, and every row it did not write stays as the target has it. `Err`: the image is of another
    /// engine, `self` is not a thaw target, or its store lacks a table the login changed row by row; nothing of `self` is
    /// changed then, so its caller runs the login in it instead.
    pub fn thaw_login_image(&mut self, image: &LoginImage) -> Result<(), NotThawed> {
        if self.engine.uid != image.engine {
            return Err(NotThawed::Engine);
        }
        if let Some(field) = self.thaw_target_why() {
            return Err(NotThawed::Target(field));
        }
        // Every table the login changed row by row is one the start's base held; a target whose base has its shape (the
        // replay's gate compares them) holds each, and another target is refused here, before anything changes.
        image.rows.put_on(&mut self.store).ok_or(NotThawed::Rows)?;
        // The counters a baseline carries are plain fields (next_guid, autoinc, sequences, isolated storage, last_stamp,
        // the copies, the restamped rows), and `parts` gives the committed row version and the baseline origin.
        image.plain.give(self);
        image.parts.give(self);
        thaw::thaw(self, image);
        Ok(())
    }

    /// The first fact of a thaw's target that the thaw does not write and that does not hold a new session's value, by
    /// the rules the image is taken by (`SessionParts::take`): the transaction state besides what a Commit leaves, the
    /// event state besides what the image carries, the UI state, and the Temp Blob fields (the thaw adds its own).
    fn unwritten_fact_why(&self) -> Option<&'static str> {
        if let Some(field) = crate::install_left::tx_not_as_new(&self.tx) {
            return Some(field);
        }
        if !crate::install_left::clients::ui_left_as_new(&self.ui_state) {
            return Some("ui_state");
        }
        if let Some(field) = crate::install_left::clients::events_not_as_new(&self.events) {
            return Some(field);
        }
        (!self.temp_blob_impl_fields.as_new()).then_some("temp_blob_impl_fields")
    }
}

#[cfg(test)]
impl LoginImage {
    /// Gives the second reference to the first instance that has two a node of its own. `false`: none has two.
    pub(crate) fn split_first_shared_instance(&mut self) -> bool {
        use arena::{Frozen, Node, NodeId};
        let mut seen = std::collections::HashSet::new();
        // The references in walk order: `singletons`, then the globals of each instance node.
        let mut second: Option<(Option<usize>, usize, NodeId)> = None;
        for (k, &(_, id)) in self.singletons.iter().enumerate() {
            if !seen.insert(id.0) {
                second = Some((None, k, id));
                break;
            }
        }
        if second.is_none() {
            'nodes: for (i, n) in self.arena.nodes.iter().enumerate() {
                let Node::Instance(inst) = n else { continue };
                for (g, v) in inst.globals.iter().enumerate() {
                    let Frozen::Node(id) = v else { continue };
                    if matches!(self.arena.node(*id), Node::Instance(_)) && !seen.insert(id.0) {
                        second = Some((Some(i), g, *id));
                        break 'nodes;
                    }
                }
            }
        }
        let Some((owner, at, id)) = second else { return false };
        let Node::Instance(original) = self.arena.node(id) else {
            return false;
        };
        let copy = Node::Instance(original.clone());
        let new = NodeId(self.arena.nodes.len() as u32);
        self.arena.nodes.push(copy);
        match owner {
            None => self.singletons[at].1 = new,
            Some(i) => {
                if let Node::Instance(inst) = &mut self.arena.nodes[i] {
                    inst.globals[at] = Frozen::Node(new);
                }
            }
        }
        true
    }

    /// Forgets the clock's draws.
    pub(crate) fn forget_clock_draws(&mut self) {
        self.plain.clock_draws = 0;
    }
}

#[cfg(test)]
mod tests;
