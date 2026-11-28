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

// Review round 1 of the stacked pull request: each keeps main's refusal.
// Own RDLCLayout and ExcelLayout properties, no DefaultLayout.
report 97151 "NBX Own Props Rpt"
{
    UsageCategory = None;
    RDLCLayout = './src/NbxOwn.rdl';
    ExcelLayout = './src/NbxOwn.xlsx';

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
        Site: Codeunit "NBX Site";
}

reportextension 97152 "NBX Own Props Ext" extends "NBX Own Props Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxOwnPropsExt.docx';
        }
    }
}

// An own WordLayout property.
report 97153 "NBX Own Word Prop Rpt"
{
    UsageCategory = None;
    WordLayout = './src/NbxOwnWord.docx';

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
        Site: Codeunit "NBX Site";
}

reportextension 97154 "NBX Own Word Prop Ext" extends "NBX Own Word Prop Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxOwnWordExt.docx';
        }
    }
}

// An own rendering Word layout, no DefaultRenderingLayout.
report 97155 "NBX Own Rendering Rpt"
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
        }
    }
    rendering
    {
        layout(W)
        {
            Type = Word;
            LayoutFile = './src/NbxOwnRendering.docx';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97156 "NBX Own Rendering Ext" extends "NBX Own Rendering Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxOwnRenderingExt.docx';
        }
    }
}

// The Word-layout extension has a request page trigger of its own.
report 97157 "NBX Ext Page Rpt"
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
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97158 "NBX Word Page Ext" extends "NBX Ext Page Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxWordPage.docx';
        }
    }

    requestpage
    {
        trigger OnOpenPage()
        begin
            Site.Fire(12);
        end;
    }

    var
        Site: Codeunit "NBX Site";
}

// A second extension that only extends the request page.
report 97159 "NBX Two Ext Page Rpt"
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
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97161 "NBX Word Only Ext" extends "NBX Two Ext Page Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxWordOnly.docx';
        }
    }
}

reportextension 97162 "NBX Page Only Ext" extends "NBX Two Ext Page Rpt"
{
    requestpage
    {
        trigger OnInit()
        begin
            Site.Fire(12);
        end;
    }

    var
        Site: Codeunit "NBX Site";
}

// The Word-layout extension has an OnPreReport trigger.
report 97163 "NBX Ext Trigger Rpt"
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
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97164 "NBX Word Trigger Ext" extends "NBX Ext Trigger Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxWordTrigger.docx';
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(12);
    end;

    var
        Site: Codeunit "NBX Site";
}

// The Word-layout extension has labels.
report 97165 "NBX Ext Labels Rpt"
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
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97166 "NBX Word Labels Ext" extends "NBX Ext Labels Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxWordLabels.docx';
        }
    }

    labels
    {
        XL = 'NBX label';
    }
}

// The extension adds two Word layouts.
report 97167 "NBX Ext Two Layouts Rpt"
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
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97168 "NBX Two Word Ext" extends "NBX Ext Two Layouts Rpt"
{
    rendering
    {
        layout(XA)
        {
            Type = Word;
            LayoutFile = './src/NbxTwoA.docx';
        }
        layout(XB)
        {
            Type = Word;
            LayoutFile = './src/NbxTwoB.docx';
        }
    }
}

// Two extensions: one adds the Word layout, one a column.
report 97169 "NBX Two Ext Rpt"
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
        }
    }

    trigger OnPreReport()
    begin
        Site.Fire(1);
    end;

    var
        Site: Codeunit "NBX Site";
}

reportextension 97170 "NBX Two Ext Word" extends "NBX Two Ext Rpt"
{
    rendering
    {
        layout(X)
        {
            Type = Word;
            LayoutFile = './src/NbxTwoExtWord.docx';
        }
    }
}

reportextension 97171 "NBX Two Ext Column" extends "NBX Two Ext Rpt"
{
    dataset
    {
        add(Int)
        {
            column(XN; Number) { }
        }
    }
}
