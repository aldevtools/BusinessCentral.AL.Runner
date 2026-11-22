//! A workspace is a set of apps. Each app is its manifest plus its sources
//! (path, text). Loading parses every file and builds the object table.

use crate::check::posmap::{FxMap as HashMap, FxSet as HashSet};
use al_syntax::ast;
use rayon::prelude::*;
use std::collections::BTreeSet;
use std::sync::OnceLock;

mod dump;
mod moved_tables;
pub(crate) mod stored;
mod subscriber_publisher;

use stored::GivenFile;
pub use stored::{StoredFile, StoredWorkspace};

/// Which tiers of a bare name's search `Workspace::resolve_in_scope` takes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tiers {
    /// The compiler's: the file's namespace, its usings, the global namespace, then a unique match in any namespace.
    Compiler,
    /// Only those no absent app could shadow (`Workspace::resolve_ref_certain`).
    Certain,
    /// The compiler's, without the match in any namespace for a file that has a namespace
    /// (`Workspace::resolve_relation_target`).
    Relation,
}

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
            .map(|i| {
                crate::halt::check();
                self.loader.text(i)
            })
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

impl Diagnostic {
    /// What makes two diagnostics one finding: the app, the file, the offset and the code all equal (a path is relative
    /// to its app; two apps can hold the same one).
    pub fn site(&self) -> (usize, &str, usize, &'static str) {
        (self.app, self.file.as_str(), self.offset, self.code)
    }
}

/// The position of each file in `Workspace::files` by its app and path (a path is relative to its app; two apps can hold
/// the same one). The one order of `Workspace::diagnostics`, `CheckResult::diagnostics` and `Lowered::al_diagnostics`;
/// the printed order of compile errors is `compile_errors`' own (al-testrun).
pub struct FileOrder<'a>(HashMap<(usize, &'a str), usize>);

impl FileOrder<'_> {
    /// Sorts by file, then offset; a diagnostic of a file the workspace does not hold comes first; diagnostics at one
    /// position keep their order.
    pub fn sort(&self, diags: &mut [Diagnostic]) {
        diags.sort_by_key(|d| (self.0.get(&(d.app, d.file.as_str())).copied(), d.offset));
    }
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
    /// Whether the file's text names `MaxLength` anywhere (`Workspace::names_max_length`), found once per file.
    max_length: OnceLock<bool>,
    /// Whether the file's text names `AccessByPermission` anywhere (`Workspace::names_access_by_permission`), found once per file.
    access_by_permission: OnceLock<bool>,
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

/// Whether the bytes hold `word` (lower case ASCII) in any case. The text of every file of every app is searched once, so
/// the cost per byte counts: the search runs a vector scan for the case pairs of one letter of the word that is rare in
/// source (`pivot`, an index into `word`) and compares the whole word only around those.
fn names_word_in(b: &[u8], word: &[u8], pivot: usize) -> bool {
    let (lower, upper) = (word[pivot], word[pivot].to_ascii_uppercase());
    let mut from = pivot;
    while let Some(at) = memchr::memchr2(lower, upper, b.get(from..).unwrap_or_default()) {
        let start = from + at - pivot;
        if b.get(start..start + word.len())
            .is_some_and(|w| w.eq_ignore_ascii_case(word))
        {
            return true;
        }
        from += at + 1;
    }
    false
}

/// Whether the bytes hold `maxlength` in any case. Scans for `x`.
fn names_max_length_in(b: &[u8]) -> bool {
    names_word_in(b, b"maxlength", 2)
}

/// Whether the bytes hold `accessbypermission` in any case. Scans for `b`.
fn names_access_by_permission_in(b: &[u8]) -> bool {
    names_word_in(b, b"accessbypermission", 6)
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
    /// Event subscribers of scanned objects by the publisher name as written: (kind, lower-case name or id, namespace and name
    /// when written) to (subscriber object, index in its `subscribers`). The platform codeunits, which no app need declare, are
    /// found here (`codeunit_subscribers`).
    subscribers_of: HashMap<(ObjKind, String), Vec<(usize, usize)>>,
    /// Event subscribers of scanned objects by the publisher object their name names (`subscribers_of`).
    subscribers_by_publisher: HashMap<usize, Vec<(usize, usize)>>,
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
    /// See [`Workspace::loaded_files`].
    loaded_files: usize,
    /// Whether every table has the four user-name system FlowFields (`members::USER_NAME_FIELDS`): the target is BC 29 or
    /// later. The default is the corpus symbol of the gate's BC major (`BC<major>PLUS`); a compile with a target sets it from the target's runtime
    /// ([`Workspace::set_user_name_fields_for`]).
    user_name_fields: bool,
    /// The ids (lower case) of the apps the platform package's `InternalsVisibleTo` grants access to its internal
    /// members, matched by id and publisher by the caller ([`Workspace::set_internals_granted`]).
    internals_granted: Vec<String>,
}

/// The gate that says from which BC major every table has the user-name system FlowFields.
const USER_NAME_FIELDS_GATE: al_version::Gate = al_version::Gate::PlatformMetadataColumnsFrom29;

impl Workspace {
    /// The order of the files, to sort diagnostics by (`FileOrder::sort`).
    pub fn file_order(&self) -> FileOrder<'_> {
        FileOrder(
            self.files
                .iter()
                .enumerate()
                .map(|(i, f)| ((f.app, f.path.as_str()), i))
                .collect(),
        )
    }

    /// Whether every table has the user-name system FlowFields (SystemCreatedByUserName and the others, BC 29 and later).
    pub fn user_name_fields(&self) -> bool {
        self.user_name_fields
    }

    /// Sets [`Workspace::user_name_fields`] from the run target's BC major; `None` (no target) keeps the default.
    pub fn set_user_name_fields_for(&mut self, bc_major: Option<u16>) {
        if let Some(m) = bc_major {
            self.user_name_fields = m >= USER_NAME_FIELDS_GATE.since_bc();
        }
    }

    /// Sets the apps the platform package's `InternalsVisibleTo` grants (ids; issue 7853). The caller matches each grant
    /// by app id and publisher, as alc 18.0.39.10160 does.
    pub fn set_internals_granted(&mut self, app_ids: Vec<String>) {
        self.internals_granted = app_ids;
    }

    /// Whether the platform package grants app `app` access to its internal members (see `set_internals_granted`).
    pub fn internals_granted_to(&self, app: usize) -> bool {
        let id = &self.apps[app].id;
        self.internals_granted.iter().any(|g| g.eq_ignore_ascii_case(id))
    }

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
        let indexes: Vec<Option<Vec<GivenFile>>> = lazy
            .into_iter()
            .map(|l| l.map(|l| l.index.into_iter().map(GivenFile::scanned).collect()))
            .collect();
        Workspace::load_given(apps, kinds, symbols, symbols_of, loaders, indexes, None)
    }

    /// The load of [`load_apps_symbols`](Self::load_apps_symbols), with the files of some apps given (`indexes`, by app)
    /// and, with `diagnostics`, the references not resolved again (`stored.rs`).
    fn load_given(
        apps: Vec<AppSource>,
        kinds: Vec<AppKind>,
        symbols: &BTreeSet<String>,
        symbols_of: Vec<BTreeSet<String>>,
        loaders: Vec<Option<std::sync::Arc<dyn SourceLoader>>>,
        indexes: Vec<Option<Vec<GivenFile>>>,
        diagnostics: Option<Vec<Diagnostic>>,
    ) -> Workspace {
        let indexes = indexes.into_iter().map(|l| l.map(Vec::into_iter)).collect();
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
            symbols: symbols_of,
            ext_target: Vec::new(),
            ext_of: HashMap::default(),
            subscribers_of: HashMap::default(),
            subscribers_by_publisher: HashMap::default(),
            implementers_of: HashMap::default(),
            iface_base: HashMap::default(),
            iface_derived: HashMap::default(),
            usable_cache: Vec::new(),
            flakes: Default::default(),
            compiled: OnceLock::new(),
            unreached: OnceLock::new(),
            loaders,
            lazy_text: Vec::new(),
            loaded_files: 0,
            user_name_fields: symbols.contains(&format!("BC{}PLUS", USER_NAME_FIELDS_GATE.since_bc())),
            internals_granted: Vec::new(),
        };
        let all: Vec<&AppSource> = apps.iter().collect();
        ws.visible = (0..apps.len()).map(|i| visible_from(&all, i)).collect();
        ws.ingest(apps, indexes, diagnostics);
        ws
    }

    /// Adds `apps` (dependency apps, each with its preprocessor symbols in `app_symbols`, or `symbols` when that is
    /// `None`) after the apps already loaded, as if [`load_apps_symbols`](Self::load_apps_symbols) had been given the
    /// apps of this workspace followed by `apps` (ADR-022 amendment 2026-10-27, decision 2): objects, files and
    /// `DotNet` aliases of the new apps come after those already there, the derived tables grow by the new objects
    /// only, and what a compile set (`restrict_to`, `set_unreached`) is cleared. The files already loaded are not read
    /// or scanned again; trees parsed before stay (a parse is a function of the text and the symbols).
    ///
    /// An error says the result would not equal a fresh load, and nothing has changed: an app already loaded would
    /// see another set of apps (a dependency that only the new apps provide), or a loaded app is not a dependency.
    pub fn append_apps(
        &mut self,
        apps: Vec<AppSource>,
        symbols: &BTreeSet<String>,
        app_symbols: &[Option<BTreeSet<String>>],
    ) -> Result<(), String> {
        if self.kinds.iter().any(|k| *k != AppKind::Dependency) {
            return Err("only a workspace of dependency apps grows".into());
        }
        let kinds = vec![AppKind::Dependency; apps.len()];
        self.append_apps_of(apps, kinds, symbols, app_symbols)
    }

    /// Whether `apps` can be appended ([`append_apps_of`](Self::append_apps_of)): no app already loaded would see one of
    /// them. An error says why not.
    pub fn can_append(&self, apps: &[AppSource]) -> Result<(), String> {
        self.visible_with(apps).map(|_| ())
    }

    fn visible_with(&self, apps: &[AppSource]) -> Result<Vec<BTreeSet<usize>>, String> {
        let all: Vec<&AppSource> = self.apps.iter().chain(apps.iter()).collect();
        let visible: Vec<BTreeSet<usize>> = (0..all.len()).map(|i| visible_from(&all, i)).collect();
        if visible[..self.apps.len()] != self.visible[..] {
            return Err("an app already loaded would see the new apps".into());
        }
        Ok(visible)
    }

    /// [`append_apps`](Self::append_apps) for a workspace of any apps, with the kind of each new app (ADR-063: a later
    /// pass of a run adds its test apps and dependency apps to the compile of the main pass). The result is the workspace
    /// [`load_apps_symbols`](Self::load_apps_symbols) makes of the apps already loaded followed by `apps`, with their
    /// kinds; a workspace app among `apps` is parsed, a dependency app scanned.
    pub fn append_apps_of(
        &mut self,
        apps: Vec<AppSource>,
        kinds: Vec<AppKind>,
        symbols: &BTreeSet<String>,
        app_symbols: &[Option<BTreeSet<String>>],
    ) -> Result<(), String> {
        assert_eq!(apps.len(), kinds.len(), "one kind per app");
        self.visible = self.visible_with(&apps)?;
        self.kinds.extend(kinds);
        self.symbols
            .extend((0..apps.len()).map(|i| app_symbols.get(i).cloned().flatten().unwrap_or_else(|| symbols.clone())));
        self.loaders.extend((0..apps.len()).map(|_| None));
        self.compiled = OnceLock::new();
        self.unreached = OnceLock::new();
        self.ingest(apps, (0..self.kinds.len()).map(|_| None).collect(), None);
        Ok(())
    }

    /// Cuts the workspace back to its first `n` apps (ADR-063: the main pass's own test apps leave the compile that a
    /// later pass grows), and gives the others back. The result is the workspace [`load_apps_symbols`]
    /// (Self::load_apps_symbols) makes of the first `n` apps, with the trees parsed so far: the apps of a workspace see
    /// only apps before them or among themselves; one of the first `n` that sees one of the others is a bug of the caller
    /// (a panic). What a compile set (`restrict_to`, `set_unreached`) is cleared.
    pub fn truncate_apps(&mut self, n: usize) -> Vec<AppSource> {
        assert!(
            self.visible[..n].iter().all(|v| v.iter().all(|&a| a < n)),
            "an app that stays sees an app that goes"
        );
        let n_obj = self
            .objects
            .iter()
            .position(|o| o.app >= n)
            .unwrap_or(self.objects.len());
        let n_file = self.files.iter().position(|f| f.app >= n).unwrap_or(self.files.len());
        assert!(
            self.objects[n_obj..].iter().all(|o| o.app >= n) && self.files[n_file..].iter().all(|f| f.app >= n),
            "the objects and files of an app follow those of the apps before it"
        );
        let removed = self.apps.split_off(n);
        self.kinds.truncate(n);
        self.visible.truncate(n);
        self.symbols.truncate(n);
        self.loaders.truncate(n);
        self.files.truncate(n_file);
        self.lazy_text.truncate(n_file);
        self.objects.truncate(n_obj);
        self.usable_cache.truncate(n_obj);
        self.ext_target.truncate(n_obj);
        self.dotnet.retain(|d| d.app < n);
        self.diagnostics.retain(|d| d.app < n);
        let keep = |v: &mut Vec<usize>| {
            v.retain(|&i| i < n_obj);
            !v.is_empty()
        };
        self.by_name.retain(|_, v| keep(v));
        self.by_id.retain(|_, v| keep(v));
        self.ext_of.retain(|&k, v| k < n_obj && keep(v));
        self.implementers_of.retain(|_, v| keep(v));
        self.iface_derived.retain(|&k, v| k < n_obj && keep(v));
        self.iface_base.retain(|&k, &mut b| k < n_obj && b < n_obj);
        self.subscribers_of.retain(|_, v| {
            v.retain(|&(o, _)| o < n_obj);
            !v.is_empty()
        });
        self.subscribers_by_publisher.retain(|&k, v| {
            v.retain(|&(o, _)| o < n_obj);
            k < n_obj && !v.is_empty()
        });
        self.loaded_files = self.files.len();
        self.compiled = OnceLock::new();
        self.unreached = OnceLock::new();
        removed
    }

    /// How many files this workspace has read and scanned or parsed while loading and growing: each file once.
    pub fn loaded_files(&self) -> usize {
        self.loaded_files
    }

    /// Reads `apps`, which come after `self.apps` (their kinds, symbols, loaders and visibility are set), and
    /// indexes their objects. `indexes` holds, per app of the whole workspace, the scan a lazy app comes with, or what
    /// the parse of a workspace app's files found when a stored compile gives them (`stored.rs`). With `diagnostics` (a
    /// stored compile's), the references are not resolved again, which would parse the files: those are the diagnostics.
    fn ingest(
        &mut self,
        apps: Vec<AppSource>,
        mut indexes: Vec<Option<std::vec::IntoIter<GivenFile>>>,
        diagnostics: Option<Vec<Diagnostic>>,
    ) {
        let first_app = self.apps.len();
        let first_object = self.objects.len();
        let first_file = self.files.len();
        let ws = self;
        let symbols_of = &ws.symbols;
        // Parsing dominates loading and each file is independent, so it runs on
        // all cores. `collect` keeps the input order, so the sequential merge
        // below sees files, diagnostics and objects exactly as a serial load would.
        // Expression-valued page properties become routines first (ADR-012).
        let inputs: Vec<(usize, usize, bool, &str, &str)> = apps
            .iter()
            .enumerate()
            .flat_map(|(ai, app)| {
                let ai = first_app + ai;
                let lazy = ws.kinds[ai] == AppKind::Dependency;
                app.files
                    .iter()
                    .enumerate()
                    .map(move |(fi, (path, src))| (ai, fi, lazy, path.as_str(), src.as_str()))
            })
            .collect();
        ws.loaded_files += inputs.len();
        // The files of a lazy app come with their scan, those of a stored compile's workspace app with their parse's.
        let given: Vec<Option<(ParsedFile, Vec<ByteRange>)>> = inputs
            .iter()
            .map(|&(ai, ..)| {
                indexes[ai].as_mut().map(|it| {
                    let GivenFile { index: f, synthetic } = it.next().expect("one index entry per file");
                    let file = ParsedFile {
                        green: None,
                        errors: Vec::new(),
                        namespace: f.namespace,
                        usings: f.usings,
                        objects: f.objects,
                        dotnet: f.dotnet,
                    };
                    (file, synthetic)
                })
            })
            .collect();
        let parsed: Vec<(ParsedFile, Vec<ByteRange>)> = inputs
            .par_iter()
            .zip(given.into_par_iter())
            .map(|(&(ai, _, lazy, _, src), given)| {
                crate::halt::check(); // A hard memory limit stops the compile here (`halt.rs`).
                if let Some(given) = given {
                    return given;
                }
                let symbols = &symbols_of[ai];
                if lazy {
                    (scanned_file(src, symbols), Vec::new())
                } else {
                    match crate::pages::expand_or_parse(src, symbols) {
                        crate::pages::Expansion::Expanded(_, added, parse) => (parsed_file(parse), added),
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
                max_length: OnceLock::new(),
                access_by_permission: OnceLock::new(),
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
        ws.apps.extend(apps);
        ws.lazy_text
            .extend((first_file..ws.files.len()).map(|_| OnceLock::new()));
        ws.usable_cache
            .extend((first_object..ws.objects.len()).map(|_| OnceLock::new()));
        ws.index_extensions_and_subscribers(first_object);
        if let Some(stored) = diagnostics {
            ws.diagnostics = stored;
            return;
        }
        ws.resolve_references(first_object);
        let mut diagnostics = std::mem::take(&mut ws.diagnostics);
        ws.file_order().sort(&mut diagnostics);
        ws.diagnostics = diagnostics;
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
        let old_green = self.green(fi);
        let symbols = &self.symbols[ai];
        let (parsed, added): (ParsedFile, Vec<ByteRange>) = match crate::pages::expand_or_parse(&text, symbols) {
            crate::pages::Expansion::Expanded(_, added, parse) => (parsed_file(parse), added),
            crate::pages::Expansion::Unchanged(parse) => (parsed_file(parse), Vec::new()),
        };
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
        let mut diagnostics = std::mem::take(&mut self.diagnostics);
        self.file_order().sort(&mut diagnostics);
        self.diagnostics = diagnostics;
        Ok((in_file, declared))
    }

    /// Extension targets and event subscribers, found from the object index alone.
    /// For the objects from `first` on: the objects before it are indexed, and an object never sees one after it
    /// (`append_apps` keeps the apps it could see unchanged).
    fn index_extensions_and_subscribers(&mut self, first: usize) {
        self.ext_target.resize(self.objects.len(), None);
        for i in first..self.objects.len() {
            let o = &self.objects[i];
            if let (Some(kind), Some(parts)) = (o.kind.extended(), &o.extends) {
                // Not `resolve_ref`: that would parse the target's file, and an index of
                // thousands of extensions must not parse their targets.
                if let Resolution::Found(t) = self.resolve_in_scope(o.file, kind, parts, Tiers::Compiler, false) {
                    self.ext_target[i] = Some(t);
                    self.ext_of.entry(t).or_default().push(i);
                }
            }
        }
        for i in first..self.objects.len() {
            let o = &self.objects[i];
            if let (ObjKind::Interface, Some(parts)) = (o.kind, &o.extends) {
                // Like the extension targets above: no parse of the base interface's file.
                if let Resolution::Found(b) =
                    self.resolve_in_scope(o.file, ObjKind::Interface, parts, Tiers::Compiler, false)
                {
                    self.iface_base.insert(i, b);
                    self.iface_derived.entry(b).or_default().push(i);
                }
            }
        }
        for i in first..self.objects.len() {
            let o = &self.objects[i];
            for (si, sub) in o.subscribers.iter().enumerate() {
                let kind = ObjKind::from_keyword(&sub.publisher_kind);
                for p in self.scanned_subscriber_publishers(o.file, kind, &sub.publisher) {
                    self.subscribers_by_publisher.entry(p).or_default().push((i, si));
                }
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
    /// base after it). A cycle ends the list. Pinned by codeunits/TestInterfaceRuntimeContracts.al:
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

    /// Subscribers (object, index in its `subscribers`) of scanned objects that name `publisher` as their publisher (see
    /// `scanned_subscriber_publishers`).
    pub fn subscribers_of(&self, publisher: usize) -> Vec<(usize, usize)> {
        self.subscribers_by_publisher
            .get(&publisher)
            .cloned()
            .unwrap_or_default()
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

    /// Subscribers to the platform's `Reporting Triggers` codeunit (`al_ir::REPORTING_CODEUNIT`), by name or id, whose
    /// `CustomDocumentMergerEx` the runtime raises when a report renders with a custom layout.
    pub fn reporting_triggers_subscribers(&self) -> Vec<(usize, usize)> {
        self.platform_codeunit_subscribers(al_ir::REPORTING_CODEUNIT)
    }

    /// Subscribers to the platform's `UI Helper Triggers` codeunit (`al_ir::UI_HELPER_CODEUNIT`), by name or id, whose
    /// `CaptionClassTranslate` the runtime raises for the `CaptionClassTranslate` statement.
    pub fn ui_helper_triggers_subscribers(&self) -> Vec<(usize, usize)> {
        self.platform_codeunit_subscribers(al_ir::UI_HELPER_CODEUNIT)
    }

    /// Subscribers to the platform's `Filter Helper Triggers` codeunit (`al_ir::FILTER_HELPER_CODEUNIT`), by name or id,
    /// whose `MakeDateFilter` (and `MakeIntFilter`, `MakeTimeFilter`, `RAISED_FILTER_EVENTS`) the runtime raises for a Date
    /// filter text with a piece that does not read.
    pub fn filter_helper_triggers_subscribers(&self) -> Vec<(usize, usize)> {
        self.platform_codeunit_subscribers(al_ir::FILTER_HELPER_CODEUNIT)
    }

    /// The subscribers to platform codeunit `(id, name)` written with its name or its id (al-runner issue 8466), both forms
    /// in one order of the subscribing objects (al-link lists them so).
    fn platform_codeunit_subscribers(&self, (id, name): (u32, &str)) -> Vec<(usize, usize)> {
        let mut found = self.codeunit_subscribers(&[&name.to_ascii_lowercase(), &id.to_string()]);
        found.sort_unstable();
        found
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

    /// Whether object `oi` (of an app under test, whose file is parsed) declares a subscriber to a login event the test
    /// session raises (al-testrun `login.rs`, `al_ir::names_login_event`).
    pub fn declares_login_subscriber(&self, oi: usize) -> bool {
        if self.objects[oi].kind != ObjKind::Codeunit {
            return false;
        }
        ast::routines_in(self.object_ast(oi).syntax()).iter().any(|r| {
            r.attributes().any(|a| {
                a.name().eq_ignore_ascii_case("EventSubscriber")
                    && al_ir::names_login_event(&a.syntax().text().to_string())
            })
        })
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

    /// Whether the text of file `fi` names `MaxLength` anywhere, in any case. Searched once per file.
    pub fn names_max_length(&self, fi: usize) -> bool {
        *self.files[fi]
            .max_length
            .get_or_init(|| names_max_length_in(self.text(fi).as_bytes()))
    }

    /// Whether the text of file `fi` names `AccessByPermission` anywhere, in any case. Searched once per file: a page of a file
    /// that does not has no mention for `lower::shared::access_by_permission_refusals` to count in its syntax text.
    pub fn names_access_by_permission(&self, fi: usize) -> bool {
        *self.files[fi]
            .access_by_permission
            .get_or_init(|| names_access_by_permission_in(self.text(fi).as_bytes()))
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

    /// Drops the parse tree of every file: on the corpus about a sixth of a run's memory, and after the lowering only
    /// what a later compile changes needs a tree. A later [`Workspace::green`] parses the file again; a parse is a
    /// function of the text, so the tree is the same. What the first parse found besides the tree (the synthetic
    /// ranges, the parse problem) is kept.
    pub fn drop_trees(&mut self) {
        for f in &mut self.files {
            f.green = OnceLock::new();
        }
    }

    /// The parse tree of file `fi`, parsing it now if it was only scanned at load or its tree was dropped
    /// ([`Workspace::drop_trees`]).
    pub fn green(&self, fi: usize) -> rowan::GreenNode {
        let f = &self.files[fi];
        f.green
            .get_or_init(|| {
                let (kept, flake) = settle(self.parse_lazy(fi, f, false), || self.parse_lazy(fi, f, true));
                if let Some(note) = flake {
                    self.note_parse_flake(format!("{}: {note}", f.path));
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
        // Only a page, report or table can have properties to turn into routines; skip the extra parse otherwise. The parse that looked
        // is the file's parse, with the added routines spliced in when there are any; a `fresh` parse (the second one, of a
        // file whose first parse had a problem) is made anew.
        let (expanded, reused) = match f
            .has_page
            .then(|| crate::pages::expand_or_parse(src, &self.symbols[f.app]))
        {
            Some(crate::pages::Expansion::Expanded(text, added, parse)) => {
                let reused = (!fresh).then(|| parsed_file(parse));
                (Some((text, added)), reused)
            }
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
    /// Records a file whose second parse did not give the result of the first ([`Workspace::parse_flakes`]).
    pub fn note_parse_flake(&self, note: String) {
        self.flakes.lock().unwrap_or_else(|e| e.into_inner()).push(note);
    }

    /// Files whose second parse did not give the result of the first (a defect: a parse is a function of
    /// the text), as one line each with both results. Empty in every run so far.
    pub fn parse_flakes(&self) -> Vec<String> {
        self.flakes.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The byte ranges of file `fi`'s parsed text that the compiler added (page expression routines).
    pub fn synthetic(&self, fi: usize) -> &[ByteRange] {
        if self.files[fi].synthetic.get().is_none() {
            self.green(fi);
        }
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
        if f.problem.get().is_none() {
            self.green(fi);
        }
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
    /// it depends on. Aliases are case-insensitive. `alias` holds the written parts joined by
    /// `QUAL_SEP`: one part (bare or quoted, dots and all) is an alias in any namespace; `A.B.C` is
    /// alias `C` declared in namespace `A.B`, never a .NET full type name (Microsoft's System
    /// Application declares `type("System.DateTime"; "DateTime")` in `namespace System;` and its AFS
    /// Format Helper `DotNet System.DateTime`; al-runner issue 6488, corpus writer Y20,
    /// al-language-onprem-dotnet/dotnet/dotnet-fullname/Y20DotNetFullTypeName.al, must-not-compile 2357, 2358).
    pub fn resolve_dotnet(&self, file: usize, alias: &str) -> Option<usize> {
        let app = self.files[file].app;
        let (namespace, name) = match alias.rsplit_once(crate::types::QUAL_SEP) {
            Some((ns, name)) => (Some(ns.replace(crate::types::QUAL_SEP, ".")), name),
            None => (None, alias),
        };
        let in_namespace = |d: &DotNetSym| match &namespace {
            None => true,
            Some(ns) => self.files[d.file]
                .namespace
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(ns)),
        };
        let find = |a: usize| {
            self.dotnet
                .iter()
                .position(|d| d.app == a && d.decl.alias.eq_ignore_ascii_case(name) && in_namespace(d))
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

    /// Whether `app` is a dependency app that stands on a workspace app, directly or through other dependency apps
    /// (al-runner issue 6044: a `--dep` app on a workspace app).
    pub fn dependency_on_workspace(&self, app: usize) -> bool {
        self.kinds.get(app) == Some(&AppKind::Dependency)
            && self.visible[app]
                .iter()
                .any(|&a| self.kinds.get(a).is_some_and(|k| *k != AppKind::Dependency))
    }

    /// Whether `app` sees `other`: itself or one of its dependencies, directly or through others.
    pub fn app_sees(&self, app: usize, other: usize) -> bool {
        self.visible.get(app).is_some_and(|v| v.contains(&other))
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
        self.resolve_in_scope(file, kind, parts, Tiers::Compiler, true)
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
        !self.no_app_absent(self.files[file].app)
            || !matches!(
                self.resolve_in_scope(file, kind, parts, Tiers::Compiler, false),
                Resolution::Missing
            )
    }

    /// Whether every app `app` could see is loaded: no dependency that `app` or an app it sees declares is missing
    /// ([`has_unloaded_dependency`](Self::has_unloaded_dependency)), and the platform app System, an implicit dependency of
    /// every app, is loaded. Only then is the absence of a name known.
    pub fn no_app_absent(&self, app: usize) -> bool {
        self.apps.iter().any(|a| a.id.eq_ignore_ascii_case(PLATFORM_APP_ID)) && !self.has_unloaded_dependency(app)
    }

    /// Whether an object of `kind` named `name` (bare) is among the objects `file` searches before the any-namespace
    /// fallback of [`resolve_ref`](Self::resolve_ref): those of its own namespace, of its `using`s, and those without a
    /// namespace.
    pub fn in_scope_tiers(&self, file: usize, kind: ObjKind, name: &str) -> bool {
        let f = &self.files[file];
        self.by_name
            .get(&(kind, name.to_lowercase()))
            .into_iter()
            .flatten()
            .filter(|&&i| self.visible[f.app].contains(&self.objects[i].app))
            .any(|&i| {
                let ns = &self.objects[i].namespace;
                ns.is_none() || eq_ns(ns, &f.namespace) || f.usings.iter().any(|u| eq_ns(ns, &Some(u.clone())))
            })
    }

    /// The objects of `kind` named `name` (bare) among the apps `file`'s app sees, in object order.
    pub fn named_objects_visible(&self, file: usize, kind: ObjKind, name: &str) -> Vec<usize> {
        let app = self.files[file].app;
        self.by_name
            .get(&(kind, name.to_lowercase()))
            .into_iter()
            .flatten()
            .copied()
            .filter(|&i| self.visible[app].contains(&self.objects[i].app))
            .collect()
    }

    /// Whether an object of any kind, in any namespace, among the apps `file`'s app sees is named `name` (bare).
    pub fn names_any_visible_object(&self, file: usize, name: &str) -> bool {
        ObjKind::ALL
            .iter()
            .any(|&kind| !self.named_objects_visible(file, kind, name).is_empty())
    }

    /// Whether `app` or an app it sees declares a dependency that is not among the loaded apps.
    /// The `Application` aggregate is never loaded (it has no source) and declares no objects: it hides nothing once Base
    /// Application, which depends on the rest of the aggregate, is among the apps `app` sees.
    pub fn has_unloaded_dependency(&self, app: usize) -> bool {
        let loaded = |id: &str| self.apps.iter().any(|x| x.id.eq_ignore_ascii_case(id));
        let sees_base = self.visible[app]
            .iter()
            .any(|&a| self.apps[a].id.eq_ignore_ascii_case(BASE_APPLICATION_APP_ID));
        self.visible[app].iter().any(|&a| {
            self.apps[a]
                .dependencies
                .iter()
                .any(|d| !loaded(d) && !(sees_base && d.eq_ignore_ascii_case(APPLICATION_AGGREGATE_APP_ID)))
        })
    }

    /// The id of the platform table `parts` names when nothing provides rows for it. The id is
    /// a constant of its declaration, so `Database::"X"` can name it.
    pub fn unprovided_platform_table_id(&self, file: usize, parts: &[String]) -> Option<i32> {
        match self.resolve_in_scope(file, ObjKind::Table, parts, Tiers::Compiler, false) {
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
        self.resolve_in_scope(file, kind, parts, Tiers::Certain, true)
    }

    /// Whether Microsoft's Base Application is among the apps `app` sees. Then no app is absent that
    /// could shadow a name, and the tiers resolve as the compiler resolves them.
    pub(crate) fn sees_base_application(&self, app: usize) -> bool {
        self.visible[app].iter().any(|&a| {
            self.kinds[a] == AppKind::Dependency && self.apps[a].name.eq_ignore_ascii_case("Base Application")
        })
    }

    /// A `TableRelation` target. The file of a dependency app resolves it among the apps that app
    /// saw (`visible`), as BC fixed it when the app compiled: a same-named table of a dependent app
    /// is not a candidate (corpus recordref-fieldref/fieldref/TestRelationTargetNameCollision.al). The file of a
    /// workspace app resolves it the same way when no app it could see is absent: when it sees Base Application, or
    /// when every app it could see is loaded ([`no_app_absent`](Self::no_app_absent)). The name then means what it
    /// means to the compiler: its own namespace, its usings (TestRelationTargetNamespaceScope.al:235, a table of the app
    /// reached through a using), the global namespace, and a unique match in any namespace only from a file without a
    /// namespace (types/PZMisc.al:27, :31 `Field."No."` and metadata/CmInstallObjects.al:217 `AllObj."Object ID"`, a
    /// platform table named from a file without namespace or using). A file with a namespace does not reach a table of
    /// another namespace without a using (must-not-compile 2081-namespace-record-integer-without-using.al, AL0185).
    /// Otherwise it resolves as `resolve_ref_certain` does.
    pub fn resolve_relation_target(&self, file: usize, kind: ObjKind, parts: &[String]) -> Resolution {
        let app = self.files[file].app;
        if self.kinds[app] == AppKind::Dependency || self.sees_base_application(app) || self.no_app_absent(app) {
            self.resolve_in_scope(file, kind, parts, Tiers::Relation, true)
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
        tiers: Tiers,
        check_usable: bool,
    ) -> Resolution {
        let certain_only = tiers == Tiers::Certain;
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
            if certain_only || (tiers == Tiers::Relation && !no_ns) {
                Vec::new()
            } else {
                cands.clone()
            },
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
    /// An object marked `ObsoleteState = Removed` gives way the same way: naming a removed table does not
    /// compile (corpus must-not-compile 0970-obsolete-removed-table.al, AL0433), so code that compiles with a
    /// removed and a real candidate names the real one (Subscription Billing's SubContrAnalysisEntry.Table.al,
    /// no namespace and no `using`, `TableRelation = "Dimension Set Entry"`: Base Application's table and Power BI
    /// Report embeddings' removed table 36950 of that name).
    fn pick_real(&self, tier: Vec<usize>) -> Resolution {
        pick(self.without_moved_stubs(tier))
    }

    /// The candidates of `tier` without the stubs that give way (see [`Self::pick_real`]): unchanged when it holds one candidate
    /// or only stubs.
    fn without_moved_stubs(&self, tier: Vec<usize>) -> Vec<usize> {
        let moved = |i: &usize| {
            self.objects[*i].scanned_properties.iter().any(|(n, v)| {
                n.eq_ignore_ascii_case("ObsoleteState")
                    && (v.trim().eq_ignore_ascii_case("Moved") || v.trim().eq_ignore_ascii_case("Removed"))
            })
        };
        // A workspace table that is the moved stub of its dependency's table gives way the same way
        // (`moved_table_of_dependency`, assumption `moved-table-stub-of-dependency`).
        let moved = |i: &usize| moved(i) || self.moved_table_of_dependency(*i).is_some();
        if tier.len() > 1 && tier.iter().any(|i| !moved(i)) {
            return tier.into_iter().filter(|i| !moved(i)).collect();
        }
        tier
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
    fn resolve_references(&mut self, first: usize) {
        let per_object: Vec<Vec<Diagnostic>> = self.objects[first..]
            .par_iter()
            .map(|o| {
                crate::halt::check();
                self.unresolved_references_of(o)
            })
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
        // A statement block holds no type reference: the grammar of statements and expressions
        // (al-syntax grammar/statements.rs, grammar/expressions.rs) never parses one, only declarations do.
        // Blocks are most of the tree, so they are not entered; the order of the rest is unchanged.
        let type_refs = ast::find_nodes(
            &node,
            |k| k == al_syntax::SyntaxKind::TypeRef,
            |k| k != al_syntax::SyntaxKind::Block,
        );
        for n in type_refs {
            let Some(t) = ast::TypeRef::cast(n) else { continue };
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

/// Id of Microsoft's `Application` aggregate: a package without source or objects that only depends on System
/// Application, Business Foundation and Base Application (al-deps `select::APPLICATION_APP_ID`).
pub const APPLICATION_AGGREGATE_APP_ID: &str = "c1335042-3002-4257-bf8a-75c898ccb1b8";

/// Id of Microsoft's Base Application.
pub const BASE_APPLICATION_APP_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

/// Id of the platform app `System`, which declares the system tables. BC always provides it, so it
/// is an implicit dependency of every app (ADR-013 decision 4).
pub const PLATFORM_APP_ID: &str = "8874ed3a-0643-4247-9ced-7a7002f7135d";

fn visible_from(apps: &[&AppSource], start: usize) -> BTreeSet<usize> {
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
mod tests;
