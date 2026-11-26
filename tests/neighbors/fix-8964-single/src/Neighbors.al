// Neighbors of the registers of corpus PR 1599 (ClearSharedCodeunitNestedSingleInstance.al): each changes one fact of a
// measured register or holds it in a cell the probe does not measure. Every test ends in Error(state), so a test that
// reaches the end fails; each one is not run on main and on head.

codeunit 86370 "NKS SI D4"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86371 "NKS D4 M3"
{
    var
        Next: Codeunit "NKS SI D4";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86372 "NKS D4 M2"
{
    var
        Next: Codeunit "NKS D4 M3";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86373 "NKS D4 M1"
{
    var
        Next: Codeunit "NKS D4 M2";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86374 "NKS R D4"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS D4 M1";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86375 "NKS O D4"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R D4";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R D4")
    begin
        Reg := R;
    end;
}

codeunit 86376 "NKS SI PO"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86377 "NKS R PO"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS SI PO";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86378 "NKS O PO"
{
    var
        Reg: Codeunit "NKS R PO";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R PO")
    begin
        Reg := R;
    end;
}

codeunit 86379 "NKS SI Dict"
{
    SingleInstance = true;

    var
        N: Integer;
        Map: Dictionary of [Text, Integer];

    procedure SetN(V: Integer)
    begin
        N := V;
        Map.Set('k', V);
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86380 "NKS R Dict"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS SI Dict";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86381 "NKS O Dict"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R Dict";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R Dict")
    begin
        Reg := R;
    end;
}

codeunit 86382 "NKS SI Read"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86383 "NKS R Read"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS SI Read";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86384 "NKS O Read"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R Read";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R Read")
    begin
        Reg := R;
    end;
}

codeunit 86385 "NKS SI Cyc"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86386 "NKS Cyc"
{
    var
        Back: Codeunit "NKS R Cyc";
        N: Integer;

    procedure Link(var R: Codeunit "NKS R Cyc")
    begin
        Back := R;
    end;
}

codeunit 86387 "NKS R Cyc"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS SI Cyc";
        Cyc: Codeunit "NKS Cyc";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;

    procedure LinkSelf(var Me: Codeunit "NKS R Cyc")
    begin
        Cyc.Link(Me);
    end;
}

codeunit 86388 "NKS O Cyc"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R Cyc";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R Cyc")
    begin
        Reg := R;
    end;
}

codeunit 86389 "NKS List D2"
{
    var
        N: Integer;
        Names: List of [Text];

    procedure SetN(V: Integer)
    begin
        N := V;
        Names.Add('a');
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86390 "NKS List Mid"
{
    var
        Next: Codeunit "NKS List D2";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86391 "NKS R List"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS List Mid";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86392 "NKS O List"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R List";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R List")
    begin
        Reg := R;
    end;
}

codeunit 86393 "NKS SI Out"
{
    SingleInstance = true;

    var
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86394 "NKS R Out"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS SI Out";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;

    procedure GetChild(var C: Codeunit "NKS SI Out")
    begin
        C := Child;
    end;
}

codeunit 86395 "NKS O Out"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R Out";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R Out")
    begin
        Reg := R;
    end;
}

codeunit 86396 "NKS Rec Child"
{
    var
        Cached: Record "CKK Row";
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
        if Cached.FindFirst() then;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86397 "NKS R Rec"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS Rec Child";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86398 "NKS O Rec"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R Rec";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R Rec")
    begin
        Reg := R;
    end;
}

codeunit 86399 "NKS SI Max"
{
    SingleInstance = true;

    var
        N: Integer;
        Lbl: Label 'max', MaxLength = 5;

    procedure SetN(V: Integer)
    begin
        N := V;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86400 "NKS R Max"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS SI Max";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86401 "NKS O Max"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R Max";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R Max")
    begin
        Reg := R;
    end;
}

codeunit 86402 "NKS IntList"
{
    var
        N: Integer;
        Nums: List of [Integer];

    procedure SetN(V: Integer)
    begin
        N := V;
        Nums.Add(V);
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86403 "NKS R IntList"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS IntList";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86404 "NKS O IntList"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R IntList";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R IntList")
    begin
        Reg := R;
    end;
}

codeunit 86405 "NKS Deep Rec"
{
    var
        Cached: Record "CKK Row";
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
        Cached."No." := V;
    end;

    procedure State(): Text
    begin
        exit(Format(N));
    end;
}

codeunit 86406 "NKS DR M3"
{
    var
        Next: Codeunit "NKS Deep Rec";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86407 "NKS DR M2"
{
    var
        Next: Codeunit "NKS DR M3";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86408 "NKS DR M1"
{
    var
        Next: Codeunit "NKS DR M2";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86409 "NKS R DR"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "NKS DR M1";
        Entry: Integer;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86410 "NKS O DR"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R DR";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R DR")
    begin
        Reg := R;
    end;
}

// Register-level kinds the 1599 registers do not declare, beside a SingleInstance codeunit (al-runner issue 9176).
codeunit 86411 "NKS R RCL"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "CKS SI PU";
        Entry: Integer;
        Caption: Label 'Register', Comment = 'Register caption';

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86412 "NKS O RCL"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R RCL";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R RCL")
    begin
        Reg := R;
    end;
}

codeunit 86413 "NKS R RPL"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "CKS SI PU";
        Entry: Integer;
        Caption: Label 'Register';

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86414 "NKS O RPL"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R RPL";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R RPL")
    begin
        Reg := R;
    end;
}

codeunit 86415 "NKS R RDec"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "CKS SI PU";
        Entry: Integer;
        Amount: Decimal;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
        Amount := 2.5;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86416 "NKS O RDec"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R RDec";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R RDec")
    begin
        Reg := R;
    end;
}

// A codeunit that is not SingleInstance three levels below the register, with a record or a Comment Label
// (al-runner issue 9176): 1599 has only SingleInstance codeunits at that depth.
codeunit 86417 "NKS P3 Rec"
{
    var
        Cached: Record "CKK Row";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Cached."No." := V;
    end;

    procedure State(): Text
    begin
        exit(Format(I));
    end;
}

codeunit 86418 "NKS P3 Rec M2"
{
    var
        Next: Codeunit "NKS P3 Rec";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86419 "NKS P3 Rec M1"
{
    var
        Next: Codeunit "NKS P3 Rec M2";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86420 "NKS R P3Rec"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "CKS SI PU";
        Entry: Integer;
        Chain: Codeunit "NKS P3 Rec M1";

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
        Chain.SetN(V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86421 "NKS O P3Rec"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R P3Rec";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R P3Rec")
    begin
        Reg := R;
    end;
}

codeunit 86422 "NKS P3 Lbl"
{
    var
        Caption: Label 'Leaf', Comment = 'Leaf caption';
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        
    end;

    procedure State(): Text
    begin
        exit(Format(I));
    end;
}

codeunit 86423 "NKS P3 Lbl M2"
{
    var
        Next: Codeunit "NKS P3 Lbl";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86424 "NKS P3 Lbl M1"
{
    var
        Next: Codeunit "NKS P3 Lbl M2";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Next.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Next.State()));
    end;
}

codeunit 86425 "NKS R P3Lbl"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        Child: Codeunit "CKS SI PU";
        Entry: Integer;
        Chain: Codeunit "NKS P3 Lbl M1";

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        Child.SetN(V);
        Entry := 5;
        Chain.SetN(V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 child=%2 entry=%3', Location."No.", Child.State(), Entry));
    end;
}

codeunit 86426 "NKS O P3Lbl"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "NKS R P3Lbl";

    trigger OnRun()
    begin
        ClearAll();
    end;

    procedure SetReg(var R: Codeunit "NKS R P3Lbl")
    begin
        Reg := R;
    end;
}

codeunit 86430 "NKS Neighbor Test"
{
    Subtype = Test;

    [Test]
    procedure SingleInstance_DepthFour()
    var
        Reg: Codeunit "NKS R D4";
        Owner: Codeunit "NKS O D4";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure SingleInstance_PlainOwner()
    var
        Reg: Codeunit "NKS R PO";
        Owner: Codeunit "NKS O PO";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run();
        Error(Reg.State());
    end;

    [Test]
    procedure SingleInstance_Dictionary()
    var
        Reg: Codeunit "NKS R Dict";
        Owner: Codeunit "NKS O Dict";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure SingleInstance_OnlyRead()
    var
        Reg: Codeunit "NKS R Read";
        Owner: Codeunit "NKS O Read";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Peek();
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure SingleInstance_WithBackLink()
    var
        Reg: Codeunit "NKS R Cyc";
        Owner: Codeunit "NKS O Cyc";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Reg.LinkSelf(Reg);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedList_DepthTwo()
    var
        Reg: Codeunit "NKS R List";
        Owner: Codeunit "NKS O List";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure SingleInstance_VariableHandedOut()
    var
        Reg: Codeunit "NKS R Out";
        Owner: Codeunit "NKS O Out";
        Row: Record "CKK Row";
        Ext: Codeunit "NKS SI Out";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Reg.GetChild(Ext);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedRecord_ReadRow()
    var
        Reg: Codeunit "NKS R Rec";
        Owner: Codeunit "NKS O Rec";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Row.Insert();
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure SingleInstance_LabelMaxLength()
    var
        Reg: Codeunit "NKS R Max";
        Owner: Codeunit "NKS O Max";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedList_OfInteger()
    var
        Reg: Codeunit "NKS R IntList";
        Owner: Codeunit "NKS O IntList";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedRecord_DepthFour()
    var
        Reg: Codeunit "NKS R DR";
        Owner: Codeunit "NKS O DR";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;
}

// Its own codeunit: a refusal in "NKS Neighbor Test" would keep the later tests there from running.
codeunit 86429 "NKS Narrow Test"
{
    Subtype = Test;

    [Test]
    procedure NestedSI_RegisterCommentLabel()
    var
        Reg: Codeunit "NKS R RCL";
        Owner: Codeunit "NKS O RCL";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedSI_RegisterPlainLabel()
    var
        Reg: Codeunit "NKS R RPL";
        Owner: Codeunit "NKS O RPL";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedSI_RegisterDecimal()
    var
        Reg: Codeunit "NKS R RDec";
        Owner: Codeunit "NKS O RDec";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedSI_DepthThreePlainWithRecord()
    var
        Reg: Codeunit "NKS R P3Rec";
        Owner: Codeunit "NKS O P3Rec";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;

    [Test]
    procedure NestedSI_DepthThreePlainWithCommentLabel()
    var
        Reg: Codeunit "NKS R P3Lbl";
        Owner: Codeunit "NKS O P3Lbl";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Error(Reg.State());
    end;
}
