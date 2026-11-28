//! The watch of the `Global Triggers` raises while one partner login subscriber runs (al-testrun `partner_login::raise`,
//! al-runner issue 8945): per watched SingleInstance codeunit and global, the raises whose code changed it.
//!
//! The watch stamps the globals it was given of the instances of their codeunits (`Session::singles_of`, the same
//! per-global fingerprints as `LeftStamp`) when it begins, at the start and the end of every `Global Triggers` raise (nested ones included) and
//! when it is taken. What differs between two consecutive stamps changed in that stretch, and is noted under the raises
//! that were running there, outermost first, joined by `>` (`ondatabaseinsert>getdatabasetabletriggersetup`: a setup ask
//! made inside an insert raise), or under the empty name outside any raise. Only the globals given are stamped, so its
//! cost does not grow with the other globals of those codeunits or the other SingleInstance state of the session: a
//! partner login raises the insert event once per row it writes to a table a subscriber turned the event on for.
//!
//! Entry points: `Session::watch_global_trigger_changes`, `Session::take_global_trigger_changes`; globaltriggers.rs
//! `raise_global` calls `raise_watch_boundary`.

use crate::Session;
use crate::state_stamp::{FreshGlobals, Single, singles_changed};
use std::collections::{BTreeMap, BTreeSet};

/// SingleInstance codeunit (object index) -> global (index) -> where it changed: the lower-case names of the `Global
/// Triggers` raises running there, outermost first, joined by `>`, or the empty name outside any raise.
pub type RaiseChanges = BTreeMap<usize, BTreeMap<usize, BTreeSet<String>>>;

/// A running watch ([`Session::watch_global_trigger_changes`]).
pub(crate) struct RaiseWatch {
    /// The globals (index) it stamps, per codeunit (object index).
    globals: BTreeMap<usize, BTreeSet<usize>>,
    /// Their stamp at the last boundary.
    last: BTreeMap<usize, Single>,
    /// What the fresh instances' globals hold, kept across the boundaries.
    fresh: FreshGlobals,
    /// The events of the raises running now, outermost first.
    open: Vec<String>,
    noted: RaiseChanges,
}

impl Session {
    /// From now on, notes per global of `globals` (SingleInstance codeunit, object index -> its globals, index) where it
    /// changes ([`RaiseChanges`]), until [`Session::take_global_trigger_changes`].
    pub fn watch_global_trigger_changes(&mut self, globals: BTreeMap<usize, BTreeSet<usize>>) {
        let mut fresh = FreshGlobals::new();
        let last = self.singles_of(Some(&globals), &mut fresh);
        self.events.raise_watch = Some(Box::new(RaiseWatch {
            globals,
            last,
            fresh,
            open: Vec::new(),
            noted: RaiseChanges::new(),
        }));
    }

    /// What [`Session::watch_global_trigger_changes`] noted since it began, the stretch since the last raise included;
    /// ends the watch.
    pub fn take_global_trigger_changes(&mut self) -> RaiseChanges {
        self.raise_watch_boundary(None);
        self.events.raise_watch.take().map(|w| w.noted).unwrap_or_default()
    }

    /// A boundary of the watch, when one runs: notes what changed since the last one under the raises running, then
    /// opens a raise of `event`, or with `None` closes the innermost one.
    pub(crate) fn raise_watch_boundary(&mut self, event: Option<&str>) {
        let Some(mut w) = self.events.raise_watch.take() else {
            return;
        };
        let now = self.singles_of(Some(&w.globals), &mut w.fresh);
        let at = w.open.join(">");
        for (o, globals) in singles_changed(&w.last, &now, Some(&w.globals)) {
            let memo = w.noted.entry(o).or_default();
            for g in globals {
                memo.entry(g).or_default().insert(at.clone());
            }
        }
        match event {
            Some(e) => w.open.push(e.to_ascii_lowercase()),
            None => {
                w.open.pop();
            }
        }
        w.last = now;
        self.events.raise_watch = Some(w);
    }
}

#[cfg(test)]
mod tests {
    use al_testkit::Built;
    use std::collections::{BTreeMap, BTreeSet};

    /// Memo, watched: `Asked` changes in the setup ask of Watched, `Seen` in its insert event, `Direct` outside any raise,
    /// `Both` outside and in the insert event, `Nested` in the setup ask of Inner that the insert event of Watched makes.
    /// `Left`, a global of Memo the watch is not given, and Other, not watched, change in the insert event too.
    const SRC: &str = r#"
table 50401 Watched { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
table 50405 Inner { fields { field(1; "No."; Integer) { } } keys { key(PK; "No.") { } } }
codeunit 50402 Memo
{
    SingleInstance = true;
    var Asked: Boolean; Seen: Boolean; Direct: Boolean; Both: Integer; Nested: Boolean; Left: Integer;
    procedure Ask() begin Asked := true; end;
    procedure See() begin Seen := true; Both += 1; Left += 1; end;
    procedure Write() begin Direct := true; Both += 1; end;
    procedure Nest() begin Nested := true; end;
}
codeunit 50406 Other { SingleInstance = true; var N: Integer; procedure Bump() begin N += 1; end; }
codeunit 50403 GT
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'GetDatabaseTableTriggerSetup', '', false, false)]
    local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean)
    var M: Codeunit Memo;
    begin
        if TableId = Database::Watched then M.Ask();
        if TableId = Database::Inner then M.Nest();
        OnDatabaseInsert := TableId = Database::Watched;
    end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Global Triggers", 'OnDatabaseInsert', '', false, false)]
    local procedure DI(RecRef: RecordRef)
    var M: Codeunit Memo; O: Codeunit Other; I: Record Inner;
    begin
        if RecRef.Number <> Database::Watched then exit;
        M.See(); O.Bump(); I."No." := 1; I.Insert();
    end;
}
codeunit 50404 Writer
{
    procedure Run() var W: Record Watched; M: Codeunit Memo; begin M.Write(); W."No." := 1; W.Insert(); end;
}
"#;

    // al-runner issue 8945: the partner login gate tells a global the setup ask changed from one a database event changed,
    // from one changed outside any `Global Triggers` raise, from one changed both outside and inside, and from one a setup
    // ask nested in an insert raise changed; a codeunit it was not given is not stamped, nor a global of a watched
    // codeunit it was not given (`Left`, global 5).
    #[test]
    fn the_watch_notes_each_changed_global_under_the_raises_running_there() {
        let mut b = Built::from_source(SRC);
        let (memo, _) = b.routine_in("Memo", "Ask");
        let (other, _) = b.routine_in("Other", "Bump");
        let watched = BTreeMap::from([(memo, BTreeSet::from([0, 1, 2, 3, 4]))]);
        b.session.watch_global_trigger_changes(watched);
        b.run_in("Writer", "Run").unwrap();
        let noted = b.session.take_global_trigger_changes();
        let by_global: Vec<(usize, Vec<String>)> = (noted.get(&memo).into_iter().flatten())
            .map(|(g, events)| (*g, events.iter().cloned().collect()))
            .collect();
        let at = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            by_global,
            vec![
                (0, at(&["getdatabasetabletriggersetup"])),
                (1, at(&["ondatabaseinsert"])),
                (2, at(&[""])),
                (3, at(&["", "ondatabaseinsert"])),
                (4, at(&["ondatabaseinsert>getdatabasetabletriggersetup"])),
            ]
        );
        assert!(!noted.contains_key(&other), "{noted:?}");
        assert!(
            b.session.take_global_trigger_changes().is_empty(),
            "the take ends the watch"
        );
    }
}
