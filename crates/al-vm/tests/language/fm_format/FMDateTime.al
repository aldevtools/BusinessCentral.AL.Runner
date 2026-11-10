// The helpers of corpus master format-evaluate/format-matrix/FMDateTime.al (corpus PRs 1249, 1250, 1256), for fm_format_matrix.rs.
    [TryFunction]
    local procedure TryN(V: Variant; N: Integer; var R: Text)
    begin
        R := Format(V, 0, N);
    end;

    [TryFunction]
    local procedure TryS(V: Variant; S: Text; var R: Text)
    begin
        R := Format(V, 0, S);
    end;

    [TryFunction]
    local procedure TryDmy(D: Date; Part: Integer; var R: Integer)
    begin
        R := Date2DMY(D, Part);
    end;

    [TryFunction]
    local procedure TryDwy(D: Date; Part: Integer; var R: Integer)
    begin
        R := Date2DWY(D, Part);
    end;

    [TryFunction]
    local procedure TryDmy2Date(Day: Integer; Month: Integer; Year: Integer; var R: Date)
    begin
        R := DMY2Date(Day, Month, Year);
    end;
