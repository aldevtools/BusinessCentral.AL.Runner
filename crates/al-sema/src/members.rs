//! Members of objects: table fields (with extensions merged), enum values
//! (with extensions merged), and procedure signatures.

use crate::types::Ty;
use crate::workspace::{ObjKind, Workspace};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldClass {
    Normal,
    FlowField,
    FlowFilter,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInfo {
    pub id: u32,
    pub name: String,
    pub ty: Ty,
    pub class: FieldClass,
    /// Object index of the table or tableextension that declares it (None for system fields).
    pub declared_in: Option<usize>,
    /// The `InitValue` property as written.
    pub init_value: Option<String>,
    pub auto_increment: bool,
    /// The `CalcFormula` property as written.
    pub calc_formula: Option<String>,
    pub table_relation: bool,
    /// Lower-case name of the table the `TableRelation` points to.
    pub relation_table: Option<String>,
    /// Lower-case names of every table of the relation (all arms of a conditional one).
    pub relation_tables: Vec<String>,
    /// Offset of the field's OnValidate trigger node (in `declared_in`'s file).
    pub on_validate: Option<usize>,
    /// Offset of the field's OnLookup trigger node (in `declared_in`'s file).
    pub on_lookup: Option<usize>,
    /// Every property of the field as written (`Caption`, `OptionCaption`, ...).
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcParam {
    pub name: String,
    pub ty: Ty,
    pub by_var: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcSig {
    pub name: String,
    pub params: Vec<ProcParam>,
    /// None when the procedure returns nothing.
    pub ret: Option<Ty>,
    pub local: bool,
    /// Object index of the declaring object.
    pub declared_in: usize,
    /// Offset of the routine node in its file.
    pub offset: usize,
}

/// Where an interface implementation of an enum is declared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImplSlot {
    /// `Implementation = I = C` on the value with this ordinal.
    Value(u32),
    /// `DefaultImplementation = I = C` on the enum.
    Default,
    /// `UnknownValueImplementation = I = C` on the enum.
    Unknown,
}

/// One `Interface = Codeunit` pair of an enum's `Implementation`, `DefaultImplementation` or
/// `UnknownValueImplementation` property, names as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumImpl {
    pub slot: ImplSlot,
    pub interface: Vec<String>,
    pub codeunit: Vec<String>,
    /// Object index of the enum or enumextension that declares it (names resolve from its file).
    pub declared_in: usize,
}

#[derive(Debug, Default, Clone)]
pub struct Members {
    /// Interface implementations of an enum, extensions merged.
    pub enum_impls: Vec<EnumImpl>,
    pub fields: Vec<FieldInfo>,
    pub enum_values: Vec<(u32, String)>,
    /// `Caption` of enum values that declare one, by ordinal.
    pub enum_captions: Vec<(u32, String)>,
    /// An enum states `AssignmentCompatibility = true`: an enum of another type assigns into it
    /// (corpus must-not-compile 0074, 0075; types/PXEnumRecordRef.al:7-10).
    pub assignment_compatible: bool,
    pub procedures: Vec<ProcSig>,
}

impl Members {
    pub fn field(&self, name: &str) -> Option<&FieldInfo> {
        self.fields.iter().find(|f| f.name.eq_ignore_ascii_case(name))
    }
    pub fn procedures_named(&self, name: &str) -> Vec<&ProcSig> {
        self.procedures
            .iter()
            .filter(|p| p.name.eq_ignore_ascii_case(name))
            .collect()
    }
}

/// The members of every object, computed when first asked for. An extended object's
/// members include those of its extensions. Asking parses the object's file if it is a
/// dependency file that was only scanned at load (ADR-013 decision 6).
pub struct MemberTable<'a> {
    ws: &'a Workspace,
    cells: Vec<std::sync::OnceLock<Members>>,
}

impl<'a> MemberTable<'a> {
    pub fn new(ws: &'a Workspace) -> MemberTable<'a> {
        MemberTable {
            ws,
            cells: (0..ws.objects.len()).map(|_| std::sync::OnceLock::new()).collect(),
        }
    }

    /// The members object `i` declares itself, without those of its extensions.
    pub fn own(&self, i: usize) -> Members {
        if self.ws.usable(i) {
            own_members(self.ws, i)
        } else {
            Members::default()
        }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn get(&self, i: usize) -> &Members {
        self.cells[i].get_or_init(|| merged_members(self.ws, i))
    }

    /// Every object's members, computing the ones not asked for yet.
    pub fn iter(&self) -> impl Iterator<Item = &Members> {
        (0..self.cells.len()).map(|i| self.get(i))
    }
}

impl std::ops::Index<usize> for MemberTable<'_> {
    type Output = Members;
    fn index(&self, i: usize) -> &Members {
        self.get(i)
    }
}

/// Builds the member table; an extended object's members include those of its extensions.
pub fn build(ws: &Workspace) -> MemberTable<'_> {
    MemberTable::new(ws)
}

/// Members of object `i` with its extensions merged in. An object whose source cannot be
/// used (a dependency file that does not parse) has none.
fn merged_members(ws: &Workspace, i: usize) -> Members {
    if !ws.usable(i) {
        return Members::default();
    }
    let mut target = own_members(ws, i);
    for &e in ws.extensions_of(i) {
        let o = &ws.objects[e];
        if !ws.usable(e) {
            continue;
        }
        let ext = own_members(ws, e);
        target.fields.extend(ext.fields);
        target.enum_values.extend(ext.enum_values);
        target.enum_captions.extend(ext.enum_captions);
        target.enum_impls.extend(ext.enum_impls);
        target.procedures.extend(ext.procedures);
        // `modify(Field) { Caption = ...; }` changes properties of an existing field
        // (corpus tableextension/TestTableExtFieldModify.al:40-67, record/TestRecordCaptionSeparators.al:79-85).
        if o.kind == ObjKind::TableExtension {
            for (name, props) in modify_properties(ws, e) {
                let Some(field) = target.fields.iter_mut().find(|f| f.name.eq_ignore_ascii_case(&name)) else {
                    continue;
                };
                for (k, v) in props {
                    // A modify block that sets TableRelation gives the field a relation (record/ table extension: misc/
                    // lt-object-properties-runtime-effect/TableExtModifyTableRelation.al: Validate of the field finds the related table, and
                    // `FieldRef.Relation()` names it).
                    if k.eq_ignore_ascii_case("TableRelation") {
                        // A field that has a relation already keeps it, and the extension's arms come after its arms
                        // (tableextension/TestTableExtFieldTableRelation.al:241-298): the property is listed once more, and
                        // `relation_arms` in lower/tables.rs joins the arms (or refuses a combination that is not pinned).
                        // The first entry stays what a single relation answers (`Field.RelationTableNo` names the base's
                        // table, :262).
                        if field.table_relation
                            && field
                                .properties
                                .iter()
                                .any(|(pk, _)| pk.eq_ignore_ascii_case("TableRelation"))
                        {
                            field.relation_tables.extend(relation_targets(&v));
                            field.properties.push((k, v));
                            continue;
                        }
                        field.table_relation = true;
                        field.relation_table = Some(relation_target(&v));
                        field.relation_tables = relation_targets(&v);
                    }
                    match field.properties.iter_mut().find(|(pk, _)| pk.eq_ignore_ascii_case(&k)) {
                        Some(slot) => slot.1 = v,
                        None => field.properties.push((k, v)),
                    }
                }
            }
        }
    }
    drop_moved_duplicates(&mut target);
    target
}

/// A field with `ObsoleteState = Moved` has moved to another app, which declares the live one (Business Foundation's
/// `Source Code Setup` extension lists `Sales` as Moved, Base Application's declares it): the moved declaration gives way
/// to a live field of the same id or name, so the table has one field and not two.
fn drop_moved_duplicates(m: &mut Members) {
    let moved = |f: &FieldInfo| {
        f.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("ObsoleteState") && v.trim().eq_ignore_ascii_case("Moved"))
    };
    if !m.fields.iter().any(moved) {
        return;
    }
    let live: Vec<(u32, String)> = m
        .fields
        .iter()
        .filter(|f| !moved(f))
        .map(|f| (f.id, f.name.to_ascii_lowercase()))
        .collect();
    m.fields.retain(|f| {
        !moved(f)
            || !live
                .iter()
                .any(|(id, name)| *id == f.id || *name == f.name.to_ascii_lowercase())
    });
}

/// The `modify(Field) { Property = ...; }` blocks of table extension `e`, in source order: the field
/// name and the properties the block sets.
pub(crate) fn modify_properties(ws: &Workspace, e: usize) -> Vec<(String, Vec<(String, String)>)> {
    let mut out = Vec::new();
    for sec in ws
        .object_ast(e)
        .sections()
        .filter(|s| s.name().eq_ignore_ascii_case("fields"))
    {
        for m in sec.sections().filter(|m| m.name().eq_ignore_ascii_case("modify")) {
            let Some(name) = m.args().first().cloned() else {
                continue;
            };
            out.push((name, m.properties().map(|p| (p.name(), p.value_text())).collect()));
        }
    }
    out
}

/// Platform fields every table has (learn.microsoft.com, "System fields").
const SYSTEM_FIELDS: &[(u32, &str, Ty)] = &[
    (2000000000, "SystemId", Ty::Guid),
    (2000000001, "SystemCreatedAt", Ty::DateTime),
    (2000000002, "SystemCreatedBy", Ty::Guid),
    (2000000003, "SystemModifiedAt", Ty::DateTime),
    (2000000004, "SystemModifiedBy", Ty::Guid),
    (2000000005, "SystemRowVersion", Ty::BigInteger),
];

/// The text of the first `'…'` literal in a property value (`'It''s', Locked = true` → `It's`).
pub(crate) fn first_string_literal(value: &str) -> Option<String> {
    let rest = value.trim().strip_prefix('\'')?;
    let mut out = String::new();
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                chars.next();
                out.push('\'');
            } else {
                return Some(out);
            }
        } else {
            out.push(c);
        }
    }
    None
}

/// The fields object `i` (a table or a table extension) declares itself, by id (`--tdd`, `check::tdd_decl`).
pub(crate) fn own_fields(ws: &Workspace, i: usize) -> Vec<(u32, FieldInfo)> {
    own_members(ws, i)
        .fields
        .into_iter()
        .filter(|f| f.declared_in == Some(i))
        .map(|f| (f.id, f))
        .collect()
}

/// The values object `i` (an enum or an enum extension) declares itself (`--tdd`, `check::tdd_decl`).
pub(crate) fn own_enum_values(ws: &Workspace, i: usize) -> Vec<(u32, String)> {
    own_members(ws, i).enum_values
}

fn own_members(ws: &Workspace, i: usize) -> Members {
    let obj = ws.object_ast(i);
    let mut m = Members::default();
    for sec in obj.sections() {
        let name = sec.name().to_ascii_lowercase();
        if name == "fields" {
            for f in sec.sections().filter(|f| f.name().eq_ignore_ascii_case("field")) {
                let args = f.args();
                let (Some(id), Some(fname)) = (args.first().and_then(|a| a.parse().ok()), args.get(1)) else {
                    continue;
                };
                let mut ty = args
                    .get(2)
                    .and_then(|t| al_syntax::parse_type(t))
                    .map(|t| Ty::from_type_ref(&t))
                    .unwrap_or(Ty::Unknown);
                let class = match f
                    .properties()
                    .find(|p| p.name().eq_ignore_ascii_case("FieldClass"))
                    .map(|p| p.value_text().to_ascii_lowercase())
                {
                    Some(c) if c == "flowfield" => FieldClass::FlowField,
                    Some(c) if c == "flowfilter" => FieldClass::FlowFilter,
                    _ => FieldClass::Normal,
                };
                let prop = |n: &str| {
                    f.properties()
                        .find(|p| p.name().eq_ignore_ascii_case(n))
                        .map(|p| p.value_text())
                };
                // A table field of type Option lists its members in a property.
                if let (Ty::Option(_), Some(members)) = (&ty, prop("OptionMembers")) {
                    ty = Ty::Option(split_option_members(&members));
                }
                let init_value = prop("InitValue");
                let auto_increment = prop("AutoIncrement").is_some_and(|v| v.trim().eq_ignore_ascii_case("true"));
                let on_validate = f
                    .triggers()
                    .find(|t| t.name().eq_ignore_ascii_case("OnValidate"))
                    .map(|t| t.syntax().text_range().start().into());
                let on_lookup = f
                    .triggers()
                    .find(|t| t.name().eq_ignore_ascii_case("OnLookup"))
                    .map(|t| t.syntax().text_range().start().into());
                m.fields.push(FieldInfo {
                    id,
                    name: fname.clone(),
                    ty,
                    class,
                    declared_in: Some(i),
                    init_value,
                    auto_increment,
                    calc_formula: prop("CalcFormula"),
                    table_relation: prop("TableRelation").is_some(),
                    relation_table: prop("TableRelation").map(|v| relation_target(&v)),
                    relation_tables: prop("TableRelation").map(|v| relation_targets(&v)).unwrap_or_default(),
                    on_validate,
                    on_lookup,
                    properties: f.properties().map(|p| (p.name(), p.value_text())).collect(),
                });
            }
        } else if name == "value" {
            let args = sec.args();
            if let (Some(id), Some(v)) = (args.first().and_then(|a| a.parse().ok()), args.get(1)) {
                m.enum_values.push((id, v.clone()));
                for p in sec
                    .properties()
                    .filter(|p| p.name().eq_ignore_ascii_case("Implementation"))
                {
                    push_impls(&mut m, ImplSlot::Value(id), &p.value_text(), i);
                }
                let caption = sec
                    .properties()
                    .find(|p| p.name().eq_ignore_ascii_case("Caption"))
                    .and_then(|p| first_string_literal(&p.value_text()));
                if let Some(c) = caption {
                    m.enum_captions.push((id, c));
                }
            }
        }
    }
    for p in obj.properties() {
        let slot = match p.name().to_ascii_lowercase().as_str() {
            "defaultimplementation" => ImplSlot::Default,
            "unknownvalueimplementation" => ImplSlot::Unknown,
            "assignmentcompatibility" => {
                m.assignment_compatible = p.value_text().trim().eq_ignore_ascii_case("true");
                continue;
            }
            _ => continue,
        };
        push_impls(&mut m, slot, &p.value_text(), i);
    }
    if ws.objects[i].kind == ObjKind::Table {
        for (id, name, ty) in SYSTEM_FIELDS {
            m.fields.push(FieldInfo {
                id: *id,
                name: name.to_string(),
                ty: ty.clone(),
                class: FieldClass::Normal,
                declared_in: None,
                init_value: None,
                auto_increment: false,
                calc_formula: None,
                table_relation: false,
                relation_table: None,
                relation_tables: Vec::new(),
                on_validate: None,
                on_lookup: None,
                // SystemId's caption is pinned by record/TestRecordSystemId.al:178, the others by
                // record/TestPlatformMetadataDeclarationContracts.al:233-250.
                properties: match *id {
                    2000000000 => vec![("Caption".into(), "'System ID'".into())],
                    2000000001 => vec![("Caption".into(), "'Created At'".into())],
                    2000000002 => vec![("Caption".into(), "'Created By'".into())],
                    2000000003 => vec![("Caption".into(), "'Modified At'".into())],
                    2000000004 => vec![("Caption".into(), "'Modified By'".into())],
                    _ => Vec::new(),
                },
            });
        }
    }
    for r in obj.routines() {
        m.procedures.push(ProcSig {
            name: r.name(),
            params: r
                .params()
                .map(|p| ProcParam {
                    name: p.name(),
                    ty: p.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown),
                    by_var: p.is_var(),
                })
                .collect(),
            ret: r.return_type().map(|t| Ty::from_type_ref(&t)),
            local: r.is_local(),
            declared_in: i,
            offset: r.syntax().text_range().start().into(),
        });
    }
    m
}

/// The `Interface = Codeunit` pairs of an implementation property (`I = C, J = D`).
fn push_impls(m: &mut Members, slot: ImplSlot, value: &str, declared_in: usize) {
    for pair in split_outside_quotes(value, ',') {
        let sides = split_outside_quotes(&pair, '=');
        if let [interface, codeunit] = sides.as_slice() {
            m.enum_impls.push(EnumImpl {
                slot: slot.clone(),
                interface: name_parts(interface),
                codeunit: name_parts(codeunit),
                declared_in,
            });
        }
    }
}

fn split_outside_quotes(text: &str, sep: char) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut in_quote = false;
    for c in text.chars() {
        match c {
            '"' => {
                in_quote = !in_quote;
                out.last_mut().expect("non-empty").push(c);
            }
            c if c == sep && !in_quote => out.push(String::new()),
            c => out.last_mut().expect("non-empty").push(c),
        }
    }
    out
}

/// `Ns."Some Name"` as the parts of an object reference.
fn name_parts(text: &str) -> Vec<String> {
    split_outside_quotes(text.trim(), '.')
        .into_iter()
        .map(|p| p.trim().trim_matches('"').to_string())
        .collect()
}

/// `Open,"In Progress",,Closed` as member names: quotes removed and kept spaces inside them,
/// blank members kept, unquoted names trimmed.
fn split_option_members(text: &str) -> Vec<String> {
    let mut out: Vec<(String, bool)> = vec![(String::new(), false)];
    let mut in_quote = false;
    for c in text.chars() {
        match c {
            '"' => {
                in_quote = !in_quote;
                out.last_mut().expect("non-empty").1 = true;
            }
            ',' if !in_quote => out.push((String::new(), false)),
            c => out.last_mut().expect("non-empty").0.push(c),
        }
    }
    out.into_iter()
        .map(|(name, quoted)| match (quoted, name.is_empty()) {
            // `""` is one space, an unquoted empty member stays empty (see `TypeRef::option_members`).
            (true, true) => " ".to_string(),
            (true, false) => name,
            _ => name.trim().to_string(),
        })
        .collect()
}

/// Every table a `TableRelation` names: `if (..) A.F else if (..) B.F else C.F` gives A, B, C.
/// The `where(..)` filter of an arm names no table.
fn relation_targets(v: &str) -> Vec<String> {
    let c: Vec<char> = v.chars().collect();
    let mut out = Vec::new();
    let (mut i, mut expect_table) = (0, true);
    let skip_parens = |i: &mut usize| {
        // At a '(': to just past its matching ')', ignoring quoted text.
        let mut depth = 0;
        while *i < c.len() {
            match c[*i] {
                '"' | '\'' => {
                    let q = c[*i];
                    *i += 1;
                    while *i < c.len() && c[*i] != q {
                        *i += 1;
                    }
                }
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        *i += 1;
                        return;
                    }
                }
                _ => {}
            }
            *i += 1;
        }
    };
    while i < c.len() {
        if c[i].is_whitespace() || c[i] == '.' {
            i += 1;
            continue;
        }
        if c[i] == '(' {
            skip_parens(&mut i);
            continue;
        }
        let word = if c[i] == '"' {
            i += 1;
            let start = i;
            while i < c.len() && c[i] != '"' {
                i += 1;
            }
            let w: String = c[start..i].iter().collect();
            i += 1;
            (w, true)
        } else {
            let start = i;
            while i < c.len() && (c[i].is_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            if start == i {
                i += 1;
                continue;
            }
            (c[start..i].iter().collect::<String>(), false)
        };
        match (word.1, word.0.to_ascii_lowercase().as_str()) {
            (false, "if" | "else" | "where") => expect_table = !word.0.eq_ignore_ascii_case("where"),
            _ if expect_table => {
                out.push(word.0.to_lowercase());
                expect_table = false;
            }
            _ => {}
        }
    }
    out
}

/// The table a `TableRelation` value names first: `"Cust".No.` → `cust`.
fn relation_target(v: &str) -> String {
    let v = v.trim();
    let name = match v.strip_prefix('"') {
        Some(rest) => rest.split('"').next().unwrap_or(""),
        None => v.split(['.', ' ', '(']).next().unwrap_or(""),
    };
    name.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{AppSource, Resolution};

    fn ws(files: &[(&str, &str)]) -> Workspace {
        Workspace::load(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
            }],
            &Default::default(),
        )
    }

    fn members_of(ws: &Workspace, kind: ObjKind, name: &str) -> Members {
        let m = build(ws);
        let Resolution::Found(i) = ws.resolve_ref(0, kind, &[name.to_string()]) else {
            panic!("{name} not found")
        };
        m[i].clone()
    }

    const TABLES: &str = r#"
table 50900 "ALT Universal"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { }
        field(3; "Total"; Decimal) { FieldClass = FlowField; CalcFormula = sum("ALT Universal".Amount); }
        field(4; "Filter"; Date) { FieldClass = FlowFilter; }
    }
    procedure Double(X: Integer): Integer
    begin
        exit(X * 2);
    end;
    local procedure Hidden(var R: Record "ALT Universal")
    begin
    end;
}
tableextension 50901 "ALT Ext" extends "ALT Universal"
{
    fields { field(50100; "Ext Field"; Text[50]) { } }
    procedure FromExt(): Boolean
    begin
    end;
}
"#;

    #[test]
    fn table_fields_with_types_and_classes() {
        let w = ws(&[("t.al", TABLES)]);
        let m = members_of(&w, ObjKind::Table, "ALT Universal");
        let f = m.field("no.").unwrap();
        assert_eq!(
            (f.id, f.ty.clone(), f.class.clone()),
            (1, Ty::Code(Some(20)), FieldClass::Normal)
        );
        assert_eq!(m.field("Total").unwrap().class, FieldClass::FlowField);
        assert_eq!(m.field("Filter").unwrap().class, FieldClass::FlowFilter);
    }

    #[test]
    fn option_fields_take_their_members_from_the_option_members_property() {
        let w = ws(&[(
            "t.al",
            "table 50902 T { fields { field(1; K; Code[10]) { } field(2; St; Option) { OptionMembers = Open,\"In Progress\",Closed; } field(3; Bl; Option) { OptionMembers = ,A,B; } field(4; Sp; Option) { OptionMembers = \" \",Draft; } } }",
        )]);
        let m = members_of(&w, ObjKind::Table, "T");
        assert_eq!(
            m.field("St").unwrap().ty,
            Ty::Option(vec!["Open".into(), "In Progress".into(), "Closed".into()])
        );
        assert_eq!(
            m.field("Bl").unwrap().ty,
            Ty::Option(vec!["".into(), "A".into(), "B".into()])
        );
        // A quoted name keeps its spaces: a member named with one space is not a blank one.
        assert_eq!(m.field("Sp").unwrap().ty, Ty::Option(vec![" ".into(), "Draft".into()]));
    }

    // corpus _fixtures/tables/ALTRelationParentChild.al:49-96: every arm of a conditional
    // relation names a table whose rename the relating field follows.
    #[test]
    fn relation_tables_lists_every_table_of_a_conditional_relation() {
        let w = ws(&[(
            "t.al",
            "table 50903 T { fields { field(1; K; Code[10]) { }
                field(2; A; Code[10]) { TableRelation = \"Parent A\".\"Code\"; }
                field(3; B; Code[10]) { TableRelation = if (K = const(A)) \"Parent A\".\"Code\" else \"Parent B\".\"Code\"; }
                field(4; C; Code[10]) { TableRelation = if (K = const(A)) \"Parent A\".\"Code\" else if (K = const(B)) \"Parent B\".Code else \"Parent C\".\"Code\"; }
                field(5; D; Code[10]) { TableRelation = \"Parent A\".\"Code\" where(Blocked = const(false)); }
                field(6; E; Code[10]) { TableRelation = if (K = const(A)) Other.Code; }
                field(7; F; Code[10]) { } } }",
        )]);
        let m = members_of(&w, ObjKind::Table, "T");
        let rel = |n: &str| m.field(n).unwrap().relation_tables.clone();
        assert_eq!(rel("A"), ["parent a"]);
        assert_eq!(rel("B"), ["parent a", "parent b"]);
        assert_eq!(rel("C"), ["parent a", "parent b", "parent c"]);
        assert_eq!(rel("D"), ["parent a"]);
        assert_eq!(rel("E"), ["other"]);
        assert!(rel("F").is_empty());
    }

    #[test]
    fn calc_formula_text_is_kept_for_flow_fields() {
        let w = ws(&[("t.al", TABLES)]);
        let m = members_of(&w, ObjKind::Table, "ALT Universal");
        assert_eq!(
            m.field("Total").unwrap().calc_formula.as_deref(),
            Some("sum(\"ALT Universal\".Amount)")
        );
        assert_eq!(m.field("Amount").unwrap().calc_formula, None);
    }

    #[test]
    fn table_extension_fields_and_procedures_are_merged_into_the_table() {
        let w = ws(&[("t.al", TABLES)]);
        let m = members_of(&w, ObjKind::Table, "ALT Universal");
        assert_eq!(m.field("Ext Field").unwrap().ty, Ty::Text(Some(50)));
        assert_eq!(m.procedures_named("FromExt").len(), 1);
    }

    #[test]
    fn a_tableextension_modify_block_overrides_field_properties() {
        // corpus tableextension/TestTableExtFieldModify.al:40-67
        let w = ws(&[(
            "t.al",
            "table 50920 T { fields { field(1; A; Integer) { Caption = 'Orig A'; } field(2; B; Integer) { Caption = 'Orig B'; } } keys { key(PK; A) { } } }
             tableextension 50921 X extends T { fields { modify(A) { Caption = 'New A'; } } }",
        )]);
        let m = members_of(&w, ObjKind::Table, "T");
        let caption = |n: &str| {
            m.field(n)
                .unwrap()
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
                .map(|(_, v)| v.clone())
        };
        assert_eq!(caption("A").as_deref(), Some("'New A'"));
        assert_eq!(
            caption("B").as_deref(),
            Some("'Orig B'"),
            "an unmodified field keeps its caption"
        );
    }

    #[test]
    fn system_fields_are_present() {
        let w = ws(&[("t.al", TABLES)]);
        let m = members_of(&w, ObjKind::Table, "ALT Universal");
        assert_eq!(m.field("SystemId").unwrap().ty, Ty::Guid);
        assert_eq!(m.field("SystemCreatedAt").unwrap().ty, Ty::DateTime);
        assert_eq!(m.field("SystemModifiedBy").unwrap().ty, Ty::Guid);
        assert_eq!(m.field("SystemRowVersion").unwrap().ty, Ty::BigInteger);
    }

    #[test]
    fn procedure_signatures() {
        let w = ws(&[("t.al", TABLES)]);
        let m = members_of(&w, ObjKind::Table, "ALT Universal");
        let d = m.procedures_named("double")[0];
        assert_eq!(
            d.params,
            vec![ProcParam {
                name: "X".into(),
                ty: Ty::Integer,
                by_var: false
            }]
        );
        assert_eq!(d.ret, Some(Ty::Integer));
        assert!(!d.local);
        let h = m.procedures_named("Hidden")[0];
        assert!(h.local && h.params[0].by_var && h.ret.is_none());
    }

    #[test]
    fn enum_values_with_extension_values() {
        let w = ws(&[(
            "e.al",
            "enum 50900 \"ALT Status\" { Extensible = true; value(0; Open) { } value(1; \"In Progress\") { } }\nenumextension 50901 X extends \"ALT Status\" { value(50000; Extra) { } }",
        )]);
        let m = members_of(&w, ObjKind::Enum, "ALT Status");
        assert_eq!(
            m.enum_values,
            vec![(0, "Open".into()), (1, "In Progress".into()), (50000, "Extra".into())]
        );
    }

    #[test]
    fn codeunit_procedures() {
        let w = ws(&[(
            "c.al",
            "codeunit 1 C { procedure A() begin end; procedure A(X: Text) begin end; internal procedure B(): Text begin end; }",
        )]);
        let m = members_of(&w, ObjKind::Codeunit, "C");
        assert_eq!(m.procedures_named("A").len(), 2, "overloads");
        assert_eq!(m.procedures_named("B")[0].ret, Some(Ty::Text(None)));
    }
}
