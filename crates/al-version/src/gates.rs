//! Behaviour that differs between BC versions, as named facts in one table (ADR-015 3.4).
//!
//! A gate is added only when the per-version oracles pin both sides (the corpus has a
//! `#if BCnnPLUS` guard for it). Code asks [`Target::has`]; it never compares a version.
//! A target newer than every gate gets every behaviour: a new major inherits the newest
//! known behaviour until an oracle case says otherwise.

use crate::Target;

/// A corpus test file and the lines of its `#if BCnnPLUS` guards, relative to `corpus/tests/al-language/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub file: &'static str,
    pub lines: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Gate {
    /// A FlowField Sum or Average over an `Integer` source into a `BigInteger` FlowField is
    /// accepted (and the reverse). Before, it is refused with "The following fields must have
    /// the same type".
    FlowFieldSumIntegerIntoBigInteger,
    /// Min and max FlowFields over `SystemRowVersion` are refused when calculated. Before, they
    /// calculate.
    FlowFieldMinMaxOverRowVersionRefused,
    /// Looking up the `SUPER` or `SECURITY` Metadata Permission Set under any App ID finds the
    /// null-App-ID row. Before, the lookup keys on the App ID. Not consumed yet: the
    /// permission provider is not implemented.
    SuperPermissionSetIgnoresAppId,
    /// `Environment Information.IsSaaSInfrastructure` is false. Before, it is true (the oracle
    /// runs of 27.5 and 28.5 answer true, the 29.0 run false, on the same container setup). It is
    /// `NavUserAccountHelper.IsAzure()` in the server-class registry.
    NotSaaSInfrastructure,
    /// The header of the dataset XML that `Report.SaveAs(..., ReportFormat::Xml, ...)` writes has a
    /// `ReportCaption` element after `ReportName` and, after `Minute`, the company information
    /// block. Before, it has neither.
    ReportSaveAsXmlHeaderHasCompanyBlock,
    /// Every `DataItem` element of the dataset XML that `Report.SaveAs` writes carries
    /// ` tableId="<table id>" systemId="<guid>"` after its name. Before BC 28 it carries neither
    /// (27.5 absent, 28.5 and 29.0 present; BC 28.0 to 28.4 were not measured).
    ReportSaveAsDataItemIdentity,
    /// The platform table "Feature Key" has another list of rows from BC 28: the corpus asserts the 27.x list (16 rows)
    /// only below BC 28 (`#if not BC28PLUS`, record/TestFieldLenAndFeatureKeyContracts.al); above, only that the table
    /// has rows. Not consumed yet (see `feature_key_rows_known`).
    FeatureKeyListChangedFrom28,
    /// Column 15 of the Permission table ("AL Object Name") is Text[100] from BC 29, Text[30] before
    /// (permissions/W6PermissionColumns.al, corpus bump 9). Not consumed yet.
    PermissionObjectNameLongerFrom29,
    /// A query aggregate (Sum, Average, Min, Max) over a BigInteger column is accepted from BC 29
    /// (query/W7QueryAggregates.al, corpus bump 9). Not consumed yet.
    QueryAggregateOverBigInteger,
    /// Typing the key of a card after a line was entered in its list part raises an error that cannot be caught on BC 27
    /// (an uncatchable CLR exception), so the corpus probe exists only from BC 28
    /// (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al, corpus bump 23). Not consumed yet.
    PartLineBeforeHostKeyFrom28,
    /// `RecordRef.IsDirty` exists from BC 28; the corpus probe is compiled only from BC 28
    /// (recordref/lt-recordref-isdirty-edges/RecordRefIsDirtyEdges.al, corpus bump 22). Not consumed yet.
    RecordRefIsDirtyFrom28,
    /// The platform table "Custom Report Layout" (table 2000000233) has two more columns from BC 29,
    /// "Header Part Name" and "Theme Part Name" (Text[300]); before, it ends at "App ID"
    /// (platform/lt-platform-tables/PtCloudTables.al, corpus bump 22). Not consumed yet.
    ReportLayoutSelectionPartNamesFrom29,
    /// The report metadata properties `DefaultHeaderFooterPartName` and `DefaultThemePartName` exist from BC 29 (runtime 18.0,
    /// empty by default); before they are absent (report/lt-y-report-props/YReportPropsTests.al, corpus bump 25). Not consumed yet.
    ReportPartNamePropertiesFrom29,
    /// A request page extension also runs `OnAfterGetCurrRecord` when a report request page opens, from BC 28 (pages/lt-ab-tests/ABTests.al, corpus bump 26). Not consumed yet.
    RequestPageExtensionRecordTriggerOrderFrom28,
    /// `FieldRef.Value` set from an Option text that is not a value: the Evaluate error from BC 28, `The type NavOption is unknown.` before (record/lt-u-table-field-props/UTableCdsOrdinals.al, corpus bump 26). Not consumed yet.
    OptionFieldRefSetEvaluateErrorFrom28,
    /// `AL:MetadataNotFound` for an unknown report id names the calling test codeunit from BC 28, and `Test Runner - Mgt.` before (report/lt-t-validate-layout/TValidateLayout.al, corpus bump 26). Not consumed yet.
    MetadataNotFoundNamesTheCallerFrom28,
    /// The fully qualified name overloads of Report.Run, Page.Run and RecordRef.Open exist from BC 28 (runtime 17.0); the test file is compiled from BC 28 on (report/lt-t-fqn-run/LtTFqnRun.al, corpus bump 26). Not consumed yet.
    FullyQualifiedNameRunAndOpenFrom28,
    /// Setting a field whose control add-in was not created raises `NSValidationFailed:DotNetControlAddInNotCreated` from BC 28 (lt-ac/ACControlAddIn.al, corpus bump 26). Not consumed yet.
    ControlAddInNotInstantiatedErrorFrom28,
    /// `JsonObject.SelectTokens` and `JsonArray.SelectTokens` into a List of [JsonToken] need app runtime 17: the test file is
    /// compiled from BC 28 on, and BC 27 refuses the methods at compile time (json/lt-json-selecttokens/SelectTokensList.al,
    /// corpus bump 27). Not consumed yet.
    JsonSelectTokensListFrom28,
    /// The system table Windows Language has 212 rows from BC 29 (208 before) (platform/lt-ag-contoso/AgWindowsLanguage.al, corpus bump 26). Not consumed yet.
    WindowsLanguageRowsFrom29,
    /// The platform table Language Selection has the row ff-NG and other rows from BC 29 (platform/lt-platform-rows-fresh/PlatformRowsLanguageSelection.al, corpus bump 26). Consumed by the Language Selection provider (al-vm platform/host_tables.rs).
    LanguageSelectionRowsFrom29,
    /// The report dataset header has the CompanyAddressLine1..8 elements from BC 29 (report/lt-ah-dataset-header/AhDatasetHeader.al, corpus bump 26). Not consumed yet.
    DatasetHeaderCompanyAddressLinesFrom29,
    /// `Report.SaveAs` of a report with inherent permissions is refused with `DB:ClientReadDenied` from BC 29 (report/lt-u-report-props/UPropertyTests.al, corpus bump 26). Not consumed yet.
    ReportSaveAsInherentPermissionsFrom29,
    /// Attributes on procedures and variables including the Obsolete state RequiredPending exist from BC 29 (lt-ac/ACAttributes.al, corpus bump 26). Not consumed yet.
    ObsoleteRequiredPendingAttributeFrom29,
    /// ModuleInfo.ContextSensitiveHelpUrl, EULA, Help, PrivacyStatement and the NavApp resource overloads with an AppId exist from BC 29 (lt-ac/ACBc29ModuleNavApp.al, corpus bump 26). Not consumed yet.
    ModuleInfoAndNavAppResourceMethodsFrom29,
    /// DataSourceContext and TestHandlerContext exist from BC 29 (lt-ac/ACContexts.al, corpus bump 26). Not consumed yet.
    DataSourceAndTestHandlerContextFrom29,
    /// The IsolatedStorage.Get overloads with an IsolationLevel exist from BC 29 (lt-ac/ACIsolatedStorageIsolation.al, corpus bump 26). Not consumed yet.
    IsolatedStorageIsolationLevelOverloadsFrom29,
    /// `Record.IsDirty()` exists from BC 29 (lt-ac/ACRecordIsDirty.al, corpus bump 26). Not consumed yet.
    RecordIsDirtyFrom29,
    /// `Get` with a Text argument for an Integer key: the Evaluate error `The value "..." can't be evaluated into type Integer.`
    /// (code `Evaluate:Format`) from BC 28; before, `The type NavInteger is unknown.` (record/lt-get-text-argument/GetWithTextArgument.al,
    /// corpus bump 22). Not consumed yet.
    GetTextArgumentEvaluateErrorFrom28,
    /// A BigInteger can be assigned to an Enum variable from BC 29 (an Option variable accepts it in every version).
    /// Before, it is refused with AL0122 (types/lt-bigint-enum-option/BigIntToEnumOption.al, corpus bump 20; must-not-compile
    /// case 0451). Not consumed yet.
    BigIntegerToEnumVariableFrom29,
    /// `Base64Convert.FromBase64` of text that is not Base64 raises from BC 28 (types/PVBase64.al, corpus bump 9).
    /// Not consumed yet.
    FromBase64InvalidTextRaises,
    /// BC 29 pins the first bytes of an exported Media (types/W7Media.al:134,145, an observation only BC 29 makes). Every major
    /// re-encodes an imported PNG to 99 bytes (W7Media.al:141-145, corpus bump 34; before bc-linux #161 the Linux image gave the
    /// imported bytes back on 29). Not consumed: Media.ExportStream refuses a look at the exported bytes on every major.
    MediaExportFirstBytesProbedFrom29,
    /// The platform metadata tables (All Obj, All Obj with Caption, Field, Key, Table and Codeunit Metadata) list other
    /// columns and rows from BC 29 (metadata/W6*.al, corpus bump 9). Consumed by `RecordRef.FieldExist(2000000005)`
    /// (al-vm refs.rs) and by the four user-name system fields of the Field table at 2000000005-8 (al-vm platform/fields.rs,
    /// system-fields/W6SystemFieldStamps.al:239-246); a record does not hold them, so RecordRef.Field on one is refused.
    PlatformMetadataColumnsFrom29,
    /// `SetView` and `SetFilter` of text that is no table filter raise the Evaluate error `The value "..." can't be
    /// evaluated into type TableFilter.` from BC 28 (records/W4PlatformAndMeta.al, corpus bump 9). Not consumed yet.
    TableFilterEvaluateError,
    /// The conversion and argument error texts of BC 27.5 and later (`The value "x" can't be evaluated into type Integer.`)
    /// differ from those of BC 27.0 and 27.3 (`The type NavInteger is unknown.`, `Invalid Conversion. Source: NavChar, ...`).
    /// The corpus compiles those tests from the BC 28 symbols on (`#if BC28PLUS`), so BC 27.5 is not covered either, and only the
    /// BC 28 and 29 values are pinned. HttpHeaders.al also pins the code of a blank header name per major (27 Format,
    /// 28 Argument, 29 Format). BeEnumSetValueBlank.al pins both sides for a Text in a Variant on an Enum or Option TestPage
    /// control (27.5 `The type NavOption is unknown.`, NotSupportedType): consumed by `al-vm` pages/controls.rs.
    ConversionErrorTextsFrom28,
    /// A test codeunit that runs a `List of [Interface ...]` is compiled only for BC 29: on BC 27 the server's generated C# does not
    /// compile and on BC 28 no result comes back (codeunit/lt-enums-interfaces/OjT71052.al, corpus bump 10). Corpus bump 30 adds
    /// `List of [Interface ...]`.AddRange and a `Dictionary of [..., Interface ...]` (collections/lt-cs-odd-leads/CsCollections.al:332,
    /// CsDictInterface.al:9), compiled only from BC 29 for the same reason. Not consumed yet.
    InterfaceListCodeunitFrom29,
    /// Default interface methods (an interface member with a body) exist from app runtime 18.0 (BC 29); BC 27 and 28 refuse the
    /// body at compile time (must-not-compile case 0325), so the test file is compiled only from BC 29
    /// (codeunit/lt-cw-interface-leads/CwDefaultInterfaceMethods.al:19, corpus bump 30). Not consumed yet.
    DefaultInterfaceMethodsFrom29,
    /// Probed values that are the same on BC 28 and 29 and differ on BC 27 (the laptop writer's probes-needed files, corpus
    /// bump 11): the date control shorthands, metadata table rows and keys, report dataset rows and names, the SystemRowVersion
    /// aggregates, Media import with three arguments. Each guarded test names its fact; split into one gate per fact when the
    /// runtime implements it. Corpus bumps 31 and 32 add two tests held on Linux that assert only from BC 28, because BC 27.5
    /// through the TestRunnerHub ends them with CallbackNotAllowed: the control add-in error of a test page
    /// (pages/lt-ao-testpage/AoTests.al:786) and the Reporting Triggers events of Report.Run/RunModal/Execute
    /// (events/lt-platform-events-v/PeVTests.al:725). Corpus bump 37 pins the corpus database's General Ledger Setup: its
    /// Additional Reporting Currency is EUR from BC 28 and blank on BC 27 (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:808,
    /// QF_GLSetup_CorpusRow). Not consumed yet.
    ProbedValuesDifferFrom28,
    /// A BigInteger literal in a Boolean control expression (`Enabled = 0L <> Rec.Big`) compiles on the client and runs.
    /// Before, the page fails to open ("Error compiling dynamic control expression ... Not a valid number."); the runtime
    /// refuses the page there, because the error text's index rule is not pinned.
    BigIntegerLiteralInControlExpressionFrom28,
    /// Probed values that are the same on BC 29 and differ on BC 27 and 28 (corpus bump 11): BLOB length after a fresh Get, the
    /// BigInteger/Integer FlowField type mismatch, the report dataset header bytes and lines and a second SaveAs on one variable,
    /// Report/Field metadata rows, Media import, BigInteger into an Enum field. Corpus bump 30 adds the RDLC Pdf output tests,
    /// asserted only below BC 29 (`#if not BC29PLUS`): the BC 29 Linux container has no renderer, which is not BC behavior
    /// (report/lt-am-rdlc-output/AMRdlcOutput.al:9-11). Corpus bump 34 lifts the RDLC holds of
    /// report/lt-ci-report-leads/CiReportRunLeads.al, which then has no guard left. Corpus bump 36 asserts the control add-in
    /// error of pages/lt-f3-pages/F3ValidationAddinFilter.al:409,447 only from BC 29, the one major checked on Windows; BC 27
    /// and 28 stay held on Linux there (BC 27.5 gives a client callback error, :417-420). Not consumed yet.
    ProbedValuesDifferFrom29,
    /// `Base64Convert.ToBase64` of a Text with `TextEncoding::Windows` or `MSDos` and no code page gives the code page 1252
    /// bytes from BC 28 (`äöü` is `5Pb8`); before, the UTF-8 bytes (`w6TDtsO8`) (types/PVBase64.al, corpus bump 13).
    Base64TextIsWindows1252From28,
    /// The `TestField` error text for a Guid value has braces around the expected value from BC 28 (`must be equal to
    /// '{1111...}'`); BC 27 writes it without braces. A Byte value is written as a number on BC 27. The corpus guards the
    /// typed-value tests with `#if BC28PLUS` (record/lt-testfield-typed-values/, corpus bump 17). Not consumed yet.
    TestFieldTypedValueTexts,
    /// `File.Download` with no client to receive the file (an OnPrem test run) is probed only from BC 28: the guarded test
    /// compiles on 28 and later and has no BC 27 counterpart (al-language-onprem/lt-file-type/FileQProbes.al:92, corpus bump 24).
    /// Not consumed yet; the test is refused by the runtime today.
    FileDownloadWithoutClientFrom28,
    /// Under `TestHttpRequestPolicy = BlockOutboundRequests` an `[HttpClientHandler]` answers the request from BC 29; before,
    /// the request raises the unhandled-request error as if no handler were bound, and the unused handler does not fail the test
    /// (al-language-onprem/session/PZHttpBlock.al:43-58 passes on BC 27 and 28: oracle/27/junit-onprem.xml:430, oracle/28/junit-onprem.xml:445).
    HttpHandlerUnderBlockOutboundFrom29,
    /// Under `TestHttpRequestPolicy = AllowOutboundFromHandler` an `[HttpClientHandler]` answers the request from BC 29; before,
    /// the request raises the unhandled-request error, the handler is not called and the unused handler does not fail the test
    /// (al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al, the BC 27 and 28 legs of every test).
    HttpHandlerUnderAllowFromHandlerFrom29,
    /// `HttpHeaders.Add` with a name of blanks raises `InvalidHeader:Format` from BC 29 and `InvalidHeader:Argument` on BC 28
    /// (network/lt-http/HttpHeaders.al:304-308).
    HttpBlankHeaderNameIsFormatFrom29,
    /// The header name rules of `HttpHeaders.Add` are probed from BC 28: the test is `#if BC28PLUS` (network/lt-http/HttpHeaders.al:282).
    HttpHeaderNameRulesProbedFrom28,
    /// `DataTransfer.AddDestinationFilter` needs app runtime 17: the corpus tests are compiled from BC 28 on and BC 27 refuses the
    /// method at compile time (record/lt-as-records/AsDataTransferDestFilter.al, AsDtInstallRead.al, AsDtInstaller.al; must-not-compile
    /// case 0365; corpus bump 28). Not consumed yet.
    DataTransferAddDestinationFilterFrom28,
    /// A `case` over a Time with a `0T:` label runs from BC 29; on BC 28.5 the server's generated C# for the codeunit does not compile
    /// (types/TestCaseContracts.al:391-416, corpus bump 28). Not consumed yet.
    CaseTimeZeroLabelFrom29,
    /// A count FlowField whose filter links a BigInteger source field to an Integer field (`Big = field(Id)`) counts from BC 29;
    /// before, CalcFields raises `The following fields must have the same type` (`DB:TypeMisMatch`)
    /// (calcfields/lt-bm-flowfield-conversions/BmFlowFieldConversions.al:166-186, corpus bump 28).
    FlowFieldCountFilterBigIntegerFromIntegerFrom29,
    /// `Format` of a Decimal under the fr-FR format region groups with U+00A0 (no-break space) from BC 29; BC 27 and 28 group
    /// with U+202F (narrow no-break space) (misc/lt-probes-needed-8/PnH4CurrReport.al:247,311; corpus bump 43).
    FrFrGroupSeparatorNoBreakSpaceFrom29,
    /// Under the default `TestHttpRequestPolicy` and `AllowAllOutboundRequests` an `[HttpClientHandler]` answers the request from
    /// BC 29; before, the handler is not called, the request is sent for real and the unused handler does not fail the test
    /// (al-language-onprem/network/lt-ay-http-handler/PyHttpPolicy.al, every `calls` line; corpus bump 43).
    HttpHandlerUnderDefaultPolicyFrom29,
    /// `Format` of a Decimal field with `AutoFormatType = 1` (an amount) rounds to 2 decimals from BC 28, under the default format
    /// and under `<Standard Format,2>` and format number 2 or 9; BC 27 writes every digit
    /// (types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al:684-723, :749-788; corpus bump 49). Consumed by
    /// `al-builtins` `format_decimal_field` through `FormatCtx::format_gate`, for a type 1 field without AutoFormatExpression.
    AutoFormatAmountFieldRoundsFrom28,
    /// The corpus CI runs a test codeunit without handlers through BC's TestRunnerHub from BC 28, under the codeunit's
    /// `RequiredTestIsolation`; on BC 27 it runs every codeunit under runner 130450 (Codeunit isolation). A difference of the
    /// CI's harness, not of the platform (codeunit/lt-qu-commit-isolation/QuCommitIsolation.al:10-16, 36-39). Consumed by
    /// `al-test-cli` (`RunOptions::required_isolation`).
    TestRunnerHubRequiredIsolationFrom28,
    /// `Media.ImportFile` of a .docx file keeps the file's bytes as Tenant Media content and gives them back on export on BC 27
    /// and 28; BC 29 on the Linux CI tier stores a re-zipped package (same parts, 919 bytes against 911), a Linux suspect the
    /// corpus ignores (al-language-onprem/media/MediaFiles.al:505-513, RD_Media_ImportFile_Docx; corpus bump 57). Consumed by
    /// `al-builtins` media.rs: from 29 a look at the row or the export is refused.
    DocxMediaRezippedFrom29,
    /// `MediaSet.ImportFile` of a .jpg file holding a JPEG keeps the file's bytes as Tenant Media content on BC 29; BC 27 and 28
    /// re-encode it (al-language-onprem/media/MediaFiles.al:897-901, RJ_MediaSet_ExportFile_ConfigPackageShape, `BC29PLUS`;
    /// corpus bump 61). Consumed by `al-builtins` media.rs: `MediaSet.ExportFile` writes those bytes from 29 and is refused
    /// below.
    JpegMediaSetKeptFrom29,
}

impl Gate {
    pub const ALL: &'static [Gate] = &[
        Gate::FrFrGroupSeparatorNoBreakSpaceFrom29,
        Gate::HttpHandlerUnderDefaultPolicyFrom29,
        Gate::AutoFormatAmountFieldRoundsFrom28,
        Gate::FlowFieldCountFilterBigIntegerFromIntegerFrom29,
        Gate::CaseTimeZeroLabelFrom29,
        Gate::DataTransferAddDestinationFilterFrom28,
        Gate::FlowFieldSumIntegerIntoBigInteger,
        Gate::FlowFieldMinMaxOverRowVersionRefused,
        Gate::SuperPermissionSetIgnoresAppId,
        Gate::NotSaaSInfrastructure,
        Gate::ReportSaveAsXmlHeaderHasCompanyBlock,
        Gate::ReportSaveAsDataItemIdentity,
        Gate::FeatureKeyListChangedFrom28,
        Gate::PermissionObjectNameLongerFrom29,
        Gate::QueryAggregateOverBigInteger,
        Gate::PartLineBeforeHostKeyFrom28,
        Gate::RecordRefIsDirtyFrom28,
        Gate::ReportLayoutSelectionPartNamesFrom29,
        Gate::ReportPartNamePropertiesFrom29,
        Gate::GetTextArgumentEvaluateErrorFrom28,
        Gate::RequestPageExtensionRecordTriggerOrderFrom28,
        Gate::OptionFieldRefSetEvaluateErrorFrom28,
        Gate::MetadataNotFoundNamesTheCallerFrom28,
        Gate::FullyQualifiedNameRunAndOpenFrom28,
        Gate::ControlAddInNotInstantiatedErrorFrom28,
        Gate::JsonSelectTokensListFrom28,
        Gate::WindowsLanguageRowsFrom29,
        Gate::LanguageSelectionRowsFrom29,
        Gate::DatasetHeaderCompanyAddressLinesFrom29,
        Gate::ReportSaveAsInherentPermissionsFrom29,
        Gate::ObsoleteRequiredPendingAttributeFrom29,
        Gate::ModuleInfoAndNavAppResourceMethodsFrom29,
        Gate::DataSourceAndTestHandlerContextFrom29,
        Gate::IsolatedStorageIsolationLevelOverloadsFrom29,
        Gate::RecordIsDirtyFrom29,
        Gate::BigIntegerToEnumVariableFrom29,
        Gate::FromBase64InvalidTextRaises,
        Gate::MediaExportFirstBytesProbedFrom29,
        Gate::PlatformMetadataColumnsFrom29,
        Gate::TableFilterEvaluateError,
        Gate::ConversionErrorTextsFrom28,
        Gate::InterfaceListCodeunitFrom29,
        Gate::DefaultInterfaceMethodsFrom29,
        Gate::ProbedValuesDifferFrom28,
        Gate::BigIntegerLiteralInControlExpressionFrom28,
        Gate::ProbedValuesDifferFrom29,
        Gate::Base64TextIsWindows1252From28,
        Gate::TestFieldTypedValueTexts,
        Gate::FileDownloadWithoutClientFrom28,
        Gate::HttpHandlerUnderBlockOutboundFrom29,
        Gate::HttpHandlerUnderAllowFromHandlerFrom29,
        Gate::HttpBlankHeaderNameIsFormatFrom29,
        Gate::HttpHeaderNameRulesProbedFrom28,
        Gate::TestRunnerHubRequiredIsolationFrom28,
        Gate::DocxMediaRezippedFrom29,
        Gate::JpegMediaSetKeptFrom29,
    ];

    /// The first BC major with the new behaviour.
    pub fn since_bc(self) -> u16 {
        match self {
            Gate::FlowFieldSumIntegerIntoBigInteger
            | Gate::FlowFieldMinMaxOverRowVersionRefused
            | Gate::SuperPermissionSetIgnoresAppId
            | Gate::NotSaaSInfrastructure
            | Gate::ReportSaveAsXmlHeaderHasCompanyBlock
            | Gate::PermissionObjectNameLongerFrom29
            | Gate::QueryAggregateOverBigInteger
            | Gate::ReportLayoutSelectionPartNamesFrom29
            | Gate::ReportPartNamePropertiesFrom29
            | Gate::BigIntegerToEnumVariableFrom29
            | Gate::MediaExportFirstBytesProbedFrom29
            | Gate::PlatformMetadataColumnsFrom29
            | Gate::InterfaceListCodeunitFrom29
            | Gate::DefaultInterfaceMethodsFrom29
            | Gate::ProbedValuesDifferFrom29
            | Gate::WindowsLanguageRowsFrom29
            | Gate::LanguageSelectionRowsFrom29
            | Gate::DatasetHeaderCompanyAddressLinesFrom29
            | Gate::ReportSaveAsInherentPermissionsFrom29
            | Gate::ObsoleteRequiredPendingAttributeFrom29
            | Gate::ModuleInfoAndNavAppResourceMethodsFrom29
            | Gate::DataSourceAndTestHandlerContextFrom29
            | Gate::IsolatedStorageIsolationLevelOverloadsFrom29
            | Gate::RecordIsDirtyFrom29
            | Gate::HttpHandlerUnderBlockOutboundFrom29
            | Gate::HttpHandlerUnderAllowFromHandlerFrom29
            | Gate::CaseTimeZeroLabelFrom29
            | Gate::FlowFieldCountFilterBigIntegerFromIntegerFrom29
            | Gate::HttpBlankHeaderNameIsFormatFrom29
            | Gate::FrFrGroupSeparatorNoBreakSpaceFrom29
            | Gate::HttpHandlerUnderDefaultPolicyFrom29
            | Gate::DocxMediaRezippedFrom29
            | Gate::JpegMediaSetKeptFrom29 => 29,
            Gate::ReportSaveAsDataItemIdentity
            | Gate::FeatureKeyListChangedFrom28
            | Gate::FromBase64InvalidTextRaises
            | Gate::TableFilterEvaluateError
            | Gate::ConversionErrorTextsFrom28
            | Gate::ProbedValuesDifferFrom28
            | Gate::BigIntegerLiteralInControlExpressionFrom28
            | Gate::Base64TextIsWindows1252From28
            | Gate::TestFieldTypedValueTexts
            | Gate::FileDownloadWithoutClientFrom28
            | Gate::GetTextArgumentEvaluateErrorFrom28
            | Gate::RecordRefIsDirtyFrom28
            | Gate::PartLineBeforeHostKeyFrom28
            | Gate::RequestPageExtensionRecordTriggerOrderFrom28
            | Gate::OptionFieldRefSetEvaluateErrorFrom28
            | Gate::MetadataNotFoundNamesTheCallerFrom28
            | Gate::FullyQualifiedNameRunAndOpenFrom28
            | Gate::ControlAddInNotInstantiatedErrorFrom28
            | Gate::JsonSelectTokensListFrom28
            | Gate::DataTransferAddDestinationFilterFrom28
            | Gate::HttpHeaderNameRulesProbedFrom28
            | Gate::AutoFormatAmountFieldRoundsFrom28
            | Gate::TestRunnerHubRequiredIsolationFrom28 => 28,
        }
    }

    /// The corpus guards that pin both sides.
    pub fn evidence(self) -> &'static [Evidence] {
        match self {
            Gate::AutoFormatAmountFieldRoundsFrom28 => &[Evidence {
                file: "types/lt-at-partner-gaps/AtPrecisionStandardFormat2.al",
                lines: "684,697,749,762",
            }],
            Gate::FrFrGroupSeparatorNoBreakSpaceFrom29 => &[Evidence {
                file: "misc/lt-probes-needed-8/PnH4CurrReport.al",
                lines: "247,311",
            }],
            Gate::DocxMediaRezippedFrom29 => &[Evidence {
                file: "../al-language-onprem/media/MediaFiles.al",
                lines: "506,511",
            }],
            Gate::JpegMediaSetKeptFrom29 => &[Evidence {
                file: "../al-language-onprem/media/MediaFiles.al",
                lines: "898,900",
            }],
            Gate::HttpHandlerUnderDefaultPolicyFrom29 => &[Evidence {
                file: "../al-language-onprem/network/lt-ay-http-handler/PyHttpPolicy.al",
                lines: "129,147",
            }],
            Gate::FlowFieldCountFilterBigIntegerFromIntegerFrom29 => &[Evidence {
                file: "calcfields/lt-bm-flowfield-conversions/BmFlowFieldConversions.al",
                lines: "38,95,127,147,155,176,181",
            }],
            Gate::CaseTimeZeroLabelFrom29 => &[Evidence {
                file: "types/TestCaseContracts.al",
                lines: "391,399,416",
            }],
            Gate::DataTransferAddDestinationFilterFrom28 => &[
                Evidence {
                    file: "record/lt-as-records/AsDataTransferDestFilter.al",
                    lines: "9",
                },
                Evidence {
                    file: "record/lt-as-records/AsDtInstallRead.al",
                    lines: "135",
                },
                Evidence {
                    file: "record/lt-as-records/AsDtInstaller.al",
                    lines: "26,182",
                },
            ],
            Gate::FlowFieldSumIntegerIntoBigInteger => &[
                Evidence {
                    file: "record/TestCalcFormulaValidationTests.Codeunit.al",
                    lines: "205,227",
                },
                Evidence {
                    file: "record/TestProbeCalcFieldsContracts.al",
                    lines: "120,125",
                },
                Evidence {
                    file: "calcfields/W4CalcFields.al",
                    lines: "31",
                },
            ],
            Gate::FlowFieldMinMaxOverRowVersionRefused => &[
                Evidence {
                    file: "record/TestCalcFormulaSystemFieldsTests.Codeunit.al",
                    lines: "298,306,353",
                },
                Evidence {
                    file: "calcfields/W4CalcFields.al",
                    lines: "105",
                },
            ],
            Gate::SuperPermissionSetIgnoresAppId => &[Evidence {
                file: "permissions/TestMetadataPermissionSet.al",
                lines: "98,106",
            }],
            Gate::NotSaaSInfrastructure => &[Evidence {
                file: "session/TestEnvironmentUserTenantContracts.al",
                lines: "169",
            }],
            Gate::ReportSaveAsXmlHeaderHasCompanyBlock => &[
                Evidence {
                    file: "report/lt-bd-format-region/BdRegionTests.al",
                    lines: "125,142,159,176",
                },
                Evidence {
                    file: "handlers/TestReportSaveAsDatasetContracts.al",
                    lines: "243",
                },
                Evidence {
                    file: "report/W3ReportDataset.al",
                    lines: "164,209",
                },
            ],
            Gate::PermissionObjectNameLongerFrom29 => &[Evidence {
                file: "permissions/W6PermissionColumns.al",
                lines: "122",
            }],
            Gate::BigIntegerToEnumVariableFrom29 => &[
                Evidence {
                    file: "misc/lt-au-language-edges/AuEdgeTests.al",
                    lines: "72",
                },
                Evidence {
                    file: "types/lt-bigint-enum-option/BigIntToEnumOption.al",
                    lines: "8,28,39,145",
                },
            ],
            Gate::ReportLayoutSelectionPartNamesFrom29 => &[
                Evidence {
                    file: "platform/lt-aw-fresh-tenant/AwTests.al",
                    lines: "227,243,259,275,291",
                },
                Evidence {
                    file: "report/lt-aq/AqTests.al",
                    lines: "275,291",
                },
                Evidence {
                    file: "platform/lt-platform-tables/PtCloudTables.al",
                    lines: "109,115,121,129",
                },
            ],
            Gate::GetTextArgumentEvaluateErrorFrom28 => &[
                Evidence {
                    file: "record/lt-as-records/AsGetTextKey.al",
                    lines: "95,110,152,167",
                },
                Evidence {
                    file: "record/lt-get-text-argument/GetWithTextArgument.al",
                    lines: "103,109,115,150,155",
                },
            ],
            Gate::RecordRefIsDirtyFrom28 => &[
                Evidence {
                    file: "record/lt-as-records/AsGetTableIsDirty.al",
                    lines: "9",
                },
                Evidence {
                    file: "recordref/lt-recordref-isdirty-edges/RecordRefIsDirtyEdges.al",
                    lines: "12",
                },
            ],
            Gate::ReportPartNamePropertiesFrom29 => &[
                Evidence {
                    file: "platform/lt-ba-columns/BaLayoutNoSubtype.al",
                    lines: "40,93,102",
                },
                Evidence {
                    file: "report/lt-aq/AqParts.al",
                    lines: "10",
                },
                Evidence {
                    file: "report/lt-y-report-props/YReportPropsTests.al",
                    lines: "45,60,75,90,105,120,135,151,169,187,237",
                },
                Evidence {
                    file: "report/lt-y-request-page/YRequestPageTests.al",
                    lines: "398",
                },
                Evidence {
                    file: "report/lt-y-report-props/YReportExt.al",
                    lines: "176,195,216",
                },
                Evidence {
                    file: "report/lt-y-report-props/YReportParts.al",
                    lines: "11",
                },
            ],
            Gate::RequestPageExtensionRecordTriggerOrderFrom28 => &[Evidence {
                file: "pages/lt-ab-tests/ABTests.al",
                lines: "1236",
            }],
            Gate::OptionFieldRefSetEvaluateErrorFrom28 => &[Evidence {
                file: "record/lt-u-table-field-props/UTableCdsOrdinals.al",
                lines: "170,175",
            }],
            Gate::MetadataNotFoundNamesTheCallerFrom28 => &[Evidence {
                file: "report/lt-t-validate-layout/TValidateLayout.al",
                lines: "144,150",
            }],
            Gate::FullyQualifiedNameRunAndOpenFrom28 => &[
                Evidence {
                    file: "misc/lt-cu-odd-leads/CuWriteTxTests.al",
                    lines: "235",
                },
                Evidence {
                    file: "report/lt-bj-fqn-page-run/BjFqnPageRun.al",
                    lines: "8",
                },
                Evidence {
                    file: "report/lt-t-fqn-run/LtTFqnRun.al",
                    lines: "7",
                },
            ],
            Gate::ControlAddInNotInstantiatedErrorFrom28 => &[Evidence {
                file: "lt-ac/ACControlAddIn.al",
                lines: "99",
            }],
            Gate::JsonSelectTokensListFrom28 => &[
                Evidence {
                    file: "json/lt-dc-json-leads/DcJsonLeads.al",
                    lines: "388",
                },
                Evidence {
                    file: "json/lt-ay-json/AyJsonTokenSelectTokens.al",
                    lines: "12",
                },
                Evidence {
                    file: "json/lt-json-selecttokens/SelectTokensList.al",
                    lines: "8",
                },
            ],
            Gate::WindowsLanguageRowsFrom29 => &[Evidence {
                file: "platform/lt-ag-contoso/AgWindowsLanguage.al",
                lines: "58",
            }],
            Gate::LanguageSelectionRowsFrom29 => &[Evidence {
                file: "platform/lt-platform-rows-fresh/PlatformRowsLanguageSelection.al",
                lines: "238,377,401,489,581",
            }],
            Gate::DatasetHeaderCompanyAddressLinesFrom29 => &[Evidence {
                file: "report/lt-ah-dataset-header/AhDatasetHeader.al",
                lines: "9",
            }],
            Gate::ReportSaveAsInherentPermissionsFrom29 => &[Evidence {
                file: "report/lt-u-report-props/UPropertyTests.al",
                lines: "185,190,195",
            }],
            Gate::ObsoleteRequiredPendingAttributeFrom29 => &[Evidence {
                file: "lt-ac/ACAttributes.al",
                lines: "146",
            }],
            Gate::ModuleInfoAndNavAppResourceMethodsFrom29 => &[Evidence {
                file: "lt-ac/ACBc29ModuleNavApp.al",
                lines: "7",
            }],
            Gate::DataSourceAndTestHandlerContextFrom29 => &[Evidence {
                file: "lt-ac/ACContexts.al",
                lines: "10",
            }],
            Gate::IsolatedStorageIsolationLevelOverloadsFrom29 => &[Evidence {
                file: "lt-ac/ACIsolatedStorageIsolation.al",
                lines: "7",
            }],
            Gate::RecordIsDirtyFrom29 => &[
                Evidence {
                    file: "events/lt-with-statement-scopes/WithStatementAmbiguous.al",
                    lines: "254,266,276,380",
                },
                Evidence {
                    file: "lt-ac/ACRecordIsDirty.al",
                    lines: "7",
                },
            ],
            Gate::PartLineBeforeHostKeyFrom28 => &[Evidence {
                file: "pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al",
                lines: "322",
            }],
            Gate::QueryAggregateOverBigInteger => &[Evidence {
                file: "query/W7QueryAggregates.al",
                lines: "148",
            }],
            Gate::FromBase64InvalidTextRaises => &[Evidence {
                file: "types/PVBase64.al",
                lines: "145",
            }],
            Gate::PlatformMetadataColumnsFrom29 => &[
                Evidence {
                    file: "system-fields/W6SystemFieldStamps.al",
                    lines: "182",
                },
                Evidence {
                    file: "metadata/W6AllObj.al",
                    lines: "207,212",
                },
                Evidence {
                    file: "metadata/W6FieldKeyRows.al",
                    lines: "167,262",
                },
                Evidence {
                    file: "metadata/W6TableCodeunitMetadata.al",
                    lines: "258",
                },
                Evidence {
                    file: "recordref/lt-recordref-fieldref-members/P92Fields.al",
                    lines: "72",
                },
                // The four user-name system fields in the Field rows of a table (corpus bump 54).
                Evidence {
                    file: "types/lt-aj-probes/AjMisc.al",
                    lines: "612,626,640",
                },
            ],
            Gate::TableFilterEvaluateError => &[Evidence {
                file: "records/W4PlatformAndMeta.al",
                lines: "256",
            }],
            Gate::ConversionErrorTextsFrom28 => &[
                Evidence {
                    file: "pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al",
                    lines: "541,760,999,1218",
                },
                Evidence {
                    file: "streams/lt-dd-stream-leads/DdStreamLeads.al",
                    lines: "540",
                },
                Evidence {
                    file: "errors/lt-error-handling/ErTRuntimeErrors.al",
                    lines: "13",
                },
                Evidence {
                    file: "network/lt-http/HttpHeaders.al",
                    lines: "282,304",
                },
                Evidence {
                    file: "record/lt-record-read/RcAGetFind.al",
                    lines: "200",
                },
                Evidence {
                    file: "recordref/lt-refs/RfTFieldFilters.al",
                    lines: "118",
                },
                Evidence {
                    file: "recordref/lt-refs/RfTValues.al",
                    lines: "178",
                },
            ],
            Gate::InterfaceListCodeunitFrom29 => &[
                Evidence {
                    file: "codeunit/lt-enums-interfaces/OjT71052.al",
                    lines: "43",
                },
                Evidence {
                    file: "collections/lt-cs-odd-leads/CsCollections.al",
                    lines: "332",
                },
                Evidence {
                    file: "collections/lt-cs-odd-leads/CsDictInterface.al",
                    lines: "9",
                },
            ],
            Gate::DefaultInterfaceMethodsFrom29 => &[Evidence {
                file: "codeunit/lt-cw-interface-leads/CwDefaultInterfaceMethods.al",
                lines: "19",
            }],
            Gate::BigIntegerLiteralInControlExpressionFrom28 => &[Evidence {
                file: "pages/lt-cc-page-leads/CcPageLeads.al",
                lines: "801",
            }],
            Gate::ProbedValuesDifferFrom28 => &[
                Evidence {
                    file: "collections/lt-cs-odd-leads/CsCollections.al",
                    lines: "594,607",
                },
                Evidence {
                    file: "pages/lt-ao-testpage/AoTests.al",
                    lines: "786",
                },
                Evidence {
                    file: "events/lt-platform-events-v/PeVTests.al",
                    lines: "725",
                },
                Evidence {
                    file: "pages/lt-px-controls/PxControls.al",
                    lines: "1115,1118,1125,1128",
                },
                Evidence {
                    file: "network/lt-http-send-failure/LtDaSendFailure.al",
                    lines: "10,140",
                },
                Evidence {
                    file: "install/lt-bf-install-probes/BfInstallTests.al",
                    lines: "84,89,124,131",
                },
                Evidence {
                    file: "query/lt-az-query/AzQueryTests.al",
                    lines: "262,284,291",
                },
                Evidence {
                    file: "platform/lt-aw-fresh-tenant/AwTests.al",
                    lines: "229,245,261,277,293,611",
                },
                Evidence {
                    file: "misc/lt-probes-needed-1/PnACalcFieldsBC29.al",
                    lines: "210",
                },
                Evidence {
                    file: "misc/lt-probes-needed-2/PnBPages.al",
                    lines: "297",
                },
                Evidence {
                    file: "misc/lt-probes-needed-2/PnBReportSaveAs.al",
                    lines: "185,291",
                },
                Evidence {
                    file: "misc/lt-probes-needed-3/PnC21Metadata.al",
                    lines: "297,385,468",
                },
                Evidence {
                    file: "misc/lt-probes-needed-5/PnE06Media.al",
                    lines: "65",
                },
                Evidence {
                    file: "pages/lt-page-runtime/P82TestField.al",
                    lines: "77,208",
                },
                Evidence {
                    file: "recordref/lt-recordref-fieldref-members/P92DirtyRef.al",
                    lines: "8",
                },
                Evidence {
                    file: "misc/lt-open-overloads-collections-isolated/P107RecRefNames.al",
                    lines: "99",
                },
                Evidence {
                    file: "record/lt-testfield-fielderror-typed/P96ErrorInfo.al",
                    lines: "142",
                },
                Evidence {
                    file: "record/lt-testfield-fielderror-typed/P96Temporal.al",
                    lines: "171",
                },
                Evidence {
                    file: "streams/StTCopyBlob.al",
                    lines: "451,456,462,484",
                },
                Evidence {
                    file: "error-handling/lt-last-error-text-forms/FindEmptyTableExcludeFlag.al",
                    lines: "76",
                },
                Evidence {
                    file: "metadata/lt-platform-metadata-columns/PlatformMetadataColumnValues.al",
                    lines: "220",
                },
                Evidence {
                    file: "record/lt-qd-captionclass-areas/QdCaptionClassAreas.al",
                    lines: "808",
                },
            ],
            Gate::ProbedValuesDifferFrom29 => &[
                Evidence {
                    file: "report/lt-am-report-output/AMReportOutputTests.al",
                    lines: "286",
                },
                Evidence {
                    file: "report/lt-am-rdlc-output/AMRdlcOutput.al",
                    lines: "313,386",
                },
                Evidence {
                    file: "misc/lt-cq-odd-leads/CqOddLeadObjects.al",
                    lines: "40,52",
                },
                Evidence {
                    file: "misc/lt-cq-odd-leads/CqOddLeads.al",
                    lines: "259,289",
                },
                Evidence {
                    file: "network/lt-cz-http-leads/CzHttpOutbound.al",
                    lines: "268",
                },
                Evidence {
                    file: "query/lt-cf-query-leads/CfQueryLeads.al",
                    lines: "283",
                },
                Evidence {
                    file: "types/lt-bu-property-conversions/BuPropertyConversions.al",
                    lines: "25,83",
                },
                Evidence {
                    file: "misc/lt-probes-needed-1/PnABlobLength.al",
                    lines: "34",
                },
                Evidence {
                    file: "misc/lt-probes-needed-1/PnACalcFieldsBC29.al",
                    lines: "87,253",
                },
                Evidence {
                    file: "misc/lt-probes-needed-2/PnBRecordMisc.al",
                    lines: "627",
                },
                Evidence {
                    file: "misc/lt-probes-needed-2/PnBReportSaveAs.al",
                    lines: "103,143,417",
                },
                Evidence {
                    file: "misc/lt-probes-needed-3/PnC21Metadata.al",
                    lines: "255,427",
                },
                Evidence {
                    file: "misc/lt-probes-needed-5/PnE06Media.al",
                    lines: "23",
                },
                Evidence {
                    file: "misc/lt-probes-needed-5/PnE13EnumConv.al",
                    lines: "159",
                },
                Evidence {
                    file: "recordref/lt-recordref-fieldref-members/P92Fields.al",
                    lines: "31",
                },
                Evidence {
                    file: "report/lt-report-dataset-header/ReportDatasetHeaderRaw.al",
                    lines: "192,197,202,207,212,217,222,227,232,237,242",
                },
                Evidence {
                    file: "pages/lt-f3-pages/F3ValidationAddinFilter.al",
                    lines: "409,447",
                },
            ],
            Gate::Base64TextIsWindows1252From28 => &[Evidence {
                file: "types/PVBase64.al",
                lines: "83,88,231",
            }],
            Gate::TestFieldTypedValueTexts => &[
                Evidence {
                    file: "record/lt-as-records/AsTestFieldTexts.al",
                    lines: "41,60,140,163",
                },
                Evidence {
                    file: "record/lt-testfield-typed-values/FieldRefTestFieldTypedNumeric.al",
                    lines: "143,165,175,207",
                },
                Evidence {
                    file: "record/lt-testfield-typed-values/FieldRefTestFieldTypedOther.al",
                    lines: "137,159,169,201",
                },
                Evidence {
                    file: "record/lt-testfield-typed-values/RecordTestFieldTypedOther.al",
                    lines: "121,139,149,177",
                },
                Evidence {
                    file: "record/lt-testfield-typed-values/RecordTestFieldWrongTypeGuidOnText.al",
                    lines: "8,53,60,67,75",
                },
                // Corpus bump 24: the Guid and Byte texts again, per value type (a Guid field against a Guid, a Byte on a Number field).
                Evidence {
                    file: "record/lt-testfield-value-types/RecordTestFieldGroupB.al",
                    lines: "30,72,78,84",
                },
                Evidence {
                    file: "fieldref/lt-testfield-value-types/FieldRefTestFieldGroupA.al",
                    lines: "260,306,312,318",
                },
                Evidence {
                    file: "fieldref/lt-testfield-value-types/FieldRefTestFieldGroupB.al",
                    lines: "486,532,538,544",
                },
            ],
            Gate::HttpHandlerUnderBlockOutboundFrom29 => &[Evidence {
                file: "../al-language-onprem/session/PZHttpBlock.al",
                lines: "54-57",
            }],
            Gate::HttpHandlerUnderAllowFromHandlerFrom29 => &[Evidence {
                file: "../al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al",
                lines: "190-207",
            }],
            Gate::HttpBlankHeaderNameIsFormatFrom29 => &[Evidence {
                file: "network/lt-http/HttpHeaders.al",
                lines: "304-308",
            }],
            Gate::TestRunnerHubRequiredIsolationFrom28 => &[Evidence {
                file: "codeunit/lt-qu-commit-isolation/QuCommitIsolation.al",
                lines: "443,531,724",
            }],
            Gate::HttpHeaderNameRulesProbedFrom28 => &[Evidence {
                file: "network/lt-http/HttpHeaders.al",
                lines: "282",
            }],
            Gate::FileDownloadWithoutClientFrom28 => &[Evidence {
                file: "../al-language-onprem/lt-file-type/FileQProbes.al",
                lines: "92",
            }],
            Gate::MediaExportFirstBytesProbedFrom29 => &[Evidence {
                file: "types/W7Media.al",
                lines: "134,145",
            }],
            Gate::FeatureKeyListChangedFrom28 => &[Evidence {
                file: "record/TestFieldLenAndFeatureKeyContracts.al",
                lines: "97",
            }],
            Gate::ReportSaveAsDataItemIdentity => &[
                Evidence {
                    file: "report/lt-aq/AqTests.al",
                    lines: "170,232",
                },
                Evidence {
                    file: "report/lt-bc-xml-blob/BcXmlBlob.al",
                    lines: "101,118,148,224",
                },
                Evidence {
                    file: "handlers/TestReportSaveAsDatasetContracts.al",
                    lines: "299",
                },
                Evidence {
                    file: "report/W3ReportDataset.al",
                    lines: "294,314",
                },
            ],
        }
    }
}

impl Target {
    /// Whether the target has the newer behaviour of `gate`.
    pub fn has(&self, gate: Gate) -> bool {
        self.bc.major >= gate.since_bc()
    }
}

/// [`Target::has`] for a run that may have no target: none means the newest behaviour.
pub fn has(target: Option<&Target>, gate: Gate) -> bool {
    target.is_none_or(|t| t.has(gate))
}

/// Which list of Feature Key rows a target has (record/PFFeatureKeyColumns.al keys them on the version of the application):
/// the rows differ per minor, and were measured on the newest minor of each major. The 27.0 and 28.0 rows differ from
/// those and are not served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureKeyRows {
    Bc27,
    Bc28,
    Bc29,
}

pub fn feature_key_rows(target: Option<&Target>) -> Option<FeatureKeyRows> {
    match target.map(|t| (t.bc.major, t.bc.minor))? {
        (27, _) => Some(FeatureKeyRows::Bc27),
        (28, 5) => Some(FeatureKeyRows::Bc28),
        (29, 0) => Some(FeatureKeyRows::Bc29),
        _ => None,
    }
}

/// The name of the test runner's outermost method, which the TryFunctionWrite error names when the test method itself uses
/// the result of the TryFunction: 'RunTests' (the classic runner "Test Runner - Mgt.") on BC 27.5, 'RootMethodScope' (the
/// TestRunnerHub runner) on 28.5 and 29.0. The corpus masks the name in its asserts; the measured names are recorded in
/// corpus commit ff42e3d3 ("CU: mask the test-runner scope name in the TryFunction write errors", BC 27.5 full run
/// 39322214364) and in misc/lt-cu-odd-leads/CuTryTests.al:236-238 and CuCommitTests.al:437-439. Not a [`Gate`]: the corpus
/// has no `#if` guard for it. A major that was not measured gets `None` (the name is not pinned there); no target means the
/// newest behaviour, as [`has`] does.
pub fn test_runner_root_scope(target: Option<&Target>) -> Option<&'static str> {
    match target.map(|t| t.bc.major) {
        Some(27) => Some("RunTests"),
        Some(28 | 29) | None => Some("RootMethodScope"),
        Some(_) => None,
    }
}

/// Whether the platform tables of System.app that are `Scope = OnPrem`, internal or removed are known for
/// this BC major: corpus must-not-compile 0020 to 0024 hold the lists for the symbols of BC 27 only, so
/// the AL0296 / AL0161 / AL0433 verdicts for them are not given for another major.
pub fn platform_table_scopes_known(bc_major: Option<u32>) -> bool {
    bc_major == Some(27)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BcVersion;

    fn target(bc: &str) -> Target {
        Target::for_bc(BcVersion::parse(bc).unwrap())
    }

    /// Corpus commit ff42e3d3, misc/lt-cu-odd-leads/CuTryTests.al:236-238.
    #[test]
    fn the_test_runner_root_scope_is_named_per_measured_major() {
        assert_eq!(test_runner_root_scope(Some(&target("27.5"))), Some("RunTests"));
        assert_eq!(test_runner_root_scope(Some(&target("28.5"))), Some("RootMethodScope"));
        assert_eq!(test_runner_root_scope(Some(&target("29.0"))), Some("RootMethodScope"));
        assert_eq!(test_runner_root_scope(None), Some("RootMethodScope"));
        assert_eq!(test_runner_root_scope(Some(&target("30.0"))), None);
    }

    #[test]
    fn a_gate_opens_at_its_major() {
        for g in Gate::ALL {
            let since = g.since_bc();
            assert!(!target("27.5").has(*g), "{g:?}");
            assert_eq!(target("28.5").has(*g), since <= 28, "{g:?}");
            assert!(target("29").has(*g), "{g:?}");
        }
    }

    #[test]
    fn a_newer_major_than_every_gate_has_every_behaviour() {
        for g in Gate::ALL {
            assert!(target("30").has(*g));
            assert!(has(None, *g), "no target means the newest");
        }
        assert!(!has(Some(&target("27")), Gate::FlowFieldSumIntegerIntoBigInteger));
    }

    #[test]
    fn every_gate_is_listed_once_and_has_evidence() {
        let mut seen = std::collections::BTreeSet::new();
        for g in Gate::ALL {
            assert!(seen.insert(*g), "{g:?} listed twice");
            assert!(!g.evidence().is_empty(), "{g:?} has no corpus evidence");
        }
    }
}
