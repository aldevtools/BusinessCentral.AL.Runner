//! The registry of behaviors the runtime implements on an unprobed assumption.
//!
//! Owner rule (CLAUDE.md hard rule 3, exception of 2026-10-15): behavior a Microsoft test needs may be
//! implemented before its corpus probe exists, but every test that passes through such behavior is a
//! FLAGGED pass: it relies on something no real-BC measurement has confirmed. Each such code path calls
//! [`assume`] with its entry of [`Assumption`] when it runs; the runner collects the entries a test hit and
//! reports the pass as flagged (console summary, JUnit property, JSON, `--explain`, `--top-assumptions`).
//!
//! One entry per assumption: its stable id, the behavior assumed, the Microsoft test(s) that need it and the
//! probe wanted. `docs/probes-needed.md` holds a table generated from this registry (a test checks it; run
//! the test with `UPDATE_ASSUMPTIONS_DOC=1` to rewrite it). When a probe pins an assumption, delete its entry:
//! the compiler then names every place that still marks it.
//!
//! An entry for a user issue fixed before its corpus probe sets `awaiting`: the al-runner issue(s) and the corpus
//! file(s) that will pin it. A test requires it when the entry or its `docs/probes-needed.md` lines name an
//! al-runner issue; `al-test-cli --awaiting-assumptions` fails once every awaited file is in the pinned corpus.
//!
//! A test (`every_entry_is_marked_somewhere`) checks that each entry is used by at least one code path; an
//! unregistered marker does not compile.

use std::cell::Cell;

macro_rules! registry {
    ($( $(#[$m:meta])* $variant:ident => {
        id: $id:expr,
        assumed: $assumed:expr,
        tests: $tests:expr,
        probe: $probe:expr,
        $(note: $note:expr,)?
        $(awaiting: $awaiting:expr,)?
    } ),* $(,)?) => {
        /// A behavior implemented on an assumption no probe has confirmed.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
        #[repr(u8)]
        pub enum Assumption { $( $(#[$m])* $variant ),* }

        impl Assumption {
            /// Every entry, in registry order.
            pub const ALL: &'static [Assumption] = &[ $( Assumption::$variant ),* ];

            /// The entry's text.
            pub fn info(self) -> Info {
                match self {
                    $( Assumption::$variant => Info {
                        id: $id,
                        assumed: $assumed,
                        tests: $tests,
                        probe: $probe,
                        note: registry!(@note $($note)?),
                        awaiting: registry!(@awaiting $($awaiting)?),
                    } ),*
                }
            }
        }
    };
    (@note) => { None };
    (@note $n:expr) => { Some($n) };
    (@awaiting) => { None };
    (@awaiting $w:expr) => { Some($w) };
}

/// What the registry says about one assumption.
#[derive(Debug, Clone, Copy)]
pub struct Info {
    /// The stable id: lower case words with hyphens.
    pub id: &'static str,
    /// The behavior assumed, in one line.
    pub assumed: &'static str,
    /// The Microsoft test(s) that need it.
    pub tests: &'static str,
    /// The probe wanted.
    pub probe: &'static str,
    /// An open point about the entry (a probe that contradicts it, say).
    pub note: Option<&'static str>,
    /// The corpus pin that will replace the assumption, when one is on its way. Required for an entry that
    /// names an al-runner issue (a runtime fix for a user issue that landed before its probe); `al-test-cli
    /// --awaiting-assumptions` fails once every awaited file is in the pinned corpus.
    pub awaiting: Option<Awaiting>,
}

/// The corpus pin an assumption waits for.
#[derive(Debug, Clone, Copy)]
pub struct Awaiting {
    /// The al-runner issue number(s) the assumption serves; empty for a writer's probe that serves no issue.
    pub issues: &'static [u32],
    /// The corpus test file(s) that will pin it, relative to the corpus root (`tests/al-language/...al`); for a test
    /// that will be added to a file already in the corpus, `<file>::<test procedure>`.
    pub files: &'static [&'static str],
    /// The day the entry started waiting, `YYYY-MM-DD` (`--awaiting-assumptions --summary` lists the oldest first).
    pub since: &'static str,
}

/// One sentence on what `w` waits for, for the generated table and `--list-assumptions`.
pub fn awaiting_text(w: &Awaiting) -> String {
    let issues: Vec<String> = w.issues.iter().map(u32::to_string).collect();
    let files: Vec<String> = w.files.iter().map(|f| format!("`{f}`")).collect();
    if issues.is_empty() {
        return format!("Awaiting since {}: corpus {}.", w.since, files.join(", "));
    }
    format!(
        "Awaiting since {}: al-runner issue{} {}, corpus {}.",
        w.since,
        if w.issues.len() == 1 { "" } else { "s" },
        issues.join(", "),
        files.join(", ")
    )
}

registry! {
    ReadUncommittedReadOpensNoWriteTransaction => {
        id: "readuncommitted-read-opens-no-write-transaction",
        assumed: "A read under ReadIsolation ReadUncommitted does not open a write transaction (IsInWriteTransaction No, a guarded Codeunit.Run allowed), as a ReadCommitted read does not (codeunit/TestCodeunitRunWriteTransaction.al:254-278 `readcommitted_read` No).",
        tests: "Tests-ERM Manual Payment Registration.PostPaymentForSalesInvoiceBalAccountBank and about 25 more Tests-ERM tests whose posting reads under ReadUncommitted before a guarded Codeunit.Run or a write-transaction check.",
        probe: "Commit, then a FindFirst under ReadIsolation ReadUncommitted on a database table: IsInWriteTransaction, and a guarded Codeunit.Run afterwards.",
    },
    PageFieldFollowsSourceFieldEditableFalse => {
        id: "page-field-follows-source-field-editable-false",
        assumed: "A page field over a source table field that is a FlowFilter and declares Editable = false, with no Editable of its own on the control, reads Editable() No on a TestPage. A stored field (pages/lt-page-field-editable-source/PageFieldEditableSource.al) and a FlowField (handlers/N582TestPageEdges.al:324) are pinned.",
        tests: "Payment Export Mgt Unit Test.FilterByHasPaymentExportError (Gen. Journal Line.\"Has Payment Export Error\", a FlowField with Editable = false, on the Payment Journal).",
        probe: "A FlowFilter field with Editable = false and a page with a plain field control over it, and one that sets Editable = true itself: Editable() of each control on an open TestPage.",
    },
    RenameFollowsFilterConditionArms => {
        id: "rename-follows-filter-condition-arms",
        assumed: "A rename of a primary key rewrites a referencing field through a TableRelation arm whose conditions include `Field = filter(x)`, when the referencing row's field passes the filter, as it does for `const` conditions. A single `filter(<> X)` is pinned (record/lt-table-relation-arms/TableRelationArms.al:208); other filter forms (`< 0`, `<> A & <> B`) are assumed alike.",
        tests: "Tests-ERM price list tests that rename an Item, Resource, G/L Account or Item Variant (Assembly Header.\"Bin Code\" `Quantity = filter(< 0)`, Purchase Line.\"No.\" and \"Variant Code\" `Document Type = filter(<> \"Credit Memo\" & <> \"Return Order\")`); about 28 Tests-ERM tests.",
        probe: "A table with a field whose TableRelation is `if (Qty = filter(< 0)) Parent` and one with `if (Kind = filter(<> B & <> C)) Parent`: rename a Parent row with referencing rows inside and outside each filter; read the fields.",
    },
    PackageAutoIncrementFromOne => {
        id: "package-auto-increment-from-one",
        assumed: "A table built from a RapidStart package whose rows hold no value for the table's AutoIncrement field (Dimension Value.\"Dimension Value ID\" is not in the shipped package) gets 1, 2, 3 in the order of the package's rows, as the database numbers rows it inserts, and the counter continues after them.",
        tests: "Sales-Calc. Discount Test LcyDiscountTest and ERM Sales Quotes SalesQuoteWithContact (a dimension set entry must carry a nonzero Dimension Value ID); Production & Assembly Costing AssemblyOrderWithDimensionShouldPostWithoutError.",
        probe: "Not measurable from the corpus (it depends on the import tool): import a RapidStart package into Dimension Value and read the field. Until then the rule rests on the field being AutoIncrement and absent from Microsoft's package.",
    },
    DatabaseTriggerSetupPerCompany => {
        id: "database-trigger-setup-per-company",
        assumed: "The answer of the Global Triggers event GetDatabaseTableTriggerSetup is not reused after the session's company changes (the install baseline's per-database roots run with no company after the per-company roots).",
        tests: "Microsoft's install roots Plan Installer, Upgrade Tag Install and No. Series Copilot Install must run to their end on real BC for Microsoft's tests to pass; their setup subscribers (Change Log Management.GetDatabaseTableTriggerSetup, CRM Integration Management.GetDatabaseTableTriggerSetup) exit when CompanyName() is empty.",
        probe: "A GetDatabaseTableTriggerSetup subscriber that logs CompanyName(); write a table, ChangeCompany, write it again: is the setup asked again per company? Asked in corpus #651.",
    },
    GetUrlWebBaseAddress => {
        id: "geturl-web-base-address",
        assumed: "The host part (host name and port) of every GetUrl(ClientType::Web) and GetUrl(ClientType::Default) answer comes from the host's server base address as set (--server-url, default http://localhost:8080/BC; the corpus tool uses http://bcserver:8080/BC); GetUrl(ClientType::Web) alone returns the address unchanged (ADR-042). The scheme http, the port, the path /BC without a trailing slash, the Default forms dynamicsnav://<host>/BC/<company>/[runpage?page=N[&bookmark=...]|runreport?report=N] and the query after the address are pinned by the corpus (session/lt-geturl-shape/GetUrlShape.al:270, platform/lt-t-geturl/TGetUrl.al:203-210, platform/lt-system-database-members/GetUrl*.al) and are not part of the assumption.",
        tests: "Company-Initialize (Doc. Exch. Service Setup.SetURLsToDefault -> OAuth2Impl.GetDefaultRedirectUrl), so the install baseline of a company built from the RapidStart package; every Microsoft test that reads Application Area Setup or Workflow after it.",
        probe: "GetUrl(ClientType::Web) inside a per-database install trigger (no company); the host name is environmental and is masked by the corpus (GetUrlShape.al:283).",
    },
    PageOnInsertRecordFalseAfterCodeInsert => {
        id: "page-oninsertrecord-false-after-code-insert",
        assumed: "When a page's OnInsertRecord inserts the row itself and answers false, the page goes on with that row as a stored one: the next edit of the row runs OnModifyRecord instead of OnInsertRecord again.",
        tests: "Item Tracking Lines (InsertRecord inserts into the page's temporary source, ValidateAndInsert answers false); Tests-Manufacturing PostProductionJournalUsingLotItemTracking and the item tracking tests that set a lot and a quantity.",
        probe: "A page with OnInsertRecord { Rec.Insert(); exit(false) } and OnModifyRecord { Rec.Modify(); exit(false) }: OpenNew, set the key, set a second field, OK; read the row and count the trigger calls. Asked in corpus #641.",
    },
    XmlportExportsFlowfieldCalculated => {
        id: "xmlport-exports-flowfield-calculated",
        assumed: "An XmlPort field element or attribute bound to a FlowField exports the calculated value, as if CalcFields had run on the row. (An Xml import skips a Lookup, Sum or Count FlowField element: pinned by xmlport/lt-ay-xmlport/AyXmlPortLookupFlowField.al and XmlPortFlowFieldSumCount.al.)",
        tests: "Import / Export Workflow (\"Workflow Step Argument\".\"Response Option Group\", a Lookup FlowField); 76 Tests-Workflow tests.",
        probe: "An export-only XmlPort with a fieldelement over a Sum FlowField and over a Lookup FlowField of a row with children: the exported text with and without a CalcFields on the row before; an import of the same element. Asked in corpus #650.",
    },
    EvaluateRecordIdCompositeKey => {
        id: "evaluate-recordid-composite-key",
        assumed: "Evaluate into a RecordId reads a Code key value that has blanks around it as the trimmed value (the length check counts the trimmed text).",
        tests: "Tests-Rapid Start (ERM RS Package Base Operations: the Config. Package Record IDs hold ' GUAAAAAAAB'); about 39 tests.",
        probe: "Evaluate(Rid, 'T: ' + ' ABC') and Evaluate(Rid, 'T: ABC ') into a RecordId of a table with a Code[10] key, and a value of 10 characters with a leading blank: the key value and whether it is accepted.",
    },
    TestPageAssertEqualsIntegerOnTextControl => {
        id: "testpage-assertequals-integer-on-text-control",
        assumed: "TestPageField.AssertEquals(<BigInteger>) on a control over a Text or Code value compares the number spelled in plain digits with the control's text, for a BigInteger outside 0 to 9 (an Integer and a Decimal are pinned). An Enum there stays refused.",
        tests: "Tests-Marketing Marketing Matrix Management.TasksMatrixHandlerWithValue (Field1.AssertEquals(1) on the Text[1024] cells of Tasks Matrix); the 10 TaskWith* tests.",
        probe: "A page with a field over a Text[30] variable holding '1000' and '-5': TestPage Field.AssertEquals(1000), AssertEquals(-5) with a BigInteger expected value.",
    },
    TestPageSetValueBlankOptionMember => {
        id: "testpage-setvalue-blank-option-member",
        assumed: "TestPageField.SetValue of a blank text on an Option control whose first member has no OptionCaption (the member name is blank) selects that member. A control with an OptionCaption starting with ' ' is pinned.",
        tests: "Tests-Marketing Marketing Matrix Management.TaskWithStatus* (Tasks.StatusFilter.SetValue(StatusFilter::\" \") resets the filter); 7 tests.",
        probe: "A page with an Option variable control without OptionCaption whose first member is \" \": TestPage SetValue(' ') after SetValue(A), then Value.",
    },
    TestPageSetValueBlankEnumMember => {
        id: "testpage-setvalue-blank-enum-member",
        assumed: "TestPageField.SetValue of an Integer on a control bound to an Enum page variable selects the value of that ordinal when its caption is blank (all spaces), on a list, a page without a source table or a page without a row. A card with a source row is pinned (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:2366).",
        tests: "Tests-ERM ERM Budget (134922, 36 tests) and ERM Budget UI (134927, 11 tests): Initialize calls Budget.GLAccCategory.SetValue(0) on page 113 Budget, a control over GLAccCategoryFilter: Enum \"G/L Account Category\" (value 0 \" \", Caption ' ').",
        probe: "The BeEnumSetValueBlank cases (SetValue(0) on a blank-captioned Enum variable control) on a page without a source table and on a list page.",
    },
    PartFieldUnderTruePartExpression => {
        id: "part-field-under-true-part-expression",
        assumed: "TestField.Editable()/Enabled() of a field of a part whose part control has an Editable or Enabled expression that comes out true answer as without the property. An expression that comes out false is refused; a literal Editable = false is pinned (PnGPartRows.al:117).",
        tests: "Tests-ERM sales and purchase document tests that read fields of a lines part behind `Editable = IsSalesLinesEditable; Enabled = IsSalesLinesEditable;` (and the purchase twins), about 170 passes in Tests-ERM.",
        probe: "A host card with parts `Editable = Flag` and `Enabled = Flag`: with Flag false and true (set in OnOpenPage, then changed by an action), a part field's Editable() and Enabled(), and SetValue through the part.",
    },
    EvaluateTimeDigitForms => {
        id: "evaluate-time-digit-forms",
        assumed: "Evaluate into a Time (format 0) reads six digits as hhmmss and one or two digits as an hour; an hour from 24, minutes or seconds from 60 are rejected.",
        tests: "Calendar Management CalcTimeDelta tests (Evaluate of '085606') and Evaluate(RandomTime, Format(RandInt(23))); 11 Tests-SCM-Manufacturing tests.",
        probe: "Evaluate(T, '085606'), '23', '5', '246000', '24', '0856', '8:56' into a Time: the result and the return value; with and without a format number. Asked in corpus #649.",
    },
    CodeCoverageSnapshotWhileLogging => {
        id: "code-coverage-snapshot-while-logging",
        assumed: "While CodeCoverageLog(true) is on, the Code Coverage tables hold the snapshot that CodeCoverageRefresh wrote and do not grow with later hits (a read before any refresh is empty, pinned by platform/lt-code-coverage-refresh/CodeCoverageRefreshLogged.al).",
        tests: "Tests that read the Code Coverage table after a refresh while logging is still on: none known in the buckets.",
        probe: "CodeCoverageLog(true, false), run code, CodeCoverageRefresh(), count the rows, run more code, count again (still logging), then CodeCoverageRefresh() and count. Asked in corpus #647.",
    },
    TableRelationQuotes => {
        id: "table-relation-quotes",
        assumed: "A filter(...) text of a TableRelation where() clause or arm condition that quotes an identifier with \" is read like a SetFilter text after the quotes are rewritten to '. (Pinned, no mark, on an Enum or Option field: members joined by |, one quoted member alone and <> one quoted member, record/lt-table-relation-quotes/TableRelationQuotes.al:41-46; a range with one quoted and one plain bound, A..\"B C\" and \"B C\"..D, two <> terms joined by & with one quoted, and on an Enum field \"B C\".., TableRelationQuotes.al:279-397; in the if (...) condition of an arm on an Option field, <> \"B C\" & <> D and \" \" | A, TableRelationQuotes.al QP_Arms_* and QP_BlankOr_*.) Assumed: other ranges and filter operators (..\"B C\", two quoted bounds, more & terms), a quoted blank member in a where() clause, a field of another type, and the other forms of an arm condition.",
        tests: "Microsoft relations with a quoted member in another form than the pinned ones.",
        probe: "TableRelation ... where(F = filter(..\"B C\")), filter(\" \" | A) and an arm if (F = filter(\"B C\")), Validate with a value of each kind.",
    },
    TextDigitsToInteger => {
        id: "text-digits-to-integer",
        assumed: "A Text or Code of digits (optional sign, surrounding spaces ignored) converts into an Integer or BigInteger where a Variant meets a number, such as the key of Get; a text with anything else stays refused.",
        tests: "Incoming Document.FindFromIncomingDocumentEntryNo (`Get(Format(FieldRef.Value))`): about 400 Tests-ERM tests behind the Incoming Doc. Attach. FactBox.",
        probe: "Rec.Get(Format(5)) on an Integer key; Get('5'); Get('x'); Get('') and Get(' 5 ') on an Integer and a BigInteger key. Asked in corpus #644.",
    },
    CustomLayoutMerger => {
        id: "custom-layout-merger",
        assumed: "A SaveAs of a report whose selected rendering layout has Type = Custom raises the platform's CustomDocumentMergerEx with ObjectID, a ReportAction named after the format (SaveAsPdf, SaveAsWord, SaveAsExcel, SaveAsHtml), an EMPTY ObjectPayload, XmlData holding the Xml dataset SaveAs writes, LayoutData holding the layout file (read as UTF-8) and DocumentStream on the stream SaveAs was given (UTF-8); a subscriber that sets Success/IsHandled makes SaveAs answer true. A SaveAs no subscriber handles is refused.",
        tests: "handlers/TestReportLayoutByNameMergerCapture.al pins the subscriber's signature and the layout text; Pageworks (about 50 tests whose first refusal is SaveAs as Pdf).",
        probe: "A custom layout report saved as Pdf, Word, Excel and Html with a Base Application subscriber that records every argument: the ReportAction ordinals, the keys of ObjectPayload (layoutmimetype, layoutname, ...), the form of XmlData, the encoding of the streams, and SaveAs with no subscriber.",
    },
    PageExpressionControlLiveReadAfterNextFilterActivate => {
        id: "page-expression-control-live-read-after-next-filter-activate",
        assumed: "An expression control the page does not keep (it reads the record or runs other code) shows its value as computed at the read after TestPage.Next, TestPage.Filter, the TestFilter calls (SetFilter, ...), TestField.Activate and TestField.Caption: BC refreshes the page at each of these. Pinned only for the kept codeunit-getter controls of a Card and its parts (pages/lt-qj-part-propagation/QjPartPropagation.al:4629-4710).",
        tests: "Price Lists UI.T003_SalesPriceListsDiscountsFromCustomersList and 5 more after TestPage.Next, T171_ShowPriceListLinesForItemDiscGroup after TestPage.Filter; Price List Line UT.VerifyPriceListLineforJobTaskAfterCopyJob after TestField.Activate, VarifyFieldsOnPurchaseJobPriceListsPage and VarifyFieldsOnSalesJobPriceListsPage after TestField.Caption; ERM Invoice Discount And VAT and Document Totals Pages.SalesOrderTotalsUnchangedWhenLineDescriptionChanged after TestFilter.SetFilter.",
        probe: "A list whose expression control reads the record (`Rec.Amount * 2`) and one that calls a page procedure: read after Next(), Filter.SetFilter, Activate and a control's Caption.",
    },
    ExtLocalProcNamedLikeBase => {
        id: "ext-local-proc-named-like-base",
        assumed: "A page or report extension of another app's object may declare a local procedure with the name and parameter types of a public or protected procedure of its base. A bare call in the extension's code runs the extension's local procedure; the base's own calls and calls on a variable of the object from outside run the base's procedure. alc compiles it and binds the calls this way (probed with alc: a local F(): Text next to a protected F(): Integer makes `I := F()` in the extension a Text-to-Integer error).",
        tests: "No Microsoft test: al-runner issue 6043, the reporter's run on real BC (Continia's extension of Customer Card declares local ActivateFields next to the page's protected one).",
        probe: "Two apps: a page with protected ActivateFields() and public Shared() that log, and a page extension of another app with local ActivateFields() and local Shared() that log; a call of each from the base, from the extension, and P.Shared() from a test codeunit. Corpus writer: not yet assigned.",
        awaiting: Awaiting {
            issues: &[6043],
            files: &["tests/al-language/pageext/lt-ext-local-same-name/ExtLocalSameNameTests.al"],
            since: "2026-10-28",
        },
    },
    PageExpressionControlSetValue => {
        id: "page-expression-control-setvalue",
        assumed: "TestPage.SetValue on a field control whose source is an assignable path (a record in an array, `Totals[1].\"Inv. Discount Amount\"`) assigns the path, then runs the control's OnValidate; the value is converted to the type of the value the path holds now. An entry the client would refuse is refused with the variable-control texts.",
        tests: "Purchase and Sales Order Statistics (`InvDiscountAmount_General`): ConfirmationOnInvDiscUpdateInPurchOrderWithInvoicedLine and about 55 more Tests-ERM tests.",
        probe: "A page field(Ctl; Arr[1].Amount) with an OnValidate; TestPage.Ctl.SetValue(5) then the global and the OnValidate effect; SetValue('x') on it. Asked in corpus #641.",
    },
    GuidAsTextArgument => {
        id: "guid-as-text-argument",
        assumed: "A Guid passed where a function wants a Text is its text form {XXXXXXXX-...}, as in an assignment. Pinned for CopyStr, StrLen, UpperCase, LowerCase, StrPos, Notification.SetData, StrSubstNo's format, DelStr, PadStr, ConvertStr, IncStr, SelectStr, InsStr, DelChr, Contains, IndexOf, StartsWith, Replace and a procedure's Text and Code parameter (types/lt-guid-as-text-argument/GuidAsTextArgument.al); the other text parameters that read through the shared text argument are assumed alike.",
        tests: "Match General Jnl Lines UT (CopyStr(CreateGuid(), 1, 50)); 61 Tests-General Journal tests. Error Message Handler.ShowNotification (Notification.SetData with a Guid); 27 Tests-ERM tests.",
        probe: "A Guid passed to the Text parameters not in GuidAsTextArgument.al (for example Text.EndsWith, Text.Split, Text.LastIndexOf, TextBuilder.Append).",
    },
    FeatureKeyEnabledModify => {
        id: "feature-key-enabled-modify",
        assumed: "Modify of a Feature Key with a changed Enabled (the only writable column) is kept for the rest of the run: later reads answer the new value, a rollback undoes it with the other writes.",
        tests: "Tests-ERM Initialize (`FeatureKey.Enabled := FeatureKey.Enabled::None; FeatureKey.Modify()` for ReminderTermsCommunicationTexts): 243 tests.",
        probe: "Get a Feature Key, change Enabled, Modify; Get again and read Enabled; FindSet and Count; the same inside a test with TestIsolation, then in the next test. Asked in corpus #648.",
    },
    FeatureKeyUnchangedModify => {
        id: "feature-key-unchanged-modify",
        assumed: "Modify on a Feature Key row that was read and not changed (Enabled set to the value it has) succeeds and changes nothing.",
        tests: "Tests-ERM Initialize (`FeatureKey.Enabled := FeatureKey.Enabled::None; FeatureKey.Modify()` for ReminderTermsCommunicationTexts): 243 tests.",
        probe: "FindFirst, Modify() with nothing changed; Enabled := Enabled then Modify; Enabled changed then Modify, then Get again and read Enabled; after a rollback. Asked in corpus #648.",
    },
    RequestPageCaptionSet => {
        id: "request-page-caption-set",
        assumed: "`RequestOptionsPage.Caption := Text` in the code of a report outside the request page's OnOpenPage (in OnInit, a control's OnValidate, a report trigger, a report extension) changes nothing a test can see. Set in OnOpenPage it is the handler's caption (pinned by report/lt-aq/AqTests.al:313-330).",
        tests: "Financial Report (`RequestOptionsPage.Caption := FinancialReport.Description` in ValidateAccSchedName, also called from OnValidate of the request page controls): about 86 Tests-ERM tests.",
        probe: "A report whose request page sets RequestOptionsPage.Caption in OnInit, in a control's OnValidate (the handler then reads TestRequestPage.Caption after SetValue), and in OnPreReport.",
    },
    FilterEmptyTextDateFormula => {
        id: "filter-empty-text-dateformula",
        assumed: "In a filter on a DateFormula field the empty text is the empty formula, so a filter of not-equal and the empty text selects the rows that have a formula.",
        tests: "Library - ERM GetDiscountPaymentTerm (PaymentTerms.SetFilter(Due Date Calculation, not-equal and the empty text)): about 170 Tests-ERM tests.",
        probe: "Payment Terms rows with and without Due Date Calculation; SetFilter on it with the empty text, not-equal and the empty text, then Count. Asked in corpus #644.",
    },
    RequestPageOnInitAtFirstUse => {
        id: "request-page-oninit-at-first-use",
        assumed: "A report variable whose procedure runs before its run has its request page's OnInit run at that first use, right after OnInitReport and before the procedure's body; the run then does not run the OnInit again, whether the page is shown or not.",
        tests: "Tests-SCM 137402 SCM Costing Batch CapacityCostPostedToGLForTwoItemsInOneProductionOrder (Library - Manufacturing RefreshProdOrder calls Refresh Production Order's InitializeRequest(CalcLines = false); the request page OnInit sets CalcLines := true). Refusing the path instead blocks 2031 Tests-SCM tests (measured 2026-10-21 on the oldest supported BC major).",
        probe: "A report with a request page whose OnInit sets a global and logs it, and a procedure that sets the same global: call the procedure, then Run with UseRequestPage(false) and with a request page handler; log the global in OnPreReport and in the handler.",
    },
    UpdateRefreshOnFindRecord => {
        id: "update-refresh-onfindrecord",
        assumed: "CurrPage.Update on a page with OnFindRecord and OnAfterGetCurrRecord asks OnFindRecord for the current row with Which `=`, then reads the row as after a move: OnAfterGetRecord, OnAfterGetCurrRecord and the parts' reads.",
        tests: "Discount Setup Notifications (T130_NotifyAboutMissingSalesSetupOnInvDiscountValidation, a Sales document page) and about 87 more Tests-ERM tests.",
        probe: "A card with OnFindRecord and OnAfterGetCurrRecord that log, an action that calls CurrPage.Update: the Which string and the trigger order; the same on a list, and after an edit not yet saved.",
    },
    UpdateRefreshListOnFindRecord => {
        id: "update-refresh-list-onfindrecord",
        assumed: "CurrPage.Update on a list with OnFindRecord (with or without OnNextRecord) reads the rows as the pinned table of 3 rows with the cursor on the middle one does (misc/W2FindRecordUpdate.al), for any number of rows and any cursor position below a window.",
        tests: "ERM Sales Subform (InvoiceAddingLinesUpdatesTotals) and about 120 more Tests-ERM tests whose sales and purchase subforms refresh.",
        probe: "The same list with 1, 2, 5 and 25 rows and the cursor on the first, last and a middle row; a list with OnFindRecord only; a list part of a card.",
    },
    ListOpenFullWindowRowsBefore => {
        id: "list-open-full-window-rows-before",
        assumed: "The open read of a list with OnFindRecord, OnNextRecord and OnAfterGetRecord but no OnAfterGetCurrRecord, whose first window of 20 rows is full, steps back with OnNextRecord(-1) and reads each row until it answers 0 (pinned with 9 rows above, for a page of its own and a ListPart: pages/lt-ai-list-order/AiListOrderTests.al:1090, :1127, misc/lt-probes-needed-6/PnF06FindListMoves.al:1947, :1968). Assumed: a step back that reaches 20 rows above stops there and asks no more (as the descending find list's step back does, PnF06FindListMoves.al RK_DescKind_FindNext_21Rows).",
        tests: "ERM Base Calendar (BaseCalendarEntriesShowCorrespondingRecords): Base Calendar Entries Subform keeps its position on the Date table in a global that OnAfterGetRecord sets.",
        probe: "The list of PnF06FindListMoves.al SF_DayList_OpensOnDayTen_Next_Previous (rows on every day 1 to 40), opened on day 30 (more than a window above): the reads at open, then Previous, First and GoToKey.",
    },
    SetValueWholeNumberIntoTextControl => {
        id: "setvalue-whole-number-into-text-control",
        assumed: "TestPage SetValue of an Integer, BigInteger or whole Decimal (below 1000 in size) on a Text or Code control enters the number's digits.",
        tests: "SCM RTAM Item Tracking-II (RegisterWhseJournalLineWithSNWarehouseTracking): WhseItemTrackingPageHandler sets Serial No. to a Decimal.",
        probe: "SetValue(14), SetValue(14.0), SetValue(1234) and SetValue(-3) on a Text[30] and a Code[30] control bound to a field and to a variable. Asked in corpus #641.",
    },
    SetTableViewKeepsSetRecordPosition => {
        id: "set-table-view-keeps-set-record-position",
        assumed: "On a Page variable, SetTableView(Rec) after SetRecord(Rec) takes the filters of the view and keeps the record SetRecord handed over as the position the page opens on, so OnOpenPage reads it as Rec.",
        tests: "VendEntry-Apply Posted Entries / CustEntry-Apply Posted Entries (SetRecord then SetTableView, then Apply Vendor Entries OnOpenPage reads Rec.\"Vendor No.\"): about 39 Tests-ERM tests.",
        probe: "A page variable with SetRecord(A) then SetTableView(B) where B has a filter and another position: the Rec OnOpenPage reads, the rows shown, and the same in the other order. Asked in corpus #641.",
    },
    RecordRefIsDirtyAgainstStoredRow => {
        id: "record-ref-is-dirty-against-stored-row",
        assumed: "RecordRef.IsDirty of a buffer that did not come from a read or write through the ref (after GetTable, Init or Clear) compares the buffer with the row stored under its primary key (a buffer with no stored row is compared with a fresh buffer). After a read or write through the ref the pinned snapshot is used and this does not apply.",
        tests: "Item Card OnAssistEdit and the other pages that ask whether the record changed: about 345 Microsoft tests (Tests-SCM, Tests-ERM).",
        probe: "IsDirty of a RecordRef right after GetTable of a Record that was read, modified without Modify, or Init: is it compared with what the Record read, or with the stored row. Asked in corpus #644.",
    },
    PageRowDeleteEventNotRaised => {
        id: "page-row-delete-event-not-raised",
        assumed: "A page whose table has a subscriber to OnDeleteRecordEvent opens; the test client has no operation that deletes the page's row, so the event is never raised.",
        tests: "Base Application subscribes on the General Journal and other journals; 88 Tests-General Journal tests.",
        probe: "Invoke the standard Delete action on a page row of a table with such a subscriber: is the subscriber run. Asked in corpus #651.",
    },
    NewRowEqualFiltersSeveralGroups => {
        id: "new-row-equal-filters-several-groups",
        assumed: "A new page row takes the value of a field that has equality filters in several filter groups when they all name the same value; different values are not stamped.",
        tests: "Payment Journal and Incom. Doc. Attach. FactBox tests of Tests-General Journal (Cash Receipt Journal).",
        probe: "Open a page on a record with SetRange(F, 'A') in group 0 and group 2, insert a row, read F. Asked in corpus #641.",
    },
    DatabaseSidNumbers => {
        id: "database-sid-numbers",
        assumed: "Database.SID answers S-1-5-21-a-b-c-rid with a fixed domain part and a stable hash of the name as the rid.",
        tests: "Library - Document Approvals.GetUser; 214 Tests-Dimension tests.",
        probe: "Nothing in AL can compare the numbers with BC's; the shape is pinned (PXDatabaseSid.al).",
    },
    SaveValuesWithExtensions => {
        id: "savevalues-with-extensions",
        assumed: "A page extension that adds only controls bound to Rec fields, labels, parts or actions changes nothing about what a SaveValues page stores.",
        tests: "Pages with SaveValues and page extensions in the buckets.",
        probe: "A SaveValues page extended by a control bound to a variable and by one bound to a field. Asked in corpus #646.",
    },
    TrappedPageLeftOpenIsReplaced => {
        id: "trapped-testpage-left-open-is-replaced",
        assumed: "A TestPage variable that trapped a page and was left open (never closed) can trap the next RunObject action of its page: the new page replaces the open one, whose close triggers do not run. (Page.Run closes the open page first, with its close triggers: pinned by pages/lt-f3-pages/F3TrappedAndModal.al:36.)",
        tests: "Library - Error Message keeps one TestPage \"Error Messages\" in a codeunit variable, traps it in every VerifyDimErrors and never closes it; Tests-Misc Check Dimensions On Posting (35 tests) trap it again in the next test of the codeunit.",
        probe: "A codeunit with a TestPage variable that Trap()s and runs a page in test one and leaves it open; test two Trap()s the same variable and runs the page again: does it open, and do OnQueryClosePage and OnClosePage of the first page run? Asked in corpus #641.",
    },
    RunModalFieldKeptUnlessLookupOk => {
        id: "runmodal-field-kept-unless-lookupok",
        assumed: "Page.RunModal(Id, Rec, Field) with a handler that answers anything but LookupOK leaves the field and the record as the handler left them; only LookupOK copies the page's value into the field (a LookupOK answer is refused).",
        tests: "Segment Line.RunCreateInteraction (PAGE.RunModal(PAGE::\"Create Interaction\", Rec, \"Interaction Template Code\")) in Tests-Marketing: CampaignAndInteractionsLinked and about 32 more, answered by Page-typed handlers that leave Response alone.",
        probe: "Page.RunModal(Page::P, Rec, Rec.F) with a ModalPageHandler that (a) invokes OK, (b) Cancel, (c) does nothing, (d) a Page-typed handler leaving Response; with P a card and a list in lookup mode, for LookupOK: what Rec.F holds and what the call returns. Asked in corpus #652.",
    },
    SubPageLinkEmptyTerm => {
        id: "subpagelink-empty-term-selects-blank",
        assumed: "An empty const('') or filter('') term of a SubPageLink or RunPageLink selects the rows whose field is blank, as the same term of a TableRelation where() does (pinned there, not for links).",
        tests: "Microsoft's Production BOM page (part ProdBOMLine: SubPageLink \"Version Code\" = const('')), opened by SCM Manufacturing 7.0 CalculateProdOrderWithSKUWithRoutingAndProdBOMNo and about 10 more Tests-SCM-Manufacturing tests.",
        probe: "A part page with SubPageLink F = const('') and another with F = filter(''), over rows with blank and non-blank F: which rows show; the same for RunPageLink. Asked in corpus #641.",
    },
    DateToDateTime => {
        id: "date-to-datetime-is-midnight",
        assumed: "A Date assigned or converted to a DateTime is that day at midnight; the blank Date is the blank DateTime.",
        tests: "Microsoft's production order and routing code (Starting Date-Time := Starting Date): Tests-SCM-Manufacturing, about 20 tests.",
        probe: "DT := D for a defined and for a blank Date; Format(DT, 0, 9); DT := ClosingDate(D). Asked in corpus #649.",
    },
    BooleanToTextAsFormat => {
        id: "boolean-to-text-as-format",
        assumed: "A Boolean given where a Text is expected (Record.Get(Boolean) on a Text key) converts as Format writes it: Yes or No. The Code case is pinned (types/lt-ax-text-format-dates/AxEvaluateText.al:348 GetTrue_CodeKey_Yes: YES).",
        tests: "Corpus baseline cases 74885 AssertEquals_TextControl_NumbersAndText and 70974 P189_FldRefMismatch_OnTextField (refusing the Text case loses them). Item Tracking Code Get(Serial) in the SCM kitting tests; about 90 tests of the Tests-SCM-Assembly bucket.",
        probe: "Get(true) and Get(false) on a table with a Text[10] key holding the rows 'Yes', 'YES', 'No' and '1'. Asked in corpus #649 (the Code key half is pinned).",
    },
    FormatCodeSpelling => {
        id: "format-code-spelling",
        assumed: "In a Format string `Filler Char` stands for `Filler Character`. (Any case of the other code names, and blanks after the comma, are pinned by types/lt-format-codes-textbuilder-pagerun-labels/P144FormatCodes.al.)",
        tests: "The Data Exchange tests' GetNodeID (`<Integer,4><Filler Char,0>`, about 30 tests).",
        probe: "Format(5, 0, '<Integer,4><Filler Char,0>') next to '<Integer,4><Filler Character,0>', for an Integer, a Decimal and a Text. Asked in corpus #649.",
    },
    ReportViewOverStoredRequestFilters => {
        id: "report-view-over-stored-request-filters",
        assumed: "A data item the caller gave a view (SetTableView, or the record of Report.Run or RunModal) keeps that view when the report has SaveValues and an earlier run stored other filters for the data item. (That an OK stores the view's filters is pinned by report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:P147_Filter_SetTableViewVsStored.)",
        tests: "Tests-ERM tests that run `Suggest Fin. Charge Memo Lines` for a second memo in one codeunit (IssueFinanceChargeMemoWith* and others).",
        probe: "A report with SaveValues: run it once with the filter A* stored by OK, then with SetTableView(C*) and see the data item's filter on the request page and in the run; the same with the record of Report.RunModal. Asked in corpus #646.",
    },
    RequestPageCancelKeepsSavedFilters => {
        id: "request-page-cancel-keeps-saved-filters",
        assumed: "A SaveValues request page closed by Cancel keeps the data item filters an earlier OK stored: a filter the handler changed before the Cancel is not stored, as a changed variable is not (pinned for variables by handlers/TestReportRequestPageSaveValues.al and misc/lt-probes-needed-6/PnF02SaveValuesExt.al; that an OK stores a data item filter is pinned by report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:P147_Filter_SetTableViewVsStored).",
        tests: "No Microsoft test: al-runner issue 6065, the reporter's run on real BC (a data item filter confirmed by OK, changed and cancelled on the next request page, read on the third).",
        probe: "A report with SaveValues and RequestFilterFields: OK with a filter A* set by the handler; next run SetFilter B* and Cancel; third run read the filter on the request page and in OnPreReport. Also with a filter cleared before the Cancel, and for a variable control next to it. Corpus writer: not yet assigned.",
        awaiting: Awaiting {
            issues: &[6065],
            files: &["tests/al-language/report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilterCancel.al"],
            since: "2026-10-28",
        },
    },
    HttpRelativeRequestPath => {
        id: "http-relative-request-path",
        assumed: "HttpClient.Get/Post/Put/Delete/Patch with a path that is not an absolute uri joins it to the client's base address like new Uri(base, path), as a [HttpClientHandler] sees it: a rooted path replaces the base path, any other joins the base path's directory, and dot segments are removed. Pinned (no assumption) for a base address ending in a slash and a plain word, a rooted word, ../ and a word, or a query alone (al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al:BaseAddress_RelativePaths_1 and _2, newest major).",
        tests: "Microsoft and NP Retail code that sets SetBaseAddress and then Get('/orders').",
        probe: "SetBaseAddress('https://example.invalid/api') (no trailing slash) then Get('orders'); SetBaseAddress('https://example.invalid/api/v1/') then Get('a/b'), Get('./x'), Get('a/../b'), Get('../../x'), Get('orders?x=1#f') with a handler printing Request.Path.",
    },
    HttpContentLengthAfterRead => {
        id: "http-content-length-after-read",
        assumed: "A Content-Length added by hand stays as it is when WriteFrom then writes a body of another length. (Pinned: one a read put there stays through WriteFrom, network/lt-ay-http/AyContentLength.al PY_ContentLength_*; one added by hand stays through ReadAs, SE_ContentLength_AddedByHandBeforeRead.)",
        tests: "Code that sets Content-Length by hand and then rewrites the content.",
        probe: "Add('Content-Length', '3') then WriteFrom('a longer text') and GetValues, and the length a request sends.",
    },
    HttpTypedHeaderValueShapes => {
        id: "http-typed-header-value-shapes",
        assumed: "A value of a typed header (other than Cache-Control) in a shape no probe measured follows the rule of the shapes measured for that header (network/lt-ay-http/AyHeaderAddResult.al, the lt-http-header-rules/P87 files and lt-se-header-values/SeHeaderValues.al measured a few valid and invalid values of each); a name of the other kind of message the probes did not add is stored as written there, and one the probes did not add to the client's default headers checks there as on request headers.",
        tests: "Every Microsoft or NP Retail test that builds a request with headers (Authorization, Accept, Content-Type, custom X- headers).",
        probe: "More value shapes per typed header on request, response, content and client headers (lists, parameters, spacing, case, two-digit years, negative offsets), and the request-only names Accept-Charset, Accept-Encoding, Accept-Language, From, If-Modified-Since, If-None-Match, If-Range, If-Unmodified-Since, Proxy-Authorization and TE on response headers. Split from http-header-rules (2026-10-25).",
    },
    OpenNewSkipsOnFindRecord => {
        id: "open-new-skips-onfindrecord",
        assumed: "TestPage.OpenNew on a page with an OnFindRecord trigger does not run that trigger: the page finds its last row for xRec by itself (as for a page without the trigger) and goes to OnNewRecord.",
        tests: "Service Credit Memo, Service Invoice and Service Order pages (OnFindRecord: Rec.Find(Which), else SetRange(\"No.\") and Find again): ServiceCreditMemoByPage, ServiceInvoiceByPage and about 30 more Tests-SCM-Service tests, and the 24 that follow them in their codeunits.",
        probe: "A list or card with OnFindRecord(Which) that notes Which in a trace, a source table with and without rows: OpenNew, then the trace, xRec of OnNewRecord and the first Rec. Asked in corpus #641.",
    },
    NewLineStampsAllBeforeValidating => {
        id: "new-line-stamps-all-before-validating",
        assumed: "The blank line of a part (or list) that takes two or more single filter values of its key fields (a key field from SourceTableView and the SubPageLink fields) puts them all on the record first, and only then validates each, in field order, when a validation can see the others (an OnValidate, a table relation, a validate trigger of a table extension or a subscriber). (One validated value is pinned, and a SourceTableView field outside the key is not stamped: pages/lt-new-line-stamps-filters/NewLineStampsFilters.al:214-312. Two key fields that the SubPageLink gives, validated by OnValidate and a table relation, in key order, are pinned and not marked: NewLineStampsFilters.al QP_TwoKeyLink_*. Without anything that can see the order, no mark.)",
        tests: "Library - Service.CreateServiceOrderHeaderUsingPage (Service Order Subform: key Document Type, Document No., Line No.; the relation of Document No. filters on Document Type). Kept because refusing it costs 7 corpus baseline cases (handlers/TestPageActionRunPageView.al 67007, misc/lt-probes-needed-7/PnGRefresh.al 70177).",
        probe: "A part whose source table's key is (Type, Document No., Line No.), SourceTableView where(Type = const(B)), SubPageLink Document No. = field(No.), the relation of Document No. filtering on Type and an OnValidate of Document No. that reads Type: open the host on a header of Type B and A, New(), read the draft line and what the OnValidate saw.",
    },
    GroupedQueryTiesInGroupingOrder => {
        id: "grouped-query-ties-in-grouping-order",
        assumed: "Groups of an aggregated query that tie on every OrderBy column are read in the order of the query's grouping columns (column declaration order, each ascending).",
        tests: "Create Pick's CalcQtyInWhseEntries (OrderBy Block_Movement, ten grouping columns): SCM Extend Warehouse CheckItemAvailabilityWhenCreatingPickAndExistReservedQtyForProdOrder and the tests that create a pick for a directed put-away location; about 77 Tests-SCM tests.",
        probe: "An aggregated query with an OrderBy on a column most groups share and several grouping columns, rows inserted in non-key order: the order of the tied groups on Read. Asked in corpus #651.",
        note: "Confirmed by writer SG on corpus bump PR 738 (branch al-runtime-rs-staging-pub-sg 37b221b7, green on every supported BC major): query/lt-az-query/AzQueryTests.al SG_OrderByTies_GroupingSubThenCat, _GroupingCatThenSub, _DescendingKind, _DescendingSum. No al-runner issue, so no `awaiting` field: remove this entry when that bump lands.",
    },
    TableRelationSeveralWhereNonKey => {
        id: "table-relation-several-where-non-key",
        assumed: "A TableRelation check that fails, with several where() conditions none of which is on a primary key field of the related table, raises DB:NothingInsideFilter, as one condition on a non-key field does. (A mix with key fields stays refused.)",
        tests: "Item Blocked Test and Item Variant Blocked Test (Item where(Blocked = const(false), \"Service Blocked\" = const(false)) and the sales and purchase variants), SCM Dedicated Bins; about 26 Tests-SCM tests. They compare the message only.",
        probe: "A relation with two non-key where() conditions and a value that fails one of them, and one that is missing: GetLastErrorCode; the same with one key and one non-key condition.",
    },
    TestFilterSetFilterEmptyClears => {
        id: "testfilter-setfilter-empty-clears",
        assumed: "TestFilter.SetFilter(Field, '') (a text empty or of spaces) on a page or request page filter removes the user's filter on that field, as FieldRef.SetFilter does (pinned: P134DateTextFilters.al:P134_FieldRef_Date_EmptyAndBlankForms).",
        tests: "SCM Inventory Misc. II (Inventory Pick and Put-away \"WithAdditionalFilters\"), SCM Invt Item Tracking III, SCM Plan-Req. Wksht, SCM Inventory Journals, SCM Inventory Miscellaneous: about 17 Tests-SCM tests that clear a request page filter with an empty text.",
        probe: "TestPage.Filter.SetFilter(F, '') and (F, ' ') after a filter on F: GetFilter(F) and the rows; the same on a request page's data item filter. Asked in corpus #641.",
    },
    PageRunRecordViewOrderApplies => {
        id: "page-run-record-view-order-applies",
        assumed: "Page.Run with a record on a page whose SourceTableView names an ORDER(ascending|descending): the page runs in the view's direction, as the view alone does. (P145PageRunView.al pins the view's SORTING and the record's direction when the view names none.)",
        tests: "Pages with SourceTableView = sorting(...) order(descending), such as Item Ledger Entries, opened with a record: SCM Item Journal and SCM Warehouse tests; about 11 Tests-SCM tests.",
        probe: "Page.Run(Page, Rec) with Rec ascending and the page's view order(descending), and the reverse: the first row the page shows. Asked in corpus #641.",
    },
    ReportRenderDiscarded => {
        id: "report-render-discarded",
        assumed: "A rendering save (SaveAsPdf, SaveAsExcel, SaveAsWord, SaveAsHtml, SaveAs to Pdf, Excel, Word or Html) of a report on the allow-list (ADR-040) runs the report as the Xml path does, answers true and writes no output; AL that looks at the output is refused.",
        tests: "Post Inventory Cost to G/L through Library - Costing and Library - Assembly PostInvtCostToGL (SCM Kitting D3 Part 1 and 2, SCM WIP Costing Production I and II, SCM Costing Batch); Check.SaveAsPdf (ERM Cash Manager, Match Bank Rec. Scenarios).",
        probe: "A report on the list saved to a temp file and to a stream on a Windows tier: the answer, the order of the triggers, File.Exists and TempBlob.Length afterwards.",
    },
    BitmapPictureNotDrawn => {
        id: "bitmap-picture-not-drawn",
        assumed: "O365 Brand Color.MakePicture (new Bitmap(100, 100), Graphics.FromImage, ColorTranslator.FromHtml, new SolidBrush, FillEllipse, Graphics.Dispose, Bitmap.Save(InStream, ImageFormat.Png), Bitmap.Dispose) runs without error on real BC, draws nothing here, and Media.ImportStream of the saved stream gives a field with a value (HasValue true); AL that looks at the picture (stream read, length, export, Tenant Media row) is refused (ADR-044).",
        tests: "O365BrandColorTests.Codeunit.al:74 and :158 (Sample Picture.HasValue after MakePicture), and every Contoso Coffee Demo Dataset step that reaches Create Company Information.CreateSetupCompany (O365 Brand Color.CreateDefaultBrandColors).",
        probe: "On a Windows tier: new Bitmap(100, 100), FillEllipse, Save(InStream of a Temp Blob, ImageFormat.Png), Media.ImportStream: HasValue, then ExportStream length and its first 8 bytes (PNG signature 89 50 4E 47 0D 0A 1A 0A); the PNG header width and height (expect 100 by 100) and the pixel at the center versus a corner; Save with Jpeg and Bmp.",
    },
    ReportProcedureFilterUnderRequestPage => {
        id: "report-procedure-filter-under-request-page",
        assumed: "A filter a procedure of the report set on a data item before a run with a request page is not in that run: the page does not show it (pinned) and the filters of the page replace it when the page closes with OK (assumed).",
        tests: "Microsoft tests that call a report procedure which filters a data item and then RunModal the report with its request page.",
        probe: "A report procedure RowDi.SetFilter(Code, 'B*'), RunModal with a handler that clicks OK and a report that counts the rows in OnPreDataItem and OnAfterGetRecord: are the B rows alone read? Asked in corpus #646.",
    },
    ExceptionToStringStackFrames => {
        id: "exception-tostring-stack-frames",
        assumed: "ToString and Format of the exception GetLastErrorObject answers, for a trace the corpus does not pin (an error caught by a TryFunction or asserterror, a record-not-found or division error, a dialog error raised deeper than one procedure below a Codeunit.Run's OnRun): the lines after the first are the AL frames of the error. BC writes .NET frames there (pinned shapes: GetLastErrorObjectKinds.al:384, AzLastErrorObject.al:180, :199), so a read of those lines would differ; only the first line and the `   at ` marker are pinned (GetLastErrorObject.al:85-87). Kept for the four baseline cases that read only those.",
        tests: "Corpus baseline cases 61471 GetLastErrorObject_AfterErrorInTryFunction, _AfterErrorInAsserterror, _AfterGetWithoutRow, _AfterDivisionByZero (onprem-dotnet suite; refusing them loses them). Microsoft tests that read ToString or Format of a last-error exception.",
        probe: "Format(GetLastErrorObject()) split into lines after Error('x') caught by a TryFunction, by asserterror, after Get of a missing row and after a division by zero, each on every supported BC major: every line. Asked in corpus #652.",
    },
    PageOpenCaptionClassExpressions => {
        id: "page-open-caption-class-expressions",
        assumed: "A page whose source table has fields with a CaptionClass expression runs every one of them with an empty record when it opens, then once for each control over such a field with the first row; the other rows the open reads and a Next() run none, and Caption() of the control reads what the open gave. Pinned (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al Page_ControlCaptions, QF_*, and corpus bump 45: QO_Doc_OpenEdit_Open, QO_Doc_OpenEdit_PartSteps, QO_Doc_OpenNew, QO_Card_OpenEdit, :958-1137) for a Card opened with OpenView or OpenEdit, a List opened with OpenView, OpenEdit or Page.Run, a Document page opened with OpenEdit or OpenNew, a ListPart on such a Card or Document page, a SetValue (runs nothing), a part refilled after its host moved (its first row only, nothing when it has no row), and Next() after a change on a Card or a ListPart (once with the saved row), and corpus bump 46 matched: an action with CurrPage.Update on a Document page after a SetValue of the host runs the host's three times, then the part's once (QO_Doc_OpenEdit_HostSteps `update`), and the SI_ tests (:1463-1960): GoToRecord and GoToKey on a Card or a Document page (OpenView or OpenEdit) run them once more with the row the captions came from and Caption() keeps its value, after a SetValue the save runs them once with the saved row first; First and Last on a Card run them with the new row and Previous runs none. Assumed: other page types and open modes, other fills after the open, a GoToRecord or GoToKey that finds no row, an action after a SetValue whose trigger has no CurrPage.Update (the part's are not run again), and a refresh after an action on a page without a SetValue before it.",
        tests: "Microsoft tests that open sales and purchase documents and their subforms (Sales Line, Purchase Line \"No.\" and amount fields have CaptionClass = GetCaptionClass(..)), for example ERM Sales Order and ERM Purchase Order tests in Tests-ERM; they pass in Microsoft's CI.",
        probe: "CurrPage.Update on a Document page without a SetValue before it, and an action without CurrPage.Update after a SetValue: how many times the host's and the part's expressions run, and in which order.",
    },
    PageOpenCaptionClassEmptyRecordCount => {
        id: "page-open-caption-class-empty-record-count",
        assumed: "A page open that runs the CaptionClass expressions of its source table's fields with an empty record, in a shape writer SX did not pin (another page type or open mode, a part, a modal open, three or more expression fields, two with different expressions), runs each one 20 times. BC gives no stable number for these: the corpus saw 240, 480 and 492 calls on the newest major for the same 12-field Card open (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:657) and asserts only that such runs happen and give the same count per field and per open (:1227 SF_Count_CardOpenViewFiveTimes). Narrowed: the number matters only when an expression leaves something behind, so such a page open whose expression can leave anything behind (a database write, a dialog or error, a .NET or interface call, a write to a variable that outlives the call, also through an event subscriber) is refused, decided from the code before the expressions run (al-vm effects.rs). Kept, not pinned: an expression whose only lasting effect is a write to a variable of a SingleInstance codeunit still runs 20 times (refusing it costs 33 baseline cases of codeunit 86504, which count the runs in a SingleInstance log on a 12-field table, and Base Application's Sales Line CaptionClass Mgmt keeps the document header in one).",
        tests: "The corpus cases of QdCaptionClassAreas.al that open a page whose source table has CaptionClass expressions (they assert only that empty-record runs happen, the same number per field and per open), and Microsoft tests that open sales and purchase documents and their subforms (Sales Line and Purchase Line fields with CaptionClass = GetCaptionClass(..)) in Tests-ERM; they pass in Microsoft's CI.",
        probe: "The count for the shapes not pinned yet, if BC gives a stable one: a Document page and its part, OpenNew, Page.Run and a modal open, and tables with three and more expression fields and with two different expressions (the order of the runs of each field).",
    },
    WorksheetTempSourceOpenRow => {
        id: "worksheet-temp-source-open-row",
        assumed: "Which row a Worksheet page over a temporary source opens on, when it is opened without a record and its OnOpenPage moved Rec or changed its key, order or filters, leaving it on a row other than the first of its view. Two paths, both assumed: a Rec in the primary key, outside lookup mode and without OnNextRecord, from which `=><` lands on a row other than the first, opens on that row and reads its rows as a List does; every other such Worksheet (a key changed, lookup mode, OnNextRecord, or `=><` landing on the first row) opens on its first row. Pinned, and no longer assumed: a row a procedure of the page variable set before the open, which OnOpenPage left as it was (pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:2378-2705, the QT_Temp* tests); the List and the Card whose OnOpenPage fills them (:409 QR_TempCard_OpenView_NoMove, :474 QR_TempList_OpenView_NoMove).",
        tests: "The first-row path is reached by Item Tracking Lines (a temporary Worksheet whose OnOpenPage changes the key) in many tests of Tests-SCM-Reservation, Tests-SCM-Warehouse, Tests-SCM and Tests-ERM; they pass in Microsoft's CI.",
        probe: "A Worksheet page over a temporary table whose OnOpenPage fills it or moves Rec (Get of row 2, FindLast), and one whose OnOpenPage calls SetCurrentKey and then Get: run modally, in lookup mode and with Page.Run, with and without OnNextRecord: which row the TestPage shows, and the OnAfterGetRecord/OnAfterGetCurrRecord order.",
    },
    PageUnwrittenBlobNoChangeOtherPage => {
        id: "page-unwritten-blob-no-change-other-page",
        assumed: "On a page whose ModifyAllowed is false, a Blob of Rec that no stream wrote since it was read (also after a CalcFields loaded it) is no change of the row, so the page saves nothing after an action or a SetValue: pinned for a List (pages/lt-outside-worksheet/RmReadOnlyBlob.al:316-:470); assumed for any other page type.",
        tests: "Tests-ERM Edit Posted Documents (codeunit 134658) PostedServiceInvoiceUpdateSetValuesOK and PostedServiceInvoiceUpdateSetValuesCancel (Posted Serv. Inv. - Update); they pass in Microsoft's CI.",
        probe: "RmReadOnlyBlob.al's table under a Card with ModifyAllowed = false (Editable true and false): a no-change action and a SetValue on another control, then Close: the log of OnModifyRecord and the table's OnModify.",
    },
    RunnerPermissionSetsGrantAll => {
        id: "runner-permission-sets-grant-all",
        assumed: "The permission sets BC's test runner assigns before each test method whose TestPermissions is not Disabled (codeunit 130302 \"Set State Test Run\": All Objects, D365 Full Access, TEST TABLES, D365 BASIC, LOCAL through Permissions Mock) grant every access and every permission answer the test then makes, as the unrestricted session does here (permissions are not enforced, ADR-027 owner decision 2). Known wrong in general: on BC they grant what D365 Full Access, D365 Basic and LOCAL name, plus Execute on every object; a test app table no set names gives ReadPermission false and DB:ClientReadDenied, Tenant Media has only indirect write, and TEST TABLES does not exist without Tests-TestLibraries (permissions/lt-qc-runner-sets/QcRunnerSets.al:1-24, :174-291).",
        tests: "Every Microsoft test method of a codeunit without TestPermissions = Disabled when Tests-TestLibraries is loaded (about 6500 tests of the Base Application buckets in the 2026-10-20 measurement); they pass in Microsoft's CI under these sets.",
        probe: "Done for the corpus database (QcRunnerSets.al). Still wanted: the same reads and writes with Tests-TestLibraries installed, so TEST TABLES exists: a table of a Microsoft test app and of a test library under the runner sets.",
    },
    HostSaveRecordRefreshesLinkedParts => {
        id: "host-save-record-refreshes-linked-parts",
        assumed: "A SetValue on a control of a host page that is not a Document page (a Card), or a Document page with a part hidden by a Visible expression, whose OnValidate calls CurrPage.SaveRecord() (and no CurrPage.Update), makes the host's parts that have a SubPageLink read their rows again from their first rows, as after CurrPage.Update on the host (testpart/TestPartTriggerOrderAndRefresh.al HostCurrPageUpdate_ReReadsThePart), without the host reading its own row again. On a Document host testpart/QiHostSaveParts.al pins it: every part reads again where it stands, the unlinked ones too.",
        tests: "Corpus baseline case 60412 NewHostRowSavedByItsOwnTrigger_IsInsertedOnce (a Card host); Tests-ERM ERM Purchase Subform (134394) and Document Totals Pages: Purchase documents have parts hidden by a Visible expression; they pass in Microsoft's CI.",
        probe: "The QiHostSaveParts.al probe (testpart/QiHostSaveParts.al QI_HostSaveRecord_*) on a Card host with a linked ListPart and an unlinked one: which part triggers run, in which order, with which Rec and xRec.",
    },
    PartRowSavedBeforeHostSetValue => {
        id: "part-row-saved-before-host-set-value",
        assumed: "A SetValue on a control of a host page that is not a Document page, while a part holds a typed row that is not saved (a new row or an edited one), saves that row first (Insert or Modify with its triggers), as on a Document host (testpart/QiHostSaveParts.al:500 QI_PartNewRow_ThenHostSetValue, :554 QI_PartEditedRow_ThenHostSetValue); then the host field validates.",
        tests: "Microsoft tests that type a line into a subform of a host other than a Document page, then set a host field; and the corpus cases that do the same on a Card host.",
        probe: "The QiHostSaveParts.al probe (QI_PartNewRow_ThenHostSetValue, QI_PartEditedRow_ThenHostSetValue) on a Card host.",
    },
    MediaImportFilePng => {
        id: "media-importfile-png",
        assumed: "Media.ImportFile of a `.png` file that holds a PNG imports it as Media.ImportStream does a PNG (`image/png`, the picture's size) and its Tenant Media File Name is the path as given, as the corpus pins for a `.jpg` file holding a JPEG (onprem media/MediaFiles.al:293-340). For an upper-case `.PNG` file the corpus pins the Mime Type, the size and that the File Name contains the file's name (MediaFiles.al:394-447), not the File Name itself; a lower-case `.png` into a MediaSet is assumed to import as a `.PNG` does.",
        tests: "Every test of Microsoft's Legacy test lane (for example Tests-SCM-Warehouse codeunit 137015): their company is built by the DemoTool, whose Create Getting Started Data imports the WalkMeTour `.png` files through Media Resources Mgt.InsertMediaFromFile (Base App MediaResourcesMgt.Codeunit.al:34), and they pass in Microsoft's CI.",
        probe: "Media.ImportFile and MediaSet.ImportFile of a lower-case `.png` file holding a PNG (and a `.jpeg` file): Tenant Media Mime Type, Width, Height; the exact File Name of a `.PNG` Media import on Windows BC.",
    },
    TernaryLongerTextFalseBranchFits => {
        id: "ternary-longer-text-false-branch-fits",
        assumed: "A conditional expression whose true branch is a Text[n] and whose false branch is a longer Text or Code, outside an assignment to a target of exactly Text[n] (pinned: types/lt-qx-ternary-sized/QxTernarySized.al:142-155), yields the false branch's value unchanged when it is no longer than n. A longer value is refused when the branch is taken.",
        tests: "Every test of Microsoft's Legacy test lane (for example Tests-SCM-Warehouse codeunit 137015): their company is built by the DemoTool, which inserts Acc. Schedule Lines whose OnInsert runs `Description := Description = '' ? GLAccCategory.Description : Description` (Base App AccScheduleLine.Table.al:113, a Text[80] true branch and a Text[100] false branch into a Text[100] field), and they pass in Microsoft's CI.",
        probe: "`T100 := Cond ? T80 : T100` and `T50 := Cond ? T80 : T100` with a false-branch value of 10, 80 and 90 characters: the value read back or the error (text and code); the same with a Code[100] false branch.",
    },
    TernaryLabelBranch => {
        id: "ternary-label-branch",
        assumed: "A conditional expression with a Label branch against a Code[N] or Text[N] branch, or a non-empty text literal true branch against a Label false branch: the true branch decides the result type. A Code[N] true branch makes the Label false branch a Code (upper case; StringLengthExceeded past N, with the label's own text in the message, even when the result goes into an unbounded Text); a Text[N] true branch limits the Label to N; a Label or literal true branch is a Text that takes the false branch as it is. A TextConst or MaxLength label in these pairs is refused. Pinned only by corpus writer TI's run on real BC (Code[20] and Text[10] against Labels of 7, 10, 15, 20 and 30 characters, each way).",
        tests: "No Microsoft test: al-runner issue 6047 (a partner suite's `Validate(Field, Cond ? Rec.\"Code\" : DefaultCodeTok)`, a Code[20] true branch and a Locked Label false branch, 38 tests).",
        probe: "Done on writer TI's branch: the TI_Ternary_* tests of types/TestTernaryContracts.al; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[6047],
            files: &[
                "tests/al-language/types/TestTernaryContracts.al::TI_Ternary_CodeTrueLabelFalse_1",
                "tests/al-language/types/TestTernaryContracts.al::TI_Ternary_LabelTrueCodeFalse_1",
                "tests/al-language/types/TestTernaryContracts.al::TI_Ternary_TextNLabel_1",
                "tests/al-language/types/TestTernaryContracts.al::TI_Ternary_LabelLabelAndConst_1",
                "tests/al-language/types/TestTernaryContracts.al::TI_Ternary_CodeLabelOverflowTexts",
            ],
            since: "2026-10-28",
        },
    },
    ResourceFileNameCase => {
        id: "resource-file-name-case",
        assumed: "A File.Open, File.Exists or read of a resource file of the company data (`--company-data demotool`) whose name differs from the file on disk only in case finds the file, as on the Windows server BCApps' CI builds the DemoTool company on (the DemoTool opens `CurrencyData.txt`, BCApps ships `CurrencyData.TXT`). A name that matches exactly does not take this path.",
        tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: their company is built by the DemoTool (Create Demonstration Data.OnRun opens `CurrencyData.txt`), and they pass in Microsoft's CI.",
        probe: "Not measurable from the corpus: the case rule is the server's file system, and the corpus CI runs BC on Linux. On the Windows nightly: File.Exists and File.Open of a file in the server's folder by a name in other case.",
    },
    CompanyBuildCarriedContentNotEmpty => {
        id: "company-build-carried-content-not-empty",
        assumed: "In the company build, content carried unread (owner decision 2026-10-23 \"Carry bytes unread\": a file MediaSet.ExportFile wrote for a re-encoded item, a .docx Media.ExportStream re-zips) is never empty: Blob.HasValue answers Yes and Format of the Blob shows `*`; for System Application's Base64 Convert Impl. only, InStream.EOS answers No and InStream.Length answers 1 (its two tests, empty and more than 10 MB, come out as for the real length on every platform). Every other read of the bytes, length or text is refused.",
        tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: the DemoTool's Export RapidStart Packages step exports the Config. Media Buffer rows (ConfigXMLExchange.Codeunit.al:1429, ConvertBlobToBase64String; Temp Blob.FromFieldRef asks HasValue), and they pass in Microsoft's CI.",
        probe: "Not needed for the values (a re-encoded picture and a re-zipped package are not empty on any platform: 99, 103, 108 and 919 bytes measured); a corpus test that a Media.ExportStream of an imported PNG into a Blob gives HasValue Yes and Format `*` would pin it.",
    },
    MediaExportStreamJfif300DpiKept => {
        id: "media-exportstream-jfif-dpi-jpeg-kept",
        assumed: "From the newest major on (the same major where MediaSet keeps a JPEG), Media.ExportStream of a JPEG that Media.ImportFile read, whose JFIF header has units 1 (dots per inch) and density 300 x 300, writes the file's bytes unchanged. Measured on Windows BC of the newest major (probe cu 61288 SC_DemoTool_Media_ExportStream, case JD: export = file, Tenant Media Content = file). Other JPEG headers are rewritten there (units 0: three header bytes; no JFIF: a JFIF segment inserted) and stay refused.",
        tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: the DemoTool's Export RapidStart Packages step exports the Person, Contact, Customer, Vendor, Employee and Salesperson pictures (Config. XML Exchange.ExportMedia, ConfigXMLExchange.Codeunit.al:1486), all 300 dpi JFIF JPEGs, and they pass in Microsoft's CI.",
        probe: "A corpus test of Media.ImportFile then Media.ExportStream of a 300 dpi JFIF JPEG on the newest major (and the two older majors), asserting the exported bytes equal the file.",
    },
    GZipStreamCloseIsDispose => {
        id: "gzipstream-close-is-dispose",
        assumed: "DotNet System.IO.Compression.GZipStream.Close of a compressing stream does what Dispose does: it writes the compressed bytes to the wrapped stream. Measured on Windows BC of the newest major (probe cu 61289 SC_GZipStream_Close_ConfigPackageShape: no error, the file starts 1F 8B 08 00, Data Compression.GZipDecompress gives the source back).",
        tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: the DemoTool's Export RapidStart Packages step compresses each package (Config. Pckg. Compression Mgt.ProcessGZip, ConfigPckgCompressionMgt.Codeunit.al:80), and they pass in Microsoft's CI.",
        probe: "A corpus test of GZipStream(OutStream, Compress), CopyStream, Close on the newest major, then GZipDecompress of the result.",
    },
    CompanyBuildFolderListingNameOrder => {
        id: "company-build-folder-listing-name-order",
        assumed: "In the company build (`--company-data demotool`), DotNet System.IO.Directory.GetFiles (File Management.GetServerDirectoryFilesList) of a folder AL code created in the temporary folder lists the files written directly in it, in name order without case (pinned on every major since corpus bump 62: onprem media/MediaFiles.al RJ_GetServerDirectoryFilesList_ListingOrder), each joined to the folder with `/`. Real BC joins with a backslash (MediaFiles.al RJ_MediaSet_ExportFile_ConfigPackageShape `<media>\\<set1>-1`, also for a folder named with '/'); the runtime keeps `/` because the runtime's .NET System.IO.Path runs on Linux, where a backslash is no separator, and File Management takes the file name apart with it. An empty folder stays refused.",
        tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: the DemoTool's Export RapidStart Packages step reads the Media folder back (Config. XML Exchange.UpdateConfigPackageMediaSet, ConfigXMLExchange.Codeunit.al:1505-1522), and they pass in Microsoft's CI.",
        probe: "None for the order (pinned). The separator needs no probe but a runtime change: System.IO.Path members that split a path must treat a backslash as a separator, as Windows .NET does; then the listing can join with a backslash.",
    },
    OptionInitValueOrdinal => {
        id: "option-init-value-ordinal",
        assumed: "An Option field whose InitValue is a number that names no member (`InitValue = 1`) starts at the member of that ordinal: on a fresh variable, after Init and after Insert. Measured on Windows BC of the newest major (corpus branch winprobe-wc6, tests/wc6-probe/ContosoProbe.al, cu 61320 CS_OptionInitValueOrdinal: Sunday..Saturday with InitValue = 1 gives Monday/1 three times). A number that is also the name of a member at another ordinal stays refused.",
        tests: "The Contoso company build of Microsoft's IntegrationTest and Uncategorized lanes for apps that pull in Power BI Report embeddings: its Accounting Period table extension reads PowerBI Reports Setup (\"First Day Of Week\", InitValue = 1) from Create Accounting Period, and the build runs in Microsoft's CI.",
        probe: "A corpus test of the measured case (fresh variable, Init, Insert and Get) on all three majors; the same with InitValue = 0 and with a number past the last member.",
    },
    FieldObsoleteStatePendingMove => {
        id: "field-obsolete-state-pending-move",
        assumed: "The Field row of a field with ObsoleteState = PendingMove exists and its ObsoleteState is ordinal 3 (Format prints `3`; the members of Field.ObsoleteState are No,Pending,Removed), so a filter `<> Removed` keeps it; ObsoleteReason is kept. Measured on Windows BC of the newest major (corpus branch winprobe-wc6, tests/wc6-probe/ContosoProbe.al, cu 61320 CS_FieldRowPendingMove).",
        tests: "The Contoso company build of Microsoft's IntegrationTest and Uncategorized lanes for apps that pull in Power BI Report embeddings: Dimension Management.FillNormalFieldBuffer scans the Field table with `ObsoleteState <> Removed` (Default Dimension.OnValidate), and PowerBI Reports Setup.\"Subs. Billing Report Name\" is PendingMove; the build runs in Microsoft's CI.",
        probe: "A corpus test of the measured case on all three majors: Get of the Field row, Format of ObsoleteState with formats 0 and 2, a filter `<> Removed` and `= Pending`.",
    },
    ContosoBuildNoClient => {
        id: "contoso-build-no-client",
        assumed: "While the Contoso company is built (`--recipe microsoft`, IntegrationTest and Uncategorized lanes; Company-Initialize and every Contoso step), GuiAllowed answers false: BCApps' CI runs each step through Invoke-NavContainerCodeunit (build/scripts/ImportTestDataInBcContainer.ps1 of releases/29.0, Invoke-ContosoDemoTool :307-326), a session without a client (owner decision 2026-10-23). The tests themselves keep GuiAllowed true. Message is captured and Confirm refused as in any company build.",
        tests: "Every test of Microsoft's IntegrationTest and Uncategorized lanes: their company is built by the Contoso Demo Tool, and with a client the all-data build stops at Sales Header.ConfirmKeepExistingDimensions (SalesHeader.Table.al:5163, guarded by GuiAllowed); they pass in Microsoft's CI.",
        probe: "A recipe fact, not a language rule: GuiAllowed in a session started by Invoke-NAVCodeunit on the Windows nightly. The corpus pins GuiAllowed false only for a page background task (bgtasks/lt-bj-no-gui/BjNoGuiDialogs.al:8-9).",
    },
    StaleBufferOtherOriginWritten => {
        id: "stale-buffer-other-origin-written",
        assumed: "A Modify or Delete through a buffer that is older than the last Commit's row, whose row version came neither from a Get nor from the variable's own Insert (a Find, an assignment, TransferFields), is written when the open transaction already wrote the row: the last writer wins. (A buffer read by Get is refused there, record/TestRecordStaleBufferWrite.al:604-660; one from the Insert of the transaction before is written, :495-584.)",
        tests: "No. Series - Batch Impl.UpdateNoSeriesLine (Get, TransferFields from the batch's temporary line, Modify(true)) in Gen. Jnl.-Post Batch; 110 Tests-ERM tests (for example Payment Export Validation UT CustLedgEntryBankAccountMissingFormat).",
        probe: "A buffer read by FindFirst, by assignment from another variable and by TransferFields from a temporary copy, made stale by a Commit, then the row written in the new transaction (changed and unchanged Modify), then a Modify through the buffer: accepted or DB:RecordChanged, and the stored row.",
    },
    RunObjectTargetViewWithoutLink => {
        id: "run-object-target-view-without-link",
        assumed: "A List reached by an action's RunObject with RunPageOnRec, or with RunPageView and no RunPageLink, whose page declares SourceTableView, combines the two as RunPageLink does (pages/lt-rz-outside-page-moves/RzRunPageLinkView.al): the view's filters in group 2, the action's RunPageView where filter in group 3, the target's SORTING and ORDER kept; it lists the rows of both and opens on the first. RunPageLink beside the view is pinned by that file.",
        tests: "No Microsoft test: the reporter shape with RunPageLink is pinned; RunPageOnRec and RunPageView alone keep the same rule.",
        probe: "RunPageOnRec, and RunPageView without RunPageLink, to a List with SourceTableView = sorting(...) order(descending), = where(...), and both: the open row, the rows from First/Next and Filter.GetFilter of the view's field.",
    },
    FormatWeekCodeIso => {
        id: "format-week-code-iso",
        assumed: "`<Week>` and `<Week,2>` of a Date give the ISO 8601 week number (weeks start on Monday, week 1 holds the year's first Thursday) for every week, and `<Weekday>` the ISO day number, Monday 1 to Sunday 7. Pinned are the weeks 1, 3, 10, 19, 28, 40, 52 and 53 at year boundaries and mid-year (types/lt-format-week-code/FormatWeekCodeIso.al:21-123, types/lt-datetime/DateFuncs.al:415-427) and the days 3 and 7 (DateFuncs.al:415,425; FormatWeekCodeIso.al:121,128); a Format of another week or day is a flagged pass.",
        tests: "No Microsoft test: the rule generalizes the pinned weeks (review of al-runner PR 5864).",
        probe: "`<Week>`, `<Week,2>` and `<Weekday>` of one date in every ISO week of 2021 (53 dates) and of the seven days of one week.",
    },
    TestCodeunitOnRunRows => {
        id: "test-codeunit-onrun-rows",
        assumed: "How the runner reports the OnRun trigger of a test codeunit, on every major (ADR-055): as Microsoft's Test Runner app (page 130455, runner codeunit 130450, the way BcContainerHelper and AL-Go run tests) does. No row 'OnRun'; a codeunit whose OnRun raises is Skipped with no rows and the run stays green; the tests of a codeunit whose OnRun does not raise run after it. Measured on the newest major by the reporter of al-runner issue 5859 and on the oldest supported major by corpus writer RL (probe commit c69419f6, codeunit/lt-outside-onrun-error/RlOnRunError.al); the middle major through that runner is not measured. The tests of a codeunit with a declared OnRun are flagged.",
        tests: "No Microsoft test: al-runner issue 5859 (a user's run through BcContainerHelper on the newest major; owner decision 2026-10-28, ADR-055).",
        probe: "Through Microsoft's Test Runner app on each supported major: a test codeunit whose OnRun raises, one whose OnRun does not, and one whose OnRun calls a test method; the rows and outcomes the runner reports.",
        awaiting: Awaiting {
            issues: &[5859],
            files: &["tests/al-language/codeunit/lt-sw-onrun-runner/SwOnRunRunner.al"],
            since: "2026-10-28",
        },
    },



    BigTextValueCopy => {
        id: "bigtext-value-copy",
        assumed: "A BigText given as an event parameter by value gets a copy the subscriber's AddText does not reach, and a returned BigText is a copy. Assignment, a by-value and a var procedure parameter and Clear of a copy are pinned (text/TestBigText.al:108-160); the marker cannot tell those from the unpinned uses and fires on every copy.",
        tests: "No Microsoft test: a partner app's tests (A=2;B=1;C=0 after `B := A`, a by-value AddText and Clear of a copy).",
        probe: "A BigText passed by value to an event subscriber that calls AddText: the publisher's length; a procedure that returns a BigText the caller then grows: the callee's copy.",
    },
    GroupEnabledDisablesField => {
        id: "group-enabled-disables-field",
        assumed: "A control inside a group whose Enabled reads No is disabled whatever its own Enabled says on a page type other than a card or a request page, and for a group Enabled bound to an expression. Pinned: a request page (misc/lt-object-properties-runtime-effect/ReportRequestPageGroupEditable.al:P194_ReportGroup_EditableEnabled) and a card's field with or without an Enabled of its own in a group with `Enabled = false` (pages/lt-ap-testpage-642/ApTestPageTests.al:895 Ap16_GroupEnabledFalse_Card, :1082 Ap16_GroupEnabledFalse_FieldEnabledTrue).",
        tests: "No Microsoft test: a partner app's card test (a field with Enabled = true inside a group with Enabled = false reads Enabled No on the TestPage).",
        probe: "A list and a document page with a field (Enabled = true, and without Enabled) in a group with Enabled = false, and with the group's Enabled bound to a variable that OnOpenPage sets: Enabled() of the field on the TestPage.",
    },
    RelationErrorExtCaption => {
        id: "relation-error-ext-caption",
        assumed: "The TableRelation error of a Validate (`... cannot be found in the related table (<caption>).`) names the related table by the Caption of the one tableextension that sets a Caption over the table's own Caption. A table without a Caption of its own is pinned (tableextension/TestTableExtFieldTableRelation.al TableExt_CaptionOfRelatedTable_InValidateErrorText); the corpus could not pin a Caption over the table's own (AL0839 in one app). TableCaption, RecordRef.Caption and other error texts of such a table stay refused.",
        tests: "No Microsoft test: a partner app's tests (Validate of a field related to Work Center, extended with a Caption).",
        probe: "A table with a Caption in one app and a tableextension with a Caption in another app: the TableRelation error of a Validate, TableCaption, RecordRef.Caption.",
    },
    StaleOwnInsertTwoCommitsOtherInsert => {
        id: "stale-own-insert-two-commits-other-insert",
        assumed: "A stale buffer from the variable's own Insert, or its own Modify after it, made in the transaction before the last Commit or the one before it, is written whole (the stale buffer wins) when the transaction after the last Commit inserted another row of the table and did not touch the row itself, lock the table or roll back. Pinned for an own Insert and Modify, one or two Commits, and an Insert of another row as the only write of the new transaction (record/lt-sb-stale-own-modify/SbStaleOwnModify.al:666-745); an own Insert without the Modify, and other writes of the table beside the Insert, are not told apart and stay assumed. The marker fires on the pinned shape too.",
        tests: "No Microsoft test: a partner app's tests (Library - Manufacturing.RefreshProdOrder commits before and after Refresh Production Order).",
        probe: "The SH_OwnModify_ tests of SbStaleOwnModify.al with an own Insert only, with a Modify of another row beside the Insert, and with three Commits.",
    },
    StaleOwnInsertLockOtherOrOtherRowWrite => {
        id: "stale-own-insert-lock-other-or-other-row-write",
        assumed: "A stale buffer from the variable's own Insert, or its own Modify after it: (1) two Commits later, when the new transaction ran LockTable on other tables only, inserted only into those tables and read the buffer's table by FindSet, a Modify raises DB:RecordChanged; (2) three Commits later, when the new transaction's only writes were Modifies of other rows of the table, a Modify writes the whole buffer; (3) three Commits later, when its only writes were Deletes of other rows of the table, a Delete deletes the row. Each needs no rollback, no other lock and no write of the row itself. Pinned only by the reporter's run of al-runner issue 6028 on real BC; the other counts and forms writers TE and TH measured follow stale-own-write-neighbour-any-commits.",
        tests: "No Microsoft test: al-runner issue 6028, the reporter's run on a real BC container (3/3 tests; a partner app's Prod. Order Component, Item and ledger-entry tests).",
        probe: "The TE_ tests of SbStaleOwnModify.al: 1, 2 and 3 Commits, crossed with LockTable on another table with an Insert and a read, LockTable only, LockTable on the same table, Modify, Delete, Insert or Rename of another row, for a stale Modify and a stale Delete. Corpus writer TE (branch al-runtime-rs-staging-te).",
        awaiting: Awaiting {
            issues: &[6028],
            files: &[
                "tests/al-language/record/lt-sb-stale-own-modify/SbStaleOwnModify.al::TE_2C_LockOtherInsertRead_StaleModify",
                "tests/al-language/record/lt-sb-stale-own-modify/SbStaleOwnModify.al::TE_3C_ModifyOtherRow_StaleModify",
                "tests/al-language/record/lt-sb-stale-own-modify/SbStaleOwnModify.al::TE_3C_DeleteOtherRow_StaleDelete",
            ],
            since: "2026-10-28",
        },
    },
    QueryTopWhileOpenAfterLastRow => {
        id: "query-top-while-open-after-last-row",
        assumed: "Query.TopNumberOfRows(N) on a query that is open and was read to its end (a Read returned false: an empty result or the last row read) raises nothing, the getter returns N, the next Read raises the metadata-changed error until Open, and Open again (without Close) reads at most N rows; the last value set before the Open counts. The same on a query not yet read to its end is pinned (misc/lt-probes-needed-1/PnAQueryOpenRead.al:247-290, PX_TopNumberOfRows_AroundOpen). Pinned only by the reporter's run of al-runner issue 6039 on real BC.",
        tests: "No Microsoft test: al-runner issue 6039, the reporter's run on a real BC container (2/2 tests; Base App table 7346 \"Internal Movement Header\" GetFirstLocationCodeFromLocationsofWhseEmployee calls TopNumberOfRows(1) and Open twice on one query variable).",
        probe: "The TH_ tests of PnAQueryOpenRead.al: TopNumberOfRows after an empty Open and Read, after a full read, twice while open, then Open and the rows read. Corpus writer TH (branch al-runtime-rs-staging-pub-th).",
        awaiting: Awaiting {
            issues: &[6039],
            files: &[
                "tests/al-language/misc/lt-probes-needed-1/PnAQueryOpenRead.al::TH_TopAfterEmptyOpen_ThenReopen",
                "tests/al-language/misc/lt-probes-needed-1/PnAQueryOpenRead.al::TH_TopAfterFullRead_ThenReopen",
                "tests/al-language/misc/lt-probes-needed-1/PnAQueryOpenRead.al::TH_TopTwiceWhileOpen_LastValueCounts",
            ],
            since: "2026-10-28",
        },
    },
    StaleOwnWriteNeighbourAnyCommits => {
        id: "stale-own-write-neighbour-any-commits",
        assumed: "A stale buffer from the variable's own Insert (or its own Modify after it), any number of Commits later, when the new transaction did not write that row: (1) LockTable on other tables only with no write, or Inserts into other tables only (locked or not) and a FindSet of the buffer's table: a Modify or Delete through the buffer raises DB:RecordChanged; (2) LockTable on the buffer's table only with no write: a Modify writes the whole buffer and a Delete deletes the row; (3) only Modifies, only Deletes, only Inserts or only Renames of other rows of the table, with no lock: a Modify writes the whole buffer and a Delete deletes the row. With nothing in between it stays refused as pinned. Pinned only by writer TE's and TH's runs on real BC (1, 2, 3, 4, 5, 7, 8 and 10 Commits, not every form at every count) and the reporter's run of al-runner issue 6040; every other count is assumed to behave the same. A ModifyAll or DeleteAll, a Rename whose triggers or related fields write, a mix of write kinds, a write beside a lock, a rollback and a ReadIsolation lock stay refused.",
        tests: "No Microsoft test: al-runner issue 6040, the reporter's run on a real BC container (3/3 tests; a partner app's Prod. Order Component, Item and ledger-entry tests).",
        probe: "The TE_ and TH_ tests of SbStaleOwnModify.al (1 to 10 Commits crossed with each neighbour). Corpus writers TE and TH (branch al-runtime-rs-staging-pub-th).",
        awaiting: Awaiting {
            issues: &[6040],
            files: &[
                "tests/al-language/record/lt-sb-stale-own-modify/SbStaleOwnModify.al::TH_4C_LockOtherInsertRead_StaleModify",
                "tests/al-language/record/lt-sb-stale-own-modify/SbStaleOwnModify.al::TH_8C_ChangeAfter6_LockOtherInsertRead_StaleModify",
                "tests/al-language/record/lt-sb-stale-own-modify/SbStaleOwnModify.al::TH_7C_LockFindSetModifyMiddle_DeleteOtherRow_StaleDelete",
                "tests/al-language/record/lt-sb-stale-own-modify/SbStaleOwnModify.al::TE_1C_LockSameTable_StaleModify",
            ],
            since: "2026-10-28",
        },
    },
    PartKeepsRowAfterUpdateInValidate => {
        id: "part-keeps-row-after-update-in-validate",
        assumed: "A ListPart (without OnFindRecord or OnNextRecord) of a Document page, standing on a stored row other than the first of its view, whose control's OnValidate runs CurrPage.Update() or CurrPage.Update(false) while the host's row stays as the host read it, stays on that row: it reads from the row to the end of the view, the row again when it moved, the rows before it back to the first, then the row twice and OnAfterGetCurrRecord. Pinned only by the reporter's run of al-runner issue 6038 on real BC (a view of three rows, in the primary key and with a SubPageView sorting); a view longer than a window is refused.",
        tests: "No Microsoft test: al-runner issue 6038, the reporter's run on a real BC container (2/2 tests; page 5771 \"Whse. Put-away Subform\" calls CurrPage.Update() in the OnValidate of Bin Code, page 7376 \"Invt. Put-away Subform\" CurrPage.Update(true) in that of Qty. to Handle).",
        probe: "The TH_ tests of ThPartUpdateCursor.al: SetValue on line 2 and 3 of 3 with Update(true), Update(false), SaveRecord and no trigger, the trigger log and the walk First/SetValue/Next, also with a SubPageView sorting. Corpus writer TH (branch al-runtime-rs-staging-pub-th).",
        awaiting: Awaiting {
            issues: &[6038],
            files: &["tests/al-language/pages/lt-th-part-update-cursor/ThPartUpdateCursor.al"],
            since: "2026-10-28",
        },
    },
    ViewBlankDateFilter => {
        id: "view-blank-date-filter",
        assumed: "A view property's condition `filter('')` on a Date field (an action's RunPageView, a part's SourceTableView or SubPageView, a report's or XMLport's view) selects the blank date, as a top-level page's SourceTableView does (pages/lt-cd-testpage-leads/CdTestPageLeads.al:842 SH_SourceTableView_DateFilterBlank_Rows) and `filter('')` of a TableRelation or CalcFormula selects the blank value (record/lt-relation-where-empty-term/RelationWhereEmptyTerm.al).",
        tests: "No Microsoft test: a partner app's tests (a list page with where(Date = filter('')) opens on the blank-date row and Next() is false).",
        probe: "An action with RunPageView = where(D = filter('')) over rows with a blank and a set date: the rows the opened list shows; the same view on a ListPart.",
    },
    TreeWorksheetGoneOnlyRowBlankRec => {
        id: "tree-worksheet-gone-only-row-blank-rec",
        assumed: "A Worksheet without the new-row line whose repeater shows its rows as a tree (ShowAsTree, IndentationColumn), opened for edit, that lost its only row to an action stands on a blank Rec as the same Worksheet without the tree does: OnNewRecord and OnAfterGetCurrRecord once, at that action and at a later one. Pinned for a page with InsertAllowed, DeleteAllowed and ModifyAllowed false (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:509, :535); with DeleteAllowed or ModifyAllowed left true it is assumed.",
        tests: "No Microsoft test: a partner app's tests (a check worksheet's finish action that recalculates to no rows).",
        probe: "The tree Worksheet of RmWorksheetGoneRow.al with DeleteAllowed and ModifyAllowed left true, with an action that calls CurrPage.Update(false), and over a table that is not temporary.",
    },
    ReadOnlyViewListGoneOnlyRow => {
        id: "read-only-view-list-gone-only-row",
        assumed: "A List page with Editable = false, opened for view, without the new-row line, that lost its only row to an action raises no trigger after the action and shows no row, as a List with the line opened for view does (pages/W1Update.al:534). Pinned with InsertAllowed, DeleteAllowed and ModifyAllowed false and an action that calls CurrPage.Update(false) (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:481); with DeleteAllowed or ModifyAllowed left true it is assumed.",
        tests: "No Microsoft test: a partner app's tests (a \"Retry Now\" action on an error log list).",
        probe: "The List of RmWorksheetGoneRow.al SH_ListViewNotEditable_ with the Allowed properties left true, without CurrPage.Update, and with a second action.",
    },
    EmptyWorksheetFilledByAction => {
        id: "empty-worksheet-filled-by-action",
        assumed: "A Worksheet with the new-row line (AutoSplitKey, DelayedInsert) opened for edit on an empty view, after an action that stored three rows or more (up to a window) through another record variable without CurrPage.Update: the client reads each row of the view once, the first row twice more, runs OnAfterGetCurrRecord on it with a blank xRec and goes back to its new-row line; a later action first reads the new line. Pinned with two rows (pages/lt-outside-worksheet/RmWorksheetGoneRow.al:573, :601).",
        tests: "No Microsoft test: a partner app's tests (page 7372 Bin Creation Worksheet: Calculate Bins, then Create Bins).",
        probe: "The Worksheet of RmWorksheetGoneRow.al SH_WsWithNew_ with three and with 30 rows stored by the first action.",
    },
    TestFilterEnumRangeOrdinals => {
        id: "test-filter-enum-range-ordinals",
        assumed: "TestFilter.GetFilter of an Enum field filtered to a range over more than two values (SetRange on the record handed to the page, or a SetFilter text range) reads as the two ordinals, `0..2`, as a range of two values does (pages/lt-cd-testpage-leads/CdTestPageLeads.al:923 SH_TestPageFilter_GetFilter_EnumRangeAndList `0..1`, `1..2`).",
        tests: "No Microsoft test: a partner app's test on its own list page.",
        probe: "TestFilter.GetFilter of an Enum field after SetRange(First, Third) on the record handed to Page.Run, and after a SourceTableView range over three values.",
    },
    XmlReadFromStreamOptions => {
        id: "xml-readfrom-stream-options",
        assumed: "XmlDocument.ReadFrom(InStream, XmlReadOptions, var XmlDocument) reads as the stream form without options does when PreserveWhitespace is false (whitespace-only text nodes are kept), and a document that does not parse answers false with the error text of the stream form in GetLastErrorText. PreserveWhitespace true is pinned (xml/XmlDocReadWrite.al:290-307); the statement form of a document that does not parse stays refused.",
        tests: "No Microsoft test: al-runner issue 6049, the reporter's run on real BC (a shared XML helper reads every document through this overload; 1,299 tests of a partner suite). Writer TJ's run on real BC: branch al-runtime-rs-staging-pub-tj, xml/XmlDocReadWrite.al:331-389.",
        probe: "Writer TJ's ReadFrom_InStream_Options_WhitespaceNodes and ReadFrom_InStream_Options_Invalid (corpus PR 774).",
        awaiting: Awaiting {
            issues: &[6049],
            files: &[
                "tests/al-language/xml/XmlDocReadWrite.al::ReadFrom_InStream_Options_WhitespaceNodes",
                "tests/al-language/xml/XmlDocReadWrite.al::ReadFrom_InStream_Options_Invalid"
            ],
            since: "2026-10-28",
        },
    },
    TextFnDecimalArgument => {
        id: "text-fn-decimal-argument",
        assumed: "A Decimal where PadStr, CopyStr, DelStr, InsStr or SelectStr takes an Integer: a whole value within the Integer range works as that Integer; a fraction raises ConversionCast `Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value X to System.Int32.`, the arguments converted left to right. Pinned only by corpus writer TG's run on real BC (whole values 2 and 3, fractions 0.5 to 3.6 and -1.5). A whole value beyond the Integer range is refused.",
        tests: "No Microsoft test: a partner app's receipt printing (PadStr with a Decimal width that was rounded down to a whole number).",
        probe: "Done on writer TG's branch (corpus PR 771): the Decimal tests of text/lt-text-functions; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/text/lt-text-functions/PadStr.Codeunit.al::PadStrDecimalLen1",
                "tests/al-language/text/lt-text-functions/PadStr.Codeunit.al::PadStrDecimalFill1",
                "tests/al-language/text/lt-text-functions/PadStr.Codeunit.al::PadStrDecimalLiteral",
                "tests/al-language/text/lt-text-functions/CopyStr.Codeunit.al::CopyStrDecimalPos1",
                "tests/al-language/text/lt-text-functions/CopyStr.Codeunit.al::CopyStrDecimalPosLen1",
                "tests/al-language/text/lt-text-functions/DelStr.Codeunit.al::DelStrDecimalPos1",
                "tests/al-language/text/lt-text-functions/DelStr.Codeunit.al::DelStrDecimalPosLen1",
                "tests/al-language/text/lt-text-functions/InsStr.Codeunit.al::InsStrDecimalPos1",
                "tests/al-language/text/lt-text-functions/SelectStr.Codeunit.al::SelectStrDecimalNum1",
            ],
            since: "2026-10-28",
        },
    },
    XmlportUdnTextAttribute => {
        id: "xmlport-udn-text-attribute",
        assumed: "An XmlPort with UseDefaultNamespace = true and no DefaultNamespace that has text attributes: export writes the root's attributes before its `xmlns` declaration of BC's namespace, and every attribute in no namespace; import reads attributes in no namespace, raises the schema error for a missing required text or field attribute, and fails the schema check for a root attribute prefixed into the port's namespace (`The '<ns>:<name>' attribute is not declared.`). Pinned only by corpus writer TF's run on real BC.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (UseDefaultNamespace with text attributes).",
        probe: "Done on writer TF's branch: the TF_NsAttr_* tests of xmlport/lt-sg-xmlport-occurs; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_NsAttr_Export",
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_NsAttr_Import",
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_NsAttr_Import_Qualified",
            ],
            since: "2026-10-28",
        },
    },
    IsolatedStorageOverwriteTx => {
        id: "isolated-storage-overwrite-tx",
        assumed: "IsolatedStorage.Set or SetEncrypted over a key that already exists, outside a write transaction, opens the write transaction (Database.IsInWriteTransaction is true after it), as a Set of a new key does; the same value, encrypted over plain and plain over encrypted alike. Pinned only by corpus writer TO's run on real BC (right after a Commit).",
        tests: "No Microsoft test: a partner app's EFT tests overwrite a stored key with SetEncrypted after a Commit.",
        probe: "Done on writer TO's branch al-runtime-rs-staging-pub-to (corpus PR 780): CnIsolatedStorageLeads.al TO_IsoOverwrite_OutsideWriteTransaction; wanted: the bump that puts it on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/session/lt-cn-session-leads/CnIsolatedStorageLeads.al::TO_IsoOverwrite_OutsideWriteTransaction"],
            since: "2026-10-28",
        },
    },
    XmlportTextInTable => {
        id: "xmlport-text-in-table",
        assumed: "A text element right below a table element whose nodes are only text attributes, or text attributes and leaf text and field elements: export runs its OnBeforePassVariable before its attributes' and writes its text after its attributes (`<item option_value=\"1\">REQUIRED</item>`, empty text gives `<item option_value=\"2\" />`); import reads the attributes (a text attribute without Occurrence is required), then the elements below, then the element's own text with its OnAfterAssignVariable; a child element in a text-only element fails the schema check. Pinned only by corpus writer TO's run on real BC.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (a text element with an option_value attribute, and request elements with content, below a table element).",
        probe: "Done on writer TO's branch al-runtime-rs-staging-pub-to (corpus PR 780): the TO_AttrItem_* and TO_TextContent_* tests of xmlport/lt-qr-xmlport-container-occurs; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TO_AttrItem_Trigger_Export",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TO_AttrItem_Plain_Import",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TO_TextContent_Import_Shapes",
            ],
            since: "2026-10-28",
        },
    },
    Utf8ReadNonAsciiUnterminated => {
        id: "utf-eight-read-non-ascii-unterminated",
        assumed: "InStream.Read into a Text on a UTF-8 stream of non-ASCII characters with no zero byte after them reads them to the end and counts their bytes plus one, as for ASCII (`wörld` gives 7). Pinned only by corpus writer TO's run on real BC.",
        tests: "No Microsoft test: a partner app's report tests read a UTF-8 stream of non-ASCII text.",
        probe: "Done on writer TO's branch al-runtime-rs-staging-pub-to (corpus PR 780): PnE04Streams.al TO_ReadText_NonAscii_NoTerminator; wanted: the bump that puts it on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/misc/lt-probes-needed-5/PnE04Streams.al::TO_ReadText_NonAscii_NoTerminator"],
            since: "2026-10-28",
        },
    },
    XmlportAttrsOverUnlinked => {
        id: "xmlport-attrs-over-unlinked",
        assumed: "XmlPort export of a text element (MinOccurs = Zero, MaxOccurs = Once, no triggers) right below a temporary table element with MaxOccurs = Once, holding text attributes and then one text element (MinOccurs = Zero, MaxOccurs = Once, no triggers) over one temporary table element without LinkTable (MinOccurs = Once, MaxOccurs = Unbounded): both text elements are always written, the attributes first (an empty Required one as attr=\"\"), then the rows of the inner table, or that table once with the record as it stands when it has none (no OnAfterGetRecord). Pinned only by corpus writer TT's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts (a ticket change reply with its admissions).",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Change_Export_*; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Change_Export_TwoRows",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Change_Export_TokenNoRows",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Change_Export_Error",
            ],
            since: "2026-10-29",
        },
    },
    XmlportUnlinkedReply => {
        id: "xmlport-unlinked-reply",
        assumed: "XmlPort export of a text element (MinOccurs = Zero, MaxOccurs = Once, no triggers) right below a temporary table element that holds one temporary table element without LinkTable, optional or with MinOccurs = Once and MaxOccurs = Unbounded, with any supported nodes below it, after leaf field elements of the outer table without triggers: the inner table writes the rows it holds when it is reached (an outer row's OnAfterGetRecord that empties and refills it gives each outer row its own rows, in SourceTableView order); an optional inner table without rows leaves the text element empty; a MinOccurs = Once inner table without rows writes one element from its record as it is, without OnAfterGetRecord. A table element with MaxOccurs = Once, export triggers and two rows writes the first with one OnAfterGetRecord, and its record holds the second row afterwards. Pinned only by corpus writer TR's run on real BC (a SaaS sandbox) and by writer TS on every supported version, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts (a reply row over lines over parts, each line refilling its parts).",
        probe: "Done on writer TR's branch al-runtime-rs-staging-tr: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TR_Booking_* and TR_Member_*; wanted: the bump that puts them on corpus master. Not covered: a MaxOccurs = Once table element with three or more rows and triggers, or whose first OnAfterGetRecord skips or breaks.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Booking_Reply_NoLines",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Booking_Reply_OneLineOnePart",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Booking_Reply_TwoLinesTwoParts",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Booking_Reply_LinesWithoutParts",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Booking_TwoReplyRows",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Booking_ImportThenExport",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Member_ImportThenExport",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Member_Reply_Cards",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_Member_ErrorReply",
            ],
            since: "2026-10-29",
        },
    },
    XmlportExportAgain => {
        id: "xmlport-export-again",
        assumed: "A table element record after an export under its SourceTableView holds the last row the export read (with its SystemId), the view's key, the filters the code set, and the view's filters in a filter group GetView does not show (Count counts through them). Field reads, Count, GetView(false), SetCurrentKey, SetRange and SetFilter in filter group 0, Init (primary key kept, SystemId null), TransferFields(From, true) from the same table (SystemId not copied) and an assignment to it (SystemId copied) run on it; Reset gives it the primary key and no filters. A later Export on the same instance reads every table element from the start, keeps each record's current key and applies the SourceTableView filters again; a required table element without rows writes its record as it is. A MaxOccurs = Once table element with export triggers and three or more rows writes the first with one OnAfterGetRecord (the row its record holds afterwards is refused). Pinned only by corpus writer TS's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts (one port instance exported twice).",
        probe: "Done on writer TS's branch al-runtime-rs-staging-pub-ts (corpus PR 785): xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Again_* and TS_View_SecondExport; wanted: the bump that puts them on corpus master. Not covered: SetView on such a record before a later export, a view with ORDER on a later export.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_SameData",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_AssignBetween",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_TransferFieldsBetween",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_InitBetween",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_KeyBetween",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_LineFilter",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_View_SecondExport",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_TrBooking",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Again_TrMember",
            ],
            since: "2026-10-29",
        },
    },
    XmlportViewFilterUnlinked => {
        id: "xmlport-view-filter-unlinked",
        assumed: "XmlPort export of a table element without LinkTable whose SourceTableView has a WHERE: the export reads only the rows of the filter, in the view's key, and filters the code set on the record before the export combine with it. Pinned only by corpus writer TS's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's XmlPorts.",
        probe: "Done on writer TS's branch al-runtime-rs-staging-pub-ts (corpus PR 785): xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_View_FirstExport and TS_View_SecondExport; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_View_FirstExport",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_View_SecondExport",
            ],
            since: "2026-10-29",
        },
    },
    MediaTextContentExported => {
        id: "media-text-content-exported",
        assumed: "UTF-8 text (no picture, no MIME type given) imported into a Media field with the two argument ImportStream exports with Media.ExportStream as the same bytes, answering true, on every major; ReadText of the export gives the text back. Pinned only by corpus writer TR's run on real BC (the newest major, SaaS sandbox), not yet on corpus master; the older majors come from the corpus CI majors check.",
        tests: "No Microsoft test: a partner app's fiscal tests store receipt text in a Media field and read it back.",
        probe: "Done on writer TR's branch al-runtime-rs-staging-tr: streams/StTMedia.al TR_MediaImportStream_TextContent_ExportStream; wanted: the bump that puts it on corpus master. Not covered: text imported with a MIME type, text with control characters, and bytes that are no UTF-8.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/streams/StTMedia.al::TR_MediaImportStream_TextContent_ExportStream"],
            since: "2026-10-29",
        },
    },
    XmlportSelfClosedLeafAssignTwice => {
        id: "xmlport-self-closed-leaf-assign-twice",
        assumed: "XmlPort import of a leaf text element (no attributes or elements in the schema) written `<x />` runs its OnAfterAssignVariable twice, the variable set to the empty text before each run; `<x></x>`, `<x> </x>` and `<x>v</x>` run it once, and white space alone (`<x> </x>`) gives the empty text. Pinned below the root, below a table element and inside a text element below a table element by corpus writer TR's run on real BC (the newest major, SaaS sandbox), not yet on corpus master; assumed also for leaf text elements inside deeper text elements read the same way. Whether the variable is set again before the second run is not pinned (the corpus triggers only read it).",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts import replies with empty leaf elements whose triggers log or count.",
        probe: "Done on writer TR's branch al-runtime-rs-staging-tr: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TR_EmptyLeaf_Import; wanted: the bump that puts it on corpus master, and a trigger that changes its own variable on the first run.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TR_EmptyLeaf_Import"],
            since: "2026-10-29",
        },
    },
    XmlportDefaultContainerUnlinked => {
        id: "xmlport-default-container-unlinked",
        assumed: "XmlPort export of a text element with the default MinOccurs and MaxOccurs, without triggers, right below a temporary table element, holding only one temporary table element without LinkTable (MaxOccurs = Once, MinOccurs = Zero): without rows the text element is written empty, with rows it holds the first row only, with one OnAfterGetRecord. Pinned only by corpus writer TS's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts (a member reply with its cards).",
        probe: "Done on writer TS's branch al-runtime-rs-staging-pub-ts (corpus PR 785): xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Cards_DefaultOccurs_Export; wanted: the bump that puts it on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Cards_DefaultOccurs_Export"],
            since: "2026-10-29",
        },
    },
    XmlportLinkedMaxUnbounded => {
        id: "xmlport-linked-max-unbounded",
        assumed: "An explicit MaxOccurs = Unbounded on a table element with LinkTable right below another table element is the default: export and import run exactly as without it (same XML, trigger order, imported rows and schema errors). Pinned only by corpus writer TS's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (ticket rows linked to a request row).",
        probe: "Done on writer TS's branch al-runtime-rs-staging-pub-ts (corpus PR 785): xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Linked_Unbounded_*; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Linked_Unbounded_Export",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Linked_Unbounded_Export_NoRows",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Linked_Unbounded_Import",
            ],
            since: "2026-10-29",
        },
    },
    XmlportLinkedOnce => {
        id: "xmlport-linked-once",
        assumed: "XmlPort export of a table element with LinkTable and no LinkFields, MinOccurs = Zero and MaxOccurs = Once, right below another table element: each pass (each row of the element above) writes the first row of the record's view only, with one OnAfterGetRecord, and nothing without rows; the row the record holds after a pass of two or more rows is not pinned (its reads refuse). Import of such an element is refused. Pinned only by corpus writer TT's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts (a ticket confirmation with its reservation detail).",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Confirm_Export_*; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Confirm_Export_NpFilters",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Confirm_Export_NoKindFilter",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Confirm_Export_NoPreFilter",
            ],
            since: "2026-10-29",
        },
    },
    XmlportLinkPassReset => {
        id: "xmlport-link-pass-reset",
        assumed: "XmlPort export of a table element with LinkTable and no LinkFields right below another table element: LinkTable adds no filter, and every pass (each row of the element above) starts from the filters the record had at its first pass, with its SourceTableView, and from the SourceTableView's sorting (or the primary key without a sorting in the view), ascending: filters, a key, a direction and a Reset that OnPreXmlItem set at an earlier pass do not carry over. The SourceTableView filters do not show in GetFilters, Count obeys them. A key, direction, filter or mark of the code's own on the record at the first pass, and a view with ORDER, are refused. Pinned only by corpus writer TT's and TU's runs on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts (a ticket confirmation: tickets below a reply row, reservation detail below an admission row).",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt (QrXpContainerOccurs.al TT_Confirm_Export_*) and writer TU's branch al-runtime-rs-staging-tu (QrXpContainerOccurs.al TU_LinkedKey_*, the key per pass); wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Confirm_Export_NpFilters",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Confirm_Export_NoKindFilter",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Confirm_Export_TwoReplies",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TU_LinkedKey_KeyAtFirstPass",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TU_LinkedKey_DescendingAtFirstPass",
            ],
            since: "2026-10-29",
        },
    },
    XmlportRootMinOccursOnce => {
        id: "xmlport-root-min-occurs-once",
        assumed: "An explicit MinOccurs = Once on the root text element is the default: export runs as without it; import accepts an empty root element, fails the schema check on another root element and fails with 'Root element is missing.' on an empty document, as without it. Pinned only by corpus writer TS's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (a ticket change request).",
        probe: "Done on writer TS's branch al-runtime-rs-staging-pub-ts (corpus PR 785): xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_RootOnce_*; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_RootOnce_Export",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_RootOnce_Import",
            ],
            since: "2026-10-29",
        },
    },
    FormatTimeCodeNames => {
        id: "format-time-code-names",
        assumed: "Format of a Time with the code names <Hours>, <Hour>, <Ho>, <H> (the 24-hour hour), <Minute>, <Min>, <Second>, <Sec> and <Thou>, in any case, without padding; <Hours,N> pads with blanks to N for N = 1 to 3 and writes '*' when the hour is longer, N = 0 is no width; on a DateTime <Hours,2> pads with a zero; the Length argument cuts a longer Time result to its first characters. Other widths and other short names are refused, and only the pinned unknown names raise the invalid-format error. Pinned only by corpus writer TU's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's fiscal requests (a time written with <Hours,2>:<Minutes,2>:<Seconds,2>).",
        probe: "Done on writer TU's branch al-runtime-rs-staging-tu: types/lt-datetime/Format.al TU_Time_Format_CodeNames; wanted: the bump that puts it on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/types/lt-datetime/Format.al::TU_Time_Format_CodeNames"],
            since: "2026-10-29",
        },
    },
    XmlDocBlankText => {
        id: "xml-doc-blank-text",
        assumed: "XmlDocument.Create and XmlDocument.Add with an empty Text add no node; a blank Text, or an empty or blank XmlText, is added as a document node before or after the root without an error, counts in GetChildNodes, and WriteTo writes it as it is (no line break before the root after it). Pinned only by corpus writer TT's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's fiscal web service requests (a document created with two empty texts).",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt: misc/lt-probes-needed-1/PnAXmlNodeAdds.al TT_EmptyOrBlankTextIntoXmlDocument; wanted: the bump that puts it on corpus master. Open: two text nodes in one document, AddFirst and the sibling methods.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/misc/lt-probes-needed-1/PnAXmlNodeAdds.al::TT_EmptyOrBlankTextIntoXmlDocument"],
            since: "2026-10-29",
        },
    },
    XmlportImportAgain => {
        id: "xmlport-import-again",
        assumed: "A second Import on an XmlPort instance whose first Import succeeded (unguarded, Format = Xml) reads nothing and stores nothing: only OnPreXmlPort and OnPostXmlPort run again (not OnInitXmlPort), also after SetSource on the same or a new stream, with or without Codeunit.Run. Pinned only by corpus writer TT's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP handler that imports the port it was handed again (loyalty points).",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_ImportTwice_*; wanted: the bump that puts them on corpus master. Open: a guarded second Import, one after a failed Import, and text formats.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_ImportTwice_SameSource",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_ImportTwice_SameStreamAgain",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_ImportTwice_NewSourceOtherRows",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_ImportTwice_NewSourceSameRows",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_ImportTwice_Direct",
            ],
            since: "2026-10-29",
        },
    },
    XmlportImportWholeSource => {
        id: "xmlport-import-whole-source",
        assumed: "An XML import reads the whole source stream, also when the stream was read to its end before SetSource, and leaves the stream's read position where it was. Pinned only by corpus writer TT's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP handler that reads the request stream before it imports it.",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Import_SourceReadBefore, TT_ImportTwice_SameSource; wanted: the bump that puts them on corpus master. Open: a stream read partly before the import.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Import_SourceReadBefore",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_ImportTwice_SameSource",
            ],
            since: "2026-10-29",
        },
    },
    XmlportOptionalNoRows => {
        id: "xmlport-optional-no-rows",
        assumed: "XmlPort export of an optional table element (MinOccurs = Zero) without LinkTable that is not below another table element, without rows: only its OnPreXmlItem runs; no OnBeforePassField or OnBeforePassVariable of its nodes runs and nothing is written. Pinned only by corpus writer TT's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts.",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TT_Optional_Unlinked_NoRows; wanted: the bump that puts it on corpus master. Open: the same below another table element.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TT_Optional_Unlinked_NoRows"],
            since: "2026-10-29",
        },
    },
    PageExtValidateTableOnValidate => {
        id: "page-ext-validate-table-onvalidate",
        assumed: "A SetValue on a page control with OnBeforeValidate or OnAfterValidate in a page extension's modify block, whose field has a table OnValidate: the table's validation (its OnBeforeValidateEvent, OnValidate, OnAfterValidateEvent) runs first, then the page's OnBeforeValidateEvent, the modify OnBeforeValidate, the control's OnValidate, the modify OnAfterValidate and the page's OnAfterValidateEvent. Pinned only by corpus writer TT's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's page extension tests (a variety matrix on a transfer order).",
        probe: "Done on writer TT's branch al-runtime-rs-staging-tt: pageextensiontrigger/TestModifiedControlValidateTriggers_Tests.al TT_TableOnValidate_Order, TT_TableOnValidate_Order_WithEvents; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/pageextensiontrigger/TestModifiedControlValidateTriggers_Tests.al::TT_TableOnValidate_Order",
                "tests/al-language/pageextensiontrigger/TestModifiedControlValidateTriggers_Tests.al::TT_TableOnValidate_Order_WithEvents",
            ],
            since: "2026-10-29",
        },
    },
    XmlportTimeMsXml => {
        id: "xmlport-time-ms-xml",
        assumed: "XmlPort export with FormatEvaluate = Xml of a Time or a DateTime with milliseconds writes them as Format standard 9 does: the milliseconds without trailing zeros (12:34:56.789, 12:34:56.5, 12:34:56.007), a DateTime with the same fraction and Z; a zero Duration is P0DT0H0M0.0S. Pinned only by corpus writer TU's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's loyalty SOAP replies (a time stamp with milliseconds).",
        probe: "Done on writer TU's branch al-runtime-rs-staging-tu: xmlport/PWXmlPortFormatEvaluate.al TU_Export_TimeMilliseconds; wanted: the bump that puts it on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/xmlport/PWXmlPortFormatEvaluate.al::TU_Export_TimeMilliseconds"],
            since: "2026-10-29",
        },
    },
    XmlportNestedUnlinked => {
        id: "xmlport-nested-unlinked",
        assumed: "XmlPort export of a table element without LinkTable right below another table element: every pass after the first starts without the filters the pass before left (GetFilters is empty) and the record keeps its row; MinOccurs = Once with MaxOccurs = Unbounded without rows writes the element once from the record as it stands, without OnAfterGetRecord, and runs the elements below; MinOccurs = Zero with MaxOccurs = Once writes the first row and leaves the record on the second row, also of three or more rows. Filters, a key, a filter group or marks the code sets between passes, a key set at a pass, filters of the code's own at the first pass and a SourceTableView are refused, and so is import of MaxOccurs = Once. Pinned only by corpus writer TU's run on real BC, not yet on corpus master.",
        tests: "No Microsoft test: a partner app's SOAP reply XmlPorts (a ticket change request reply: access entries below a ticket, reservation detail below an access entry).",
        probe: "Done on writer TU's branch al-runtime-rs-staging-tu: xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TU_NestedReal_*, TU_NestedTemp_*; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TU_NestedReal_ItemFilterFirstPass",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TU_NestedReal_SlotNoFilter",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TU_NestedTemp_TwoTwoNoneFiltered",
            ],
            since: "2026-10-29",
        },
    },
    XmlportGuidXml => {
        id: "xmlport-guid-xml",
        assumed: "XmlPort with FormatEvaluate = Xml and a Guid field element or attribute: import takes the 8-4-4-4-12 form bare, in braces or in parentheses, or the 32 digits bare, in any case, with spaces around; empty or missing gives the null Guid; a text with other characters, or the 8-4-4-4-12 form one digit short, fails with 'Invalid format of GUID string. ...' (InvalidGuidFormat). Export writes `{UPPER-CASE}`. Pinned only by corpus writer TS's run on real BC, not yet on corpus master; other forms are refused.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (Guid tokens in requests and replies).",
        probe: "Done on writer TS's branch al-runtime-rs-staging-pub-ts (corpus PR 785): xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Guid_*; wanted: the bump that puts them on corpus master. Not covered: mismatched brackets, the 32 digits in brackets, other lengths.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Guid_Import_Element",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Guid_Import_Attribute",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TS_Guid_Export",
            ],
            since: "2026-10-29",
        },
    },
    TenantMediaDelete => {
        id: "tenant-media-delete",
        assumed: "Delete() and Delete(true) of a Tenant Media row: the media leaves Tenant Media and Delete returns true (the statement form raises no error), every row that holds it keeps the MediaId with HasValue false and ExportStream false; a missing id returns false, or in the statement form fails with `The Tenant Media does not exist. Identification fields and values: ID='{..}'` (DB:RecordNotFound). Pinned only by corpus writers TP and TQ on real BC (BC containers; a SaaS sandbox refuses every Tenant Media Delete for license reasons), not yet on corpus master.",
        tests: "No Microsoft test: a partner app's tests (a cleanup that deletes the Tenant Media row of a picture).",
        probe: "Done on writer TQ's branch al-runtime-rs-staging-pub-tq (corpus PR 783, includes TP): media/TestMediaPngImport.al TP_TenantMedia_Delete and TQ_TenantMedia_DeleteTrue_*; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/media/TestMediaPngImport.al::TP_TenantMedia_Delete",
                "tests/al-language/media/TestMediaPngImport.al::TQ_TenantMedia_DeleteTrue_Statement",
                "tests/al-language/media/TestMediaPngImport.al::TQ_TenantMedia_DeleteTrue_Used",
                "tests/al-language/media/TestMediaPngImport.al::TQ_TenantMedia_DeleteTrue_SharedMedia",
                "tests/al-language/media/TestMediaPngImport.al::TQ_TenantMedia_DeleteTrue_Missing_Statement",
            ],
            since: "2026-10-29",
        },
    },
    FormatPrecisionFirstCode => {
        id: "format-precision-first-code",
        assumed: "Format of a Decimal with `<Precision,A:B>` as the first code and no `<Standard Format,N>`: the value is rounded half away from zero to B decimals before `<Sign>`, `<Integer>`, `<Integer Thousand>` and `<Decimals>`, and `<Decimals>` shows at least A decimals (`-1234.57`, `12.00`, `1,234,50` with `<Comma,,>`). Also `<Precision,2:26>` before `<Standard Format,N>` acts as the other precisions (`1234.50 ` under format 4). Pinned only by corpus writers TP and TQ on real BC, not yet on corpus master: first without a standard format for 2:2 (TP) and 0:0, 1:3, 3:3 (TQ, with `<Comma,x>`), other A:B refused where they would show; 2:26 under formats 0, 2, 4 and 9.",
        tests: "No Microsoft test: a partner app's receipt printing (amounts formatted with `<Precision,2:2>` and a decimal comma).",
        probe: "Done on writer TQ's branch al-runtime-rs-staging-pub-tq (corpus PR 783, includes TP): types/lt-format-comma-format-string/FormatCommaFormatString.al TP_Format_PrecisionStrings(_Small), TQ_Format_PrecisionStd_*, TQ_Format_PrecisionComma; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/types/lt-format-comma-format-string/FormatCommaFormatString.al::TP_Format_PrecisionStrings",
                "tests/al-language/types/lt-format-comma-format-string/FormatCommaFormatString.al::TP_Format_PrecisionStrings_Small",
                "tests/al-language/types/lt-format-comma-format-string/FormatCommaFormatString.al::TQ_Format_PrecisionStd_Long",
                "tests/al-language/types/lt-format-comma-format-string/FormatCommaFormatString.al::TQ_Format_PrecisionComma",
            ],
            since: "2026-10-29",
        },
    },
    XmlportNestedTextInTable => {
        id: "xmlport-nested-text-in-table",
        assumed: "A text element right below a table element whose nodes are field and text attributes and one text element of only text attributes: export runs the outer element's OnBeforePassVariable, writes its attributes, then the inner element as `<admission option_value=\"1\">A1</admission>` (its OnBeforePassVariable before its attribute's, an empty text gives `<admission option_value=\"2\" />`, an empty Required attribute `option_value=\"\"`); import reads the outer attributes (a text attribute without Occurrence is required), then the inner attributes and text with its OnAfterAssignVariable, then the outer element's own text; a missing inner element or Required attribute fails the schema check. Pinned only by corpus writer TP's run on real BC.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (a schedule entry element with an option_value element inside it, below a table element).",
        probe: "Done on writer TP's branch al-runtime-rs-staging-pub-tp (corpus PR 782): the TP_Nested_* tests of xmlport/lt-qr-xmlport-container-occurs; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TP_Nested_Export",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TP_Nested_Import_Shapes",
            ],
            since: "2026-10-29",
        },
    },
    XmlportUnlinkedTempInText => {
        id: "xmlport-unlinked-temp-in-text",
        assumed: "A text element (MinOccurs = Zero, MaxOccurs = Once, no triggers) right below a temporary table element that holds one temporary table element without LinkTable (MinOccurs = Zero, only field attributes): export writes all the inner records under every outer row, `<attributes />` when there are none; import stores each inner row (its OnBeforeInsertRecord sees the outer row's attributes) before the outer row's OnBeforeInsertRecord, and the inner rows stay across outer rows; a second container or one after a later element fails the schema check. Pinned only by corpus writer TP's run on real BC.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (an `attributes` element of attribute rows inside a request row).",
        probe: "Done on writer TP's branch al-runtime-rs-staging-pub-tp (corpus PR 782): the TP_Unlinked_* tests of xmlport/lt-qr-xmlport-container-occurs; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TP_Unlinked_Export",
                "tests/al-language/xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al::TP_Unlinked_Import_Shapes",
            ],
            since: "2026-10-29",
        },
    },
    XmlNodeListClear => {
        id: "xml-node-list-clear",
        assumed: "Clear of an XmlNodeList leaves it empty (Count 0, Get false, foreach runs no time), a copy taken before keeps its nodes, and SelectNodes fills it again. Pinned only by corpus writer TP's run on real BC.",
        tests: "No Microsoft test: a partner app's tests (a report that clears a node list it reuses).",
        probe: "Done on writer TP's branch al-runtime-rs-staging-pub-tp (corpus PR 782): xml/XmlNodes.al TP_NodeList_Clear; wanted: the bump that puts it on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &["tests/al-language/xml/XmlNodes.al::TP_NodeList_Clear"],
            since: "2026-10-29",
        },
    },
    QueryReadStateSharedCommitted => {
        id: "query-read-state-shared-committed",
        assumed: "A query with ReadState = ReadShared or ReadState = ReadCommitted returns the rows ReadUncommitted returns: the session's own uncommitted row and the same row after a Commit (query/TestQueryReadStateContracts.al pins ReadUncommitted).",
        tests: "No Microsoft test: al-runner issue 6050, the reporter's run on real BC (47 tests of a partner suite reach a query with ReadState = ReadShared). Writer TJ's run on real BC: branch al-runtime-rs-staging-pub-tj, query/TestQueryReadStateContracts.al:135-163.",
        probe: "Writer TJ's ReadStateValues_SeeTheSessionsOwnUncommittedRow_AndTheCommittedOne (corpus PR 774).",
        awaiting: Awaiting {
            issues: &[6050],
            files: &["tests/al-language/query/TestQueryReadStateContracts.al::ReadStateValues_SeeTheSessionsOwnUncommittedRow_AndTheCommittedOne"],
            since: "2026-10-28",
        },
    },
    OptionDecimalConversion => {
        id: "option-decimal-conversion",
        assumed: "An Option (field, variable or member) converts to a Decimal as its ordinal: assignment, a Decimal parameter, Validate of a Decimal field, and arithmetic with a Decimal or Integer. A Decimal converts to an Option when it is a whole number within Integer (an ordinal the Option lacks is kept, Format shows the number); a fraction raises ConversionCast `Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value <d> to System.Int32.`. A Decimal beyond Integer is refused.",
        tests: "No Microsoft test: al-runner issue 6060, the reporter's run on real BC (a payroll suite assigns an Option field to a Decimal field). Writer TL's run on real BC: branch al-runtime-rs-staging-pub-tl, types/lt-implicit-conversions/P79OptionDecimal.al.",
        probe: "Writer TL's P79OptionDecimal.al (corpus PR 778); a Decimal beyond Integer into an Option is still open.",
        awaiting: Awaiting {
            issues: &[6060],
            files: &["tests/al-language/types/lt-implicit-conversions/P79OptionDecimal.al"],
            since: "2026-10-28",
        },
    },
    XmlportMinOccursOnceTable => {
        id: "xmlport-min-occurs-once-table",
        assumed: "An explicit MinOccurs = Once on an XmlPort table element that is not below another table element (temporary or real, with or without MaxOccurs = Once) behaves as the default: an export with no rows writes the record as it stands, an import without the element fails the schema check. Pinned only by corpus writer TF's run on real BC.",
        tests: "No Microsoft test: a partner app's SOAP XmlPorts (MinOccurs = Once on their table elements).",
        probe: "Done on writer TF's branch: the TF_MinOnce_*, TF_MinMaxOnce_* and TF_MinMaxOnceReal_* tests of xmlport/lt-sg-xmlport-occurs; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_MinOnce_Export",
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_MinOnce_Import",
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_MinMaxOnce_Export",
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_MinMaxOnce_Import",
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_MinMaxOnceReal_Export",
                "tests/al-language/xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al::TF_MinMaxOnceReal_Import",
            ],
            since: "2026-10-28",
        },
    },
    InstallCompanyTriggersAllApps => {
        id: "install-company-triggers-all-apps",
        assumed: "In the install session of an app under test, Company Triggers OnCompanyOpen and then OnCompanyOpenCompleted reach the subscribers of every installed app (the dependency apps, Microsoft's included, the apps under test installed before it, and the app itself), each event in ascending codeunit id, before that company's install routines (owner decision 2026-10-19, ADR-046 amendment 1, A4 item 4). Pinned only for the subscribers of the app being installed (install/lt-bf-install-probes/BfInstallTests.al:125-140). A test that reads a table those other subscribers write, or one written by a per-company install routine that reads such a table or shares a SingleInstance codeunit with them, is a flagged pass.",
        tests: "No Microsoft test: owner decision for partner apps whose per-company install routine reads a table a dependency app's Company Triggers subscriber writes (a reporter's suite: 126 tests refused before).",
        probe: "Probe Q3 of ADR-046: a fixture app subscriber to OnCompanyOpen increments a SingleInstance counter; the main app's per-company install routine logs it, and a test counts how often the subscriber ran during the fixture app's own install.",
    },
    ShowPrintStatusInert => {
        id: "show-print-status-inert",
        assumed: "A report that sets ShowPrintStatus = true runs as one without it: Run, RunModal, Execute and SaveAs give the same triggers, order, results and handler needs. Measured by corpus writer TK on real BC, not yet in the pinned corpus; `false` with Report.Run is pinned (pages/lt-report-page-properties/03-report-showprint.al:9, :43-45).",
        tests: "No Microsoft test: al-runner issue 6056, a partner app whose install routine reaches a report with ShowPrintStatus (4,245 tests refused).",
        probe: "Run, RunModal, Execute and SaveAs of processing-only reports with and without a request page and of an RDLC layout report, each with ShowPrintStatus true, false and unset: trigger log, handler needs, result. Corpus writer TK (branch al-runtime-rs-staging-tk, corpus PR 777).",
        awaiting: Awaiting {
            issues: &[6056],
            files: &["tests/al-language/report/lt-tk-showprintstatus/LtTkShowPrintStatus.al"],
            since: "2026-10-28",
        },
    },
    DotnetTypeUnassignedEnum => {
        id: "dotnet-type-unassigned-enum",
        assumed: "GetDotNetType of an unassigned DotNet variable declared as the enum System.DayOfWeek or System.Security.Cryptography.X509Certificates.X509ContentType gives that enum type (FullName and Format the full name, IsEnum true); System.Enum.Parse(that type, name) gives the member (Pkcs12 gives Pfx, 3), and a name that is not a member, or one in another case, fails with 'A call to System.Enum.Parse failed with this message: Requested value '<name>' was not found.' (DotNetInvoke:Argument). Pinned only by corpus writer TV's run on real BC, not yet on corpus master; other enums are refused.",
        tests: "No Microsoft test: a partner app's tests that reach System Application's X509 certificate code (Enum.Parse of an X509ContentType name).",
        probe: "Done on writer TV's branch al-runtime-rs-staging-tv (corpus PR 788): lt-bb-dotnet/BbGetDotNetType.al TV_GetDotNetType_UnassignedEnumVariables, TV_EnumParse_GetDotNetTypeOfUnassignedX509ContentType; wanted: the bump that puts them on corpus master.",
        awaiting: Awaiting {
            issues: &[],
            files: &[
                "tests/al-language-onprem-dotnet/lt-bb-dotnet/BbGetDotNetType.al::TV_GetDotNetType_UnassignedEnumVariables",
                "tests/al-language-onprem-dotnet/lt-bb-dotnet/BbGetDotNetType.al::TV_EnumParse_GetDotNetTypeOfUnassignedX509ContentType",
            ],
            since: "2026-10-29",
        },
    },
}

/// The words of a [`HitBits`]: room for 256 assumptions.
const HIT_WORDS: usize = 4;

/// The assumption marks as a bit set: bit `a as u32` for assumption `a`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HitBits(pub [u64; HIT_WORDS]);

impl HitBits {
    /// No marks.
    pub const EMPTY: HitBits = HitBits([0; HIT_WORDS]);

    /// The bit set with the mark of `a` added.
    #[inline]
    pub fn with(self, a: Assumption) -> HitBits {
        let mut w = self.0;
        let i = a as usize;
        w[i / 64] |= 1u64 << (i % 64);
        HitBits(w)
    }

    /// Whether the mark of `a` is set.
    #[inline]
    pub fn has(self, a: Assumption) -> bool {
        let i = a as usize;
        self.0[i / 64] & (1u64 << (i % 64)) != 0
    }
}

impl std::ops::BitOr for HitBits {
    type Output = HitBits;
    fn bitor(self, o: HitBits) -> HitBits {
        let mut w = self.0;
        for (a, b) in w.iter_mut().zip(o.0) {
            *a |= b;
        }
        HitBits(w)
    }
}

thread_local! {
    static HITS: Cell<HitBits> = const { Cell::new(HitBits::EMPTY) };
}

/// Notes that the code path of `a` ran. Cheap: one thread-local read and write. The runner collects the
/// marks of each test with [`take_hits`].
#[inline]
pub fn assume(a: Assumption) {
    HITS.with(|h| h.set(h.get().with(a)));
}

/// [`assume`] that answers true, for a match guard that decides to take the assumed path.
#[inline]
pub fn hit(a: Assumption) -> bool {
    assume(a);
    true
}

/// Forgets the marks of this thread (a test is about to start).
pub fn clear_hits() {
    HITS.with(|h| h.set(HitBits::EMPTY));
}

/// The marks of this thread as a bit set (bit `a as u32` for assumption `a`); clears them. With [`add_hit_bits`], a
/// caller that replays a recorded run of code puts back the marks that run made.
pub fn take_hit_bits() -> HitBits {
    HITS.with(|h| h.replace(HitBits::EMPTY))
}

/// Adds the marks of the bit set `bits` (from [`take_hit_bits`]) to this thread's marks.
pub fn add_hit_bits(bits: HitBits) {
    HITS.with(|h| h.set(h.get() | bits));
}

/// The marks of this thread since the last [`clear_hits`] or `take_hits`, in registry order; clears them.
pub fn take_hits() -> Vec<Assumption> {
    let bits = HITS.with(|h| h.replace(HitBits::EMPTY));
    Assumption::ALL.iter().copied().filter(|a| bits.has(*a)).collect()
}

/// The entry with this id.
pub fn by_id(id: &str) -> Option<Assumption> {
    Assumption::ALL.iter().copied().find(|a| a.info().id == id)
}

/// The registry as the table `docs/probes-needed.md` holds between its markers.
pub fn markdown_table() -> String {
    let mut s = String::from("| Id | Assumed | Microsoft tests that need it | Probe wanted |\n|---|---|---|---|\n");
    for a in Assumption::ALL {
        let i = a.info();
        let cell = |t: &str| t.replace('|', "\\|");
        let note = i.note.map(|n| format!(" **{}**", cell(n))).unwrap_or_default();
        let awaiting = i
            .awaiting
            .map(|w| format!(" {}", cell(&awaiting_text(&w))))
            .unwrap_or_default();
        s.push_str(&format!(
            "| `{}` | {}{} | {} | {}{} |\n",
            i.id,
            cell(i.assumed),
            note,
            cell(i.tests),
            cell(i.probe),
            awaiting
        ));
    }
    s
}

/// The registry for `--list-assumptions`; `counts` gives, per id, (flagged passes, other tests that hit it)
/// of the last run.
pub fn listing(counts: &dyn Fn(&str) -> Option<(usize, usize)>) -> String {
    let mut s = String::new();
    for a in Assumption::ALL {
        let i = a.info();
        let hits = match counts(i.id) {
            Some((flagged, other)) => format!("{flagged} flagged passes, {other} other tests"),
            None => "not hit in the last run".to_string(),
        };
        s.push_str(&format!(
            "{}  ({hits})\n    assumed: {}\n    needed by: {}\n    probe wanted: {}\n",
            i.id, i.assumed, i.tests, i.probe
        ));
        if let Some(n) = i.note {
            s.push_str(&format!("    NOTE: {n}\n"));
        }
        if let Some(w) = &i.awaiting {
            s.push_str(&format!("    {}\n", awaiting_text(w)));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_are_collected_per_thread_and_cleared() {
        clear_hits();
        assume(Assumption::ReadUncommittedReadOpensNoWriteTransaction);
        assume(Assumption::ReadUncommittedReadOpensNoWriteTransaction);
        assume(Assumption::CodeCoverageSnapshotWhileLogging);
        assert_eq!(
            take_hits(),
            vec![
                Assumption::ReadUncommittedReadOpensNoWriteTransaction,
                Assumption::CodeCoverageSnapshotWhileLogging
            ]
        );
        assert!(take_hits().is_empty());
        assume(Assumption::ReadUncommittedReadOpensNoWriteTransaction);
        std::thread::spawn(|| assert!(take_hits().is_empty())).join().unwrap();
        clear_hits();
    }

    #[test]
    fn ids_are_unique_stable_words() {
        let mut ids: Vec<&str> = Assumption::ALL.iter().map(|a| a.info().id).collect();
        for id in &ids {
            assert!(
                !id.is_empty() && id.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{id}"
            );
        }
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), Assumption::ALL.len());
        assert!(Assumption::ALL.len() <= 64 * HIT_WORDS, "the marks are a HitBits");
        for a in Assumption::ALL {
            assert_eq!(by_id(a.info().id), Some(*a));
        }
    }

    /// Every `*.rs` file under `crates/*/src`.
    fn sources() -> Vec<(std::path::PathBuf, String)> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if p.is_dir() {
                    if matches!(name, "target" | "tests" | ".git" | "corpus") {
                        continue;
                    }
                    stack.push(p);
                } else if name.ends_with(".rs") && p.to_string_lossy().contains("/src/") {
                    if let Ok(t) = std::fs::read_to_string(&p) {
                        out.push((p, t));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn every_entry_is_marked_somewhere() {
        let files = sources();
        let mut unused = Vec::new();
        for a in Assumption::ALL {
            let needle = format!("Assumption::{a:?}");
            let used = files
                .iter()
                .filter(|(p, _)| !p.ends_with("assumptions.rs"))
                .any(|(_, t)| t.contains(&needle));
            if !used {
                unused.push(a.info().id);
            }
        }
        assert!(
            unused.is_empty(),
            "registered but never marked by a code path (delete the entry or add the marker): {unused:?}"
        );
    }

    /// The al-runner issue numbers a text names: `al-runner issue 5806`, `al-runner issues 5606, 5641`,
    /// `al-runner 5808`, `al-runner #5808`.
    fn al_runner_issues(text: &str) -> Vec<u32> {
        let mut out = Vec::new();
        for (at, _) in text.match_indices("al-runner") {
            let rest = text[at + "al-runner".len()..].trim_start();
            let rest = rest
                .strip_prefix("issues")
                .or_else(|| rest.strip_prefix("issue"))
                .unwrap_or(rest)
                .trim_start();
            // A list of numbers: `5606, 5641`, `#5606 and #5641`.
            let mut s = rest;
            loop {
                let t = s.trim_start_matches(['#', ' ']);
                let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
                let Ok(n) = digits.parse() else { break };
                out.push(n);
                s = &t[digits.len()..];
                s = s
                    .strip_prefix(", ")
                    .or_else(|| s.strip_prefix(" and "))
                    .or_else(|| s.strip_prefix(" or "))
                    .unwrap_or("");
            }
        }
        out
    }

    #[test]
    fn al_runner_issue_references_are_read() {
        assert_eq!(al_runner_issues("as BC does (al-runner issue 5806)"), vec![5806]);
        assert_eq!(
            al_runner_issues("al-runner issues 5606, 5641 and 5700"),
            vec![5606, 5641, 5700]
        );
        assert_eq!(al_runner_issues("Repeater (al-runner 5808)"), vec![5808]);
        assert_eq!(al_runner_issues("al-runner #5808"), vec![5808]);
        assert!(al_runner_issues("AL Runner v2 does it, al-runner-rs too").is_empty());
    }

    /// The problems of one entry's `awaiting`, given the al-runner issues its texts and docs name.
    fn awaiting_problems(id: &str, awaiting: Option<&Awaiting>, named: &[u32]) -> Vec<String> {
        let mut out = Vec::new();
        let Some(w) = awaiting else {
            if !named.is_empty() {
                out.push(format!(
                    "{id}: names al-runner issue(s) {named:?} but has no `awaiting` (a runtime fix for a user issue must say which corpus pin replaces it)"
                ));
            }
            return out;
        };
        // An entry may await a corpus pin that serves no al-runner issue: a writer's probe of an unprobed assumption (corpus PR 737 for
        // the HTTP entries, 2026-10-25). Its issues list is then empty.
        for n in named {
            if !w.issues.contains(n) {
                out.push(format!(
                    "{id}: names al-runner issue {n}, which `awaiting` does not list"
                ));
            }
        }
        if w.files.is_empty() {
            out.push(format!("{id}: `awaiting` names no corpus file"));
        }
        for f in w.files {
            // `file::Test`: one test of a file already in the corpus.
            let f = f.split_once("::").map_or(*f, |(f, _)| f);
            if !f.starts_with("tests/") || !f.ends_with(".al") || f.contains('\\') || f.contains("..") {
                out.push(format!(
                    "{id}: awaited corpus file `{f}` must be a path relative to the corpus root, `tests/<app>/.../<file>.al`"
                ));
            }
        }
        let b = w.since.as_bytes();
        let date = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if !date {
            out.push(format!("{id}: `awaiting.since` `{}` is not a date YYYY-MM-DD", w.since));
        }
        out
    }

    const SAMPLE: Awaiting = Awaiting {
        issues: &[5806],
        files: &["tests/al-language/bgtasks/lt-x/X.al"],
        since: "2026-10-24",
    };

    #[test]
    fn an_entry_that_names_an_al_runner_issue_must_await_its_pin() {
        assert_eq!(awaiting_problems("a", None, &[]), Vec::<String>::new());
        assert_eq!(awaiting_problems("a", None, &[5806]).len(), 1);
        assert!(awaiting_problems("a", Some(&SAMPLE), &[5806]).is_empty());
        assert_eq!(awaiting_problems("a", Some(&SAMPLE), &[5806, 5807]).len(), 1);
        let bad = Awaiting {
            issues: &[],
            files: &["corpus/tests/x.al", "bgtasks/X.al"],
            since: "24.10.2026",
        };
        assert_eq!(awaiting_problems("a", Some(&bad), &[]).len(), 3);
    }

    /// The lines of `docs/probes-needed.md` outside the generated table that name the entry `id` (in backticks).
    fn doc_lines_naming<'a>(doc: &'a str, id: &str) -> Vec<&'a str> {
        let needle = format!("`{id}`");
        let (a, b) = (doc.find("<!-- assumptions:begin"), doc.find(END));
        doc.lines()
            .filter(|l| l.contains(&needle))
            .filter(|l| {
                let at = l.as_ptr() as usize - doc.as_ptr() as usize;
                !matches!((a, b), (Some(a), Some(b)) if at >= a && at <= b)
            })
            .collect()
    }

    #[test]
    fn every_entry_that_names_an_al_runner_issue_awaits_its_pin() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/probes-needed.md");
        let doc = std::fs::read_to_string(&path).expect("docs/probes-needed.md");
        let mut problems = Vec::new();
        for a in Assumption::ALL {
            let i = a.info();
            let mut named = Vec::new();
            for t in [i.assumed, i.tests, i.probe, i.note.unwrap_or("")] {
                named.extend(al_runner_issues(t));
            }
            for l in doc_lines_naming(&doc, i.id) {
                named.extend(al_runner_issues(l));
            }
            named.sort();
            named.dedup();
            problems.extend(awaiting_problems(i.id, i.awaiting.as_ref(), &named));
        }
        assert!(problems.is_empty(), "{}", problems.join("\n"));
    }

    #[test]
    fn the_table_and_listing_name_what_an_entry_awaits() {
        let s = awaiting_text(&SAMPLE);
        assert_eq!(
            s,
            "Awaiting since 2026-10-24: al-runner issue 5806, corpus `tests/al-language/bgtasks/lt-x/X.al`."
        );
    }

    const BEGIN: &str = "<!-- assumptions:begin (generated from crates/al-builtins/src/assumptions.rs; UPDATE_ASSUMPTIONS_DOC=1 cargo test -p al-builtins assumptions rewrites it) -->";
    const END: &str = "<!-- assumptions:end -->";

    #[test]
    fn the_table_in_probes_needed_is_generated_from_the_registry() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/probes-needed.md");
        let text = std::fs::read_to_string(&path).expect("docs/probes-needed.md");
        let want = format!("{BEGIN}\n{}{END}", markdown_table());
        let (Some(a), Some(b)) = (text.find("<!-- assumptions:begin"), text.find(END)) else {
            panic!("docs/probes-needed.md has no assumptions block: add the two marker lines");
        };
        let have = &text[a..b + END.len()];
        if have != want {
            if std::env::var_os("UPDATE_ASSUMPTIONS_DOC").is_some() {
                let new = format!("{}{want}{}", &text[..a], &text[b + END.len()..]);
                std::fs::write(&path, new).expect("write docs/probes-needed.md");
            } else {
                panic!(
                    "the assumptions table of docs/probes-needed.md is out of date: run UPDATE_ASSUMPTIONS_DOC=1 cargo test -p al-builtins assumptions"
                );
            }
        }
    }
}
