//! `Evaluate` into an Option: a variable (`variable`), a `var` parameter declared without members
//! (`memberless_var_parameter`, `memberless_var_array_element`) and a field (`field`).

use super::{Parsed, unsupported};
use crate::native::{Ctx, Signal};
use crate::value::Value;
use std::sync::Arc;

/// An Option reads an ordinal or a member name (types/PROptionInteger.al:Option_InCaseAndLoopBounds: `1` is B, `C` is C).
/// A variable: an ordinal past the members is kept (`3` of three members formats as `3`, format-evaluate/P86Eval.al:1364,
/// :1376) and one above i32 refuses; a text that is neither an ordinal nor a member name is rejected, and the error names
/// the members (P86Eval.al, codeunit "LT P86 Eval Option").
pub(super) fn variable(c: &mut dyn Ctx, members: &Arc<[String]>, text: &crate::text::Txt) -> Result<Parsed, Signal> {
    let t = trimmed(text);
    // The empty text, a blank text and NUL name the member declared as " " at any place (format-evaluate/format-matrix/
    // FMOptionBlankVar.al, FMEvalOption.al:64-69, :262-267), and no member declared as "" or left empty (ScT70062.al:
    // Option_EvaluateByName `empty`): that member has no name. Names are read in any case.
    let by_name = if t.is_empty() {
        members.iter().position(|m| m == " ")
    } else {
        members.iter().position(|m| m.eq_ignore_ascii_case(t))
    };
    Ok(match (ordinal(t), by_name) {
        (Some(n), _) => match i32::try_from(n) {
            Ok(n) => Parsed::Value(Value::Option(n, members.clone())),
            Err(_) => {
                return unsupported(format!("Evaluate of the ordinal `{text}` into an Option (not pinned)"));
            }
        },
        (None, Some(n)) => Parsed::Value(Value::Option(n as i32, members.clone())),
        _ if c.ret_used() => return Ok(Parsed::Rejected),
        // A TAB shows as `\t` in the error (format-evaluate/OptionEvaluateCaptions.al:330
        // ResultUnused_Errors `var tab`), a lone NUL as `\u0000` (FMOptionBlankVar.al st_nul_text); the other
        // control characters are not pinned.
        _ if text.as_str() != "\0" && text.chars().any(|ch| ch.is_control() && ch != '\t') => {
            return unsupported(
                "Evaluate into an Option variable failing with the result unused, of a text with a control character other than TAB (not pinned)",
            );
        }
        _ => {
            return crate::native::raise(
                format!(
                    "'{}' is not an option. The existing options are: {}",
                    text.replace('\t', "\\t").replace('\0', "\\u0000"),
                    members.join(",")
                ),
                "InvalidOptionString",
            );
        }
    })
}

/// The text a variable reads: NUL alone reads as the empty text (format-evaluate/format-matrix/FMEvalOption.al:222-224,
/// corpus PR 1256), any other text without its blanks.
fn trimmed(text: &crate::text::Txt) -> &str {
    if text.as_str() == "\0" { "" } else { text.trim() }
}

/// The ordinal a trimmed text gives a variable: digits after an optional `+` that fit a u64 (`3` of three members,
/// format-evaluate/P86Eval.al:1364, :1376; `00`, `+1` and ` 1 `, :1450-1454, :1462-1466). A longer text is no ordinal;
/// `variable` refuses one above i32.
fn ordinal(t: &str) -> Option<u64> {
    if t.starts_with(|ch: char| ch.is_ascii_digit() || ch == '+') {
        t.parse::<u64>().ok()
    } else {
        None
    }
}

/// `Evaluate` into a `var` Option parameter declared without members (the lowering's `memberless_var_option_parameter`), or
/// into an element of a `var` array parameter of such Options: the parameter holds the caller's members (al-vm
/// `var_option::var_binding`), and the text is read against them as by any Option variable (`variable`): a name in any case
/// and without its blanks, an ordinal past the members kept, a rejected text false with the result used and the value kept,
/// as a statement the error that lists the caller's members, also for the empty text (assumption var-option-caller-members,
/// corpus PR 1432, OptionVarMemberlessEvaluate.al:77-244, :170), also after the routine stored into the parameter (corpus
/// PR 1439, OptionVarMemberlessWrites.al:130, :162, :212). A blank text is measured from callers whose members all have a name
/// (First,Second,Third, OptionVarMemberlessEvaluate.al:170-185; " ",First, OptionVarMemberlessEvaluate.al:286-325); from a
/// caller with an empty member (`,A,B`, or a caller without members) it refuses.
pub(super) fn memberless_var_parameter(a: &[Value]) -> Result<(), Signal> {
    let Value::Option(_, members) = &a[0] else {
        return Ok(());
    };
    let blank = crate::native::text_of(&a[1]).is_some_and(|t| trimmed(&t).is_empty());
    if blank && members.iter().any(String::is_empty) {
        return unsupported(
            "Evaluate of a blank text into a `var` Option parameter declared without members, from a caller with an empty member or without members (not measured, assumption var-option-caller-members)",
        );
    }
    crate::assumptions::assume(crate::assumptions::Assumption::VarOptionCallerMembers);
    Ok(())
}

/// `Evaluate` into an element of a `var` array parameter of Options declared without members: the element holds the caller's
/// members and keeps them when it is stored (al-vm `var_option::element_store`). Measured for a member name with the result used
/// (OptionVarMemberlessEvaluate.al:552, assumption var-option-caller-members); a text read as an ordinal or a name is stored the
/// same way. A rejected text is not measured: `parsed` false or an error refuses.
pub(super) fn memberless_var_array_element(parsed: Result<Value, Signal>) -> Result<Value, Signal> {
    match parsed {
        Ok(Value::Boolean(false)) | Err(Signal::Error(_)) => unsupported(
            "Evaluate of a text that names no member into an element of a `var` array of Options declared without members (not measured, assumption var-option-caller-members)",
        ),
        other => {
            crate::assumptions::assume(crate::assumptions::Assumption::VarOptionCallerMembers);
            other
        }
    }
}

/// An Option field reads a member name and an ordinal below the member count.
pub(super) fn field(c: &mut dyn Ctx, members: &Arc<[String]>, text: &crate::text::Txt) -> Result<Parsed, Signal> {
    // Blank, empty and NUL texts name the member declared as " " (an Option field, format-evaluate/format-matrix/
    // FMOptionBlankField.al, Observe `ev_empty`/`ev_sp`: `Yes|<ordinal>`), else read false; as a statement not pinned.
    let blank = text.trim().is_empty() || text.as_str() == "\0";
    let t = if blank { "" } else { text.trim() };
    if blank && !members.iter().any(|m| m == " ") {
        if c.ret_used() {
            return Ok(Parsed::Rejected);
        }
        return unsupported(
            "Evaluate of a blank text into an Option field without a member named \" \" as a statement (not pinned)",
        );
    }
    let by_name = if blank {
        members.iter().position(|m| m == " ")
    } else {
        members.iter().position(|m| m.eq_ignore_ascii_case(t))
    };
    Ok(
        match (t.parse::<usize>().ok().filter(|n| *n < members.len()), by_name) {
            (Some(n), _) | (None, Some(n)) => Parsed::Value(Value::Option(n as i32, members.clone())),
            // A platform option without an OptionCaption rejects a number past its members, as far as measured:
            // TableConnectionType 5 to 10 (format-evaluate/W5TableConnections.al:117-136, TableConnectionType_Members), ExecutionMode
            // 2 to 12 (format-evaluate/PnC11SystemEnums.al:13-52, ExecutionMode_Members). Larger numbers are not pinned.
            _ if crate::sysenum::rejected_number_past_members(members)
                .is_some_and(|last| t.parse::<usize>().is_ok_and(|n| (members.len()..=last).contains(&n))) =>
            {
                Parsed::Rejected
            }
            // An OptionCaption may name it (recordref-fieldref/fieldref/TestPrecompiledFieldOptionCaption.al:Evaluate_Caption_SelectsThatMember);
            // the value does not carry the captions.
            _ => {
                return unsupported(format!(
                    "Evaluate of `{text}` into an Option: neither an ordinal nor a member name (an OptionCaption could name it, not pinned)"
                ));
            }
        },
    )
}
