// Copy of merged corpus PR 1596 (merge commit d2552ce6) ClearSharedCodeunitTableNoRegister.al, object ids +100 to stay clear of this app.
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/methods-auto/system/system-clearall-method
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); "CKK Row" (table 86270) of ClearSharedCodeunitGlobalKinds.al; self-contained objects 86393 to 86412
// Probe: written against BC 27.5, 28.5 and 29.0 (al-runner issue 8964), observed first, then asserted.
// Facts: a codeunit that holds a register codeunit through a setter (the register is also a variable of the caller) and
// runs ClearAll resets that shared instance for every register shape measured: with a TableNo or without, primed, only
// read, never called, or run before the share, with nested codeunits 2, 3 and 4 levels below it, and with a cycle
// (the register holds a codeunit that holds the register back). The owner and the caller see the same reset instance:
// records, counters, flags and nested codeunits are at their defaults, the first entry registered afterwards is 1.
// A Label (also with Comment) keeps its text. Cycle, primed: the nested codeunit's own Integer is reset, and the caller
// still reads the register's entry counter through the nested codeunit's back link (the back link is kept). No difference between BC 27.5, 28.5 and 29.0.
// BC versions: 27, 28, 29

codeunit 86393 "CKT Leaf"
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

codeunit 86394 "CKT M3"
{
    var
        Leaf: Codeunit "CKT Leaf";
        I: Integer;

    procedure SetN(V: Integer)
    begin
        I := V;
        Leaf.SetN(V + 1);
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Leaf.State()));
    end;
}

codeunit 86395 "CKT M2"
{
    var
        Next: Codeunit "CKT M3";
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

codeunit 86396 "CKT M1"
{
    var
        Next: Codeunit "CKT M2";
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

// "Whse. Jnl.-Register Line" shape with a TableNo: four records, two nested codeunits (depth 2 and depth 3 below it),
// an Integer, a Label with Comment, a plain Label, a Boolean.
codeunit 86397 "CKT Reg"
{
    TableNo = "CKK Row";

    var
        Location: Record "CKK Row";
        WhseJnlLine: Record "CKK Row";
        Bin: Record "CKK Row";
        WhseReg: Record "CKK Row";
        Tracking: Codeunit "CKT M3";
        WMS: Codeunit "CKT M2";
        Entry: Integer;
        Text000: Label 'is not sufficient %1 %2', Comment = '%1 = a, %2 = b';
        Text001: Label 'Serial No. %1 is found.';
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(var Self: Codeunit "CKT Reg")
    begin
        Location.SetRange("No.", 1);
        Location."No." := 1;
        WhseJnlLine.Txt := 'JNL';
        Bin."No." := 3;
        WhseReg."No." := 50;
        Tracking.SetN(10);
        WMS.SetN(20);
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
        exit(StrSubstNo('loc=%1|%2 jnl=%3 bin=%4 reg=%5 trk=%6 wms=%7 entry=%8 mov=%9 lbl=%10',
            Location.GetFilters(), Location."No.", WhseJnlLine.Txt, Bin."No.", WhseReg."No.", Tracking.State(),
            WMS.State(), Entry, Format(OnMovement, 0, 9), StrLen(StrSubstNo(Text000, 1, 2)) + StrLen(Text001)));
    end;
}

// Four nested levels below the register.
codeunit 86398 "CKT Deep Reg"
{
    TableNo = "CKK Row";

    var
        Chain: Codeunit "CKT M1";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(var Self: Codeunit "CKT Deep Reg")
    begin
        Chain.SetN(10);
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
        exit(StrSubstNo('chain=%1 entry=%2 mov=%3', Chain.State(), Entry, Format(OnMovement, 0, 9)));
    end;
}

// A cycle: the register holds Cyc, Cyc holds the register back.
codeunit 86399 "CKT Cyc"
{
    var
        Back: Codeunit "CKT Cyc Reg";
        I: Integer;

    procedure Link(var R: Codeunit "CKT Cyc Reg")
    begin
        Back := R;
        I := 5;
    end;

    procedure State(): Text
    begin
        exit(StrSubstNo('%1/%2', I, Back.Peek()));
    end;
}

codeunit 86400 "CKT Cyc Reg"
{
    TableNo = "CKK Row";

    var
        Cyc: Codeunit "CKT Cyc";
        Entry: Integer;
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := Rec."No.";
    end;

    procedure Prime(var Self: Codeunit "CKT Cyc Reg")
    begin
        Cyc.Link(Self);
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
        exit(StrSubstNo('cyc=%1 entry=%2 mov=%3', Cyc.State(), Entry, Format(OnMovement, 0, 9)));
    end;
}

// A store without TableNo that holds a Label with Comment.
codeunit 86401 "CKT Plain Reg"
{
    var
        Entry: Integer;
        Text000: Label 'is not sufficient %1 %2', Comment = '%1 = a, %2 = b';
        OnMovement: Boolean;

    trigger OnRun()
    begin
        Entry := 7;
    end;

    procedure Prime(var Self: Codeunit "CKT Plain Reg")
    begin
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
        exit(StrSubstNo('entry=%1 mov=%2 lbl=%3', Entry, Format(OnMovement, 0, 9), StrLen(StrSubstNo(Text000, 1, 2))));
    end;
}

// "Sales-Post" shape with a TableNo: the register is a global handed in by a setter, ClearAll, flag restored, then the register is used.
codeunit 86402 "CKT Owner"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKT Reg";
        Suppress: Boolean;
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        Process();
    end;

    procedure Process()
    var
        Saved: Boolean;
    begin
        Saved := Suppress;
        ClearAll();
        Suppress := Saved;
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKT Reg")
    begin
        Reg := R;
    end;

    procedure SetSuppress(V: Boolean)
    begin
        Suppress := V;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
        Observer.Observe('owner.suppress', Format(Suppress, 0, 9));
    end;
}


// "Sales-Post" shape without a TableNo: the register is a global handed in by a setter, ClearAll, flag restored, then the register is used.
codeunit 86403 "CKT Plain Owner"
{
    var
        Reg: Codeunit "CKT Reg";
        Suppress: Boolean;
        After: Text;
        EntryAfter: Integer;

    procedure Process()
    var
        Saved: Boolean;
    begin
        Saved := Suppress;
        ClearAll();
        Suppress := Saved;
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKT Reg")
    begin
        Reg := R;
    end;

    procedure SetSuppress(V: Boolean)
    begin
        Suppress := V;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
        Observer.Observe('owner.suppress', Format(Suppress, 0, 9));
    end;
}


// "Sales-Post" shape with a TableNo: the register is a global handed in by a setter, ClearAll, flag restored, then the register is used.
codeunit 86404 "CKT Deep Owner"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKT Deep Reg";
        Suppress: Boolean;
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        Process();
    end;

    procedure Process()
    var
        Saved: Boolean;
    begin
        Saved := Suppress;
        ClearAll();
        Suppress := Saved;
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKT Deep Reg")
    begin
        Reg := R;
    end;

    procedure SetSuppress(V: Boolean)
    begin
        Suppress := V;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
        Observer.Observe('owner.suppress', Format(Suppress, 0, 9));
    end;
}


// "Sales-Post" shape with a TableNo: the register is a global handed in by a setter, ClearAll, flag restored, then the register is used.
codeunit 86405 "CKT Cyc Owner"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKT Cyc Reg";
        Suppress: Boolean;
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        Process();
    end;

    procedure Process()
    var
        Saved: Boolean;
    begin
        Saved := Suppress;
        ClearAll();
        Suppress := Saved;
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKT Cyc Reg")
    begin
        Reg := R;
    end;

    procedure SetSuppress(V: Boolean)
    begin
        Suppress := V;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
        Observer.Observe('owner.suppress', Format(Suppress, 0, 9));
    end;
}


// "Sales-Post" shape with a TableNo: the register is a global handed in by a setter, ClearAll, flag restored, then the register is used.
codeunit 86406 "CKT PlainReg Owner"
{
    TableNo = "CKK Row";

    var
        Reg: Codeunit "CKT Plain Reg";
        Suppress: Boolean;
        After: Text;
        EntryAfter: Integer;

    trigger OnRun()
    begin
        Process();
    end;

    procedure Process()
    var
        Saved: Boolean;
    begin
        Saved := Suppress;
        ClearAll();
        Suppress := Saved;
        After := Reg.State();
        Reg.RegisterEntry();
        EntryAfter := Reg.Peek();
    end;

    procedure SetReg(var R: Codeunit "CKT Plain Reg")
    begin
        Reg := R;
    end;

    procedure SetSuppress(V: Boolean)
    begin
        Suppress := V;
    end;

    procedure ObservePost(var Observer: Codeunit "Probe Observer")
    begin
        Observer.Observe('owner.after', After);
        Observer.Observe('owner.entry', Format(EntryAfter));
        Observer.Observe('owner.suppress', Format(Suppress, 0, 9));
    end;
}


codeunit 86407 "CKT Register Test"
{
    Subtype = Test;

    local procedure ShareReg(Before: Integer; var Observer: Codeunit "Probe Observer")
    var
        Reg: Codeunit "CKT Reg";
        Owner: Codeunit "CKT Owner";
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Row."No." := 7;
        case Before of
            1:
                Reg.Prime(Reg);
            2:
                Observer.Observe('peek', Format(Reg.Peek()));
            4:
                Reg.Run(Row);
            5:
                begin
                    Reg.Run(Row);
                    Reg.Prime(Reg);
                end;
        end;
        Owner.SetReg(Reg);
        Owner.SetSuppress(true);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
    end;

    local procedure ShareRegPlain(Before: Integer; var Observer: Codeunit "Probe Observer")
    var
        Reg: Codeunit "CKT Reg";
        Owner: Codeunit "CKT Plain Owner";
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Row."No." := 7;
        case Before of
            1:
                Reg.Prime(Reg);
            2:
                Observer.Observe('peek', Format(Reg.Peek()));
            4:
                Reg.Run(Row);
            5:
                begin
                    Reg.Run(Row);
                    Reg.Prime(Reg);
                end;
        end;
        Owner.SetReg(Reg);
        Owner.SetSuppress(true);
        Owner.Process();
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
    end;

    local procedure ShareDeep(Before: Integer; var Observer: Codeunit "Probe Observer")
    var
        Reg: Codeunit "CKT Deep Reg";
        Owner: Codeunit "CKT Deep Owner";
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Row."No." := 7;
        case Before of
            1:
                Reg.Prime(Reg);
            2:
                Observer.Observe('peek', Format(Reg.Peek()));
            4:
                Reg.Run(Row);
            5:
                begin
                    Reg.Run(Row);
                    Reg.Prime(Reg);
                end;
        end;
        Owner.SetReg(Reg);
        Owner.SetSuppress(true);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
    end;

    local procedure ShareCyc(Before: Integer; var Observer: Codeunit "Probe Observer")
    var
        Reg: Codeunit "CKT Cyc Reg";
        Owner: Codeunit "CKT Cyc Owner";
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Row."No." := 7;
        case Before of
            1:
                Reg.Prime(Reg);
            2:
                Observer.Observe('peek', Format(Reg.Peek()));
            4:
                Reg.Run(Row);
            5:
                begin
                    Reg.Run(Row);
                    Reg.Prime(Reg);
                end;
        end;
        Owner.SetReg(Reg);
        Owner.SetSuppress(true);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
    end;

    local procedure SharePlainReg(Before: Integer; var Observer: Codeunit "Probe Observer")
    var
        Reg: Codeunit "CKT Plain Reg";
        Owner: Codeunit "CKT PlainReg Owner";
        Row: Record "CKK Row";
    begin
        Row.DeleteAll();
        Row."No." := 7;
        case Before of
            1:
                Reg.Prime(Reg);
            2:
                Observer.Observe('peek', Format(Reg.Peek()));
            4:
                Reg.Run();
            5:
                begin
                    Reg.Run();
                    Reg.Prime(Reg);
                end;
        end;
        Owner.SetReg(Reg);
        Owner.SetSuppress(true);
        Owner.Run(Row);
        Owner.ObservePost(Observer);
        Observer.Observe('caller.after', Reg.State());
        Reg.RegisterEntry();
        Observer.Observe('caller.next', Reg.State());
    end;

    [Test]
    procedure TableNoReg_Primed_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareReg(1, Observer);
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoReg_Peeked_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareReg(2, Observer);
        Observer.Expect('peek', '0');
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoReg_NeverCalled_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareReg(3, Observer);
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoReg_RunBeforeShare_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareReg(4, Observer);
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoReg_RunThenPrimed_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareReg(5, Observer);
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoReg_Primed_PlainOwner()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareRegPlain(1, Observer);
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoReg_NeverCalled_PlainOwner()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareRegPlain(3, Observer);
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure TableNoReg_RunBeforeShare_PlainOwner()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareRegPlain(4, Observer);
        Observer.Expect('owner.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=0 mov=false lbl=44');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=1 mov=false lbl=44');
        Observer.Expect('caller.next', 'loc=|0 jnl= bin=0 reg=0 trk=0/0 wms=0/0/0 entry=2 mov=false lbl=44');
        Observer.Verify();
    end;

    [Test]
    procedure DeepReg_Primed_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareDeep(1, Observer);
        Observer.Expect('owner.after', 'chain=0/0/0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'chain=0/0/0/0 entry=1 mov=false');
        Observer.Expect('caller.next', 'chain=0/0/0/0 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure DeepReg_Peeked_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareDeep(2, Observer);
        Observer.Expect('peek', '0');
        Observer.Expect('owner.after', 'chain=0/0/0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'chain=0/0/0/0 entry=1 mov=false');
        Observer.Expect('caller.next', 'chain=0/0/0/0 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure DeepReg_NeverCalled_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareDeep(3, Observer);
        Observer.Expect('owner.after', 'chain=0/0/0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'chain=0/0/0/0 entry=1 mov=false');
        Observer.Expect('caller.next', 'chain=0/0/0/0 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure DeepReg_RunBeforeShare_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareDeep(4, Observer);
        Observer.Expect('owner.after', 'chain=0/0/0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'chain=0/0/0/0 entry=1 mov=false');
        Observer.Expect('caller.next', 'chain=0/0/0/0 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure CycReg_Primed_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareCyc(1, Observer);
        Observer.Expect('owner.after', 'cyc=0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'cyc=0/1 entry=1 mov=false');
        Observer.Expect('caller.next', 'cyc=0/2 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure CycReg_Peeked_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareCyc(2, Observer);
        Observer.Expect('peek', '0');
        Observer.Expect('owner.after', 'cyc=0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'cyc=0/0 entry=1 mov=false');
        Observer.Expect('caller.next', 'cyc=0/0 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure CycReg_NeverCalled_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareCyc(3, Observer);
        Observer.Expect('owner.after', 'cyc=0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'cyc=0/0 entry=1 mov=false');
        Observer.Expect('caller.next', 'cyc=0/0 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure CycReg_RunBeforeShare_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        ShareCyc(4, Observer);
        Observer.Expect('owner.after', 'cyc=0/0 entry=0 mov=false');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'cyc=0/0 entry=1 mov=false');
        Observer.Expect('caller.next', 'cyc=0/0 entry=2 mov=false');
        Observer.Verify();
    end;

    [Test]
    procedure PlainRegLabel_Primed_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        SharePlainReg(1, Observer);
        Observer.Expect('owner.after', 'entry=0 mov=false lbl=21');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'entry=1 mov=false lbl=21');
        Observer.Expect('caller.next', 'entry=2 mov=false lbl=21');
        Observer.Verify();
    end;

    [Test]
    procedure PlainRegLabel_Peeked_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        SharePlainReg(2, Observer);
        Observer.Expect('peek', '0');
        Observer.Expect('owner.after', 'entry=0 mov=false lbl=21');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'entry=1 mov=false lbl=21');
        Observer.Expect('caller.next', 'entry=2 mov=false lbl=21');
        Observer.Verify();
    end;

    [Test]
    procedure PlainRegLabel_NeverCalled_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        SharePlainReg(3, Observer);
        Observer.Expect('owner.after', 'entry=0 mov=false lbl=21');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'entry=1 mov=false lbl=21');
        Observer.Expect('caller.next', 'entry=2 mov=false lbl=21');
        Observer.Verify();
    end;

    [Test]
    procedure PlainRegLabel_RunBeforeShare_OwnerRun()
    var
        Observer: Codeunit "Probe Observer";
    begin
        SharePlainReg(4, Observer);
        Observer.Expect('owner.after', 'entry=0 mov=false lbl=21');
        Observer.Expect('owner.entry', '1');
        Observer.Expect('owner.suppress', 'true');
        Observer.Expect('caller.after', 'entry=1 mov=false lbl=21');
        Observer.Expect('caller.next', 'entry=2 mov=false lbl=21');
        Observer.Verify();
    end;
}
