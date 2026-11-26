//! xRec in a table extension's insert and delete triggers is not measured (al-runner issue 8986): every read of it refuses
//! (`al-vm` `records/ext_trigger_xrec.rs`). Main gave `OnAfterInsert` the blank xRec of a plain call, a wrong value.
//! Each test reaches one reader of the mark (`Get` and `Count` too: xRec's view is not measured either); the triggers that do not read xRec, and the pinned `OnAfterModify`
//! (table-objects/extensions/TestTableExtensionTableTriggers_Tests.al:183), still run, and so do the methods that read neither its buffer nor its view (`TableCaption`, `FieldNo`, `FieldCaption`).

use al_testrun::Outcome;

const SRC: &str = r#"
codeunit 50400 "Z Mode" { SingleInstance = true; var M: Text; Log: Text;
    procedure Set(V: Text) begin M := V; Log := ''; end; procedure Get(): Text begin exit(M); end;
    procedure Add(V: Text) begin Log += V; end; procedure Take(): Text begin exit(Log); end; }
table 50401 "Z Row"
{
    fields { field(1; Id; Integer) { } field(2; Serial; Integer) { }
        field(3; Mirror; Integer) { trigger OnValidate() var Mode: Codeunit "Z Mode"; begin Mode.Add(Format(xRec.Serial)); end; } }
    keys { key(PK; Id) { Clustered = true; } }
    procedure BaseX(): Integer begin exit(xRec.Serial); end;
    procedure SetMirror(V: Integer) begin Validate(Mirror, V); end;
}
tableextension 50402 "Z Ext" extends "Z Row"
{
    trigger OnAfterInsert()
    var Mode: Codeunit "Z Mode"; Other: Record "Z Row";
    begin
        case Mode.Get() of
            'read': Mode.Add(Format(xRec.Serial));
            'rec': Mode.Add(Format(Rec.Serial));
            'proc': Mode.Add(Format(Rec.BaseX()));
            'copy': begin Other.Copy(xRec); Mode.Add('c'); end;
            'write': begin xRec.Serial := 1; Mode.Add('w'); end;
            'get': begin xRec.Get(Rec.Id); Mode.Add(Format(xRec.Serial)); end;
            'count': Mode.Add(Format(xRec.Count()));
            'validate': Rec.SetMirror(5);
            'caption': Mode.Add(xRec.TableCaption() + Format(xRec.FieldNo(Serial)) + xRec.FieldCaption(Serial));
        end;
    end;
    trigger OnDelete()
    var Mode: Codeunit "Z Mode";
    begin
        if Mode.Get() = 'delete' then Mode.Add(Format(xRec.Serial));
    end;
    trigger OnAfterModify()
    var Mode: Codeunit "Z Mode";
    begin
        if Mode.Get() = 'modify' then Mode.Add(Format(xRec.Serial));
    end;
}
codeunit 50403 "Z Steps"
{
    procedure Run(How: Text): Text
    var Mode: Codeunit "Z Mode"; Row: Record "Z Row";
    begin
        Row.DeleteAll();
        Mode.Set(How);
        Row.Id := 1; Row.Validate(Serial, 42);
        Row.Insert(true);
        if How = 'delete' then Row.Delete(true);
        if How = 'modify' then begin Row.Serial := 43; Row.Modify(true); end;
        exit(Mode.Take());
    end;
}
codeunit 50404 "Z Read" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('read'); end; }
codeunit 50405 "Z Proc" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('proc'); end; }
codeunit 50406 "Z Copy" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('copy'); end; }
codeunit 50407 "Z Write" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('write'); end; }
codeunit 50408 "Z Delete" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('delete'); end; }
codeunit 50411 "Z Get" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('get'); end; }
codeunit 50412 "Z Count" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('count'); end; }
codeunit 50413 "Z Validate" { Subtype = Test; [Test] procedure T() var S: Codeunit "Z Steps"; begin S.Run('validate'); end; }
codeunit 50414 "Z Caption" { Subtype = Test;
    [Test] procedure T() var S: Codeunit "Z Steps"; begin if S.Run('caption') <> 'Z Row2Serial' then Error('other'); end; }
codeunit 50409 "Z Rec" { Subtype = Test;
    [Test] procedure T() var S: Codeunit "Z Steps"; begin if S.Run('rec') <> '42' then Error('other'); end; }
codeunit 50410 "Z Modify" { Subtype = Test;
    [Test] procedure T() var S: Codeunit "Z Steps"; begin if S.Run('modify') <> '43' then Error('other'); end; }
"#;

fn run(codeunit: &str) -> Vec<(String, Outcome)> {
    al_testkit::outcomes_in(SRC, codeunit)
}

const MARK: &str = "xRec in a table extension's insert or delete trigger";

#[test]
fn every_reader_of_the_ext_trigger_xrec_refuses() {
    for codeunit in [
        "Z Read",
        "Z Proc",
        "Z Copy",
        "Z Write",
        "Z Delete",
        "Z Get",
        "Z Count",
        "Z Validate",
    ] {
        let o = run(codeunit);
        assert!(
            matches!(o.as_slice(), [(_, Outcome::NotRun(why))] if why.contains(MARK)),
            "{codeunit}: {o:?}"
        );
    }
}

#[test]
fn a_trigger_that_does_not_read_xrec_and_the_pinned_after_modify_run() {
    for codeunit in ["Z Rec", "Z Modify", "Z Caption"] {
        let o = run(codeunit);
        assert!(matches!(o.as_slice(), [(_, Outcome::Pass)]), "{codeunit}: {o:?}");
    }
}
