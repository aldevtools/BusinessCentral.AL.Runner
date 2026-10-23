//! Companies: the company a record works in, `ChangeCompany`, and what the Company table does to
//! the rows of the companies it names.
//!
//! The store keys a per-company table by company (ADR-005), so a record in another company is a
//! record whose access context names that company: no seam changes. A company exists while its row
//! is in the Company table (record/TestRecordChangeCompanyRows.al:128-145), plus the session company.

use crate::Session;
use crate::records::Rec;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::ops;
use al_builtins::value::Value;
use al_data::{AccessContext, Store, TableDef};

/// The object id of the platform's Company table.
pub(crate) const COMPANY_TABLE: u32 = 2_000_000_006;

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
        // A temporary record keeps its rows when it changes company (record/TestChangeCompanyFiltersCopy
        // DeleteContracts.al:133-136): its rows do not belong to a company.
        let Some(company) = r.company.as_ref().filter(|_| !r.temporary) else {
            return Ok(self.cx.clone());
        };
        if t.per_company
            && !r.temporary
            && (self.engine.unseeded.contains(&(t.object as usize))
                || self.baseline_rows.contains(&t.object)
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
    /// with triggers takes its data with it and a new company of the name starts empty
    /// (record/TestChangeCompanyFiltersCopyDeleteContracts.al:170-212); without triggers what happens
    /// to its data is not pinned, so a company that holds rows is refused.
    pub(crate) fn before_company_row_delete(&mut self, name: &str, run_trigger: bool) -> Result<(), Signal> {
        if !self.cx.company.is_empty() && name == &*self.cx.company {
            return unsupported("Delete of the session company's row in the Company table (not pinned)");
        }
        if !run_trigger && self.store.has_company_rows(name) {
            return unsupported("Delete(false) of a company that holds rows (not pinned)");
        }
        Ok(())
    }

    /// After a Company row went: the company's rows are gone.
    pub(crate) fn after_company_row_delete(&mut self, name: &str) {
        self.store.drop_company(name);
    }
}
