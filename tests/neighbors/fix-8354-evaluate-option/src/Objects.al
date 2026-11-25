table 71900 "NB8354 Row"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Reporter; Option) { OptionMembers = First,Second,Third,,,Skip; OptionCaption = 'First one,Second one,Third one,,,Skip it'; }
        field(3; FirstEmpty; Option) { OptionMembers = "",A,B; OptionCaption = ',A cap,B cap'; }
        field(4; EmptyNamedCap; Option) { OptionMembers = "",A,B; OptionCaption = 'Blank0,A cap,B cap'; }
        field(5; SpaceEmptyCap; Option) { OptionMembers = " ",A,B; OptionCaption = ',A cap,B cap'; }
        field(6; NamedEmptyCap; Option) { OptionMembers = None,A,B; OptionCaption = ',A cap,B cap'; }
        field(7; NamedSpaceCapMid; Option) { OptionMembers = A,None,B; OptionCaption = 'A cap, ,B cap'; }
        field(8; EmptySpaceCapMid; Option) { OptionMembers = A,"",B; OptionCaption = 'A cap, ,B cap'; }
        field(9; Pinned; Option) { OptionMembers = " ",Alpha,Beta; OptionCaption = ' ,Alpha cap,Beta cap'; }
        field(10; NoBlank; Option) { OptionMembers = A,B,C; OptionCaption = 'One,Two,Three'; }
        field(11; NoCaption; Option) { OptionMembers = First,,Skip; }
        field(12; ReporterML; Option) { OptionMembers = First,,,Skip; OptionCaptionML = ENU = 'First one,,,Skip it'; }
        field(13; Pair; Option) { OptionMembers = Alpha,Beta; OptionCaption = 'Alpha,Beta'; }
        field(14; Kind; Enum "NB8354 Last") { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

enum 71901 "NB8354 Last"
{
    Extensible = false;
    value(0; Alpha) { Caption = 'Alpha cap'; }
    value(5; Beta) { Caption = 'Beta cap'; }
    value(10; Blank) { Caption = ''; }
}

enum 71902 "NB8354 Space Mid"
{
    Extensible = false;
    value(0; Alpha) { Caption = 'Alpha cap'; }
    value(1; None) { Caption = ' '; }
    value(2; Beta) { Caption = 'Beta cap'; }
}

enum 71903 "NB8354 Two Blank"
{
    Extensible = false;
    value(0; Alpha) { Caption = 'Alpha cap'; }
    value(1; N1) { Caption = ''; }
    value(2; N2) { Caption = ' '; }
    value(3; Beta) { Caption = 'Beta cap'; }
}

enum 71904 "NB8354 Empty Mid"
{
    Extensible = true;
    value(0; Alpha) { Caption = 'Alpha cap'; }
    value(3; Blank) { Caption = ''; }
    value(7; Beta) { Caption = 'Beta cap'; }
}

enum 71905 "NB8354 First Space"
{
    Extensible = true;
    value(0; None) { Caption = ' '; }
    value(1; First) { Caption = 'First cap'; }
}

enum 71906 "NB8354 First Empty"
{
    Extensible = true;
    value(0; None) { Caption = ''; }
    value(1; First) { Caption = 'First cap'; }
}
