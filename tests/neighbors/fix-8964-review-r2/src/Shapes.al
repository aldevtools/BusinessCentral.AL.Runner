table 50300 "R Row" { fields { field(1; "No."; Integer) { } field(2; Txt; Text[30]) { } } keys { key(PK; "No.") { } } }

// A: the back link holds another register instance the test shares.
codeunit 50301 "R Cyc" { var Back: Codeunit "R Cyc Reg"; I: Integer;
    procedure Link(var R: Codeunit "R Cyc Reg") begin Back := R; I := 5; end;
    procedure State(): Text begin exit(StrSubstNo('%1/%2', I, Back.Peek())); end; }
codeunit 50302 "R Cyc Reg" { TableNo = "R Row"; var Cyc: Codeunit "R Cyc"; Entry: Integer;
    trigger OnRun() begin Entry := Rec."No."; end;
    procedure Prime(var Other: Codeunit "R Cyc Reg") begin Cyc.Link(Other); Entry := 5; end;
    procedure SetEntry(V: Integer) begin Entry := V; end;
    procedure Peek(): Integer begin exit(Entry); end;
    procedure RegisterEntry() begin Entry += 1; end;
    procedure State(): Text begin exit(StrSubstNo('cyc=%1 entry=%2', Cyc.State(), Entry)); end; }
codeunit 50303 "R Owner A" { TableNo = "R Row"; var Reg: Codeunit "R Cyc Reg";
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "R Cyc Reg") begin Reg := R; end; }

// B: a nested instance two levels below the register is shared with the test.
codeunit 50304 "R Leaf" { var N: Integer;
    procedure SetN(V: Integer) begin N := V; end;
    procedure GetN(): Integer begin exit(N); end; }
codeunit 50305 "R Mid" { var Leaf: Codeunit "R Leaf"; I: Integer;
    procedure SetN(V: Integer) begin I := V; Leaf.SetN(V + 1); end;
    procedure Share(var L: Codeunit "R Leaf") begin L := Leaf; end; }
codeunit 50306 "R Reg B" { TableNo = "R Row"; var Mid: Codeunit "R Mid"; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin Mid.SetN(10); Entry := 5; end;
    procedure ShareLeaf(var L: Codeunit "R Leaf") begin Mid.Share(L); end; }
codeunit 50307 "R Owner B" { TableNo = "R Row"; var Reg: Codeunit "R Reg B";
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "R Reg B") begin Reg := R; end; }

// C: the owner makes the cycle register itself; only the back link shares it.
codeunit 50308 "R Owner C" { var Reg: Codeunit "R Cyc Reg";
    procedure Go(): Text begin Reg.Prime(Reg); ClearAll(); Reg.RegisterEntry(); exit(Reg.State()); end; }

// D: the chain of four nested levels (M1, M2, M3, Leaf) under the plain owner (1596 measures it under the TableNo owner only).
codeunit 50325 "R M3" { var Leaf: Codeunit "R Leaf"; I: Integer; procedure SetN(V: Integer) begin I := V; Leaf.SetN(V + 1); end;
    procedure State(): Text begin exit(StrSubstNo('%1/%2', I, Leaf.GetN())); end; }
codeunit 50309 "R M2" { var Next: Codeunit "R M3"; I: Integer; procedure SetN(V: Integer) begin I := V; Next.SetN(V + 1); end;
    procedure State(): Text begin exit(StrSubstNo('%1/%2', I, Next.State())); end; }
codeunit 50310 "R M1" { var Next: Codeunit "R M2"; I: Integer; procedure SetN(V: Integer) begin I := V; Next.SetN(V + 1); end;
    procedure State(): Text begin exit(StrSubstNo('%1/%2', I, Next.State())); end; }
codeunit 50311 "R Reg D" { TableNo = "R Row"; var Chain: Codeunit "R M1"; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin Chain.SetN(10); Entry := 5; end;
    procedure State(): Text begin exit(StrSubstNo('chain=%1 entry=%2', Chain.State(), Entry)); end; }
codeunit 50312 "R Owner D" { var Reg: Codeunit "R Reg D";
    procedure SetReg(var R: Codeunit "R Reg D") begin Reg := R; end;
    procedure Go() begin ClearAll(); end; }

// E: the back link holds a register instance that only the nested codeunit holds, made and written there.
codeunit 50313 "R Cyc E" { var Back: Codeunit "R Cyc Reg E"; I: Integer;
    procedure Fill() begin Back.SetEntry(9); I := 5; end;
    procedure State(): Text begin exit(StrSubstNo('%1/%2', I, Back.Peek())); end; }
codeunit 50314 "R Cyc Reg E" { TableNo = "R Row"; var Cyc: Codeunit "R Cyc E"; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin Cyc.Fill(); Entry := 5; end;
    procedure SetEntry(V: Integer) begin Entry := V; end;
    procedure Peek(): Integer begin exit(Entry); end;
    procedure State(): Text begin exit(StrSubstNo('cyc=%1 entry=%2', Cyc.State(), Entry)); end; }
codeunit 50315 "R Owner E" { TableNo = "R Row"; var Reg: Codeunit "R Cyc Reg E";
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "R Cyc Reg E") begin Reg := R; end; }

// F: a SingleInstance codeunit nested in the register.
codeunit 50316 "R SI" { SingleInstance = true; var N: Integer; procedure SetN(V: Integer) begin N := V; end;
    procedure GetN(): Integer begin exit(N); end; }
codeunit 50317 "R Reg F" { TableNo = "R Row"; var SI: Codeunit "R SI"; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() begin SI.SetN(10); Entry := 5; end;
    procedure State(): Text begin exit(StrSubstNo('si=%1 entry=%2', SI.GetN(), Entry)); end; }
codeunit 50318 "R Owner F" { TableNo = "R Row"; var Reg: Codeunit "R Reg F";
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "R Reg F") begin Reg := R; end; }

// G: an Interface global in the register.
interface "R Face" { procedure GetN(): Integer; }
codeunit 50319 "R Impl" implements "R Face" { var N: Integer; procedure SetN(V: Integer) begin N := V; end;
    procedure GetN(): Integer begin exit(N); end; }
codeunit 50320 "R Reg G" { TableNo = "R Row"; var Face: Interface "R Face"; Entry: Integer;
    trigger OnRun() begin end;
    procedure Prime() var Impl: Codeunit "R Impl"; begin Impl.SetN(10); Face := Impl; Entry := 5; end;
    procedure State(): Text begin exit(StrSubstNo('face=%1 entry=%2', Face.GetN(), Entry)); end; }
codeunit 50321 "R Owner G" { TableNo = "R Row"; var Reg: Codeunit "R Reg G";
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "R Reg G") begin Reg := R; end; }

// H: a Label with Locked = true in the register.
codeunit 50322 "R Reg H" { TableNo = "R Row"; var Entry: Integer; Txt: Label 'Locked text', Locked = true;
    trigger OnRun() begin end;
    procedure Prime() begin Entry := 5; end;
    procedure State(): Text begin exit(StrSubstNo('%1 entry=%2', Txt, Entry)); end; }
codeunit 50323 "R Owner H" { TableNo = "R Row"; var Reg: Codeunit "R Reg H";
    trigger OnRun() begin ClearAll(); end;
    procedure SetReg(var R: Codeunit "R Reg H") begin Reg := R; end; }

// K: control that runs on both sides, so the app is compared (no codeunit global, nothing shared).
codeunit 50324 "R Owner K" { var X: Integer; procedure Go(): Integer begin X := 3; ClearAll(); exit(X); end; }

codeunit 50391 "R Test 2" { Subtype = Test;
    [Test] procedure TableNoReg_BackLinkOtherRegister() var Reg: Codeunit "R Cyc Reg"; Other: Codeunit "R Cyc Reg"; O: Codeunit "R Owner A"; Row: Record "R Row";
    begin Other.SetEntry(7); Reg.Prime(Other); O.SetReg(Reg); O.Run(Row); Error('A other=%1 reg=%2', Other.Peek(), Reg.State()); end;
}
codeunit 50392 "R Test 3" { Subtype = Test;
    [Test] procedure NestedSharedByCaller_DepthTwo() var Reg: Codeunit "R Reg B"; L: Codeunit "R Leaf"; O: Codeunit "R Owner B"; Row: Record "R Row";
    begin Reg.Prime(); Reg.ShareLeaf(L); O.SetReg(Reg); O.Run(Row); Error('B leaf=%1', L.GetN()); end;
}
codeunit 50393 "R Test 4" { Subtype = Test;
    [Test] procedure CycReg_OwnerMadeOnly() var O: Codeunit "R Owner C";
    begin Error('C %1', O.Go()); end;
}
codeunit 50394 "R Test 5" { Subtype = Test;
    [Test] procedure DeepReg_Primed_PlainOwner() var Reg: Codeunit "R Reg D"; O: Codeunit "R Owner D";
    begin Reg.Prime(); O.SetReg(Reg); O.Go(); Error('D %1', Reg.State()); end;
}
codeunit 50395 "R Test 6" { Subtype = Test;
    [Test] procedure TableNoReg_BackLinkOwnInstance() var Reg: Codeunit "R Cyc Reg E"; O: Codeunit "R Owner E"; Row: Record "R Row";
    begin Reg.Prime(); O.SetReg(Reg); O.Run(Row); Error('E %1', Reg.State()); end;
}
codeunit 50396 "R Test 7" { Subtype = Test;
    [Test] procedure SingleInstanceNested_Primed() var Reg: Codeunit "R Reg F"; O: Codeunit "R Owner F"; Row: Record "R Row";
    begin Reg.Prime(); O.SetReg(Reg); O.Run(Row); Error('F %1', Reg.State()); end;
}
codeunit 50397 "R Test 8" { Subtype = Test;
    [Test] procedure InterfaceGlobal_Primed() var Reg: Codeunit "R Reg G"; O: Codeunit "R Owner G"; Row: Record "R Row";
    begin Reg.Prime(); O.SetReg(Reg); O.Run(Row); Error('G %1', Reg.State()); end;
}
codeunit 50398 "R Test 9" { Subtype = Test;
    [Test] procedure LockedLabel_Primed() var Reg: Codeunit "R Reg H"; O: Codeunit "R Owner H"; Row: Record "R Row";
    begin Reg.Prime(); O.SetReg(Reg); O.Run(Row); Error('H %1', Reg.State()); end;
}
codeunit 50399 "R Test 10" { Subtype = Test;
    [Test] procedure Control_ClearAll_NoCodeunitGlobal() var O: Codeunit "R Owner K";
    begin if O.Go() <> 0 then Error('K %1', O.Go()); end;
}
