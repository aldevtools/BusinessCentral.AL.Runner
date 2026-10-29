// The reporter's repro of al-runner issue 6116 (green on real BC), app and test files in one source.
table 50100 "Repro Rule Line"
{
    fields
    {
        field(1; "Group Code"; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Value; Text[50]) { }
        field(4; Weight; Integer) { }
    }

    keys
    {
        key(PK; "Group Code", "Line No.")
        {
            Clustered = true;
        }
    }
}

codeunit 50102 "Repro Rule Store"
{
    procedure ReadWeight(GroupCode: Code[20]; LineNo: Integer): Integer
    var
        RuleLine: Record "Repro Rule Line";
    begin
        RuleLine.SetLoadFields(Weight);
        if not RuleLine.Get(GroupCode, LineNo) then
            exit(0);
        exit(RuleLine.Weight);
    end;

    procedure SumWeights(GroupCode: Code[20]): Integer
    var
        RuleLine: Record "Repro Rule Line";
        Total: Integer;
    begin
        RuleLine.SetRange("Group Code", GroupCode);
        RuleLine.SetLoadFields(Weight);
        if RuleLine.FindSet() then
            repeat
                Total += RuleLine.Weight;
            until RuleLine.Next() = 0;
        exit(Total);
    end;

    procedure RaiseWeights(GroupCode: Code[20]; LastLineNo: Integer)
    var
        RuleLine: Record "Repro Rule Line";
        LineNo: Integer;
    begin
        for LineNo := 1 to LastLineNo do begin
            RuleLine.Get(GroupCode, LineNo);
            RuleLine.Weight += 100;
            RuleLine.Modify();
        end;
    end;

    procedure BuildPath(GroupCode: Code[20]; Depth: Integer): Integer
    var
        ParentNode: Integer;
        i: Integer;
    begin
        for i := 1 to Depth do
            ParentNode := GetOrCreateNode(GroupCode, ParentNode, i * 31);
        exit(ParentNode);
    end;

    local procedure GetOrCreateNode(GroupCode: Code[20]; ParentNode: Integer; Hash: Integer): Integer
    var
        TreeNode: Record "Repro Tree Node";
    begin
        TreeNode.SetRange("Group Code", GroupCode);
        TreeNode.SetRange("Parent Node", ParentNode);
        TreeNode.SetRange(Hash, Hash);
        if TreeNode.FindFirst() then
            exit(TreeNode."Node ID");

        TreeNode.Init();
        TreeNode."Group Code" := GroupCode;
        TreeNode."Parent Node" := ParentNode;
        TreeNode.Hash := Hash;
        TreeNode.Insert(true);
        exit(TreeNode."Node ID");
    end;
}

table 50101 "Repro Tree Node"
{
    fields
    {
        field(1; "Group Code"; Code[20]) { }
        field(2; "Parent Node"; Integer) { }
        field(3; Hash; Integer) { }
        field(4; "Node ID"; Integer)
        {
            AutoIncrement = true;
        }
    }

    keys
    {
        key(PK; "Group Code", "Parent Node", Hash)
        {
            Clustered = true;
        }
        key(NodeID; "Group Code", "Node ID")
        {
            Unique = true;
        }
    }
}

codeunit 50103 "Repro Sql Budget Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit "Library Assert";
        BudgetMsg: Label 'SQL statements: %1 (budget %2), rows read: %3 (budget %4)', Locked = true;

    [Test]
    procedure GetOfOneRowStaysWithinBudget()
    var
        RuleStore: Codeunit "Repro Rule Store";
        StmtsBefore: BigInteger;
        RowsBefore: BigInteger;
        StmtsDelta: BigInteger;
        RowsDelta: BigInteger;
        Weight: Integer;
    begin
        InsertLines('READ1', 10);

        StmtsBefore := SessionInformation.SqlStatementsExecuted();
        RowsBefore := SessionInformation.SqlRowsRead();
        Weight := RuleStore.ReadWeight('READ1', 5);
        StmtsDelta := SessionInformation.SqlStatementsExecuted() - StmtsBefore;
        RowsDelta := SessionInformation.SqlRowsRead() - RowsBefore;

        Assert.AreEqual(5, Weight, 'Weight of line 5');
        AssertWithinBudget(StmtsDelta, 5, RowsDelta, 5);
    end;

    [Test]
    procedure FilteredFindSetStaysWithinBudget()
    var
        RuleStore: Codeunit "Repro Rule Store";
        StmtsBefore: BigInteger;
        RowsBefore: BigInteger;
        StmtsDelta: BigInteger;
        RowsDelta: BigInteger;
        Total: Integer;
    begin
        InsertLines('READ2', 10);

        StmtsBefore := SessionInformation.SqlStatementsExecuted();
        RowsBefore := SessionInformation.SqlRowsRead();
        Total := RuleStore.SumWeights('READ2');
        StmtsDelta := SessionInformation.SqlStatementsExecuted() - StmtsBefore;
        RowsDelta := SessionInformation.SqlRowsRead() - RowsBefore;

        Assert.AreEqual(55, Total, 'Sum of the weights');
        AssertWithinBudget(StmtsDelta, 5, RowsDelta, 25);
    end;

    [Test]
    procedure GetAndModifyStaysWithinBudget()
    var
        RuleLine: Record "Repro Rule Line";
        RuleStore: Codeunit "Repro Rule Store";
        StmtsBefore: BigInteger;
        RowsBefore: BigInteger;
        StmtsDelta: BigInteger;
        RowsDelta: BigInteger;
    begin
        InsertLines('WRITE1', 10);

        StmtsBefore := SessionInformation.SqlStatementsExecuted();
        RowsBefore := SessionInformation.SqlRowsRead();
        RuleStore.RaiseWeights('WRITE1', 5);
        StmtsDelta := SessionInformation.SqlStatementsExecuted() - StmtsBefore;
        RowsDelta := SessionInformation.SqlRowsRead() - RowsBefore;

        RuleLine.Get('WRITE1', 5);
        Assert.AreEqual(105, RuleLine.Weight, 'Weight of line 5 after the raise');
        AssertWithinBudget(StmtsDelta, 25, RowsDelta, 25);
    end;

    [Test]
    procedure FindFirstOrInsertStaysWithinBudget()
    var
        TreeNode: Record "Repro Tree Node";
        RuleStore: Codeunit "Repro Rule Store";
        StmtsBefore: BigInteger;
        RowsBefore: BigInteger;
        StmtsDelta: BigInteger;
        RowsDelta: BigInteger;
        LastNode: Integer;
    begin
        StmtsBefore := SessionInformation.SqlStatementsExecuted();
        RowsBefore := SessionInformation.SqlRowsRead();
        LastNode := RuleStore.BuildPath('TREE1', 5);
        StmtsDelta := SessionInformation.SqlStatementsExecuted() - StmtsBefore;
        RowsDelta := SessionInformation.SqlRowsRead() - RowsBefore;

        TreeNode.SetRange("Group Code", 'TREE1');
        Assert.AreEqual(5, TreeNode.Count(), 'Nodes of the path');
        Assert.AreNotEqual(0, LastNode, 'Node ID of the last node');
        AssertWithinBudget(StmtsDelta, 25, RowsDelta, 15);
    end;

    local procedure InsertLines(GroupCode: Code[20]; LineCount: Integer)
    var
        RuleLine: Record "Repro Rule Line";
        i: Integer;
    begin
        for i := 1 to LineCount do begin
            RuleLine.Init();
            RuleLine."Group Code" := GroupCode;
            RuleLine."Line No." := i;
            RuleLine.Value := 'Value';
            RuleLine.Weight := i;
            RuleLine.Insert();
        end;
    end;

    local procedure AssertWithinBudget(StmtsDelta: BigInteger; StmtsBudget: BigInteger; RowsDelta: BigInteger; RowsBudget: BigInteger)
    begin
        Assert.IsTrue(
            (StmtsDelta <= StmtsBudget) and (RowsDelta <= RowsBudget),
            StrSubstNo(BudgetMsg, StmtsDelta, StmtsBudget, RowsDelta, RowsBudget));
    end;
}

