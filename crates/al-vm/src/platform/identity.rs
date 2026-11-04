//! The Session and Active Session tables: one row, the session that is reading.
//!
//! Pinned by corpus session-system/TestSessionVirtualTable.al (one row, flagged `My Session`, keyed by
//! `SessionId()`, user `UserId()`, a login date and time, a host name, `Application Name` the
//! text of the Active Session `Client Type`, `Database Name` the Active Session one) and
//! session-system/TestActiveSessionTable.al (one row keyed by `ServiceInstanceId()` and `SessionId()`
//! with the user id and security id, a login date time and a session unique id). A session id
//! that belongs to no session finds nothing (TestSessionVirtualTable.al "GetOnAConnectionIdThat
//! IsNoSession", TestActiveSessionTable.al "GetOnASessionIdThatIsNoSession").
//!
//! Columns no test reads keep the type default, as AL Runner does ("Server Instance Name"). "Client Computer Name" is not in `PINNED_COLUMNS`: the row holds it
//! unknown, so a read is refused (it is the test runner's, TestActiveSessionHostColumnsContracts.al:9).

use super::{Ident, blank, option, pos};
use al_builtins::value::Value;
use al_data::{Row, TableDef};

const MS_PER_DAY: i64 = 86_400_000;

/// The name of the database the session reads: not blank, and the same in the Session and the Active Session row
/// (session-system/W6SessionTimeZone.al:ActiveSession_OwnRow_HostColumns and ActiveSession_DatabaseName_EqualsSessionDatabaseName,
/// BC 27.5 and 29.0). The name is the one of the oracle's database, `CRONUS` (metadata/platform-tables/AwTests.al:49
/// for Session, :71 for Active Session).
const DATABASE_NAME: &str = "CRONUS";

/// The ordinal of the `Client Type` option member of Active Session for a host client type, and
/// the text `Format` gives it.
fn client_type_ordinal(t: al_host::ClientType) -> (i32, &'static str) {
    match t {
        al_host::ClientType::Background => (5, "Background"),
        al_host::ClientType::Web => (7, "Web Client"),
        al_host::ClientType::Windows => (0, "Windows Client"),
    }
}

fn unix_epoch_day() -> i64 {
    i64::from(al_builtins::format::day_from_civil(1970, 1, 1))
}

/// A session unique id: not the null guid, and one per session id.
fn session_unique_id(session_id: i32) -> [u8; 16] {
    let mut g = [0u8; 16];
    g[0] = 0x53; // 'S'
    g[6] = 0x40;
    g[8] = 0x80;
    g[12..].copy_from_slice(&session_id.to_be_bytes());
    g
}

pub(super) fn session_row(t: &TableDef, i: &Ident) -> Row {
    let mut r = blank(t);
    r[pos(t, 1)] = Value::Integer(i.session_id);
    r[pos(t, 2)] = Value::Text(i.user_id.clone().into());
    r[pos(t, 3)] = Value::Boolean(true);
    r[pos(t, 5)] = Value::Time(i.login_millis.rem_euclid(MS_PER_DAY) + 1);
    r[pos(t, 6)] = Value::Date((i.login_millis.div_euclid(MS_PER_DAY) + unix_epoch_day()) as i32, false);
    r[pos(t, 16)] = Value::Text(DATABASE_NAME.into());
    r[pos(t, 17)] = Value::Text(client_type_ordinal(i.client_type).1.into());
    // "Login Type": the web client session of the tests logs in by user name and password (session/
    // W6SessionTimeZone.al:22-23). Another client type's login is not pinned: unknown.
    r[pos(t, 18)] = match i.client_type {
        al_host::ClientType::Web => option(t, 18, 2),
        _ => super::unknown(t, 18, "a session that is not a web client session"),
    };
    r[pos(t, 19)] = Value::Text(i.host_name.clone().into());
    r
}

pub(super) fn active_session_row(t: &TableDef, i: &Ident) -> Row {
    let mut r = blank(t);
    r[pos(t, 1)] = Value::Guid(i.user_sid);
    r[pos(t, 2)] = Value::Integer(i.instance_id);
    r[pos(t, 3)] = Value::Integer(i.session_id);
    // "Server Instance Name" and "Server Computer Name" are not blank; only their shape is pinned (session/
    // TestActiveSessionHostColumnsContracts.al).
    r[pos(t, 4)] = Value::Text("BC".into());
    r[pos(t, 5)] = Value::Text(i.host_name.clone().into());
    r[pos(t, 6)] = Value::Text(i.user_id.clone().into());
    r[pos(t, 7)] = option(t, 7, client_type_ordinal(i.client_type).0);
    r[pos(t, 9)] = crate::hostvalues::datetime_of_unix_millis(i.login_millis);
    r[pos(t, 10)] = Value::Text(DATABASE_NAME.into());
    r[pos(t, 11)] = Value::Guid(session_unique_id(i.session_id));
    r
}
