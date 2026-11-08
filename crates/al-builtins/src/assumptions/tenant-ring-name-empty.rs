TenantRingNameEmpty => {
    id: "tenant-ring-name-empty",
    assumed: "DotNet NavTenantSettingsHelper.GetRingName() returns an empty text, so System Application's \"Environment Information\".IsPreview and IsEarlyPreview answer false and ShowEarlyPreviewNotification sends nothing. The empty text is the default of an unset tenant setting, as GetEnvironmentName and GetLinkedPowerPlatformEnvironmentId read on the probe tier; the ring itself is not measured.",
    tests: "No corpus test. AL Runner issue 6675: the reporter's TestPage OpenEdit and Close of Base Application pages \"SO Processor Activities\" and \"O365 Activities\" pass on a w1 container of the current BC major; both reach GetRingName in OnOpenPage (Role Center Notification Mgt..ShowNotifications, EnvironmentInformationImpl.Codeunit.al:130).",
    probe: "The raw GetRingName() in the OnPrem DotNet app, and IsPreview / IsEarlyPreview of \"Environment Information\" in the Cloud app, on the corpus CI containers.",
    awaiting: Awaiting {
        issues: &[6675],
        files: &[
            "tests/al-language-onprem-dotnet/dotnet/dotnet/BbNavHelpers.al::TenantSettings_GetRingName",
            "tests/al-language/types/PVEnvironment.al::EnvironmentInformation_PreviewRings",
        ],
        since: "2026-11-08",
    },
}
