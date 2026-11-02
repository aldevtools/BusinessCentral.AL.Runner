//! A quoted `"this"` is a plain variable; the unquoted `this` is always the running object, even where a variable is
//! named `this` (al-runner issue 6750; values measured on real BC in the corpus writer's Y73 case, files
//! tests/al-language/types/lt-y73-quoted-this/Y73ThisCodeunit.al; needs corpus bump).
//! S = the running object, V = the variable's instance.

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 50801 "Q Pub"
{
    procedure Run(): Text
    var T: Text;
    begin OnGet(T); exit(T); end;
    [IntegrationEvent(false, false)]
    local procedure OnGet(var T: Text) begin end;
}
codeunit 50802 "Q Subj"
{
    EventSubscriberInstance = Manual;
    var
        "this": Codeunit "Q Subj";
        Tag: Text;
    procedure SetTag(T: Text) begin Tag := T; end;
    procedure GetTag(): Text begin exit(Tag); end;
    procedure TakeByValue(C: Codeunit "Q Subj") begin C.SetTag('P'); end;
    procedure TakeVar(var C: Codeunit "Q Subj") begin C.SetTag('P'); end;
    procedure RetQuoted(): Codeunit "Q Subj" begin exit("this"); end;
    procedure RetKeyword(): Codeunit "Q Subj" begin exit(this); end;
    procedure Members(): Text
    begin
        Tag := 'S';
        "this".SetTag('V');
        exit(Tag + '|' + "this".GetTag() + '|' + this.GetTag());
    end;
    procedure KeywordSet(): Text
    begin
        this.SetTag('K');
        exit(Tag + '|' + "this".GetTag() + '|' + this.GetTag());
    end;
    procedure Bind(): Text
    var P: Codeunit "Q Pub"; R: Text;
    begin
        "this".SetTag('V');
        BindSubscription("this");
        R := P.Run();
        UnbindSubscription("this");
        exit(R + '/' + P.Run());
    end;
    procedure BindKeyword(): Text
    var P: Codeunit "Q Pub"; R: Text;
    begin
        Tag := 'S';
        "this".SetTag('V');
        BindSubscription(this);
        R := P.Run();
        UnbindSubscription(this);
        exit(R + '/' + P.Run());
    end;
    procedure ByValue(): Text
    begin
        Tag := 'S';
        "this".SetTag('V');
        TakeByValue("this");
        exit(Tag + '|' + "this".GetTag());
    end;
    procedure ByValueKeyword(): Text
    begin
        Tag := 'S';
        "this".SetTag('V');
        TakeByValue(this);
        exit(Tag + '|' + "this".GetTag());
    end;
    procedure ByVar(): Text
    begin
        Tag := 'S';
        "this".SetTag('V');
        TakeVar("this");
        exit(Tag + '|' + "this".GetTag());
    end;
    procedure Exits(): Text
    var X: Codeunit "Q Subj";
    begin
        Tag := 'S';
        "this".SetTag('V');
        X := RetQuoted();
        X.SetTag('R');
        exit(Tag + '|' + "this".GetTag() + '|' + X.GetTag());
    end;
    procedure ExitsKeyword(): Text
    var X: Codeunit "Q Subj";
    begin
        Tag := 'S';
        "this".SetTag('V');
        X := RetKeyword();
        X.SetTag('R');
        exit(Tag + '|' + "this".GetTag() + '|' + X.GetTag());
    end;
    procedure Assigns(): Text
    var X: Codeunit "Q Subj";
    begin
        Tag := 'S';
        X.SetTag('W');
        "this" := X;
        exit(Tag + '|' + "this".GetTag() + '|' + X.GetTag());
    end;
    procedure ClearsVar(): Text
    var Before: Text;
    begin
        Tag := 'S';
        "this".SetTag('V');
        Before := "this".GetTag();
        Clear("this");
        exit(Before + '|' + Tag + '|' + "this".GetTag());
    end;
    procedure LocalShadows(): Text
    var "this": Codeunit "Q Subj";
    begin
        Tag := 'S';
        "this".SetTag('LOC');
        exit(Tag + '|' + "this".GetTag() + '|' + this.GetTag());
    end;
    procedure ParamShadows("this": Codeunit "Q Subj"): Text
    begin
        Tag := 'S';
        "this".SetTag('PAR');
        exit(Tag + '|' + "this".GetTag() + '|' + this.GetTag());
    end;
    procedure ParamShadowsRun(): Text
    var A: Codeunit "Q Subj";
    begin
        exit(ParamShadows(A));
    end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Q Pub", 'OnGet', '', false, false)]
    local procedure OnGetSub(var T: Text) begin T += '[' + Tag + ']'; end;
}
"#;

fn go(routine: &str) -> String {
    text(run_in(SRC, "Q Subj", routine))
}

// Expected values: the corpus writer's Y73 measurements on real BC (hand-off values quoted in the PR).
#[test]
fn member_calls_on_the_quoted_name_run_on_the_variable() {
    assert_eq!(go("Members"), "S|V|S");
    assert_eq!(go("KeywordSet"), "K||K");
}

#[test]
fn bind_and_unbind_take_the_variable_or_the_object() {
    assert_eq!(go("Bind"), "[V]/");
    assert_eq!(go("BindKeyword"), "[S]/");
}

#[test]
fn arguments_and_returns() {
    assert_eq!(go("ByValue"), "S|P");
    assert_eq!(go("ByValueKeyword"), "P|V");
    assert_eq!(go("ByVar"), "S|P");
    assert_eq!(go("Exits"), "S|R|R");
    assert_eq!(go("ExitsKeyword"), "R|V|R");
}

#[test]
fn assignment_and_clear() {
    assert_eq!(go("Assigns"), "S|W|W");
    assert_eq!(go("ClearsVar"), "V|S|");
}

#[test]
fn a_local_or_parameter_named_this_hides_the_global_but_not_the_keyword() {
    assert_eq!(go("LocalShadows"), "S|LOC|S");
    assert_eq!(go("ParamShadowsRun"), "S|PAR|S");
}

#[test]
fn in_a_table_the_quoted_name_is_the_variable_and_the_keyword_is_the_record() {
    let src = r#"
codeunit 50811 "Q Cu" { var Tag: Text; procedure SetTag(T: Text) begin Tag := T; end; procedure GetTag(): Text begin exit(Tag); end; }
table 50812 "Q Tab"
{
    fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } }
    keys { key(PK; Id) { } }
    var "this": Codeunit "Q Cu";
    procedure Probe(): Text
    begin
        "this".SetTag('V');
        this.Txt := 'T';
        exit("this".GetTag() + '|' + Txt + '|' + this.Txt);
    end;
}
codeunit 50813 "Q Run"
{
    procedure Go(): Text
    var R: Record "Q Tab";
    begin
        exit(R.Probe());
    end;
}
"#;
    assert_eq!(text(run_in(src, "Q Run", "Go")), "V|T|T");
}
