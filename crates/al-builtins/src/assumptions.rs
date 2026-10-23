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
                    } ),*
                }
            }
        }
    };
    (@note) => { None };
    (@note $n:expr) => { Some($n) };
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
}

registry! {
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
    QueryClosedAfterLastRow => {
        id: "query-closed-after-last-row",
        assumed: "Open on a query whose Read has returned false after its last row runs the query again, with the filters set since then (a SetRange or SetFilter there is pinned: it takes the filter and a Read before the next Open raises, errors/lt-ms-refusals-131/P131QuerySetRangeAfterRead.al:154-170).",
        tests: "Workflow Step Instance.SetTableRelationValues (one query variable, filtered and opened once per table relation); 190 Tests-Workflow tests (2026-10-19).",
        probe: "Open a query, Read to false, SetRange on a column, Open and Read again (no asserterror in between, it rolls back the rows); the same without the SetRange; Open twice after the last row. Asked in corpus #651.",
    },
    XmlportExportsFlowfieldCalculated => {
        id: "xmlport-exports-flowfield-calculated",
        assumed: "An XmlPort field element or attribute bound to a FlowField exports the calculated value, as if CalcFields had run on the row. (An Xml import skips a Lookup, Sum or Count FlowField element: pinned by xmlport/lt-ay-xmlport/AyXmlPortLookupFlowField.al and XmlPortFlowFieldSumCount.al.)",
        tests: "Import / Export Workflow (\"Workflow Step Argument\".\"Response Option Group\", a Lookup FlowField); 76 Tests-Workflow tests.",
        probe: "An export-only XmlPort with a fieldelement over a Sum FlowField and over a Lookup FlowField of a row with children: the exported text with and without a CalcFields on the row before; an import of the same element. Asked in corpus #650.",
    },
    RecordRefIntoRecord => {
        id: "recordref-into-record",
        assumed: "A RecordRef (held by a Variant) assigned or passed to a Record of the same table gives a copy of the record the RecordRef is on, filters included; another table is refused.",
        tests: "Approvals Mgmt.InformUserOnStatusChange (ShowSalesApprovalStatus(Variant)); 73 Tests-Workflow tests.",
        probe: "V := RecRef (a RecordRef on a filtered row); Rec := V and a Record parameter p(V): the fields, GetFilters, position and a later Modify; the same with a RecordRef of another table. Asked in corpus #643.",
    },
    NavUserAccountHelperIsDelegatedUser => {
        id: "nav-user-account-helper-is-delegated-user",
        assumed: "DotNet NavUserAccountHelper.IsDelegatedUser() answers false for the test session user (the member is in no corpus test; the plan-based Azure AD User Management.IsUserDelegated is false in session/TestEnvironmentUserTenantContracts.al:245).",
        tests: "Azure AD Graph User Impl.IsUserDelegated, reached by 165 Microsoft tests (for example Suggest Price Lines UT T202_NotificationActionFixOpensJobQueueEntryCard).",
        probe: "A DotNet NavUserAccountHelper variable in a test session: IsDelegatedUser(), IsUserDelegatedAdmin(); and Codeunit \"Azure AD Graph User\".IsUserDelegated().",
    },
    FieldRefCalcFieldOfABlob => {
        id: "fieldref-calcfield-of-a-blob",
        assumed: "FieldRef.CalcField on a Blob field is accepted and loads the blob, as Record.CalcFields does (record/TestCalcFieldsFieldClass.Codeunit.al); on another stored field it stays refused.",
        tests: "Temp Blob Impl.FromFieldRef (FieldRef.CalcField on each Blob it reads); 14 Tests-SCM-Manufacturing tests (Demand Forecast Card).",
        probe: "A FieldRef on a Blob field of a stored row read without CalcFields: CalcField() then Value and HasValue; the same on a Code field (the error text) and on a FlowField. Asked in corpus #644.",
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
    NextWholeDecimal => {
        id: "next-whole-decimal",
        assumed: "Record.Next(Decimal) with a whole number steps like the Integer; a fraction is refused.",
        tests: "Phys. Invt. Count.-Management.CalcPeriod (Calendar.Next(12 / CountFrequency)).",
        probe: "R.Next(1.5) and R.Next(-1.5). Asked in corpus #643.",
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
    CookieSetterResult => {
        id: "cookie-setter-result",
        assumed: "Cookie.Name(NewName) and Cookie.Value(NewValue), with the result used, answer the new text.",
        tests: "platform/lt-xmlport-session-navapp-cookie/P94Cookie.al assigns the result to a Text it never reads (SetName_DoesNotTouchOtherMembers, Copy_AndClear).",
        probe: "Format of the result of C.Name('n') and C.Value('v') on a cookie that had other values before. Asked in corpus #650.",
    },
    PageExpressionControlSetValue => {
        id: "page-expression-control-setvalue",
        assumed: "TestPage.SetValue on a field control whose source is an assignable path (a record in an array, `Totals[1].\"Inv. Discount Amount\"`) assigns the path, then runs the control's OnValidate; the value is converted to the type of the value the path holds now. An entry the client would refuse is refused with the variable-control texts.",
        tests: "Purchase and Sales Order Statistics (`InvDiscountAmount_General`): ConfirmationOnInvDiscUpdateInPurchOrderWithInvoicedLine and about 55 more Tests-ERM tests.",
        probe: "A page field(Ctl; Arr[1].Amount) with an OnValidate; TestPage.Ctl.SetValue(5) then the global and the OnValidate effect; SetValue('x') on it. Asked in corpus #641.",
    },
    GuidAsTextArgument => {
        id: "guid-as-text-argument",
        assumed: "A Guid passed where a function wants a Text is its text form {XXXXXXXX-...}, as in an assignment. Pinned for CopyStr, StrLen, UpperCase, LowerCase, StrPos and Notification.SetData (types/lt-guid-as-text-argument/GuidAsTextArgument.al); every other Text parameter is assumed alike.",
        tests: "Match General Jnl Lines UT (CopyStr(CreateGuid(), 1, 50)); 61 Tests-General Journal tests. Error Message Handler.ShowNotification (Notification.SetData with a Guid); 27 Tests-ERM tests.",
        probe: "A Guid passed to other Text parameters: StrSubstNo('%1', ...) as the format, DelStr, PadStr, ConvertStr, IncStr, SelectStr, Text.Contains.",
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
    QueryColumnRecordId => {
        id: "query-column-record-id",
        assumed: "A query column over a RecordId field returns the stored RecordId, groups and orders by it like the field, and cannot carry a Method, a ColumnFilter or ReverseSign (refused).",
        tests: "Base Application's \"Workflow Instance\" query (column Record_ID), opened by Workflow Step Instance.SetTableRelationValues and Approvals Mgmt.; 142 Tests-Workflow tests.",
        probe: "A query with a column over a RecordId field of a table with rows: Open, Read, Format of the column, grouping and OrderBy on it, SetFilter on the column, SaveAsCsv. Asked in corpus #651.",
    },
    RecordRefGetTableOfRecordRef => {
        id: "recordref-gettable-of-recordref",
        assumed: "RecordRef.GetTable(OtherRecordRef) makes the ref a copy of the other ref's record: table, buffer, filters and current row, not shared afterwards.",
        tests: "Tests-ERM SalesInvoiceTryPreview and the other posting preview tests (Gen. Jnl.-Post Preview): about 30 tests.",
        probe: "B.GetTable(A) with A open on a table with a filter and a found row; change A, read B; with A not open; with B open on another table. Asked in corpus #644.",
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
        assumed: "The open read of a list or part with OnFindRecord, OnNextRecord and OnAfterGetRecord but no OnAfterGetCurrRecord, whose first window of 20 rows is full and whose OnNextRecord(-1) from the first row answers a step, ends as a list with OnAfterGetCurrRecord does: OnFindRecord('=') and the first row read again (`R20;R1;N(-1);F(=);R1`). Pinned only with OnAfterGetCurrRecord (PnF06FindListMoves.al FindList_Ascending_25Rows) and, without it, for an OnNextRecord(-1) that answers 0.",
        tests: "ERM Base Calendar (BaseCalendarEntriesShowCorrespondingRecords): Base Calendar Entries Subform keeps its position on the Date table in a global that OnAfterGetRecord sets.",
        probe: "A temporary ListPart whose OnFindRecord and OnNextRecord walk a position global (rows on every day, inserted on demand) and whose OnAfterGetRecord sets that global from Rec, logging each call; the host card refills it from its OnAfterGetRecord (DeleteAll, CurrPage.Update). Log the reads at open and after the host moves with Next, and read a part field before and after. The same list as a page of its own, opened on day 10 of 40.",
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
    DurationDividedByDecimal => {
        id: "duration-divided-by-decimal",
        assumed: "A Duration divided by a Decimal counts the Duration as its milliseconds and answers a Decimal (no whole-millisecond requirement, unlike the pinned division by an Integer).",
        tests: "Microsoft's Calculate Routing Line CalcDuration (Time difference / CalendarMgt.TimeFactor): Tests-SCM-Manufacturing, SCM Production Journal CheckAllocatedCapacityTimeCalculation and about 20 more.",
        probe: "D: Duration := 26820000 and 30000; X := D / Dec60000 into a Decimal; Format(D / Dec60000); a Decimal divisor that is zero. Asked in corpus #649.",
    },
    DateToDateTime => {
        id: "date-to-datetime-is-midnight",
        assumed: "A Date assigned or converted to a DateTime is that day at midnight; the blank Date is the blank DateTime.",
        tests: "Microsoft's production order and routing code (Starting Date-Time := Starting Date): Tests-SCM-Manufacturing, about 20 tests.",
        probe: "DT := D for a defined and for a blank Date; Format(DT, 0, 9); DT := ClosingDate(D). Asked in corpus #649.",
    },
    BooleanToTextAsFormat => {
        id: "boolean-to-text-as-format",
        assumed: "A Boolean given where a Text or Code is expected (Record.Get(Boolean) on a Code key) converts as Format writes it: Yes or No, upper case for a Code.",
        tests: "Item Tracking Code Get(Serial) in the SCM kitting tests; about 90 tests of the Tests-SCM-Assembly bucket.",
        probe: "Get(true) on a table with a Code[10] key holding the rows 'Yes' and 'YES' and '1'; assign a Boolean to a Text and a Code variable. Asked in corpus #649.",
    },
    FormatCodeSpelling => {
        id: "format-code-spelling",
        assumed: "In a Format string `Filler Char` stands for `Filler Character`. (Any case of the other code names, and blanks after the comma, are pinned by types/lt-format-codes-textbuilder-pagerun-labels/P144FormatCodes.al.)",
        tests: "The Data Exchange tests' GetNodeID (`<Integer,4><Filler Char,0>`, about 30 tests).",
        probe: "Format(5, 0, '<Integer,4><Filler Char,0>') next to '<Integer,4><Filler Character,0>', for an Integer, a Decimal and a Text. Asked in corpus #649.",
    },
    DateTimeDateTimeMethods => {
        id: "datetime-date-time-methods",
        assumed: "The instance methods DateTime.Date() and DateTime.Time() give what DT2Date and DT2Time give, the undefined DateTime included. (DT2Date and DT2Time are pinned by text/TestProbeTextDateFormatContracts.al:Dates_DateTime_DT2DateTime.)",
        tests: "About 975 Tests-ERM, Tests-SCM and Tests-General Journal tests (Library - Random and Library - Utility date and time helpers).",
        probe: "Dt.Date() and Dt.Time() for a set DateTime and for 0DT, next to DT2Date(Dt) and DT2Time(Dt). Asked in corpus #649.",
    },
    ReportViewOverStoredRequestFilters => {
        id: "report-view-over-stored-request-filters",
        assumed: "A data item the caller gave a view (SetTableView, or the record of Report.Run or RunModal) keeps that view when the report has SaveValues and an earlier run stored other filters for the data item. (That an OK stores the view's filters is pinned by report/lt-requestpage-savevalues-date-text-setvalue/P147SaveValuesFilter.al:P147_Filter_SetTableViewVsStored.)",
        tests: "Tests-ERM tests that run `Suggest Fin. Charge Memo Lines` for a second memo in one codeunit (IssueFinanceChargeMemoWith* and others).",
        probe: "A report with SaveValues: run it once with the filter A* stored by OK, then with SetTableView(C*) and see the data item's filter on the request page and in the run; the same with the record of Report.RunModal. Asked in corpus #646.",
    },
    HttpRelativeRequestPath => {
        id: "http-relative-request-path",
        assumed: "HttpClient.Get/Post/Put/Delete/Patch with a path that is not an absolute uri joins it to the client's base address like new Uri(base, path), as a [HttpClientHandler] sees it: a rooted path replaces the base path, any other joins the base path's directory, and dot segments are removed. Pinned (no assumption) for a base address ending in a slash and a plain word, a rooted word, ../ and a word, or a query alone (al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al:BaseAddress_RelativePaths_1 and _2, newest major).",
        tests: "Microsoft and NP Retail code that sets SetBaseAddress and then Get('/orders').",
        probe: "SetBaseAddress('https://example.invalid/api') (no trailing slash) then Get('orders'); SetBaseAddress('https://example.invalid/api/v1/') then Get('a/b'), Get('./x'), Get('a/../b'), Get('../../x'), Get('orders?x=1#f') with a handler printing Request.Path.",
    },
    HttpHeadersAddUsedResult => {
        id: "http-headers-add-used-result",
        assumed: "HttpHeaders.Add whose result is used answers false and leaves the last error as it was. (The false is pinned for refused values, names and collections by network/lt-ay-http/AyHeaderAddResult.al; the last error after it is not measured.)",
        tests: "Code that tests the result of HttpHeaders.Add and then reads GetLastErrorText.",
        probe: "ClearLastError(); if not H.Add('Accept', 'json') then Message(GetLastErrorText()), and the same for an empty name and a content header on request headers.",
    },
    HttpContentLengthAfterRead => {
        id: "http-content-length-after-read",
        assumed: "ReadAs(Text) of a content adds a Content-Length header of the body's length to its headers, and the headers handed out before the read show it too. (PnE12HttpContent.al:WriteFromText_ThenReadAndHeaders reads 5 after WriteFrom('hello') and ReadAs; network/lt-ay-http/AyContentLength.al pins the header for headers taken after the read, none after ReadAs(InStream), and that a later WriteFrom('') or a body of the same length leaves it as it was; a body of another length is refused.)",
        tests: "Code that reads a content and then its headers; ReadAs(InStream) is not covered.",
        probe: "WriteFrom('hello'), GetHeaders, Contains('Content-Length') before and after ReadAs(Text) and ReadAs(InStream); WriteFrom a longer text after the read and read the header. Asked in corpus #650.",
    },
    HttpHeaderRules => {
        id: "http-header-rules",
        assumed: "HttpHeaders.Add checks a header the way the probes measured: the name is a token (empty: Argument, otherwise Format), the value has no line break, a content header is refused on request, response and client headers, and the typed header values are checked as the probes measured them (network/lt-ay-http/AyHeaderAddResult.al and the P87 files measured a few valid and invalid values of each); a value of another shape follows the same rule, and the headers .NET types that have no rule here (Via, Warning, TE, ...) and the typed headers on a collection the probes did not use are accepted as written.",
        tests: "Every Microsoft or NP Retail test that builds a request with headers (Authorization, Accept, Content-Type, custom X- headers).",
        probe: "network/lt-http/HttpHeaders.al covers the measured part; the case and the content-or-not of all 39 known names are pinned by network/lt-http-header-names-cache-control/HeaderNameCase*.al. Wanted: the value rules of the typed headers other than Cache-Control (Range, If-Match, Content-Disposition, Content-Encoding, Expires and the rest) on request, response and content headers; Cache-Control (directive order, values, merging) is pinned by CacheControl*.al. Asked in corpus #650.",
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
    DeleteAllMarkedOnly => {
        id: "delete-all-marked-only",
        assumed: "Record.DeleteAll with MarkedOnly(true) deletes the marked rows that pass the filters and no other row, like Find, Next, Count and ModifyAll work on the marked set; with triggers and events as a DeleteAll with a filter.",
        tests: "Post Inventory Cost to G/L (PostValueEntryToGL.MarkedOnly(true); DeleteAll() in OnPostDataItem, then CapValueEntryLoop Gets the capacity entries left): SCM Kitting D3 Part 2 and SCM Costing Batch, and every test that posts inventory cost with the report.",
        probe: "Mark two of three rows, MarkedOnly(true), DeleteAll: which rows remain; the same with a filter that excludes a marked row, and DeleteAll(true) with an OnDelete trigger. Asked in corpus #644.",
    },
    GroupedQueryTiesInGroupingOrder => {
        id: "grouped-query-ties-in-grouping-order",
        assumed: "Groups of an aggregated query that tie on every OrderBy column are read in the order of the query's grouping columns (column declaration order, each ascending).",
        tests: "Create Pick's CalcQtyInWhseEntries (OrderBy Block_Movement, ten grouping columns): SCM Extend Warehouse CheckItemAvailabilityWhenCreatingPickAndExistReservedQtyForProdOrder and the tests that create a pick for a directed put-away location; about 77 Tests-SCM tests.",
        probe: "An aggregated query with an OrderBy on a column most groups share and several grouping columns, rows inserted in non-key order: the order of the tied groups on Read. Asked in corpus #651.",
    },
    ReportItemCalcFieldsNestedOrList => {
        id: "report-dataitem-calcfields-nested-or-list",
        assumed: "The CalcFields property of a report data item that is nested under another item, or that lists several fields, calculates every listed FlowField of each row before the row's triggers, as for a top-level item with one field (pinned by misc/lt-probes-needed-8/PnH3Link.al).",
        tests: "Get Inbound Source Documents (CalcFields = \"Put-away Qty.\", \"Put-away Qty. (Base)\" on Posted Whse. Receipt Line): about 32 Tests-SCM tests.",
        probe: "A report with a nested item that has CalcFields = A, B (two FlowFields), and a reportextension-added item with CalcFields: the values in OnAfterGetRecord; one name that is not a FlowField. Asked in corpus #646.",
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
        assumed: "ToString and Format of the exception GetLastErrorObject answers: the lines after the first are the AL frames of the error (or one line saying they are not tracked); BC writes .NET server frames. Only the first line `<class>: <message>` and the `   at ` marker are pinned.",
        tests: "Microsoft tests that read ToString or Format of a last-error exception.",
        probe: "Format(GetLastErrorObject()) after Error('x') in a codeunit run from a test: every line,. Asked in corpus #652.",
    },
    PageOpenCaptionClassExpressions => {
        id: "page-open-caption-class-expressions",
        assumed: "A page whose source table has fields with a CaptionClass expression runs every one of them with an empty record when it opens, then once for each control over such a field with the first row; the other rows the open reads and a Next() run none, and Caption() of the control reads what the open gave. Pinned (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al Page_ControlCaptions, QF_*, and corpus bump 45: QO_Doc_OpenEdit_Open, QO_Doc_OpenEdit_PartSteps, QO_Doc_OpenNew, QO_Card_OpenEdit, :958-1137) for a Card opened with OpenView or OpenEdit, a List opened with OpenView, OpenEdit or Page.Run, a Document page opened with OpenEdit or OpenNew, a ListPart on such a Card or Document page, a SetValue (runs nothing), a part refilled after its host moved (its first row only, nothing when it has no row), and Next() after a change on a Card or a ListPart (once with the saved row), and corpus bump 46 matched: an action with CurrPage.Update on a Document page after a SetValue of the host runs the host's three times, then the part's once (QO_Doc_OpenEdit_HostSteps `update`). Assumed: other page types and open modes, other fills after the open, an action after a SetValue whose trigger has no CurrPage.Update (the part's are not run again), and a refresh after an action on a page without a SetValue before it.",
        tests: "Microsoft tests that open sales and purchase documents and their subforms (Sales Line, Purchase Line \"No.\" and amount fields have CaptionClass = GetCaptionClass(..)), for example ERM Sales Order and ERM Purchase Order tests in Tests-ERM; they pass in Microsoft's CI.",
        probe: "CurrPage.Update on a Document page without a SetValue before it, and an action without CurrPage.Update after a SetValue: how many times the host's and the part's expressions run, and in which order.",
    },
    PageOpenCaptionClassEmptyRecordCount => {
        id: "page-open-caption-class-empty-record-count",
        assumed: "A page open that runs the CaptionClass expressions of its source table's fields with an empty record runs each one 20 times. The corpus pins only that such runs happen: their number differs between BC runs (240, 480 and 492 calls on the newest major for the same open, record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:422 Page_ControlCaptions `open_calls`), so it is no oracle. Every open that runs them marks this assumption (owner decision 2026-10-21: refusing them costs 11 baseline cases of the newest major and 5 QF cases).",
        tests: "The corpus cases of QdCaptionClassAreas.al that open a page whose source table has CaptionClass expressions, and Microsoft tests that open sales and purchase documents and their subforms (Sales Line and Purchase Line fields with CaptionClass = GetCaptionClass(..)) in Tests-ERM; they pass in Microsoft's CI.",
        probe: "A stable count, if BC has one: the same open repeated on a fresh service tier and after a warm-up, on every major, to tell whether the number depends on caching in the tier.",
    },
    WorksheetTempSourceOpenRow => {
        id: "worksheet-temp-source-open-row",
        assumed: "Which row a Worksheet page over a temporary source opens on, when it is opened without a record and its OnOpenPage moved Rec or changed its key, order or filters, leaving it on a row other than the first of its view. Two paths, both assumed: a Rec in the primary key, outside lookup mode and without OnNextRecord, from which `=><` lands on a row other than the first, opens on that row and reads its rows as a List does; every other such Worksheet (a key changed, lookup mode, OnNextRecord, or `=><` landing on the first row) opens on its first row. Pinned, and no longer assumed: a row a procedure of the page variable set before the open, which OnOpenPage left as it was (pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:2378-2705, the QT_Temp* tests); the List and the Card whose OnOpenPage fills them (:409 QR_TempCard_OpenView_NoMove, :474 QR_TempList_OpenView_NoMove).",
        tests: "The first-row path is reached by Item Tracking Lines (a temporary Worksheet whose OnOpenPage changes the key) in many tests of Tests-SCM-Reservation, Tests-SCM-Warehouse, Tests-SCM and Tests-ERM; they pass in Microsoft's CI.",
        probe: "A Worksheet page over a temporary table whose OnOpenPage fills it or moves Rec (Get of row 2, FindLast), and one whose OnOpenPage calls SetCurrentKey and then Get: run modally, in lookup mode and with Page.Run, with and without OnNextRecord: which row the TestPage shows, and the OnAfterGetRecord/OnAfterGetCurrRecord order.",
    },
    ListOtherKeyOtherMoves => {
        id: "list-other-key-other-moves",
        assumed: "A plain list (no OnFindRecord or OnNextRecord, with OnAfterGetRecord and OnAfterGetCurrRecord) that opened in a key other than the primary key reads each row once more at the open and at Next and Previous (pinned: pages/lt-qr-onopenpage-moves-rec/QrMovedTests.al:345 QR_List_OpenView_SetCurrentKeyGet and the QS_Key*_List_* tests from :856). Assumed: First, Last, GoToKey, GoToRecord and a refill of its rows while Rec stays in that key and order read as in the primary key, without the extra read; so does the open of such a list whose view has one row or more rows than a window holds (unless OnOpenPage left Rec on a row other than the first: refused).",
        tests: "Corpus baseline case 67990 PrecompiledHostLinkedPartShowsTheRowTheViewExcluded (handlers/TestPartSubPageViewPrecompiled.al:71): GoToRecord on Microsoft's G/L Account Categories page, a list in a key other than the primary key; it passes on real BC and does not observe the reads. Tests-ERM O365 Correct Sales Invoice (CancelInvoiceActionInvisibleOnCancelledPostedSalesInvoiceList, TestListPageActionCorrectInvoice) and Payments using Creditor Number (ApplyPurchInvWithCreditorInfoManual, UpdateVendorLedgerEntriesCreditorNoNumericOnlyDisabled) open such lists; they pass in Microsoft's CI.",
        probe: "A list with SourceTableView = sorting(Name) and OnAfterGetRecord/OnAfterGetCurrRecord logging, opened with OpenView: the log of First, Last, GoToKey and GoToRecord, and of a CurrPage.Update, compared with the same page in the primary key.",
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
        assumed: "In the company build (`--company-data demotool`), DotNet System.IO.Directory.GetFiles (File Management.GetServerDirectoryFilesList) of a folder AL code created in the temporary folder lists the files written directly in it, in name order without case, joined to the folder with `/`. Windows BC of the newest major lists the files MediaSet.ExportFile wrote, full path, no extension (windows-check-results 2026-10-23 RJ item B, corpus cu 61286 RJ_MediaSet_ExportFile_ConfigPackageShape), and lists every folder in name order without case, never in creation order (windows-check-results 2026-10-23 listing order, cu 61287 LO_ListingOrder: created c,b,a lists a,b,c; a,C,b lists a,b,C; B,a lists a,B; two MediaSet.ExportFile sets whose Guids sort against creation order list in name order). The bc-linux image now lists the same way; corpus bump 62 pins it and then replaces this entry. An empty folder stays refused.",
        tests: "Every test of Microsoft's Legacy test lane on a release whose artifact has no RapidStart package: the DemoTool's Export RapidStart Packages step reads the Media folder back (Config. XML Exchange.UpdateConfigPackageMediaSet, ConfigXMLExchange.Codeunit.al:1505-1522), and they pass in Microsoft's CI.",
        probe: "Measured on Windows BC of the newest major (cu 61287 LO_ListingOrder); the asserted corpus tests arrive with corpus bump 62. Still open: the two older majors.",
    },
}

thread_local! {
    static HITS: Cell<u128> = const { Cell::new(0) };
}

/// Notes that the code path of `a` ran. Cheap: one thread-local read and write. The runner collects the
/// marks of each test with [`take_hits`].
#[inline]
pub fn assume(a: Assumption) {
    HITS.with(|h| h.set(h.get() | (1u128 << (a as u32))));
}

/// [`assume`] that answers true, for a match guard that decides to take the assumed path.
#[inline]
pub fn hit(a: Assumption) -> bool {
    assume(a);
    true
}

/// Forgets the marks of this thread (a test is about to start).
pub fn clear_hits() {
    HITS.with(|h| h.set(0));
}

/// The marks of this thread since the last [`clear_hits`] or `take_hits`, in registry order; clears them.
pub fn take_hits() -> Vec<Assumption> {
    let bits = HITS.with(|h| h.replace(0));
    Assumption::ALL
        .iter()
        .copied()
        .filter(|a| bits & (1u128 << (*a as u32)) != 0)
        .collect()
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
        s.push_str(&format!(
            "| `{}` | {}{} | {} | {} |\n",
            i.id,
            cell(i.assumed),
            note,
            cell(i.tests),
            cell(i.probe)
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
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_are_collected_per_thread_and_cleared() {
        clear_hits();
        assume(Assumption::QueryClosedAfterLastRow);
        assume(Assumption::QueryClosedAfterLastRow);
        assume(Assumption::NextWholeDecimal);
        assert_eq!(
            take_hits(),
            vec![Assumption::QueryClosedAfterLastRow, Assumption::NextWholeDecimal]
        );
        assert!(take_hits().is_empty());
        assume(Assumption::QueryClosedAfterLastRow);
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
        assert!(Assumption::ALL.len() <= 128, "the marks are a u128");
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
