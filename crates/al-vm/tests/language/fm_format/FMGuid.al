// The helpers of corpus master format-evaluate/format-matrix/FMGuid.al (corpus PRs 1249, 1250, 1256), for fm_format_matrix.rs.
    [TryFunction]
    local procedure TryN(V: Guid; N: Integer; var R: Text)
    begin
        R := Format(V, 0, N);
    end;

    [TryFunction]
    local procedure TryS(V: Guid; S: Text; var R: Text)
    begin
        R := Format(V, 0, S);
    end;

    local procedure EmptyGuid(): Guid
    var
        G: Guid;
    begin
        exit(G);
    end;

    local procedure SomeGuid(): Guid
    var
        G: Guid;
    begin
        Evaluate(G, '{12345678-1234-5678-9ABC-DEF012345678}');
        exit(G);
    end;
