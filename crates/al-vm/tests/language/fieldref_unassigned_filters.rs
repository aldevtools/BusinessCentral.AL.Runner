//! Filter members of a FieldRef that holds no field (al-runner issue 7964 item 2; corpus PR 1382, recordref-fieldref/fieldref/
//! FieldRefUnassignedFilters.al:88-127, flagged through `fieldref-unassigned-filters`). The members measured only on a FieldRef never
//! assigned keep the refusal: the runtime cannot tell it from a cleared one.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 69981 "FU Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
codeunit 69982 "FU C"
{
    procedure F(State: Integer; Member: Integer): Text
    var
        RecRef: RecordRef;
        Fld: FieldRef;
        R: Text;
    begin
        if State = 1 then begin
            RecRef.Open(Database::"FU Row");
            Fld := RecRef.Field(2);
            Clear(Fld);
        end;
        case Member of
            1: Fld.SetRange();
            2: Fld.SetRange('A');
            3: Fld.SetRange(1);
            4: Fld.SetRange('A', 'B');
            5: Fld.SetFilter('A');
            6: Fld.SetFilter('');
            7: R := Fld.GetFilter();
            8: R := Format(Fld.GetRangeMin());
            9: R := Format(Fld.GetRangeMax());
            10: Fld.SetFilter('%1', 'A');
        end;
        exit('no error ' + R);
    end;
}
"#;

fn call(state: i32, member: i32) -> Result<Value, Signal> {
    take_hits();
    al_testkit::run_with(SRC, "FU C", "F", vec![Value::Integer(state), Value::Integer(member)]).map(|(v, _)| v)
}

#[test]
fn the_measured_members_raise_not_initialized_flagged() {
    for state in [0, 1] {
        for member in [1, 2, 5, 7, 8] {
            let r = call(state, member);
            let Err(Signal::Error(e)) = &r else {
                panic!("state {state} member {member}: {r:?}")
            };
            assert_eq!(
                e.message,
                "Microsoft.Dynamics.Nav.Runtime.NavFieldRef variable not initialized."
            );
            assert_eq!(e.code, "NotInitialized");
            assert!(
                take_hits().contains(&Assumption::FieldrefUnassignedFilters),
                "state {state} member {member}"
            );
        }
    }
}

#[test]
fn the_members_measured_only_never_assigned_keep_the_refusal() {
    for member in [3, 4, 6, 9, 10] {
        let r = call(0, member);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "member {member}: {r:?}");
        assert!(!take_hits().contains(&Assumption::FieldrefUnassignedFilters));
    }
}
