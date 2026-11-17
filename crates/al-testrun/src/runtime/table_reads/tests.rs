use super::*;

fn lowered(src: &str) -> Lowered {
    al_sema::lower::compile(
        vec![al_sema::workspace::AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    )
    .1
}

const TABLE: &str =
    "table 50300 Co { fields { field(1; Name; Text[30]) { } field(2; N; Integer) { } } keys { key(PK; Name) { } }
    [IntegrationEvent(false, false)] procedure OnCo() begin end; }\n";

/// `reads_only` of procedure P of codeunit C (`codeunit`: its body) for table `table`, with `objects` declared too.
fn ask_with(objects: &str, codeunit: &str, table: &str) -> bool {
    let l = lowered(&format!("{objects}codeunit 50301 C {{ {codeunit} }}"));
    let objs = &l.program.objects;
    let t = objs.heads().iter().position(|o| o.name == table).unwrap();
    let o = objs.heads().iter().position(|o| o.name == "C").unwrap();
    let r = objs[o].routines.iter().position(|r| r.name == "P").unwrap();
    reads_only(&l, o, r, t)
}

fn ask(body: &str) -> bool {
    ask_with(TABLE, body, "Co")
}

/// The platform table Company as the symbols declare it (its id is what `measured_reads` asks).
const COMPANY_TABLE: &str = "table 2000000006 Company { DataPerCompany = false;
    fields { field(1; Name; Text[30]) { } field(3; \"Display Name\"; Text[250]) { } } keys { key(PK; Name) { } } }\n";

fn ask_company(body: &str) -> bool {
    ask_with(COMPANY_TABLE, body, "Company")
}

// TiReadEventTests.al:47-92, :95-113, :116-169 of corpus PR 1431: the reads of an app table that raise no table event.
#[test]
fn the_measured_reads_of_an_app_table_and_a_field_read_only_read() {
    assert!(ask(
        "procedure P() var R: Record Co; begin if R.IsEmpty() then exit; end;"
    ));
    assert!(ask(
        "procedure P() var R: Record Co; X: Integer; Name: Text; begin R.Reset(); R.SetCurrentKey(Name); R.SetAscending(Name, false); \
         R.SetRange(N, 1); R.SetFilter(Name, 'A*'); if R.Get('A') then; if R.Find('-') then; if R.FindFirst() then; if R.FindLast() then; \
         X := R.Count(); X := R.CountApprox(); if R.FindSet() then repeat Name := R.Name; until R.Next() = 0; \
         if R.FindSet(true) then; X := R.Next(-1); R.Init(); R.SetLoadFields(Name); R.LockTable(); \
         Name := R.GetFilters(); if R.HasFilter() then; end;"
    ));
    assert!(ask(
        "var G: Record Co; procedure P() begin if G.IsEmpty() then exit; end;"
    ));
}

// TiCompanyReadEventTests.al:26-70 of corpus PR 1431: the reads of Company that raise no table event.
#[test]
fn the_measured_reads_of_company_only_read() {
    assert!(ask_company(
        "procedure P() var Company: Record Company; begin if Company.IsEmpty() then exit; end;"
    ));
    assert!(ask_company(
        "procedure P() var Company: Record Company; X: Integer; T: Text; begin X := Company.Count(); if Company.FindFirst() then; \
         if Company.Get(CompanyName()) then; if Company.FindSet() then repeat T := Company.\"Display Name\"; until Company.Next() = 0; \
         Company.SetRange(Name, CompanyName()); Company.Reset(); end;"
    ));
}

#[test]
fn a_write_or_any_other_use_is_not_only_a_read() {
    for body in [
        "procedure P() var R: Record Co; begin R.Name := 'A'; R.Insert(); end;",
        "procedure P() var R: Record Co; begin if R.Get('A') then R.Modify(); end;",
        "procedure P() var R: Record Co; begin if R.Get('A') then R.Delete(); end;",
        "procedure P() var R: Record Co; begin R.DeleteAll(); end;",
        "procedure P() var R: Record Co; begin R.ModifyAll(N, 1); end;",
        "procedure P() var R: Record Co; begin if R.Get('A') then R.Rename('B'); end;",
        "procedure P() var R: Record Co; begin R.Validate(N, 1); end;",
        "procedure P() var R: Record Co; begin R.Name := 'A'; if R.IsEmpty() then; end;",
        "procedure P() var R: Record Co; S: Record Co; begin R.Copy(S); end;",
        "procedure P() var R: Record Co; S: Record Co; begin R.TransferFields(S); end;",
        "procedure P() var R: Record Co; begin if R.FindSet(true, true) then; end;",
        "procedure P() var R: Record Co; begin R.CalcFields(N); end;",
        "procedure P() var R: Record Co; begin Q(R); end; procedure Q(var X: Record Co) begin end;",
        "procedure P() var R: Record Co; begin Q(R); end; procedure Q(X: Record Co) begin end;",
        "procedure P() var R: Record Co; S: Record Co; begin S := R; end;",
        "procedure P() var R: Record Co; begin if R.IsEmpty() then; G.Insert(); end; var G: Record Co;",
    ] {
        assert!(!ask(body), "{body}");
    }
}

// Shapes the probe did not measure keep the full reach: a temporary record, another table type, a table with a trigger
// or a table extension, a codeunit with a TableNo, a Company method the probe did not call.
#[test]
fn an_unmeasured_table_or_record_shape_is_not_asked() {
    assert!(!ask(
        "procedure P() var R: Record Co temporary; begin if R.IsEmpty() then; end;"
    ));
    assert!(!ask(
        "var G: Record Co temporary; procedure P() var R: Record Co; begin if R.IsEmpty() then; end;"
    ));
    let typed = "table 50300 Co { TableType = Temporary; fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }\n";
    assert!(!ask_with(
        typed,
        "procedure P() var R: Record Co; begin if R.IsEmpty() then; end;",
        "Co"
    ));
    let normal =
        "table 50300 Co { TableType = Normal; fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } }\n";
    assert!(!ask_with(
        normal,
        "procedure P() var R: Record Co; begin if R.IsEmpty() then; end;",
        "Co"
    ));
    let trigger = "table 50300 Co { fields { field(1; Name; Text[30]) { } } keys { key(PK; Name) { } } trigger OnInsert() begin end; }\n";
    assert!(!ask_with(
        trigger,
        "procedure P() var R: Record Co; begin if R.IsEmpty() then; end;",
        "Co"
    ));
    let extended =
        format!("{TABLE}tableextension 50310 CoX extends Co {{ fields {{ field(50310; X; Integer) {{ }} }} }}\n");
    assert!(!ask_with(
        &extended,
        "procedure P() var R: Record Co; begin if R.IsEmpty() then; end;",
        "Co"
    ));
    assert!(!ask_company(
        "procedure P() var Company: Record Company; begin Company.Init(); end;"
    ));
    assert!(!ask_company(
        "procedure P() var Company: Record Company; X: Integer; begin X := Company.CountApprox(); end;"
    ));
    let user = "table 2000000120 User { DataPerCompany = false; fields { field(1; \"User Security ID\"; Guid) { } } keys { key(PK; \"User Security ID\") { } } }\n";
    assert!(!ask_with(
        user,
        "procedure P() var U: Record User; begin if U.IsEmpty() then; end;",
        "User"
    ));
    let l = lowered(&format!(
        "{TABLE}codeunit 50302 T {{ TableNo = Co; trigger OnRun() begin end; procedure P() var R: Record Co; begin if R.IsEmpty() then; end; }}"
    ));
    let objs = &l.program.objects;
    let t = objs.heads().iter().position(|o| o.name == "Co").unwrap();
    let o = objs.heads().iter().position(|o| o.name == "T").unwrap();
    let r = objs[o].routines.iter().position(|r| r.name == "P").unwrap();
    assert!(!reads_only(&l, o, r, t));
}

/// The reporter's mechanism (al-runner issue 8238): a routine that only calls IsEmpty on a table no longer reaches the
/// table's delete subscriber; a routine that deletes still does, and the table's integration event subscriber stays.
/// Main's reach (`subscribers_reached_from`) is unchanged.
#[test]
fn a_routine_that_only_reads_reaches_no_table_event_subscriber() {
    let l = lowered(&format!(
        "{TABLE}codeunit 50303 Subs {{
            [EventSubscriber(ObjectType::Table, Database::Co, 'OnAfterDeleteEvent', '', false, false)]
            local procedure Deleted(var Rec: Record Co) begin end;
            [EventSubscriber(ObjectType::Table, Database::Co, 'OnCo', '', false, false)]
            local procedure Own() begin end; }}
        codeunit 50304 C {{ procedure Reads() var R: Record Co; begin if R.IsEmpty() then; end;
            procedure Deletes() var R: Record Co; begin R.DeleteAll(); end; }}"
    ));
    let engine = al_vm::Engine::new(l.program.clone(), Vec::new());
    let objs = &l.program.objects;
    let at = |object: &str, routine: &str| {
        let o = objs.heads().iter().position(|o| o.name == object).unwrap();
        (o, objs[o].routines.iter().position(|r| r.name == routine).unwrap())
    };
    let narrowed = |root| super::super::blocked::subscribers_reached_reading(&engine, &l, &[root]);
    let reads = narrowed(at("C", "Reads"));
    assert!(!reads.contains(&at("Subs", "Deleted")), "{reads:?}");
    assert!(reads.contains(&at("Subs", "Own")), "{reads:?}");
    let deletes = narrowed(at("C", "Deletes"));
    assert!(deletes.contains(&at("Subs", "Deleted")), "{deletes:?}");
    let main = super::super::blocked::subscribers_reached_from(&engine, &l, &[at("C", "Reads")]);
    assert!(
        main.contains(&at("Subs", "Deleted")),
        "main's reach is unchanged: {main:?}"
    );
}
