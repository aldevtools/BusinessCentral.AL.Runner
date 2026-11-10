// The helpers of corpus master format-evaluate/format-matrix/FMCode.al (corpus PRs 1249, 1250, 1256), for fm_format_matrix.rs.
    [TryFunction]
    local procedure TryN(V: Code[20]; N: Integer; var R: Text)
    begin
        R := Format(V, 0, N);
    end;

    [TryFunction]
    local procedure TryS(V: Code[20]; S: Text; var R: Text)
    begin
        R := Format(V, 0, S);
    end;
