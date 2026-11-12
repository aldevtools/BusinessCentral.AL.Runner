//! `Database.SetUserPassword(UserSecurityId, Password)` (OnPrem): the server sets the password of a user who signs in with
//! a Business Central password. The runner keeps it, hashed, in the user's User Property row for the run (AL Runner issue 7447).
//!
//! Pinned by the OnPrem corpus app (al-language-onprem session-system/database-and-blob-operations), BC 27.5, 28.5 and 29.0:
//! - A user with no User Property row (an unknown User Security ID, the null GUID): the NothingInsideFilter error of User
//!   Property, also inside a TryFunction (ODatabaseOps.al:73-80; SetUserPasswordStatement.al UnknownUser, NullGuid).
//! - Inside a TryFunction whose result is used, a user who has the row: the TryFunction write error for MODIFY, whatever the
//!   password and the user (SetUserPasswordExistingUser.al:91-178), so that check comes after the row is found and before
//!   the check of the user name.
//! - As a statement, a user with a blank User Name: the TestField error of User Name (SetUserPasswordStatement.al
//!   UserWithBlankName).
//! - As a statement, any other user, enabled or disabled, and any password (blank, short, without digits or symbols, 300
//!   characters, the user name): no error; Password then holds 27 characters that are not the plain text, a different
//!   password changes them and the same password again does not (SetUserPasswordStatement.al ExistingUser_*, DisabledUser).
//!
//! - `NavUserAccountHelper.IsPasswordSet` reads the same Password field (`user_password_is_set`).
//!
//! The 27 characters BC stores are its own hash, which no test can predict; the runner stores 27 characters worked out from
//! the password alone, with the properties above. Whether two users with the same password get the same text is not pinned.
//! Refused: a User Property row with no User row (not pinned), and a call whose return value is used (not pinned).

use crate::Session;
use crate::usertable::{USER, USER_PROPERTY};
use al_builtins::native::{AlError, Signal, raise, unsupported};
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::Store;

/// Field id of "User Name" in User.
const F_NAME: u32 = 2;

impl Session {
    pub(crate) fn set_user_password(&mut self, args: &[Value]) -> Result<Value, Signal> {
        if self.ret_used {
            return unsupported("Database.SetUserPassword whose return value is used (not pinned)");
        }
        let (sid, password) = match args {
            [Value::Guid(sid), p] => match al_builtins::native::text_of(p) {
                Some(t) => (*sid, t.to_string()),
                None => return unsupported("Database.SetUserPassword with a password that is not a plain text"),
            },
            _ => return unsupported("Database.SetUserPassword with arguments that are not a Guid and a text"),
        };
        let (Some(up), Some(user)) = (self.table_def(USER_PROPERTY), self.table_def(USER)) else {
            return unsupported("Database.SetUserPassword in a program that does not compile User and User Property");
        };
        let cx = self.cx.clone();
        let key = [Value::Guid(sid)];
        let Some(mut prop) = self.store.get(&cx, &up, &key)? else {
            return self.no_user_property(&up, sid);
        };
        self.gate_used_try_write("MODIFY")?;
        self.gate_write(RecordOp::Modify)?;
        let Some(user_row) = self.store.get(&cx, &user, &key)? else {
            return unsupported("Database.SetUserPassword for a User Property row that has no User row (not pinned)");
        };
        let name = crate::platform::pos(&user, F_NAME);
        if matches!(&user_row[name], Value::Code(s) | Value::Text(s) if s.is_empty()) {
            let message = format!(
                "{} must have a value in {}: {}={}. It cannot be zero or empty.",
                self.field_caption(&user, name),
                self.caption(&user),
                self.field_caption(&user, crate::platform::pos(&user, 1)),
                self.message_value(&key[0])?
            );
            return Err(Signal::Error(Box::new(AlError {
                message,
                code: "TestField".into(),
                ..Default::default()
            })));
        }
        let Some(field) = up.fields.iter().position(|f| f.name.eq_ignore_ascii_case("Password")) else {
            return unsupported("User Property has no Password field");
        };
        prop[field] = Value::Text(stored_text(&password).into());
        self.store.modify(&cx, &up, prop)?;
        Ok(Value::Null)
    }

    /// `NavUserAccountHelper.IsPasswordSet(UserSecurityId)`, which Base Application's "Identity Management".IsUserPasswordSet
    /// asks and the User Card reaches (AL Runner issue 7447). Measured on BC 27.5, 28.5 and 29.0 (corpus PR 1330,
    /// al-language-onprem session-system/database-and-blob-operations/SetUserPasswordStatement.al:284): No for a user row AL
    /// inserted (its User Property row has a blank Password), Yes after `SetUserPassword` with a strong and with a blank
    /// password (Password then holds the stored text), and for an unknown user the NothingInsideFilter error of User Property,
    /// the same error `SetUserPassword` raises.
    pub(crate) fn user_password_is_set(&mut self, sid: [u8; 16]) -> Result<bool, Signal> {
        let Some(up) = self.table_def(USER_PROPERTY) else {
            return unsupported("NavUserAccountHelper.IsPasswordSet in a program that does not compile User Property");
        };
        let Some(field) = up.fields.iter().position(|f| f.name.eq_ignore_ascii_case("Password")) else {
            return unsupported("User Property has no Password field");
        };
        let cx = self.cx.clone();
        let Some(prop) = self.store.get(&cx, &up, &[Value::Guid(sid)])? else {
            return self.no_user_property(&up, sid);
        };
        match &prop[field] {
            Value::Text(t) | Value::Code(t) => Ok(!t.is_empty()),
            _ => unsupported("User Property Password that is not a text"),
        }
    }

    /// The NothingInsideFilter error of User Property for a User Security ID that has no row (ODatabaseOps.al:73-80;
    /// SetUserPasswordStatement.al UnknownUser, NullGuid, IsUserPasswordSet).
    fn no_user_property<T>(&self, up: &al_data::TableDef, sid: [u8; 16]) -> Result<T, Signal> {
        let shown = self.message_value(&Value::Guid(sid))?;
        raise(
            format!(
                "There is no {} within the filter.{BR}{BR}Filters: {}: {shown}",
                self.caption(up),
                self.field_caption(up, crate::platform::pos(up, 1)),
                BR = crate::records::BR
            ),
            "DB:NothingInsideFilter",
        )
    }
}

/// 27 characters of the base64 alphabet worked out from `password` alone (the length BC stores; the text is not BC's).
fn stored_text(password: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(27);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in password.bytes() {
        h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
    }
    for i in 0..27u64 {
        h = (h ^ i).wrapping_mul(0x0000_0100_0000_01b3);
        h ^= h >> 29;
        out.push(char::from(ALPHABET[(h % 64) as usize]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::stored_text;

    #[test]
    fn the_stored_text_has_the_measured_shape() {
        let a = stored_text("LT-Sup3r!Passw0rd#1");
        assert_eq!(a.len(), 27);
        assert_ne!(a, "LT-Sup3r!Passw0rd#1");
        assert_eq!(a, stored_text("LT-Sup3r!Passw0rd#1"));
        assert_ne!(a, stored_text("LT-Oth3r!Passw0rd#2"));
        assert_eq!(stored_text("").len(), 27);
    }
}
