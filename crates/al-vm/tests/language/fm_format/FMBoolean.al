// The helpers of corpus master format-evaluate/format-matrix/FMBoolean.al (corpus PRs 1249, 1250, 1256), for fm_format_matrix.rs.
    [TryFunction]
    local procedure TryN(V: Boolean; N: Integer; var R: Text)
    begin
        R := Format(V, 0, N);
    end;

    [TryFunction]
    local procedure TryS(V: Boolean; S: Text; var R: Text)
    begin
        R := Format(V, 0, S);
    end;
