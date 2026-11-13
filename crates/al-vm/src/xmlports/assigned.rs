//! The separators and the text encoding of a VariableText port that the program set (`currXMLport.FieldSeparator := v`
//! or the call form `FieldSeparator(v)`), against the declared ones.

use super::text::text_separator;
use super::*;

/// Separator `kind` (0 delimiter, 1 field, 2 record, 3 table) as an export or import uses it. A value the program set is
/// used as it is: a token such as `<TAB>`, `<None>` or `<CR/LF>` is written literally, an empty text leaves the
/// separator out (xmlports/shapes-properties/Y67XpSeparators.al:314-366 m8, m9, m13; :367-391 m16, m18; :393-418
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

/// The port declares no FieldDelimiter, FieldSeparator, RecordSeparator or TableSeparator and the program set none of them
/// (`Instance::xp_text` kinds 0 to 3), as port 75182 of corpus pull request 1340 (xmlports/import/XpLegacyGuidImport.al:53-71).
pub(super) fn default_separators(inst: &Inst, xp: &ir::XmlPort) -> bool {
    xp.field_delimiter.is_none()
        && xp.field_separator.is_none()
        && xp.record_separator.is_none()
        && xp.table_separator.is_none()
        && inst.borrow().xp_text.iter().all(|(k, _)| *k > 3)
}

impl Session {
    /// `FieldDelimiter`, `FieldSeparator`, `RecordSeparator` and `TableSeparator` of a text xmlport. The setter stores the text
    /// as given; the getter reads the separator as an export uses it (`separator`): a value the program set as it was given
    /// (metadata/session-system/P94XpDelim.al:Getters_*, Setters_*, *_Variants), a declared one with its tokens read as
    /// `text_separator` reads them (xmlports/shapes-properties/XpSeparatorGetterTokens.al:105-118 for <>, <None>, <none>,
    /// <TAB>, <tab>, <LF>, <CR>, <CR/LF>, <NewLine>, ;<TAB>, <TAB><TAB>; :155-156 for <none>, <tab>, ;<LF>, <None> as record and
    /// table separators), an undeclared one as the default.
    pub(super) fn xp_separator_builtin(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
    ) -> Result<Value, Signal> {
        let inst = self.xp_instance(receiver)?;
        let kind: u8 = match id {
            ids::XMLPORT_FIELDDELIMITER_1 => 0,
            ids::XMLPORT_FIELDSEPARATOR_1 => 1,
            ids::XMLPORT_RECORDSEPARATOR_1 => 2,
            _ => 3,
        };
        if let Some(v) = args.first() {
            let Some(text) = al_builtins::native::text_of(v) else {
                return unsupported(format!("{id} with {}", v.describe()));
            };
            let mut i = inst.borrow_mut();
            i.xp_text.retain(|(k, _)| *k != kind);
            i.xp_text.push((kind, text.to_string()));
            return Ok(Value::Null);
        }
        let object = inst.borrow().object as usize;
        let Some(xp) = self.engine.program.objects[object].xmlport.as_ref() else {
            return unsupported(format!("object {object} is not an xmlport"));
        };
        let (declared, default) = match kind {
            0 => (&xp.field_delimiter, "\""),
            1 => (&xp.field_separator, ","),
            2 => (&xp.record_separator, "\r\n"),
            _ => (&xp.table_separator, "\r\n\r\n"),
        };
        Ok(Value::Text(
            separator(&inst, kind, declared.as_deref(), default)?.into(),
        ))
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
/// One set on the variable before Export() keeps the handling it had: metadata/session-system/
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
