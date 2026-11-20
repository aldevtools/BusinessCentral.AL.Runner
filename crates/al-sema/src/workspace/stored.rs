//! A workspace as a stored compile keeps it (ADR-022 amendment 2026-11-15, the workspace layer): what the parse of each
//! file of a workspace app found besides its tree, the diagnostics of the load, and what the compile set on the
//! workspace (`restrict_to`, `set_unreached`). [`Workspace::load_stored`] makes from it and the same sources the
//! workspace a fresh load and compile make, without parsing a workspace file: a file is parsed the first time its tree
//! is needed ([`Workspace::green`]), as after [`Workspace::drop_trees`].

use super::*;

/// What [`Workspace::ingest`] takes for a file it does not read: its index entry, and for a workspace app's file the
/// byte ranges its parse added (page expression routines).
pub(super) struct GivenFile {
    pub(super) index: FileIndex,
    pub(super) synthetic: Vec<ByteRange>,
}

impl GivenFile {
    /// The scan of a dependency app's file.
    pub(super) fn scanned(index: FileIndex) -> GivenFile {
        GivenFile {
            index,
            synthetic: Vec::new(),
        }
    }
}

/// One file of a workspace app as its parse found it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredFile {
    /// The path as the app names it, which must match the file of the run.
    pub path: String,
    pub index: FileIndex,
    pub synthetic: Vec<(usize, usize)>,
}

/// A diagnostic with its code as text (`Diagnostic::code` is a static string).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StoredDiagnostic {
    pub file: String,
    pub app: usize,
    pub offset: usize,
    pub code: String,
    pub message: String,
}

impl StoredDiagnostic {
    pub fn of(d: &Diagnostic) -> StoredDiagnostic {
        let Diagnostic {
            file,
            app,
            offset,
            code,
            message,
        } = d;
        StoredDiagnostic {
            file: file.clone(),
            app: *app,
            offset: *offset,
            code: code.to_string(),
            message: message.clone(),
        }
    }

    pub fn to_diagnostic(&self) -> Diagnostic {
        Diagnostic {
            file: self.file.clone(),
            app: self.app,
            offset: self.offset,
            code: intern(&self.code),
            message: self.message.clone(),
        }
    }
}

/// The static string for a diagnostic code read back from a stored compile. Each distinct code is leaked once: the
/// codes are a closed set of short texts, so the memory is bounded however often a process reads an entry.
pub fn intern(code: &str) -> &'static str {
    static CODES: std::sync::Mutex<std::collections::BTreeSet<&'static str>> =
        std::sync::Mutex::new(std::collections::BTreeSet::new());
    let mut codes = CODES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(c) = codes.get(code) {
        return c;
    }
    let leaked: &'static str = Box::leak(code.to_string().into_boxed_str());
    codes.insert(leaked);
    leaked
}

/// What a stored compile keeps of its workspace.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredWorkspace {
    /// Every file of the apps that are not dependencies, in the workspace's file order.
    pub files: Vec<StoredFile>,
    /// The diagnostics of the load (parse errors and unresolved references), in their order.
    pub diagnostics: Vec<StoredDiagnostic>,
    /// What `restrict_to` set, if the compile set it.
    pub compiled: Option<Vec<bool>>,
    /// What `set_unreached` set, if the compile set it, sorted.
    pub unreached: Option<Vec<(usize, usize)>>,
    /// The parse problem of each lazily parsed file the compile parsed and found one in, by file (review HIGH 2: a run
    /// that reads the compile says them as the fresh one did).
    pub problems: Vec<(usize, String)>,
}

impl Workspace {
    /// What a stored compile keeps of this workspace, after the compile (see the module).
    pub fn stored(&self) -> StoredWorkspace {
        let mut by_file: Vec<Vec<&ObjectSym>> = vec![Vec::new(); self.files.len()];
        for o in &self.objects {
            by_file[o.file].push(o);
        }
        let mut dotnet: Vec<Vec<ast::DotNetTypeDecl>> = vec![Vec::new(); self.files.len()];
        for d in &self.dotnet {
            dotnet[d.file].push(d.decl.clone());
        }
        let files = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| self.kinds[f.app] != AppKind::Dependency)
            .map(|(fi, f)| {
                let mut objects = std::mem::take(&mut by_file[fi]);
                objects.sort_by_key(|o| o.ordinal);
                StoredFile {
                    path: f.path.clone(),
                    index: FileIndex {
                        namespace: f.namespace.clone(),
                        usings: f.usings.clone(),
                        objects: objects.into_iter().map(file_object).collect(),
                        dotnet: std::mem::take(&mut dotnet[fi]),
                    },
                    synthetic: f.synthetic.get().cloned().unwrap_or_default(),
                }
            })
            .collect();
        let unreached = self.unreached.get().map(|u| {
            let mut v: Vec<(usize, usize)> = u.iter().copied().collect();
            v.sort_unstable();
            v
        });
        StoredWorkspace {
            files,
            diagnostics: self.diagnostics.iter().map(StoredDiagnostic::of).collect(),
            compiled: self.compiled.get().cloned(),
            unreached,
            problems: self
                .files
                .iter()
                .enumerate()
                .filter(|(_, f)| f.lazy)
                .filter_map(|(fi, f)| Some((fi, f.problem.get()?.clone()?)))
                .collect(),
        }
    }

    /// The workspace of a stored compile: [`load_apps_lazy`](Self::load_apps_lazy) of the same apps, where the files
    /// of the apps that are not dependencies are not parsed (`stored` gives what their parse found) and the references
    /// are not resolved (`stored` gives the diagnostics), with what the compile set on it. An error says `stored` does
    /// not fit the apps, and gives the apps back.
    pub fn load_stored(
        apps: Vec<AppSource>,
        kinds: Vec<AppKind>,
        symbols: &BTreeSet<String>,
        app_symbols: &[Option<BTreeSet<String>>],
        mut lazy: Vec<Option<LazyDep>>,
        stored: StoredWorkspace,
    ) -> Result<Workspace, (Vec<AppSource>, String)> {
        let StoredWorkspace {
            files,
            diagnostics,
            compiled,
            unreached,
            problems,
        } = stored;
        lazy.resize_with(apps.len(), || None);
        let loaders = lazy.iter().map(|l| l.as_ref().map(|l| l.loader.clone())).collect();
        let indexes = match given_files(&apps, &kinds, lazy, files) {
            Ok(i) => i,
            Err(why) => return Err((apps, why)),
        };
        let symbols_of: Vec<BTreeSet<String>> = (0..apps.len())
            .map(|i| app_symbols.get(i).cloned().flatten().unwrap_or_else(|| symbols.clone()))
            .collect();
        let diagnostics = diagnostics.iter().map(StoredDiagnostic::to_diagnostic).collect();
        let mut ws = Workspace::load_given(apps, kinds, symbols, symbols_of, loaders, indexes, Some(diagnostics));
        if let Some(c) = compiled {
            if c.len() != ws.objects.len() {
                let why = "the entry's compiled objects do not fit the workspace".to_string();
                return Err((std::mem::take(&mut ws.apps), why));
            }
            ws.restrict_to(c);
        }
        if let Some(u) = unreached {
            ws.set_unreached(u.into_iter().collect());
        }
        // The parse problems the compile found in lazily parsed files: the run says them (`lazy_parse_problems`), and a
        // later `file_problem` answers them without a parse (a parse would find the same: it is a function of the text).
        for (fi, why) in problems {
            match ws.files.get(fi).filter(|f| f.lazy) {
                Some(f) => {
                    let _ = f.problem.set(Some(why));
                }
                None => {
                    return Err((
                        std::mem::take(&mut ws.apps),
                        "a parse problem of no lazily parsed file".into(),
                    ));
                }
            }
        }
        Ok(ws)
    }
}

/// What [`Workspace::load_stored`] gives `ingest` per app: a lazy app's scan, a workspace app's stored files (each must
/// be the app's file of the same path, in order), nothing for a dependency app read with its text.
fn given_files(
    apps: &[AppSource],
    kinds: &[AppKind],
    lazy: Vec<Option<LazyDep>>,
    files: Vec<StoredFile>,
) -> Result<Vec<Option<Vec<GivenFile>>>, String> {
    if apps.len() != kinds.len() {
        return Err("one kind per app".into());
    }
    let mut files = files.into_iter();
    let mut indexes: Vec<Option<Vec<GivenFile>>> = Vec::with_capacity(apps.len());
    for (i, l) in lazy.into_iter().enumerate() {
        if let Some(l) = l {
            if l.index.len() != apps[i].files.len() || kinds[i] != AppKind::Dependency {
                return Err("a lazy app's index does not fit it".into());
            }
            indexes.push(Some(l.index.into_iter().map(GivenFile::scanned).collect()));
            continue;
        }
        if kinds[i] == AppKind::Dependency {
            indexes.push(None);
            continue;
        }
        let mut given = Vec::with_capacity(apps[i].files.len());
        for (path, _) in &apps[i].files {
            let f = files
                .next()
                .ok_or("the entry holds fewer workspace files than the apps")?;
            if f.path != *path {
                return Err(format!("the entry holds file {}, the app {path}", f.path));
            }
            given.push(GivenFile {
                index: f.index,
                synthetic: f.synthetic,
            });
        }
        indexes.push(Some(given));
    }
    if files.next().is_some() {
        return Err("the entry holds more workspace files than the apps".into());
    }
    Ok(indexes)
}

/// The index entry of an object of a parsed file.
fn file_object(o: &ObjectSym) -> FileObject {
    let ObjectSym {
        app: _,
        file: _,
        kind,
        id,
        name,
        namespace: _,
        offset,
        ordinal: _,
        extends,
        implements,
        subscribers,
        scanned_properties,
        scanned_layouts,
        install,
        manual_subscribers,
    } = o;
    FileObject {
        kind: *kind,
        id: *id,
        name: name.clone(),
        offset: *offset,
        extends: extends.clone(),
        implements: implements.clone(),
        subscribers: subscribers.clone(),
        properties: scanned_properties.clone(),
        layouts: scanned_layouts.clone(),
        install: *install,
        manual_subscribers: *manual_subscribers,
    }
}
