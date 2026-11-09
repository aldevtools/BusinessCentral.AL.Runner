// The helpers of corpus master format-evaluate/format-matrix/FMEnum.al (corpus PRs 1249, 1250, 1256), for fm_format_matrix.rs.
    [TryFunction]
    local procedure TryN(V: Enum "Warehouse Activity Type"; N: Integer; var R: Text)
    begin
        R := Format(V, 0, N);
    end;

    [TryFunction]
    local procedure TryS(V: Enum "Warehouse Activity Type"; S: Text; var R: Text)
    begin
        R := Format(V, 0, S);
    end;
