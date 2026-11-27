//! `IsEmpty` of G/L Entry against the corpus database's company, before the session's code wrote to it: BC holds
//! G/L entries there (test-framework/RlWorkDateAtStart.al:40 read, :48 `G/L entries exist` Yes). Only the unfiltered
//! `IsEmpty` of the session's own company is answered; every other shape keeps the demo-row refusal (records/op_inner.rs).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 17 "G/L Entry"
{
    fields { field(1; "Entry No."; Integer) { } field(4; "Posting Date"; Date) { } }
    keys { key(PK; "Entry No.") { } }
}
codeunit 69989 R
{
    procedure Plain(): Text
    var GLEntry: Record "G/L Entry";
    begin
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure Filtered(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.SetRange("Entry No.", 1);
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure FilteredOtherGroup(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.FilterGroup(2);
        GLEntry.SetRange("Entry No.", 1);
        GLEntry.FilterGroup(0);
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure MarkedOnly(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.MarkedOnly(true);
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure Temporary(): Text
    var GLEntry: Record "G/L Entry" temporary;
    begin
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure OwnCompanyByName(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry.ChangeCompany(CompanyName());
        exit(Format(GLEntry.IsEmpty()));
    end;
    procedure Counted(): Text
    var GLEntry: Record "G/L Entry";
    begin
        exit(Format(GLEntry.Count()));
    end;
    procedure WriteThenIsEmpty(): Text
    var GLEntry: Record "G/L Entry";
    begin
        GLEntry."Entry No." := 1;
        GLEntry.Insert();
        exit(Format(GLEntry.IsEmpty()));
    end;
}
"#;

fn session(demo: bool) -> impl Fn(&str) -> Result<al_builtins::value::Value, Signal> {
    let l = al_testkit::compile(SRC);
    let mut engine = al_testkit::engine(&l);
    let t = l.program.objects.iter().position(|o| o.name == "G/L Entry").unwrap();
    if demo {
        std::sync::Arc::get_mut(&mut engine).unwrap().demo_tables.insert(t);
    }
    let oi = l.program.objects.iter().position(|o| o.name == "R").unwrap();
    move |routine: &str| {
        let mut s = al_vm::Session::new(engine.clone());
        let ri = l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == routine)
            .unwrap();
        let inst = s.instance(oi);
        s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
    }
}

fn refused(r: Result<al_builtins::value::Value, Signal>) {
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("demo rows")),
        "{r:?}"
    );
}

// RlWorkDateAtStart.al:40/:48: the corpus company holds G/L entries before any test code ran.
#[test]
fn the_unfiltered_is_empty_of_gl_entry_answers_not_empty_in_the_corpus_company() {
    assert_eq!(text(session(true)("Plain")), "No");
}

// The pin reads an unfiltered record only: a filter, in any filter group, or MarkedOnly is not measured.
#[test]
fn a_filtered_or_marked_is_empty_of_gl_entry_keeps_the_refusal() {
    let call = session(true);
    refused(call("Filtered"));
    refused(call("FilteredOtherGroup"));
    refused(call("MarkedOnly"));
}

// The pin reads the session's company without ChangeCompany; a ChangeCompany record is not measured.
#[test]
fn an_is_empty_of_gl_entry_after_change_company_keeps_the_refusal() {
    refused(session(true)("OwnCompanyByName"));
}

// The pin measures IsEmpty only: Count keeps the refusal.
#[test]
fn a_count_of_gl_entry_keeps_the_refusal() {
    refused(session(true)("Counted"));
}

// A temporary record holds no database rows; after the session's own write the stored rows are its own: both stay as
// main answered them.
#[test]
fn a_temporary_record_or_one_after_a_write_answers_from_the_runtime_rows() {
    let call = session(true);
    assert_eq!(text(call("Temporary")), "Yes");
    assert_eq!(text(call("WriteThenIsEmpty")), "No");
}

// With --test-data or --company-data the runner fills no demo tables: IsEmpty answers from the runtime's own rows.
#[test]
fn without_demo_tables_is_empty_of_gl_entry_answers_from_the_runtime_rows() {
    assert_eq!(text(session(false)("Plain")), "Yes");
}
