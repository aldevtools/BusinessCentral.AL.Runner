//! The rows a tenant holds before any AL runs, for the stored platform tables whose rows AL reads
//! as facts about the session: the Company, the User and the SUPER Access Control row (ADR-013
//! decision 5, decision 9; AL Runner `RecordPatches.CompanySystemTable`, `UserSystemTable`,
//! `AccessControlSeed`).
//!
//! They are ordinary rows of ordinary tables. AL may read, change and delete them, and a test may
//! insert more (record/TestRecordChangeCompanyRows.al inserts and deletes a company). They are
//! written and committed when the session starts, so a rollback never removes them.

use super::{Ident, pos};
use crate::Session;
use al_builtins::native::Ctx;
use al_builtins::value::{RtType, Value};
use al_data::{Row, Store, TableDef};

pub(super) const COMPANY: u32 = 2_000_000_006;
pub(super) const USER: u32 = 2_000_000_120;
pub(super) const ACCESS_CONTROL: u32 = 2_000_000_053;
pub(super) const USER_PERSONALIZATION: u32 = 2_000_000_073;
pub(super) const USER_PROPERTY: u32 = crate::usertable::USER_PROPERTY;
pub(super) const WEB_SERVICE: u32 = 2_000_000_076;
pub(super) const TENANT_WEB_SERVICE: u32 = 2_000_000_168;

pub(super) const TENANT_REPORT_LAYOUT_SELECTION: u32 = 2_000_000_233;
pub(super) const ODATA_EDM_TYPE: u32 = 2_000_000_179;

/// The Tenant Report Layout Selection rows of a fresh tenant (platform/lt-platform-tables/PtCloudTables.al:378-424,
/// QX_TenantReportLayoutSelection_RowsAndDemoShape, 'rows'): (Report ID, Layout Name), blank Company Name and null User ID.
/// The App ID is the Base Application's (`BASE_APPLICATION_APP_ID`).
const BASE_APPLICATION_APP_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

const TENANT_REPORT_LAYOUT_SELECTIONS: &[(i32, &str)] = &[
    (1302, "StandardSalesProFormaInv.docx"),
    (1303, "StandardDraftSalesInvoiceBlue.docx"),
    (1304, "StandardSalesQuoteBlue.docx"),
    (1306, "StandardSalesInvoiceBlueSimple.docx"),
];

/// The OData Edm Type rows of a fresh tenant (PtCloudTables.al:435-482, QX_ODataEdmType_RowsAndEdmXml, 'fresh keys' and
/// 'fresh count' 6): (Key, Description, Edm Xml as UTF-8 text). Description and Edm Xml: :533 RE_ODataEdmType_FreshDescriptions
/// and :572-606 (one test per key), the same on BC 27, 28 and 29.
const ODATA_EDM_TYPES: &[(&str, &str, &str)] = &[
    (
        "BOOKINGSDATE",
        "Graph CDM - Bookings Date complex type",
        r#"<ComplexType Name="dateTimeTimeZoneType">  <Property Name="dateTime" Type="Edm.String" Nullable="false" />  <Property Name="timeZone" Type="Edm.String" /></ComplexType>"#,
    ),
    (
        "DIMENSION",
        "Graph CDM - Dimension complex type.",
        r#"<ComplexType Name="dimensionType"><Property Name="code" Type="Edm.String" Nullable="false" /><Property Name="displayName" Type="Edm.String" Nullable="true" /><Property Name="valueCode" Type="Edm.String" Nullable="false" /><Property Name="valueDisplayName" Type="Edm.String" Nullable="true" /></ComplexType>"#,
    ),
    (
        "DOCUMENTLINEOBJECTDETAILS",
        "Graph API - Complex type exposing details of the line object that is being sold.",
        r#"<ComplexType Name="documentLineObjectDetailsType"><Property Name="number" Type="Edm.String" Nullable="true" MaxLength="20"/><Property Name="displayName" Type="Edm.String" Nullable="true" MaxLength="100"/></ComplexType>"#,
    ),
    (
        "ITEM-UOM",
        "Graph CDM - Unit of Measure complex type on Item Entity page",
        r#"<ComplexType Name="unitOfMeasureType"><Property Name="code" Type="Edm.String" Nullable="true" MaxLength="10" /><Property Name="displayName" Type="Edm.String" Nullable="true" MaxLength="50" /><Property Name="symbol" Type="Edm.String" Nullable="true" MaxLength="10" /><Property Name="unitConversion" Type="ITEM-UOM-CONVERSION" Nullable="true" /></ComplexType>"#,
    ),
    (
        "ITEM-UOM-CONVERSION",
        "Graph CDM - Unit of Measure Conversions complex type on Item Entity page",
        r#"<ComplexType Name="itemUnitOfMeasureConversionType"><Property Name="toUnitOfMeasure" Type="Edm.String" Nullable="true" MaxLength="10" /><Property Name="fromToConversionRate" Type="Edm.Decimal" Nullable="true" /></ComplexType>"#,
    ),
    (
        "POSTALADDRESS",
        "Graph CDM - Postal Address complex type",
        r#"<ComplexType Name="postalAddressType"><Property Name="street" Type="Edm.String" Nullable="true" MaxLength="152" /><Property Name="city" Type="Edm.String" Nullable="true" MaxLength="30" /><Property Name="state" Type="Edm.String" Nullable="true" MaxLength="30" /><Property Name="countryLetterCode" Type="Edm.String" Nullable="true" MaxLength="10" /><Property Name="postalCode" Type="Edm.String" Nullable="true" MaxLength="20" /></ComplexType>"#,
    ),
];

/// The Web Service rows of a fresh tenant, every column (onprem platform/PtWebServiceRows.al:39-73, WebService_Content:
/// 17 rows): (Object Type ordinal, Service Name, Object ID). Published is Yes and both Exclude columns No on every row.
/// They are rows of the database the service tier starts with, not written by AL of an app.
const WEB_SERVICES: &[(i32, &str, i32)] = &[
    (5, "CompanySetupService", 1801),
    (5, "ExchangeServiceSetup", 5324),
    (5, "PageActionProvider", 2915),
    (5, "SummaryProvider", 2718),
    (8, "AccountantPortalActivityCues", 1314),
    (8, "AccountantPortalFinanceCues", 1315),
    (8, "AccountantPortalUserTasks", 1316),
    (8, "powerbifinance", 197),
    (8, "SalesOrder", 42),
    (8, "UserTaskSetComplete", 1170),
    (9, "DimensionSets", 257),
    (9, "ItemSalesAndProfit", 103),
    (9, "ItemSalesByCustomer", 102),
    (9, "SalesDashboard", 101),
    (9, "SalesOpportunities", 105),
    (9, "SalesOrdersBySalesPerson", 104),
    (9, "TopCustomerOverview", 100),
];

/// The Tenant Web Service rows of a fresh tenant, every column (platform/lt-platform-rows-fresh/PlatformRowsWebService.al:66-199,
/// 64 rows, count at :215): (Object Type ordinal, Service Name, Object ID). Published is Yes and both Exclude columns No on every
/// row. Microsoft's DemoTool calls CreateTenantWebService for rows that exist and changes nothing (onprem
/// platform/PtWebServiceRows.al:229, RD_CreateTenantWebService_DemoToolCalls).
const TENANT_WEB_SERVICES: &[(i32, &str, i32)] = &[
    (5, "WorkflowActionResponse", 1544),
    (8, "Chart of Accounts", 16),
    (8, "ExcelTemplateAgedAccountsPayable", 5500),
    (8, "ExcelTemplateAgedAccountsReceivable", 5499),
    (8, "ExcelTemplateBalanceSheet", 5501),
    (8, "ExcelTemplateCashFlowStatement", 5493),
    (8, "ExcelTemplateIncomeStatement", 5503),
    (8, "ExcelTemplateRetainedEarnings", 5497),
    (8, "ExcelTemplateTrialBalance", 5502),
    (8, "ExcelTemplateViewCompanyInformation", 1320),
    (8, "Job List", 89),
    (8, "Job Planning Lines", 1007),
    (8, "Job Task Lines", 1002),
    (8, "Power BI Aged Acc. Payable", 6307),
    (8, "Power BI Aged Acc. Receivable", 6308),
    (8, "Power BI Aged Inventory Chart", 6309),
    (8, "Power BI Job Act. v. Budg. Cost", 6312),
    (8, "Power BI Job Act. v. Budg. Price", 6310),
    (8, "Power BI Job Profitability", 6311),
    (8, "Power BI Sales Pipeline", 6314),
    (8, "Power BI Top 5 Opportunities", 6315),
    (8, "Power BI WorkDate Calc.", 6313),
    (8, "purchaseDocumentLines", 6405),
    (8, "purchaseDocuments", 6404),
    (8, "salesDocumentLines", 6403),
    (8, "salesDocuments", 6402),
    (8, "workflowCustomers", 6408),
    (8, "workflowGenJournalBatches", 6406),
    (8, "workflowGenJournalLines", 6407),
    (8, "workflowItems", 6409),
    (8, "workflowPurchaseDocumentLines", 6405),
    (8, "workflowPurchaseDocuments", 6404),
    (8, "workflowSalesDocumentLines", 6403),
    (8, "workflowSalesDocuments", 6402),
    (8, "workflowVendors", 6410),
    (8, "workflowWebhookSubscriptions", 831),
    (9, "BankAccountLedgerEntries", 264),
    (9, "Cust.LedgerEntries", 262),
    (9, "DimensionSetEntries", 260),
    (9, "FALedgerEntries", 267),
    (9, "G/LBudgetEntries", 270),
    (9, "G/LEntries", 261),
    (9, "ItemLedgerEntries", 265),
    (9, "JobLedgerEntries", 268),
    (9, "Power BI Cust. Item Ledg. Ent.", 61),
    (9, "Power BI Cust. Ledger Entries", 62),
    (9, "Power BI Customer List", 50),
    (9, "Power BI GL Amount List", 53),
    (9, "Power BI GL BudgetedAmount", 58),
    (9, "Power BI Item Purchase List", 52),
    (9, "Power BI Item Sales List", 57),
    (9, "Power BI Jobs List", 54),
    (9, "Power BI Purchase Hdr. Vendor", 64),
    (9, "Power BI Purchase List", 56),
    (9, "Power BI Sales Hdr. Cust.", 60),
    (9, "Power BI Sales List", 55),
    (9, "Power BI Top Cust. Overview", 59),
    (9, "Power BI Vend. Item Ledg. Ent.", 65),
    (9, "Power BI Vendor Ledger Entries", 63),
    (9, "Power BI Vendor List", 51),
    (9, "Res.LedgerEntries", 269),
    (9, "SegmentLines", 106),
    (9, "ValueEntries", 266),
    (9, "VendorLedgerEntries", 263),
];

/// The Full Name of the session user's User row. The corpus pins only that it is not blank and is not the user name
/// (system-fields/W6SystemFieldStamps.al:240, `is_user_id=No` and `blank=No` for SystemCreatedByFullName, which holds it);
/// the text itself is AL Runner's (docs/probes-needed.md, Identity).
pub(crate) const SESSION_USER_FULL_NAME: &str = "Test User";

/// A text value for the field `id`, as a Code or a Text by the field's type.
fn text(t: &TableDef, id: u32, s: &str) -> Value {
    match t.fields.iter().find(|f| f.id == id).map(|f| &f.ty) {
        Some(RtType::Code(_)) => Value::Code(s.to_string().into()),
        _ => Value::Text(s.to_string().into()),
    }
}

impl Session {
    /// Inserts one seed row of a table that holds several, with a SystemId like any inserted row.
    fn insert_seed(&mut self, t: &TableDef, mut r: Row) {
        if let Some(p) = t.fields.iter().position(|f| f.id == 2_000_000_000) {
            r[p] = Value::Guid(self.fresh_system_id());
        }
        let _ = self.store.insert(&self.cx, t, r);
    }

    /// Writes the platform's seed rows into the tables the program compiled, then commits.
    pub(crate) fn seed_platform_rows(&mut self, i: &Ident) {
        let engine = self.engine.clone();
        let mut wrote = false;
        for (oi, o) in engine.program.objects.heads().iter().enumerate() {
            let (Some(id), al_ir::ObjKind::Table) = (o.id, if o.uncompiled { al_ir::ObjKind::Other } else { o.kind })
            else {
                continue;
            };
            let Some(t) = engine.tables[oi].as_ref() else { continue };
            let row: Row = match id {
                super::profiles::ALL_PROFILE => {
                    wrote |= self.seed_profiles(t, i);
                    continue;
                }
                super::profile_settings::TENANT_PROFILE_SETTING => {
                    wrote |= self.seed_profile_settings(t, i);
                    continue;
                }
                TENANT_REPORT_LAYOUT_SELECTION => {
                    for &(report, layout) in TENANT_REPORT_LAYOUT_SELECTIONS {
                        let mut r = super::blank(t);
                        r[pos(t, 1)] = Value::Integer(report);
                        r[pos(t, 2)] = text(t, 2, layout);
                        // The Base Application's id on every row (platform/lt-platform-rows-fresh/PlatformRowsLayoutSelection.al:82-85,
                        // BC 27, 28 and 29).
                        r[pos(t, 3)] =
                            Value::Guid(al_builtins::ops::parse_guid(BASE_APPLICATION_APP_ID).expect("a literal guid"));
                        self.insert_seed(t, r);
                    }
                    wrote = true;
                    continue;
                }
                ODATA_EDM_TYPE => {
                    for &(key, description, edm_xml) in ODATA_EDM_TYPES {
                        let mut r = super::blank(t);
                        r[pos(t, 1)] = text(t, 1, key);
                        r[pos(t, 2)] = text(t, 2, description);
                        let blob = al_builtins::stream::new_blob();
                        al_builtins::stream::write_bytes(&blob, edm_xml.as_bytes());
                        r[pos(t, 10)] = Value::Blob(blob);
                        self.insert_seed(t, r);
                    }
                    wrote = true;
                    continue;
                }
                WEB_SERVICE | TENANT_WEB_SERVICE => {
                    let rows = if id == WEB_SERVICE {
                        WEB_SERVICES
                    } else {
                        TENANT_WEB_SERVICES
                    };
                    for &(object_type, name, object_id) in rows {
                        let mut r = super::blank(t);
                        r[pos(t, 3)] = super::option(t, 3, object_type);
                        r[pos(t, 9)] = text(t, 9, name);
                        r[pos(t, 6)] = Value::Integer(object_id);
                        r[pos(t, 12)] = Value::Boolean(true);
                        self.insert_seed(t, r);
                    }
                    wrote = true;
                    continue;
                }
                COMPANY if !i.companies.is_empty() => {
                    // ADR-046 step 1 under the `--companies` opt-in: one row per declared company, in the order given.
                    // The session company's row is the row of the default mode. Another company's row has `Evaluation
                    // Company` No and `Display Name` empty, as "My Company" of the corpus database
                    // (install/lt-bf-install-probes/BfInstallTests.al:47-64, ADR-046 amendment 1, A6); its `Id` is a host
                    // value, as the session company's is (ADR-013 decision 9). The corpus database's session company is
                    // an evaluation company (:61), but a run's own database need not be one, and Company-Initialize reads
                    // the flag, so it is not set here (ADR-046 amendment 2).
                    for (k, name) in i.companies.iter().enumerate() {
                        let mut r = super::blank(t);
                        r[pos(t, 1)] = text(t, 1, name);
                        let mut id = i.company_id;
                        if *name != i.company {
                            id[8] = (k >> 8) as u8;
                            id[9] = k as u8;
                        }
                        r[pos(t, 8000)] = Value::Guid(id);
                        if let Some(p) = t.fields.iter().position(|f| f.id == 2_000_000_000) {
                            r[p] = Value::Guid(self.fresh_system_id());
                        }
                        let _ = self.store.insert(&self.cx, t, r);
                    }
                    wrote = true;
                    continue;
                }
                COMPANY => {
                    let mut r = super::blank(t);
                    r[pos(t, 1)] = text(t, 1, &i.company);
                    r[pos(t, 8000)] = Value::Guid(i.company_id);
                    r
                }
                USER => {
                    let mut r = super::blank(t);
                    r[pos(t, 1)] = Value::Guid(i.user_sid);
                    r[pos(t, 2)] = text(t, 2, &i.user_id);
                    r[pos(t, 3)] = text(t, 3, SESSION_USER_FULL_NAME);
                    r
                }
                // Every User has a User Property row (onprem access-control/TestUserPropertySessionUser.al:312-330).
                USER_PROPERTY => {
                    let telemetry = self.new_guid();
                    crate::usertable::property_seed(t, i.user_sid, telemetry)
                }
                // The session user's row exists when a test runs: the runner's client session is the
                // sign-in that creates it. One row, Scope Tenant, App ID Base Application, language
                // and locale 1033, a profile id that is not blank; the profile, time zone and company
                // are tenant data and not pinned (session/TestUserPersonalizationRowContracts.al:11-35).
                // The profile is the default one, an assumption.
                USER_PERSONALIZATION => {
                    let mut r = super::blank(t);
                    let mut set = |id: u32, v: Value| {
                        if let Some(p) = t.fields.iter().position(|f| f.id == id) {
                            r[p] = v;
                        }
                    };
                    set(3, Value::Guid(i.user_sid));
                    set(9, text(t, 9, "BUSINESS MANAGER"));
                    set(
                        10,
                        Value::Guid([
                            0x43, 0x7d, 0xbf, 0x0e, 0x84, 0xff, 0x41, 0x7a, 0x96, 0x5d, 0xed, 0x2b, 0xb9, 0x65, 0x09,
                            0x72,
                        ]),
                    );
                    if let Some(RtType::Option(m)) = t.fields.iter().find(|f| f.id == 11).map(|f| &f.ty) {
                        set(11, Value::Option(1, m.clone().into()));
                    }
                    set(12, Value::Integer(1033));
                    set(27, Value::Integer(1033));
                    // The row carries the session company and the host's time zone (session/W6UserPersonalization.al:
                    // row_company_blank and row_time_zone_blank are No, and SessionSettings.Init matches the row).
                    for (name, value) in [("Company", i.company.clone()), ("Time Zone", i.time_zone.clone())] {
                        if let Some(p) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(name)) {
                            r[p] = Value::Text(value.into());
                        }
                    }
                    r
                }
                ACCESS_CONTROL => {
                    let mut r = super::blank(t);
                    r[pos(t, 1)] = Value::Guid(i.user_sid);
                    r[pos(t, 2)] = text(t, 2, "SUPER");
                    r
                }
                _ => continue,
            };
            let mut row = row;
            // Every table has a SystemId; a seeded row gets one like any inserted row.
            if let Some(p) = t.fields.iter().position(|f| f.id == 2_000_000_000) {
                row[p] = Value::Guid(self.fresh_system_id());
            }
            let _ = self.store.insert(&self.cx, t, row);
            wrote = true;
        }
        wrote |= self.restore_company_rows();
        if wrote {
            self.commit_transaction();
        }
    }

    /// A run with test data: each company's row of the Company table takes every column the restored database's own row
    /// of that company holds (`Evaluation Company`, `Display Name`, `Id`, ...), not the runner's default (al-runtime
    /// issue 6505; data of the database, not a language rule). Which companies there are stays the runner's (ADR-046): a
    /// row of the database for a company the run has none for is not added. True when a row changed.
    fn restore_company_rows(&mut self) -> bool {
        let (Some(td), Some(t)) = (self.engine.test_data.clone(), self.company_table()) else {
            return false;
        };
        let Some(name_at) = t.fields.iter().position(|f| f.id == 1) else {
            return false;
        };
        let (fields, restored) = crate::testdata::TestBacking::new(&self.engine, td).company_rows(t.object, &t);
        let mut changed = false;
        for b in restored {
            let Ok(Some(mut row)) = self.store.get(&self.cx, &t, std::slice::from_ref(&b[name_at])) else {
                continue;
            };
            for &f in &fields {
                row[f] = b[f].clone();
            }
            changed |= self.store.modify(&self.cx, &t, row).is_ok();
        }
        changed
    }
}
