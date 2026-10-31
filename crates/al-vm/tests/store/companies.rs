//! Companies: `ChangeCompany`, the Company table, `RecordRef.Open` with a company name. Each test
//! names the corpus test that pins it.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const TABLES: &str = r#"
table 50910 "CC Row"
{
    fields { field(1; Id; Integer) { } field(2; V; Integer) { } }
    keys { key(PK; Id) { } }
}
table 50911 "CC Tenant"
{
    DataPerCompany = false;
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { } }
}
table 50912 "CC Rel"
{
    fields { field(1; Id; Integer) { } field(2; RowId; Integer) { TableRelation = "CC Row".Id; } }
    keys { key(PK; Id) { } }
}
table 50913 "CC Parent"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kids; Integer) { FieldClass = FlowField; CalcFormula = count("CC Child" where(ParentId = field(Id))); }
    }
    keys { key(PK; Id) { } }
}
table 50914 "CC Child"
{
    fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } }
    keys { key(PK; Id) { } }
}
table 2000000006 Company
{
    DataPerCompany = false;
    fields
    {
        field(1; Name; Text[30]) { }
        field(2; "Evaluation Company"; Boolean) { }
        field(3; "Display Name"; Text[250]) { }
        field(8000; Id; Guid) { }
        field(8005; "Business Profile Id"; Text[250]) { }
    }
    keys { key(PK; Name) { } }
}
"#;

const NO_COMPANY_TABLE: &str = r#"
table 50910 "CC Row"
{
    fields { field(1; Id; Integer) { } field(2; V; Integer) { } }
    keys { key(PK; Id) { } }
}
"#;

fn run_with(tables: &str, vars: &str, body: &str) -> Result<Value, Signal> {
    let src = format!("{tables}\ncodeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    al_testkit::run_in(&src, "C", "F")
}

fn text(vars: &str, body: &str) -> String {
    match run_with(TABLES, vars, body) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

const NEW_CO: &str = "Co: Record Company;";
const MAKE_CO: &str = "Co.Name := 'OTHER'; Co.Insert(); Commit();";

#[test]
fn change_company_to_the_empty_name_is_the_current_company() {
    // record/TestRecordGet.al:145-153 (`ChangeCompany('')` is true: empty string = current company).
    let r = run_with(
        NO_COMPANY_TABLE,
        "R: Record \"CC Row\"; ok: Boolean;",
        "ok := R.ChangeCompany(''); exit(Format(ok) + R.CurrentCompany());",
    );
    assert!(
        matches!(r, Ok(Value::Text(ref s)) if s.starts_with("Yes") && s.len() > 3),
        "{r:?}"
    );
}

#[test]
fn change_company_to_an_unknown_name_is_false() {
    // record/TestSecurityFilteringContracts.al:149-160 (a name nobody created answers false). The
    // test's app does not use the Company table, so the session company is the only company.
    let r = run_with(
        NO_COMPANY_TABLE,
        "R: Record \"CC Row\"; ok: Boolean;",
        "ok := R.ChangeCompany('NonExistentCompany12345XYZ'); exit(Format(ok));",
    );
    assert!(matches!(r, Ok(Value::Text(ref s)) if s == "No"), "{r:?}");
    // With the Company table in the program: the same answer while no row has that name.
    assert_eq!(
        text(
            "R: Record \"CC Row\"; ok: Boolean;",
            "ok := R.ChangeCompany('NOPE'); exit(Format(ok));"
        ),
        "No"
    );
}

#[test]
fn the_answer_follows_the_company_table() {
    // record/TestRecordChangeCompanyRows.al:128-145 (ChangeCompany_ToCompanyThatNoLongerExists_AnswersFalse).
    let out = text(
        &format!("{NEW_CO} R: Record \"CC Row\"; L: Record \"CC Row\";"),
        &format!(
            "{MAKE_CO} Co.Get('OTHER');
             exit(Format(R.ChangeCompany('OTHER')) + '|' + R.CurrentCompany() + '|');
             Co.Delete(); Commit();
             exit(Format(L.ChangeCompany('OTHER')));"
        ),
    );
    // the first exit ends the routine; the deleted-company arm is the next test
    assert_eq!(out, "Yes|OTHER|");
    let out = text(
        &format!("{NEW_CO} R: Record \"CC Row\"; L: Record \"CC Row\"; a: Text;"),
        &format!(
            "{MAKE_CO} a := Format(R.ChangeCompany('OTHER'));
             Co.Get('OTHER'); Co.Delete(); Commit();
             exit(a + Format(L.ChangeCompany('OTHER')));"
        ),
    );
    assert_eq!(out, "YesNo");
}

/// AL that appends `Format(e) + ';'` to `s` for each expression, between the statements.
fn log(e: &str) -> String {
    format!("s += Format({e}) + ';';")
}

#[test]
fn the_rows_of_two_companies_are_apart_and_a_table_not_per_company_is_shared() {
    // record/TestRecordChangeCompanyRows.al:75-132 (ChangeCompany_ToInsertedCompany_ReadsAndWritesThatCompanysOwnRows).
    let out = text(
        &format!(
            "{NEW_CO} Home: Record \"CC Row\"; Other: Record \"CC Row\"; Back: Record \"CC Row\";
             T: Record \"CC Tenant\"; TOther: Record \"CC Tenant\"; s: Text;"
        ),
        &format!(
            "Home.Id := 1; Home.V := 11; Home.Insert(); {MAKE_CO}
             {} {} {}
             Other.Id := 1; Other.V := 22; Other.Insert();
             {}
             Other.Get(1); {}
             Home.Get(1); {} {}
             Other.V := 33; Other.Modify(); Home.Get(1); {}
             Other.DeleteAll(); {} {}
             {} Back.Get(1); {} {}
             T.Id := 7; T.Insert();
             {} {} {}
             exit(s);",
            log("Other.ChangeCompany('OTHER')"),
            log("Other.CurrentCompany()"),
            log("Other.Count()"),
            log("Other.Count()"),
            log("Other.V"),
            log("Home.V"),
            log("Home.Count()"),
            log("Home.V"),
            log("Other.Count()"),
            log("Home.Count()"),
            log("Back.ChangeCompany(CompanyName())"),
            log("Back.Count()"),
            log("Back.V"),
            log("TOther.ChangeCompany('OTHER')"),
            log("TOther.Count()"),
            log("TOther.Get(7)"),
        ),
    );
    assert_eq!(out, "Yes;OTHER;0;1;22;11;1;11;0;1;Yes;1;11;Yes;1;Yes;");
}

#[test]
fn a_table_relation_is_validated_against_the_rows_of_the_records_own_company() {
    // record/TestRecordChangeCompanyRows.al:121-131: the session company holds Id 1, the other
    // company does not, so Validate fails there until the other company holds the row.
    let out = text(
        &format!("{NEW_CO} Home: Record \"CC Row\"; Other: Record \"CC Row\"; Rel: Record \"CC Rel\"; s: Text;"),
        &format!(
            "Home.Id := 1; Home.Insert(); {MAKE_CO}
             Rel.ChangeCompany('OTHER'); Rel.Id := 1;
             asserterror Rel.Validate(RowId, 1);
             {}
             Other.ChangeCompany('OTHER'); Other.Id := 1; Other.Insert();
             Rel.Validate(RowId, 1); {}
             exit(s);",
            log("StrPos(GetLastErrorText(), 'cannot be found in the related table') > 0"),
            log("Rel.RowId"),
        ),
    );
    assert_eq!(out, "Yes;1;");
}

#[test]
fn an_error_rolls_back_the_uncommitted_writes_in_the_other_company_only() {
    // record/TestRecordChangeCompanyRows.al:187-232 (ChangeCompany_ErrorRollsBackAnUncommittedWriteInTheOtherCompany).
    let out = text(
        &format!("{NEW_CO} Other: Record \"CC Row\"; s: Text;"),
        &format!(
            "{MAKE_CO} Other.ChangeCompany('OTHER');
             Other.Id := 4; Other.Insert(); {}
             asserterror Error('roll back the first write'); {}
             Other.Id := 5; Other.Insert(); Commit();
             Other.Id := 6; Other.Insert(); {}
             asserterror Error('roll back'); {} {} {}
             exit(s);",
            log("Other.Count()"),
            log("Other.Count()"),
            log("Other.Count()"),
            log("Other.Count()"),
            log("Other.Get(5)"),
            log("Other.Get(6)"),
        ),
    );
    assert_eq!(out, "1;0;2;1;Yes;No;");
}

#[test]
fn a_flowfield_counts_the_rows_of_the_company_its_record_is_on() {
    // record/TestRecordChangeCompanyRows.al:234-285 (ChangeCompany_FlowFieldOnARecordInTheOtherCompany_CountsThatCompanysRows).
    let out = text(
        &format!(
            "{NEW_CO} HP: Record \"CC Parent\"; HC: Record \"CC Child\"; OP: Record \"CC Parent\"; OC: Record \"CC Child\"; s: Text;"
        ),
        &format!(
            "HP.Id := 1; HP.Insert(); HC.Id := 1; HC.ParentId := 1; HC.Insert(); {MAKE_CO}
             OP.ChangeCompany('OTHER'); OC.ChangeCompany('OTHER');
             OP.Id := 1; OP.Insert();
             OC.Id := 1; OC.ParentId := 1; OC.Insert(); OC.Id := 2; OC.Insert(); OC.Id := 3; OC.Insert();
             OP.Get(1); OP.CalcFields(Kids); {}
             HP.Get(1); HP.CalcFields(Kids); {}
             exit(s);",
            log("OP.Kids"),
            log("HP.Kids"),
        ),
    );
    assert_eq!(out, "3;1;");
}

#[test]
fn recordref_open_and_change_company_use_the_other_companys_rows() {
    // record/TestRecordChangeCompanyRows.al:287-330 (RecordRef_ChangeCompany_ReadsAndWritesTheOtherCompanysRows).
    let out = text(
        &format!("{NEW_CO} Home: Record \"CC Row\"; RR: RecordRef; Opened: RecordRef; s: Text;"),
        &format!(
            "Home.Id := 1; Home.V := 11; Home.Insert(); {MAKE_CO}
             RR.Open(50910); {} {}
             RR.Init(); RR.Field(1).Value := 1; RR.Field(2).Value := 77; RR.Insert(); {}
             RR.Close();
             Home.Get(1); {} {}
             Opened.Open(50910, false, 'OTHER'); {} {}
             Opened.FindFirst(); {}
             exit(s);",
            log("RR.ChangeCompany('OTHER')"),
            log("RR.Count()"),
            log("RR.Count()"),
            log("Home.V"),
            log("Home.Count()"),
            log("Opened.CurrentCompany()"),
            log("Opened.Count()"),
            log("Opened.Field(2).Value"),
        ),
    );
    assert_eq!(out, "Yes;0;1;11;1;OTHER;1;77;");
}

#[test]
fn recordref_open_with_the_session_company_name_opens_the_session_company() {
    // Base App's Change Log Management.LogModification opens `xRecRef.Open(No, false, RecRef.CurrentCompany())`;
    // the onprem tests of the User table pass through it (access-control/TestUserAuthEmailTrigger.al:187).
    let out = text(
        "Home: Record \"CC Row\"; Opened: RecordRef; s: Text;",
        &format!(
            "Home.Id := 1; Home.Insert();
             Opened.Open(50910, false, Home.CurrentCompany()); {}
             Opened.Close(); Opened.Open(50910, false, ''); {}
             exit(s);",
            log("Opened.Count()"),
            log("Opened.Count()"),
        ),
    );
    assert_eq!(out, "1;1;");
}

#[test]
fn deleting_a_company_drops_its_rows() {
    // record/TestChangeCompanyFiltersCopyDeleteContracts.al:204-212: a company deleted with triggers takes its rows;
    // a new company of the same name starts empty.
    let out = text(
        &format!("{NEW_CO} Other: Record \"CC Row\"; s: Text;"),
        &format!(
            "{MAKE_CO} Other.ChangeCompany('OTHER'); Other.Id := 1; Other.Insert(); Commit();
             Co.Get('OTHER'); Co.Delete(true); Commit();
             {MAKE_CO} Other.ChangeCompany('OTHER'); {}
             exit(s);",
            log("Other.Count()"),
        ),
    );
    assert_eq!(out, "0;");
}

// ---- record/TestChangeCompanyFiltersCopyDeleteContracts.al (BC 27.5) -------------------------------------------

#[test]
fn a_missing_company_is_false_when_the_result_is_used_and_an_error_when_it_is_not() {
    // :215-231. The session company is the first row of the Company table here, so the session name exists.
    let out = text(
        "R: Record \"CC Row\"; T: Record \"CC Row\" temporary; ok: Boolean; s: Text;",
        "ok := R.ChangeCompany('NoSuchCompany'); s += Format(ok) + Format(R.CurrentCompany() = CompanyName());
         asserterror R.ChangeCompany('NoSuchCompany');
         s += '|' + GetLastErrorText() + '|' + GetLastErrorCode();
         ok := T.ChangeCompany('NoSuchCompany'); s += '|' + Format(ok); exit(s);",
    );
    assert_eq!(
        out,
        "NoYes|The company \"NoSuchCompany\" does not exist.|DB:RecordNotFound|No"
    );
}

#[test]
fn the_company_name_is_case_sensitive() {
    // :234-246.
    let out = text(
        "R: Record \"CC Row\"; ok: Boolean; s: Text;",
        "ok := R.ChangeCompany(''); s += Format(ok) + Format(R.CurrentCompany() = CompanyName());
         ok := R.ChangeCompany(LowerCase(CompanyName())); s += '|' + Format(ok) + Format(R.CurrentCompany() = CompanyName()); exit(s);",
    );
    assert_eq!(out, "YesYes|NoYes");
}

#[test]
fn record_ref_open_on_a_missing_company_raises_the_same_error() {
    // :249-257.
    let out = text(
        "Ref: RecordRef;",
        "asserterror Ref.Open(50910, false, 'NoSuchCompany'); exit(GetLastErrorText() + '|' + GetLastErrorCode());",
    );
    assert_eq!(out, "The company \"NoSuchCompany\" does not exist.|DB:RecordNotFound");
}

#[test]
fn change_company_keeps_the_filters_the_marks_and_works_on_a_temporary_record() {
    // :25-62 (the company arm): filters, the current key and the marks stay; a temporary record keeps its rows.
    let out = text(
        &format!("{NEW_CO} R: Record \"CC Row\"; T: Record \"CC Row\" temporary; s: Text;"),
        &format!(
            "R.Id := 1; R.Insert(); R.Mark(true); R.SetFilter(Id, '>0');
             {MAKE_CO}
             {} s += R.GetFilter(Id) + '|';
             T.Id := 5; T.Insert(); {} {}
             exit(s);",
            log("R.ChangeCompany('OTHER')"),
            log("T.ChangeCompany('OTHER')"),
            log("T.Count()"),
        ),
    );
    assert_eq!(out, "Yes;>0|Yes;1;");
}

// ---- session/TestCrmTableNoConnectionContracts.al:111-118 (BC 27.5) --------------------------------------------

#[test]
fn record_ref_open_of_a_crm_table_without_a_connection_raises_the_connection_error() {
    let tables = format!(
        "{TABLES}\ntable 50920 \"CC Crm\" {{ TableType = CRM; fields {{ field(1; Id; Guid) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}\n"
    );
    let out = match run_with(
        &tables,
        "Ref: RecordRef;",
        "asserterror Ref.Open(50920); exit(GetLastErrorText() + '|' + GetLastErrorCode());",
    ) {
        Ok(Value::Text(s)) => s,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        out,
        "Table connection  for table type CRM must be registered using RegisterTableConnection or cmdlet New-NAVTableConnection before it can be used.|Argument"
    );
}

const CRM_TABLE: &str = r#"
table 50920 "CRM Entity"
{
    TableType = CRM;
    fields { field(1; Id; Guid) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
"#;

// session/TestCrmTableNoConnectionContracts.al:15-17,111-118: with no CRM table connection registered
// the first use of a record variable of a CRM table fails (Init, TableCaption, a field, IsEmpty), and
// the statement after it does not run; declaring the variable is no use.
#[test]
fn a_record_of_a_crm_table_fails_on_first_use_without_a_connection() {
    let ok = run_with(CRM_TABLE, "E: Record \"CRM Entity\";", "exit('declared');");
    assert!(matches!(ok, Ok(Value::Text(ref s)) if s == "declared"), "{ok:?}");
    for use_ in [
        "E.Init();",
        "if E.TableCaption() <> '' then;",
        "E.Name := 'x';",
        "if E.IsEmpty() then;",
    ] {
        let r = run_with(
            CRM_TABLE,
            "E: Record \"CRM Entity\"; s: Text;",
            &format!(
                "asserterror begin {use_} s := 'reached'; end; exit(s + GetLastErrorText() + '|' + GetLastErrorCode());"
            ),
        );
        assert!(
            matches!(&r, Ok(Value::Text(s)) if s == "Table connection  for table type CRM must be registered using RegisterTableConnection or cmdlet New-NAVTableConnection before it can be used.|Argument"),
            "{use_}: {r:?}"
        );
    }
}

// A temporary record of a CRM table is an in-memory table: no connection is needed. Not pinned by a corpus
// line. Base Application's `CRM Synch. Helper.ClearCache` does `Reset()` and `DeleteAll()` on temporary
// `CRM Pricelevel`, `CRM Transactioncurrency` and the like, and Microsoft's tests run it with no connection
// registered (22 tests of Tests-CRM integration start with `LibraryCRMIntegration.ResetEnvironment`).
// Listed in docs/probes-needed.md.
#[test]
fn a_temporary_record_of_a_crm_table_needs_no_connection() {
    let r = run_with(
        CRM_TABLE,
        "T: Record \"CRM Entity\" temporary;",
        "T.Reset(); T.DeleteAll(); T.Name := 'x'; T.Insert(); exit(Format(T.Count()));",
    );
    assert!(matches!(&r, Ok(Value::Text(s)) if s == "1"), "{r:?}");
    // A record that is not temporary still needs it.
    let r = run_with(
        CRM_TABLE,
        "E: Record \"CRM Entity\";",
        "asserterror E.Init(); exit(GetLastErrorCode());",
    );
    assert!(matches!(&r, Ok(Value::Text(s)) if s == "Argument"), "{r:?}");
}

// record/TestChangeCompanyFiltersCopyDeleteContracts.al:170-212: a renamed company takes its data with it
// and the old name answers false; a company deleted with triggers takes its data, and a new company of
// the same name starts empty.
#[test]
fn a_renamed_company_takes_its_data_and_a_deleted_one_drops_it() {
    let out = text(
        &format!("{NEW_CO} Data: Record \"CC Row\"; Later: Record \"CC Row\"; s: Text;"),
        &format!(
            "{MAKE_CO} Data.ChangeCompany('OTHER'); Data.Id := 1; Data.Insert(); Commit();
             Co.Get('OTHER'); Co.Rename('RENAMED'); Commit();
             {} {} {}
             Co.Get('RENAMED'); Co.Delete(true); Commit();
             {}
             Co.Name := 'RENAMED'; Co.Insert(); Commit();
             {} {}
             exit(s);",
            log("Later.ChangeCompany('OTHER')"),
            log("Later.ChangeCompany('RENAMED')"),
            log("Later.Count()"),
            log("Later.ChangeCompany('RENAMED')"),
            log("Later.ChangeCompany('RENAMED')"),
            log("Later.Count()"),
        ),
    );
    assert_eq!(out, "No;Yes;1;No;Yes;0;");
}

// :74-148: Copy and SetTable do not move the target to the source's company; GetTable takes it.
#[test]
fn copy_and_set_table_do_not_move_the_target_but_get_table_does() {
    let out = text(
        &format!(
            "{NEW_CO} Home: Record \"CC Row\"; Other: Record \"CC Row\"; Target: Record \"CC Row\"; RR: RecordRef; s: Text;"
        ),
        &format!(
            "Home.Id := 1; Home.Insert(); {MAKE_CO}
             Other.ChangeCompany('OTHER'); Other.Id := 5; Other.Insert();
             Target.Copy(Other); {}
             RR.GetTable(Other); {} {}
             RR.SetTable(Target); {} {}
             exit(s);",
            log("Target.CurrentCompany() = CompanyName()"),
            log("RR.CurrentCompany()"),
            log("RR.Count()"),
            log("Target.CurrentCompany() = CompanyName()"),
            log("Target.Count()"),
        ),
    );
    assert_eq!(out, "Yes;OTHER;1;Yes;1;");
}

// Base App's Price Source List (Tests-ERM Price Source List UT.T001): a codeunit keeps a temporary record
// global, Insert(true) fills it, and `GetList` shares its table with the caller's temporary record.
#[test]
fn a_codeunit_global_temporary_record_is_shared_with_the_callers_through_copy() {
    let tables = r#"
table 50930 "TS Src" { fields { field(1; Id; Integer) { } field(2; Nm; Text[10]) { } } keys { key(PK; Id) { } } }
codeunit 50931 "TS List"
{
    var Temp: Record "TS Src" temporary; Next: Integer;
    procedure Add(Kind: Option A,B) begin Next += 1; Temp.Init(); Temp.Id := Next; Temp.Insert(true); end;
    procedure Add(Src: Record "TS Src") begin Next += 1; Temp.TransferFields(Src); Temp.Insert(true); end;
    procedure GetList(var Dest: Record "TS Src" temporary): Boolean
    begin
        if Dest.IsTemporary then Dest.Copy(Temp, true);
        exit(Dest.FindSet());
    end;
}
"#;
    let out = text_in(
        tables,
        "L: Codeunit \"TS List\"; T: Record \"TS Src\" temporary;",
        "L.Add(0); L.Add(1); L.GetList(T); exit(Format(T.Count()));",
    );
    assert_eq!(out, "2");
}

fn text_in(tables: &str, vars: &str, body: &str) -> String {
    match run_with(tables, vars, body) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

// Microsoft's Assert.RecordCount(Variant, N) counts a caller's temporary record through a Variant parameter (Tests-ERM
// Price Source List UT.T001 expects 2 on BC). A by-value copy of a temporary record is an empty buffer
// (record/TestTempTableContracts.al:141-148), but a Variant parameter keeps the caller's table
// (recordref/PWRecordMisc.al:TempRecord_PassedToVariantParameter_ByValue).
#[test]
fn a_temporary_record_handed_to_a_variant_parameter_keeps_its_rows() {
    let tables = r#"
table 50930 "TS Src" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50932 "TS Assert" { procedure RecordCount(RecVariant: Variant): Integer var RR: RecordRef; begin RR.GetTable(RecVariant); exit(RR.Count()); end; }
"#;
    let out = text_in(
        tables,
        "A: Codeunit \"TS Assert\"; T: Record \"TS Src\" temporary;",
        "exit(Format(A.RecordCount(T)));",
    );
    assert_eq!(out, "0");
    let out = text_in(
        tables,
        "A: Codeunit \"TS Assert\"; T: Record \"TS Src\" temporary;",
        "T.Id := 1; T.Insert(); T.Id := 2; T.Insert(); exit(Format(A.RecordCount(T)) + Format(T.Count()));",
    );
    assert_eq!(out, "22");
}

// ---- Database.CopyCompany: lt-o-small-builtins/OSmallBuiltins.al (BC 27, 28, 29) ---------------------------------

#[test]
fn copy_company_from_a_missing_source_fails_and_makes_no_company() {
    // OSmallBuiltins.al:98-108: RecordNotFound on the Company table, and the destination does not exist after it.
    let out = text(
        "Co: Record Company;",
        "asserterror CopyCompany('LT O No Such Company', 'LT O Copy Target');
         exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Co.Get('LT O Copy Target')));",
    );
    assert_eq!(
        out,
        "The Company does not exist. Identification fields and values: Name='LT O No Such Company'|DB:RecordNotFound|No"
    );
}

#[test]
fn copy_company_to_an_existing_destination_fails() {
    // OSmallBuiltins.al:102-110: copying the session company onto itself is RecordExists on the Company table.
    let out = text(
        "",
        "asserterror CopyCompany(CompanyName(), CompanyName());
         exit(GetLastErrorText() + '|' + GetLastErrorCode());",
    );
    let company = text("", "exit(CompanyName());");
    assert_eq!(
        out,
        format!(
            "The record in table Company already exists. Identification fields and values: Name='{company}'|DB:RecordExists"
        )
    );
}

#[test]
fn copy_company_with_the_result_used_still_raises_for_a_missing_source() {
    // Y22CopyCompany.al Y22_CopyCompany_ReturnValueMissingSource (SaaS BC 29): `Ok :=` does not catch the error.
    let out = text(
        "ok: Boolean; Co: Record Company;",
        "asserterror ok := CopyCompany('LT O No Such Company', 'X');
         exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Co.Get('X')));",
    );
    assert_eq!(
        out,
        "The Company does not exist. Identification fields and values: Name='LT O No Such Company'|DB:RecordNotFound|No"
    );
}

#[test]
fn copy_company_with_the_result_used_and_an_existing_destination_is_refused() {
    // Only the statement form of this error is pinned (OSmallBuiltins.al:102, :129).
    let r = run_with(
        TABLES,
        "ok: Boolean;",
        "ok := CopyCompany(CompanyName(), CompanyName());",
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

// ---- Database.CopyCompany that succeeds: session/lt-y22-copy-company/Y22CopyCompany.al (corpus writer Y22, BC 29) ------

const COPY_OBJECTS: &str = r#"
table 50930 "CP Row"
{
    fields { field(1; No; Integer) { } field(2; Name; Text[30]) { } field(3; ByTrigger; Boolean) { } }
    keys { key(PK; No) { } }
    trigger OnInsert()
    var
        L: Record "CC Tenant";
    begin
        ByTrigger := true;
        L.Id := 1000 + No;
        L.Insert();
    end;
}
codeunit 50931 "CP Subs"
{
    [EventSubscriber(ObjectType::Table, Database::"CP Row", 'OnAfterInsertEvent', '', false, false)]
    local procedure RowAfterInsert(var Rec: Record "CP Row")
    var
        L: Record "CC Tenant";
    begin
        L.Id := 2000 + Rec.No;
        L.Insert();
    end;

    [EventSubscriber(ObjectType::Table, Database::Company, 'OnAfterInsertEvent', '', false, false)]
    local procedure CompanyAfterInsert(var Rec: Record Company)
    var
        L: Record "CC Tenant";
    begin
        L.Id := 3000 + StrLen(Rec.Name);
        L.Insert();
    end;
}
"#;

const COPY_VARS: &str = "Src: Record Company; Dst: Record Company; Row: Record \"CP Row\"; Other: Record \"CP Row\"; L: Record \"CC Tenant\"; s: Text;";

const COPY_SETUP: &str = "Src.Name := 'Source'; Src.\"Display Name\" := 'Disp'; Src.\"Evaluation Company\" := true;
     Src.\"Business Profile Id\" := 'Prof'; Src.Insert();
     Row.ChangeCompany('Source');
     Row.No := 1; Row.Name := 'r1'; Row.Insert(true);
     Row.No := 2; Row.Name := 'r2'; Row.Insert(true);
     Commit();";

fn copy_text(extra: &str, vars: &str, body: &str) -> String {
    let tables = format!("{TABLES}{COPY_OBJECTS}{extra}");
    match run_with(&tables, &format!("{COPY_VARS}{vars}"), &format!("{COPY_SETUP}{body}")) {
        Ok(Value::Text(s)) | Ok(Value::Code(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn copy_company_makes_the_company_and_copies_its_rows_as_they_are() {
    // Y22CopyCompany.al Y22_CopyCompany_CopiesCompanyAndRows: the rows come over with the same values, SystemId and
    // stamps; no trigger or table event of the copied rows runs; only the Company's OnAfterInsertEvent, once. The new
    // Company row: Display Name and Business Profile Id empty, Evaluation Company copied, a new Id and SystemId.
    let out = copy_text(
        "",
        "",
        "s := Format(L.Count()) + '|';
         CopyCompany('Source', 'Dst');
         s += Format(L.Count()) + Format(L.Get(3003)) + '|';
         Src.Get('Source');
         s += Format(Dst.Get('Dst')) + '|' + Dst.Name + '|' + Dst.\"Display Name\" + '|' + Format(Dst.\"Evaluation Company\") + '|'
             + Dst.\"Business Profile Id\" + '|' + Format(IsNullGuid(Dst.Id)) + Format(Dst.Id = Src.Id) + Format(Dst.SystemId = Src.SystemId) + '|';
         Other.ChangeCompany('Dst');
         s += Format(Other.Count()) + '|';
         Other.Get(2); Row.Get(2);
         s += Other.Name + Format(Other.ByTrigger) + Format(Other.SystemId = Row.SystemId) + Format(Other.SystemCreatedAt = Row.SystemCreatedAt) + '|';
         s += Format(Row.Count());
         exit(s);",
    );
    assert_eq!(out, "5|6Yes|Yes|Dst||Yes||NoNoNo|2|r2YesYesYes|2");
}

#[test]
fn copy_company_rows_are_apart_from_the_source_after_the_copy() {
    // Y22CopyCompany.al: the copy is a company of its own; a write there leaves the source alone.
    let out = copy_text(
        "",
        "",
        "CopyCompany('Source', 'Dst');
         Other.ChangeCompany('Dst'); Other.No := 3; Other.Insert();
         exit(Format(Other.Count()) + Format(Row.Count()));",
    );
    assert_eq!(out, "32");
}

#[test]
fn copy_company_refuses_a_company_subscriber_that_reads_run_trigger() {
    // Y22CopyCompany.al logs the Company OnAfterInsertEvent but not its RunTrigger.
    let extra = r#"
codeunit 50932 "CP Run Trigger"
{
    [EventSubscriber(ObjectType::Table, Database::Company, 'OnAfterInsertEvent', '', false, false)]
    local procedure CompanyAfterInsert(var Rec: Record Company; RunTrigger: Boolean)
    var
        L: Record "CC Tenant";
    begin
        if RunTrigger then
            L.Id := 4000
        else
            L.Id := 4001;
        L.Insert();
    end;
}
"#;
    let tables = format!("{TABLES}{COPY_OBJECTS}{extra}");
    let r = run_with(
        &tables,
        COPY_VARS,
        &format!("{COPY_SETUP} CopyCompany('Source', 'Dst');"),
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn copy_company_returns_true_when_the_result_is_used() {
    // Y22CopyCompany.al Y22_CopyCompany_ReturnValue: Yes, the company is made, no last error.
    let out = copy_text(
        "",
        "ok: Boolean;",
        "ClearLastError(); ok := CopyCompany('Source', 'Dst');
         exit(Format(ok) + '|' + Format(Dst.Get('Dst')) + '|' + GetLastErrorText());",
    );
    assert_eq!(out, "Yes|Yes|");
}

#[test]
fn copy_company_inside_a_write_transaction_is_rolled_back_with_it() {
    // Y22CopyCompany.al Y22_CopyCompany_InWriteTransaction: the copy is part of the session's transaction.
    let out = copy_text(
        "",
        "",
        "L.Id := 7; L.Insert();
         CopyCompany('Source', 'Dst');
         s := Format(Dst.Get('Dst')) + '|' + Format(L.Get(7)) + '|';
         asserterror Error('roll back');
         exit(s + Format(Dst.Get('Dst')) + '|' + Format(L.Get(7)));",
    );
    assert_eq!(out, "Yes|Yes|No|No");
}

#[test]
fn copy_company_of_a_source_with_uncommitted_rows_is_refused() {
    // Y22CopyCompany.al writes only a shared table before the copy in the transaction: whether the copy takes the
    // source's own uncommitted rows is not pinned.
    let tables = format!("{TABLES}{COPY_OBJECTS}");
    let r = run_with(
        &tables,
        COPY_VARS,
        &format!("{COPY_SETUP} Row.No := 9; Row.Insert(); CopyCompany('Source', 'Dst');"),
    );
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn copy_company_destination_name_errors() {
    // Y22CopyCompany.al Y22_CopyCompany_DestinationNames: too long, empty, the source's name in another case.
    let out = copy_text(
        "",
        "",
        "asserterror CopyCompany('Source', 'LT Y22 C Destination Name 31ChX');
         s := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(Dst.Get('LT Y22 C Destination Name 31Ch')) + '#';
         asserterror CopyCompany('Source', '');
         s += GetLastErrorText() + '|' + GetLastErrorCode() + '#';
         asserterror CopyCompany('Source', 'source');
         exit(s + GetLastErrorText() + '|' + GetLastErrorCode());",
    );
    assert_eq!(
        out,
        "The length of the string is 31, but it must be less than or equal to 30 characters. Value: LT Y22 C Destination Name 31ChX|StringLengthExceeded|No#\
         Company name cannot be empty or only consist of spaces.|DB:Ok#\
         Cannot rename or create a new company with the name (source) because a company with the same name already exists|DB:Ok"
    );
}

const BASELINE_CODE: &str = r#"
codeunit 50100 C
{
    procedure Install()
    var
        Co: Record Company;
        Row: Record "CP Row";
    begin
        Co.Name := CompanyName();
        if Co.Insert() then;
        Row.No := 1;
        if Row.Insert() then;
    end;

    procedure InstallAndCopy()
    begin
        Install();
        Commit();
        CopyCompany(CompanyName(), 'Copy');
    end;

    procedure ReadFresh(): Text
    var
        Co: Record Company;
        Other: Record "CP Row";
    begin
        Co.Name := 'Fresh'; Co.Insert(); Commit();
        Other.ChangeCompany('Fresh');
        exit(Format(Other.Count()));
    end;

    procedure CopyAndRead(): Text
    begin
        CopyCompany(CompanyName(), 'Copy');
        exit(ReadCopy());
    end;

    procedure ReadCopy(): Text
    var
        Other: Record "CP Row";
    begin
        Other.ChangeCompany('Copy');
        exit(Format(Other.Count()));
    end;
}
"#;

fn baseline_built() -> al_testkit::Built {
    al_testkit::Built::from_source(&format!("{TABLES}{COPY_OBJECTS}{BASELINE_CODE}"))
}

#[test]
fn a_copy_of_the_session_company_holds_the_rows_install_code_wrote() {
    // A new company would hold the install rows too, which the runtime made only for the session company: refused. A copy
    // holds the session company's rows as they are (Y22CopyCompany.al:Y22_CopyCompany_CopiesCompanyAndRows): answered.
    let mut b = baseline_built();
    b.run_in("C", "Install").unwrap();
    let baseline = b.session.capture_baseline();
    b.session.restore_baseline(&baseline);
    let fresh = b.run_in("C", "ReadFresh");
    assert!(matches!(fresh, Err(Signal::Unsupported(_))), "{fresh:?}");
    b.session.restore_baseline(&baseline);
    assert_eq!(al_testkit::text(b.run_in("C", "CopyAndRead")), "1");
}

#[test]
fn a_copy_made_by_install_code_stays_a_copy_through_the_baseline_bytes() {
    let mut b = baseline_built();
    b.run_in("C", "InstallAndCopy").unwrap();
    let bytes = b.session.capture_baseline().to_bytes().expect("plain rows");
    let baseline = al_vm::Baseline::from_bytes(&bytes).expect("the same layout");
    let mut fresh = baseline_built();
    fresh.session.restore_baseline(&baseline);
    assert_eq!(al_testkit::text(fresh.run_in("C", "ReadCopy")), "1");
}
