//! The tables that list the apps of the tenant: NAV App Installed App and Published Application.
//! Their rows are the apps of the program, which is every app the run loaded.
//!
//! Pinned by corpus record/TestNavAppInstalledAppSysTable.al (a row per app keyed by its id, with the
//! manifest's name, publisher and version, a package id that is not blank and not shared between
//! apps). The package id is `ids::package_id`, which AllObj reports for the app's objects too, so the
//! ownership check of the retention policy module compares equal for an app and its own tables
//! (install/TestRetenPolAllowedTables_Tests.al).

use super::mat::Entry;
use super::{Providers, blank, ids, pos};
use al_builtins::value::Value;
use al_data::TableDef;

fn version(v: &str) -> Option<[i32; 4]> {
    let parts: Vec<i32> = v.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
    parts.try_into().ok()
}

/// NAV App Installed App: "App ID" (1), "Package ID" (2), Name (3), Publisher (4), version (5 to 8), "Published As" (16).
pub(super) fn nav_app_installed_app_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    p.engine()
        .program
        .apps
        .iter()
        // The platform's System app has no row (record/lt-y12-published-as/Y12PublishedAs.al:41, :46 `no row`).
        .filter(|a| !a.id.eq_ignore_ascii_case(super::objects::al_sema_platform_app_id()))
        .filter_map(|a| {
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
            // "Published As" follows how the app was published: an app under test, as the corpus apps through the development
            // endpoint, is Dev (2); Microsoft's apps are Global (0) (record/lt-y12-published-as/Y12PublishedAs.al:31-80, al-runner
            // issue 6438). How
            // another dependency app was published is not known.
            r[pos(t, 16)] = if !a.dependency {
                super::option(t, 16, 2)
            } else if a.publisher == "Microsoft" {
                super::option(t, 16, 0)
            } else {
                let whose = format!(
                    "app '{}' by '{}', a dependency not published by Microsoft",
                    a.name, a.publisher
                );
                super::unknown(t, 16, &whose)
            };
            Some(Entry::ok(r))
        })
        .collect()
}

/// Published Application: "Runtime Package ID" (1), "Package ID" (2), ID (3), Name (4), Publisher (5),
/// version (6 to 9). The two package ids of an app are one value (AL Runner measured that on a real tier
/// for an app published through the development endpoint). "Tenant ID" (12) is blank: install/TestRetenPolAllowedTables_Tests.al
/// only pins that the row of Base Application satisfies `SetFilter("Tenant ID", '%1|%2', '', TenantId)`, which blank does
/// for every app (docs/probes-needed.md: per-tenant apps may carry the tenant id on a real tier).
pub(super) fn published_application_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
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
            // Description holds the manifest's description as UTF-8 text (onprem session/PZPublishedApplicationColumns.al:55-59:
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
            // session/PZPublishedApplicationColumns.al:162-163). An app whose manifest declares a PNG logo has a media with a
            // value whose Media row holds the file's bytes ([`logo_media_rows`]; onprem session/UsAppLogo.al:79-148, corpus PR
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
            // The columns the row fills above, and those whose default the corpus pins for every app (11 Published As, 12
            // Tenant ID, 13, 30, 31). The others (Brief, the links, Blob, the source and build columns of
            // Y48PublishedApplication.al) are filled for the Application row only: here unknown, a read is refused.
            const SERVED: [u32; 16] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 15, 20, 30, 31];
            for f in &t.fields {
                if f.stored && f.id < al_builtins::platform::SYSTEM_OBJECT_BASE && !SERVED.contains(&f.id) {
                    r[pos(t, f.id)] = super::unknown(t, f.id, &format!("app '{}'", a.name));
                }
            }
            Some(Entry::ok(r))
        })
        .collect::<Vec<_>>();
    // The `Application` aggregate (application_row.rs).
    if let Some(id) = super::application_row::application_id(p) {
        rows.push(super::application_row::published_application_row(p, t, id));
    }
    rows
}

/// Media (2000000181): one row per app whose manifest declares a PNG logo, under the id its Published Application Logo
/// holds. The row holds the logo file's bytes unchanged, `image/png`, the picture's height and width, and every other
/// column empty (Description, Company Name, Expiration Date, File Name, Security Token, Creating User; Prohibit Cache No):
/// onprem session/UsAppLogo.al:110-116 (a 1 x 1 PNG of 67 bytes) and :140-145 (a 300 x 300 PNG of 90393 bytes, same SHA-256).
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
/// are Yes for it (onprem session/PZPublishedApplicationColumns.al:53-57).
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
