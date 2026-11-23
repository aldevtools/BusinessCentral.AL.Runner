//! Whether the routines of a static reach only read the records of a table, in a shape corpus PR 1431 measured (al-runner
//! issue 8238; probe tests/al-language/record/table-events-reads/TiReadEventTests.al and TiCompanyReadEventTests.al, open):
//! such reads raise none of the table's ten table events (On(Before|After)(Insert|Modify|Delete|Rename|Validate)Event).
//! The static reach of an install routine that could not run (`blocked.rs` `subscribers_reached_with`, asked by
//! `install.rs` `touched`) leaves out their subscribers for such a table ([`ReadOnlyTables`]), on assumption
//! `table-reads-raise-no-table-event`.

use al_builtins::value::RtType;
use al_ir::{Arg, Const, Expr, ObjKind, Place};
use al_sema::lower::Lowered;
use std::collections::BTreeSet;

/// The arguments of one measured call, as the probe wrote them.
#[derive(Clone, Copy)]
enum Args {
    /// This many arguments, of any value (a key value, a field, a filter).
    Count(usize),
    /// One constant text argument.
    Text(&'static str),
    /// One constant Boolean argument.
    Bool(bool),
    /// One constant integer argument.
    Int(i32),
}

use Args::{Bool, Count, Int, Text};

/// The record calls the probe makes on table 73600 "LT TI Read Row" without a table event, with their arguments
/// (TiReadEventTests.al): IsEmpty :54, :103; Get :55-56, :149; Find('-'), Find('+'), Find('=') :57-59; FindFirst :60,
/// :104, :136; FindLast :61, :151; FindSet() :63, :105; FindSet(true) :69; Next() :66, :72; Next(-1) :74; Count :75, :106,
/// :129; CountApprox :76; SetRange with a range :125; SetFilter with two values :127; Reset :130; SetCurrentKey :132;
/// SetAscending :134; Init :142; SetLoadFields :148; LockTable :150; GetFilters and HasFilter :152.
const APP_TABLE_READS: [(&str, Args); 23] = [
    ("Record.IsEmpty", Count(0)),
    ("Record.Get", Count(1)),
    ("Record.Find", Text("-")),
    ("Record.Find", Text("+")),
    ("Record.Find", Text("=")),
    ("Record.FindFirst", Count(0)),
    ("Record.FindLast", Count(0)),
    ("Record.FindSet", Count(0)),
    ("Record.FindSet", Bool(true)),
    ("Record.Next", Count(0)),
    ("Record.Next", Int(-1)),
    ("Record.Count", Count(0)),
    ("Record.CountApprox", Count(0)),
    ("Record.SetRange", Count(3)),
    ("Record.SetFilter", Count(4)),
    ("Record.Reset", Count(0)),
    ("Record.SetCurrentKey", Count(1)),
    ("Record.SetAscending", Count(2)),
    ("Record.Init", Count(0)),
    ("Record.SetLoadFields", Count(1)),
    ("Record.LockTable", Count(0)),
    ("Record.GetFilters", Count(0)),
    ("Record.HasFilter", Count(0)),
];

/// The record calls the probe makes on the platform table Company without a table event, with their arguments
/// (TiCompanyReadEventTests.al): IsEmpty :36; Count :38, :49; FindFirst :40; Get :42, :52; FindSet() :43; Next() :46;
/// SetRange with one value :48; Reset :50.
const COMPANY_READS: [(&str, Args); 8] = [
    ("Record.IsEmpty", Count(0)),
    ("Record.Count", Count(0)),
    ("Record.FindFirst", Count(0)),
    ("Record.Get", Count(1)),
    ("Record.FindSet", Count(0)),
    ("Record.Next", Count(0)),
    ("Record.SetRange", Count(2)),
    ("Record.Reset", Count(0)),
];

/// The object id of the platform table Company (TiCompanyReadEventTests.al:29).
const COMPANY: u32 = 2_000_000_006;

/// Whether the arguments `args` of a call are the form `want`.
fn args_are(want: Args, args: &[Arg]) -> bool {
    let one = match args {
        [Arg::Value(e)] => Some(e),
        _ => None,
    };
    match (want, one) {
        (Count(n), _) => args.len() == n,
        (Text(s), Some(Expr::Const(Const::Text(t)))) => t == s,
        (Bool(b), Some(Expr::Const(Const::Boolean(x)))) => *x == b,
        (Int(n), Some(Expr::Const(Const::Integer(x)))) => *x == n,
        // `-1` is written as the negation of the constant 1.
        (Int(n), Some(Expr::Unary(al_ir::UnOp::Neg, x))) => matches!(&**x, Expr::Const(Const::Integer(x)) if -x == n),
        _ => false,
    }
}

/// Of each table object, the subscribers of its events other than the ten table events (`impact_trace::TABLE_EVENTS`),
/// cached on the engine (`Engine::derived`).
struct OtherSubscribers(std::collections::HashMap<usize, Vec<(usize, usize)>>);

/// The subscribers of the events of table object `t` other than its ten table events: what a reach whose routines only
/// read `t` ([`ReadOnlyTables`]) can reach of `t`'s subscribers.
fn non_table_event_subscribers(engine: &al_vm::Engine, t: usize) -> Vec<(usize, usize)> {
    let index = engine.derived(|| {
        let mut index: std::collections::HashMap<usize, Vec<(usize, usize)>> = Default::default();
        for ((p, event), list) in &engine.events.subscribers {
            if !crate::impact_trace::TABLE_EVENTS.contains(&event.as_str()) {
                (index.entry(*p).or_default()).extend(list.iter().map(|s| (s.object, s.routine)));
            }
        }
        OtherSubscribers(index)
    });
    index.0.get(&t).cloned().unwrap_or_default()
}

/// The read calls measured on table object `t` (see [`APP_TABLE_READS`] and [`COMPANY_READS`]): the platform table
/// Company by its id; or an app table (an id below 2000000000) with no TableType property, only Normal fields, no table
/// trigger and no table extension, as table 73600 "LT TI Read Row" of the probe (TiReadEventObjects.al:13-29). `None` for
/// every other table.
fn measured_reads(lowered: &Lowered, t: usize) -> Option<&'static [(&'static str, Args)]> {
    let objs = &lowered.program.objects;
    let head = objs.heads().get(t)?;
    if head.kind == ObjKind::Table && head.id == Some(COMPANY) {
        return Some(&COMPANY_READS);
    }
    let app_table = head.kind == ObjKind::Table && head.id.is_some_and(|id| (1..2_000_000_000).contains(&id));
    let no_table_type = head
        .properties
        .iter()
        .all(|(k, _)| !k.eq_ignore_ascii_case("TableType"));
    // Every field the table declares is Normal, as the probe's are (TiReadEventTests.al:47-92); the platform's system
    // fields (ids from 2000000000, the user-name FlowFields among them) are on every table, the probe's too.
    let normal_fields = (objs[t].table.as_ref()).is_some_and(|d| {
        (d.fields.iter().filter(|f| f.id < 2_000_000_000))
            .all(|f| al_ir::FieldFacts::of(f).class == al_ir::FieldClass::Normal)
    });
    let no_trigger = objs[t].routines.iter().all(|r| !r.is_trigger);
    let extended = (objs.heads().iter()).any(|h| h.kind == ObjKind::TableExtension && h.extends_id == head.id);
    (app_table && no_table_type && normal_fields && no_trigger && !extended).then_some(&APP_TABLE_READS)
}

/// Whether type `ty` holds a record of table object `t`: the record type, or an array of it.
fn holds(ty: &RtType, t: usize) -> bool {
    match ty {
        RtType::Record(x, _) => *x as usize == t,
        RtType::Array(_, el) => holds(el, t),
        _ => false,
    }
}

/// Whether routine `r` of object `o` only reads table object `t` in a measured shape: `o` is a codeunit without a
/// TableNo property (no implicit `Rec`); `t` has measured reads ([`measured_reads`]); every slot that holds a record
/// of `t` is a local the routine declares, as the probe's records are (TiReadEventTests.al:50, :98, :119-120;
/// TiCompanyReadEventTests.al:29), and a record that is not temporary (`Record(t, false)`), not an array, a parameter, the
/// return value or a global; and every occurrence of those slots is the receiver of a measured call or the record of a
/// field read. Any other use (an assignment, a field assignment, a `var` or value argument, a write, Validate, Copy,
/// TransferFields, an unmeasured method or argument form) answers `false`.
pub(crate) fn reads_only(lowered: &Lowered, o: usize, r: usize, t: usize) -> bool {
    let obj = &lowered.program.objects[o];
    if obj.kind != ObjKind::Codeunit || obj.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case("TableNo")) {
        return false;
    }
    let Some(measured) = measured_reads(lowered, t) else {
        return false;
    };
    let rt = &obj.routines[r];
    if rt.ret.as_ref().is_some_and(|ty| holds(ty, t)) || obj.globals.iter().any(|s| holds(&s.ty, t)) {
        return false;
    }
    let mut slots: Vec<usize> = Vec::new();
    for (i, s) in rt.locals.iter().enumerate().filter(|(_, s)| holds(&s.ty, t)) {
        let declared = i >= rt.params && rt.ret_local != Some(i);
        if !(declared && s.ty == RtType::Record(t as u32, false)) {
            return false;
        }
        slots.push(i);
    }
    let mine = |p: &Place| super::written::root(p).is_some_and(|(global, i)| !global && slots.contains(&i));
    let whole = |p: &Place| matches!(p, Place::Local(i) if slots.contains(i));
    let read = |name: &str, args: &[Arg]| {
        let method = name.split('#').next().unwrap_or_default();
        (measured.iter()).any(|(m, want)| *m == method && args_are(*want, args))
    };
    let (mut uses, mut reads) = (0usize, 0usize);
    super::written::stmt_places(&rt.body, &mut |p| uses += usize::from(mine(p)));
    let refs = |args: &[Arg], uses: &mut usize| {
        for a in args {
            if let Arg::Ref(p) = a {
                *uses += usize::from(mine(p));
            }
        }
    };
    al_ir::types::for_each_expr_post(&rt.body, &mut |e| match e {
        Expr::Load(p) => {
            uses += usize::from(mine(p));
            if let Place::Field(b, _) = p {
                reads += usize::from(whole(b));
            }
        }
        Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. } => refs(args, &mut uses),
        Expr::Builtin { id, receiver, args, .. } => {
            refs(args, &mut uses);
            let read = read(id.written_base().name(), args);
            match receiver.as_deref() {
                Some(Arg::Ref(p)) => {
                    uses += usize::from(mine(p));
                    reads += usize::from(read && whole(p));
                }
                // The load under it is counted by the `Expr::Load` arm.
                Some(Arg::Value(Expr::Load(p))) => reads += usize::from(read && whole(p)),
                _ => {}
            }
        }
        _ => {}
    });
    uses == reads
}

/// The tables whose table events routine `r` of `o` can raise, as [`ReadOnlyTables`] counts them.
enum Kept {
    /// Any table: it runs an object, writes a table not known statically or has a part that is not known
    /// (`ReachIndex::own_writes`), opens a `RecordRef` on a table known only at run time
    /// (`baseline::recordref_opens`), takes a `RecordRef` or `FieldRef` from outside (a parameter or a global), or
    /// calls a record method on something other than a variable (a returned record, an array element).
    All,
    /// These: the tables it writes, the tables a `RecordRef` of it opens, and every table a slot of it holds a record of
    /// that it does not only read ([`reads_only`]).
    Tables(BTreeSet<usize>),
}

/// The [`Kept`] of routine `r` of `o`.
fn kept(engine: &al_vm::Engine, lowered: &Lowered, o: usize, r: usize) -> Kept {
    let index = engine.derived(|| super::written::ReachIndex::new(&engine.events, lowered));
    let Some(writes) = index.own_writes(&engine.events, lowered, o, r) else {
        return Kept::All;
    };
    let Some(opens) = crate::baseline::recordref_opens(lowered, o, r) else {
        return Kept::All;
    };
    let obj = &lowered.program.objects[o];
    let rt = &obj.routines[r];
    let outside_ref = (rt.locals[..rt.params].iter().chain(&obj.globals)).any(|s| {
        use al_builtins::library_kind::LibraryKind;
        matches!(s.ty, RtType::Library(LibraryKind::RecordRef | LibraryKind::FieldRef))
    });
    if outside_ref || record_call_on_no_variable(&rt.body) {
        return Kept::All;
    }
    let mut kept: BTreeSet<usize> = writes.into_iter().chain(opens).map(|t| t as usize).collect();
    let mut held = BTreeSet::new();
    for ty in rt.locals.iter().chain(&obj.globals).map(|s| &s.ty).chain(&rt.ret) {
        record_tables(ty, &mut held);
    }
    kept.extend(held.into_iter().filter(|&t| !reads_only(lowered, o, r, t)));
    Kept::Tables(kept)
}

/// The tables type `ty` holds records of ([`holds`]), added to `out`.
fn record_tables(ty: &RtType, out: &mut BTreeSet<usize>) {
    match ty {
        RtType::Record(t, _) => {
            out.insert(*t as usize);
        }
        RtType::Array(_, el) => record_tables(el, out),
        _ => {}
    }
}

/// Whether `body` calls a record method whose receiver is not a local or global variable: a returned record, an array
/// element, a record field of another object.
fn record_call_on_no_variable(body: &[al_ir::Stmt]) -> bool {
    let mut found = false;
    al_ir::types::for_each_expr_post(body, &mut |e| {
        if let Expr::Builtin { id, receiver, .. } = e {
            let variable = matches!(
                receiver.as_deref(),
                Some(
                    Arg::Ref(Place::Local(_) | Place::Global(_))
                        | Arg::Value(Expr::Load(Place::Local(_) | Place::Global(_)))
                )
            );
            found |= id.name().starts_with("Record.") && !variable;
        }
    });
    found
}

/// What the walk of `blocked.rs` `subscribers_reached_with` (`reading`) has seen of the routines it visited, per table:
/// the subscribers of a table's ten table events stay out of the reach only while every routine visited only reads the
/// table ([`Kept`]). A routine that keeps a table, met before or after, brings them back.
#[derive(Default)]
pub(crate) struct ReadOnlyTables {
    /// The tables a record slot of a visited routine names (the tables whose subscribers main's walk reaches).
    met: BTreeSet<usize>,
    /// The tables a visited routine keeps (`Kept::Tables`).
    kept: BTreeSet<usize>,
    /// A visited routine keeps every table (`Kept::All`).
    all: bool,
}

impl ReadOnlyTables {
    /// Takes routine `(o, r)` of the walk, whose record slots name `tables`: calls `reach` on the subscribers of the
    /// tables of `tables` met first here (all of a table's subscribers, or those of its other events while every routine
    /// so far only reads it), and on all the subscribers of a table met before that this routine keeps.
    pub(crate) fn visit(
        &mut self,
        engine: &al_vm::Engine,
        lowered: &Lowered,
        (o, r): (usize, usize),
        tables: &BTreeSet<usize>,
        reach: &mut dyn FnMut((usize, usize)),
    ) {
        let newly_kept: Vec<usize> = match kept(engine, lowered, o, r) {
            Kept::All if self.all => Vec::new(),
            Kept::All => {
                self.all = true;
                self.met.iter().copied().collect()
            }
            Kept::Tables(k) => (k.into_iter())
                .filter(|t| self.kept.insert(*t) && !self.all && self.met.contains(t))
                .collect(),
        };
        for t in newly_kept {
            engine.events.subscribers_of(t).iter().copied().for_each(&mut *reach);
        }
        for &t in tables {
            if !self.met.insert(t) {
                continue;
            }
            if self.all || self.kept.contains(&t) {
                engine.events.subscribers_of(t).iter().copied().for_each(&mut *reach);
            } else {
                non_table_event_subscribers(engine, t).into_iter().for_each(&mut *reach);
            }
        }
    }
}

#[cfg(test)]
mod tests;
