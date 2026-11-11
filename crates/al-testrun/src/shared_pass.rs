//! ADR-073 Phase 2, point 5: whether a suite app may share a pass with suites whose CI job does not have it.
//!
//! The try-write suite runs in the main pass on an engine of its own (its CI job sets DisableWriteInsideTryFunctions =
//! false), so its app is in the program the main corpus app's tests run on, which BC's main job does not have. That is
//! only allowed when no test of the other suites can run code of the app or touch one of its objects, so that no result
//! of theirs can change (owner, 2026-11-11). [`check`] says so, and names the object and the rule when it cannot.
//!
//! The rules, each a way code of `app` comes into the run of a test of another suite:
//! 1. an event subscriber in `app` to an event published outside `app` (a platform codeunit, Microsoft's apps, the main
//!    corpus app, another app's table or page): the event raised by the other test would run it;
//! 2. an object of `app` that extends an object of another app (a table, page, report, enum or permission set
//!    extension, a profile extension), or an object of a kind the program does not model (a page customization): the
//!    other test would see its fields, controls, values, permissions or profile;
//! 3. a routine or a table of `app` in the static reach of the other suites' apps: every routine of every object of
//!    those apps (test codeunits, their helpers, install code) is a root (`written.rs`), and the walk follows calls,
//!    triggers, subscribers, objects run by a constant id and interface calls (every codeunit that implements the
//!    interface, so an implementation of an outside interface is caught here).
//!
//! The ADR's fourth rule (install code that writes another app's table, a number sequence or isolated storage) has no
//! case to look at: the install code of `app` never runs on the other suites' engine.
//!
//! What the walk cannot follow fails the check, with one exception (owner, 2026-11-11): a routine that is not compiled
//! (`al_sema::check::is_not_compiled`, or the triggers or the run of an object that is not compiled). A call of such a
//! routine is refused, so it cannot run in part and change a result silently; it is reported in [`Reach::not_compiled`].
//! Any other reason (an object run by an id known only at run time, a refused construct, a report that cannot run) is
//! a breach. The SystemId and row version counters stay outside this check; the outcome differential covers them for
//! today's cases.
//!
//! There is no allow list. A breach is fixed by giving the suite its own pass again (`AL_SERVER_SETTINGS_OWN_PASS=1`
//! in the corpus job), never by an exception.

use crate::runtime::written::{ReachIndex, Unknown, union_reach_visited};
use al_ir::ObjKind;
use al_sema::lower::Lowered;
use std::collections::BTreeSet;

/// Why an app may not share a pass: the object and the rule it broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breach(pub String);

impl std::fmt::Display for Breach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the check found when the app may share the pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reach {
    /// The parts of the other suites' reach the walk cannot follow because they are not compiled (each a reason, in order).
    pub not_compiled: Vec<String>,
}

/// Whether program app `app` may share a pass with the test codeunits `others` (object indices) of suites whose CI job
/// does not install it. `Err` names the first object that breaks a rule of this module.
pub fn check(engine: &al_vm::Engine, lowered: &Lowered, app: u32, others: &[usize]) -> Result<Reach, Breach> {
    let events = &engine.events;
    let index = engine.derived(|| ReachIndex::new(events, lowered));
    let heads = lowered.program.objects.heads();
    let in_app = |o: usize| heads.get(o).is_some_and(|h| h.app == app);
    let named = |o: usize| {
        let h = &heads[o];
        format!("{:?} {} \"{}\"", h.kind, h.id.unwrap_or(0), h.name)
    };
    // Rule 1. Every subscriber routine the app declares must be registered under a publisher of the app itself; one that
    // is registered elsewhere, or not at all (a missing or platform publisher), listens outside.
    let mut inside: BTreeSet<(usize, usize)> = BTreeSet::new();
    for ((publisher, _), subs) in &events.subscribers {
        if in_app(*publisher) {
            inside.extend(subs.iter().map(|s| (s.object, s.routine)));
        }
    }
    for (o, h) in heads.iter().enumerate().filter(|(o, _)| in_app(*o)) {
        for ev in &h.events {
            let subscriber = ev
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("EventSubscriber"));
            if subscriber && !inside.contains(&(o, ev.routine as usize)) {
                return Err(Breach(format!(
                    "{} has the event subscriber {} to an event published outside its app (ADR-073 Phase 2, point 5, rule 1)",
                    named(o),
                    ev.name
                )));
            }
        }
    }
    // Rule 2. An extension of an object of another app.
    for (o, h) in heads.iter().enumerate().filter(|(o, _)| in_app(*o)) {
        // A profile extension: a Profile whose `$ExtendsProfile` names a profile (by name) of another app.
        if let Some(target) = extended_profile(h) {
            let own = (heads.iter().enumerate())
                .any(|(b, bh)| bh.kind == ObjKind::Profile && bh.name.eq_ignore_ascii_case(target) && in_app(b));
            if !own {
                return Err(Breach(format!(
                    "{} extends a profile of another app (ADR-073 Phase 2, point 5, rule 2)",
                    named(o)
                )));
            }
            continue;
        }
        let base = match h.kind {
            // An object of a kind the program does not model (a page customization): it may change another app's.
            ObjKind::Other => {
                return Err(Breach(format!(
                    "{} is of a kind this check does not read (ADR-073 Phase 2, point 5, rule 2)",
                    named(o)
                )));
            }
            ObjKind::TableExtension => ObjKind::Table,
            ObjKind::PageExtension => ObjKind::Page,
            ObjKind::ReportExtension => ObjKind::Report,
            ObjKind::EnumExtension => ObjKind::Enum,
            ObjKind::PermissionSetExtension => ObjKind::PermissionSet,
            _ => continue,
        };
        let own = heads
            .iter()
            .enumerate()
            .any(|(b, bh)| bh.kind == base && bh.id.is_some() && bh.id == h.extends_id && in_app(b));
        if !own {
            return Err(Breach(format!(
                "{} extends an object of another app (ADR-073 Phase 2, point 5, rule 2)",
                named(o)
            )));
        }
    }
    // Rule 3. The joined static reach of the other suites' test codeunits: every routine of each.
    let roots: Vec<(usize, usize)> = others
        .iter()
        .flat_map(|&o| (0..heads[o].routine_count as usize).map(move |r| (o, r)))
        .collect();
    let reach = union_reach_visited(events, lowered, &index, &roots);
    if let Some(&(o, r)) = reach.visited.iter().find(|(o, _)| in_app(*o)) {
        let routine = lowered.program.objects[o]
            .routines
            .get(r)
            .map_or(String::new(), |r| format!(" (routine {})", r.name));
        return Err(Breach(format!(
            "{}{routine} is in the static reach of a test codeunit of another suite of the pass (ADR-073 Phase 2, point 5, rule 3)",
            named(o)
        )));
    }
    if let Some(&t) = reach.facts.touched.iter().find(|&&t| in_app(t as usize)) {
        return Err(Breach(format!(
            "{} is in the static reach of a test codeunit of another suite of the pass (ADR-073 Phase 2, point 5, rule 3)",
            named(t as usize)
        )));
    }
    // What the walk cannot follow: only a routine that is not compiled is allowed (owner, 2026-11-11), as a call of it is
    // refused.
    judge_unknown(&reach.unknowns)
}

/// The check's answer on the parts of the reach the walk cannot follow: a breach for the first that is not a routine
/// that is not compiled; else the reasons of those that are not compiled.
fn judge_unknown(unknowns: &BTreeSet<(Unknown, String)>) -> Result<Reach, Breach> {
    if let Some((_, why)) = unknowns.iter().find(|(kind, _)| *kind != Unknown::NotCompiled) {
        return Err(Breach(format!(
            "the static reach of the other suites of the pass has a part the walk cannot follow: {why} (ADR-073 Phase 2, \
             point 5: only a routine that is not compiled may be unknown)"
        )));
    }
    Ok(Reach {
        not_compiled: unknowns.iter().map(|(_, why)| why.clone()).collect(),
    })
}

/// The name of the profile that profile `h` extends, if it is a profile extension.
fn extended_profile(h: &al_ir::ObjectHead) -> Option<&str> {
    (h.kind == ObjKind::Profile)
        .then(|| h.properties.iter().find(|(k, _)| k == al_ir::EXTENDS_PROFILE))
        .flatten()
        .map(|(_, v)| v.trim_matches('"'))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The main app: a table, a publisher, a test codeunit (the other suite's tests) and `more` objects. `run` is the body
    /// of its test.
    fn main_app(run: &str, more: &str) -> String {
        format!(
            "table 50100 MainRow {{ fields {{ field(1; K; Integer) {{ }} }} }}
             codeunit 50101 MainPub {{ [IntegrationEvent(false, false)] procedure OnThing() begin end;
                 procedure Raise() begin OnThing(); end; }}
             codeunit 50102 MainTests {{ Subtype = Test;
                 [Test] procedure T() var P: Codeunit MainPub; R: Record MainRow; begin P.Raise(); R.Insert(); {run} end; }}
             {more}"
        )
    }

    /// Whether the app `tw` (depending on the main app) may share a pass with the main app's objects named `roots`.
    fn check_with(main: &str, tw: &str, roots: &[&str]) -> Result<Reach, Breach> {
        let apps = [("main", vec![], main), ("tw", vec!["main".to_string()], tw)];
        let (_, l) = al_sema::lower::compile(
            apps.iter()
                .map(|(id, deps, src)| al_sema::workspace::AppSource {
                    id: (*id).into(),
                    name: (*id).into(),
                    dependencies: deps.clone(),
                    files: vec![(format!("{id}.al"), (*src).into())],
                })
                .collect(),
            &Default::default(),
        );
        let heads = l.program.objects.heads();
        let of = |name: &str| heads.iter().position(|h| h.name == name).unwrap();
        let app = heads[of("TwTests")].app;
        let engine = al_vm::Engine::new(l.program.clone(), vec![]);
        let roots: Vec<usize> = roots.iter().map(|r| of(r)).collect();
        check(&engine, &l, app, &roots)
    }

    /// Whether the app `tw` may share a pass with the main app's test codeunit, whose test also runs `main_run`.
    fn check_of(main_run: &str, tw: &str) -> Result<Reach, Breach> {
        check_with(&main_app(main_run, ""), tw, &["MainTests"])
    }

    /// The try-write app's shape: tables of its own, install code and tests that use only its own objects, and an event
    /// of its own with its own subscriber.
    const CLEAN: &str = "table 81701 TwRow { fields { field(1; K; Integer) { } } }
        codeunit 81702 TwPub { [IntegrationEvent(false, false)] procedure OnTw() begin end; }
        codeunit 81703 TwSub { [EventSubscriber(ObjectType::Codeunit, Codeunit::TwPub, 'OnTw', '', false, false)]
            local procedure H() begin end; }
        codeunit 81704 TwTests { Subtype = Test;
            [Test] procedure T() var R: Record TwRow; M: Record MainRow; begin R.Insert(); M.Insert(); end; }";

    #[test]
    fn an_app_that_only_uses_its_own_objects_may_share_the_pass() {
        assert_eq!(check_of("", CLEAN), Ok(Reach::default()));
    }

    /// A codeunit run by an id known only at run time: the walk cannot follow it, so the check fails (owner, 2026-11-11:
    /// only a routine that is not compiled may be unknown).
    #[test]
    fn a_run_by_an_id_known_only_at_run_time_breaks_the_check() {
        let why = check_of("Codeunit.Run(R.K);", CLEAN).unwrap_err().0;
        assert!(why.contains("known only at run time"), "{why}");
    }

    /// A routine of a dependency app that no code reached while compiling (`al_sema::check::NOT_REACHED`): a call of it is
    /// refused, so the check reports it and passes (owner, 2026-11-11).
    #[test]
    fn a_dependency_routine_that_is_not_compiled_is_reported_and_allowed() {
        let app = |id: &str, deps: &[&str], src: &str| al_sema::workspace::AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: vec![(format!("{id}.al"), src.into())],
        };
        use al_sema::workspace::AppKind::{Dependency, Workspace};
        let (_, l) = al_sema::lower::compile_apps(
            vec![
                app(
                    "dep",
                    &[],
                    "codeunit 10 Lib { procedure Used() begin end; procedure Unused() begin end; }",
                ),
                app(
                    "main",
                    &["dep"],
                    &main_app(
                        "",
                        "codeunit 50105 MainUsesLib { procedure Go() var D: Codeunit Lib; begin D.Used(); end; }",
                    ),
                ),
                app("tw", &["main"], CLEAN),
            ],
            vec![Dependency, Workspace, Workspace],
            &Default::default(),
        );
        let heads = l.program.objects.heads();
        let of = |name: &str| heads.iter().position(|h| h.name == name).unwrap();
        let engine = al_vm::Engine::new(l.program.clone(), vec![]);
        // The dependency codeunit is a root: its routine Used is compiled (the main app calls it), Unused is not.
        let r = check(&engine, &l, heads[of("TwTests")].app, &[of("MainTests"), of("Lib")]).unwrap();
        assert!(
            r.not_compiled
                .iter()
                .any(|why| why.starts_with("Lib.Unused: not compiled")),
            "{r:?}"
        );
    }

    #[test]
    fn an_unknown_reach_that_is_not_a_routine_not_compiled_breaks_the_check() {
        let of = |v: &[(Unknown, &str)]| v.iter().map(|(k, w)| (*k, w.to_string())).collect::<BTreeSet<_>>();
        let only = of(&[(Unknown::NotCompiled, "A.B: not compiled")]);
        assert_eq!(judge_unknown(&only).unwrap().not_compiled, ["A.B: not compiled"]);
        let mixed = of(&[
            (Unknown::NotCompiled, "A.B: not compiled"),
            (Unknown::Other, "C.D: refused"),
        ]);
        let why = judge_unknown(&mixed).unwrap_err().0;
        assert!(why.contains("C.D: refused"), "{why}");
    }

    #[test]
    fn a_profile_that_extends_a_profile_of_another_app_breaks_rule_2() {
        let main = main_app(
            "",
            "profile MainProfile { RoleCenter = 50103; }\n page 50103 MainRc { PageType = RoleCenter; }",
        );
        let tw = format!("{CLEAN}\n profileextension TwProfileExt extends MainProfile {{ Caption = 'x'; }}");
        let why = check_with(&main, &tw, &["MainTests"]).unwrap_err().0;
        assert!(why.contains("TwProfileExt") && why.contains("rule 2"), "{why}");
    }

    /// The other suites' objects that are not test codeunits are roots too (their install code, their helper codeunits).
    #[test]
    fn a_codeunit_a_main_helper_runs_breaks_rule_3() {
        let tw = format!("{CLEAN}\n codeunit 81707 TwRunMe {{ trigger OnRun() begin end; }}");
        let main = main_app(
            "",
            "codeunit 50104 MainHelper { procedure Go() begin Codeunit.Run(81707); end; }",
        );
        let why = check_with(&main, &tw, &["MainTests", "MainHelper"]).unwrap_err().0;
        assert!(why.contains("TwRunMe") && why.contains("rule 3"), "{why}");
    }

    #[test]
    fn a_subscriber_to_an_event_of_another_app_breaks_rule_1() {
        let tw = format!(
            "{CLEAN}
            codeunit 81705 TwOutside {{ [EventSubscriber(ObjectType::Codeunit, Codeunit::MainPub, 'OnThing', '', false, false)]
                local procedure H() begin end; }}"
        );
        let why = check_of("", &tw).unwrap_err().0;
        assert!(why.contains("TwOutside") && why.contains("rule 1"), "{why}");
    }

    #[test]
    fn a_subscriber_to_a_table_event_of_another_app_breaks_rule_1() {
        let tw = format!(
            "{CLEAN}
            codeunit 81705 TwOutside {{ [EventSubscriber(ObjectType::Table, Database::MainRow, 'OnAfterInsertEvent', '', false, false)]
                local procedure H(var Rec: Record MainRow; RunTrigger: Boolean) begin end; }}"
        );
        let why = check_of("", &tw).unwrap_err().0;
        assert!(why.contains("TwOutside") && why.contains("rule 1"), "{why}");
    }

    #[test]
    fn an_extension_of_an_object_of_another_app_breaks_rule_2() {
        let tw = format!(
            "{CLEAN}
            tableextension 81706 TwExt extends MainRow {{ fields {{ field(81700; Extra; Integer) {{ }} }} }}"
        );
        let why = check_of("", &tw).unwrap_err().0;
        assert!(why.contains("TwExt") && why.contains("rule 2"), "{why}");
    }

    #[test]
    fn a_codeunit_another_suite_runs_by_a_constant_id_breaks_rule_3() {
        let tw = format!("{CLEAN}\n codeunit 81707 TwRunMe {{ trigger OnRun() begin end; }}");
        let why = check_of("Codeunit.Run(81707);", &tw).unwrap_err().0;
        assert!(why.contains("TwRunMe") && why.contains("rule 3"), "{why}");
    }

    #[test]
    fn a_page_another_suite_runs_by_a_constant_id_breaks_rule_3() {
        let tw = format!("{CLEAN}\n page 81708 TwPage {{ SourceTable = TwRow; trigger OnOpenPage() begin end; }}");
        let why = check_of("Page.Run(81708);", &tw).unwrap_err().0;
        assert!(why.contains("TwPage") && why.contains("rule 3"), "{why}");
    }
}
