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
//!    extension): the other test would see its fields, controls, values or permissions;
//! 3. a routine or a table of `app` in the static reach of a test codeunit of the other suites (`written.rs`), which
//!    follows calls, triggers, subscribers, objects run by a constant id and interface calls (every codeunit that
//!    implements the interface, so an implementation of an outside interface is caught here).
//!
//! The ADR's fourth rule (install code that writes another app's table, a number sequence or isolated storage) has no
//! case to look at: the install code of `app` never runs on the other suites' engine (`Engine::uninstalled_apps`).
//!
//! What this cannot rule out (ADR-073, Phase 2): a test that lists objects or installed apps and counts every app's, an
//! object opened or run by an id computed at run time (the reach is then unknown, not a breach), and the SystemId and
//! row version counters. The outcome differential covers those for today's cases.
//!
//! There is no allow list. A breach is fixed by giving the suite its own pass again (`AL_SERVER_SETTINGS_OWN_PASS=1`
//! in the corpus job), never by an exception.

use crate::runtime::written::{ReachIndex, union_reach_visited};
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

/// Whether program app `app` may share a pass with the test codeunits `others` (object indices) of suites whose CI job
/// does not install it. `Err` names the first object that breaks a rule of this module.
pub fn check(engine: &al_vm::Engine, lowered: &Lowered, app: u32, others: &[usize]) -> Result<(), Breach> {
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
        let base = match h.kind {
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
    let (facts, visited) = union_reach_visited(events, lowered, &index, &roots);
    if let Some(&(o, r)) = visited.iter().find(|(o, _)| in_app(*o)) {
        let routine = lowered.program.objects[o]
            .routines
            .get(r)
            .map_or(String::new(), |r| format!(" (routine {})", r.name));
        return Err(Breach(format!(
            "{}{routine} is in the static reach of a test codeunit of another suite of the pass (ADR-073 Phase 2, point 5, rule 3)",
            named(o)
        )));
    }
    if let Some(&t) = facts.touched.iter().find(|&&t| in_app(t as usize)) {
        return Err(Breach(format!(
            "{} is in the static reach of a test codeunit of another suite of the pass (ADR-073 Phase 2, point 5, rule 3)",
            named(t as usize)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The main app: a table, a publisher, and a test codeunit (the other suite's tests). `run` is the body of its test.
    fn main_app(run: &str) -> String {
        format!(
            "table 50100 MainRow {{ fields {{ field(1; K; Integer) {{ }} }} }}
             codeunit 50101 MainPub {{ [IntegrationEvent(false, false)] procedure OnThing() begin end;
                 procedure Raise() begin OnThing(); end; }}
             codeunit 50102 MainTests {{ Subtype = Test;
                 [Test] procedure T() var P: Codeunit MainPub; R: Record MainRow; begin P.Raise(); R.Insert(); {run} end; }}"
        )
    }

    /// Whether the app `tw` (depending on the main app) may share a pass with the main app's test codeunit.
    fn check_of(main_run: &str, tw: &str) -> Result<(), Breach> {
        let main = main_app(main_run);
        let apps = [("main", vec![], main.as_str()), ("tw", vec!["main".to_string()], tw)];
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
        check(&engine, &l, app, &[of("MainTests")])
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
        assert_eq!(check_of("", CLEAN), Ok(()));
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
