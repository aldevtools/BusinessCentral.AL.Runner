//! The part a list's caption takes from a DataCaptionFields field with a TableRelation (corpus probe
//! pages/lt-y45-list-caption-relation/Y45ListCaptionRelation.al, file header lines 6-11, measured on BC 27.5, 28.5, 29.0 and
//! the BC 29 SaaS sandbox).

use super::*;

impl Session {
    /// What field `pos` of the page record `rec` adds to a list caption when its filter holds the one value `value`, the field
    /// having a TableRelation: the related record's own DataCaptionFields (`Y45 Cap RefDcf - D1 ∙ Dcf One`, :367
    /// List_DcfRelatedTableDcf_Filter_Caption), or the value alone when the related table declares none (its Name or Description
    /// is not used: :130 List_Dcf_FilterRefNo_Caption `- R1`, :164 a blank Name `- R2`, :384 `- E1`). `None` when no related
    /// record has the value (:147 List_Dcf_FilterRefNoMissingRef_Caption, :418: no suffix). A conditional or filtered relation,
    /// one to another field than a one-field primary key, and a blank DataCaptionFields value are not pinned.
    pub(super) fn related_caption_parts(
        &mut self,
        rec: &Rec,
        pos: usize,
        value: &Value,
    ) -> Result<Option<Vec<String>>, Signal> {
        let engine = self.engine();
        let table = self.table(rec)?;
        let field = engine.program.objects[table.object as usize]
            .table
            .as_ref()
            .and_then(|t| t.fields.get(pos))
            .ok_or_else(|| Signal::Unsupported("a caption field of a table without a schema".into()))?;
        let refused = || {
            unsupported("the caption of a list filtered on a data caption field with this TableRelation (not pinned)")
        };
        let [arm] = field.relation_arms.as_slice() else {
            return refused();
        };
        let (Some(target), true, Some(true)) = (
            arm.table,
            arm.conditions.is_empty(),
            arm.wheres.as_ref().map(Vec::is_empty),
        ) else {
            return refused();
        };
        let Some(related) = engine.tables[target].clone() else {
            return refused();
        };
        if related.keys[0].len() != 1 || arm.field.is_some_and(|f| f != related.keys[0][0]) {
            return refused();
        }
        let key = ops::convert(value.clone(), &related.fields[related.keys[0][0]].ty)?;
        let cx = self.cx_for(rec, &related)?;
        self.unseeded_read(&related, false, "the caption of a list")?;
        let Some(row) = self
            .store
            .get(&cx, &related, std::slice::from_ref(&key))
            .map_err(|e| Signal::Unsupported(format!("{e}")))?
        else {
            return Ok(None);
        };
        let names = engine.program.objects[target]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("DataCaptionFields"))
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        let fmt = |v: &Value| pagefmt::control_text(v, &pagefmt::ControlFormat::default(), &|_, o| o.to_string());
        let mut parts = Vec::new();
        for name in names
            .split(',')
            .map(|n| n.trim().trim_matches('"'))
            .filter(|n| !n.is_empty())
        {
            let Some(i) = related.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) else {
                return refused();
            };
            let text = fmt(&row[i]);
            if text.is_empty() || !matches!(related.fields[i].ty, RtType::Code(_) | RtType::Text(_)) {
                return refused();
            }
            parts.push(text);
        }
        if parts.is_empty() {
            parts.push(fmt(&key));
        }
        Ok(Some(parts))
    }
}
