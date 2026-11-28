//! The tables that list the apps of the tenant: NAV App Installed App and Published Application.
//! Their rows are the apps of the program, which is every app the run loaded.
//!
//! Pinned by corpus record/TestNavAppInstalledAppSysTable.al (a row per app keyed by its id, with the
//! manifest's name, publisher and version, a package id that is not blank and not shared between
//! apps). The package id is `ids::package_id`, which AllObj reports for the app's objects too, so the
//! ownership check of the retention policy module compares equal for an app and its own tables
//! (install-upgrade/TestRetenPolAllowedTables_Tests.al).

use super::mat::{Entry, Mat, SharedMat};
use super::{Providers, blank, ids, pos};
use al_builtins::value::Value;
use al_data::TableDef;
use std::sync::{Arc, Mutex};

fn version(v: &str) -> Option<[i32; 4]> {
    let parts: Vec<i32> = v.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    parts.try_into().ok()
}

/// How BC published an app: what "Published As" and "Tenant ID" of its rows follow.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    /// Through the development endpoint, as a per-tenant app: Published As Dev (2), Tenant ID `TenantId()`.
    Dev,
    /// As a global app: Published As Global (0), Tenant ID blank.
    Global,
}

/// The route of app `a` and the assumption the answer rests on, if any. One rule for NAV App Installed App and
/// Published Application:
/// - an app under test, as the corpus apps through the development endpoint, is Dev; Microsoft's apps are Global
///   (record/table-properties/Y12PublishedAs.al:31-80, al-runner issue 6438; onprem
///   metadata/platform-tables/AwOnPremTests.al:137-139 `tenant= as=Global` for Base Application, `tenant=default as= Dev`
///   for the two corpus apps; onprem session-system/PZPublishedApplicationColumns.al:96-109);
/// - a dependency app of another publisher is Dev as well, as one published through the development endpoint
///   (record/lt-r7142-installed-app/R7169DepInstallPublishedAs.al:70, al-runner issue 7169: the install codeunit of XApp A
///   reads it there; AwOnPremTests.al:138, the coverage app the onprem app depends on);
/// - `--dependencies-published-global` makes such a dependency Global, for a pipeline that publishes dependencies as
///   compiled apps (a documented setting: assumption `dependency-published-as-global`).
fn route(a: &al_ir::AppInfo, dependencies_global: bool) -> (Route, Option<al_builtins::assumptions::Assumption>) {
    if !a.dependency {
        (Route::Dev, None)
    } else if a.publisher.eq_ignore_ascii_case("Microsoft") {
        (Route::Global, None)
    } else if dependencies_global {
        (
            Route::Global,
            Some(al_builtins::assumptions::Assumption::DependencyPublishedAsGlobal),
        )
    } else {
        (Route::Dev, None)
    }
}

impl Route {
    /// The ordinal of "Published As" (Global, PTE, Dev).
    fn published_as(self) -> i32 {
        match self {
            Route::Dev => 2,
            Route::Global => 0,
        }
    }
}

/// NAV App Installed App: "App ID" (1), "Package ID" (2), Name (3), Publisher (4), version (5 to 8), compatibility (9 to 12),
/// "Hash Algorithm" (14), "Extension Type" (15), "Published As" (16).
pub(super) fn nav_app_installed_app_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let view = p.install_view();
    let global = p.engine().dependencies_published_global;
    p.engine()
        .program
        .apps
        .iter()
        .enumerate()
        // While an app installs, the apps its install has not reached and its own app have no row
        // (record/lt-r7142-installed-app/R7169DepInstallPublishedAs.al:36-45 `install_own` = `no row`, the install-time filter counts).
        .filter(|(i, _)| !view.not_installed.contains(&(*i as u32)) && view.installing != Some(*i as u32))
        .map(|(i, a)| (view.order_unknown.contains(&(i as u32)), a))
        // The platform's System app has no row (record/table-properties/Y12PublishedAs.al:41, :46 `no row`).
        .filter(|(_, a)| !a.id.eq_ignore_ascii_case(super::objects::al_sema_platform_app_id()))
        .filter_map(|(order_unknown, a)| {
            let id = al_builtins::ops::parse_guid(&a.id)?;
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Guid(id);
            r[pos(t, 2)] = Value::Guid(ids::package_id(id));
            r[pos(t, 3)] = Value::Text(a.name.clone().into());
            r[pos(t, 4)] = Value::Text(a.publisher.clone().into());
            let v = version(&a.version)?;
            for (n, part) in v.iter().enumerate() {
                r[pos(t, 5 + n as u32)] = Value::Integer(*part);
            }
            // Compatibility Major to Revision are 0, Hash Algorithm 1 and Extension Type 2, for the apps under test and for
            // Microsoft's alike; the compatibility parts do not follow the version (System Application and Base Application,
            // whose version parts are not 0). Pinned on the 27.5, 28.5 and 29.0 legs by
            // record/lt-r7142-installed-app/R7142InstalledAppCompat.al:25-63, :113-123 (corpus PR 1156; al-runner issue 7142).
            for (c, v) in [(9, 0), (10, 0), (11, 0), (12, 0), (14, 1), (15, 2)] {
                if let Some(i) = t.fields.iter().position(|f| f.id == c) {
                    r[i] = Value::Integer(v);
                }
            }
            // Content Hash (13) is not blank (R7142InstalledAppCompat.al:121 `content hash blank` = No), but its value is not
            // pinned: unknown, so a read is refused. The column counts as pinned since the pin reads it, so the cell itself
            // must refuse.
            if t.fields.iter().any(|f| f.id == 13) {
                r[pos(t, 13)] = super::unknown(t, 13, &format!("app '{}'", a.name));
            }
            // "Published As" follows how the app was published ([`route`]).
            let (route, assumed) = route(a, global);
            r[pos(t, 16)] = super::option(t, 16, route.published_as());
            if order_unknown {
                let why = format!(
                    "app '{}': whether BC has installed it while the app whose install runs installs is not known (no dependency path between them)",
                    a.name
                );
                return Some(Entry::poisoned(r, why));
            }
            let entry = Entry::ok(r);
            Some(match assumed {
                Some(x) => entry.assuming(x),
                None => entry,
            })
        })
        .collect()
}

/// Published Application: "Runtime Package ID" (1), "Package ID" (2), ID (3), Name (4), Publisher (5),
/// version (6 to 9). The two package ids of an app are one value (AL Runner measured that on a real tier
/// for an app published through the development endpoint). "Published As" (11) and "Tenant ID" (12) follow how the app was published ([`route`]): Dev and
/// `TenantId()`, or Global and a blank. Either passes `SetFilter("Tenant ID", '%1|%2', '', TenantId)`, which
/// install-upgrade/TestRetenPolAllowedTables_Tests.al needs of the row of the app that registers a table.
pub(super) fn published_application_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let global = p.engine().dependencies_published_global;
    let mut rows = p
        .engine()
        .program
        .apps
        .iter()
        .filter_map(|a| {
            let id = al_builtins::ops::parse_guid(&a.id)?;
            let pkg = ids::package_id(id);
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Guid(ids::runtime_package_id(id, a.dependency));
            r[pos(t, 2)] = Value::Guid(pkg);
            r[pos(t, 3)] = Value::Guid(id);
            r[pos(t, 4)] = Value::Text(a.name.clone().into());
            r[pos(t, 5)] = Value::Text(a.publisher.clone().into());
            let v = version(&a.version)?;
            for (n, part) in v.iter().enumerate() {
                r[pos(t, 6 + n as u32)] = Value::Integer(*part);
            }
            // "Published As" (11) and "Tenant ID" (12) follow how the app was published ([`route`]): a per-tenant app
            // carries `TenantId()`, a global one a blank.
            let (route, assumed) = route(a, global);
            r[pos(t, 11)] = super::option(t, 11, route.published_as());
            r[pos(t, 12)] = Value::Text(match route {
                Route::Dev => p.ident.tenant_id.clone().into(),
                Route::Global => String::new().into(),
            });
            // Description holds the manifest's description as UTF-8 text (onprem session-system/PZPublishedApplicationColumns.al:55-59:
            // a value for the corpus app, Base Application, System Application and Application; the corpus app's text). An app
            // whose manifest was not read, or whose description is empty, is not pinned: unknown rather than a blank Blob.
            r[pos(t, 15)] = match a.description.as_deref() {
                Some(d) if !d.is_empty() => {
                    let blob = al_builtins::stream::new_blob();
                    al_builtins::stream::write_bytes(&blob, d.as_bytes());
                    Value::Blob(blob)
                }
                _ => super::unknown(
                    t,
                    15,
                    &format!("app '{}': its manifest's description is not known", a.name),
                ),
            };
            // Logo (20): blank (null MediaId, no Media row) for an app whose app.json declares no logo (onprem
            // session-system/PZPublishedApplicationColumns.al:162-163); the media id of a logo [`served_logo`] serves; else unknown.
            // A logo served by an assumption marks every read of the row ([`Entry::assuming_in`], the Logo column).
            let mut logo_assumed = None;
            match served_logo(a, assumed, p.engine().target.as_ref()) {
                Logo::None => {}
                Logo::Picture { assumed, .. } => {
                    r[pos(t, 20)] = Value::Guid(ids::logo_media_id(id));
                    logo_assumed = assumed;
                }
                Logo::Unknown(whose) => r[pos(t, 20)] = super::unknown(t, 20, &whose),
            }
            // The columns the row fills above, and those whose default the corpus pins for every app (13, 30, 31). The others (Brief, the links, Blob, the source and build columns of
            // Y48PublishedApplication.al) are filled for the Application row only: here unknown, a read is refused.
            const SERVED: [u32; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 15, 20, 30, 31];
            for f in &t.fields {
                if f.stored && f.id < al_builtins::platform::SYSTEM_OBJECT_BASE && !SERVED.contains(&f.id) {
                    r[pos(t, f.id)] = super::unknown(t, f.id, &format!("app '{}'", a.name));
                }
            }
            let entry = Entry::ok(r);
            Some(match (assumed, logo_assumed) {
                (Some(x), _) => entry.assuming(x),
                (None, Some(x)) => entry.assuming_in(x, vec![pos(t, 20)]),
                (None, None) => entry,
            })
        })
        .collect::<Vec<_>>();
    // The `Application` aggregate (application_row.rs).
    if let Some(id) = super::application_row::application_id(p) {
        rows.push(super::application_row::published_application_row(p, t, id));
    }
    rows
}

/// The Published Application listings of an engine's sessions (`Engine::derived`), by what its rows follow besides the
/// program ([`published_application_rows`]): the run's setting for dependency apps and the tenant id. Every test codeunit
/// has a session of its own, and Microsoft's code reads the table in many of them.
#[derive(Default)]
struct PublishedListings(Mutex<Vec<(ListingKey, Arc<SharedMat>)>>);

/// The run's setting for dependency apps and the tenant id ([`PublishedListings`]).
type ListingKey = (bool, String);

/// The Published Application listing of the session `p`, shared with the engine's other sessions with the same setting and
/// tenant id; `Err` with the listing `build` made when it holds a handle (a description Blob) and stays the session's own.
pub(super) fn published_application_listing(
    p: &Providers,
    build: impl FnOnce() -> Mat,
) -> Result<Arc<SharedMat>, Box<Mat>> {
    let engine = p.engine();
    let key = (engine.dependencies_published_global, p.ident.tenant_id.clone());
    let kept = engine.derived(PublishedListings::default);
    if let Some((_, m)) = kept
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find(|(k, _)| *k == key)
    {
        return Ok(m.clone());
    }
    let m = Arc::new(SharedMat::try_new(build())?);
    kept.0.lock().unwrap_or_else(|e| e.into_inner()).push((key, m.clone()));
    Ok(m)
}

/// Media (2000000181): the row under `media_id`, the id the Published Application Logo of an app whose logo [`served_logo`]
/// serves holds; `None` when no app's served logo has that id. Only that app's logo is built: an app's id gives its media id
/// alone ([`ids::logo_media_id`]), so the other apps' logos are never copied. The row holds the logo file's bytes unchanged,
/// `image/png`, the picture's height and width, and every other column empty (Description, Company Name, Expiration Date,
/// File Name, Security Token, Creating User; Prohibit Cache No): onprem session-system/UsAppLogo.al:110-116 (a 1 x 1 PNG of
/// 67 bytes) and :140-145 (a 300 x 300 PNG of 90393 bytes, same SHA-256). A JPEG logo [`served_logo`] serves has the same
/// row, with the Mime Type [`served_logo`] gives it; the row of a logo served by an assumption marks it, and its columns that
/// assumption's probe does not observe refuse.
pub(super) fn logo_media_row(p: &Providers, t: &TableDef, media_id: [u8; 16]) -> Option<Entry> {
    let engine = p.engine();
    let (app, bytes, (width, height), mime, assumed) = engine
        .program
        .apps
        .iter()
        .filter(|a| al_builtins::ops::parse_guid(&a.id).is_some_and(|id| ids::logo_media_id(id) == media_id))
        .find_map(|a| {
            match served_logo(
                a,
                route(a, engine.dependencies_published_global).1,
                engine.target.as_ref(),
            ) {
                Logo::Picture {
                    bytes,
                    size,
                    mime,
                    assumed,
                } => Some((&a.name, bytes, size, mime, assumed)),
                Logo::None | Logo::Unknown(_) => None,
            }
        })?;
    let field = |name: &str| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
    let mut r = blank(t);
    r[pos(t, 1)] = Value::Guid(media_id);
    let blob = al_builtins::stream::new_blob();
    al_builtins::stream::write_bytes(&blob, bytes);
    #[cfg(test)]
    LOGO_BLOBS_WRITTEN.with(|c| c.set(c.get() + 1));
    r[field("Content")?] = Value::Blob(blob);
    r[field("Mime Type")?] = Value::Text(mime.into());
    r[field("Height")?] = Value::Integer(height);
    r[field("Width")?] = Value::Integer(width);
    let Some(x) = assumed else {
        return Some(Entry::ok(r));
    };
    // The assumption's probe observes only these columns (UsAppLogoJpegSizes.al:96-121); the others are pinned for the 8 x 8
    // logo alone (UsAppLogoJpeg.al:112-114), so a read of one refuses.
    let observed = [pos(t, 1), field("Content")?, field("Mime Type")?, field("Height")?, field("Width")?];
    let whose = format!("the logo of app '{app}' (open corpus PR 1594 observes only its ID, Content, Mime Type, Height and Width)");
    for (i, f) in t.fields.iter().enumerate().filter(|(i, _)| !observed.contains(i)) {
        r[i] = super::unknown(t, f.id, &whose);
    }
    Some(Entry::ok(r).assuming(x))
}

#[cfg(test)]
thread_local! {
    /// How many logo Blobs the Media rows wrote: a read of one id builds only the logo it finds.
    static LOGO_BLOBS_WRITTEN: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What the Published Application row of app `a` and its Media row ([`logo_media_row`]) serve of the logo its manifest
/// declares, for a row that rests on `row_assumed`.
enum Logo<'a> {
    /// The manifest declares no logo.
    None,
    /// A picture with a media id and a Media row: its bytes, size, the Mime Type of the Media row, and the assumption it
    /// rests on (`None`: pinned).
    Picture {
        bytes: &'a [u8],
        size: (i32, i32),
        mime: &'static str,
        assumed: Option<al_builtins::assumptions::Assumption>,
    },
    /// Not pinned: why.
    Unknown(String),
}

/// The size of the measured JPEG logo, width by height (UsAppLogoJpeg.al:113-117 `Height=8;Width=8`).
const MEASURED_JPEG_SIZE: (i32, i32) = (8, 8);

/// A PNG logo is served (onprem session-system/UsAppLogo.al:108-117 UsLogo_TinyPng_AppD, :140-146 UsLogo_BigPng_AppC), Mime
/// Type `image/png`. A JPEG built as the measured one ([`al_builtins::media::jfif_logo_size`]) of the measured size, 8 x 8, is
/// served the same (UsAppLogoJpeg.al:82-120, cross-app fixture E's logo.jpg) on a row that rests on no assumption (the
/// fixture's row rests on none), Mime Type `image/png` before BC 29 and `image/jpeg` from BC 29 (UsAppLogoJpeg.al:113-117).
/// The other logos of open corpus PR 1594 are served by assumption `app-logo-jpeg-sizes` ([`al_builtins::media::measured_jpeg_logo`]). Every other logo,
/// a JPEG on a row that rests on an assumption, and an app whose manifest was not read, are not pinned.
fn served_logo<'a>(
    a: &'a al_ir::AppInfo,
    row_assumed: Option<al_builtins::assumptions::Assumption>,
    target: Option<&al_version::Target>,
) -> Logo<'a> {
    let bytes = match &a.logo {
        Some(al_ir::AppLogo::None) => return Logo::None,
        Some(al_ir::AppLogo::File(bytes)) => bytes.as_slice(),
        None => return Logo::Unknown(format!("app '{}': its manifest's logo was not read", a.name)),
    };
    let picture = |size, mime, assumed| Logo::Picture {
        bytes,
        size,
        mime,
        assumed,
    };
    if row_assumed.is_none()
        && let Some(al_builtins::media::MeasuredJpegLogo::Sized(w, h)) = al_builtins::media::measured_jpeg_logo(bytes, target)
    {
        let assumed = Some(al_builtins::assumptions::Assumption::AppLogoJpegSizes);
        return picture((w, h), "image/jpeg", assumed);
    }
    let jpeg_mime = if al_version::gate_on(target, al_version::Gate::ProbedValuesDifferFrom29) {
        "image/jpeg"
    } else {
        "image/png"
    };
    match (
        al_builtins::media::png_size(bytes),
        al_builtins::media::jfif_logo_size(bytes),
        row_assumed,
    ) {
        (Some(size), _, _) => picture(size, "image/png", None),
        (None, Some(MEASURED_JPEG_SIZE), None) => picture(MEASURED_JPEG_SIZE, jpeg_mime, None),
        (None, Some((width, height)), None) => Logo::Unknown(format!(
            "app '{}', whose JPEG logo is {width} x {height}: only the measured 8 x 8 is pinned, and assumption \
             app-logo-jpeg-sizes covers the logo files of open corpus PR 1594 only, on BC 27 and 29",
            a.name
        )),
        (None, Some(_), Some(other)) => Logo::Unknown(format!(
            "app '{}', whose JPEG logo is served only on a row that rests on no assumption (this row rests on '{}')",
            a.name,
            other.info().id
        )),
        (None, None, _) => Logo::Unknown(format!(
            "app '{}', whose logo is neither a PNG nor a JPEG built as the measured one (the Media row of another picture is not pinned)",
            a.name
        )),
    }
}

/// NAV App Extra: "Runtime Package ID" (1), "Tenant Visible" (2), "PerTenant Or Installed" (3). One row per listed app, both
/// Booleans true: Published Application looks them up by "Runtime Package ID" and reads true for this app and for the app
/// it depends on (record/TestPublishedApplicationSysTable.al:294-365).
pub(super) fn nav_app_extra_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    p.engine()
        .program
        .apps
        .iter()
        .filter_map(|a| {
            let id = al_builtins::ops::parse_guid(&a.id)?;
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Guid(ids::runtime_package_id(id, a.dependency));
            r[pos(t, 2)] = Value::Boolean(true);
            r[pos(t, 3)] = Value::Boolean(true);
            Some(Entry::ok(r))
        })
        .chain(application_runtime_row(p, t, true))
        .collect()
}

/// The NAV App Extra or Installed Application row of the Application app: Tenant Visible, PerTenant Or Installed and Installed
/// are Yes for it (onprem session-system/PZPublishedApplicationColumns.al:53-57).
fn application_runtime_row(p: &Providers, t: &TableDef, extra: bool) -> Option<Entry> {
    let id = super::application_row::application_id(p)?;
    let mut r = blank(t);
    r[pos(t, 1)] = Value::Guid(super::application_row::runtime_package_id(id));
    if extra {
        r[pos(t, 2)] = Value::Boolean(true);
        r[pos(t, 3)] = Value::Boolean(true);
    }
    Some(Entry::ok(r))
}

/// Installed Application: "Runtime Package ID" (1). One row per listed app: every app of the run is installed
/// (record/TestPublishedApplicationSysTable.al:275-290 for this app, :342-365 for the app it depends on).
pub(super) fn installed_application_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    p.engine()
        .program
        .apps
        .iter()
        .filter_map(|a| {
            let id = al_builtins::ops::parse_guid(&a.id)?;
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Guid(ids::runtime_package_id(id, a.dependency));
            Some(Entry::ok(r))
        })
        .chain(application_runtime_row(p, t, false))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_builtins::assumptions::Assumption;

    /// A JPEG logo on a row that rests on an assumption is not served, so neither the row's Logo nor a Media row has it; on a
    /// row that rests on none the measured 8 x 8 one is served without an assumption, and one of another size is not served.
    /// A file that is no picture is not served either way.
    #[test]
    fn a_jpeg_logo_is_served_only_on_a_row_without_an_assumption() {
        let app = |logo: Vec<u8>| al_ir::AppInfo {
            name: "D".into(),
            logo: Some(al_ir::AppLogo::File(logo)),
            ..Default::default()
        };
        let bytes = jfif_8x8();
        let global = Some(Assumption::DependencyPublishedAsGlobal);
        assert!(matches!(
            served_logo(&app(bytes.clone()), None, None),
            Logo::Picture {
                mime: "image/jpeg",
                size: (8, 8),
                assumed: None,
                ..
            }
        ));
        let mut wide = bytes.clone();
        let sof = wide.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
        wide[sof + 8] = 16;
        assert_eq!(al_builtins::media::jfif_logo_size(&wide), Some((16, 8)));
        assert!(matches!(served_logo(&app(wide), None, None), Logo::Unknown(why) if why.contains("8 x 8")));
        assert!(
            matches!(served_logo(&app(bytes), global, None), Logo::Unknown(why) if why.contains("dependency-published-as-global"))
        );
        assert!(matches!(served_logo(&app(vec![1, 2, 3]), None, None), Logo::Unknown(_)));
    }

    /// Assumption `app-logo-jpeg-sizes` serves a JPEG with the probe's JFIF header in a measured size on BC 27 and 29 only, on
    /// a row that rests on no other assumption, with Mime Type image/jpeg; one fact changed at a time keeps main's answer: BC
    /// 28, another size, another JFIF header, a row that rests on an assumption.
    #[test]
    fn a_jpeg_logo_of_a_measured_size_is_served_by_its_assumption_only_as_measured() {
        let app = |logo: Vec<u8>| al_ir::AppInfo {
            name: "D".into(),
            logo: Some(al_ir::AppLogo::File(logo)),
            ..Default::default()
        };
        let target = |major| Some(al_version::Target::for_bc(al_version::BcVersion::new(major, 0)));
        let sized = |w: u16, h: u16, header: [u8; 9]| {
            let mut b = jfif_8x8();
            b[11..20].copy_from_slice(&header);
            let sof = b.windows(2).position(|w| w == [0xFF, 0xC0]).unwrap();
            b[sof + 5..sof + 7].copy_from_slice(&h.to_be_bytes());
            b[sof + 7..sof + 9].copy_from_slice(&w.to_be_bytes());
            b
        };
        let measured_header = [1, 1, 0, 0, 1, 0, 1, 0, 0];
        for major in [27, 29] {
            for (w, h) in [(535, 534), (16, 8), (400, 300)] {
                let logo = app(sized(w, h, measured_header));
                assert!(matches!(
                    served_logo(&logo, None, target(major).as_ref()),
                    Logo::Picture { mime: "image/jpeg", size, assumed: Some(Assumption::AppLogoJpegSizes), .. }
                        if size == (i32::from(w), i32::from(h))
                ));
            }
        }
        let g = app(sized(535, 534, measured_header));
        assert!(matches!(served_logo(&g, None, target(28).as_ref()), Logo::Unknown(_)));
        let global = Some(Assumption::DependencyPublishedAsGlobal);
        assert!(matches!(served_logo(&g, global, target(29).as_ref()), Logo::Unknown(_)));
        let other_size = app(sized(300, 300, measured_header));
        assert!(matches!(
            served_logo(&other_size, None, target(29).as_ref()),
            Logo::Unknown(_)
        ));
        let dpi96 = app(sized(535, 534, [1, 1, 1, 0, 0x60, 0, 0x60, 0, 0]));
        assert!(matches!(
            served_logo(&dpi96, None, target(29).as_ref()),
            Logo::Unknown(_)
        ));
        // The pinned 8 x 8 logo with the probe's header stays pinned: image/png on 27, no assumption.
        let e = app(sized(8, 8, measured_header));
        assert!(matches!(
            served_logo(&e, None, target(27).as_ref()),
            Logo::Picture {
                mime: "image/png",
                assumed: None,
                ..
            }
        ));
    }

    /// A read of Media by one app's logo id writes that app's logo alone, not the logo of every app (issue 8608): of
    /// three apps with PNG logos, the id of the second finds its row, a 2 x 3 picture, and one Blob is written.
    #[test]
    fn a_media_read_by_id_builds_only_that_apps_logo() {
        let png = |w: u8, h: u8| {
            let mut b = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
            b.extend_from_slice(b"IHDR");
            b.extend_from_slice(&[0, 0, 0, w, 0, 0, 0, h]);
            b
        };
        let ids = [
            "11111111-0000-0000-0000-000000000001",
            "22222222-0000-0000-0000-000000000002",
            "33333333-0000-0000-0000-000000000003",
        ];
        let program = al_ir::Program {
            apps: ids
                .iter()
                .zip(1u8..)
                .map(|(id, n)| al_ir::AppInfo {
                    id: (*id).into(),
                    name: format!("App {n}"),
                    logo: Some(al_ir::AppLogo::File(png(n, n + 1))),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let engine = crate::Engine::new(program, Vec::new());
        let p = Providers::new(
            &engine,
            super::super::Ident::of(&al_host::Host::default()),
            std::rc::Rc::default(),
            al_builtins::media::MediaStore::default(),
        );
        let field = |id: u32, name: &str, ty| al_data::FieldDef {
            id,
            name: name.into(),
            ty,
            stored: true,
            code_storage: Default::default(),
        };
        use al_builtins::value::RtType;
        let t = TableDef {
            object: 2000000181,
            name: "Media".into(),
            fields: vec![
                field(1, "ID", RtType::Guid),
                field(3, "Content", RtType::Integer),
                field(4, "Mime Type", RtType::Text(None)),
                field(5, "Height", RtType::Integer),
                field(6, "Width", RtType::Integer),
            ],
            keys: vec![vec![0]],
            per_company: false,
        };
        let media_id = ids::logo_media_id(al_builtins::ops::parse_guid(ids[1]).unwrap());
        let before = LOGO_BLOBS_WRITTEN.with(|c| c.get());
        let found = p.media_of_id(&t, Some(&Value::Guid(media_id))).unwrap();
        assert_eq!(LOGO_BLOBS_WRITTEN.with(|c| c.get()) - before, 1);
        let row = found.row_at(0).unwrap();
        assert!(matches!(row[0], Value::Guid(g) if g == media_id));
        assert!(matches!((&row[3], &row[4]), (Value::Integer(3), Value::Integer(2))));
        assert!(found.row_at(1).is_none());
    }

    /// The segments of the measured JPEG logo's structure, each with filler bytes, for an 8 x 8 picture.
    fn jfif_8x8() -> Vec<u8> {
        let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        b.extend_from_slice(b"JFIF\0");
        b.extend_from_slice(&[0; 9]);
        b.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x04, 0, 0]);
        b.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 8, 0, 8, 0, 8, 3]);
        b.extend_from_slice(&[0; 9]);
        b.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x04, 0, 0]);
        b.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x0C, 3]);
        b.extend_from_slice(&[0; 9]);
        b.extend_from_slice(&[0x12, 0xFF, 0xD9]);
        b
    }
}
