//! Records end to end: AL source → compile → run against the in-memory store.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_testkit::text;
use al_vm::{Engine, Session};

const TABLE: &str = r#"
table 50900 "ALT T"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { InitValue = 5; trigger OnValidate() begin Log += 'V' + Format(xRec.Amount) + '>' + Format(Amount); end; }
        field(3; Grp; Integer) { }
        field(4; Log; Text[250]) { }
    }
    keys { key(PK; "No.") { } key(ByGrp; Grp, "No.") { } }

    trigger OnInsert() begin Log += 'I'; end;
    trigger OnModify() begin Log += 'M'; end;
    trigger OnDelete() begin if "No." = 'X' then Error('cannot delete X'); end;

    procedure Bump(N: Decimal) begin Amount += N; end;
}
"#;

fn run(body: &str, routine: &str) -> Result<Value, Signal> {
    let src = format!("{TABLE}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("{routine} is blocked: {r}");
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn insert_get_count_deleteall() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.\"No.\" := 'a'; R.Insert();
            R.\"No.\" := 'b'; R.Insert();
            Clear(R);
            R.Get('A');
            exit(Format(R.Count()) + R.\"No.\" + Format(R.IsEmpty()));
        end;",
        "F",
    );
    assert_eq!(text(r), "2ANo");
}

#[test]
fn init_applies_initvalue_and_keeps_the_primary_key() {
    // Doc (Record.Init): primary key fields are not initialized.
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin R.\"No.\" := 'K'; R.Amount := 9; R.Init(); exit(R.\"No.\" + Format(R.Amount)); end;",
        "F",
    );
    assert_eq!(text(r), "K5");
}

#[test]
fn duplicate_insert_raises_or_returns_false() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin R.\"No.\" := 'a'; R.Insert(); asserterror begin R.Insert(); R.Count(); end; exit(GetLastErrorText()); end;",
        "F",
    );
    // The failure of an unguarded duplicate Insert comes out at the next access of the table.
    assert!(text(r).contains("already exists"));
    let r = run(
        "procedure F(): Boolean var R: Record \"ALT T\"; begin R.\"No.\" := 'a'; R.Insert(); exit(R.Insert()); end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Boolean(false))));
}

#[test]
fn get_missing_raises_or_returns_false() {
    let r = run(
        "procedure F(): Boolean var R: Record \"ALT T\"; begin exit(R.Get('nope')); end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Boolean(false))));
    let r = run("procedure F() var R: Record \"ALT T\"; begin R.Get('nope'); end;", "F");
    assert!(matches!(r, Err(Signal::Error(e)) if e.message.contains("does not exist")));
}

#[test]
fn modify_and_delete() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.\"No.\" := 'a'; R.Insert();
            R.Amount := 7; R.Modify();
            Clear(R); R.Get('A');
            R.Delete();
            exit(Format(R.Get('A')) + Format(R.Amount));
        end;",
        "F",
    );
    // Get fails (returns No) and leaves the buffer as it was (Amount 7).
    assert_eq!(text(r), "No7");
}

#[test]
fn find_and_next_follow_the_current_key() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; S: Text; begin
            R.\"No.\" := 'C'; R.Grp := 1; R.Insert();
            R.\"No.\" := 'A'; R.Grp := 2; R.Insert();
            R.\"No.\" := 'B'; R.Grp := 1; R.Insert();
            if R.FindSet() then repeat S += R.\"No.\"; until R.Next() = 0;
            S += '|';
            R.SetCurrentKey(Grp);
            if R.FindSet() then repeat S += R.\"No.\"; until R.Next() = 0;
            S += '|';
            if R.FindLast() then S += R.\"No.\";
            exit(S);
        end;",
        "F",
    );
    assert_eq!(text(r), "ABC|BCA|A");
}

#[test]
fn setrange_filters_and_reset() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Grp := 1; R.Insert();
            R.\"No.\" := 'B'; R.Grp := 2; R.Insert();
            R.\"No.\" := 'C'; R.Grp := 3; R.Insert();
            R.SetRange(Grp, 2, 3);
            exit(Format(R.Count()) + Format(R.FindFirst()) + R.\"No.\");
        end;",
        "F",
    );
    assert_eq!(text(r), "2YesB");
    let r = run(
        "procedure F(): Integer var R: Record \"ALT T\"; begin R.\"No.\" := 'A'; R.Insert(); R.SetRange(Grp, 9); R.Reset(); exit(R.Count()); end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Integer(1))));
}

#[test]
fn deleteall_respects_filters() {
    let r = run(
        "procedure F(): Integer var R: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Grp := 1; R.Insert();
            R.\"No.\" := 'B'; R.Grp := 2; R.Insert();
            R.SetRange(Grp, 2); R.DeleteAll(); R.Reset();
            exit(R.Count());
        end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Integer(1))));
}

#[test]
fn triggers_run_only_when_asked() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Insert(true);
            R.Modify(true);
            R.\"No.\" := 'B'; R.Insert(false);
            exit(R.Log);
        end;",
        "F",
    );
    // OnInsert then OnModify ran on A's buffer; B's insert ran no trigger.
    assert_eq!(text(r), "IM");
}

#[test]
fn trigger_errors_propagate() {
    let r = run(
        "procedure F() var R: Record \"ALT T\"; begin R.\"No.\" := 'X'; R.Insert(); R.Delete(true); end;",
        "F",
    );
    assert!(matches!(r, Err(Signal::Error(e)) if e.message == "cannot delete X"));
}

#[test]
fn validate_runs_onvalidate_with_xrec() {
    // xRec.Amount is 5: InitValue applies at variable declaration
    // (record/TestInitValueContracts.al:204).
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin R.Validate(Amount, 8); exit(R.Log); end;",
        "F",
    );
    assert_eq!(text(r), "V5>8");
}

#[test]
fn assignment_copies_fields_but_not_filters() {
    let r = run(
        "procedure F(): Text var A: Record \"ALT T\"; B: Record \"ALT T\"; begin
            A.\"No.\" := 'A'; A.Insert();
            A.\"No.\" := 'B'; A.Insert();
            A.SetRange(\"No.\", 'A');
            B := A;
            B.\"No.\" := 'Z';
            exit(A.\"No.\" + B.\"No.\" + Format(B.Count()));
        end;",
        "F",
    );
    assert_eq!(text(r), "BZ2");
}

#[test]
fn record_parameters_by_value_and_by_var() {
    let r = run(
        "local procedure ByVal(R: Record \"ALT T\") begin R.\"No.\" := 'V'; end;
         local procedure ByVar(var R: Record \"ALT T\") begin R.\"No.\" := 'R'; end;
         procedure F(): Text var R: Record \"ALT T\"; S: Text; begin R.\"No.\" := 'A'; ByVal(R); S := R.\"No.\"; ByVar(R); exit(S + R.\"No.\"); end;",
        "F",
    );
    assert_eq!(text(r), "AR");
}

#[test]
fn table_procedures_run_on_the_receiver() {
    let r = run(
        "procedure F(): Decimal var R: Record \"ALT T\"; begin R.Bump(2); R.Bump(3); exit(R.Amount); end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Decimal(d)) if d == 10.into()), "{r:?}");
}

#[test]
fn asserterror_rolls_back_to_the_last_commit() {
    // record/TestRecordRenameRollback.al: an error caught by asserterror rolls back to the last commit point.
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Insert(); Commit();
            asserterror begin R.\"No.\" := 'B'; R.Insert(); Error('x'); end;
            exit(Format(R.Get('A')) + Format(R.Get('B')));
        end;",
        "F",
    );
    assert_eq!(text(r), "YesNo");
}

#[test]
fn temporary_records_have_their_own_table() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; T: Record \"ALT T\" temporary; begin
            T.\"No.\" := 'A'; T.Insert();
            exit(Format(T.Count()) + Format(R.Count()));
        end;",
        "F",
    );
    assert_eq!(text(r), "10");
}

// record/TestTempTableContracts.al:TempVar_PassedByValue_CallerNotAffected -- a temporary record
// passed by value arrives with an independent, empty table; by var it is the caller's table.
#[test]
fn temporary_records_passed_by_value_get_an_independent_empty_table() {
    let r = run(
        "procedure F(): Text var Caller: Record \"ALT T\" temporary; begin
            Caller.\"No.\" := 'A'; Caller.Insert();
            exit(Format(ByValue(Caller)) + Format(Caller.Count()) + Format(ByRef(Caller)) + Format(Caller.Count()));
        end;
        local procedure ByValue(T: Record \"ALT T\" temporary): Integer begin T.\"No.\" := 'B'; T.Insert(); exit(T.Count()); end;
        local procedure ByRef(var T: Record \"ALT T\" temporary): Integer begin T.\"No.\" := 'C'; T.Insert(); exit(T.Count()); end;",
        "F",
    );
    assert_eq!(text(r), "1122");
}

// record/TestRefusedWriteRowVersion.al -- a refused write hands the record no rowversion.
#[test]
fn refused_writes_leave_the_record_without_a_rowversion() {
    let r = run(
        "procedure F(): Text var A: Record \"ALT T\"; B: Record \"ALT T\"; C: Record \"ALT T\"; begin
            A.\"No.\" := 'a'; A.Insert();
            B.\"No.\" := 'a';
            C.\"No.\" := 'zz';
            exit(Format(B.Insert()) + Format(B.SystemRowVersion) + Format(C.Modify()) + Format(C.SystemRowVersion)
                + Format(A.SystemRowVersion <> 0));
        end;",
        "F",
    );
    assert_eq!(text(r), "No0No0Yes");
}

#[test]
fn find_without_argument_is_find_equal_by_primary_key() {
    // Doc (Record.Find): Which defaults to '='.
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.\"No.\" := 'A'; R.Grp := 1; R.Insert();
            R.Grp := 7;
            exit(Format(R.Find()) + Format(R.Grp));
        end;",
        "F",
    );
    assert_eq!(text(r), "Yes1");
}

// Find honours the active filters; a key typed by hand outside the filter is not found
// (record/TestFilterTextBooleanFindContracts.al:143, Find_AllForms_HonourTheActiveFilters).
#[test]
fn find_equal_honours_the_filters() {
    let r = run(
        "procedure F(): Boolean var R: Record \"ALT T\"; begin R.\"No.\" := 'A'; R.Insert(); R.SetRange(Grp, 1); exit(R.Find()); end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Boolean(false))), "{r:?}");
}

#[test]
fn date_and_time_init_values() {
    let src = "table 50910 D { fields { field(1; K; Integer) { } field(2; Dt; Date) { InitValue = 20240115D; } field(3; Tm; Time) { InitValue = 120000T; } } }\n";
    let body =
        "procedure F(): Text var R: Record D; begin exit(Format(R.Dt, 0, 9) + ' ' + Format(R.Tm = 120000T)); end;";
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), format!("{src}codeunit 50100 C {{ {body} }}"))],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty());
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut s = Session::new(Engine::new(l.program, blocked));
    let inst = s.instance(oi);
    assert_eq!(text(s.call(oi, 0, inst, vec![]).map(|(v, _)| v)), "2024-01-15 Yes");
}

// Was `rename_of_a_table_other_fields_relate_to_is_unsupported`: the refusal stood in for rename
// propagation until it was implemented (tests/rename.rs holds the full set); the same scenario now
// follows the relation (corpus record/TestRenamePropagation.al:20-40).
#[test]
fn rename_of_a_table_other_fields_relate_to_rewrites_them() {
    let src = "table 50911 P { fields { field(1; K; Code[10]) { } } }\ntable 50912 Ch { fields { field(1; Id; Integer) { } field(2; Parent; Code[10]) { TableRelation = P.K; } } }\n";
    let body = "procedure F(): Text var R: Record P; C: Record Ch; begin R.K := 'A'; R.Insert(); C.Id := 1; C.Parent := 'A'; C.Insert(); R.Rename('B'); C.Get(1); exit(C.Parent); end;";
    let (_, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), format!("{src}codeunit 50100 C {{ {body} }}"))],
        }],
        &Default::default(),
    );
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut s = Session::new(Engine::new(l.program, blocked));
    let inst = s.instance(oi);
    assert_eq!(text(s.call(oi, 0, inst, vec![]).map(|(v, _)| v)), "B");
}

#[test]
fn setfilter_with_expressions_and_placeholders() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; begin
            R.\"No.\" := 'AP1'; R.Grp := 5; R.Insert();
            R.\"No.\" := 'AP2'; R.Grp := 15; R.Insert();
            R.\"No.\" := 'B'; R.Grp := 25; R.Insert();
            R.SetFilter(\"No.\", 'ap*');
            R.SetFilter(Grp, '>%1', 10);
            exit(Format(R.Count()) + Format(R.FindFirst()) + R.\"No.\");
        end;",
        "F",
    );
    assert_eq!(text(r), "1YesAP2");
}

#[test]
fn manual_subscribers_that_nothing_binds_do_not_block_writes() {
    // corpus _fixtures/events/ALTManualTableEventSubscriber.al: a Manual subscriber only fires while bound.
    let src = format!("{TABLE}\ncodeunit 50101 Sub {{ EventSubscriberInstance = Manual;
        [EventSubscriber(ObjectType::Table, Database::\"ALT T\", 'OnAfterInsertEvent', '', false, false)]
        local procedure OnAfterInsert(var Rec: Record \"ALT T\") begin Error('must not fire'); end; }}
        codeunit 50100 C {{ procedure F(): Integer var R: Record \"ALT T\"; begin R.\"No.\" := 'A'; R.Insert(); exit(R.Count()); end; }}");
    let (_, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut s = Session::new(Engine::new(l.program, blocked));
    let inst = s.instance(oi);
    assert!(matches!(s.call(oi, 0, inst, vec![]), Ok((Value::Integer(1), _))));
}

#[test]
fn extension_field_triggers_do_not_block_other_writes() {
    let src = format!("{TABLE}\ntableextension 50102 X extends \"ALT T\" {{ fields {{ field(50000; Ext; Integer) {{ trigger OnValidate() begin Log := 'ext'; end; }} }} }}
        codeunit 50100 C {{ procedure F(): Text var R: Record \"ALT T\"; begin R.\"No.\" := 'A'; R.Insert(); R.Validate(Ext, 1); R.DeleteAll(); exit(R.Log + Format(R.Count())); end; }}");
    let (_, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let mut s = Session::new(Engine::new(l.program, blocked));
    let inst = s.instance(oi);
    assert_eq!(text(s.call(oi, 0, inst, vec![]).map(|(v, _)| v)), "ext0");
}

#[test]
fn writes_assign_increasing_row_versions() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; V1: BigInteger; V2: BigInteger; begin
            R.\"No.\" := 'A'; R.Insert(); R.Get('A'); V1 := R.SystemRowVersion;
            R.Grp := 1; R.Modify(); R.Get('A'); V2 := R.SystemRowVersion;
            exit(Format(V1 > 0) + Format(V2 > V1));
        end;",
        "F",
    );
    assert_eq!(text(r), "YesYes");
}

// record/TestRenamePropagationConditional.al: a rename must re-key related rows, and a conditional
// relation relates to every table of its arms (the else table too). Was a refusal until rename
// propagation existed; now every arm's table renames, and tests/rename.rs pins what happens to the
// related rows.
#[test]
fn renaming_a_table_any_arm_of_a_relation_points_to_succeeds() {
    let src = r#"
table 50901 PA { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50902 PB { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50903 PC { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50904 Child
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Option) { OptionMembers = A,B; }
        field(3; Ref; Code[10]) { TableRelation = if (Kind = const(A)) PA.Code else PB.Code; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50100 C
{
    procedure Rename(Which: Integer)
    var A: Record PA; B: Record PB; C: Record PC;
    begin
        case Which of
            1: begin A.Code := 'X'; A.Insert(); A.Rename('Y'); end;
            2: begin B.Code := 'X'; B.Insert(); B.Rename('Y'); end;
            3: begin C.Code := 'X'; C.Insert(); C.Rename('Y'); end;
        end;
    end;
    procedure R1() begin Rename(1); end;
    procedure R2() begin Rename(2); end;
    procedure R3() begin Rename(3); end;
}"#;
    let call = |r: &str| {
        let (ws, l) = al_sema::lower::compile(
            vec![AppSource {
                id: "a".into(),
                name: "a".into(),
                dependencies: vec![],
                files: vec![("f.al".into(), src.into())],
            }],
            &Default::default(),
        );
        assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
        let blocked: Vec<Vec<Option<String>>> = l
            .unsupported
            .iter()
            .map(|o| o.iter().map(|r| r.first().cloned()).collect())
            .collect();
        let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
        let ri = l.program.objects[oi].routines.iter().position(|x| x.name == r).unwrap();
        let mut s = Session::new(Engine::new(l.program, blocked));
        let inst = s.instance(oi);
        s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
    };
    assert!(call("R1").is_ok(), "{:?}", call("R1"));
    assert!(call("R2").is_ok(), "{:?}", call("R2"));
    assert!(call("R3").is_ok(), "{:?}", call("R3"));
}

// record/TestRecordStaleBufferWrite.al: a Modify through a buffer another variable has since
// changed is accepted (last writer wins) -- unless a Commit sits between that other write and
// this one, which refuses it with DB:RecordChanged. A buffer that never read the row (no
// rowversion) is never stale.
#[test]
fn modify_through_a_stale_buffer_is_refused_only_after_a_commit() {
    let flow = |commit: bool| {
        run(
            &format!(
                "procedure F(): Text var Rec: Record \"ALT T\"; Stale: Record \"ALT T\"; begin
                    Rec.\"No.\" := 'a'; Rec.Insert(); Commit();
                    Stale.Get('a'); Rec.Get('a'); Rec.Grp := 42; Rec.Modify();
                    {c}
                    Stale.Grp := 7;
                    if {commit} then begin
                        asserterror Stale.Modify();
                        exit(GetLastErrorCode() + '|' + GetLastErrorText());
                    end;
                    Stale.Modify();
                    Rec.Get('a');
                    exit(Format(Rec.Grp));
                end;",
                c = if commit { "Commit();" } else { "" },
                commit = if commit { "true" } else { "false" }
            ),
            "F",
        )
    };
    let refused = text(flow(true));
    assert!(refused.starts_with("DB:RecordChanged|"), "{refused}");
    assert!(
        refused.contains("not up-to-date") && refused.contains("No.='A'"),
        "{refused}"
    );
    assert_eq!(text(flow(false)), "7");
    // A buffer that never read the row has no rowversion to be stale against.
    let r = run(
        "procedure F(): Integer var Rec: Record \"ALT T\"; Fresh: Record \"ALT T\"; begin
            Rec.\"No.\" := 'a'; Rec.Insert(); Commit();
            Rec.Grp := 1; Rec.Modify(); Commit();
            Fresh.\"No.\" := 'a'; Fresh.Grp := 9; Fresh.Modify();
            Rec.Get('a'); exit(Rec.Grp);
        end;",
        "F",
    );
    assert!(matches!(r, Ok(Value::Integer(9))), "{r:?}");
}

// OnInstallAppPerDatabase runs with CompanyName() empty (measured on real BC 27.5, not yet a corpus
// test; docs/probes-needed.md): a per-company table cannot be accessed there, a table with
// DataPerCompany = false can.
#[test]
fn with_no_company_chosen_only_a_shared_table_can_be_accessed() {
    use al_builtins::native::Signal;
    use al_sema::workspace::AppSource;
    let src = r#"
table 50301 PerCompany { fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
table 50302 Shared { DataPerCompany = false; fields { field(1; K; Integer) { } } keys { key(PK; K) { } } }
codeunit 50100 C
{
    procedure Shared(): Integer var R: Record Shared; begin R.K := 1; R.Insert(); exit(R.Count()); end;
    procedure Own() var R: Record PerCompany; begin R.K := 1; R.Insert(); end;
}"#;
    let (_, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    let engine = al_vm::Engine::new(l.program.clone(), vec![]);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = |n: &str| l.program.objects[oi].routines.iter().position(|r| r.name == n).unwrap();
    let mut s = al_vm::Session::new(engine);
    assert_eq!(s.set_company(""), "CRONUS International Ltd.");
    let inst = s.instance(oi);
    assert!(matches!(
        s.call(oi, ri("Shared"), inst.clone(), vec![]),
        Ok((al_builtins::value::Value::Integer(1), _))
    ));
    assert!(matches!(
        s.call(oi, ri("Own"), inst.clone(), vec![]),
        Err(Signal::Unsupported(r)) if r.contains("no company chosen")
    ));
    s.set_company("CRONUS International Ltd.");
    assert!(s.call(oi, ri("Own"), inst, vec![]).is_ok());
}

#[test]
fn clear_of_an_empty_temporary_record_resets_fields_and_filters() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\" temporary; begin
            R.\"No.\" := 'a'; R.Grp := 7; R.SetRange(Grp, 7);
            Clear(R);
            exit(R.\"No.\" + Format(R.Grp) + Format(R.GetFilters()) + Format(R.IsEmpty()));
        end;",
        "F",
    );
    assert_eq!(text(r), "0Yes");
}

#[test]
fn clear_of_a_temporary_record_that_holds_rows_keeps_the_rows() {
    // types/ProbeTypeFormsAndFilters.al:Clear_TemporaryRecordWithRows_ThenCount (BC 27.5): the buffer and the
    // filters are reset, the rows stay and the record stays temporary.
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\" temporary; begin
            R.\"No.\" := 'a'; R.Insert(); R.\"No.\" := 'b'; R.Insert(); R.SetRange(\"No.\", 'a');
            Clear(R);
            exit(R.\"No.\" + '|' + Format(R.Count()) + '|' + R.GetFilters() + '|' + Format(R.IsTemporary()));
        end;",
        "F",
    );
    assert_eq!(text(r), "|2||Yes");
}

#[test]
fn overloads_that_differ_by_record_type_are_told_apart() {
    // System Application's "Retention Policy Setup Impl." has ValidateRetentionPeriod(Record "Retention
    // Policy Setup") and ValidateRetentionPeriod(Record "Retention Policy Setup Line"); AL picks by the
    // record's table.
    let src = r#"
table 50901 "ALT Other" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[20]) { } } keys { key(PK; "No.") { } } }
codeunit 50101 D {
  procedure Which(R: Record "ALT T"): Text begin exit('T' + R."No."); end;
  procedure Which(R: Record "ALT Other"): Text begin exit('O' + R.Name); end;
  procedure F(): Text var A: Record "ALT T"; B: Record "ALT Other"; begin
    A."No." := 'x'; B.Name := 'y'; exit(Which(A) + Which(B)); end;
}"#;
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), format!("{TABLE}\n{src}"))],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "D").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let r = s.call(oi, ri, inst, vec![]).map(|(v, _)| v);
    assert_eq!(text(r), "TXOy");
}

#[test]
fn copy_filter_copies_one_fields_filter_to_a_field_of_another_record() {
    // record/TestRecordFilter.al:226-239 (the filter string of the source arrives on the target) and
    // record/TestFilterAdvancedContracts.al:216-223 (a source without a filter clears the target's).
    let r = run(
        "procedure F(): Text var A: Record \"ALT T\"; B: Record \"ALT T\"; C: Record \"ALT T\"; R: Text; begin
            A.SetRange(Grp, 1, 5);
            A.CopyFilter(Grp, B.Grp);
            R := A.GetFilter(Grp) + '|' + B.GetFilter(Grp) + '|';
            B.SetRange(Grp, 42);
            C.CopyFilter(Grp, B.Grp);
            R += B.GetFilter(Grp) + '|' + Format(B.HasFilter());
            exit(R);
        end;",
        "F",
    );
    assert_eq!(text(r), "1..5|1..5||No");
}

// A second SetCurrentKey on a prefix of the record's ad hoc key (fields no key starts with) is
// another ad hoc key: rows by those fields, then by the primary key (record/TestRecordNavContracts.al:180-195,
// xmlports/TestXmlPortTriggersReplaceViewContracts.al:339-375). It matched the ad hoc key itself and
// left the record on a key index the table does not have (a panic in Tests-General Journal).
#[test]
fn set_current_key_on_a_prefix_of_the_ad_hoc_key() {
    let r = run(
        "procedure F(): Text var R: Record \"ALT T\"; S: Text; begin
            R.\"No.\" := 'A'; R.Insert(); R.Amount := 2; R.Modify();
            R.\"No.\" := 'C'; R.Insert(); R.Amount := 1; R.Log := 'b'; R.Modify();
            R.\"No.\" := 'B'; R.Insert(); R.Amount := 1; R.Log := 'c'; R.Modify();
            R.SetCurrentKey(Amount, Log);
            R.SetCurrentKey(Amount);
            if R.FindSet() then repeat S += R.\"No.\"; until R.Next() = 0;
            exit(S);
        end;",
        "F",
    );
    assert_eq!(text(r), "BCA");
}

// record/ProbeClearAllSetKeyHyperlink.al:276-318 (BC 27.5): SetCurrentKey on a FlowField goes through, on a FlowFilter
// field or a Blob it raises `Sorting cannot be done based on the <field> field(s) in the <table> table.` (DB:Ok).
#[test]
fn set_current_key_on_a_flow_field_works_and_on_a_flow_filter_or_blob_raises() {
    let src = "table 50950 \"Rb\" { fields { field(1; Id; Integer) { } field(4; Flow; Integer) { FieldClass = FlowField; CalcFormula = count(\"ALT T\"); }
        field(5; FlowFilt; Integer) { FieldClass = FlowFilter; } field(6; Pic; Blob) { } } keys { key(PK; Id) { } } }
        codeunit 50101 C2 { procedure F(): Text var R: Record Rb; S: Text; begin
            R.SetCurrentKey(Flow); if R.FindFirst() then; S := 'flow ok';
            asserterror R.SetCurrentKey(FlowFilt); S += '|' + GetLastErrorText() + '|' + GetLastErrorCode();
            asserterror R.SetCurrentKey(Pic); S += '|' + GetLastErrorText(); exit(S); end; }";
    let full = format!("{TABLE}\n{src}");
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), full)],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C2").unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    let out = text(s.call(oi, 0, inst, vec![]).map(|(v, _)| v));
    assert_eq!(
        out,
        "flow ok|Sorting cannot be done based on the FlowFilt field(s) in the Rb table.|DB:Ok|Sorting cannot be done based on the Pic field(s) in the Rb table."
    );
}
