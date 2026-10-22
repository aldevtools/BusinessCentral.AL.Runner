//! An `.app` package: its manifest and its AL source entries.

use crate::select::DepDecl;
use crate::version::Version;
use crate::zip::{Zip, ZipError, navx_payload};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub version: Version,
    /// `Platform` and `Application` floors, when the manifest has them.
    pub platform: Option<Version>,
    pub application: Option<Version>,
    pub target: String,
    pub dependencies: Vec<DepDecl>,
    /// `preprocessorSymbols` of the app's `app.json`, when the package is a source archive or
    /// folder. A compiled package carries none: its publisher compiled it with its own.
    pub preprocessor_symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    Zip(ZipError),
    /// No `NavxManifest.xml`.
    NoManifest,
    Manifest(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageError::Zip(e) => write!(f, "{e}"),
            PackageError::NoManifest => write!(f, "the package has no NavxManifest.xml"),
            PackageError::Manifest(m) => write!(f, "NavxManifest.xml: {m}"),
        }
    }
}

impl std::error::Error for PackageError {}

impl From<ZipError> for PackageError {
    fn from(e: ZipError) -> PackageError {
        PackageError::Zip(e)
    }
}

/// The AL source of a package: (path inside the package, text), sorted by path.
pub type Sources = Vec<(String, String)>;

/// Parses `NavxManifest.xml`.
pub fn parse_manifest(xml: &str) -> Result<Manifest, PackageError> {
    use quick_xml::events::{BytesStart, Event};
    let bad = |m: String| PackageError::Manifest(m);
    let attrs = |e: &BytesStart| -> Result<Vec<(String, String)>, PackageError> {
        e.attributes()
            .map(|a| {
                let a = a.map_err(|err| bad(err.to_string()))?;
                let v = a.unescape_value().map_err(|err| bad(err.to_string()))?;
                Ok((String::from_utf8_lossy(a.key.as_ref()).into_owned(), v.into_owned()))
            })
            .collect()
    };
    let get = |a: &[(String, String)], k: &str| a.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let version = |a: &[(String, String)], k: &str| -> Result<Option<Version>, PackageError> {
        get(a, k)
            .filter(|v| !v.is_empty())
            .map(|v| v.parse().map_err(bad))
            .transpose()
    };
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut manifest: Option<Manifest> = None;
    let mut in_dependencies = false;
    let mut dependencies = Vec::new();
    loop {
        match reader.read_event() {
            Err(e) => return Err(bad(e.to_string())),
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.name().as_ref() {
                b"App" => {
                    let a = attrs(&e)?;
                    manifest = Some(Manifest {
                        id: get(&a, "Id").unwrap_or_default().to_ascii_lowercase(),
                        name: get(&a, "Name").unwrap_or_default(),
                        publisher: get(&a, "Publisher").unwrap_or_default(),
                        version: version(&a, "Version")?.unwrap_or_default(),
                        platform: version(&a, "Platform")?,
                        application: version(&a, "Application")?,
                        target: get(&a, "Target").unwrap_or_default(),
                        dependencies: Vec::new(),
                        preprocessor_symbols: Vec::new(),
                    });
                }
                b"Dependencies" => in_dependencies = true,
                b"Dependency" if in_dependencies => {
                    let a = attrs(&e)?;
                    dependencies.push(DepDecl {
                        id: get(&a, "Id").unwrap_or_default().to_ascii_lowercase(),
                        name: get(&a, "Name").unwrap_or_default(),
                        publisher: get(&a, "Publisher").unwrap_or_default(),
                        min: version(&a, "MinVersion")?.unwrap_or_default(),
                        optional: false,
                    });
                }
                _ => {}
            },
            Ok(Event::End(e)) if e.name().as_ref() == b"Dependencies" => in_dependencies = false,
            Ok(_) => {}
        }
    }
    let mut m = manifest.ok_or_else(|| bad("no <App> element".into()))?;
    m.dependencies = dependencies;
    Ok(m)
}

/// How deep a wrapper package may nest an inner `.app`.
const MAX_NESTING: usize = 2;

pub(crate) fn is_al(name: &str) -> bool {
    name.len() > 3 && name[name.len() - 3..].eq_ignore_ascii_case(".al")
}

pub(crate) fn is_app(name: &str) -> bool {
    name.len() > 4 && name[name.len() - 4..].eq_ignore_ascii_case(".app")
}

/// Microsoft's build variables, which the source archives of Base Application and Application hold
/// in `app.json` where the compiled package holds numbers (`"version": "$(app_currentVersion)"`).
///
/// The values are derived from the build the archive came from. Evidence: the compiled Base
/// Application 27.5.46862.48827 in the corpus's `.alpackages` has `Version="27.5.46862.48827"`,
/// `Platform="27.0.0.0"` and `MinVersion="27.5.0.0"` on its Microsoft dependencies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vars {
    current: Option<String>,
    minimum: Option<String>,
    platform: Option<String>,
}

impl Vars {
    /// The variables of a build `major.minor.build.revision`; `None` when it is not one.
    pub fn for_build(build: &str) -> Option<Vars> {
        let parts: Vec<&str> = build.trim().split('.').collect();
        let numbers = parts.len() == 4
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
        numbers.then(|| Vars {
            current: Some(build.trim().to_string()),
            minimum: Some(format!("{}.{}.0.0", parts[0], parts[1])),
            platform: Some(format!("{}.0.0.0", parts[0])),
        })
    }

    fn apply(&self, text: &str) -> String {
        let mut out = text.to_string();
        for (name, value) in [
            ("$(app_currentVersion)", &self.current),
            ("$(app_minimumVersion)", &self.minimum),
            ("$(app_platformVersion)", &self.platform),
        ] {
            if let Some(v) = value {
                out = out.replace(name, v);
            }
        }
        out
    }
}

/// A source archive (`<App>.Source.zip` in the platform artifact) has no `NavxManifest.xml`; its
/// root `app.json` describes the whole app, and every `.al` file under any folder belongs to it.
/// Module folders inside carry an `app.json` of their own, which does not describe a separate app.
pub(crate) fn manifest_of_source_zip(app_json: &str, vars: &Vars) -> Result<Manifest, PackageError> {
    let text = app_json.strip_prefix('\u{feff}').unwrap_or(app_json);
    let text = vars.apply(text);
    let text = text.as_str();
    if let Some(at) = text.find("$(app_") {
        let var: String = text[at..].chars().take_while(|c| *c != ')').chain([')']).collect();
        return Err(PackageError::Manifest(format!(
            "app.json holds Microsoft's build variable `{var}`, which only the build it came from can fill; \
             keep the archive under a folder named for its build (four numbers, as the artifact cache names it), beside a `<file>.json` record naming the build, or pass the exact build as `--bc-version`"
        )));
    }
    let j = crate::load::parse_app_json(text).map_err(PackageError::Manifest)?;
    let target = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| v["target"].as_str().map(str::to_string))
        .unwrap_or_default();
    Ok(Manifest {
        id: j.id,
        name: j.name,
        publisher: j.publisher,
        version: j.version,
        platform: j.platform,
        application: j.application,
        target,
        dependencies: j.dependencies,
        preprocessor_symbols: serde_json::from_str::<serde_json::Value>(text)
            .ok()
            .map(|v| {
                v["preprocessorSymbols"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|s| s.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

/// The package whose archive holds the source: the one given, or the `.app`
/// nested in a precompiled wrapper that holds none itself. `source_zip` tells the caller whether
/// the archive was a source archive (its names are not encoded).
fn with_archive<T>(
    app: &[u8],
    depth: usize,
    vars: &Vars,
    f: &mut dyn FnMut(&Zip<'_>, Manifest, bool) -> Result<T, PackageError>,
) -> Result<T, PackageError> {
    let zip = Zip::parse(navx_payload(app)?)?;
    if let Some(m) = zip.find("NavxManifest.xml") {
        let manifest = parse_manifest(&String::from_utf8_lossy(&zip.read(m)?))?;
        if depth < MAX_NESTING && !zip.entries.iter().any(|e| is_al(&e.name)) {
            if let Some(inner) = zip.entries.iter().find(|e| is_app(&e.name)) {
                let bytes = zip.read(inner)?;
                return with_archive(&bytes, depth + 1, vars, f);
            }
        }
        return f(&zip, manifest, false);
    }
    // A Ready-to-Run wrapper (an artifact's Extensions folder) has no manifest of its own: `readytorunappmanifest.json`
    // and the one real app inside.
    if depth < MAX_NESTING && zip.find("readytorunappmanifest.json").is_some() && zip.find("app.json").is_none() {
        let mut apps = zip.entries.iter().filter(|e| is_app(&e.name));
        if let (Some(inner), None) = (apps.next(), apps.next()) {
            let bytes = zip.read(inner)?;
            return with_archive(&bytes, depth + 1, vars, f);
        }
    }
    let root = zip.find("app.json").ok_or(PackageError::NoManifest)?;
    let manifest = manifest_of_source_zip(&String::from_utf8_lossy(&zip.read(root)?), vars)?;
    f(&zip, manifest, true)
}

/// Reads only the manifest and whether the package has AL source.
pub fn read_info(app: &[u8]) -> Result<(Manifest, bool), PackageError> {
    read_info_with(app, &Vars::default())
}

/// [`read_info`] for a source archive that may hold build variables.
pub fn read_info_with(app: &[u8], vars: &Vars) -> Result<(Manifest, bool), PackageError> {
    with_archive(app, 0, vars, &mut |zip, m, _| {
        Ok((m, zip.entries.iter().any(|e| is_al(&e.name))))
    })
}

/// Reads the manifest and every `.al` entry, each checked against its CRC-32.
pub fn read_package(app: &[u8]) -> Result<(Manifest, Sources), PackageError> {
    read_package_with(app, &Vars::default())
}

/// [`read_package`] for a source archive that may hold build variables.
pub fn read_package_with(app: &[u8], vars: &Vars) -> Result<(Manifest, Sources), PackageError> {
    use rayon::prelude::*;
    with_archive(app, 0, vars, &mut |zip, m, source_zip| {
        let al: Vec<&crate::zip::Entry> = zip.entries.iter().filter(|e| is_al(&e.name)).collect();
        let read: Vec<Result<(String, String), PackageError>> = al
            .par_iter()
            .map(|e| {
                let bytes = zip.read(e)?;
                // The decoded bytes become the text without a copy when they are valid UTF-8.
                let mut text = match String::from_utf8(bytes) {
                    Ok(text) => text,
                    Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
                };
                if text.starts_with('\u{feff}') {
                    text.drain(..'\u{feff}'.len_utf8());
                }
                let name = if source_zip {
                    e.name.clone()
                } else {
                    percent_decode_once(&e.name)
                };
                Ok((name, text))
            })
            .collect();
        let mut sources = read.into_iter().collect::<Result<Sources, _>>()?;
        sources.sort();
        Ok((m, sources))
    })
}

/// The packaged resources of a package: `(name relative to the resource folder, content)`, sorted by name.
///
/// A compiled `.app` holds them under `resources/`, with the entry names percent-encoded like every other
/// entry (`Fabrikam%20Invoice.PDF`); the names are decoded once, as for the `.al` entries. A source
/// archive holds them in the folders its `app.json` names in `resourceFolders`, under their real names.
pub fn read_resources(app: &[u8], vars: &Vars) -> Result<Vec<(String, Vec<u8>)>, PackageError> {
    with_archive(app, 0, vars, &mut |zip, _, source_zip| {
        let mut wanted: Vec<(String, &crate::zip::Entry)> = Vec::new();
        if source_zip {
            let json = zip
                .find("app.json")
                .map(|e| zip.read(e))
                .transpose()?
                .unwrap_or_default();
            let text = String::from_utf8_lossy(&json);
            let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
            let folders: Vec<String> = serde_json::from_str::<serde_json::Value>(text)
                .ok()
                .map(|v| {
                    v["resourceFolders"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|f| f.as_str())
                        .map(|f| f.trim_start_matches("./").trim_end_matches('/').to_string())
                        .filter(|f| !f.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            for e in &zip.entries {
                for f in &folders {
                    if let Some(rel) = e.name.strip_prefix(&format!("{f}/")) {
                        if !rel.is_empty() && !rel.ends_with('/') {
                            wanted.push((rel.to_string(), e));
                        }
                    }
                }
            }
        } else {
            for e in &zip.entries {
                if let Some(rel) = e.name.strip_prefix("resources/") {
                    if !rel.is_empty() && !rel.ends_with('/') {
                        wanted.push((percent_decode_once(rel), e));
                    }
                }
            }
        }
        let mut out = Vec::with_capacity(wanted.len());
        for (name, e) in wanted {
            out.push((name, zip.read(e)?));
        }
        out.sort();
        Ok(out)
    })
}

/// `%20` to a space, once. Entry names in packages are encoded twice by the packer.
pub(crate) fn percent_decode_once(name: &str) -> String {
    let b = name.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = |c: u8| (c as char).to_digit(16);
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::testutil::{Builder, navx};

    const MANIFEST: &str = r#"<Package xmlns="http://schemas.microsoft.com/navx/2015/manifest">
  <App Id="437DBF0E-84FF-417A-965D-ED2BB9650972" Name="Base &amp; Co" Publisher="Microsoft" Version="27.5.46862.48827" Platform="27.0.0.0" Application="27.5.0.0" Target="OnPrem" />
  <IdRanges><IdRange MinObjectId="1" MaxObjectId="49999" /></IdRanges>
  <Dependencies>
    <Dependency Id="63ca2fa4-4f03-4f2b-a480-172fef340d3f" Name="System Application" Publisher="Microsoft" MinVersion="27.5.0.0" CompatibilityId="0.0.0.0" />
    <Dependency Id="f3552374-a1f2-4356-848e-196002525837" Name="Business Foundation" Publisher="Microsoft" MinVersion="27.5.0.0" />
  </Dependencies>
  <InternalsVisibleTo><Module Id="6992416f-3f39-4d3c-8242-3fff61350bea" Name="Other" Publisher="Microsoft" /></InternalsVisibleTo>
</Package>"#;

    fn app_with(files: &[(&str, &str)]) -> Vec<u8> {
        let mut b = Builder::new().add("NavxManifest.xml", MANIFEST.as_bytes(), true);
        for (n, t) in files {
            b = b.add(n, t.as_bytes(), true);
        }
        navx(&b.finish())
    }

    #[test]
    fn the_manifest_gives_identity_floors_and_dependencies() {
        let m = parse_manifest(MANIFEST).unwrap();
        assert_eq!(m.id, "437dbf0e-84ff-417a-965d-ed2bb9650972");
        assert_eq!(m.name, "Base & Co");
        assert_eq!(m.publisher, "Microsoft");
        assert_eq!(m.version, "27.5.46862.48827".parse().unwrap());
        assert_eq!(m.platform, Some("27.0".parse().unwrap()));
        assert_eq!(m.application, Some("27.5".parse().unwrap()));
        assert_eq!(m.target, "OnPrem");
        assert_eq!(
            m.dependencies
                .iter()
                .map(|d| (d.name.as_str(), d.min.to_string()))
                .collect::<Vec<_>>(),
            [
                ("System Application", "27.5.0.0".to_string()),
                ("Business Foundation", "27.5.0.0".to_string())
            ]
        );
        // Modules that may see internals are not dependencies.
        assert!(m.dependencies.iter().all(|d| d.name != "Other"));
    }

    #[test]
    fn a_manifest_without_an_app_element_is_an_error() {
        assert!(matches!(parse_manifest("<Package/>"), Err(PackageError::Manifest(_))));
        assert!(matches!(parse_manifest("not xml <"), Err(PackageError::Manifest(_))));
    }

    #[test]
    fn sources_are_the_al_entries_sorted_by_path() {
        let app = app_with(&[
            ("src/B.Codeunit.al", "codeunit 2 B { }"),
            ("src/A.Codeunit.al", "codeunit 1 A { }"),
            ("SymbolReference.json", "{}"),
            ("src/readme.txt", "x"),
        ]);
        let (m, src) = read_package(&app).unwrap();
        assert_eq!(m.name, "Base & Co");
        assert_eq!(
            src,
            [
                ("src/A.Codeunit.al".to_string(), "codeunit 1 A { }".to_string()),
                ("src/B.Codeunit.al".to_string(), "codeunit 2 B { }".to_string())
            ]
        );
    }

    #[test]
    fn entry_names_are_percent_decoded_once_and_a_byte_order_mark_is_dropped() {
        let app = app_with(&[("src/Azure%2520Function/F.al", "\u{feff}codeunit 1 F { }")]);
        let (_, src) = read_package(&app).unwrap();
        assert_eq!(src[0].0, "src/Azure%20Function/F.al");
        assert_eq!(src[0].1, "codeunit 1 F { }");
    }

    #[test]
    fn resources_of_a_compiled_package_are_read_with_their_names_decoded() {
        let app = app_with(&[
            ("src/A.al", "codeunit 1 A { }"),
            ("resources/Images/Item/1896-S.jpg", "jpg"),
            ("resources/IncomingDocuments/Fabrikam%20Invoice.PDF", "pdf"),
            ("resources/", ""),
            ("Translations/x.xlf", "not a resource"),
        ]);
        let r = read_resources(&app, &Vars::default()).unwrap();
        assert_eq!(
            r,
            [
                ("Images/Item/1896-S.jpg".to_string(), b"jpg".to_vec()),
                ("IncomingDocuments/Fabrikam Invoice.PDF".to_string(), b"pdf".to_vec())
            ]
        );
        let none = app_with(&[("src/A.al", "codeunit 1 A { }")]);
        assert!(read_resources(&none, &Vars::default()).unwrap().is_empty());
    }

    #[test]
    fn resources_of_a_source_archive_come_from_its_resource_folders() {
        let zip = Builder::new()
            .add(
                "app.json",
                br#"{"id":"aa","name":"S","publisher":"P","version":"1.0.0.0","resourceFolders":[".resources"]}"#,
                true,
            )
            .add("src/A.al", b"codeunit 1 A { }", true)
            .add(".resources/CRONUS.jpg", b"jpg", true)
            .add(".resources/sub/a b.txt", b"t", true)
            .add("other/x.txt", b"no", true)
            .finish();
        let r = read_resources(&zip, &Vars::default()).unwrap();
        assert_eq!(
            r,
            [
                ("CRONUS.jpg".to_string(), b"jpg".to_vec()),
                ("sub/a b.txt".to_string(), b"t".to_vec())
            ]
        );
    }

    #[test]
    fn read_info_reports_whether_there_is_source_without_reading_it() {
        let (m, has) = read_info(&app_with(&[("src/A.al", "codeunit 1 A { }")])).unwrap();
        assert_eq!(m.version.0, 27);
        assert!(has);
        let (_, has) = read_info(&app_with(&[("SymbolReference.json", "{}")])).unwrap();
        assert!(!has);
    }

    /// A Ready-to-Run wrapper in an artifact's Extensions folder has no NavxManifest.xml of its own: only
    /// `readytorunappmanifest.json` and the real app (al-runner issue 5710). The fixture is made here, not copied from one.
    #[test]
    fn a_ready_to_run_wrapper_without_a_manifest_is_unwrapped() {
        let inner = app_with(&[("src/a.al", "codeunit 50100 A { }")]);
        let wrapper = navx(
            &Builder::new()
                .add("readytorunappmanifest.json", b"{}", true)
                .add("Microsoft_Base Application_27.5.app", &inner, false)
                .finish(),
        );
        let (m, src) = read_package(&wrapper).unwrap();
        assert_eq!(m.name, "Base & Co");
        assert_eq!(src.len(), 1);
        assert!(read_info(&wrapper).unwrap().1);
        // Without the Ready-to-Run manifest, or with two apps inside, it stays an error.
        let bare = navx(&Builder::new().add("x.app", &inner, false).finish());
        assert_eq!(read_package(&bare).unwrap_err(), PackageError::NoManifest);
        let two = navx(
            &Builder::new()
                .add("readytorunappmanifest.json", b"{}", true)
                .add("a.app", &inner, false)
                .add("b.app", &inner, false)
                .finish(),
        );
        assert_eq!(read_package(&two).unwrap_err(), PackageError::NoManifest);
    }

    #[test]
    fn a_wrapper_package_around_an_inner_app_is_unwrapped() {
        // Precompiled packages hold the real `.app` as an entry and carry no source themselves.
        let inner = app_with(&[("src/A.al", "codeunit 1 A { }")]);
        let outer = navx(
            &Builder::new()
                .add("NavxManifest.xml", MANIFEST.as_bytes(), true)
                .add("Microsoft_Base Application.app", &inner, false)
                .finish(),
        );
        let (_, has) = read_info(&outer).unwrap();
        assert!(has);
        let (_, src) = read_package(&outer).unwrap();
        assert_eq!(src.len(), 1);
    }

    #[test]
    fn a_package_with_a_bad_crc_is_an_error_not_partial_source() {
        let zip = Builder::new()
            .add("NavxManifest.xml", MANIFEST.as_bytes(), true)
            .add_with_crc("src/A.al", b"codeunit 1 A { }", 1)
            .finish();
        let err = read_package(&navx(&zip)).unwrap_err();
        assert!(matches!(err, PackageError::Zip(ZipError::Crc { .. })), "{err:?}");
    }

    #[test]
    fn a_package_without_a_manifest_is_an_error() {
        let zip = Builder::new().add("src/A.al", b"codeunit 1 A { }", false).finish();
        assert_eq!(read_package(&navx(&zip)).unwrap_err(), PackageError::NoManifest);
    }

    const APP_JSON: &str = r#"{
  "id": "9856AE4F-D1A7-46EF-89BB-6EF056398228",
  "name": "System Application Test Library",
  "publisher": "Microsoft",
  "version": "27.5.46862.56249",
  "platform": "27.0.0.0",
  "application": "27.5.0.0",
  "target": "OnPrem",
  "dependencies": [
    { "id": "63ca2fa4-4f03-4f2b-a480-172fef340d3f", "name": "System Application", "publisher": "Microsoft", "version": "27.5.0.0" },
    { "id": "e7320ebb-08b3-4406-b1ec-b4927d3e280b", "name": "Any", "publisher": "Microsoft", "version": "27.5.0.0" }
  ]
}"#;

    /// A repository-layout archive as the platform artifact ships it: an `app.json` at the root, and
    /// module folders that carry an `app.json` of their own.
    fn source_zip(files: &[(&str, &str)]) -> Vec<u8> {
        let mut b = Builder::new().add("app.json", format!("\u{feff}{APP_JSON}").as_bytes(), true);
        for (n, t) in files {
            b = b.add(n, t.as_bytes(), true);
        }
        b.finish()
    }

    #[test]
    fn a_source_zip_is_one_app_described_by_its_root_app_json() {
        let z = source_zip(&[
            (
                "Azure AD Plan/app.json",
                r#"{"id":"e63749f1","name":"Azure AD Plan Test Library"}"#,
            ),
            (
                "Azure AD Plan/Plan.Codeunit.al",
                "codeunit 132916 \"Azure AD Plan Test Library\" { }",
            ),
            ("src/B.Codeunit.al", "codeunit 2 B { }"),
            ("Translations/x.g.lcg", "<x/>"),
            ("Directory.App.Props.json", "{}"),
        ]);
        let (m, src) = read_package(&z).unwrap();
        assert_eq!(m.id, "9856ae4f-d1a7-46ef-89bb-6ef056398228");
        assert_eq!(m.name, "System Application Test Library");
        assert_eq!(m.publisher, "Microsoft");
        assert_eq!(m.version, "27.5.46862.56249".parse().unwrap());
        assert_eq!(m.platform, Some("27.0".parse().unwrap()));
        assert_eq!(m.target, "OnPrem");
        assert_eq!(
            m.dependencies.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
            ["System Application", "Any"]
        );
        // Every .al file of every module folder belongs to the one app; the nested app.json do not.
        assert_eq!(
            src.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
            ["Azure AD Plan/Plan.Codeunit.al", "src/B.Codeunit.al"]
        );
        let (info, has) = read_info(&z).unwrap();
        assert_eq!(info, m);
        assert!(has);
    }

    #[test]
    fn source_zip_names_are_not_percent_decoded() {
        // A packed .app encodes names twice; a source archive holds them as they are.
        let z = source_zip(&[("100%25 Mock/M.al", "codeunit 1 M { }")]);
        let (_, src) = read_package(&z).unwrap();
        assert_eq!(src[0].0, "100%25 Mock/M.al");
    }

    #[test]
    fn a_zip_with_neither_manifest_is_an_error_and_a_nested_app_json_is_not_the_root() {
        let only_nested = Builder::new()
            .add("Module/app.json", APP_JSON.as_bytes(), true)
            .finish();
        assert_eq!(read_package(&only_nested).unwrap_err(), PackageError::NoManifest);
        let bad = Builder::new().add("app.json", b"{ nope", true).finish();
        assert!(matches!(read_package(&bad), Err(PackageError::Manifest(_))));
    }

    #[test]
    fn a_source_zip_with_a_bad_crc_is_an_error_not_partial_source() {
        let z = Builder::new()
            .add("app.json", APP_JSON.as_bytes(), true)
            .add_with_crc("src/A.al", b"codeunit 1 A { }", 1)
            .finish();
        assert!(matches!(
            read_package(&z).unwrap_err(),
            PackageError::Zip(ZipError::Crc { .. })
        ));
    }

    /// Base Application's and Application's source archives hold Microsoft's build variables where
    /// the compiled package holds numbers.
    const TEMPLATED: &str = r#"{
  "id": "437dbf0e-84ff-417a-965d-ed2bb9650972", "name": "Base Application", "publisher": "Microsoft",
  "version": "$(app_currentVersion)", "platform": "$(app_platformVersion)", "application": "$(app_minimumVersion)",
  "dependencies": [ { "id": "63ca2fa4-4f03-4f2b-a480-172fef340d3f", "name": "System Application", "publisher": "Microsoft", "version": "$(app_minimumVersion)" } ]
}"#;

    #[test]
    fn build_variables_in_a_source_zip_are_filled_from_the_build_it_came_from() {
        // Evidence: the compiled Base Application 27.5.46862.48827 in the corpus's .alpackages has
        // Version 27.5.46862.48827, Platform 27.0.0.0 and dependency MinVersion 27.5.0.0.
        let z = Builder::new()
            .add("app.json", TEMPLATED.as_bytes(), true)
            .add("src/A.al", b"codeunit 1 A { }", true)
            .finish();
        let vars = Vars::for_build("27.5.46862.56249").unwrap();
        let (m, src) = read_package_with(&z, &vars).unwrap();
        assert_eq!(m.version, "27.5.46862.56249".parse().unwrap());
        assert_eq!(m.platform, Some("27.0.0.0".parse().unwrap()));
        assert_eq!(m.application, Some("27.5.0.0".parse().unwrap()));
        assert_eq!(m.dependencies[0].min, "27.5.0.0".parse().unwrap());
        assert_eq!(src.len(), 1);
        assert_eq!(read_info_with(&z, &vars).unwrap().0, m);
    }

    #[test]
    fn a_build_variable_without_a_build_is_an_error_naming_it() {
        let z = Builder::new().add("app.json", TEMPLATED.as_bytes(), true).finish();
        let e = read_info(&z).unwrap_err().to_string();
        assert!(e.contains("$(app_currentVersion)"), "{e}");
        assert!(Vars::for_build("27.5").is_none(), "a build has four parts");
    }

    #[test]
    fn a_source_archive_reports_the_preprocessor_symbols_of_its_own_app_json() {
        let with = r#"{"id":"zz","name":"Zed","publisher":"P","version":"2.0.0.0","dependencies":[],"preprocessorSymbols":["BC27","MINE"]}"#;
        let m = manifest_of_source_zip(with, &Vars::default()).unwrap();
        assert_eq!(m.preprocessor_symbols, vec!["BC27".to_string(), "MINE".to_string()]);
        let without = r#"{"id":"zz","name":"Zed","publisher":"P","version":"2.0.0.0","dependencies":[]}"#;
        assert!(
            manifest_of_source_zip(without, &Vars::default())
                .unwrap()
                .preprocessor_symbols
                .is_empty()
        );
    }
}
