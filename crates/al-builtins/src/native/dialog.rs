//! The Dialog methods, and what Clear does to a Dialog.

use super::*;

/// A cleared Dialog, opened or not, is a closed one: Update and Close raise that it is not open and Open works again; ClearAll
/// and Clear of a `var` parameter do the same (al-runner issue 6656; corpus branch al-runtime-rs-staging-y64 85d86e7b,
/// pages/lt-page-runtime/P82Dialog.al:442-547).
pub(super) fn cleared(name: &crate::library_kind::LibraryKind) -> Value {
    Value::Library(
        name.clone(),
        std::rc::Rc::new(std::cell::RefCell::new(LibraryValue::Empty)),
    )
}

/// The numbers of the `#n####` and `@n@@@@` fields of a dialog template.
fn dialog_fields(template: &str) -> Vec<i32> {
    let c: Vec<char> = template.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if matches!(c[i], '#' | '@') {
            let mut j = i + 1;
            while j < c.len() && c[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
                if let Ok(n) = c[i + 1..j].iter().collect::<String>().parse::<i32>() {
                    out.push(n);
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn dialog_not_open() -> Signal {
    Signal::Error(Box::new(AlError {
        message: "The operation failed because the dialog box is not open.".into(),
        code: "Dialog".into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// `Dialog.Open(Template)`: the dialog is open until `Close`; opening an open one is not an error
/// (pages/lt-page-runtime/P82Dialog.al:Dialog_EdgeCases `open-reopen`, `open-empty-template`).
pub(super) fn dialog_open(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    if let (Some(Value::Library(_, h)), Some(t)) = (r, a.first().and_then(text_of)) {
        *h.borrow_mut() = LibraryValue::Text(t.to_string());
    }
    Ok(Value::Null)
}

/// `Dialog.Update(Number, Value, ...)`. An unopened or closed dialog raises, and so does a number that is
/// no field of the template, naming the template with each field as `%s` (errorinfo/lt-errnotif/EnTDialogSession.al:
/// Dialog_Errors, Dialog_UpdateWithoutOpen; pages/lt-page-runtime/P82Dialog.al:Dialog_EdgeCases). Only `#` fields have their
/// template text pinned: a failing number against a template with `@` fields is refused. `Update()` is a no-op
/// on an open dialog (handlers/TestHandlersDialogProgressNoOp.al).
pub(super) fn dialog_update(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let template = match r {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::Text(t) => t.clone(),
            _ => return Err(dialog_not_open()),
        },
        _ => return unsupported("Dialog.Update on something that is no Dialog"),
    };
    let fields = dialog_fields(&template);
    for n in a.iter().step_by(2) {
        let Value::Integer(n) = n else {
            return unsupported("Dialog.Update with a field number that is not an Integer");
        };
        if !fields.contains(n) {
            if template.contains('@') {
                return unsupported(
                    "Dialog.Update of a field number the template lacks, with an @ field (message not pinned)",
                );
            }
            let mut shown = String::new();
            let chars: Vec<char> = template.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '#' && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
                    i += 1;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '#') {
                        i += 1;
                    }
                    shown.push_str("%s");
                } else {
                    shown.push(chars[i]);
                    i += 1;
                }
            }
            return Err(Signal::Error(Box::new(AlError {
                message: format!("No parameter with index '{n}' was found in the format string '{shown}'."),
                code: "FormatStringParameterNotFound".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
    }
    Ok(Value::Null)
}

/// `Dialog.HideSubsequentDialogs(Hide)`: a headless session shows no dialog, so it changes nothing that AL sees: Open, Update and
/// Close and their errors behave the same after `true` and after `false`, also on a dialog opened before and on another Dialog
/// variable (errorinfo/lt-errnotif/EnTDialogSession.al UJ_Dialog_HideSubsequent_OpenUpdateClose, UJ_Dialog_HideSubsequent_OpenedBefore,
/// corpus branch al-runtime-rs-staging-uj 1dc037a8). Microsoft's code calls it as a statement (System Application, Apply Retention
/// Policy Impl.). The value it returns is pinned only after `true` (Yes); any other use of the result is an assumption.
pub(super) fn dialog_hide_subsequent(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    // Pinned by errorinfo/lt-errnotif/EnTDialogSession.al UJ_Dialog_HideSubsequent_OpenUpdateClose, UJ_Dialog_HideSubsequent_OpenedBefore (was assumption `dialog-hide-subsequent-dialogs-no-headless-effect`).
    let hide = match a.first() {
        Some(Value::Boolean(b)) => Some(*b),
        None => None,
        Some(_) => return unsupported("Dialog.HideSubsequentDialogs with an argument that is not a Boolean"),
    };
    if c.ret_used() && hide != Some(true) {
        return unsupported("the result of Dialog.HideSubsequentDialogs without the argument true (not pinned)");
    }
    Ok(Value::Boolean(true))
}

/// `Dialog.Close()`: closing a dialog that is not open raises.
pub(super) fn dialog_close(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    match r {
        Some(Value::Library(_, h)) => {
            let open = matches!(&*h.borrow(), LibraryValue::Text(_));
            if !open {
                return Err(dialog_not_open());
            }
            *h.borrow_mut() = LibraryValue::Empty;
            Ok(Value::Null)
        }
        _ => unsupported("Dialog.Close on something that is no Dialog"),
    }
}
