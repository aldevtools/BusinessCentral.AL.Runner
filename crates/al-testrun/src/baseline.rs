//! The install baseline (ADR-013 decision 5), mirroring AL Runner (`InstallTriggerRunner`,
//! `CompanyInitializer`, `TestExecutor`): once per run, before the first test,
//!
//! 1. every dependency app's install code (install codeunit triggers and `Extension Triggers` subscribers, ADR-046
//!    decision 1), in dependency order; per app every `OnInstallAppPerCompany`, then every `OnInstallAppPerDatabase`;
//! 2. Base Application's `Company-Initialize` (codeunit 2), once, after the dependency installs;
//! 3. the install code of the apps under test, one app after the other (`install.rs`, ADR-046 step A);
//! 4. a capture of the data ([`Baseline`], ADR-004), put back at the start of every test codeunit.
//!
//! Only install triggers run, never upgrade triggers. A trigger that cannot run stops where it
//! stops: `Company-Initialize` keeps the rows it wrote before (as AL Runner's partial company
//! initialization does), an install trigger is rolled back. What did not run is recorded in the
//! engine's `unseeded` set, so that a `Get` that finds nothing in dependency code is refused for
//! exactly those tables instead of answered.

use crate::company_data::CompanyData;
use al_sema::lower::Lowered;
use al_vm::{Baseline, Engine, Session};
use std::collections::BTreeSet;
use std::sync::Arc;

/// What the baseline run leaves.
#[derive(Clone)]
pub struct BaselineRun {
    pub baseline: Baseline,
    /// Tables the roots that did not run (or stopped early) would have written rows to (a record
    /// variable's Insert, Modify, Rename or ModifyAll in the root's own call closure), and that hold no row in the baseline or
    /// belong to a root that was rolled back (al-runner issue 6973), unless the restored company holds their rows: object
    /// indexes.
    pub unseeded: BTreeSet<u32>,
    /// How many tables hold a row in the baseline.
    pub filled: usize,
    /// One line per root that could not run to its end.
    pub notes: Vec<String>,
    /// Roots that ran to their end, and all roots.
    pub ran: usize,
    pub total: usize,
    /// Changes the roots made to rows the restored company holds (allowed, ADR-020 amendment).
    pub restored: Vec<al_data::memory::RestoredChange>,
    /// With [`run_traced`]: what the install code raised and touched (`impact_trace::keys`).
    pub trace: Option<Vec<String>>,
    /// The recorded runs of the login code over this baseline (`install::login_work_date`), kept in its file of the
    /// compile cache so that a later run replays them instead of running the code again.
    pub login: Vec<LoginRecord>,
}

/// One recorded run of the login code that sets WorkDate when a test session opens the company
/// (`install::login_work_date`), over a baseline no table of which changed since: the state it was recorded in (the
/// company and the WorkDate before it ran) and what it left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginRecord {
    pub company: String,
    pub work_date: i32,
    /// The WorkDate the login code answered.
    pub day: i32,
    /// The globals of LogInManagement the run may have changed (global index), and their Date values after it.
    pub globals: Vec<(usize, i32, bool)>,
    /// The assumption marks the run made.
    pub hits: al_builtins::assumptions::HitBits,
}

impl BaselineRun {
    /// One line per table the roots changed restored rows of: how many by kind, and how many
    /// Modify wrote the values the row already had.
    pub fn restored_lines(&self) -> Vec<String> {
        let mut by_table: std::collections::BTreeMap<&str, (usize, usize, usize, usize)> = Default::default();
        let mut fields: std::collections::BTreeMap<&str, std::collections::BTreeSet<&str>> = Default::default();
        for c in &self.restored {
            fields
                .entry(&c.table)
                .or_default()
                .extend(c.fields.iter().map(String::as_str));
            let e = by_table.entry(&c.table).or_default();
            match (c.op, c.same) {
                ("Modify", true) => e.0 += 1,
                ("Modify", false) => e.1 += 1,
                ("Rename", _) => e.2 += 1,
                _ => e.3 += 1,
            }
        }
        by_table
            .into_iter()
            .map(|(t, (same, other, rename, delete))| {
                let names = fields[t].iter().copied().collect::<Vec<_>>().join(", ");
                format!(
                    "install code changed restored rows of {t}: {same} Modify with the values the row has, {other} Modify with other values{}, {rename} Rename, {delete} Delete",
                    if names.is_empty() { String::new() } else { format!(" (fields: {names})") }
                )
            })
            .collect()
    }
}

pub(crate) fn is_install(o: &al_ir::Object) -> bool {
    o.is_kind(al_ir::ObjKind::Codeunit)
        && o.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Install"))
}

pub(crate) fn trigger(o: &al_ir::Object, name: &str) -> Option<usize> {
    o.routines
        .iter()
        .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case(name))
}

const ASSUMPTION_NOTE: &str = "the baseline relied on the unprobed assumption ";

/// What the run reports from the baseline notes besides the notes themselves.
#[derive(Debug, Clone, Default)]
pub struct NoteFacts {
    /// [`assumption_ids`].
    pub assumptions: Vec<String>,
    /// The lines on partner apps installed in several orders (`install_orders::INSTALL_ORDERS_NOTE`, ADR-046 amendment 7).
    pub install_orders: Vec<String>,
}

impl NoteFacts {
    /// Adds the facts of `other` that this does not hold yet (the install baseline of another engine of the run).
    pub fn merge(&mut self, other: NoteFacts) {
        for (mine, theirs) in [
            (&mut self.assumptions, other.assumptions),
            (&mut self.install_orders, other.install_orders),
        ] {
            for x in theirs {
                if !mine.contains(&x) {
                    mine.push(x);
                }
            }
        }
    }

    pub fn of(notes: &[String]) -> NoteFacts {
        NoteFacts {
            assumptions: assumption_ids(notes),
            install_orders: notes
                .iter()
                .filter(|n| n.starts_with(crate::install_orders::INSTALL_ORDERS_NOTE))
                .cloned()
                .collect(),
        }
    }
}

/// The assumption ids named by the notes of a baseline run (they survive the baseline cache as text).
pub fn assumption_ids(notes: &[String]) -> Vec<String> {
    notes
        .iter()
        .filter_map(|n| n.strip_prefix(ASSUMPTION_NOTE))
        .filter_map(|r| r.strip_prefix('`')?.split('`').next().map(String::from))
        .collect()
}

/// The dependency routine a company step runs: the codeunit by id, checked by name, and its method or `OnRun`.
pub(crate) fn step_routine(
    lowered: &Lowered,
    step: &crate::company_data::CompanyStep,
) -> Result<(usize, usize), String> {
    let objs = &lowered.program.objects;
    let Some(oi) = (0..objs.len()).find(|&oi| {
        lowered.dependency[oi]
            && objs.heads()[oi].is_kind(al_ir::ObjKind::Codeunit)
            && objs.heads()[oi].id == Some(step.codeunit)
    }) else {
        return Err(format!(
            "no dependency app has codeunit {} \"{}\"",
            step.codeunit, step.name
        ));
    };
    if !objs[oi].name.eq_ignore_ascii_case(&step.name) {
        return Err(format!(
            "codeunit {} is \"{}\", not \"{}\"",
            step.codeunit, objs[oi].name, step.name
        ));
    }
    match &step.method {
        None => trigger(&objs[oi], "OnRun")
            .map(|ri| (oi, ri))
            .ok_or_else(|| format!("codeunit {} \"{}\" has no OnRun", step.codeunit, objs[oi].name)),
        Some(m) => objs[oi]
            .routines
            .iter()
            .position(|r| !r.is_trigger && r.name.eq_ignore_ascii_case(m) && r.params == 0)
            .map(|ri| (oi, ri))
            .ok_or_else(|| {
                format!(
                    "codeunit {} \"{}\" has no method {m} without parameters",
                    step.codeunit, objs[oi].name
                )
            }),
    }
}

/// Runs the baseline on a fresh session of `engine`. `None`: no dependency app has anything to run.
pub fn run(engine: &Arc<Engine>, lowered: &Lowered, company: Option<&CompanyData>) -> Option<BaselineRun> {
    run_traced(engine, lowered, false, company, &Default::default())
}

/// [`run`], and with `trace` what the install code raised and touched (`BaselineRun::trace`, ADR-043 step 2).
///
/// With `company`, the `OnRun` of its codeunit runs last of the dependency roots, after `Company-Initialize`, with the
/// files of its resource folder (`company_data.rs`). When it does not run to its end, or the program has no such
/// codeunit, the notes say why under [`crate::company_data::BLOCKED_NOTE`].
pub fn run_traced(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    trace: bool,
    company: Option<&CompanyData>,
    perturb: &crate::stability::Perturb,
) -> Option<BaselineRun> {
    run_traced_halting(engine, lowered, trace, company, perturb, None)
}

/// [`run_traced`] that stops its install code once `halt` is set: the run stopped at its hard memory limit (ADR-066
/// amendment 10, decision 6).
pub fn run_traced_halting(
    engine: &Arc<Engine>,
    lowered: &Lowered,
    trace: bool,
    company: Option<&CompanyData>,
    perturb: &crate::stability::Perturb,
    halt: Option<Arc<std::sync::atomic::AtomicBool>>,
) -> Option<BaselineRun> {
    let objs = &lowered.program.objects;
    // (object, routine, keep what it wrote when it stops early, no company chosen)
    //
    // Per dependency app, in dependency order: its install code (install codeunit triggers and "Extension Triggers"
    // subscribers, ADR-046 decision 1), every per-company routine, then every per-database routine, which runs last and
    // with CompanyName() empty. Measured on real BC 27.5 (a fresh install with two companies, a scratch app;
    // docs/probes-needed.md) and pinned by record/BfInstallTests.al:138-140, :239.
    //
    // With a company data codeunit whose recipe installs only some apps first (`CompanyData::installed_first`; Microsoft's
    // Legacy lane, BCApps build/scripts/ImportTestDataInBcContainer.ps1:907-923: `Install-BaseAppsForDemoTool`,
    // `New-TestCompany`, `Invoke-LegacyDemoDataTool`, then `Install-AllApps`), the install code of the other apps runs
    // after the codeunit, and their subscribers do not run before that.
    let late_apps: Option<BTreeSet<u32>> = company.and_then(|c| c.installed_first.as_ref()).map(|first| {
        lowered
            .program
            .apps
            .iter()
            .enumerate()
            .filter(|(_, a)| !first.iter().any(|n| n.eq_ignore_ascii_case(&a.name)))
            .map(|(i, _)| i as u32)
            .collect()
    });
    let is_late = |app: usize| late_apps.as_ref().is_some_and(|l| l.contains(&(app as u32)));
    let mut roots: Vec<(usize, usize, bool, bool)> = Vec::new();
    let mut late_roots: Vec<(usize, usize, bool, bool)> = Vec::new();
    // The test-toolkit apps install after the application apps (`install::TEST_TOOLKIT_APP_IDS`, ADR-046 amendment 6).
    let toolkit = crate::install::test_toolkit_apps(lowered);
    let mut toolkit_roots: Vec<(usize, usize, bool, bool)> = Vec::new();
    // Microsoft's apps install before the partner apps (ADR-046 amendment 7, `install_orders.rs`): the partner apps' roots
    // come after the toolkit's.
    let view = crate::install::InstallView::new(lowered, false);
    let under_test = crate::install::apps_under_test(lowered);
    let late_group = late_apps.clone().unwrap_or_default();
    let early_group: BTreeSet<u32> = (0..lowered.program.apps.len() as u32)
        .filter(|a| !under_test.contains(a) && !late_group.contains(a))
        .collect();
    let (toolkit_group, app_group): (BTreeSet<u32>, BTreeSet<u32>) =
        early_group.iter().copied().partition(|a| toolkit.contains(a));
    let (app_group, partner_group) = crate::install_orders::split_microsoft(lowered, &view, &app_group);
    let mut partner_roots: Vec<(usize, usize, bool, bool)> = Vec::new();
    let mut partner_apps: Vec<u32> = Vec::new();
    for app in crate::install::plan(lowered, &engine.events, true) {
        let into = if is_late(app.app) {
            &mut late_roots
        } else if toolkit.contains(&(app.app as u32)) {
            &mut toolkit_roots
        } else if partner_group.contains(&(app.app as u32)) {
            partner_apps.push(app.app as u32);
            &mut partner_roots
        } else {
            &mut roots
        };
        for &(oi, ri) in &app.company {
            into.push((oi, ri, false, false));
        }
        for &(oi, ri) in &app.database {
            into.push((oi, ri, false, true));
        }
    }
    // The dependency install roots that run before Company-Initialize: `roots[..early_installs]`, the application apps' first,
    // the test-toolkit apps' from `toolkit_from`.
    let toolkit_from = roots.len();
    roots.extend(toolkit_roots);
    let partner_from = roots.len();
    roots.extend(partner_roots);
    let early_installs = roots.len();
    let company_initialize = (0..objs.len()).find(|&oi| {
        lowered.dependency[oi]
            && objs.heads()[oi].is_kind(al_ir::ObjKind::Codeunit)
            && objs.heads()[oi].id == Some(2)
            && objs.heads()[oi].name.eq_ignore_ascii_case("Company-Initialize")
    });
    let mut company_initialize_root: Option<usize> = None;
    if let Some(oi) = company_initialize {
        if let Some(ri) = trigger(&objs[oi], "OnRun") {
            company_initialize_root = Some(roots.len());
            roots.push((oi, ri, true, false));
        }
    }
    // The steps that build the company's data, after Company-Initialize (BCApps' CI: `Invoke-LegacyDemoDataTool` runs
    // codeunit 2, then the DemoTool's action; `Invoke-ContosoDemoTool` runs codeunit 2, then each Contoso step).
    let mut company_roots: Vec<usize> = Vec::new();
    let mut company_missing: Option<String> = None;
    for step in company.map(CompanyData::steps).unwrap_or_default() {
        match step_routine(lowered, &step) {
            Ok((oi, ri)) => {
                company_roots.push(roots.len());
                roots.push((oi, ri, false, false));
            }
            Err(why) => {
                company_missing = Some(why);
                break;
            }
        }
    }
    if company_missing.is_some() {
        // No step runs when one is missing: the company could not be built.
        for &k in company_roots.iter().rev() {
            roots.remove(k);
        }
        company_roots.clear();
    }
    // Without a client (`CompanyData::no_client`): from Company-Initialize to the last step.
    let no_client = company.is_some_and(|c| c.no_client);
    let no_client_from = company_initialize_root.or(company_roots.first().copied());
    let no_client_to = company_roots.last().copied();
    // Where the apps installed after the company is built begin.
    let late_from = late_apps.as_ref().map(|_| roots.len());
    roots.extend(late_roots);
    if roots.is_empty() && company_missing.is_none() {
        return None;
    }

    crate::install::ensure_facts(engine, lowered);
    // A variant of a stability run (ADR-058 section 7): install code sees the variant's host values (clock, seeds,
    // identity, company), as the tests do.
    let host = crate::host::host_perturbed(None, perturb);
    // The company data codeunit reads its resource files through `File`; the rest of the install code sees them too,
    // as every session of BCApps' CI server does once the files are in the user folder.
    let host = match company.and_then(|c| c.resources.clone()) {
        Some(dir) => host.with_files(crate::company_data::ResourceFiles::new(dir)),
        None => host,
    };
    let mut session = Session::with_host(engine.clone(), host);
    // A run that stops at its hard memory limit stops its install code too (ADR-066 amendment 10, decision 6).
    let _halt = crate::runtime::memory::StopBridge::start(halt, session.stop_flag());
    session.set_install_scope(true);
    if trace {
        session.enable_impact_trace();
    }
    // So that a root that stops can say where (fault sites are noted only on an error or a refusal).
    session.trace_faults(true);
    let mut notes = Vec::new();
    // Per root that stopped: the tables it writes, and whether its rows before the stop are kept.
    let mut stopped: Vec<(BTreeSet<u32>, bool)> = Vec::new();
    al_builtins::assumptions::clear_hits();
    let run_environment = session.environment();
    let company_name = session.set_company("");
    session.set_company(&company_name);
    // The company is built on the server kind its recipe names; the apps under test install, and the tests run, on the
    // run's own.
    if let Some(e) = company.and_then(|c| c.environment) {
        session.set_environment(e);
    }
    // The apps under test install after the company exists: while the dependency apps install, Company-Initialize and the
    // company build run, they are not installed, so none of their subscribers runs there (no `OnCompanyInitialize`
    // subscriber, no `OnGetPerCompanyUpgradeTags` subscriber answering `SetAllUpgradeTags`). Their per-company install code
    // then sees `HasUpgradeTag` = No for its own tag and no row its `OnCompanyInitialize` subscriber writes
    // (install-upgrade/BfUpgradeTagInstallTests.al:42-43).
    let mut held = late_apps.clone().unwrap_or_default();
    held.extend(under_test.iter().copied());
    session.set_apps_not_installed(Some(held.clone()));
    // The dependency apps install one at a time as well (`install::InstallView`): Microsoft's application apps, then the
    // test-toolkit apps with every application app installed (ADR-046 amendment 6, owner 2026-10-30), then the partner
    // apps with every Microsoft app installed (amendment 7), then the late ones among themselves. Nothing pins the order of
    // two dependency apps that do not depend on each other (`publish_order_known` covers only the apps under test).
    let mut held_for_apps = held.clone();
    held_for_apps.extend(toolkit_group.iter().copied());
    held_for_apps.extend(partner_group.iter().copied());
    let mut held_for_toolkit = held.clone();
    held_for_toolkit.extend(partner_group.iter().copied());
    if let Some(why) = &company_missing {
        notes.push(format!("{}{why}", crate::company_data::BLOCKED_NOTE));
    }
    // The company is an evaluation company (`New-TestCompany -EvaluationCompany`) from the start, before any install code.
    if company.is_some_and(|c| c.evaluation_company) {
        session.mark_evaluation_company();
    }
    let mut st = crate::baseline_roots::RootRun {
        lowered,
        roots: &roots,
        company_roots: &company_roots,
        company_initialize_root,
        no_client: no_client.then_some((no_client_from, no_client_to)),
        company_name: company_name.clone(),
        notes: &mut notes,
        stopped: &mut stopped,
        ran: 0,
        steps_built: 0,
        step_failed: false,
        refused: Vec::new(),
    };
    let mut k = 0;
    while k < roots.len() {
        if k == partner_from && partner_from < early_installs {
            let group = crate::install_orders::PartnerGroup {
                view: &view,
                group: partner_group.clone(),
                held: held.clone(),
                apps: partner_apps.clone(),
            };
            let mut inst = crate::baseline_roots::DependencyInstalls {
                run: &mut st,
                from: partner_from,
                to: early_installs,
            };
            // What the partner apps' install code may write: a subscriber of one of them whose order is open reads nothing
            // else when it runs under watch (al-vm order_trial.rs, al-runner issue 6972). Microsoft's install code ran
            // before, the same in every order.
            let written = crate::install::tables_install_code_of_may_write(lowered, engine, |o| {
                partner_group.contains(&objs[o].app)
            });
            session.set_install_written_tables(written);
            crate::install_orders::install_partner_apps(lowered, &mut session, &group, &mut inst);
            session.set_install_written_tables(None);
            k = early_installs;
            continue;
        }
        let app = objs[roots[k].0].app;
        if k < toolkit_from {
            view.apply(&mut session, app, &app_group, &held_for_apps);
        } else if k < early_installs {
            view.apply(&mut session, app, &toolkit_group, &held_for_toolkit);
        } else if late_from.is_some_and(|f| k >= f) {
            view.apply(&mut session, app, &late_group, &under_test);
        } else {
            // Company-Initialize and the company build: every early dependency app is installed.
            session.set_apps_not_installed(Some(held.clone()));
            session.set_apps_install_order_unknown(None);
        }
        st.run_root(&mut session, k);
        k += 1;
    }
    let crate::baseline_roots::RootRun {
        ran,
        steps_built,
        refused: order_refused,
        ..
    } = st;
    let company_built = !company_roots.is_empty() && steps_built == company_roots.len();
    session.set_apps_not_installed(None);
    session.set_apps_install_order_unknown(None);
    session.set_environment(run_environment);
    session.trace_faults(false);
    // The baseline is not a test, so no test carries its marks; the run says what the shared start relied on.
    for a in al_builtins::assumptions::take_hits() {
        notes.push(format!(
            "{ASSUMPTION_NOTE}`{}` (--list-assumptions describes it)",
            a.info().id
        ));
    }
    // ADR-046 amendment 4: under `--companies`, a run without test data models the corpus CI database, whose session
    // company is an evaluation company (record/BfInstallTests.al:61). Its Company-Initialize ran
    // with the row of the default mode, which gives Experience Tier Essential there
    // (session-system/company/CompanySetupRows.al:50-66); from here on the apps under test and the tests see the
    // flag. With test data, or with a company the run builds, the row stays as it is.
    if company.is_none() && engine.test_data.is_none() && session.companies().len() > 1 {
        session.mark_evaluation_company();
    }
    // The workspace app's own install codeunits run here, once, so that the data they write is part of what every
    // test codeunit starts from (publishing the app runs them once; they are not re-run for each codeunit).
    let mut workspace_installs = crate::runtime::run_install_codeunits(lowered, &mut session);
    // Tables whose rows depend on the install order of two partner dependency apps (ADR-046 amendment 7): a test that
    // touches one is not run, as for an app under test's install routine that could not run.
    workspace_installs.extend(order_refused);
    let restored = session.restored_changes();
    let trace = trace.then(|| crate::impact_trace::keys(&lowered.program, &session.take_impact_trace()));
    session.set_install_scope(false);
    let mut baseline = session.capture_baseline();
    baseline.workspace_installs = Some(workspace_installs);
    baseline.company_built = company_built;
    let filled = baseline.nonempty_tables();
    let unseeded = unseeded_tables(&stopped, &filled, &session);
    Some(BaselineRun {
        baseline,
        unseeded,
        filled: filled.len(),
        notes,
        ran,
        total: roots.len(),
        restored,
        trace,
        login: Vec::new(),
    })
}

/// The tables that code reachable from `routine` of `object` writes rows to: a record variable's
/// `Insert`, `Modify`, `Rename`, `ModifyAll` or `Validate`-free equivalents, found in the lowered
/// bodies of everything it calls (not of event subscribers). Temporary
/// records are left out. Writes through a `RecordRef` have no static table and are not counted.
/// The Base Application tables whose rows the corpus database's company holds and the runtime does not: Accounting Period
/// (36 rows, 2025-01-01 to 2027-12-01) and Company Information (one row) (session-system/company/
/// CompanyInformationValues.al:45 QG_CompanyInformation_CorpusRow `count` 1, :89 QG_AccountingPeriod_CorpusRows `count` 36).
/// ADR-047 (seed them) is rejected, so without test data a `Count` or `IsEmpty` of one of them before the test's own code
/// wrote to it is refused, the shape those two tests read (install code may have written a row with other values). Other reads stay answered: refusing every read of those tables lost 116 BC 29
/// baseline cases (reports and pages that read Company Information without asserting its values). General Ledger Setup holds
/// one demo row there too (record/captions/QdCaptionClassAreas.al:804 QF_GLSetup_CorpusRow `count` 1, :815).
/// G/L Entry holds demo entries there (test-framework/RlWorkDateAtStart.al:48 `G/L entries exist` Yes, a company
/// whose Company Information says Demo Company = Yes, :47); the runtime's has none.
/// The platform table Media Resources holds 46 rows there (metadata/platform-tables/PtCloudTables.al:269
/// QX_MediaResources_FreshTenantCodes); al-vm records.rs refuses every read of it, not only Count and IsEmpty.
/// Object indexes of the dependency tables 17, 50, 79, 98 and 2000000182.
pub fn demo_row_tables(lowered: &Lowered) -> std::collections::HashSet<usize> {
    let objs = &lowered.program.objects;
    (0..objs.len())
        .filter(|&oi| {
            lowered.dependency[oi]
                && objs.heads()[oi].table.is_some()
                && matches!(
                    (objs.heads()[oi].id, objs.heads()[oi].name.as_str()),
                    (Some(17), "G/L Entry")
                        | (Some(50), "Accounting Period")
                        | (Some(79), "Company Information")
                        | (Some(98), "General Ledger Setup")
                        | (Some(2_000_000_182), "Media Resources")
                )
        })
        .collect()
}

/// The tables the roots that stopped would have written rows to, that the tests may not read as they are. A table the
/// restored company holds rows for is seeded, whether or not a root could write it. A root that stopped and was rolled back
/// wrote none of its rows, so a table it writes is unseeded even when other roots put rows in it: a read that finds no row,
/// or a scan, is not BC's answer (al-runner issue 6973: another app's flags in the table, the stopped install's flag
/// missing). A root whose rows before the stop are kept (`keep_prefix`) counts only for tables that hold no row.
fn unseeded_tables(stopped: &[(BTreeSet<u32>, bool)], filled: &BTreeSet<u32>, session: &Session) -> BTreeSet<u32> {
    stopped
        .iter()
        .flat_map(|(tables, kept)| tables.iter().filter(move |t| !kept || !filled.contains(t)))
        .copied()
        .filter(|&t| !session.backs_rows(t))
        .collect()
}

pub(crate) fn tables_written(lowered: &Lowered, object: usize, routine: usize) -> BTreeSet<u32> {
    tables_written_in(lowered, object, routine, false)
}

/// [`tables_written`] and the tables a `NavApp.LoadPackageData` with a constant table id fills (al-runner issue 7359): what
/// a root that stopped or could not run leaves without the rows BC has. The order analyses of the install routines keep
/// [`tables_written`] (a load writes no SystemCreatedAt the corpus orders).
pub(crate) fn tables_written_or_loaded(lowered: &Lowered, object: usize, routine: usize) -> BTreeSet<u32> {
    tables_written_in(lowered, object, routine, true)
}

fn tables_written_in(lowered: &Lowered, object: usize, routine: usize, loads: bool) -> BTreeSet<u32> {
    use al_builtins::value::RtType;
    use al_ir::{Arg, Expr, Place, Stmt};

    struct W<'a> {
        locals: &'a [al_ir::Slot],
        globals: &'a [al_ir::Slot],
        heads: &'a [al_ir::ObjectHead],
        loads: bool,
        out: &'a mut BTreeSet<u32>,
    }
    impl W<'_> {
        /// `NavApp.LoadPackageData(Database::X)` with a constant table id writes the rows of table X (al-runner issue 7359:
        /// a refused load leaves the table without the rows the app ships, as a stopped root's Insert does).
        fn package_data_table(&mut self, args: &[Arg]) {
            let Some(Arg::Value(Expr::Const(al_ir::Const::Integer(n)))) = args.first() else {
                return;
            };
            let found = self
                .heads
                .iter()
                .position(|o| o.kind == al_ir::ObjKind::Table && o.id.is_some_and(|id| i64::from(id) == i64::from(*n)));
            if let Some(t) = found {
                self.out.insert(t as u32);
            }
        }
        fn root(p: &Place) -> Option<&Place> {
            match p {
                Place::Local(_) | Place::Global(_) => Some(p),
                Place::Index(inner, _) => Self::root(inner),
                // The base object's variable (ADR-023): page and report extension code, which no
                // install routine runs.
                Place::Field(..) | Place::BaseGlobal(_) | Place::ExtGlobal(..) | Place::RecGlobal(..) => None,
            }
        }
        fn record_of(&self, p: &Place) -> Option<u32> {
            let ty = match Self::root(p)? {
                Place::Local(i) => &self.locals.get(*i)?.ty,
                Place::Global(i) => &self.globals.get(*i)?.ty,
                _ => return None,
            };
            match ty {
                RtType::Record(t, false) => Some(*t),
                _ => None,
            }
        }
        fn arg(&mut self, a: &Arg) {
            match a {
                Arg::Value(e) => self.expr(e),
                Arg::Ref(p) => self.place(p),
            }
        }
        fn place(&mut self, p: &Place) {
            match p {
                Place::Index(inner, idx) => {
                    self.place(inner);
                    idx.iter().for_each(|e| self.expr(e));
                }
                Place::Field(inner, _) => self.place(inner),
                _ => {}
            }
        }
        fn expr(&mut self, e: &Expr) {
            match e {
                Expr::Const(_)
                | Expr::BaseInstance
                | Expr::ThisInstance
                | Expr::Refused(_)
                | Expr::Fail { .. }
                | Expr::ControlAddInCall { .. } => {}
                Expr::TddStub { args, .. } => args.iter().for_each(|a| self.expr(a)),
                Expr::TddFit { inner, .. } | Expr::FitOrRefuse { inner, .. } => self.expr(inner),
                Expr::Load(p) => self.place(p),
                Expr::Unary(_, a) | Expr::Convert(_, a) | Expr::DotNetCoerce(_, a) | Expr::DotNetOperand(a, _) => {
                    self.expr(a)
                }
                Expr::Binary(_, a, b) | Expr::CharAt(a, b) => {
                    self.expr(a);
                    self.expr(b);
                }
                Expr::In(a, set) => {
                    self.expr(a);
                    for s in set {
                        match s {
                            al_ir::SetElem::One(e) => self.expr(e),
                            al_ir::SetElem::Range(a, b) => {
                                a.iter().chain(b).for_each(|e| self.expr(e));
                            }
                        }
                    }
                }
                Expr::Ternary(a, b, c) => {
                    self.expr(a);
                    self.expr(b);
                    self.expr(c);
                }
                Expr::Call { instance, args, .. } => {
                    instance.iter().for_each(|e| self.expr(e));
                    args.iter().for_each(|a| self.arg(a));
                }
                Expr::InterfaceCall { receiver, args, .. } => {
                    self.expr(receiver);
                    args.iter().for_each(|a| self.arg(a));
                }
                Expr::InterfaceTest { value, .. } | Expr::EnumImpl { value, .. } => self.expr(value),
                Expr::Control { page, .. } => self.expr(page),
                Expr::QueryColumn { query, .. } => self.expr(query),
                Expr::FieldOf(base, _) => self.expr(base),
                Expr::DotNet { receiver, args, .. } => {
                    receiver.iter().for_each(|e| self.expr(e));
                    args.iter().for_each(|a| self.arg(a));
                }
                Expr::Builtin { id, receiver, args, .. } => {
                    if self.loads && id.name() == "NavApp.LoadPackageData#1" {
                        self.package_data_table(args);
                    }
                    if let Some(r) = receiver {
                        if let Some(rest) = id.name().strip_prefix("Record.") {
                            let method = rest.split('#').next().unwrap_or("");
                            if matches!(method, "Insert" | "Modify" | "Rename" | "ModifyAll") {
                                let place = match &**r {
                                    Arg::Ref(p) => Some(p),
                                    Arg::Value(Expr::Load(p)) => Some(p),
                                    Arg::Value(_) => None,
                                };
                                if let Some(t) = place.and_then(|p| self.record_of(p)) {
                                    self.out.insert(t);
                                }
                            }
                        }
                        self.arg(r);
                    }
                    args.iter().for_each(|a| self.arg(a));
                }
            }
        }
        fn stmts(&mut self, body: &[Stmt]) {
            for s in body {
                match s {
                    Stmt::Assign(p, e) => {
                        self.place(p);
                        self.expr(e);
                    }
                    Stmt::Expr(e) => self.expr(e),
                    Stmt::If(c, a, b) => {
                        self.expr(c);
                        self.stmts(a);
                        self.stmts(b);
                    }
                    Stmt::While(c, b) | Stmt::Repeat(b, c) => {
                        self.expr(c);
                        self.stmts(b);
                    }
                    Stmt::For {
                        var, from, to, body, ..
                    } => {
                        self.place(var);
                        self.expr(from);
                        self.expr(to);
                        self.stmts(body);
                    }
                    Stmt::Foreach { var, collection, body } => {
                        self.place(var);
                        self.expr(collection);
                        self.stmts(body);
                    }
                    Stmt::Case {
                        scrutinee,
                        branches,
                        otherwise,
                    } => {
                        self.expr(scrutinee);
                        for (labels, b) in branches {
                            for l in labels {
                                match l {
                                    al_ir::SetElem::One(e) => self.expr(e),
                                    al_ir::SetElem::Range(a, b) => {
                                        a.iter().chain(b).for_each(|e| self.expr(e));
                                    }
                                }
                            }
                            self.stmts(b);
                        }
                        self.stmts(otherwise);
                    }
                    Stmt::Exit(e) => e.iter().for_each(|e| self.expr(e)),
                    Stmt::AssertError(b) => self.stmts(b),
                    Stmt::Break | Stmt::Continue | Stmt::Pos(_) | Stmt::Assume(_) | Stmt::TddMark(_) => {}
                }
            }
        }
    }

    let mut out = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut stack = vec![(object, routine)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        let obj = &lowered.program.objects[o];
        let rt = &obj.routines[r];
        W {
            locals: &rt.locals,
            globals: &obj.globals,
            heads: lowered.program.objects.heads(),
            loads,
            out: &mut out,
        }
        .stmts(&rt.body);
        lowered.calls_into(o, r, &mut stack);
    }
    // Rows that event subscribers of those writes add are not counted: which of them run depends on
    // data (a subscriber that writes a table when something else is configured), and counting every
    // subscriber the tables could reach marks tables BC does not fill (measured: Dimension
    // Correction). A table only a subscriber fills stays unmarked (docs/probes-needed.md).
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn assumption_ids_are_read_back_from_the_notes() {
        let notes = vec![
            "Plan Installer.OnInstallAppPerDatabase did not run to its end: x".to_string(),
            "the baseline relied on the unprobed assumption `geturl-web-base-address` (--list-assumptions describes it)".into(),
            "the baseline relied on the unprobed assumption `a-b` (--list-assumptions describes it)".into(),
        ];
        assert_eq!(super::assumption_ids(&notes), vec!["geturl-web-base-address", "a-b"]);
    }
}
