// Generated from corpus master format-evaluate/format-matrix/FMText.al (corpus PRs 1249, 1250, 1256): one procedure per Format cell.
codeunit 50980 FMC
{
    [TryFunction]
    local procedure TryN(V: Text; N: Integer; var R: Text)
    begin
        R := Format(V, 0, N);
    end;

    [TryFunction]
    local procedure TryS(V: Text; S: Text; var R: Text)
    begin
        R := Format(V, 0, S);
    end;

    procedure C0(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C12(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C13(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Text,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C14(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Filler Character,x><Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C15(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C16(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Integer,5>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C17(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C18(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C19(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C20(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C21(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C22(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C23(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C24(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C25(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Standard Format,4>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C26(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C27(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, 'x<Text>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C28(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C29(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C30(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '';
        if TryS(V, '<Precision,2:2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C31(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C32(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C33(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C34(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C35(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C36(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C37(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C38(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C39(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C40(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C41(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C42(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
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
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C44(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Text,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C45(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Filler Character,x><Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C46(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C47(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Integer,5>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C48(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C49(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C50(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C51(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C52(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C53(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C54(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C55(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C56(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Standard Format,4>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C57(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C58(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, 'x<Text>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C59(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C60(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C61(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'abc';
        if TryS(V, '<Precision,2:2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C62(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C63(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C64(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C65(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C66(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C67(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C68(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C69(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C70(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C71(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C72(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C73(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C74(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C75(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Text,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C76(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Filler Character,x><Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C77(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C78(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Integer,5>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C79(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C80(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C81(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C82(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C83(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C84(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C85(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C86(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C87(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Standard Format,4>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C88(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C89(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, 'x<Text>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C90(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C91(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C92(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Hello World';
        if TryS(V, '<Precision,2:2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C93(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C94(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C95(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C96(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C97(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C98(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C99(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C100(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C101(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C102(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C103(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C104(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C105(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C106(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Text,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C107(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Filler Character,x><Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C108(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C109(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Integer,5>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C110(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C111(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C112(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C113(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C114(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C115(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C116(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C117(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C118(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Standard Format,4>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C119(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C120(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, 'x<Text>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C121(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C122(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C123(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := 'Ärger';
        if TryS(V, '<Precision,2:2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C124(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C125(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C126(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C127(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C128(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C129(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C130(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C131(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C132(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C133(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C134(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C135(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C136(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C137(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Text,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C138(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Filler Character,x><Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C139(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C140(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Integer,5>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C141(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C142(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C143(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C144(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C145(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C146(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C147(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C148(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C149(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Standard Format,4>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C150(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C151(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, 'x<Text>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C152(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C153(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C154(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1033);
        V := '123456789012345678901234567890';
        if TryS(V, '<Precision,2:2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C155(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 0, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C156(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 1, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C157(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 2, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C158(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 3, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C159(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 4, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C160(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 5, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C161(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 6, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C162(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 7, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C163(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 8, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C164(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 9, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C165(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryN(V, 10, R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C166(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Text>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C167(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C168(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Text,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C169(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Filler Character,x><Text,10>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C170(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Integer>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C171(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Integer,5>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C172(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Sign>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C173(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Decimals>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C174(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Day>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C175(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Hours24>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C176(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Standard Format,0>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C177(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Standard Format,1>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C178(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Standard Format,2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C179(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Standard Format,3>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C180(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Standard Format,4>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C181(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Standard Format,9>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C182(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, 'x<Text>y', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C183(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, 'literal', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C184(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;

    procedure C185(): Text
    var
        R: Text;
        V: Text;
    begin
        GlobalLanguage(1031);
        V := 'abc';
        if TryS(V, '<Precision,2:2>', R) then
            R := 'ok:' + R
        else
            R := 'err:' + GetLastErrorCode();
        GlobalLanguage(1033);
        exit(R);
    end;
}
