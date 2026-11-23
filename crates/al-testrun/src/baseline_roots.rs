//! One root of the dependency apps' install baseline (`baseline::run_traced`): an install routine, Company-Initialize or a
//! step of the company build, run with what the session sees set by the caller. Split from `baseline.rs`, so that the
//! partner apps' roots can run again in another install order (`install_orders.rs`, ADR-046 amendment 7).
//!
//! One root of the dependency baseline (`RootRun::run_root`: install routine, Company-Initialize or a
//! company build step), and the partner apps' roots as `install_orders::AppInstalls`
//! (`DependencyInstalls`). Entry points: `RootRun`, `DependencyInstalls`.

use al_builtins::native::Signal;
use al_sema::lower::Lowered;
use al_vm::Session;
use std::collections::BTreeSet;

/// A root that stopped, or was not run.
pub(crate) struct StoppedRoot {
    /// The tables it writes.
    pub tables: BTreeSet<u32>,
    /// Whether its rows before the stop are kept.
    pub kept: bool,
    /// The root and why it stopped, without the site of the stop (`Installer.OnInstallAppPerCompany stopped: <reason>`): the
    /// refusal of a test that reads one of its tables names it (al-runner issue 8104).
    pub cause: String,
}

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
    /// The roots that stopped or were not run.
    pub stopped: &'a mut Vec<StoppedRoot>,
    pub ran: usize,
    pub steps_built: usize,
    pub step_failed: bool,
    /// Tables whose rows depend on the install order of partner apps, and why (`install_orders.rs`).
    pub refused: Vec<al_vm::SkippedInstall>,
    /// The app whose install session holds a language or WorkDate the runner does not know, and why
    /// (`stopped_state::roll_back_stopped_install`): its later install roots are not run.
    pub unknown_session: Option<(u32, String)>,
    /// The app whose install session the last root ran in; `None` after a root of no install session (al-vm
    /// `group1_sessions.rs`).
    pub install_app: Option<u32>,
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
        // The app's install code, which runs in the app's install session (not Company-Initialize or a step).
        let install_session = !is_step && self.company_initialize_root != Some(k);
        let app = objs[oi].app;
        if let Some((_, why)) = self
            .unknown_session
            .as_ref()
            .filter(|(a, _)| install_session && *a == app)
        {
            self.notes.push(format!("{label} was not run: {why}"));
            self.stopped.push(StoppedRoot {
                tables: crate::baseline::tables_written_or_loaded(self.lowered, oi, ri),
                kept: false,
                cause: format!("{label} was not run: {why}"),
            });
            return;
        }
        if install_session {
            // Another app's install session begins: the group 1 filters of the one before are gone (al-vm
            // `group1_sessions.rs`). The dependency apps' per-company part runs in the session company.
            if self.install_app != Some(app) {
                session.end_group1_install_session();
                self.install_app = Some(app);
            }
            session.set_group1_install_shape(crate::install::group1_trigger_shape(
                self.lowered,
                !no_company,
                (oi, ri),
            ));
        } else {
            self.install_app = None;
        }
        if no_company {
            session.set_company("");
        }
        session.clear_fault_site();
        let before = session.carried_state();
        let mark = session.store_mut().change_counter();
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
        session.end_group1_install_routine(called.is_ok());
        if is_step {
            session.set_company_build(false);
            for m in session.stop_capturing_messages() {
                self.notes.push(format!("{}{m}", crate::company_data::MESSAGE_NOTE));
            }
        }
        let by_runner = matches!(called, Err(Signal::Unsupported(_)));
        // Why it stopped, and the same with where: the innermost routine and its source position.
        let why = match called {
            Ok(_) => None,
            Err(Signal::Unsupported(r)) => {
                let reason = r.into_reason();
                let at = crate::explain::site_suffix(&self.lowered.program, session.fault_site());
                Some((reason.clone(), format!("{reason}{at}")))
            }
            Err(Signal::Error(e)) => {
                let text = format!("failed with an error: {}", e.message);
                Some((text.clone(), text))
            }
            Err(_) => None,
        };
        session.set_company(company_name);
        match why {
            None => {
                self.ran += 1;
                self.steps_built += usize::from(is_step);
                session.commit_transaction();
            }
            Some((reason, why)) => {
                // The rows it wrote before the stop, through any path (a RecordRef too), are rolled back, and on BC it ran
                // to its end and kept them (review of al-runner PR 8119, probe n19c; the same rule as `install.rs`
                // `Stopped::roll_back`).
                let mut tables = crate::baseline::tables_written_or_loaded(self.lowered, oi, ri);
                if !keep_prefix {
                    tables.extend(session.store_mut().tables_rolled_back_since(mark));
                }
                if keep_prefix {
                    session.commit_transaction();
                } else if let Some(unknown) =
                    crate::stopped_state::roll_back_stopped_install(self.lowered, session, (oi, ri), before, by_runner)
                    && install_session
                {
                    self.unknown_session = Some((app, unknown));
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
                self.stopped.push(StoppedRoot {
                    tables,
                    kept: keep_prefix,
                    cause: format!("{label} stopped: {reason}"),
                });
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
            let mut skipped: Vec<_> = self
                .stopped
                .iter()
                .map(|root| al_vm::SkippedInstall::new(root.tables.clone(), root.cause.clone()))
                .collect();
            match crate::install::company_open(self.lowered, session, &skipped, true) {
                Ok(()) => {
                    session.commit_transaction();
                    for &k in &company_roots {
                        self.run_root_in_company(session, k, &company);
                    }
                }
                Err(stopped) => {
                    let why = stopped.why.clone();
                    let mut tables = BTreeSet::new();
                    let not_installed = session.apps_not_installed().cloned().unwrap_or_default();
                    for (oi, ri) in
                        crate::install::company_open_subscribers(self.lowered, &session.engine.events, |oi| {
                            !not_installed.contains(&self.lowered.program.objects[oi].app)
                        })
                    {
                        tables.extend(crate::baseline::tables_written_or_loaded(self.lowered, oi, ri));
                    }
                    let refused: Vec<_> = company_roots
                        .iter()
                        .map(|&k| {
                            let (oi, ri, _, _) = self.roots[k];
                            (oi, ri)
                        })
                        .collect();
                    let skipped_before = skipped.len();
                    let unknown = stopped.roll_back(
                        self.lowered,
                        session,
                        &mut skipped,
                        (&company, &self.company_name),
                        &refused,
                    );
                    for s in skipped.drain(skipped_before..) {
                        self.stopped.push(StoppedRoot {
                            tables: s.tables,
                            kept: false,
                            cause: s.why,
                        });
                    }
                    if let Some(why) = unknown {
                        if let Some(&k) = company_roots.first() {
                            let (oi, _, _, _) = self.roots[k];
                            self.unknown_session = Some((self.lowered.program.objects[oi].app, why));
                        }
                    }
                    for &k in &company_roots {
                        let (oi, ri, keep_prefix, _) = self.roots[k];
                        let root_tables = crate::baseline::tables_written_or_loaded(self.lowered, oi, ri);
                        tables.extend(root_tables.iter().copied());
                        let label = format!(
                            "{}.{}",
                            self.lowered.program.objects[oi].name, self.lowered.program.objects[oi].routines[ri].name
                        );
                        self.notes.push(format!(
                            "{label} did not run to its end: Company Triggers before it could not run: {why}"
                        ));
                        self.stopped.push(StoppedRoot {
                            tables: root_tables,
                            kept: keep_prefix,
                            cause: format!("{label} stopped: Company Triggers before it could not run: {why}"),
                        });
                    }
                    self.stopped.push(StoppedRoot {
                        tables,
                        kept: false,
                        cause: format!("Company Triggers could not run: {why}"),
                    });
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
    type Mark = (usize, usize, usize, usize, Option<(u32, String)>);

    fn mark(&self) -> Self::Mark {
        (
            self.run.notes.len(),
            self.run.stopped.len(),
            self.run.ran,
            self.run.refused.len(),
            self.run.unknown_session.clone(),
        )
    }

    fn rewind(&mut self, m: &Self::Mark) {
        self.run.notes.truncate(m.0);
        self.run.stopped.truncate(m.1);
        self.run.ran = m.2;
        self.run.refused.truncate(m.3);
        self.run.unknown_session = m.4.clone();
    }

    fn install(&mut self, session: &mut Session, app: u32) {
        // Each install of an app, in every order tried, is an install session of its own.
        self.run.install_app = None;
        let roots: Vec<_> = (self.from..self.to)
            .filter(|&k| self.run.lowered.program.objects[self.run.roots[k].0].app == app)
            .collect();
        self.run.run_app(session, &roots);
    }

    fn refuse(&mut self, tables: BTreeSet<u32>, why: String) {
        self.run.refused.push(al_vm::SkippedInstall::new(tables, why));
    }

    fn note(&mut self, line: String) {
        self.run.notes.push(line);
    }
}
