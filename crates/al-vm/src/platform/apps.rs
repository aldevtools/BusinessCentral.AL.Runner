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
            // Logo (20): blank (null MediaId, no Media row) for an app whose app.json declares no logo (corpus branch
            // al-runtime-rs-staging-ug eff7eb2a, onprem session/PZPublishedApplicationColumns.al
            // Logo_GuidedExperienceSequence_OfThisAppAndMicrosoftApps; assumption
            // published-application-logo-blank-without-manifest-logo until the bump). Microsoft's apps have a logo whose
            // content the corpus pins only as present, and an app that declares one is not pinned: unknown.
            if a.logo != Some(false) {
                let whose = match a.logo {
                    Some(_) => format!("app '{}', whose app.json declares a logo (its content is not pinned)", a.name),
                    None => format!(
                        "app '{}': its manifest was not read, and an app with a logo (Microsoft's apps) has content the corpus does not pin",
                        a.name
                    ),
                };
                r[pos(t, 20)] = super::unknown(t, 20, &whose);
            }
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
