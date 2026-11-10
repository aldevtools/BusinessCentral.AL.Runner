//! `Evaluate` into an Enum.

use super::{Parsed, unsupported};
use crate::native::{Ctx, Signal};
use crate::value::Value;

/// An enum reads an ordinal (also one no member has), a member name in any case or a caption, with format 0 and with format 9
/// (types/ProbeTypeFormsAndFilters.al:Enum_Evaluate_Forms, BC 27.5; the InitValue of a field reads a name the same way,
/// types/TestBlankEnumInitTests.Codeunit.al:92). A text that is none of these is rejected.
pub(super) fn enum_text(c: &mut dyn Ctx, id: &u32, text: &crate::text::Txt) -> Result<Parsed, Signal> {
    let Some(members) = c.enum_members(*id) else {
        return unsupported("Evaluate into an Enum without its members");
    };
    // White space around the text is ignored (types/W7EvaluateTests.al:EvaluateIntoAnEnum_DefaultFormat,
    // ` Second ` gives Second).
    let original = text.clone();
    // NUL alone reads as the empty text (format-evaluate/format-matrix/FMEvalEnum.al:214-216, corpus PR 1256).
    let text = if text.as_str() == "\0" { "" } else { text.trim() };
    let digits = text.strip_prefix('+').unwrap_or(text);
    let ordinal = (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
        .then(|| digits.parse::<i32>().ok())
        .flatten();
    // A sign is allowed before the digits and an ordinal no member has is kept, with format 9 too
    // (P86Eval.al, codeunit "LT P86 Eval Enum": `+5`, `00`, `11`).
    // The captions a translation file could change in the language in force are refused (ADR-053 step 1).
    if let Some(why) = c.enum_caption_refused(*id, None) {
        return unsupported(why);
    }
    let captions = c.enum_captions(*id);
    // Under a translation file (ADR-053 step 2) only a translated caption is accepted, as al-runner issue 5772's real-BC
    // run shows (`Vrijgegeven` under 2067); whether an ordinal, a member name or the source caption is still accepted,
    // and what the error lists, is not pinned (ADR-053 P11).
    let mut forced = None;
    if let Some(source) = c.enum_source_captions(*id) {
        let translated = captions.iter().find(|(o, n)| {
            !text.is_empty() && n.eq_ignore_ascii_case(text) && source.iter().any(|(so, sn)| so == o && sn != n)
        });
        match translated {
            Some((o, _)) => forced = Some(*o),
            None => {
                return unsupported(format!(
                    "Evaluate of `{text}` into an Enum whose captions a translation file changes (only a translated caption is pinned)"
                ));
            }
        }
    }
    let found = forced
        .or(ordinal)
        .or_else(|| {
            members
                .iter()
                // A blank member matches the blank text (types/TestBlankEnumInitTests.Codeunit.al:
                // EvaluateMatchesBlankMemberAndRejectsNonMember).
                // A value named `""` reads the blank text only while no Caption replaces its (empty) name; a value named `" "`
                // reads it with or without a Caption (format-evaluate/format-matrix/FMOptionBlankEnum.al EnumBlank_FQCap_Evaluate
                // `ev_empty` No, EnumBlank_QS2Cap_Evaluate `ev_empty` Yes|2).
                .find(|(o, n)| {
                    n.trim().eq_ignore_ascii_case(text)
                        && (!n.is_empty() || !captions.iter().any(|(co, cap)| co == o && !cap.is_empty()))
                })
                .map(|(o, _)| *o)
        })
        .or_else(|| {
            captions
                .iter()
                .find(|(_, n)| !text.is_empty() && n.eq_ignore_ascii_case(text))
                .map(|(o, _)| *o)
        });
    Ok(match found {
        Some(o) => Parsed::Value(Value::Enum(*id, o)),
        None if c.ret_used() => Parsed::Rejected,
        // The error names every member by its caption, in ordinal order, and quotes the text as given.
        None => {
            let options: Vec<String> = members
                .iter()
                .map(|(o, n)| {
                    captions
                        .iter()
                        .find(|(co, _)| co == o)
                        .map_or(n.clone(), |(_, cap)| cap.clone())
                })
                .collect();
            return crate::native::raise(
                format!(
                    "'{original}' is not an option. The existing options are: {}",
                    options.join(",")
                ),
                "InvalidOptionString",
            );
        }
    })
}
