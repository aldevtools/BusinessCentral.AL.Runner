//! One root of the dependency apps' install baseline (`baseline::run_traced`): an install routine, Company-Initialize or a
//! step of the company build, run with what the session sees set by the caller. Split from `baseline.rs`, so that the
//! partner apps' roots can run again in another install order (`install_orders.rs`, ADR-046 amendment 7).

use al_builtins::native::Signal;
use al_sema::lower::Lowered;
use al_vm::Session;
use std::collections::BTreeSet;

/// What the roots of one baseline run share and count.
pub(crate) struct RootRun<'a> {
    pub lowered: &'a Lowered,
    /// (object, routine, keep what it wrote when it stops early, no company chosen)
    pub roots: &'a [(usize, usize, bool, bool)],
    /// The positions in `roots` of the steps that build the company.
    pub company_roots: &'a [usize],
    pub company_initialize_root: Option<usize>,
    /// The roots from the first to the last of the pair run without a client (`CompanyData::no_client`).
    pub no_client: Option<(Option<usize>, Option<usize>)>,
    pub company_name: String,
    pub notes: &'a mut Vec<String>,
    /// Per root that stopped: the tables it writes, and whether its rows before the stop are kept.
    pub stopped: &'a mut Vec<(BTreeSet<u32>, bool)>,
    pub ran: usize,
    pub steps_built: usize,
    pub step_failed: bool,
    /// Tables whose rows depend on the install order of partner apps, and why (`install_orders.rs`).
    pub refused: Vec<(BTreeSet<u32>, String)>,
}

impl RootRun<'_> {
    /// Runs root `k`.
    pub(crate) fn run_root(&mut self, session: &mut Session, k: usize) {
        let company_name = self.company_name.clone();
        self.run_root_in_company(session, k, &company_name);
    }

    fn run_root_in_company(&mut self, session: &mut Session, k: usize, company_name: &str) {
        let objs = &self.lowered.program.objects;
        let (oi, ri, keep_prefix, no_company) = self.roots[k];
        let is_step = self.company_roots.contains(&k);
        if is_step && self.step_failed {
            // BCApps' script stops at the step that failed; the company is blocked already.
            return;
        }
        if is_step && self.company_roots.first() != Some(&k) {
            // Each step is a session of its own (`Invoke-NavContainerCodeunit`): SingleInstance state starts again.
            session.reset_single_instances();
        }
        let without_client = self
            .no_client
            .is_some_and(|(from, to)| from.is_some_and(|f| f <= k) && to.is_some_and(|t| k <= t));
        session.set_no_client(without_client);
        // An app's install code runs in BC's install session, which has no client (`Session::set_install_without_client`);
        // Company-Initialize and the company build keep their own rules.
        session.set_install_without_client(!is_step && self.company_initialize_root != Some(k));
        let label = format!("{}.{}", objs[oi].name, objs[oi].routines[ri].name);
        if no_company {
            session.set_company("");
        }
        session.clear_fault_site();
        // A Message while the company is built goes to the run log (`company_data::MESSAGE_NOTE`).
        if is_step {
            session.capture_messages();
            session.set_company_build(true);
        }
        let called = if crate::baseline::is_install(&objs[oi]) || keep_prefix {
            let inst = session.instance(oi);
            session.call(oi, ri, inst, Vec::new()).map(|_| ())
        } else {
            session.call_parameterless_subscriber(oi, ri)
        };
        session.set_no_client(false);
        session.set_install_without_client(false);
        if is_step {
            session.set_company_build(false);
            for m in session.stop_capturing_messages() {
                self.notes.push(format!("{}{m}", crate::company_data::MESSAGE_NOTE));
            }
        }
        let why = match called {
            Ok(_) => None,
            // Where it stopped: the innermost routine and its source position.
            Err(Signal::Unsupported(r)) => Some(match session.fault_site().last() {
                Some(f) => {
                    let at = crate::explain::describe(&self.lowered.program, f.object, f.routine, f.pos);
                    format!("{r} [in {} at {}]", at.who, at.at)
                }
                None => r.into_reason(),
            }),
            Err(Signal::Error(e)) => Some(format!("failed with an error: {}", e.message)),
            Err(_) => None,
        };
        session.set_company(company_name);
        match why {
            None => {
                self.ran += 1;
                self.steps_built += usize::from(is_step);
                session.commit_transaction();
            }
            Some(why) => {
                if keep_prefix {
                    session.commit_transaction();
                } else {
                    session.rollback_transaction();
                }
                if is_step {
                    self.step_failed = true;
                    self.notes.push(format!(
                        "{}{label} did not run to its end: {why}",
                        crate::company_data::BLOCKED_NOTE
                    ));
                } else {
                    self.notes.push(format!(
                        "{label} did not run to its end{}: {why}",
                        if keep_prefix {
                            " (the rows before it are kept)"
                        } else {
                            ""
                        }
                    ));
                }
                self.stopped.push((
                    crate::baseline::tables_written_or_loaded(self.lowered, oi, ri),
                    keep_prefix,
                ));
            }
        }
    }

    /// ADR-046 amendment 9: backup-backed dependency installs visit selected companies, then the database part.
    pub(crate) fn run_app(&mut self, session: &mut Session, roots: &[usize]) {
        let company_roots: Vec<_> = roots.iter().copied().filter(|&k| !self.roots[k].3).collect();
        let database_roots: Vec<_> = roots.iter().copied().filter(|&k| self.roots[k].3).collect();
        if company_roots.is_empty() || session.engine.test_data.is_none() || session.companies().len() < 2 {
            for k in roots {
                self.run_root(session, *k);
            }
            return;
        }

        let companies = if session.companies().is_empty() {
            vec![self.company_name.clone()]
        } else {
            session.companies().to_vec()
        };
        for company in companies {
            session.reset_single_instances();
            session.set_company(&company);
            match crate::install::company_open(self.lowered, session, true) {
                Ok(()) => {
                    session.commit_transaction();
                    for &k in &company_roots {
                        self.run_root_in_company(session, k, &company);
                    }
                }
                Err(why) => {
                    session.rollback_transaction();
                    let mut tables = BTreeSet::new();
                    let not_installed = session.apps_not_installed().cloned().unwrap_or_default();
                    for (oi, ri) in
                        crate::install::company_open_subscribers(self.lowered, &session.engine.events, |oi| {
                            !not_installed.contains(&self.lowered.program.objects[oi].app)
                        })
                    {
                        tables.extend(crate::baseline::tables_written_or_loaded(self.lowered, oi, ri));
                    }
                    for &k in &company_roots {
                        let (oi, ri, keep_prefix, _) = self.roots[k];
                        tables.extend(crate::baseline::tables_written_or_loaded(self.lowered, oi, ri));
                        let label = format!(
                            "{}.{}",
                            self.lowered.program.objects[oi].name, self.lowered.program.objects[oi].routines[ri].name
                        );
                        self.notes.push(format!(
                            "{label} did not run to its end: Company Triggers before it could not run: {why}"
                        ));
                        self.stopped.push((
                            crate::baseline::tables_written_or_loaded(self.lowered, oi, ri),
                            keep_prefix,
                        ));
                    }
                    self.stopped.push((tables, false));
                }
            }
        }
        if !database_roots.is_empty() {
            session.reset_single_instances();
            for k in database_roots {
                self.run_root(session, k);
            }
        }
        session.set_company(&self.company_name);
    }
}

/// The partner apps' roots `roots[from..to]`, for `install_orders::install_partner_apps`.
pub(crate) struct DependencyInstalls<'r, 'a> {
    pub run: &'r mut RootRun<'a>,
    pub from: usize,
    pub to: usize,
}

impl crate::install_orders::AppInstalls for DependencyInstalls<'_, '_> {
    type Mark = (usize, usize, usize, usize);

    fn mark(&self) -> Self::Mark {
        (
            self.run.notes.len(),
            self.run.stopped.len(),
            self.run.ran,
            self.run.refused.len(),
        )
    }

    fn rewind(&mut self, m: &Self::Mark) {
        self.run.notes.truncate(m.0);
        self.run.stopped.truncate(m.1);
        self.run.ran = m.2;
        self.run.refused.truncate(m.3);
    }

    fn install(&mut self, session: &mut Session, app: u32) {
        let roots: Vec<_> = (self.from..self.to)
            .filter(|&k| self.run.lowered.program.objects[self.run.roots[k].0].app == app)
            .collect();
        self.run.run_app(session, &roots);
    }

    fn refuse(&mut self, tables: BTreeSet<u32>, why: String) {
        self.run.refused.push((tables, why));
    }

    fn note(&mut self, line: String) {
        self.run.notes.push(line);
    }
}
