//! Which overload of a procedure AL code declares runs when an argument is an enum, or goes into an enum parameter
//! (al-runner issue 7445). An enum converts to an enum of another type only when the TARGET states
//! `AssignmentCompatibility = true` (must-not-compile 0074, 0406; types/enum-option/PnE13EnumConv.al:81-111), an Integer
//! converts to an enum (PnE13EnumConv.al:70-73 `TakeDst(I)`), and an enum to an Integer (compile matrix, `check/conv.rs`).
//! When exactly one overload takes every argument, BC calls it: the other ones do not compile. When two take every
//! argument and one of them needs one of these conversions, the one that declares the argument's own type wins (Option
//! against enum pinned, the rest the assumption `enum-overload-own-type`); with no such overload the call is refused.

use al_builtins::native::Signal;
use al_testkit::text;

const ENUMS: &str = r#"
enum 69800 "EO Line" { Extensible = true; AssignmentCompatibility = true; value(0; " ") { } value(1; Account) { } value(2; Item) { } }
enum 69801 "EO Other Line" { Extensible = true; AssignmentCompatibility = true; value(0; " ") { } value(1; Account) { } value(2; Item) { } }
enum 69802 "EO Sales Doc" { Extensible = true; value(0; Quote) { } value(1; Order) { } }
enum 69803 "EO Purch Doc" { Extensible = true; value(0; Quote) { } value(1; Order) { } }
"#;

fn src(body: &str) -> String {
    format!("{ENUMS}\ncodeunit 69804 C\n{{\n{body}\n}}\n")
}

/// The issue's shape: the first overload takes the line type as it is but not the purchase document type; the second
/// takes the line type by AssignmentCompatibility and the document type as it is. Only the second compiles.
#[test]
fn the_only_overload_that_takes_every_argument_runs() {
    let s = src(r#"
    local procedure Find(R: RecordRef; L: Enum "EO Line"; No: Code[20]; D: Enum "EO Sales Doc"): Text
    begin
        exit('sales ' + Format(L.AsInteger()));
    end;
    procedure Find(R: RecordRef; L: Enum "EO Other Line"; No: Code[20]; D: Enum "EO Purch Doc"): Text
    begin
        case L of
            L::Item:
                exit('purch item');
            else
                exit('purch other ' + Format(L.AsInteger()));
        end;
    end;
    procedure F(): Text
    var R: RecordRef; L: Enum "EO Line"; D: Enum "EO Purch Doc";
    begin
        L := L::Item;
        exit(Find(R, L, 'X', D));
    end;"#);
    assert_eq!(text(al_testkit::run_in(&s, "C", "F")), "purch item");
}

/// An enum into a parameter of an enum without AssignmentCompatibility does not compile (0406): the Integer overload is
/// the one that takes it.
#[test]
fn an_enum_goes_to_the_integer_overload_when_the_other_enum_lacks_assignment_compatibility() {
    let s = src(r#"
    local procedure Take(D: Enum "EO Sales Doc"): Text
    begin
        exit('sales');
    end;
    local procedure Take(I: Integer): Text
    begin
        exit('integer ' + Format(I));
    end;
    procedure F(): Text
    var D: Enum "EO Purch Doc";
    begin
        D := D::Order;
        exit(Take(D));
    end;"#);
    assert_eq!(text(al_testkit::run_in(&s, "C", "F")), "integer 1");
}

/// An Integer converts to an enum (PnE13EnumConv.al:70-73) but not to a Text: the enum overload takes it.
#[test]
fn an_integer_goes_to_the_enum_overload_over_a_text_one() {
    let s = src(r#"
    local procedure Take(T: Text): Text
    begin
        exit('text');
    end;
    local procedure Take(D: Enum "EO Sales Doc"): Text
    begin
        exit('enum ' + Format(D));
    end;
    procedure F(): Text
    var I: Integer;
    begin
        I := 1;
        exit(Take(I));
    end;"#);
    assert_eq!(text(al_testkit::run_in(&s, "C", "F")), "enum Order");
}

/// Two overloads take the argument, one as it is and one by AssignmentCompatibility: its own enum wins (assumption
/// `enum-overload-own-type`, needed by Base Application's Non-Deductible VAT `GetNonDeductibleVATPct`).
#[test]
fn an_enum_goes_to_its_own_enum_over_a_compatible_one() {
    let s = src(r#"
    local procedure Take(L: Enum "EO Other Line"): Text
    begin
        exit('other');
    end;
    local procedure Take(L: Enum "EO Line"): Text
    begin
        exit('line');
    end;
    procedure F(): Text
    var L: Enum "EO Line";
    begin
        exit(Take(L));
    end;"#);
    assert_eq!(text(al_testkit::run_in(&s, "C", "F")), "line");
}

/// Two overloads take the argument, both by AssignmentCompatibility: not pinned, so refused.
#[test]
fn two_overloads_that_both_convert_an_enum_argument_are_refused() {
    let s = src(r#"
    local procedure Take(L: Enum "EO Line"): Text
    begin
        exit('line');
    end;
    local procedure Take(L: Enum "EO Other Line"): Text
    begin
        exit('other');
    end;
    procedure F(): Text
    var D: Enum "EO Sales Doc";
    begin
        exit(Take(D));
    end;"#);
    let r = al_testkit::run_in(&s, "C", "F");
    match r {
        Err(Signal::Unsupported(m)) => assert!(m.contains("overload"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// An Integer argument where an Integer and an enum overload both take it, and an enum argument there: each goes to the
/// overload of its own type (assumption `enum-overload-own-type`, needed by System Application's Email Account Impl.).
#[test]
fn an_integer_and_an_enum_go_to_the_overload_of_their_own_type() {
    let s = src(r#"
    local procedure Take(D: Enum "EO Sales Doc"): Text
    begin
        exit('enum');
    end;
    local procedure Take(I: Integer): Text
    begin
        exit('integer');
    end;
    procedure F(): Text
    var I: Integer; D: Enum "EO Sales Doc";
    begin
        exit(Take(I) + ' ' + Take(D));
    end;"#);
    assert_eq!(text(al_testkit::run_in(&s, "C", "F")), "integer enum");
}

/// codeunits/CwInterfaceLeads.al:143-167 `CW_Overload_OptionVsEnum`, with the enum overload declared first: an Integer
/// and an Option go to the Option overload, an enum to its own.
#[test]
fn option_against_enum_overloads_follow_the_pinned_choice() {
    let s = src(r#"
    local procedure P(E: Enum "EO Sales Doc"): Text
    begin
        exit('enum ' + Format(E.AsInteger()));
    end;
    local procedure P(O: Option A,B,C): Text
    begin
        exit('option ' + Format(O, 0, 2));
    end;
    procedure F(): Text
    var O: Option A,B,C; E: Enum "EO Sales Doc"; I: Integer;
    begin
        O := O::B;
        E := E::Order;
        I := 2;
        exit(P(1) + ';' + P(I) + ';' + P(O) + ';' + P(E));
    end;"#);
    assert_eq!(
        text(al_testkit::run_in(&s, "C", "F")),
        "option 1;option 2;option 1;enum 1"
    );
}

/// Overloads that differ only in other parameters keep working when the enum is passed as it is.
#[test]
fn an_enum_of_the_parameter_type_picks_by_the_other_arguments() {
    let s = src(r#"
    local procedure Take(D: Enum "EO Sales Doc"; T: Text): Text
    begin
        exit('text');
    end;
    local procedure Take(D: Enum "EO Sales Doc"; B: Boolean): Text
    begin
        exit('boolean');
    end;
    procedure F(): Text
    var D: Enum "EO Sales Doc";
    begin
        exit(Take(D, true));
    end;"#);
    assert_eq!(text(al_testkit::run_in(&s, "C", "F")), "boolean");
}

/// A `var` parameter takes only its own enum (compile matrix arg_var), AssignmentCompatibility or not: the Integer
/// overload takes the enum by value.
#[test]
fn a_var_enum_parameter_of_another_type_does_not_take_the_enum() {
    let s = src(r#"
    local procedure Take(var L: Enum "EO Other Line"): Text
    begin
        exit('other');
    end;
    local procedure Take(I: Integer): Text
    begin
        exit('integer ' + Format(I));
    end;
    procedure F(): Text
    var L: Enum "EO Line";
    begin
        L := L::Item;
        exit(Take(L));
    end;"#);
    assert_eq!(text(al_testkit::run_in(&s, "C", "F")), "integer 2");
}
