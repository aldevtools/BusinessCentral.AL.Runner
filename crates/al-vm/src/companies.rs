//! Companies: the company a record works in, `ChangeCompany`, and what the Company table does to
//! the rows of the companies it names.
//!
//! The store keys a per-company table by company (ADR-005), so a record in another company is a
//! record whose access context names that company: no seam changes. A company exists while its row
//! is in the Company table (record/TestRecordChangeCompanyRows.al:128-145), plus the session company.

use crate::Session;
use crate::records::Rec;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::ops;
use al_builtins::value::Value;
use al_data::{AccessContext, Store, TableDef};

/// The object id of the platform's Company table.
pub(crate) const COMPANY_TABLE: u32 = 2_000_000_006;

/// The company of the corpus CI database other than the session company (install/lt-bf-install-probes/BfInstallTests.al:47-64).
const CORPUS_OTHER_COMPANY: &str = "My Company";

/// Base Application's app id.
const BASE_APPLICATION_APP_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

/// The Base Application tables that hold no row in [`CORPUS_OTHER_COMPANY`] (see [`Session::starts_empty_in`]): Company
/// Information, General Ledger Setup, Source Code Setup, Sales & Receivables Setup, CDS Connection Setup, CRM Connection
/// Setup.
const EMPTY_IN_CORPUS_OTHER_COMPANY: [u32; 6] = [79, 98, 242, 311, 5330, 7200];

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

impl Session {
    /// The Company table, if the program compiled it.
    pub(crate) fn company_table(&self) -> Option<std::sync::Arc<TableDef>> {
        let oi = self
            .engine
            .program
            .objects
            .heads()
            .iter()
            .position(|o| o.id == Some(COMPANY_TABLE) && o.table.is_some())?;
        self.engine.tables[oi].clone()
    }

    /// Whether `t` is the Company table.
    pub(crate) fn is_company_table(&self, t: &TableDef) -> bool {
        self.engine.program.objects[t.object as usize].id == Some(COMPANY_TABLE)
    }

    /// The name of the company `name` refers to, as the Company table spells it. The session
    /// company always exists; another one exists while its row does. A program that never compiled
    /// the Company table has no way to make a company, so the session company is the only one.
    pub(crate) fn find_company(&self, name: &str) -> Result<Option<String>, Signal> {
        // The name is case sensitive (record/TestChangeCompanyFiltersCopyDeleteContracts.al:243-245).
        if !self.cx.company.is_empty() && name == &*self.cx.company {
            return Ok(Some(self.cx.company.to_string()));
        }
        let Some(t) = self.company_table() else {
            return Ok(None);
        };
        let Ok(key) = ops::convert(Value::Text(name.to_string().into()), &t.fields[0].ty) else {
            // A name longer than the key field cannot be a company's.
            return Ok(None);
        };
        let row = self
            .store
            .get(&self.cx, &t, &[key])
            .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
        // The store matches a text key without regard to case; the company's own spelling must match.
        Ok(row
            .and_then(|r| al_builtins::native::text_of(&r[0]))
            .map(String::from)
            .filter(|stored| stored == name))
    }

    /// The error of a company that does not exist, for `ChangeCompany` with the answer dropped and
    /// for `RecordRef.Open` (record/TestChangeCompanyFiltersCopyDeleteContracts.al:225-227, 253-256).
    pub(crate) fn company_missing(name: &str) -> Signal {
        crate::transactions::al_error_code(&format!("The company \"{name}\" does not exist."), "DB:RecordNotFound")
    }

    /// The access context of an operation on `rec` (table `t`): the record's company, else the
    /// session's. A record in another company is refused on a table whose rows the install
    /// baseline made for the session company, since a company BC creates holds such rows too.
    pub(crate) fn cx_for(&self, rec: &Rec, t: &TableDef) -> Result<AccessContext, Signal> {
        let r = rec.borrow();
        // ADR-046 decision 5, under the `--companies` opt-in: in a declared company other than the session company only
        // the per-company tables of the apps under test are answered (new at that app's install, so they hold what install
        // code and tests wrote there). What a dependency app, Company-Initialize or the platform put into a per-company
        // table of that company is not known: install/lt-bf-install-probes/BfInstallTests.al:74-95 counts rows there that
        // the runtime does not make (P3).
        if !self.companies.is_empty() && t.per_company && !r.temporary {
            let company = r.company.as_deref().unwrap_or(&self.cx.company);
            if let Some(why) = self.company_unknown.get(&(company.to_string(), t.object)) {
                return unsupported(format!("{} in company {company}: {why}", t.name));
            }
            if !company.is_empty()
                && company != self.companies[0]
                && self.companies.iter().any(|c| c == company)
                && !self
                    .engine
                    .install_facts
                    .get()
                    .is_some_and(|f| f.under_test_tables.contains(&(t.object as usize)))
            {
                if self.starts_empty_in(company, t) {
                    return Ok(AccessContext {
                        company: company.into(),
                        user: self.cx.user.clone(),
                    });
                }
                return unsupported(format!(
                    "{} in company {company}: what the dependency apps and the platform hold in a company other than the session company is not known; under --companies only the tables of the apps under test are answered there (ADR-046 decision 5)",
                    t.name
                ));
            }
            if r.company.is_some()
                && self.companies.iter().any(|c| c == company)
                && !self.engine.unseeded.contains(&(t.object as usize))
            {
                return Ok(AccessContext {
                    company: company.into(),
                    user: self.cx.user.clone(),
                });
            }
        }
        // A temporary record keeps its rows when it changes company (record/TestChangeCompanyFiltersCopy
        // DeleteContracts.al:133-136): its rows do not belong to a company.
        let Some(company) = r.company.as_ref().filter(|_| !r.temporary) else {
            return Ok(self.cx.clone());
        };
        // A company `Database.CopyCompany` made from the session company holds its rows as they were
        // (session/lt-y22-copy-company/Y22CopyCompany.al:Y22_CopyCompany_CopiesCompanyAndRows), install rows included.
        let full_copy = self.full_copies.contains(company.as_str());
        if t.per_company
            && !r.temporary
            && (self.engine.unseeded.contains(&(t.object as usize))
                || (self.baseline_rows.contains(&t.object) && !full_copy)
                || self.backs_rows(t.object))
        {
            return unsupported(format!(
                "{} in company {company}: a company BC creates holds the rows installing the apps and Company-Initialize make there, and this runtime made them only for the session company",
                t.name
            ));
        }
        Ok(AccessContext {
            company: company.clone().into(),
            user: self.cx.user.clone(),
        })
    }

    /// Whether the per-company table `t` of a dependency app or the platform holds no row in `company`, a declared company
    /// other than the session company, in the database the run models (ADR-046 amendment 4). Its rows there are then
    /// what install code and tests write.
    ///
    /// - With test data, the restored database says (`TableProvider::holds_rows_in`): BC has what that database holds.
    /// - Without test data the run models the corpus CI database, whose "My Company" holds no row of these Base
    ///   Application tables, the same on the three supported majors: CDS Connection Setup
    ///   (install/lt-bf-install-probes/BfInstallTests.al `SecondCompany_CdsConnectionSetup_ComparedWithSessionCompany`,
    ///   corpus PR 804), CRM Connection Setup (the same file, `SecondCompany_CrmConnectionSetup_ComparedWithSessionCompany` at :317-318,
    ///   corpus issue 811), General Ledger Setup, Sales & Receivables Setup and Company Information (the same file,
    ///   `SecondCompany_SetupTablesReadByCompanyInitialize_ComparedWithSessionCompany`), Source Code Setup
    ///   (`BfInstallTests.al:74-95`). Every other table there holds rows no test pins (Report Selections, No. Series, Job
    ///   Queue Entry) or is not pinned, and stays refused (decision 5).
    fn starts_empty_in(&self, company: &str, t: &TableDef) -> bool {
        let program = &self.engine.program;
        let obj = &program.objects[t.object as usize];
        let app_id = program.apps.get(obj.app as usize).map(|a| a.id.as_str());
        match &self.engine.test_data {
            Some(td) => {
                let request = al_host::testdata::TableRequest {
                    name: obj.name.clone(),
                    app_id: app_id.map(str::to_string),
                    per_company: t.per_company,
                    ..Default::default()
                };
                td.holds_rows_in(company, &request) == Some(false)
            }
            None => {
                company == CORPUS_OTHER_COMPANY
                    && app_id.is_some_and(|a| a.eq_ignore_ascii_case(BASE_APPLICATION_APP_ID))
                    && obj.id.is_some_and(|id| EMPTY_IN_CORPUS_OTHER_COMPANY.contains(&id))
            }
        }
    }

    /// `CurrentCompany` of `rec`.
    pub(crate) fn current_company_of(&self, rec: &Rec) -> String {
        rec.borrow()
            .company
            .clone()
            .unwrap_or_else(|| self.cx.company.to_string())
    }

    /// The company a name stands for in `Open(.., Company)` and `ChangeCompany`: the empty name and
    /// the session company's name are the session company (`None`); another name must be in the
    /// Company table.
    pub(crate) fn resolve_company(&self, name: &str) -> Result<Option<Option<String>>, Signal> {
        if name.is_empty() {
            return Ok(Some(None));
        }
        Ok(self
            .find_company(name)?
            .map(|c| (c.as_str() != &*self.cx.company).then_some(c)))
    }

    /// `Record.ChangeCompany` and `RecordRef.ChangeCompany`: true once `rec` works in the company;
    /// false when no such company exists. An empty name is the session company
    /// (record/TestRecordGet.al:145-153).
    pub(crate) fn change_company(&mut self, rec: &Rec, args: &[Value]) -> Result<Value, Signal> {
        let name = match args.first() {
            None => String::new(),
            Some(v) => match al_builtins::native::text_of(v) {
                Some(n) => n,
                None => return unsupported(format!("ChangeCompany with {}", v.describe())),
            }
            .to_string(),
        };
        let Some(target) = self.resolve_company(&name)? else {
            // `Boolean?`: false when the answer is used (also on a temporary record), the error when
            // it is dropped (record/TestChangeCompanyFiltersCopyDeleteContracts.al:221-231).
            if !self.ret_used && !rec.borrow().temporary {
                return Err(Self::company_missing(&name));
            }
            return Ok(Value::Boolean(false));
        };
        // The filters, the current key and the marks stay (a mark is a primary key), and a
        // temporary record keeps its rows (:25-30).
        let mut r = rec.borrow_mut();
        if r.company == target {
            return Ok(Value::Boolean(true));
        }
        r.company = target;
        Ok(Value::Boolean(true))
    }

    /// `Database.CopyCompany(SourceName, DestinationName)`. Only the two errors are pinned
    /// (lt-o-small-builtins/OSmallBuiltins.al:92-112, the statement form at :129): a source that is not a company is
    /// RecordNotFound on the Company table and makes no destination; a destination that is already a company is
    /// RecordExists. A copy that would succeed is refused: what it copies, and whether it runs triggers or events, is
    /// not pinned (BC's test runner cannot run one, :87-90).
    pub(crate) fn copy_company(&mut self, args: &[Value]) -> Result<Value, Signal> {
        if self.ret_used {
            return unsupported("Database.CopyCompany with the result used (only the statement form is pinned)");
        }
        let (Some(source), Some(destination)) = (
            args.first().and_then(al_builtins::native::text_of),
            args.get(1).and_then(al_builtins::native::text_of),
        ) else {
            return unsupported("Database.CopyCompany with arguments that are not text");
        };
        let (source, destination) = (source.to_string(), destination.to_string());
        let source_is = self.company_spelled(&source)?;
        let destination_is = self.company_spelled(&destination)?;
        match (source_is, destination_is) {
            // Pinned with a destination that is not a company (OSmallBuiltins.al:98, :106-108).
            (Some(false), Some(false)) => Err(crate::transactions::al_error_code(
                &format!("The Company does not exist. Identification fields and values: Name='{source}'"),
                "DB:RecordNotFound",
            )),
            // Pinned with the source as its own destination (OSmallBuiltins.al:102, :109-110).
            (Some(true), Some(true)) => Err(crate::transactions::al_error_code(
                &format!(
                    "The record in table Company already exists. Identification fields and values: Name='{destination}'"
                ),
                "DB:RecordExists",
            )),
            (Some(true), Some(false)) => self.copy_company_rows(&source, &destination),
            _ => unsupported(
                "Database.CopyCompany with a missing source and an existing destination, a name in another case than the Company table's, or a name longer than the Company table's key (not pinned)",
            ),
        }
    }

    /// The copy of `Database.CopyCompany` (session/lt-y22-copy-company/Y22CopyCompany.al:Y22_CopyCompany_CopiesCompanyAndRows,
    /// SaaS BC 29, corpus writer Y22): a new Company row with the destination's name, `Evaluation Company` as the source's,
    /// `Display Name` and `Business Profile Id` empty, a new `Id` and `SystemId`; every per-company row of the source as it
    /// is (system fields included); tables that are not per company unchanged. No trigger or table event of a copied row
    /// and no database trigger runs; the Company's `OnAfterInsertEvent` runs once for the new row.
    fn copy_company_rows(&mut self, source: &str, destination: &str) -> Result<Value, Signal> {
        assume(Assumption::CopyCompanyInSaas);
        // On BC the copy runs beside the session's transaction: copying the session company inside a test meets the test
        // runner's own row (lt-o-small-builtins/OSmallBuiltins.al:87-90).
        if self.in_test_codeunit && source == &*self.cx.company {
            return unsupported(
                "Database.CopyCompany of the session company while a test runs: on BC it meets the test runner's own row in Test Method Line (lt-o-small-builtins/OSmallBuiltins.al:87-90; the error is not pinned)",
            );
        }
        if self.store.has_uncommitted() {
            return unsupported(
                "Database.CopyCompany inside a write transaction (not pinned: Y22CopyCompany.al commits before every copy)",
            );
        }
        if !self.companies.is_empty() && source != &*self.cx.company {
            return unsupported(
                "Database.CopyCompany of a company other than the session company under --companies (ADR-046 decision 5: its rows are not all known)",
            );
        }
        let Some(t) = self.company_table() else {
            return unsupported("Database.CopyCompany without the Company table");
        };
        // The Company's OnBeforeInsertEvent and its database trigger were not observed (Y22CopyCompany.al binds neither).
        if self.has_listeners(t.object as usize, "onbeforeinsertevent", None)? {
            return unsupported(
                "Database.CopyCompany with a subscriber to the Company's OnBeforeInsertEvent (not pinned)",
            );
        }
        if self.global_enabled(&t, crate::globaltriggers::GlobalOp::Insert)? {
            return unsupported(
                "Database.CopyCompany with the database insert trigger on for the Company table (not pinned)",
            );
        }
        let key = ops::convert(Value::Text(source.to_string().into()), &t.fields[0].ty)
            .map_err(|_| Signal::Unsupported("Database.CopyCompany: a source name that is no Company key".into()))?;
        let Some(source_row) = self
            .store
            .get(&self.cx, &t, &[key])
            .map_err(|e| Signal::Unsupported(format!("store error {e}")))?
        else {
            return unsupported("Database.CopyCompany of a company that has no row in the Company table");
        };
        let at = |id: u32| t.fields.iter().position(|f| f.id == id);
        let (Some(name_at), Some(evaluation_at), Some(id_at)) = (at(1), at(2), at(8000)) else {
            return unsupported("Database.CopyCompany on a Company table without Name, Evaluation Company and Id");
        };
        // Only these fields were observed; a source with any other field set (system fields aside) is not pinned.
        for (i, f) in t.fields.iter().enumerate() {
            let observed = [1, 2, 3, 8000, 8005].contains(&f.id) || f.id >= 2_000_000_000;
            let default = self.default_value(&f.ty);
            if !observed && !ops::compare(&source_row[i], &default).is_ok_and(|o| o.is_eq()) {
                return unsupported(format!(
                    "Database.CopyCompany of a company whose field {} is set (what the copy holds there is not pinned)",
                    f.name
                ));
            }
        }
        self.store.copy_company(source, destination);
        if source == &*self.cx.company || self.full_copies.contains(source) {
            self.full_copies.insert(destination.to_string());
        }
        let Value::Record(company) = self.default_value(&al_builtins::value::RtType::Record(t.object, false)) else {
            unreachable!("a record type makes a record")
        };
        {
            let mut c = company.borrow_mut();
            c.fields[name_at] = ops::convert(Value::Text(destination.to_string().into()), &t.fields[name_at].ty)
                .map_err(|_| {
                    Signal::Unsupported("Database.CopyCompany: a destination name that is no Company key".into())
                })?;
            c.fields[evaluation_at] = source_row[evaluation_at].clone();
            c.fields[id_at] = Value::Guid(self.fresh_guid());
        }
        self.run_trigger_unknown = Some(
            "the RunTrigger of the Company insert that Database.CopyCompany makes is not pinned (Y22CopyCompany.al does not log it)",
        );
        let inserted = self.record_op(
            al_builtins::record_op::RecordOp::Insert,
            &company,
            &mut [Value::Boolean(false)],
        );
        self.run_trigger_unknown = None;
        inserted?;
        Ok(Value::Null)
    }

    /// Whether `name` is a company spelled exactly so (`Some(true)`), no company in any case (`Some(false)`), or a
    /// company spelled in another case or a name that is no key of the Company table (`None`).
    fn company_spelled(&self, name: &str) -> Result<Option<bool>, Signal> {
        if self.find_company(name)?.is_some() {
            return Ok(Some(true));
        }
        let Some(t) = self.company_table() else {
            return Ok(Some(false));
        };
        let Ok(key) = ops::convert(Value::Text(name.to_string().into()), &t.fields[0].ty) else {
            return Ok(None);
        };
        let row = self
            .store
            .get(&self.cx, &t, &[key])
            .map_err(|e| Signal::Unsupported(format!("store error {e}")))?;
        Ok(row.is_none().then_some(false))
    }

    /// Marks the session company as an evaluation company: its row of the Company table gets `Evaluation Company` Yes
    /// (field 2), committed. BCApps' CI creates the Uncategorized lane's company so (`New-CompanyInBcContainer
    /// -evaluationCompany`, build/scripts/ImportTestDataInBcContainer.ps1:903, :954); a recipe fact, not a language rule.
    /// Without the Company table, or without the session company's row, nothing changes.
    pub fn mark_evaluation_company(&mut self) {
        let Some(t) = self.company_table() else { return };
        let Some(at) = t.fields.iter().position(|f| f.id == 2) else {
            return;
        };
        let Ok(key) = ops::convert(Value::Text(self.cx.company.to_string().into()), &t.fields[0].ty) else {
            return;
        };
        let Ok(Some(mut row)) = self.store.get(&self.cx, &t, &[key]) else {
            return;
        };
        row[at] = Value::Boolean(true);
        if self.store.modify(&self.cx, &t, row).is_ok() {
            self.commit_transaction();
        }
    }

    /// Before a Company row goes: the session company's row stays (not pinned). A company deleted
    /// with or without triggers takes its data with it and a new company of the name starts empty
    /// (record/TestChangeCompanyFiltersCopyDeleteContracts.al:170-212 with triggers, :425-461
    /// DeleteWithoutTriggers_OfACompanyWithRows_RemovesTheCompany_AndANewOneStartsEmpty without).
    pub(crate) fn before_company_row_delete(&mut self, name: &str, _run_trigger: bool) -> Result<(), Signal> {
        if !self.cx.company.is_empty() && name == &*self.cx.company {
            return unsupported("Delete of the session company's row in the Company table (not pinned)");
        }
        Ok(())
    }

    /// After a Company row went: the company's rows are gone.
    pub(crate) fn after_company_row_delete(&mut self, name: &str) {
        self.store.drop_company(name);
        self.full_copies.remove(name);
    }
}
