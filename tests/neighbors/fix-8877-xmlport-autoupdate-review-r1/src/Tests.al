// One codeunit per test: a test refused while it ran makes the later tests of its codeunit untrusted.

codeunit 50260 "NR T01"
{
    Subtype = Test;

    // The suite exists: OnBeforeInsertRecord skips it. BC keeps the row (Suite_SameAppSameHash_Skips).
    [Test]
    procedure Again_FirstSaveSkipped_UncarriedSame()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50210, '<Root><Suite Code="A" Description="da"><Line Cu="5"/></Suite></Root>');
        L.Run(50210, '<Root><Suite Code="A" Description="db"><Line Cu="6"/></Suite></Root>');
        L.AreEqual('A/da/app1', L.SuiteRow('A'));
    end;
}

codeunit 50261 "NR T02"
{
    Subtype = Test;

    // The suite exists: the key attribute's trigger skips it.
    [Test]
    procedure Again_FirstSaveSkippedByAttribute()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50211, '<Root><Suite Code="A" Description="da"><Line Cu="5"/></Suite></Root>');
        L.Run(50211, '<Root><Suite Code="A" Description="db"><Line Cu="6"/></Suite></Root>');
        L.AreEqual('A/da/app1', L.SuiteRow('A'));
    end;
}

codeunit 50262 "NR T03"
{
    Subtype = Test;

    // OnPostXmlPort reads the record of the linked const element.
    [Test]
    procedure ViewConst_OtherTriggerReadsRecord()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50212, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50263 "NR T04"
{
    Subtype = Test;

    // LinkTable without LinkFields.
    [Test]
    procedure ViewConst_NoLinkFields()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50213, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('/0/e0;', L.EvalRows());
    end;
}

codeunit 50264 "NR T05"
{
    Subtype = Test;

    // filter(0) instead of const(0).
    [Test]
    procedure ViewConst_FilterEq()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50214, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50265 "NR T06"
{
    Subtype = Test;

    // Two rows of the element under one suite.
    [Test]
    procedure ViewConst_TwoRows()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50215, '<Root><Suite Code="A"><Evaluator Name="e0"/><Evaluator Name="e1"/></Suite></Root>');
        L.AreEqual('A/0/e0;A/0/e1;', L.EvalRows());
    end;
}

codeunit 50266 "NR T07"
{
    Subtype = Test;

    // AutoUpdate on the suite element.
    [Test]
    procedure ViewConst_ParentAutoUpdate()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50216, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50267 "NR T08"
{
    Subtype = Test;

    // A second pair in LinkFields.
    [Test]
    procedure ViewConst_TwoLinkFields()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50217, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50268 "NR T09"
{
    Subtype = Test;

    // Stored again after its child rows; the assign trigger is on a leaf text element.
    [Test]
    procedure Again_TextElementTrigger()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50218, '<Root><P><Id>2</Id><Tag>tg</Tag><C><CId>10</CId></C></P></Root>');
        L.AreEqual('2//tg/', L.ParRow(2));
    end;
}

codeunit 50269 "NR T10"
{
    Subtype = Test;

    // AutoUpdate, OnAfterInitRecord assigns an uncarried field, fields as attributes.
    [Test]
    procedure UpdInit_Attributes()
    var
        L: Codeunit "NR Lib";
    begin
        L.SeedPar();
        L.Run(50219, '<Root><P Id="1" Name="new"/></Root>');
        L.AreEqual('1/new/s0/e0', L.ParRow(1));
    end;
}

codeunit 50270 "NR T11"
{
    Subtype = Test;

    // AutoUpdate with a text element that holds an attribute.
    [Test]
    procedure UpdText_WithAttribute()
    var
        L: Codeunit "NR Lib";
    begin
        L.SeedPar();
        L.Run(50220, '<Root><P><Id>1</Id><Info Kind="k">nm</Info></P></Root>');
        L.AreEqual('1/nm/s0/e0', L.ParRow(1));
    end;
}

codeunit 50271 "NR T12"
{
    Subtype = Test;

    // AutoUpdate, the key element's assign trigger, a plain field attribute beside it.
    [Test]
    procedure UpdKey_PlainAttribute()
    var
        L: Codeunit "NR Lib";
    begin
        L.SeedPar();
        L.Run(50221, '<Root><P Name="new"><Id>1</Id></P></Root>');
        L.AreEqual('1/new/s0/e0', L.ParRow(1));
    end;
}

codeunit 50272 "NR T13"
{
    Subtype = Test;

    // AutoUpdate, init values equal the stored row, OnBeforeModifyRecord changes a carried field (main's path).
    [Test]
    procedure UpdInit_BeforeModifyChangesCarried()
    var
        L: Codeunit "NR Lib";
    begin
        L.SeedPar();
        L.Run(50222, '<Root><P><Id>1</Id><Name>new</Name></P></Root>');
        L.AreEqual('1/bm/s0/e0', L.ParRow(1));
    end;
}

codeunit 50273 "NR T14"
{
    Subtype = Test;

    // AutoUpdate, init value differs from the stored row; OnBeforeModifyRecord writes it back and changes a carried field.
    [Test]
    procedure UpdInit_BeforeModifyRestoresUncarried()
    var
        L: Codeunit "NR Lib";
    begin
        L.SeedPar();
        L.Run(50223, '<Root><P><Id>1</Id><Name>new</Name></P></Root>');
        L.AreEqual('1/bm/s0/e0', L.ParRow(1));
    end;
}

codeunit 50274 "NR T15"
{
    Subtype = Test;

    // The const filter is on a Code field.
    [Test]
    procedure ViewConst_CodeField()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50224, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50275 "NR T16"
{
    Subtype = Test;

    // OnPreXmlPort runs ClearAll.
    [Test]
    procedure ViewConst_ClearAll()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50225, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50276 "NR T17"
{
    Subtype = Test;

    // AutoReplace on the suite element.
    [Test]
    procedure ViewConst_ParentAutoReplace()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50226, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50277 "NR T18"
{
    Subtype = Test;

    // UseTemporary on the suite element: the suite row stays in the temporary table.
    [Test]
    procedure ViewConst_ParentTemporary()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50227, '<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>');
        L.AreEqual('A/0/e0;', L.EvalRows());
    end;
}

codeunit 50278 "NR T19"
{
    Subtype = Test;

    // Stored again after its child rows; a text element holding a field attribute beside the triggered key.
    [Test]
    procedure Again_TextContainerChild()
    var
        L: Codeunit "NR Lib";
    begin
        L.Wipe();
        L.Run(50228, '<Root><P><Id>3</Id><Info Name="nm"/><C><CId>11</CId></C></P></Root>');
        L.AreEqual('3/nm/st/', L.ParRow(3));
    end;
}
