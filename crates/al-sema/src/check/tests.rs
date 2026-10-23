// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use crate::workspace::AppSource;

fn ws(files: &[&str]) -> Workspace {
    Workspace::load(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: files
                .iter()
                .enumerate()
                .map(|(i, s)| (format!("f{i}.al"), s.to_string()))
                .collect(),
        }],
        &Default::default(),
    )
}

fn run(files: &[&str]) -> (Workspace, CheckResult) {
    let w = ws(files);
    assert!(w.diagnostics.is_empty(), "load diagnostics: {:?}", w.diagnostics);
    let m = crate::members::build(&w);
    let r = check(&w, &m, &Catalog::load());
    (w, r)
}

fn diags(files: &[&str]) -> Vec<String> {
    run(files)
        .1
        .diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

/// Type of the expression in the first `exit(...)` of the last file.
fn ty_of(files: &[&str]) -> Ty {
    let (w, r) = run(files);
    let fi = w.files.len() - 1;
    let root = al_syntax::SyntaxNode::new_root(w.green(fi));
    let exit = root
        .descendants()
        .find(|n| n.kind() == al_syntax::SyntaxKind::ExitStmt)
        .expect("an exit");
    let e = exit.children().next().expect("exit has an expression");
    let range = e.text_range();
    let key = (fi, usize::from(range.start()), usize::from(range.end()));
    assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    r.types
        .get(&key)
        .cloned()
        .unwrap_or_else(|| panic!("no type for `{}`", e.text()))
}

fn cu(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

const TABLE: &str = "table 50900 \"ALT Universal\" { fields { field(1; \"No.\"; Code[20]) { } field(2; Amount; Decimal) { } field(3; Status; Option Open,Closed) { } } procedure Double(X: Integer): Integer begin exit(X * 2); end; }";

#[test]
fn literals() {
    assert_eq!(ty_of(&[&cu("procedure P(): Integer begin exit(1); end;")]), Ty::Integer);
    assert_eq!(
        ty_of(&[&cu("procedure P(): Decimal begin exit(1.5); end;")]),
        Ty::Decimal
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Text begin exit('ab'); end;")]),
        Ty::Text(None)
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Boolean begin exit(true); end;")]),
        Ty::Boolean
    );
    assert_eq!(ty_of(&[&cu("procedure P(): Date begin exit(0D); end;")]), Ty::Date);
}

#[test]
fn locals_params_and_globals() {
    assert_eq!(
        ty_of(&[&cu("procedure P(A: Decimal): Decimal begin exit(A); end;")]),
        Ty::Decimal
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Code[10] var L: Code[10]; begin exit(L); end;")]),
        Ty::Code(Some(10))
    );
    assert_eq!(
        ty_of(&[&cu("var G: Guid; procedure P(): Guid begin exit(G); end;")]),
        Ty::Guid
    );
}

#[test]
fn names_are_case_insensitive() {
    assert_eq!(
        ty_of(&[&cu(
            "procedure P(): Decimal var MyVar: Decimal; begin exit(myvar); end;"
        )]),
        Ty::Decimal
    );
}

#[test]
fn record_fields_and_table_procedures() {
    let src = cu("procedure P(): Decimal var R: Record \"ALT Universal\"; begin exit(R.Amount); end;");
    assert_eq!(ty_of(&[TABLE, &src]), Ty::Decimal);
    let src = cu("procedure P(): Integer var R: Record \"ALT Universal\"; begin exit(R.Double(2)); end;");
    assert_eq!(ty_of(&[TABLE, &src]), Ty::Integer);
}

#[test]
fn record_builtin_methods_come_from_the_catalog() {
    let src = cu("procedure P(): Boolean var R: Record \"ALT Universal\"; begin exit(R.Insert(true)); end;");
    assert_eq!(ty_of(&[TABLE, &src]), Ty::Boolean);
    let src = cu("procedure P(): Integer var R: Record \"ALT Universal\"; begin exit(R.Count()); end;");
    assert_eq!(ty_of(&[TABLE, &src]), Ty::Integer);
}

#[test]
fn global_builtins_and_own_procedures() {
    assert_eq!(
        ty_of(&[&cu("procedure P(): Text begin exit(Format(1)); end;")]),
        Ty::Text(None)
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Integer begin exit(StrLen('x')); end;")]),
        Ty::Integer
    );
    assert_eq!(ty_of(&[&cu("procedure P(): Date begin exit(Today()); end;")]), Ty::Date);
    assert_eq!(
        ty_of(&[&cu(
            "procedure Q(): Guid begin end; procedure P(): Guid begin exit(Q()); end;"
        )]),
        Ty::Guid
    );
}

#[test]
fn parameterless_calls_without_parentheses() {
    assert_eq!(ty_of(&[&cu("procedure P(): Date begin exit(Today); end;")]), Ty::Date);
    let src = cu("procedure P(): Integer var R: Record \"ALT Universal\"; begin exit(R.Count); end;");
    assert_eq!(ty_of(&[TABLE, &src]), Ty::Integer);
}

#[test]
fn codeunit_variable_procedures() {
    let other = "codeunit 50101 Other { procedure Get(): Text begin end; }";
    assert_eq!(
        ty_of(&[
            other,
            &cu("procedure P(): Text var O: Codeunit Other; begin exit(O.Get()); end;")
        ]),
        Ty::Text(None)
    );
}

#[test]
fn library_type_methods_and_generics() {
    assert_eq!(
        ty_of(&[&cu(
            "procedure P(): Integer var L: List of [Decimal]; begin exit(L.Count()); end;"
        )]),
        Ty::Integer
    );
    assert_eq!(
        ty_of(&[&cu(
            "procedure P(): Decimal var L: List of [Decimal]; begin exit(L.Get(1)); end;"
        )]),
        Ty::Decimal
    );
    assert_eq!(
        ty_of(&[&cu(
            "procedure P(): Boolean var J: JsonObject; begin exit(J.Contains('a')); end;"
        )]),
        Ty::Boolean
    );
    assert_eq!(
        ty_of(&[&cu(
            "procedure P(): Integer var T: Text; begin exit(T.IndexOf('a')); end;"
        )]),
        Ty::Integer
    );
}

#[test]
fn operators() {
    assert_eq!(
        ty_of(&[&cu("procedure P(): Boolean begin exit(1 < 2); end;")]),
        Ty::Boolean
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Decimal begin exit(1 + 1.5); end;")]),
        Ty::Decimal
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Text begin exit('a' + 'b'); end;")]),
        Ty::Text(None)
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Boolean begin exit(not true); end;")]),
        Ty::Boolean
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Boolean begin exit(1 in [1, 2..3]); end;")]),
        Ty::Boolean
    );
}

#[test]
fn implicit_rec_in_table_code() {
    let t =
        "table 50900 T { fields { field(1; Amount; Decimal) { } } procedure P(): Decimal begin exit(Amount); end; }";
    assert_eq!(ty_of(&[t]), Ty::Decimal);
    let t = "table 50900 T { fields { field(1; Amount; Decimal) { } } procedure P(): Decimal begin exit(Rec.Amount); end; }";
    assert_eq!(ty_of(&[t]), Ty::Decimal);
    let t = "table 50900 T { fields { field(1; A; Integer) { } } procedure P(): Integer begin exit(Count()); end; }";
    assert_eq!(ty_of(&[t]), Ty::Integer);
}

#[test]
fn enum_and_option_values() {
    let e = "enum 50900 \"ALT Status\" { value(0; Open) { } }";
    assert_eq!(
        ty_of(&[
            e,
            &cu("procedure P(): Enum \"ALT Status\" begin exit(Enum::\"ALT Status\"::Open); end;")
        ]),
        Ty::Enum("ALT Status".into())
    );
    assert_eq!(
        ty_of(&[
            e,
            &cu("procedure P(): Enum \"ALT Status\" begin exit(\"ALT Status\"::Open); end;")
        ]),
        Ty::Enum("ALT Status".into())
    );
    let src =
        cu("procedure P(): Boolean var R: Record \"ALT Universal\"; begin exit(R.Status = R.Status::Closed); end;");
    assert_eq!(ty_of(&[TABLE, &src]), Ty::Boolean);
}

#[test]
fn object_id_scope() {
    assert_eq!(
        ty_of(&[
            TABLE,
            &cu("procedure P(): Integer begin exit(Database::\"ALT Universal\"); end;")
        ]),
        Ty::Integer
    );
    assert_eq!(
        ty_of(&[&cu("procedure P(): Integer begin exit(Codeunit::C); end;")]),
        Ty::Integer
    );
}

#[test]
fn unresolved_name_member_and_call_are_diagnosed() {
    assert_eq!(
        diags(&[&cu("procedure P() begin Nope := 1; end;")]),
        vec![
            "AR1000 unresolved name `Nope`",
            "AL0118 The name 'Nope' does not exist in the current context."
        ]
    );
    assert_eq!(
        diags(&[
            TABLE,
            &cu("procedure P() var R: Record \"ALT Universal\"; begin R.Nope := 1; end;")
        ]),
        vec![
            "AR1002 no member `Nope` on Record \"ALT Universal\"; not known to the catalog (docs for runtime up to 18.0)",
            // And the compiler's own code for it (corpus must-not-compile 0060).
            "AL0132 'Record \"ALT Universal\"' does not contain a definition for 'Nope'"
        ]
    );
    assert_eq!(
        diags(&[&cu("procedure P() begin Format(); end;")]),
        vec!["AR1003 no overload of `Format` takes 0 argument(s)"]
    );
}

fn diags_for(bc: &str, files: &[&str]) -> Vec<String> {
    let w = ws(files);
    let m = crate::members::build(&w);
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    check(&w, &m, &Catalog::for_target(&target))
        .diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn a_member_from_a_newer_runtime_is_refused_with_al0666() {
    // Record.FullyQualifiedName is available from runtime 17.0 (BC 28); the text is the AL compiler's
    // (must-not-compile 0620-record-fullyqualifiedname-runtime-16.al:1).
    let src = cu("procedure P() var R: Record \"ALT Universal\"; T: Text; begin T := R.FullyQualifiedName(); end;");
    let on27 = diags_for("27.5", &[TABLE, &src]);
    assert_eq!(
        on27,
        vec![
            "AL0666 'FullyQualifiedName' is not available in runtime version '16.0'. The supported runtime versions are: '17.0' or greater."
        ]
    );
    assert_eq!(diags_for("28", &[TABLE, &src]), Vec::<String>::new());
}

fn diags_declared(bc: &str, runtime: &str, files: &[&str]) -> Vec<String> {
    let w = ws(files);
    let m = crate::members::build(&w);
    let mut target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    target.app_runtimes = vec![("a".into(), al_version::Runtime::parse(runtime).unwrap())];
    check(&w, &m, &Catalog::for_target(&target))
        .diagnostics
        .iter()
        .map(|d| format!("{} {}", d.code, d.message))
        .collect()
}

#[test]
fn an_app_declaring_an_older_runtime_than_the_target_gets_al0666_for_its_own_runtime() {
    // al-runner issue 5573: on BC 28 (runtime 17.0) an app that declares runtime 16.0 cannot call Codeunit.Run by name.
    let by_name = cu("procedure P() begin Codeunit.Run('Some Codeunit'); end;");
    assert_eq!(diags_for("28", &[&by_name]), Vec::<String>::new());
    assert_eq!(
        diags_declared("28", "16.0", &[&by_name]),
        vec![
            "AL0666 'Run' is not available in runtime version '16.0'. The supported runtime versions are: '17.0' or greater."
        ]
    );
    assert_eq!(diags_declared("28", "17.0", &[&by_name]), Vec::<String>::new());
}

#[test]
fn below_the_runtime_floor_the_al_compilers_table_decides() {
    // al-runner issue 5573: the AL compiler (30.0.42.60748) rejects NavApp.GetResourceAsText(Text, TextEncoding) below runtime 14.0
    // (catalog/alc-runtimes.txt). The sentence is the one must-not-compile 0620 pins; the versions part is alc's.
    let src =
        cu("procedure P() var T: Text; begin T := NavApp.GetResourceAsText('value.txt', TextEncoding::UTF8); end;");
    assert_eq!(
        diags_declared("28", "13.0", &[&src]),
        vec![
            "AL0666 'GetResourceAsText' is not available in runtime version '13.0'. The supported runtime versions are: '14.0' or greater."
        ]
    );
    assert_eq!(diags_declared("28", "14.0", &[&src]), Vec::<String>::new());
    // Message compiles at every runtime alc accepts.
    let msg = cu("procedure P() begin Message('x'); end;");
    assert_eq!(diags_declared("28", "1.0", &[&msg]), Vec::<String>::new());
    // Codeunit.Run by name is tagged 17.0: below the floor the table gives alc's text too.
    let by_name = cu("procedure P() begin Codeunit.Run('Some Codeunit'); end;");
    assert_eq!(
        diags_declared("28", "13.0", &[&by_name]),
        vec![
            "AL0666 'Run' is not available in runtime version '13.0'. The supported runtime versions are: '17.0' or greater."
        ]
    );
}

#[test]
fn below_the_runtime_floor_a_call_without_an_al0666_answer_is_refused_with_ar1007() {
    // Below runtime 4.0 an app cannot target OnPrem, so alc reports AL0296 (scope) for File.Exists, not AL0666.
    let src = cu("procedure P() var B: Boolean; begin B := File.Exists('x'); end;");
    let got = diags_declared("28", "3.0", &[&src]);
    assert!(
        got.len() == 1
            && got[0].starts_with("AR1007 `Exists` does not compile at runtime 3.0 with AL compiler 30.0.43.45776,"),
        "{got:?}"
    );
    assert_eq!(diags_declared("28", "4.0", &[&src]), Vec::<String>::new());
}

#[test]
fn a_method_removed_in_an_older_runtime_is_al0666_at_every_later_runtime() {
    // must-not-compile 0561-testpage-prev.al:1: `'Prev' is not available in runtime version '`; alc 30.0.42.60748:
    // "The supported runtime versions are: '3.0' until, but not including '13.0'."
    let src = "codeunit 50100 T { Subtype = Test; procedure P() var Pg: TestPage \"ALT Page\"; begin Pg.Prev(); end; }";
    let page = "page 50101 \"ALT Page\" { }";
    let want = |at: &str| {
        vec![format!(
            "AL0666 'Prev' is not available in runtime version '{at}'. The supported runtime versions are: '3.0' until, but not including '13.0'."
        )]
    };
    assert_eq!(diags_for("27", &[src, page]), want("16.0"));
    assert_eq!(diags_declared("28", "13.0", &[src, page]), want("13.0"));
    assert_eq!(diags_declared("28", "12.0", &[src, page]), Vec::<String>::new());
}

#[test]
fn an_overload_from_a_newer_runtime_is_refused_when_it_fits_better() {
    // Codeunit.Run(FullyQualifiedName: Text) is 17.0; Codeunit.Run(Number: Integer) is older.
    let by_name = cu("procedure P() begin Codeunit.Run('Some Codeunit'); end;");
    let on27 = diags_for("27", &[&by_name]);
    assert_eq!(on27.len(), 1, "{on27:?}");
    assert!(
        on27[0].starts_with("AL0666 'Run' is not available in runtime version '16.0'"),
        "{on27:?}"
    );
    assert_eq!(diags_for("28", &[&by_name]), Vec::<String>::new());
    // An integer id fits the older overload on every target.
    let by_id = cu("procedure P() begin Codeunit.Run(50100); end;");
    assert_eq!(diags_for("27", &[&by_id]), Vec::<String>::new());
    assert_eq!(diags_for("28", &[&by_id]), Vec::<String>::new());
}

#[test]
fn an_unknown_builtin_member_says_it_is_not_in_the_catalog() {
    let src = cu("procedure P() var J: JsonObject; begin J.Nope(); end;");
    let d = diags_for("27", &[&src]);
    // The AL compiler's own code comes with it (alcodes: AL0132).
    assert_eq!(d.len(), 2, "{d:?}");
    assert_eq!(d[1], "AL0132 'JsonObject' does not contain a definition for 'Nope'");
    assert!(d[0].starts_with("AR1002 no member `Nope` on JsonObject"), "{d:?}");
    assert!(
        d[0].contains("not known to the catalog") && d[0].contains("up to 18.0"),
        "{d:?}"
    );
}

#[test]
fn unknown_types_do_not_cascade() {
    // `Customer` is not in the workspace: AR0002 at load, nothing more here.
    let w = ws(&[&cu(
        "procedure P() var C: Record Customer; begin C.Name := 'x'; C.Foo(); end;",
    )]);
    let m = crate::members::build(&w);
    assert!(check(&w, &m, &Catalog::load()).diagnostics.is_empty());
}

#[test]
fn field_arguments_resolve_against_the_receiver_table() {
    let src = cu(
        "procedure P() var R: Record \"ALT Universal\"; begin R.SetRange(\"No.\", 'A'); R.CalcFields(Amount, Status); R.SetCurrentKey(Amount); R.TestField(\"No.\"); end;",
    );
    assert_eq!(diags(&[TABLE, &src]), Vec::<String>::new());
    let src = cu("procedure P(): Integer var R: Record \"ALT Universal\"; begin exit(R.FieldNo(Amount)); end;");
    assert_eq!(ty_of(&[TABLE, &src]), Ty::Integer);
}

#[test]
fn field_positions_do_not_capture_value_arguments() {
    // `Amount` here is the filter VALUE: it must resolve as a variable, so an
    // undeclared one is still an error.
    let src = cu("procedure P() var R: Record \"ALT Universal\"; begin R.SetRange(\"No.\", Nope); end;");
    assert_eq!(
        diags(&[TABLE, &src]),
        vec![
            "AR1000 unresolved name `Nope`",
            "AL0118 The name 'Nope' does not exist in the current context."
        ]
    );
}

const PAGES: &str = "page 50900 \"ALT List\" { SourceTable = \"ALT Universal\"; layout { area(Content) { field(\"No.\"; Rec.\"No.\") { } part(Lines; \"ALT Sub\") { } } } actions { area(Processing) { action(Go) { } } } }\npage 50901 \"ALT Sub\" { SourceTable = \"ALT Universal\"; layout { area(Content) { field(Amt; Rec.Amount) { } } } }";

#[test]
fn test_page_controls_parts_and_filter() {
    let src = cu(
        "procedure P() var TP: TestPage \"ALT List\"; begin TP.\"No.\".SetValue('x'); TP.Go.Invoke(); TP.Lines.Amt.SetValue(1); TP.Filter.SetFilter(Amount, '1'); end;",
    );
    assert_eq!(diags(&[TABLE, PAGES, &src]), Vec::<String>::new());
}

#[test]
fn field_arguments_on_an_unresolved_table_are_not_reported() {
    // `Date` is a platform table not in the workspace (AR0002 at load).
    let w = ws(&[&cu(
        "procedure P() var D: Record Date; begin D.SetRange(\"Period Type\", 1); end;",
    )]);
    let m = crate::members::build(&w);
    assert!(check(&w, &m, &Catalog::load()).diagnostics.is_empty());
}

#[test]
fn builtin_test_page_methods_win_over_controls_when_called() {
    let page = "page 50902 \"ALT Modal\" { layout { area(Content) { field(OK; X) { } } } }";
    let src = cu("procedure P() var TP: TestPage \"ALT Modal\"; begin TP.OK().Invoke(); end;");
    assert_eq!(diags(&[page, &src]), Vec::<String>::new());
}

#[test]
fn curr_field_no_in_table_code() {
    let t = "table 50900 T { fields { field(1; A; Integer) { trigger OnValidate() begin A := CurrFieldNo; end; } } }";
    assert_eq!(diags(&[t]), Vec::<String>::new());
}

#[test]
fn xmlport_text_elements_are_variables() {
    let x = "xmlport 50900 X { schema { textelement(Root) { textattribute(NoteAttr) { trigger OnBeforePassVariable() begin NoteAttr := UpperCase(NoteAttr); end; } } } }";
    assert_eq!(diags(&[x]), Vec::<String>::new());
}

#[test]
fn xmlport_bigtext_text_element_is_a_bigtext_variable() {
    // corpus xmlport/lt-xmlport-bigtext/XmlPortBigText.al: a text element with TextType = BigText is a BigText variable.
    let x = "xmlport 50901 X { schema { textelement(Root) { textelement(Big) { TextType = BigText; } } } trigger OnPreXmlPort() var N: Integer; begin Big.AddText('x'); N := Big.Length(); end; }";
    assert_eq!(diags(&[x]), Vec::<String>::new());
}

#[test]
fn page_extension_controls_are_test_page_members() {
    let ext =
        "pageextension 50903 E extends \"ALT List\" { layout { addlast(Content) { field(ExtCtl; Rec.Amount) { } } } }";
    let src = cu(
        "procedure P() var TP: TestPage \"ALT List\"; begin TP.ExtCtl.SetValue(1); TP.Lines.Filter.SetFilter(Amount, '1'); end;",
    );
    assert_eq!(diags(&[TABLE, PAGES, ext, &src]), Vec::<String>::new());
}

#[test]
fn unresolved_enum_type_before_scope_is_an_external_reference() {
    assert_eq!(
        diags(&[&cu(
            "procedure P() var X: Integer; begin X := \"Reservation From Stock\"::\" \"; end;"
        )]),
        vec!["AR0002 unresolved enum `Reservation From Stock`"]
    );
}

#[test]
fn namespace_qualified_object_after_scope() {
    let t = "namespace A.B;\ntable 50910 \"Ship\" { }";
    assert_eq!(
        ty_of(&[
            t,
            &cu("procedure P(): Integer begin exit(Database::A.B.\"Ship\"); end;")
        ]),
        Ty::Integer
    );
    assert_eq!(
        diags(&[
            t,
            &cu("procedure P() var R: RecordRef; begin R.Open(Database::X.Y.\"Ship\"); end;")
        ]),
        vec!["AR0002 unresolved table `X.Y.Ship`"]
    );
}

#[test]
fn call_syntax_prefers_functions_over_fields_of_the_same_name() {
    let t = "table 50911 U { fields { field(1; UserSecurityId; Guid) { } } procedure P() begin UserSecurityId := UserSecurityId(); end; }";
    assert_eq!(diags(&[t]), Vec::<String>::new());
}

const REPORT: &str = "report 50920 \"ALT Rep\" { dataset { dataitem(Rows; \"ALT Universal\") { trigger OnAfterGetRecord() begin Amount := Amount + 1; end; } } }";

#[test]
fn data_item_triggers_see_the_data_item_record_unqualified() {
    assert_eq!(diags(&[TABLE, REPORT]), Vec::<String>::new());
}

#[test]
fn report_extensions_see_base_data_items() {
    let ext = "reportextension 50921 E extends \"ALT Rep\" { dataset { modify(Rows) { trigger OnAfterAfterGetRecord() begin Rows.Amount := Amount; end; } } }";
    assert_eq!(diags(&[TABLE, REPORT, ext]), Vec::<String>::new());
}

#[test]
fn test_request_page_data_item_is_a_filter_on_its_table() {
    let src = cu("procedure P(RP: TestRequestPage \"ALT Rep\") begin RP.Rows.SetFilter(Amount, '1'); end;");
    assert_eq!(diags(&[TABLE, REPORT, &src]), Vec::<String>::new());
}

#[test]
fn table_extension_of_an_external_table_still_has_rec() {
    let w = ws(&[
        "tableextension 50930 E extends Job { fields { field(50000; F; Integer) { trigger OnValidate() begin Rec.F := CurrFieldNo; end; } } }",
    ]);
    let m = crate::members::build(&w);
    assert!(check(&w, &m, &Catalog::load()).diagnostics.is_empty());
}

/// Resolution of `sub`, located inside the first occurrence of `context`.
fn res_in(files: &[&str], context: &str, sub: &str) -> Res {
    let (w, r) = run(files);
    let fi = w.files.len() - 1;
    let src = al_syntax::SyntaxNode::new_root(w.green(fi)).text().to_string();
    let start = src.find(context).unwrap_or_else(|| panic!("`{context}` not in source"))
        + context.find(sub).expect("sub in context");
    let key = (fi, start, start + sub.len());
    r.resolutions.get(&key).cloned().unwrap_or_else(|| {
        panic!(
            "no resolution for `{sub}`; have {:?}",
            r.resolutions.keys().filter(|k| k.0 == fi).collect::<Vec<_>>()
        )
    })
}

#[test]
fn resolutions_for_variables() {
    assert_eq!(
        res_in(
            &[&cu("var G: Integer; procedure P() var L: Integer; begin L := G; end;")],
            "L := G",
            "G"
        ),
        Res::Global("g".into())
    );
}

#[test]
fn resolutions_for_locals_and_params() {
    let src = cu("procedure P(A: Integer) var L: Integer; begin L := A; end;");
    assert_eq!(res_in(&[&src], "L := A", "L"), Res::Local("l".into()));
    assert_eq!(res_in(&[&src], "L := A", "A"), Res::Local("a".into()));
}

#[test]
fn resolutions_for_calls_pick_the_overload() {
    let r = res_in(
        &[&cu("procedure P() var T: Text; begin T := Format(1, 0, 2); end;")],
        "Format(1, 0, 2)",
        "Format(1, 0, 2)",
    );
    assert_eq!(r, Res::Builtin("System.Format#1".into()));
    let r = res_in(
        &[&cu(
            "procedure P() var T: Text; begin T := Format(1, 0, '<Integer>'); end;",
        )],
        "Format(1, 0, '<Integer>')",
        "Format(1, 0, '<Integer>')",
    );
    assert_eq!(r, Res::Builtin("System.Format#2".into()));
}

#[test]
fn resolutions_for_procedures() {
    let src = cu("procedure Q(): Integer begin end; procedure P() var I: Integer; begin I := Q(); end;");
    let Res::Proc { object, offset } = res_in(&[&src], "I := Q()", "Q()") else {
        panic!()
    };
    assert_eq!(object, 0);
    assert_eq!(offset, src.find("procedure Q").unwrap());
}

#[test]
fn resolutions_for_enum_and_option_values() {
    let e = "enum 50900 \"ALT Status\" { value(0; Open) { } value(5; Closed) { } }";
    let r = res_in(
        &[
            e,
            &cu("procedure P(): Enum \"ALT Status\" begin exit(\"ALT Status\"::Closed); end;"),
        ],
        "exit(\"ALT Status\"::Closed",
        "\"ALT Status\"::Closed",
    );
    assert!(matches!(r, Res::EnumValue { value: 5, .. }), "{r:?}");
    let src =
        cu("procedure P() var R: Record \"ALT Universal\"; B: Boolean; begin B := R.Status = R.Status::Closed; end;");
    assert_eq!(
        res_in(&[TABLE, &src], "= R.Status::Closed", "R.Status::Closed"),
        Res::OptionValue(1)
    );
}

#[test]
fn statements_are_walked() {
    assert_eq!(
        diags(&[&cu(
            "procedure P() var i: Integer; begin if X then; for i := 1 to Y do; case Z of 1: ; end; repeat until W; end;"
        )]),
        ["X", "Y", "Z", "W"]
            .iter()
            .flat_map(|n| {
                [
                    format!("AR1000 unresolved name `{n}`"),
                    format!("AL0118 The name '{n}' does not exist in the current context."),
                ]
            })
            .collect::<Vec<_>>()
    );
}

/// Parameter names the AL compiler rejects (30.0.42.60748-beta, the corpus CI's compiler; measured 2026-10-22 with one
/// probe file per word, `procedure P1(<word>: Integer)` and `procedure P1(var <word>: Integer)`): the first diagnostic
/// alc gives for each, at the name. A quoted name, and every other keyword (`local`, `if`, `fields`, ...), compiles.
#[test]
fn keywords_alc_rejects_as_parameter_names() {
    let one = |params: &str| diags(&[&cu(&format!("procedure P1({params}) begin end;"))]);
    for w in ["begin", "procedure", "trigger"] {
        assert_eq!(
            one(&format!("{w}: Integer")),
            vec!["AL0104 Syntax error, ')' expected"],
            "{w}"
        );
    }
    for w in [
        "end",
        "of",
        "true",
        "false",
        "event",
        "key",
        "area",
        "actions",
        "chartpart",
        "cuegroup",
        "extends",
        "fieldattribute",
        "fieldelement",
        "fieldgroups",
        "pageextension",
        "profileextension",
        "tableelement",
        "tableextension",
        "textattribute",
        "textelement",
    ] {
        assert_eq!(
            one(&format!("{w}: Integer")),
            vec![format!("AL0105 Syntax error, identifier expected; '{w}' is a keyword")],
            "{w}"
        );
    }
    // The word is named in lower case whatever its case in the source.
    assert_eq!(
        one("Key: Integer"),
        vec!["AL0105 Syntax error, identifier expected; 'key' is a keyword"]
    );
    let invalid =
        "AL0107 Syntax error, identifier expected. Provide a valid name (letters, digits, and underscores only).";
    for w in [
        "begin",
        "end",
        "of",
        "var",
        "procedure",
        "trigger",
        "true",
        "key",
        "textelement",
    ] {
        assert_eq!(one(&format!("var {w}: Integer")), vec![invalid], "var {w}");
    }
    // Accepted by alc: quoted names, `local`, and the other keywords.
    for params in [
        "\"key\": Integer; \"begin\": Integer",
        "local: Integer",
        "var local: Integer",
        "if: Integer",
        "exit: Integer",
        "fields: Integer",
        "layout: Integer",
        "keys: Integer",
        "var x: Integer; y: Integer",
    ] {
        assert_eq!(one(params), Vec::<String>::new(), "{params}");
    }
}

/// The full diagnostic lists alc gives (its recovery included) for the two shapes the must-not-compile corpus pins whole:
/// a file of two codeunits whose one procedure takes `<word>: Integer` and `var <word>: Integer` (cases 2016-2039), and a
/// variable declared `"If"` used unquoted (case 2040). Only the AL codes, sorted, as the corpus harness compares them.
#[test]
fn keyword_name_files_give_the_full_lists_alc_gives() {
    // Two of the shapes do not parse here (`var: Integer`, `If := 1`): the parse error stands, the AL codes come anyway.
    let al = |src: &str| {
        let w = ws(&[src]);
        let r = check(&w, &crate::members::build(&w), &Catalog::load());
        let mut v: Vec<String> = r
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("AL"))
            .map(|d| format!("{} {}", d.code, d.message))
            .collect();
        v.sort();
        v
    };
    let pair = |w: &str| {
        format!(
            "codeunit 50000 \"A\"\n{{\n    procedure P1({w}: Integer)\n    begin\n    end;\n}}\n\ncodeunit 50001 \"B\"\n{{\n    procedure P2(var {w}: Integer)\n    begin\n    end;\n}}\n"
        )
    };
    let invalid =
        "AL0107 Syntax error, identifier expected. Provide a valid name (letters, digits, and underscores only).";
    let mut of = vec![
        "AL0105 Syntax error, identifier expected; 'of' is a keyword".to_string(),
        "AL0104 Syntax error, ':' expected".into(),
        invalid.into(),
        invalid.into(),
        "AL0104 Syntax error, ':' expected".into(),
        "AL0105 Syntax error, identifier expected; 'of' is a keyword".into(),
        "AL0104 Syntax error, ';' expected".into(),
    ];
    of.sort();
    assert_eq!(al(&pair("of")), of);
    assert_eq!(al(&pair("Of")), of, "the word is named in lower case");
    let mut var = vec![
        invalid.to_string(),
        invalid.into(),
        "AL0104 Syntax error, ':' expected".into(),
        "AL0105 Syntax error, identifier expected; 'var' is a keyword".into(),
        "AL0104 Syntax error, ';' expected".into(),
    ];
    var.sort();
    assert_eq!(al(&pair("var")), var);
    for (w, n) in [("begin", 15), ("procedure", 13), ("trigger", 15), ("textelement", 7)] {
        assert_eq!(al(&pair(w)).len(), n, "{w}");
    }
    let mut unquoted = vec![
        "AL0224 Expression expected. Provide a valid expression (variable, constant, calculation, or method call)."
            .to_string(),
        "AL0104 Syntax error, 'then' expected".into(),
    ];
    unquoted.sort();
    assert_eq!(
        al(
            "codeunit 50100 \"C\"\n{\n    procedure P()\n    var\n        \"If\": Integer;\n    begin\n        If := 1;\n    end;\n}\n"
        ),
        unquoted
    );
    // Another shape keeps the first diagnostic only: one codeunit, or a second statement.
    assert_eq!(
        al("codeunit 50000 \"A\" { procedure P1(of: Integer) begin end; }"),
        vec!["AL0105 Syntax error, identifier expected; 'of' is a keyword"]
    );
    assert_eq!(
        al("codeunit 50100 \"C\" { procedure P() var \"If\": Integer; begin If := 1; If := 2; end; }"),
        Vec::<String>::new()
    );
}

/// AL0398 for a Decimal constant where an Option ordinal is expected: a case label of an Option selector
/// (corpus tests/must-not-compile/cases/2008-case-option-decimal-label.al:1) and the `in` list of an Option
/// (2009-option-in-decimal-list.al:1-2). Comparing an Option with a Decimal compiles, and so does a Decimal
/// variable as a case label (types/PROptionInteger.al:175-178, :308-322).
#[test]
fn al0398_decimal_constant_against_an_option() {
    let case = cu(
        "procedure P(): Boolean var O: Option None,Quantity,Amount; begin case O of 1.0: exit(true); end; exit(false); end;",
    );
    assert_eq!(
        diags(&[&case]),
        vec!["AL0398 Constant value '1.0' is outside of the valid ordinal range for this Option type"]
    );
    let list = cu("procedure P(): Boolean var O: Option None,Quantity,Amount; begin exit(O in [0.0, 1.5]); end;");
    assert_eq!(
        diags(&[&list]),
        vec![
            "AL0398 Constant value '0.0' is outside of the valid ordinal range for this Option type",
            "AL0398 Constant value '1.5' is outside of the valid ordinal range for this Option type",
        ]
    );
    let fine = cu(
        "procedure P(): Boolean var O: Option None,Quantity,Amount; D: Decimal; begin case O of D: exit(true); 1: exit(O = 1.5); end; exit(O < 1.0); end;",
    );
    assert_eq!(diags(&[&fine]), Vec::<String>::new());
}

// Language features by runtime (al-runner issue 5573): `al-builtins/catalog/alc-features.txt`, measured with the AL
// compiler. The sentence is the one the must-not-compile corpus pins; the name and the versions part are alc's.
const FT_TABLE: &str =
    "table 50900 \"FT T\" { fields { field(1; \"No.\"; Integer) { } } keys { key(PK; \"No.\") { } } }";

fn al0666(name: &str, at: &str, versions: &str) -> String {
    format!(
        "AL0666 '{name}' is not available in runtime version '{at}'. The supported runtime versions are: {versions}."
    )
}

#[test]
fn a_page_area_takes_no_obsolete_state_at_runtime_13_and_later() {
    // must-not-compile 0799-obsoletestate-on-page-areas.al:2 (16.1), 0778 (18.0).
    let page = "page 50901 \"FT P\" { PageType = Card; SourceTable = \"FT T\"; layout { area(Content) { ObsoleteState = Pending; field(F; Rec.\"No.\") { } } } actions { area(Processing) { ObsoleteTag = '1.0'; action(A) { } } } }";
    assert_eq!(
        diags_declared("29", "16.1", &[FT_TABLE, page]),
        vec![
            al0666("ObsoleteState", "16.1", "'4.0' until, but not including '13.0'"),
            al0666("ObsoleteTag", "16.1", "'4.3' until, but not including '13.0'"),
        ]
    );
    assert_eq!(diags_declared("29", "12.0", &[FT_TABLE, page]), Vec::<String>::new());
}

#[test]
fn importance_optional_is_refused_by_value() {
    // must-not-compile 1431-page-field-importance-optional.al:1: `'Optional' is not available in runtime version '`.
    let page = |imp: &str| {
        format!(
            "page 50901 \"FT P\" {{ PageType = Card; SourceTable = \"FT T\"; layout {{ area(Content) {{ field(F; Rec.\"No.\") {{ Importance = {imp}; }} }} }} }}"
        )
    };
    assert_eq!(
        diags_for("29", &[FT_TABLE, &page("Optional")]),
        vec![al0666("Optional", "18.0", "'1.0' until, but not including '3.0'")]
    );
    assert_eq!(diags_for("29", &[FT_TABLE, &page("Promoted")]), Vec::<String>::new());
}

#[test]
fn report_parts_need_runtime_18() {
    // must-not-compile 0800-report-defaultheaderfooterpart-bc27-28.al:1, 0803-layout-subtype-bc27-28.al:1.
    let report = "report 50902 \"FT R\" { DefaultRenderingLayout = L; DefaultHeaderFooterPart = HF; dataset { dataitem(D; \"FT T\") { column(No; \"No.\") { } } } rendering { layout(L) { Type = Word; LayoutFile = 'l.docx'; } layout(HF) { Type = Word; LayoutFile = 'hf.docx'; Subtype = HeaderFooter; } } }";
    assert_eq!(
        diags_declared("29", "17.1", &[FT_TABLE, report]),
        vec![
            al0666("DefaultHeaderFooterPart", "17.1", "'18.0' or greater"),
            al0666("Subtype", "17.1", "'18.0' or greater"),
        ]
    );
    assert_eq!(diags_for("29", &[FT_TABLE, report]), Vec::<String>::new());
}

#[test]
fn bc29_types_and_attributes_need_runtime_18() {
    // must-not-compile 0995-datasourcecontext-runtime-16.al:1, 0996-testhandlercontext-runtime-16.al:1,
    // 0997-requiredpending-runtime-16.al:1.
    let src = "codeunit 50903 \"FT C\" { [RequiredPending('x', '29.0')] procedure A() begin end; procedure B() var Ctx: DataSourceContext; begin end; }";
    assert_eq!(
        diags_declared("29", "16.0", &[src]),
        vec![
            al0666("RequiredPending", "16.0", "'18.0' or greater"),
            al0666("DataSourceContext", "16.0", "'18.0' or greater"),
        ]
    );
    assert_eq!(diags_for("29", &[src]), Vec::<String>::new());
}

#[test]
fn a_page_extension_cannot_modify_editable_of_a_part() {
    // must-not-compile 0684-pageext-modify-part-editable.al:1. Of a field it can (no row says otherwise).
    let part = "page 50904 \"FT LP\" { PageType = ListPart; SourceTable = \"FT T\"; layout { area(Content) { repeater(R) { field(No; Rec.\"No.\") { } } } } }";
    let card = "page 50905 \"FT P\" { PageType = Card; SourceTable = \"FT T\"; layout { area(Content) { field(No; Rec.\"No.\") { } part(Pt; \"FT LP\") { } } } }";
    let ext = |ctl: &str| {
        format!(
            "pageextension 50906 \"FT PE\" extends \"FT P\" {{ layout {{ modify({ctl}) {{ Editable = false; }} }} }}"
        )
    };
    assert_eq!(
        diags_for("29", &[FT_TABLE, part, card, &ext("Pt")]),
        vec![al0666(
            "Modifying Editable from Extensions",
            "18.0",
            "'1.0' until, but not including '10.1'"
        )]
    );
    assert_eq!(
        diags_for("29", &[FT_TABLE, part, card, &ext("No")]),
        Vec::<String>::new()
    );
}
