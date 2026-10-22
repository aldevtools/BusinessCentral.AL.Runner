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

pub(super) const TENANT_REPORT_LAYOUT_SELECTION: u32 = 2_000_000_233;
pub(super) const ODATA_EDM_TYPE: u32 = 2_000_000_179;

/// The Tenant Report Layout Selection rows of a fresh tenant (platform/lt-platform-tables/PtCloudTables.al:378-424,
/// QX_TenantReportLayoutSelection_RowsAndDemoShape, 'rows'): (Report ID, Layout Name), blank Company Name and null User ID.
/// The App ID is not null, but its value is not pinned: unknown, so a read of it is refused.
const TENANT_REPORT_LAYOUT_SELECTIONS: &[(i32, &str)] = &[
    (1302, "StandardSalesProFormaInv.docx"),
    (1303, "StandardDraftSalesInvoiceBlue.docx"),
    (1304, "StandardSalesQuoteBlue.docx"),
    (1306, "StandardSalesInvoiceBlueSimple.docx"),
];

/// The OData Edm Type keys of a fresh tenant (PtCloudTables.al:435-482, QX_ODataEdmType_RowsAndEdmXml, 'fresh keys' and
/// 'fresh count' 6). Their Description and Edm Xml are not pinned: unknown, so a read of either is refused.
const ODATA_EDM_TYPES: &[&str] = &[
    "BOOKINGSDATE",
    "DIMENSION",
    "DOCUMENTLINEOBJECTDETAILS",
    "ITEM-UOM",
    "ITEM-UOM-CONVERSION",
    "POSTALADDRESS",
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
        for (oi, o) in engine.program.objects.iter().enumerate() {
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
                TENANT_REPORT_LAYOUT_SELECTION => {
                    for &(report, layout) in TENANT_REPORT_LAYOUT_SELECTIONS {
                        let mut r = super::blank(t);
                        r[pos(t, 1)] = Value::Integer(report);
                        r[pos(t, 2)] = text(t, 2, layout);
                        r[pos(t, 3)] = super::unknown(t, 3, &format!("the row of report {report} on a fresh tenant"));
                        self.insert_seed(t, r);
                    }
                    wrote = true;
                    continue;
                }
                ODATA_EDM_TYPE => {
                    for &key in ODATA_EDM_TYPES {
                        let mut r = super::blank(t);
                        r[pos(t, 1)] = text(t, 1, key);
                        let whose = format!("the row {key} on a fresh tenant");
                        r[pos(t, 2)] = super::unknown(t, 2, &whose);
                        r[pos(t, 10)] = super::unknown(t, 10, &whose);
                        self.insert_seed(t, r);
                    }
                    wrote = true;
                    continue;
                }
                WEB_SERVICE => {
                    for &(object_type, name, object_id) in WEB_SERVICES {
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
                    r[pos(t, 3)] = text(t, 3, &i.user_id);
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
        if wrote {
            self.commit_transaction();
        }
    }
}
