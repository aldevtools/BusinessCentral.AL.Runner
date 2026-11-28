// Neighbor app of al-runner issue 9423: xmlports whose Namespaces declare prefixes no element uses.
// The cells mirror corpus PR 1663 (tests/al-language/xmlports/shapes-properties/XpUnusedNsPrefix.al); the neighbors flip one fact.

table 50100 "NB XUP Item"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50101 "NB XUP Src"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// Cell 1: one unused prefix, DefaultNamespace + UseDefaultNamespace.
xmlport 50110 "NB XUP One Unused"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 2: the reporter's shape, two unused prefixes.
xmlport 50111 "NB XUP Two Unused"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 3 (served on main): the root uses xsd, xsi unused.
xmlport 50112 "NB XUP Mix Root Used"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            NamespacePrefix = 'xsd';
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 3b (served on main): the child uses xsi, xsd unused.
xmlport 50113 "NB XUP Mix Child Used"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                NamespacePrefix = 'xsi';
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 4: no DefaultNamespace, no UseDefaultNamespace.
xmlport 50114 "NB XUP No Default"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 4b: DefaultNamespace, UseDefaultNamespace = false.
xmlport 50115 "NB XUP Dn No Udn"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = false;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 4c: UseDefaultNamespace = true without DefaultNamespace.
xmlport 50116 "NB XUP Udn No Dn"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 5: a root text attribute with NamespacePrefix xsi is the only user.
xmlport 50117 "NB XUP Text Attr"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textattribute(Ver)
            {
                NamespacePrefix = 'xsi';
                trigger OnBeforePassVariable()
                begin
                    Ver := 'v1';
                end;
            }
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 5b (served on main): a field attribute with NamespacePrefix xsi, no DefaultNamespace.
xmlport 50118 "NB XUP Field Attr"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Items)
        {
            tableelement(Rw; "NB XUP Item")
            {
                XmlName = 'Item';
                fieldattribute(Id; Rw.Id) { NamespacePrefix = 'xsi'; }
                fieldelement(Name; Rw.Name) { }
            }
        }
    }
}

// Cell 1b: a table element below the root, DefaultNamespace + UseDefaultNamespace.
xmlport 50119 "NB XUP Table"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Items)
        {
            tableelement(Rw; "NB XUP Item")
            {
                XmlName = 'Item';
                fieldattribute(Id; Rw.Id) { }
                fieldelement(Name; Rw.Name) { }
            }
        }
    }
}

// Cell 2b: the reporter's shape in UTF-16.
xmlport 50120 "NB XUP Two Unused Utf16"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF16;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Cell 7: the table shape with Direction = Both.
xmlport 50121 "NB XUP Both"
{
    Direction = Both;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Items)
        {
            tableelement(Rw; "NB XUP Item")
            {
                XmlName = 'Item';
                fieldattribute(Id; Rw.Id) { }
                fieldelement(Name; Rw.Name) { }
            }
        }
    }
}

// Neighbor: the empty prefix beside an unused one (main's result).
xmlport 50130 "NB XUP Empty Prefix"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    Namespaces = "" = 'urn:nb:xup:e', "xsd" = 'http://www.w3.org/2001/XMLSchema';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: Direction = Import.
xmlport 50131 "NB XUP Import Dir"
{
    Direction = Import;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Items)
        {
            tableelement(Rw; "NB XUP Item")
            {
                XmlName = 'Item';
                fieldattribute(Id; Rw.Id) { }
                fieldelement(Name; Rw.Name) { }
            }
        }
    }
}

// Neighbor: Direction = Both with the text leaf shape.
xmlport 50132 "NB XUP Both Leaf"
{
    Direction = Both;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: UseLax.
xmlport 50134 "NB XUP Lax"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    UseLax = true;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: InlineSchema.
xmlport 50135 "NB XUP Inline"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    InlineSchema = true;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: PreserveWhiteSpace.
xmlport 50136 "NB XUP Pws"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    PreserveWhiteSpace = true;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: XmlVersionNo = V11.
xmlport 50137 "NB XUP V11"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    XmlVersionNo = V11;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: three unused prefixes.
xmlport 50138 "NB XUP Three"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance', "q" = 'urn:nb:q';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: Encoding not declared.
xmlport 50139 "NB XUP No Encoding"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: two text leaves below the root.
xmlport 50140 "NB XUP Two Leaves"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
            textelement(Code)
            {
                trigger OnBeforePassVariable()
                begin
                    Code := 'c';
                end;
            }
        }
    }
}

// Neighbor: a root text attribute without NamespacePrefix.
xmlport 50142 "NB XUP Plain Text Attr"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textattribute(Ver)
            {
                trigger OnBeforePassVariable()
                begin
                    Ver := 'v1';
                end;
            }
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: a prefixed root text attribute without DefaultNamespace.
xmlport 50143 "NB XUP Attr No Dn"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textattribute(Ver)
            {
                NamespacePrefix = 'xsi';
                trigger OnBeforePassVariable()
                begin
                    Ver := 'v1';
                end;
            }
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: the table shape without DefaultNamespace.
xmlport 50144 "NB XUP Table No Dn"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Items)
        {
            tableelement(Rw; "NB XUP Item")
            {
                XmlName = 'Item';
                fieldattribute(Id; Rw.Id) { }
                fieldelement(Name; Rw.Name) { }
            }
        }
    }
}

// Neighbor: UTF-16 without DefaultNamespace.
xmlport 50145 "NB XUP Utf16 No Dn"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF16;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: a text container between the root and the leaf.
xmlport 50141 "NB XUP Nested"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Response)
        {
            textelement(Body)
            {
                textelement(Status)
                {
                    trigger OnBeforePassVariable()
                    begin
                        Status := 'ok';
                    end;
                }
            }
        }
    }
}

// Neighbor: Format = VariableText with Namespaces.
xmlport 50133 "NB XUP VarText"
{
    Direction = Export;
    Format = VariableText;
    UseRequestPage = false;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Root)
        {
            tableelement(Rw; "NB XUP Item")
            {
                fieldelement(Id; Rw.Id) { }
                fieldelement(Name; Rw.Name) { }
            }
        }
    }
}

// Neighbor: other prefix names and uris than the probe's xsd and xsi.
xmlport 50146 "NB XUP Other Prefixes"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "a" = 'urn:nb:a', "b" = 'urn:nb:b';

    schema
    {
        textelement(Response)
        {
            textelement(Status)
            {
                trigger OnBeforePassVariable()
                begin
                    Status := 'ok';
                end;
            }
        }
    }
}

// Neighbor: the table layout with MinOccurs = Zero on the table element.
xmlport 50147 "NB XUP Table MinOccurs"
{
    Direction = Export;
    Format = Xml;
    UseRequestPage = false;
    Encoding = UTF8;
    DefaultNamespace = 'urn:nb:xup:order';
    UseDefaultNamespace = true;
    Namespaces = "xsd" = 'http://www.w3.org/2001/XMLSchema', "xsi" = 'http://www.w3.org/2001/XMLSchema-instance';

    schema
    {
        textelement(Items)
        {
            tableelement(Rw; "NB XUP Item")
            {
                XmlName = 'Item';
                MinOccurs = Zero;
                fieldattribute(Id; Rw.Id) { }
                fieldelement(Name; Rw.Name) { }
            }
        }
    }
}
