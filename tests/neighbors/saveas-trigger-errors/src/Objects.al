// Neighbors of an error a report trigger raises inside Report.SaveAs (issue 8486, item 3). The site codeunit holds where the
// error is raised; every report calls Site.Fire at each of its triggers.
table 97100 "NBS Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Tag; Code[20]) { }
        field(3; Data; Blob) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

// Site: 0 none, 1 OnPreReport Error('literal'), 2 OnAfterGetRecord Error('literal'), 3 OnPreDataItem TestField of an empty
// field, 4 OnPostReport Error('literal'), 5 OnPreReport Error(Label), 6 OnPreReport Get of a missing row, 7 OnPostDataItem
// Error('literal').
codeunit 97101 "NBS Site"
{
    SingleInstance = true;

    var
        Site: Integer;
        Rep: Integer;
        Fmt: ReportFormat;
        LabelErr: Label 'NBS label error';

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
    var
        Row: Record "NBS Row";
    begin
        if Site <> Where then
            exit;
        case Where of
            1:
                Error('N1');
            2:
                Error('N2');
            3:
                Row.TestField(Tag);
            4:
                Error('N4');
            5:
                Error(LabelErr);
            6:
                Row.Get('MISSING');
            7:
                Error('N7');
        end;
    end;
}

report 97102 "NBS Word Rpt"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
                Site.Fire(3);
            end;

            trigger OnAfterGetRecord()
            begin
                Site.Fire(2);
            end;

            trigger OnPostDataItem()
            begin
                Site.Fire(7);
            end;
        }
    }

    rendering
    {
        layout(L)
        {
            Type = Word;
            LayoutFile = './src/NbsWord.docx';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
        Site.Fire(5);
        Site.Fire(6);
    end;

    trigger OnPostReport()
    begin
        Site.Fire(4);
    end;

    var
        Site: Codeunit "NBS Site";
}

report 97103 "NBS Rdlc Rpt"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;
        }
    }

    rendering
    {
        layout(L)
        {
            Type = RDLC;
            LayoutFile = './src/NbsRdlc.rdl';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

report 97104 "NBS Rdlc Col Rpt"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;
        }
    }

    rendering
    {
        layout(L)
        {
            Type = RDLC;
            LayoutFile = './src/NbsRdlcCol.rdl';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

reportextension 97105 "NBS Rdlc Col Ext" extends "NBS Rdlc Col Rpt"
{
    dataset
    {
        add(Int)
        {
            column(XN; Number) { }
        }
    }
}

report 97106 "NBS Excel Rpt"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;

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

    rendering
    {
        layout(L)
        {
            Type = Excel;
            LayoutFile = './src/NbsExcel.xlsx';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

report 97107 "NBS Processing Rpt"
{
    UsageCategory = None;
    ProcessingOnly = true;

    dataset
    {
        dataitem(Int; Integer)
        {
            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

report 97108 "NBS No Layout Rpt"
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
        Site: Codeunit "NBS Site";
}

report 97109 "NBS Word Two Items Rpt"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;

    dataset
    {
        dataitem(First; Integer)
        {
            column(A; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;
        }
        dataitem(Second; Integer)
        {
            column(B; Number) { }

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

    rendering
    {
        layout(L)
        {
            Type = Word;
            LayoutFile = './src/NbsWordTwo.docx';
        }
    }

    var
        Site: Codeunit "NBS Site";
}

report 97110 "NBS Word Ext Rpt"
{
    UsageCategory = None;
    DefaultRenderingLayout = L;

    dataset
    {
        dataitem(Int; Integer)
        {
            column(N; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;
        }
    }

    rendering
    {
        layout(L)
        {
            Type = Word;
            LayoutFile = './src/NbsWordExt.docx';
        }
    }
}

// An extension with a trigger of its own: the error comes from the extension, not from the report.
reportextension 97111 "NBS Word Trigger Ext" extends "NBS Word Ext Rpt"
{
    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

// The statement form, run inside Codeunit.Run so the raised error's code and text can be read afterwards.
codeunit 97112 "NBS Statement"
{
    trigger OnRun()
    var
        Site: Codeunit "NBS Site";
        TempRow: Record "NBS Row" temporary;
        OutS: OutStream;
    begin
        TempRow.Data.CreateOutStream(OutS);
        Report.SaveAs(Site.GetRep(), '', Site.GetFmt(), OutS);
    end;
}
