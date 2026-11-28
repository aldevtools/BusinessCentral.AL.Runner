// Invented tables: targets of a TableRelation that names no field, keyed in the shapes of issue 8592 and next to them.
table 50100 "NB RFN One Code"
{
    fields
    {
        field(1; K1; Code[20]) { }
    }
}

table 50101 "NB RFN Code Text"
{
    fields
    {
        field(1; K1; Code[20]) { }
        field(2; K2; Text[30]) { }
    }
    keys
    {
        key(PK; K1, K2) { Clustered = true; }
    }
}

table 50102 "NB RFN Int Code"
{
    fields
    {
        field(1; K1; Integer) { }
        field(2; K2; Code[20]) { }
    }
    keys
    {
        key(PK; K1, K2) { Clustered = true; }
    }
}

table 50103 "NB RFN Code Int"
{
    fields
    {
        field(1; K1; Code[20]) { }
        field(2; K2; Integer) { }
    }
    keys
    {
        key(PK; K1, K2) { Clustered = true; }
    }
}

table 50104 "NB RFN Int Code10"
{
    fields
    {
        field(1; K1; Integer) { }
        field(2; K2; Code[10]) { }
    }
    keys
    {
        key(PK; K1, K2) { Clustered = true; }
    }
}

table 50105 "NB RFN Code Text50"
{
    fields
    {
        field(1; K1; Code[20]) { }
        field(2; K2; Text[50]) { }
    }
    keys
    {
        key(PK; K1, K2) { Clustered = true; }
    }
}

table 50106 "NB RFN Three"
{
    fields
    {
        field(1; K1; Integer) { }
        field(2; K2; Code[20]) { }
        field(3; K3; Integer) { }
    }
    keys
    {
        key(PK; K1, K2, K3) { Clustered = true; }
    }
}

// The source fields. The primary key is (Id, KeyCode), so KeyCode is inside its own key.
table 50110 "NB RFN Source"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; KeyCode; Code[20]) { TableRelation = "NB RFN One Code"; }
        field(10; Control; Code[20]) { TableRelation = "NB RFN One Code"; }
        field(11; CodeToCodeText; Code[20]) { TableRelation = "NB RFN Code Text"; }
        field(12; IntToIntCode; Integer) { TableRelation = "NB RFN Int Code"; }
        field(13; CodeToIntCode; Code[20]) { TableRelation = "NB RFN Int Code"; }
        field(14; CodeToCodeInt; Code[20]) { TableRelation = "NB RFN Code Int"; }
        field(15; IntToCodeInt; Integer) { TableRelation = "NB RFN Code Int"; }
        field(16; NamedK2; Code[20]) { TableRelation = "NB RFN Int Code".K2; }
        field(20; Code10ToIntCode; Code[10]) { TableRelation = "NB RFN Int Code"; }
        field(21; CodeToIntCode10; Code[20]) { TableRelation = "NB RFN Int Code10"; }
        field(22; CodeToCodeText50; Code[20]) { TableRelation = "NB RFN Code Text50"; }
        field(23; TextToIntCode; Text[20]) { TableRelation = "NB RFN Int Code"; }
        field(24; CodeToIntCodeWhere; Code[20]) { TableRelation = "NB RFN Int Code" where(K1 = const(1)); }
        field(25; CodeToThree; Code[20]) { TableRelation = "NB RFN Three"; }
        field(26; Code30ToCodeText; Code[30]) { TableRelation = "NB RFN Code Text"; }
        field(27; BigToIntCode; BigInteger) { TableRelation = "NB RFN Int Code"; }
    }
    keys
    {
        key(PK; Id, KeyCode) { Clustered = true; }
    }
}

// A source keyed on Id alone: the mixed-key relations outside the source's key.
table 50111 "NB RFN Source Plain"
{
    fields
    {
        field(1; Id; Integer) { }
        field(13; CodeToIntCode; Code[20]) { TableRelation = "NB RFN Int Code"; }
        field(15; IntToCodeInt; Integer) { TableRelation = "NB RFN Code Int"; }
    }
}

// A field inside its own (Integer, Code) key related to a two-field key.
table 50112 "NB RFN Source Own Two"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; KeyCode; Code[20]) { TableRelation = "NB RFN Int Code"; }
    }
    keys
    {
        key(PK; Id, KeyCode) { Clustered = true; }
    }
}

// A field inside its own key of another shape: (Code, Integer), the Code field first.
table 50113 "NB RFN Source Code First"
{
    fields
    {
        field(1; KeyCode; Code[20]) { TableRelation = "NB RFN One Code"; }
        field(2; Id; Integer) { }
    }
    keys
    {
        key(PK; KeyCode, Id) { Clustered = true; }
    }
}

// A field that is the whole of its own key.
table 50114 "NB RFN Source Own One"
{
    fields
    {
        field(1; KeyCode; Code[20]) { TableRelation = "NB RFN One Code"; }
    }
}
