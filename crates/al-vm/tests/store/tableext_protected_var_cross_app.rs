//! `protected var` of another app's extension of the same table, used by bare name from a table extension in an app that
//! depends on the declaring one (al-runner issue 6983). Probe: corpus PR 1126 (branch al-runtime-rs-staging-r6983),
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
}
"#;

fn run(routine: &str) -> String {
    let apps = vec![app("a", &[], A), app("b", &["a"], B), app("m", &["a", "b"], M)];
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    out(b.run_in("T", routine))
}

/// ProtectedVarCrossApp.al ProtVar_OtherAppExtensionVar_ReadAndWrite (issue 6983).
#[test]
fn an_extension_in_a_dependent_app_reads_and_writes_another_apps_extension_var() {
    assert_eq!(run("CrossApp"), "Yes|No|No");
}

/// ProtectedVarCrossApp.al ProtVar_OtherAppExtensionVar_ReadInFieldTrigger (the reporter's shape of issue 6983).
#[test]
fn a_field_trigger_reads_another_apps_extension_var() {
    assert_eq!(run("FieldTrigger"), "No|Yes");
}

/// Must-not-compile 2952: an app that does not depend on the declaring app does not see its protected variable.
#[test]
fn an_app_that_does_not_depend_on_the_declaring_app_does_not_see_its_var() {
    let d = "tableextension 50563 \"PVM D\" extends \"PVM T\" { procedure DGet(): Integer begin exit(XbCounter); end; }\n\
             codeunit 50568 T { procedure Run(): Integer var R: Record \"PVM T\"; begin exit(R.DGet()); end; }";
    let apps = vec![app("a", &[], A), app("b", &["a"], B), app("d", &["a"], d)];
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    let r = out(b.run_in("T", "Run"));
    assert!(r.starts_with("refused:"), "{r}");
}
