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
// Error('literal'), 8 OnPreDataItem Error('literal'), 9 OnPreReport TestField of an empty field, 10 OnPreDataItem TestField
// with a value, 11 OnPreReport inserts a row and then Error('literal'), 12 never raises (a trigger that only calls Fire).
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
            8:
                Error('N8');
            9:
                Row.TestField(Tag);
            10:
                Row.TestField(Tag, 'X');
            11:
                begin
                    Row.Code := 'INSERTED';
                    Row.Insert();
                    Error('N11');
                end;
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
                Site.Fire(8);
                Site.Fire(10);
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
        Site.Fire(9);
        Site.Fire(11);
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

report 97113 "NBS Processing Init Rpt"
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

    trigger OnInitReport()
    begin
        Site.Fire(12);
    end;

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

report 97114 "NBS Processing Page Rpt"
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

    requestpage
    {
        trigger OnInit()
        begin
            Site.Fire(12);
        end;
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

report 97115 "NBS Excel Init Rpt"
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
            Type = Excel;
            LayoutFile = './src/NbsExcelInit.xlsx';
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
        Site: Codeunit "NBS Site";
}

report 97116 "NBS Excel Word Rpt"
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
            Type = Excel;
            LayoutFile = './src/NbsExcelWordX.xlsx';
        }
        layout(W)
        {
            Type = Word;
            LayoutFile = './src/NbsExcelWordW.docx';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

// No layout, and the data items of the pinned report (text/PnBReportSaveAs.al:698): two top-level, one inside the first.
report 97117 "NBS No Layout Pin Rpt"
{
    UsageCategory = None;

    dataset
    {
        dataitem(First; Integer)
        {
            column(A; Number) { }

            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;

            dataitem(Inner; Integer)
            {
                column(B; Number) { }

                trigger OnPreDataItem()
                begin
                    SetRange(Number, 1, 1);
                end;
            }
        }
        dataitem(Second; Integer)
        {
            column(C; Number) { }

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

report 97118 "NBS Processing Ext Page Rpt"
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

// A reportextension that only adds a request page OnInit.
reportextension 97119 "NBS Processing Page Ext" extends "NBS Processing Ext Page Rpt"
{
    requestpage
    {
        trigger OnInit()
        begin
            Site.Fire(12);
        end;
    }

    var
        Site: Codeunit "NBS Site";
}

report 97122 "NBS Word Ext Page Rpt"
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
            LayoutFile = './src/NbsWordExtPage.docx';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

// A reportextension that only adds a request page trigger.
reportextension 97123 "NBS Word Page Ext" extends "NBS Word Ext Page Rpt"
{
    requestpage
    {
        trigger OnOpenPage()
        begin
            Site.Fire(12);
        end;
    }

    var
        Site: Codeunit "NBS Site";
}

// Processing-only reports that still declare a layout: a lone WordLayout property, and a rendering Word layout without
// DefaultRenderingLayout.
report 97124 "NBS Processing Word Prop Rpt"
{
    UsageCategory = None;
    ProcessingOnly = true;
    WordLayout = './src/NbsProcessingWord.docx';

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

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

report 97125 "NBS Processing Rendering Rpt"
{
    UsageCategory = None;
    ProcessingOnly = true;

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
        layout(W)
        {
            Type = Word;
            LayoutFile = './src/NbsProcessingRendering.docx';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBS Site";
}

// A processing-only report that fails in its OnPostDataItem (not measured), run by a measured Word report's OnPreReport.
report 97126 "NBS Inner Rpt"
{
    UsageCategory = None;
    ProcessingOnly = true;
    UseRequestPage = false;

    dataset
    {
        dataitem(Int; Integer)
        {
            trigger OnPreDataItem()
            begin
                SetRange(Number, 1, 1);
            end;

            trigger OnPostDataItem()
            begin
                Error('N26');
            end;
        }
    }
}

report 97127 "NBS Word Nested Rpt"
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
            LayoutFile = './src/NbsWordNested.docx';
        }
    }

    trigger OnPreReport()
    begin
        Report.Run(Report::"NBS Inner Rpt", false);
    end;
}

table 97128 "NBS Crm Entity"
{
    TableType = CRM;

    fields
    {
        field(1; EntityId; Guid) { }
        field(2; Name; Text[100]) { }
    }

    keys
    {
        key(PK; EntityId) { }
    }
}

// A measured Word report whose OnPreReport writes a CRM row before its error.
report 97129 "NBS Word Crm Rpt"
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
            LayoutFile = './src/NbsWordCrm.docx';
        }
    }

    trigger OnPreReport()
    var
        E: Record "NBS Crm Entity";
    begin
        E.Init();
        E.EntityId := CreateGuid();
        E.Name := 'x';
        E.Insert();
        Error('N29');
    end;
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
