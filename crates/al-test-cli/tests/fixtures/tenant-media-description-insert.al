// BC Documentation: https://learn.microsoft.com/dynamics365/business-central/dev-itpro/developer/methods-auto/record/record-insert-boolean-method
// Scope: Cloud-compatible standalone repro
// BC versions: 27.5, 28.5, 29.0
// local object: The system Tenant Media table is used directly with one reserved probe GUID.

codeunit 50140 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Record_Insert_TenantMediaDescriptionOnly_ReadsBack()
    var
        TenantMedia: Record "Tenant Media";
        Id: Guid;
    begin
        Initialize();
        Id := ProbeId();
        TenantMedia.Init();
        TenantMedia.ID := Id;
        TenantMedia.Description := 'Synthetic description';
        TenantMedia.Insert(false);
        Clear(TenantMedia);
        TenantMedia.Get(Id);
        TenantMedia.TestField(Description, 'Synthetic description');
        TenantMedia.Delete();
    end;
    local procedure Initialize()
    var
        TenantMedia: Record "Tenant Media";
    begin
        if TenantMedia.Get(ProbeId()) then
            TenantMedia.Delete();
    end;

    local procedure ProbeId(): Guid
    begin
        exit('eb5700b7-a069-42c4-a86e-404e2839defc');
    end;
}
