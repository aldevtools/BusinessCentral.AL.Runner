//! `--tdd` stub declarations (ADR-051 decision 10): a field, an enum value or an object that code of a test app names
//! and that does not exist is declared in an extra source file of that app ([`TDD_FILE`]), and the apps are compiled
//! again. A field goes into a table extension of its table, a value into an enum extension of its enum (also of a
//! dependency's), and a missing codeunit, table or enum becomes an object with the first free id of the app's
//! `idRanges`. The next compile reads the declarations back ([`decls`]): lowering notes a use of one before the
//! statement that holds it (`ir::Stmt::TddMark`), so a test that touches one is reported as stubbed.
//!
//! This is tooling, not AL semantics: no corpus case pins it, and without `--tdd` nothing here runs.
use super::tdd::{TddKind, TddRet};
use super::*;

/// The path of the stub file in the app it is added to. No file of a real app has it: it starts with a character a
/// path the runner reads from disk never starts with.
pub const TDD_FILE: &str = "<tdd>/stubs.al";

/// How many times the apps are compiled again for stub declarations at most: a table stub can need fields, which
/// are found only once the table exists.
pub const MAX_RECOMPILES: usize = 3;

/// A declaration the next compile needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TddNeed {
    /// Field `name` of table `table` (the object's name), declared with AL type `ty` and shown with type `shown`.
    Field {
        app: String,
        table: String,
        name: String,
        ty: String,
        shown: String,
    },
    /// Value `name` of enum `enum_name`.
    EnumValue {
        app: String,
        enum_name: String,
        name: String,
    },
    /// An object of kind codeunit, table or enum.
    Object { app: String, kind: ObjKind, name: String },
}

impl TddNeed {
    fn app(&self) -> &str {
        match self {
            TddNeed::Field { app, .. } | TddNeed::EnumValue { app, .. } | TddNeed::Object { app, .. } => app,
        }
    }
}

/// The AL type a stub field of type `t` is declared with: the fixed-width types, an enum, and Text and Code with the
/// largest length (the real one is not known yet). `None`: a field cannot have the type.
pub fn field_type(t: &Ty) -> Option<String> {
    Some(match t {
        Ty::Integer => "Integer".into(),
        Ty::BigInteger => "BigInteger".into(),
        Ty::Decimal => "Decimal".into(),
        Ty::Boolean => "Boolean".into(),
        Ty::Date => "Date".into(),
        Ty::Time => "Time".into(),
        Ty::DateTime => "DateTime".into(),
        Ty::Duration => "Duration".into(),
        Ty::Guid => "Guid".into(),
        Ty::Text(_) => format!("Text[{STUB_TEXT_LENGTH}]"),
        Ty::Code(_) => format!("Code[{STUB_TEXT_LENGTH}]"),
        Ty::Enum(e) => format!("Enum {}", quoted(e)),
        _ => return None,
    })
}

/// The length a stub Text or Code field is declared with (decision 10).
pub const STUB_TEXT_LENGTH: u32 = 2048;

/// The reason a declaration site is refused in a compile that does not declare it.
pub fn site_reason(site: &super::tdd::TddSite) -> String {
    match &site.ret {
        TddRet::Refused(why) => why.clone(),
        _ => format!("--tdd: no stub declaration could be generated for {}", site.shown()),
    }
}

fn quoted(name: &str) -> String {
    format!("\"{name}\"")
}

/// Whether object `oi` is a stub declaration (it lives in a [`TDD_FILE`]).
pub fn is_stub(ws: &Workspace, oi: usize) -> bool {
    ws.files[ws.objects[oi].file].path == TDD_FILE
}

/// The declarations a checked program needs, in source order, each once: the field and enum value sites of the
/// checker, and the objects the code of the test apps names that no app declares.
pub fn needs(ws: &Workspace, checked: &CheckResult) -> Vec<TddNeed> {
    let mut out: Vec<TddNeed> = Vec::new();
    let mut push = |n: TddNeed| {
        if !out.contains(&n) {
            out.push(n);
        }
    };
    let mut sites: Vec<_> = checked.resolutions.iter().collect();
    sites.sort_by_key(|(k, _)| **k);
    for (&(file, _, _), r) in sites {
        let Res::TddStub(site) = r.inner() else { continue };
        let app = ws.apps[ws.files[file].app].id.clone();
        match (site.kind, &site.ret) {
            (TddKind::Field, TddRet::Value(t)) => {
                let Some(ty) = field_type(t) else { continue };
                push(TddNeed::Field {
                    app,
                    table: ws.objects[site.object].name.clone(),
                    name: site.name.clone(),
                    ty,
                    shown: match t {
                        Ty::Text(_) => "Text".into(),
                        Ty::Code(_) => "Code".into(),
                        t => t.to_string(),
                    },
                });
            }
            (TddKind::EnumValue, _) => push(TddNeed::EnumValue {
                app,
                enum_name: ws.objects[site.object].name.clone(),
                name: site.name.clone(),
            }),
            _ => {}
        }
    }
    let apps = super::tdd::test_apps(ws);
    for oi in 0..ws.objects.len() {
        let o = &ws.objects[oi];
        if !apps.get(o.app).copied().unwrap_or(false) || ws.is_dependency(oi) || is_stub(ws, oi) {
            continue;
        }
        let app = ws.apps[o.app].id.clone();
        let node = ws.object_ast(oi).syntax().clone();
        let mut named: Vec<(ObjKind, String)> = Vec::new();
        for t in node.descendants().filter_map(ast::TypeRef::cast) {
            if let Some(kind) = ObjKind::from_type_name(&t.type_name()) {
                if let [name] = t.object_name_parts().as_slice() {
                    named.push((kind, name.clone()));
                }
            }
        }
        // `Codeunit::"X"`, `Database::"X"`, `Enum::"X"`.
        for s in node.descendants().filter(|n| n.kind() == SyntaxKind::ScopeExpr) {
            let kids: Vec<SyntaxNode> = s.children().collect();
            let [first, second, ..] = kids.as_slice() else { continue };
            if first.kind() != SyntaxKind::NameRef || second.kind() != SyntaxKind::NameRef {
                continue;
            }
            let kind = match name_of(first).to_ascii_lowercase().as_str() {
                "codeunit" => ObjKind::Codeunit,
                "database" => ObjKind::Table,
                "enum" => ObjKind::Enum,
                _ => continue,
            };
            named.push((kind, name_of(second)));
        }
        for (kind, name) in named {
            if !matches!(kind, ObjKind::Codeunit | ObjKind::Table | ObjKind::Enum) {
                continue;
            }
            // Refused when an object of the name exists anywhere, also out of reach (another namespace, a missing
            // dependency): the compile error stays.
            if !matches!(
                ws.resolve_ref(o.file, kind, std::slice::from_ref(&name)),
                Resolution::Missing
            ) || (0..ws.objects.len())
                .any(|i| ws.objects[i].kind == kind && ws.objects[i].name.eq_ignore_ascii_case(&name))
            {
                continue;
            }
            push(TddNeed::Object {
                app: app.clone(),
                kind,
                name,
            });
        }
    }
    out
}

/// The text of the stub file of each app that needs one, by app id: the declarations `needs` asks for. A
/// declaration without a free id is left out (its site stays refused). `ranges` gives each app's `idRanges` by app
/// id; an app without any gets no stub objects or extensions.
pub fn stub_files(
    ws: &Workspace,
    needs: &[TddNeed],
    ranges: &std::collections::BTreeMap<String, Vec<(u32, u32)>>,
) -> Vec<(String, String)> {
    let mut apps: Vec<&str> = Vec::new();
    for n in needs {
        if !apps.contains(&n.app()) {
            apps.push(n.app());
        }
    }
    let mut out = Vec::new();
    for app in apps {
        let ranges = ranges.get(app).map(Vec::as_slice).unwrap_or(&[]);
        let mut taken: HashSet<(ObjKind, u32)> = (0..ws.objects.len())
            .filter(|&i| !is_stub(ws, i))
            .filter_map(|i| ws.objects[i].id.map(|id| (ws.objects[i].kind, id)))
            .collect();
        let mut free = |kind: ObjKind| -> Option<u32> {
            let id = ranges
                .iter()
                .flat_map(|&(a, b)| a..=b)
                .find(|id| !taken.contains(&(kind, *id)))?;
            taken.insert((kind, id));
            Some(id)
        };
        let mine: Vec<&TddNeed> = needs.iter().filter(|n| n.app() == app).collect();
        let mut text = String::new();
        let mut usings: Vec<String> = Vec::new();
        // Objects first: the fields and values of a stub object go into the object.
        let mut objects: Vec<(ObjKind, String, u32)> = Vec::new();
        for n in &mine {
            if let TddNeed::Object { kind, name, .. } = n {
                if let Some(id) = free(*kind) {
                    objects.push((*kind, name.clone(), id));
                }
            }
        }
        let stub_object = |kind: ObjKind, name: &str| {
            objects
                .iter()
                .any(|(k, n, _)| *k == kind && n.eq_ignore_ascii_case(name))
                || (0..ws.objects.len()).any(|i| {
                    is_stub(ws, i) && ws.objects[i].kind == kind && ws.objects[i].name.eq_ignore_ascii_case(name)
                })
        };
        for (kind, name, id) in &objects {
            match kind {
                ObjKind::Codeunit => text.push_str(&format!("codeunit {id} {}\n{{\n}}\n", quoted(name))),
                ObjKind::Table => {
                    text.push_str(&format!(
                        "table {id} {}\n{{\n    fields\n    {{\n        field(1; \"TDD Key\"; Integer) {{ AutoIncrement = true; }}\n",
                        quoted(name)
                    ));
                    let mut fid = 2;
                    for n in &mine {
                        if let TddNeed::Field { table, name: f, ty, .. } = n {
                            if table.eq_ignore_ascii_case(name) {
                                text.push_str(&format!("        field({fid}; {}; {ty}) {{ }}\n", quoted(f)));
                                fid += 1;
                            }
                        }
                    }
                    text.push_str(
                        "    }\n    keys\n    {\n        key(PK; \"TDD Key\") { Clustered = true; }\n    }\n}\n",
                    );
                }
                ObjKind::Enum => {
                    text.push_str(&format!("enum {id} {}\n{{\n    Extensible = true;\n", quoted(name)));
                    let mut ord = 0;
                    for n in &mine {
                        if let TddNeed::EnumValue { enum_name, name: v, .. } = n {
                            if enum_name.eq_ignore_ascii_case(name) {
                                text.push_str(&format!("    value({ord}; {}) {{ }}\n", quoted(v)));
                                ord += 1;
                            }
                        }
                    }
                    text.push_str("}\n");
                }
                _ => {}
            }
        }
        // Fields of existing tables, by table, in a table extension each.
        let mut tables: Vec<&str> = Vec::new();
        for n in &mine {
            if let TddNeed::Field { table, .. } = n {
                if !stub_object(ObjKind::Table, table) && !tables.iter().any(|t| t.eq_ignore_ascii_case(table)) {
                    tables.push(table);
                }
            }
        }
        for table in tables {
            let Some(ti) = existing(ws, ObjKind::Table, table) else {
                continue;
            };
            let mut used: HashSet<u32> = HashSet::default();
            for oi in std::iter::once(ti).chain(ws.extensions_of(ti).iter().copied()) {
                if !is_stub(ws, oi) {
                    used.extend(crate::members::own_fields(ws, oi).into_iter().map(|(id, _)| id));
                }
            }
            let Some(id) = free(ObjKind::TableExtension) else {
                continue;
            };
            let mut fields = String::new();
            let mut field_ids = ranges.iter().flat_map(|&(a, b)| a..=b).filter(|i| !used.contains(i));
            for n in &mine {
                if let TddNeed::Field {
                    table: t, name: f, ty, ..
                } = n
                {
                    if t.eq_ignore_ascii_case(table) {
                        let Some(fid) = field_ids.next() else { break };
                        fields.push_str(&format!("        field({fid}; {}; {ty}) {{ }}\n", quoted(f)));
                    }
                }
            }
            if let Some(ns) = namespace_of(ws, ti) {
                if !usings.contains(&ns) {
                    usings.push(ns);
                }
            }
            text.push_str(&format!(
                "tableextension {id} \"TDD Stub {id}\" extends {}\n{{\n    fields\n    {{\n{fields}    }}\n}}\n",
                quoted(&ws.objects[ti].name)
            ));
        }
        // Values of existing enums, by enum, in an enum extension each.
        let mut enums: Vec<&str> = Vec::new();
        for n in &mine {
            if let TddNeed::EnumValue { enum_name, .. } = n {
                if !stub_object(ObjKind::Enum, enum_name) && !enums.iter().any(|e| e.eq_ignore_ascii_case(enum_name)) {
                    enums.push(enum_name);
                }
            }
        }
        for e in enums {
            let Some(ei) = existing(ws, ObjKind::Enum, e) else {
                continue;
            };
            let mut top: Option<u32> = None;
            for oi in std::iter::once(ei).chain(ws.extensions_of(ei).iter().copied()) {
                if !is_stub(ws, oi) {
                    for (ord, _) in crate::members::own_enum_values(ws, oi) {
                        top = Some(top.map_or(ord, |t| t.max(ord)));
                    }
                }
            }
            let Some(id) = free(ObjKind::EnumExtension) else {
                continue;
            };
            let mut ord = top.map_or(0, |t| t + 1);
            let mut values = String::new();
            for n in &mine {
                if let TddNeed::EnumValue { enum_name, name: v, .. } = n {
                    if enum_name.eq_ignore_ascii_case(e) {
                        values.push_str(&format!("    value({ord}; {}) {{ }}\n", quoted(v)));
                        ord += 1;
                    }
                }
            }
            if let Some(ns) = namespace_of(ws, ei) {
                if !usings.contains(&ns) {
                    usings.push(ns);
                }
            }
            text.push_str(&format!(
                "enumextension {id} \"TDD Stub {id}\" extends {}\n{{\n{values}}}\n",
                quoted(&ws.objects[ei].name)
            ));
        }
        if text.is_empty() {
            continue;
        }
        let head: String = usings.iter().map(|ns| format!("using {ns};\n")).collect();
        out.push((app.to_string(), format!("{head}{text}")));
    }
    out
}

/// Puts the stub files `files` (app id, text) into `apps`: the stub file of an app is replaced, or added.
pub fn set_stub_files(apps: &mut [crate::workspace::AppSource], files: Vec<(String, String)>) {
    for (id, text) in files {
        let Some(app) = apps.iter_mut().find(|a| a.id == id) else {
            continue;
        };
        match app.files.iter_mut().find(|(p, _)| p == TDD_FILE) {
            Some(f) => f.1 = text,
            None => app.files.push((TDD_FILE.to_string(), text)),
        }
    }
}

/// The declarations among `needs` that `seen` does not hold yet, added to it. Empty: another compile would add
/// nothing (`MAX_RECOMPILES` bounds the rounds).
pub fn take_new(seen: &mut Vec<TddNeed>, needs: &[TddNeed]) -> bool {
    let mut added = false;
    for n in needs {
        if !seen.contains(n) {
            seen.push(n.clone());
            added = true;
        }
    }
    added
}

/// The object of `kind` named `name` that is not a stub, if one exists.
fn existing(ws: &Workspace, kind: ObjKind, name: &str) -> Option<usize> {
    (0..ws.objects.len())
        .find(|&i| ws.objects[i].kind == kind && ws.objects[i].name.eq_ignore_ascii_case(name) && !is_stub(ws, i))
}

fn namespace_of(ws: &Workspace, oi: usize) -> Option<String> {
    ws.files[ws.objects[oi].file].namespace.clone()
}

/// The stub declarations of a compile, read back from its stub files: what lowering marks (`ir::Stmt::TddMark`) and
/// what the report lists.
#[derive(Debug, Default)]
pub struct TddDecls {
    /// (table, field id) -> signature.
    pub fields: HashMap<(usize, u32), String>,
    /// (enum, value) -> signature.
    pub values: HashMap<(usize, i32), String>,
    /// stub object -> signature.
    pub objects: HashMap<usize, String>,
    /// Every signature, sorted, each once.
    pub all: Vec<String>,
}

impl TddDecls {
    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }
}

/// The declarations of the stub files of `ws` (empty without any).
pub fn decls(ws: &Workspace) -> TddDecls {
    let mut d = TddDecls::default();
    if !ws.files.iter().any(|f| f.path == TDD_FILE) {
        return d;
    }
    for oi in (0..ws.objects.len()).filter(|&i| is_stub(ws, i)) {
        let o = &ws.objects[oi];
        let shown = crate::types::display_name(&o.name);
        match o.kind {
            ObjKind::Codeunit | ObjKind::Table | ObjKind::Enum => {
                let kind = match o.kind {
                    ObjKind::Codeunit => "codeunit",
                    ObjKind::Table => "table",
                    _ => "enum",
                };
                d.objects
                    .insert(oi, format!("{kind} \"{shown}\" (id {})", o.id.unwrap_or(0)));
            }
            _ => {}
        }
        let base = match o.kind {
            ObjKind::TableExtension | ObjKind::EnumExtension => match ws.extension_target(oi) {
                Some(b) => b,
                None => continue,
            },
            _ => oi,
        };
        let base_shown = crate::types::display_name(&ws.objects[base].name);
        if matches!(o.kind, ObjKind::Table | ObjKind::TableExtension) {
            for (id, f) in crate::members::own_fields(ws, oi) {
                if o.kind == ObjKind::Table && f.name.eq_ignore_ascii_case("TDD Key") {
                    continue;
                }
                let ty = match &f.ty {
                    Ty::Text(_) => "Text".to_string(),
                    Ty::Code(_) => "Code".to_string(),
                    t => t.to_string(),
                };
                d.fields
                    .insert((base, id), format!("{base_shown}: field \"{}\": {ty}", f.name));
            }
        }
        if matches!(o.kind, ObjKind::Enum | ObjKind::EnumExtension) {
            for (ord, v) in crate::members::own_enum_values(ws, oi) {
                d.values
                    .insert((base, ord as i32), format!("{base_shown}: value \"{v}\""));
            }
        }
    }
    let mut all: Vec<String> = d
        .fields
        .values()
        .chain(d.values.values())
        .chain(d.objects.values())
        .cloned()
        .collect();
    all.sort();
    all.dedup();
    d.all = all;
    d
}

/// Whether a field `f` of a stub declaration is longer than a write may make it (decision 10): Text and Code
/// stub fields take [`STUB_TEXT_LENGTH`] characters.
pub fn stub_field_reason(name: &str) -> String {
    format!("--tdd: the length of the stub field {name} is unknown")
}

#[cfg(test)]
mod tests {
    use crate::workspace::{AppKind, AppSource};

    const OBJECTS: &str = r#"
table 50100 "Item"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(50100; "Taken"; Integer) { }
    }
}
enum 50101 "Color"
{
    Extensible = true;
    value(0; Red) { }
    value(5; Green) { }
}
"#;

    /// Compiles `OBJECTS` and a test codeunit with `body` as its test's statements and `vars` as its locals, with the
    /// app's ids from 50100 to 50119.
    fn compile(vars: &str, body: &str, tdd: bool) -> (crate::Workspace, crate::lower::Lowered) {
        let test = format!(
            "codeunit 50110 \"T\"\n{{\n    Subtype = Test;\n    [Test]\n    procedure P()\n    var\n        Item: Record \"Item\";\n        I: Integer;\n{vars}\n    begin\n{body}\n    end;\n}}\n"
        );
        crate::lower::compile_apps_with(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("o.al".into(), OBJECTS.into()), ("test.al".into(), test)],
            }],
            vec![AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                tdd,
                tdd_id_ranges: [("a".to_string(), vec![(50100, 50119)])].into_iter().collect(),
                ..Default::default()
            },
        )
    }

    fn reasons(ws: &crate::Workspace, l: &crate::lower::Lowered) -> Vec<String> {
        let oi = ws
            .objects
            .iter()
            .position(|o| o.id == Some(50110) && o.kind == super::ObjKind::Codeunit);
        l.unsupported[oi.expect("test codeunit")]
            .iter()
            .flatten()
            .cloned()
            .collect()
    }

    fn stub_text(ws: &crate::Workspace) -> String {
        ws.apps[0]
            .files
            .iter()
            .find(|(p, _)| p == super::TDD_FILE)
            .map(|(_, t)| t.clone())
            .unwrap_or_default()
    }

    #[test]
    fn a_missing_field_takes_its_type_from_the_use_and_is_declared_in_a_table_extension() {
        let (ws, l) = compile("T: Text;", "Item.Weight := 7; T := Item.Note; if Item.Flag then;", true);
        assert!(reasons(&ws, &l).is_empty(), "{:?}", reasons(&ws, &l));
        for want in [
            r#"Item: field "Weight": Integer"#,
            r#"Item: field "Note": Text"#,
            r#"Item: field "Flag": Boolean"#,
        ] {
            assert!(
                l.program.tdd_stubs.contains(&want.to_string()),
                "{want}: {:?}",
                l.program.tdd_stubs
            );
        }
        let text = stub_text(&ws);
        // The first free id of the range for each kind and each field; Text gets the largest length.
        assert!(
            text.contains("tableextension 50100 \"TDD Stub 50100\" extends \"Item\""),
            "{text}"
        );
        assert!(text.contains("field(50101; \"Weight\"; Integer)"), "{text}");
        assert!(text.contains("field(50102; \"Note\"; Text[2048])"), "{text}");
    }

    #[test]
    fn a_missing_enum_value_gets_the_first_ordinal_above_the_highest() {
        let (ws, l) = compile("C: Enum \"Color\";", "C := Enum::\"Color\"::Blue;", true);
        assert!(reasons(&ws, &l).is_empty(), "{:?}", reasons(&ws, &l));
        assert!(l.program.tdd_stubs.contains(&r#"Color: value "Blue""#.to_string()));
        assert!(stub_text(&ws).contains("value(6; \"Blue\")"), "{}", stub_text(&ws));
    }

    #[test]
    fn missing_objects_get_the_first_free_id_and_their_members() {
        let vars = "N: Codeunit \"New Calc\"; E: Record \"New Entry\"; K: Enum \"New Kind\";";
        let body = "I := N.Compute(); E.Amount := 3; K := Enum::\"New Kind\"::First;";
        let (ws, l) = compile(vars, body, true);
        assert!(reasons(&ws, &l).is_empty(), "{:?}", reasons(&ws, &l));
        for want in [
            r#"codeunit "New Calc" (id 50100)"#,
            r#"table "New Entry" (id 50101)"#,
            r#"enum "New Kind" (id 50100)"#,
            r#"New Entry: field "Amount": Integer"#,
            r#"New Kind: value "First""#,
            r#"New Calc: procedure "Compute"(): Integer"#,
        ] {
            assert!(
                l.program.tdd_stubs.contains(&want.to_string()),
                "{want}: {:?}",
                l.program.tdd_stubs
            );
        }
        assert!(stub_text(&ws).contains("field(1; \"TDD Key\"; Integer) { AutoIncrement = true; }"));
    }

    #[test]
    fn a_field_whose_type_no_use_gives_is_refused_with_the_reason() {
        let (ws, l) = compile("", "Item.Weight := Item.Height;", true);
        let got = reasons(&ws, &l);
        assert!(
            got.iter()
                .any(|r| r.contains("--tdd: cannot infer the type of the stub field Item.")),
            "{got:?}"
        );
        assert!(stub_text(&ws).is_empty());
    }

    #[test]
    fn a_name_an_app_out_of_reach_declares_gets_no_stub_object() {
        let other = AppSource {
            id: "b".into(),
            name: "b".into(),
            dependencies: vec![],
            files: vec![("b.al".into(), "codeunit 50150 \"Elsewhere\" { }".into())],
        };
        let test =
            "codeunit 50110 \"T\" { Subtype = Test; [Test] procedure P() var X: Codeunit \"Elsewhere\"; begin end; }";
        let (ws, l) = crate::lower::compile_apps_with(
            vec![
                other,
                AppSource {
                    id: "a".into(),
                    name: "a".into(),
                    dependencies: vec![],
                    files: vec![("test.al".into(), test.into())],
                },
            ],
            vec![AppKind::Workspace, AppKind::Workspace],
            &Default::default(),
            None,
            crate::check::Roots {
                tdd: true,
                tdd_id_ranges: [("a".to_string(), vec![(50100, 50119)])].into_iter().collect(),
                ..Default::default()
            },
        );
        assert!(l.program.tdd_stubs.is_empty(), "{:?}", l.program.tdd_stubs);
        assert!(ws.apps[1].files.iter().all(|(p, _)| p != super::TDD_FILE));
    }

    #[test]
    fn without_tdd_nothing_is_declared() {
        let (ws, l) = compile(
            "C: Enum \"Color\";",
            "Item.Weight := 7; C := Enum::\"Color\"::Blue;",
            false,
        );
        assert!(l.tdd_needs.is_empty() && l.program.tdd_stubs.is_empty());
        assert!(stub_text(&ws).is_empty());
        assert!(!reasons(&ws, &l).is_empty());
    }
}
