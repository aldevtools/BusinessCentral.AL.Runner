//! Lowering of table schemas: relations, field triggers, CalcFormulas.
use super::*;

impl Lowerer<'_> {
    pub(crate) fn field_pos(&self, table: usize, id: u32) -> Option<usize> {
        self.members[table].fields.iter().position(|f| f.id == id)
    }

    /// The arms of a field's `TableRelation`, with names resolved from the declaring file's scope.
    pub(crate) fn relation_arms(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        with_filters: bool,
    ) -> Vec<ir::RelationArm> {
        let texts: Vec<&String> = f
            .properties
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("TableRelation"))
            .map(|(_, v)| v)
            .collect();
        if texts.len() > 1 {
            return self.joined_relation_arms(oi, f, with_filters, &texts);
        }
        let Some(text) = texts.first().copied() else {
            return Vec::new();
        };
        self.relation_arms_of(oi, f, with_filters, text)
    }

    /// The arms of a field whose relation a table extension's `modify` extended: the base's arms, then the extension's
    /// (tableextension/TestTableExtFieldTableRelation.al:241-298). Pinned when every relation is one unconditional arm
    /// (the first decides, :241-268) or every arm has a condition (:270-298). Any other combination is not understood
    /// (no arms), so validating the field is refused.
    fn joined_relation_arms(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        with_filters: bool,
        texts: &[&String],
    ) -> Vec<ir::RelationArm> {
        let parts: Vec<Vec<ir::RelationArm>> = texts
            .iter()
            .map(|t| self.relation_arms_of(oi, f, with_filters, t))
            .collect();
        if parts.iter().any(Vec::is_empty) {
            return Vec::new();
        }
        let all_unconditional = parts.iter().all(|p| p.len() == 1 && p[0].conditions.is_empty());
        let all_conditional = parts.iter().flatten().all(|a| !a.conditions.is_empty());
        if !all_unconditional && !all_conditional {
            return Vec::new();
        }
        parts.into_iter().flatten().collect()
    }

    fn relation_arms_of(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        with_filters: bool,
        text: &str,
    ) -> Vec<ir::RelationArm> {
        let Some(parsed) = (if with_filters {
            crate::relation::parse_lenient(text)
        } else {
            crate::relation::parse(text)
        }) else {
            return Vec::new();
        };
        let file = f
            .declared_in
            .map_or(self.ws.objects[oi].file, |d| self.ws.objects[d].file);
        let own = &self.members[oi];
        let field_pos =
            |m: &crate::members::Members, name: &str| m.fields.iter().position(|x| x.name.eq_ignore_ascii_case(name));
        let mut arms = Vec::new();
        for a in parsed {
            let mut conditions = Vec::new();
            for (k, (name, value)) in a.conditions.iter().enumerate() {
                let Some(pos) = field_pos(own, name) else {
                    return Vec::new();
                };
                if a.filter_conditions.get(k).copied().unwrap_or(false) {
                    // The filter text as written; AL quotes an identifier with `"`, the filter grammar with `'`.
                    let raw = a.condition_raw.get(k).cloned().unwrap_or_default();
                    let raw = if raw.contains('"') { rewrite_quotes(&raw) } else { raw };
                    conditions.push(ir::RelationCond {
                        field: pos,
                        value: ir::Const::Text(raw),
                        filter: true,
                    });
                    continue;
                }
                // `const(" ")` on an Option names its blank member, pinned for a blank FIRST member (OptionMembers = " ",A:
                // record/lt-table-relation-arms/TableRelationArms.al, the arm of the blank member and of A). A blank member
                // elsewhere is not pinned: the relation is not understood.
                if let Ty::Option(members) = &own.fields[pos].ty {
                    if value.trim().trim_matches('"').trim().is_empty()
                        && members.iter().position(|m| m.trim().is_empty()) != Some(0)
                    {
                        return Vec::new();
                    }
                }
                let Some(value) = self.condition_const(file, value, &own.fields[pos].ty) else {
                    return Vec::new();
                };
                conditions.push(ir::RelationCond {
                    field: pos,
                    value,
                    filter: false,
                });
            }
            // The whole path names a table, or all but the last part do and the last is a field.
            let resolve = |parts: &[String]| match self.ws.resolve_relation_target(file, ObjKind::Table, parts) {
                Resolution::Found(t) => Some(t),
                _ => None,
            };
            let (table, field) = match resolve(&a.path) {
                Some(t) => (Some(t), None),
                None if a.path.len() > 1 => match resolve(&a.path[..a.path.len() - 1]) {
                    Some(t) => match field_pos(&self.members[t], &a.path[a.path.len() - 1]) {
                        Some(p) => (Some(t), Some(p)),
                        None => return Vec::new(),
                    },
                    None => (None, None),
                },
                None => (None, None),
            };
            // A relation onto SystemRowVersion relates to the primary key instead (BC encodes
            // the field as id 0): corpus record/TestCalcFormulaSystemFieldsTests.Codeunit.al:229-260.
            let field = match (table, field) {
                (Some(t), Some(p)) if self.members[t].fields[p].id == 2_000_000_005 => None,
                (_, f) => f,
            };
            let wheres = match (&a.wheres, table) {
                (Some(ws), Some(t)) => ws
                    .iter()
                    .map(|w| match w {
                        crate::relation::ParsedWhere::Value { related, text } => {
                            let p = field_pos(&self.members[t], related)?;
                            // `Database::"X"` / `Report::"X"` etc.: the object's id, as for a
                            // CalcFormula const (corpus record/CdcOwner.Table.al:67,79).
                            let text = if !text.starts_with('\'') && text.contains("::") {
                                match calc_const(self.ws, self.members, file, text, &self.members[t].fields[p].ty) {
                                    Ok(ir::Const::Integer(id)) => id.to_string(),
                                    Ok(ir::Const::BigInteger(id)) => id.to_string(),
                                    Ok(ir::Const::Decimal(id)) => id,
                                    _ => return None,
                                }
                            } else if text.contains('"') {
                                // `filter(Inventory | "Non-Inventory")`: AL quotes an identifier with `"`,
                                // the filter grammar with `'` (corpus
                                // record/TestCalcFormulaQuotedFilterTests.Codeunit.al:9 pins it for a
                                // CalcFormula filter; docs/probes-needed.md for the TableRelation).
                                rewrite_quotes(text)
                            } else if text.is_empty() {
                                // `const('')` / `filter('')` select the blank value (corpus record/lt-relation-where-empty-term/
                                // RelationWhereEmptyTerm.al:Relation_ErrorTextOfFailedValidate, CalcFormula_CountWhereEmptyTerms).
                                "''".to_string()
                            } else if !text.is_empty() && text.trim().is_empty() {
                                // `filter(" ")` / `const(" ")` name the member or value of one blank; the unquoted text would be an
                                // empty filter term (Service Contract Line: `Blocked = filter(" ")`).
                                format!("'{text}'")
                            } else {
                                text.clone()
                            };
                            Some(ir::RelationWhere::Value { related: p, text })
                        }
                        crate::relation::ParsedWhere::Field { related, own: o } => Some(ir::RelationWhere::Field {
                            related: field_pos(&self.members[t], related)?,
                            own: field_pos(own, o)?,
                        }),
                    })
                    .collect(),
                (Some(ws), None) if ws.is_empty() => Some(Vec::new()),
                _ => None,
            };
            // A filter text with an AL-quoted identifier read as a filter text. Pinned for an Enum or Option field of the
            // related table when the text is members joined by `|`, a quoted one alone, or `<>` and a quoted one
            // (record/lt-table-relation-quotes/TableRelationQuotes.al:41-46, :126-207; misc/lt-probes-needed-6/
            // PnF03RelationQuotedFilter.al:40-48; query/lt-query-links-relations-httpstream/RelQuotedOption.al:27,
            // RelQuotedEnum.al:36, :235), and the ranges and `&` forms of `quoted_relation_text_pinned`
            // (TableRelationQuotes.al:279-397). Any other form stays under the assumption.
            let quoted = a.wheres.iter().flatten().any(|w| {
                matches!(w, crate::relation::ParsedWhere::Value { related, text }
                    if !text.starts_with('\'') && !text.contains("::") && text.contains('"')
                        && !table
                            .and_then(|t| field_pos(&self.members[t], related).map(|p| &self.members[t].fields[p].ty))
                            .is_some_and(|ty| quoted_relation_text_pinned(text, ty)))
            }) || a.condition_raw.iter().zip(&a.filter_conditions).zip(&a.conditions).any(
                |((raw, filter), (name, _))| {
                    *filter
                        && raw.contains('"')
                        && !field_pos(own, name).is_some_and(|p| quoted_condition_text_pinned(raw, &own.fields[p].ty))
                },
            );
            arms.push(ir::RelationArm {
                conditions,
                table,
                table_name: a.path.join("."),
                field,
                wheres,
                assumes: {
                    let mut v = Vec::new();
                    if quoted {
                        v.push(al_builtins::assumptions::Assumption::TableRelationQuotes);
                    }
                    v
                },
            });
        }
        arms
    }

    /// The relation of a field the runtime does not understand (`relation_arms` is empty), as far
    /// as the Table Relations Metadata table lists it: per arm the related table and field and the
    /// own fields its conditions name, whatever their constants. Empty when even that is unclear.
    pub(crate) fn relation_meta(&self, oi: usize, f: &crate::members::FieldInfo) -> Vec<ir::MetaRelationArm> {
        let Some(text) = f
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("TableRelation"))
            .map(|(_, v)| v)
        else {
            return Vec::new();
        };
        let Some(parsed) = crate::relation::parse_lenient(text) else {
            return Vec::new();
        };
        let file = f
            .declared_in
            .map_or(self.ws.objects[oi].file, |d| self.ws.objects[d].file);
        let own = &self.members[oi];
        let field_pos =
            |m: &crate::members::Members, name: &str| m.fields.iter().position(|x| x.name.eq_ignore_ascii_case(name));
        let mut arms = Vec::new();
        for a in parsed {
            let mut conditions = Vec::new();
            let mut filters = Vec::new();
            let mut values = Vec::new();
            for (k, (name, text)) in a.conditions.iter().enumerate() {
                let Some(pos) = field_pos(own, name) else {
                    return Vec::new();
                };
                conditions.push(pos);
                let is_filter = a.filter_conditions.get(k).copied().unwrap_or(false);
                filters.push(is_filter);
                // The value the table lists: the ordinal of an option member, the ordinals of an alternative
                // `A | B` of members joined by `|` (session/PWPlatformTables.al:109-123). Any other
                // shape of a filter is not pinned.
                let members = match &own.fields[pos].ty {
                    Ty::Option(m) => Some(m),
                    _ => None,
                };
                let ordinal = |word: &str| {
                    let word = word.trim().trim_matches('"');
                    members?
                        .iter()
                        .position(|m| m.eq_ignore_ascii_case(word))
                        .map(|i| i.to_string())
                };
                values.push(if is_filter && members.is_some() {
                    // Members by ordinal, with the alternatives `|` and ranges `..` kept: `A .. B` is `0..1`
                    // (metadata/W6TableRelations.al:Relations_ConditionOnTheSourceField, BC 27.5 and 29.0).
                    text.split('|')
                        .map(|alt| {
                            alt.split("..")
                                .map(&ordinal)
                                .collect::<Option<Vec<_>>>()
                                .map(|v| v.join(".."))
                        })
                        .collect::<Option<Vec<_>>>()
                        .map(|v| v.join("|"))
                } else if is_filter {
                    // A filter on a field that is no option shows its text as written, without the quotes of a
                    // quoted literal: `filter('X|Y')` is `X|Y`, `filter('<>X')` is `<>X`.
                    let t = text.trim();
                    Some(match t.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
                        Some(inner) => inner.replace("''", "'"),
                        None => t.to_string(),
                    })
                } else {
                    ordinal(text)
                });
            }
            let resolve = |parts: &[String]| match self.ws.resolve_relation_target(file, ObjKind::Table, parts) {
                Resolution::Found(t) => Some(t),
                _ => None,
            };
            let (table, field) = match resolve(&a.path) {
                Some(t) => (t, None),
                None if a.path.len() > 1 => match resolve(&a.path[..a.path.len() - 1]) {
                    Some(t) => match field_pos(&self.members[t], &a.path[a.path.len() - 1]) {
                        Some(p) => (t, Some(p)),
                        None => return Vec::new(),
                    },
                    None => return Vec::new(),
                },
                None => return Vec::new(),
            };
            arms.push(ir::MetaRelationArm {
                conditions,
                filters,
                values,
                table,
                field,
            });
        }
        arms
    }

    /// The constant of a `Field = const(x)` condition, typed by the field.
    pub(crate) fn condition_const(&self, file: usize, text: &str, ty: &Ty) -> Option<ir::Const> {
        if let Ty::Enum(name) = ty {
            // The enum is named in the file that declares the field, as a relation target is: a
            // dependency's file resolves it among the apps it saw (its `using`s included), so that an
            // enum of another namespace is found (Gen. Journal Line."IC Account No.", issue 23).
            let Resolution::Found(e) =
                self.ws
                    .resolve_relation_target(file, ObjKind::Enum, &crate::types::qualified_parts(name))
            else {
                return None;
            };
            let (value, _) = self.members[e]
                .enum_values
                .iter()
                // Both sides trimmed: the blank member `" "` is named `" "` in `const(" ")`
                // (Base Application "Standard Service Line"."No.").
                .find(|(_, n)| n.trim().eq_ignore_ascii_case(text.trim()))?;
            return Some(ir::Const::Enum(e as u32, *value as i32));
        }
        init_const(text, ty)
    }

    /// The position of field `id`.
    pub(crate) fn readable_field_pos(&self, table: usize, id: u32) -> L<usize> {
        // A column of a platform table that no corpus test pins is refused, not defaulted. The
        // table's own triggers read its columns as they are declared.
        if let Some(why) = self.ws.objects[table]
            .id
            .filter(|_| self.oi != table && self.ws.platform_table(table))
            .and_then(|t| al_builtins::platform::unpinned_column(t, id))
        {
            return Err(why);
        }
        self.field_pos(table, id).ok_or_else(|| "field position".to_string())
    }

    /// The triggers named `trigger` that `modify(field)` blocks of the table's `extensions` declare.
    pub(crate) fn ext_field_triggers(
        &self,
        extensions: &[usize],
        field: &str,
        trigger: &str,
        routine_at: &RoutineAt,
    ) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for &e in extensions {
            if !self.ws.usable(e) {
                continue;
            }
            for sec in ast::sections_in(self.ws.object_ast(e).syntax()) {
                if !sec.name().eq_ignore_ascii_case("modify")
                    || !sec.args().first().is_some_and(|a| a.eq_ignore_ascii_case(field))
                {
                    continue;
                }
                for t in sec.triggers().filter(|t| t.name().eq_ignore_ascii_case(trigger)) {
                    let offset: usize = t.syntax().text_range().start().into();
                    if let Some(r) = routine_at.get(&(e, offset)) {
                        out.push((e, r));
                    }
                }
            }
        }
        out
    }

    /// The routine that evaluates field `f`'s `CaptionClass` expression: of the object that declares the field, or of the
    /// last of the table's `extensions` whose `modify` block sets one.
    fn field_caption_class(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        extensions: &[usize],
        routine_at: &RoutineAt,
    ) -> Option<(usize, usize)> {
        self.field_expression_routine(
            oi,
            f,
            extensions,
            routine_at,
            crate::pages::field_caption_class_routines,
        )
    }

    /// The routine that evaluates field `f`'s `AutoFormatExpression` expression, found as `field_caption_class`.
    fn field_auto_format(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        extensions: &[usize],
        routine_at: &RoutineAt,
    ) -> Option<(usize, usize)> {
        self.field_expression_routine(oi, f, extensions, routine_at, crate::pages::field_auto_format_routines)
    }

    /// Why field `f`'s `AutoFormatExpression` is refused: the table and its `extensions` set it two or more times and
    /// one is an expression (which one BC reads is not pinned).
    fn field_auto_format_refusal(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        extensions: &[usize],
    ) -> Option<String> {
        let declared = f.declared_in.unwrap_or(oi);
        let (mut settings, mut expressions) = (0, 0);
        for o in std::iter::once(declared).chain(extensions.iter().copied().filter(|&e| e != declared)) {
            if !self.ws.usable(o) {
                continue;
            }
            let obj = self.ws.object_ast(o);
            settings += ast::sections_in(obj.syntax())
                .iter()
                .filter(|s| {
                    let at = if s.name().eq_ignore_ascii_case("field") {
                        1
                    } else if s.name().eq_ignore_ascii_case("modify") {
                        0
                    } else {
                        return false;
                    };
                    s.args().get(at).is_some_and(|n| n.eq_ignore_ascii_case(&f.name))
                        && s.properties()
                            .any(|p| p.name().eq_ignore_ascii_case("AutoFormatExpression"))
                })
                .count();
            expressions += crate::pages::field_auto_format_routines(&obj)
                .iter()
                .filter(|(n, _, _)| n.eq_ignore_ascii_case(&f.name))
                .count();
        }
        (settings >= 2 && expressions >= 1).then(|| {
            format!(
                "the format of field {}, whose AutoFormatExpression is set {settings} times by the table and its extensions, one of them an expression (which one BC reads is not pinned)",
                f.name
            )
        })
    }

    /// The routine `routines` names for field `f`: of the object that declares the field, or of the last of the table's
    /// `extensions` whose `modify` block sets one.
    fn field_expression_routine(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        extensions: &[usize],
        routine_at: &RoutineAt,
        routines: fn(&ast::Object) -> Vec<(String, String, String)>,
    ) -> Option<(usize, usize)> {
        let declared = f.declared_in.unwrap_or(oi);
        let mut found = None;
        for o in std::iter::once(declared).chain(extensions.iter().copied().filter(|&e| e != declared)) {
            if !self.ws.usable(o) {
                continue;
            }
            let obj = self.ws.object_ast(o);
            for (field, name, _) in routines(&obj) {
                if !field.eq_ignore_ascii_case(&f.name) {
                    continue;
                }
                if let Some(r) = obj.routines().find(|r| r.name() == name) {
                    let offset: usize = r.syntax().text_range().start().into();
                    if let Some(r) = routine_at.get(&(o, offset)) {
                        found = Some((o, r));
                    }
                }
            }
        }
        found
    }

    /// Why the caption of field `f` is refused: the table or table extension that declares it and the `modify` blocks of
    /// its `extensions` set its `CaptionClass` two or more times, and one is an expression. Which one BC reads is not
    /// pinned (the corpus pins one expression, pageext/lt-page-ext-properties/TableExtCaptionClass.al:124-149).
    fn field_caption_class_refusal(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
        extensions: &[usize],
    ) -> Option<String> {
        let declared = f.declared_in.unwrap_or(oi);
        let (mut settings, mut expressions) = (0, 0);
        for o in std::iter::once(declared).chain(extensions.iter().copied().filter(|&e| e != declared)) {
            if !self.ws.usable(o) {
                continue;
            }
            let obj = self.ws.object_ast(o);
            settings += crate::pages::field_caption_class_settings(&obj)
                .iter()
                .filter(|n| n.eq_ignore_ascii_case(&f.name))
                .count();
            expressions += crate::pages::field_caption_class_routines(&obj)
                .iter()
                .filter(|(n, _, _)| n.eq_ignore_ascii_case(&f.name))
                .count();
        }
        (settings >= 2 && expressions >= 1).then(|| {
            format!(
                "the caption of field {}, whose CaptionClass is set {settings} times by the table and its extensions, one of them an expression (which one BC reads is not pinned)",
                f.name
            )
        })
    }

    /// What a field's `TableRelation` gives the runtime: the arms, the arms of a relation with filter
    /// conditions, and the metadata of a relation the runtime does not understand. They read the properties of the
    /// field as the table and its extensions leave them (a `modify` block can change `TableRelation`).
    pub(crate) fn field_relations(
        &self,
        oi: usize,
        f: &crate::members::FieldInfo,
    ) -> (Vec<ir::RelationArm>, Vec<ir::RelationArm>, Vec<ir::MetaRelationArm>) {
        let arms = self.relation_arms(oi, f, false);
        let runtime = if arms.is_empty() && f.table_relation {
            self.relation_arms(oi, f, true)
        } else {
            Vec::new()
        };
        let meta = if arms.is_empty() && f.table_relation {
            self.relation_meta(oi, f)
        } else {
            Vec::new()
        };
        (arms, runtime, meta)
    }

    /// The runtime type of a field. An enum named by a field that a table extension of another app adds is
    /// resolved from the extension's file: the table's own app cannot see it.
    fn field_rt(&self, oi: usize, f: &crate::members::FieldInfo) -> RtType {
        let file = self.ws.objects[f.declared_in.unwrap_or(oi)].file;
        if let Some(r) = super::shared::scalar_rt(self.ws, file, &f.ty) {
            return r.unwrap_or(RtType::Variant);
        }
        self.rt(&f.ty).unwrap_or(RtType::Variant)
    }

    pub(crate) fn table_schema(
        &self,
        oi: usize,
        obj: &ast::Object,
        extensions: Vec<usize>,
        routine_at: &RoutineAt,
    ) -> (ir::Table, Vec<usize>) {
        let m = &self.members[oi];
        let fields = m
            .fields
            .iter()
            .map(|f| {
                let (calc, refusal) = match (&f.class, &f.calc_formula) {
                    (crate::members::FieldClass::FlowField, Some(text)) => {
                        match self.lower_calc(oi, f.declared_in.unwrap_or(oi), text) {
                            Ok(c) => (Some(c), None),
                            Err(e) => (None, Some(e)),
                        }
                    }
                    _ => (None, None),
                };
                let (arms, runtime_arms, meta) = if self.early_relations {
                    self.field_relations(oi, f)
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                };
                ir::Field {
                    id: f.id,
                    name: f.name.clone(),
                    ty: self.field_rt(oi, f),
                    stored: f.class == crate::members::FieldClass::Normal,
                    init: f.init_value.as_ref().and_then(|v| {
                        init_const(v, &f.ty).or_else(|| match f.ty {
                            // A member of the field's enum, by name; a blank-named member too
                            // (types/TestBlankEnumInitTests.Codeunit.al).
                            Ty::Enum(_) => {
                                let file = self.ws.objects[f.declared_in.unwrap_or(oi)].file;
                                self.calc_const(file, v, &f.ty).ok()
                            }
                            _ => None,
                        })
                    }),
                    auto_increment: f.auto_increment,
                    relation: f.table_relation,
                    init_text: f.init_value.clone(),
                    relation_table: f.relation_table.clone(),
                    relation_tables: f.relation_tables.clone(),
                    properties: f.properties.clone(),
                    relation_arms: arms,
                    relation_runtime: runtime_arms,
                    relation_meta: meta,
                    relation_texts: Vec::new(),
                    on_validate: match (f.declared_in, f.on_validate) {
                        (Some(o), Some(off)) => routine_at.get(&(o, off)).map(|r| (o, r)),
                        _ => None,
                    },
                    ext_before_validate: self.ext_field_triggers(&extensions, &f.name, "OnBeforeValidate", routine_at),
                    ext_after_validate: self.ext_field_triggers(&extensions, &f.name, "OnAfterValidate", routine_at),
                    on_lookup: match (f.declared_in, f.on_lookup) {
                        (Some(o), Some(off)) => routine_at.get(&(o, off)).map(|r| (o, r)),
                        _ => None,
                    },
                    caption_class: self.field_caption_class(oi, f, &extensions, routine_at),
                    caption_class_refusal: self.field_caption_class_refusal(oi, f, &extensions),
                    auto_format_expression: self.field_auto_format(oi, f, &extensions, routine_at),
                    auto_format_refusal: self.field_auto_format_refusal(oi, f, &extensions),
                    flow_filter: f.class == crate::members::FieldClass::FlowFilter,
                    calc,
                    calc_refusal: refusal,
                    declared_in: f.declared_in.map(|o| o as u32),
                }
            })
            .collect();
        let mut keys = Vec::new();
        let mut unique_keys = Vec::new();
        let mut key_props: Vec<Vec<(String, String)>> = Vec::new();
        // The object that declares each key (the table or one of its extensions).
        let mut key_owners: Vec<usize> = Vec::new();
        for o in std::iter::once(oi).chain(extensions.iter().copied()) {
            for sec in self
                .ws
                .object_ast(o)
                .sections()
                .filter(|s| s.name().eq_ignore_ascii_case("keys"))
            {
                for k in sec.sections().filter(|k| k.name().eq_ignore_ascii_case("key")) {
                    // A Removed key is gone: it takes no index position
                    // (corpus recordref/TestKeyMetadataRowVersionRemoved.al:10-20, :150-170).
                    if k.properties().any(|p| {
                        p.name().eq_ignore_ascii_case("ObsoleteState")
                            && p.value_text().trim().eq_ignore_ascii_case("Removed")
                    }) {
                        continue;
                    }
                    let names: Vec<String> = k
                        .args()
                        .get(1)
                        .map(|a| a.split(',').map(|n| n.trim().trim_matches('"').to_string()).collect())
                        .unwrap_or_default();
                    let pos: Vec<usize> = names
                        .iter()
                        .filter_map(|n| m.fields.iter().position(|f| f.name.eq_ignore_ascii_case(n)))
                        .collect();
                    if pos.len() == names.len() && !pos.is_empty() {
                        keys.push(pos);
                        key_owners.push(o);
                        key_props.push(k.properties().map(|p| (p.name(), p.value_text())).collect());
                        unique_keys.push(k.properties().any(|p| {
                            p.name().eq_ignore_ascii_case("Unique")
                                && p.value_text().trim().eq_ignore_ascii_case("true")
                        }));
                    }
                }
            }
        }
        if keys.is_empty() {
            // Without a keys section the first field is the primary key (doc: Table keys).
            keys.push(vec![0]);
            key_owners.push(oi);
            unique_keys.push(false);
            key_props.push(Vec::new());
        }
        let per_company = !obj.properties().any(|p| {
            p.name().eq_ignore_ascii_case("DataPerCompany") && p.value_text().trim().eq_ignore_ascii_case("false")
        });
        (
            ir::Table {
                fields,
                keys,
                unique_keys,
                key_props,
                per_company,
                extensions,
                unsupported: super::shared::over_max_length(obj),
            },
            key_owners,
        )
    }

    /// A FlowField's `CalcFormula` with field names resolved against the table
    /// `oi` it belongs to and object names in the file of `declared_in` (the table or table
    /// extension that wrote it). `Err` is why it cannot be calculated yet.
    pub(crate) fn lower_calc(&self, oi: usize, declared_in: usize, text: &str) -> Result<ir::CalcFormula, String> {
        use crate::calcformula::{CondValue, parse};
        let f = parse(text)?;
        // Names resolve where the field was written: its table or table extension.
        let file = self.ws.objects[declared_in].file;
        let Resolution::Found(src) = self.ws.resolve_ref(file, ObjKind::Table, &f.table_path) else {
            return Err(format!(
                "CalcFormula source: {}",
                self.ws.outside_workspace(ObjKind::Table, &f.table)
            ));
        };
        let src_fields = &self.members[src].fields;
        let src_pos = |name: &str| {
            src_fields
                .iter()
                .position(|x| x.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("CalcFormula field `{name}` is not in table `{}`", f.table))
        };
        let own = &self.members[oi].fields;
        let own_pos = |name: &str| {
            own.iter()
                .position(|x| x.name.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("CalcFormula refers to field `{name}`, which the table does not have"))
        };
        let field = f.field.as_deref().map(src_pos).transpose()?;
        let mut conds = Vec::new();
        for c in &f.conds {
            let pos = src_pos(&c.field)?;
            let decl = src_fields[pos].declared_in.map_or(file, |d| self.ws.objects[d].file);
            let value = match &c.value {
                CondValue::Const(raw) => ir::CalcValue::Const(self.calc_const(decl, raw, &src_fields[pos].ty)?),
                CondValue::Filter(raw) => {
                    // One quoted literal with filter operators in it (`filter('10|200..299')`): BC 29 reads the text as a
                    // filter (al-runner issue 5575 sums the entries of 250), the filter grammar here as one value. No
                    // corpus test pins the quoted form in a CalcFormula, so it is refused (docs/probes-needed.md "User
                    // issues (lane-user-trust)"). Without an operator both readings are the same value.
                    if crate::queries::whole_literal(raw)
                        .is_some_and(|inner| inner.contains(['|', '&', '.', '<', '>', '=', '*', '@', '(', ')']))
                    {
                        return Err(format!(
                            "CalcFormula filter {} is one quoted literal with filter operators in it: whether BC reads it as a filter is not pinned",
                            raw.trim()
                        ));
                    }
                    ir::CalcValue::Filter(rewrite_quotes(raw))
                }
                CondValue::Field(n) => ir::CalcValue::Field(own_pos(n)?),
                CondValue::UpperLimit(n) => ir::CalcValue::UpperLimit(own_pos(n)?),
                CondValue::FieldFilter(n) => ir::CalcValue::FieldFilter(own_pos(n)?),
            };
            conds.push(ir::CalcCond { field: pos, value });
        }
        // Two conditions on one source field: only a link replaced by a
        // field(filter()) on the same field is pinned (corpus
        // record/TestCalcFormulaFlowFiltersTests.Codeunit.al:230); anything
        // else is refused rather than guessed.
        for (i, c) in conds.iter().enumerate() {
            for d in &conds[i + 1..] {
                let replaces = matches!(
                    (&c.value, &d.value),
                    (ir::CalcValue::Field(_), ir::CalcValue::FieldFilter(_))
                        | (ir::CalcValue::FieldFilter(_), ir::CalcValue::Field(_))
                );
                if c.field == d.field && !replaces {
                    return Err(format!(
                        "CalcFormula has two conditions on the same source field `{}` (their combination is not pinned yet)",
                        src_fields[c.field].name
                    ));
                }
            }
        }
        Ok(ir::CalcFormula {
            method: f.method,
            negate: f.negate,
            table: src,
            field,
            conds,
        })
    }

    /// A `const(..)` value as a constant of the source field's type.
    pub(crate) fn calc_const(&self, file: usize, raw: &str, ty: &Ty) -> Result<ir::Const, String> {
        calc_const(self.ws, self.members, file, raw, ty)
    }
}

/// Whether a `filter(...)` or `const(...)` text of a TableRelation with a `"`-quoted member has a pinned form: on an Enum
/// or Option field, terms joined by `|`, each a plain member name or a quoted one, or `<>` and one quoted member
/// (record/lt-table-relation-quotes/TableRelationQuotes.al:41-46), and the ranges and `&` forms below.
/// Whether the filter(...) text of an arm's if (...) condition that quotes a member with `"` is a pinned form: on an Option
/// field, two `<>` terms joined by `&`, one quoted and one plain (`filter(<> "B C" & <> D)`), and members joined by `|`, quoted
/// or plain, the quoted blank member included (`filter(" " | A)`): record/lt-table-relation-quotes/TableRelationQuotes.al:76-123
/// (table 87157), QP_Arms_* and QP_BlankOr_* (corpus bump 46).
fn quoted_condition_text_pinned(text: &str, ty: &Ty) -> bool {
    if !matches!(ty, Ty::Option(_)) {
        return false;
    }
    let quoted = |t: &str| t.len() >= 2 && t.starts_with('"') && t.ends_with('"') && !t[1..t.len() - 1].contains('"');
    let plain = |t: &str| !t.is_empty() && t.chars().all(|c| c.is_alphanumeric() || c == '_');
    if text.contains('&') {
        let terms: Option<Vec<&str>> = text
            .split('&')
            .map(|p| p.trim().strip_prefix("<>").map(str::trim))
            .collect();
        return terms.is_some_and(|ts| {
            ts.len() == 2
                && ts.iter().filter(|t| quoted(t)).count() == 1
                && ts
                    .iter()
                    .all(|t| (quoted(t) && !t[1..t.len() - 1].trim().is_empty()) || plain(t))
        });
    }
    let terms: Vec<&str> = text.split('|').map(str::trim).collect();
    terms.len() >= 2 && terms.iter().all(|t| quoted(t) || plain(t))
}

fn quoted_relation_text_pinned(text: &str, ty: &Ty) -> bool {
    if !matches!(ty, Ty::Enum(_) | Ty::Option(_)) {
        return false;
    }
    // A quoted blank member (`" "`) is not among the pinned forms.
    let quoted = |t: &str| {
        t.len() >= 2
            && t.starts_with('"')
            && t.ends_with('"')
            && !t[1..t.len() - 1].contains('"')
            && !t[1..t.len() - 1].trim().is_empty()
    };
    let plain = |t: &str| !t.is_empty() && t.chars().all(|c| c.is_alphanumeric() || c == '_');
    // A range with one quoted and one plain bound (`A .. "B C"`, `"B C" .. D`, on an Enum and an Option field), and a quoted
    // lower bound with no upper one on an Enum field (`"B C" ..`): TableRelationQuotes.al:279-363, :383-397 (corpus bump 45).
    if let Some((l, r)) = text.split_once("..") {
        let (l, r) = (l.trim(), r.trim());
        if r.is_empty() {
            return quoted(l) && matches!(ty, Ty::Enum(_));
        }
        return (quoted(l) && plain(r)) || (plain(l) && quoted(r));
    }
    // Two `<>` terms joined by `&`, one quoted and one plain (`<> A & <> "B C"` on an Enum field, `<> "B C" & <> D` on an
    // Option field): TableRelationQuotes.al:313-329, :365-381 (corpus bump 45).
    if text.contains('&') {
        let terms: Option<Vec<&str>> = text
            .split('&')
            .map(|p| p.trim().strip_prefix("<>").map(str::trim))
            .collect();
        return terms.is_some_and(|ts| {
            ts.len() == 2 && ts.iter().filter(|t| quoted(t)).count() == 1 && ts.iter().all(|t| quoted(t) || plain(t))
        });
    }
    if let Some(rest) = text.trim().strip_prefix("<>") {
        return quoted(rest.trim());
    }
    text.split('|').map(str::trim).all(|t| quoted(t) || plain(t))
}
