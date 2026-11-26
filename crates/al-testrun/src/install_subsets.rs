//! Unit runs (ADR-079 decision 3): the proof that a partner group's install leaves the same database in every order.
//!
//! The units of the influence graph (`install_proof.rs`) are proven one by one. A unit with inputs, or a tangle of two
//! or three apps, installs once per order of its apps and per place of each input app among them ([`plan`]; for a unit
//! of one app whose inputs are of one app each, once per subset S of its inputs: S, then the unit, then the inputs not
//! in S; the apps of an input of several apps are placed one by one): from the database before the group, the units it
//! reads from through others first, then the run of the plan, each in a fixed order. The run also installs the
//! dependency predecessors of these apps and the units those read from (the run set, [`Proof::run_set`]), each at its
//! place in the fixed order (ADR-079 decision 3, clarification of 2026-11-23). While an app installs, an app of the run
//! placed before it is installed; one placed after it is not installed only when the round's graph has the edge "the
//! installing app reads from it" (`InstallView::of_ordered`), else it stays of unknown order. The other apps of the
//! group are not part of the run and stay of unknown order. A meet with an app of unknown order is refused and noted.
//! The end databases are compared with `Baseline::install_order_differences` (ADR-046 amendments 7 and 8; the
//! orchestrator's clarification of decision 3 in ADR-079: the whole database, a superset of what the unit leaves).
//! Every run's footprints count for the graph: a run that adds an edge starts the proof again on the grown graph; a
//! meet that adds none stops it ([`next_round`]). A final install of the whole group in an order of the units (or,
//! where the dependencies rule that out, the plan order: `PartnerGroup::final_order`), every app's place fixed, ends
//! the session; its footprints are checked the same way. The tables that differ are closed over the footprints
//! ([`tainted`]): an install that read one may have written other rows from it.

use crate::install_orders::{
    AppInstalls, INSTALL_ORDER_DIFFERENCE_NOTE, INSTALL_ORDERS_NOTE, MAX_INPUTS, MAX_INTERACTING, MAX_UNIT_RUNS,
    PartnerGroup, SHOWN_DIFFERENCES, names, orders,
};
use crate::install_proof::{Footprints, Graph, everything_lines, tie};
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
    /// How many unit runs ran.
    runs: usize,
    /// Every difference a comparison measured, in every round: each is a difference between two real install orders.
    measured: BTreeSet<Measured>,
}

/// A difference between two install orders, measured by the runs of one unit.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Measured {
    /// The tables that differ.
    pub tables: BTreeSet<u32>,
    /// The refusal text, with the first [`SHOWN_DIFFERENCES`] differences.
    pub why: String,
    /// The apps of the unit (their own writes were compared in every run, so a read of those tables does not taint them).
    pub own: BTreeSet<u32>,
    /// Every difference, one baseline note each: the refusal's "--verbose lists them" (al-runner issue 9058).
    pub lines: Vec<String>,
}

/// Refuses the tables of each of `measured` and notes its every difference, whichever end the proof took
/// ([`Settled::apply`], [`Unproven::apply`]). Gives the sources for [`tainted`].
pub(crate) fn refuse_measured<'m>(
    measured: impl IntoIterator<Item = &'m Measured>,
    inst: &mut impl AppInstalls,
) -> Vec<Source> {
    let mut sources = Vec::new();
    for m in measured {
        for l in &m.lines {
            inst.note(l.clone());
        }
        sources.push((m.tables.clone(), m.own.clone()));
        inst.refuse(m.tables.clone(), m.why.clone());
    }
    sources
}

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
    /// Lines for the baseline notes besides `why`: every line `why` shows only the first of.
    detail: Vec<String>,
}

/// The graph of one round of the proof and its units.
struct Round<'r> {
    graph: &'r Graph,
    units: &'r [Vec<u32>],
}

enum UnitProof {
    /// The unit's orders and the places of its inputs were compared.
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
            let units = graph.units(&self.g.group, |a, b| self.g.view.needs(a, b));
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
            let wanted: Vec<u32> = units.iter().flat_map(|u| self.first_order(u)).collect();
            // The view and the run take the same order (ADR-079 decision 3, clarification of 2026-11-23, round 3).
            let fixed = self.g.final_order(&wanted).map_err(|apps| self.ruled(apps))?;
            let hit = (self.g).recorded_run(
                session,
                inst,
                &self.g.sequence(&fixed),
                &fixed,
                &graph.reads,
                &mut self.fp,
            );
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
            detail: Vec::new(),
            apps: BTreeSet::from([x, y]),
            why: format!(
                "the install of {} met {}, whose order the run left unknown, although the proof counted that meet \
                 already",
                self.name(x),
                self.name(y)
            ),
        })
    }

    /// The apps of the group a run of the apps `start` installs: those, the apps they depend on, the other apps of their
    /// units, and the units those read from (directly or through others), until nothing is added. A unit comes whole:
    /// an app of a tangle meets the others, and one left out stays of unknown order (two tangles whose dependencies
    /// cross; issue 8907).
    fn run_set(&self, round: &Round, start: &[u32]) -> BTreeSet<u32> {
        let mut set = self.g.closure(start);
        loop {
            let mut more: Vec<u32> = set.iter().copied().collect();
            for u in round.units.iter().filter(|u| u.iter().any(|a| set.contains(a))) {
                more.extend(u);
                more.extend(round.graph.ancestors(round.units, u).into_iter().flatten());
            }
            let more = self.g.closure(&more);
            if more == set {
                return set;
            }
            set = more;
        }
    }

    /// [`PartnerGroup::order`] of `fixed`. An order the dependencies rule out is not run, so the proof stops there
    /// (ADR-079 decision 3, clarification of 2026-11-23, review round 3).
    fn order(&self, fixed: &[u32]) -> Result<Vec<u32>, Unproven> {
        self.g.order(fixed).map_err(|apps| self.ruled(apps))
    }

    fn ruled(&self, apps: BTreeSet<u32>) -> Unproven {
        Unproven {
            detail: Vec::new(),
            why: format!(
                "the proof needs {} installed in an order their dependencies rule out",
                names(self.lowered, &apps)
            ),
            apps,
        }
    }

    /// [`plan`] has up to `n` runs for the unit `own` (with its inputs, `who`), more than [`MAX_UNIT_RUNS`].
    fn too_many(&self, n: usize, own: &BTreeSet<u32>, who: &BTreeSet<u32>) -> Unproven {
        Unproven {
            detail: Vec::new(),
            apps: who.clone(),
            why: format!(
                "placing the inputs of {} among its apps takes up to {n} runs, more than the {MAX_UNIT_RUNS} the runner makes \
                 for one unit",
                names(self.lowered, own)
            ),
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
            return Err(self.too_large(graph, unit, own));
        }
        if inputs.len() > MAX_INPUTS {
            return Err(Unproven {
                detail: Vec::new(),
                apps: who,
                why: format!(
                    "the install of {} reads from {} apps or groups of apps, more than the {MAX_INPUTS} the runner \
                     places among the apps of one unit",
                    names(self.lowered, &own),
                    inputs.len()
                ),
            });
        }
        let before: Vec<u32> = (graph.ancestors(units, unit).into_iter())
            .filter(|v| !inputs.contains(v))
            .flat_map(|v| self.first_order(v))
            .collect();
        let placed: Vec<u32> = inputs.iter().flat_map(|v| self.first_order(v)).collect();
        let reads = |a: u32, y: u32| graph.reads.contains(&(a, y));
        let runs = plan(&orders(self.g, &own), &placed, reads, |a, b| self.g.view.needs(a, b))
            .map_err(|n| self.too_many(n, &own, &who))?;
        let mut ends = Vec::new();
        for planned in runs {
            let mut fixed = before.clone();
            fixed.extend(planned);
            rewind(session, inst);
            // The clock readings around this run: a value inside them was read from the clock (amendment 8).
            let from = session.clock_now_millis();
            let run = self.run_set(round, &fixed);
            let fixed: Vec<u32> = (self.order(&fixed)?.into_iter()).filter(|a| run.contains(a)).collect();
            let sequence: Vec<u32> = (fixed.iter().copied()).filter(|a| self.g.apps.contains(a)).collect();
            let hit = (self.g).recorded_run(session, inst, &sequence, &fixed, &graph.reads, &mut self.fp);
            self.runs += 1;
            if self.grew(graph, hit)? {
                return Ok(UnitProof::Grew);
            }
            ends.push((session.capture_baseline(), (from, session.clock_now_millis())));
        }
        self.compare(session, &own, &who, &inputs, &ends, out)?;
        Ok(UnitProof::Done)
    }

    /// Compares the end databases of one unit's runs: the summary note goes to `out`, a difference to `measured` (its lines
    /// with it, so every end of the proof notes them: [`refuse_measured`]).
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
                    detail: Vec::new(),
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
        if !differ.is_empty() {
            let tables = differ.iter().map(|&t| table_name(t)).collect::<Vec<_>>().join(", ");
            let first: Vec<&str> = detail.iter().take(SHOWN_DIFFERENCES).map(String::as_str).collect();
            let more = if detail.len() > SHOWN_DIFFERENCES {
                format!("; {} more, --verbose lists them", detail.len() - SHOWN_DIFFERENCES)
            } else {
                String::new()
            };
            let why = format!(
                "the install order of apps {} is not known (no dependency path between them), and installing them in \
                 each order leaves different rows in {tables} (ADR-079 decision 3; {}{more})",
                names_of(who),
                first.join("; ")
            );
            self.measured.insert(Measured {
                tables: differ,
                why,
                own: own.clone(),
                lines: detail
                    .iter()
                    .map(|l| format!("{INSTALL_ORDER_DIFFERENCE_NOTE}{l}"))
                    .collect(),
            });
        }
        Ok(())
    }

    /// The refusal of a unit of more than [`MAX_INTERACTING`] apps: what ties them. When installs touch everything, each
    /// reason with the code it was noted from and the installs that reached it ([`everything_lines`]; the first
    /// [`SHOWN_DIFFERENCES`] in the reason, every one in the notes), else one tie between two of them.
    fn too_large(&self, graph: &Graph, unit: &[u32], own: BTreeSet<u32>) -> Unproven {
        let program = &self.lowered.program;
        let place = |site: &[al_vm::FaultFrame]| match site {
            [] => String::new(),
            [_] => crate::explain::site_suffix(program, site),
            [first, ..] => {
                let from = crate::explain::describe(program, first.object, first.routine, first.pos);
                format!(
                    "{}, reached from {} at {}",
                    crate::explain::site_suffix(program, site),
                    from.who,
                    from.at
                )
            }
        };
        let lines = everything_lines(&self.fp, &own, place, |apps| names(self.lowered, apps));
        let tied = if lines.is_empty() {
            let (x, y) = (graph.reads.iter().chain(&graph.collide))
                .copied()
                .find(|(x, y)| own.contains(x) && own.contains(y))
                .unwrap_or((unit[0], unit[1]));
            tie(&self.fp, x, y, |a| self.name(a), |t| self.table_name(t))
        } else {
            let more = match lines.len().checked_sub(SHOWN_DIFFERENCES) {
                Some(n) if n > 0 => format!("; {n} more, --verbose lists them"),
                _ => String::new(),
            };
            let first: Vec<&str> = lines.iter().take(SHOWN_DIFFERENCES).map(String::as_str).collect();
            format!("installs that touch everything: {}{more}", first.join("; "))
        };
        Unproven {
            why: format!(
                "{} apps tied by their installs ({tied}), more than the {MAX_INTERACTING} whose every order the runner \
                 tries",
                unit.len()
            ),
            detail: (lines.into_iter())
                .map(|l| format!("{INSTALL_ORDERS_NOTE}touches everything: {l}"))
                .collect(),
            apps: own,
        }
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
        let sources = refuse_measured(&self.refusals, inst);
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
    /// comparison measured before the proof stopped, its lines noted ([`refuse_measured`]); the tables written by every
    /// install of the group that reads what another unordered app's install wrote, or writes what another writes (every
    /// table when one of them touches everything or writes a number sequence or isolated storage); and what [`tainted`]
    /// adds to those.
    pub(crate) fn apply(self, lowered: &Lowered, g: &PartnerGroup, proof: &Proof, inst: &mut impl AppInstalls) {
        let why = format!(
            "{INSTALL_ORDERS_NOTE}{}: the runner cannot prove that every order of the apps with no dependency path \
             between them leaves the same database: {}; what their install code reaches stays refused",
            names(lowered, &self.apps),
            self.why
        );
        inst.note(why.clone());
        for line in self.detail {
            inst.note(line);
        }
        let mut sources = refuse_measured(&proof.measured, inst);
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

/// ADR-079 decision 3, clarification of 2026-11-23 (issue 8907): the runs of a unit with inputs, as orders of the unit's
/// apps with each app of its inputs (`inputs`, in install order) placed among them. Per order of the unit's apps
/// (`orders`), an input app's place counts only relative to the apps of the unit that read from it (`reads(a, y)`):
/// every other app of the unit commutes with it (neither reads from the other and they write nothing in common, else
/// they would be one unit). So an input app gets one place per count c of its readers it installs after (c = 0 to all
/// of them), at the earliest slot of that class the dependencies allow (`needs(a, b)`: a depends on b): after every app
/// of the unit it depends on, before every app of the unit that depends on it, and not before an input app it depends
/// on. For a unit of one app with inputs of one app each these are the subsets of the inputs. A class no such slot fits
/// holds no order BC gives, so it is skipped (in the measured shape, a tangle whose input depends on one of its apps,
/// main before ADR-079 passes; neighbor app `install-order-footprint-tangle-dependent-input`). The apps of an input are
/// placed one by one: on BC they can install on both sides of an app of the unit. The first run puts every input app
/// after its readers. Once there are more runs than [`MAX_UNIT_RUNS`], `Err` with the count of classes (the runs before
/// the skips): the plan stops there and does not place the remaining classes.
pub(crate) fn plan(
    orders: &[Vec<u32>],
    inputs: &[u32],
    reads: impl Fn(u32, u32) -> bool,
    needs: impl Fn(u32, u32) -> bool,
) -> Result<Vec<Vec<u32>>, usize> {
    let k = inputs.len();
    let readers: Vec<Vec<Vec<usize>>> = (orders.iter())
        .map(|order| {
            let t = order.len();
            (inputs.iter())
                .map(|&y| (0..t).filter(|&p| reads(order[p], y)).collect())
                .collect()
        })
        .collect();
    let classes = |r: &Vec<Vec<usize>>| r.iter().fold(1usize, |n, r| n.saturating_mul(r.len() + 1));
    let bound = readers.iter().map(classes).fold(0usize, usize::saturating_add);
    let mut runs = Vec::new();
    for (order, readers) in orders.iter().zip(&readers) {
        let t = order.len();
        for code in 0..classes(readers) {
            let (mut lo, mut hi) = (vec![0; k], vec![t; k]);
            let mut rest = code;
            for (i, r) in readers.iter().enumerate() {
                let c = r.len() - rest % (r.len() + 1);
                rest /= r.len() + 1;
                lo[i] = if c == 0 { 0 } else { r[c - 1] + 1 };
                hi[i] = if c == r.len() { t } else { r[c] };
                for (p, &a) in order.iter().enumerate() {
                    if needs(inputs[i], a) {
                        lo[i] = lo[i].max(p + 1);
                    }
                    if needs(a, inputs[i]) {
                        hi[i] = hi[i].min(p);
                    }
                }
            }
            for _ in 0..k {
                for i in 0..k {
                    for j in 0..k {
                        if needs(inputs[i], inputs[j]) {
                            lo[i] = lo[i].max(lo[j]);
                        }
                    }
                }
            }
            if (0..k).any(|i| lo[i] > hi[i]) {
                continue;
            }
            let mut run = Vec::new();
            for s in 0..=t {
                run.extend((0..k).filter(|&i| lo[i] == s).map(|i| inputs[i]));
                run.extend(order.get(s));
            }
            runs.push(run);
            if runs.len() > MAX_UNIT_RUNS {
                return Err(bound);
            }
        }
    }
    Ok(runs)
}

/// Whether the proof starts another round after a run: `after`, the graph with the run's footprints, has an edge
/// `before` lacks. A meet of an app of unknown order (`hit`) that added no edge cannot change in another round
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
