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
            // session-system/PZPublishedApplicationColumns.al:162-163). An app whose manifest declares a PNG logo has a media with a
            // value whose Media row holds the file's bytes ([`logo_media_rows`]; onprem session-system/UsAppLogo.al:79-148, corpus PR
            // 816; assumption published-application-logo-is-the-logo-file until the pin includes it).
            // A logo that is no PNG and an app whose manifest was not read are not pinned: unknown.
            match &a.logo {
                Some(al_ir::AppLogo::None) => {}
                Some(al_ir::AppLogo::File(bytes)) if al_builtins::media::png_size(bytes).is_some() => {
                    r[pos(t, 20)] = Value::Guid(ids::logo_media_id(id));
                }
                Some(al_ir::AppLogo::File(_)) => {
                    let whose = format!(
                        "app '{}', whose logo is no PNG (the Media row of another picture format is not pinned)",
                        a.name
                    );
                    r[pos(t, 20)] = super::unknown(t, 20, &whose);
                }
                None => {
                    let whose = format!("app '{}': its manifest's logo was not read", a.name);
                    r[pos(t, 20)] = super::unknown(t, 20, &whose);
                }
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
            Some(match assumed {
                Some(x) => entry.assuming(x),
                None => entry,
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

/// Media (2000000181): one row per app whose manifest declares a PNG logo, under the id its Published Application Logo
/// holds. The row holds the logo file's bytes unchanged, `image/png`, the picture's height and width, and every other
/// column empty (Description, Company Name, Expiration Date, File Name, Security Token, Creating User; Prohibit Cache No):
/// onprem session-system/UsAppLogo.al:110-116 (a 1 x 1 PNG of 67 bytes) and :140-145 (a 300 x 300 PNG of 90393 bytes, same SHA-256).
pub(super) fn logo_media_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let field = |name: &str| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name));
    p.engine()
        .program
        .apps
        .iter()
        .filter_map(|a| {
            let id = al_builtins::ops::parse_guid(&a.id)?;
            let Some(al_ir::AppLogo::File(bytes)) = &a.logo else {
                return None;
            };
            let (width, height) = al_builtins::media::png_size(bytes)?;
            let mut r = blank(t);
            r[pos(t, 1)] = Value::Guid(ids::logo_media_id(id));
            let blob = al_builtins::stream::new_blob();
            al_builtins::stream::write_bytes(&blob, bytes);
            r[field("Content")?] = Value::Blob(blob);
            r[field("Mime Type")?] = Value::Text("image/png".into());
            r[field("Height")?] = Value::Integer(height);
            r[field("Width")?] = Value::Integer(width);
            Some(Entry::ok(r))
        })
        .collect()
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
