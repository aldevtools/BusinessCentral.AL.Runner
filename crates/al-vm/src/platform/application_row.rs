//! The Published Application row of the `Application` aggregate app. It holds no source, so the program has no app for it,
//! but a tier always publishes it (onprem session/PZPublishedApplicationColumns.al:53, :58).
//!
//! Pinned by al-language-onprem/platform/lt-y48-published-app/Y48PublishedApplication.al (corpus Y48, CI BC 27.5, 28.5 and
//! 29.0): one row (:135 PA_ApplicationRow_Columns `count` 1); Package ID and Runtime Package ID not null and different, equal to
//! no other app's Package ID, and no AllObjWithCaption row carries it (:154 PA_ApplicationRow_PackageIds); the four version
//! parts are Base Application's (and System Application's); Package Type Extension, Published As Global, Tenant ID blank,
//! Help, Url, EULA and Privacy Statement Microsoft's links, Resource Exposure Policy 7, Applicable To Dev Extension No, Source
//! Repository Url, Source Commit ID, Build By and Build Url blank (:148-151). Brief ('Application (W1)' there) names the
//! localization, which the runtime does not know; Description, Logo, the hashes and the blobs hold content no test pins: those
//! columns, and every other one, stay unknown.

use super::mat::Entry;
use super::{Providers, blank, ids, pos};
use al_builtins::value::Value;
use al_data::TableDef;

const BASE_APPLICATION: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";
const SYSTEM_APPLICATION: &str = "63ca2fa4-4f03-4f2b-a480-172fef340d3f";

/// The id of the Application app, unless the program holds an app with it (then it is listed as any other).
pub(super) fn application_id(p: &Providers) -> Option<[u8; 16]> {
    let application = al_builtins::moduleinfo::APPLICATION_APP_ID;
    let listed = p
        .engine()
        .program
        .apps
        .iter()
        .any(|a| a.id.eq_ignore_ascii_case(application));
    if listed {
        return None;
    }
    al_builtins::ops::parse_guid(application)
}

/// The Runtime Package ID of the Application row: another value than its Package ID, as for every Microsoft app.
pub(super) fn runtime_package_id(id: [u8; 16]) -> [u8; 16] {
    ids::runtime_package_id(id, true)
}

/// The row of the Application app in Published Application.
pub(super) fn published_application_row(p: &Providers, t: &TableDef, id: [u8; 16]) -> Entry {
    let mut r = blank(t);
    let mut known = vec![1, 2, 3, 4, 5, 10, 11, 12, 16, 17, 18, 19, 36, 37, 38, 39, 40, 41];
    r[pos(t, 1)] = Value::Guid(runtime_package_id(id));
    r[pos(t, 2)] = Value::Guid(ids::package_id(id));
    r[pos(t, 3)] = Value::Guid(id);
    r[pos(t, 4)] = Value::Text("Application".into());
    r[pos(t, 5)] = Value::Text("Microsoft".into());
    let engine = p.engine();
    let version = [BASE_APPLICATION, SYSTEM_APPLICATION].iter().find_map(|want| {
        let app = engine.program.apps.iter().find(|a| a.id.eq_ignore_ascii_case(want))?;
        let parts: Vec<i32> = app.version.split('.').map(|v| v.parse().ok()).collect::<Option<_>>()?;
        <[i32; 4]>::try_from(parts).ok()
    });
    if let Some(v) = version {
        for (n, part) in v.iter().enumerate() {
            r[pos(t, 6 + n as u32)] = Value::Integer(*part);
        }
        known.extend([6, 7, 8, 9]);
    }
    r[pos(t, 10)] = option_member(t, 10, "Extension");
    r[pos(t, 11)] = super::option(t, 11, 0);
    for (field, link) in [
        (16, "https://go.microsoft.com/fwlink/?LinkId=724009"),
        (17, "https://go.microsoft.com/fwlink/?linkid=2182906"),
        (18, "https://go.microsoft.com/fwlink/?LinkId=724011"),
        (19, "https://go.microsoft.com/fwlink/?linkid=2104024"),
    ] {
        r[pos(t, field)] = Value::Text(link.into());
    }
    r[pos(t, 36)] = Value::Integer(7);
    r[pos(t, 37)] = Value::Boolean(false);
    for (i, f) in t.fields.iter().enumerate() {
        if f.stored && f.id < al_builtins::platform::SYSTEM_OBJECT_BASE && !known.contains(&f.id) {
            r[i] = super::unknown(t, f.id, "the Application app");
        }
    }
    Entry::ok(r)
}

/// The option value of column `id` whose member is named `member`; unknown when the declaration has no such member.
fn option_member(t: &TableDef, id: u32, member: &str) -> Value {
    let blank = super::option(t, id, 0);
    let Value::Option(_, names) = &blank else {
        return super::unknown(t, id, "the Application app");
    };
    match names.iter().position(|n| n.trim().eq_ignore_ascii_case(member)) {
        Some(i) => Value::Option(i as i32, names.clone()),
        None => super::unknown(t, id, "the Application app"),
    }
}
