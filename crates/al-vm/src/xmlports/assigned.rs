//! The separators and the text encoding of a VariableText port that the program set (`currXMLport.FieldSeparator := v`
//! or the call form `FieldSeparator(v)`), against the declared ones.

use super::text::text_separator;
use super::*;

/// Separator `kind` (0 delimiter, 1 field, 2 record, 3 table) as an export or import uses it. A value the program set is
/// used as it is: a token such as `<TAB>`, `<None>` or `<CR/LF>` is written literally, an empty text leaves the
/// separator out (xmlport/lt-y67-xmlport-separators/Y67XpSeparators.al:314-366 m8, m9, m13; :367-391 m16, m18; :393-418
/// m26; :441-471 Assign_Import `record sep crlf token`). A declared one reads its tokens (`text_separator`).
pub(super) fn separator(inst: &Inst, kind: u8, declared: Option<&str>, default: &str) -> Result<String, Signal> {
    let set = inst
        .borrow()
        .xp_text
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, v)| v.clone());
    match set {
        Some(v) => Ok(v),
        None => text_separator(declared, default),
    }
}

/// The text encoding the program set (`currXMLport.TextEncoding := v`, the member index MSDos 0, UTF8 1, UTF16 2,
/// Windows 3), if any.
pub(super) fn set_encoding(inst: &Inst) -> Option<i64> {
    inst.borrow()
        .xp_text
        .iter()
        .find(|(k, _)| *k == 4)
        .and_then(|(_, v)| v.parse().ok())
}

/// The `Instance::xp_text` kind that holds the encoding set when the run started (`mark_run_start`).
const ENCODING_AT_START: u8 = 17;

/// Notes the encoding set on the variable when a run starts, before OnPreXmlPort.
pub(super) fn mark_run_start(inst: &Inst) {
    let now = set_encoding(inst).map_or(String::new(), |e| e.to_string());
    let mut i = inst.borrow_mut();
    i.xp_text.retain(|(k, _)| *k != ENCODING_AT_START);
    i.xp_text.push((ENCODING_AT_START, now));
}

/// The encoding the port's triggers set during this run (in OnPreXmlPort or OnAfterGetRecord: Y67XpSeparators.al:393-439).
/// One set on the variable before Export() keeps the handling it had: platform/lt-xmlport-session-navapp-cookie/
/// P94XpEncoding.al observes no byte of it, so its effect is not pinned.
pub(super) fn set_in_run(inst: &Inst) -> Option<i64> {
    let start = inst
        .borrow()
        .xp_text
        .iter()
        .find(|(k, _)| *k == ENCODING_AT_START)
        .and_then(|(_, v)| v.parse::<i64>().ok());
    set_encoding(inst).filter(|e| Some(*e) != start)
}
