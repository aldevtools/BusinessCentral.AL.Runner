// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; self-contained objects 86310 to 86369
// Probe: written against BC 27.5, 28.5 and 29.0 (al-runner issue 8964), observed first, then asserted.
// Facts measured, identical on BC 27.5, 28.5 and 29.0:
// 1. A SingleInstance codeunit nested in a shared register (depth 1, 2 and 3, also below a non-SingleInstance middle codeunit)
//    keeps its state when the owner of the register calls ClearAll. The value written before ClearAll is still there after it,
//    for the caller and for a separate variable of the same SingleInstance codeunit.
// 2. This holds whether the state was written through the register, through another variable, or not at all.
// 3. After ClearAll the register's nested SingleInstance codeunit is still the shared instance: a later write through a separate
//    variable (link.ext) shows in the register (link.reg), also at depth 3 and in the Whse. Jnl.-Register-shaped chain with
//    two SingleInstance leaves (one shared by two parents).
// 4. Records, temporary records and Labels held by the SingleInstance codeunit survive too. Locked and Comment Labels read the same.
// 5. A non-SingleInstance nested codeunit with a temporary record, List, Dictionary and Labels is reset by ClearAll
//    and is not linked to the separate variable (Kinds cells), as 1587 and 1596 measured.
// singleinstance-shared-on-purpose: the subject is a SingleInstance codeunit nested in a register that ClearAll resets.
// BC versions: 27, 28, 29
codeunit 86310 "CKS Leaf"
{
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

codeunit 86311 "CKS SI PU"
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

codeunit 86312 "CKS R D1PU"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS SI PU";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86313 "CKS O D1PU"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D1PU";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D1PU")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86314 "CKS SI PW"
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

codeunit 86315 "CKS R D1PW"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS SI PW";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86316 "CKS O D1PW"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D1PW";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D1PW")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86317 "CKS SI PE"
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

codeunit 86318 "CKS R D1PE"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS SI PE";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86319 "CKS O D1PE"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D1PE";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D1PE")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86320 "CKS SI RU"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86321 "CKS R D1U"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS SI RU";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86322 "CKS O D1U"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D1U";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D1U")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86323 "CKS SI RW"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86324 "CKS R D1W"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS SI RW";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86325 "CKS O D1W"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D1W";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D1W")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86326 "CKS SI RE"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86327 "CKS R D1E"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS SI RE";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86328 "CKS O D1E"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D1E";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D1E")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86329 "CKS SI D2U"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86330 "CKS Mid2U"
{
    var
        Next: Codeunit "CKS SI D2U";
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

codeunit 86331 "CKS R D2U"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS Mid2U";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86332 "CKS O D2U"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D2U";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D2U")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86333 "CKS SI D2W"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86334 "CKS Mid2W"
{
    var
        Next: Codeunit "CKS SI D2W";
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

codeunit 86335 "CKS R D2W"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS Mid2W";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86336 "CKS O D2W"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D2W";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D2W")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86337 "CKS SI D2E"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86338 "CKS Mid2E"
{
    var
        Next: Codeunit "CKS SI D2E";
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

codeunit 86339 "CKS R D2E"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS Mid2E";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86340 "CKS O D2E"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D2E";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D2E")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86341 "CKS SI D3U"
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

codeunit 86342 "CKS Mid3bU"
{
    var
        Next: Codeunit "CKS SI D3U";
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

codeunit 86343 "CKS Mid3aU"
{
    var
        Next: Codeunit "CKS Mid3bU";
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

codeunit 86344 "CKS R D3U"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS Mid3aU";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86345 "CKS O D3U"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D3U";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D3U")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86346 "CKS SI D3W"
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

codeunit 86347 "CKS Mid3bW"
{
    var
        Next: Codeunit "CKS SI D3W";
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

codeunit 86348 "CKS Mid3aW"
{
    var
        Next: Codeunit "CKS Mid3bW";
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

codeunit 86349 "CKS R D3W"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS Mid3aW";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86350 "CKS O D3W"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R D3W";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R D3W")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86351 "CKS SI CU"
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

codeunit 86352 "CKS SI UU"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86353 "CKS Trk U"
{
    var
        TempA: Record "CKK Row" temporary;
        TempB: Record "CKK Row" temporary;
        Cached: Record "CKK Row";
        Confirm: Codeunit "CKS SI CU";
        UOM: Codeunit "CKS SI UU";
        I: Integer;
        Lbl: Label 'trk %1', Comment = '%1 = a';

    procedure SetN(V: Integer)
    begin
        I := V;
        TempA."No." := V;
        TempA.Insert();
        Cached."No." := V;
        Confirm.SetN(V + 2);
        UOM.SetN(V + 3);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('i=%1 a=%2 b=%3 c=%4 confirm=%5 uom=%6 lbl=%7', I, TempA.Count(), TempB.Count(), Cached."No.", Confirm.State(), UOM.State(), StrLen(StrSubstNo(Lbl, 1))));
    end;

}

codeunit 86354 "CKS WMS U"
{
    var
        TempErr: Record "CKK Row" temporary;
        Loc: Record "CKK Row";
        Trk: Codeunit "CKS Trk U";
        UOM: Codeunit "CKS SI UU";
        Next: Integer;
        Lbl1: Label 'wms %1', Comment = '%1 = a';
        Lbl2: Label 'plain';

    procedure SetN(V: Integer)
    begin
        Next := V;
        TempErr."No." := V;
        TempErr.Insert();
        Loc."No." := V;
        Trk.SetN(V + 10);
        UOM.SetN(V + 4);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('next=%1 err=%2 loc=%3 trk=[%4] uom=%5 lbl=%6', Next, TempErr.Count(), Loc."No.", Trk.State(), UOM.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86355 "CKS R CHU"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Trk: Codeunit "CKS Trk U";
        WMS: Codeunit "CKS WMS U";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Trk.SetN(V);
        WMS.SetN(V + 100);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 trk=[%5] wms=[%6] entry=%7 mov=%8', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Trk.State(), WMS.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86356 "CKS O CHU"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R CHU";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R CHU")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86357 "CKS SI CW"
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

codeunit 86358 "CKS SI UW"
{
    SingleInstance = true;

    var
        N: Integer;
        Tmp: Record "CKK Row" temporary;
        Leaf: Codeunit "CKS Leaf";
        Lbl1: Label 'x %1', Comment = '%1 = a';
        Lbl2: Label 'locked', Locked = true;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%1 rows=%2 leaf=%3 lbl=%4', N, Tmp.Count(), Leaf.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86359 "CKS Trk W"
{
    var
        TempA: Record "CKK Row" temporary;
        TempB: Record "CKK Row" temporary;
        Cached: Record "CKK Row";
        Confirm: Codeunit "CKS SI CW";
        UOM: Codeunit "CKS SI UW";
        I: Integer;
        Lbl: Label 'trk %1', Comment = '%1 = a';

    procedure SetN(V: Integer)
    begin
        I := V;
        TempA."No." := V;
        TempA.Insert();
        Cached."No." := V;
        Confirm.SetN(V + 2);
        UOM.SetN(V + 3);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('i=%1 a=%2 b=%3 c=%4 confirm=%5 uom=%6 lbl=%7', I, TempA.Count(), TempB.Count(), Cached."No.", Confirm.State(), UOM.State(), StrLen(StrSubstNo(Lbl, 1))));
    end;

}

codeunit 86360 "CKS WMS W"
{
    var
        TempErr: Record "CKK Row" temporary;
        Loc: Record "CKK Row";
        Trk: Codeunit "CKS Trk W";
        UOM: Codeunit "CKS SI UW";
        Next: Integer;
        Lbl1: Label 'wms %1', Comment = '%1 = a';
        Lbl2: Label 'plain';

    procedure SetN(V: Integer)
    begin
        Next := V;
        TempErr."No." := V;
        TempErr.Insert();
        Loc."No." := V;
        Trk.SetN(V + 10);
        UOM.SetN(V + 4);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('next=%1 err=%2 loc=%3 trk=[%4] uom=%5 lbl=%6', Next, TempErr.Count(), Loc."No.", Trk.State(), UOM.State(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86361 "CKS R CHW"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Trk: Codeunit "CKS Trk W";
        WMS: Codeunit "CKS WMS W";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Trk.SetN(V);
        WMS.SetN(V + 100);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 trk=[%5] wms=[%6] entry=%7 mov=%8', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Trk.State(), WMS.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86362 "CKS O CHW"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R CHW";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R CHW")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86363 "CKS Nest U"
{
    var
        Tmp: Record "CKK Row" temporary;
        Names: List of [Text];
        Map: Dictionary of [Text, Integer];
        Lbl1: Label 'x %%1', Comment = '%%1 = a';
        Lbl2: Label 'locked', Locked = true;
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Names.Add('a');
        Map.Add('k', V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%%1 rows=%%2 names=%%3 map=%%4 lbl=%%5', N, Tmp.Count(), Names.Count(), Map.Count(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86364 "CKS R KU"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS Nest U";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86365 "CKS O KU"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R KU";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R KU")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}

codeunit 86366 "CKS Nest W"
{
    var
        Tmp: Record "CKK Row" temporary;
        Names: List of [Text];
        Map: Dictionary of [Text, Integer];
        Lbl1: Label 'x %%1', Comment = '%%1 = a';
        Lbl2: Label 'locked', Locked = true;
        N: Integer;

    procedure SetN(V: Integer)
    begin
        N := V;
        Tmp."No." := V;
        Tmp.Insert();
        Names.Add('a');
        Map.Add('k', V);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('n=%%1 rows=%%2 names=%%3 map=%%4 lbl=%%5', N, Tmp.Count(), Names.Count(), Map.Count(), StrLen(StrSubstNo(Lbl1, 1)) + StrLen(Lbl2)));
    end;

}

codeunit 86367 "CKS R KW"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Child: Codeunit "CKS Nest W";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(V: Integer)
    begin
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Child.SetN(V);
        Entry := 5;
        OnMovement := true;
    end;

    procedure Peek(): Integer
    begin
        exit(Entry);
    end;

    procedure RegisterEntry()
    begin
        Entry += 1;
        OnMovement := false;
    end;

    procedure ChildState(): Text
    begin
        exit(Child.State());
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('loc=%1 jnl=%2 bin=%3 reg=%4 child=%5 entry=%6 mov=%7', Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Child.State(), Entry, Format(OnMovement, 0, 9)));
    end;

}

codeunit 86368 "CKS O KW"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKS R KW";
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        ClearAll();
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKS R KW")
    begin
        Reg := R;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
    end;

}


codeunit 86369 "CKS Nested SI Test"
{
    Subtype = Test;

    [Test]
    procedure D1P_Untouched_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D1PU";
        Owner: Codeunit "CKS O D1PU";
        Ext: Codeunit "CKS SI PU";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=0 entry=1 mov=false');
        Observer.Expect('ext.after', '0');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=0 entry=2 mov=false');
        Observer.Expect('link.ext', '5');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=5 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D1P_Written_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D1PW";
        Owner: Codeunit "CKS O D1PW";
        Ext: Codeunit "CKS SI PW";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=10 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=10 entry=1 mov=false');
        Observer.Expect('ext.after', '10');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=10 entry=2 mov=false');
        Observer.Expect('link.ext', '5');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=5 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D1P_WrittenByOtherVar_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D1PE";
        Owner: Codeunit "CKS O D1PE";
        Ext: Codeunit "CKS SI PE";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Ext.SetN(77);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=77 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=77 entry=1 mov=false');
        Observer.Expect('ext.after', '77');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=77 entry=2 mov=false');
        Observer.Expect('link.ext', '5');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=5 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D1_Untouched_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D1U";
        Owner: Codeunit "CKS O D1U";
        Ext: Codeunit "CKS SI RU";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=n=0 rows=0 leaf=0 lbl=9 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=n=0 rows=0 leaf=0 lbl=9 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=0 rows=0 leaf=0 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=n=0 rows=0 leaf=0 lbl=9 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=1 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=n=5 rows=1 leaf=6 lbl=9 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D1_Written_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D1W";
        Owner: Codeunit "CKS O D1W";
        Ext: Codeunit "CKS SI RW";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=n=10 rows=1 leaf=11 lbl=9 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=n=10 rows=1 leaf=11 lbl=9 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=10 rows=1 leaf=11 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=n=10 rows=1 leaf=11 lbl=9 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=2 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=n=5 rows=2 leaf=6 lbl=9 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D1_WrittenByOtherVar_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D1E";
        Owner: Codeunit "CKS O D1E";
        Ext: Codeunit "CKS SI RE";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Ext.SetN(77);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=n=77 rows=1 leaf=78 lbl=9 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=n=77 rows=1 leaf=78 lbl=9 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=77 rows=1 leaf=78 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=n=77 rows=1 leaf=78 lbl=9 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=2 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=n=5 rows=2 leaf=6 lbl=9 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D2_Untouched_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D2U";
        Owner: Codeunit "CKS O D2U";
        Ext: Codeunit "CKS SI D2U";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=0/n=0 rows=0 leaf=0 lbl=9 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=0/n=0 rows=0 leaf=0 lbl=9 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=0 rows=0 leaf=0 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=0/n=0 rows=0 leaf=0 lbl=9 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=1 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=0/n=5 rows=1 leaf=6 lbl=9 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D2_Written_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D2W";
        Owner: Codeunit "CKS O D2W";
        Ext: Codeunit "CKS SI D2W";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=0/n=11 rows=1 leaf=12 lbl=9 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=0/n=11 rows=1 leaf=12 lbl=9 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=11 rows=1 leaf=12 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=0/n=11 rows=1 leaf=12 lbl=9 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=2 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=0/n=5 rows=2 leaf=6 lbl=9 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D2_WrittenByOtherVar_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D2E";
        Owner: Codeunit "CKS O D2E";
        Ext: Codeunit "CKS SI D2E";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Ext.SetN(77);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=0/n=77 rows=1 leaf=78 lbl=9 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=0/n=77 rows=1 leaf=78 lbl=9 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=77 rows=1 leaf=78 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=0/n=77 rows=1 leaf=78 lbl=9 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=2 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=0/n=5 rows=2 leaf=6 lbl=9 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D3_Untouched_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D3U";
        Owner: Codeunit "CKS O D3U";
        Ext: Codeunit "CKS SI D3U";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=0/0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=0/0/0 entry=1 mov=false');
        Observer.Expect('ext.after', '0');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=0/0/0 entry=2 mov=false');
        Observer.Expect('link.ext', '5');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=0/0/5 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure D3_Written_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R D3W";
        Owner: Codeunit "CKS O D3W";
        Ext: Codeunit "CKS SI D3W";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=0/0/12 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=0/0/12 entry=1 mov=false');
        Observer.Expect('ext.after', '12');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=0/0/12 entry=2 mov=false');
        Observer.Expect('link.ext', '5');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=0/0/5 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure Chain_Untouched_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R CHU";
        Owner: Codeunit "CKS O CHU";
        Ext: Codeunit "CKS SI UU";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=0 rows=0 leaf=0 lbl=9 lbl=5] wms=[next=0 err=0 loc=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=0 rows=0 leaf=0 lbl=9 lbl=5] uom=n=0 rows=0 leaf=0 lbl=9 lbl=10] entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=0 rows=0 leaf=0 lbl=9 lbl=5] wms=[next=0 err=0 loc=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=0 rows=0 leaf=0 lbl=9 lbl=5] uom=n=0 rows=0 leaf=0 lbl=9 lbl=10] entry=1 mov=false');
        Observer.Expect('ext.after', 'n=0 rows=0 leaf=0 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=0 rows=0 leaf=0 lbl=9 lbl=5] wms=[next=0 err=0 loc=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=0 rows=0 leaf=0 lbl=9 lbl=5] uom=n=0 rows=0 leaf=0 lbl=9 lbl=10] entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=1 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=5 rows=1 leaf=6 lbl=9 lbl=5] wms=[next=0 err=0 loc=0 trk=[i=0 a=0 b=0 c=0 confirm=0 uom=n=5 rows=1 leaf=6 lbl=9 lbl=5] uom=n=5 rows=1 leaf=6 lbl=9 lbl=10] entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure Chain_Written_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R CHW";
        Owner: Codeunit "CKS O CHW";
        Ext: Codeunit "CKS SI UW";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=114 rows=3 leaf=115 lbl=9 lbl=5] wms=[next=0 err=1 loc=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=114 rows=3 leaf=115 lbl=9 lbl=5] uom=n=114 rows=3 leaf=115 lbl=9 lbl=10] entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=114 rows=3 leaf=115 lbl=9 lbl=5] wms=[next=0 err=1 loc=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=114 rows=3 leaf=115 lbl=9 lbl=5] uom=n=114 rows=3 leaf=115 lbl=9 lbl=10] entry=1 mov=false');
        Observer.Expect('ext.after', 'n=114 rows=3 leaf=115 lbl=9');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=114 rows=3 leaf=115 lbl=9 lbl=5] wms=[next=0 err=1 loc=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=114 rows=3 leaf=115 lbl=9 lbl=5] uom=n=114 rows=3 leaf=115 lbl=9 lbl=10] entry=2 mov=false');
        Observer.Expect('link.ext', 'n=5 rows=4 leaf=6 lbl=9');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=5 rows=4 leaf=6 lbl=9 lbl=5] wms=[next=0 err=1 loc=0 trk=[i=0 a=1 b=0 c=0 confirm=122 uom=n=5 rows=4 leaf=6 lbl=9 lbl=5] uom=n=5 rows=4 leaf=6 lbl=9 lbl=10] entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure Kinds_Untouched_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R KU";
        Owner: Codeunit "CKS O KU";
        Ext: Codeunit "CKS Nest U";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%0 names=%0 map=%0 lbl=%10 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%0 names=%0 map=%0 lbl=%10 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=%0 rows=%0 names=%0 map=%0 lbl=%10');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%0 names=%0 map=%0 lbl=%10 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=%5 rows=%1 names=%1 map=%1 lbl=%10');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%0 names=%0 map=%0 lbl=%10 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure Kinds_Written_OwnerClearAll()
    var
        Observer: Codeunit "Probe Observer";
        Reg: Codeunit "CKS R KW";
        Owner: Codeunit "CKS O KW";
        Ext: Codeunit "CKS Nest W";
        Row: Record "CKK Row";
    begin
        Row."No." := 7;
        Reg.Prime(10);
        Owner.SetReg(Reg);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Observer.Observe('ext.after', Ext.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
        Ext.SetN(5);
        Observer.Observe('link.ext', Ext.State());
        Observer.Observe('link.reg', Reg.State());
        Observer.Expect('owner.after', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%1 names=%0 map=%0 lbl=%10 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('caller.after', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%1 names=%0 map=%0 lbl=%10 entry=1 mov=false');
        Observer.Expect('ext.after', 'n=%0 rows=%0 names=%0 map=%0 lbl=%10');
        Observer.Expect('caller.next', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%1 names=%0 map=%0 lbl=%10 entry=2 mov=false');
        Observer.Expect('link.ext', 'n=%5 rows=%1 names=%1 map=%1 lbl=%10');
        Observer.Expect('link.reg', 'loc=0 jnl= bin=0 reg=0 child=n=%0 rows=%1 names=%0 map=%0 lbl=%10 entry=2 mov=false');
        Observer.Verify();
    end;
}
