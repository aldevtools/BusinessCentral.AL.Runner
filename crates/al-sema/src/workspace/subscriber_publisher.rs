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
    /// The candidates are [`Self::publisher_candidates`]. None: [`PublisherRef::NotVisible`]. One: [`PublisherRef::Only`].
    /// Several: [`PublisherRef::Picked`] when [`Self::pick_twin`] names one, else [`PublisherRef::Unpicked`].
    pub fn subscriber_publisher(&self, file: usize, kind: ObjKind, parts: &[String]) -> PublisherRef {
        let twins = self.publisher_candidates(file, kind, parts);
        let index = |i: usize| i as u32;
        match twins.as_slice() {
            [] => PublisherRef::NotVisible,
            [one] => PublisherRef::Only(index(*one)),
            _ => {
                let all = twins.iter().copied().map(index).collect();
                match self.pick_twin(file, kind, parts, &twins) {
                    Some(object) => PublisherRef::Picked {
                        object: index(object),
                        twins: all,
                    },
                    None => PublisherRef::Unpicked(all),
                }
            }
        }
    }

    /// The objects of kind `kind` and the name (or, for one numeric part, the id) of `parts` in the apps `file`'s app sees,
    /// without the moved stubs that give way to a real object (`without_moved_stubs`).
    fn publisher_candidates(&self, file: usize, kind: ObjKind, parts: &[String]) -> Vec<usize> {
        let Some(name) = parts.last() else {
            return Vec::new();
        };
        let named = match parts {
            [one] => match one.parse::<u32>() {
                Ok(id) => self.by_id.get(&(kind, id)),
                Err(_) => self.by_name.get(&(kind, name.to_lowercase())),
            },
            _ => self.by_name.get(&(kind, name.to_lowercase())),
        };
        let app = self.files[file].app;
        let visible: Vec<usize> = named
            .into_iter()
            .flatten()
            .copied()
            .filter(|&i| self.visible[app].contains(&self.objects[i].app))
            .collect();
        self.without_moved_stubs(visible)
    }

    /// The one of `twins` that the publisher name `parts`, written in `file`, names; `None` outside the measured shapes. The
    /// shapes are measured on codeunit twins that each have a namespace (corpus PR 1423, events/subscriber-namespace/: two
    /// codeunits "Shared Name" in namespaces of two apps), so `kind` must be `Codeunit` and every twin must have a namespace.
    /// Then each shape is required by equality:
    /// - a qualified name in a file with a namespace: the one twin whose namespace is the written one (the reporter's alc 18.1
    ///   compiles `Codeunit::Repro.Copy."Shared Name"` in a file of namespace Repro.Tests, issue 8171; corpus PR 1423
    ///   SnUsingTests.al, each qualified name, file namespace LT.SN.Tests);
    /// - a plain name in a file with a namespace that exactly one twin has (corpus PR 1423 SnOwnNamespaceTests.al), whatever the
    ///   file's `using`s (SnOwnOverUsingTests.al);
    /// - a plain name in a file with a namespace that no twin has, whose `using`s name the namespace of exactly one twin (issue
    ///   8171, alc 18.1 binds through `using Repro.Copy;` to 51655; SnUsingTests.al).
    ///
    /// Not picked: table, page and other kinds of twins; a twin without a namespace; an id two objects have; a file without a
    /// namespace, for a qualified name (not measured) and a plain one (alc ignores its `using`s, AL0789, and reports AL0275,
    /// must-not-compile 3171); `using`s of two twins' namespaces
    /// (AL0275, 3172); no `using` of a twin's namespace (AL0118, 3173); two twins in the namespace that decides.
    fn pick_twin(&self, file: usize, kind: ObjKind, parts: &[String], twins: &[usize]) -> Option<usize> {
        let measured = kind == ObjKind::Codeunit && twins.iter().all(|&i| self.objects[i].namespace.is_some());
        let by_id = matches!(parts, [one] if one.parse::<u32>().is_ok());
        if !measured || by_id {
            return None;
        }
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
        let f = &self.files[file];
        let file_ns = Some(f.namespace.clone()?);
        if parts.len() > 1 {
            return only(in_ns(&Some(parts[..parts.len() - 1].join("."))));
        }
        let own = in_ns(&file_ns);
        if !own.is_empty() {
            return only(own);
        }
        only(f.usings.iter().flat_map(|u| in_ns(&Some(u.clone()))).collect())
    }

    /// The objects whose events a subscriber of a scanned object with publisher name `parts`, written in `file`, is indexed under
    /// (`Workspace::subscribers_of`, which the subscribers that are not compiled come from). Such a subscriber does not run: an
    /// event of an object it is indexed under is refused. As the index by written name did, among the visible candidates
    /// ([`Self::publisher_candidates`]): a plain name or an id under each of them, a qualified name under those of the written
    /// namespace. No twin is picked here: that a subscriber that does not run leaves the other twins' events alone is not
    /// measured.
    pub(super) fn scanned_subscriber_publishers(&self, file: usize, kind: ObjKind, parts: &[String]) -> Vec<usize> {
        let candidates = self.publisher_candidates(file, kind, parts);
        if parts.len() < 2 {
            return candidates;
        }
        let ns = Some(parts[..parts.len() - 1].join("."));
        candidates
            .into_iter()
            .filter(|&i| eq_ns(&self.objects[i].namespace, &ns))
            .collect()
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
            ("namespace S;\nusing B;", &["A", "Pub"][..], true),
            ("namespace S;\nusing B;", &["B", "Pub"][..], false),
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
            ("namespace S;", &["C", "Pub"][..]),
            // A qualified name in a file without a namespace: the probe's qualified subscribers sit in a file with one.
            ("", &["A", "Pub"][..]),
            ("", &["B", "Pub"][..]),
        ] {
            let (got, a, b) = resolve(header, parts);
            assert_eq!(got, PublisherRef::Unpicked(vec![a, b]), "{header:?} {parts:?}");
        }
        // Twins the probe did not measure, in the shapes that pick between two namespaced codeunits: a twin without a namespace,
        // and table and page twins.
        let global = "codeunit 10 Pub { }";
        let cu_b = "namespace B;\ncodeunit 20 Pub { }";
        let (tab_a, tab_b) = ("namespace A;\ntable 10 Pub { }", "namespace B;\ntable 20 Pub { }");
        let (page_a, page_b) = ("namespace A;\npage 10 Pub { }", "namespace B;\npage 20 Pub { }");
        for (a_src, b_src, kind, header, parts) in [
            (global, cu_b, ObjKind::Codeunit, "namespace S;\nusing B;", &["Pub"][..]),
            (global, cu_b, ObjKind::Codeunit, "namespace B;", &["Pub"][..]),
            (global, cu_b, ObjKind::Codeunit, "namespace S;", &["B", "Pub"][..]),
            (global, cu_b, ObjKind::Codeunit, "", &["Pub"][..]),
            (tab_a, tab_b, ObjKind::Table, "namespace S;\nusing B;", &["Pub"][..]),
            (tab_a, tab_b, ObjKind::Table, "namespace A;", &["Pub"][..]),
            (tab_a, tab_b, ObjKind::Table, "", &["A", "Pub"][..]),
            (page_a, page_b, ObjKind::Page, "namespace S;\nusing B;", &["Pub"][..]),
            (page_a, page_b, ObjKind::Page, "", &["B", "Pub"][..]),
        ] {
            let ws = Workspace::load_apps(
                vec![
                    app("s", &["a", "b"], &[("s.al", &format!("{header}\n{SUB}"))]),
                    app("a", &[], &[("a.al", a_src)]),
                    app("b", &["a"], &[("b.al", b_src)]),
                ],
                vec![AppKind::Workspace, AppKind::Dependency, AppKind::Dependency],
                &Default::default(),
            );
            let file = ws.files.iter().position(|f| f.path == "s.al").unwrap();
            let parts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
            let (a, b) = (ix(&ws, 10), ix(&ws, 20));
            assert_eq!(
                ws.subscriber_publisher(file, kind, &parts),
                PublisherRef::Unpicked(vec![a, b]),
                "{a_src:?} {header:?} {parts:?}"
            );
        }
    }

    #[test]
    fn a_name_one_visible_object_has_is_that_object() {
        let (got, a, _) = resolve("namespace S;", &["10"]);
        assert_eq!(got, PublisherRef::Only(a));
        let (got, ..) = resolve("namespace S;", &["Missing"]);
        assert_eq!(got, PublisherRef::NotVisible);
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
