//! The session image (al-runner issue 8211, PERF-01 step 2): what a test session holds after the raised login, as owned
//! data any thread may keep ([`LoginImage`], [`Session::login_image`]), and a new session made to hold it
//! ([`Session::thaw_login_image`]). Nothing replays from it yet: al-testrun `login_replay.rs` freezes each new key's
//! session, thaws it into a twin and compares the two with the login replay check, its differential test.
//!
//! Every field of `Session` is in one class of the table in `fields.rs`. The image holds the `plain` fields by value, the
//! sub-states and counters of `session_parts.rs`, the committed rows as a `SharedBaseline` (ADR-069's form, so no Blob
//! buffer is shared with the session that goes on to run its tests), and the SingleInstance object graph as an arena
//! (`arena.rs`). A field of the `refuse` or `rest` class that does not hold a new session's value means no image.
//!
//! The thaw's target is a session made by `Session::with_host` on the image's engine with the run's session flags
//! applied: every field the image does not write keeps that new session's value. It writes through the session's own
//! setters where one exists, so the thread-locals they keep in step follow:
//! - the filter language and the Turkic case rule follow GlobalLanguage: `Session::restore_carried`;
//! - the SQL counters re-read the store's operation count: `SqlCounters::restore_left`;
//! - an instance's `Born` reads a thread-local by default (al-builtins `value.rs` `BORN_RESTRICTED`): the thaw writes it;
//! - a new instance is noted in the thread's instance registry (`instance_cycles::shared`), so its `collect` can free it.

use crate::Session;

mod arena;
mod fields;
mod freeze;
mod library;
mod record;
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
    /// Every committed row, with the counters of `Session::data_now`.
    rows: crate::SharedBaseline,
    /// The tables a DeleteAll without filters emptied (`MemStore::committed_marks`): the bytes form of the rows does
    /// not keep the mark, so the thaw marks them again.
    emptied: Vec<(String, u32)>,
    /// The tables a write reached while the login ran (`MemStore::tables_changed_since`), for [`ImageSize`].
    rows_own: std::collections::BTreeSet<u32>,
    arena: arena::Arena,
    /// The instances `singletons` holds: (object index, node), in ascending object index.
    singletons: Vec<(usize, arena::NodeId)>,
    /// The live manual bindings in order, by instance node.
    bindings: Vec<arena::NodeId>,
    /// The live records of `Session::temp_blob_impl_fields`, by record node, with their field positions.
    temp_blob_fields: Vec<(arena::NodeId, usize)>,
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
    /// Code runs or a call left something behind (`Session::at_rest`).
    NotAtRest,
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
            NotImaged::NotAtRest => "not at rest".to_string(),
            NotImaged::Value { what, .. } => what.clone(),
            NotImaged::HeldOutside { .. } => "a handle held outside the image".to_string(),
            NotImaged::Rows(why) => format!("rows: {why}"),
        }
    }
}

/// How much an image holds ([`LoginImage::size`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImageSize {
    pub rows_bytes: u64,
    pub rows_own_bytes: u64,
    pub rows: usize,
    pub state_bytes: u64,
    pub nodes: usize,
    pub instances: usize,
    pub temp_tables: usize,
    pub temp_rows: usize,
}

impl Session {
    /// What this session holds, as an image any thread may keep. `before_login`: the store's change counter before the
    /// login ran (`MemStore::change_counter`), for `ImageSize::rows_own_bytes`.
    pub fn login_image(&self, before_login: u64) -> Result<LoginImage, NotImaged> {
        if !self.at_rest() {
            return Err(NotImaged::NotAtRest);
        }
        if let Some(field) = self.refused_field() {
            return Err(NotImaged::Field(field));
        }
        if self.store.has_uncommitted() {
            return Err(NotImaged::Rows("an uncommitted change".into()));
        }
        let parts = session_parts::SessionParts::take(self)?;
        let walk = freeze::Walker::new(self).walk()?;
        let rows = self.data_now().freeze().map_err(|why| match why {
            crate::NotShared::Value(v) => NotImaged::Rows(format!("a {} with no bytes form", v.kind)),
            crate::NotShared::NoReadBack => NotImaged::Rows("the rows do not read back from their bytes".into()),
        })?;
        let (emptied, _) = self.store.committed_marks();
        if let Some((company, object)) = emptied
            .iter()
            .find(|(c, o)| self.store.committed_table(c, *o).is_none())
        {
            let id = self
                .engine
                .program
                .objects
                .get(*object as usize)
                .and_then(|o| o.id)
                .unwrap_or(0);
            return Err(NotImaged::Rows(format!(
                "table {id} of {company} emptied with no entry in the store"
            )));
        }
        Ok(LoginImage {
            emptied,
            engine: self.engine.uid,
            plain: fields::PlainFields::take(self),
            parts,
            rows,
            rows_own: self.store.tables_changed_since(before_login),
            arena: walk.arena,
            singletons: walk.singletons,
            bindings: walk.bindings,
            temp_blob_fields: walk.temp_blob_fields,
        })
    }

    /// Makes this session hold what the image holds. `self`: made by `Session::with_host` on the image's engine, with
    /// the run's session flags applied.
    pub fn thaw_login_image(&mut self, image: &LoginImage) {
        assert_eq!(
            self.engine.uid, image.engine,
            "an image is thawn on the engine it was taken on"
        );
        debug_assert!(
            self.at_rest() && self.refused_field().is_none(),
            "the thaw's target is a new session"
        );
        self.restore_baseline(&image.rows.thaw(Some(&self.engine)));
        // The tables a DeleteAll emptied: the rows the baseline gave, marked emptied again (a new stamp, as a write gives).
        let emptied: Vec<_> = (image.emptied.iter())
            .filter_map(|(company, object)| self.store.committed_table(company, *object))
            .map(|(table, _)| (table, true))
            .collect();
        if !emptied.is_empty() {
            self.store = std::mem::take(&mut self.store).with_committed_tables(emptied, &[]);
        }
        // After the baseline: the plain fields hold what the session held, not what the restore set from the baseline.
        image.plain.give(self);
        image.parts.give(self);
        thaw::thaw(self, image);
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
