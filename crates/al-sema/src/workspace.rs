//! A workspace is a set of apps. Each app is its manifest plus its sources
//! (path, text). Loading parses every file and builds the object table.

use crate::check::posmap::{FxMap as HashMap, FxSet as HashSet};
use al_syntax::ast;
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::sync::OnceLock;

pub use al_ir::ObjKind;

#[derive(Debug, Clone)]
pub struct AppSource {
    pub id: String,
    pub name: String,
    pub dependencies: Vec<String>,
    pub files: Vec<(String, String)>,
}

/// Reads the text of one file of a dependency app when something needs it (ADR-022, "pure tests"). The
/// text is what an eager read of the same file gives: no byte order mark, invalid UTF-8 replaced.
pub trait SourceLoader: Send + Sync + std::fmt::Debug {
    /// The text of file `file_in_app` of the app (the position in `AppSource::files`).
    fn text(&self, file_in_app: usize) -> String;
}

/// What a run knows of a dependency app whose files hold no text yet (`AppSource::files` has the
/// paths and empty texts): how to read a file, and a hash of the app's source made without reading it
/// (for a zip package, of the name, size and CRC-32 of every entry).
#[derive(Debug, Clone)]
pub struct LazySource {
    pub loader: std::sync::Arc<dyn SourceLoader>,
    pub identity: [u8; 32],
}

impl LazySource {
    /// Reads every file of `app` into its `files`, on all cores.
    pub fn fill(&self, app: &mut AppSource) {
        let texts: Vec<String> = (0..app.files.len())
            .into_par_iter()
            .map(|i| self.loader.text(i))
            .collect();
        for ((_, slot), text) in app.files.iter_mut().zip(texts) {
            *slot = text;
        }
    }
}

/// What the scan of one source file found: the index entry of a file that is not parsed at load.
/// A cache stores these for a dependency set so that a run need not read or scan the sources.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileIndex {
    pub namespace: Option<String>,
    pub usings: Vec<String>,
    /// The objects of the file, in source order.
    pub objects: Vec<FileObject>,
    /// The `type(..)` declarations of the file's `dotnet { }` blocks.
    pub dotnet: Vec<ast::DotNetTypeDecl>,
}

/// A dependency app for [`Workspace::load_apps_lazy`]: the scan of each of its files (so nothing is
/// scanned) and the way to read a file later.
pub struct LazyDep {
    pub index: Vec<FileIndex>,
    pub loader: std::sync::Arc<dyn SourceLoader>,
}

/// Whether an app is the code being tested or a dependency it was compiled against.
/// No test is discovered or run in a dependency (ADR-013 decision 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppKind {
    #[default]
    Workspace,
    Dependency,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub file: String,
    /// The app of `file` (index into `Workspace::apps`): two apps can hold files of the same path.
    pub app: usize,
    pub offset: usize,
    pub code: &'static str,
    pub message: String,
}

pub struct SourceFileData {
    pub path: String,
    pub app: usize,
    /// Position of the file in `apps[app].files`.
    pub file_in_app: usize,
    /// The parse tree. A file of a dependency app is only scanned at load and parsed
    /// the first time something needs it (`Workspace::green`).
    green: OnceLock<rowan::GreenNode>,
    /// For a lazily parsed file: why its objects cannot be used, if its source does not
    /// parse cleanly (the first parse error, with its line).
    problem: OnceLock<Option<String>>,
    /// Whether the file was only scanned at load (a dependency app's file).
    pub lazy: bool,
    /// Whether the file declares a page, report, table or one of their extensions, the only objects `pages::expand_ranges`
    /// changes.
    has_page: bool,
    pub namespace: Option<String>,
    pub usings: Vec<String>,
    /// Byte ranges of the text the compiler added to the file (page expression routines). The
    /// green tree holds that text; source positions leave it out. Known once the file is parsed.
    synthetic: OnceLock<Vec<ByteRange>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    Found(usize),
    Missing,
    Ambiguous(Vec<usize>),
}

#[derive(Debug, Clone)]
pub struct ObjectSym {
    pub app: usize,
    pub file: usize,
    pub kind: ObjKind,
    pub id: Option<u32>,
    pub name: String,
    pub namespace: Option<String>,
    /// Offset of the object node in its file (in the original text for a scanned file).
    pub offset: usize,
    /// Position among the objects of its file, which finds its node after a (lazy) parse.
    pub ordinal: usize,
    /// `extends X` of an extension object, as name parts.
    pub extends: Option<Vec<String>>,
    pub implements: Vec<Vec<String>>,
    /// Event subscriptions it declares, for objects of scanned files.
    pub subscribers: Vec<al_syntax::scan::ScannedSubscriber>,
    /// The properties written directly in the body of an object of a scanned file, as
    /// (name, value text). Empty for an object of a parsed file.
    pub scanned_properties: Vec<(String, String)>,
    /// The layouts a scanned report declares.
    pub scanned_layouts: Vec<al_syntax::scan::ScannedLayout>,
    /// A codeunit with `Subtype = Install` (known for objects of scanned files only).
    pub install: bool,
    /// The object declares `EventSubscriberInstance = Manual` (known for objects of scanned files
    /// only): its subscribers fire only while an instance is bound.
    pub manual_subscribers: bool,
}

/// A byte range of a parsed text.
type ByteRange = (usize, usize);

/// A `DotNet` alias declared by a `dotnet { }` block (ADR-014).
#[derive(Debug, Clone)]
pub struct DotNetSym {
    pub app: usize,
    pub file: usize,
    pub decl: ast::DotNetTypeDecl,
}

pub struct Workspace {
    pub apps: Vec<AppSource>,
    pub files: Vec<SourceFileData>,
    pub objects: Vec<ObjectSym>,
    /// Every declared `DotNet` alias, in file and source order. The index is the one
    /// `RtType::DotNet` and `ir::Program::dotnet_types` use.
    pub dotnet: Vec<DotNetSym>,
    pub diagnostics: Vec<Diagnostic>,
    /// The kind of each app, by index in `apps`.
    pub kinds: Vec<AppKind>,
    /// For each app, the set of apps it can see (itself + transitive dependencies).
    visible: Vec<BTreeSet<usize>>,
    by_name: HashMap<(ObjKind, String), Vec<usize>>,
    by_id: HashMap<(ObjKind, u32), Vec<usize>>,
    /// The preprocessor symbols of each app, for parsing files later.
    symbols: Vec<BTreeSet<String>>,
    /// For an extension object, the object it extends (found without parsing its target).
    ext_target: Vec<Option<usize>>,
    /// For an extended object, its extensions in object order.
    ext_of: HashMap<usize, Vec<usize>>,
    /// Event subscribers of scanned objects by publisher: (kind, lower-case name or id) to
    /// (subscriber object, index in its `subscribers`).
    subscribers_of: HashMap<(ObjKind, String), Vec<(usize, usize)>>,
    /// Scanned objects that implement an interface, by lower-case interface name.
    implementers_of: HashMap<String, Vec<usize>>,
    /// For an interface that `extends` another, the interface it extends.
    iface_base: HashMap<usize, usize>,
    /// For an interface, the interfaces that extend it directly, in object order.
    iface_derived: HashMap<usize, Vec<usize>>,
    /// Per object: whether it can be used (see `usable`); filled on first ask.
    usable_cache: Vec<OnceLock<bool>>,
    /// See [`Workspace::parse_flakes`].
    flakes: std::sync::Mutex<Vec<String>>,
    /// Once set, the dependency objects that take part in the compile (see `restrict_to`).
    compiled: OnceLock<Vec<bool>>,
    /// Once set (`Roots::reach_only`): the (object, routine offset) of the workspace procedures that
    /// no selected test names; they are not checked and compile to stubs that refuse.
    unreached: OnceLock<HashSet<(usize, usize)>>,
    /// Per app: how to read a file of a dependency app whose `files` hold no text (`load_apps_lazy`).
    loaders: Vec<Option<std::sync::Arc<dyn SourceLoader>>>,
    /// Per file: the text a loader gave, kept so a file is read once.
    lazy_text: Vec<OnceLock<String>>,
}

impl Workspace {
    /// The preprocessor symbols app `app` was parsed with.
    pub fn symbols_of(&self, app: usize) -> &BTreeSet<String> {
        &self.symbols[app]
    }

    pub fn load(apps: Vec<AppSource>, symbols: &BTreeSet<String>) -> Workspace {
        let kinds = vec![AppKind::Workspace; apps.len()];
        Workspace::load_apps(apps, kinds, symbols)
    }

    /// Like `load`, with the kind of each app. The files of a dependency app are not parsed
    /// here: a cheap scan indexes their objects, and a file is parsed when something needs
    /// one of its objects (ADR-013 decision 6).
    pub fn load_apps(apps: Vec<AppSource>, kinds: Vec<AppKind>, symbols: &BTreeSet<String>) -> Workspace {
        Workspace::load_apps_lazy(apps, kinds, symbols, Vec::new())
    }

    /// [`load_apps`](Self::load_apps) where the dependency apps in `lazy` (by app index; missing or
    /// `None`: an app that has its text) are not scanned: their index comes with them, and their text
    /// is read through the loader the first time a file is parsed or its lines are needed. The
    /// result is the workspace `load_apps` makes from the same sources.
    pub fn load_apps_lazy(
        apps: Vec<AppSource>,
        kinds: Vec<AppKind>,
        symbols: &BTreeSet<String>,
        lazy: Vec<Option<LazyDep>>,
    ) -> Workspace {
        Workspace::load_apps_symbols(apps, kinds, symbols, &[], lazy)
    }

    /// [`load_apps_lazy`](Self::load_apps_lazy) where app `i` with `app_symbols[i]` set is preprocessed
    /// with those symbols and not with `symbols` (a dependency app is compiled by its publisher with
    /// the symbols of its own app.json; the project's symbols are for the project's apps).
    pub fn load_apps_symbols(
        apps: Vec<AppSource>,
        kinds: Vec<AppKind>,
        symbols: &BTreeSet<String>,
        app_symbols: &[Option<BTreeSet<String>>],
        mut lazy: Vec<Option<LazyDep>>,
    ) -> Workspace {
        let symbols_of: Vec<BTreeSet<String>> = (0..apps.len())
            .map(|i| app_symbols.get(i).cloned().flatten().unwrap_or_else(|| symbols.clone()))
            .collect();
        assert_eq!(apps.len(), kinds.len(), "one kind per app");
        lazy.resize_with(apps.len(), || None);
        for (i, l) in lazy.iter().enumerate() {
            if let Some(l) = l {
                assert_eq!(l.index.len(), apps[i].files.len(), "one index entry per file");
                assert_eq!(kinds[i], AppKind::Dependency, "only a dependency app is lazy");
            }
        }
        let loaders: Vec<Option<std::sync::Arc<dyn SourceLoader>>> =
            lazy.iter().map(|l| l.as_ref().map(|l| l.loader.clone())).collect();
        let mut indexes: Vec<Option<std::vec::IntoIter<FileIndex>>> =
            lazy.into_iter().map(|l| l.map(|l| l.index.into_iter())).collect();
        let mut ws = Workspace {
            kinds,
            visible: Vec::new(),
            files: Vec::new(),
            objects: Vec::new(),
            dotnet: Vec::new(),
            diagnostics: Vec::new(),
            by_name: HashMap::default(),
            by_id: HashMap::default(),
            apps: Vec::new(),
            symbols: symbols_of.clone(),
            ext_target: Vec::new(),
            ext_of: HashMap::default(),
            subscribers_of: HashMap::default(),
            implementers_of: HashMap::default(),
            iface_base: HashMap::default(),
            iface_derived: HashMap::default(),
            usable_cache: Vec::new(),
            flakes: Default::default(),
            compiled: OnceLock::new(),
            unreached: OnceLock::new(),
            loaders,
            lazy_text: Vec::new(),
        };
        ws.visible = (0..apps.len()).map(|i| visible_from(&apps, i)).collect();
        // Parsing dominates loading and each file is independent, so it runs on
        // all cores. `collect` keeps the input order, so the sequential merge
        // below sees files, diagnostics and objects exactly as a serial load would.
        // Expression-valued page properties become routines first (ADR-012).
        let inputs: Vec<(usize, usize, bool, &str, &str)> = apps
            .iter()
            .enumerate()
            .flat_map(|(ai, app)| {
                let lazy = ws.kinds[ai] == AppKind::Dependency;
                app.files
                    .iter()
                    .enumerate()
                    .map(move |(fi, (path, src))| (ai, fi, lazy, path.as_str(), src.as_str()))
            })
            .collect();
        // The files of a lazy app come with their scan.
        let given: Vec<Option<ParsedFile>> = inputs
            .iter()
            .map(|&(ai, ..)| {
                indexes[ai].as_mut().map(|it| {
                    let f = it.next().expect("one index entry per file");
                    ParsedFile {
                        green: None,
                        errors: Vec::new(),
                        namespace: f.namespace,
                        usings: f.usings,
                        objects: f.objects,
                        dotnet: f.dotnet,
                    }
                })
            })
            .collect();
        let parsed: Vec<(ParsedFile, Vec<ByteRange>)> = inputs
            .par_iter()
            .zip(given.into_par_iter())
            .map(|(&(ai, _, lazy, _, src), given)| {
                if let Some(file) = given {
                    return (file, Vec::new());
                }
                let symbols = &symbols_of[ai];
                if lazy {
                    (scanned_file(src, symbols), Vec::new())
                } else {
                    match crate::pages::expand_or_parse(src, symbols) {
                        crate::pages::Expansion::Expanded(text, added) => {
                            (with_tree_cache(|cache| parse_file(&text, symbols, cache)), added)
                        }
                        // The parse that looked for expressions is the file's parse.
                        crate::pages::Expansion::Unchanged(parse) => (parsed_file(parse), Vec::new()),
                    }
                }
            })
            .collect();
        for ((ai, file_in_app, lazy, path, _), (file, added)) in inputs.into_iter().zip(parsed) {
            for e in &file.errors {
                ws.diagnostics.push(Diagnostic {
                    file: path.to_string(),
                    app: ai,
                    offset: e.offset,
                    code: "AR0000",
                    message: e.message.clone(),
                });
            }
            let fi = ws.files.len();
            ws.files.push(SourceFileData {
                path: path.to_string(),
                app: ai,
                file_in_app,
                green: file.green.map(OnceLock::from).unwrap_or_default(),
                problem: OnceLock::new(),
                lazy,
                has_page: file.objects.iter().any(|o| {
                    matches!(
                        o.kind,
                        ObjKind::Page
                            | ObjKind::PageExtension
                            | ObjKind::Report
                            | ObjKind::ReportExtension
                            | ObjKind::Table
                            | ObjKind::TableExtension
                    )
                }),
                namespace: file.namespace.clone(),
                usings: file.usings,
                synthetic: if lazy { OnceLock::new() } else { OnceLock::from(added) },
            });
            for decl in file.dotnet {
                ws.dotnet.push(DotNetSym {
                    app: ai,
                    file: fi,
                    decl,
                });
            }
            for (ordinal, o) in file.objects.into_iter().enumerate() {
                ws.add_object(ObjectSym {
                    app: ai,
                    file: fi,
                    kind: o.kind,
                    id: o.id,
                    name: o.name,
                    namespace: file.namespace.clone(),
                    offset: o.offset,
                    ordinal,
                    extends: o.extends,
                    implements: o.implements,
                    subscribers: o.subscribers,
                    scanned_properties: o.properties,
                    scanned_layouts: o.layouts,
                    install: o.install,
                    manual_subscribers: o.manual_subscribers,
                });
            }
        }
        ws.apps = apps;
        ws.lazy_text = ws.files.iter().map(|_| OnceLock::new()).collect();
        ws.usable_cache = (0..ws.objects.len()).map(|_| OnceLock::new()).collect();
        ws.index_extensions_and_subscribers();
        ws.resolve_references();
        let positions: HashMap<&str, usize> = ws.files.iter().enumerate().map(|(i, f)| (f.path.as_str(), i)).collect();
        let key = |d: &Diagnostic| (positions.get(d.file.as_str()).copied().unwrap_or(usize::MAX), d.offset);
        let mut diagnostics = std::mem::take(&mut ws.diagnostics);
        diagnostics.sort_by_key(key);
        ws.diagnostics = diagnostics;
        ws
    }

    /// The incremental compile's way in: replaces the text of file `fi` of a workspace app with `text` when
    /// the new text differs from the old one only inside the bodies of procedures and triggers (the
    /// declarations, in tokens, are the same: `skeleton`). Then no other object can tell the files apart
    /// except through the offsets of this file's routines. The objects of the file keep their numbers; their
    /// offsets, the tree, and the diagnostics of the file are renewed, and the result is what a load of
    /// the new text gives. The objects of the file, in order, are returned.
    ///
    /// An error says why the text cannot be taken that way (the file is a dependency's, it does not
    /// parse cleanly, or a declaration changed); nothing has changed then.
    pub fn replace_file(&mut self, fi: usize, text: String) -> Result<Vec<usize>, String> {
        self.replace_file_with(fi, text, false).map(|(objects, _)| objects)
    }

    /// [`replace_file`](Self::replace_file) that also takes a text whose declarations differ (a procedure more, a
    /// changed signature, a variable). The objects of the file must be the same ones: kind, id, name, what they
    /// extend and implement. The flag says whether a declaration changed (ADR-033 step 5: the caller then has the
    /// readers of the objects checked and lowered again).
    pub fn replace_file_declared(&mut self, fi: usize, text: String) -> Result<(Vec<usize>, bool), String> {
        self.replace_file_with(fi, text, true)
    }

    fn replace_file_with(&mut self, fi: usize, text: String, declarations: bool) -> Result<(Vec<usize>, bool), String> {
        let file = self.files.get(fi).ok_or("no such file")?;
        if file.lazy {
            return Err("a file of a dependency app".into());
        }
        let (ai, file_in_app, path) = (file.app, file.file_in_app, file.path.clone());
        let old_green = file.green.get().cloned().ok_or("the file was never parsed")?;
        let symbols = &self.symbols[ai];
        let (src, added): (String, Vec<ByteRange>) = match crate::pages::expand_ranges(&text, symbols) {
            Some((expanded, added)) => (expanded, added),
            None => (text.clone(), Vec::new()),
        };
        let parsed = with_tree_cache(|cache| parse_file(&src, symbols, cache));
        if !parsed.errors.is_empty() {
            return Err("the new text does not parse cleanly".into());
        }
        let green = parsed.green.clone().ok_or("no tree")?;
        let declared = crate::incremental::skeleton(&old_green) != crate::incremental::skeleton(&green);
        if declared && !declarations {
            return Err("a declaration changed".into());
        }
        let in_file: Vec<usize> = (0..self.objects.len())
            .filter(|&i| self.objects[i].file == fi)
            .collect();
        let same = in_file.len() == parsed.objects.len()
            && in_file.iter().zip(&parsed.objects).all(|(&i, n)| {
                let o = &self.objects[i];
                (o.kind, o.id, &o.name, &o.extends, &o.implements) == (n.kind, n.id, &n.name, &n.extends, &n.implements)
            })
            && parsed.namespace == self.files[fi].namespace
            && parsed.usings == self.files[fi].usings;
        if !same {
            return Err("the objects of the file changed".into());
        }
        let dotnet_now: Vec<&DotNetSym> = self.dotnet.iter().filter(|d| d.file == fi).collect();
        if dotnet_now.len() != parsed.dotnet.len() {
            return Err("the dotnet declarations changed".into());
        }
        // Everything holds: take it.
        for (&i, n) in in_file.iter().zip(&parsed.objects) {
            self.objects[i].offset = n.offset;
        }
        self.apps[ai].files[file_in_app].1 = text;
        let f = &mut self.files[fi];
        f.green = OnceLock::from(green);
        f.problem = OnceLock::new();
        f.synthetic = OnceLock::from(added);
        for (d, new) in self.dotnet.iter_mut().filter(|d| d.file == fi).zip(&parsed.dotnet) {
            d.decl = new.clone();
        }
        // The diagnostics of the file: its reference errors are renewed (the parse errors are none).
        let renewed: Vec<Diagnostic> = in_file
            .iter()
            .flat_map(|&i| self.unresolved_references_of(&self.objects[i]))
            .collect();
        self.diagnostics.retain(|d| d.file != path || d.app != ai);
        self.diagnostics.extend(renewed);
        let positions: HashMap<&str, usize> = self
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.as_str(), i))
            .collect();
        let key = |d: &Diagnostic| (positions.get(d.file.as_str()).copied().unwrap_or(usize::MAX), d.offset);
        let mut diagnostics = std::mem::take(&mut self.diagnostics);
        diagnostics.sort_by_key(key);
        self.diagnostics = diagnostics;
        Ok((in_file, declared))
    }

    /// Extension targets and event subscribers, found from the object index alone.
    fn index_extensions_and_subscribers(&mut self) {
        self.ext_target = vec![None; self.objects.len()];
        for i in 0..self.objects.len() {
            let o = &self.objects[i];
            if let (Some(kind), Some(parts)) = (o.kind.extended(), &o.extends) {
                // Not `resolve_ref`: that would parse the target's file, and an index of
                // thousands of extensions must not parse their targets.
                if let Resolution::Found(t) = self.resolve_in_scope(o.file, kind, parts, false, false) {
                    self.ext_target[i] = Some(t);
                    self.ext_of.entry(t).or_default().push(i);
                }
            }
        }
        for i in 0..self.objects.len() {
            let o = &self.objects[i];
            if let (ObjKind::Interface, Some(parts)) = (o.kind, &o.extends) {
                // Like the extension targets above: no parse of the base interface's file.
                if let Resolution::Found(b) = self.resolve_in_scope(o.file, ObjKind::Interface, parts, false, false) {
                    self.iface_base.insert(i, b);
                    self.iface_derived.entry(b).or_default().push(i);
                }
            }
        }
        for (i, o) in self.objects.iter().enumerate() {
            for (si, sub) in o.subscribers.iter().enumerate() {
                let kind = ObjKind::from_keyword(&sub.publisher_kind);
                let name = sub.publisher.join(".").to_lowercase();
                self.subscribers_of.entry((kind, name)).or_default().push((i, si));
            }
            for imp in &o.implements {
                if let Some(last) = imp.last() {
                    self.implementers_of.entry(last.to_lowercase()).or_default().push(i);
                }
            }
        }
    }

    /// The interfaces interface `i` extends, nearest first (`interface I extends J`, and J's own
    /// base after it). A cycle ends the list. Pinned by codeunit/TestInterfaceRuntimeContracts.al:
    /// `Probe IBar extends Probe IFoo` (:19).
    pub fn interface_bases(&self, i: usize) -> Vec<usize> {
        let mut out = Vec::new();
        let mut at = i;
        while let Some(&b) = self.iface_base.get(&at) {
            if b == i || out.contains(&b) {
                break;
            }
            out.push(b);
            at = b;
        }
        out
    }

    /// Interface `i` and every interface that extends it, directly or through others.
    pub fn interface_family(&self, i: usize) -> Vec<usize> {
        let mut out = vec![i];
        let mut next = 0;
        while next < out.len() {
            for &d in self.iface_derived.get(&out[next]).map_or(&[][..], Vec::as_slice) {
                if !out.contains(&d) {
                    out.push(d);
                }
            }
            next += 1;
        }
        out
    }

    /// The object `i` extends, if it is an extension and the target is in the workspace.
    pub fn extension_target(&self, i: usize) -> Option<usize> {
        self.ext_target[i]
    }

    /// The extensions of object `i`, in object order.
    pub fn extensions_of(&self, i: usize) -> &[usize] {
        self.ext_of.get(&i).map_or(&[], Vec::as_slice)
    }

    /// Subscribers (object, index in its `subscribers`) of scanned objects that name `publisher`
    /// as their publisher, by name or by id.
    pub fn subscribers_of(&self, publisher: usize) -> Vec<(usize, usize)> {
        let o = &self.objects[publisher];
        let mut keys = vec![o.name.to_lowercase()];
        if let Some(id) = o.id {
            keys.push(id.to_string());
        }
        // `Codeunit::Microsoft.Foo."Name"` writes the namespace; the index key is the written text.
        if let Some(ns) = &o.namespace {
            keys.push(format!("{ns}.{}", o.name).to_lowercase());
        }
        keys.into_iter()
            .flat_map(|k| self.subscribers_of.get(&(o.kind, k)).into_iter().flatten().copied())
            .collect()
    }

    /// Subscribers (object, index in its `subscribers`) of scanned objects to the platform codeunit
    /// `Global Triggers` (2000000002), which no app need declare: its events are raised by the
    /// runtime around record writes.
    pub fn global_trigger_subscribers(&self) -> Vec<(usize, usize)> {
        self.codeunit_subscribers(&["global triggers", "2000000002"])
    }

    /// Subscribers to the platform's `Caption Class` codeunit (42), whose `OnResolveCaptionClass`
    /// the page client raises for a `CaptionClass` no built-in area resolves.
    pub fn caption_class_subscribers(&self) -> Vec<(usize, usize)> {
        self.codeunit_subscribers(&["caption class", "42"])
    }

    /// Subscribers to the platform's `Reporting Triggers` codeunit, whose `CustomDocumentMergerEx` the runtime raises
    /// when a report renders with a custom layout.
    pub fn reporting_triggers_subscribers(&self) -> Vec<(usize, usize)> {
        self.codeunit_subscribers(&["reporting triggers"])
    }

    /// Subscribers to the platform's `Filter Helper Triggers` codeunit, whose `MakeDateFilter` (and `MakeIntFilter`, `MakeTimeFilter`,
    /// `RAISED_FILTER_EVENTS`) the runtime raises
    /// for a Date filter text with a piece that does not read.
    pub fn filter_helper_triggers_subscribers(&self) -> Vec<(usize, usize)> {
        self.codeunit_subscribers(&["filter helper triggers"])
    }

    /// Subscribers to the platform's `Extension Triggers` codeunit (2000000010): its `OnInstallAppPerCompany` and
    /// `OnInstallAppPerDatabase` run at the subscriber's app's install (ADR-046 decision 1).
    pub fn extension_triggers_subscribers(&self) -> Vec<(usize, usize)> {
        self.codeunit_subscribers(&["extension triggers", "2000000010"])
    }

    /// Subscribers to the platform's `Company Triggers` codeunit (2000000003): the install session raises its
    /// `OnCompanyOpen` and `OnCompanyOpenCompleted` before the per-company install code (ADR-046 amendment 1, A4 item 4).
    pub fn company_triggers_subscribers(&self) -> Vec<(usize, usize)> {
        self.codeunit_subscribers(&["company triggers", "2000000003"])
    }

    fn codeunit_subscribers(&self, keys: &[&str]) -> Vec<(usize, usize)> {
        keys.iter()
            .flat_map(|k| {
                self.subscribers_of
                    .get(&(ObjKind::Codeunit, (*k).to_string()))
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect()
    }

    /// Objects of scanned files that list `interface` (by its last name part) in `implements`.
    pub fn implementers_of(&self, interface: &str) -> &[usize] {
        self.implementers_of
            .get(&interface.to_lowercase())
            .map_or(&[], Vec::as_slice)
    }

    /// The text of file `fi` as it was given: for a file of a lazy dependency app, read now (once).
    pub fn text(&self, fi: usize) -> &str {
        let f = &self.files[fi];
        match &self.loaders[f.app] {
            Some(loader) => self.lazy_text[fi].get_or_init(|| loader.text(f.file_in_app)),
            None => &self.apps[f.app].files[f.file_in_app].1,
        }
    }

    /// The scan of every file of the dependency apps, per app (empty for a workspace app): what
    /// [`load_apps_lazy`](Self::load_apps_lazy) takes in place of the sources.
    pub fn export_index(&self) -> Vec<Vec<FileIndex>> {
        let mut out: Vec<Vec<FileIndex>> = vec![Vec::new(); self.apps.len()];
        let mut by_file: Vec<Vec<usize>> = vec![Vec::new(); self.files.len()];
        for (oi, o) in self.objects.iter().enumerate() {
            by_file[o.file].push(oi);
        }
        let mut dotnet: Vec<Vec<ast::DotNetTypeDecl>> = vec![Vec::new(); self.files.len()];
        for d in &self.dotnet {
            dotnet[d.file].push(d.decl.clone());
        }
        for (fi, f) in self.files.iter().enumerate() {
            if self.kinds[f.app] != AppKind::Dependency {
                continue;
            }
            let mut objects: Vec<&ObjectSym> = by_file[fi].iter().map(|&oi| &self.objects[oi]).collect();
            objects.sort_by_key(|o| o.ordinal);
            out[f.app].push(FileIndex {
                namespace: f.namespace.clone(),
                usings: f.usings.clone(),
                objects: objects
                    .into_iter()
                    .map(|o| FileObject {
                        kind: o.kind,
                        id: o.id,
                        name: o.name.clone(),
                        offset: o.offset,
                        extends: o.extends.clone(),
                        implements: o.implements.clone(),
                        subscribers: o.subscribers.clone(),
                        properties: o.scanned_properties.clone(),
                        layouts: o.scanned_layouts.clone(),
                        install: o.install,
                        manual_subscribers: o.manual_subscribers,
                    })
                    .collect(),
                dotnet: std::mem::take(&mut dotnet[fi]),
            });
        }
        out
    }

    /// The parse tree of file `fi`, parsing it now if it was only scanned at load.
    pub fn green(&self, fi: usize) -> rowan::GreenNode {
        let f = &self.files[fi];
        f.green
            .get_or_init(|| {
                let (kept, flake) = settle(self.parse_lazy(fi, f, false), || self.parse_lazy(fi, f, true));
                if let Some(note) = flake {
                    self.flakes
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .push(format!("{}: {note}", f.path));
                }
                let (green, synthetic) = kept.green;
                let _ = f.synthetic.set(synthetic);
                let _ = f.problem.set(kept.problem);
                green
            })
            .clone()
    }

    /// One parse of a lazily parsed file: the text with its synthetic routines, the tree, and the first
    /// parse error if there is one. `fresh` starts from an empty token cache.
    fn parse_lazy(&self, fi: usize, f: &SourceFileData, fresh: bool) -> Attempt<(rowan::GreenNode, Vec<ByteRange>)> {
        let src = self.text(fi);
        // Only a page, report or table can have properties to turn into routines; skip the extra parse otherwise. When there is nothing to
        // add, the parse that looked is the file's parse; a `fresh` parse (the second one, of a file whose first parse had
        // a problem) is made anew.
        let (expanded, reused) = match f
            .has_page
            .then(|| crate::pages::expand_or_parse(src, &self.symbols[f.app]))
        {
            Some(crate::pages::Expansion::Expanded(text, added)) => (Some((text, added)), None),
            Some(crate::pages::Expansion::Unchanged(parse)) if !fresh => (None, Some(parsed_file(parse))),
            _ => (None, None),
        };
        let text = expanded.as_ref().map(|(t, _)| t.as_str()).unwrap_or(src);
        let synthetic = expanded.as_ref().map(|(_, a)| a.clone()).unwrap_or_default();
        let parsed = match reused {
            Some(parsed) => parsed,
            None if fresh => parse_file(text, &self.symbols[f.app], &mut al_syntax::TreeCache::default()),
            None => with_tree_cache(|cache| parse_file(text, &self.symbols[f.app], cache)),
        };
        let problem = parsed.errors.first().map(|e| {
            let line = text[..e.offset.min(text.len())].matches('\n').count() + 1;
            let shown = text.lines().nth(line - 1).unwrap_or("").trim();
            format!("{}:{line}: {} (`{shown}`)", f.path, e.message)
        });
        Attempt {
            green: (parsed.green.expect("a parsed file has a tree"), synthetic),
            problem,
        }
    }

    /// Files whose second parse did not give the result of the first (a defect: a parse is a function of
    /// the text), as one line each with both results. Empty in every run so far.
    pub fn parse_flakes(&self) -> Vec<String> {
        self.flakes.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The byte ranges of file `fi`'s parsed text that the compiler added (page expression routines).
    pub fn synthetic(&self, fi: usize) -> &[ByteRange] {
        self.green(fi);
        self.files[fi].synthetic.get().map_or(&[], Vec::as_slice)
    }

    /// Why the objects of file `fi` cannot be used: its source does not parse cleanly.
    /// Only a scanned (dependency) file can have such a problem; the workspace's own files
    /// report parse errors as diagnostics. Parses the file if that has not happened yet.
    pub fn file_problem(&self, fi: usize) -> Option<&str> {
        let f = &self.files[fi];
        if !f.lazy {
            return None;
        }
        self.green(fi);
        f.problem.get().and_then(|p| p.as_deref())
    }

    /// From now on only the dependency objects marked in `compiled` resolve. The checker has
    /// decided which of them the workspace's code reaches; the others are not compiled, so
    /// code that needs one anyway is refused as outside the workspace instead of finding it
    /// half there. Set once; a second call changes nothing.
    pub fn restrict_to(&self, compiled: Vec<bool>) {
        let _ = self.compiled.set(compiled);
    }

    /// Records the procedures a reach-only compile leaves out. Set once.
    pub fn set_unreached(&self, unreached: HashSet<(usize, usize)>) {
        let _ = self.unreached.set(unreached);
    }

    /// Whether the routine at `offset` of object `oi` was left out as unreached.
    pub fn routine_unreached(&self, oi: usize, offset: usize) -> bool {
        self.unreached.get().is_some_and(|u| u.contains(&(oi, offset)))
    }

    /// Whether object `i` takes part in the compile: not a dependency's, or not left out by `restrict_to`.
    fn in_compile(&self, i: usize) -> bool {
        match self.compiled.get() {
            Some(c) => c[i] || self.kinds[self.objects[i].app] != AppKind::Dependency,
            None => true,
        }
    }

    /// Whether object `i` can be used: its file is a workspace file, or parses cleanly.
    pub fn usable(&self, i: usize) -> bool {
        let o = &self.objects[i];
        if !self.files[o.file].lazy {
            return true;
        }
        *self.usable_cache[i].get_or_init(|| self.file_problem(o.file).is_none() && self.object_node(o).is_some())
    }

    /// Whether object `i` is a table of the platform app `System`.
    pub fn platform_table(&self, i: usize) -> bool {
        let o = &self.objects[i];
        o.kind == ObjKind::Table && self.apps[o.app].id.eq_ignore_ascii_case(PLATFORM_APP_ID)
    }

    /// Whether object `i` is a table of the platform app `System` that nothing provides rows for
    /// (ADR-013 decision 7). Declared and empty would be silently wrong, so its persistent rows are
    /// refused when they are read or written (`platform::Virtual::Unprovided`); a temporary record
    /// of it is an ordinary temporary table.
    pub fn platform_table_without_provider(&self, i: usize) -> bool {
        self.platform_table(i)
            && self.objects[i]
                .id
                .is_none_or(|id| al_builtins::platform::provider(id).is_none())
    }

    /// Why name `name` of `kind` does not resolve. `name` is as written in the source.
    pub fn outside_workspace(&self, kind: ObjKind, name: &str) -> String {
        let bare = name.trim().trim_matches('"');
        format!("{} `{}` is outside the workspace", kind.label(), bare)
    }

    /// Why object `i` is unusable, for messages.
    pub fn unusable_reason(&self, i: usize) -> Option<String> {
        let fi = self.objects[i].file;
        match self.file_problem(fi) {
            Some(p) => Some(format!("dependency source does not parse: {p}")),
            None if self.files[fi].lazy && self.object_node(&self.objects[i]).is_none() => Some(format!(
                "{}: the object was not found after parsing",
                self.files[fi].path
            )),
            None => None,
        }
    }

    /// The files that were parsed after load and did not parse cleanly, as (path, message).
    pub fn lazy_parse_problems(&self) -> Vec<(String, String)> {
        self.lazy_parse_problems_from(0)
    }

    /// [`lazy_parse_problems`](Self::lazy_parse_problems) for the files of the apps from index `first_app` on.
    pub fn lazy_parse_problems_from(&self, first_app: usize) -> Vec<(String, String)> {
        self.files
            .iter()
            .filter(|f| f.app >= first_app)
            .filter_map(|f| Some((f.path.clone(), f.problem.get()?.clone()?)))
            .collect()
    }

    /// The `DotNet` alias `alias` as seen from `file`: the file's own app first, then the apps
    /// it depends on. Aliases are case-insensitive.
    pub fn resolve_dotnet(&self, file: usize, alias: &str) -> Option<usize> {
        let app = self.files[file].app;
        let find = |a: usize| {
            self.dotnet
                .iter()
                .position(|d| d.app == a && d.decl.alias.eq_ignore_ascii_case(alias))
        };
        find(app).or_else(|| self.visible[app].iter().find_map(|&a| find(a)))
    }

    fn add_object(&mut self, o: ObjectSym) {
        let oi = self.objects.len();
        let file = self.files[o.file].path.clone();
        // The AL compiler has already vetted a dependency app; its clashes are not ours to report.
        let report = self.kinds[o.app] != AppKind::Dependency;
        if let Some(id) = o.id {
            let ids = self.by_id.entry((o.kind, id)).or_default();
            if !ids.is_empty() && report {
                self.diagnostics.push(Diagnostic {
                    file: file.clone(),
                    app: o.app,
                    offset: o.offset,
                    code: "AR0001",
                    message: format!("duplicate {} id {id}", o.kind.label()),
                });
            }
            ids.push(oi);
        }
        // One object of a kind per name in a module, and per name in a namespace.
        let names = self.by_name.entry((o.kind, o.name.to_lowercase())).or_default();
        let clash = names.iter().any(|&j| {
            let other = &self.objects[j];
            other.app == o.app || eq_ns(&other.namespace, &o.namespace)
        });
        if clash && report {
            self.diagnostics.push(Diagnostic {
                file,
                app: o.app,
                offset: o.offset,
                code: "AR0001",
                message: format!("duplicate {} name `{}`", o.kind.label(), o.name),
            });
        }
        names.push(oi);
        self.objects.push(o);
    }

    /// How many apps `app` can see (itself and its transitive dependencies). A dependent sees
    /// more than anything it depends on, so this orders dependencies before their dependents.
    pub fn app_depth(&self, app: usize) -> usize {
        self.visible[app].len()
    }

    /// Resolves an object reference written in `file`. `parts` is the name
    /// split into namespace segments, or a single numeric id.
    ///
    /// Qualified: only the named namespace. Unqualified, closest scope first:
    /// the file's namespace, its `using`s, objects without a namespace, then a
    /// unique match in any namespace (corpus evidence: files with neither
    /// namespace nor `using` reference namespaced Base App tables and compile).
    pub fn resolve_ref(&self, file: usize, kind: ObjKind, parts: &[String]) -> Resolution {
        self.resolve_in_scope(file, kind, parts, false, true)
    }

    /// Whether any object of `kind` that `file` can see has the name (or id) `parts` says, whether or not
    /// the compile includes it. `false` means the name is nowhere (AL0118).
    ///
    /// When an app the file can see declares a dependency that is not loaded (`--no-deps`, or a package
    /// that was not found), the name may live there, so the answer is `true`: absence is only known when
    /// every declared dependency is present. The platform app System is an implicit dependency of every
    /// app (see `visible_from`), so the same holds while it is not loaded (corpus must-not-compile
    /// 0740-subscribe-internal-event-get-role-center-banner.al:5 names its codeunit "System Action Triggers").
    pub fn names_object(&self, file: usize, kind: ObjKind, parts: &[String]) -> bool {
        self.has_unloaded_dependency(self.files[file].app)
            || !self.apps.iter().any(|a| a.id.eq_ignore_ascii_case(PLATFORM_APP_ID))
            || !matches!(
                self.resolve_in_scope(file, kind, parts, false, false),
                Resolution::Missing
            )
    }

    /// Whether a table named `name` (bare) is among the objects `file` searches before the any-namespace fallback of
    /// [`resolve_ref`](Self::resolve_ref): those of its own namespace, of its `using`s, and those without a namespace.
    pub fn table_in_scope_tiers(&self, file: usize, name: &str) -> bool {
        let f = &self.files[file];
        self.by_name
            .get(&(ObjKind::Table, name.to_lowercase()))
            .into_iter()
            .flatten()
            .filter(|&&i| self.visible[f.app].contains(&self.objects[i].app))
            .any(|&i| {
                let ns = &self.objects[i].namespace;
                ns.is_none()
                    || eq_ns(ns, &f.namespace)
                    || f.usings.iter().any(|u| eq_ns(ns, &Some(u.clone())))
            })
    }

    /// Whether `app` or an app it sees declares a dependency that is not among the loaded apps.
    pub fn has_unloaded_dependency(&self, app: usize) -> bool {
        self.visible[app].iter().any(|&a| {
            self.apps[a]
                .dependencies
                .iter()
                .any(|d| !self.apps.iter().any(|x| x.id.eq_ignore_ascii_case(d)))
        })
    }

    /// The id of the platform table `parts` names when nothing provides rows for it. The id is
    /// a constant of its declaration, so `Database::"X"` can name it.
    pub fn unprovided_platform_table_id(&self, file: usize, parts: &[String]) -> Option<i32> {
        match self.resolve_in_scope(file, ObjKind::Table, parts, false, false) {
            Resolution::Found(i) if self.platform_table_without_provider(i) => self.objects[i].id.map(|id| id as i32),
            _ => None,
        }
    }

    /// What a name means at run time (a `TableRelation` target), where BC resolves it against
    /// every installed app: this workspace has no Base Application, so a hit is only certain
    /// when no tier above it could hold a Base Application object of that name. That is the
    /// file's own namespace; the `using`s when the file has no namespace of its own; the global
    /// namespace when it has neither. Anything else is `Missing`, never a guess (corpus fieldref/
    /// TestRelationTargetNamespaceScope.al and TestRelationOwnNamespaceDependency.al: an
    /// imported or global workspace table must not stand in for a Base Application one).
    pub fn resolve_ref_certain(&self, file: usize, kind: ObjKind, parts: &[String]) -> Resolution {
        self.resolve_in_scope(file, kind, parts, true, true)
    }

    /// Whether Microsoft's Base Application is among the apps `app` sees. Then no app is absent that
    /// could shadow a name, and the tiers resolve as the compiler resolves them.
    fn sees_base_application(&self, app: usize) -> bool {
        self.visible[app].iter().any(|&a| {
            self.kinds[a] == AppKind::Dependency && self.apps[a].name.eq_ignore_ascii_case("Base Application")
        })
    }

    /// A `TableRelation` target. The file of a dependency app resolves it among the apps that app
    /// saw (`visible`), as BC fixed it when the app compiled: a same-named table of a dependent app
    /// is not a candidate (corpus fieldref/TestRelationTargetNameCollision.al). The file of the
    /// workspace resolves it as `resolve_ref_certain` does, unless it sees Base Application.
    pub fn resolve_relation_target(&self, file: usize, kind: ObjKind, parts: &[String]) -> Resolution {
        if self.kinds[self.files[file].app] == AppKind::Dependency || self.sees_base_application(self.files[file].app) {
            self.resolve_ref(file, kind, parts)
        } else {
            self.resolve_ref_certain(file, kind, parts)
        }
    }

    /// `check_usable`: leave out objects of a dependency file that does not parse. That parses
    /// the file of every candidate, which the extension index must not trigger.
    fn resolve_in_scope(
        &self,
        file: usize,
        kind: ObjKind,
        parts: &[String],
        certain_only: bool,
        check_usable: bool,
    ) -> Resolution {
        let f = &self.files[file];
        let visible = |i: &usize| {
            self.visible[f.app].contains(&self.objects[*i].app)
                && (!check_usable || (self.in_compile(*i) && self.usable(*i)))
        };
        let Some(name) = parts.last() else {
            return Resolution::Missing;
        };
        if parts.len() == 1 {
            if let Ok(id) = name.parse::<u32>() {
                return self.pick_real(
                    self.by_id
                        .get(&(kind, id))
                        .into_iter()
                        .flatten()
                        .copied()
                        .filter(visible)
                        .collect(),
                );
            }
        }
        let cands: Vec<usize> = self
            .by_name
            .get(&(kind, name.to_lowercase()))
            .into_iter()
            .flatten()
            .copied()
            .filter(visible)
            .collect();
        if parts.len() > 1 {
            let ns = Some(parts[..parts.len() - 1].join("."));
            return self.pick_real(
                cands
                    .into_iter()
                    .filter(|&i| eq_ns(&self.objects[i].namespace, &ns))
                    .collect(),
            );
        }
        let in_ns = |ns: &Option<String>| {
            cands
                .iter()
                .copied()
                .filter(|&i| eq_ns(&self.objects[i].namespace, ns))
                .collect::<Vec<_>>()
        };
        let no_ns = f.namespace.is_none();
        let tiers = [
            f.namespace
                .as_ref()
                .map(|n| in_ns(&Some(n.clone())))
                .unwrap_or_default(),
            if certain_only && !no_ns {
                Vec::new()
            } else {
                f.usings.iter().flat_map(|u| in_ns(&Some(u.clone()))).collect()
            },
            if certain_only && !(no_ns && f.usings.is_empty()) {
                Vec::new()
            } else {
                in_ns(&None)
            },
            if certain_only { Vec::new() } else { cands.clone() },
        ];
        for tier in tiers {
            if !tier.is_empty() {
                return self.pick_real(tier);
            }
        }
        Resolution::Missing
    }

    /// `pick`, but a stub marked `ObsoleteState = Moved` gives way to a candidate that is not one:
    /// the object moved to another app and the stub only stays behind for upgrade. Only a
    /// choice between stub and real object changes; two real candidates stay ambiguous.
    /// Assumption, not pinned by the corpus (docs/probes-needed.md, "Probe batch 4"); evidence:
    /// Microsoft's own libraries name "No. Series" with both Base Application's stub and
    /// Business Foundation's table visible, and compile.
    fn pick_real(&self, tier: Vec<usize>) -> Resolution {
        let moved = |i: &usize| {
            self.objects[*i]
                .scanned_properties
                .iter()
                .any(|(n, v)| n.eq_ignore_ascii_case("ObsoleteState") && v.trim().eq_ignore_ascii_case("Moved"))
        };
        if tier.len() > 1 && tier.iter().any(|i| !moved(i)) {
            return pick(tier.into_iter().filter(|i| !moved(i)).collect());
        }
        pick(tier)
    }

    /// Looks an object up by kind and name (case-insensitive) or by numeric id text.
    pub fn find(&self, kind: ObjKind, name_or_id: &str) -> Option<(String, Option<u32>)> {
        self.lookup(None, kind, name_or_id)
            .map(|i| (self.objects[i].name.clone(), self.objects[i].id))
    }

    /// Resolves a reference as seen from `from_app` (None = from anywhere).
    pub fn lookup(&self, from_app: Option<usize>, kind: ObjKind, name_or_id: &str) -> Option<usize> {
        let visible = |i: &&usize| {
            from_app.is_none_or(|a| self.visible[a].contains(&self.objects[**i].app))
                && self.in_compile(**i)
                && self.usable(**i)
        };
        let hits = match name_or_id.parse::<u32>() {
            Ok(id) => self.by_id.get(&(kind, id)),
            Err(_) => self.by_name.get(&(kind, name_or_id.to_lowercase())),
        };
        // A stub marked Moved gives way to the object it moved to (`pick_real`).
        let mut seen = hits?.iter().filter(visible).copied();
        let first = seen.next()?;
        match self.pick_real(std::iter::once(first).chain(seen).collect()) {
            Resolution::Found(i) => Some(i),
            _ => Some(first),
        }
    }

    /// Whether object `i` belongs to a dependency app.
    pub fn is_dependency(&self, i: usize) -> bool {
        self.kinds[self.objects[i].app] == AppKind::Dependency
    }

    /// The syntax node of object `i`.
    pub fn object_ast(&self, i: usize) -> ast::Object {
        self.object_node(&self.objects[i])
            .expect("object node at recorded offset")
    }

    /// Objects are direct children of the file's root, so look there rather than
    /// walking every node of the file.
    fn object_node(&self, o: &ObjectSym) -> Option<ast::Object> {
        let root = al_syntax::SyntaxNode::new_root(self.green(o.file));
        root.children().filter_map(ast::Object::cast).nth(o.ordinal)
    }

    /// Checks every object-name reference of the workspace's own objects. The objects are
    /// independent (each reads the object index, which is complete before this runs), so they
    /// run on all cores. `collect` keeps the objects in index order and the diagnostics of one
    /// object in the order the serial loop produced them, so the list does not depend on the
    /// number of threads (the final sort by file and offset is stable).
    fn resolve_references(&mut self) {
        let per_object: Vec<Vec<Diagnostic>> = self
            .objects
            .par_iter()
            .map(|o| self.unresolved_references_of(o))
            .collect();
        self.diagnostics.extend(per_object.into_iter().flatten());
    }

    fn unresolved_references_of(&self, o: &ObjectSym) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        let file = &self.files[o.file];
        // A dependency app compiled; its references are resolved when its code is used.
        if file.lazy {
            return diags;
        }
        let Some(obj) = self.object_node(o) else {
            return diags;
        };
        let node = obj.syntax().clone();
        let mut check = |kind: ObjKind, parts: &[String], offset: usize| {
            let name = parts.join(".");
            let (code, message) = match self.resolve_ref(o.file, kind, parts) {
                Resolution::Found(_) => return,
                Resolution::Missing => ("AR0002", format!("unresolved {} `{name}`", kind.label())),
                Resolution::Ambiguous(c) => (
                    "AR0003",
                    format!("ambiguous {} `{name}` ({} candidates)", kind.label(), c.len()),
                ),
            };
            diags.push(Diagnostic {
                file: file.path.clone(),
                app: file.app,
                offset,
                code,
                message,
            });
        };
        let header_off = o.offset;
        if let (Some(target), Some(base)) = (o.kind.extended(), obj.extends_parts()) {
            check(target, &base, header_off);
        }
        for i in obj.implements_parts() {
            check(ObjKind::Interface, &i, header_off);
        }
        for t in node.descendants().filter_map(ast::TypeRef::cast) {
            if let Some(kind) = ObjKind::from_type_name(&t.type_name()) {
                let parts = t.object_name_parts();
                if !parts.is_empty() {
                    check(kind, &parts, t.syntax().text_range().start().into());
                }
            }
        }
        diags
    }
}

/// What loading needs from one file; plain data, so it can be produced
/// on a worker thread (syntax nodes cannot cross threads, green trees can).
struct ParsedFile {
    /// The tree; none for a file that was only scanned.
    green: Option<rowan::GreenNode>,
    errors: Vec<al_syntax::ParseError>,
    namespace: Option<String>,
    usings: Vec<String>,
    /// The objects of the file, in source order.
    objects: Vec<FileObject>,
    /// The `type(..)` declarations of the file's `dotnet { }` blocks.
    dotnet: Vec<ast::DotNetTypeDecl>,
}

/// One object of a scanned or parsed file.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FileObject {
    pub kind: ObjKind,
    pub id: Option<u32>,
    pub name: String,
    pub offset: usize,
    pub extends: Option<Vec<String>>,
    pub implements: Vec<Vec<String>>,
    pub subscribers: Vec<al_syntax::scan::ScannedSubscriber>,
    pub properties: Vec<(String, String)>,
    pub layouts: Vec<al_syntax::scan::ScannedLayout>,
    pub install: bool,
    pub manual_subscribers: bool,
}

/// Runs `f` with this thread's token cache (one per thread: a cache is 128 KB to set up, and
/// there are tens of thousands of files). A parse that starts inside another gets a cache of its own.
pub(crate) fn with_tree_cache<T>(f: impl FnOnce(&mut al_syntax::TreeCache) -> T) -> T {
    thread_local! {
        static CACHE: std::cell::RefCell<al_syntax::TreeCache> = Default::default();
    }
    CACHE.with(|c| match c.try_borrow_mut() {
        Ok(mut cache) => f(&mut cache),
        Err(_) => f(&mut al_syntax::TreeCache::default()),
    })
}

/// One parse of a file: its tree and the first error, if any.
struct Attempt<G> {
    green: G,
    problem: Option<String>,
}

/// A file that fails to parse is parsed once more; a parse is a function of the text, so a second result
/// that differs is a defect (a race, a torn read of the archive) and is reported with both results. The
/// clean parse is kept when there is one. A failure that repeats is the file's own.
fn settle<G>(first: Attempt<G>, again: impl FnOnce() -> Attempt<G>) -> (Attempt<G>, Option<String>) {
    let Some(first_problem) = first.problem.clone() else {
        return (first, None);
    };
    let second = again();
    if second.problem.as_ref() == Some(&first_problem) {
        return (first, None);
    }
    let note = format!(
        "parsed again with a different result: first `{first_problem}`, second {}",
        match &second.problem {
            Some(p) => format!("`{p}`"),
            None => "clean".to_string(),
        }
    );
    let kept = if second.problem.is_none() { second } else { first };
    (kept, Some(note))
}

fn parse_file(src: &str, symbols: &BTreeSet<String>, cache: &mut al_syntax::TreeCache) -> ParsedFile {
    parsed_file(al_syntax::parse_with_cache(src, symbols, cache))
}

/// The index entry of a parsed file.
fn parsed_file(parse: al_syntax::Parse) -> ParsedFile {
    let root = ast::SourceFile::cast(parse.syntax()).expect("root is a SourceFile");
    ParsedFile {
        namespace: root.namespace(),
        usings: root.usings(),
        objects: root
            .objects()
            .map(|o| FileObject {
                kind: ObjKind::from_keyword(&o.kind()),
                id: o.id(),
                name: o.name(),
                offset: o.syntax().text_range().start().into(),
                extends: o.extends_parts(),
                implements: o.implements_parts(),
                subscribers: Vec::new(),
                properties: Vec::new(),
                layouts: Vec::new(),
                install: false,
                manual_subscribers: false,
            })
            .collect(),
        dotnet: root.dotnet_types(),
        green: Some(parse.green),
        errors: parse.errors,
    }
}

/// The index entry of a file that is not parsed at load.
fn scanned_file(src: &str, symbols: &BTreeSet<String>) -> ParsedFile {
    let scan = al_syntax::scan::scan(src, symbols);
    // A `dotnet { }` block declares type aliases other apps use; it holds no object, so reading
    // such a file at load costs nothing a lazy parse would have saved.
    let dotnet = if scan.dotnet_block {
        let parse = al_syntax::parse(src, symbols);
        ast::SourceFile::cast(parse.syntax())
            .map(|r| r.dotnet_types())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    ParsedFile {
        dotnet,
        green: None,
        errors: Vec::new(),
        namespace: scan.namespace,
        usings: scan.usings,
        objects: scan
            .objects
            .into_iter()
            .map(|o| FileObject {
                kind: ObjKind::from_keyword(&o.kind),
                id: o.id,
                name: o.name,
                offset: o.offset,
                extends: o.extends,
                implements: o.implements,
                subscribers: o.subscribers,
                properties: o.properties,
                layouts: o.layouts,
                install: o.install,
                manual_subscribers: o.manual_subscribers,
            })
            .collect(),
    }
}

fn eq_ns(a: &Option<String>, b: &Option<String>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    }
}

fn pick(c: Vec<usize>) -> Resolution {
    match c.len() {
        0 => Resolution::Missing,
        1 => Resolution::Found(c[0]),
        _ => Resolution::Ambiguous(c),
    }
}

/// Id of the platform app `System`, which declares the system tables. BC always provides it, so it
/// is an implicit dependency of every app (ADR-013 decision 4).
pub const PLATFORM_APP_ID: &str = "8874ed3a-0643-4247-9ced-7a7002f7135d";

fn visible_from(apps: &[AppSource], start: usize) -> BTreeSet<usize> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![start];
    if let Some(sys) = apps.iter().position(|a| a.id.eq_ignore_ascii_case(PLATFORM_APP_ID)) {
        stack.push(sys);
    }
    while let Some(i) = stack.pop() {
        if seen.insert(i) {
            for d in &apps[i].dependencies {
                if let Some(j) = apps.iter().position(|a| a.id.eq_ignore_ascii_case(d)) {
                    stack.push(j);
                }
            }
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A parse is a function of the text. When a dependency file fails to parse, it is parsed once more
    /// (a fresh token cache, a fresh expansion); a second result that differs is a defect somewhere (a
    /// race, a torn read of the archive) and is reported with both texts, and the clean parse is kept.
    #[test]
    fn a_second_parse_that_differs_is_reported_and_the_clean_one_is_kept() {
        let bad = Attempt {
            green: 1,
            problem: Some("f.al:7: expected `;` (`oAdd`)".into()),
        };
        let good = Attempt {
            green: 2,
            problem: None,
        };
        let (kept, flake) = settle(bad, || good);
        assert_eq!(kept.green, 2);
        assert!(kept.problem.is_none());
        let flake = flake.expect("the two parses differ");
        assert!(
            flake.contains("oAdd") && flake.contains("parsed again") && flake.contains("clean"),
            "{flake}"
        );
    }

    #[test]
    fn a_failure_that_repeats_is_the_files_own_and_is_not_a_flake() {
        let bad = || Attempt {
            green: 1,
            problem: Some("f.al:7: expected `;` (`x`)".into()),
        };
        let (kept, flake) = settle(bad(), bad);
        assert_eq!(kept.problem.as_deref(), Some("f.al:7: expected `;` (`x`)"));
        assert!(flake.is_none());
        // A clean first parse is never parsed again.
        let (kept, flake) = settle(
            Attempt {
                green: 3,
                problem: None,
            },
            || panic!("no second parse"),
        );
        assert_eq!(kept.green, 3);
        assert!(flake.is_none());
    }

    fn app(id: &str, deps: &[&str], files: &[(&str, &str)]) -> AppSource {
        AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
        }
    }

    fn load(apps: Vec<AppSource>) -> Workspace {
        Workspace::load(apps, &al_version::corpus_symbols(27))
    }

    fn codes(ws: &Workspace) -> Vec<(&'static str, String)> {
        ws.diagnostics.iter().map(|d| (d.code, d.message.clone())).collect()
    }

    #[test]
    fn a_table_moved_to_another_app_does_not_make_its_name_ambiguous() {
        // Base Application 27.5 keeps `table 230 "Source Code"` with `ObsoleteState = Moved` while
        // Business Foundation declares the live one; `Record "Source Code"` in Company-Initialize
        // names the live table.
        let live = app(
            "foundation",
            &[],
            &[(
                "sc.al",
                "namespace Microsoft.Foundation.AuditCodes; table 230 \"Source Code\" { fields { field(1; Code; Code[10]) { } } }",
            )],
        );
        let old = app(
            "base",
            &["foundation"],
            &[
                (
                    "sc.al",
                    "namespace Microsoft.Foundation.AuditCodes; table 230 \"Source Code\" { ObsoleteState = Moved; MovedTo = 'f3552374'; fields { field(1; Code; Code[10]) { } } }",
                ),
                (
                    "ci.al",
                    "namespace Microsoft.Foundation.Company; using Microsoft.Foundation.AuditCodes; codeunit 2 CI { var S: Record \"Source Code\"; }",
                ),
            ],
        );
        let ws = Workspace::load_apps(
            vec![live, old],
            vec![AppKind::Dependency, AppKind::Dependency],
            &al_version::corpus_symbols(27),
        );
        let file = ws.objects.iter().find(|o| o.name == "CI").unwrap().file;
        match ws.resolve_ref(file, ObjKind::Table, &["Source Code".to_string()]) {
            Resolution::Found(i) => assert_eq!(ws.apps[ws.objects[i].app].id, "foundation"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn run_time_resolution_only_trusts_tiers_no_absent_app_could_shadow() {
        // corpus fieldref/TestRelationOwnNamespaceDependency.al, TestRelationTargetNamespaceScope.al:
        // a workspace table must not stand in for a Base Application one that is not here.
        let ws = load(vec![app(
            "a",
            &[],
            &[
                ("t.al", "namespace Imported; table 50900 \"Agent\" { }"),
                ("g.al", "table 50901 \"Global Agent\" { }"),
                ("own.al", "namespace Own; table 50902 \"Own Agent\" { }"),
                ("c1.al", "namespace Own; using Imported; codeunit 60100 C1 { }"),
                ("c2.al", "using Imported; codeunit 60101 C2 { }"),
                ("c3.al", "codeunit 60102 C3 { }"),
                ("c4.al", "namespace Other; codeunit 60103 C4 { }"),
            ],
        )]);
        let file = |name: &str| ws.objects.iter().find(|o| o.name == name).unwrap().file;
        let found = |r: Resolution| matches!(r, Resolution::Found(_));
        let one = |n: &str| vec![n.to_string()];
        // The ordinary compile-time resolution finds each of them.
        assert!(found(ws.resolve_ref(file("C1"), ObjKind::Table, &one("Agent"))));
        // Own namespace: certain.
        assert!(found(ws.resolve_ref_certain(
            file("C1"),
            ObjKind::Table,
            &one("Own Agent")
        )));
        // A `using` hit under a namespaced file could be shadowed by a Base Application table of the
        // file's own namespace: not certain. Without a namespace of its own it is.
        assert!(!found(ws.resolve_ref_certain(
            file("C1"),
            ObjKind::Table,
            &one("Agent")
        )));
        assert!(found(ws.resolve_ref_certain(file("C2"), ObjKind::Table, &one("Agent"))));
        // A global table is only certain for a file with neither namespace nor usings.
        assert!(!found(ws.resolve_ref_certain(
            file("C2"),
            ObjKind::Table,
            &one("Global Agent")
        )));
        assert!(found(ws.resolve_ref_certain(
            file("C3"),
            ObjKind::Table,
            &one("Global Agent")
        )));
        assert!(!found(ws.resolve_ref_certain(
            file("C4"),
            ObjKind::Table,
            &one("Global Agent")
        )));
        // Qualified names are exact.
        assert!(found(ws.resolve_ref_certain(
            file("C4"),
            ObjKind::Table,
            &["Imported".to_string(), "Agent".to_string()]
        )));
    }

    #[test]
    fn with_base_application_loaded_a_using_resolves_a_relation_target() {
        // corpus fieldref/TestRelationTargetNamespaceScope.al RelationScope_Usings_*: a table in a
        // file with a namespace, naming "Shipping Agent" with `using Microsoft.Foundation.Shipping`,
        // relates to Base Application's table 291. Base Application is here, so the tiers above the
        // using can be told to hold nothing (the same holds for the name written in Base
        // Application's own files, which resolve among the apps they saw).
        let base = app(
            "Base Application",
            &[],
            &[(
                "sa.al",
                "namespace Microsoft.Foundation.Shipping; table 291 \"Shipping Agent\" { }",
            )],
        );
        let mine = app(
            "mine",
            &["Base Application"],
            &[(
                "c.al",
                "namespace Mine.Scope; using Microsoft.Foundation.Shipping; codeunit 60100 C1 { }",
            )],
        );
        let ws = Workspace::load_apps(
            vec![base, mine],
            vec![AppKind::Dependency, AppKind::Workspace],
            &al_version::corpus_symbols(27),
        );
        let file = ws.objects.iter().find(|o| o.name == "C1").unwrap().file;
        let name = vec!["Shipping Agent".to_string()];
        assert!(matches!(
            ws.resolve_relation_target(file, ObjKind::Table, &name),
            Resolution::Found(_)
        ));
        // Without Base Application a hit under a using stays uncertain (the test above).
        let alone = Workspace::load(
            vec![app(
                "mine",
                &[],
                &[
                    (
                        "t.al",
                        "namespace Microsoft.Foundation.Shipping; table 291 \"Shipping Agent\" { }",
                    ),
                    (
                        "c.al",
                        "namespace Mine.Scope; using Microsoft.Foundation.Shipping; codeunit 60100 C1 { }",
                    ),
                ],
            )],
            &al_version::corpus_symbols(27),
        );
        let file = alone.objects.iter().find(|o| o.name == "C1").unwrap().file;
        assert!(!matches!(
            alone.resolve_relation_target(file, ObjKind::Table, &name),
            Resolution::Found(_)
        ));
    }

    /// Issue 15: one eager run of Microsoft's apps reported a parse error in a report that parsed fine in every
    /// other run (a stray token next to a synthetic column routine). A parse is a function of the text, so
    /// many threads parsing many files with synthetic routines must agree with one thread, every time. Not
    /// a reproduction (the failure did not repeat in 36 runs); the guard against it.
    #[test]
    fn parsing_report_and_page_files_in_parallel_gives_the_same_trees_as_parsing_them_alone() {
        let report = |i: usize| {
            format!(
                "report {} R{i} {{ ProcessingOnly = true; dataset {{ dataitem(A; Integer) {{ column(Num; Number) {{ }} column(Twice; Number * 2) {{ }} dataitem(B; Integer) {{ column(Inner; Number + {i}) {{ }} }} }} }} \
                 requestpage {{ layout {{ area(Content) {{ field(F; Opt) {{ ApplicationArea = All; Visible = Opt > {i}; }} }} }} }} var Opt: Integer; }}",
                60000 + i
            )
        };
        let page = |i: usize| {
            format!(
                "page {} P{i} {{ PageType = Card; layout {{ area(Content) {{ field(F; Opt) {{ ApplicationArea = All; Editable = Opt > {i}; Caption = 'x'; }} }} }} var Opt: Integer; }}",
                61000 + i
            )
        };
        let files: Vec<(String, String)> = (0..120)
            .map(|i| (format!("f{i:03}.al"), if i % 2 == 0 { report(i) } else { page(i) }))
            .collect();
        let dep = AppSource {
            id: "d".into(),
            name: "d".into(),
            dependencies: vec![],
            files,
        };
        let parse_all = |parallel: bool| {
            let ws = Workspace::load_apps(
                vec![dep.clone()],
                vec![AppKind::Dependency],
                &al_version::corpus_symbols(27),
            );
            let ids: Vec<usize> = (0..ws.files.len()).collect();
            if parallel {
                ids.par_iter().for_each(|&fi| {
                    ws.green(fi);
                });
            }
            let trees: Vec<String> = ids.iter().map(|&fi| format!("{:?}", ws.green(fi))).collect();
            let synthetic: Vec<usize> = ids.iter().map(|&fi| ws.synthetic(fi).len()).collect();
            assert!(ws.lazy_parse_problems().is_empty(), "{:?}", ws.lazy_parse_problems());
            assert!(ws.parse_flakes().is_empty(), "{:?}", ws.parse_flakes());
            (trees, synthetic)
        };
        let alone = parse_all(false);
        assert!(alone.1.iter().all(|&n| n > 0), "every file has synthetic routines");
        for round in 0..40 {
            assert!(
                parse_all(true) == alone,
                "round {round}: a parallel parse differs from a serial one"
            );
        }
    }

    #[test]
    fn files_objects_and_diagnostics_keep_input_order_with_parallel_parsing() {
        // Many files so that rayon really splits the work; a parse error and a
        // duplicate id are planted in the middle and must be reported in file order.
        let mut files: Vec<(String, String)> = (0..200)
            .map(|i| (format!("f{i:03}.al"), format!("codeunit {} C{i} {{ }}", 50000 + i)))
            .collect();
        files[100].1 = "codeunit 50100 C100 { procedure ( }".into();
        files[150].1 = "codeunit 50007 Dup { }".into();
        let a = AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files,
        };
        let b = app("b", &["a"], &[("z.al", "codeunit 60000 Z { }")]);
        let ws = load(vec![a.clone(), b.clone()]);
        let paths: Vec<_> = ws.files.iter().map(|f| f.path.as_str()).collect();
        let want: Vec<String> = a.files.iter().chain(&b.files).map(|f| f.0.clone()).collect();
        assert_eq!(paths, want.iter().map(String::as_str).collect::<Vec<_>>());
        let names: Vec<_> = ws.objects.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names[..3], ["C0", "C1", "C2"]);
        assert_eq!(names.last(), Some(&"Z"));
        let diag_files: Vec<_> = ws.diagnostics.iter().map(|d| d.file.as_str()).collect();
        let mut sorted = diag_files.clone();
        sorted.sort();
        assert_eq!(diag_files, sorted, "diagnostics come in file order");
        assert!(diag_files.contains(&"f100.al") && diag_files.contains(&"f150.al"));
        // Loading is deterministic.
        let again = load(vec![a, b]);
        assert_eq!(codes(&ws), codes(&again));
        assert_eq!(
            ws.objects.iter().map(|o| (o.file, o.offset)).collect::<Vec<_>>(),
            again.objects.iter().map(|o| (o.file, o.offset)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn reference_diagnostics_keep_object_then_source_order_over_many_files() {
        // `resolve_references` runs per object on all cores (owner decision 2026-10-15, ADR-026
        // option 4). The list of diagnostics must be the list a serial loop gives: files in input
        // order, within a file by offset, and several diagnostics of one object in the order of
        // `extends`, `implements`, then type references in source order.
        let files: Vec<(String, String)> = (0..300)
            .map(|i| {
                (
                    format!("f{i:03}.al"),
                    format!(
                        "codeunit {} C{i} implements MissingI{i} {{ var R: Record MissingT{i}; L: List of [Codeunit MissingC{i}]; \
                         procedure P(x: Page MissingP{i}) begin end; }}\n\
                         tableextension {} E{i} extends MissingBase{i} {{ }}",
                        50000 + i,
                        70000 + i
                    ),
                )
            })
            .collect();
        let mut want = Vec::new();
        for i in 0..300 {
            for (kind, name) in [
                ("interface", format!("MissingI{i}")),
                ("table", format!("MissingT{i}")),
                ("codeunit", format!("MissingC{i}")),
                ("page", format!("MissingP{i}")),
                ("table", format!("MissingBase{i}")),
            ] {
                want.push(("AR0002", format!("unresolved {kind} `{name}`")));
            }
        }
        let a = AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files,
        };
        for round in 0..10 {
            let ws = load(vec![a.clone()]);
            assert_eq!(codes(&ws), want, "round {round}");
            let offsets: Vec<_> = ws.diagnostics.iter().map(|d| (d.file.clone(), d.offset)).collect();
            let mut sorted = offsets.clone();
            sorted.sort();
            assert_eq!(offsets, sorted);
        }
    }

    #[test]
    fn finds_objects_by_name_case_insensitively_and_by_id() {
        let ws = load(vec![app("a", &[], &[("t.al", "table 50900 \"ALT Universal\" { }")])]);
        assert_eq!(
            ws.find(ObjKind::Table, "alt universal"),
            Some(("ALT Universal".into(), Some(50900)))
        );
        assert_eq!(
            ws.find(ObjKind::Table, "50900"),
            Some(("ALT Universal".into(), Some(50900)))
        );
        assert_eq!(ws.find(ObjKind::Codeunit, "ALT Universal"), None);
    }

    #[test]
    fn resolved_type_references_produce_no_diagnostics() {
        let ws = load(vec![app(
            "a",
            &[],
            &[
                ("t.al", "table 50900 \"ALT Universal\" { }"),
                (
                    "c.al",
                    "codeunit 60100 T { var R: Record \"ALT Universal\"; L: List of [Integer]; procedure P(x: Codeunit T) begin end; }",
                ),
            ],
        )]);
        assert_eq!(codes(&ws), vec![]);
    }

    #[test]
    fn unresolved_object_reference_is_diagnosed() {
        let ws = load(vec![app(
            "a",
            &[],
            &[("c.al", "codeunit 1 C { var R: Record Customer; }")],
        )]);
        assert_eq!(codes(&ws), vec![("AR0002", "unresolved table `Customer`".to_string())]);
    }

    #[test]
    fn references_inside_routines_and_nested_type_args_are_checked() {
        let ws = load(vec![app(
            "a",
            &[],
            &[(
                "c.al",
                "codeunit 1 C { procedure P(): Record X var L: List of [Codeunit Y]; begin end; }",
            )],
        )]);
        assert_eq!(
            codes(&ws),
            vec![
                ("AR0002", "unresolved table `X`".to_string()),
                ("AR0002", "unresolved codeunit `Y`".to_string())
            ]
        );
    }

    #[test]
    fn objects_of_a_dependency_are_visible_but_not_the_reverse() {
        let ws = load(vec![
            app("base", &[], &[("t.al", "table 1 T { var C: Codeunit Top; }")]),
            app("top", &["base"], &[("c.al", "codeunit 2 Top { var R: Record T; }")]),
        ]);
        assert_eq!(codes(&ws), vec![("AR0002", "unresolved codeunit `Top`".to_string())]);
    }

    #[test]
    fn duplicate_object_ids_and_names_are_diagnosed() {
        let ws = load(vec![app(
            "a",
            &[],
            &[("x.al", "codeunit 1 A { }\ncodeunit 1 B { }\ncodeunit 2 a { }")],
        )]);
        assert_eq!(
            codes(&ws),
            vec![
                ("AR0001", "duplicate codeunit id 1".to_string()),
                ("AR0001", "duplicate codeunit name `a`".to_string())
            ]
        );
    }

    #[test]
    fn extends_and_implements_are_resolved() {
        let ws = load(vec![app(
            "a",
            &[],
            &[(
                "x.al",
                "interface I { }\nenum 1 E implements I, J { }\ntableextension 2 X extends Missing { }",
            )],
        )]);
        assert_eq!(
            codes(&ws),
            vec![
                ("AR0002", "unresolved interface `J`".to_string()),
                ("AR0002", "unresolved table `Missing`".to_string())
            ]
        );
    }

    #[test]
    fn namespace_qualified_reference_resolves_in_that_namespace() {
        let ws = load(vec![app(
            "a",
            &[],
            &[
                ("b.al", "namespace System.Utilities;\ncodeunit 1 \"Temp Blob\" { }"),
                (
                    "u.al",
                    "codeunit 2 U { var B: Codeunit System.Utilities.\"Temp Blob\"; }",
                ),
            ],
        )]);
        assert_eq!(codes(&ws), vec![]);
    }

    #[test]
    fn qualified_reference_does_not_fall_back_to_an_object_without_namespace() {
        let ws = load(vec![app(
            "a",
            &[],
            &[(
                "x.al",
                "codeunit 1 \"Temp Blob\" { }\ncodeunit 2 U { var B: Codeunit System.Utilities.\"Temp Blob\"; }",
            )],
        )]);
        assert_eq!(
            codes(&ws),
            vec![("AR0002", "unresolved codeunit `System.Utilities.Temp Blob`".to_string())]
        );
    }

    #[test]
    fn namespace_qualified_reference_to_a_name_containing_a_dot() {
        let ws = load(vec![app(
            "a",
            &[],
            &[
                ("t.al", "namespace A.B;\ntable 1 \"Config. Package Table\" { }"),
                ("u.al", "codeunit 2 U { var R: Record A.B.\"Config. Package Table\"; }"),
            ],
        )]);
        assert_eq!(codes(&ws), vec![]);
    }

    fn lazy_ws() -> Workspace {
        let dep = app(
            "dep",
            &[],
            &[
                (
                    "Dep/cust.al",
                    "namespace Dep.Sales;\ntable 18 Customer { fields { field(1; \"No.\"; Code[20]) { } } }",
                ),
                ("Dep/item.al", "namespace Dep.Inv;\ntable 27 Item { }"),
                (
                    "Dep/ext.al",
                    "namespace Dep.Sales;\ntableextension 50 \"Cust Ext\" extends Customer { }",
                ),
                (
                    "Dep/sub.al",
                    "codeunit 60 Subs { [EventSubscriber(ObjectType::Table, Database::Customer, 'OnAfterInsertEvent', '', false, false)] local procedure A() begin end; }",
                ),
                ("Dep/impl.al", "codeunit 61 Impl implements \"Dep I\" { }"),
                (
                    "Dep/broken.al",
                    "table 70 Broken { fields { field(1; F; Integer) { } } procedure P( begin end; }",
                ),
            ],
        );
        let main = app(
            "main",
            &["dep"],
            &[("m.al", "codeunit 1 M { var C: Record Customer; }")],
        );
        Workspace::load_apps(
            vec![main, dep],
            vec![AppKind::Workspace, AppKind::Dependency],
            &["BC27PLUS".to_string()].into(),
        )
    }

    #[test]
    fn dependency_files_are_scanned_at_load_and_parsed_on_first_use() {
        let ws = lazy_ws();
        let file = |p: &str| ws.files.iter().position(|f| f.path == p).unwrap();
        assert!(
            ws.files[file("m.al")].green.get().is_some(),
            "workspace files parse at load"
        );
        // The index knows every object, with its namespace.
        let customer = ws.objects.iter().position(|o| o.name == "Customer").unwrap();
        assert_eq!(ws.objects[customer].namespace.as_deref(), Some("Dep.Sales"));
        assert_eq!(ws.objects[customer].id, Some(18));
        // The workspace's own reference to Customer resolved without a diagnostic. That parsed
        // the file of Customer and no other.
        assert_eq!(ws.diagnostics, vec![]);
        assert!(ws.files[file("Dep/cust.al")].green.get().is_some());
        for p in [
            "Dep/item.al",
            "Dep/ext.al",
            "Dep/sub.al",
            "Dep/impl.al",
            "Dep/broken.al",
        ] {
            assert!(ws.files[file(p)].green.get().is_none(), "{p} is only scanned");
        }
    }

    #[test]
    fn extensions_and_subscribers_are_found_from_the_index() {
        let ws = lazy_ws();
        let ix = |n: &str| ws.objects.iter().position(|o| o.name == n).unwrap();
        assert_eq!(ws.extension_target(ix("Cust Ext")), Some(ix("Customer")));
        assert_eq!(ws.extensions_of(ix("Customer")), [ix("Cust Ext")]);
        assert_eq!(ws.extension_target(ix("Customer")), None);
        let subs = ws.subscribers_of(ix("Customer"));
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].0, ix("Subs"));
        assert_eq!(ws.objects[subs[0].0].subscribers[subs[0].1].event, "OnAfterInsertEvent");
        assert_eq!(ws.implementers_of("Dep I"), [ix("Impl")]);
        assert!(ws.subscribers_of(ix("Item")).is_empty());
    }

    #[test]
    fn a_dependency_object_whose_file_does_not_parse_is_unusable_and_says_why() {
        let ws = lazy_ws();
        let broken = ws.objects.iter().position(|o| o.name == "Broken").unwrap();
        assert!(!ws.usable(broken));
        let why = ws.unusable_reason(broken).unwrap();
        assert!(why.contains("Dep/broken.al") && why.contains("does not parse"), "{why}");
        // A reference to it resolves to nothing rather than to half a table.
        let fi = ws.files.iter().position(|f| f.path == "m.al").unwrap();
        assert_eq!(
            ws.resolve_ref(fi, ObjKind::Table, &["Broken".to_string()]),
            Resolution::Missing
        );
        assert!(ws.lazy_parse_problems().iter().any(|(p, _)| p == "Dep/broken.al"));
        // The healthy objects are unaffected.
        let customer = ws.objects.iter().position(|o| o.name == "Customer").unwrap();
        assert!(ws.usable(customer));
        assert_eq!(ws.unusable_reason(customer), None);
    }

    #[test]
    fn clashes_between_dependency_objects_are_not_reported() {
        let ws = Workspace::load_apps(
            vec![app(
                "d",
                &[],
                &[("a.al", "codeunit 1 A { }\ncodeunit 1 B { }\ncodeunit 2 a { }")],
            )],
            vec![AppKind::Dependency],
            &Default::default(),
        );
        assert_eq!(ws.diagnostics, vec![]);
    }

    fn resolved(ws: &Workspace, file: &str, kind: ObjKind, parts: &[&str]) -> Option<(Option<String>, u32)> {
        let fi = ws.files.iter().position(|f| f.path == file).unwrap();
        let parts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
        match ws.resolve_ref(fi, kind, &parts) {
            Resolution::Found(i) => Some((ws.objects[i].namespace.clone(), ws.objects[i].id.unwrap())),
            _ => None,
        }
    }

    fn ns_app() -> Workspace {
        load(vec![app(
            "a",
            &[],
            &[
                (
                    "local.al",
                    "namespace X.Local;\ntable 1 \"Ship\" { }\ntable 4 \"Only Local\" { }",
                ),
                ("base.al", "namespace X.Base;\ntable 2 \"Ship\" { }"),
                ("global.al", "table 3 \"Glob\" { }"),
                ("user_local.al", "namespace X.Local;\ncodeunit 10 A { }"),
                ("user_using.al", "namespace X.Other;\nusing X.Base;\ncodeunit 11 B { }"),
                ("user_none.al", "codeunit 12 C { }"),
            ],
        )])
    }

    #[test]
    fn qualified_reference_resolves_only_in_the_named_namespace() {
        let ws = ns_app();
        assert_eq!(
            resolved(&ws, "user_none.al", ObjKind::Table, &["X", "Base", "Ship"]),
            Some((Some("X.Base".into()), 2))
        );
        assert_eq!(
            resolved(&ws, "user_none.al", ObjKind::Table, &["X", "Local", "Ship"]),
            Some((Some("X.Local".into()), 1))
        );
        assert_eq!(
            resolved(&ws, "user_none.al", ObjKind::Table, &["X", "Base", "Only Local"]),
            None
        );
    }

    #[test]
    fn unqualified_reference_prefers_the_own_namespace_then_usings() {
        let ws = ns_app();
        assert_eq!(
            resolved(&ws, "user_local.al", ObjKind::Table, &["Ship"]),
            Some((Some("X.Local".into()), 1))
        );
        assert_eq!(
            resolved(&ws, "user_using.al", ObjKind::Table, &["Ship"]),
            Some((Some("X.Base".into()), 2))
        );
    }

    #[test]
    fn unqualified_reference_falls_back_to_a_unique_match_anywhere() {
        // Corpus evidence: files without namespace or using reference Base App
        // tables such as `Record Customer` (namespace Microsoft.Sales.Customer) and compile.
        let ws = ns_app();
        assert_eq!(
            resolved(&ws, "user_none.al", ObjKind::Table, &["Glob"]),
            Some((None, 3))
        );
        assert_eq!(
            resolved(&ws, "user_none.al", ObjKind::Table, &["Only Local"]),
            Some((Some("X.Local".into()), 4))
        );
    }

    #[test]
    fn unqualified_reference_matching_several_namespaces_is_ambiguous() {
        let ws = ns_app();
        let fi = ws.files.iter().position(|f| f.path == "user_none.al").unwrap();
        assert!(matches!(
            ws.resolve_ref(fi, ObjKind::Table, &["Ship".to_string()]),
            Resolution::Ambiguous(_)
        ));
    }

    #[test]
    fn qualified_extends_resolves_by_namespace() {
        let ws = load(vec![app(
            "a",
            &[],
            &[
                ("p.al", "namespace N.Local;\npage 1 \"Svc\" { }"),
                (
                    "e.al",
                    "pageextension 2 E1 extends N.Local.\"Svc\" { }\npageextension 3 E2 extends N.Other.\"Svc\" { }",
                ),
            ],
        )]);
        assert_eq!(
            codes(&ws),
            vec![("AR0002", "unresolved page `N.Other.Svc`".to_string())]
        );
    }

    /// A table that moved between apps stays in the old app as a stub marked `ObsoleteState = Moved`
    /// (Base Application's "No. Series", which Business Foundation now declares). Microsoft's own
    /// test libraries compile against both apps, so the name must reach the real table. Nothing in
    /// the corpus pins this (docs/probes-needed.md, "Probe batch 4").
    #[test]
    fn a_moved_stub_does_not_make_the_real_table_ambiguous() {
        let bf = app(
            "bf",
            &[],
            &[(
                "t.al",
                "namespace M.NoSeries; table 308 \"No. Series\" { fields { field(1; Code; Code[20]) { } } }",
            )],
        );
        let base = app(
            "base",
            &["bf"],
            &[(
                "t.al",
                "namespace M.NoSeries; table 308 \"No. Series\" { ObsoleteState = Moved; MovedTo = 'x'; \
                 fields { field(1; Code; Code[20]) { } } }",
            )],
        );
        let lib = app("lib", &["base", "bf"], &[("c.al", "codeunit 131000 \"Lib\" { }")]);
        let ws = Workspace::load_apps(
            vec![bf, base, lib],
            vec![AppKind::Dependency; 3],
            &al_version::corpus_symbols(27),
        );
        let file = ws.objects.iter().find(|o| o.name == "Lib").unwrap().file;
        let name = vec!["No. Series".to_string()];
        let Resolution::Found(t) = ws.resolve_ref(file, ObjKind::Table, &name) else {
            panic!("the moved stub made the name ambiguous");
        };
        assert_eq!(ws.apps[ws.objects[t].app].name, "bf");
        // Both stubs: nothing real to prefer, so it stays ambiguous.
        let only_stubs = Workspace::load_apps(
            vec![
                app("s1", &[], &[("t.al", "table 5 T { ObsoleteState = Moved; }")]),
                app("s2", &[], &[("t.al", "table 5 T { ObsoleteState = Moved; }")]),
                app("lib", &["s1", "s2"], &[("c.al", "codeunit 9 \"Lib\" { }")]),
            ],
            vec![AppKind::Dependency; 3],
            &al_version::corpus_symbols(27),
        );
        let file = only_stubs.objects.iter().find(|o| o.name == "Lib").unwrap().file;
        assert!(matches!(
            only_stubs.resolve_ref(file, ObjKind::Table, &["T".to_string()]),
            Resolution::Ambiguous(_)
        ));
    }

    #[test]
    fn parse_errors_are_reported_as_diagnostics() {
        let ws = load(vec![app("a", &[], &[("x.al", "codeunit 1 A { 42 }")])]);
        assert_eq!(ws.diagnostics.len(), 1);
        assert_eq!(ws.diagnostics[0].code, "AR0000");
    }
}
