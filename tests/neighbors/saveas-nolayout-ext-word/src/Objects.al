// Neighbors of a report with no layout of its own whose report extension adds a Word layout, saved by Report.SaveAs
// (issue 8486, item 3; corpus pull request 1579, SaveAs_NoLayoutExtWord_*). Site: 0 none, 1 OnPreReport Error('literal'),
// 2 OnAfterGetRecord Error('literal'), 12 never raises.
table 97140 "NBX Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Data; Blob) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 97141 "NBX Site"
{
    SingleInstance = true;

    var
        Site: Integer;
        Rep: Integer;
        Fmt: ReportFormat;

    procedure Set(NewRep: Integer; NewSite: Integer; NewFmt: ReportFormat)
    begin
        Rep := NewRep;
        Site := NewSite;
        Fmt := NewFmt;
    end;

    procedure GetRep(): Integer
    begin
        exit(Rep);
    end;

    procedure GetFmt(): ReportFormat
    begin
        exit(Fmt);
    end;

    procedure Fire(Where: Integer)
    begin
        if Site <> Where then
            exit;
        case Where of
            1:
                Error('X1');
            2:
                Error('X2');
        end;
    end;
}

codeunit 97142 "NBX Statement"
{
    trigger OnRun()
    var
        Site: Codeunit "NBX Site";
        TempRow: Record "NBX Row" temporary;
        OutS: OutStream;
    begin
        TempRow.Data.CreateOutStream(OutS);
        Report.SaveAs(Site.GetRep(), '', Site.GetFmt(), OutS);
    end;
}

report 97143 "NBX No Layout Rpt"
{
    UsageCategory = None;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;

            trigger OnAfterGetRecord()
            begin
                Site.Fire(2);
            end;
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97144 "NBX Word Layout Ext" extends "NBX No Layout Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxWord.docx';
        }
    }
}

// The extension adds an RDLC layout instead.
report 97145 "NBX No Layout Rdlc Rpt"
{
    UsageCategory = None;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;

            trigger OnAfterGetRecord()
            begin
                Site.Fire(2);
            end;
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97146 "NBX Rdlc Layout Ext" extends "NBX No Layout Rdlc Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = RDLC;
            LayoutFile = './src/NbxRdlc.rdl';
        }
    }
}

// The extension adds a Word layout and a column.
report 97147 "NBX No Layout Col Rpt"
{
    UsageCategory = None;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;

            trigger OnAfterGetRecord()
            begin
                Site.Fire(2);
            end;
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97148 "NBX Word Column Ext" extends "NBX No Layout Col Rpt"
{
    dataset
    {
        add(Int)
        {
            column(XN; Number) { }
        }
    }

    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxWordCol.docx';
        }
    }
}

// The report declares OnInitReport.
report 97149 "NBX No Layout Init Rpt"
{
    UsageCategory = None;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;

            trigger OnAfterGetRecord()
            begin
                Site.Fire(2);
            end;
        }
    }

    trigger OnInitReport()
    begin
        Site.Fire(12);
    end;

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97150 "NBX Word Init Ext" extends "NBX No Layout Init Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxWordInit.docx';
        }
    }
}
