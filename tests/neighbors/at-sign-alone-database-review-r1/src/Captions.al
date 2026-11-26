// Review round 1: exact captions and declaration facts of AtSignAloneTypes.al:739, :761 and :905.
enum 97603 "AS Review Exact Enum"
{
    Extensible = false;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}

enum 97604 "AS Review Extensible Enum"
{
    Extensible = true;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}

enumextension 97604 "AS Review Enum Extension" extends "AS Review Extensible Enum"
{
    value(50000; Extra) { Caption = 'Extra'; }
}

enum 97605 "AS Review Out Of Order Enum"
{
    Extensible = false;
    value(1; Alpha) { Caption = 'First'; }
    value(0; None) { }
    value(2; Beta) { Caption = 'Second'; }
}

enum 97606 "AS Review Blank Enum"
{
    Extensible = false;
    value(0; " ") { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}

enum 97607 "AS Review Extensible No Extension"
{
    Extensible = true;
    value(0; None) { }
    value(1; Alpha) { Caption = 'First'; }
    value(2; Beta) { Caption = 'Second'; }
}

enum 97608 "AS Review CaptionML Enum"
{
    Extensible = false;
    value(0; None) { }
    value(1; Alpha) { CaptionML = ENU = 'First'; }
    value(2; Beta) { CaptionML = ENU = 'Second'; }
}

table 97603 "AS Review Caption Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Plain; Option) { OptionMembers = A,B; }
        field(3; Captioned; Option) { OptionMembers = A,B; OptionCaption = 'Alpha,Beta'; }
        field(4; Other; Option) { OptionMembers = A,C; }
        field(5; Other Caption; Option) { OptionMembers = A,B; OptionCaption = 'Alpha,Gamma'; }
        field(6; Blank Caption; Option) { OptionMembers = A,B; OptionCaption = 'Al pha,Beta'; }
        field(7; Ml Caption; Option) { OptionMembers = A,B; OptionCaptionML = ENU = 'Alpha,Beta'; }
        field(8; Exact Enum; Enum "AS Review Exact Enum") { }
        field(9; Extensible Enum; Enum "AS Review Extensible Enum") { }
        field(10; Out Of Order Enum; Enum "AS Review Out Of Order Enum") { }
        field(11; Blank Enum; Enum "AS Review Blank Enum") { }
        field(12; Empty Caption; Option) { OptionMembers = A,B; OptionCaption = ',Beta'; }
        field(13; Open Enum; Enum "AS Review Extensible No Extension") { }
        field(14; Dependency Enum; Enum "AS Review Dependency Enum") { }
        field(15; CaptionML Enum; Enum "AS Review CaptionML Enum") { }
    }
    keys { key(PK; "No.") { } }
}

table 97604 "AS Review Caption Host"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Plain Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), Plain = filter('@'))); }
        field(3; "Caption Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), Captioned = filter('@'))); }
        field(4; "Other Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), Other = filter('@'))); }
        field(5; "Other Caption Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Other Caption" = filter('@'))); }
        field(6; "Blank Caption Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Blank Caption" = filter('@'))); }
        field(7; "Ml Caption Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Ml Caption" = filter('@'))); }
        field(8; "Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Exact Enum" = filter('@'))); }
        field(9; "Extended Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Extensible Enum" = filter('@'))); }
        field(10; "Order Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Out Of Order Enum" = filter('@'))); }
        field(11; "Blank Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Blank Enum" = filter('@'))); }
        field(12; "Empty Caption Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Empty Caption" = filter('@'))); }
        field(13; "Open Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Open Enum" = filter('@'))); }
        field(14; "Dependency Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "Dependency Enum" = filter('@'))); }
        field(15; "CaptionML Enum Count At"; Integer) { FieldClass = FlowField; CalcFormula = count("AS Review Caption Row" where("No." = field("No."), "CaptionML Enum" = filter('@'))); }
    }
    keys { key(PK; "No.") { } }
}

codeunit 97602 "AS Review Caption Tests"
{
    Subtype = Test;
    TestIsolation = Function;

    local procedure NewHost(var Host: Record "AS Review Caption Host")
    begin
        Host."No." := 'C';
        Host.Insert();
    end;

    [Test] procedure Captions_PlainAB() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Plain Count At"); end;
    [Test] procedure Captions_OptionAlphaBeta() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Caption Count At"); end;
    [Test] procedure Captions_EnumDeclarationOrder() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Enum Count At"); end;
    [Test] procedure Captions_OtherMembers() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Other Count At"); end;
    [Test] procedure Captions_OtherCaption() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Other Caption Count At"); end;
    [Test] procedure Captions_BlankCaption() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Blank Caption Count At"); end;
    [Test] procedure Captions_EmptyCaption() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Empty Caption Count At"); end;
    [Test] procedure Captions_CaptionML() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Ml Caption Count At"); end;
    [Test] procedure Captions_EnumCaptionML() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("CaptionML Enum Count At"); end;
    [Test] procedure Captions_ExtendedEnum() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Extended Enum Count At"); end;
    [Test] procedure Captions_ExtensibleWithoutExtension() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Open Enum Count At"); end;
    [Test] procedure Captions_OutOfOrderEnum() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Order Enum Count At"); end;
    [Test] procedure Captions_BlankEnum() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Blank Enum Count At"); end;
    [Test] procedure Captions_DependencyEnum() var Host: Record "AS Review Caption Host"; begin NewHost(Host); asserterror Host.CalcFields("Dependency Enum Count At"); end;
    [Test] procedure Captions_TranslatedCaption() var Host: Record "AS Review Caption Host"; begin NewHost(Host); GlobalLanguage(1031); asserterror Host.CalcFields("Caption Count At"); end;
}
