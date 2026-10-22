//! The Field and Key tables: one row per field and per key of every compiled table.
//!
//! Pinned by corpus record/TestFieldDataClassificationVirtualTable.al,
//! TestFieldDataClassificationInheritance.al, TestFieldEnumTypeVirtualTable.al,
//! TestFieldObsoleteStateVirtualTable.al and TestKeyVirtualTable.al. A table nothing compiled has
//! no fields or keys in the program, so its rows are not known: they are poisoned, and a read that
//! might touch them is refused (`mat.rs`).

use super::mat::Entry;
use super::objects::{Catalog, ident, prop, string_lit, truthy};
use super::{Blank, Providers, blank, pos};
use al_builtins::value::{RtType, Value};
use al_data::TableDef;
use al_ir::{Field, Object};

const OBSOLETE_STATES: &[&str] = &["No", "Pending", "Removed"];
const DATA_CLASSIFICATIONS: &[&str] = &[
    "CustomerContent",
    "ToBeClassified",
    "EndUserIdentifiableInformation",
    "AccountData",
    "EndUserPseudonymousIdentifiers",
    "OrganizationIdentifiableInformation",
    "SystemMetadata",
];

fn ordinal_of(members: &[&str], text: &str) -> Option<i32> {
    members
        .iter()
        .position(|m| m.eq_ignore_ascii_case(&ident(text)))
        .map(|i| i as i32)
}

/// The Field "Type" member (ordinal of `TableFilter,RecordID,OemText,Date,Time,DateFormula,Decimal,
/// Media,MediaSet,Text,Code,Binary,BLOB,Boolean,Integer,OemCode,Option,BigInteger,Duration,GUID,
/// DateTime`) for a field type. An enum field is an option (TestFieldEnumTypeVirtualTable.al:56).
fn type_ordinal(ty: &RtType) -> Option<i32> {
    Some(match ty {
        RtType::Library(n) => match n.to_ascii_lowercase().as_str() {
            "recordid" => 1,
            "media" => 7,
            "mediaset" => 8,
            "blob" => 12,
            "tablefilter" => 0,
            _ => return None,
        },
        RtType::Date => 3,
        RtType::Time => 4,
        RtType::DateFormula => 5,
        RtType::Decimal => 6,
        RtType::Text(_) => 9,
        RtType::Code(_) => 10,
        RtType::Boolean => 13,
        RtType::Integer => 14,
        RtType::Option(_) | RtType::Enum(_) => 16,
        RtType::BigInteger => 17,
        RtType::Duration => 18,
        RtType::Guid => 19,
        RtType::DateTime => 20,
        _ => return None,
    })
}

/// The Field "Len" column: the declared length of a Text or Code, else the type's storage size
/// (record/TestFieldLenAndFeatureKeyContracts.al:104-129).
fn field_len(ty: &RtType) -> Option<i32> {
    Some(match ty {
        RtType::Text(Some(n)) | RtType::Code(Some(n)) => *n as i32,
        RtType::Integer | RtType::Boolean | RtType::Date | RtType::Time | RtType::Option(_) | RtType::Enum(_) => 4,
        RtType::BigInteger | RtType::DateTime | RtType::Duration => 8,
        RtType::Decimal => 12,
        RtType::Guid => 16,
        RtType::DateFormula => 32,
        RtType::Library(n) => match n.to_ascii_lowercase().as_str() {
            "blob" => 8,
            "media" | "mediaset" => 16,
            "recordid" => 448,
            "tablefilter" => 504,
            _ => return None,
        },
        _ => return None,
    })
}

/// A table of the program that nothing compiled: it has a name and an id and no fields.
fn shell_entries(o: &Object, t: &TableDef, key_of: &dyn Fn(u32) -> Vec<(u32, Value)>) -> Entry {
    let mut k = blank(t);
    for (id, v) in key_of(o.id.unwrap_or(0)) {
        k[pos(t, id)] = v;
    }
    Entry::poisoned(
        k,
        format!(
            "table '{}' was not compiled, so its fields and keys are not known",
            o.name
        ),
    )
}

fn table_of<'a>(c: &'a Catalog, i: usize) -> Option<(&'a Object, &'a al_ir::Table)> {
    let o = &c.objects[i];
    o.table.as_ref().map(|t| (o, t))
}

/// A `Get` of a Field row of a table of the app under test gives null App Package ID and App Runtime Package ID; a `Get` of a
/// Base Application table, and FindFirst and Next of either, give the AllObj ones
/// (metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:287-311). The app under test is the app whose two
/// ids are equal (`ids::runtime_package_id`).
pub(super) fn field_get(t: &TableDef, mut r: al_data::Row) -> al_data::Row {
    let col = |name: &str| t.fields.iter().position(|x| x.name == name);
    if let (Some(p), Some(rt)) = (col("App Package ID"), col("App Runtime Package ID")) {
        if let (Value::Guid(a), Value::Guid(b)) = (&r[p], &r[rt]) {
            if a == b {
                r[p] = Value::Guid([0; 16]);
                r[rt] = Value::Guid([0; 16]);
            }
        }
    }
    r
}

pub(super) fn field_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let blank = Blank::new(t);
    let mut out = Vec::new();
    // Four more system fields per table from the gate on (see [`USER_NAME_SYSTEM_FIELDS`]).
    let user_name_fields = al_version::gate_on(engine.target.as_ref(), al_version::Gate::PlatformMetadataColumnsFrom29);
    for (i, o) in c.objects.iter().enumerate() {
        let (Some(tid), al_ir::ObjKind::Table) = (o.id, o.kind) else {
            continue;
        };
        let Some((_, schema)) = table_of(&c, i) else {
            out.push(shell_entries(o, t, &|id| {
                vec![(1, Value::Integer(id as i32)), (2, Value::Integer(1))]
            }));
            continue;
        };
        for f in &schema.fields {
            // The rowversion is not a row of the Field table (record/TestFieldLenAndFeatureKeyContracts.al:107-112).
            if f.id == 2_000_000_005 {
                continue;
            }
            // A field with ObsoleteState = Moved has no row (Pending and Removed have one): the lookup finds nothing
            // (types/PWMisc.al:FieldTable_MovedAndPendingField, BC 27.5 and 29.0).
            if f.properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("ObsoleteState") && ident(v.trim()).eq_ignore_ascii_case("Moved"))
            {
                continue;
            }
            let mut r = blank.row();
            r[pos(t, 1)] = Value::Integer(tid as i32);
            r[pos(t, 2)] = Value::Integer(f.id as i32);
            // IsPartOfPrimaryKey: the field is in the primary key (metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:
            // FieldTable_IsPartOfPrimaryKey_OwnTable and _BaseApp).
            if let Some(col) = t.fields.iter().position(|x| x.name == "IsPartOfPrimaryKey") {
                let idx = schema.fields.iter().position(|x| x.id == f.id);
                let in_key = idx.is_some_and(|i| schema.keys.first().is_some_and(|k| k.contains(&i)));
                r[col] = Value::Boolean(in_key);
            }
            match field_row(&c, o, schema, f, t, &blank, &mut r) {
                Ok(()) => out.push(Entry::ok(r)),
                Err(why) => {
                    let mut k = blank.row();
                    k[pos(t, 1)] = Value::Integer(tid as i32);
                    k[pos(t, 2)] = Value::Integer(f.id as i32);
                    out.push(Entry::poisoned(k, why));
                }
            }
        }
        if let Some(e) = rowversion_key_row(schema, t, &blank, tid, &o.name) {
            out.push(e);
        }
        if user_name_fields {
            out.extend(user_name_system_field_rows(t, &blank, tid));
        }
    }
    out
}

/// The Field row of the rowversion of a table that declares a key over SystemRowVersion (its own key or one a
/// tableextension adds). A table without such a key has no row for it (metadata/W6FieldKeyRows.al:263-270,
/// record/TestFieldLenAndFeatureKeyContracts.al:107-116); whether a table with one has a row is not pinned
/// (docs/probes-needed.md). The row is poisoned: a read that might see it is refused. Its number is 0 and its name
/// `timestamp`, the number and name of RecordRef.Field(FieldNo(SystemRowVersion))
/// (record/TestRowVersionFieldNumberFieldActiveContracts.al:63-67). The name is kept as known so that a lookup by the name
/// of another field (names are unique in a table) still answers.
fn rowversion_key_row(schema: &al_ir::Table, t: &TableDef, blank: &Blank, tid: u32, table: &str) -> Option<Entry> {
    let rv = schema.fields.iter().position(|f| f.id == 2_000_000_005)?;
    if !schema.keys.iter().any(|k| k.contains(&rv)) {
        return None;
    }
    let mut r = blank.row();
    r[pos(t, 1)] = Value::Integer(tid as i32);
    r[pos(t, 2)] = Value::Integer(0);
    let name = pos(t, 4);
    r[name] = Value::Text("timestamp".into());
    Some(
        Entry::poisoned(
            r,
            format!(
                "the Field table row of SystemRowVersion of table '{table}', which declares a key over SystemRowVersion: whether such a table lists its rowversion in the Field table is not pinned (docs/probes-needed.md)"
            ),
        )
        .with_known_cols(vec![name]),
    )
}

/// The FlowFields every table has under `Gate::PlatformMetadataColumnsFrom29`, with the columns the corpus pins: number, name, type Text, length and class
/// FlowField (system-fields/W6SystemFieldStamps.al:239-246; their order after SystemModifiedBy: metadata/W6FieldKeyRows.al:
/// 267-268). The runtime's schema does not hold them (it keeps SystemRowVersion at 2000000005, where BC 27 and 28 have it;
/// where BC 29 keeps it is not pinned); RecordRef refuses them (refs.rs).
pub(crate) const USER_NAME_SYSTEM_FIELDS: [(u32, &str, i32); 4] = [
    (2_000_000_005, "SystemCreatedByUserName", 50),
    (2_000_000_006, "SystemCreatedByFullName", 80),
    (2_000_000_007, "SystemModifiedByUserName", 50),
    (2_000_000_008, "SystemModifiedByFullName", 80),
];

/// The columns of a [`USER_NAME_SYSTEM_FIELDS`] row that are set: TableNo, No., FieldName, Type, Len, Class (pinned above),
/// RelationTableNo 0 (handlers 69519 FieldTable_RelationTableNo_OfConditionalRelations on BC 29: a `<>0` filter lists
/// only the declared relations and SystemCreatedBy/SystemModifiedBy) and ObsoleteState. ObsoleteState is pinned only as
/// not Removed (record/TestFieldObsoleteStateVirtualTable.al:66-73 on BC 29); `No` is kept, not pinned, so that a filter
/// on it still answers (docs/probes-needed.md).
const PINNED_USER_NAME_COLUMNS: [u32; 8] = [1, 2, 4, 5, 6, 7, 21, 25];

fn user_name_system_field_rows(t: &TableDef, blank: &Blank, tid: u32) -> Vec<Entry> {
    static WHY: std::sync::OnceLock<std::sync::Arc<str>> = std::sync::OnceLock::new();
    let why = WHY
        .get_or_init(|| {
            "the value of a column of the Field row of a user-name system field (SystemCreatedByUserName and the others) is not known: only TableNo, No., FieldName, Type, Len, Class, RelationTableNo and ObsoleteState are known (docs/probes-needed.md)"
                .into()
        })
        .clone();
    USER_NAME_SYSTEM_FIELDS
        .iter()
        .map(|&(id, name, len)| {
            let mut r = blank.row();
            for (i, f) in t.fields.iter().enumerate() {
                if !PINNED_USER_NAME_COLUMNS.contains(&f.id) {
                    r[i] = Value::Unknown(why.clone());
                }
            }
            r[pos(t, 1)] = Value::Integer(tid as i32);
            r[pos(t, 2)] = Value::Integer(id as i32);
            r[pos(t, 4)] = Value::Text(name.into());
            r[pos(t, 5)] = blank.option(t, 5, 9);
            r[pos(t, 6)] = Value::Integer(len);
            r[pos(t, 7)] = blank.option(t, 7, 1);
            r[pos(t, 21)] = Value::Integer(0);
            r[pos(t, 25)] = blank.option(t, 25, 0);
            Entry::ok(r)
        })
        .collect()
}

fn field_row(
    c: &Catalog,
    table: &Object,
    schema: &al_ir::Table,
    f: &Field,
    t: &TableDef,
    blank: &Blank,
    r: &mut al_data::Row,
) -> Result<(), String> {
    let what = || format!("field '{}' of table '{}'", f.name, table.name);
    let fprop = |name: &str| {
        f.properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
    };
    // SystemId is named `$systemId` (record/TestFieldLenAndFeatureKeyContracts.al:107-112).
    // FieldName is cut to its 30 characters; a tableextension field may have a longer name
    // (tableextension/TestTableExtFieldModify.al:204-216).
    r[pos(t, 4)] = Value::Text(if f.id == 2_000_000_000 {
        "$systemId".into()
    } else {
        f.name.chars().take(30).collect::<String>().into()
    });
    // App Package ID and App Runtime Package ID: the table's AllObj ones (PlatformMetadataColumnValues.al:283-311, after
    // FindFirst and Next; `Get` of a table of the app under test leaves them null, see `field_get`). For a platform table they
    // are not pinned.
    for (name, which) in [("App Package ID", 0), ("App Runtime Package ID", 1)] {
        if let Some(col) = t.fields.iter().position(|x| x.name == name) {
            r[col] = if super::objects::platform_object(table) {
                super::unknown(
                    t,
                    t.fields[col].id,
                    &format!("table '{}', a platform table", table.name),
                )
            } else {
                let ids = super::objects::package_ids(c, table);
                Value::Guid(if which == 0 { ids.0 } else { ids.1 })
            };
        }
    }
    r[pos(t, 5)] = blank.option(
        t,
        5,
        type_ordinal(&f.ty).ok_or_else(|| format!("{}: the type is not understood", what()))?,
    );
    r[pos(t, 6)] =
        Value::Integer(field_len(&f.ty).ok_or_else(|| format!("{}: the length of the type is not pinned", what()))?);
    // Class: Normal, FlowField, FlowFilter (TestFieldDataClassificationInheritance.al, the FlowField case).
    let is_flow_field = !f.stored && !f.flow_filter;
    r[pos(t, 7)] = blank.option(t, 7, if f.flow_filter { 2 } else { i32::from(is_flow_field) });
    // TableName, Enabled and Field Caption come from the declaration (docs/probes-needed.md, probe batch 5):
    // the table's name, `Enabled = false`, and the Caption (else the English CaptionML, else the name).
    r[pos(t, 3)] = Value::Text(table.name.chars().take(30).collect());
    r[pos(t, 8)] = Value::Boolean(fprop("Enabled").is_none_or(truthy));
    r[pos(t, 20)] = Value::Text(
        super::objects::declared_caption(fprop("Caption"), fprop("CaptionML"), &f.name)
            .map_err(|e| format!("{}: {e}", what()))?
            .chars()
            .take(80)
            .collect(),
    );
    // Type Name of a Text field is `Text` and its length (record/PZFieldObsoleteRows.al:FieldRows_AllColumns_OfLivePendingAndRemovedField,
    // `Text30`). The other types' names are not pinned: unknown, so a read of one is refused.
    r[pos(t, 9)] = match &f.ty {
        RtType::Text(Some(n)) => Value::Text(format!("Text{n}").into()),
        // One reason for all of them: the Field table has tens of thousands of rows.
        _ => {
            static WHY: std::sync::OnceLock<std::sync::Arc<str>> = std::sync::OnceLock::new();
            let why = WHY.get_or_init(|| super::unknown_why(t, 9, "a field that is not a Text field"));
            Value::Unknown(why.clone())
        }
    };
    // ExternalName (10) is blank and SQLDataType (23) Varchar for a Text field that declares neither (record/
    // PZFieldObsoleteRows.al:48, :52: `Text30` fields). A declared ExternalName and the SQLDataType of the other
    // types are pinned by no corpus test: unknown, so a read of one is refused.
    if let Some(v) = fprop("ExternalName") {
        r[pos(t, 10)] = super::unknown(t, 10, &format!("{}, which declares ExternalName = {v}", what()));
    }
    if t.fields.iter().any(|x| x.id == 23) {
        r[pos(t, 23)] = match (&f.ty, fprop("SQLDataType")) {
            (RtType::Text(Some(_)), None) => match &blank.row[pos(t, 23)] {
                Value::Option(_, m) => match m.iter().position(|x| x.eq_ignore_ascii_case("Varchar")) {
                    Some(n) => Value::Option(n as i32, m.clone()),
                    None => super::unknown(t, 23, &format!("{}: SQLDataType has no member Varchar", what())),
                },
                _ => super::unknown(t, 23, &format!("{}: SQLDataType is not an option", what())),
            },
            _ => {
                static WHY: std::sync::OnceLock<std::sync::Arc<str>> = std::sync::OnceLock::new();
                let why = WHY.get_or_init(|| super::unknown_why(t, 23, "a field that is not a Text field"));
                Value::Unknown(why.clone())
            }
        };
    }
    // OptimizeForTextSearch: the declared property of a Text field, false where it is not declared
    // (lt-ak-fulltext/LtAkFulltext.al:119, :122). Declared on another type it is pinned by no test: unknown.
    if let Some(col) = t.fields.iter().position(|x| x.id == 62) {
        if let Some(v) = fprop("OptimizeForTextSearch") {
            r[col] = match &f.ty {
                RtType::Text(_) => Value::Boolean(truthy(v)),
                _ => super::unknown(
                    t,
                    62,
                    &format!("{}, which declares OptimizeForTextSearch on a non-Text field", what()),
                ),
            };
        }
    }
    // OptionString: the member names of an option or enum field, comma-joined (TestFieldEnumTypeVirtualTable.al:19-47).
    let option_string = match &f.ty {
        RtType::Option(m) => m.join(","),
        RtType::Enum(e) => c.objects[*e as usize]
            .enum_values
            .iter()
            .map(|(_, n)| n.as_str())
            .collect::<Vec<_>>()
            .join(","),
        _ => String::new(),
    };
    r[pos(t, 24)] = Value::Text(option_string.into());
    // ObsoleteState and ObsoleteReason (TestFieldObsoleteStateVirtualTable.al:18-45).
    let state = match fprop("ObsoleteState") {
        None => 0,
        Some(v) => ordinal_of(OBSOLETE_STATES, v)
            .ok_or_else(|| format!("{}: ObsoleteState `{v}` is not understood", what()))?,
    };
    r[pos(t, 25)] = blank.option(t, 25, state);
    r[pos(t, 26)] = Value::Text(fprop("ObsoleteReason").and_then(string_lit).unwrap_or_default().into());
    // DataClassification: the field's own; else the table's; a FlowField is SystemMetadata and a
    // silent Blob is CustomerContent (TestFieldDataClassificationInheritance.al:17-50).
    let is_blob = matches!(&f.ty, RtType::Library(al_builtins::library_kind::LibraryKind::Blob));
    let dc = match (fprop("DataClassification"), is_flow_field, is_blob) {
        (_, true, _) => 6,
        (Some(v), _, _) => ordinal_of(DATA_CLASSIFICATIONS, v)
            .ok_or_else(|| format!("{}: DataClassification `{v}` is not understood", what()))?,
        (None, _, true) => 0,
        (None, _, false) => match prop(table, "DataClassification") {
            None => 0,
            Some(v) => ordinal_of(DATA_CLASSIFICATIONS, v)
                .ok_or_else(|| format!("table '{}': DataClassification `{v}` is not understood", table.name))?,
        },
    };
    r[pos(t, 27)] = blank.option(t, 27, dc);
    // RelationTableNo and RelationFieldNo (TestFieldDataClassificationVirtualTable.al:50-72).
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
            return Ok(());
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
            // (handlers/ProbeObjectMetadataTables.al:FieldTable_RelationTableNo_OfConditionalRelations,
            // EnumConditionRef; a `where(...)` clause does not make a relation conditional, :FilterConditionRef).
            ([arm], _) if !arm.conditions.is_empty() => return Ok(()),
            ([arm], _) => Some((arm.table, arm.field, arm.table_name.clone())),
            ([], [m]) => Some((Some(m.table), m.field, c.objects[m.table].name.clone())),
            // A conditional relation (if ... else if ...) reads 0 and 0 (record/
            // TestPlatformMetadataDeclarationContracts.al:262-264).
            _ => return Ok(()),
        };
    // Created By and Modified By relate to the User table's field 0 (record/
    // TestPlatformMetadataDeclarationContracts.al:240-241).
    if matches!(f.id, 2_000_000_002 | 2_000_000_004) {
        r[pos(t, 21)] = Value::Integer(2_000_000_120);
        r[pos(t, 22)] = Value::Integer(0);
        return Ok(());
    }
    if let Some((table_index, field, table_name)) = single {
        let Some(rt) = table_index else {
            return Err(format!("{}: the related table `{table_name}` is not compiled", what()));
        };
        let related = &c.objects[rt];
        let Some(related_schema) = &related.table else {
            return Err(format!("{}: the related table `{table_name}` is not compiled", what()));
        };
        let field_pos = field.unwrap_or_else(|| {
            related_schema
                .keys
                .first()
                .and_then(|k| k.first())
                .copied()
                .unwrap_or(0)
        });
        r[pos(t, 21)] = Value::Integer(related.id.unwrap_or(0) as i32);
        r[pos(t, 22)] = Value::Integer(related_schema.fields.get(field_pos).map_or(0, |x| x.id) as i32);
    }
    let _ = schema;
    Ok(())
}

pub(super) fn key_rows(p: &Providers, t: &TableDef) -> Vec<Entry> {
    let engine = p.engine();
    let c = Catalog::new(&engine);
    let blank = Blank::new(t);
    let mut out = Vec::new();
    for (i, o) in c.objects.iter().enumerate() {
        let (Some(tid), al_ir::ObjKind::Table) = (o.id, o.kind) else {
            continue;
        };
        let Some((_, schema)) = table_of(&c, i) else {
            out.push(shell_entries(o, t, &|id| {
                vec![(1, Value::Integer(id as i32)), (2, Value::Integer(1))]
            }));
            continue;
        };
        for (n, key) in schema.keys.iter().enumerate() {
            let props = schema.key_props.get(n).map_or(&[][..], Vec::as_slice);
            let kprop = |name: &str| {
                props
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(name))
                    .map(|(_, v)| v.trim())
            };
            let mut r = blank.row();
            r[pos(t, 1)] = Value::Integer(tid as i32);
            r[pos(t, 2)] = Value::Integer(n as i32 + 1);
            // The fields, comma-separated in declaration order (TestKeyVirtualTable.al:73-82).
            r[pos(t, 4)] = Value::Text(
                key.iter()
                    .map(|&f| schema.fields[f].name.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
                    .into(),
            );
            // SumIndexFields: the names of the property, comma-joined without the blanks and quotes
            // (metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al:KeyTable_SumIndexFields_OwnTable, _BaseApp).
            if let (Some(col), Some(sum)) = (
                t.fields.iter().position(|x| x.name == "SumIndexFields"),
                kprop("SumIndexFields"),
            ) {
                let names: Vec<String> = sum.split(',').map(|n| n.trim().trim_matches('"').to_string()).collect();
                r[col] = Value::Text(names.join(",").into());
            }
            r[pos(t, 7)] = Value::Boolean(kprop("Enabled").is_none_or(truthy));
            r[pos(t, 10)] = Value::Boolean(n == 0 || kprop("Clustered").is_some_and(truthy));
            // True only for a key that has SumIndexFields and does not say `MaintainSIFTIndex = false`
            // (record/TestPlatformMetadataDeclarationContracts.al:269-280).
            r[pos(t, 9)] =
                Value::Boolean(kprop("SumIndexFields").is_some() && kprop("MaintainSIFTIndex").is_none_or(truthy));
            let state = match kprop("ObsoleteState") {
                None => 0,
                Some(v) => match ordinal_of(OBSOLETE_STATES, v) {
                    Some(i) => i,
                    None => {
                        out.push(Entry::poisoned(
                            {
                                let mut k = blank.row();
                                k[pos(t, 1)] = Value::Integer(tid as i32);
                                k[pos(t, 2)] = Value::Integer(n as i32 + 1);
                                k
                            },
                            format!(
                                "key {} of table '{}': ObsoleteState `{v}` is not understood",
                                n + 1,
                                o.name
                            ),
                        ));
                        continue;
                    }
                },
            };
            r[pos(t, 11)] = blank.option(t, 11, state);
            r[pos(t, 13)] = Value::Boolean(schema.unique_keys.get(n).copied().unwrap_or(false));
            out.push(Entry::ok(r));
        }
        // BC adds a unique key on the SystemId after the declared ones (TestKeyVirtualTable.al:50-56).
        let mut r = blank.row();
        r[pos(t, 1)] = Value::Integer(tid as i32);
        r[pos(t, 2)] = Value::Integer(schema.keys.len() as i32 + 1);
        r[pos(t, 4)] = Value::Text("$systemId".into());
        r[pos(t, 7)] = Value::Boolean(true);
        r[pos(t, 13)] = Value::Boolean(true);
        out.push(Entry::ok(r));
    }
    out
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
