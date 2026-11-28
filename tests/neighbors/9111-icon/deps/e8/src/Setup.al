// The 8 x 8 JPEG logo of corpus UsAppLogoJpeg.al (fixture E); registers an assisted setup, as the fixture does.
codeunit 50182 "Icon E8 Setup"
{
    trigger OnRun()
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure RegisterAssistedSetup()
    var
        GuidedExperience: Codeunit "Guided Experience";
    begin
        GuidedExperience.InsertAssistedSetup('Icon E assisted', 'Icon E', 'Icon E', 1, ObjectType::Codeunit, 50182, Enum::"Assisted Setup Group"::Uncategorized, '', Enum::"Video Category"::Uncategorized, '');
    end;
}
