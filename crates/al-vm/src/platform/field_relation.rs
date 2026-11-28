//! The Field table's RelationTableNo and RelationFieldNo columns (columns 21 and 22) of one field row.
//!
//! Pinned by corpus record/TestFieldDataClassificationVirtualTable.al:50-72,
//! record/TestPlatformMetadataDeclarationContracts.al:240-264, metadata/platform-tables/ProbeObjectMetadataTables.al and
//! tableextension/TestTableExtFieldTableRelation.al. A relation that names no field: metadata/platform-tables/
//! FieldRelationFieldNo.al (`no_field_measured`).
//!
//! RelationTableNo and RelationFieldNo of a Field row (`relation_columns`, called by `field_row`); a
//! relation without a field reads RelationFieldNo 0 in the shapes `no_field_measured` allows
//! (metadata/platform-tables/FieldRelationFieldNo.al), in the two-field key shapes of issue 8592 through the
//! assumption `relation-field-no-key-shapes` (`ASSUMED_SHAPES`), and refuses a read of it in every other.

use super::objects::{Catalog, prop};
use super::pos;
use al_builtins::assumptions::Assumption;
use al_builtins::value::{RtType, Value};
use al_data::TableDef;
use al_ir::{Field, ObjectHead as Object, RelationArm, RelationWhere, Table};

/// The platform User table, the target of `TableRelation = User`.
const USER_TABLE: u32 = 2_000_000_120;

/// Writes RelationTableNo and RelationFieldNo of field `f` of `table` into row `r`, with the assumption the values written
/// rest on, if any. `Err`: the whole row is refused.
pub(super) fn relation_columns(
    c: &Catalog,
    table: &Object,
    f: &Field,
    t: &TableDef,
    r: &mut al_data::Row,
) -> Result<Option<Assumption>, String> {
    let what = || format!("field '{}' of table '{}'", f.name, table.name);
    // The one related table of a relation: from the arms the runtime understands, or, for a relation
    // with a `filter(...)` condition, from the tables and fields the compile kept (`relation_meta`).
    // A relation a tableextension's `modify(..) { TableRelation }` extended (one property per object, the base's
    // first): a conditional base relation reads 0 whatever the extension adds (tableextension/
    // TestTableExtFieldTableRelation.al:368, :409, :450, :491). An unconditional base relation keeps its table when
    // the extension's relation ends in an unconditional arm (an unconditional relation :266, an else arm :532); an
    // extension relation that ends in a condition is not pinned.
    let relations: Vec<&str> = f.relation_texts.iter().map(String::as_str).collect();
    if relations.len() > 1 {
        if relation_starts_with_if(relations[0]) {
            return Ok(None);
        }
        if !relations[1..].iter().all(|t| relation_ends_unconditional(t)) {
            return Err(format!(
                "{}: the TableRelation a tableextension adds ends in a condition; the Field row of such a relation is not pinned",
                what()
            ));
        }
    }
    let single: Option<(Option<usize>, Option<usize>, String)> =
        match (f.relation_arms.as_slice(), f.relation_meta.as_slice()) {
            (arms, _) if relations.len() > 1 && !arms.is_empty() => {
                Some((arms[0].table, arms[0].field, arms[0].table_name.clone()))
            }
            (_, metas) if relations.len() > 1 && !metas.is_empty() => {
                let m = &metas[0];
                Some((Some(m.table), m.field, c.objects[m.table].name.clone()))
            }
            ([], []) if f.relation => return Err(format!("{}: the TableRelation is not understood", what())),
            ([], []) => None,
            // A relation whose only arm has an `if` condition is conditional too: 0 and 0
            // (metadata/platform-tables/ProbeObjectMetadataTables.al:FieldTable_RelationTableNo_OfConditionalRelations,
            // EnumConditionRef; a `where(...)` clause does not make a relation conditional, :FilterConditionRef).
            ([arm], _) if !arm.conditions.is_empty() => return Ok(None),
            ([arm], _) => Some((arm.table, arm.field, arm.table_name.clone())),
            ([], [m]) => Some((Some(m.table), m.field, c.objects[m.table].name.clone())),
            // A conditional relation (if ... else if ...) reads 0 and 0 (record/
            // TestPlatformMetadataDeclarationContracts.al:262-264).
            _ => return Ok(None),
        };
    // Created By and Modified By relate to the User table's field 0 (record/
    // TestPlatformMetadataDeclarationContracts.al:240-241).
    if matches!(f.id, 2_000_000_002 | 2_000_000_004) {
        r[pos(t, 21)] = Value::Integer(USER_TABLE as i32);
        r[pos(t, 22)] = Value::Integer(0);
        return Ok(None);
    }
    if let Some((table_index, field, table_name)) = single {
        let Some(rt) = table_index else {
            return Err(format!("{}: the related table `{table_name}` is not compiled", what()));
        };
        let related = &c.objects[rt];
        let Some(related_schema) = &related.table else {
            return Err(format!("{}: the related table `{table_name}` is not compiled", what()));
        };
        r[pos(t, 21)] = Value::Integer(related.id.unwrap_or(0) as i32);
        if let Some(p) = field {
            r[pos(t, 22)] = Value::Integer(related_schema.fields.get(p).map_or(0, |x| x.id) as i32);
            return Ok(None);
        }
        // No field written: 0 in the shapes FieldRelationFieldNo.al measured, refused in every other.
        let measured = match f.relation_arms.as_slice() {
            // `relation_texts` is empty when no tableextension `modify` extended the relation.
            [arm] if relations.is_empty() => no_field_measured(c, table, f, arm, related, related_schema),
            _ => NoField::NotMeasured,
        };
        let assumed = match measured {
            NoField::Pinned => None,
            NoField::Assumed => Some(Assumption::RelationFieldNoKeyShapes),
            NoField::NotMeasured => {
                r[pos(t, 22)] = super::unknown(
                    t,
                    22,
                    &format!("{}, a TableRelation without a field of a shape not measured", what()),
                );
                return Ok(None);
            }
        };
        r[pos(t, 22)] = Value::Integer(0);
        return Ok(assumed);
    }
    Ok(None)
}

/// What [`no_field_measured`] found for a relation that names a table and no field.
enum NoField {
    /// A shape the pinned corpus measured.
    Pinned,
    /// A shape of [`ASSUMED_SHAPES`], through the assumption `relation-field-no-key-shapes`.
    Assumed,
    /// Any other shape: a read of RelationFieldNo is refused.
    NotMeasured,
}

/// A field type of the shapes corpus PR 1546 measured, length included.
#[derive(Clone, Copy, PartialEq)]
enum Measured {
    Integer,
    Code20,
    Text30,
}

fn measured_type(t: &RtType) -> Option<Measured> {
    match t {
        RtType::Integer => Some(Measured::Integer),
        RtType::Code(Some(20)) => Some(Measured::Code20),
        RtType::Text(Some(30)) => Some(Measured::Text30),
        _ => None,
    }
}

/// The primary key of the source table the shapes of [`ASSUMED_SHAPES`] were measured on: (Integer, Code[20])
/// (FieldRelationFieldNo.al:570-591, table 75929 "RFN Source Keyed"). A source table keyed otherwise is not measured.
const MEASURED_SOURCE_KEY: [Measured; 2] = [Measured::Integer, Measured::Code20];

/// The shapes of corpus PR 1546 (merged, not yet pinned; FieldRelationFieldNo.al:382-447, tables :516-591), each by
/// equality: the source field's type, its position in [`MEASURED_SOURCE_KEY`] (`None`: outside the key), and the types of
/// the target's primary key. The control at :371 (a Code[20] field outside its key onto one Code[20] key field) is a pinned
/// shape of [`no_field_measured`].
const ASSUMED_SHAPES: [(Measured, Option<usize>, &[Measured]); 6] = [
    // :382, the source field the second field of its own key.
    (Measured::Code20, Some(1), &[Measured::Code20]),
    // :393
    (Measured::Code20, None, &[Measured::Code20, Measured::Text30]),
    // :404
    (Measured::Integer, None, &[Measured::Integer, Measured::Code20]),
    // :415, the shape of issue 8592.
    (Measured::Code20, None, &[Measured::Integer, Measured::Code20]),
    // :426
    (Measured::Code20, None, &[Measured::Code20, Measured::Integer]),
    // :437
    (Measured::Integer, None, &[Measured::Code20, Measured::Integer]),
];

/// Whether field `f` of `table` onto a target keyed by fields of types `key` is one of [`ASSUMED_SHAPES`], its table keyed
/// by [`MEASURED_SOURCE_KEY`].
fn assumed_shape(table: &Object, f: &Field, key: &[&RtType]) -> bool {
    let Some(schema) = table.table.as_ref() else {
        return false;
    };
    let own_key = schema.keys.first().map(Vec::as_slice).unwrap_or_default();
    let own_types: Option<Vec<Measured>> = own_key
        .iter()
        .map(|&p| schema.fields.get(p).and_then(|x| measured_type(&x.ty)))
        .collect();
    let key_types: Option<Vec<Measured>> = key.iter().map(|t| measured_type(t)).collect();
    let (Some(own_types), Some(key_types), Some(source)) = (own_types, key_types, measured_type(&f.ty)) else {
        return false;
    };
    let Some(idx) = schema.fields.iter().position(|x| x.id == f.id) else {
        return false;
    };
    let place = own_key.iter().position(|&p| p == idx);
    own_types == MEASURED_SOURCE_KEY
        && ASSUMED_SHAPES
            .iter()
            .any(|(s, p, k)| *s == source && *p == place && *k == key_types.as_slice())
}

/// The shapes of a relation that names a table and no field whose RelationFieldNo BC gives as 0, the target table staying
/// RelationTableNo (metadata/platform-tables/FieldRelationFieldNo.al:161-180 FieldTable_RelationWithoutField, :217-233
/// FieldTable_FlowFilterExtensionPlatformBaseApp, :236-260 FieldTable_FilterOnRelationFieldNo; fields :88-124 and :137).
/// Each fact is required by equality, as the probe has it:
/// - one arm, no condition (the caller), the target named alone, also after a namespace (`table_name`; a relation onto
///   SystemRowVersion is lowered to no field and names the field after the table);
/// - a Normal (:88) or FlowFilter (:113-117) field, declared by the table or a table extension of the table's app (:137);
/// - the field has no ObsoleteState, Enabled or TestTableRelation other than the default (none in the probe;
///   ValidateTableRelation = false is measured, :120-124);
/// - source and target tables of TableType Normal, two different tables, the field outside the source's primary key
///   (:80-131, key on Id);
/// - a Code field onto a table of the table's app keyed by one Code field of the same length (Code[20], :88, :28) or by two
///   Code fields of the same length (Code[10], :93, :62-63), with no where or one `const` where (:96, :97); an Integer field
///   onto one keyed by one Integer field (:91, :46), no where; a Guid field onto the platform User table (:118), no where.
///
/// [`NoField::Assumed`] (issue 8592): a shape of [`ASSUMED_SHAPES`], with the facts above but for the keys, narrowed to what
/// the probe had: a Normal field the table itself declares, no where, the target of the table's app.
fn no_field_measured(
    c: &Catalog,
    table: &Object,
    f: &Field,
    arm: &RelationArm,
    related: &Object,
    related_schema: &Table,
) -> NoField {
    let name = arm.table_name.replace('"', "");
    let named_alone = name.eq_ignore_ascii_case(&related.name)
        || name.len() > related.name.len()
            && name.is_char_boundary(name.len() - related.name.len() - 1)
            && name[name.len() - related.name.len() - 1..].eq_ignore_ascii_case(&format!(".{}", related.name));
    let no_where = matches!(arm.wheres.as_deref(), None | Some([]));
    let one_const = matches!(
        arm.wheres.as_deref(),
        Some([RelationWhere::Value { filter: false, .. }])
    );
    let class = f.stored || f.flow_filter;
    // The table or a table extension of the table's app declares the field.
    let declared = f
        .declared_in
        .and_then(|i| c.objects.get(i as usize))
        .is_some_and(|o| o.app == table.app);
    let defaults = [
        ("ObsoleteState", "No"),
        ("Enabled", "true"),
        ("TestTableRelation", "true"),
    ]
    .iter()
    .all(|(k, default)| {
        f.properties
            .iter()
            .filter(|(n, _)| n.eq_ignore_ascii_case(k))
            .all(|(_, v)| v.trim().eq_ignore_ascii_case(default))
    });
    let normal = |o: &Object| prop(o, "TableType").is_none_or(|v| v.eq_ignore_ascii_case("Normal"));
    let tables = normal(table) && normal(related) && related.id.is_some() && related.id != table.id;
    let outside_key = table.table.as_ref().is_some_and(|s| {
        let idx = s.fields.iter().position(|x| x.id == f.id);
        idx.is_some() && !s.keys.first().is_some_and(|k| idx.is_some_and(|i| k.contains(&i)))
    });
    let key: Vec<&RtType> = related_schema
        .keys
        .first()
        .map(|k| {
            k.iter()
                .filter_map(|&p| related_schema.fields.get(p).map(|x| &x.ty))
                .collect()
        })
        .unwrap_or_default();
    let same_app = related.app == table.app;
    let types = match (&f.ty, key.as_slice()) {
        (RtType::Code(n), [RtType::Code(k)]) => same_app && n == k && (no_where || one_const),
        (RtType::Code(n), [RtType::Code(a), RtType::Code(b)]) => {
            same_app && n == a && n == b && (no_where || one_const)
        }
        (RtType::Integer, [RtType::Integer]) => same_app && no_where,
        (RtType::Guid, _) => related.id == Some(USER_TABLE) && no_where,
        _ => false,
    };
    let common = named_alone && defaults && tables && same_app;
    if named_alone && class && declared && defaults && tables && outside_key && types {
        return NoField::Pinned;
    }
    // The probe's fields are Normal fields the table itself declares (FieldRelationFieldNo.al:570-591).
    let declared_by_table = f
        .declared_in
        .and_then(|i| c.objects.get(i as usize))
        .is_some_and(|o| o.kind == al_ir::ObjKind::Table && o.app == table.app);
    let normal_field = f.stored && !f.flow_filter;
    if common && normal_field && declared_by_table && no_where && assumed_shape(table, f, &key) {
        return NoField::Assumed;
    }
    NoField::NotMeasured
}

/// Whether a `TableRelation` text starts with an `if` condition.
fn relation_starts_with_if(text: &str) -> bool {
    let t = text.trim_start();
    t.get(..2).is_some_and(|s| s.eq_ignore_ascii_case("if"))
        && !t[2..].starts_with(|c: char| c.is_alphanumeric() || c == '_' || c == '"')
}

/// Whether a `TableRelation` text's last arm has no condition: it has no `if`, or its last `else` is not an
/// `else if`. Words inside quotes or `where(..)` texts are not told apart; a name with the word `else` in it
/// reads as an else arm only when quoted, so quoted parts are skipped.
fn relation_ends_unconditional(text: &str) -> bool {
    if !relation_starts_with_if(text) {
        return true;
    }
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in text.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => {
                quote = Some(c);
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
                words.push(String::from("\u{0}"));
            }
            None if c.is_alphanumeric() || c == '_' => cur.push(c.to_ascii_lowercase()),
            None => {
                if !cur.is_empty() {
                    words.push(std::mem::take(&mut cur));
                }
            }
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    match words.iter().rposition(|w| w == "else") {
        Some(i) => words.get(i + 1).is_some_and(|w| w != "if"),
        None => false,
    }
}
