//! `SessionSettings` and `Notification`: in-memory value objects (corpus session-system/TestSessionSettings.al,
//! ui-handlers/TestNotificationHandler.al, session-system/TestCalcDateNotificationContracts.al,
//! session-system/TestMiscTypes.al).

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{LibraryValue, NotificationState, SessionSettingsState, Value};

type Out = Result<Value, Signal>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn with_settings<R>(r: Option<&mut Value>, f: impl FnOnce(&mut SessionSettingsState) -> R) -> Result<R, Signal> {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("SessionSettings method without a receiver");
    };
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::SessionSettings(SessionSettingsState::default());
    }
    match &mut *lib {
        LibraryValue::SessionSettings(s) => Ok(f(s)),
        other => unsupported(format!("SessionSettings method on {}", other.describe())),
    }
}

fn with_notification<R>(r: Option<&mut Value>, f: impl FnOnce(&mut NotificationState) -> R) -> Result<R, Signal> {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("Notification method without a receiver");
    };
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::Notification(NotificationState::default());
    }
    match &mut *lib {
        LibraryValue::Notification(s) => Ok(f(s)),
        other => unsupported(format!("Notification method on {}", other.describe())),
    }
}

fn text(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of) {
        Some(t) => Ok(t.to_string()),
        None => unsupported(format!("a text argument, got {}", a.get(i).describe())),
    }
}

/// A text that is only stored (a message, a data value): a call stack text is carried (`text_of_carrying_call_stack`).
fn text_stored(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of_carrying_call_stack) {
        Some(t) => Ok(t.to_string()),
        None => unsupported(format!("a text argument, got {}", a.get(i).describe())),
    }
}

fn int(a: &[Value], i: usize) -> Result<i32, Signal> {
    match a.get(i) {
        Some(Value::Integer(n)) => Ok(*n),
        other => unsupported(format!("an Integer argument, got {}", other.describe())),
    }
}

/// Property accessors: with an argument they set, without one they get. A property nobody wrote
/// is refused on read (see [`SessionSettingsState`]).
fn language_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        // Untouched settings read as 0, empty and the null Guid (session-system/TestProbeSessionMiscContracts.al:
        // SessionSettings_PristineProperties).
        return with_settings(r, |s| s.language_id.unwrap_or(0)).map(Value::Integer);
    }
    let n = int(a, 0)?;
    with_settings(r, |s| s.language_id = Some(n)).map(|()| Value::Null)
}

fn locale_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_settings(r, |s| s.locale_id.unwrap_or(0)).map(Value::Integer);
    }
    let n = int(a, 0)?;
    with_settings(r, |s| s.locale_id = Some(n)).map(|()| Value::Null)
}

fn time_zone(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_settings(r, |s| s.time_zone.clone().unwrap_or_default()).map(|s| Value::Text(s.into()));
    }
    let t = text(a, 0)?;
    with_settings(r, |s| s.time_zone = Some(t)).map(|()| Value::Null)
}

fn profile_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_settings(r, |s| s.profile_id.clone().unwrap_or_default()).map(|s| Value::Text(s.into()));
    }
    let t = text(a, 0)?;
    with_settings(r, |s| s.profile_id = Some(t)).map(|()| Value::Null)
}

fn profile_app_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_settings(r, |s| s.profile_app_id.unwrap_or([0; 16])).map(Value::Guid);
    }
    let Value::Guid(g) = a[0] else {
        return unsupported(format!("ProfileAppId with {}", a[0].describe()));
    };
    with_settings(r, |s| s.profile_app_id = Some(g)).map(|()| Value::Null)
}

/// An untouched instance is the empty company (session-system/TestSessionSettings.al:575-595).
fn company(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_settings(r, |s| Value::Text(s.company.clone().into()));
    }
    let t = text(a, 0)?;
    with_settings(r, |s| s.company = t).map(|()| Value::Null)
}

/// System profiles are deprecated: the setter discards its argument and the property stays false
/// (session-system/TestSessionSettings.al:172-215).
fn profile_system_scope(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        Ok(Value::Boolean(false))
    } else {
        Ok(Value::Null)
    }
}

/// `Init()` loads the language, locale and time zone of the running session and replaces what
/// the instance held (session-system/TestSessionSettings.al:371-455). The company stays empty, as BC 27.5
/// leaves it (the same test, "MEASURED"). The profile properties are not pinned: they go back to
/// "nobody set", which a read refuses.
fn init(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let Some((language, locale, zone)) = c.session_locale() else {
        return unsupported(
            "SessionSettings.Init reads the language, locale and time zone of the running session, which this context does not have (host value, ADR-013)",
        );
    };
    // The profile and the company are those of the session user's User Personalization row (session/
    // W6UserPersonalization.al:SessionSettings_Init_ComparedWithTheRow, BC 27.5 and 29.0).
    let profile = c.session_profile();
    with_settings(r, |s| {
        *s = SessionSettingsState {
            language_id: Some(language),
            locale_id: Some(locale),
            time_zone: Some(zone),
            profile_id: profile.as_ref().map(|p| p.0.clone()),
            profile_app_id: profile.as_ref().map(|p| p.1),
            company: profile.map(|p| p.2).unwrap_or_default(),
        }
    })
    .map(|()| Value::Null)
}

/// `RequestSessionUpdate(false)` asks the client for a new session and is routed to the test's
/// `[SessionSettingsHandler]`, which gets the assigned settings; the instance stays as it was and
/// so does the running session (session-system/TestSessionSettings.al:464-545). `true` also writes the
/// settings to "User Personalization", which the corpus does not exercise: refused.
fn request_update(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        Some(Value::Boolean(false)) => {}
        Some(Value::Boolean(true)) => {
            return unsupported(
                "SessionSettings.RequestSessionUpdate(true) writes User Personalization (not pinned by the corpus)",
            );
        }
        other => {
            return unsupported(format!(
                "SessionSettings.RequestSessionUpdate with {}",
                other.describe()
            ));
        }
    }
    let sender = match &r {
        Some(Value::Library(_, cell)) => cell.clone(),
        _ => return unsupported("SessionSettings.RequestSessionUpdate without a receiver"),
    };
    let state = with_settings(r, |s| s.clone())?;
    let copy = Value::Library(
        "SessionSettings".into(),
        std::rc::Rc::new(std::cell::RefCell::new(LibraryValue::SessionSettings(state))),
    );
    let copy = c.ui_session_settings(copy.clone())?.unwrap_or(copy);
    // The handler's `var` parameter is the sender's variable: what it changed is there afterwards, whatever it
    // answers (session-system/W5SessionSettings.al:SessionSettings_Update_*, BC 27.5 and 29.0: the handler sets the language
    // to 1031 and the sender reads 1031; a second call sees 1031).
    let after = match &copy {
        Value::Library(_, cell) => match &*cell.borrow() {
            LibraryValue::SessionSettings(s) => Some(s.clone()),
            _ => None,
        },
        _ => None,
    };
    if let Some(after) = after {
        *sender.borrow_mut() = LibraryValue::SessionSettings(after);
    }
    Ok(Value::Null)
}

// ---- Notification ------------------------------------------------------------------------

/// `Message(text)` sets, `Message()` reads (ui-handlers/TestNotificationHandler.al:28-35). A message
/// nobody set is empty (ui-handlers/TestNotificationSendRecallContracts.al:65).
fn n_message(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_notification(r, |n| n.message.clone().unwrap_or_default()).map(|s| Value::Text(s.into()));
    }
    let t = text_stored(a, 0)?;
    with_notification(r, |n| n.message = Some(t)).map(|()| Value::Null)
}

/// `Id := guid` sets, `Id` reads (ui-handlers/TestNotificationHandler.al:55-56). The id of a
/// notification nobody gave one, and that was not sent, is the null Guid
/// (ui-handlers/TestNotificationSendRecallContracts.al:69).
fn n_id(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        Some(Value::Guid(g)) => {
            let g = *g;
            with_notification(r, |n| n.id = Some(g)).map(|()| Value::Null)
        }
        None => with_notification(r, |n| n.id.unwrap_or([0; 16])).map(Value::Guid),
        // A Text takes the way a Text assigns into a Guid (record/TestRecordGetBySystemId*.al; Microsoft's Guided
        // Experience tests write `Notification.Id := '9d730988-...'`).
        Some(v @ (Value::Text(_) | Value::Code(_))) => {
            let Value::Guid(g) = crate::ops::convert(v.clone(), &crate::value::RtType::Guid)? else {
                return unsupported("Notification.Id with a Text that is no Guid");
            };
            with_notification(r, |n| n.id = Some(g)).map(|()| Value::Null)
        }
        Some(other) => unsupported(format!("Notification.Id with {}", other.describe())),
    }
}

/// Setting a scope must not throw (session-system/TestCalcDateNotificationContracts.al:182-188); reading one
/// back is not pinned.
fn n_scope(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        Some(Value::Option(o, _)) => {
            let o = *o;
            with_notification(r, |n| n.scope = Some(o)).map(|()| Value::Null)
        }
        // A notification nobody set a scope on is LocalScope (ui-handlers/TestNotificationSendRecallContracts.al:66).
        None => with_notification(r, |n| n.scope.unwrap_or(0)).map(|o| {
            let members: std::sync::Arc<[String]> = crate::sysenum::option_members("NotificationScope")
                .expect("a system option")
                .into();
            Value::Option(o, members)
        }),
        Some(other) => unsupported(format!("Notification.Scope with {}", other.describe())),
    }
}

/// Data is a text map: a second `SetData` of a name replaces its value
/// (session-system/TestCalcDateNotificationContracts.al:142-180).
fn n_set_data(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    // A Guid value is its text form: types/guid/GuidAsTextArgument.al:66
    // Guid_AsTextArgument_NotificationData (SetData with a Guid, GetData reads `{XXXXXXXX-...}`).
    let v = match a.get(1) {
        Some(Value::Guid(g)) => crate::format::format_guid(g),
        _ => text_stored(a, 1)?,
    };
    let k = text(a, 0)?;
    with_notification(r, |n| match n.data.iter_mut().find(|(name, _)| *name == k) {
        Some(slot) => slot.1 = v,
        None => n.data.push((k, v)),
    })
    .map(|()| Value::Null)
}

fn n_has_data(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let k = text(a, 0)?;
    with_notification(r, |n| Value::Boolean(n.data.iter().any(|(name, _)| *name == k)))
}

/// A name nobody set reads empty (ui-handlers/TestNotificationSendRecallContracts.al:68).
fn n_get_data(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let k = text(a, 0)?;
    with_notification(r, |n| {
        n.data
            .iter()
            .find(|(name, _)| *name == k)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    })
    .map(|s| Value::Text(s.into()))
}

/// Stored, never run: an action only matters when the notification is sent to a client
/// (session-system/TestSystemExtended.al:553-575, TestMiscComplete.al:106-119 pin that it does not throw).
fn n_add_action(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (caption, codeunit, method) = (text(a, 0)?, int(a, 1)?, text(a, 2)?);
    let description = if a.len() > 3 { Some(text(a, 3)?) } else { None };
    with_notification(r, |n| n.actions.push((caption, codeunit, method, description))).map(|()| Value::Null)
}

/// A copy of the notification for a handler, which gets its own (ui-handlers/TestNotificationHandler.al:
/// 74-79 read the message from the parameter).
fn notification_copy(r: Option<&mut Value>) -> Result<Value, Signal> {
    let state = with_notification(r, |n| n.clone())?;
    Ok(Value::Library(
        "Notification".into(),
        std::rc::Rc::new(std::cell::RefCell::new(LibraryValue::Notification(state))),
    ))
}

/// `Send()` gives a notification without one an Id, goes to the test's `[SendNotificationHandler]`
/// if there is one, and answers true whatever the handler answers. What the handler changed in its
/// `var Notification` is seen by the sender (ui-handlers/TestNotificationSendRecallContracts.al:59-93).
fn n_send(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let Some(recv) = r else {
        return unsupported("Notification.Send without a receiver");
    };
    // A notification with no message cannot be sent (errors/errorinfo/EnTNotification.al:Send_EmptyMessage); no handler runs.
    if with_notification(Some(&mut *recv), |n| n.message.is_none())? {
        return Err(Signal::Error(Box::new(crate::native::AlError {
            message: "Message cannot be empty when sending a Notification.".into(),
            code: "Notification".into(),
            call_stack: String::new(),
            ..Default::default()
        })));
    }
    let fresh = c.new_guid();
    let id = with_notification(Some(&mut *recv), |n| *n.id.get_or_insert(fresh))?;
    let copy = notification_copy(Some(&mut *recv))?;
    if let Some(Value::Library(_, cell)) = c.ui_send_notification(copy, id)? {
        let left = match &*cell.borrow() {
            LibraryValue::Notification(s) => Some(s.clone()),
            _ => None,
        };
        if let Some(left) = left {
            with_notification(Some(recv), |n| {
                n.message = left.message;
                n.scope = left.scope;
                n.data = left.data;
                n.actions = left.actions;
            })?;
        }
    }
    Ok(Value::Boolean(true))
}

/// `Recall()` of a sent notification is true, of one never sent false
/// (ui-handlers/TestNotificationSendRecallContracts.al:95-131). A notification with no Id goes to the UI as the null
/// Guid: `Ctx::ui_recall_notification` decides what a never-sent one does with a recall handler bound.
fn n_recall(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let Some(recv) = r else {
        return unsupported("Notification.Recall without a receiver");
    };
    let id = with_notification(Some(&mut *recv), |n| n.id)?.unwrap_or([0; 16]);
    let copy = notification_copy(Some(recv))?;
    c.ui_recall_notification(copy, id).map(Value::Boolean)
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    Some(match (sig.receiver.as_str(), sig.name.as_str()) {
        ("SessionSettings", "LanguageId") => language_id,
        ("SessionSettings", "LocaleId") => locale_id,
        ("SessionSettings", "TimeZone") => time_zone,
        ("SessionSettings", "ProfileId") => profile_id,
        ("SessionSettings", "ProfileAppId") => profile_app_id,
        ("SessionSettings", "Company") => company,
        ("SessionSettings", "ProfileSystemScope") => profile_system_scope,
        ("SessionSettings", "Init") => init,
        ("SessionSettings", "RequestSessionUpdate") => request_update,
        ("Notification", "Message") => n_message,
        ("Notification", "Scope") => n_scope,
        ("Notification", "Id") => n_id,
        ("Notification", "SetData") => n_set_data,
        ("Notification", "HasData") => n_has_data,
        ("Notification", "GetData") => n_get_data,
        ("Notification", "AddAction") => n_add_action,
        ("Notification", "Send") => n_send,
        ("Notification", "Recall") => n_recall,
        _ => return None,
    })
}
