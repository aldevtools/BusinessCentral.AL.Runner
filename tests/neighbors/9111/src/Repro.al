page 50154 "JL Setup Guide"
{
    PageType = NavigatePage;
    ApplicationArea = All;
    layout { area(Content) { } }
}

codeunit 50154 "JL Setup Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Guided Experience", 'OnRegisterAssistedSetup', '', true, true)]
    local procedure RegisterSetup()
    var
        GuidedExperience: Codeunit "Guided Experience";
        AssistedSetupGroup: Enum "Assisted Setup Group";
        VideoCategory: Enum "Video Category";
    begin
        GuidedExperience.InsertAssistedSetup('Set up Repro', 'Set up Repro', 'Repro setup guide', 0, ObjectType::Page, Page::"JL Setup Guide", AssistedSetupGroup::Uncategorized, '', VideoCategory::Uncategorized, '');
    end;
}
