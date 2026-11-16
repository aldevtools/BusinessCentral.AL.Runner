//! The object an `[EventSubscriber]` names as its publisher (al-runner issue 8171). BC binds a subscriber to the publisher object
//! the compiler resolved its name to. Where one visible object has the name, that object. Where several have it (the same name in
//! two namespaces of two apps; one app may not declare it twice, AL0197), the subscriber's file decides, by the shapes the
//! assumption `subscriber-publisher-namespace-scope` covers; every other shape names none of them.

use super::{ObjKind, Workspace, eq_ns};
use al_ir::PublisherRef;

impl Workspace {
    /// What the publisher name `parts` (namespace segments and name, or one numeric id) of kind `kind` names from `file`, as indexes
    /// into `objects` (the indexes of the program the compile makes).
    ///
    /// The candidates are the objects of that kind and name (or id) in the apps `file`'s app sees, without the moved stubs that give
    /// way to a real object (`without_moved_stubs`). One candidate: [`PublisherRef::Only`]. Several: [`PublisherRef::Picked`] when
    /// [`Self::pick_twin`] names one, else [`PublisherRef::Unpicked`].
    pub fn subscriber_publisher(&self, file: usize, kind: ObjKind, parts: &[String]) -> PublisherRef {
        let Some(name) = parts.last() else {
            return PublisherRef::None;
        };
        let id = if parts.len() == 1 {
            name.parse::<u32>().ok()
        } else {
            None
        };
        let named = match id {
            Some(id) => self.by_id.get(&(kind, id)),
            None => self.by_name.get(&(kind, name.to_lowercase())),
        };
        let app = self.files[file].app;
        let visible: Vec<usize> = named
            .into_iter()
            .flatten()
            .copied()
            .filter(|&i| self.visible[app].contains(&self.objects[i].app))
            .collect();
        let twins = self.without_moved_stubs(visible);
        let index = |i: usize| i as u32;
        match twins.as_slice() {
            [] => PublisherRef::None,
            [one] => PublisherRef::Only(index(*one)),
            _ => {
                let all = twins.iter().copied().map(index).collect();
                match self.pick_twin(file, parts, id.is_some(), &twins) {
                    Some(object) => PublisherRef::Picked {
                        object: index(object),
                        twins: all,
                    },
                    None => PublisherRef::Unpicked(all),
                }
            }
        }
    }

    /// The one of `twins` that the publisher name `parts`, written in `file`, names; `None` outside the measured shapes. Each shape
    /// is required by equality:
    /// - a qualified name: the one twin whose namespace is the written one (the reporter's alc 18.1 compiles
    ///   `Codeunit::Repro.Copy."Shared Name"`, issue 8171; corpus PR 1423 SnUsingTests.al, each qualified name);
    /// - a plain name in a file with a namespace that exactly one twin has (corpus PR 1423 SnOwnNamespaceTests.al), whatever the
    ///   file's `using`s (SnOwnOverUsingTests.al);
    /// - a plain name in a file with a namespace that no twin has, whose `using`s name the namespace of exactly one twin (issue
    ///   8171, alc 18.1 binds through `using Repro.Copy;` to 51655; SnUsingTests.al).
    ///
    /// Not picked: an id two objects have; a file without a namespace (alc ignores its `using`s, AL0789, and reports AL0275,
    /// must-not-compile 3171); `using`s of two twins' namespaces (AL0275, 3172); no `using` of a twin's namespace (AL0118, 3173);
    /// two twins in the namespace that decides.
    fn pick_twin(&self, file: usize, parts: &[String], by_id: bool, twins: &[usize]) -> Option<usize> {
        let in_ns = |ns: &Option<String>| -> Vec<usize> {
            twins
                .iter()
                .copied()
                .filter(|&i| eq_ns(&self.objects[i].namespace, ns))
                .collect()
        };
        let only = |hits: Vec<usize>| -> Option<usize> {
            match hits.as_slice() {
                [one] => Some(*one),
                _ => None,
            }
        };
        if by_id {
            return None;
        }
        if parts.len() > 1 {
            return only(in_ns(&Some(parts[..parts.len() - 1].join("."))));
        }
        let f = &self.files[file];
        let own = in_ns(&Some(f.namespace.clone()?));
        if !own.is_empty() {
            return only(own);
        }
        only(f.usings.iter().flat_map(|u| in_ns(&Some(u.clone()))).collect())
    }
}

impl Workspace {
    /// The objects whose events a subscriber of a scanned object with publisher name `parts`, written in `file`, is indexed under
    /// (`Workspace::subscribers_of`, which the subscribers that are not compiled come from). Such a subscriber does not run: an
    /// event of an object it is indexed under is refused. The one object [`Self::subscriber_publisher`] names, except that a plain
    /// name several objects have stays indexed under each of them, and a qualified one under the twin of that namespace only, as
    /// the index by written name did: that a subscriber that does not run leaves the other twins' events alone is not measured.
    pub(super) fn scanned_subscriber_publishers(&self, file: usize, kind: ObjKind, parts: &[String]) -> Vec<usize> {
        let qualified = parts.len() > 1;
        match self.subscriber_publisher(file, kind, parts) {
            PublisherRef::None => Vec::new(),
            PublisherRef::Only(o) => vec![o as usize],
            PublisherRef::Picked { object, .. } if qualified => vec![object as usize],
            PublisherRef::Unpicked(_) if qualified => Vec::new(),
            PublisherRef::Picked { twins, .. } | PublisherRef::Unpicked(twins) => {
                twins.into_iter().map(|t| t as usize).collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{AppKind, AppSource};
    use super::*;

    fn app(id: &str, deps: &[&str], files: &[(&str, &str)]) -> AppSource {
        AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
        }
    }

    type Files<'a> = Vec<(&'a str, &'a str)>;

    const SUB: &str = "codeunit 50 Sub { [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnX', '', false, false)] \
                       local procedure H() begin end; }";

    /// Twin codeunits `Pub` in namespaces A (app a) and B (app b), and in app s a file with `header` holding [`SUB`]. Dependency
    /// apps a and b are scanned, so `subscribers_of` sees a subscriber there too: `scanned` puts the subscriber in app b's
    /// second file instead.
    fn workspace(header: &str, scanned: bool) -> Workspace {
        let sub = format!("{header}\n{SUB}");
        let (b_files, s_files): (Files, Files) = if scanned {
            (
                vec![("b.al", "namespace B;\ncodeunit 20 Pub { }"), ("bs.al", &sub)],
                vec![],
            )
        } else {
            (
                vec![("b.al", "namespace B;\ncodeunit 20 Pub { }")],
                vec![("s.al", &sub)],
            )
        };
        Workspace::load_apps(
            vec![
                app("s", &["a", "b"], &s_files),
                app("a", &[], &[("a.al", "namespace A;\ncodeunit 10 Pub { }")]),
                app("b", &["a"], &b_files),
            ],
            vec![AppKind::Workspace, AppKind::Dependency, AppKind::Dependency],
            &Default::default(),
        )
    }

    fn ix(ws: &Workspace, id: u32) -> u32 {
        ws.objects.iter().position(|o| o.id == Some(id)).unwrap() as u32
    }

    fn resolve(header: &str, parts: &[&str]) -> (PublisherRef, u32, u32) {
        let ws = workspace(header, false);
        let file = ws.files.iter().position(|f| f.path == "s.al").unwrap();
        let parts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
        (
            ws.subscriber_publisher(file, ObjKind::Codeunit, &parts),
            ix(&ws, 10),
            ix(&ws, 20),
        )
    }

    #[test]
    fn the_measured_shapes_pick_a_twin() {
        for (header, parts, want_a) in [
            ("namespace S;\nusing B;", &["Pub"][..], false),
            ("namespace A;", &["Pub"][..], true),
            ("namespace A;\nusing B;", &["Pub"][..], true),
            ("", &["A", "Pub"][..], true),
            ("namespace S;\nusing B;", &["A", "Pub"][..], true),
            ("", &["B", "Pub"][..], false),
        ] {
            let (got, a, b) = resolve(header, parts);
            let object = if want_a { a } else { b };
            assert_eq!(
                got,
                PublisherRef::Picked {
                    object,
                    twins: vec![a, b]
                },
                "{header:?} {parts:?}"
            );
        }
    }

    #[test]
    fn every_other_shape_picks_none() {
        for (header, parts) in [
            ("", &["Pub"][..]),
            ("using B;", &["Pub"][..]),
            ("namespace S;", &["Pub"][..]),
            ("namespace S;\nusing A;\nusing B;", &["Pub"][..]),
            ("", &["C", "Pub"][..]),
        ] {
            let (got, a, b) = resolve(header, parts);
            assert_eq!(got, PublisherRef::Unpicked(vec![a, b]), "{header:?} {parts:?}");
        }
    }

    #[test]
    fn a_name_one_visible_object_has_is_that_object() {
        let (got, a, _) = resolve("namespace S;", &["10"]);
        assert_eq!(got, PublisherRef::Only(a));
        let (got, ..) = resolve("namespace S;", &["Missing"]);
        assert_eq!(got, PublisherRef::None);
        // App a sees only its own Pub: no twin from there.
        let ws = workspace("", false);
        let file = ws.files.iter().position(|f| f.path == "a.al").unwrap();
        assert_eq!(
            ws.subscriber_publisher(file, ObjKind::Codeunit, &["Pub".to_string()]),
            PublisherRef::Only(ix(&ws, 10))
        );
    }

    /// A subscriber that is not compiled: a plain name stays under every twin, a qualified one under its twin only, as by the
    /// written name.
    #[test]
    fn a_scanned_subscriber_is_indexed_as_by_its_written_name() {
        let ws = workspace("namespace A;", true);
        let (a, b) = (ix(&ws, 10) as usize, ix(&ws, 20) as usize);
        let sub = ws.objects.iter().position(|o| o.name == "Sub").unwrap();
        assert_eq!(ws.subscribers_of(a), [(sub, 0)]);
        assert_eq!(ws.subscribers_of(b), [(sub, 0)]);
        let qualified = Workspace::load_apps(
            vec![
                app("a", &[], &[("a.al", "namespace A;\ncodeunit 10 Pub { }")]),
                app(
                    "b",
                    &["a"],
                    &[
                        ("b.al", "namespace B;\ncodeunit 20 Pub { }"),
                        ("bs.al", &SUB.replace("Codeunit::Pub", "Codeunit::A.Pub")),
                    ],
                ),
            ],
            vec![AppKind::Dependency, AppKind::Dependency],
            &Default::default(),
        );
        let sub = qualified.objects.iter().position(|o| o.name == "Sub").unwrap();
        assert_eq!(qualified.subscribers_of(ix(&qualified, 10) as usize), [(sub, 0)]);
        assert!(qualified.subscribers_of(ix(&qualified, 20) as usize).is_empty());
    }
}
