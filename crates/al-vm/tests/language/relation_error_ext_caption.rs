//! The TableRelation error of a Validate names the related table by the Caption a tableextension sets over the table's own
//! (al-runner issue 5955, the reporter's repro on real BC; registered assumption `relation-error-ext-caption`). Other reads
//! of that caption stay refused.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;

const SRC: &str = "
table 50501 WC { Caption = 'Work Center'; fields { field(1; No; Code[20]) { } } keys { key(PK; No) { } } }
tableextension 50500 WCExt extends WC { Caption = 'Repro Work Center Extension'; }
table 50500 Ref { fields { field(1; Entry; Integer) { } field(2; WcNo; Code[20]) { Caption = 'Work Center No.'; TableRelation = WC; } }
    keys { key(PK; Entry) { } } }
codeunit 50502 C {
    procedure F(): Text var R: Record Ref; begin asserterror R.Validate(WcNo, 'NOWC'); exit(GetLastErrorText()); end;
    procedure G(): Text var W: Record WC; begin exit(W.TableCaption()); end; }";

#[test]
fn the_relation_error_names_the_extension_caption() {
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "The field Work Center No. of table Ref contains a value (NOWC) that cannot be found in the related table (Repro Work Center Extension)."
    );
    assert!(take_hits().contains(&Assumption::RelationErrorExtCaption));
    assert!(matches!(al_testkit::run_in(SRC, "C", "G"), Err(Signal::Unsupported(_))));
}
