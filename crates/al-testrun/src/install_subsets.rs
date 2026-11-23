//! Subset runs (ADR-079 decision 3): the proof that a partner group's install leaves the same database in every order.
//!
//! The units of the influence graph (`install_proof.rs`) are proven one by one. A unit with inputs, or a tangle of two
//! or three apps, installs once per subset S of its inputs and per order of its apps: from the database before the
//! group, the units it reads from through others first, then S, then the unit, then the inputs not in S, each in a fixed
//! order. The run also installs the dependency predecessors of these apps and the units those read from (the run set,
//! [`Proof::run_set`]), each at its place in the fixed order, so every app the run installs sees every other at a known
//! place (ADR-079 decision 3, clarification of 2026-11-23). The other apps of the group are not part of the run and
//! stay of unknown order, so a meet with one is refused and noted. The end databases are compared with
//! `Baseline::install_order_differences` (ADR-046 amendments 7 and 8; the orchestrator's clarification of decision 3
//! in ADR-079: the whole database, a superset of what the unit leaves). Every run's footprints count for the graph: a
//! run that adds an edge starts the proof again on the grown graph; a meet that adds none stops it ([`next_round`]).
//! A final install of the whole group in an order of the units, every app's place fixed, ends the session; its
//! footprints are checked the same way. The tables that differ are closed over the footprints ([`tainted`]): an
//! install that read one may have written other rows from it.

use crate::install_orders::{
    AppInstalls, INSTALL_ORDER_DIFFERENCE_NOTE, INSTALL_ORDERS_NOTE, MAX_INPUTS, MAX_INTERACTING, PartnerGroup,
    SHOWN_DIFFERENCES, names, orders,
};
use crate::install_proof::{Footprints, Graph, tie};
use al_sema::lower::Lowered;
use al_vm::install_footprint::InstallFootprint;
use al_vm::install_footprint::Lasting;
use al_vm::{Baseline, Session};
use std::collections::{BTreeMap, BTreeSet};

/// The proof in progress for one partner group.
pub(crate) struct Proof<'p, 'g> {
    lowered: &'p Lowered,
    g: &'p PartnerGroup<'g>,
    start: &'p Baseline,
    /// The footprint of each app, over every run so far.
    fp: Footprints,
    /// How many subset runs ran.
    runs: usize,
    /// Every difference a comparison measured, in every round: each is a difference between two real install orders.
    /// (tables, reason, the apps of the unit whose runs measured it).
    measured: BTreeSet<Measured>,
}

/// A difference between two install orders: the tables that differ, the reason, and the apps of the unit whose runs
/// measured it (their own writes were compared in every run, so a read of those tables does not taint them).
type Measured = (BTreeSet<u32>, String, BTreeSet<u32>);

/// A proven group: what the baseline notes say and which tables differ between the orders.
#[derive(Default)]
pub(crate) struct Settled {
    notes: Vec<String>,
    refusals: Vec<Measured>,
    units: usize,
    tangles: usize,
    runs: usize,
}

/// Apps the proof cannot cover, and why (a limit of decision 5, or a difference outside the tables).
pub(crate) struct Unproven {
    apps: BTreeSet<u32>,
    why: String,
}

/// The graph of one round of the proof and its units.
struct Round<'r> {
    graph: &'r Graph,
    units: &'r [Vec<u32>],
}

enum UnitProof {
    /// The unit's orders and subsets were compared.
    Done,
    /// A run met an app of unknown order or added an edge: the proof starts again.
    Grew,
}

impl<'p, 'g> Proof<'p, 'g> {
    pub(crate) fn new(lowered: &'p Lowered, g: &'p PartnerGroup<'g>, start: &'p Baseline, fp: Footprints) -> Self {
        Proof {
            lowered,
            g,
            start,
            fp,
            runs: 0,
            measured: BTreeSet::new(),
        }
    }

    /// Proves every unit, then installs the whole group once in an order of the units. A round starts again only when a
    /// run added an edge ([`next_round`]); footprints only grow and the graph has a bounded number of edges, so the
    /// proof ends.
    pub(crate) fn settle<I: AppInstalls>(
        &mut self,
        session: &mut Session,
        inst: &mut I,
        rewind: &impl Fn(&mut Session, &mut I),
    ) -> Result<Settled, Unproven> {
        'round: loop {
            let graph = self.g.graph(&self.fp);
            let units = graph.units(&self.g.group);
            let mut out = Settled {
                units: units.len(),
                tangles: units.iter().filter(|u| u.len() > 1).count(),
                ..Default::default()
            };
            let round = Round {
                graph: &graph,
                units: &units,
            };
            for unit in &units {
                if let UnitProof::Grew = self.prove_unit(session, inst, rewind, &round, unit, &mut out)? {
                    continue 'round;
                }
            }
            rewind(session, inst);
            let fixed: Vec<u32> = units.iter().flat_map(|u| self.first_order(u)).collect();
            let hit = self
                .g
                .recorded_run(session, inst, &self.g.sequence(&fixed), &fixed, &mut self.fp);
            if self.grew(&graph, hit)? {
                continue;
            }
            out.runs = self.runs;
            out.refusals = std::mem::take(&mut self.measured).into_iter().collect();
            return Ok(out);
        }
    }

    /// Whether the last run added an edge to `graph` ([`next_round`]).
    fn grew(&self, graph: &Graph, hit: Option<(u32, u32)>) -> Result<bool, Unproven> {
        next_round(graph, &self.g.graph(&self.fp), hit).map_err(|(x, y)| Unproven {
            apps: BTreeSet::from([x, y]),
            why: format!(
                "the install of {} met {}, which the run did not install, although the proof counted that meet \
                 already",
                self.name(x),
                self.name(y)
            ),
        })
    }

    /// The apps of the group a run of the apps `start` installs: those, the apps they depend on, and the units those read
    /// from (directly or through others), until nothing is added.
    fn run_set(&self, round: &Round, start: &[u32]) -> BTreeSet<u32> {
        let mut set = self.g.closure(start);
        loop {
            let mut more: Vec<u32> = set.iter().copied().collect();
            for u in round.units.iter().filter(|u| u.iter().any(|a| set.contains(a))) {
                more.extend(round.graph.ancestors(round.units, u).into_iter().flatten());
            }
            let more = self.g.closure(&more);
            if more == set {
                return set;
            }
            set = more;
        }
    }

    /// The first order of `unit`'s apps that no dependency path rules out.
    fn first_order(&self, unit: &[u32]) -> Vec<u32> {
        let set: BTreeSet<u32> = unit.iter().copied().collect();
        orders(self.g, &set).into_iter().next().unwrap_or_else(|| unit.to_vec())
    }

    fn prove_unit<I: AppInstalls>(
        &mut self,
        session: &mut Session,
        inst: &mut I,
        rewind: &impl Fn(&mut Session, &mut I),
        round: &Round,
        unit: &[u32],
        out: &mut Settled,
    ) -> Result<UnitProof, Unproven> {
        let Round { graph, units } = *round;
        let inputs = graph.inputs(units, unit);
        if unit.len() == 1 && inputs.is_empty() {
            return Ok(UnitProof::Done);
        }
        let own: BTreeSet<u32> = unit.iter().copied().collect();
        let who: BTreeSet<u32> = own.iter().chain(inputs.iter().copied().flatten()).copied().collect();
        if unit.len() > MAX_INTERACTING {
            let (x, y) = (graph.reads.iter().chain(&graph.collide))
                .copied()
                .find(|(x, y)| own.contains(x) && own.contains(y))
                .unwrap_or((unit[0], unit[1]));
            return Err(Unproven {
                apps: own,
                why: format!(
                    "{} apps tied by their installs ({}), more than the {MAX_INTERACTING} whose every order the runner \
                     tries",
                    unit.len(),
                    tie(&self.fp, x, y, |a| self.name(a), |t| self.table_name(t))
                ),
            });
        }
        if inputs.len() > MAX_INPUTS {
            return Err(Unproven {
                apps: who,
                why: format!(
                    "the install of {} reads from {} apps or groups of apps, more than the {MAX_INPUTS} whose every \
                     subset the runner installs before it",
                    names(self.lowered, &own),
                    inputs.len()
                ),
            });
        }
        let before: Vec<u32> = (graph.ancestors(units, unit).into_iter())
            .filter(|v| !inputs.contains(v))
            .flat_map(|v| self.first_order(v))
            .collect();
        let mut ends = Vec::new();
        for mask in 0..1usize << inputs.len() {
            let chosen = |k: usize| mask & (1 << k) != 0;
            for order in orders(self.g, &own) {
                let mut fixed = before.clone();
                for (_, v) in inputs.iter().enumerate().filter(|(k, _)| chosen(*k)) {
                    fixed.extend(self.first_order(v));
                }
                fixed.extend(&order);
                for (_, v) in inputs.iter().enumerate().filter(|(k, _)| !chosen(*k)) {
                    fixed.extend(self.first_order(v));
                }
                rewind(session, inst);
                // The clock readings around this run: a value inside them was read from the clock (amendment 8).
                let from = session.clock_now_millis();
                let run = self.run_set(round, &fixed);
                let fixed: Vec<u32> = (self.g.order(&fixed).into_iter()).filter(|a| run.contains(a)).collect();
                let sequence: Vec<u32> = (fixed.iter().copied()).filter(|a| self.g.apps.contains(a)).collect();
                let hit = self.g.recorded_run(session, inst, &sequence, &fixed, &mut self.fp);
                self.runs += 1;
                if self.grew(graph, hit)? {
                    return Ok(UnitProof::Grew);
                }
                ends.push((session.capture_baseline(), (from, session.clock_now_millis())));
            }
        }
        self.compare(session, &own, &who, &inputs, &ends, out)?;
        Ok(UnitProof::Done)
    }

    /// Compares the end databases of one unit's runs and adds the notes and refusals to `out`.
    fn compare(
        &mut self,
        session: &Session,
        own: &BTreeSet<u32>,
        who: &BTreeSet<u32>,
        inputs: &[&Vec<u32>],
        ends: &[(Baseline, (i64, i64))],
        out: &mut Settled,
    ) -> Result<(), Unproven> {
        let mut lines: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        for (e, clock) in ends.iter().skip(1) {
            let d = e.install_order_differences(&ends[0].0, self.start, &session.engine, (*clock, ends[0].1));
            let Some(d) = d else {
                return Err(Unproven {
                    apps: who.clone(),
                    why: "the orders differ in more than table rows (isolated storage, number sequences or companies)"
                        .into(),
                });
            };
            for (t, l) in d {
                lines.entry(t).or_default().extend(l);
            }
        }
        let differ: BTreeSet<u32> = lines.keys().copied().collect();
        let table_name = |t: u32| self.table_name(t);
        // Every difference by table, key and field (`--verbose` prints the baseline notes; al-runner issue 6644 b).
        let mut detail = Vec::new();
        for (t, ls) in &lines {
            let mut ls = ls.clone();
            ls.sort();
            ls.dedup();
            detail.extend(ls.into_iter().map(|l| format!("{}, {l}", table_name(*t))));
        }
        let names_of = |s: &BTreeSet<u32>| names(self.lowered, s);
        let read: BTreeSet<u32> = inputs.iter().copied().flatten().copied().collect();
        let how = if read.is_empty() {
            format!(
                "the installs of {} read from each other or write the same resource",
                names_of(own)
            )
        } else {
            format!("the install of {} reads from {}", names_of(own), names_of(&read))
        };
        out.notes.push(format!(
            "{INSTALL_ORDERS_NOTE}{}: no dependency path between them and {how} (ADR-079), so their install code ran in \
             {} orders; {}",
            names_of(who),
            ends.len(),
            if differ.is_empty() {
                "every order left the same database".to_string()
            } else {
                format!("the orders left different rows in {} table(s)", differ.len())
            }
        ));
        out.notes
            .extend(detail.iter().map(|l| format!("{INSTALL_ORDER_DIFFERENCE_NOTE}{l}")));
        if !differ.is_empty() {
            let tables = differ.iter().map(|&t| table_name(t)).collect::<Vec<_>>().join(", ");
            let first: Vec<&str> = detail.iter().take(SHOWN_DIFFERENCES).map(String::as_str).collect();
            let more = if detail.len() > SHOWN_DIFFERENCES {
                format!("; {} more, --verbose lists them", detail.len() - SHOWN_DIFFERENCES)
            } else {
                String::new()
            };
            self.measured.insert((
                differ,
                format!(
                    "the install order of apps {} is not known (no dependency path between them), and installing them \
                     in each order leaves different rows in {tables} (ADR-079 decision 3; {}{more})",
                    names_of(who),
                    first.join("; ")
                ),
                own.clone(),
            ));
        }
        Ok(())
    }

    fn table_name(&self, t: u32) -> String {
        (self.lowered.program.objects.get(t as usize)).map_or_else(|| "?".into(), |o| o.name.clone())
    }

    fn name(&self, app: u32) -> String {
        names(self.lowered, &BTreeSet::from([app]))
    }
}

impl Settled {
    /// Adds the notes and refusals of the proof, after the final install (a rewind forgets what came before it), and the
    /// tables an install wrote after it read one that differs ([`tainted`]).
    pub(crate) fn apply(self, lowered: &Lowered, g: &PartnerGroup, proof: &Proof, inst: &mut impl AppInstalls) {
        for n in self.notes {
            inst.note(n);
        }
        let mut sources = Vec::new();
        for (tables, why, own) in self.refusals {
            sources.push((tables.clone(), own));
            inst.refuse(tables, why);
        }
        refuse_tainted(lowered, g, &proof.fp, &sources, inst);
        inst.note(format!(
            "{INSTALL_ORDERS_NOTE}{}: {} unit(s), {} tangle(s), {} subset run(s); proven by footprints (ADR-079)",
            names(lowered, &g.group),
            self.units,
            self.tangles,
            self.runs
        ));
    }
}

impl Unproven {
    /// Notes why the proof stopped. The install ran again with the order not known, so a meet stays refused where it is
    /// reached. Refused besides, since a run with the order not known does not refuse them: every difference a
    /// comparison measured before the proof stopped; the tables written by every install of the group that reads what
    /// another unordered app's install wrote, or writes what another writes (every table when one of them touches
    /// everything or writes a number sequence or isolated storage); and what [`tainted`] adds to those.
    pub(crate) fn apply(self, lowered: &Lowered, g: &PartnerGroup, proof: &Proof, inst: &mut impl AppInstalls) {
        let why = format!(
            "{INSTALL_ORDERS_NOTE}{}: the runner cannot prove that every order of the apps with no dependency path \
             between them leaves the same database: {}; what their install code reaches stays refused",
            names(lowered, &self.apps),
            self.why
        );
        inst.note(why.clone());
        let mut sources = Vec::new();
        for (tables, w, own) in &proof.measured {
            sources.push((tables.clone(), own.clone()));
            inst.refuse(tables.clone(), w.clone());
        }
        // The edges a run with the order not known does not refuse: footprints without the meets.
        let data: Footprints = (proof.fp.iter())
            .map(|(&a, f)| {
                let mut f = f.clone();
                f.apps_met.clear();
                (a, f)
            })
            .collect();
        let edges = g.graph(&data);
        let tied: BTreeSet<u32> = (edges.reads.iter().chain(&edges.collide))
            .flat_map(|&(x, y)| [x, y])
            .collect();
        if !tied.is_empty() {
            match written_tables(&data, &tied) {
                Some(tables) => {
                    sources.push((tables.clone(), BTreeSet::new()));
                    inst.refuse(tables, why);
                }
                None => {
                    inst.refuse(every_table(lowered), why);
                    return;
                }
            }
        }
        refuse_tainted(lowered, g, &proof.fp, &sources, inst);
    }
}

/// Whether the proof starts another round after a run: `after`, the graph with the run's footprints, has an edge
/// `before` lacks. A meet of an app the run did not install (`hit`) that added no edge cannot change in another round
/// (the same graph gives the same runs): `Err` with that pair, the proof stops.
pub(crate) fn next_round(before: &Graph, after: &Graph, hit: Option<(u32, u32)>) -> Result<bool, (u32, u32)> {
    if after != before {
        return Ok(true);
    }
    match hit {
        Some(pair) => Err(pair),
        None => Ok(false),
    }
}

/// Tables whose rows differ by the install order, and the apps whose own runs measured that (exempt from the taint).
pub(crate) type Source = (BTreeSet<u32>, BTreeSet<u32>);

/// ADR-079 decision 4, clarification of 2026-11-23: the rows of a source's tables differ by the install order, so an
/// install that read one may have written other rows from it. An install of app R is tainted when it read a table of a
/// source R is not exempt from (a row, its rows, its AutoIncrement counter, the row version, or everything) that an
/// install of another app W of the group wrote, W not installing after R (`after(R, W)`: W depends on R, so R never
/// reads its rows). An app of the unit whose runs measured the difference is exempt: its own writes were compared in
/// every one of those runs. The tables a tainted R wrote become a source R is exempt from, until nothing is added.
/// Ordered and unordered readers alike. Gives the readers and the tables added beyond the sources'; `None` for the
/// tables when a tainted install touches everything or wrote a number sequence, isolated storage or the row version
/// (every table).
pub(crate) fn tainted(
    fp: &Footprints,
    sources: &[Source],
    after: impl Fn(u32, u32) -> bool,
) -> (BTreeSet<u32>, Option<BTreeSet<u32>>) {
    let mut sources = sources.to_vec();
    let given: BTreeSet<u32> = sources.iter().flat_map(|(t, _)| t.iter().copied()).collect();
    let mut readers = BTreeSet::new();
    loop {
        let taints = |r: u32, f: &InstallFootprint| {
            (sources.iter()).any(|(tables, exempt)| {
                !exempt.contains(&r)
                    && tables.iter().any(|&t| {
                        reads_table(f, t) && (fp.iter()).any(|(&w, wf)| w != r && !after(r, w) && writes_table(wf, t))
                    })
            })
        };
        let Some((&r, _)) = (fp.iter()).find(|&(&r, f)| !readers.contains(&r) && taints(r, f)) else {
            let added: BTreeSet<u32> = (sources.iter().flat_map(|(t, _)| t.iter().copied()))
                .filter(|t| !given.contains(t))
                .collect();
            return (readers, Some(added));
        };
        readers.insert(r);
        match written_tables(fp, &BTreeSet::from([r])) {
            Some(w) => sources.push((w, BTreeSet::from([r]))),
            None => return (readers, None),
        }
    }
}

/// Whether the install session `f` read a resource of table `t` (every table when it touches everything).
fn reads_table(f: &InstallFootprint, t: u32) -> bool {
    f.touches_everything()
        || f.reads.iter().any(|l| match l {
            Lasting::Store(r) => r.object() == t,
            Lasting::AutoIncrement(_, x) => *x == t,
            Lasting::RowVersion => true,
            Lasting::Sequence(..) | Lasting::IsolatedStorage(..) => false,
        })
}

/// Whether the install session `f` wrote a resource of table `t` (every table when it touches everything).
fn writes_table(f: &InstallFootprint, t: u32) -> bool {
    f.touches_everything()
        || f.writes.iter().any(|l| match l {
            Lasting::Store(r) => r.object() == t,
            Lasting::AutoIncrement(_, x) => *x == t,
            Lasting::RowVersion | Lasting::Sequence(..) | Lasting::IsolatedStorage(..) => false,
        })
}

/// Refuses the tables [`tainted`] adds to `sources`, naming the installs that read a table that differs.
fn refuse_tainted(
    lowered: &Lowered,
    g: &PartnerGroup,
    fp: &Footprints,
    sources: &[Source],
    inst: &mut impl AppInstalls,
) {
    // W installs after R when W depends on R.
    let (readers, added) = tainted(fp, sources, |r, w| g.view.needs(w, r));
    if readers.is_empty() {
        return;
    }
    let table_name = |t: u32| (lowered.program.objects.get(t as usize)).map_or_else(|| "?".into(), |o| o.name.clone());
    let differ: BTreeSet<u32> = sources.iter().flat_map(|(t, _)| t.iter().copied()).collect();
    let read = differ.iter().map(|&t| table_name(t)).collect::<Vec<_>>().join(", ");
    let (tables, wrote) = match added {
        Some(t) if t.is_empty() => return,
        Some(t) => {
            let list = t.iter().map(|&x| table_name(x)).collect::<Vec<_>>().join(", ");
            (t, format!("the rows they wrote in {list} may differ too"))
        }
        None => (every_table(lowered), "every table may differ".to_string()),
    };
    inst.refuse(
        tables,
        format!(
            "the installs of {} read rows of {read}, which differ by the install order of apps with no dependency path \
             between them, so {wrote} (ADR-079 decision 4, clarification of 2026-11-23)",
            names(lowered, &readers)
        ),
    );
}

/// Every table of the program (object indexes).
fn every_table(lowered: &Lowered) -> BTreeSet<u32> {
    (lowered.program.objects.iter().enumerate())
        .filter(|(_, o)| o.kind == al_ir::ObjKind::Table)
        .map(|(k, _)| k as u32)
        .collect()
}

/// The tables (object indexes) the installs of `apps` wrote; `None` when one of them touches everything or wrote
/// something that is not a table's (a number sequence, isolated storage).
fn written_tables(fp: &Footprints, apps: &BTreeSet<u32>) -> Option<BTreeSet<u32>> {
    let mut out = BTreeSet::new();
    for f in apps.iter().filter_map(|a| fp.get(a)) {
        if f.touches_everything() {
            return None;
        }
        for w in &f.writes {
            match w {
                Lasting::Store(r) => out.insert(r.object()),
                Lasting::AutoIncrement(_, t) => out.insert(*t),
                Lasting::Sequence(..) | Lasting::IsolatedStorage(..) | Lasting::RowVersion => return None,
            };
        }
    }
    Some(out)
}

#[cfg(test)]
#[path = "install_subsets_tests.rs"]
mod tests;
