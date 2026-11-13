//! A stand-in of the parts of Base Application a rendering save's layout selection runs, for the fixtures of this folder.
//! The runtime raises SelectReportLayoutCode and SelectedBuiltinLayoutType to Base Application's ReportManagement and
//! reads no "Report Layout Selection" row itself (`report_save::fact_selection`), so a fixture that holds the table holds
//! these too: table 9651 "Report Layout Selection" with its `SelectedBuiltinLayoutType` procedure, table 9650 "Custom Report
//! Layout" (its `Type` an option of the two kinds the subscriber maps), codeunit 9654 "Design-time Report Selection" and the
//! two subscribers of codeunit 44 "ReportManagement", with the bodies of Base Application 29 (the FeatureKey branch and the
//! OnSelectReportLayoutCode event left out). They sit in an app with Base Application's id, the only ReportManagement the
//! runtime raises the events to.

use al_testkit::{AppSource, Built};

const BASE_APPLICATION_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

const BASE: &str = r#"
table 9651 "Report Layout Selection"
{
    fields
    {
        field(1; "Report ID"; Integer) { }
        field(3; "Company Name"; Text[30]) { }
        field(4; Type; Option) { OptionMembers = "RDLC (built-in)","Word (built-in)","Custom Layout","Excel Layout","External Layout"; }
        field(6; "Custom Report Layout Code"; Code[20]) { }
    }
    keys { key(PK; "Report ID", "Company Name") { } }

    procedure SelectedBuiltinLayoutType(ReportID: Integer): Integer
    begin
        if not Get(ReportID, CompanyName) then
            exit(0);
        case Type of
            Type::"RDLC (built-in)":
                exit(1);
            Type::"Word (built-in)":
                exit(2);
            Type::"Excel Layout":
                exit(3);
            Type::"External Layout":
                exit(4);
            else
                exit(0);
        end;
    end;
}

table 9650 "Custom Report Layout"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Report ID"; Integer) { }
        field(6; Type; Option) { OptionMembers = RDLC,Word; }
    }
    keys { key(PK; "Code") { } }
}

codeunit 9654 "Design-time Report Selection"
{
    SingleInstance = true;
    var SelectedCustomLayoutCode: Code[20]; SelectedPlatformLayoutName: Text[250]; SelectedPlatformLayoutAppID: Guid; EmptyGuid: Guid;
    procedure SetSelectedCustomLayout(NewCustomLayoutCode: Code[20]) begin SelectedCustomLayoutCode := NewCustomLayoutCode; end;
    procedure GetSelectedCustomLayout(): Code[20] begin exit(SelectedCustomLayoutCode); end;
    procedure SetSelectedLayout(LayoutName: Text[250]) begin SelectedPlatformLayoutName := LayoutName; end;
    procedure SetSelectedLayout(LayoutName: Text[250]; AppID: Guid) begin SelectedPlatformLayoutName := LayoutName; SelectedPlatformLayoutAppID := AppID; end;
    procedure GetSelectedLayout(): Text[250] begin if SelectedPlatformLayoutName = '' then exit(SelectedCustomLayoutCode); exit(SelectedPlatformLayoutName); end;
    procedure GetSelectedAppID(): Guid begin exit(SelectedPlatformLayoutAppID); end;
    procedure ClearLayoutSelection() begin SelectedPlatformLayoutName := ''; SelectedCustomLayoutCode := ''; SelectedPlatformLayoutAppID := EmptyGuid; end;
}

codeunit 44 "ReportManagement"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectedBuiltinLayoutType', '', false, false)]
    local procedure SelectedBuiltinLayoutType(ObjectID: Integer; var LayoutType: Option "None",RDLC,Word,Excel,Custom)
    var ReportLayoutSelection: Record "Report Layout Selection";
    begin
        LayoutType := ReportLayoutSelection.SelectedBuiltinLayoutType(ObjectID);
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SelectReportLayoutCode', '', false, false)]
    local procedure SelectReportLayoutCode(ObjectId: Integer; var LayoutCode: Text; var LayoutType: Option "None",RDLC,Word,Excel,Custom; var Success: Boolean)
    var
        CustomReportLayout: Record "Custom Report Layout";
        ReportLayoutSelection: Record "Report Layout Selection";
        DesignTimeReportSelection: Codeunit "Design-time Report Selection";
        SelectedLayoutName: Text[250];
        SelectedAppID: Guid;
    begin
        LayoutType := LayoutType::None;
        SelectedLayoutName := DesignTimeReportSelection.GetSelectedLayout();
        SelectedAppID := DesignTimeReportSelection.GetSelectedAppID();
        if SelectedLayoutName = '' then
            if ReportLayoutSelection.Get(ObjectId, CompanyName) and
               (ReportLayoutSelection.Type = ReportLayoutSelection.Type::"Custom Layout")
            then
                SelectedLayoutName := ReportLayoutSelection."Custom Report Layout Code";
        if (SelectedLayoutName <> '') and (StrLen(SelectedLayoutName) <= MaxStrLen(CustomReportLayout."Code")) then
            if CustomReportLayout.Get(SelectedLayoutName.ToUpper()) then begin
                LayoutCode := CustomReportLayout.Code;
                case CustomReportLayout.Type of
                    CustomReportLayout.Type::RDLC:
                        LayoutType := LayoutType::RDLC;
                    CustomReportLayout.Type::Word:
                        LayoutType := LayoutType::Word;
                end;
                Success := true;
                exit;
            end;
        if SelectedLayoutName <> '' then begin
            if IsNullGuid(SelectedAppID) then
                LayoutCode := SelectedLayoutName
            else
                LayoutCode := SelectedLayoutName + '::' + Format(SelectedAppID);
            Success := true;
        end;
        DesignTimeReportSelection.ClearLayoutSelection();
    end;
}
"#;

/// `src` compiled in a test app over the stand-in; `src` does not declare the stand-in's objects.
pub fn built(src: &str) -> Built {
    let base = AppSource {
        id: BASE_APPLICATION_ID.into(),
        name: "Base Application".into(),
        dependencies: vec![],
        files: vec![("base.al".into(), BASE.into())],
    };
    let test = AppSource {
        id: "a".into(),
        name: "a".into(),
        dependencies: vec![BASE_APPLICATION_ID.into()],
        files: vec![("f.al".into(), src.into())],
    };
    Built::new(al_testkit::compile_apps(vec![base, test]))
}
