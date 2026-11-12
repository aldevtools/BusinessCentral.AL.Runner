//! A table an app keeps as a stub after it moved the table to one of its dependencies (`ObsoleteState = Moved`).

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
    /// measured pair holds (al-runner issue 7914, the reporter's alc 18.1 repro: alc compiles the pair; assumption
    /// `moved-table-stub-of-dependency`, probe corpus PR 1363):
    /// - `i` is a table of a workspace app with `ObsoleteState = Moved`;
    /// - its `MovedTo` is the id of an app that `i`'s app names as a dependency, and that app is loaded as a dependency
    ///   package (`AppKind::Dependency`); a `MovedTo` app that is itself a workspace app is not the measured pair;
    /// - that app declares a table with the same id and the same name, without an `ObsoleteState` property.
    ///
    /// Microsoft's Base Application 28.4 and Business Foundation are such pairs (tables 231, 308, 309, 310, 1263).
    /// Every other shape (another obsolete state, no `MovedTo`, a transitive dependency, a `MovedTo` app compiled from
    /// source in the same run, a different name, a dependency table that is itself obsolete) is `None`.
    pub fn moved_table_of_dependency(&self, i: usize) -> Option<usize> {
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
        if self.kinds[app] != AppKind::Dependency {
            return None;
        }
        same_id.iter().copied().find(|&j| {
            let t = &self.objects[j];
            t.app == app && t.name.eq_ignore_ascii_case(&o.name) && self.object_property(j, "ObsoleteState").is_none()
        })
    }

    /// Whether any workspace table is the moved stub of a dependency's table ([`Self::moved_table_of_dependency`]).
    pub fn has_moved_table_stub(&self) -> bool {
        (0..self.objects.len()).any(|i| self.moved_table_of_dependency(i).is_some())
    }
}
