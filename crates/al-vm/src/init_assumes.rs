//! The unprobed assumptions a fresh record of a table rests on through its fields' `InitValue`
//! (`TableFacts::init_assumes`): a fresh buffer marks them (`Session::initial_fields`).

use al_builtins::assumptions::Assumption;
use al_builtins::value::RtType;

/// The assumptions the `InitValue`s of `t`'s fields rest on, each once.
pub(crate) fn of_table(t: &al_ir::Table) -> Vec<Assumption> {
    let mut out = Vec::new();
    for f in &t.fields {
        let (Some(c), Some(text)) = (&f.init, &f.init_text) else {
            continue;
        };
        let text = text.trim();
        let a = match (&f.ty, c) {
            // An Option InitValue written as a number no member is named (al-sema `option_init_ordinal`).
            (RtType::Option(m), al_ir::Const::Option(..)) => {
                let t = text.trim_matches('"').trim();
                (t.parse::<u32>().is_ok() && !m.iter().any(|x| x.trim().eq_ignore_ascii_case(t)))
                    .then_some(Assumption::OptionInitValueOrdinal)
            }
            _ => None,
        };
        if let Some(a) = a.filter(|a| !out.contains(a)) {
            out.push(a);
        }
    }
    out
}

/// Marks the assumptions of `facts` (a fresh buffer of the table).
pub(crate) fn mark(facts: Option<&crate::TableFacts>) {
    for a in facts.map_or(&[][..], |f| &f.init_assumes[..]) {
        al_builtins::assumptions::assume(*a);
    }
}
