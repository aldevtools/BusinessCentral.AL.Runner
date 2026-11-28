// The layout of the measured parent cells (DataItemNamesDottedQualifier.al at cf6884b9): parent Qty no. 3 and Amt no. 4
// (:32-33), child Note no. 3 and Price no. 4 (:54-55), the parent's own columns (:112-113), the child's (:129-130). Each
// report below flips one fact of that layout; the tests are in "NB RCS N Tests" (Neighbors.al).

table 51131 "NB RCS N PRow"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { Caption = 'N Id cap'; }
        field(2; Name; Text[30]) { Caption = 'N Name cap'; }
        field(3; Qty; Decimal) { Caption = 'N Qty cap'; DecimalPlaces = 0 : 3; }
        field(4; Amt; Decimal) { Caption = 'N Amt cap'; AutoFormatType = 1; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// The probe's child table (:46-55).
table 51132 "NB RCS N KidC"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { Caption = 'N Kid Id cap'; }
        field(2; RowId; Integer) { }
        field(3; Note; Text[30]) { Caption = 'N Note cap'; }
        field(4; Price; Decimal) { Caption = 'N Price cap'; DecimalPlaces = 0 : 4; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// Note and Price swapped: the child field with Amt's number is Text, the one with Qty's number the Decimal 0:4 (cells P7, P8).
table 51133 "NB RCS N KidT"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { Caption = 'N Kid Id cap'; }
        field(2; RowId; Integer) { }
        field(3; Price; Decimal) { Caption = 'N Price cap'; DecimalPlaces = 0 : 4; }
        field(4; Note; Text[30]) { Caption = 'N Note cap'; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// The probe's child table with Note and Price without Caption.
table 51134 "NB RCS N KidN"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { Caption = 'N Kid Id cap'; }
        field(2; RowId; Integer) { }
        field(3; Note; Text[30]) { }
        field(4; Price; Decimal) { DecimalPlaces = 0 : 4; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// Price at a number no parent field uses (cell P6).
table 51140 "NB RCS N Kid6"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { Caption = 'N Kid Id cap'; }
        field(2; RowId; Integer) { }
        field(3; Note; Text[30]) { Caption = 'N Note cap'; }
        field(5; Price; Decimal) { Caption = 'N Price cap'; DecimalPlaces = 0 : 4; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// The measured cell (:111-131).
report 51135 "NB RCS N Cell"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(CRow; "NB RCS N PRow")
        {
            column(DQty; CRow.Qty) { IncludeCaption = true; }
            column(DAmt; CRow.Amt) { IncludeCaption = true; }

            dataitem(CKid; "NB RCS N KidC")
            {
                DataItemLink = RowId = field(Id);

                column(CParentQty; CRow.Qty) { IncludeCaption = true; }
                column(CParentAmt; CRow.Amt) { IncludeCaption = true; }
                column(CNote; CKid.Note) { IncludeCaption = true; }
                column(CPrice; CKid.Price) { IncludeCaption = true; }
            }
        }
    }
}

report 51136 "NB RCS N Cell Swapped"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(CRow; "NB RCS N PRow")
        {
            column(DQty; CRow.Qty) { IncludeCaption = true; }
            column(DAmt; CRow.Amt) { IncludeCaption = true; }

            dataitem(CKid; "NB RCS N KidT")
            {
                DataItemLink = RowId = field(Id);

                column(TParentQty; CRow.Qty) { IncludeCaption = true; }
                column(TParentAmt; CRow.Amt) { IncludeCaption = true; }
                column(CNote; CKid.Note) { IncludeCaption = true; }
                column(CPrice; CKid.Price) { IncludeCaption = true; }
            }
        }
    }
}

// The parent item has no column of its own over Qty or Amt (cell P9).
report 51137 "NB RCS N Cell No Own"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(CRow; "NB RCS N PRow")
        {
            dataitem(CKid; "NB RCS N KidC")
            {
                DataItemLink = RowId = field(Id);

                column(OParentQty; CRow.Qty) { IncludeCaption = true; }
                column(OParentAmt; CRow.Amt) { IncludeCaption = true; }
                column(CNote; CKid.Note) { IncludeCaption = true; }
                column(CPrice; CKid.Price) { IncludeCaption = true; }
            }
        }
    }
}

// The parent's own columns state no property.
report 51138 "NB RCS N Cell Own NoProp"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(CRow; "NB RCS N PRow")
        {
            column(DQty; CRow.Qty) { }
            column(DAmt; CRow.Amt) { }

            dataitem(CKid; "NB RCS N KidC")
            {
                DataItemLink = RowId = field(Id);

                column(PParentQty; CRow.Qty) { IncludeCaption = true; }
                column(PParentAmt; CRow.Amt) { IncludeCaption = true; }
                column(CNote; CKid.Note) { IncludeCaption = true; }
                column(CPrice; CKid.Price) { IncludeCaption = true; }
            }
        }
    }
}

report 51139 "NB RCS N Cell NoCap"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(CRow; "NB RCS N PRow")
        {
            column(DQty; CRow.Qty) { IncludeCaption = true; }
            column(DAmt; CRow.Amt) { IncludeCaption = true; }

            dataitem(CKid; "NB RCS N KidN")
            {
                DataItemLink = RowId = field(Id);

                column(NParentQty; CRow.Qty) { IncludeCaption = true; }
                column(NParentAmt; CRow.Amt) { IncludeCaption = true; }
                column(CNote; CKid.Note) { IncludeCaption = true; }
                column(CPrice; CKid.Price) { IncludeCaption = true; }
            }
        }
    }
}

// A child name with a dot under an undotted parent (the cell has both undotted or both dotted).
report 51141 "NB RCS N Cell Dotted Kid"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(CRow; "NB RCS N PRow")
        {
            column(DQty; CRow.Qty) { IncludeCaption = true; }
            column(DAmt; CRow.Amt) { IncludeCaption = true; }

            dataitem("C. Kid"; "NB RCS N KidC")
            {
                DataItemLink = RowId = field(Id);

                column(KParentQty; CRow.Qty) { IncludeCaption = true; }
                column(KParentAmt; CRow.Amt) { IncludeCaption = true; }
                column(CNote; "C. Kid".Note) { IncludeCaption = true; }
                column(CPrice; "C. Kid".Price) { IncludeCaption = true; }
            }
        }
    }
}

report 51142 "NB RCS N Cell P6"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(CRow; "NB RCS N PRow")
        {
            column(DQty; CRow.Qty) { IncludeCaption = true; }
            column(DAmt; CRow.Amt) { IncludeCaption = true; }

            dataitem(CKid; "NB RCS N Kid6")
            {
                DataItemLink = RowId = field(Id);

                column(SParentQty; CRow.Qty) { IncludeCaption = true; }
                column(SParentAmt; CRow.Amt) { IncludeCaption = true; }
                column(CNote; CKid.Note) { IncludeCaption = true; }
                column(CPrice; CKid.Price) { IncludeCaption = true; }
            }
        }
    }
}

// The Option cell's field with its OptionCaption or OptionCaptionML changed by a tableextension (the cell has no extension).
tableextension 51143 "NB RCS N Row Ext" extends "NB RCS N Row"
{
    fields
    {
        modify(KindExt) { OptionCaption = ' ,X Open cap,X Closed cap'; }
        modify(KindExtMl) { OptionCaptionML = ENU = ' ,X Open cap,X Closed cap'; }
    }
}

report 51144 "NB RCS N Kind Ext"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(NRow; "NB RCS N Row")
        {
            column(KindExtQual; NRow.KindExt) { IncludeCaption = true; }
            column(KindExtMlQual; NRow.KindExtMl) { IncludeCaption = true; }
        }
    }
}
