//! `Evaluate` into an Option: a variable (`variable`) and a field (`field`).

use super::{Parsed, unsupported};
use crate::native::{Ctx, Signal};
use crate::value::Value;
use std::sync::Arc;

/// An Option reads an ordinal or a member name (types/PROptionInteger.al:Option_InCaseAndLoopBounds: `1` is B, `C` is C).
/// A variable: an ordinal of any size is kept (`3` of three members formats as `3`), a text that is neither an ordinal
/// nor a member name is rejected, and the error names the members (P86Eval.al, codeunit "LT P86 Eval Option").
pub(super) fn variable(c: &mut dyn Ctx, members: &Arc<[String]>, text: &crate::text::Txt) -> Result<Parsed, Signal> {
    // NUL alone reads as the empty text (format-evaluate/format-matrix/FMEvalOption.al:222-224, corpus PR 1256).
    let t = if text.as_str() == "\0" { "" } else { text.trim() };
    // The empty text, a blank text and NUL name the member declared as " " at any place (format-evaluate/format-matrix/
    // FMOptionBlankVar.al, FMEvalOption.al:64-69, :262-267), and no member declared as "" or left empty (ScT70062.al:
    // Option_EvaluateByName `empty`): that member has no name. Names are read in any case.
    let by_name = if t.is_empty() {
        members.iter().position(|m| m == " ")
    } else {
        members.iter().position(|m| m.eq_ignore_ascii_case(t))
    };
    let ordinal = if t.starts_with(|ch: char| ch.is_ascii_digit() || ch == '+') {
        t.parse::<u64>().ok()
    } else {
        None
    };
    Ok(match (ordinal, by_name) {
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
