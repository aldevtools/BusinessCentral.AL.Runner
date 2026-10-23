//! The table element record after an export under `SourceTableView`. The export order is pinned
//! (xmlport/lt-ra-xmlport/RaXmlPortExport.al:416, :448, :474); what the record's key, order, filters and position are
//! afterwards is not, so every read of them refuses until the code sets a view of its own (Reset, SetView).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69980 "VA Line"
{
    fields { field(1; Id; Integer) { } field(2; B; Code[10]) { } }
    keys { key(PK; Id) { } key(ByB; B) { } }
}
xmlport 69981 "VA Sorting"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Line; "VA Line")
            {
                UseTemporary = true;
                SourceTableView = sorting(B);
                fieldattribute(Id; Line.Id) { }
                fieldattribute(B; Line.B) { }
            }
        }
    }
    procedure AddLine(NewId: Integer; NewB: Code[10]) begin Line.Init(); Line.Id := NewId; Line.B := NewB; Line.Insert(); end;
    procedure Key(): Text begin exit(Line.CurrentKey()); end;
    procedure Filters(): Text begin exit(Line.GetFilters()); end;
    procedure First(): Integer begin Line.FindFirst(); exit(Line.Id); end;
    procedure CurrentId(): Integer begin exit(Line.Id); end;
    procedure ResetKey(): Text begin Line.Reset(); exit(Line.CurrentKey()); end;
    procedure ResetFirst(): Integer begin Line.Reset(); Line.FindFirst(); exit(Line.Id); end;
    procedure ResetId(): Integer begin Line.Reset(); exit(Line.Id); end;
    procedure ViewKey(): Text begin Line.SetView('sorting(B)'); exit(Line.CurrentKey()); end;
    procedure CopyKey(): Text var L: Record "VA Line" temporary; begin L.Copy(Line); exit(L.CurrentKey()); end;
    procedure AssignId(): Integer var L: Record "VA Line"; begin L := Line; exit(L.Id); end;
}
xmlport 69982 "VA Post"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Line; "VA Line")
            {
                UseTemporary = true;
                SourceTableView = sorting(B);
                fieldattribute(Id; Line.Id) { }
            }
        }
    }
    trigger OnPostXmlPort() begin Seen := Line.CurrentKey(); end;
    procedure AddLine(NewId: Integer; NewB: Code[10]) begin Line.Init(); Line.Id := NewId; Line.B := NewB; Line.Insert(); end;
    var Seen: Text;
}
codeunit 69983 C
{
    local procedure Run(var P: XmlPort "VA Sorting"): Text
    var B: Codeunit "Temp Blob"; OutS: OutStream; InS: InStream; L: Text; T: Text;
    begin
        P.AddLine(1, 'C'); P.AddLine(2, 'A'); P.AddLine(3, 'B');
        B.CreateOutStream(OutS); P.SetDestination(OutS); P.Export();
        B.CreateInStream(InS); while not InS.EOS() do begin InS.ReadText(L); T += L + '|'; end; exit(T);
    end;

    procedure Exported(): Text var P: XmlPort "VA Sorting"; begin exit(Run(P)); end;
    procedure Key(): Text var P: XmlPort "VA Sorting"; begin Run(P); exit(P.Key()); end;
    procedure Filters(): Text var P: XmlPort "VA Sorting"; begin Run(P); exit(P.Filters()); end;
    procedure First(): Integer var P: XmlPort "VA Sorting"; begin Run(P); exit(P.First()); end;
    procedure CurrentId(): Integer var P: XmlPort "VA Sorting"; begin Run(P); exit(P.CurrentId()); end;
    procedure ResetKey(): Text var P: XmlPort "VA Sorting"; begin Run(P); exit(P.ResetKey()); end;
    procedure ResetFirst(): Integer var P: XmlPort "VA Sorting"; begin Run(P); exit(P.ResetFirst()); end;
    procedure ResetId(): Integer var P: XmlPort "VA Sorting"; begin Run(P); exit(P.ResetId()); end;
    procedure ViewKey(): Text var P: XmlPort "VA Sorting"; begin Run(P); exit(P.ViewKey()); end;
    procedure CopyKey(): Text var P: XmlPort "VA Sorting"; begin Run(P); exit(P.CopyKey()); end;
    procedure AssignId(): Integer var P: XmlPort "VA Sorting"; begin Run(P); exit(P.AssignId()); end;
    procedure Post()
    var P: XmlPort "VA Post"; B: Codeunit "Temp Blob"; OutS: OutStream;
    begin
        P.AddLine(1, 'C'); P.AddLine(2, 'A');
        B.CreateOutStream(OutS); P.SetDestination(OutS); P.Export();
    end;
}
codeunit 69984 "Temp Blob"
{
    var R: Record "VA Blob" temporary;
    procedure CreateOutStream(var OutS: OutStream) begin R.Data.CreateOutStream(OutS); end;
    procedure CreateInStream(var InS: InStream) begin R.Data.CreateInStream(InS, TextEncoding::UTF16); end;
}
table 69985 "VA Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

fn refused(routine: &str) {
    match run(routine) {
        Err(Signal::Unsupported(why)) => assert!(
            why.contains("after an export under SourceTableView is not pinned"),
            "{routine}: {why}"
        ),
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_export_itself_runs_in_view_order() {
    let t = text(run("Exported"));
    let at = |id: &str| t.find(&format!("Id=\"{id}\"")).unwrap_or_else(|| panic!("{t}"));
    assert!(at("2") < at("3") && at("3") < at("1"), "{t}");
}

#[test]
fn reads_of_the_view_after_the_export_refuse() {
    refused("Key");
    refused("Filters");
    refused("First");
    refused("CurrentId");
    refused("Post");
    refused("CopyKey");
    refused("AssignId");
}

#[test]
fn reset_or_setview_gives_the_record_a_view_again() {
    assert_eq!(text(run("ResetKey")), "Id");
    assert_eq!(text(run("ViewKey")), "B");
    assert!(matches!(run("ResetFirst"), Ok(al_builtins::value::Value::Integer(1))));
    // The buffer still holds what the export left there.
    refused("ResetId");
}
