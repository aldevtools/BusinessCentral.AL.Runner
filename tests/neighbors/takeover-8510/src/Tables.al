table 50100 "NB Src"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F2; Boolean) { }
        field(3; F3; Code[20]) { }
        field(4; F4; Text[30]) { }
        field(5; F5; Integer) { }
    }
    keys { key(PK; "No.") { } }
}

table 50101 "NB T BoolToCode"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F2; Code[5]) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}

table 50102 "NB T IntFromText"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(4; F4; Integer) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}

table 50103 "NB T LongCode"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(3; F3; Code[5]) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}

table 50104 "NB T IntSame"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(5; F5; Integer) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}

table 50105 "NB T TextSame"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(4; F4; Text[30]) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}

table 50106 "NB T DecFromInt"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(5; F5; Decimal) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}

table 50107 "NB S Removed"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F2; Boolean) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
        field(5; F5; Integer) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
        field(6; F6; Decimal) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
        field(7; F7; Boolean) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
    keys { key(PK; "No.") { } }
}

table 50108 "NB T LiveInt"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F2; Integer) { }
        field(5; F5; Integer) { }
    }
    keys { key(PK; "No.") { } }
}

table 50109 "NB T LiveDec"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(6; F6; Decimal) { }
    }
    keys { key(PK; "No.") { } }
}

table 50110 "NB T LiveDate"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(7; F7; Date) { }
    }
    keys { key(PK; "No.") { } }
}

table 50111 "NB T LiveCode"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F2; Code[5]) { }
    }
    keys { key(PK; "No.") { } }
}

table 50112 "NB T Plain"
{
    fields
    {
        field(1; "No."; Code[20]) { }
    }
    keys { key(PK; "No.") { } }
}

tableextension 50113 "NB Src Ext" extends "NB Src"
{
    fields
    {
        field(50190; ExtBool; Boolean) { }
    }
}

tableextension 50114 "NB T Plain Ext" extends "NB T Plain"
{
    fields
    {
        field(50190; ExtCode; Code[5]) { ObsoleteState = Removed; ObsoleteReason = 'Neighbor: removed field'; ObsoleteTag = '1.0'; }
    }
}
