//! A TestPage SetValue of a text into a control with a TableRelation: which row's key the control takes, or the error it
//! raises (al-runner issues 8339 and 9336).
//!
//! Corpus PR 1646 (tests/al-language/testpage/SetValueTableRelation.al, open: assumption
//! `test-page-set-value-table-relation`) measures related tables whose primary key is one Code field:
//! - a Text or Code variable of a Card without a source table or of a report's request page, related to the table (or to
//!   its key field, KeyFieldVar_LowerPrefix), takes the stored key of the first key in key order, inside the relation's
//!   filter, that starts with the text, case ignored ('ABC' and 'abc' give ABC, 'MN' and 'mn' give MNO, 'AB' gives ABC also
//!   when ABD was written first: TextVar_Exact, TextVar_LowerCase, CodeVar_Exact, CodeVar_LowerCase, TextVar_PrefixOneKey,
//!   TextVar_LowerPrefix, CodeVar_PrefixOneKey, TextVar_PrefixTwoKeys, CodeVar_PrefixTwoKeys, TextVar_PrefixTwoKeysAbdFirst,
//!   RequestPage_LowerCase, RequestPage_Prefix, Filtered_Inside, Filtered_PrefixFirstOutside); else an exact key outside
//!   the filter as typed ('XYZ', 'xyz': Filtered_Outside, Filtered_OutsideLowerCase); else it fails with the does-not-exist
//!   error naming the captions and the text upper-cased (TextVar_NoRow, TextVar_NoRowLower, CodeVar_NoRow,
//!   TextVar_InsideKey 'BC', TextVar_NameStart 'Unique', CapVar_NoRow, RequestPage_NoRow, Filtered_PrefixOutside); the
//!   control keeps the value it held and OnValidate does not run (TextVar_ValueThenNoRow). A lower-case exact key inside a
//!   filter is not measured and refused. A Code variable of a request page keeps main's path in every unmeasured shape;
//! - a Text variable related to a field in no key fails every entry, the blank one too, with the primary-key error
//!   (NameVar_*, NameVar_Blank);
//! - a Code table field related to the bare table completes a prefix the same way (HostField_PrefixOneKey,
//!   HostField_PrefixTwoKeys); its exact key, its no-row error, its blank entry and a filtered relation's errors were
//!   main's answer already (HostField_Exact, HostField_NoRow, HostField_Blank, HostActive_OutsideExact,
//!   HostActive_OutsidePrefix, CapHost_NoRow).
//!
//! Every other shape of a variable control on such a page is refused, and so is a Text variable control with a
//! TableRelation elsewhere (al-runner issue 8339: which value BC takes there is not measured). A relation, without a
//! condition, to one table with `TableType = Temporary` keeps its path: such a table stores no row a value could be taken
//! from. A Code variable elsewhere and a table field outside the measured shape keep their paths too.
//!
//! Entry point: `relation_text_entry` (by `control_set_value`).

use super::relation_lookup::{simple_relation, tables_named};
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::FieldFilter;
use al_data::{CodeStorage, TableDef};

/// The refusal of a variable control's entry outside the cells corpus PR 1646 measures.
const UNMEASURED: &str =
    "SetValue of a text into a page variable with a TableRelation, in a shape corpus PR 1646 does not measure";

/// A variable control's relation in a measured shape (`measured_relation`).
enum Relation {
    /// To the primary key, optionally under one `const` condition on a field outside it.
    Key(Option<FieldFilter>),
    /// To the field at this position, which is in no key.
    NonKey(usize),
}

impl Session {
    /// Before a SetValue writes control `idx`: replaces the text in `args` by the related row's key, raises the related
    /// table's error, refuses, or leaves the entry as it is (module doc).
    pub(super) fn relation_text_entry(&mut self, st: &St, idx: usize, args: &mut [Value]) -> Result<(), Signal> {
        let Some(value) = args.first() else {
            return Ok(());
        };
        let typed = match value {
            Value::Text(t) | Value::Code(t) => Some(t.to_string()),
            _ => None,
        };
        let blank = typed.as_ref().is_some_and(|t| t.trim().is_empty());
        let engine = self.engine();
        let c = &page_of(&engine, st).controls[idx];
        let own = match c.prop("tablerelation") {
            Some(ir::PropVal::Lit(t)) => Some(Some(t.clone())),
            Some(_) => Some(None),
            None => None,
        };
        match (&c.binding, own) {
            (ir::Binding::Var(_) | ir::Binding::BaseVar(_), Some(text)) => {
                if text.as_deref().is_some_and(|t| relates_to_temporary_table(&engine, t)) {
                    return Ok(());
                }
                let ty = self.control_type(st, idx, value);
                if let Some(ty) = ty.clone().filter(|ty| self.measured_variable(st, idx, ty)) {
                    let (Some(typed), Some(text)) = (typed, text) else {
                        return self.unmeasured(st, &ty);
                    };
                    return self.variable_entry(st, idx, &typed, &ty, &text, args);
                }
                if matches!(ty, Some(RtType::Text(_))) && !blank {
                    return unsupported(
                        "SetValue of a text into a Text page variable with a TableRelation (whether BC takes the related row's stored value is not pinned)",
                    );
                }
                Ok(())
            }
            (ir::Binding::Field(pos), None) => match typed {
                Some(typed) if !blank => self.field_prefix_entry(st, idx, *pos, &typed, args),
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    /// The measured variable controls: a Text or Code variable of a Card without a source table, opened on its own (not as a
    /// part), or of a report's request page; the control the object's own, bound to its own variable, which no extension
    /// modifies.
    fn measured_variable(&self, st: &St, idx: usize, ty: &RtType) -> bool {
        let engine = self.engine();
        let c = &page_of(&engine, st).controls[idx];
        let own = matches!(c.binding, ir::Binding::Var(_))
            && c.owner.is_none()
            && c.ext_triggers.is_empty()
            && c.ext_replaced.is_empty()
            && st.borrow().host.is_none();
        let card = self.is_card(st) && !self.has_source(st) && !self.is_request_page(st);
        let request = self.is_request_page(st);
        own && matches!(ty, RtType::Text(_) | RtType::Code(_)) && (card || request)
    }

    /// The answer for an entry outside the measured cells: refused, except into a Code variable of a request page, which
    /// keeps main's path (it took the typed text before this rule).
    fn unmeasured(&self, st: &St, ty: &RtType) -> Result<(), Signal> {
        if matches!(ty, RtType::Code(_)) && self.is_request_page(st) {
            return Ok(());
        }
        unsupported(UNMEASURED)
    }

    /// The entry `typed` into a measured variable control of type `ty` related by `text` (module doc).
    fn variable_entry(
        &mut self,
        st: &St,
        idx: usize,
        typed: &str,
        ty: &RtType,
        text: &str,
        args: &mut [Value],
    ) -> Result<(), Signal> {
        let text_variable = matches!(ty, RtType::Text(_));
        let Some((related, relation)) = self.measured_relation(text, text_variable)? else {
            return self.unmeasured(st, ty);
        };
        let key = related.keys[0][0];
        let fits = |len: &Option<u32>| len.is_none_or(|n| Value::text_len(typed) <= n as usize);
        let var_fits = matches!(ty, RtType::Text(n) | RtType::Code(n) if fits(n));
        let english = al_builtins::format::FormatCtx::language(self) == 1033;
        if !(var_fits && english) {
            return self.unmeasured(st, ty);
        }
        let filter = match relation {
            Relation::NonKey(field) => {
                // Measured from an empty control (NameVar_*); what a control that held a value then shows is not.
                if !matches!(self.control_value(st, idx)?, Value::Text(t) | Value::Code(t) if t.is_empty()) {
                    return self.unmeasured(st, ty);
                }
                assume(Assumption::TestPageSetValueTableRelation);
                let msg = format!(
                    "The following field must be included into the table's primary key: Field: {} Table: {}",
                    related.fields[field].name, related.name
                );
                return Err(self.relation_entry_error(st, idx, msg)?);
            }
            Relation::Key(filter) => filter,
        };
        // The blank entry is taken and OnValidate runs (TextVar_Blank), as main did.
        if typed.trim().is_empty() {
            return Ok(());
        }
        if !matches!(&related.fields[key].ty, RtType::Code(n) if fits(n)) {
            return self.unmeasured(st, ty);
        }
        let upper = typed.to_uppercase();
        let rows = self.related_rows(None, &related, &[])?;
        let inside = match &filter {
            Some(f) => self.related_rows(None, &related, std::slice::from_ref(f))?,
            None => rows.clone(),
        };
        let exact = rows.iter().any(|r| key_text(&r[key]) == upper);
        // A Code variable held the upper-cased text before this rule: the same value, read as main read it.
        if exact && !text_variable && filter.is_none() {
            return Ok(());
        }
        self.unseeded_read(&related, false, "the TableRelation completion of a TestPage entry")?;
        // The first key in key order inside the filter that starts with the text, case ignored, gives its stored key (an exact
        // key sorts before the keys it starts). A lower-case exact key inside a filter is not measured.
        if let Some(row) = inside.iter().find(|r| key_text(&r[key]).starts_with(&upper)) {
            if filter.is_some() && typed != upper && key_text(&row[key]) == upper {
                return self.unmeasured(st, ty);
            }
            assume(Assumption::TestPageSetValueTableRelation);
            args[0] = Value::Text(key_text(&row[key]).into());
            return Ok(());
        }
        assume(Assumption::TestPageSetValueTableRelation);
        // Else an exact key outside the filter is taken as typed (Filtered_Outside, Filtered_OutsideLowerCase).
        if exact {
            args[0] = Value::Text(typed.into());
            return Ok(());
        }
        let msg = format!(
            "The {} does not exist. Identification fields and values: {}='{upper}'",
            self.caption(&related),
            self.field_caption(&related, key)
        );
        Err(self.relation_entry_error(st, idx, msg)?)
    }

    /// The related table of a variable control's relation `text` and its measured shape, if it is one: the bare table or
    /// `Table.Field`, the table one of the workspace whose primary key is one Code field stored as text and that declares no
    /// CaptionML. `Table.Field` of the key field is the bare table (KeyFieldVar_LowerPrefix); of a field in no key it is measured on a Text
    /// variable (`text_variable`), the captions of the table and the field their names. A `where()` of one
    /// `Field = const(x)` on a stored field outside the primary key is measured on a Text variable related to the bare table.
    fn measured_relation(&self, text: &str, text_variable: bool) -> Result<Option<(Arc<TableDef>, Relation)>, Signal> {
        let head = before_where(text);
        let Some((table_name, field_name)) = simple_relation(head) else {
            return Ok(None);
        };
        let engine = self.engine();
        let tables = tables_named(&engine, &table_name);
        let [table] = tables.as_slice() else {
            return Ok(None);
        };
        let Some(related) = engine.tables[*table].clone() else {
            return Ok(None);
        };
        let object = &engine.program.objects[*table];
        let key = match related.keys.first().map(Vec::as_slice) {
            Some([k]) if matches!(related.fields[*k].ty, RtType::Code(_)) => *k,
            _ => return Ok(None),
        };
        let caption_ml = object
            .properties
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("CaptionML"));
        if caption_ml || related.fields[key].code_storage != CodeStorage::Text {
            return Ok(None);
        }
        let named = |pos: usize| self.field_caption(&related, pos) == related.fields[pos].name;
        let position = |n: &str| related.fields.iter().position(|f| f.name.eq_ignore_ascii_case(n));
        let wheres = text[head.len()..].trim();
        let relation = match (field_name.map(|n| position(&n)), wheres.is_empty()) {
            (Some(Some(p)), true) if p == key => Relation::Key(None),
            (Some(Some(p)), true)
                if text_variable
                    && named(p)
                    && self.caption(&related) == related.name
                    && related.keys.iter().all(|k| !k.contains(&p)) =>
            {
                Relation::NonKey(p)
            }
            (None, true) => Relation::Key(None),
            (None, false) if text_variable => {
                let Some((name, constant)) = one_const_where(wheres) else {
                    return Ok(None);
                };
                let Some(p) = position(&name) else {
                    return Ok(None);
                };
                let class = engine.field_facts[related.object as usize][p].class;
                if p == key || class != ir::FieldClass::Normal || !related.fields[p].stored {
                    return Ok(None);
                }
                let expr = self
                    .parse_filter_text(&constant, &related.fields[p].ty)
                    .map_err(|e| Signal::unsupported(format!("where() of a page control's TableRelation: {e}")))?;
                Relation::Key(Some(FieldFilter {
                    lang: al_builtins::filter::filter_language(),
                    field: p,
                    expr,
                    group: 0,
                }))
            }
            _ => return Ok(None),
        };
        Ok(Some((related, relation)))
    }

    /// The error of a variable control's refused entry: `msg` in the frame of a validation error, no Refresh hint, code
    /// TestValidation (SetValueTableRelation.al TextVar_NoRow, NameVar_Exact).
    fn relation_entry_error(&mut self, st: &St, idx: usize, msg: String) -> Result<Signal, Signal> {
        let caption = self.control_caption(st, idx)?;
        let frame = self.validation_frame(st, idx, &caption);
        Ok(super::validation_text::validation_signal(
            &self.engine(),
            st,
            idx,
            msg,
            "",
            "TestValidation",
            frame,
        ))
    }

    /// The entry `typed` into a Code table field (position `pos` of the page's record) related to a table: a prefix of a key,
    /// case ignored, where no key is the text, takes the first such key in key order (HostField_PrefixOneKey,
    /// HostField_PrefixTwoKeys). Measured on a Card over its source table, on a row, related to the bare table whose primary
    /// key is one Code field. Elsewhere (a where() relation applies its filter to the exact key too: HostActive_*), and for an
    /// exact key or no key that starts with the text, the entry keeps its path.
    fn field_prefix_entry(
        &mut self,
        st: &St,
        idx: usize,
        pos: usize,
        typed: &str,
        args: &mut [Value],
    ) -> Result<(), Signal> {
        let engine = self.engine();
        let own_control = page_of(&engine, st).controls[idx].owner.is_none();
        let page = self.is_card(st)
            && self.has_source(st)
            && !self.is_list(st)
            && !self.is_request_page(st)
            && st.borrow().host.is_none()
            && !self.shows_no_row(st, idx);
        let Some(field) = self.control_field(st, idx) else {
            return Ok(());
        };
        let fits = matches!(field.ty, RtType::Code(Some(n)) if Value::text_len(typed) <= n as usize);
        let arm = match field.relation_arms.as_slice() {
            [arm] if arm.conditions.is_empty() && arm.wheres.as_deref() == Some(&[]) => arm.clone(),
            _ => return Ok(()),
        };
        if !(own_control && page && fits && field.relation_runtime.is_empty()) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let Some(related) = arm.table.and_then(|t| engine.tables[t].clone()) else {
            return Ok(());
        };
        let key = match related.keys.first().map(Vec::as_slice) {
            Some([k]) if matches!(related.fields[*k].ty, RtType::Code(_)) => *k,
            _ => return Ok(()),
        };
        let checked = engine.field_facts[table.object as usize][pos].validate_relation;
        let temporary = engine.object_facts[related.object as usize].temporary();
        if !checked
            || temporary
            || arm.field.is_some_and(|f| f != key)
            || related.fields[key].code_storage != CodeStorage::Text
        {
            return Ok(());
        }
        let rows = self.related_rows(Some(&rec), &related, &[])?;
        let upper = typed.to_uppercase();
        if rows.iter().any(|r| key_text(&r[key]) == upper) {
            return Ok(());
        }
        let Some(row) = rows.iter().find(|r| key_text(&r[key]).starts_with(&upper)) else {
            return Ok(());
        };
        self.unseeded_read(&related, false, "the TableRelation completion of a TestPage entry")?;
        assume(Assumption::TestPageSetValueTableRelation);
        args[0] = Value::Code(key_text(&row[key]).into());
        Ok(())
    }
}

/// The text of a key value.
fn key_text(v: &Value) -> String {
    match v {
        Value::Text(t) | Value::Code(t) => t.to_string(),
        _ => String::new(),
    }
}

/// Whether a control's own TableRelation `text` is `Table` or `Table.Field`, with or without a `where(..)`, and names
/// exactly one table of the workspace, whose `TableType` is `Temporary`. A relation with an `if` condition does not.
fn relates_to_temporary_table(engine: &Engine, text: &str) -> bool {
    let Some((table, _)) = simple_relation(before_where(text)) else {
        return false;
    };
    matches!(tables_named(engine, &table).as_slice(), [t] if engine.object_facts[*t].temporary())
}

/// `text` up to its first `where` word outside double quotes (all of it when it has none).
fn before_where(text: &str) -> &str {
    let mut quoted = false;
    for (i, c) in text.char_indices() {
        if c == '"' {
            quoted = !quoted;
        } else if !quoted && text.get(i..i + 5).is_some_and(|w| w.eq_ignore_ascii_case("where")) {
            let starts_word = !text[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_');
            let ends_word = !text[i + 5..].starts_with(|c: char| c.is_alphanumeric() || c == '_');
            if starts_word && ends_word {
                return &text[..i];
            }
        }
    }
    text
}

/// The field name and the constant of a `where(Field = const(x))` clause with one condition (`Field` bare or in double
/// quotes, `x` without quotes, parentheses or commas), if `wheres` is one.
fn one_const_where(wheres: &str) -> Option<(String, String)> {
    let word = |s: &str, w: &str| s.get(..w.len()).is_some_and(|h| h.eq_ignore_ascii_case(w));
    if !word(wheres, "where") {
        return None;
    }
    let inner = wheres[5..].trim().strip_prefix('(')?.strip_suffix(')')?.trim();
    let (name, rest) = match inner.strip_prefix('"') {
        Some(r) => r.split_once('"')?,
        None => inner.split_at(inner.find(|c: char| !(c.is_alphanumeric() || c == '_'))?),
    };
    let rest = rest.trim_start().strip_prefix('=')?.trim_start();
    if !word(rest, "const") {
        return None;
    }
    let constant = rest[5..].trim_start().strip_prefix('(')?.strip_suffix(')')?.trim();
    let plain = !constant.is_empty() && !constant.contains(['(', ')', '\'', '"', ',']);
    (!name.is_empty() && plain).then(|| (name.to_string(), constant.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_const_where_reads_one_condition_only() {
        assert_eq!(
            one_const_where("where(Active = const(true))"),
            Some(("Active".into(), "true".into()))
        );
        assert_eq!(
            one_const_where("WHERE (\"Is Active\"=CONST(Yes))"),
            Some(("Is Active".into(), "Yes".into()))
        );
        assert_eq!(one_const_where("where(Active = filter(true))"), None);
        assert_eq!(one_const_where("where(Active = const(true), Name = const(A))"), None);
        assert_eq!(one_const_where("where(Name = const('A B'))"), None);
        assert_eq!(one_const_where("where(Code = field(Other))"), None);
    }
}
