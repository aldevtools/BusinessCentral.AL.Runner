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
//!   a table with an AutoIncrement field (its counter is the session's, `autoinc.rs`) and a table whose Code key is
//!   stored as a number (its key order is not written with the rows) are refused.
//! - The row version (SystemRowVersion in the buffer, `row_origin.version`, `failed_insert`, the version in
//!   `last_modify`) of a record that is not temporary: carried when it is at or below the session's committed row
//!   version at the point it began to track the single instances. Such a row was committed before the test codeunit
//!   began, and every session of the run starts from the same committed rows, so the number names the same row
//!   version there. A higher number was given by the earlier session to a row that BC rolled back at the end of that
//!   test codeunit (TestCuP7BReader.al:24-37: rows are rolled back per codeunit) while the next session hands out the
//!   same numbers again: refused. Temporary rows keep their numbers with their rows (a temporary record's row
//!   version is no database row version: `stale_verdict` returns Fresh for it first).
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarryLib {
    Empty,
    Text(String),
    TextCap(String, i32),
    RecordRef(Box<CarryRec>),
    RecordId {
        table: u32,
        table_no: u32,
        name: String,
        key: Vec<u8>,
    },
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
}

/// A record variable (see the module for the meaning of each session-relative part).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarryRec {
    pub table: u32,
    pub temporary: bool,
    /// The rows of its temporary table (`None`: the record has no table of its own yet).
    pub temp_rows: Option<Vec<(String, u32, Vec<(Vec<u8>, Vec<u8>)>)>>,
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

/// How deep handles inside handles are carried.
const DEPTH: usize = 12;

fn row_bytes(row: &[Value]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    vc::encode_row(row, &mut out).then_some(out)
}

fn row_of(bytes: &[u8]) -> Option<Vec<Value>> {
    vc::decode_row(&mut Reader::new(bytes))
}

fn put_filter(e: &FilterExpr, out: &mut Vec<u8>) -> bool {
    use vc::{put_str, put_u8, put_u32};
    let one = |tag: u8, v: &Value, out: &mut Vec<u8>| {
        put_u8(out, tag);
        vc::encode(v, out)
    };
    match e {
        FilterExpr::Range(a, b) => {
            put_u8(out, 0);
            for end in [a, b] {
                match end {
                    None => put_u8(out, 0),
                    Some(v) => {
                        put_u8(out, 1);
                        if !vc::encode(v, out) {
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
                    *end = Some(vc::decode(r)?);
                }
            }
            let [a, b] = ends;
            FilterExpr::Range(a, b)
        }
        1 => FilterExpr::Eq(vc::decode(r)?),
        2 => FilterExpr::RangeSame(vc::decode(r)?),
        3 => FilterExpr::Ne(vc::decode(r)?),
        4 => FilterExpr::Lt(vc::decode(r)?),
        5 => FilterExpr::Le(vc::decode(r)?),
        6 => FilterExpr::Gt(vc::decode(r)?),
        7 => FilterExpr::Ge(vc::decode(r)?),
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

/// Writes the values reachable from a set of roots (see the module). A handle reached twice (two globals sharing a
/// list, a FieldRef into a carried RecordRef's buffer, two records sharing a temporary table) is refused: the copies
/// would no longer share it.
pub(crate) struct Enc<'s> {
    session: &'s Session,
    shared: HashSet<usize>,
    shared_temps: HashSet<u64>,
    /// The committed row version when the session began to track the single instances.
    bound: i64,
}

fn ptr<T>(rc: &Rc<T>) -> usize {
    Rc::as_ptr(rc) as *const () as usize
}

impl<'s> Enc<'s> {
    pub(crate) fn new<'v>(session: &'s Session, roots: impl IntoIterator<Item = &'v Value>, bound: i64) -> Enc<'s> {
        let mut seen: HashMap<usize, u32> = HashMap::new();
        let mut temps: HashMap<u64, u32> = HashMap::new();
        for v in roots {
            count(v, &mut seen, &mut temps, 0);
        }
        Enc {
            session,
            shared: seen.into_iter().filter(|&(_, n)| n > 1).map(|(p, _)| p).collect(),
            shared_temps: temps.into_iter().filter(|&(_, n)| n > 1).map(|(p, _)| p).collect(),
            bound,
        }
    }

    fn unshared<T>(&self, rc: &Rc<T>, v: &Value) -> Result<(), String> {
        if self.shared.contains(&ptr(rc)) {
            return Err(format!("{} that another carried variable shares", v.describe()));
        }
        Ok(())
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
                self.unshared(inst, v)?;
                CarryVal::Codeunit(Box::new(self.inst(inst, depth)?))
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
                DotNetKind::Internal { type_name, state } => {
                    self.unshared(obj, v)?;
                    let state = state.try_borrow().map_err(busy)?;
                    CarryVal::DotNetInternal {
                        type_name: type_name.clone(),
                        state: state.iter().map(|x| self.val(x, depth + 1)).collect::<Result<_, _>>()?,
                    }
                }
                _ => return Err(v.describe()),
            },
            Value::List(l) => {
                self.unshared(l, v)?;
                let l = l.try_borrow().map_err(busy)?;
                CarryVal::List {
                    items: l.iter().map(|x| self.val(x, depth + 1)).collect::<Result<_, _>>()?,
                    version: l.version,
                    read_only: l.read_only,
                }
            }
            Value::Dictionary(d) => {
                self.unshared(d, v)?;
                let d = d.try_borrow().map_err(busy)?;
                CarryVal::Dictionary(
                    d.iter()
                        .map(|(k, x)| Ok((self.val(k, depth + 1)?, self.val(x, depth + 1)?)))
                        .collect::<Result<_, String>>()?,
                )
            }
            Value::Array(a) => {
                self.unshared(a, v)?;
                let a = a.try_borrow().map_err(busy)?;
                CarryVal::Array(a.iter().map(|x| self.val(x, depth + 1)).collect::<Result<_, _>>()?)
            }
            Value::Record(rc) => {
                self.unshared(rc, v)?;
                CarryVal::Record(Box::new(self.rec(rc, depth)?))
            }
            Value::Library(kind, lv) => {
                self.unshared(lv, v)?;
                let lv = lv.try_borrow().map_err(busy)?;
                let lib = match (kind, &*lv) {
                    (LibraryKind::RecordRef | LibraryKind::TextBuilder, LibraryValue::Empty) => CarryLib::Empty,
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
                        key: row_bytes(key).ok_or_else(|| v.describe())?,
                    },
                    _ => return Err(v.describe()),
                };
                CarryVal::Library(kind.clone(), lib)
            }
            _ => {
                let mut out = Vec::new();
                if !vc::encode(v, &mut out) {
                    return Err(v.describe());
                }
                CarryVal::Plain(out)
            }
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
        if *temporary {
            if self.session.auto_increment_field(&t).map_or(true, |a| a.is_some()) {
                return refuse("a temporary table with an AutoIncrement field, whose counter is the session's");
            }
            if let Some(id) = temp_id {
                if self.shared_temps.contains(&id.id()) {
                    return refuse("a temporary table another carried record shares");
                }
                if al_data::code_storage::pk_storage(&t).is_some() {
                    return refuse("a temporary table whose Code key is stored as a number");
                }
                let tables = match self.session.temps.get(&id.id()) {
                    Some(store) => store.export_current().ok_or_else(what)?,
                    None => Vec::new(),
                };
                let mut out = Vec::new();
                for x in tables {
                    let mut rows = Vec::with_capacity(x.rows.len());
                    for (k, row) in &x.rows {
                        rows.push((row_bytes(k).ok_or_else(what)?, row_bytes(row).ok_or_else(what)?));
                    }
                    out.push((x.company, x.object, rows));
                }
                temp_rows = Some(out);
            }
        } else {
            if row_origin.is_some_and(|o| o.own_write) {
                return refuse("a buffer of the variable's own write, which BC rolled back with the test codeunit");
            }
            max_version = version_of(fields)
                .max(row_origin.map_or(0, |o| o.version))
                .max(failed_insert.unwrap_or(0))
                .max(last_modify.as_deref().map_or(0, version_of));
            if max_version > self.bound {
                return refuse(
                    "a row version an earlier test codeunit wrote: BC rolled that row back, and the next session \
                     hands out the same row versions again",
                );
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
                    Some(row) => Some(row_bytes(row).ok_or_else(what)?),
                },
                init: *init,
            }),
        };
        let opt_row = |r: &Option<Vec<Value>>| -> Result<Option<Vec<u8>>, String> {
            r.as_deref().map(|r| row_bytes(r).ok_or_else(what)).transpose()
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
            fields: row_bytes(fields).ok_or_else(what)?,
            filters: filters_bytes(filters).ok_or_else(what)?,
            filter_group: *filter_group,
            marks: marks
                .iter()
                .map(|m| row_bytes(m).ok_or_else(what))
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
            dirty_base: opt_row(dirty_base)?,
            is_dirty_base: match is_dirty_base {
                DirtyBase::Unknown(s) => CarryDirty::Unknown(s),
                DirtyBase::Fresh => CarryDirty::Fresh,
                DirtyBase::Synced => CarryDirty::Synced,
                DirtyBase::Row {
                    base,
                    alt,
                    dirty_only,
                    stale_store,
                } => CarryDirty::Row {
                    base: row_bytes(base).ok_or_else(what)?,
                    alt: opt_row(alt)?,
                    dirty_only: *dirty_only,
                    stale_store: *stale_store,
                },
            },
            last_modify: opt_row(last_modify)?,
            failed_insert: *failed_insert,
            row_origin: row_origin.map(|o| RowOrigin {
                commits: RowOrigin::EARLIER_SESSION,
                ..o
            }),
            globals: self.table_globals(globals, depth)?,
            xrec_globals: xrec,
            unpinned_buffer: *unpinned_buffer,
            max_version,
        })
    }
}

/// Counts how often each handle (and each temporary table) is reached from the roots; a handle is entered once.
fn count(v: &Value, seen: &mut HashMap<usize, u32>, temps: &mut HashMap<u64, u32>, depth: usize) {
    if depth >= DEPTH {
        return;
    }
    let mut first = |p: usize| {
        let n = seen.entry(p).or_default();
        *n += 1;
        *n == 1
    };
    match v {
        Value::Codeunit(inst) => {
            if inst.try_borrow().is_ok_and(|i| !i.single) && first(ptr(inst)) {
                if let Ok(i) = inst.try_borrow() {
                    for g in &i.globals {
                        count(g, seen, temps, depth + 1);
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
                    count(x, seen, temps, depth + 1);
                }
            }
        }
        Value::List(l) if first(ptr(l)) => {
            if let Ok(l) = l.try_borrow() {
                for x in l.iter() {
                    count(x, seen, temps, depth + 1);
                }
            }
        }
        Value::Dictionary(d) if first(ptr(d)) => {
            if let Ok(d) = d.try_borrow() {
                for (k, x) in d.iter() {
                    count(k, seen, temps, depth + 1);
                    count(x, seen, temps, depth + 1);
                }
            }
        }
        Value::Array(a) if first(ptr(a)) => {
            if let Ok(a) = a.try_borrow() {
                for x in a.iter() {
                    count(x, seen, temps, depth + 1);
                }
            }
        }
        Value::Record(rc) => count_rec(rc, seen, temps, depth),
        Value::Library(_, lv) if first(ptr(lv)) => {
            if let Ok(lv) = lv.try_borrow() {
                match &*lv {
                    LibraryValue::RecordRef(rc)
                    | LibraryValue::FieldRef(rc, _)
                    | LibraryValue::KeyFieldRef(rc, _)
                    | LibraryValue::KeyRef(rc, _)
                    | LibraryValue::UserNameFieldRef { rec: rc, .. } => count_rec(rc, seen, temps, depth),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn count_rec(
    rc: &Rc<RefCell<RecordState>>,
    seen: &mut HashMap<usize, u32>,
    temps: &mut HashMap<u64, u32>,
    depth: usize,
) {
    let n = seen.entry(ptr(rc)).or_default();
    *n += 1;
    if *n > 1 {
        return;
    }
    let Ok(r) = rc.try_borrow() else { return };
    if let Some(id) = &r.temp_id {
        *temps.entry(id.id()).or_default() += 1;
    }
    let mut insts = r.globals.instances();
    if !r.globals.is_unset() {
        insts.extend(r.globals.xrec().instances());
    }
    for (_, i) in insts {
        count(&Value::Codeunit(i), seen, temps, depth + 1);
    }
}

impl Session {
    /// The value `c` stands for in this session (see the module); the reason when it has none here. `bound`: this
    /// session's committed row version when it began to track the single instances.
    pub(crate) fn carry_decode(&mut self, c: &CarryVal, bound: i64) -> Result<Value, String> {
        let bad = || "a carried value that does not read back".to_string();
        Ok(match c {
            CarryVal::Plain(b) => vc::decode(&mut Reader::new(b)).ok_or_else(bad)?,
            CarryVal::Single(object) => Value::Codeunit(self.codeunit_instance(*object as usize)),
            CarryVal::Codeunit(i) => Value::Codeunit(self.carry_inst(i, bound)?),
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
                    .map(|x| self.carry_decode(x, bound))
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
                    .map(|x| self.carry_decode(x, bound))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut l = ListVec::from(items);
                l.version = *version;
                l.read_only = *read_only;
                Value::List(Rc::new(RefCell::new(l)))
            }
            CarryVal::Dictionary(pairs) => {
                let mut out = Vec::with_capacity(pairs.len());
                for (k, x) in pairs {
                    out.push((self.carry_decode(k, bound)?, self.carry_decode(x, bound)?));
                }
                Value::Dictionary(Rc::new(RefCell::new(out)))
            }
            CarryVal::Array(xs) => {
                let xs = xs
                    .iter()
                    .map(|x| self.carry_decode(x, bound))
                    .collect::<Result<Vec<_>, _>>()?;
                Value::Array(Rc::new(RefCell::new(xs)))
            }
            CarryVal::Record(r) => Value::Record(self.carry_rec(r, bound)?),
            CarryVal::Library(kind, lib) => {
                let lv = match lib {
                    CarryLib::Empty => LibraryValue::Empty,
                    CarryLib::Text(s) => LibraryValue::Text(s.clone()),
                    CarryLib::TextCap(s, n) => LibraryValue::TextCap(s.clone(), *n),
                    CarryLib::RecordRef(r) => LibraryValue::RecordRef(self.carry_rec(r, bound)?),
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

    fn carry_inst(&mut self, i: &CarryInst, bound: i64) -> Result<Rc<RefCell<Instance>>, String> {
        let globals = i
            .globals
            .iter()
            .map(|g| self.carry_decode(g, bound))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Rc::new(RefCell::new(Instance {
            object: i.object,
            globals,
            initialized: i.initialized,
            rerun: i.rerun,
            base: None,
            exts: Vec::new(),
            xp_text: i.xp_text.clone(),
            born: al_builtins::value::Born(i.born),
            single: false,
        })))
    }

    fn carry_rec(&mut self, c: &CarryRec, bound: i64) -> Result<Rc<RefCell<RecordState>>, String> {
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
            let inst = self.carry_inst(i, bound)?;
            globals.set_instance(*o, inst);
        }
        if !c.xrec_globals.is_empty() {
            let x = globals.xrec();
            for (o, i) in &c.xrec_globals {
                let inst = self.carry_inst(i, bound)?;
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
