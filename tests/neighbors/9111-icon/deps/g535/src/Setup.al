// The 535 x 534 baseline JPEG logo of corpus PR 1594 (fixture G); registers an assisted setup, as the fixture does.
codeunit 50181 "Icon G535 Setup"
{
    trigger OnRun()
    begin
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', false, false)]
    local procedure RegisterAssistedSetup()
    var
        GuidedExperience: Codeunit "Guided Experience";
    begin
        GuidedExperience.InsertAssistedSetup('Icon G assisted', 'Icon G', 'Icon G', 1, ObjectType::Codeunit, 50181, Enum::"Assisted Setup Group"::Uncategorized, '', Enum::"Video Category"::Uncategorized, '');
    end;
}
