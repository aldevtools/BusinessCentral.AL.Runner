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

/// NAV App Installed App: "App ID" (1), "Package ID" (2), Name (3), Publisher (4), version (5 to 8).
pub(super) fn nav_app_installed_app_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    p.engine()
        .program
        .apps
        .iter()
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
    p.engine()
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
            // Description holds the manifest's description (onprem session/PZPublishedApplicationColumns.al:55, :59);
            // the program does not keep it (`AppInfo`), so the column is unknown rather than a blank Blob.
            r[pos(t, 15)] = super::unknown(t, 15, "an app: the runtime does not keep the manifest's description");
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
            Some(Entry::ok(r))
        })
        .collect()
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
        .collect()
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
        .collect()
}
