//! A table an app keeps as a stub after it moved the table to one of its dependencies (`ObsoleteState = Moved`).
//!
//! a workspace table kept as the `ObsoleteState = Moved` stub of a direct dependency's table (al-
//! runner issue 7914, corpus al-language-xapp-b/MvXbMovedStub.al): `alcodes::duplicate_ids` lets it
//! compile, `pick_real` resolves to the dependency's table; only a stub of a dependency package with
//! other fields or keys is unmeasured, and `lower/assemble.rs` then puts the assumption on
//! `ir::Program::assumes` (marked for every test). Entry points: `moved_table_of_dependency`,
//! `has_unmeasured_moved_table_stub`.

use super::{AppKind, ObjKind, Workspace};

impl Workspace {
    /// The value of the object-level property `name` of object `i`, as written (a text keeps its quotes). An object of a
    /// scanned file has its properties from the scan, the others from their syntax tree.
    fn object_property(&self, i: usize, name: &str) -> Option<String> {
        let o = &self.objects[i];
        if self.files[o.file].lazy {
            return o
                .scanned_properties
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim().to_string());
        }
        self.object_ast(i)
            .properties()
            .find(|p| p.name().eq_ignore_ascii_case(name))
            .map(|p| p.value_text().trim().to_string())
    }

    /// The table of a direct dependency that the workspace table `i` is the moved stub of, when every fact of the
    /// pinned pair holds (corpus al-language-xapp-b/MvXbMovedStub.al:6-23 and al-language-xapp-a/MvXaLiveTable.al:6-19,
    /// test al-language/table-objects/moved-tables/MvMovedTableTests.al:18-42; al-runner issue 7914):
    /// - `i` is a table of a workspace app with `ObsoleteState = Moved` (MvXbMovedStub.al:9);
    /// - its `MovedTo` is the id of an app that `i`'s app names as a dependency (MvXbMovedStub.al:12,
    ///   al-language-xapp-b/app.json:22), whether that app compiles from source in the same run or is loaded as a
    ///   dependency package: in BC an app always compiles against the symbols of the apps it depends on;
    /// - that app declares a table with the same id and the same name, without an `ObsoleteState` property;
    /// - the stub's `fields` and `keys` are the live table's, token for token ([`Self::same_fields_and_keys`]), or the
    ///   app is a dependency package: other fields or keys are not measured and run only under assumption
    ///   `moved-table-stub-of-dependency` ([`Self::has_unmeasured_moved_table_stub`]; the reporter's alc 18.1 repro).
    ///
    /// Microsoft's Base Application 28.4 and Business Foundation are such pairs (tables 231, 308, 309, 310, 1263).
    /// Every other shape (another obsolete state, no `MovedTo`, a transitive dependency, a different name, a dependency
    /// table that is itself obsolete, other fields or keys of a workspace `MovedTo` app) is `None`.
    pub fn moved_table_of_dependency(&self, i: usize) -> Option<usize> {
        let (j, app) = self.moved_pair(i)?;
        (self.kinds[app] == AppKind::Dependency || self.same_fields_and_keys(i, j)).then_some(j)
    }

    /// The live table and its app of the stub `i`, every fact of [`Self::moved_table_of_dependency`] but the fields.
    fn moved_pair(&self, i: usize) -> Option<(usize, usize)> {
        let o = &self.objects[i];
        if !(o.kind == ObjKind::Table && self.kinds[o.app] == AppKind::Workspace) {
            return None;
        }
        let id = o.id?;
        let same_id = self.by_id.get(&(ObjKind::Table, id))?;
        if same_id.len() < 2 {
            return None;
        }
        let state = self.object_property(i, "ObsoleteState")?;
        if !state.eq_ignore_ascii_case("Moved") {
            return None;
        }
        let moved_to = self.object_property(i, "MovedTo")?;
        let moved_to = moved_to.trim_matches('\'');
        let target = self.apps[o.app]
            .dependencies
            .iter()
            .find(|d| d.eq_ignore_ascii_case(moved_to))?;
        let app = self.apps.iter().position(|a| a.id.eq_ignore_ascii_case(target))?;
        same_id
            .iter()
            .copied()
            .find(|&j| {
                let t = &self.objects[j];
                t.app == app
                    && t.name.eq_ignore_ascii_case(&o.name)
                    && self.object_property(j, "ObsoleteState").is_none()
            })
            .map(|j| (j, app))
    }

    /// Whether tables `i` and `j` have the same `fields` and `keys` sections, token for token as written (trivia aside)
    /// (the pinned stub repeats the live table's, MvXbMovedStub.al:13-22 and MvXaLiveTable.al:10-18).
    fn same_fields_and_keys(&self, i: usize, j: usize) -> bool {
        let tokens = |k: usize, section: &str| -> Option<Vec<String>> {
            let s = self
                .object_ast(k)
                .sections()
                .find(|s| s.name().eq_ignore_ascii_case(section))?;
            Some(
                s.syntax()
                    .descendants_with_tokens()
                    .filter_map(|e| e.into_token())
                    .filter(|t| !t.kind().is_trivia())
                    .map(|t| t.text().to_string())
                    .collect(),
            )
        };
        ["fields", "keys"].iter().all(|s| {
            let a = tokens(i, s);
            a.is_some() && a == tokens(j, s)
        })
    }

    /// Whether a workspace table is the moved stub of a dependency package's table with other fields or keys than the
    /// live table's: the shape no pin measures (assumption `moved-table-stub-of-dependency`).
    pub fn has_unmeasured_moved_table_stub(&self) -> bool {
        (0..self.objects.len()).any(|i| {
            self.moved_pair(i)
                .is_some_and(|(j, app)| self.kinds[app] == AppKind::Dependency && !self.same_fields_and_keys(i, j))
        })
    }
}
