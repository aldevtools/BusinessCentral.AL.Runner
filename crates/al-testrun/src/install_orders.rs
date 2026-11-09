//! The install order of apps with no dependency path between them (ADR-046 amendment 7, owner 2026-11-08, al-runner
//! issue 6644). BC keeps no order for such apps (measured on 27.5, 28.5 and 29.0, owner 2026-10-30), and there is no flag
//! to state one (owner, 2026-11-02). The runner orders them so:
//!
//! 1. Microsoft's apps install before partner apps ([`split_microsoft`]): a container's database comes from the BC
//!    artifact with Microsoft's apps installed, and a pipeline publishes and installs the partner apps into it afterwards.
//! 2. Partner apps whose install code may interact are installed in every order and the databases are compared
//!    ([`install_partner_apps`]): the same database, the run goes on; different rows, the tests that touch a table that
//!    differs are not run, with both apps and the table named. With more than [`MAX_INTERACTING`] interacting apps, or a
//!    difference outside the tables, the runner refuses as before.
//!
//! The install code runs more than once only for apps whose install code met an app of the other kind of refusal
//! (`Session::take_order_unknown_hits`); a run without such apps installs every app once, as before.

use crate::install::InstallView;
use al_sema::lower::Lowered;
use al_vm::Session;
use std::collections::BTreeSet;

/// The most partner apps whose every order the runner tries (3! = 6 orders). Beyond it, it refuses as before.
pub(crate) const MAX_INTERACTING: usize = 3;

/// The start of the baseline note that says which partner apps were installed in several orders (`--timings`).
pub const INSTALL_ORDERS_NOTE: &str = "install order: ";

/// The start of a baseline note that names one difference between two install orders by table, key and field.
pub const INSTALL_ORDER_DIFFERENCE_NOTE: &str = "install order difference: ";

/// How many differences the reason of a refusal names; the baseline notes list every one.
const SHOWN_DIFFERENCES: usize = 3;

/// Whether app `app` (index in `Program::apps`) counts as Microsoft's: its manifest's publisher is `Microsoft`, or no
/// manifest gave a publisher (then the app keeps the rules of amendment 6: no partner rule without the fact).
fn is_microsoft(lowered: &Lowered, app: u32) -> bool {
    lowered.program.apps.get(app as usize).is_none_or(|a| {
        let p = a.publisher.trim();
        p.is_empty() || p.eq_ignore_ascii_case("Microsoft")
    })
}

/// Splits `group` into (Microsoft's side, partner side). An app is on the partner side when its publisher is known and not
/// Microsoft and no Microsoft app of `group` depends on it (a dependency of a Microsoft app installs before it).
pub(crate) fn split_microsoft(
    lowered: &Lowered,
    view: &InstallView,
    group: &BTreeSet<u32>,
) -> (BTreeSet<u32>, BTreeSet<u32>) {
    let microsoft: Vec<u32> = group.iter().copied().filter(|&a| is_microsoft(lowered, a)).collect();
    group
        .iter()
        .copied()
        .partition(|&a| is_microsoft(lowered, a) || microsoft.iter().any(|&m| view.needs(m, a)))
}

/// One kind of install run, for [`install_partner_apps`]: the dependency apps' install baseline or the apps under test.
pub(crate) trait AppInstalls {
    /// What [`AppInstalls::rewind`] goes back to.
    type Mark;
    fn mark(&self) -> Self::Mark;
    /// Forgets what the installs since `mark` noted (refusals, notes, counts).
    fn rewind(&mut self, mark: &Self::Mark);
    /// Runs the install code of `app`; the session's install view is set.
    fn install(&mut self, session: &mut Session, app: u32);
    /// The tests that touch `tables` (object indexes) are not run, for `why`.
    fn refuse(&mut self, tables: BTreeSet<u32>, why: String);
    /// A line for the baseline notes.
    fn note(&mut self, line: String);
}

/// The partner apps of one install group.
pub(crate) struct PartnerGroup<'a> {
    pub view: &'a InstallView,
    /// The partner apps that install one after the other.
    pub group: BTreeSet<u32>,
    /// The apps that are not installed while they install.
    pub held: BTreeSet<u32>,
    /// The apps of `group` with install code, in the order the plan lists them (dependency order).
    pub apps: Vec<u32>,
}

impl PartnerGroup<'_> {
    /// Two apps whose install order is not known: no dependency path between them, and the run does not list the order.
    fn unordered(&self, a: u32, b: u32) -> bool {
        !self.view.listed() && a != b && !self.view.needs(a, b) && !self.view.needs(b, a)
    }

    fn run(&self, session: &mut Session, inst: &mut impl AppInstalls, sequence: &[u32], fixed: &[u32]) {
        for &app in sequence {
            self.view.apply_ordered(session, app, &self.group, &self.held, fixed);
            inst.install(session, app);
        }
    }

    /// The apps of `group` met in a refusal of the install order since the last call.
    fn hits(&self, session: &mut Session) -> BTreeSet<u32> {
        session
            .take_order_unknown_hits()
            .into_iter()
            .filter(|(a, b)| self.group.contains(a) && self.group.contains(b))
            .flat_map(|(a, b)| [a, b])
            .collect()
    }

    /// The order of `apps` in which the apps of `fixed` install in that order: dependency order, else the plan's.
    fn sequence(&self, fixed: &[u32]) -> Vec<u32> {
        let rank = |a: u32| (self.apps.iter().position(|&x| x == a).unwrap_or(usize::MAX), a);
        let before = |x: u32, y: u32| {
            self.view.needs(y, x)
                || matches!((fixed.iter().position(|&f| f == x), fixed.iter().position(|&f| f == y)), (Some(i), Some(j)) if i < j)
        };
        let mut left: Vec<u32> = self.group.iter().copied().collect();
        left.sort_by_key(|&a| rank(a));
        let mut out = Vec::new();
        while !left.is_empty() {
            let k = left
                .iter()
                .position(|&a| !left.iter().any(|&b| b != a && before(b, a)))
                .unwrap_or(0);
            out.push(left.remove(k));
        }
        out.retain(|a| self.apps.contains(a));
        out
    }
}

/// The orders of `apps` that no dependency path rules out.
fn orders(group: &PartnerGroup, apps: &BTreeSet<u32>) -> Vec<Vec<u32>> {
    fn permute(rest: &[u32], at: &mut Vec<u32>, out: &mut Vec<Vec<u32>>) {
        if rest.is_empty() {
            out.push(at.clone());
        }
        for i in 0..rest.len() {
            let mut r = rest.to_vec();
            at.push(r.remove(i));
            permute(&r, at, out);
            at.pop();
        }
    }
    let v: Vec<u32> = apps.iter().copied().collect();
    let mut all = Vec::new();
    permute(&v, &mut Vec::new(), &mut all);
    all.retain(|o| (0..o.len()).all(|i| (i + 1..o.len()).all(|j| !group.view.needs(o[i], o[j]))));
    all
}

fn names(lowered: &Lowered, apps: &BTreeSet<u32>) -> String {
    apps.iter()
        .map(|&a| lowered.program.apps.get(a as usize).map_or("?", |x| x.name.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Installs the partner apps of `g`. Without two of them that have no dependency path between them, once each, in plan
/// order. Otherwise once with the install order of such pairs not known (as before); when that install code met another
/// partner app of the group (an event its subscriber would get, `NavApp.GetModuleInfo`, a table extension trigger), the
/// apps met are installed again in every order (at most [`MAX_INTERACTING`] apps) from the database before them, and the
/// databases compared ([`al_vm::Baseline::install_order_differences`]). The session ends on the database of the last order.
pub(crate) fn install_partner_apps<I: AppInstalls>(
    lowered: &Lowered,
    session: &mut Session,
    g: &PartnerGroup,
    inst: &mut I,
) {
    let may_interact = g.apps.iter().any(|&a| g.group.iter().any(|&b| g.unordered(a, b)));
    if !may_interact {
        g.run(session, inst, &g.apps, &[]);
        return;
    }
    let start = session.capture_baseline();
    let clock = session.clock_offset_ms();
    let mark = inst.mark();
    session.take_order_unknown_hits();
    g.run(session, inst, &g.apps, &[]);
    let mut met = g.hits(session);
    if met.is_empty() {
        return;
    }
    let rewind = |session: &mut Session, inst: &mut I| {
        session.rewind_install(&start);
        session.set_clock_offset_ms(clock);
        inst.rewind(&mark);
        session.reset_single_instances();
    };
    'grow: loop {
        if met.len() > MAX_INTERACTING {
            break 'grow;
        }
        let mut ends = Vec::new();
        for order in orders(g, &met) {
            rewind(session, inst);
            // The clock readings around this order: a value inside them was read from the clock (al-runner issue 6644 b).
            let from = session.clock_now_millis();
            g.run(session, inst, &g.sequence(&order), &order);
            let new = g.hits(session);
            if !new.is_subset(&met) {
                met.extend(new);
                continue 'grow;
            }
            ends.push((session.capture_baseline(), (from, session.clock_now_millis())));
        }
        let mut lines: std::collections::BTreeMap<u32, Vec<String>> = std::collections::BTreeMap::new();
        for (e, clock) in ends.iter().skip(1) {
            match e.install_order_differences(&ends[0].0, &start, &session.engine, (*clock, ends[0].1)) {
                Some(d) => {
                    for (t, l) in d {
                        lines.entry(t).or_default().extend(l);
                    }
                }
                None => break 'grow,
            }
        }
        let differ: BTreeSet<u32> = lines.keys().copied().collect();
        let table_name = |t: u32| lowered.program.objects.get(t as usize).map_or("?", |o| o.name.as_str());
        // Every difference by table, key and field (`--verbose` prints the baseline notes; al-runner issue 6644 b).
        let mut detail = Vec::new();
        for (t, ls) in &lines {
            let mut ls = ls.clone();
            ls.sort();
            ls.dedup();
            for l in ls {
                detail.push(format!("{}, {l}", table_name(*t)));
            }
        }
        let who = names(lowered, &met);
        inst.note(format!(
            "{INSTALL_ORDERS_NOTE}{who}: no dependency path between them and their install code meets the other's code, so \
             their install code ran in {} orders; {}",
            ends.len(),
            if differ.is_empty() {
                "every order left the same database".to_string()
            } else {
                format!("the orders left different rows in {} table(s)", differ.len())
            }
        ));
        for l in &detail {
            inst.note(format!("{INSTALL_ORDER_DIFFERENCE_NOTE}{l}"));
        }
        if !differ.is_empty() {
            let tables = differ.iter().map(|&t| table_name(t)).collect::<Vec<_>>().join(", ");
            let first: Vec<&str> = detail.iter().take(SHOWN_DIFFERENCES).map(String::as_str).collect();
            inst.refuse(
                differ,
                format!(
                    "the install order of apps {who} is not known (no dependency path between them), and installing them \
                     in each order leaves different rows in {tables} (ADR-046 amendment 7; {}{})",
                    first.join("; "),
                    if detail.len() > SHOWN_DIFFERENCES {
                        format!("; {} more, --verbose lists them", detail.len() - SHOWN_DIFFERENCES)
                    } else {
                        String::new()
                    }
                ),
            );
        }
        return;
    }
    // Too many apps, or a difference outside the tables: once more with the order not known, as before.
    rewind(session, inst);
    g.run(session, inst, &g.apps, &[]);
    session.take_order_unknown_hits();
    inst.note(format!(
        "{INSTALL_ORDERS_NOTE}{}: no dependency path between them and their install code meets the other's code; {}, so \
         what it reaches stays refused",
        names(lowered, &met),
        if met.len() > MAX_INTERACTING {
            format!("more than {MAX_INTERACTING} apps, whose every order the runner does not try")
        } else {
            "the orders differ in more than table rows (isolated storage, number sequences or companies)".to_string()
        }
    ));
}
