//! Subset runs (ADR-079 decision 3): the proof that a partner group's install leaves the same database in every order.
//!
//! The units of the influence graph (`install_proof.rs`) are proven one by one. A unit with inputs, or a tangle of two
//! or three apps, installs once per subset S of its inputs and per order of its apps: from the database before the
//! group, the units it reads from through others first, then S, then the unit, then the inputs not in S, each in a fixed
//! order; the other apps of the group stay of unknown order, so a meet with one is refused and noted. The end databases
//! are compared with `Baseline::install_order_differences` (ADR-046 amendments 7 and 8; the orchestrator's
//! clarification of decision 3 in ADR-079: the whole database, a superset of what the unit leaves). Every run's
//! footprints count for the graph: a run that meets an app of unknown order, or adds an edge, starts the proof again on
//! the grown graph. A final install of the whole group in an order of the units, every app's place fixed, ends the
//! session; its footprints are checked the same way.

use crate::install_orders::{
    AppInstalls, INSTALL_ORDER_DIFFERENCE_NOTE, INSTALL_ORDERS_NOTE, MAX_INPUTS, MAX_INTERACTING, PartnerGroup,
    SHOWN_DIFFERENCES, names, orders,
};
use crate::install_proof::{Footprints, Graph, tie};
use al_sema::lower::Lowered;
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
}

/// A proven group: what the baseline notes say and which tables differ between the orders.
#[derive(Default)]
pub(crate) struct Settled {
    notes: Vec<String>,
    refusals: Vec<(BTreeSet<u32>, String)>,
    units: usize,
    tangles: usize,
    runs: usize,
}

/// Apps the proof cannot cover, and why (a limit of decision 5, or a difference outside the tables).
pub(crate) struct Unproven {
    apps: BTreeSet<u32>,
    why: String,
}

/// What proving one unit gave.
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
        }
    }

    /// Proves every unit, then installs the whole group once in an order of the units. Footprints only grow, so the
    /// proof starts again at most once per edge the graph can have.
    pub(crate) fn settle<I: AppInstalls>(
        &mut self,
        session: &mut Session,
        inst: &mut I,
        rewind: &impl Fn(&mut Session, &mut I),
    ) -> Result<Settled, Unproven> {
        let n = self.g.group.len();
        for _ in 0..n * n + 2 {
            let graph = self.g.graph(&self.fp);
            let units = graph.units(&self.g.group);
            let mut out = Settled {
                units: units.len(),
                tangles: units.iter().filter(|u| u.len() > 1).count(),
                ..Default::default()
            };
            let mut grew = false;
            let round = Round {
                graph: &graph,
                units: &units,
            };
            for unit in &units {
                if let UnitProof::Grew = self.prove_unit(session, inst, rewind, &round, unit, &mut out)? {
                    grew = true;
                    break;
                }
            }
            if grew {
                continue;
            }
            rewind(session, inst);
            let fixed: Vec<u32> = units.iter().flat_map(|u| self.first_order(u)).collect();
            let hit = self
                .g
                .recorded_run(session, inst, &self.g.sequence(&fixed), &fixed, &mut self.fp);
            if hit || self.g.graph(&self.fp) != graph {
                continue;
            }
            out.runs = self.runs;
            return Ok(out);
        }
        Err(Unproven {
            apps: self.g.group.clone(),
            why: format!("the footprints of the installs still grew after {} rounds", n * n + 2),
        })
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
                    "{} apps whose installs read from each other or write the same resource ({}), more than the \
                     {MAX_INTERACTING} whose every order the runner tries",
                    unit.len(),
                    tie(&self.fp, x, y, |a| self.name(a))
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
                let closure = self.g.closure(&fixed);
                let sequence: Vec<u32> = (self.g.sequence(&fixed).into_iter())
                    .filter(|a| closure.contains(a))
                    .collect();
                let hit = self.g.recorded_run(session, inst, &sequence, &fixed, &mut self.fp);
                self.runs += 1;
                if hit || self.g.graph(&self.fp) != *graph {
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
        &self,
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
        let table_name = |t: u32| {
            self.lowered
                .program
                .objects
                .get(t as usize)
                .map_or("?", |o| o.name.as_str())
        };
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
            out.refusals.push((
                differ,
                format!(
                    "the install order of apps {} is not known (no dependency path between them), and installing them \
                     in each order leaves different rows in {tables} (ADR-079 decision 3; {}{more})",
                    names_of(who),
                    first.join("; ")
                ),
            ));
        }
        Ok(())
    }

    fn name(&self, app: u32) -> String {
        names(self.lowered, &BTreeSet::from([app]))
    }
}

impl Settled {
    /// Adds the notes and refusals of the proof, after the final install (a rewind forgets what came before it).
    pub(crate) fn apply(self, lowered: &Lowered, g: &PartnerGroup, inst: &mut impl AppInstalls) {
        for n in self.notes {
            inst.note(n);
        }
        for (tables, why) in self.refusals {
            inst.refuse(tables, why);
        }
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
    /// reached. An unproven app whose install reads what another unordered app's install wrote, or writes what another
    /// writes, refuses the tests that touch the tables those installs wrote (every table when one of them touches
    /// everything or writes a number sequence or isolated storage): a run with the order not known does not refuse that.
    pub(crate) fn apply(self, lowered: &Lowered, g: &PartnerGroup, proof: &Proof, inst: &mut impl AppInstalls) {
        let why = format!(
            "{INSTALL_ORDERS_NOTE}{}: no dependency path between them, and the runner cannot prove that every install \
             order leaves the same database: {}; what their install code reaches stays refused",
            names(lowered, &self.apps),
            self.why
        );
        inst.note(why.clone());
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
            .filter(|(x, y)| self.apps.contains(x) || self.apps.contains(y))
            .flat_map(|&(x, y)| [x, y])
            .collect();
        if tied.is_empty() {
            return;
        }
        let tables = written_tables(&data, &tied).unwrap_or_else(|| {
            (lowered.program.objects.iter().enumerate())
                .filter(|(_, o)| o.kind == al_ir::ObjKind::Table)
                .map(|(k, _)| k as u32)
                .collect()
        });
        inst.refuse(tables, why);
    }
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
