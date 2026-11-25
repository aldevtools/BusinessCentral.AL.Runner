// A copy of the corpus probe record/filters/AtSignAloneTypes.al of corpus pull request 1495 at 78c92723 (its lines are the probe's
// lines plus two).
// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/properties/devenv-calcformula-property
// Scope: in-scope
// Fixtures used: Assert (60021); own enum 97470, tables 97471 and 97472 and test codeunit 97470
// Probe (al-runner issue 8928, the rest of al-runner issue 8876): a CalcFormula's quoted filter('@') and filter('<>@') over a
//   source field of every type that is not Code or Text, with every method; the same filters through field(filter()) of a
//   normal field that holds them; and the error texts in other languages. Every value asserted below is what a local
//   Business Central 29.1 OnPrem W1 container produced, 3 runs.
// Facts: the rows of a new group are 1 (K: every field holds a value, Amount 10) and 2 (every field empty, Amount 5).
//   CalcFields reads each FlowField in a TryFunction; an error reads as its code and text. Value None of enum 97470 has no
//   Caption; Alpha and Beta have one.
// BC versions: 27, 28, 29

enum 97470 "AT Kind"
{
    Extensible = false;

    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}

table 97471 "AT Type Row"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "Group"; Code[20]) { }
        field(2; Id; Integer) { }
        field(3; Amount; Decimal) { }
        field(4; "Code Kind"; Code[20]) { }
        field(5; "Text Kind"; Text[50]) { }
        field(10; Day; Date) { }
        field(11; Num; Integer) { }
        field(12; Dec; Decimal) { }
        field(13; Flag; Boolean) { }
        field(14; Opt; Option) { OptionMembers = A,B; }
        field(15; "Opt Cap"; Option) { OptionMembers = A,B; OptionCaption = 'Alpha,Beta'; }
        field(16; Tm; Time) { }
        field(17; Moment; DateTime) { }
        field(18; Dur; Duration) { }
        field(19; Big; BigInteger) { }
        field(20; Uid; Guid) { }
        field(21; Kind; Enum "AT Kind") { }
    }

    keys
    {
        key(PK; "Group", Id) { Clustered = true; }
    }
}

table 97472 "AT Type Host"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Pattern; Text[30]) { }
        field(100; "Day Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Day = filter('@'))); }
        field(101; "Day Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Day = filter('<>@'))); }
        field(102; "Day Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Day = filter('@'))); }
        field(103; "Day Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Day = filter('<>@'))); }
        field(104; "Day Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Day = filter('@'))); }
        field(105; "Day Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Day = filter('<>@'))); }
        field(106; "Day Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Day = filter('@'))); }
        field(107; "Day Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Day = filter('<>@'))); }
        field(108; "Day Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Day = filter('@'))); }
        field(109; "Day Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Day = filter('<>@'))); }
        field(110; "Day Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Day = filter('@'))); }
        field(111; "Day Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Day = filter('<>@'))); }
        field(120; "Num Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Num = filter('@'))); }
        field(121; "Num Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Num = filter('<>@'))); }
        field(122; "Num Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Num = filter('@'))); }
        field(123; "Num Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Num = filter('<>@'))); }
        field(124; "Num Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Num = filter('@'))); }
        field(125; "Num Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Num = filter('<>@'))); }
        field(126; "Num Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Num = filter('@'))); }
        field(127; "Num Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Num = filter('<>@'))); }
        field(128; "Num Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Num = filter('@'))); }
        field(129; "Num Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Num = filter('<>@'))); }
        field(130; "Num Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Num = filter('@'))); }
        field(131; "Num Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Num = filter('<>@'))); }
        field(140; "Dec Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Dec = filter('@'))); }
        field(141; "Dec Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Dec = filter('<>@'))); }
        field(142; "Dec Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Dec = filter('@'))); }
        field(143; "Dec Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Dec = filter('<>@'))); }
        field(144; "Dec Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Dec = filter('@'))); }
        field(145; "Dec Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Dec = filter('<>@'))); }
        field(146; "Dec Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Dec = filter('@'))); }
        field(147; "Dec Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Dec = filter('<>@'))); }
        field(148; "Dec Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Dec = filter('@'))); }
        field(149; "Dec Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Dec = filter('<>@'))); }
        field(150; "Dec Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Dec = filter('@'))); }
        field(151; "Dec Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Dec = filter('<>@'))); }
        field(160; "Flag Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Flag = filter('@'))); }
        field(161; "Flag Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Flag = filter('<>@'))); }
        field(162; "Flag Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Flag = filter('@'))); }
        field(163; "Flag Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Flag = filter('<>@'))); }
        field(164; "Flag Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Flag = filter('@'))); }
        field(165; "Flag Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Flag = filter('<>@'))); }
        field(166; "Flag Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Flag = filter('@'))); }
        field(167; "Flag Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Flag = filter('<>@'))); }
        field(168; "Flag Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Flag = filter('@'))); }
        field(169; "Flag Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Flag = filter('<>@'))); }
        field(170; "Flag Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Flag = filter('@'))); }
        field(171; "Flag Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Flag = filter('<>@'))); }
        field(180; "Opt Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Opt = filter('@'))); }
        field(181; "Opt Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Opt = filter('<>@'))); }
        field(182; "Opt Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Opt = filter('@'))); }
        field(183; "Opt Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Opt = filter('<>@'))); }
        field(184; "Opt Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Opt = filter('@'))); }
        field(185; "Opt Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Opt = filter('<>@'))); }
        field(186; "Opt Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Opt = filter('@'))); }
        field(187; "Opt Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Opt = filter('<>@'))); }
        field(188; "Opt Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Opt = filter('@'))); }
        field(189; "Opt Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Opt = filter('<>@'))); }
        field(190; "Opt Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Opt = filter('@'))); }
        field(191; "Opt Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Opt = filter('<>@'))); }
        field(200; "OptCap Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), "Opt Cap" = filter('@'))); }
        field(201; "OptCap Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), "Opt Cap" = filter('<>@'))); }
        field(202; "OptCap Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), "Opt Cap" = filter('@'))); }
        field(203; "OptCap Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), "Opt Cap" = filter('<>@'))); }
        field(204; "OptCap Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('@'))); }
        field(205; "OptCap Sum Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('<>@'))); }
        field(206; "OptCap Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('@'))); }
        field(207; "OptCap Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('<>@'))); }
        field(208; "OptCap Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('@'))); }
        field(209; "OptCap Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('<>@'))); }
        field(210; "OptCap Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('@'))); }
        field(211; "OptCap Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('<>@'))); }
        field(212; "OptCap Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('@'))); }
        field(213; "OptCap Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), "Opt Cap" = filter('<>@'))); }
        field(220; "Tm Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Tm = filter('@'))); }
        field(221; "Tm Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Tm = filter('<>@'))); }
        field(222; "Tm Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Tm = filter('@'))); }
        field(223; "Tm Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Tm = filter('<>@'))); }
        field(224; "Tm Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Tm = filter('@'))); }
        field(225; "Tm Sum Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Tm = filter('<>@'))); }
        field(226; "Tm Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Tm = filter('@'))); }
        field(227; "Tm Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Tm = filter('<>@'))); }
        field(228; "Tm Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Tm = filter('@'))); }
        field(229; "Tm Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Tm = filter('<>@'))); }
        field(230; "Tm Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Tm = filter('@'))); }
        field(231; "Tm Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Tm = filter('<>@'))); }
        field(232; "Tm Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Tm = filter('@'))); }
        field(233; "Tm Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Tm = filter('<>@'))); }
        field(240; "Moment Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Moment = filter('@'))); }
        field(241; "Moment Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Moment = filter('<>@'))); }
        field(242; "Moment Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Moment = filter('@'))); }
        field(243; "Moment Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Moment = filter('<>@'))); }
        field(244; "Moment Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Moment = filter('@'))); }
        field(245; "Moment Sum Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Moment = filter('<>@'))); }
        field(246; "Moment Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Moment = filter('@'))); }
        field(247; "Moment Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Moment = filter('<>@'))); }
        field(248; "Moment Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Moment = filter('@'))); }
        field(249; "Moment Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Moment = filter('<>@'))); }
        field(250; "Moment Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Moment = filter('@'))); }
        field(251; "Moment Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Moment = filter('<>@'))); }
        field(252; "Moment Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Moment = filter('@'))); }
        field(253; "Moment Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Moment = filter('<>@'))); }
        field(260; "Dur Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Dur = filter('@'))); }
        field(261; "Dur Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Dur = filter('<>@'))); }
        field(262; "Dur Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Dur = filter('@'))); }
        field(263; "Dur Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Dur = filter('<>@'))); }
        field(264; "Dur Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Dur = filter('@'))); }
        field(265; "Dur Sum Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Dur = filter('<>@'))); }
        field(266; "Dur Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Dur = filter('@'))); }
        field(267; "Dur Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Dur = filter('<>@'))); }
        field(268; "Dur Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Dur = filter('@'))); }
        field(269; "Dur Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Dur = filter('<>@'))); }
        field(270; "Dur Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Dur = filter('@'))); }
        field(271; "Dur Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Dur = filter('<>@'))); }
        field(272; "Dur Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Dur = filter('@'))); }
        field(273; "Dur Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Dur = filter('<>@'))); }
        field(280; "Big Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Big = filter('@'))); }
        field(281; "Big Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Big = filter('<>@'))); }
        field(282; "Big Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Big = filter('@'))); }
        field(283; "Big Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Big = filter('<>@'))); }
        field(284; "Big Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Big = filter('@'))); }
        field(285; "Big Sum Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Big = filter('<>@'))); }
        field(286; "Big Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Big = filter('@'))); }
        field(287; "Big Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Big = filter('<>@'))); }
        field(288; "Big Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Big = filter('@'))); }
        field(289; "Big Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Big = filter('<>@'))); }
        field(290; "Big Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Big = filter('@'))); }
        field(291; "Big Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Big = filter('<>@'))); }
        field(292; "Big Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Big = filter('@'))); }
        field(293; "Big Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Big = filter('<>@'))); }
        field(300; "Uid Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Uid = filter('@'))); }
        field(301; "Uid Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Uid = filter('<>@'))); }
        field(302; "Uid Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Uid = filter('@'))); }
        field(303; "Uid Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Uid = filter('<>@'))); }
        field(304; "Uid Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Uid = filter('@'))); }
        field(305; "Uid Sum Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Uid = filter('<>@'))); }
        field(306; "Uid Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Uid = filter('@'))); }
        field(307; "Uid Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Uid = filter('<>@'))); }
        field(308; "Uid Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Uid = filter('@'))); }
        field(309; "Uid Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Uid = filter('<>@'))); }
        field(310; "Uid Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Uid = filter('@'))); }
        field(311; "Uid Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Uid = filter('<>@'))); }
        field(312; "Uid Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Uid = filter('@'))); }
        field(313; "Uid Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Uid = filter('<>@'))); }
        field(320; "Kind Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Kind = filter('@'))); }
        field(321; "Kind Count Not At"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), Kind = filter('<>@'))); }
        field(322; "Kind Exist At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Kind = filter('@'))); }
        field(323; "Kind Exist Not At"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), Kind = filter('<>@'))); }
        field(324; "Kind Sum At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Kind = filter('@'))); }
        field(325; "Kind Sum Not At"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), Kind = filter('<>@'))); }
        field(326; "Kind Average At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Kind = filter('@'))); }
        field(327; "Kind Average Not At"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), Kind = filter('<>@'))); }
        field(328; "Kind Min At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Kind = filter('@'))); }
        field(329; "Kind Min Not At"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), Kind = filter('<>@'))); }
        field(330; "Kind Max At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Kind = filter('@'))); }
        field(331; "Kind Max Not At"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), Kind = filter('<>@'))); }
        field(332; "Kind Lookup At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Kind = filter('@'))); }
        field(333; "Kind Lookup Not At"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), Kind = filter('<>@'))); }
        field(400; "Code Count By Pattern"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), "Code Kind" = field(filter(Pattern)))); }
        field(401; "Code Exist By Pattern"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), "Code Kind" = field(filter(Pattern)))); }
        field(402; "Code Sum By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), "Code Kind" = field(filter(Pattern)))); }
        field(403; "Code Average By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), "Code Kind" = field(filter(Pattern)))); }
        field(404; "Code Min By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), "Code Kind" = field(filter(Pattern)))); }
        field(405; "Code Max By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), "Code Kind" = field(filter(Pattern)))); }
        field(406; "Code Lookup By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), "Code Kind" = field(filter(Pattern)))); }
        field(407; "Text Count By Pattern"; Integer) { FieldClass = FlowField; CalcFormula = count("AT Type Row" where("Group" = field("No."), "Text Kind" = field(filter(Pattern)))); }
        field(408; "Text Exist By Pattern"; Boolean) { FieldClass = FlowField; CalcFormula = exist("AT Type Row" where("Group" = field("No."), "Text Kind" = field(filter(Pattern)))); }
        field(409; "Text Sum By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = sum("AT Type Row".Amount where("Group" = field("No."), "Text Kind" = field(filter(Pattern)))); }
        field(410; "Text Average By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = average("AT Type Row".Amount where("Group" = field("No."), "Text Kind" = field(filter(Pattern)))); }
        field(411; "Text Min By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = min("AT Type Row".Amount where("Group" = field("No."), "Text Kind" = field(filter(Pattern)))); }
        field(412; "Text Max By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = max("AT Type Row".Amount where("Group" = field("No."), "Text Kind" = field(filter(Pattern)))); }
        field(413; "Text Lookup By Pattern"; Decimal) { FieldClass = FlowField; CalcFormula = lookup("AT Type Row".Amount where("Group" = field("No."), "Text Kind" = field(filter(Pattern)))); }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

codeunit 97470 "AT Type Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Assert: Codeunit Assert;

    // A new group: row 1 (K) with a value in every field and Amount 10, row 2 with every field empty and Amount 5.
    local procedure NewGroup(): Code[20]
    var
        Row: Record "AT Type Row";
    begin
        Row."Group" := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18);
        Row.Id := 1;
        Row.Amount := 10;
        Row."Code Kind" := 'K';
        Row."Text Kind" := 'K';
        Row.Day := 20260101D;
        Row.Num := 1;
        Row.Dec := 1.5;
        Row.Flag := true;
        Row.Opt := Row.Opt::B;
        Row."Opt Cap" := Row."Opt Cap"::B;
        Row.Tm := 120000T;
        Row.Moment := CreateDateTime(20260101D, 120000T);
        Row.Dur := 1000;
        Row.Big := 7;
        Row.Uid := CreateGuid();
        Row.Kind := "AT Kind"::Beta;
        Row.Insert();
        Row.Init();
        Row.Id := 2;
        Row.Amount := 5;
        Row.Insert();
        exit(Row."Group");
    end;

    [TryFunction]
    local procedure TryCalc(var Host: Record "AT Type Host"; FieldNo: Integer)
    begin
        case FieldNo of
            Host.FieldNo("Day Count At"):
                Host.CalcFields("Day Count At");
            Host.FieldNo("Day Count Not At"):
                Host.CalcFields("Day Count Not At");
            Host.FieldNo("Day Exist At"):
                Host.CalcFields("Day Exist At");
            Host.FieldNo("Day Exist Not At"):
                Host.CalcFields("Day Exist Not At");
            Host.FieldNo("Day Average At"):
                Host.CalcFields("Day Average At");
            Host.FieldNo("Day Average Not At"):
                Host.CalcFields("Day Average Not At");
            Host.FieldNo("Day Min At"):
                Host.CalcFields("Day Min At");
            Host.FieldNo("Day Min Not At"):
                Host.CalcFields("Day Min Not At");
            Host.FieldNo("Day Max At"):
                Host.CalcFields("Day Max At");
            Host.FieldNo("Day Max Not At"):
                Host.CalcFields("Day Max Not At");
            Host.FieldNo("Day Lookup At"):
                Host.CalcFields("Day Lookup At");
            Host.FieldNo("Day Lookup Not At"):
                Host.CalcFields("Day Lookup Not At");
            Host.FieldNo("Num Count At"):
                Host.CalcFields("Num Count At");
            Host.FieldNo("Num Count Not At"):
                Host.CalcFields("Num Count Not At");
            Host.FieldNo("Num Exist At"):
                Host.CalcFields("Num Exist At");
            Host.FieldNo("Num Exist Not At"):
                Host.CalcFields("Num Exist Not At");
            Host.FieldNo("Num Average At"):
                Host.CalcFields("Num Average At");
            Host.FieldNo("Num Average Not At"):
                Host.CalcFields("Num Average Not At");
            Host.FieldNo("Num Min At"):
                Host.CalcFields("Num Min At");
            Host.FieldNo("Num Min Not At"):
                Host.CalcFields("Num Min Not At");
            Host.FieldNo("Num Max At"):
                Host.CalcFields("Num Max At");
            Host.FieldNo("Num Max Not At"):
                Host.CalcFields("Num Max Not At");
            Host.FieldNo("Num Lookup At"):
                Host.CalcFields("Num Lookup At");
            Host.FieldNo("Num Lookup Not At"):
                Host.CalcFields("Num Lookup Not At");
            Host.FieldNo("Dec Count At"):
                Host.CalcFields("Dec Count At");
            Host.FieldNo("Dec Count Not At"):
                Host.CalcFields("Dec Count Not At");
            Host.FieldNo("Dec Exist At"):
                Host.CalcFields("Dec Exist At");
            Host.FieldNo("Dec Exist Not At"):
                Host.CalcFields("Dec Exist Not At");
            Host.FieldNo("Dec Average At"):
                Host.CalcFields("Dec Average At");
            Host.FieldNo("Dec Average Not At"):
                Host.CalcFields("Dec Average Not At");
            Host.FieldNo("Dec Min At"):
                Host.CalcFields("Dec Min At");
            Host.FieldNo("Dec Min Not At"):
                Host.CalcFields("Dec Min Not At");
            Host.FieldNo("Dec Max At"):
                Host.CalcFields("Dec Max At");
            Host.FieldNo("Dec Max Not At"):
                Host.CalcFields("Dec Max Not At");
            Host.FieldNo("Dec Lookup At"):
                Host.CalcFields("Dec Lookup At");
            Host.FieldNo("Dec Lookup Not At"):
                Host.CalcFields("Dec Lookup Not At");
            Host.FieldNo("Flag Count At"):
                Host.CalcFields("Flag Count At");
            Host.FieldNo("Flag Count Not At"):
                Host.CalcFields("Flag Count Not At");
            Host.FieldNo("Flag Exist At"):
                Host.CalcFields("Flag Exist At");
            Host.FieldNo("Flag Exist Not At"):
                Host.CalcFields("Flag Exist Not At");
            Host.FieldNo("Flag Average At"):
                Host.CalcFields("Flag Average At");
            Host.FieldNo("Flag Average Not At"):
                Host.CalcFields("Flag Average Not At");
            Host.FieldNo("Flag Min At"):
                Host.CalcFields("Flag Min At");
            Host.FieldNo("Flag Min Not At"):
                Host.CalcFields("Flag Min Not At");
            Host.FieldNo("Flag Max At"):
                Host.CalcFields("Flag Max At");
            Host.FieldNo("Flag Max Not At"):
                Host.CalcFields("Flag Max Not At");
            Host.FieldNo("Flag Lookup At"):
                Host.CalcFields("Flag Lookup At");
            Host.FieldNo("Flag Lookup Not At"):
                Host.CalcFields("Flag Lookup Not At");
            Host.FieldNo("Opt Count At"):
                Host.CalcFields("Opt Count At");
            Host.FieldNo("Opt Count Not At"):
                Host.CalcFields("Opt Count Not At");
            Host.FieldNo("Opt Exist At"):
                Host.CalcFields("Opt Exist At");
            Host.FieldNo("Opt Exist Not At"):
                Host.CalcFields("Opt Exist Not At");
            Host.FieldNo("Opt Average At"):
                Host.CalcFields("Opt Average At");
            Host.FieldNo("Opt Average Not At"):
                Host.CalcFields("Opt Average Not At");
            Host.FieldNo("Opt Min At"):
                Host.CalcFields("Opt Min At");
            Host.FieldNo("Opt Min Not At"):
                Host.CalcFields("Opt Min Not At");
            Host.FieldNo("Opt Max At"):
                Host.CalcFields("Opt Max At");
            Host.FieldNo("Opt Max Not At"):
                Host.CalcFields("Opt Max Not At");
            Host.FieldNo("Opt Lookup At"):
                Host.CalcFields("Opt Lookup At");
            Host.FieldNo("Opt Lookup Not At"):
                Host.CalcFields("Opt Lookup Not At");
            Host.FieldNo("OptCap Count At"):
                Host.CalcFields("OptCap Count At");
            Host.FieldNo("OptCap Count Not At"):
                Host.CalcFields("OptCap Count Not At");
            Host.FieldNo("OptCap Exist At"):
                Host.CalcFields("OptCap Exist At");
            Host.FieldNo("OptCap Exist Not At"):
                Host.CalcFields("OptCap Exist Not At");
            Host.FieldNo("OptCap Sum At"):
                Host.CalcFields("OptCap Sum At");
            Host.FieldNo("OptCap Sum Not At"):
                Host.CalcFields("OptCap Sum Not At");
            Host.FieldNo("OptCap Average At"):
                Host.CalcFields("OptCap Average At");
            Host.FieldNo("OptCap Average Not At"):
                Host.CalcFields("OptCap Average Not At");
            Host.FieldNo("OptCap Min At"):
                Host.CalcFields("OptCap Min At");
            Host.FieldNo("OptCap Min Not At"):
                Host.CalcFields("OptCap Min Not At");
            Host.FieldNo("OptCap Max At"):
                Host.CalcFields("OptCap Max At");
            Host.FieldNo("OptCap Max Not At"):
                Host.CalcFields("OptCap Max Not At");
            Host.FieldNo("OptCap Lookup At"):
                Host.CalcFields("OptCap Lookup At");
            Host.FieldNo("OptCap Lookup Not At"):
                Host.CalcFields("OptCap Lookup Not At");
            Host.FieldNo("Tm Count At"):
                Host.CalcFields("Tm Count At");
            Host.FieldNo("Tm Count Not At"):
                Host.CalcFields("Tm Count Not At");
            Host.FieldNo("Tm Exist At"):
                Host.CalcFields("Tm Exist At");
            Host.FieldNo("Tm Exist Not At"):
                Host.CalcFields("Tm Exist Not At");
            Host.FieldNo("Tm Sum At"):
                Host.CalcFields("Tm Sum At");
            Host.FieldNo("Tm Sum Not At"):
                Host.CalcFields("Tm Sum Not At");
            Host.FieldNo("Tm Average At"):
                Host.CalcFields("Tm Average At");
            Host.FieldNo("Tm Average Not At"):
                Host.CalcFields("Tm Average Not At");
            Host.FieldNo("Tm Min At"):
                Host.CalcFields("Tm Min At");
            Host.FieldNo("Tm Min Not At"):
                Host.CalcFields("Tm Min Not At");
            Host.FieldNo("Tm Max At"):
                Host.CalcFields("Tm Max At");
            Host.FieldNo("Tm Max Not At"):
                Host.CalcFields("Tm Max Not At");
            Host.FieldNo("Tm Lookup At"):
                Host.CalcFields("Tm Lookup At");
            Host.FieldNo("Tm Lookup Not At"):
                Host.CalcFields("Tm Lookup Not At");
            Host.FieldNo("Moment Count At"):
                Host.CalcFields("Moment Count At");
            Host.FieldNo("Moment Count Not At"):
                Host.CalcFields("Moment Count Not At");
            Host.FieldNo("Moment Exist At"):
                Host.CalcFields("Moment Exist At");
            Host.FieldNo("Moment Exist Not At"):
                Host.CalcFields("Moment Exist Not At");
            Host.FieldNo("Moment Sum At"):
                Host.CalcFields("Moment Sum At");
            Host.FieldNo("Moment Sum Not At"):
                Host.CalcFields("Moment Sum Not At");
            Host.FieldNo("Moment Average At"):
                Host.CalcFields("Moment Average At");
            Host.FieldNo("Moment Average Not At"):
                Host.CalcFields("Moment Average Not At");
            Host.FieldNo("Moment Min At"):
                Host.CalcFields("Moment Min At");
            Host.FieldNo("Moment Min Not At"):
                Host.CalcFields("Moment Min Not At");
            Host.FieldNo("Moment Max At"):
                Host.CalcFields("Moment Max At");
            Host.FieldNo("Moment Max Not At"):
                Host.CalcFields("Moment Max Not At");
            Host.FieldNo("Moment Lookup At"):
                Host.CalcFields("Moment Lookup At");
            Host.FieldNo("Moment Lookup Not At"):
                Host.CalcFields("Moment Lookup Not At");
            Host.FieldNo("Dur Count At"):
                Host.CalcFields("Dur Count At");
            Host.FieldNo("Dur Count Not At"):
                Host.CalcFields("Dur Count Not At");
            Host.FieldNo("Dur Exist At"):
                Host.CalcFields("Dur Exist At");
            Host.FieldNo("Dur Exist Not At"):
                Host.CalcFields("Dur Exist Not At");
            Host.FieldNo("Dur Sum At"):
                Host.CalcFields("Dur Sum At");
            Host.FieldNo("Dur Sum Not At"):
                Host.CalcFields("Dur Sum Not At");
            Host.FieldNo("Dur Average At"):
                Host.CalcFields("Dur Average At");
            Host.FieldNo("Dur Average Not At"):
                Host.CalcFields("Dur Average Not At");
            Host.FieldNo("Dur Min At"):
                Host.CalcFields("Dur Min At");
            Host.FieldNo("Dur Min Not At"):
                Host.CalcFields("Dur Min Not At");
            Host.FieldNo("Dur Max At"):
                Host.CalcFields("Dur Max At");
            Host.FieldNo("Dur Max Not At"):
                Host.CalcFields("Dur Max Not At");
            Host.FieldNo("Dur Lookup At"):
                Host.CalcFields("Dur Lookup At");
            Host.FieldNo("Dur Lookup Not At"):
                Host.CalcFields("Dur Lookup Not At");
            Host.FieldNo("Big Count At"):
                Host.CalcFields("Big Count At");
            Host.FieldNo("Big Count Not At"):
                Host.CalcFields("Big Count Not At");
            Host.FieldNo("Big Exist At"):
                Host.CalcFields("Big Exist At");
            Host.FieldNo("Big Exist Not At"):
                Host.CalcFields("Big Exist Not At");
            Host.FieldNo("Big Sum At"):
                Host.CalcFields("Big Sum At");
            Host.FieldNo("Big Sum Not At"):
                Host.CalcFields("Big Sum Not At");
            Host.FieldNo("Big Average At"):
                Host.CalcFields("Big Average At");
            Host.FieldNo("Big Average Not At"):
                Host.CalcFields("Big Average Not At");
            Host.FieldNo("Big Min At"):
                Host.CalcFields("Big Min At");
            Host.FieldNo("Big Min Not At"):
                Host.CalcFields("Big Min Not At");
            Host.FieldNo("Big Max At"):
                Host.CalcFields("Big Max At");
            Host.FieldNo("Big Max Not At"):
                Host.CalcFields("Big Max Not At");
            Host.FieldNo("Big Lookup At"):
                Host.CalcFields("Big Lookup At");
            Host.FieldNo("Big Lookup Not At"):
                Host.CalcFields("Big Lookup Not At");
            Host.FieldNo("Uid Count At"):
                Host.CalcFields("Uid Count At");
            Host.FieldNo("Uid Count Not At"):
                Host.CalcFields("Uid Count Not At");
            Host.FieldNo("Uid Exist At"):
                Host.CalcFields("Uid Exist At");
            Host.FieldNo("Uid Exist Not At"):
                Host.CalcFields("Uid Exist Not At");
            Host.FieldNo("Uid Sum At"):
                Host.CalcFields("Uid Sum At");
            Host.FieldNo("Uid Sum Not At"):
                Host.CalcFields("Uid Sum Not At");
            Host.FieldNo("Uid Average At"):
                Host.CalcFields("Uid Average At");
            Host.FieldNo("Uid Average Not At"):
                Host.CalcFields("Uid Average Not At");
            Host.FieldNo("Uid Min At"):
                Host.CalcFields("Uid Min At");
            Host.FieldNo("Uid Min Not At"):
                Host.CalcFields("Uid Min Not At");
            Host.FieldNo("Uid Max At"):
                Host.CalcFields("Uid Max At");
            Host.FieldNo("Uid Max Not At"):
                Host.CalcFields("Uid Max Not At");
            Host.FieldNo("Uid Lookup At"):
                Host.CalcFields("Uid Lookup At");
            Host.FieldNo("Uid Lookup Not At"):
                Host.CalcFields("Uid Lookup Not At");
            Host.FieldNo("Kind Count At"):
                Host.CalcFields("Kind Count At");
            Host.FieldNo("Kind Count Not At"):
                Host.CalcFields("Kind Count Not At");
            Host.FieldNo("Kind Exist At"):
                Host.CalcFields("Kind Exist At");
            Host.FieldNo("Kind Exist Not At"):
                Host.CalcFields("Kind Exist Not At");
            Host.FieldNo("Kind Sum At"):
                Host.CalcFields("Kind Sum At");
            Host.FieldNo("Kind Sum Not At"):
                Host.CalcFields("Kind Sum Not At");
            Host.FieldNo("Kind Average At"):
                Host.CalcFields("Kind Average At");
            Host.FieldNo("Kind Average Not At"):
                Host.CalcFields("Kind Average Not At");
            Host.FieldNo("Kind Min At"):
                Host.CalcFields("Kind Min At");
            Host.FieldNo("Kind Min Not At"):
                Host.CalcFields("Kind Min Not At");
            Host.FieldNo("Kind Max At"):
                Host.CalcFields("Kind Max At");
            Host.FieldNo("Kind Max Not At"):
                Host.CalcFields("Kind Max Not At");
            Host.FieldNo("Kind Lookup At"):
                Host.CalcFields("Kind Lookup At");
            Host.FieldNo("Kind Lookup Not At"):
                Host.CalcFields("Kind Lookup Not At");
            Host.FieldNo("Code Count By Pattern"):
                Host.CalcFields("Code Count By Pattern");
            Host.FieldNo("Code Exist By Pattern"):
                Host.CalcFields("Code Exist By Pattern");
            Host.FieldNo("Code Sum By Pattern"):
                Host.CalcFields("Code Sum By Pattern");
            Host.FieldNo("Code Average By Pattern"):
                Host.CalcFields("Code Average By Pattern");
            Host.FieldNo("Code Min By Pattern"):
                Host.CalcFields("Code Min By Pattern");
            Host.FieldNo("Code Max By Pattern"):
                Host.CalcFields("Code Max By Pattern");
            Host.FieldNo("Code Lookup By Pattern"):
                Host.CalcFields("Code Lookup By Pattern");
            Host.FieldNo("Text Count By Pattern"):
                Host.CalcFields("Text Count By Pattern");
            Host.FieldNo("Text Exist By Pattern"):
                Host.CalcFields("Text Exist By Pattern");
            Host.FieldNo("Text Sum By Pattern"):
                Host.CalcFields("Text Sum By Pattern");
            Host.FieldNo("Text Average By Pattern"):
                Host.CalcFields("Text Average By Pattern");
            Host.FieldNo("Text Min By Pattern"):
                Host.CalcFields("Text Min By Pattern");
            Host.FieldNo("Text Max By Pattern"):
                Host.CalcFields("Text Max By Pattern");
            Host.FieldNo("Text Lookup By Pattern"):
                Host.CalcFields("Text Lookup By Pattern");
        end;
    end;

    // The FlowField's value after CalcFields, or the error CalcFields raised: its code and its text.
    local procedure Calc(var Host: Record "AT Type Host"; FieldNo: Integer): Text
    var
        Ref: RecordRef;
    begin
        if not TryCalc(Host, FieldNo) then
            exit('error ' + GetLastErrorCode() + ' ' + GetLastErrorText());
        Ref.GetTable(Host);
        exit(Format(Ref.Field(FieldNo).Value));
    end;

    local procedure NewHost(var Host: Record "AT Type Host")
    begin
        Host."No." := NewGroup();
        Host.Insert();
    end;

    // filter('@') and filter('<>@') over the Date field Day: count, exist, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_DaySource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Day Count At")) + ',' + Calc(Host, Host.FieldNo("Day Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Day Exist At")) + ',' + Calc(Host, Host.FieldNo("Day Exist Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Day Average At")) + ',' + Calc(Host, Host.FieldNo("Day Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Day Min At")) + ',' + Calc(Host, Host.FieldNo("Day Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Day Max At")) + ',' + Calc(Host, Host.FieldNo("Day Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Day Lookup At")) + ',' + Calc(Host, Host.FieldNo("Day Lookup Not At"));
        Assert.AreEqual('count=0,2|' +
            'exist=No,Yes|' +
            'average=0,7.5|' +
            'min=0,5|' +
            'max=0,10|' +
            'lookup=0,10', Seen, 'count, exist, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Integer field Num: count, exist, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_NumSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Num Count At")) + ',' + Calc(Host, Host.FieldNo("Num Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Num Exist At")) + ',' + Calc(Host, Host.FieldNo("Num Exist Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Num Average At")) + ',' + Calc(Host, Host.FieldNo("Num Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Num Min At")) + ',' + Calc(Host, Host.FieldNo("Num Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Num Max At")) + ',' + Calc(Host, Host.FieldNo("Num Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Num Lookup At")) + ',' + Calc(Host, Host.FieldNo("Num Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.|' +
            'exist=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.|' +
            'average=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.|' +
            'min=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.|' +
            'max=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.|' +
            'lookup=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.', Seen, 'count, exist, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Decimal field Dec: count, exist, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_DecSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Dec Count At")) + ',' + Calc(Host, Host.FieldNo("Dec Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Dec Exist At")) + ',' + Calc(Host, Host.FieldNo("Dec Exist Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Dec Average At")) + ',' + Calc(Host, Host.FieldNo("Dec Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Dec Min At")) + ',' + Calc(Host, Host.FieldNo("Dec Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Dec Max At")) + ',' + Calc(Host, Host.FieldNo("Dec Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Dec Lookup At")) + ',' + Calc(Host, Host.FieldNo("Dec Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.|' +
            'exist=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.|' +
            'average=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.|' +
            'min=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.|' +
            'max=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.|' +
            'lookup=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.', Seen, 'count, exist, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Boolean field Flag: count, exist, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_FlagSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Flag Count At")) + ',' + Calc(Host, Host.FieldNo("Flag Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Flag Exist At")) + ',' + Calc(Host, Host.FieldNo("Flag Exist Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Flag Average At")) + ',' + Calc(Host, Host.FieldNo("Flag Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Flag Min At")) + ',' + Calc(Host, Host.FieldNo("Flag Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Flag Max At")) + ',' + Calc(Host, Host.FieldNo("Flag Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Flag Lookup At")) + ',' + Calc(Host, Host.FieldNo("Flag Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.|' +
            'exist=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.|' +
            'average=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.|' +
            'min=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.|' +
            'max=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.|' +
            'lookup=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.', Seen, 'count, exist, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Option field Opt: count, exist, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_OptSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Opt Count At")) + ',' + Calc(Host, Host.FieldNo("Opt Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Opt Exist At")) + ',' + Calc(Host, Host.FieldNo("Opt Exist Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Opt Average At")) + ',' + Calc(Host, Host.FieldNo("Opt Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Opt Min At")) + ',' + Calc(Host, Host.FieldNo("Opt Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Opt Max At")) + ',' + Calc(Host, Host.FieldNo("Opt Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Opt Lookup At")) + ',' + Calc(Host, Host.FieldNo("Opt Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B|' +
            'exist=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B|' +
            'average=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B|' +
            'min=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B|' +
            'max=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B|' +
            'lookup=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B', Seen, 'count, exist, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Option field Opt Cap: count, exist, sum, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_OptCapSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("OptCap Count At")) + ',' + Calc(Host, Host.FieldNo("OptCap Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("OptCap Exist At")) + ',' + Calc(Host, Host.FieldNo("OptCap Exist Not At"));
        Seen += '|' + 'sum=' + Calc(Host, Host.FieldNo("OptCap Sum At")) + ',' + Calc(Host, Host.FieldNo("OptCap Sum Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("OptCap Average At")) + ',' + Calc(Host, Host.FieldNo("OptCap Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("OptCap Min At")) + ',' + Calc(Host, Host.FieldNo("OptCap Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("OptCap Max At")) + ',' + Calc(Host, Host.FieldNo("OptCap Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("OptCap Lookup At")) + ',' + Calc(Host, Host.FieldNo("OptCap Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'exist=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'sum=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'average=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'min=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'max=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'lookup=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta', Seen, 'count, exist, sum, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Time field Tm: count, exist, sum, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_TmSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Tm Count At")) + ',' + Calc(Host, Host.FieldNo("Tm Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Tm Exist At")) + ',' + Calc(Host, Host.FieldNo("Tm Exist Not At"));
        Seen += '|' + 'sum=' + Calc(Host, Host.FieldNo("Tm Sum At")) + ',' + Calc(Host, Host.FieldNo("Tm Sum Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Tm Average At")) + ',' + Calc(Host, Host.FieldNo("Tm Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Tm Min At")) + ',' + Calc(Host, Host.FieldNo("Tm Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Tm Max At")) + ',' + Calc(Host, Host.FieldNo("Tm Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Tm Lookup At")) + ',' + Calc(Host, Host.FieldNo("Tm Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'exist=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'sum=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'average=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'min=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'max=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'lookup=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.', Seen, 'count, exist, sum, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the DateTime field Moment: count, exist, sum, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_MomentSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Moment Count At")) + ',' + Calc(Host, Host.FieldNo("Moment Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Moment Exist At")) + ',' + Calc(Host, Host.FieldNo("Moment Exist Not At"));
        Seen += '|' + 'sum=' + Calc(Host, Host.FieldNo("Moment Sum At")) + ',' + Calc(Host, Host.FieldNo("Moment Sum Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Moment Average At")) + ',' + Calc(Host, Host.FieldNo("Moment Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Moment Min At")) + ',' + Calc(Host, Host.FieldNo("Moment Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Moment Max At")) + ',' + Calc(Host, Host.FieldNo("Moment Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Moment Lookup At")) + ',' + Calc(Host, Host.FieldNo("Moment Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'exist=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'sum=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'average=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'min=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'max=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'lookup=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.', Seen, 'count, exist, sum, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Duration field Dur: count, exist, sum, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_DurSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Dur Count At")) + ',' + Calc(Host, Host.FieldNo("Dur Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Dur Exist At")) + ',' + Calc(Host, Host.FieldNo("Dur Exist Not At"));
        Seen += '|' + 'sum=' + Calc(Host, Host.FieldNo("Dur Sum At")) + ',' + Calc(Host, Host.FieldNo("Dur Sum Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Dur Average At")) + ',' + Calc(Host, Host.FieldNo("Dur Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Dur Min At")) + ',' + Calc(Host, Host.FieldNo("Dur Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Dur Max At")) + ',' + Calc(Host, Host.FieldNo("Dur Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Dur Lookup At")) + ',' + Calc(Host, Host.FieldNo("Dur Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'exist=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'sum=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'average=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'min=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'max=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'lookup=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.', Seen, 'count, exist, sum, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the BigInteger field Big: count, exist, sum, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_BigSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Big Count At")) + ',' + Calc(Host, Host.FieldNo("Big Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Big Exist At")) + ',' + Calc(Host, Host.FieldNo("Big Exist Not At"));
        Seen += '|' + 'sum=' + Calc(Host, Host.FieldNo("Big Sum At")) + ',' + Calc(Host, Host.FieldNo("Big Sum Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Big Average At")) + ',' + Calc(Host, Host.FieldNo("Big Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Big Min At")) + ',' + Calc(Host, Host.FieldNo("Big Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Big Max At")) + ',' + Calc(Host, Host.FieldNo("Big Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Big Lookup At")) + ',' + Calc(Host, Host.FieldNo("Big Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'exist=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'sum=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'average=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'min=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'max=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'lookup=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.,error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.', Seen, 'count, exist, sum, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Guid field Uid: count, exist, sum, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_UidSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Uid Count At")) + ',' + Calc(Host, Host.FieldNo("Uid Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Uid Exist At")) + ',' + Calc(Host, Host.FieldNo("Uid Exist Not At"));
        Seen += '|' + 'sum=' + Calc(Host, Host.FieldNo("Uid Sum At")) + ',' + Calc(Host, Host.FieldNo("Uid Sum Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Uid Average At")) + ',' + Calc(Host, Host.FieldNo("Uid Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Uid Min At")) + ',' + Calc(Host, Host.FieldNo("Uid Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Uid Max At")) + ',' + Calc(Host, Host.FieldNo("Uid Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Uid Lookup At")) + ',' + Calc(Host, Host.FieldNo("Uid Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.,error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'exist=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.,error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'sum=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.,error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'average=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.,error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'min=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.,error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'max=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.,error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'lookup=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.,error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.', Seen, 'count, exist, sum, average, min, max, lookup; @, <>@');
    end;

    // filter('@') and filter('<>@') over the Enum "AT Kind" field Kind: count, exist, sum, average, min, max, lookup.
    [Test]
    procedure AtSignAlone_KindSource_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        Seen := 'count=' + Calc(Host, Host.FieldNo("Kind Count At")) + ',' + Calc(Host, Host.FieldNo("Kind Count Not At"));
        Seen += '|' + 'exist=' + Calc(Host, Host.FieldNo("Kind Exist At")) + ',' + Calc(Host, Host.FieldNo("Kind Exist Not At"));
        Seen += '|' + 'sum=' + Calc(Host, Host.FieldNo("Kind Sum At")) + ',' + Calc(Host, Host.FieldNo("Kind Sum Not At"));
        Seen += '|' + 'average=' + Calc(Host, Host.FieldNo("Kind Average At")) + ',' + Calc(Host, Host.FieldNo("Kind Average Not At"));
        Seen += '|' + 'min=' + Calc(Host, Host.FieldNo("Kind Min At")) + ',' + Calc(Host, Host.FieldNo("Kind Min Not At"));
        Seen += '|' + 'max=' + Calc(Host, Host.FieldNo("Kind Max At")) + ',' + Calc(Host, Host.FieldNo("Kind Max Not At"));
        Seen += '|' + 'lookup=' + Calc(Host, Host.FieldNo("Kind Lookup At")) + ',' + Calc(Host, Host.FieldNo("Kind Lookup Not At"));
        Assert.AreEqual('count=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second|' +
            'exist=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second|' +
            'sum=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second|' +
            'average=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second|' +
            'min=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second|' +
            'max=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second|' +
            'lookup=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second,error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second', Seen, 'count, exist, sum, average, min, max, lookup; @, <>@');
    end;

    [TryFunction]
    local procedure TrySetLanguage(Language: Integer)
    begin
        GlobalLanguage(Language);
    end;

    // The errors of count with filter('@') over each type, in a session of language 1031; the language is 1033 again after.
    [Test]
    procedure AtSignAlone_OtherSourceErrors_Language1031()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        if not TrySetLanguage(1031) then
            Seen := 'no language: ' + GetLastErrorText()
        else begin
            Seen := 'Day=' + Calc(Host, Host.FieldNo("Day Count At"));
            Seen += '|' + 'Num=' + Calc(Host, Host.FieldNo("Num Count At"));
            Seen += '|' + 'Dec=' + Calc(Host, Host.FieldNo("Dec Count At"));
            Seen += '|' + 'Flag=' + Calc(Host, Host.FieldNo("Flag Count At"));
            Seen += '|' + 'Opt=' + Calc(Host, Host.FieldNo("Opt Count At"));
            Seen += '|' + 'OptCap=' + Calc(Host, Host.FieldNo("OptCap Count At"));
            Seen += '|' + 'Tm=' + Calc(Host, Host.FieldNo("Tm Count At"));
            Seen += '|' + 'Moment=' + Calc(Host, Host.FieldNo("Moment Count At"));
            Seen += '|' + 'Dur=' + Calc(Host, Host.FieldNo("Dur Count At"));
            Seen += '|' + 'Big=' + Calc(Host, Host.FieldNo("Big Count At"));
            Seen += '|' + 'Uid=' + Calc(Host, Host.FieldNo("Uid Count At"));
            Seen += '|' + 'Kind=' + Calc(Host, Host.FieldNo("Kind Count At"));
            GlobalLanguage(1033);
        end;
        Assert.AreEqual('Day=0|' +
            'Num=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.|' +
            'Dec=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.|' +
            'Flag=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.|' +
            'Opt=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B|' +
            'OptCap=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'Tm=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'Moment=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'Dur=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'Big=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'Uid=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'Kind=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second', Seen, 'count @ over each type in language 1031');
    end;

    // The errors of count with filter('@') over each type, in a session of language 1030; the language is 1033 again after.
    [Test]
    procedure AtSignAlone_OtherSourceErrors_Language1030()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        NewHost(Host);
        if not TrySetLanguage(1030) then
            Seen := 'no language: ' + GetLastErrorText()
        else begin
            Seen := 'Day=' + Calc(Host, Host.FieldNo("Day Count At"));
            Seen += '|' + 'Num=' + Calc(Host, Host.FieldNo("Num Count At"));
            Seen += '|' + 'Dec=' + Calc(Host, Host.FieldNo("Dec Count At"));
            Seen += '|' + 'Flag=' + Calc(Host, Host.FieldNo("Flag Count At"));
            Seen += '|' + 'Opt=' + Calc(Host, Host.FieldNo("Opt Count At"));
            Seen += '|' + 'OptCap=' + Calc(Host, Host.FieldNo("OptCap Count At"));
            Seen += '|' + 'Tm=' + Calc(Host, Host.FieldNo("Tm Count At"));
            Seen += '|' + 'Moment=' + Calc(Host, Host.FieldNo("Moment Count At"));
            Seen += '|' + 'Dur=' + Calc(Host, Host.FieldNo("Dur Count At"));
            Seen += '|' + 'Big=' + Calc(Host, Host.FieldNo("Big Count At"));
            Seen += '|' + 'Uid=' + Calc(Host, Host.FieldNo("Uid Count At"));
            Seen += '|' + 'Kind=' + Calc(Host, Host.FieldNo("Kind Count At"));
            GlobalLanguage(1033);
        end;
        Assert.AreEqual('Day=0|' +
            'Num=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Integer.|' +
            'Dec=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Decimal.|' +
            'Flag=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Boolean.|' +
            'Opt=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: A,B|' +
            'OptCap=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: Alpha,Beta|' +
            'Tm=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Time.|' +
            'Moment=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type DateTime.|' +
            'Dur=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type Duration.|' +
            'Big=error InvalidFilterExpression:Evaluate The value "@" can''t be evaluated into type BigInteger.|' +
            'Uid=error InvalidFilterExpression:InvalidGuidFormat Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.|' +
            'Kind=error InvalidFilterExpression:InvalidOptionString ''@'' is not an option. The existing options are: None,First,Second', Seen, 'count @ over each type in language 1030');
    end;

    // Pattern set to Pattern on Host, then field(filter(Pattern)) over the Code and the Text field with every method.
    local procedure PatternReads(var Host: Record "AT Type Host"; Pattern: Text): Text
    var
        Seen: Text;
    begin
        Host.Pattern := CopyStr(Pattern, 1, MaxStrLen(Host.Pattern));
        Host.Modify();
        Seen := Pattern + ':';
        Seen += 'Code=' + Calc(Host, Host.FieldNo("Code Count By Pattern")) + ',' + Calc(Host, Host.FieldNo("Code Exist By Pattern")) + ',' + Calc(Host, Host.FieldNo("Code Sum By Pattern")) + ',' + Calc(Host, Host.FieldNo("Code Average By Pattern")) + ',' + Calc(Host, Host.FieldNo("Code Min By Pattern")) + ',' + Calc(Host, Host.FieldNo("Code Max By Pattern")) + ',' + Calc(Host, Host.FieldNo("Code Lookup By Pattern")) + ';';
        Seen += 'Text=' + Calc(Host, Host.FieldNo("Text Count By Pattern")) + ',' + Calc(Host, Host.FieldNo("Text Exist By Pattern")) + ',' + Calc(Host, Host.FieldNo("Text Sum By Pattern")) + ',' + Calc(Host, Host.FieldNo("Text Average By Pattern")) + ',' + Calc(Host, Host.FieldNo("Text Min By Pattern")) + ',' + Calc(Host, Host.FieldNo("Text Max By Pattern")) + ',' + Calc(Host, Host.FieldNo("Text Lookup By Pattern")) + ';';
        exit(Seen);
    end;

    // A host whose Pattern holds @, then <>@: field(filter(Pattern)) over the Code and the Text field, every
    // method; the source rows are in the database.
    [Test]
    procedure AtSignAlone_FieldFilterOfPattern_Methods()
    var
        Host: Record "AT Type Host";
        Seen: Text;
    begin
        Host."No." := NewGroup();
        Host.Insert();
        Seen := PatternReads(Host, '@') + PatternReads(Host, '<>@');
        Assert.AreEqual('@:Code=1,Yes,5,5,5,5,5;Text=1,Yes,5,5,5,5,5;<>@:Code=1,Yes,10,10,10,10,10;Text=1,Yes,10,10,10,10,10;', Seen, '@ then <>@; code, text: count, exist, sum, average, min, max, lookup');
    end;

    // A temporary host whose Pattern holds @, then <>@: field(filter(Pattern)) over the Code and the Text field, every
    // method; the source rows are in the database.
    [Test]
    procedure AtSignAlone_FieldFilterOfPattern_Methods_TemporaryHost()
    var
        Host: Record "AT Type Host" temporary;
        Seen: Text;
    begin
        Host."No." := NewGroup();
        Host.Insert();
        Seen := PatternReads(Host, '@') + PatternReads(Host, '<>@');
        Assert.AreEqual('@:Code=1,Yes,5,5,5,5,5;Text=1,Yes,5,5,5,5,5;<>@:Code=1,Yes,10,10,10,10,10;Text=1,Yes,10,10,10,10,10;', Seen, '@ then <>@; code, text: count, exist, sum, average, min, max, lookup');
    end;
}
