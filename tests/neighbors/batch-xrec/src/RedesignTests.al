// the inner trigger filters xRec; the outer reads the filter after (probe Validate_*InnerFilters_Observed, xRec only)
codeunit 50160 "NBX T60 InnerFilter_TwoProcs"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure InnerFilter_TwoProcs()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-inner-filter');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/NEW/I1;I filters=Series: F;');
    end;
}

// the inner trigger calls a [TryFunction] table procedure that assigns xRec then raises (probe Validate_TwoProcsInnerTryAssigns_Observed)
codeunit 50161 "NBX T61 InnerTry_TwoProcs"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure InnerTry_TwoProcs()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-inner-try');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;caught;I after x=0/TRY/I1;');
    end;
}

// two levels: Grade's trigger assigns, Mode's and Item's read it after (probe Validate_TwoLevelInnerAssigns_Observed)
codeunit 50162 "NBX T62 TwoLevel_DeepAssign"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure TwoLevel_DeepAssign()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-level-deep-assign');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;G x=0/NEW/I1;T after x=0/GRD/I1;I after x=0/GRD/I1;');
    end;
}

// two levels: Item's trigger assigns before it nests (probe Validate_TwoLevelOuterAssigns_Observed)
codeunit 50163 "NBX T63 TwoLevel_OuterAssign"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure TwoLevel_OuterAssign()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-level-outer-assign');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;G x=0/OUT/I1;T after x=0/OUT/I1;I after x=0/OUT/I1;');
    end;
}

// the outer Validate in a [TryFunction] of the test codeunit, its result used (probe Validate_FromTestTryFunction_Observed)
codeunit 50164 "NBX T64 FromTryFunction"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure FromTryFunction()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-inner-assign');
        if TryValidate(Line) then;
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/INN/I1;');
    end;

    [TryFunction]
    local procedure TryValidate(var Line: Record "NBX Shape Line")
    begin
        Line.Validate("Item No.", 'I2');
    end;
}

// the outer Validate in a [TryFunction] called as a statement: not measured
codeunit 50165 "NBX T65 FromTryFunctionStateme"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure FromTryFunctionStatement()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-inner-assign');
        TryValidate(Line);
        Setup.Check('I x=0/NEW/I1;T x=0/NEW/I1;I after x=0/INN/I1;');
    end;

    [TryFunction]
    local procedure TryValidate(var Line: Record "NBX Shape Line")
    begin
        Line.Validate("Item No.", 'I2');
    end;
}

// the outer Validate in a [MessageHandler] (probe Validate_FromMessageHandler_Observed)
codeunit 50166 "NBX T66 FromMessageHandler"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure FromMessageHandler()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-outer-assign');
        HandlerLine := Line;
        Message('validate');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    [MessageHandler]
    procedure ValidateHandler(Msg: Text[1024])
    begin
        HandlerLine.Validate("Item No.", 'I2');
    end;
}

// the outer Validate in a [ConfirmHandler]: not measured
codeunit 50167 "NBX T67 FromConfirmHandler"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    [HandlerFunctions('ValidateHandler')]
    procedure FromConfirmHandler()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('two-outer-assign');
        HandlerLine := Line;
        if Confirm('validate') then;
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;

    [ConfirmHandler]
    procedure ValidateHandler(Question: Text[1024]; var Reply: Boolean)
    begin
        HandlerLine.Validate("Item No.", 'I2');
        Reply := true;
    end;
}

// four Validates in one another: not measured (the probe has three)
codeunit 50168 "NBX T68 FourLevels"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure FourLevels()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('four-level-outer-assign');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;G x=0/OUT/I1;D x=0/OUT/I1;T after x=0/OUT/I1;I after x=0/OUT/I1;');
    end;
}

// the nested Validate in a procedure of a table extension: not measured
codeunit 50169 "NBX T69 ExtProcedureNests"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure ExtProcedureNests()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('ext-outer-assign');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;
}

// the nested Validate with its value in a Variant: not measured
codeunit 50170 "NBX T70 VariantValue"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure VariantValue()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('variant-outer-assign');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/OUT/I1;I after x=0/OUT/I1;');
    end;
}

// Validate on another record variable, a copy: its own xRec (probe Validate_OtherVarCopy_Observed)
codeunit 50171 "NBX T71 OtherVariableCopy"
{
    Subtype = Test;

    var
        Cfg: Codeunit "NBX Cfg";
        Setup: Codeunit "NBX Setup";
        Shape: Codeunit "NBX Shape Setup";
        HandlerLine: Record "NBX Shape Line";

    [Test]
    procedure OtherVariableCopy()
    var
        Line: Record "NBX Shape Line";
    begin
        Shape.AfterInsert(Line);
        Cfg.Set('other-copy');
        Line.Validate("Item No.", 'I2');
        Setup.Check('I x=0/NEW/I1;T x=0/MID/I2;I after x=0/NEW/I1;');
    end;
}
