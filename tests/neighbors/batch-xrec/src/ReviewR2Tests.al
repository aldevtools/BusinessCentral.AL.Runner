// an OnAfterValidate of Mode in a table extension assigns xRec
codeunit 50060 "NBX T40 InnerFieldExtHook"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure InnerFieldExtensionHookAssignsXRec()
    var
        Line: Record "NBX HookIn Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
        ValidateItem(Line, 'on');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/HOOK/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX HookIn Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}

// an OnBeforeValidate of "Item No." in a table extension assigns xRec
codeunit 50061 "NBX T41 OuterFieldExtHook"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure OuterFieldExtensionHookAssignsXRec()
    var
        Line: Record "NBX HookOut Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
        ValidateItem(Line, 'on');
        Setup.Check('I x=0/HOOK/I1;T x=0/HOOK/I1;I after x=0/HOOK/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX HookOut Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}

// an OnBeforeValidateEvent subscriber of "Item No." assigns xRec
codeunit 50062 "NBX T42 OuterFieldSubscriber"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure OuterFieldSubscriberAssignsXRec()
    var
        Line: Record "NBX SubOut Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
        ValidateItem(Line, 'on');
        Setup.Check('I x=0/SUB/I1;T x=0/SUB/I1;I after x=0/SUB/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX SubOut Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}

// an OnAfterValidateEvent subscriber of Mode assigns xRec
codeunit 50063 "NBX T43 InnerFieldSubAssign"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure InnerFieldSubscriberAssignsXRec()
    var
        Line: Record "NBX SubIn Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
        ValidateItem(Line, 'on');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/SUB/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX SubIn Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}

// the outer trigger assigns xRec, an OnBeforeValidateEvent subscriber of Mode reads it
codeunit 50064 "NBX T44 InnerFieldSubRead"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";

    [Test]
    procedure InnerFieldSubscriberReadsOuterAssignedXRec()
    var
        Line: Record "NBX SubInRead Line";
    begin
        Line.DeleteAll();
        Line.Init();
        Line."Line No." := 1;
        Line.Mode := Line.Mode::Purchase;
        Line.Series := 'OLD';
        Line."Item No." := 'I1';
        Line.Insert(false);
        Line.Series := 'NEW';
        ValidateItem(Line, 'on');
        Setup.Check('I x=0/NEW/I1;S x=0/OUT/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    local procedure ValidateItem(var Line: Record "NBX SubInRead Line"; CfgMode: Text)
    begin
        Cfg.Set(CfgMode);
        Line.Validate("Item No.", 'I2');
    end;
}
