//! The Session, Active Session and Server Instance tables: one row, the session that is reading and its server instance.
//!
//! Pinned by corpus session-system/TestSessionVirtualTable.al (one row, flagged `My Session`, keyed by
//! `SessionId()`, user `UserId()`, a login date and time, a host name, `Application Name` the
//! text of the Active Session `Client Type`, `Database Name` the Active Session one) and
//! session-system/TestActiveSessionTable.al (one row keyed by `ServiceInstanceId()` and `SessionId()`
//! with the user id and security id, a login date time and a session unique id). A session id
//! that belongs to no session finds nothing (TestSessionVirtualTable.al "GetOnAConnectionIdThat
//! IsNoSession", TestActiveSessionTable.al "GetOnASessionIdThatIsNoSession").
//!
//! Server Instance: one row (al-language-onprem metadata/platform-tables/Y21EnvColumnsOnPrem.al:276-301), whose instance
//! and computer names are those of the Active Session row.
//!
//! "Client Computer Name" is not in `PINNED_COLUMNS`: the row holds it
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

/// "Server Instance Name" of the Active Session row and of the Server Instance row: `BC`, and the same in both
/// (al-language-onprem metadata/platform-tables/Y21EnvColumnsOnPrem.al:288, :294-295: the Server Instance row reads `BC`
/// and equals the session's).
const SERVER_INSTANCE_NAME: &str = "BC";

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
    // "Server Computer Name" is not blank; only its shape is pinned (session-system/TestActiveSessionHostColumnsContracts.al).
    r[pos(t, 4)] = Value::Text(SERVER_INSTANCE_NAME.into());
    r[pos(t, 5)] = Value::Text(i.host_name.clone().into());
    r[pos(t, 6)] = Value::Text(i.user_id.clone().into());
    r[pos(t, 7)] = option(t, 7, client_type_ordinal(i.client_type).0);
    r[pos(t, 9)] = crate::hostvalues::datetime_of_unix_millis(i.login_millis);
    r[pos(t, 10)] = Value::Text(DATABASE_NAME.into());
    r[pos(t, 11)] = Value::Guid(session_unique_id(i.session_id));
    r
}

/// The one row of Server Instance, the instance the session runs on (al-language-onprem
/// metadata/platform-tables/Y21EnvColumnsOnPrem.al:276-301 Y21_ServerInstance_OwnRow, BC 27, 28, 29): keyed by
/// `ServiceInstanceId()`, service `MicrosoftDynamicsNavServer`, instance name and computer name those of the session's
/// Active Session row, server port 7085, management port 7086, Status 0 (Started), "Last Tenant Config Version" 0 and
/// a "Last Active" that is set and not after the current time. Its value is the session's start here: only that rule is
/// pinned, so a test that compares it with another time can differ from BC (docs/probes-needed.md).
pub(super) fn server_instance_row(t: &TableDef, i: &Ident) -> Row {
    let mut r = blank(t);
    r[pos(t, 1)] = Value::Integer(i.instance_id);
    r[pos(t, 2)] = Value::Text("MicrosoftDynamicsNavServer".into());
    // The texts of `active_session_row` columns 5 and 4.
    r[pos(t, 3)] = Value::Text(i.host_name.clone().into());
    r[pos(t, 4)] = crate::hostvalues::datetime_of_unix_millis(i.login_millis);
    r[pos(t, 5)] = Value::Text(SERVER_INSTANCE_NAME.into());
    r[pos(t, 6)] = Value::Integer(7085);
    r[pos(t, 7)] = Value::Integer(7086);
    r[pos(t, 8)] = option(t, 8, 0);
    r[pos(t, 9)] = Value::BigInteger(0);
    r
}
