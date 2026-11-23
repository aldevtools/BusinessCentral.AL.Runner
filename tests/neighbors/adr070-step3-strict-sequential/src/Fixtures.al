table 50100 "N70 Row"
{
    DataClassification = SystemMetadata;
    fields { field(1; K; Code[20]) { } }
    keys { key(PK; K) { Clustered = true; } }
}

table 50101 "N70 Other Row"
{
    DataClassification = SystemMetadata;
    fields { field(1; K; Code[20]) { } }
    keys { key(PK; K) { Clustered = true; } }
}

codeunit 50102 "N70 State"
{
    SingleInstance = true;
    var Flag: Text;
    procedure SetFlag(V: Text) begin Flag := V; end;
    procedure GetFlag(): Text begin exit(Flag); end;
}

codeunit 50103 "N70 Rows"
{
    procedure Put(K: Text) var R: Record "N70 Row"; begin R.K := CopyStr(K, 1, 20); R.Insert(); end;
    procedure Count(): Integer var R: Record "N70 Row"; begin exit(R.Count()); end;
    procedure CountOther(): Integer var R: Record "N70 Other Row"; begin exit(R.Count()); end;
}
