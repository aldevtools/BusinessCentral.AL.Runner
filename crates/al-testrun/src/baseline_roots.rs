//! One root of the dependency apps' install baseline (`baseline::run_traced`): an install routine, Company-Initialize or a
//! step of the company build, run with what the session sees set by the caller. Split from `baseline.rs`, so that the
//! partner apps' roots can run again in another install order (`install_orders.rs`, ADR-046 amendment 7).

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
    pub refused: Vec<(BTreeSet<u32>, String)>,
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
        let relied_on_group1 = session.end_group1_install_routine(called.is_ok());
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
                Some(match session.fault_site().last() {
                    Some(f) => {
                        let at = crate::explain::describe(&self.lowered.program, f.object, f.routine, f.pos);
                        (reason.clone(), format!("{reason} [in {} at {}]", at.who, at.at))
                    }
                    None => (reason.clone(), reason),
                })
            }
            Err(Signal::Error(e)) => {
                let text = format!("failed with an error: {}", e.message);
                Some((text.clone(), text))
            }
            Err(_) => None,
        };
        session.set_company(&self.company_name);
        match why {
            None => {
                if relied_on_group1 {
                    // It relied on the group 1 assumption (al-vm `group1_sessions.rs`): the tables main refused for it,
                    // as for a root that did not run to its end, are flagged.
                    session.flag_group1_tables(crate::baseline::tables_written_or_loaded(self.lowered, oi, ri));
                }
                self.ran += 1;
                self.steps_built += usize::from(is_step);
                session.commit_transaction();
            }
            Some((reason, why)) => {
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
                    tables: crate::baseline::tables_written_or_loaded(self.lowered, oi, ri),
                    kept: keep_prefix,
                    cause: format!("{label} stopped: {reason}"),
                });
            }
        }
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
        for k in self.from..self.to {
            if self.run.lowered.program.objects[self.run.roots[k].0].app == app {
                self.run.run_root(session, k);
            }
        }
    }

    fn refuse(&mut self, tables: BTreeSet<u32>, why: String) {
        self.run.refused.push((tables, why));
    }

    fn note(&mut self, line: String) {
        self.run.notes.push(line);
    }
}
