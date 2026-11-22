// Neighbors of al-runner issue 8637 that no cell of the corpus probe BgTaskQueryCache.al measures (appended to it), on its
// tables 97593 "BQ Wide", 97594 "BQ Child", 97597 "BQ Calc", page 97519 "BQ Plain Rows" and a table of their own. Each test
// asserts BC's answer (a local BC 29.1 container) as 'first reads / reads again'. They are past what the cells of BQ Tests 9
// measure: 100001 filters (on a table of 503 fields; the cells reach 100000 at :5564; that test runs from al-test-cli's
// Issue8637Heavy.al, past the neighbor check's CPU limit on main), a FlowField filter next to the key set
// to its value in three filter groups (the cells set it in two, :5571), and the FlowField filters @k (a value compared
// without regard to case), <>K* (a pattern not matched) and K*|X (a | of patterns) (the cells measure the case-sensitive
// patterns, :5599; @k reads the rows as they are, as @k* does at :4756). Past :3391 (the worker's own record sets filter
// group 1 while the test session holds none): the test session holds a filter in filter group 1 and the worker counts a
// fresh Record and a RecordRef of the table. And a sum whose field() is a FlowField of the row after an uncommitted note (a
// page background task's repeated CalcFields, drafted as an issue of its own). The neighbors of "@ alone" (BQ Tests 9b and 9c)
// are in the "@ alone" pull request's folder, tests/neighbors/at-sign-alone-database.

table 97532 "BQ Gate Widest"
{
    DataClassification = SystemMetadata;
    ReplicateData = false;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; F3; Integer) { }
        field(4; F4; Integer) { }
        field(5; F5; Integer) { }
        field(6; F6; Integer) { }
        field(7; F7; Integer) { }
        field(8; F8; Integer) { }
        field(9; F9; Integer) { }
        field(10; F10; Integer) { }
        field(11; F11; Integer) { }
        field(12; F12; Integer) { }
        field(13; F13; Integer) { }
        field(14; F14; Integer) { }
        field(15; F15; Integer) { }
        field(16; F16; Integer) { }
        field(17; F17; Integer) { }
        field(18; F18; Integer) { }
        field(19; F19; Integer) { }
        field(20; F20; Integer) { }
        field(21; F21; Integer) { }
        field(22; F22; Integer) { }
        field(23; F23; Integer) { }
        field(24; F24; Integer) { }
        field(25; F25; Integer) { }
        field(26; F26; Integer) { }
        field(27; F27; Integer) { }
        field(28; F28; Integer) { }
        field(29; F29; Integer) { }
        field(30; F30; Integer) { }
        field(31; F31; Integer) { }
        field(32; F32; Integer) { }
        field(33; F33; Integer) { }
        field(34; F34; Integer) { }
        field(35; F35; Integer) { }
        field(36; F36; Integer) { }
        field(37; F37; Integer) { }
        field(38; F38; Integer) { }
        field(39; F39; Integer) { }
        field(40; F40; Integer) { }
        field(41; F41; Integer) { }
        field(42; F42; Integer) { }
        field(43; F43; Integer) { }
        field(44; F44; Integer) { }
        field(45; F45; Integer) { }
        field(46; F46; Integer) { }
        field(47; F47; Integer) { }
        field(48; F48; Integer) { }
        field(49; F49; Integer) { }
        field(50; F50; Integer) { }
        field(51; F51; Integer) { }
        field(52; F52; Integer) { }
        field(53; F53; Integer) { }
        field(54; F54; Integer) { }
        field(55; F55; Integer) { }
        field(56; F56; Integer) { }
        field(57; F57; Integer) { }
        field(58; F58; Integer) { }
        field(59; F59; Integer) { }
        field(60; F60; Integer) { }
        field(61; F61; Integer) { }
        field(62; F62; Integer) { }
        field(63; F63; Integer) { }
        field(64; F64; Integer) { }
        field(65; F65; Integer) { }
        field(66; F66; Integer) { }
        field(67; F67; Integer) { }
        field(68; F68; Integer) { }
        field(69; F69; Integer) { }
        field(70; F70; Integer) { }
        field(71; F71; Integer) { }
        field(72; F72; Integer) { }
        field(73; F73; Integer) { }
        field(74; F74; Integer) { }
        field(75; F75; Integer) { }
        field(76; F76; Integer) { }
        field(77; F77; Integer) { }
        field(78; F78; Integer) { }
        field(79; F79; Integer) { }
        field(80; F80; Integer) { }
        field(81; F81; Integer) { }
        field(82; F82; Integer) { }
        field(83; F83; Integer) { }
        field(84; F84; Integer) { }
        field(85; F85; Integer) { }
        field(86; F86; Integer) { }
        field(87; F87; Integer) { }
        field(88; F88; Integer) { }
        field(89; F89; Integer) { }
        field(90; F90; Integer) { }
        field(91; F91; Integer) { }
        field(92; F92; Integer) { }
        field(93; F93; Integer) { }
        field(94; F94; Integer) { }
        field(95; F95; Integer) { }
        field(96; F96; Integer) { }
        field(97; F97; Integer) { }
        field(98; F98; Integer) { }
        field(99; F99; Integer) { }
        field(100; F100; Integer) { }
        field(101; F101; Integer) { }
        field(102; F102; Integer) { }
        field(103; F103; Integer) { }
        field(104; F104; Integer) { }
        field(105; F105; Integer) { }
        field(106; F106; Integer) { }
        field(107; F107; Integer) { }
        field(108; F108; Integer) { }
        field(109; F109; Integer) { }
        field(110; F110; Integer) { }
        field(111; F111; Integer) { }
        field(112; F112; Integer) { }
        field(113; F113; Integer) { }
        field(114; F114; Integer) { }
        field(115; F115; Integer) { }
        field(116; F116; Integer) { }
        field(117; F117; Integer) { }
        field(118; F118; Integer) { }
        field(119; F119; Integer) { }
        field(120; F120; Integer) { }
        field(121; F121; Integer) { }
        field(122; F122; Integer) { }
        field(123; F123; Integer) { }
        field(124; F124; Integer) { }
        field(125; F125; Integer) { }
        field(126; F126; Integer) { }
        field(127; F127; Integer) { }
        field(128; F128; Integer) { }
        field(129; F129; Integer) { }
        field(130; F130; Integer) { }
        field(131; F131; Integer) { }
        field(132; F132; Integer) { }
        field(133; F133; Integer) { }
        field(134; F134; Integer) { }
        field(135; F135; Integer) { }
        field(136; F136; Integer) { }
        field(137; F137; Integer) { }
        field(138; F138; Integer) { }
        field(139; F139; Integer) { }
        field(140; F140; Integer) { }
        field(141; F141; Integer) { }
        field(142; F142; Integer) { }
        field(143; F143; Integer) { }
        field(144; F144; Integer) { }
        field(145; F145; Integer) { }
        field(146; F146; Integer) { }
        field(147; F147; Integer) { }
        field(148; F148; Integer) { }
        field(149; F149; Integer) { }
        field(150; F150; Integer) { }
        field(151; F151; Integer) { }
        field(152; F152; Integer) { }
        field(153; F153; Integer) { }
        field(154; F154; Integer) { }
        field(155; F155; Integer) { }
        field(156; F156; Integer) { }
        field(157; F157; Integer) { }
        field(158; F158; Integer) { }
        field(159; F159; Integer) { }
        field(160; F160; Integer) { }
        field(161; F161; Integer) { }
        field(162; F162; Integer) { }
        field(163; F163; Integer) { }
        field(164; F164; Integer) { }
        field(165; F165; Integer) { }
        field(166; F166; Integer) { }
        field(167; F167; Integer) { }
        field(168; F168; Integer) { }
        field(169; F169; Integer) { }
        field(170; F170; Integer) { }
        field(171; F171; Integer) { }
        field(172; F172; Integer) { }
        field(173; F173; Integer) { }
        field(174; F174; Integer) { }
        field(175; F175; Integer) { }
        field(176; F176; Integer) { }
        field(177; F177; Integer) { }
        field(178; F178; Integer) { }
        field(179; F179; Integer) { }
        field(180; F180; Integer) { }
        field(181; F181; Integer) { }
        field(182; F182; Integer) { }
        field(183; F183; Integer) { }
        field(184; F184; Integer) { }
        field(185; F185; Integer) { }
        field(186; F186; Integer) { }
        field(187; F187; Integer) { }
        field(188; F188; Integer) { }
        field(189; F189; Integer) { }
        field(190; F190; Integer) { }
        field(191; F191; Integer) { }
        field(192; F192; Integer) { }
        field(193; F193; Integer) { }
        field(194; F194; Integer) { }
        field(195; F195; Integer) { }
        field(196; F196; Integer) { }
        field(197; F197; Integer) { }
        field(198; F198; Integer) { }
        field(199; F199; Integer) { }
        field(200; F200; Integer) { }
        field(201; F201; Integer) { }
        field(202; F202; Integer) { }
        field(203; F203; Integer) { }
        field(204; F204; Integer) { }
        field(205; F205; Integer) { }
        field(206; F206; Integer) { }
        field(207; F207; Integer) { }
        field(208; F208; Integer) { }
        field(209; F209; Integer) { }
        field(210; F210; Integer) { }
        field(211; F211; Integer) { }
        field(212; F212; Integer) { }
        field(213; F213; Integer) { }
        field(214; F214; Integer) { }
        field(215; F215; Integer) { }
        field(216; F216; Integer) { }
        field(217; F217; Integer) { }
        field(218; F218; Integer) { }
        field(219; F219; Integer) { }
        field(220; F220; Integer) { }
        field(221; F221; Integer) { }
        field(222; F222; Integer) { }
        field(223; F223; Integer) { }
        field(224; F224; Integer) { }
        field(225; F225; Integer) { }
        field(226; F226; Integer) { }
        field(227; F227; Integer) { }
        field(228; F228; Integer) { }
        field(229; F229; Integer) { }
        field(230; F230; Integer) { }
        field(231; F231; Integer) { }
        field(232; F232; Integer) { }
        field(233; F233; Integer) { }
        field(234; F234; Integer) { }
        field(235; F235; Integer) { }
        field(236; F236; Integer) { }
        field(237; F237; Integer) { }
        field(238; F238; Integer) { }
        field(239; F239; Integer) { }
        field(240; F240; Integer) { }
        field(241; F241; Integer) { }
        field(242; F242; Integer) { }
        field(243; F243; Integer) { }
        field(244; F244; Integer) { }
        field(245; F245; Integer) { }
        field(246; F246; Integer) { }
        field(247; F247; Integer) { }
        field(248; F248; Integer) { }
        field(249; F249; Integer) { }
        field(250; F250; Integer) { }
        field(251; F251; Integer) { }
        field(252; F252; Integer) { }
        field(253; F253; Integer) { }
        field(254; F254; Integer) { }
        field(255; F255; Integer) { }
        field(256; F256; Integer) { }
        field(257; F257; Integer) { }
        field(258; F258; Integer) { }
        field(259; F259; Integer) { }
        field(260; F260; Integer) { }
        field(261; F261; Integer) { }
        field(262; F262; Integer) { }
        field(263; F263; Integer) { }
        field(264; F264; Integer) { }
        field(265; F265; Integer) { }
        field(266; F266; Integer) { }
        field(267; F267; Integer) { }
        field(268; F268; Integer) { }
        field(269; F269; Integer) { }
        field(270; F270; Integer) { }
        field(271; F271; Integer) { }
        field(272; F272; Integer) { }
        field(273; F273; Integer) { }
        field(274; F274; Integer) { }
        field(275; F275; Integer) { }
        field(276; F276; Integer) { }
        field(277; F277; Integer) { }
        field(278; F278; Integer) { }
        field(279; F279; Integer) { }
        field(280; F280; Integer) { }
        field(281; F281; Integer) { }
        field(282; F282; Integer) { }
        field(283; F283; Integer) { }
        field(284; F284; Integer) { }
        field(285; F285; Integer) { }
        field(286; F286; Integer) { }
        field(287; F287; Integer) { }
        field(288; F288; Integer) { }
        field(289; F289; Integer) { }
        field(290; F290; Integer) { }
        field(291; F291; Integer) { }
        field(292; F292; Integer) { }
        field(293; F293; Integer) { }
        field(294; F294; Integer) { }
        field(295; F295; Integer) { }
        field(296; F296; Integer) { }
        field(297; F297; Integer) { }
        field(298; F298; Integer) { }
        field(299; F299; Integer) { }
        field(300; F300; Integer) { }
        field(301; F301; Integer) { }
        field(302; F302; Integer) { }
        field(303; F303; Integer) { }
        field(304; F304; Integer) { }
        field(305; F305; Integer) { }
        field(306; F306; Integer) { }
        field(307; F307; Integer) { }
        field(308; F308; Integer) { }
        field(309; F309; Integer) { }
        field(310; F310; Integer) { }
        field(311; F311; Integer) { }
        field(312; F312; Integer) { }
        field(313; F313; Integer) { }
        field(314; F314; Integer) { }
        field(315; F315; Integer) { }
        field(316; F316; Integer) { }
        field(317; F317; Integer) { }
        field(318; F318; Integer) { }
        field(319; F319; Integer) { }
        field(320; F320; Integer) { }
        field(321; F321; Integer) { }
        field(322; F322; Integer) { }
        field(323; F323; Integer) { }
        field(324; F324; Integer) { }
        field(325; F325; Integer) { }
        field(326; F326; Integer) { }
        field(327; F327; Integer) { }
        field(328; F328; Integer) { }
        field(329; F329; Integer) { }
        field(330; F330; Integer) { }
        field(331; F331; Integer) { }
        field(332; F332; Integer) { }
        field(333; F333; Integer) { }
        field(334; F334; Integer) { }
        field(335; F335; Integer) { }
        field(336; F336; Integer) { }
        field(337; F337; Integer) { }
        field(338; F338; Integer) { }
        field(339; F339; Integer) { }
        field(340; F340; Integer) { }
        field(341; F341; Integer) { }
        field(342; F342; Integer) { }
        field(343; F343; Integer) { }
        field(344; F344; Integer) { }
        field(345; F345; Integer) { }
        field(346; F346; Integer) { }
        field(347; F347; Integer) { }
        field(348; F348; Integer) { }
        field(349; F349; Integer) { }
        field(350; F350; Integer) { }
        field(351; F351; Integer) { }
        field(352; F352; Integer) { }
        field(353; F353; Integer) { }
        field(354; F354; Integer) { }
        field(355; F355; Integer) { }
        field(356; F356; Integer) { }
        field(357; F357; Integer) { }
        field(358; F358; Integer) { }
        field(359; F359; Integer) { }
        field(360; F360; Integer) { }
        field(361; F361; Integer) { }
        field(362; F362; Integer) { }
        field(363; F363; Integer) { }
        field(364; F364; Integer) { }
        field(365; F365; Integer) { }
        field(366; F366; Integer) { }
        field(367; F367; Integer) { }
        field(368; F368; Integer) { }
        field(369; F369; Integer) { }
        field(370; F370; Integer) { }
        field(371; F371; Integer) { }
        field(372; F372; Integer) { }
        field(373; F373; Integer) { }
        field(374; F374; Integer) { }
        field(375; F375; Integer) { }
        field(376; F376; Integer) { }
        field(377; F377; Integer) { }
        field(378; F378; Integer) { }
        field(379; F379; Integer) { }
        field(380; F380; Integer) { }
        field(381; F381; Integer) { }
        field(382; F382; Integer) { }
        field(383; F383; Integer) { }
        field(384; F384; Integer) { }
        field(385; F385; Integer) { }
        field(386; F386; Integer) { }
        field(387; F387; Integer) { }
        field(388; F388; Integer) { }
        field(389; F389; Integer) { }
        field(390; F390; Integer) { }
        field(391; F391; Integer) { }
        field(392; F392; Integer) { }
        field(393; F393; Integer) { }
        field(394; F394; Integer) { }
        field(395; F395; Integer) { }
        field(396; F396; Integer) { }
        field(397; F397; Integer) { }
        field(398; F398; Integer) { }
        field(399; F399; Integer) { }
        field(400; F400; Integer) { }
        field(401; F401; Integer) { }
        field(402; F402; Integer) { }
        field(403; F403; Integer) { }
        field(404; F404; Integer) { }
        field(405; F405; Integer) { }
        field(406; F406; Integer) { }
        field(407; F407; Integer) { }
        field(408; F408; Integer) { }
        field(409; F409; Integer) { }
        field(410; F410; Integer) { }
        field(411; F411; Integer) { }
        field(412; F412; Integer) { }
        field(413; F413; Integer) { }
        field(414; F414; Integer) { }
        field(415; F415; Integer) { }
        field(416; F416; Integer) { }
        field(417; F417; Integer) { }
        field(418; F418; Integer) { }
        field(419; F419; Integer) { }
        field(420; F420; Integer) { }
        field(421; F421; Integer) { }
        field(422; F422; Integer) { }
        field(423; F423; Integer) { }
        field(424; F424; Integer) { }
        field(425; F425; Integer) { }
        field(426; F426; Integer) { }
        field(427; F427; Integer) { }
        field(428; F428; Integer) { }
        field(429; F429; Integer) { }
        field(430; F430; Integer) { }
        field(431; F431; Integer) { }
        field(432; F432; Integer) { }
        field(433; F433; Integer) { }
        field(434; F434; Integer) { }
        field(435; F435; Integer) { }
        field(436; F436; Integer) { }
        field(437; F437; Integer) { }
        field(438; F438; Integer) { }
        field(439; F439; Integer) { }
        field(440; F440; Integer) { }
        field(441; F441; Integer) { }
        field(442; F442; Integer) { }
        field(443; F443; Integer) { }
        field(444; F444; Integer) { }
        field(445; F445; Integer) { }
        field(446; F446; Integer) { }
        field(447; F447; Integer) { }
        field(448; F448; Integer) { }
        field(449; F449; Integer) { }
        field(450; F450; Integer) { }
        field(451; F451; Integer) { }
        field(452; F452; Integer) { }
        field(453; F453; Integer) { }
        field(454; F454; Integer) { }
        field(455; F455; Integer) { }
        field(456; F456; Integer) { }
        field(457; F457; Integer) { }
        field(458; F458; Integer) { }
        field(459; F459; Integer) { }
        field(460; F460; Integer) { }
        field(461; F461; Integer) { }
        field(462; F462; Integer) { }
        field(463; F463; Integer) { }
        field(464; F464; Integer) { }
        field(465; F465; Integer) { }
        field(466; F466; Integer) { }
        field(467; F467; Integer) { }
        field(468; F468; Integer) { }
        field(469; F469; Integer) { }
        field(470; F470; Integer) { }
        field(471; F471; Integer) { }
        field(472; F472; Integer) { }
        field(473; F473; Integer) { }
        field(474; F474; Integer) { }
        field(475; F475; Integer) { }
        field(476; F476; Integer) { }
        field(477; F477; Integer) { }
        field(478; F478; Integer) { }
        field(479; F479; Integer) { }
        field(480; F480; Integer) { }
        field(481; F481; Integer) { }
        field(482; F482; Integer) { }
        field(483; F483; Integer) { }
        field(484; F484; Integer) { }
        field(485; F485; Integer) { }
        field(486; F486; Integer) { }
        field(487; F487; Integer) { }
        field(488; F488; Integer) { }
        field(489; F489; Integer) { }
        field(490; F490; Integer) { }
        field(491; F491; Integer) { }
        field(492; F492; Integer) { }
        field(493; F493; Integer) { }
        field(494; F494; Integer) { }
        field(495; F495; Integer) { }
        field(496; F496; Integer) { }
        field(497; F497; Integer) { }
        field(498; F498; Integer) { }
        field(499; F499; Integer) { }
        field(500; F500; Integer) { }
        field(501; F501; Integer) { }
        field(502; F502; Integer) { }
        field(503; F503; Integer) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

// The worker: FindFirst, FindLast, Count and IsEmpty of the filters a Mode names, or the Counts of 'freshcount'.
codeunit 97530 "BQ Gate Reader"
{
    trigger OnRun()
    var
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        WideRef: RecordRef;
        Reader: Codeunit "BQ Reader 8";
        Reader8b: Codeunit "BQ Reader 8b";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Group: Text;
        Mode: Text;
        K: Integer;
        FieldNo: Integer;
        FilterGroupNo: Integer;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('Group', Group);
        Parameters.Get('Mode', Mode);
        if Mode.StartsWith('8b-') then begin
            Results.Add('Seen', Reader8b.Forms(CopyStr(Mode, 4), Group));
            Page.SetBackgroundTaskResult(Results);
            exit;
        end;
        case Mode of
            'many100001':
                begin
                    WideRef.Open(Database::"BQ Gate Widest");
                    for K := 0 to 100000 do begin
                        FieldNo := K mod 503 + 1;
                        FilterGroupNo := K div 503;
                        if FilterGroupNo > 0 then
                            FilterGroupNo += 1;
                        WideRef.FilterGroup(FilterGroupNo);
                        case FieldNo of
                            1:
                                WideRef.Field(1).SetRange(Group);
                            2:
                                WideRef.Field(2).SetFilter('<>zz');
                            else
                                WideRef.Field(FieldNo).SetRange(0);
                        end;
                    end;
                    WideRef.FilterGroup(0);
                    Results.Add('Seen', Reader.RefForms(WideRef));
                end;
            'keythree':
                begin
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(2);
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(5);
                    Wide.SetRange("No.", Group);
                    Wide.FilterGroup(0);
                    Wide.SetRange("Child Count", 1);
                    Results.Add('Seen', Reader.FlowForms(Wide));
                end;
            'calcatk', 'calcnotlike', 'calcorlike':
                begin
                    Calc.SetRange("No.", Group);
                    case Mode of
                        'calcatk':
                            Calc.SetFilter("Line Kind", '@k');
                        'calcnotlike':
                            Calc.SetFilter("Line Kind", '<>K*');
                        'calcorlike':
                            Calc.SetFilter("Line Kind", 'K*|X');
                    end;
                    Results.Add('Seen', Reader.CalcForms(Calc));
                end;
            'freshcount':
                begin
                    WideRef.Open(Database::"BQ Wide");
                    Results.Add('Seen', Format(Wide.Count()) + ',' + Format(WideRef.Count()));
                end;
        end;
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 97531 "BQ Gate Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure NewGroup(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 18));
    end;

    local procedure AddChild(Parent: Code[20]; LineNo: Integer)
    var
        Child: Record "BQ Child";
    begin
        Child.Parent := Parent;
        Child."Line No." := LineNo;
        Child.Kind := 'K';
        Child.Amount := 5;
        Child.Insert();
        Child.SetRange(Parent, Parent);
        if Child.FindFirst() then;
    end;

    local procedure Run(var Rows: TestPage "BQ Plain Rows"; Mode: Text; Group: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('Group', Group);
        Parameters.Add('Mode', Mode);
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Gate Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    // The row a Mode reads committed (table 97593, 97597, or 97591 with its line 1 of table 97592; none for 'many'); the four
    // reads; the test inserts a row of table 97532 ('many'), a note of table 97595 (table 97591) or a line of table 97594,
    // without committing; the four reads again.
    local procedure Reads(Mode: Text): Text
    var
        Rows: TestPage "BQ Plain Rows";
        Wide: Record "BQ Wide";
        Calc: Record "BQ Calc";
        Busy: Record "BQ Busy";
        Line: Record "BQ Noted Line";
        Note: Record "BQ Note";
        Widest: Record "BQ Gate Widest";
        G: Code[20];
        Seen: Text;
    begin
        G := NewGroup();
        case true of
            Mode.StartsWith('many'):
                ;
            Mode.Contains('calc'):
                begin
                    Calc."No." := G;
                    Calc.Description := 'a';
                    Calc.Insert();
                end;
            Mode.Contains('amount'):
                begin
                    Busy."No." := G;
                    Busy.Description := 'a';
                    Busy.Insert();
                    Line.Parent := G;
                    Line."Line No." := 1;
                    Line.Amount := 5;
                    Line.Insert();
                end;
            else begin
                Wide."No." := G;
                Wide.Description := 'a';
                Wide.Insert();
            end;
        end;
        Commit();
        Rows.OpenView();
        Seen := Run(Rows, Mode, G);
        case true of
            Mode.StartsWith('many'):
                begin
                    Widest."No." := G;
                    Widest.Description := 'a';
                    Widest.Insert();
                    Widest.SetRange("No.", G);
                    if Widest.FindFirst() then;
                end;
            Mode.Contains('amount'):
                begin
                    Note.Parent := G;
                    Note."Line No." := 1;
                    Note.Insert();
                    Note.SetRange(Parent, G);
                    if Note.FindFirst() then;
                end;
            else
                AddChild(G, 1);
        end;
        Seen += ' / ' + Run(Rows, Mode, G);
        Rows.Close();
        exit(Seen);
    end;

    [Test]
    procedure FlowFieldKeyInThreeGroups_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('keythree'), 'after the insert');
    end;

    [Test]
    procedure FlowFieldCaseInsensitiveValue_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / a|a|1|No', Reads('calcatk'), '@k');
    end;

    [Test]
    procedure FlowFieldPatternNotMatched_LineAfterFirstRead()
    begin
        Assert.AreEqual('a|a|1|No / a|a|0|Yes', Reads('calcnotlike'), '<>K*');
    end;

    [Test]
    procedure FlowFieldOrOfPatterns_LineAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|1|No', Reads('calcorlike'), 'K*|X');
    end;

    [Test]
    procedure FlowFieldOverAFlowFieldOfTheRow_NoteAfterFirstRead()
    begin
        Assert.AreEqual('-|-|0|Yes / -|-|0|Yes', Reads('8b-amount'), 'Amount At Note Count 5');
    end;

    // The test session sets filter group 1 of table 97593 to F3 7 (an Integer field outside the key; every other row holds 0);
    // the worker counts a fresh Record and a RecordRef of the table (the task of page 97518, on another table); the test
    // inserts a row with F3 7; the worker counts again; the test takes its filter group 1 back. The second counts less the
    // first, as 'Record,RecordRef'.
    [Test]
    procedure TestSessionGroup1_WorkerCountsFresh_InsertAfterFirstRead()
    var
        Rows: TestPage "BQ Single Rows";
        Wide: Record "BQ Wide";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        G: Code[20];
        First: Text;
        Second: Text;
        Before: Integer;
        After: Integer;
        Seen: Text;
        K: Integer;
    begin
        G := NewGroup();
        Wide.FilterGroup(1);
        Wide.SetRange(F3, 7);
        Wide.FilterGroup(0);
        Rows.OpenView();
        Parameters.Add('Group', G);
        Parameters.Add('Mode', 'freshcount');
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Gate Reader", Parameters, false);
        Results.Get('Seen', First);
        Wide."No." := G;
        Wide.Description := 'a';
        Wide.F3 := 7;
        Wide.Insert();
        if Wide.FindFirst() then;
        Results := Rows.RunPageBackgroundTask(Codeunit::"BQ Gate Reader", Parameters, false);
        Results.Get('Seen', Second);
        Rows.Close();
        Wide.FilterGroup(1);
        Wide.SetRange(F3);
        Wide.FilterGroup(0);
        for K := 1 to 2 do begin
            Evaluate(Before, SelectStr(K, First));
            Evaluate(After, SelectStr(K, Second));
            if K = 2 then
                Seen += ',';
            Seen += Format(After - Before);
        end;
        Assert.AreEqual('0,0', Seen, 'the second counts less the first');
    end;
}
