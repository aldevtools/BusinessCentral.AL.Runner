table 50300 "AKN Entry"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Grp; Integer) { }
        field(3; X; Integer) { }
        field(4; Y; Integer) { }
        field(5; Z; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByGrp; Grp) { }
    }
}

codeunit 50301 "AKN Tests"
{
    Subtype = Test;

    local procedure Fill(Rows: Integer)
    var
        E: Record "AKN Entry";
        i: Integer;
    begin
        E.DeleteAll();
        for i := 1 to Rows do begin
            E.Id := i;
            E.Grp := i mod 4;
            E.X := (i * 7) mod 50;
            E.Y := (i * 13) mod 50;
            E.Z := i mod 5;
            E.Insert();
        end;
    end;

    local procedure KeyValue(var R: Record "AKN Entry"; Field: Integer): Integer
    begin
        case Field of
            3:
                exit(R.X);
            4:
                exit(R.Y);
            2:
                exit(R.Grp);
        end;
        exit(R.Id);
    end;

    // Reads every row of R forward (or backward) and fails when a row is out of the order (Field, Id); returns the count.
    local procedure Read(var R: Record "AKN Entry"; Field: Integer; Forward: Boolean): Integer
    var
        PrevKey: Integer;
        PrevId: Integer;
        n: Integer;
        Found: Boolean;
        Step: Integer;
    begin
        if Forward then begin
            Found := R.FindSet();
            Step := 1;
        end else begin
            Found := R.FindLast();
            Step := -1;
        end;
        if not Found then
            exit(0);
        repeat
            if n > 0 then
                if Forward then begin
                    if (KeyValue(R, Field) < PrevKey) or ((KeyValue(R, Field) = PrevKey) and (R.Id < PrevId)) then
                        Error('row %1 out of order', R.Id);
                end else
                    if (KeyValue(R, Field) > PrevKey) or ((KeyValue(R, Field) = PrevKey) and (R.Id > PrevId)) then
                        Error('row %1 out of order', R.Id);
            PrevKey := KeyValue(R, Field);
            PrevId := R.Id;
            n += 1;
        until R.Next(Step) = 0;
        exit(n);
    end;

    local procedure Expect(Got: Integer; Want: Integer; What: Text)
    begin
        if Got <> Want then
            Error('%1: %2 rows, not %3', What, Got, Want);
    end;

    // The pinned shape: two ad hoc keys on other fields, the same filters (bounding the key ByGrp), read one after the other.
    [Test]
    procedure TwoAdhocKeys_SameFilters_Forward()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
    begin
        Fill(200);
        A.SetCurrentKey(X);
        A.SetRange(Grp, 1);
        B.SetCurrentKey(Y);
        B.SetRange(Grp, 1);
        Expect(Read(A, 3, true), 50, 'A');
        Expect(Read(B, 4, true), 50, 'B');
    end;

    [Test]
    procedure TwoAdhocKeys_SameFilters_Backward()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
    begin
        Fill(200);
        A.SetCurrentKey(X);
        A.SetRange(Grp, 1);
        B.SetCurrentKey(Y);
        B.SetRange(Grp, 1);
        Expect(Read(A, 3, false), 50, 'A');
        Expect(Read(B, 4, false), 50, 'B');
    end;

    // The same ad hoc key on two records: the kept rows are in that key's order for both.
    [Test]
    procedure SameAdhocKey_TwoRecords()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
    begin
        Fill(200);
        A.SetCurrentKey(Y);
        A.SetRange(Grp, 1);
        B.SetCurrentKey(Y);
        B.SetRange(Grp, 1);
        Expect(Read(A, 4, true), 50, 'A');
        Expect(Read(B, 4, true), 50, 'B');
    end;

    // One ad hoc key and the primary key with the same filters: other key indexes, nothing shared.
    [Test]
    procedure AdhocKeyThenPrimaryKey()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
    begin
        Fill(200);
        A.SetCurrentKey(X);
        A.SetRange(Grp, 1);
        B.SetRange(Grp, 1);
        Expect(Read(A, 3, true), 50, 'A');
        Expect(Read(B, 1, true), 50, 'B');
    end;

    // Two ad hoc keys with other filters: their entries are apart by the filters already.
    [Test]
    procedure TwoAdhocKeys_OtherFilters()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
    begin
        Fill(200);
        A.SetCurrentKey(X);
        A.SetRange(Grp, 1);
        B.SetCurrentKey(Y);
        B.SetRange(Grp, 2);
        Expect(Read(A, 3, true), 50, 'A');
        Expect(Read(B, 4, true), 50, 'B');
    end;

    // Two ad hoc keys with the same filters, a Modify of a non-key field between the reads (a new table version).
    [Test]
    procedure TwoAdhocKeys_SameFilters_WriteBetween()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
        E: Record "AKN Entry";
    begin
        Fill(200);
        A.SetCurrentKey(X);
        A.SetRange(Grp, 1);
        B.SetCurrentKey(Y);
        B.SetRange(Grp, 1);
        Expect(Read(A, 3, true), 50, 'A');
        E.Get(1);
        E.Z := 9;
        E.Modify();
        Expect(Read(B, 4, true), 50, 'B');
        Expect(Read(A, 3, true), 50, 'A again');
    end;

    // Two ad hoc keys with the same filters on a small table (fewer than 64 rows): every row is looked at.
    [Test]
    procedure TwoAdhocKeys_SameFilters_SmallTable()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
    begin
        Fill(40);
        A.SetCurrentKey(X);
        A.SetRange(Grp, 1);
        B.SetCurrentKey(Y);
        B.SetRange(Grp, 1);
        Expect(Read(A, 3, true), 10, 'A');
        Expect(Read(B, 4, true), 10, 'B');
    end;

    // Two ad hoc keys whose filters also bound their own leading field.
    [Test]
    procedure TwoAdhocKeys_SameFilters_OwnFieldBoundToo()
    var
        A: Record "AKN Entry";
        B: Record "AKN Entry";
    begin
        Fill(200);
        A.SetCurrentKey(X);
        A.SetRange(Grp, 1);
        A.SetRange(X, 0, 49);
        B.SetCurrentKey(Y);
        B.SetRange(Grp, 1);
        B.SetRange(X, 0, 49);
        Expect(Read(A, 3, true), 50, 'A');
        Expect(Read(B, 4, true), 50, 'B');
    end;
}
