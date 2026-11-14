table 50940 "Probe Packing State"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Code[20]) { }
        field(2; PackageCode; Code[20]) { }
        field(3; LastScannedPackage; Code[20]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50941 "Probe Package Actions"
{
    PageType = CardPart;
    SourceTable = "Probe Packing State";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            cuegroup(Commands)
            {
                field(NewPackage; Count)
                {
                    ApplicationArea = All;

                    trigger OnDrillDown()
                    var
                        Header: Record "Probe Packing State";
                    begin
                        Rec.FilterGroup(4);
                        Header.Get(Rec.GetRangeMax(Id));
                        Rec.FilterGroup(0);
                        Header.PackageCode := 'SECOND';
                        Header.Modify(true);
                        CurrPage.Update(false);
                    end;
                }
            }
        }
    }

    var
        Count: Integer;
}

page 50942 "Probe Packing Document"
{
    PageType = Document;
    SourceTable = "Probe Packing State";
    ApplicationArea = All;

    layout
    {
        area(Content)
        {
            group(Header)
            {
                field(Id; Rec.Id) { ApplicationArea = All; }
                field(PackageCode; Rec.PackageCode) { ApplicationArea = All; }
                field(Scan; ScanValue)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Rec.LastScannedPackage := Rec.PackageCode;
                        Rec.Modify();
                        CurrPage.Update(false);
                    end;
                }
            }
            part(PackageActions; "Probe Package Actions")
            {
                ApplicationArea = All;
                SubPageLink = Id = field(Id);
                UpdatePropagation = Both;
            }
        }
    }

    var
        ScanValue: Text[20];
}

codeunit 50943 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DrilldownRefreshesTheHeaderBeforeTheNextScan()
    var
        Row: Record "Probe Packing State";
        Document: TestPage "Probe Packing Document";
        FirstShown: Text;
    begin
        Row.Id := 'ORDER';
        Row.PackageCode := 'FIRST';
        Row.Insert();
        Document.OpenEdit();
        Document.GoToRecord(Row);
        Document.PackageActions.NewPackage.DrillDown();
        FirstShown := Document.PackageCode.Value;
        Document.Scan.SetValue('SCAN');
        Row.Get('ORDER');
        if (FirstShown <> 'FIRST') or (Row.LastScannedPackage <> 'SECOND') then
            Error('Before the next scan the page showed %1. The scan used package %2.', FirstShown, Row.LastScannedPackage);
        Document.Close();
    end;
}
