//! The install order of apps with no dependency path between them (ADR-046 amendment 7, owner 2026-11-08, al-runner
//! issue 6644). BC keeps no order for such apps (measured on 27.5, 28.5 and 29.0, owner 2026-10-30), and there is no flag
//! to state one (owner, 2026-11-02). The runner orders them so:
//!
//! 1. Microsoft's apps install before partner apps ([`split_microsoft`]): a container's database comes from the BC
//!    artifact with Microsoft's apps installed, and a pipeline publishes and installs the partner apps into it afterwards.
//! 2. Partner apps of one group that may interact install once, each app's install session recorded (its footprint,
//!    al-vm `install_footprint.rs`). When no install reads from or collides with another's (`install_proof.rs`), that
//!    install stands. Else the install is proven independent of the order by subset runs (ADR-079,
//!    `install_subsets.rs`): the same database in every subset and order, the run goes on; different rows, the tests that
//!    touch a table that differs are not run, with the apps and the table named. Beyond the limits of ADR-079 decision 5
//!    ([`MAX_INTERACTING`] apps in a tangle, [`MAX_INPUTS`] inputs of a unit), or a difference outside the tables, the
//!    runner refuses as before.

use crate::install::InstallView;
use crate::install_proof::{Footprints, Graph};
use al_sema::lower::Lowered;
use al_vm::Session;
use al_vm::install_footprint::InstallFootprint;
use std::collections::BTreeSet;

/// The most partner apps of one tangle whose every order the runner tries (3! = 6 orders). Beyond it, it refuses as
/// before (ADR-079 decision 5).
pub(crate) const MAX_INTERACTING: usize = 3;

/// The most inputs of one unit whose every subset the runner installs before it (2^3 = 8 subset runs; ADR-079
/// decision 5). Beyond it, it refuses as before.
pub(crate) const MAX_INPUTS: usize = 3;

/// The start of the baseline note that says which partner apps were installed in several orders (`--timings`).
pub const INSTALL_ORDERS_NOTE: &str = "install order: ";

/// The start of a baseline note that names one difference between two install orders by table, key and field.
pub const INSTALL_ORDER_DIFFERENCE_NOTE: &str = "install order difference: ";

/// How many differences the reason of a refusal names; the baseline notes list every one.
pub(crate) const SHOWN_DIFFERENCES: usize = 3;

/// Whether app `app` (index in `Program::apps`) counts as Microsoft's: its manifest's publisher is `Microsoft`, or no
/// manifest gave a publisher (then the app keeps the rules of amendment 6: no partner rule without the fact).
fn is_microsoft(lowered: &Lowered, app: u32) -> bool {
    lowered.program.apps.get(app as usize).is_none_or(|a| {
        let p = a.publisher.trim();
        p.is_empty() || p.eq_ignore_ascii_case("Microsoft")
    })
}

/// Whether app `app` is a partner app: its manifest names a publisher, and it is not Microsoft ([`is_microsoft`]).
pub(crate) fn is_partner(lowered: &Lowered, app: u32) -> bool {
    !is_microsoft(lowered, app)
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
    pub(crate) fn unordered(&self, a: u32, b: u32) -> bool {
        !self.view.listed() && a != b && !self.view.needs(a, b) && !self.view.needs(b, a)
    }

    fn run(&self, session: &mut Session, inst: &mut impl AppInstalls, sequence: &[u32], fixed: &[u32]) {
        for &app in sequence {
            self.view.apply_ordered(session, app, &self.group, &self.held, fixed);
            inst.install(session, app);
        }
    }

    /// [`PartnerGroup::run`] with each app's install session recorded and added to its footprint in `fp`; an app of
    /// the group that an install-order refusal met counts as met (`Session::take_order_unknown_hits`). Whether such a
    /// refusal was met: then an install stopped there.
    pub(crate) fn recorded_run(
        &self,
        session: &mut Session,
        inst: &mut impl AppInstalls,
        sequence: &[u32],
        fixed: &[u32],
        fp: &mut Footprints,
    ) -> bool {
        let mut hit = false;
        for &app in sequence {
            self.view.apply_ordered(session, app, &self.group, &self.held, fixed);
            session.install_footprint_start();
            inst.install(session, app);
            // The install session ends; the safety net of the footprint compares committed rows.
            session.commit_transaction();
            let mut f = session.install_footprint_take().unwrap_or_else(|| {
                InstallFootprint::touching_everything("no footprint was recorded for the install session")
            });
            for (_, other) in session.take_order_unknown_hits() {
                if self.group.contains(&other) {
                    f.apps_met.insert(other);
                    hit = true;
                }
            }
            fp.entry(app).or_default().extend(f);
        }
        hit
    }

    /// The influence graph of the group's apps from `fp`.
    pub(crate) fn graph(&self, fp: &Footprints) -> Graph {
        Graph::of(&self.group, fp, |x, y| self.unordered(x, y))
    }

    /// The apps of the group a run of the apps `fixed` installs: those and the apps they depend on.
    pub(crate) fn closure(&self, fixed: &[u32]) -> BTreeSet<u32> {
        (self.group.iter().copied())
            .filter(|&a| fixed.contains(&a) || fixed.iter().any(|&f| self.view.needs(f, a)))
            .collect()
    }

    /// The order of `apps` in which the apps of `fixed` install in that order: dependency order, else the plan's.
    pub(crate) fn sequence(&self, fixed: &[u32]) -> Vec<u32> {
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
pub(crate) fn orders(group: &PartnerGroup, apps: &BTreeSet<u32>) -> Vec<Vec<u32>> {
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

pub(crate) fn names(lowered: &Lowered, apps: &BTreeSet<u32>) -> String {
    apps.iter()
        .map(|&a| lowered.program.apps.get(a as usize).map_or("?", |x| x.name.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Installs the partner apps of `g`. Without two of them that have no dependency path between them, once each, in plan
/// order. Otherwise once with the install order of such pairs not known (as before), each app's install session
/// recorded (its footprint, al-vm `install_footprint.rs`). When no install reads from or collides with another's
/// (`install_proof::Graph`), that install stands. Else the install is proven independent of the order by subset runs
/// (ADR-079, `install_subsets.rs`), and the session ends on a final install in an order the proof covers; what the
/// proof cannot cover stays refused, as before.
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
    let mut fp = Footprints::new();
    let hit = g.recorded_run(session, inst, &g.apps, &[], &mut fp);
    if !hit && g.graph(&fp) == Graph::default() {
        return;
    }
    let rewind = |session: &mut Session, inst: &mut I| {
        session.rewind_install(&start);
        session.set_clock_offset_ms(clock);
        inst.rewind(&mark);
        session.reset_single_instances();
    };
    let mut proof = crate::install_subsets::Proof::new(lowered, g, &start, fp);
    match proof.settle(session, inst, &rewind) {
        Ok(settled) => settled.apply(lowered, g, inst),
        Err(unproven) => {
            // Once more with the order not known, as before: a meet stays refused where it is reached.
            rewind(session, inst);
            g.run(session, inst, &g.apps, &[]);
            session.take_order_unknown_hits();
            unproven.apply(lowered, g, &proof, inst);
        }
    }
}
