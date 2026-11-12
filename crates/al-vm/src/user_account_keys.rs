//! The account keys `NavUserAccountHelper` reads for Base Application's "Identity Management", which the User Card's
//! OnAfterGetRecord asks for (AL Runner issue 7447): `TryGetAuthenticationKey`, `TryGetNameIdentifier`,
//! `TryGetWebServicesKey` and `GetAuthenticationStatus`.
//!
//! Measured on BC 27.5, 28.5 and 29.0 (corpus PR 1330, al-language-onprem
//! session-system/database-and-blob-operations/SetUserPasswordStatement.al:323): for a user row AL inserted (User Name,
//! License Type, State only), for that user after `SetUserPassword`, and for the session user, GetACSStatus formats as 0
//! (both Try calls succeed with a blank authentication key and a blank name identifier), GetWebServiceExpiryDate is blank
//! and GetWebServicesKey is blank (TryGetWebServicesKey succeeds with a blank key and 0DT), and GetAuthenticationStatus is 0.
//!
//! The runner answers those values only for the state the probe shows: a user with a User row and a User Property row
//! whose key fields (and, for the authentication status, the authentication email and object id) are blank. Nothing in the
//! runner sets a key (TrySetAuthenticationKey and TryCreateWebServicesKey are not implemented), so a filled field comes
//! from AL writing User Property itself: not pinned, refused. An unknown user is not pinned either: refused.

use crate::Session;
use crate::usertable::{USER, USER_PROPERTY};
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::Value;
use al_data::{Store, TableDef};

/// Fields of User Property that must be blank for the measured answer, by member.
const AUTHENTICATION_KEY: &[&str] = &["Authentication Key"];
const NAME_IDENTIFIER: &[&str] = &["Name Identifier"];
const WEB_SERVICES_KEY: &[&str] = &["WebServices Key", "WebServices Key Expiry Date"];
const AUTHENTICATION_STATUS: &[&str] = &["Authentication Object ID"];

impl Session {
    /// `TryGetAuthenticationKey(UserSecurityId, var Key)`: true, Key blank (SetUserPasswordStatement.al:323, acs=0).
    pub(crate) fn try_get_authentication_key(&mut self, args: &mut [Value]) -> Result<Value, Signal> {
        self.blank_key_answer("NavUserAccountHelper.TryGetAuthenticationKey", args, AUTHENTICATION_KEY)
    }

    /// `TryGetNameIdentifier(UserSecurityId, var NameID)`: true, NameID blank (SetUserPasswordStatement.al:323, acs=0).
    pub(crate) fn try_get_name_identifier(&mut self, args: &mut [Value]) -> Result<Value, Signal> {
        self.blank_key_answer("NavUserAccountHelper.TryGetNameIdentifier", args, NAME_IDENTIFIER)
    }

    /// `TryGetWebServicesKey(UserSecurityId, var Key, var ExpiryDate)`: true, Key blank, ExpiryDate 0DT
    /// (SetUserPasswordStatement.al:323, expiry= and key blank=Yes key len=0).
    pub(crate) fn try_get_web_services_key(&mut self, args: &mut [Value]) -> Result<Value, Signal> {
        const WHAT: &str = "NavUserAccountHelper.TryGetWebServicesKey";
        let [Value::Guid(sid), key, expiry] = args else {
            return unsupported(format!(
                "{WHAT} with arguments that are not a Guid, a var Text and a var DateTime"
            ));
        };
        let sid = *sid;
        if !matches!(key, Value::Text(_)) || !matches!(expiry, Value::DateTime(_)) {
            return unsupported(format!(
                "{WHAT} with arguments that are not a Guid, a var Text and a var DateTime"
            ));
        }
        self.require_blank_fields(WHAT, sid, WEB_SERVICES_KEY, false)?;
        args[1] = Value::Text(String::new().into());
        args[2] = Value::DateTime(0);
        Ok(Value::Boolean(true))
    }

    /// `GetAuthenticationStatus(UserSecurityId)`: 0 (SetUserPasswordStatement.al:323, auth=0).
    pub(crate) fn get_authentication_status(&mut self, args: &[Value]) -> Result<Value, Signal> {
        const WHAT: &str = "NavUserAccountHelper.GetAuthenticationStatus";
        let [Value::Guid(sid)] = args else {
            return unsupported(format!("{WHAT} with an argument that is not a Guid"));
        };
        let sid = *sid;
        self.require_blank_fields(WHAT, sid, AUTHENTICATION_STATUS, true)?;
        Ok(Value::Integer(0))
    }

    /// A Try member with (Guid, var Text): true and a blank text when `fields` are blank.
    fn blank_key_answer(&mut self, what: &str, args: &mut [Value], fields: &[&str]) -> Result<Value, Signal> {
        let [Value::Guid(sid), Value::Text(_)] = args else {
            return unsupported(format!("{what} with arguments that are not a Guid and a var Text"));
        };
        let sid = *sid;
        self.require_blank_fields(what, sid, fields, false)?;
        args[1] = Value::Text(String::new().into());
        Ok(Value::Boolean(true))
    }

    /// Refuses unless `sid` has a User row and a User Property row whose `fields` are blank (and, with `email`, the User's
    /// Authentication Email is blank): the only state the probe measured.
    fn require_blank_fields(&mut self, what: &str, sid: [u8; 16], fields: &[&str], email: bool) -> Result<(), Signal> {
        let (Some(up), Some(user)) = (self.table_def(USER_PROPERTY), self.table_def(USER)) else {
            return unsupported(format!(
                "{what} in a program that does not compile User and User Property"
            ));
        };
        let cx = self.cx.clone();
        let key = [Value::Guid(sid)];
        let (Some(prop), Some(user_row)) = (self.store.get(&cx, &up, &key)?, self.store.get(&cx, &user, &key)?) else {
            return unsupported(format!(
                "{what} for a user without a User and a User Property row (not pinned)"
            ));
        };
        for name in fields {
            if !field_is_blank(&up, &prop, name) {
                return unsupported(format!(
                    "{what} for a user whose User Property {name} is not blank (not pinned)"
                ));
            }
        }
        if email && !field_is_blank(&user, &user_row, "Authentication Email") {
            return unsupported(format!("{what} for a user with an Authentication Email (not pinned)"));
        }
        Ok(())
    }
}

/// Whether the field `name` of `row` holds its blank value. A field the table does not have counts as not blank, so the
/// caller refuses.
fn field_is_blank(t: &TableDef, row: &[Value], name: &str) -> bool {
    let Some(i) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) else {
        return false;
    };
    match &row[i] {
        Value::Text(s) | Value::Code(s) => s.is_empty(),
        Value::DateTime(v) => *v == 0,
        Value::Guid(g) => *g == [0; 16],
        _ => false,
    }
}
