//! The platform's triggers on the User table (2000000120): what the server does when a User row is
//! written, besides storing it. AL cannot see them as code; they show as side effects.
//!
//! Pinned by the OnPrem corpus app (the topology that asks for unique authentication emails):
//! - Insert and Modify normalise "Authentication Email" after the table's OnBefore events and
//!   triggers, so a subscriber reads the raw text and the text it assigns is the one normalised and
//!   checked (access-control/TestUserAuthEmailTriggerOrder.al:38-143, :146-164).
//! - A normalised address another ENABLED user carries is refused; a disabled user neither holds
//!   its address exclusively nor is refused (TestUserAuthEmailTrigger.al:162-204, :207-220,
//!   :275-319).
//! - Modify refuses a user name another user carries (TestUserAuthEmailTrigger.al:321-351).
//! - Modify that changes the email clears the user's "Authentication Object ID"
//!   (TestUserAuthEmailTrigger.al:241-272).
//! - Every User has a "User Property" row, whichever way it is written (TestUserPropertyPageInsert.al:
//!   185-237), the session user included (TestUserPropertySessionUser.al:312-330). Deleting the User
//!   deletes it, after the OnBeforeDelete subscribers and before the OnAfterDelete ones, for Delete
//!   and DeleteAll alike (TestUserDeleteCascadeOrder.al:26-84).
//!
//! Everything else the server does on a User write is not pinned and is refused when it could matter
//! (hard rule 4): a Windows Security ID or Application ID on the row, an address form the corpus
//! does not show, rows in the tables a delete cascades to, and ModifyAll, Rename and Truncate.
//! The full error texts and codes are pinned for the forms access-control/lt-av-user-insert/AvUserInsertErrors.al shows;
//! the other forms keep the fragments the older tests match (`user_checks`).

use crate::Session;
use crate::platform::{blank, pos};
use al_builtins::native::{AlError, Ctx, Signal};
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_data::{AccessContext, Row, Scan, Store, TableDef, WriteOutcome};
use std::sync::Arc;

pub(crate) const USER: u32 = 2_000_000_120;
pub(crate) const USER_PROPERTY: u32 = 2_000_000_121;
const ACCESS_CONTROL: u32 = 2_000_000_053;

// Field ids of the User table.
const F_SID: u32 = 1;
const F_NAME: u32 = 2;
const F_STATE: u32 = 4;
const F_WINDOWS_SID: u32 = 7;
const F_EMAIL: u32 = 11;
const F_APPLICATION_ID: u32 = 16;
// Field ids of User Property.
const P_SID: u32 = 1;
const P_OBJECT_ID: u32 = 7;
const P_TELEMETRY_ID: u32 = 10;
/// `State::Enabled`.
const ENABLED: i32 = 0;

const INVALID_EMAIL: &str = "You must specify a valid email address";
const EMAIL_TAKEN: &str = "is already being used by another user";
const NAME_TAKEN: &str = "The user name must be unique";

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

/// An error the server raises from the User table trigger whose full text and code are not pinned: the fragment the corpus
/// matches, with no code (kept from before the full texts were probed; see `user_checks`).
fn refuse<T>(message: &str) -> Result<T, Signal> {
    refuse_code(message, "")
}

/// An error the server raises from the User table trigger, with its full text and code.
fn refuse_code<T>(message: &str, code: &str) -> Result<T, Signal> {
    Err(Signal::Error(Box::new(AlError {
        message: message.into(),
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

/// Why an authentication email was not normalised.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EmailProblem {
    /// The text is not an address (no `@`): the server refuses it (pinned).
    Invalid,
    /// A form the corpus does not show: refused as unsupported.
    Unpinned(String),
}

/// The platform's normaliser for "Authentication Email": surrounding whitespace goes, `Name <a@b.c>`
/// becomes `a@b.c`, a comma separated list becomes its last address, white space alone becomes
/// empty. An address without `@` is invalid. Anything else a mail parser might accept or refuse
/// is `Unpinned`.
pub(crate) fn normalise_email(text: &str) -> Result<String, EmailProblem> {
    let unpinned = |what: &str| Err(EmailProblem::Unpinned(format!("{what} in the address '{text}'")));
    let all = text.trim();
    if all.is_empty() {
        return Ok(String::new());
    }
    if let Some(c) = all.chars().find(|c| "\"()[];\\".contains(*c) || c.is_control()) {
        return unpinned(&format!("the character '{}'", c.escape_default()));
    }
    let last = all.rsplit(',').next().unwrap_or(all).trim();
    if last.is_empty() {
        return unpinned("an empty entry");
    }
    let address = match (last.find('<'), last.find('>')) {
        (None, None) => last,
        (Some(open), Some(close)) if close == last.len() - 1 && open < close => {
            if last[open + 1..close].contains('<') {
                return unpinned("a nested angle bracket");
            }
            last[open + 1..close].trim()
        }
        _ => return unpinned("an unbalanced angle bracket"),
    };
    match address.matches('@').count() {
        0 => return Err(EmailProblem::Invalid),
        1 => {}
        _ => return unpinned("more than one '@'"),
    }
    let (local, domain) = address.split_once('@').unwrap_or_default();
    let word = |s: &str, extra: &str| {
        !s.is_empty()
            && !s.starts_with('.')
            && !s.ends_with('.')
            && s.chars().all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
    };
    if !word(local, "._+-") || !word(domain, ".-") {
        return unpinned("a local part or domain outside letters, digits and ._+-");
    }
    Ok(address.to_string())
}

fn text_of(v: &Value) -> &str {
    match v {
        Value::Text(s) | Value::Code(s) => s,
        _ => "",
    }
}

fn guid(v: &Value) -> Option<[u8; 16]> {
    match v {
        Value::Guid(g) => Some(*g),
        _ => None,
    }
}

fn is_blank_guid(v: &Value) -> bool {
    !matches!(v, Value::Guid(g) if g.iter().any(|b| *b != 0))
}

impl Session {
    fn table_id(&self, t: &TableDef) -> Option<u32> {
        self.engine.program.objects[t.object as usize].id
    }

    /// The compiled table `id`, if the program holds it (a table nothing reaches is not compiled).
    /// Profile id, profile app id and company of the User Personalization row of the session user, if the row is there.
    pub(crate) fn personalization_of_session_user(&self) -> Option<(String, [u8; 16], String)> {
        let t = self.table_def(2_000_000_073)?;
        let sid = self.host.identity.user_security_id();
        let row = self.store.get(&self.cx, &t, &[Value::Guid(sid)]).ok()??;
        let text = |p: usize| match &row[p] {
            Value::Text(s) | Value::Code(s) => s.to_string(),
            _ => String::new(),
        };
        let by_id = |id: u32| t.fields.iter().position(|f| f.id == id);
        let by_name = |n: &str| t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(n));
        let profile = text(by_id(9)?);
        let app = match &row[by_id(10)?] {
            Value::Guid(g) => *g,
            _ => [0; 16],
        };
        let company = text(by_name("Company")?);
        Some((profile, app, company))
    }

    pub(crate) fn table_def(&self, id: u32) -> Option<Arc<TableDef>> {
        self.compiled_table(id).and_then(|i| self.engine.tables[i].clone())
    }

    /// Whether `rec` is a row of the stored User table.
    pub(crate) fn is_user_row(&self, t: &TableDef, temporary: bool) -> bool {
        !temporary && self.table_id(t) == Some(USER)
    }

    pub(crate) fn all_rows(&self, cx: &AccessContext, t: &TableDef) -> Result<Vec<Row>, Signal> {
        let mut out = Vec::new();
        let mut after: Option<Vec<Value>> = None;
        loop {
            let scan = Scan {
                key: 0,
                filters: &[],
                ascending: true,
                after: after.as_deref(),
            };
            match self
                .store
                .next(cx, t, &scan)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?
            {
                Some(row) => {
                    after = Some(al_data::position(t, 0, &row));
                    out.push(row);
                }
                None => return Ok(out),
            }
        }
    }

    /// `ModifyAll`, `Rename` and `Truncate` on the User table would skip or change the triggers
    /// above in ways nothing pins.
    pub(crate) fn user_bulk_refusal(&self, t: &TableDef, temporary: bool, method: RecordOp) -> Result<(), Signal> {
        if self.is_user_row(t, temporary)
            && matches!(method, RecordOp::ModifyAll | RecordOp::Rename | RecordOp::Truncate)
        {
            return unsupported(format!(
                "{method} on the User table: the platform's User triggers on it are not pinned (docs/probes-needed.md)"
            ));
        }
        Ok(())
    }

    /// The part of the User insert and modify triggers both share: the refusals for fields whose
    /// rules are not pinned, the user name, and the authentication email. Normalises the email in
    /// `row`. `own` is the row's security id on a Modify (it does not compete with itself).
    fn user_checks(&self, t: &TableDef, row: &mut Row, insert: bool) -> Result<(), Signal> {
        if !text_of(&row[pos(t, F_WINDOWS_SID)]).is_empty() {
            return unsupported(
                "a User with a Windows Security ID: the platform's uniqueness rule for it is not pinned",
            );
        }
        if t.fields.iter().any(|f| f.id == F_APPLICATION_ID) && !is_blank_guid(&row[pos(t, F_APPLICATION_ID)]) {
            return unsupported("a User with an Application ID: the platform's validation of it is not pinned");
        }
        if is_blank_guid(&row[pos(t, F_SID)]) {
            return unsupported("a User with a blank User Security ID: what the platform does is not pinned");
        }
        let cx = self.cx.clone();
        let own = row[pos(t, F_SID)].clone();
        let others: Vec<Row> = self
            .all_rows(&cx, t)?
            .into_iter()
            .filter(|r| insert || guid(&r[pos(t, F_SID)]) != guid(&own))
            .collect();

        // User name: one user per name (TestUserAuthEmailTrigger.al:321-333). The Insert trigger is
        // said to do the same by the corpus header (TestUserAuthEmailTrigger.al:17-18).
        let name = text_of(&row[pos(t, F_NAME)]).to_string();
        // Full text and code, Insert and Modify alike (access-control/lt-av-user-insert/AvUserInsertErrors.al:79-120).
        if others.iter().any(|r| text_of(&r[pos(t, F_NAME)]) == name) {
            return refuse_code(&format!("{NAME_TAKEN}."), "UserTableUserNameMustBeUnique");
        }

        // Authentication email.
        let raw = text_of(&row[pos(t, F_EMAIL)]).to_string();
        let email = match normalise_email(&raw) {
            Ok(e) => e,
            // Full text and code of an Insert (AvUserInsertErrors.al:45-57). A Modify's are not pinned.
            Err(EmailProblem::Invalid) if insert => {
                return refuse_code(&format!("{INVALID_EMAIL}."), "CannotSpecifyAuthenticationEmail");
            }
            Err(EmailProblem::Invalid) => {
                return Err(Signal::Error(Box::new(al_builtins::native::unpinned_error_for(
                    "user-modify-invalid-email-text",
                ))));
            }
            Err(EmailProblem::Unpinned(why)) => {
                return unsupported(format!("User authentication email, {why}: not pinned"));
            }
        };
        let enabled = |r: &Row| matches!(r[pos(t, F_STATE)], Value::Option(ENABLED, _));
        if !email.is_empty() && enabled(row) {
            for r in others.iter().filter(|r| enabled(r)) {
                let theirs = text_of(&r[pos(t, F_EMAIL)]);
                if theirs == email {
                    // Full text and code of an Insert whose address needed no normalising (AvUserInsertErrors.al:59-77).
                    // Which form a normalised address takes in the text, and a Modify's text and code, are not pinned:
                    // they keep the fragment TestUserAuthEmailTrigger.al:182 and :219 match.
                    if insert && raw == email {
                        return refuse_code(
                            &format!(
                                "The specified authentication email address '{email}' {EMAIL_TAKEN}. You must specify a unique email address."
                            ),
                            "UserTableAuthEmailMustBeUnique",
                        );
                    }
                    return refuse(EMAIL_TAKEN);
                }
                if theirs.eq_ignore_ascii_case(&email) {
                    return unsupported("User authentication emails that differ only in case: not pinned");
                }
            }
        }
        if email != raw {
            row[pos(t, F_EMAIL)] = Value::Text(email.into());
        }
        Ok(())
    }

    /// Before the User row is stored by an Insert (any RunTrigger), below the OnBefore events and
    /// triggers. Normalises the email in `rec`.
    pub(crate) fn user_before_insert(&mut self, t: &TableDef, rec: &crate::records::Rec) -> Result<(), Signal> {
        let mut row = rec.borrow().fields.clone();
        self.user_checks(t, &mut row, true)?;
        rec.borrow_mut().fields = row;
        Ok(())
    }

    /// After the User row is stored: the User Property row every User has.
    pub(crate) fn user_after_insert(&mut self, t: &TableDef, rec: &crate::records::Rec) -> Result<(), Signal> {
        let Some(up) = self.table_def(USER_PROPERTY) else {
            // Nothing in the program can read the table, so its row cannot be seen.
            return Ok(());
        };
        let mut row = blank(&up);
        row[pos(&up, P_SID)] = rec.borrow().fields[pos(t, F_SID)].clone();
        row[pos(&up, P_TELEMETRY_ID)] = Value::Guid(self.new_guid());
        if let Some(p) = up.fields.iter().position(|f| f.id == 2_000_000_000) {
            row[p] = Value::Guid(self.new_guid());
        }
        let cx = self.cx.clone();
        // An existing row is left alone.
        let _ = self.store.insert(&cx, &up, row);
        Ok(())
    }

    /// Before a Modify stores the User row, below the OnBefore events and triggers.
    pub(crate) fn user_before_modify(&mut self, t: &TableDef, rec: &crate::records::Rec) -> Result<(), Signal> {
        let mut row = rec.borrow().fields.clone();
        let before = self.store.get(&self.cx.clone(), t, &t.primary_key(&row)).ok().flatten();
        self.user_checks(t, &mut row, false)?;
        let email = text_of(&row[pos(t, F_EMAIL)]).to_string();
        let stored_email = before.as_ref().map(|r| text_of(&r[pos(t, F_EMAIL)]).to_string());
        rec.borrow_mut().fields = row;
        // A changed email clears the user's Authentication Object ID (TestUserAuthEmailTrigger.al:241-272).
        let changed = stored_email.as_deref().is_some_and(|s| s != email);
        let up = self.table_def(USER_PROPERTY);
        if let Some(up) = up {
            let sid = rec.borrow().fields[pos(t, F_SID)].clone();
            let cx = self.cx.clone();
            if let Some(mut prop) = self
                .store
                .get(&cx, &up, &[sid])
                .map_err(|e| Signal::Unsupported(format!("{e}")))?
            {
                let object_id = pos(&up, P_OBJECT_ID);
                if changed {
                    if !text_of(&prop[object_id]).is_empty() {
                        prop[object_id] = Value::Text(String::new().into());
                        let _ = self.store.modify(&cx, &up, prop);
                    }
                } else if email.is_empty()
                    && stored_email.as_deref() == Some("")
                    && !text_of(&prop[object_id]).is_empty()
                {
                    // An empty email that stays empty: whether the server clears the object id
                    // here is not pinned.
                    return unsupported(
                        "Modify of a User whose authentication email is empty and stays empty, with an Authentication Object ID set: not pinned",
                    );
                }
            }
        }
        Ok(())
    }

    /// Before a Delete stores. With IsSaaS true (the default environment) the only enabled user holding
    /// the SUPER role cannot go (staging tests/al-language-onprem/session/PZUserDelete.al:
    /// DeleteAllUsers_AsTheOnlySuperUser_IsRefused, BC 29.0: the text and the code Dialog). The IsSaaS false
    /// side is not reachable on that tier, so on-prem it stays allowed (unprobed, docs/probes-needed.md).
    pub(crate) fn user_before_delete(&mut self, t: &TableDef, rec: &crate::records::Rec) -> Result<(), Signal> {
        if self.host.identity.environment() != al_host::Environment::Saas {
            return Ok(());
        }
        let Some(ac) = self.table_def(ACCESS_CONTROL) else {
            return Ok(());
        };
        let sid = rec.borrow().fields[pos(t, F_SID)].clone();
        let cx = self.cx.clone();
        let supers: Vec<Value> = self
            .all_rows(&cx, &ac)?
            .into_iter()
            .filter(|r| matches!(&r[pos(&ac, 2)], Value::Text(s) | Value::Code(s) if s.eq_ignore_ascii_case("SUPER")))
            .map(|r| r[pos(&ac, P_SID)].clone())
            .collect();
        if !supers.iter().any(|g| guid(g) == guid(&sid)) {
            return Ok(());
        }
        let mut other_enabled = false;
        for u in self.all_rows(&cx, t)? {
            let other = &u[pos(t, F_SID)];
            if guid(other) != guid(&sid)
                && supers.iter().any(|g| guid(g) == guid(other))
                && matches!(u[pos(t, F_STATE)], Value::Option(ENABLED, _))
            {
                other_enabled = true;
                break;
            }
        }
        if other_enabled {
            return Ok(());
        }
        Err(Signal::Error(Box::new(AlError {
            message: "There should be at least one enabled 'SUPER' user.".into(),
            code: "Dialog".into(),
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// After the User row is deleted: its User Property row goes (TestUserDeleteCascadeOrder.al:26-84),
    /// and so do its Access Control rows (staging tests/al-language-onprem/session/PZUserDelete.al, BC 29.0, with and
    /// without RunTrigger).
    pub(crate) fn user_after_delete(&mut self, t: &TableDef, rec: &crate::records::Rec) -> Result<(), Signal> {
        let sid = rec.borrow().fields[pos(t, F_SID)].clone();
        let cx = self.cx.clone();
        if let Some(up) = self.table_def(USER_PROPERTY) {
            if let Ok(WriteOutcome::Done | WriteOutcome::NotFound) =
                self.store.delete(&cx, &up, std::slice::from_ref(&sid))
            {}
        }
        if let Some(ac) = self.table_def(ACCESS_CONTROL) {
            let mine: Vec<Row> = self
                .all_rows(&cx, &ac)?
                .into_iter()
                .filter(|r| guid(&r[pos(&ac, P_SID)]) == guid(&sid))
                .collect();
            for row in mine {
                let pk = ac.primary_key(&row);
                if let Err(e) = self.store.delete(&cx, &ac, &pk) {
                    return unsupported(format!("deleting an Access Control row of a deleted User: {e}"));
                }
            }
        }
        Ok(())
    }
}

/// The User Property row of the session user, written with the other platform rows at start
/// (TestUserPropertySessionUser.al:312-330).
pub(crate) fn property_seed(up: &TableDef, sid: [u8; 16], telemetry: [u8; 16]) -> Row {
    let mut r = blank(up);
    r[pos(up, P_SID)] = Value::Guid(sid);
    r[pos(up, P_TELEMETRY_ID)] = Value::Guid(telemetry);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(s: &str) -> String {
        normalise_email(s).unwrap()
    }

    /// access-control/TestUserAuthEmailTrigger.al:44-125.
    #[test]
    fn the_pinned_forms_are_normalised() {
        assert_eq!(ok("   a@t.example.com    "), "a@t.example.com");
        assert_eq!(ok("Test Person <a@t.example.com>"), "a@t.example.com");
        assert_eq!(ok("  Test Person <a@t.example.com>  "), "a@t.example.com");
        assert_eq!(ok("a@t.example.com, d@t.example.com"), "d@t.example.com");
        assert_eq!(ok("a@t.example.com"), "a@t.example.com");
        assert_eq!(ok("    "), "");
        assert_eq!(ok(""), "");
    }

    /// access-control/TestUserAuthEmailTrigger.al:143-159.
    #[test]
    fn an_address_without_an_at_sign_is_invalid() {
        assert_eq!(normalise_email("Abc.example.com"), Err(EmailProblem::Invalid));
    }

    #[test]
    fn forms_the_corpus_does_not_show_are_not_decided() {
        for s in [
            "a@b@c.d",
            "\"Name\" <a@b.c>",
            "a@b.c;d@e.f",
            "a b@c.d",
            "<a@b.c",
            "a@b.c,",
            "@b.c",
            "a@",
        ] {
            assert!(matches!(normalise_email(s), Err(EmailProblem::Unpinned(_))), "{s}");
        }
    }
}
