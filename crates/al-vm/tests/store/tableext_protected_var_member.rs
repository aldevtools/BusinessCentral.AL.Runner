//! `Other.<protected var>` in a table extension: a protected variable of the table or of another extension of it, named
//! through another record variable of the same table (al-runner issue 6984), here across apps as well. Probe: corpus PR 1126 (branch al-runtime-rs-staging-r6983),
//! tests/al-language/table-objects/extensions/ProtectedVarCrossApp.al with fixtures xapp-a/R6983XaProtectedVars.al and
//! xapp-b/R6983XbProtectedVars.al; must-not-compile 2952 and 2953.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("App {id}"),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![(format!("{id}.al"), src.into())],
    }
}

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t)) => t.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

const A: &str = r#"
table 50560 "PVM T"
{
    fields { field(1; K; Code[10]) { } field(2; Synced; Boolean) { } }
    keys { key(PK; K) { Clustered = true; } }

    protected var
        BaseText: Text;

    procedure GetBaseText(): Text
    begin
        exit(BaseText);
    end;

    procedure SetBaseText(V: Text)
    begin
        BaseText := V;
    end;
}
tableextension 50561 "PVM A" extends "PVM T"
{
    procedure SetHide(V: Boolean)
    begin
        MyHide := V;
    end;

    procedure GetHide(): Boolean
    begin
        exit(MyHide);
    end;

    protected var
        MyHide: Boolean;
}
"#;

const B: &str = r#"
tableextension 50562 "PVM B" extends "PVM T"
{
    fields
    {
        field(50562; Weight; Decimal)
        {
            trigger OnValidate()
            begin
                Synced := not MyHide and (Weight <> 0);
            end;
        }
    }

    protected var
        XbCounter: Integer;

    procedure BGetHide(): Boolean
    begin
        exit(MyHide);
    end;

    procedure BSetHide(V: Boolean)
    begin
        MyHide := V;
    end;

    procedure BSetOtherHide(var Other: Record "PVM T"; V: Boolean)
    begin
        Other.MyHide := V;
    end;

    procedure BGetOtherHide(var Other: Record "PVM T"): Boolean
    begin
        exit(Other.MyHide);
    end;

    procedure BSetOtherBase(var Other: Record "PVM T"; V: Text)
    begin
        Other.BaseText := V;
    end;

    procedure BGetOtherBase(var Other: Record "PVM T"): Text
    begin
        exit(Other.BaseText);
    end;

    procedure BLocalRoundTrip(V: Text): Text
    var
        Other: Record "PVM T";
    begin
        Other.BaseText := V;
        exit(Other.GetBaseText() + '|' + BaseText);
    end;
}
"#;

const M: &str = r#"
codeunit 50569 T
{
    procedure CrossApp(): Text
    var
        R: Record "PVM T";
        Second: Record "PVM T";
    begin
        R.SetHide(true);
        exit(Format(R.BGetHide()) + '|' + CrossAppWrite(R) + '|' + Format(Second.BGetHide()));
    end;

    local procedure CrossAppWrite(var R: Record "PVM T"): Text
    begin
        R.BSetHide(false);
        exit(Format(R.GetHide()));
    end;

    procedure FieldTrigger(): Text
    var
        R: Record "PVM T";
        Res: Text;
    begin
        R.SetHide(true);
        R.Validate(Weight, 5);
        Res := Format(R.Synced);
        R.SetHide(false);
        R.Validate(Weight, 6);
        exit(Res + '|' + Format(R.Synced));
    end;

    procedure Member(): Text
    var
        R: Record "PVM T";
        Other: Record "PVM T";
        Res: Text;
    begin
        R.BSetOtherHide(Other, true);
        Res := Format(Other.GetHide()) + '|' + Format(R.GetHide());
        Other.SetHide(false);
        R.SetHide(true);
        Res += '|' + Format(R.BGetOtherHide(Other));
        R.BSetOtherBase(Other, 'other');
        Res += '|' + Other.GetBaseText() + '|' + R.GetBaseText();
        R.SetBaseText('rec');
        Res += '|' + R.BGetOtherBase(Other);
        exit(Res + '|' + R.BLocalRoundTrip('local'));
    end;
}
"#;

fn run(routine: &str) -> String {
    let apps = vec![app("a", &[], A), app("b", &["a"], B), app("m", &["a", "b"], M)];
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    out(b.run_in("T", routine))
}

/// ProtectedVarCrossApp.al ProtVar_OtherRecordVariable_MemberForm_OtherApp and ProtVar_LocalRecordVariable_MemberForm (issue 6984).
#[test]
fn the_member_form_reads_and_writes_the_other_record_variables_var() {
    assert_eq!(run("Member"), "Yes|No|No|other||other|local|rec");
}

/// Must-not-compile 2953: a codeunit does not see a protected variable of a table through a record variable.
#[test]
fn a_codeunit_does_not_see_a_protected_var_through_a_record_variable() {
    let src = format!(
        "{A}\ncodeunit 50567 T {{ procedure Run(): Text var R: Record \"PVM T\"; begin exit(R.BaseText); end; }}"
    );
    let r = out(al_testkit::run_in(&src, "T", "Run"));
    assert!(r.starts_with("refused:"), "{r}");
}

const SAME_APP: &str = r#"
table 50570 "PVM S"
{
    fields { field(1; K; Code[10]) { } }
    keys { key(PK; K) { Clustered = true; } }

    protected var
        Marker: Text;

    procedure SetMarker(V: Text)
    begin
        Marker := V;
    end;
}
tableextension 50571 "PVM S Ext" extends "PVM S"
{
    procedure GetOther(var Other: Record "PVM S"): Text
    begin
        exit(Other.Marker);
    end;

    procedure SetOther(var Other: Record "PVM S"; V: Text)
    begin
        Other.Marker := V;
    end;
}
codeunit 50572 T
{
    procedure Read(): Text
    var
        R: Record "PVM S";
        Other: Record "PVM S";
    begin
        Other.SetMarker('m');
        exit(R.GetOther(Other) + '|' + R.GetOther(R));
    end;

    procedure Write(): Text
    var
        R: Record "PVM S";
        Other: Record "PVM S";
    begin
        R.SetOther(Other, 'm');
        exit(R.GetOther(Other));
    end;
}
"#;

/// ProtectedVarCrossApp.al ProtVar_OtherRecordVariable_MemberFormRead_SameAppTable: a read in the same app works.
#[test]
fn the_member_form_reads_a_same_app_var() {
    assert_eq!(out(al_testkit::run_in(SAME_APP, "T", "Read")), "m|");
}

/// ProtectedVarCrossApp.al, the comment at table 95533: a write in the same app makes real BC fail to compile the extension's
/// C# (CS0131), so it is refused.
#[test]
fn the_member_form_write_of_a_same_app_var_is_refused() {
    let r = out(al_testkit::run_in(SAME_APP, "T", "Write"));
    assert!(r.starts_with("refused:") && r.contains("CS0131"), "{r}");
}
