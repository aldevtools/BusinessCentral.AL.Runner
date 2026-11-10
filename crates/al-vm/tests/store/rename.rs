//! Rename propagates to the fields that relate to the renamed row.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

const TABLES: &str = r#"
table 50970 "RN Parent" {
    fields { field(1; Code; Code[20]) { } field(2; Blocked; Boolean) { } }
    keys { key(PK; Code) { } }
}
table 50971 "RN Parent B" {
    fields { field(1; Code; Code[20]) { } }
    keys { key(PK; Code) { } }
}
table 50972 "RN Child" {
    fields {
        field(1; Id; Integer) { }
        field(2; Kind; Option) { OptionMembers = A,B; }
        field(3; Validated; Code[20]) { TableRelation = "RN Parent".Code; }
        field(4; Unvalidated; Code[20]) { TableRelation = "RN Parent".Code; ValidateTableRelation = false; }
        field(5; Soft; Code[20]) { }
        field(6; Cond; Code[20]) { TableRelation = if (Kind = const(A)) "RN Parent".Code else "RN Parent B".Code; }
        field(7; Single; Code[20]) { TableRelation = if (Kind = const(A)) "RN Parent".Code; }
        field(8; Filtered; Code[20]) { TableRelation = "RN Parent".Code where(Blocked = const(false)); }
    }
    keys { key(PK; Id) { } }
}
table 50973 "RN Key Child" {
    fields {
        field(1; ParentCode; Code[20]) { TableRelation = "RN Parent".Code; }
        field(2; Line; Integer) { }
        field(3; Runs; Integer) { }
    }
    keys { key(PK; ParentCode, Line) { } }
    trigger OnRename() begin Runs += 1; end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{TABLES}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.first()) {
        panic!("F is blocked: {r}");
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, 0, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn a_validated_relation_follows_the_rename_unvalidated_and_soft_fields_do_not() {
    // corpus record/TestRenamePropagation.al:20-85
    let r = run(
        "procedure F(): Text var P: Record \"RN Parent\"; C: Record \"RN Child\"; begin
            P.Code := 'OLD'; P.Insert();
            C.Id := 1; C.Validated := 'OLD'; C.Unvalidated := 'OLD'; C.Soft := 'OLD'; C.Insert();
            P.Rename('NEW');
            C.Get(1);
            exit(C.Validated + '/' + C.Unvalidated + '/' + C.Soft);
        end;",
    );
    assert_eq!(text(r), "NEW/OLD/OLD");
}

#[test]
fn conditional_arms_decide_which_rows_follow() {
    // corpus record/TestRenamePropagationConditional.al:22-160
    let r = run(
        "procedure F(): Text var P: Record \"RN Parent\"; B: Record \"RN Parent B\"; C: Record \"RN Child\"; s: Text; begin
            P.Code := 'OLD'; P.Insert();
            B.Code := 'BOLD'; B.Insert();
            C.Id := 1; C.Kind := C.Kind::A; C.Cond := 'OLD'; C.Single := 'OLD'; C.Insert();
            C.Id := 2; C.Kind := C.Kind::B; C.Cond := 'OLD'; C.Single := 'OLD'; C.Insert();
            C.Id := 3; C.Kind := C.Kind::A; C.Cond := 'BOLD'; C.Insert();
            C.Id := 4; C.Kind := C.Kind::B; C.Cond := 'BOLD'; C.Insert();
            P.Rename('NEW');
            C.Get(1); s := C.Cond + C.Single + '|';
            C.Get(2); s += C.Cond + C.Single + '|';
            B.Rename('BNEW');
            C.Get(3); s += C.Cond + '|';
            C.Get(4); s += C.Cond;
            exit(s);
        end;",
    );
    // Row 1 (Kind A): if arm follows; row 2 (Kind B): its else arm points at Parent B, so 'OLD' stays;
    // the single if-arm relation covers no arm for Kind B. The else table's rename reaches rows 3 and 4
    // alike: the else arm carries no condition.
    assert_eq!(text(r), "NEWNEW|OLDOLD|BNEW|BNEW");
}

#[test]
fn a_where_filter_on_the_related_row_gates_the_propagation() {
    // corpus record/TestRenamePropagationConditional.al:162-215
    let r = run(
        "procedure F(): Text var P: Record \"RN Parent\"; Q: Record \"RN Parent\"; C: Record \"RN Child\"; s: Text; begin
            P.Code := 'OPEN'; P.Insert();
            Q.Code := 'SHUT'; Q.Blocked := true; Q.Insert();
            C.Id := 1; C.Filtered := 'OPEN'; C.Insert();
            C.Id := 2; C.Filtered := 'SHUT'; C.Insert();
            P.Rename('OPEN2');
            Q.Rename('SHUT2');
            C.Get(1); s := C.Filtered + '|';
            C.Get(2); s += C.Filtered;
            exit(s);
        end;",
    );
    assert_eq!(text(r), "OPEN2|SHUT");
}

#[test]
fn a_key_field_relation_re_keys_the_children_without_running_their_on_rename() {
    // corpus record/TestRenameKeyRelation.al:23-95
    let r = run(
        "procedure F(): Text var P: Record \"RN Parent\"; K: Record \"RN Key Child\"; s: Text; i: Integer; begin
            P.Code := 'OLD'; P.Insert(); P.Code := 'OTHER'; P.Insert();
            for i := 1 to 3 do begin K.ParentCode := 'OLD'; K.Line := i; K.Insert(); end;
            K.ParentCode := 'OTHER'; K.Line := 1; K.Insert();
            P.Get('OLD'); P.Rename('NEW');
            s := Format(K.Get('NEW', 1)) + Format(K.Get('NEW', 3)) + Format(K.Get('OLD', 1)) + Format(K.Get('OTHER', 1));
            K.Get('NEW', 2);
            s += Format(K.Runs);
            K.Reset();
            s += '|' + Format(K.Count());
            exit(s);
        end;",
    );
    assert_eq!(text(r), "YesYesNoYes0|4");
}
