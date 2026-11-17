//! Whether a routine only reads the records of a table it declares, in a shape corpus PR 1431 measured (al-runner issue
//! 8238; probe tests/al-language/record/table-events-reads/TiReadEventTests.al and TiCompanyReadEventTests.al, open):
//! such a routine raises none of the table's ten table events (On(Before|After)(Insert|Modify|Delete|Rename|Validate)Event).
//! The static reach of an install routine that could not run (`blocked.rs` `subscribers_reached_with`, asked by
//! `install.rs` `touched`) leaves out their subscribers for it, on assumption `table-reads-raise-no-table-event`.

use al_ir::{Arg, Expr, ObjKind, Place};
use al_sema::lower::Lowered;

/// The record methods the probe calls on table 73600 "LT TI Read Row" without a table event: TiReadEventTests.al:47-92
/// (IsEmpty, Get, Find, FindFirst, FindLast, FindSet with Next, Next(-1), Count, CountApprox), :95-113 (on an empty table)
/// and :116-169 (SetRange, SetFilter, Reset, SetCurrentKey, SetAscending, Init, SetLoadFields, LockTable, GetFilters,
/// HasFilter). Of FindSet only the forms with at most one argument (`FindSet()`, `FindSet(true)`).
const APP_TABLE_READS: [&str; 19] = [
    "Record.IsEmpty",
    "Record.Get",
    "Record.Find",
    "Record.FindFirst",
    "Record.FindLast",
    "Record.FindSet",
    "Record.Next",
    "Record.Count",
    "Record.CountApprox",
    "Record.SetRange",
    "Record.SetFilter",
    "Record.Reset",
    "Record.SetCurrentKey",
    "Record.SetAscending",
    "Record.Init",
    "Record.SetLoadFields",
    "Record.LockTable",
    "Record.GetFilters",
    "Record.HasFilter",
];

/// The record methods the probe calls on the platform table Company without a table event: TiCompanyReadEventTests.al:26-70
/// (IsEmpty, Count, FindFirst, Get, FindSet with Next, SetRange, Reset).
const COMPANY_READS: [&str; 8] = [
    "Record.IsEmpty",
    "Record.Count",
    "Record.FindFirst",
    "Record.Get",
    "Record.FindSet",
    "Record.Next",
    "Record.SetRange",
    "Record.Reset",
];

/// The object id of the platform table Company (TiCompanyReadEventTests.al:26).
const COMPANY: u32 = 2_000_000_006;

/// Of each table object, the subscribers of its events other than the ten table events (`impact_trace::TABLE_EVENTS`), cached on the
/// engine (`Engine::derived`).
struct OtherSubscribers(std::collections::HashMap<usize, Vec<(usize, usize)>>);

/// The subscribers of the events of table object `t` other than its ten table events: what a routine that only reads `t`
/// ([`reads_only`]) can reach of `t`'s subscribers.
pub(crate) fn non_table_event_subscribers(engine: &al_vm::Engine, t: usize) -> Vec<(usize, usize)> {
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

/// The read methods measured on table object `t` (see [`APP_TABLE_READS`] and [`COMPANY_READS`]): the platform table
/// Company by its id; or an app table (an id below 2000000000) whose TableType is absent or Normal, with no table trigger
/// and no table extension, as table 73600 "LT TI Read Row" of the probe (TiReadEventObjects.al). `None` for every other
/// table.
fn measured_reads(lowered: &Lowered, t: usize) -> Option<&'static [&'static str]> {
    let objs = &lowered.program.objects;
    let head = objs.heads().get(t)?;
    if head.kind == ObjKind::Table && head.id == Some(COMPANY) {
        return Some(&COMPANY_READS);
    }
    let app_table = head.kind == ObjKind::Table && head.id.is_some_and(|id| (1..2_000_000_000).contains(&id));
    let normal = (head.properties.iter())
        .find(|(k, _)| k.eq_ignore_ascii_case("TableType"))
        .is_none_or(|(_, v)| v.eq_ignore_ascii_case("Normal"));
    let no_trigger = objs[t].routines.iter().all(|r| !r.is_trigger);
    let extended = (objs.heads().iter()).any(|h| h.kind == ObjKind::TableExtension && h.extends_id == head.id);
    (app_table && normal && no_trigger && !extended).then_some(&APP_TABLE_READS)
}

/// Whether routine `r` of object `o` only reads table object `t` in a measured shape: `o` is a codeunit without a
/// TableNo property (no implicit `Rec`); every local and global slot of the routine whose type names `t` is a record that
/// is not temporary (`Record(t, false)`); `t` has measured reads ([`measured_reads`]); and every occurrence of those
/// slots in the routine is the receiver of one of those reads or the record of a field read. Any other use (an assignment,
/// a field assignment, a `var` or value argument, a write, Validate, Copy, TransferFields or any other method) answers
/// `false`.
pub(crate) fn reads_only(lowered: &Lowered, o: usize, r: usize, t: usize) -> bool {
    let obj = &lowered.program.objects[o];
    if obj.kind != ObjKind::Codeunit || obj.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case("TableNo")) {
        return false;
    }
    let Some(measured) = measured_reads(lowered, t) else {
        return false;
    };
    let rt = &obj.routines[r];
    use al_builtins::value::RtType;
    let names_t = |ty: &RtType| matches!(ty, RtType::Record(x, _) if *x as usize == t);
    let mut slots: Vec<(bool, usize)> = Vec::new();
    for (global, list) in [(false, &rt.locals), (true, &obj.globals)] {
        for (i, s) in list.iter().enumerate().filter(|(_, s)| names_t(&s.ty)) {
            if !matches!(s.ty, RtType::Record(_, false)) {
                return false;
            }
            slots.push((global, i));
        }
    }
    let mine = |p: &Place| super::written::root(p).is_some_and(|k| slots.contains(&k));
    let whole = |p: &Place| match p {
        Place::Local(i) => slots.contains(&(false, *i)),
        Place::Global(i) => slots.contains(&(true, *i)),
        _ => false,
    };
    let read = |name: &str, args: usize| {
        let method = name.split('#').next().unwrap_or_default();
        measured.contains(&method) && (method != "Record.FindSet" || args <= 1)
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
            let read = read(id.name(), args.len());
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

#[cfg(test)]
mod tests;
