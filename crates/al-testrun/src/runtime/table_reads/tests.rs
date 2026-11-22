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
         R.SetRange(Name, 'A', 'B'); R.SetFilter(Name, '%1|%2', 'A', 'C'); if R.Get('A') then; if R.Find('-') then; \
         if R.Find('+') then; if R.Find('=') then; if R.FindFirst() then; if R.FindLast() then; \
         X := R.Count(); X := R.CountApprox(); if R.FindSet() then repeat Name := R.Name; until R.Next() = 0; \
         if R.FindSet(true) then; X := R.Next(-1); R.Init(); R.SetLoadFields(Name); R.LockTable(); \
         Name := R.GetFilters(); if R.HasFilter() then; end;"
    ));
}

// Review of PR 8275 (MED 1): argument forms of a measured method that the probe did not call keep the full reach. The
// probe calls Find with '-', '+' and '=', Next with no argument and with -1, FindSet with none and with true, SetRange
// with a range and SetFilter with two values on the app table (TiReadEventTests.al:57-59, :66, :69, :74, :125, :127), and
// FindSet and Next with no argument and SetRange with one value on Company (TiCompanyReadEventTests.al:43-48).
#[test]
fn an_unmeasured_argument_form_is_not_only_a_read() {
    for body in [
        "procedure P() var R: Record Co; begin if R.Find('>') then; end;",
        "procedure P() var R: Record Co; begin if R.Find() then; end;",
        "procedure P() var R: Record Co; X: Integer; begin X := R.Next(2); end;",
        "procedure P() var R: Record Co; X: Integer; begin X := R.Next(1); end;",
        "procedure P() var R: Record Co; begin if R.FindSet(false) then; end;",
        "procedure P() var R: Record Co; begin R.SetRange(N, 1); end;",
        "procedure P() var R: Record Co; begin R.SetFilter(Name, 'A*'); end;",
    ] {
        assert!(!ask(body), "{body}");
    }
    for body in [
        "procedure P() var Company: Record Company; begin if Company.FindSet(true) then; end;",
        "procedure P() var Company: Record Company; X: Integer; begin X := Company.Next(-1); end;",
        "procedure P() var Company: Record Company; begin Company.SetRange(Name, 'A', 'B'); end;",
    ] {
        assert!(!ask_company(body), "{body}");
    }
}

// Review of PR 8275 (MED 2): the probe's table has only Normal fields and its records are locals of the routine. A table
// with a FlowField, and a record that is a global, a parameter, the return value or an array keep the full reach.
#[test]
fn an_unmeasured_table_or_slot_kind_is_not_only_a_read() {
    let flow = "table 50300 Co { fields { field(1; Name; Text[30]) { } field(2; N; Integer) { FieldClass = FlowField; \
        CalcFormula = count(Co); } } keys { key(PK; Name) { } } }\n";
    assert!(!ask_with(
        flow,
        "procedure P() var R: Record Co; begin if R.IsEmpty() then; end;",
        "Co"
    ));
    for body in [
        "var G: Record Co; procedure P() begin if G.IsEmpty() then exit; end;",
        "procedure P(R: Record Co) begin if R.IsEmpty() then exit; end;",
        "procedure P() R: Record Co; begin if R.Get('A') then; end;",
        "procedure P() var A: array[2] of Record Co; R: Record Co; begin if R.IsEmpty() then; end;",
    ] {
        assert!(!ask(body), "{body}");
    }
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

/// [`kept`] of procedure P of codeunit C (`codeunit`: its body), with [`TABLE`] and `objects` declared: `None` for
/// `Kept::All`, else whether it keeps table Co.
fn kept_co(objects: &str, codeunit: &str) -> Option<bool> {
    let l = lowered(&format!("{TABLE}{objects}codeunit 50301 C {{ {codeunit} }}"));
    let engine = al_vm::Engine::new(l.program.clone(), Vec::new());
    let objs = &l.program.objects;
    let t = objs.heads().iter().position(|o| o.name == "Co").unwrap();
    let o = objs.heads().iter().position(|o| o.name == "C").unwrap();
    let r = objs[o].routines.iter().position(|r| r.name == "P").unwrap();
    match kept(&engine, &l, o, r) {
        Kept::All => None,
        Kept::Tables(k) => Some(k.contains(&t)),
    }
}

// Review of PR 8275 (HIGH 1), one fact at a time: a routine that only reads Co keeps nothing; a write, a RecordRef
// opened on Co (also without a write) and a record of Co that leaves the routine keep Co; a run by id, a RecordRef
// opened on a table known only at run time, a RecordRef or FieldRef from outside and a record call on a returned
// record keep every table.
#[test]
fn what_a_routine_keeps_of_the_tables_it_reaches() {
    assert_eq!(
        kept_co("", "procedure P() var R: Record Co; begin if R.IsEmpty() then; end;"),
        Some(false)
    );
    assert_eq!(
        kept_co("", "procedure P() var R: Record Co; begin R.Insert(); end;"),
        Some(true)
    );
    assert_eq!(
        kept_co(
            "",
            "procedure P() var RecRef: RecordRef; begin RecRef.Open(Database::Co); end;"
        ),
        Some(true)
    );
    assert_eq!(
        kept_co("", "procedure P() R: Record Co; begin if R.Get('A') then; end;"),
        Some(true)
    );
    let w = "codeunit 50305 W { trigger OnRun() begin end; }\n";
    assert_eq!(kept_co(w, "procedure P() begin Codeunit.Run(Codeunit::W); end;"), None);
    assert_eq!(
        kept_co("", "procedure P() var Id: Integer; begin Codeunit.Run(Id); end;"),
        None
    );
    assert_eq!(
        kept_co(
            "",
            "procedure P() var RecRef: RecordRef; Id: Integer; begin RecRef.Open(Id); end;"
        ),
        None
    );
    assert_eq!(kept_co("", "procedure P(RecRef: RecordRef) begin end;"), None);
    assert_eq!(kept_co("", "procedure P(var FldRef: FieldRef) begin end;"), None);
    assert_eq!(kept_co("", "var G: RecordRef; procedure P() begin end;"), None);
    assert_eq!(
        kept_co(
            "",
            "procedure P() begin if Q().IsEmpty() then; end; procedure Q() R: Record Co; begin end;"
        ),
        None
    );
}

// The walk decides per table over every routine it visits: a routine visited after one that only reads Co and keeps
// Co (here a callee that opens a RecordRef on Co) brings Co's delete subscriber back into the reach.
#[test]
fn a_later_routine_that_keeps_the_table_brings_its_subscribers_back() {
    let l = lowered(&format!(
        "{TABLE}codeunit 50303 Subs {{
            [EventSubscriber(ObjectType::Table, Database::Co, 'OnAfterDeleteEvent', '', false, false)]
            local procedure Deleted(var Rec: Record Co) begin end; }}
        codeunit 50304 C {{ procedure Reads() var R: Record Co; begin if R.IsEmpty() then; Opens(); end;
            local procedure Opens() var RecRef: RecordRef; begin RecRef.Open(Database::Co); end; }}"
    ));
    let engine = al_vm::Engine::new(l.program.clone(), Vec::new());
    let objs = &l.program.objects;
    let at = |object: &str, routine: &str| {
        let o = objs.heads().iter().position(|o| o.name == object).unwrap();
        (o, objs[o].routines.iter().position(|r| r.name == routine).unwrap())
    };
    let reach = super::super::blocked::subscribers_reached_reading(&engine, &l, &[at("C", "Reads")]);
    assert!(reach.contains(&at("Subs", "Deleted")), "{reach:?}");
}

// The platform adds its system fields to every table, the probe's table 73600 too (TiReadEventTests.al:47-92 ran
// on BC with them), among them the user-name FlowFields at 2000001005-2000001008 (al_ir::user_name_field_no). They
// are not fields the table declares, so they do not make its reads unmeasured.
#[test]
fn the_platform_s_system_flowfields_keep_a_table_measured() {
    let mut l = lowered(TABLE);
    let t = l.program.objects.heads().iter().position(|o| o.name == "Co").unwrap();
    let table = std::sync::Arc::make_mut(&mut l.program).objects[t]
        .table
        .as_mut()
        .unwrap();
    let mut user_name = table.fields[0].clone();
    user_name.id = 2_000_001_005;
    user_name.name = "SystemCreatedByUserName".into();
    user_name.properties.push(("FieldClass".into(), "FlowField".into()));
    table.fields.push(user_name);
    assert!(measured_reads(&l, t).is_some());
}
