//! OnBeforeValidate and OnAfterValidate of a pageextension `modify` on a control bound to a page variable (al-runner issue
//! 7372). The Card is pinned by corpus pages/extensions/W2ExtModify.al:367-401, which is its test; here is a shape that stays
//! refused, which the corpus does not reach.

use al_builtins::native::Signal;

/// A page type other than a Card is not pinned: the SetValue is refused.
#[test]
fn a_variable_control_of_a_list_with_modify_validate_triggers_is_still_refused() {
    let src = r#"
table 50800 Row { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
page 50801 Rows
{
    PageType = List; SourceTable = Row;
    layout { area(Content) { field(VarName; VarNameText) { } repeater(R) { field(Id; Rec.Id) { } } } }
    var VarNameText: Text;
}
pageextension 50802 RowsExt extends Rows
{
    layout { modify(VarName) { trigger OnAfterValidate() begin Seen := true; end; } }
    var Seen: Boolean;
}
codeunit 50803 T { procedure F() var P: TestPage Rows; begin P.OpenEdit(); P.VarName.SetValue('first'); end; }
"#;
    let r = al_testkit::run_in(src, "T", "F");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("on a control bound to a variable (not pinned)")),
        "{r:?}"
    );
}
