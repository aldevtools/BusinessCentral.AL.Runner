//! The form in which a SingleInstance codeunit's globals go from one test codeunit's session to the next (ADR-066
//! amendment 1, option A; `single_carry.rs` notes what is read and written).
//!
//! A [`CarryVal`] holds no `Rc`: it can go to another thread and be compared. Plain values are written with
//! [`al_builtins::value_codec`]; handles (codeunit values, lists, dictionaries, arrays, records, RecordRefs) are
//! written with what they hold. What has no exact meaning in another session is not written: the encoder gives the
//! reason instead, and a later codeunit that reads such a global first is refused (`Signal::Unsupported`).
//!
//! Each session-relative part of a record ([`RecordState`]) has one meaning in the next session:
//! - `temp_id`: the temporary table goes with the record. The next session gets a new private table holding the same
//!   rows (`MemStore::export_current`): BC rolls back no temporary table. A temporary table two carried handles share,
//!   a table with an AutoIncrement field that has rows or whose counter the session moved (the counter is the
//!   session's, `autoinc.rs`) and a table whose Code key is stored as a number (its key order is not written with the
//!   rows) are refused.
//! - The row version (SystemRowVersion in the buffer, `row_origin.version`, `failed_insert`, the version in
//!   `last_modify`) of a record that is not temporary: carried when it is at or below the session's committed row
//!   version at the point it began to track the single instances. Such a row was committed before the test codeunit
//!   began, and every session of the run starts from the same committed rows, so the number names the same row
//!   version there. A higher number, or a buffer of the variable's own write, names a row BC rolled back at the end of
//!   that test codeunit (TestCuP7BReader.al:24-37: rows are rolled back per codeunit) while the next session hands out
//!   the same numbers again. BC rolls back rows, not variables: such a buffer keeps its fields, and what it knew of the
//!   row (origin, versions, IsDirty base) is dropped and marked (`records::ROLLED_BACK_ROW`): field reads and writes,
//!   TestField, FieldCaption and the view methods run, a Get or Find that finds a row fills it again, and every other
//!   method is refused. Temporary rows keep their numbers with their rows (a temporary record's row version is no
//!   database row version: `stale_verdict` returns Fresh for it first).
//! - `row_origin.commits`: [`RowOrigin::EARLIER_SESSION`]. The origin's transaction ended with the earlier test
//!   codeunit, so no Commit count since it is a number the stale-buffer rules know; with that value none of them
//!   (`o.commits + k == self.commits`, `o.commits == self.commits`) holds. An origin of the variable's own write on a
//!   table that is not temporary is refused (its row was rolled back; covered by the row version rule).
//! - `company`, `crm`, `run_page_link`: a company name is carried (every session has the same companies at its
//!   start); a CRM table (its connection is the session's) and a page's RunPageLink filters are refused.
//! - The table's globals (`globals`): carried as codeunit instances; `xRec`'s set is carried the same way.
//! - Everything else (fields, filters, marks, key, order, position, load state, the IsDirty snapshots, flags) is a
//!   value of the variable and is carried as it is.

use crate::Session;
use al_builtins::describe::Describe;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::library_kind::LibraryKind;
use al_builtins::value::{
    Buffer, DirtyBase, DotNetKind, DotNetObject, FetchedRow, FieldSet, Instance, LibraryValue, ListVec, LoadState,
    RecordState, RowOrigin, TableGlobals, Value,
};
use al_builtins::value_codec::{self as vc, Reader};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// A value in a form that can go to another thread and be compared (see the module).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarryVal {
    /// A plain value ([`al_builtins::value_codec`]).
    Plain(Vec<u8>),
    /// A SingleInstance codeunit: the session's one instance of it.
    Single(u32),
    Codeunit(Box<CarryInst>),
    DotNetEnum {
        clr_type: String,
        name: String,
        ordinal: i64,
    },
    /// A .NET object the runtime serves itself (`dotnet/registry.rs`), with its state.
    DotNetInternal {
        type_name: String,
        state: Vec<CarryVal>,
    },
    List {
        items: Vec<CarryVal>,
        version: u64,
        read_only: bool,
    },
    Dictionary(Vec<(CarryVal, CarryVal)>),
    Array(Vec<CarryVal>),
    Record(Box<CarryRec>),
    /// An unopened RecordRef or TextBuilder (`LibraryValue::Empty`), an open RecordRef, a RecordId, a TextBuilder's
    /// text.
    Library(LibraryKind, CarryLib),
    /// The first place of a handle that other places of the same group (the globals of one single instance) also
    /// hold; the number names it for them.
    Def(u32, Box<CarryVal>),
    /// A handle written earlier in the group ([`CarryVal::Def`]).
    Ref(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarryLib {
    Empty,
    /// A JsonObject, JsonArray, JsonToken or JsonValue that is the root of its tree, with the tree.
    Json(Box<CarryJson>),
    Text(String),
    TextCap(String, i32),
    RecordRef(Box<CarryRec>),
    RecordId {
        table: u32,
        table_no: u32,
        name: String,
        key: Vec<u8>,
    },
    /// A JSON handle written earlier in the group: a variable ([`CarryVal::Def`]) or a node of a tree
    /// ([`CarryJson::def`]).
    Ref(u32),
}

/// A JSON node: its children are new handles whose parent is the node (`json::JsonCell::parent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarryJson {
    pub node: CarryJsonNode,
    pub unpinned: Option<&'static str>,
    /// The number of a node that a variable of the group also holds.
    pub def: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarryJsonNode {
    Object(Vec<(String, CarryJson)>),
    Array(Vec<CarryJson>),
    Null,
    Undefined,
    Val(Vec<u8>),
    Raw(String),
    /// A node a variable of the group holds, written there first; it gets this node as its parent.
    Ref(u32),
}

fn json_kind(k: &LibraryKind) -> bool {
    matches!(
        k,
        LibraryKind::JsonObject | LibraryKind::JsonArray | LibraryKind::JsonToken | LibraryKind::JsonValue
    )
}

/// A codeunit (or table) instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarryInst {
    pub object: u32,
    pub initialized: bool,
    pub rerun: bool,
    pub born: bool,
    pub xp_text: Vec<(u8, String)>,
    pub globals: Vec<CarryVal>,
    /// Bound by `BindSubscription` at the end of the codeunit: its place among the bindings that reach the next
    /// session ([`binding_ranks`]). BC keeps the binding of an instance a single instance's global holds
    /// (TestEventSingleInstanceBindingBoundary.al:99), so the next session binds it again.
    pub bound: Option<u32>,
}

/// One table of a temporary record's private store: company, object, and each row's key and values (encoded).
pub type CarryTable = (String, u32, Vec<(Vec<u8>, Vec<u8>)>);

/// A record variable (see the module for the meaning of each session-relative part).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarryRec {
    pub table: u32,
    pub temporary: bool,
    /// The rows of its temporary table (`None`: the record has no table of its own yet).
    pub temp_rows: Option<Vec<CarryTable>>,
    pub company: Option<String>,
    pub fields: Vec<u8>,
    pub filters: Vec<u8>,
    pub filter_group: i32,
    pub marks: Vec<Vec<u8>>,
    pub marked_only: bool,
    pub read_isolation: i32,
    pub security_filtering: i32,
    pub key: usize,
    pub adhoc: Option<(usize, Vec<usize>)>,
    pub ascending: bool,
    pub field_descending: u32,
    pub position: Option<Vec<u8>>,
    pub auto_calc: Vec<usize>,
    pub load_request: Option<Vec<usize>>,
    pub load_buffer: Option<CarryBuffer>,
    pub load_partial: bool,
    pub touched: bool,
    pub assigned: bool,
    pub inserted: bool,
    pub dirty_base: Option<Vec<u8>>,
    pub is_dirty_base: CarryDirty,
    pub last_modify: Option<Vec<u8>>,
    pub failed_insert: Option<i64>,
    /// With `commits` set to [`RowOrigin::EARLIER_SESSION`].
    pub row_origin: Option<RowOrigin>,
    pub globals: Vec<(u32, CarryInst)>,
    pub xrec_globals: Vec<(u32, CarryInst)>,
    pub unpinned_buffer: Option<&'static str>,
    /// The highest database row version the record names (0 for a temporary record): the next session must have
    /// committed it too.
    pub max_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarryBuffer {
    pub have: FieldSet,
    pub unsure: FieldSet,
    pub row: Option<String>,
    pub cleared: bool,
    pub assigned: Vec<usize>,
    pub missing_loaded: bool,
    pub fetched: Option<Vec<u8>>,
    pub init: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarryDirty {
    Unknown(&'static str),
    Fresh,
    Synced,
    Row {
        base: Vec<u8>,
        alt: Option<Vec<u8>>,
        dirty_only: bool,
        stale_store: bool,
    },
}

/// Where a handle stands in its group ([`Enc::place`]).
enum Place {
    Alone,
    First(u32),
    Again(u32),
}

/// How deep handles inside handles are carried.
const DEPTH: usize = 12;

/// A row of a record (its fields, a key, a stored row): plain values, and RecordIds (a RecordID field holds one).
/// `Err`: the value that cannot be written.
fn row_bytes(row: &[Value]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    vc::put_u32(&mut out, row.len() as u32);
    match row.iter().find(|v| !put_cell(v, &mut out)) {
        Some(v) => Err(v.describe()),
        None => Ok(out),
    }
}

fn put_cell(v: &Value, out: &mut Vec<u8>) -> bool {
    if let Value::Library(LibraryKind::RecordId, lv) = v {
        let Ok(lv) = lv.try_borrow() else { return false };
        // A RecordID field nothing was assigned to.
        if matches!(&*lv, LibraryValue::Empty) {
            vc::put_u8(out, 2);
            return true;
        }
        let LibraryValue::RecordId {
            table,
            table_no,
            name,
            key,
        } = &*lv
        else {
            return false;
        };
        vc::put_u8(out, 1);
        vc::put_u32(out, *table);
        vc::put_u32(out, *table_no);
        vc::put_str(out, name);
        vc::put_u32(out, key.len() as u32);
        return key.iter().all(|k| put_cell(k, out));
    }
    vc::put_u8(out, 0);
    vc::encode(v, out)
}

fn get_cell(r: &mut Reader) -> Option<Value> {
    match r.u8()? {
        0 => vc::decode(r),
        1 => {
            let table = r.u32()?;
            let table_no = r.u32()?;
            let name = r.str()?;
            let n = r.u32()? as usize;
            let mut key = Vec::with_capacity(n.min(64));
            for _ in 0..n {
                key.push(get_cell(r)?);
            }
            Some(Value::Library(
                LibraryKind::RecordId,
                Rc::new(RefCell::new(LibraryValue::RecordId {
                    table,
                    table_no,
                    name,
                    key,
                })),
            ))
        }
        2 => Some(Value::Library(
            LibraryKind::RecordId,
            Rc::new(RefCell::new(LibraryValue::Empty)),
        )),
        _ => None,
    }
}

fn row_of(bytes: &[u8]) -> Option<Vec<Value>> {
    let mut r = Reader::new(bytes);
    let n = r.u32()? as usize;
    let mut row = Vec::with_capacity(n.min(4096));
    for _ in 0..n {
        row.push(get_cell(&mut r)?);
    }
    Some(row)
}

fn put_filter(e: &FilterExpr, out: &mut Vec<u8>) -> bool {
    use vc::{put_str, put_u8, put_u32};
    // A filter value is written as a cell of a row: a RecordID field is filtered on a RecordId.
    let one = |tag: u8, v: &Value, out: &mut Vec<u8>| {
        put_u8(out, tag);
        put_cell(v, out)
    };
    match e {
        FilterExpr::Range(a, b) => {
            put_u8(out, 0);
            for end in [a, b] {
                match end {
                    None => put_u8(out, 0),
                    Some(v) => {
                        put_u8(out, 1);
                        if !put_cell(v, out) {
                            return false;
                        }
                    }
                }
            }
            true
        }
        FilterExpr::Eq(v) => one(1, v, out),
        FilterExpr::RangeSame(v) => one(2, v, out),
        FilterExpr::Ne(v) => one(3, v, out),
        FilterExpr::Lt(v) => one(4, v, out),
        FilterExpr::Le(v) => one(5, v, out),
        FilterExpr::Gt(v) => one(6, v, out),
        FilterExpr::Ge(v) => one(7, v, out),
        FilterExpr::Or(xs) | FilterExpr::And(xs) => {
            put_u8(out, if matches!(e, FilterExpr::Or(_)) { 8 } else { 9 });
            put_u32(out, xs.len() as u32);
            xs.iter().all(|x| put_filter(x, out))
        }
        FilterExpr::Like(s, ci) | FilterExpr::NotLike(s, ci) => {
            put_u8(out, if matches!(e, FilterExpr::Like(..)) { 10 } else { 11 });
            put_str(out, s);
            put_u8(out, u8::from(*ci));
            true
        }
        FilterExpr::AtSignAlone(none) => {
            put_u8(out, 12);
            put_u8(out, u8::from(*none));
            true
        }
        FilterExpr::EmptyViewValue => {
            put_u8(out, 13);
            true
        }
        FilterExpr::NumCode(inner) => {
            put_u8(out, 14);
            put_filter(inner, out)
        }
    }
}

fn get_filter(r: &mut Reader) -> Option<FilterExpr> {
    Some(match r.u8()? {
        0 => {
            let mut ends = [None, None];
            for end in &mut ends {
                if r.u8()? == 1 {
                    *end = Some(get_cell(r)?);
                }
            }
            let [a, b] = ends;
            FilterExpr::Range(a, b)
        }
        1 => FilterExpr::Eq(get_cell(r)?),
        2 => FilterExpr::RangeSame(get_cell(r)?),
        3 => FilterExpr::Ne(get_cell(r)?),
        4 => FilterExpr::Lt(get_cell(r)?),
        5 => FilterExpr::Le(get_cell(r)?),
        6 => FilterExpr::Gt(get_cell(r)?),
        7 => FilterExpr::Ge(get_cell(r)?),
        t @ (8 | 9) => {
            let n = r.u32()? as usize;
            let mut xs = Vec::with_capacity(n.min(1024));
            for _ in 0..n {
                xs.push(get_filter(r)?);
            }
            if t == 8 {
                FilterExpr::Or(xs)
            } else {
                FilterExpr::And(xs)
            }
        }
        t @ (10 | 11) => {
            let s = r.str()?;
            let ci = r.u8()? != 0;
            if t == 10 {
                FilterExpr::Like(s, ci)
            } else {
                FilterExpr::NotLike(s, ci)
            }
        }
        12 => FilterExpr::AtSignAlone(r.u8()? != 0),
        13 => FilterExpr::EmptyViewValue,
        14 => FilterExpr::NumCode(Box::new(get_filter(r)?)),
        _ => return None,
    })
}

fn filters_bytes(fs: &[FieldFilter]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    vc::put_u32(&mut out, fs.len() as u32);
    for f in fs {
        vc::put_u64(&mut out, f.field as u64);
        out.extend_from_slice(&f.group.to_le_bytes());
        if !put_filter(&f.expr, &mut out) {
            return None;
        }
    }
    Some(out)
}

fn filters_of(bytes: &[u8]) -> Option<Vec<FieldFilter>> {
    let mut r = Reader::new(bytes);
    let n = r.u32()? as usize;
    let mut fs = Vec::with_capacity(n.min(1024));
    for _ in 0..n {
        let field = r.u64()? as usize;
        let g = r.u32()?;
        let expr = get_filter(&mut r)?;
        fs.push(FieldFilter {
            field,
            expr,
            group: g as i32,
        });
    }
    Some(fs)
}

/// Writes the values reachable from groups of roots (see the module). A group is the globals of one single instance.
/// A handle reached twice within one group (two globals sharing a list, a token inside another global's JSON tree)
/// is written once ([`CarryVal::Def`]) and named at its other places ([`CarryVal::Ref`]), so the copies share it as
/// the originals do; decode the members of a group in order with one [`Ids`]. A handle two groups reach, a record two
/// places hold (a FieldRef into a carried RecordRef's buffer) and a temporary table two records share are refused.
pub(crate) struct Enc<'s> {
    session: &'s Session,
    /// Handles reached more than once within one group.
    shared: HashSet<usize>,
    /// Handles reached from more than one group.
    cross: HashSet<usize>,
    /// JSON handles reached as a node of a tree.
    in_tree: HashSet<usize>,
    shared_temps: HashSet<u64>,
    /// The numbers given so far in the current group.
    ids: RefCell<HashMap<usize, u32>>,
    /// The committed row version when the session began to track the single instances.
    bound: i64,
    /// Each bound instance whose binding reaches the next session, with its place ([`binding_ranks`]).
    bind_ranks: HashMap<usize, u32>,
}

/// What [`count`] notes while it walks the roots of one group.
#[derive(Default)]
struct Counts {
    seen: HashMap<usize, u32>,
    temps: HashMap<u64, u32>,
    in_tree: HashSet<usize>,
    /// The walk stopped at [`DEPTH`] somewhere: a handle below it was not seen.
    deep: bool,
}

fn ptr<T>(rc: &Rc<T>) -> usize {
    Rc::as_ptr(rc) as *const () as usize
}

impl<'s> Enc<'s> {
    pub(crate) fn new(session: &'s Session, groups: &[Vec<Value>], bound: i64) -> Enc<'s> {
        let mut reach: HashMap<usize, (u32, u32)> = HashMap::new();
        let mut temps: HashMap<u64, u32> = HashMap::new();
        let mut in_tree = HashSet::new();
        for g in groups {
            let mut c = Counts::default();
            for v in g {
                count(v, &mut c, 0);
            }
            for (p, n) in c.seen {
                let e = reach.entry(p).or_default();
                e.0 = e.0.max(n);
                e.1 += 1;
            }
            for (t, n) in c.temps {
                *temps.entry(t).or_default() += n;
            }
            in_tree.extend(c.in_tree);
        }
        Enc {
            session,
            shared: reach
                .iter()
                .filter(|(_, (n, g))| *n > 1 && *g == 1)
                .map(|(p, _)| *p)
                .collect(),
            cross: reach.iter().filter(|(_, (_, g))| *g > 1).map(|(p, _)| *p).collect(),
            in_tree,
            shared_temps: temps.into_iter().filter(|&(_, n)| n > 1).map(|(p, _)| p).collect(),
            ids: RefCell::default(),
            bound,
            bind_ranks: binding_ranks(session).ranks,
        }
    }

    /// Starts a group: its numbers start again from 0.
    pub(crate) fn begin_group(&self) {
        self.ids.borrow_mut().clear();
    }

    /// Refuses a handle another place also holds (records: their sharing is not carried).
    fn unshared<T>(&self, rc: &Rc<T>, v: &Value) -> Result<(), String> {
        let p = ptr(rc);
        if self.cross.contains(&p) {
            return Err(format!(
                "{} that a variable of another SingleInstance codeunit shares",
                v.describe()
            ));
        }
        if self.shared.contains(&p) {
            return Err(format!("{} that another carried variable shares", v.describe()));
        }
        Ok(())
    }

    /// Where a handle stands in the group, a new number for the first place of a shared one; `Err` for a handle two
    /// groups reach.
    fn place<T>(&self, rc: &Rc<T>, v: &Value) -> Result<Place, String> {
        let p = ptr(rc);
        if self.cross.contains(&p) {
            return Err(format!(
                "{} that a variable of another SingleInstance codeunit shares",
                v.describe()
            ));
        }
        if !self.shared.contains(&p) {
            return Ok(Place::Alone);
        }
        let mut ids = self.ids.borrow_mut();
        if let Some(&id) = ids.get(&p) {
            return Ok(Place::Again(id));
        }
        let id = u32::try_from(ids.len()).map_err(|_| v.describe())?;
        ids.insert(p, id);
        Ok(Place::First(id))
    }

    /// Writes a handle with `f`, or names it when the group holds it at more than one place.
    fn handle<T>(
        &self,
        rc: &Rc<T>,
        v: &Value,
        f: impl FnOnce() -> Result<CarryVal, String>,
    ) -> Result<CarryVal, String> {
        Ok(match self.place(rc, v)? {
            Place::Alone => f()?,
            Place::Again(id) => CarryVal::Ref(id),
            Place::First(id) => CarryVal::Def(id, Box::new(f()?)),
        })
    }

    pub(crate) fn enc(&self, v: &Value) -> Result<CarryVal, String> {
        self.val(v, 0)
    }

    fn val(&self, v: &Value, depth: usize) -> Result<CarryVal, String> {
        if depth >= DEPTH {
            return Err(v.describe());
        }
        let busy = |_| v.describe();
        Ok(match v {
            Value::Codeunit(inst) => {
                let i = inst.try_borrow().map_err(busy)?;
                if i.single {
                    return Ok(CarryVal::Single(i.object));
                }
                drop(i);
                self.handle(inst, v, || Ok(CarryVal::Codeunit(Box::new(self.inst(inst, depth)?))))?
            }
            Value::DotNet(obj) => match &obj.kind {
                DotNetKind::Enum {
                    clr_type,
                    name,
                    ordinal,
                } => CarryVal::DotNetEnum {
                    clr_type: clr_type.clone(),
                    name: name.clone(),
                    ordinal: *ordinal,
                },
                DotNetKind::Internal { type_name, state } => self.handle(obj, v, || {
                    let state = state.try_borrow().map_err(busy)?;
                    Ok(CarryVal::DotNetInternal {
                        type_name: type_name.clone(),
                        state: state.iter().map(|x| self.val(x, depth + 1)).collect::<Result<_, _>>()?,
                    })
                })?,
                _ => return Err(v.describe()),
            },
            Value::List(l) => self.handle(l, v, || {
                let l = l.try_borrow().map_err(busy)?;
                Ok(CarryVal::List {
                    items: l.iter().map(|x| self.val(x, depth + 1)).collect::<Result<_, _>>()?,
                    version: l.version,
                    read_only: l.read_only,
                })
            })?,
            Value::Dictionary(d) => self.handle(d, v, || {
                let d = d.try_borrow().map_err(busy)?;
                Ok(CarryVal::Dictionary(
                    d.iter()
                        .map(|(k, x)| Ok((self.val(k, depth + 1)?, self.val(x, depth + 1)?)))
                        .collect::<Result<_, String>>()?,
                ))
            })?,
            Value::Array(a) => self.handle(a, v, || {
                let a = a.try_borrow().map_err(busy)?;
                Ok(CarryVal::Array(
                    a.iter().map(|x| self.val(x, depth + 1)).collect::<Result<_, _>>()?,
                ))
            })?,
            Value::Record(rc) => {
                self.unshared(rc, v)?;
                CarryVal::Record(Box::new(self.rec(rc, depth)?))
            }
            Value::Library(kind, lv) => match self.handle(lv, v, || {
                Ok(CarryVal::Library(kind.clone(), self.lib(kind, lv, v, depth)?))
            })? {
                CarryVal::Ref(id) => CarryVal::Library(kind.clone(), CarryLib::Ref(id)),
                c => c,
            },
            _ => {
                let mut out = Vec::new();
                if !vc::encode(v, &mut out) {
                    return Err(v.describe());
                }
                CarryVal::Plain(out)
            }
        })
    }

    fn lib(
        &self,
        kind: &LibraryKind,
        lv: &Rc<RefCell<LibraryValue>>,
        v: &Value,
        depth: usize,
    ) -> Result<CarryLib, String> {
        let me = ptr(lv);
        let lv = lv.try_borrow().map_err(|_| v.describe())?;
        Ok(match (kind, &*lv) {
            (LibraryKind::RecordRef | LibraryKind::TextBuilder | LibraryKind::RecordId, LibraryValue::Empty) => {
                CarryLib::Empty
            }
            (k, LibraryValue::Empty) if json_kind(k) => CarryLib::Empty,
            (k, LibraryValue::Json(cell)) if json_kind(k) => {
                // A node of a tree: carried only with the tree, which a global of the same group holds.
                if cell.parent.as_ref().is_some_and(|p| p.upgrade().is_some()) && !self.in_tree.contains(&me) {
                    return Err(format!("{} inside a JSON tree no carried variable holds", v.describe()));
                }
                CarryLib::Json(Box::new(self.json(cell, v, depth)?))
            }
            (LibraryKind::TextBuilder, LibraryValue::Text(s)) => CarryLib::Text(s.clone()),
            (LibraryKind::TextBuilder, LibraryValue::TextCap(s, n)) => CarryLib::TextCap(s.clone(), *n),
            (LibraryKind::RecordRef, LibraryValue::RecordRef(rc)) => {
                self.unshared(rc, v)?;
                CarryLib::RecordRef(Box::new(self.rec(rc, depth)?))
            }
            (
                LibraryKind::RecordId,
                LibraryValue::RecordId {
                    table,
                    table_no,
                    name,
                    key,
                },
            ) => CarryLib::RecordId {
                table: *table,
                table_no: *table_no,
                name: name.clone(),
                key: row_bytes(key).map_err(|_| v.describe())?,
            },
            _ => return Err(v.describe()),
        })
    }

    fn json(&self, cell: &al_builtins::json::JsonCell, v: &Value, depth: usize) -> Result<CarryJson, String> {
        use al_builtins::json::{JsonNode, Scalar};
        if depth >= 64 {
            return Err(v.describe());
        }
        let child = |h: &al_builtins::json::Handle| -> Result<CarryJson, String> {
            let def = match self.place(h, v)? {
                Place::Again(id) => {
                    return Ok(CarryJson {
                        node: CarryJsonNode::Ref(id),
                        unpinned: None,
                        def: None,
                    });
                }
                Place::First(id) => Some(id),
                Place::Alone => None,
            };
            let lv = h.try_borrow().map_err(|_| v.describe())?;
            match &*lv {
                LibraryValue::Json(c) => Ok(CarryJson {
                    def,
                    ..self.json(c, v, depth + 1)?
                }),
                _ => Err(v.describe()),
            }
        };
        let node = match &cell.node {
            JsonNode::Object(kv) => CarryJsonNode::Object(
                kv.iter()
                    .map(|(k, h)| Ok((k.clone(), child(h)?)))
                    .collect::<Result<_, String>>()?,
            ),
            JsonNode::Array(xs) => CarryJsonNode::Array(xs.iter().map(child).collect::<Result<_, _>>()?),
            JsonNode::Scalar(Scalar::Null) => CarryJsonNode::Null,
            JsonNode::Scalar(Scalar::Undefined) => CarryJsonNode::Undefined,
            JsonNode::Scalar(Scalar::Raw(r)) => CarryJsonNode::Raw(r.clone()),
            JsonNode::Scalar(Scalar::Val(x)) => {
                let mut out = Vec::new();
                if !vc::encode(x, &mut out) {
                    return Err(v.describe());
                }
                CarryJsonNode::Val(out)
            }
        };
        Ok(CarryJson {
            node,
            unpinned: cell.unpinned,
            def: None,
        })
    }

    fn inst(&self, inst: &Rc<RefCell<Instance>>, depth: usize) -> Result<CarryInst, String> {
        let i = inst
            .try_borrow()
            .map_err(|_| "a codeunit instance in use".to_string())?;
        let Instance {
            object,
            globals,
            initialized,
            rerun,
            base,
            exts,
            xp_text,
            born,
            single: _,
        } = &*i;
        if base.is_some() || !exts.is_empty() {
            return Err("an instance of a page or report extension, or of a report with extensions".into());
        }
        Ok(CarryInst {
            object: *object,
            initialized: *initialized,
            rerun: *rerun,
            born: born.0,
            xp_text: xp_text.clone(),
            globals: globals
                .iter()
                .map(|g| self.val(g, depth + 1))
                .collect::<Result<_, _>>()?,
            bound: self.bind_ranks.get(&ptr(inst)).copied(),
        })
    }

    fn table_globals(&self, g: &TableGlobals, depth: usize) -> Result<Vec<(u32, CarryInst)>, String> {
        g.instances()
            .iter()
            .map(|(o, i)| Ok((*o, self.inst(i, depth)?)))
            .collect()
    }

    fn rec(&self, rc: &Rc<RefCell<RecordState>>, depth: usize) -> Result<CarryRec, String> {
        let what = || Value::Record(rc.clone()).describe();
        let t = self.session.table(rc).map_err(|_| what())?;
        let r = rc.try_borrow().map_err(|_| what())?;
        let RecordState {
            table,
            temporary,
            temp_id,
            company,
            crm,
            fields,
            filters,
            filter_group,
            marks,
            marked_only,
            read_isolation,
            security_filtering,
            key,
            adhoc,
            ascending,
            field_descending,
            position,
            auto_calc,
            load,
            touched,
            assigned,
            inserted,
            dirty_base,
            is_dirty_base,
            last_modify,
            failed_insert,
            row_origin,
            globals,
            unpinned_buffer,
            run_page_link,
        } = &*r;
        let refuse = |why: &str| Err(format!("{} ({why})", what()));
        let unrow = |cell: String| {
            format!(
                "{} (a value in its rows or buffer that cannot be carried: {cell})",
                what()
            )
        };
        if *crm {
            return refuse("a CRM table, whose connection is the session's");
        }
        if run_page_link.is_some() {
            return refuse("the Rec of a page an action opened");
        }
        let vpos = t.fields.iter().position(|f| f.id == 2_000_000_005);
        let version_of = |row: &[Value]| match vpos.and_then(|i| row.get(i)) {
            Some(Value::BigInteger(n)) => *n,
            _ => 0,
        };
        let mut max_version = 0;
        let mut temp_rows = None;
        // A buffer of a row BC rolled back with the test codeunit (the variable's own write, or a row version the
        // earlier session handed out, which the next session hands out again): its fields go, what it knew of the row
        // does not (`records::ROLLED_BACK_ROW`; pinned by record/crud/RcBTestField.al:449-458, :474-478).
        let mut rolled_back = false;
        if *temporary {
            let Ok(auto) = self.session.auto_increment_field(&t) else {
                return refuse("a temporary table with an AutoIncrement field, whose counter is the session's");
            };
            if let Some(id) = temp_id {
                if self.shared_temps.contains(&id.id()) {
                    return refuse("a temporary table another carried record shares");
                }
                if al_data::code_storage::pk_storage(&t).is_some() {
                    return refuse("a temporary table whose Code key is stored as a number");
                }
                let tables = match self.session.temps.get(&id.id()) {
                    Some(store) => store
                        .export_current()
                        .ok_or_else(|| unrow("a table with a backing or virtual tables".into()))?,
                    None => Vec::new(),
                };
                let mut out = Vec::new();
                for x in tables {
                    let mut rows = Vec::with_capacity(x.rows.len());
                    for (k, row) in &x.rows {
                        rows.push((row_bytes(k).map_err(unrow)?, row_bytes(row).map_err(unrow)?));
                    }
                    out.push((x.company, x.object, rows));
                }
                temp_rows = Some(out);
            }
            // An AutoIncrement counter is the session's (`autoinc.rs`): carried only an empty table whose counter the
            // session never moved, which starts the next session as it started this one.
            let empty = temp_rows.as_ref().is_none_or(|ts| ts.iter().all(|x| x.2.is_empty()));
            let used = self.session.autoinc.keys().any(|(_, o)| *o == t.object);
            if auto.is_some() && (!empty || used) {
                return refuse("a temporary table with an AutoIncrement field, whose counter is the session's");
            }
        } else {
            max_version = version_of(fields)
                .max(row_origin.map_or(0, |o| o.version))
                .max(failed_insert.unwrap_or(0))
                .max(last_modify.as_deref().map_or(0, version_of));
            rolled_back = row_origin.is_some_and(|o| o.own_write) || max_version > self.bound;
            if rolled_back {
                if load.partial {
                    return refuse("a partly loaded buffer of a row BC rolled back with an earlier test codeunit");
                }
                // A buffer carried in already marked goes on with the mark.
                if let Some(why) = unpinned_buffer.filter(|w| *w != crate::records::ROLLED_BACK_ROW) {
                    return Err(format!(
                        "{} (a buffer of a row BC rolled back with an earlier test codeunit, and not pinned: {why})",
                        what()
                    ));
                }
                max_version = 0;
            }
        }
        let LoadState {
            request,
            buffer,
            partial,
        } = load;
        let load_buffer = match buffer {
            None => None,
            Some(Buffer {
                have,
                unsure,
                row,
                cleared,
                assigned,
                missing_loaded,
                fetched,
                init,
            }) => Some(CarryBuffer {
                have: have.clone(),
                unsure: unsure.clone(),
                row: row.clone(),
                cleared: *cleared,
                assigned: assigned.clone(),
                missing_loaded: *missing_loaded,
                fetched: match &fetched.0 {
                    None => None,
                    Some(row) => Some(row_bytes(row).map_err(unrow)?),
                },
                init: *init,
            }),
        };
        let opt_row = |r: &Option<Vec<Value>>| -> Result<Option<Vec<u8>>, String> {
            r.as_deref().map(|r| row_bytes(r).map_err(unrow)).transpose()
        };
        let xrec = if globals.is_unset() {
            Vec::new()
        } else {
            self.table_globals(&globals.xrec(), depth)?
        };
        Ok(CarryRec {
            table: *table,
            temporary: *temporary,
            temp_rows,
            company: company.clone(),
            fields: row_bytes(fields).map_err(unrow)?,
            filters: filters_bytes(filters).ok_or_else(|| unrow("a filter".into()))?,
            filter_group: *filter_group,
            marks: marks
                .iter()
                .map(|m| row_bytes(m).map_err(unrow))
                .collect::<Result<_, _>>()?,
            marked_only: *marked_only,
            read_isolation: *read_isolation,
            security_filtering: *security_filtering,
            key: *key,
            adhoc: adhoc.clone(),
            ascending: *ascending,
            field_descending: *field_descending,
            position: opt_row(position)?,
            auto_calc: auto_calc.clone(),
            load_request: request.clone(),
            load_buffer,
            load_partial: *partial,
            touched: *touched,
            assigned: *assigned,
            inserted: *inserted,
            dirty_base: if rolled_back { None } else { opt_row(dirty_base)? },
            is_dirty_base: match is_dirty_base {
                _ if rolled_back => CarryDirty::Unknown(crate::records::ROLLED_BACK_ROW),
                DirtyBase::Unknown(s) => CarryDirty::Unknown(s),
                DirtyBase::Fresh => CarryDirty::Fresh,
                DirtyBase::Synced => CarryDirty::Synced,
                DirtyBase::Row {
                    base,
                    alt,
                    dirty_only,
                    stale_store,
                } => CarryDirty::Row {
                    base: row_bytes(base).map_err(unrow)?,
                    alt: opt_row(alt)?,
                    dirty_only: *dirty_only,
                    stale_store: *stale_store,
                },
            },
            last_modify: if rolled_back { None } else { opt_row(last_modify)? },
            failed_insert: if rolled_back { None } else { *failed_insert },
            row_origin: row_origin.filter(|_| !rolled_back).map(|o| RowOrigin {
                commits: RowOrigin::EARLIER_SESSION,
                ..o
            }),
            globals: self.table_globals(globals, depth)?,
            xrec_globals: xrec,
            unpinned_buffer: if rolled_back {
                Some(crate::records::ROLLED_BACK_ROW)
            } else {
                *unpinned_buffer
            },
            max_version,
        })
    }
}

/// Counts how often each handle (and each temporary table) is reached from the roots; a handle is entered once.
fn count(v: &Value, c: &mut Counts, depth: usize) {
    if depth >= DEPTH {
        c.deep = true;
        return;
    }
    let mut first = |p: usize| {
        let n = c.seen.entry(p).or_default();
        *n += 1;
        *n == 1
    };
    match v {
        Value::Codeunit(inst) => {
            if inst.try_borrow().is_ok_and(|i| !i.single) && first(ptr(inst)) {
                if let Ok(i) = inst.try_borrow() {
                    for g in &i.globals {
                        count(g, c, depth + 1);
                    }
                }
            }
        }
        Value::DotNet(obj) => {
            if let DotNetKind::Internal { state, .. } = &obj.kind
                && first(ptr(obj))
                && let Ok(s) = state.try_borrow()
            {
                for x in s.iter() {
                    count(x, c, depth + 1);
                }
            }
        }
        Value::List(l) if first(ptr(l)) => {
            if let Ok(l) = l.try_borrow() {
                for x in l.iter() {
                    count(x, c, depth + 1);
                }
            }
        }
        Value::Dictionary(d) if first(ptr(d)) => {
            if let Ok(d) = d.try_borrow() {
                for (k, x) in d.iter() {
                    count(k, c, depth + 1);
                    count(x, c, depth + 1);
                }
            }
        }
        Value::Array(a) if first(ptr(a)) => {
            if let Ok(a) = a.try_borrow() {
                for x in a.iter() {
                    count(x, c, depth + 1);
                }
            }
        }
        Value::Record(rc) => count_rec(rc, c, depth),
        Value::Library(_, lv) if first(ptr(lv)) => {
            if let Ok(lv) = lv.try_borrow() {
                match &*lv {
                    LibraryValue::RecordRef(rc)
                    | LibraryValue::FieldRef(rc, _)
                    | LibraryValue::KeyFieldRef(rc, _)
                    | LibraryValue::KeyRef(rc, _)
                    | LibraryValue::UserNameFieldRef { rec: rc, .. } => count_rec(rc, c, depth),
                    LibraryValue::Json(cell) => count_json(cell, c, depth),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn count_json(cell: &al_builtins::json::JsonCell, c: &mut Counts, depth: usize) {
    use al_builtins::json::JsonNode;
    if depth >= 64 {
        return;
    }
    let children: Vec<&al_builtins::json::Handle> = match &cell.node {
        JsonNode::Object(kv) => kv.iter().map(|(_, h)| h).collect(),
        JsonNode::Array(xs) => xs.iter().collect(),
        JsonNode::Scalar(_) => Vec::new(),
    };
    for h in children {
        c.in_tree.insert(ptr(h));
        let n = c.seen.entry(ptr(h)).or_default();
        *n += 1;
        if *n == 1
            && let Ok(lv) = h.try_borrow()
            && let LibraryValue::Json(cell) = &*lv
        {
            count_json(cell, c, depth + 1);
        }
    }
}

fn count_rec(rc: &Rc<RefCell<RecordState>>, c: &mut Counts, depth: usize) {
    let n = c.seen.entry(ptr(rc)).or_default();
    *n += 1;
    if *n > 1 {
        return;
    }
    let Ok(r) = rc.try_borrow() else { return };
    if let Some(id) = &r.temp_id {
        *c.temps.entry(id.id()).or_default() += 1;
    }
    let mut insts = r.globals.instances();
    if !r.globals.is_unset() {
        insts.extend(r.globals.xrec().instances());
    }
    for (_, i) in insts {
        count(&Value::Codeunit(i), c, depth + 1);
    }
}

/// The manual bindings that reach the next test codeunit's session. BC keeps, across the test codeunit boundary, the
/// binding of a SingleInstance subscriber and of an ordinary subscriber held in a SingleInstance codeunit's global, and
/// drops that of an ordinary subscriber the test codeunit held (TestEventSingleInstanceBindingBoundary.al:91, :98-99):
/// the binding lives as long as its instance, and only the single instances and what their globals hold outlive the
/// test codeunit.
#[derive(Default)]
pub(crate) struct BindingRanks {
    /// Each bound instance a single instance's globals hold, with its place among the bindings that reach the next
    /// session (oldest first, counted from 0).
    pub ranks: HashMap<usize, u32>,
    /// The single instances that are bound (object index), with their place.
    pub singles: Vec<(usize, u32)>,
    /// The walk of the single instances' globals stopped at [`DEPTH`] while an ordinary instance was bound: whether a
    /// global holds it is not known.
    pub deep: bool,
}

pub(crate) fn binding_ranks(session: &Session) -> BindingRanks {
    let mut out = BindingRanks::default();
    let live: Vec<Rc<RefCell<Instance>>> = session
        .events
        .bindings
        .iter()
        .filter_map(|b| b.instance.upgrade())
        .collect();
    if live.is_empty() {
        return out;
    }
    let mut c = Counts::default();
    if live.iter().any(|i| i.try_borrow().is_ok_and(|i| !i.single)) {
        for inst in session.singletons.values() {
            if let Ok(i) = inst.try_borrow() {
                for g in &i.globals {
                    count(g, &mut c, 0);
                }
            }
        }
        out.deep = c.deep;
    }
    let mut rank = 0;
    for inst in &live {
        let single = inst.try_borrow().map_or(None, |i| i.single.then_some(i.object as usize));
        if let Some(object) = single {
            if session.singletons.get(&object).is_some_and(|s| Rc::ptr_eq(s, inst)) {
                out.singles.push((object, rank));
                rank += 1;
            }
        } else if c.seen.contains_key(&ptr(inst)) && !out.ranks.contains_key(&ptr(inst)) {
            out.ranks.insert(ptr(inst), rank);
            rank += 1;
        }
    }
    out
}

/// The places of the bound instances carried value `c` holds ([`CarryInst::bound`]).
pub(crate) fn bound_in(c: &CarryVal, out: &mut Vec<u32>) {
    let inst = |i: &CarryInst, out: &mut Vec<u32>| {
        out.extend(i.bound);
        i.globals.iter().for_each(|g| bound_in(g, out));
    };
    match c {
        CarryVal::Codeunit(i) => inst(i, out),
        CarryVal::DotNetInternal { state: xs, .. } | CarryVal::List { items: xs, .. } | CarryVal::Array(xs) => {
            xs.iter().for_each(|x| bound_in(x, out))
        }
        CarryVal::Dictionary(kv) => kv.iter().for_each(|(k, x)| {
            bound_in(k, out);
            bound_in(x, out);
        }),
        CarryVal::Record(r) | CarryVal::Library(_, CarryLib::RecordRef(r)) => {
            r.globals.iter().chain(&r.xrec_globals).for_each(|(_, i)| inst(i, out))
        }
        CarryVal::Def(_, x) => bound_in(x, out),
        _ => {}
    }
}

/// The handles of a group decoded so far, by their number ([`CarryVal::Def`]).
pub(crate) type Ids = HashMap<u32, Value>;

/// The handle of a carried JSON node, its children pointing at it as their parent.
fn json_handle(
    j: &CarryJson,
    parent: Option<&al_builtins::json::Handle>,
    ids: &mut Ids,
) -> Option<al_builtins::json::Handle> {
    use al_builtins::json::{JsonCell, JsonNode, Scalar};
    if let CarryJsonNode::Ref(id) = &j.node {
        let Some(Value::Library(_, h)) = ids.get(id) else {
            return None;
        };
        match &mut *h.try_borrow_mut().ok()? {
            LibraryValue::Json(c) => c.parent = parent.map(Rc::downgrade),
            _ => return None,
        }
        return Some(h.clone());
    }
    let h: al_builtins::json::Handle = Rc::new(RefCell::new(LibraryValue::Json(JsonCell {
        node: JsonNode::Scalar(Scalar::Null),
        parent: parent.map(Rc::downgrade),
        unpinned: j.unpinned,
    })));
    if let Some(id) = j.def {
        ids.insert(id, Value::Library(LibraryKind::JsonToken, h.clone()));
    }
    let node = match &j.node {
        CarryJsonNode::Object(kv) => JsonNode::Object(
            kv.iter()
                .map(|(k, c)| Some((k.clone(), json_handle(c, Some(&h), ids)?)))
                .collect::<Option<_>>()?,
        ),
        CarryJsonNode::Array(xs) => JsonNode::Array(
            xs.iter()
                .map(|c| json_handle(c, Some(&h), ids))
                .collect::<Option<_>>()?,
        ),
        CarryJsonNode::Null => JsonNode::Scalar(Scalar::Null),
        CarryJsonNode::Undefined => JsonNode::Scalar(Scalar::Undefined),
        CarryJsonNode::Raw(r) => JsonNode::Scalar(Scalar::Raw(r.clone())),
        CarryJsonNode::Val(b) => JsonNode::Scalar(Scalar::Val(vc::decode(&mut Reader::new(b))?)),
        CarryJsonNode::Ref(_) => unreachable!("handled above"),
    };
    if let LibraryValue::Json(c) = &mut *h.borrow_mut() {
        c.node = node;
    }
    Some(h)
}

/// The numbers a carried value gives ([`CarryVal::Def`], [`CarryJson::def`]) and names ([`CarryVal::Ref`],
/// [`CarryLib::Ref`], [`CarryJsonNode::Ref`]).
#[derive(Default)]
pub(crate) struct IdUse {
    pub defs: HashSet<u32>,
    pub refs: HashSet<u32>,
}

impl IdUse {
    pub(crate) fn of(c: &CarryVal) -> IdUse {
        let mut u = IdUse::default();
        u.val(c);
        u
    }

    pub(crate) fn any(&self) -> bool {
        !self.defs.is_empty() || !self.refs.is_empty()
    }

    fn val(&mut self, c: &CarryVal) {
        match c {
            CarryVal::Def(id, x) => {
                self.defs.insert(*id);
                self.val(x);
            }
            CarryVal::Ref(id) => {
                self.refs.insert(*id);
            }
            CarryVal::Codeunit(i) => self.inst(i),
            CarryVal::DotNetInternal { state: xs, .. } | CarryVal::List { items: xs, .. } | CarryVal::Array(xs) => {
                xs.iter().for_each(|x| self.val(x));
            }
            CarryVal::Dictionary(pairs) => pairs.iter().for_each(|(k, x)| {
                self.val(k);
                self.val(x);
            }),
            CarryVal::Record(r) | CarryVal::Library(_, CarryLib::RecordRef(r)) => {
                r.globals.iter().chain(&r.xrec_globals).for_each(|(_, i)| self.inst(i));
            }
            CarryVal::Library(_, CarryLib::Ref(id)) => {
                self.refs.insert(*id);
            }
            CarryVal::Library(_, CarryLib::Json(j)) => self.json(j),
            CarryVal::Library(..) | CarryVal::Plain(_) | CarryVal::Single(_) | CarryVal::DotNetEnum { .. } => {}
        }
    }

    fn inst(&mut self, i: &CarryInst) {
        i.globals.iter().for_each(|g| self.val(g));
    }

    fn json(&mut self, j: &CarryJson) {
        if let Some(id) = j.def {
            self.defs.insert(id);
        }
        match &j.node {
            CarryJsonNode::Ref(id) => {
                self.refs.insert(*id);
            }
            CarryJsonNode::Object(kv) => kv.iter().for_each(|(_, c)| self.json(c)),
            CarryJsonNode::Array(xs) => xs.iter().for_each(|c| self.json(c)),
            _ => {}
        }
    }
}

impl Session {
    /// The value `c` stands for in this session (see the module); the reason when it has none here. `bound`: this
    /// session's committed row version when it began to track the single instances; `ids`: the handles of the group
    /// decoded so far (decode the members of a group in order).
    pub(crate) fn carry_decode(&mut self, c: &CarryVal, bound: i64, ids: &mut Ids) -> Result<Value, String> {
        let bad = || "a carried value that does not read back".to_string();
        Ok(match c {
            CarryVal::Def(id, x) => {
                let v = self.carry_decode(x, bound, ids)?;
                ids.insert(*id, v.clone());
                v
            }
            CarryVal::Ref(id) => ids.get(id).cloned().ok_or_else(bad)?,
            CarryVal::Plain(b) => vc::decode(&mut Reader::new(b)).ok_or_else(bad)?,
            CarryVal::Single(object) => Value::Codeunit(self.codeunit_instance(*object as usize)),
            CarryVal::Codeunit(i) => Value::Codeunit(self.carry_inst(i, bound, ids)?),
            CarryVal::DotNetEnum {
                clr_type,
                name,
                ordinal,
            } => Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Enum {
                clr_type: clr_type.clone(),
                name: name.clone(),
                ordinal: *ordinal,
            }))),
            CarryVal::DotNetInternal { type_name, state } => {
                let state = state
                    .iter()
                    .map(|x| self.carry_decode(x, bound, ids))
                    .collect::<Result<Vec<_>, _>>()?;
                Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
                    type_name: type_name.clone(),
                    state: RefCell::new(state),
                })))
            }
            CarryVal::List {
                items,
                version,
                read_only,
            } => {
                let items = items
                    .iter()
                    .map(|x| self.carry_decode(x, bound, ids))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut l = ListVec::from(items);
                l.version = *version;
                l.read_only = *read_only;
                Value::List(Rc::new(RefCell::new(l)))
            }
            CarryVal::Dictionary(pairs) => {
                let mut out = Vec::with_capacity(pairs.len());
                for (k, x) in pairs {
                    out.push((self.carry_decode(k, bound, ids)?, self.carry_decode(x, bound, ids)?));
                }
                Value::Dictionary(Rc::new(RefCell::new(out)))
            }
            CarryVal::Array(xs) => {
                let xs = xs
                    .iter()
                    .map(|x| self.carry_decode(x, bound, ids))
                    .collect::<Result<Vec<_>, _>>()?;
                Value::Array(Rc::new(RefCell::new(xs)))
            }
            CarryVal::Record(r) => Value::Record(self.carry_rec(r, bound, ids)?),
            CarryVal::Library(kind, lib) => {
                match lib {
                    CarryLib::Json(j) => {
                        return Ok(Value::Library(kind.clone(), json_handle(j, None, ids).ok_or_else(bad)?));
                    }
                    CarryLib::Ref(id) => {
                        let Some(Value::Library(_, h)) = ids.get(id) else {
                            return Err(bad());
                        };
                        return Ok(Value::Library(kind.clone(), h.clone()));
                    }
                    _ => {}
                }
                let lv = match lib {
                    CarryLib::Empty => LibraryValue::Empty,
                    CarryLib::Json(_) | CarryLib::Ref(_) => unreachable!("handled above"),
                    CarryLib::Text(s) => LibraryValue::Text(s.clone()),
                    CarryLib::TextCap(s, n) => LibraryValue::TextCap(s.clone(), *n),
                    CarryLib::RecordRef(r) => LibraryValue::RecordRef(self.carry_rec(r, bound, ids)?),
                    CarryLib::RecordId {
                        table,
                        table_no,
                        name,
                        key,
                    } => LibraryValue::RecordId {
                        table: *table,
                        table_no: *table_no,
                        name: name.clone(),
                        key: row_of(key).ok_or_else(bad)?,
                    },
                };
                Value::Library(kind.clone(), Rc::new(RefCell::new(lv)))
            }
        })
    }

    fn carry_inst(&mut self, i: &CarryInst, bound: i64, ids: &mut Ids) -> Result<Rc<RefCell<Instance>>, String> {
        let globals = i
            .globals
            .iter()
            .map(|g| self.carry_decode(g, bound, ids))
            .collect::<Result<Vec<_>, _>>()?;
        let inst = Rc::new(RefCell::new(Instance {
            object: i.object,
            globals,
            initialized: i.initialized,
            rerun: i.rerun,
            base: None,
            exts: Vec::new(),
            xp_text: i.xp_text.clone(),
            born: al_builtins::value::Born(i.born),
            single: false,
        }));
        if let Some(rank) = i.bound {
            // Bound again once every global is back, in the order of the session before (`restore_single_instances`).
            self.carried_binding(rank, &inst);
        }
        Ok(inst)
    }

    fn carry_rec(&mut self, c: &CarryRec, bound: i64, ids: &mut Ids) -> Result<Rc<RefCell<RecordState>>, String> {
        let bad = || "a carried record that does not read back".to_string();
        if c.max_version > bound {
            return Err(format!(
                "a record buffer with a row version this session has not committed ({} above {bound})",
                c.max_version
            ));
        }
        let temp_id = match &c.temp_rows {
            None => None,
            Some(tables) => {
                let mut ex = Vec::with_capacity(tables.len());
                for (company, object, rows) in tables {
                    let mut out = Vec::with_capacity(rows.len());
                    for (k, r) in rows {
                        out.push((row_of(k).ok_or_else(bad)?, row_of(r).ok_or_else(bad)?));
                    }
                    ex.push(al_data::memory::ExportedTable {
                        company: company.clone(),
                        object: *object,
                        rows: out,
                    });
                }
                let id = self.new_temp_id();
                self.temps.insert(id.id(), al_data::memory::MemStore::from_exported(ex));
                Some(id)
            }
        };
        let globals = TableGlobals::default();
        for (o, i) in &c.globals {
            let inst = self.carry_inst(i, bound, ids)?;
            globals.set_instance(*o, inst);
        }
        if !c.xrec_globals.is_empty() {
            let x = globals.xrec();
            for (o, i) in &c.xrec_globals {
                let inst = self.carry_inst(i, bound, ids)?;
                x.set_instance(*o, inst);
            }
        }
        let opt_row = |r: &Option<Vec<u8>>| -> Result<Option<Vec<Value>>, String> {
            r.as_deref().map(|r| row_of(r).ok_or_else(bad)).transpose()
        };
        Ok(Rc::new(RefCell::new(RecordState {
            table: c.table,
            temporary: c.temporary,
            temp_id,
            company: c.company.clone(),
            crm: false,
            fields: row_of(&c.fields).ok_or_else(bad)?,
            filters: filters_of(&c.filters).ok_or_else(bad)?,
            filter_group: c.filter_group,
            marks: c
                .marks
                .iter()
                .map(|m| row_of(m).ok_or_else(bad))
                .collect::<Result<_, _>>()?,
            marked_only: c.marked_only,
            read_isolation: c.read_isolation,
            security_filtering: c.security_filtering,
            key: c.key,
            adhoc: c.adhoc.clone(),
            ascending: c.ascending,
            field_descending: c.field_descending,
            position: opt_row(&c.position)?,
            auto_calc: c.auto_calc.clone(),
            load: LoadState {
                request: c.load_request.clone(),
                buffer: match &c.load_buffer {
                    None => None,
                    Some(b) => Some(Buffer {
                        have: b.have.clone(),
                        unsure: b.unsure.clone(),
                        row: b.row.clone(),
                        cleared: b.cleared,
                        assigned: b.assigned.clone(),
                        missing_loaded: b.missing_loaded,
                        fetched: FetchedRow(opt_row(&b.fetched)?.map(Rc::new)),
                        init: b.init,
                    }),
                },
                partial: c.load_partial,
            },
            touched: c.touched,
            assigned: c.assigned,
            inserted: c.inserted,
            dirty_base: opt_row(&c.dirty_base)?,
            is_dirty_base: match &c.is_dirty_base {
                CarryDirty::Unknown(s) => DirtyBase::Unknown(s),
                CarryDirty::Fresh => DirtyBase::Fresh,
                CarryDirty::Synced => DirtyBase::Synced,
                CarryDirty::Row {
                    base,
                    alt,
                    dirty_only,
                    stale_store,
                } => DirtyBase::Row {
                    base: row_of(base).ok_or_else(bad)?,
                    alt: opt_row(alt)?,
                    dirty_only: *dirty_only,
                    stale_store: *stale_store,
                },
            },
            last_modify: opt_row(&c.last_modify)?,
            failed_insert: c.failed_insert,
            row_origin: c.row_origin,
            globals,
            unpinned_buffer: c.unpinned_buffer,
            run_page_link: None,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_read_back_as_written() {
        let t = |s: &str| Value::Text(s.to_string().into());
        let fs = vec![
            FieldFilter {
                field: 2,
                expr: FilterExpr::Or(vec![
                    FilterExpr::Range(Some(Value::Integer(1)), None),
                    FilterExpr::Like("A*".into(), true),
                    FilterExpr::NumCode(Box::new(FilterExpr::Ne(Value::BigInteger(7)))),
                ]),
                group: -1,
            },
            FieldFilter {
                field: 0,
                expr: FilterExpr::And(vec![FilterExpr::Eq(t("x")), FilterExpr::AtSignAlone(true)]),
                group: 2,
            },
        ];
        let back = filters_of(&filters_bytes(&fs).expect("plain")).expect("reads back");
        assert_eq!(format!("{fs:?}"), format!("{back:?}"));
    }

    #[test]
    fn an_origin_from_an_earlier_session_meets_no_commit_count_rule() {
        // The rules compare `commits + k` (k <= 4) and `commits` with the session's count (records/stale.rs,
        // records/op_inner.rs, stale_own_modify.rs): none holds for any count a session reaches, and none overflows.
        let c = RowOrigin::EARLIER_SESSION;
        for session in [0u64, 1, 5, 1_000_000] {
            for k in 0..=4 {
                assert_ne!(c + k, session);
            }
            assert!(!matches!(session.wrapping_sub(c), 1..=3));
        }
    }
}
