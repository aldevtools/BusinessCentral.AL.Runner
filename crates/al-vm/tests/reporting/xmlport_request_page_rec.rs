//! The `Rec` of an xmlport request page that declares `SourceTable` (al-runner issue 5756), pinned by
//! xmlport/lt-y-xmlport-props/YXmlPortProps.al (xmlports 88570 and 88571 at :421 and :477): RS_XmlPortRequestPageRec_Export (:861),
//! RS_XmlPortRequestPageRec_Import (:885) and RS_XmlPortTemporaryRequestPageRec_Export (:905). On Export and Import no request page
//! opens; Rec is an unfiltered, unpositioned record of the table, Modify answers false, Insert writes the row before the export
//! reads the table, OnPostXmlPort sees the same Rec, and with SourceTableTemporary the row stays out of the table.

use al_testkit::text;

const SRC: &str = r#"
table 69980 "RQ Item" { fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } } keys { key(PK; Id) { } } }
table 69985 "RQ Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
codeunit 69981 "RQ State"
{
    SingleInstance = true;
    var Log: Text;
    procedure Put(Value: Text) begin Log += Value + ';'; end;
    procedure Get(): Text begin exit(Log); end;
}
xmlport 69982 "RQ Port"
{
    Format = Xml;
    Direction = Both;
    schema
    {
        textelement(Root)
        {
            tableelement(Rw; "RQ Item")
            {
                MinOccurs = Zero;
                fieldelement(Id; Rw.Id) { }
            }
        }
    }
    requestpage
    {
        SourceTable = "RQ Item";
        trigger OnOpenPage()
        var State: Codeunit "RQ State";
        begin
            State.Put('open');
            Rec.Modify();
        end;
    }
    trigger OnPreXmlPort()
    var State: Codeunit "RQ State";
    begin
        State.Put('pre ' + RecText() + ' filters=[' + Rec.GetFilters() + '] count=' + Format(Rec.Count()) + ' temporary=' + Format(Rec.IsTemporary()) + ' table=' + Rec.TableName());
        State.Put('modify ' + Format(Rec.Modify()));
        Rec.Init();
        Rec.Id := 50;
        Rec.Name := 'xp';
        State.Put('insert ' + Format(Rec.Insert()));
    end;
    trigger OnPostXmlPort()
    var State: Codeunit "RQ State";
    begin
        State.Put('post ' + RecText() + ' count=' + Format(Rec.Count()));
    end;
    procedure RecText(): Text
    begin
        exit('[' + Format(Rec.Id) + '/' + Rec.Name + ']');
    end;
}
xmlport 69983 "RQ Temp Port"
{
    Format = Xml;
    Direction = Export;
    schema
    {
        textelement(Root)
        {
            tableelement(Rw; "RQ Item")
            {
                MinOccurs = Zero;
                fieldelement(Id; Rw.Id) { }
            }
        }
    }
    requestpage
    {
        SourceTable = "RQ Item";
        SourceTableTemporary = true;
    }
    trigger OnPreXmlPort()
    var State: Codeunit "RQ State";
    begin
        Rec.Init();
        Rec.Id := 51;
        Rec.Name := 'temp';
        State.Put('insert ' + Format(Rec.Insert()));
        State.Put('pre [' + Format(Rec.Id) + '/' + Rec.Name + '] count=' + Format(Rec.Count()) + ' temporary=' + Format(Rec.IsTemporary()));
    end;
}
codeunit 69984 C
{
    local procedure Seed()
    var I: Record "RQ Item";
    begin
        I.Id := 1; I.Name := 'one'; I.Insert();
        I.Id := 2; I.Name := 'two'; I.Insert();
    end;

    local procedure Rows(): Text
    var I: Record "RQ Item"; R: Text;
    begin
        if I.FindSet() then
            repeat
                R += Format(I.Id) + '/' + I.Name + ';';
            until I.Next() = 0;
        exit(R);
    end;

    local procedure ExportText(Id: Integer): Text
    var B: Record "RQ Blob" temporary; OutS: OutStream; InS: InStream; Line: Text; R: Text;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Id, OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            R += Line + '|';
        end;
        exit(R);
    end;

    procedure Export(): Text
    var State: Codeunit "RQ State"; X: Text;
    begin
        Seed();
        X := ExportText(69982);
        exit(X + ' ' + State.Get() + ' ' + Rows());
    end;

    [TryFunction]
    local procedure TryExport(Id: Integer; var R: Text)
    begin
        R := ExportText(Id);
    end;

    procedure ExportInTry(): Text
    var State: Codeunit "RQ State"; X: Text;
    begin
        Seed();
        if not TryExport(69982, X) then
            exit('error:' + GetLastErrorText());
        exit(X + ' ' + State.Get() + ' ' + Rows());
    end;

    procedure ExportTemp(): Text
    var State: Codeunit "RQ State"; X: Text;
    begin
        Seed();
        X := ExportText(69983);
        exit(X + ' ' + State.Get() + ' ' + Rows());
    end;

    procedure Import(): Text
    var State: Codeunit "RQ State"; B: Record "RQ Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        Seed();
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><Rw><Id>7</Id></Rw></Root>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(69982, InS);
        exit(State.Get() + ' ' + Rows());
    end;
}
"#;

#[test]
fn export_sees_the_request_page_rec_and_its_inserted_row() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Export")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Rw>|    <Id>1</Id>|  </Rw>|  <Rw>|    <Id>2</Id>|  </Rw>|  <Rw>|    <Id>50</Id>|  </Rw>|</Root>| \
         pre [0/] filters=[] count=2 temporary=No table=RQ Item;modify No;insert Yes;post [50/xp] count=3; 1/one;2/two;50/xp;"
    );
}

// RS_XmlPortRequestPageRec_Export runs the export inside a TryFunction whose result is used: the Insert of OnPreXmlPort is the
// export's write, not the try's.
#[test]
fn an_export_inside_a_used_tryfunction_writes_from_its_triggers() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ExportInTry")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Rw>|    <Id>1</Id>|  </Rw>|  <Rw>|    <Id>2</Id>|  </Rw>|  <Rw>|    <Id>50</Id>|  </Rw>|</Root>| \
         pre [0/] filters=[] count=2 temporary=No table=RQ Item;modify No;insert Yes;post [50/xp] count=3; 1/one;2/two;50/xp;"
    );
}

#[test]
fn import_sees_the_request_page_rec_and_its_inserted_row() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Import")),
        "pre [0/] filters=[] count=2 temporary=No table=RQ Item;modify No;insert Yes;post [50/xp] count=4; 1/one;2/two;7/;50/xp;"
    );
}

#[test]
fn a_temporary_request_page_rec_stays_out_of_the_export() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ExportTemp")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Rw>|    <Id>1</Id>|  </Rw>|  <Rw>|    <Id>2</Id>|  </Rw>|</Root>| \
         insert Yes;pre [51/temp] count=1 temporary=Yes; 1/one;2/two;"
    );
}
