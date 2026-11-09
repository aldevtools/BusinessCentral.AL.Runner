// Generated from corpus master format-evaluate/format-matrix/FMNumeric.al (corpus PRs 1249, 1250, 1256): one procedure per Format cell.
codeunit 50980 FMC
{
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

    procedure C0(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C1(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C2(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C3(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C4(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C5(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C6(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C7(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C8(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C9(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C10(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C11(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C12(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C13(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C14(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C15(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C16(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C17(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C18(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C19(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C20(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C21(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C22(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C23(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C24(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C25(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C26(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C27(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C28(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C29(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C30(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C31(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C32(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C33(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C34(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C35(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C36(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C37(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C38(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C39(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C40(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C41(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C42(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C43(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C44(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C45(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C46(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C47(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C48(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C49(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C50(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C51(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C52(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C53(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C54(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C55(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C56(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C57(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C58(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C59(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C60(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C61(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C62(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C63(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C64(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C65(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C66(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C67(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C68(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C69(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C70(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C71(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C72(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C73(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C74(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C75(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C76(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C77(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C78(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C79(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C80(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C81(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C82(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C83(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C84(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C85(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C86(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C87(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C88(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C89(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C90(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C91(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C92(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C93(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C94(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C95(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C96(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C97(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C98(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C99(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C100(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C101(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C102(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C103(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C104(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C105(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C106(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C107(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C108(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C109(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C110(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C111(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C112(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C113(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C114(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C115(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C116(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C117(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C118(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C119(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C120(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C121(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C122(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C123(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C124(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C125(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C126(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C127(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C128(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C129(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C130(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C131(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C132(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C133(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C134(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C135(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C136(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C137(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C138(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C139(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C140(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C141(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C142(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C143(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C144(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C145(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C146(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C147(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C148(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C149(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C150(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C151(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C152(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C153(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C154(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C155(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C156(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C157(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C158(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C159(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C160(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C161(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C162(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C163(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C164(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C165(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C166(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C167(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C168(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C169(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C170(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C171(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C172(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C173(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C174(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C175(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C176(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C177(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C178(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C179(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 1234;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C180(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C181(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C182(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C183(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C184(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C185(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C186(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C187(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C188(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C189(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C190(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C191(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C192(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C193(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C194(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C195(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C196(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C197(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C198(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C199(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C200(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C201(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C202(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C203(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C204(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C205(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C206(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C207(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C208(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C209(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C210(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C211(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C212(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C213(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C214(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C215(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C216(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C217(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C218(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C219(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C220(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C221(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C222(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C223(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C224(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := 2147483647;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C225(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C226(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C227(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C228(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C229(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C230(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C231(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C232(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C233(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C234(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C235(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C236(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C237(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C238(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C239(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C240(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C241(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C242(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C243(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C244(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C245(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C246(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C247(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C248(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C249(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C250(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C251(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C252(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C253(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C254(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C255(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C256(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C257(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C258(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C259(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C260(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C261(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C262(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C263(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C264(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C265(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C266(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C267(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C268(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C269(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1033);
        V := -2147483647-1;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C270(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C271(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C272(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C273(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C274(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C275(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C276(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C277(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C278(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C279(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C280(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C281(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C282(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C283(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C284(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C285(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C286(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C287(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C288(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C289(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C290(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C291(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C292(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C293(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C294(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C295(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C296(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C297(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C298(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C299(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C300(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C301(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C302(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C303(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C304(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C305(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C306(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C307(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C308(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C309(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C310(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C311(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C312(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C313(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C314(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C315(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C316(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C317(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C318(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C319(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C320(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C321(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C322(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C323(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C324(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C325(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C326(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C327(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C328(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C329(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C330(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C331(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C332(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C333(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C334(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C335(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C336(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C337(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C338(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C339(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C340(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C341(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C342(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C343(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C344(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C345(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C346(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C347(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C348(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C349(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C350(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C351(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C352(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C353(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C354(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C355(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C356(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C357(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C358(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C359(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := -1;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C360(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C361(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C362(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C363(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C364(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C365(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C366(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C367(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C368(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C369(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C370(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C371(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C372(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C373(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C374(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C375(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C376(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C377(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C378(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C379(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C380(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C381(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C382(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C383(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C384(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C385(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C386(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C387(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C388(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C389(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C390(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C391(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C392(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C393(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C394(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C395(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C396(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C397(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C398(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C399(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C400(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C401(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C402(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C403(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C404(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 1234567890123L;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C405(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C406(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C407(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C408(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C409(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C410(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C411(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C412(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C413(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C414(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C415(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C416(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C417(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C418(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C419(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C420(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C421(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C422(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C423(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C424(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C425(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C426(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C427(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C428(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C429(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C430(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C431(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C432(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C433(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C434(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C435(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C436(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C437(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C438(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C439(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C440(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C441(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C442(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C443(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C444(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C445(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C446(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C447(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C448(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C449(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1033);
        V := 9223372036854775807L;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C450(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C451(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C452(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C453(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C454(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C455(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C456(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C457(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C458(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C459(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C460(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C461(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C462(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C463(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C464(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C465(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C466(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C467(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C468(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C469(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C470(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C471(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C472(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C473(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C474(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C475(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C476(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C477(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C478(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C479(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C480(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C481(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C482(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C483(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C484(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C485(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C486(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C487(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C488(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C489(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C490(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C491(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C492(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C493(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C494(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C495(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C496(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C497(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C498(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C499(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C500(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C501(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C502(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C503(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C504(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C505(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C506(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C507(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C508(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C509(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C510(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C511(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C512(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C513(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C514(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C515(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C516(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C517(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C518(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C519(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C520(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C521(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C522(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C523(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C524(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C525(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C526(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C527(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C528(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C529(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C530(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C531(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C532(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C533(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C534(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C535(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C536(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C537(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C538(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C539(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.5;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C540(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C541(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C542(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C543(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C544(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C545(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C546(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C547(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C548(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C549(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C550(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C551(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C552(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C553(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C554(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C555(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C556(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C557(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C558(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C559(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C560(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C561(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C562(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C563(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C564(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C565(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C566(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C567(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C568(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C569(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C570(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C571(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C572(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C573(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C574(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C575(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C576(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C577(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C578(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C579(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C580(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C581(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C582(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C583(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C584(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -1234.5;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C585(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C586(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C587(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C588(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C589(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C590(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C591(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C592(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C593(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C594(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C595(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C596(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C597(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C598(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C599(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C600(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C601(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C602(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C603(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C604(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C605(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C606(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C607(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C608(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C609(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C610(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C611(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C612(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C613(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C614(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C615(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C616(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C617(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C618(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C619(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C620(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C621(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C622(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C623(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C624(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C625(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C626(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C627(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C628(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C629(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567.891;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C630(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C631(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C632(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C633(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C634(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C635(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C636(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C637(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C638(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C639(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C640(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C641(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C642(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C643(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C644(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C645(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C646(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C647(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C648(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C649(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C650(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C651(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C652(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C653(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C654(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C655(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C656(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C657(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C658(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C659(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C660(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C661(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C662(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C663(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C664(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C665(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C666(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C667(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C668(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C669(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C670(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C671(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C672(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C673(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C674(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 0.000001;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C675(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C676(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C677(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C678(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C679(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C680(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C681(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C682(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C683(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C684(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C685(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C686(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C687(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C688(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C689(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C690(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C691(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C692(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C693(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C694(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C695(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C696(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C697(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C698(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C699(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C700(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C701(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C702(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C703(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C704(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C705(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C706(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C707(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C708(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C709(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C710(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C711(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C712(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C713(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C714(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C715(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C716(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C717(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C718(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C719(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1234567890123.45;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C720(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C721(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C722(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C723(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C724(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C725(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C726(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C727(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C728(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C729(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C730(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C731(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C732(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C733(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C734(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C735(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C736(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C737(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C738(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C739(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C740(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C741(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C742(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C743(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C744(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C745(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C746(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C747(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C748(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C749(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C750(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C751(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C752(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C753(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C754(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C755(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C756(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C757(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C758(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C759(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C760(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C761(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C762(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C763(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C764(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := -0.5;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C765(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C766(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C767(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C768(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C769(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C770(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C771(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C772(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C773(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C774(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C775(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C776(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C777(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C778(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C779(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C780(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C781(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C782(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C783(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C784(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C785(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C786(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C787(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C788(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C789(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C790(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C791(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C792(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C793(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C794(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C795(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C796(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C797(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C798(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C799(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C800(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C801(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C802(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C803(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C804(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C805(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C806(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C807(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C808(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C809(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1033);
        V := 1000;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C810(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C811(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C812(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C813(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C814(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C815(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C816(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C817(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C818(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C819(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C820(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C821(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C822(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C823(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C824(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C825(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C826(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C827(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C828(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C829(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C830(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C831(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C832(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C833(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C834(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C835(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C836(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C837(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C838(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C839(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C840(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C841(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C842(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C843(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C844(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C845(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C846(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C847(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C848(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C849(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C850(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C851(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C852(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C853(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C854(): Text
    var
        R: Text;
        V: Integer;
    begin
        GlobalLanguage(1031);
        V := 1234;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C855(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C856(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C857(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C858(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C859(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C860(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C861(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C862(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C863(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C864(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C865(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C866(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C867(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C868(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C869(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C870(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C871(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C872(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C873(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C874(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C875(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C876(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C877(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C878(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C879(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C880(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C881(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C882(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C883(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C884(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C885(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C886(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C887(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C888(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C889(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C890(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C891(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C892(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C893(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C894(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C895(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C896(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C897(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C898(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C899(): Text
    var
        R: Text;
        V: BigInteger;
    begin
        GlobalLanguage(1031);
        V := 1234567890123L;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C900(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C901(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C902(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C903(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C904(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C905(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C906(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C907(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C908(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C909(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C910(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C911(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C912(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer Thousand>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C913(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer,6>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C914(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer,6><Filler Character,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C915(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C916(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Sign,1><Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C917(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C918(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer><Decimals,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C919(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C920(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer><Comma,.><Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C921(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer Thousand><Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C922(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer><1000Character,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C923(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer><1000Character,.><Decimals><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C924(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Sign><Integer><1000Character,.><Decimal,3><Comma,,>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C925(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer><1000Character,,><Decimal,3><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C926(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer><Decimals><Comma>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C927(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Integer Thousand><Comma,.>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C928(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C929(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Decimals,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C930(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C931(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Precision,2:2><Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C932(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Precision,0:3><Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C933(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Precision,2><Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C934(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C935(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C936(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C937(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C938(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C939(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, 'x<Integer>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C940(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C941(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C942(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C943(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C944(): Text
    var
        R: Text;
        V: Decimal;
    begin
        GlobalLanguage(1031);
        V := 1234567.891;
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;
}
