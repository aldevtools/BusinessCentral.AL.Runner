#!/usr/bin/env python3
"""Measure, with the real AL compiler, the runtime each language feature needs: properties (per object or element
kind, and per enum-like value), data types, method attributes and object types.

Writes `crates/al-builtins/catalog/alc-features.txt` (al-runner issue 5573, the follow-up of
`tools/alc-runtimes/alc_runtimes.py`, which does the same for builtin methods). Nothing is decompiled: every number
and text comes from a diagnostic `alc` prints.

How it works
1. The features: every property of `docs/language-catalog.json` (the AL language server's completion lists of BC 27 to
   29) under the element kinds below (`CONTEXTS`), every data type and attribute there, and every object type.
2. The values to try: the right side of every `Name = Value;` in the AL files of `--harvest` folders (the corpus by
   default), most frequent first, after a few fixed ones (`FIXED`) for properties that name an object of the probe.
3. One self-contained probe file per feature (its own table, page and so on), with the feature on a line of its own.
4. At the newest runtime, per property, the first (form, value) that compiles is kept; a property with an error in
   every form is listed as not expressible, with alc's first diagnostic. Then, per kept property, one more probe per
   identifier value seen for it (`Importance = Optional`): kept when alc reports nothing, or only AL0666 on its line
   (a value removed in an older runtime).
5. The kept probes are compiled at every runtime alc accepts. Per probe: where it compiles, and what alc reports on
   the feature's line elsewhere: `AL0666 '<name>' <text after "The supported runtime versions are: ">` (several
   joined with ` && `), another code on the line, or `context` (only another line of the probe fails).

Usage (each compile takes seconds; run inside the memory slice on the owner's machine):

    systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=6G \\
      python3 tools/alc-runtimes/alc_features.py --alc <the `al` dotnet tool> \\
      --symbols <a System.app of the newest BC> --work ~/.cache/al-runtime/scratch/alc-features \\
      --package 30.0.42.60748-beta

`--alc` is the AL compiler the corpus uses (the `al_tool_version` of `corpus/.github/workflows/ci.yml`; see
alc_runtimes.py). `--symbols` puts the platform symbols next to the probes (a `systempart`, a `FileUpload` parameter).
`--runtimes` takes the runtime list (default: the one in alc_runtimes.py's `--work` state, else asks alc). `--reuse`
writes the table from the last run's state, compiling nothing.
"""
import argparse
import collections
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from string import Template

sys.path.insert(0, str(Path(__file__).resolve().parent))
import alc_runtimes as base  # noqa: E402

REPO = base.REPO
OUT = base.CATALOG / "alc-features.txt"

# ---------------------------------------------------------------- probe forms
#
# A form is the text of one probe file. `$P` is the feature's line, `$x` a prefix unique to the probe (object names),
# `$i0` .. `$i7` object ids unique to the probe. Every form is one element kind: the key in the table, the same one
# al-sema computes from the syntax (`check/features.rs`): the object type, then the section keywords without arguments
# (`layout`, `actions`, `fields` ...), then the keyword of the innermost section with arguments (`field`, `area` ...),
# and for `modify` the kind of the control or field it modifies.

T = """table $i0 "$x T"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Name; Text[50]) { }
        field(3; Amount; Decimal) { }
        field(4; Opt; Option) { OptionMembers = A,B; }
        field(5; Dt; Date) { }
        field(6; Flag; Boolean) { }
        field(7; Cd; Code[20]) { }
    }
    keys { key(PK; "No.") { } }
}
"""

LAST_FIELD = "        field(7; Cd; Code[20]) { }\n"

LP = """page $i1 "$x LP"
{
    PageType = ListPart;
    SourceTable = "$x T";
    layout { area(Content) { repeater(R) { field(No; Rec."No.") { } } } }
}
"""


def page(body, ptype="Card", top=""):
    return T + LP + f'page $i2 "$x P"\n{{\n    PageType = {ptype};\n    SourceTable = "$x T";\n{top}{body}}}\n'


CARD_LAYOUT = """    layout
    {
        area(Content)
        {
            group(G)
            {
                field(No; Rec."No.") { }
                field(Name; Rec.Name) { }
            }
            part(PP; "$x LP") { }
        }
    }
    actions { area(Processing) { action(A) { trigger OnAction() begin end; } } }
"""

LIST_LAYOUT = """    layout { area(Content) { repeater(R) { field(No; Rec."No.") { } field(Name; Rec.Name) { } } } }
    actions { area(Processing) { action(A) { trigger OnAction() begin end; } } }
"""


def card_field(src):
    return page(f"    layout {{ area(Content) {{ field(F; {src})\n{{\n$P\n}} }} }}\n")


R_RENDER = """    rendering
    {
        layout(L) { Type = Word; LayoutFile = 'l.docx'; }
        layout(HF) { Type = Word; LayoutFile = 'hf.docx'; Subtype = HeaderFooter; }
        layout(TH) { Type = Word; LayoutFile = 'th.dotx'; Subtype = Theme; }
    }
"""


def report(top="", dataset="", rest="", processing=True):
    ds = dataset or '        dataitem(D; "$x T") { column(No; "No.") { } }\n'
    po = "    ProcessingOnly = true;\n" if processing else ""
    return T + f'report $i3 "$x R"\n{{\n{po}{top}    dataset\n    {{\n{ds}    }}\n{rest}}}\n'


R_BASE = report()


def query(top="", elements=""):
    el = elements or '        dataitem(D; "$x T") { column(No; "No.") { } }\n'
    return T + f'query $i4 "$x Q"\n{{\n{top}    elements\n    {{\n{el}    }}\n}}\n'


def xmlport(top="", schema=""):
    sc = schema or '        textelement(Root) { tableelement(TE; "$x T") { fieldelement(No; TE."No.") { } } }\n'
    return T + f'xmlport $i5 "$x X"\n{{\n{top}    schema\n    {{\n{sc}    }}\n}}\n'


ENUM = 'enum $i6 "$x E"\n{\n    Extensible = true;\n    value(0; A) { }\n}\n'
PS = 'permissionset $i7 "$x PS"\n{\n    Assignable = true;\n}\n'

CONTEXTS = {
    "table": [T.replace("    fields\n", "$P\n    fields\n", 1)],
    "table/fields/field": [
        T.replace(LAST_FIELD, LAST_FIELD + f"        field(10; Probe; {ty})\n        {{\n$P\n        }}\n", 1)
        for ty in ["Integer", "Text[50]", "Decimal", "Code[20]", "Option", "Date", "Boolean", "DateTime", "Guid", "Blob", "Media", "BigInteger"]
    ]
    + [
        T.replace(
            LAST_FIELD,
            LAST_FIELD + '        field(10; Probe; Decimal) { FieldClass = FlowField; CalcFormula = sum("$x T".Amount);\n$P\n}\n',
            1,
        ),
        T.replace(LAST_FIELD, LAST_FIELD + "        field(10; Probe; Decimal) { FieldClass = FlowField;\n$P\n}\n", 1),
        T.replace(LAST_FIELD, LAST_FIELD + "        field(10; Probe; Date) { FieldClass = FlowFilter;\n$P\n}\n", 1),
    ],
    "table/keys/key": [T.replace('    keys { key(PK; "No.") { } }', '    keys { key(PK; "No.") { } key(K2; Name, Amount)\n{\n$P\n} }')],
    "table/fieldgroups/fieldgroup": [T.replace("\n}\n", '\n    fieldgroups { fieldgroup(DropDown; "No.", Name)\n{\n$P\n} }\n}\n', 1)],
    "tableextension": [T + 'tableextension $i1 "$x TE" extends "$x T"\n{\n$P\n}\n'],
    "tableextension/fields/field": [
        T + f'tableextension $i1 "$x TE" extends "$x T"\n{{\n    fields {{ field($i2; "$x F"; {ty})\n{{\n$P\n}} }}\n}}\n'
        for ty in ["Integer", "Text[50]", "Decimal", "Code[20]", "Date", "Boolean"]
    ],
    "tableextension/fields/modify": [
        T + f'tableextension $i1 "$x TE" extends "$x T"\n{{\n    fields {{ modify({f})\n{{\n$P\n}} }}\n}}\n'
        for f in ["Name", "Amount", '"No."', "Opt", "Dt", "Flag", "Cd"]
    ],
    "page": [
        page(CARD_LAYOUT, top="$P\n"),
        page(LIST_LAYOUT, "List", top="$P\n"),
        page(LIST_LAYOUT, "ListPart", top="$P\n"),
        page(CARD_LAYOUT, "Document", top="$P\n"),
        page(LIST_LAYOUT, "Worksheet", top="$P\n"),
        page(CARD_LAYOUT, "CardPart", top="$P\n"),
        page(
            LIST_LAYOUT.split("    actions")[0],
            "API",
            top="    APIPublisher = 'alr';\n    APIGroup = 'probe';\n    APIVersion = 'v1.0';\n    EntityName = 'probe';\n"
            "    EntitySetName = 'probes';\n    DelayedInsert = true;\n    ODataKeyFields = SystemId;\n$P\n",
        ),
        page(CARD_LAYOUT, "NavigatePage", top="$P\n"),
        page(CARD_LAYOUT, "StandardDialog", top="$P\n"),
        page(
            "    layout { area(Prompt) { field(Name; Rec.Name) { } } area(Content) { field(No; Rec.\"No.\") { } } }\n"
            "    actions { area(SystemActions) { systemaction(Generate) { trigger OnAction() begin end; } } }\n",
            "PromptDialog",
            top="    Extensible = false;\n$P\n",
        ),
        page("    layout { area(RoleCenter) { part(PP; \"$x LP\") { } } }\n", "RoleCenter", top="$P\n"),
    ],
    "page/layout/area": [
        page("    layout\n    {\n        area(Content)\n        {\n$P\n            field(No; Rec.\"No.\") { }\n        }\n    }\n"),
        page("    layout\n    {\n        area(FactBoxes)\n        {\n$P\n            part(PP; \"$x LP\") { }\n        }\n    }\n"),
    ],
    "page/layout/field": [card_field(s) for s in ['Rec."No."', "Rec.Name", "Rec.Amount", "Rec.Opt", "Rec.Dt", "Rec.Flag", "Rec.Cd"]]
    + [page('    layout { area(Content) { repeater(R) { field(F; Rec."No.")\n{\n$P\n} } } }\n', "List")],
    "page/layout/group": [page('    layout { area(Content) { group(G)\n{\n$P\n            field(No; Rec."No.") { } } } }\n')],
    "page/layout/repeater": [page('    layout { area(Content) { repeater(R)\n{\n$P\n            field(No; Rec."No.") { } } } }\n', "List")],
    "page/layout/cuegroup": [page("    layout { area(Content) { cuegroup(CG)\n{\n$P\n            field(Am; Rec.Amount) { } } } }\n", "CardPart")],
    "page/layout/fixed": [page('    layout { area(Content) { fixed(FX)\n{\n$P\n            group(FG) { field(No; Rec."No.") { } } } } }\n')],
    "page/layout/grid": [page('    layout { area(Content) { grid(GR)\n{\n$P\n            group(GG) { field(No; Rec."No.") { } } } } }\n')],
    "page/layout/part": [page('    layout { area(Content) { part(PP; "$x LP")\n{\n$P\n} } }\n')],
    "page/layout/systempart": [page("    layout { area(FactBoxes) { systempart(SP; Notes)\n{\n$P\n} } }\n")],
    "page/layout/usercontrol": [
        'controladdin "$x CA"\n{\n    Scripts = \'x.js\';\n}\n'
        + page('    layout { area(Content) { usercontrol(UC; "$x CA")\n{\n$P\n} } }\n')
    ],
    "page/layout/label": [
        page("    layout { area(Content) { group(G) { label(LB)\n{\n$P\n} } } }\n"),
        page("    layout { area(Content) { label(LB)\n{\n$P\n} } }\n"),
    ],
    "page/actions/area": [
        page(f"    actions\n    {{\n        area({a})\n        {{\n$P\n            action(A) {{ trigger OnAction() begin end; }}\n        }}\n    }}\n")
        for a in ["Processing", "Navigation", "Creation", "Reporting"]
    ],
    "page/actions/action": [
        page("    actions { area(Processing) { action(A)\n{\n$P\n            trigger OnAction() begin end; } } }\n"),
        page("    actions { area(Processing) { action(A)\n{\n$P\n} } }\n"),
        page("    actions { area(Navigation) { action(A)\n{\n            RunObject = page \"$x LP\";\n$P\n} } }\n"),
    ],
    "page/actions/group": [
        page("    actions { area(Processing) { group(AG)\n{\n$P\n            action(A) { trigger OnAction() begin end; } } } }\n")
    ],
    "page/actions/separator": [
        page("    actions { area(Processing) { action(A) { trigger OnAction() begin end; } separator(S)\n{\n$P\n} } }\n")
    ],
    "page/actions/actionref": [
        page(
            "    actions { area(Processing) { action(A) { trigger OnAction() begin end; } } area(Promoted) { actionref(AR; A)\n{\n$P\n} } }\n"
        )
    ],
    "page/actions/customaction": [
        page("    actions { area(Processing) { customaction(CA)\n{\n$P\n} } }\n"),
        page("    actions { area(Processing) { customaction(CA)\n{\n            CustomActionType = Flow;\n$P\n} } }\n"),
    ],
    "page/actions/fileuploadaction": [
        page(
            "    actions { area(Processing) { fileuploadaction(FU)\n{\n$P\n            trigger OnAction(Files: List of [FileUpload]) begin end; } } }\n"
        )
    ],
    "page/actions/systemaction": [
        page(
            "    layout { area(Prompt) { field(Name; Rec.Name) { } } }\n"
            "    actions { area(SystemActions) { systemaction(Generate)\n{\n$P\n            trigger OnAction() begin end; } } }\n",
            "PromptDialog",
            top="    Extensible = false;\n",
        )
    ],
    "page/views/view": [page(LIST_LAYOUT + "    views { view(V)\n{\n$P\n} }\n", "List")],
    "pageextension": [page(CARD_LAYOUT) + 'pageextension $i3 "$x PE" extends "$x P"\n{\n$P\n}\n'],
    "pageextension/layout/field": [
        page(CARD_LAYOUT)
        + 'pageextension $i3 "$x PE" extends "$x P"\n{\n    layout { addlast(G) { field(F2; Rec.Amount)\n{\n$P\n} } }\n}\n'
    ],
    "pageextension/actions/action": [
        page(CARD_LAYOUT)
        + 'pageextension $i3 "$x PE" extends "$x P"\n{\n    actions { addlast(Processing) { action(A2)\n{\n$P\n            trigger OnAction() begin end; } } }\n}\n'
    ],
    "pageextension/layout/modify:field": [
        page(CARD_LAYOUT) + 'pageextension $i3 "$x PE" extends "$x P"\n{\n    layout { modify(No)\n{\n$P\n} }\n}\n'
    ],
    "pageextension/layout/modify:part": [
        page(CARD_LAYOUT) + 'pageextension $i3 "$x PE" extends "$x P"\n{\n    layout { modify(PP)\n{\n$P\n} }\n}\n'
    ],
    "pageextension/layout/modify:group": [
        page(CARD_LAYOUT) + 'pageextension $i3 "$x PE" extends "$x P"\n{\n    layout { modify(G)\n{\n$P\n} }\n}\n'
    ],
    "pageextension/actions/modify:action": [
        page(CARD_LAYOUT) + 'pageextension $i3 "$x PE" extends "$x P"\n{\n    actions { modify(A)\n{\n$P\n} }\n}\n'
    ],
    "pagecustomization": [page(CARD_LAYOUT) + 'pagecustomization "$x PC" customizes "$x P"\n{\n$P\n}\n'],
    "pagecustomization/layout/modify:field": [
        page(CARD_LAYOUT) + 'pagecustomization "$x PC" customizes "$x P"\n{\n    layout { modify(No)\n{\n$P\n} }\n}\n'
    ],
    "pagecustomization/actions/modify:action": [
        page(CARD_LAYOUT) + 'pagecustomization "$x PC" customizes "$x P"\n{\n    actions { modify(A)\n{\n$P\n} }\n}\n'
    ],
    "report": [
        report(top="$P\n"),
        report(top="    DefaultRenderingLayout = L;\n$P\n", rest=R_RENDER, processing=False),
        report(top="$P\n", rest=R_RENDER, processing=False),
    ],
    "report/requestpage": [report(rest="    requestpage\n    {\n$P\n        layout { area(Content) { } }\n    }\n")],
    "report/dataset/dataitem": [report(dataset='        dataitem(D; "$x T")\n{\n$P\n            column(No; "No.") { } }\n')],
    "report/dataset/column": [
        report(dataset=f'        dataitem(D; "$x T") {{ column(C; {c})\n{{\n$P\n}} }}\n') for c in ['"No."', "Name", "Amount", "Dt", "Flag"]
    ],
    "report/rendering/layout": [
        report(
            top="    DefaultRenderingLayout = L;\n",
            rest=f"    rendering {{ layout(L)\n{{\n{ty}            LayoutFile = 'l.{ext}';\n$P\n}} }}\n",
            processing=False,
        )
        for ty, ext in [("            Type = Word;\n", "docx"), ("            Type = RDLC;\n", "rdl"), ("            Type = Excel;\n", "xlsx"), ("", "docx")]
    ]
    + [
        # A layout that names the header and footer part and the theme part (HeaderFooterPart = HF).
        report(
            top="    DefaultRenderingLayout = L;\n",
            rest=R_RENDER.replace("layout(L) { Type = Word; LayoutFile = 'l.docx'; }", "layout(L)\n{\n            Type = Word;\n            LayoutFile = 'l.docx';\n$P\n}"),
            processing=False,
        )
    ],
    "reportextension": [R_BASE + 'reportextension $i4 "$x RE" extends "$x R"\n{\n$P\n}\n'],
    "reportextension/dataset/modify": [
        R_BASE + 'reportextension $i4 "$x RE" extends "$x R"\n{\n    dataset { modify(D)\n{\n$P\n} }\n}\n'
    ],
    "reportextension/dataset/column": [
        R_BASE + 'reportextension $i4 "$x RE" extends "$x R"\n{\n    dataset { add(D) { column(C2; Name)\n{\n$P\n} } }\n}\n'
    ],
    "query": [
        query(top="$P\n"),
        query(top="    QueryType = API;\n    APIPublisher = 'alr';\n    APIGroup = 'probe';\n    EntityName = 'probe';\n    EntitySetName = 'probes';\n$P\n"),
    ],
    "query/elements/dataitem": [query(elements='        dataitem(D; "$x T")\n{\n$P\n            column(No; "No.") { } }\n')],
    "query/elements/column": [
        query(elements=f'        dataitem(D; "$x T") {{ column(C; {c})\n{{\n$P\n}} }}\n') for c in ['"No."', "Amount", "Name", "Dt"]
    ],
    "query/elements/filter": [query(elements='        dataitem(D; "$x T") { column(No; "No.") { } filter(Fl; Name)\n{\n$P\n} }\n')],
    "xmlport": [xmlport(top="$P\n")],
    "xmlport/schema/tableelement": [
        xmlport(schema='        textelement(Root) { tableelement(TE; "$x T")\n{\n$P\n            fieldelement(No; TE."No.") { } } }\n')
    ],
    "xmlport/schema/fieldelement": [
        xmlport(schema=f'        textelement(Root) {{ tableelement(TE; "$x T") {{ fieldelement(FE; TE.{c})\n{{\n$P\n}} }} }}\n')
        for c in ['"No."', "Name", "Amount", "Dt"]
    ],
    "xmlport/schema/fieldattribute": [
        xmlport(schema='        textelement(Root) { tableelement(TE; "$x T") { fieldattribute(FA; TE.Name)\n{\n$P\n} } }\n')
    ],
    "xmlport/schema/textelement": [
        xmlport(schema='        textelement(Root) { tableelement(TE; "$x T") { textelement(TX)\n{\n$P\n} } }\n'),
        xmlport(schema="        textelement(Root)\n{\n$P\n}\n"),
    ],
    "xmlport/schema/textattribute": [
        xmlport(schema='        textelement(Root) { tableelement(TE; "$x T") { textattribute(TA)\n{\n$P\n} } }\n')
    ],
    "codeunit": [
        'codeunit $i0 "$x C"\n{\n$P\n}\n',
        'codeunit $i0 "$x C"\n{\n    Subtype = Test;\n$P\n}\n',
        T + 'codeunit $i1 "$x C"\n{\n    TableNo = "$x T";\n$P\n}\n',
    ],
    "enum": ['enum $i0 "$x E"\n{\n$P\n    value(0; A) { }\n}\n'],
    "enum/value": ['enum $i0 "$x E"\n{\n    value(0; A)\n{\n$P\n}\n}\n'],
    "enumextension": [ENUM + 'enumextension $i1 "$x EE" extends "$x E"\n{\n$P\n    value($i2; Z) { }\n}\n'],
    "enumextension/value": [ENUM + 'enumextension $i1 "$x EE" extends "$x E"\n{\n    value($i2; Z)\n{\n$P\n}\n}\n'],
    "interface": ['interface "$x I"\n{\n$P\n    procedure M();\n}\n'],
    "controladdin": ['controladdin "$x CA"\n{\n$P\n}\n', 'controladdin "$x CA"\n{\n    Scripts = \'x.js\';\n$P\n}\n'],
    "permissionset": [
        T + 'permissionset $i1 "$x PS"\n{\n$P\n}\n',
        T + 'permissionset $i1 "$x PS"\n{\n    Permissions = tabledata "$x T" = R;\n$P\n}\n',
    ],
    "permissionsetextension": [PS + 'permissionsetextension $i1 "$x PSE" extends "$x PS"\n{\n$P\n}\n'],
    "entitlement": [
        'entitlement "$x EN"\n{\n$P\n}\n',
        'entitlement "$x EN"\n{\n    Type = Role;\n    RoleType = Local;\n    Id = \'00000000-0000-0000-0000-000000000001\';\n$P\n}\n',
    ],
    "profile": [page("    layout { area(RoleCenter) { } }\n", "RoleCenter") + 'profile "$x PR"\n{\n$P\n}\n',
                page("    layout { area(RoleCenter) { } }\n", "RoleCenter") + 'profile "$x PR"\n{\n    RoleCenter = "$x P";\n$P\n}\n'],
}

# The language catalog's owner of a property, and the element kinds above it is tried at.
OWNERS = {
    "Table": ["table"],
    "Field": ["table/fields/field", "tableextension/fields/field"],
    "Key": ["table/keys/key"],
    "FieldGroup": ["table/fieldgroups/fieldgroup"],
    "TableExtension": ["tableextension"],
    "FieldModify": ["tableextension/fields/modify"],
    "Page": ["page"],
    "PageArea": ["page/layout/area"],
    "PageActionArea": ["page/actions/area"],
    "PageField": ["page/layout/field", "pageextension/layout/field"],
    "PageGroup": ["page/layout/group"],
    "PageRepeater": ["page/layout/repeater"],
    "PageCueGroup": ["page/layout/cuegroup"],
    "PageFixed": ["page/layout/fixed"],
    "PageGrid": ["page/layout/grid"],
    "PagePart": ["page/layout/part"],
    "PageSystemPart": ["page/layout/systempart"],
    "PageUserControl": ["page/layout/usercontrol"],
    "PageLabel": ["page/layout/label"],
    "PageAction": ["page/actions/action", "pageextension/actions/action"],
    "PageActionGroup": ["page/actions/group"],
    "PageActionSeparator": ["page/actions/separator"],
    "PageActionRef": ["page/actions/actionref"],
    "PageCustomAction": ["page/actions/customaction"],
    "PageFileUploadAction": ["page/actions/fileuploadaction"],
    "PageSystemAction": ["page/actions/systemaction"],
    "PageView": ["page/views/view"],
    "PageExtension": ["pageextension"],
    "PageControlModify": [
        "pageextension/layout/modify:field",
        "pageextension/layout/modify:part",
        "pageextension/layout/modify:group",
    ],
    "PageActionModify": ["pageextension/actions/modify:action"],
    "PageCustomizationObject": ["pagecustomization"],
    "AllPageCustomization": ["pagecustomization/layout/modify:field", "pagecustomization/actions/modify:action"],
    "Report": ["report"],
    "RequestPage": ["report/requestpage"],
    "ReportDataItem": ["report/dataset/dataitem"],
    "ReportColumn": ["report/dataset/column", "reportextension/dataset/column"],
    "ReportLayout": ["report/rendering/layout"],
    "ReportExtension": ["reportextension"],
    "ReportExtensionModify": ["reportextension/dataset/modify"],
    "Query": ["query"],
    "QueryDataItem": ["query/elements/dataitem"],
    "QueryColumn": ["query/elements/column"],
    "QueryFilter": ["query/elements/filter"],
    "XmlPort": ["xmlport"],
    "XmlPortTableElement": ["xmlport/schema/tableelement"],
    "XmlPortFieldElement": ["xmlport/schema/fieldelement"],
    "XmlPortFieldAttribute": ["xmlport/schema/fieldattribute"],
    "XmlPortTextElement": ["xmlport/schema/textelement"],
    "XmlPortTextAttribute": ["xmlport/schema/textattribute"],
    "Codeunit": ["codeunit"],
    "EnumType": ["enum"],
    "EnumValue": ["enum/value", "enumextension/value"],
    "Interface": ["interface"],
    "ControlAddInObject": ["controladdin"],
    "PermissionSet": ["permissionset"],
    "PermissionSetExtension": ["permissionsetextension"],
    "Entitlement": ["entitlement"],
    "Profile": ["profile"],
}

# Values that name an object or element of the probe, tried first.
FIXED = {
    "sourcetable": ['"$x T"'],
    "tableno": ['"$x T"'],
    "tablerelation": ['"$x T"'],
    "lookuppageid": ['"$x LP"'],
    "drilldownpageid": ['"$x LP"'],
    "cardpageid": ['"$x LP"'],
    "runobject": ['page "$x LP"'],
    "calcformula": ['sum("$x T".Amount)', 'count("$x T")'],
    "defaultrenderinglayout": ["L"],
    "defaultheaderfooterpart": ["HF"],
    "defaultthemepart": ["TH"],
    "headerfooterpart": ["HF"],
    "themepart": ["TH"],
    "subtype": ["HeaderFooter", "Test"],
    "permissions": ['tabledata "$x T" = R'],
    "sourcetableview": ['where("No." = const(1))'],
    "dataitemtableview": ['sorting("No.")'],
    "sourcetabletemporary": ["true"],
    "rolecenter": ['"$x P"'],
    "customizations": ['"$x PC"'],
    "includedpermissionsets": ['"$x PS"'],
    "orderby": ["ascending(No)"],
    "linkfields": ['"No." = field("No.")'],
    "dataitemlink": ['"No." = field("No.")'],
    "subpagelink": ['"No." = field("No.")'],
    "obsoletereason": ["'x'"],
    "obsoletetag": ["'1.0'"],
}
GENERIC = ["true", "'x'", "1", "false", "ENU = 'x'"]
IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
VALUE_PROBES_PER_PROPERTY = 60
CANDIDATES_PER_PROPERTY = 80


def harvest(roots):
    """Property name (lower case) -> values, most frequent first, from `Name = Value;` in AL files."""
    pat = re.compile(r"(?<![.\w:<>=])([A-Za-z]+)\s*=\s*([^;{}=]+?);")
    vals = collections.defaultdict(collections.Counter)
    for root in roots:
        for f in Path(root).rglob("*.al"):
            for line in f.read_text(errors="replace").splitlines():
                for m in pat.finditer(line):
                    v = m[2].strip()
                    if v.count("'") % 2 == 0 and v.count("(") == v.count(")") and v.count('"') % 2 == 0:
                        vals[m[1].lower()][v] += 1
    out = {}
    for p, c in vals.items():
        seen, vs = set(), []
        for v, _ in c.most_common():
            if v.lower() not in seen:
                seen.add(v.lower())
                vs.append(v)
        out[p] = vs
    return out


def catalog_features():
    d = json.loads((REPO / "docs/language-catalog.json").read_text())
    props = collections.defaultdict(set)
    types, attrs = set(), set()
    for i in d["items"]:
        if i["area"] == "properties":
            props[i["owner"]].add(i["name"])
        elif i["area"] == "datatypes":
            types.add(i["name"])
        elif i["area"] == "attributes":
            attrs.add(i["name"])
    return props, sorted(types), sorted(attrs)


# ---------------------------------------------------------------- data types, attributes, objects

TYPE_DECL = {
    "Record": 'Record "$x T"',
    "Codeunit": 'Codeunit "$x C"',
    "Page": 'Page "$x LP"',
    "Report": 'Report "$x R"',
    "Query": 'Query "$x Q"',
    "XmlPort": 'XmlPort "$x X"',
    "TestPage": 'TestPage "$x LP"',
    "Enum": 'Enum "$x E"',
    "Interface": 'Interface "$x I"',
    "List": "List of [Integer]",
    "Dictionary": "Dictionary of [Integer, Text]",
    "Option": "Option A,B",
    "Label": "Label 'L'",
    "TextConst": "TextConst ENU = 'L'",
    "array": "array[2] of Integer",
    "Code": "Code[20]",
    "Text": "Text[30]",
}
TYPE_OBJECTS = (
    T
    + LP
    + 'codeunit $i2 "$x C" { }\n'
    + report().replace(T, "")
    + query().replace(T, "")
    + xmlport().replace(T, "")
    + 'enum $i6 "$x E" { value(0; A) { } }\n'
    + 'interface "$x I" { procedure M(); }\n'
)


def type_forms(name):
    decl = TYPE_DECL.get(name, name)
    var = f"    procedure Probe()\n    var\n        V: {decl};\n    begin\n    end;\n"
    test = name in ("TestPage", "TestPermissions", "TestHttpRequestMessage", "TestHttpResponseMessage", "TestHandlerContext")
    sub = "    Subtype = Test;\n" if test else ""
    # The declaration line is the feature's line.
    var = var.replace(f"        V: {decl};", "$P")
    forms = [TYPE_OBJECTS + f'codeunit $i7 "$x CP"\n{{\n{sub}{var}}}\n']
    if not test:
        forms.append(TYPE_OBJECTS + f'codeunit $i7 "$x CP"\n{{\n    Subtype = Test;\n{var}}}\n')
    return forms, f"        V: {decl};"


def harvest_attributes(roots):
    """Attribute name (lower case) -> [(attribute line, procedure line)], most frequent first."""
    pat = re.compile(r"^\s*\[(\w+)(\(.*\))?\]\s*$")
    proc = re.compile(r"^\s*(local\s+|internal\s+)?procedure\s+\w+\(.*\)(\s*:\s*\w+)?\s*;?\s*$", re.I)
    out = collections.defaultdict(collections.Counter)
    for root in roots:
        for f in Path(root).rglob("*.al"):
            lines = f.read_text(errors="replace").splitlines()
            for k, l in enumerate(lines[:-1]):
                m = pat.match(l)
                if not m:
                    continue
                j = k + 1
                while j < len(lines) and pat.match(lines[j]):
                    j += 1
                if j < len(lines) and proc.match(lines[j]) and "var" not in lines[j].lower().split("(")[0]:
                    p = re.sub(r"procedure\s+\w+", "procedure Probe", lines[j].strip()).rstrip(";")
                    p = re.sub(r"^(local|internal)\s+", "", p)
                    out[m[1].lower()][(l.strip(), p)] += 1
    return {k: [x for x, _ in c.most_common(8)] for k, c in out.items()}


def attribute_forms(name, harvested):
    pairs = [(f"[{name}]", "procedure Probe()")] + harvested.get(name.lower(), [])
    forms = []
    for attr, procline in pairs:
        if re.search(r"\)\s*:\s*\w+$", procline):
            body = f"{procline}\n    begin\n    end;\n"
        else:
            body = f"{procline}\n    begin\n    end;\n"
        for sub in ("", "    Subtype = Test;\n", "    EventSubscriberInstance = Manual;\n"):
            forms.append((TYPE_OBJECTS + f'codeunit $i7 "$x CP"\n{{\n{sub}$P\n    {body}}}\n', attr))
    return forms[:CANDIDATES_PER_PROPERTY]


OBJECT_FORMS = {
    "table": T,
    "page": T + LP,
    "codeunit": 'codeunit $i0 "$x C" { }\n',
    "report": report(),
    "query": query(),
    "xmlport": xmlport(),
    "enum": 'enum $i0 "$x E" { value(0; A) { } }\n',
    "enumextension": ENUM + 'enumextension $i1 "$x EE" extends "$x E" { value($i2; Z) { } }\n',
    "interface": 'interface "$x I" { procedure M(); }\n',
    "tableextension": T + 'tableextension $i1 "$x TE" extends "$x T" { }\n',
    "pageextension": T + LP + 'pageextension $i2 "$x PE" extends "$x LP" { }\n',
    "reportextension": report() + 'reportextension $i4 "$x RE" extends "$x R" { }\n',
    "permissionset": 'permissionset $i0 "$x PS" { }\n',
    "permissionsetextension": PS + 'permissionsetextension $i0 "$x PSE" extends "$x PS" { }\n',
    "entitlement": 'entitlement "$x EN" { }\n',
    "controladdin": 'controladdin "$x CA" { }\n',
    "profile": page("    layout { area(RoleCenter) { } }\n", "RoleCenter") + 'profile "$x PR" { RoleCenter = "$x P"; }\n',
    "pagecustomization": T + LP + 'pagecustomization "$x PC" customizes "$x LP" { }\n',
    "dotnet": "dotnet { }\n",
}


def object_form(kind):
    """The probe of an object type: its declaration's first line is the feature's line."""
    src = OBJECT_FORMS[kind]
    keyword = {"dotnet": "dotnet {"}.get(kind, f"{kind} ")
    k = [i for i, l in enumerate(src.splitlines()) if l.startswith(keyword)][-1]
    lines = src.splitlines()
    line = lines[k]
    lines[k] = "$P"
    return "\n".join(lines) + "\n", line


# ---------------------------------------------------------------- compiling


def probe_names(k):
    return {"x": f"F{k}", **{f"i{j}": str(100000 + k * 8 + j) for j in range(8)}}


def render(form, line, k):
    """The probe file `k`: the form with the feature's line in it, then the probe's names and ids filled in (a value
    can name an object of the probe, `SourceTable = "$x T"`), and the number of the feature's line. `line` can hold
    lines before the feature's (a property it needs, `ObsoleteState = Pending;` before `ObsoleteReason`): its last
    line is the feature's."""
    src = Template(Template(form).safe_substitute(P=line)).safe_substitute(**probe_names(k))
    last = Template(line.splitlines()[-1]).safe_substitute(**probe_names(k))
    at = [i + 1 for i, l in enumerate(src.splitlines()) if l == last]
    return src, at[0] if at else 0


# alc's AL0223 names a property the probe's property needs: `The Property 'ObsoleteReason' can only be used if the
# property 'ObsoleteState' is set to 'Pending'`.
NEEDS = re.compile(r"can only be used if the property '(\w+)' is set to '(\w+)'")


def write_app(folder, runtime, probes, symbols):
    if folder.exists():
        shutil.rmtree(folder)
    (folder / ".alpackages").mkdir(parents=True)
    if symbols:
        shutil.copy(symbols, folder / ".alpackages" / Path(symbols).name)
    major = int(runtime.split(".")[0])
    app = {"id": base.APP_ID, "name": "ALR Feature Probes", "publisher": "AL Runner", "version": "1.0.0.0", "runtime": runtime}
    if symbols:
        # alc loads the platform symbols only for an app that names a platform version.
        app["platform"] = "1.0.0.0"
    if major >= 4:
        app["target"] = "OnPrem"
    if major >= 3:
        app["idRanges"] = [{"from": 50000, "to": 999999}]
    (folder / "app.json").write_text(json.dumps(app, indent=2))
    for name, src in probes.items():
        (folder / name).write_text(src)


def compile_rounds(alc, folder, runtime, probes, log, symbols):
    """The probe files that do not compile at `runtime`, with their diagnostics (code, message, line), as in
    alc_runtimes.compile_rounds: files with errors are taken out and the rest compiled again until none has one."""
    left, failed, rounds = dict(probes), {}, 0
    while left:
        write_app(folder, runtime, left, symbols)
        errs = base.compile_app(alc, folder, log.with_name(f"{log.stem}-{rounds}.log"))
        rounds += 1
        errs = {f: e for f, e in errs.items() if f in left}
        if not errs:
            break
        for f, e in errs.items():
            failed[f] = e
            left.pop(f, None)
    return failed


AL0666 = re.compile(r"^'(?P<name>[^']+)' is not available in runtime version '[^']+'\. The supported runtime versions are: (?P<text>.*?)\.?$")


def outcome(diags, line):
    at = [d for d in diags if d[2] == line]
    if not at:
        return "context"
    texts = []
    for code, msg, _ in at:
        m = AL0666.match(msg) if code == "AL0666" else None
        if m:
            texts.append(f"'{m['name']}' {m['text']}")
    if texts:
        return "AL0666 " + " && ".join(texts)
    return at[0][0]


def only_al0666_on(diags, line, first=None):
    """Whether alc reports only AL0666, on the feature's line or the lines of what it needs before it (`first`)."""
    first = line if first is None else first
    return bool(diags) and all(d[0] == "AL0666" and first <= d[2] <= line for d in diags)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--alc", required=True)
    ap.add_argument("--work", required=True, type=Path)
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--symbols", help="a System.app to compile against")
    ap.add_argument("--harvest", action="append", help="folders of AL files to take property values from (default: corpus/tests)")
    ap.add_argument("--runtimes", help="comma-separated runtime list (default: alc_runtimes.py's list, else ask alc)")
    ap.add_argument("--runtimes-from", type=Path, help="the --work folder of an alc_runtimes.py run, for its runtime list")
    ap.add_argument("--package", help="the NuGet package version of the compiler, for the header")
    ap.add_argument("--reuse", action="store_true")
    args = ap.parse_args()
    work = args.work.expanduser().resolve()
    work.mkdir(parents=True, exist_ok=True)
    alc = os.path.expanduser(args.alc)
    symbols = os.path.expanduser(args.symbols) if args.symbols else None

    state = json.loads((work / "state.json").read_text()) if args.reuse else None
    if state:
        version, runtimes, probes, meta, unexpressible, results = (
            state["version"], state["runtimes"], state["probes"], state["meta"], state["unexpressible"], state["results"])
    else:
        version = base.alc_version(alc)
        if args.runtimes:
            runtimes = args.runtimes.split(",")
        elif args.runtimes_from:
            runtimes = json.loads((args.runtimes_from.expanduser() / "state.json").read_text())["runtimes"]
        else:
            runtimes = base.accepted_runtimes(alc, work)
        runtimes.sort(key=base.rt_key)
        probes, meta, unexpressible, results = measure(alc, work, runtimes, symbols, args.harvest or [str(REPO / "corpus/tests")])
        (work / "state.json").write_text(json.dumps(
            {"version": version, "runtimes": runtimes, "probes": probes, "meta": meta, "unexpressible": unexpressible, "results": results}))
    write_table(args, version, runtimes, probes, meta, unexpressible, results)


def measure(alc, work, runtimes, symbols, roots):
    newest = runtimes[-1]
    print(f"alc: runtimes {runtimes[0]} .. {newest} ({len(runtimes)})", file=sys.stderr)
    props, types, attrs = catalog_features()
    values = harvest(roots)
    harvested_attrs = harvest_attributes(roots)

    # Slots: (feature kind, element kind, name) -> candidates [(form, feature line, value)].
    slots = {}
    for owner, names in props.items():
        for ctx in OWNERS.get(owner, []):
            for name in sorted(names):
                vs = [Template(v) for v in FIXED.get(name.lower(), [])]
                vs = [v.template for v in vs] + values.get(name.lower(), [])[:8] + GENERIC
                seen, cands = set(), []
                for form in CONTEXTS[ctx]:
                    for v in vs:
                        if (form, v) in seen:
                            continue
                        seen.add((form, v))
                        cands.append((form, f"{name} = {v};", v))
                slots[("property", ctx, name)] = cands[:CANDIDATES_PER_PROPERTY]
    for t in types:
        if t in ("DotNet", "ControlAddIn", "Action", "Dialog", "TestRequestPage"):
            continue
        forms, line = type_forms(t)
        slots[("datatype", "-", t)] = [(f, line, "*") for f in forms]
    for a in attrs:
        slots[("attribute", "-", a)] = [(f, attr, "*") for f, attr in attribute_forms(a, harvested_attrs)]
    for kind in OBJECT_FORMS:
        form, line = object_form(kind)
        slots[("object", "-", kind)] = [(form, line, "*")]

    # Newest runtime: the first candidate per slot that compiles.
    choice = {s: 0 for s in slots}
    first_err, unexpressible = {}, {}
    undecided = set(slots)
    rnd = 0
    while undecided:
        batch, names = {}, {}
        for k, s in enumerate(sorted(undecided)):
            form, line, _ = slots[s][choice[s]]
            fname = f"q{len(batch):05}.al"
            batch[fname], at = render(form, line, len(batch))
            names[fname] = (s, at)
        errs = compile_rounds(alc, work / "app", newest, batch, work / f"newest-{rnd}.log", symbols)
        rnd += 1
        nxt = set()
        for fname, (s, at) in names.items():
            # Only AL0666 on the feature's line: the feature was removed in an older runtime (ObsoleteState on a page
            # area: '4.0' until, but not including '13.0'); the probe is right.
            # Not when alc names the value (`'Optional' is not available`): that says nothing about other values.
            value = slots[s][choice[s]][2]
            if fname not in errs or (
                only_al0666_on(errs[fname], at, at - len(slots[s][choice[s]][1].splitlines()) + 1)
                and not any(AL0666.match(d[1]) and AL0666.match(d[1])["name"].lower() == value.lower() for d in errs[fname])
            ):
                continue
            first_err.setdefault(s, errs[fname][0])
            # A property alc says needs another one: try again with that one before it, in the same form.
            form, line, value = slots[s][choice[s]]
            need = next((NEEDS.search(d[1]) for d in errs[fname] if d[0] == "AL0223" and NEEDS.search(d[1])), None)
            if need and "\n" not in line:
                slots[s].insert(choice[s] + 1, (form, f"{need[1]} = {need[2]};\n{line}", value))
            if choice[s] + 1 < len(slots[s]):
                choice[s] += 1
                nxt.add(s)
            else:
                e = first_err[s]
                unexpressible[s] = f"{e[0]}: {e[1]}"
        undecided = nxt
        print(f"newest round {rnd}: {len(undecided)} slots still trying", file=sys.stderr)

    # The probes: one per kept slot, plus one per identifier value of a kept property.
    probes, meta = {}, {}

    def add(s, value, form, line):
        k = len(probes)
        fname = f"p{k:05}.al"
        probes[fname], at = render(form, line, k)
        meta[fname] = [s[0], s[1], s[2], value, at, at - len(line.splitlines()) + 1]
        return fname

    kept = [s for s in slots if s not in unexpressible]
    for s in kept:
        form, line, value = slots[s][choice[s]]
        add(s, "*", form, line)
    extra = {}
    for s in kept:
        if s[0] != "property":
            continue
        form, line, chosen = slots[s][choice[s]]
        before = line.splitlines()[:-1]
        ids = [v for v in values.get(s[2].lower(), []) if IDENT.match(v) and v.lower() not in ("true", "false")]
        for v in ids[:VALUE_PROBES_PER_PROPERTY]:
            if v.lower() == chosen.lower():
                continue
            extra[add(s, v, form, "\n".join(before + [f"{s[2]} = {v};"]))] = s
    # Value probes that do not compile at the newest runtime for another reason than AL0666 on their line are not
    # values of the property there.
    errs = compile_rounds(alc, work / "app", newest, {f: probes[f] for f in extra}, work / "values.log", symbols)
    for f, e in errs.items():
        if not only_al0666_on(e, meta[f][4], meta[f][5]):
            del probes[f]
            del meta[f]
    print(f"{len(probes)} probes ({len(kept)} features, {len(probes) - len(kept)} values), {len(unexpressible)} not expressible", file=sys.stderr)

    results = {}
    for rt in runtimes:
        e = compile_rounds(alc, work / "app", rt, probes, work / f"rt-{rt}.log", symbols)
        results[rt] = {f: d for f, d in e.items()}
        print(f"runtime {rt}: {len(e)} probe files with errors", file=sys.stderr)
    return probes, meta, {"\t".join(k): v for k, v in unexpressible.items()}, results


def write_table(args, version, runtimes, probes, meta, unexpressible, results):
    newest = runtimes[-1]
    rows = []
    for fname, (kind, ctx, name, value, line, _) in meta.items():
        ok = [fname not in results[rt] for rt in runtimes]
        if not any(ok) and not outcome(results[newest][fname], line).startswith("AL0666"):
            if value != "*":
                # A value no runtime accepts in this form says nothing.
                continue
            e = results[newest][fname][0]
            rows.append((kind, ctx, name, value, f"{kind}\t{ctx}\t{name}\t{value}\t-\t{e[0]}: {e[1]}"))
            continue
        ranges, k = [], 0
        while k < len(runtimes):
            if ok[k]:
                j = k
                while j < len(runtimes) and ok[j]:
                    j += 1
                ranges.append(f"{runtimes[k]}..{runtimes[j] if j < len(runtimes) else ''}")
                k = j
            else:
                k += 1
        runs, k = [], 0
        while k < len(runtimes):
            if ok[k]:
                k += 1
                continue
            o, j = outcome(results[runtimes[k]][fname], line), k
            while j < len(runtimes) and not ok[j] and outcome(results[runtimes[j]][fname], line) == o:
                j += 1
            runs.append(f"{runtimes[k]}..{runtimes[j] if j < len(runtimes) else ''} {o}")
            k = j
        if value != "*" and not runs:
            continue
        rows.append((kind, ctx, name, value, f"{kind}\t{ctx}\t{name}\t{value}\t{','.join(ranges) or 'none'}\t{'; '.join(runs)}"))
    # A value row that says the same as the row of any value of its property is left out.
    star = {(r[0], r[1], r[2]): r[4].split("\t", 4)[4] for r in rows if r[3] == "*"}
    rows = [r for r in rows if r[3] == "*" or star.get((r[0], r[1], r[2])) != r[4].split("\t", 4)[4]]
    for key, why in unexpressible.items():
        kind, ctx, name = key.split("\t")
        rows.append((kind, ctx, name, "*", f"{kind}\t{ctx}\t{name}\t*\t-\t{why}"))
    rows.sort(key=lambda r: (r[0], r[1], r[2].lower(), r[3] != "*", r[3].lower()))
    header = [
        f"# GENERATED by tools/alc-runtimes/alc_features.py with AL compiler {version}. Do not edit; rerun it.",
        *([f"# Compiler package: NuGet Microsoft.Dynamics.BusinessCentral.Development.Tools {args.package}."] if args.package else []),
        f"# Runtimes compiled: {runtimes[0]} to {newest} ({len(runtimes)} values this compiler accepts).",
        "# <feature kind>\t<element kind>\t<name>\t<value>\t<runtimes>\t<what alc reports at the other runtimes>",
        "#   feature kind: property, datatype, attribute or object. element kind: where a property is written (the object",
        "#   type, the section keywords without arguments, the innermost section keyword; `modify:<kind>` names what a",
        "#   modify changes); `-` for the other feature kinds. value: `*` for any value of the property, else one value",
        "#   (listed only where it differs from `*`).",
        "#   runtimes: where the probe compiles, as ranges `from..to` (to: the first runtime it fails at again; empty: up",
        "#   to the newest; `none`: no runtime, alc reports AL0666 on it at the newest). other runtimes: runs `from..to <outcome>` separated by `; `: `AL0666 '<name>' <alc's text",
        "#   after 'The supported runtime versions are: '>` (several joined by ` && `) for an AL0666 on the feature's",
        "#   line, another code on that line, or `context` (only another line of the probe fails there).",
        "# <feature kind>\t<element kind>\t<name>\t*\t-\t<why no probe compiles at the newest runtime>",
    ]
    args.out.write_text("\n".join(header + [r[4] for r in rows]) + "\n")
    print(f"wrote {args.out}: {len(rows)} rows", file=sys.stderr)


if __name__ == "__main__":
    main()
