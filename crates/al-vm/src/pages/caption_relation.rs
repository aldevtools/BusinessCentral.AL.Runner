//! The part a list's caption takes from a DataCaptionFields field with a TableRelation (corpus probe
//! pages/properties/Y45ListCaptionRelation.al, file header lines 6-11, measured on BC 27.5, 28.5, 29.0 and
//! the BC 29 SaaS sandbox).

use super::*;

impl Session {
    /// What field `pos` of the page record `rec` adds to a list caption when its filter holds the one value `value`, the field
    /// having a TableRelation: the related record's own DataCaptionFields (`Y45 Cap RefDcf - D1 ∙ Dcf One`, :367
    /// List_DcfRelatedTableDcf_Filter_Caption), or the value alone when the related table declares none (its Name or Description
    /// is not used: :130 List_Dcf_FilterRefNo_Caption `- R1`, :164 a blank Name `- R2`, :384 `- E1`). `None` when no related
    /// record has the value (:147 List_Dcf_FilterRefNoMissingRef_Caption, :418: no suffix).
    /// A relation with where() clauses, or to a field of a table whose key has several fields, takes the first row of the related
    /// table, in its primary key order, whose related field holds the value: the where() clauses are not applied, and a blank
    /// DataCaptionFields value of that row is left out (testpage/Y72CaptionRelationWhere.al header;
    /// Caption_WhereField_OtherTemplate `- BAT ∙ Batch one` for template T2, Caption_WhereField_NoDescription `- OTHER`,
    /// Caption_WhereConst_OneKey `- X2 ∙ y` for a row the const leaves out, `Z9` no suffix). A conditional relation, a where
    /// filter() with operators, and a related row whose DataCaptionFields values are all blank are not pinned.
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
        let (Some(target), true, Some(wheres)) = (arm.table, arm.conditions.is_empty(), arm.wheres.as_ref()) else {
            return refused();
        };
        // A const() where is pinned (Y72 Caption_WhereConst_OneKey); a filter() with operators is not.
        let pinned_where = |w: &ir::RelationWhere| match w {
            ir::RelationWhere::Field { .. } => true,
            ir::RelationWhere::Value { text, .. } => {
                !text.contains(['|', '&', '<', '>', '*', '?', '@', '=']) && !text.contains("..")
            }
        };
        if !wheres.iter().all(pinned_where) {
            return refused();
        }
        let Some(related) = engine.tables[target].clone() else {
            return refused();
        };
        let one_key = related.keys[0].len() == 1 && arm.field.is_none_or(|f| f == related.keys[0][0]);
        if wheres.is_empty() && !one_key {
            return refused();
        }
        let Some(at) = arm.field.or((related.keys[0].len() == 1).then(|| related.keys[0][0])) else {
            return refused();
        };
        let key = ops::convert(value.clone(), &related.fields[at].ty)?;
        let cx = self.cx_for(rec, &related)?;
        self.unseeded_read(&related, false, "the caption of a list")?;
        let found = if one_key {
            self.store.get(&cx, &related, std::slice::from_ref(&key))
        } else {
            let filters = [al_builtins::filter::FieldFilter {
                field: at,
                expr: al_builtins::filter::FilterExpr::Eq(key.clone()),
                group: 0,
            }];
            self.store.next(
                &cx,
                &related,
                &al_data::Scan {
                    key: 0,
                    filters: &filters,
                    ascending: true,
                    after: None,
                },
            )
        };
        let Some(row) = found.map_err(|e| Signal::Unsupported(format!("{e}")))? else {
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
        let mut any_name = false;
        for name in names
            .split(',')
            .map(|n| n.trim().trim_matches('"'))
            .filter(|n| !n.is_empty())
        {
            let Some(i) = related.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) else {
                return refused();
            };
            if !matches!(related.fields[i].ty, RtType::Code(_) | RtType::Text(_)) {
                return refused();
            }
            any_name = true;
            let text = fmt(&row[i]);
            if !text.is_empty() {
                parts.push(text);
            }
        }
        if parts.is_empty() {
            if any_name {
                return refused();
            }
            parts.push(fmt(&key));
        }
        Ok(Some(parts))
    }
}
