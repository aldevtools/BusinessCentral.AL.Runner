//! The canonical dump of a workspace (ADR-022 amendment 2026-11-15, "Equality requirement"): every field that a run
//! reads after the compile, in a fixed order, so that two workspaces compare as text. Destructured without `..`: a new
//! field must be dumped or named here with the reason.

use super::*;
use std::fmt::Write as _;

/// A map's entries in key order, as text.
fn sorted<K: std::fmt::Debug + Ord, V: std::fmt::Debug>(m: &HashMap<K, V>) -> String {
    let mut v: Vec<(&K, &V)> = m.iter().collect();
    v.sort_by(|a, b| a.0.cmp(b.0));
    format!("{v:?}")
}

impl Workspace {
    /// The canonical dump (see the module). Parse trees are not in it: a tree is a function of the text, which is.
    pub fn canonical_dump(&self) -> String {
        let Workspace {
            apps,
            files,
            objects,
            dotnet,
            diagnostics,
            kinds,
            visible,
            by_name,
            by_id,
            symbols,
            ext_target,
            ext_of,
            subscribers_of,
            subscribers_by_publisher,
            implementers_of,
            iface_base,
            iface_derived,
            // Filled on first ask from the objects and the files, which are dumped.
            usable_cache: _,
            // A defect report of a second parse, never part of a compile.
            flakes: _,
            compiled,
            unreached,
            // The way to read a dependency file, and what it read: the text, which the source hash covers.
            loaders: _,
            lazy_text: _,
            // How many files the load read: the stored load gives each file without reading it.
            loaded_files: _,
            user_name_fields,
            internals_granted,
        } = self;
        let mut out = String::new();
        for (i, a) in apps.iter().enumerate() {
            let AppSource {
                id,
                name,
                dependencies,
                files,
            } = a;
            let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
            let _ = writeln!(out, "app {i} {id} {name} {dependencies:?} {:?} {paths:?}", kinds[i]);
            let _ = writeln!(out, "app {i} visible {:?} symbols {:?}", visible[i], symbols[i]);
        }
        for (fi, f) in files.iter().enumerate() {
            let SourceFileData {
                path,
                app,
                file_in_app,
                // A tree is a function of the text.
                green: _,
                problem,
                lazy,
                has_page,
                namespace,
                usings,
                synthetic,
                max_length: _,
                access_by_permission: _,
            } = f;
            // A lazily parsed file's problem and synthetic ranges are a function of its text, found the first time they are
            // asked for (`file_problem`, `synthetic`), whichever compile asked: not dumped. A workspace file's synthetic
            // ranges are its parse's at load (a workspace file reports parse errors as diagnostics, not as a problem).
            let _ = problem;
            let synthetic = if *lazy { None } else { synthetic.get() };
            let _ = writeln!(
                out,
                "file {fi} {path} app {app} at {file_in_app} lazy {lazy} page {has_page} ns {namespace:?} usings {usings:?} synthetic {synthetic:?}"
            );
        }
        for (oi, o) in objects.iter().enumerate() {
            let _ = writeln!(out, "object {oi} {o:?} ext_target {:?}", ext_target.get(oi));
        }
        for d in dotnet {
            let _ = writeln!(out, "dotnet {} {} {:?}", d.app, d.file, d.decl);
        }
        for d in diagnostics {
            let _ = writeln!(out, "diagnostic {d:?}");
        }
        let _ = writeln!(out, "by_name {}", sorted(by_name));
        let _ = writeln!(out, "by_id {}", sorted(by_id));
        let _ = writeln!(out, "ext_of {}", sorted(ext_of));
        let _ = writeln!(out, "subscribers_of {}", sorted(subscribers_of));
        let _ = writeln!(out, "subscribers_by_publisher {}", sorted(subscribers_by_publisher));
        let _ = writeln!(out, "implementers_of {}", sorted(implementers_of));
        let _ = writeln!(out, "iface_base {}", sorted(iface_base));
        let _ = writeln!(out, "iface_derived {}", sorted(iface_derived));
        let _ = writeln!(out, "compiled {:?}", compiled.get());
        let mut unreached: Option<Vec<&(usize, usize)>> = unreached.get().map(|u| u.iter().collect());
        if let Some(u) = unreached.as_mut() {
            u.sort_unstable();
        }
        let _ = writeln!(out, "unreached {unreached:?}");
        let _ = writeln!(
            out,
            "user_name_fields {user_name_fields} internals_granted {internals_granted:?}"
        );
        out
    }
}
