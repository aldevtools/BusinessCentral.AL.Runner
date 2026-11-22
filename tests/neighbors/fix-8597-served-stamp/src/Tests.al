// Neighbors of al-runner issue 8597: a company copied, then copied again from another source after Company.Delete or a
// rollback, and read through the store's caches. Each source holds 70 rows (Companies.al):
// A: keys A100..A169, Group i mod 7, Name 'Alpha', Amount i; B: keys B100..B169, Group (i + 1) mod 7, Name 'Baek',
// Amount 2 * i; C and D: the same keys K100..K169, with A's and B's Group, Name and Amount.
// A copy holds its source's rows (session-system/company/Y22CopyCompany.al:65), and a rollback takes the copy back
// (:262-288). Every test also reads a fresh copy of the last source and expects the same answer.
codeunit 50173 "N97 Tests"
{
    Subtype = Test;

    var
        Companies: Codeunit "N97 Companies";
        AGroup3Tok: Label 'A103/3;A110/3;A117/3;A124/3;A131/3;A138/3;A145/3;A152/3;A159/3;A166/3;', Locked = true;
        BGroup3Tok: Label 'B102/3;B109/3;B116/3;B123/3;B130/3;B137/3;B144/3;B151/3;B158/3;B165/3;', Locked = true;
        DGroup3Tok: Label 'K102/3;K109/3;K116/3;K123/3;K130/3;K137/3;K144/3;K151/3;K158/3;K165/3;', Locked = true;

    local procedure Name(Letter: Text; Suffix: Text): Text
    begin
        exit('N97 ' + Letter + Suffix);
    end;

    local procedure MakeSources(Suffix: Text; SameKeys: Boolean)
    begin
        if SameKeys then begin
            Companies.Make(Name('C', Suffix), 'K', 0, 'Alpha', 1);
            Companies.Make(Name('D', Suffix), 'K', 1, 'Baek', 2);
        end else begin
            Companies.Make(Name('A', Suffix), 'A', 0, 'Alpha', 1);
            Companies.Make(Name('B', Suffix), 'B', 1, 'Baek', 2);
        end;
        Commit();
    end;

    local procedure DropAll(Suffix: Text)
    begin
        Companies.Drop(Name('A', Suffix));
        Companies.Drop(Name('B', Suffix));
        Companies.Drop(Name('C', Suffix));
        Companies.Drop(Name('D', Suffix));
        Companies.Drop(Name('X', Suffix));
        Companies.Drop(Name('Y', Suffix));
        Companies.Drop(Name('R', Suffix));
        Commit();
    end;

    // The Group 3 keys of a fresh copy of Source, read in the key (Group, No).
    local procedure FreshGroupKeys(Source: Text; Suffix: Text) Result: Text
    begin
        Companies.Copy(Source, Name('R', Suffix));
        Result := Companies.GroupKeys(Name('R', Suffix), 3);
        Companies.Drop(Name('R', Suffix));
    end;

    local procedure FreshOrder(Source: Text; Suffix: Text) Result: Text
    begin
        Companies.Copy(Source, Name('R', Suffix));
        Result := Companies.SecondaryOrder(Name('R', Suffix));
        Companies.Drop(Name('R', Suffix));
    end;

    [Test]
    procedure RecopyFromOtherSourceAfterDelete_GroupInSecondaryKey()
    var
        X: Text;
    begin
        MakeSources('1', false);
        X := Name('X', '1');
        Companies.Copy(Name('A', '1'), X);
        Companies.Check('from A', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.Drop(X);
        Companies.Copy(Name('B', '1'), X);
        Companies.Check('from B', Companies.GroupKeys(X, 3), BGroup3Tok);
        Companies.Check('fresh B', Companies.GroupKeys(X, 3), FreshGroupKeys(Name('B', '1'), '1'));
        DropAll('1');
    end;

    [Test]
    procedure RecopyFromOtherSourceAfterRollback_GroupInSecondaryKey()
    var
        X: Text;
    begin
        MakeSources('2', false);
        X := Name('X', '2');
        Companies.CopyOpen(Name('A', '2'), X);
        Companies.Check('from A', Companies.GroupKeys(X, 3), AGroup3Tok);
        asserterror Error('N97 roll back');
        Companies.Check('copy after rollback', Format(Companies.Exists(X)), 'No');
        Companies.Copy(Name('B', '2'), X);
        Companies.Check('from B', Companies.GroupKeys(X, 3), BGroup3Tok);
        Companies.Check('fresh B', Companies.GroupKeys(X, 3), FreshGroupKeys(Name('B', '2'), '2'));
        DropAll('2');
    end;

    [Test]
    procedure RecopySameKeysAfterDelete_GroupFilterHoldsOnlyTheGroup()
    var
        X: Text;
    begin
        MakeSources('3', true);
        X := Name('X', '3');
        Companies.Copy(Name('C', '3'), X);
        Companies.Check('from C', Format(Companies.GroupCount(X, 3)), '10');
        Companies.Check('from C keys', CopyStr(Companies.GroupKeys(X, 3), 1, 7), 'K103/3;');
        Companies.Drop(X);
        Companies.Copy(Name('D', '3'), X);
        Companies.Check('from D', Companies.GroupKeys(X, 3), DGroup3Tok);
        Companies.Check('fresh D', Companies.GroupKeys(X, 3), FreshGroupKeys(Name('D', '3'), '3'));
        DropAll('3');
    end;

    [Test]
    procedure RecopySameKeysAfterDelete_SecondaryKeyOrder()
    var
        X: Text;
    begin
        MakeSources('4', true);
        X := Name('X', '4');
        Companies.Copy(Name('C', '4'), X);
        Companies.Check('from C', CopyStr(Companies.SecondaryOrder(X), 1, 14), 'K100/0;K107/0;');
        Companies.Drop(X);
        Companies.Copy(Name('D', '4'), X);
        Companies.Check('from D', CopyStr(Companies.SecondaryOrder(X), 1, 14), 'K106/0;K113/0;');
        Companies.Check('fresh D', Companies.SecondaryOrder(X), FreshOrder(Name('D', '4'), '4'));
        DropAll('4');
    end;

    [Test]
    procedure RecopySameKeysAfterRollback_SecondaryKeyOrder()
    var
        X: Text;
    begin
        MakeSources('5', true);
        X := Name('X', '5');
        Companies.CopyOpen(Name('C', '5'), X);
        Companies.Check('from C', CopyStr(Companies.SecondaryOrder(X), 1, 14), 'K100/0;K107/0;');
        asserterror Error('N97 roll back');
        Companies.Copy(Name('D', '5'), X);
        Companies.Check('from D', CopyStr(Companies.SecondaryOrder(X), 1, 14), 'K106/0;K113/0;');
        Companies.Check('fresh D', Companies.SecondaryOrder(X), FreshOrder(Name('D', '5'), '5'));
        DropAll('5');
    end;

    [Test]
    procedure RecopyFromOtherSource_FilteredCount()
    var
        X: Text;
    begin
        MakeSources('6', false);
        X := Name('X', '6');
        Companies.Copy(Name('A', '6'), X);
        Companies.SetGroup(Name('B', '6'), 'B100', 3);
        Commit();
        Companies.Check('from A', Format(Companies.GroupCount(X, 3)), '10');
        Companies.Drop(X);
        Companies.Copy(Name('B', '6'), X);
        Companies.Check('from B', Format(Companies.GroupCount(X, 3)), '11');
        DropAll('6');
    end;

    [Test]
    procedure RecopyFromOtherSource_CalcFields()
    var
        X: Text;
    begin
        MakeSources('7', false);
        X := Name('X', '7');
        Companies.Copy(Name('A', '7'), X);
        Companies.Check('from A', Format(Companies.GroupTotal(X)), '345');
        Companies.Drop(X);
        Companies.Copy(Name('B', '7'), X);
        Companies.Check('from B', Format(Companies.GroupTotal(X)), '670');
        DropAll('7');
    end;

    [Test]
    procedure RecopyFromOtherSource_CaseInsensitiveNameFilter()
    var
        X: Text;
    begin
        MakeSources('8', false);
        X := Name('X', '8');
        Companies.Copy(Name('A', '8'), X);
        Companies.Check('from A, @baek', Format(Companies.NameCount(X, '@baek')), '0');
        Companies.Check('from A, Baek', Format(Companies.NameCount(X, 'Baek')), '0');
        Companies.Drop(X);
        Companies.Copy(Name('B', '8'), X);
        Companies.Check('from B, @baek', Format(Companies.NameCount(X, '@baek')), '70');
        Companies.Check('from B, Baek', Format(Companies.NameCount(X, 'Baek')), '70');
        Companies.Check('from B, @alpha', Format(Companies.NameCount(X, '@alpha')), '0');
        DropAll('8');
    end;

    [Test]
    procedure CopyOnce_ReadTwice()
    var
        X: Text;
    begin
        MakeSources('10', false);
        X := Name('X', '10');
        Companies.Copy(Name('A', '10'), X);
        Companies.Check('first read', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.Check('second read', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.Check('count', Format(Companies.GroupCount(X, 3)), '10');
        DropAll('10');
    end;

    [Test]
    procedure RecopyFromSameSource()
    var
        X: Text;
    begin
        MakeSources('11', false);
        X := Name('X', '11');
        Companies.Copy(Name('A', '11'), X);
        Companies.Check('first copy', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.Drop(X);
        Companies.Copy(Name('A', '11'), X);
        Companies.Check('second copy', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.Check('fresh A', Companies.GroupKeys(X, 3), FreshGroupKeys(Name('A', '11'), '11'));
        DropAll('11');
    end;

    [Test]
    procedure ModifyCopyAfterCopy()
    var
        X: Text;
    begin
        MakeSources('12', false);
        X := Name('X', '12');
        Companies.Copy(Name('A', '12'), X);
        Companies.Check('before modify', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.SetGroup(X, 'A103', 4);
        Companies.Check('after modify', Companies.GroupKeys(X, 3), CopyStr(AGroup3Tok, 8));
        Companies.Check('source', Companies.GroupKeys(Name('A', '12'), 3), AGroup3Tok);
        DropAll('12');
    end;

    [Test]
    procedure ModifySourceAfterCopy()
    var
        X: Text;
    begin
        MakeSources('13', false);
        X := Name('X', '13');
        Companies.Copy(Name('A', '13'), X);
        Companies.Check('before modify', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.SetGroup(Name('A', '13'), 'A103', 4);
        Companies.Check('copy', Companies.GroupKeys(X, 3), AGroup3Tok);
        Companies.Check('source', Companies.GroupKeys(Name('A', '13'), 3), CopyStr(AGroup3Tok, 8));
        DropAll('13');
    end;
}
