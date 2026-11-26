table 52890 "N6 Row" { fields { field(1; K; Code[30]) { } } keys { key(PK; K) { Clustered = true; } } }

codeunit 52891 "N6 Rows"
{
    procedure Put(K: Text)
    var
        R: Record "N6 Row";
    begin
        R.K := CopyStr(K, 1, 30);
        R.Insert();
    end;

    procedure Has(K: Text): Boolean
    var
        R: Record "N6 Row";
    begin
        exit(R.Get(K));
    end;
}

codeunit 52892 "N6 Fails"
{
    trigger OnRun()
    begin
        Error('fails');
    end;
}

