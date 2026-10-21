//! Builtins that act on session state: number sequences and the transaction state.

use crate::Session;
use al_builtins::builtin_id::ids;
use al_builtins::describe::Describe;
use al_builtins::native::{AlError, Signal};
use al_builtins::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

/// One `NumberSequence`.
#[derive(Debug, Clone)]
pub(crate) struct Sequence {
    pub increment: i64,
    /// The value `Next` hands out when nothing was allocated yet.
    pub seed: i64,
    /// The last value handed out (by `Next` or the end of a `Range`).
    pub last: Option<i64>,
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// An error with a pinned code.
fn error_code<T>(message: String, code: &str) -> Result<T, Signal> {
    Err(Signal::Error(Box::new(AlError {
        message,
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

fn big(args: &[Value], i: usize) -> Option<i64> {
    match args.get(i)? {
        Value::BigInteger(n) => Some(*n),
        Value::Integer(n) => Some(i64::from(*n)),
        _ => None,
    }
}

/// The connection string of the platform's own test connection, the one a CRM table connection
/// may use in a test (corpus session/TestDatabaseTableConnectionCrm.al:8-14).
const TEST_CONNECTION: &str = "@@test@@";

impl Session {
    /// `Database.{Register,Unregister,Has,SetDefault,GetDefault}TableConnection`. Only the CRM
    /// test connection is modelled (session/TestDatabaseTableConnectionCrm.al); names match
    /// ignoring case (:Database_HasTableConnection_CrmTest_NameIsCaseInsensitive).
    fn table_connection_op(&mut self, method: &str, args: &[Value]) -> Result<Value, Signal> {
        let kind = match args.first() {
            Some(Value::Option(o, m)) => (*o, m.get(*o as usize).cloned().unwrap_or_default()),
            other => return unsupported(format!("{method}TableConnection with {} as the type", other.describe())),
        };
        let name = match args.get(1) {
            Some(v) => al_builtins::native::text_of(v),
            None => None,
        };
        match (method, kind.1.as_str()) {
            // A Cloud extension may not register an external SQL, Exchange or Microsoft Graph connection: the error and its code
            // (session/lt-tableconnection-type-register/TableConnectionTypeRegister.al:Register_ExternalSql_Result,
            // Register_Exchange_Result, Register_MicrosoftGraph_Result; session/TestMiscComplete.al:185-194). Query is not pinned.
            ("Register", k @ ("ExternalSQL" | "Exchange" | "MicrosoftGraph")) => {
                return error_code(
                    format!(
                        "You do not have permission to register a table connection of type {k}. Please contact your administrator."
                    ),
                    "NotAllowedForCompilationTarget",
                );
            }
            // Never registered, so none is found and none is the default. The corpus only calls
            // them for ExternalSQL and expects them not to throw
            // (session/TestDatabaseExtended.al:75-92, TestSystemExtended.al:664-672).
            ("Has", "ExternalSQL" | "Exchange" | "MicrosoftGraph") => return Ok(Value::Boolean(false)),
            ("GetDefault", "ExternalSQL") => return Ok(Value::Text(String::new().into())),
            // Unregistering a name that was never registered is no error, for every connection type
            // (errors/lt-ms-refusals-137/P137TableConnRisky.al:P137_Unregister_NeverRegistered_OtherTypes
            // for ExternalSQL, Exchange and MicrosoftGraph; P137TableConn.al:P137_Unregister_NeverRegistered for CRM).
            ("Unregister", "ExternalSQL" | "Exchange" | "MicrosoftGraph") => return Ok(Value::Null),
            (_, "CRM") => {}
            (m, k) => return unsupported(format!("{m}TableConnection of type {k} (not pinned)")),
        }
        let ord = kind.0;
        match method {
            "GetDefault" => Ok(Value::Text(
                self.default_table_connection
                    .get(&ord)
                    .cloned()
                    .unwrap_or_default()
                    .into(),
            )),
            "Has" | "Register" | "Unregister" | "SetDefault" => {
                let Some(name) = name else {
                    return unsupported(format!("{method}TableConnection with a name that is not text"));
                };
                let key = (ord, name.to_lowercase());
                match method {
                    "Has" => Ok(Value::Boolean(self.table_connections.contains(&key))),
                    "Register" => {
                        match args.get(2).and_then(al_builtins::native::text_of) {
                            Some(c) if c == TEST_CONNECTION => {}
                            _ => {
                                return unsupported(
                                    "RegisterTableConnection with a connection string other than @@test@@ (no Dataverse in the runtime)",
                                );
                            }
                        }
                        // The name is the pinned fragment of the error
                        // (:Database_RegisterTableConnection_CrmTest_Duplicate_Throws).
                        if !self.table_connections.insert(key) {
                            // The name as written in this call (record/lt-table-connection-duplicate/
                            // TableConnectionDuplicateRegister.al:P161_RegisterTwice_SameAndOtherCase `twiceUpper`).
                            return error_code(
                                format!("Table connection {name} for table type CRM has already been registered."),
                                "Argument",
                            );
                        }
                        Ok(Value::Null)
                    }
                    "Unregister" => {
                        // Not registered: no error and nothing changes (P137TableConn.al:P137_Unregister_NeverRegistered,
                        // _Twice `unregister_second`, _OtherNameAndOtherCase `unregister_b` and `unregister_a_again`).
                        if !self.table_connections.remove(&key) {
                            return Ok(Value::Null);
                        }
                        // Unregistering the default clears the default too
                        // (:Database_UnregisterTableConnection_CrmTest_ClearsDefaultAndHas).
                        if self
                            .default_table_connection
                            .get(&ord)
                            .is_some_and(|d| d.to_lowercase() == key.1)
                        {
                            self.default_table_connection.remove(&ord);
                        }
                        Ok(Value::Null)
                    }
                    _ => {
                        if args.len() > 2 {
                            return unsupported("SetDefaultTableConnection with a Scoped argument (not pinned)");
                        }
                        if !self.table_connections.contains(&key) {
                            return unsupported(
                                "SetDefaultTableConnection of a connection that is not registered (not pinned)",
                            );
                        }
                        self.default_table_connection.insert(ord, name.to_string());
                        Ok(Value::Null)
                    }
                }
            }
            other => unsupported(format!("{other}TableConnection")),
        }
    }

    /// The key of sequence `args[0]`; `scope` is the position of the optional CompanySpecific
    /// argument (company specific when omitted, as the default overload is: session/TestNumberSequence.al:15-30).
    fn sequence_key(&self, args: &[Value], scope: usize) -> Result<(String, String), Signal> {
        let Some(name) = args.first().and_then(al_builtins::native::text_of) else {
            return unsupported("a number sequence name that is not text");
        };
        let company_specific = match args.get(scope) {
            None => true,
            Some(Value::Boolean(b)) => *b,
            Some(other) => return unsupported(format!("CompanySpecific of {}", other.describe())),
        };
        let company = if company_specific {
            self.cx.company.to_string()
        } else {
            String::new()
        };
        Ok((company, name.to_string()))
    }

    pub(crate) fn session_op(&mut self, id: al_ir::BuiltinId, args: &mut [Value]) -> Result<Value, Signal> {
        if let Some(answer) = self.host_op(id, args) {
            return answer;
        }
        if let Some(answer) = self.encryption_op(id, args) {
            return answer;
        }
        if id == ids::SYSTEM_GETDOTNETTYPE_1 {
            return self.get_dotnet_type(args);
        }
        if id == ids::SESSIONINFORMATION_CALLSTACK_1 {
            // Frames innermost first, a backslash after each, the runner's below
            // (session/TestSessionInformationCallstackContracts.al:130-143).
            return match self.live_call_stack() {
                Some(text) => Ok(Value::Text(text.into())),
                None => unsupported(
                    "SessionInformation.Callstack with a frame whose format is not pinned (an event of a table, page or the platform, or an object type not measured)",
                ),
            };
        }
        if id == ids::DATABASE_ISINWRITETRANSACTION_1 {
            // An uncommitted write opens one, a Commit and the test boundary close it
            // (codeunit/TestWriteTxTestBoundary.al:93,103,142). Writes to temporary tables are
            // not counted: they never reach the database (not pinned).
            return Ok(Value::Boolean(self.store.has_uncommitted()));
        }
        if id == ids::COMPANYPROPERTY_DISPLAYNAME_1 {
            // Non-empty (session/TestBCPlatformContracts.al:CompanyProperty_DisplayName_NonEmpty). That it is
            // the company's name when no display name is set is an assumption (docs/probes-needed.md); a
            // company that states a display name is not modelled.
            return Ok(Value::Text(self.cx.company.clone().into()));
        }
        if id == ids::COMPANYPROPERTY_ID_1 {
            // The `Id` of the company's row of the Company table (session/lt-company-record/CompanyRecord.al:ID_EqualsTheCompanyRecordId).
            if let Some(t) = self.company_table() {
                let key = al_builtins::ops::convert(Value::Text(self.cx.company.to_string().into()), &t.fields[0].ty);
                if let (Ok(key), Some(at)) = (key, t.fields.iter().position(|f| f.name == "Id")) {
                    if let Ok(Some(row)) = al_data::Store::get(&self.store, &self.cx, &t, &[key]) {
                        if let Value::Guid(g) = &row[at] {
                            if g.iter().any(|b| *b != 0) {
                                return Ok(Value::Guid(*g));
                            }
                        }
                    }
                }
            }
            // A Guid that identifies the company; stable for a name (the Company table's SystemId is not modelled).
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in self.cx.company.bytes() {
                h = (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3);
            }
            let mut g = [0u8; 16];
            g[..8].copy_from_slice(&h.to_le_bytes());
            g[8..].copy_from_slice(&h.rotate_left(17).to_le_bytes());
            g[6] = (g[6] & 0x0f) | 0x40;
            g[8] = (g[8] & 0x3f) | 0x80;
            return Ok(Value::Guid(g));
        }
        if id == ids::COMPANYPROPERTY_URLNAME_1 {
            // The company name as it stands in a URL: unreserved characters kept, the rest percent-encoded
            // (session/TestProbeSessionMiscContracts.al:Company_Names scrubs both the name and its `%20` form).
            let mut out = String::new();
            for b in self.cx.company.bytes() {
                if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                    out.push(char::from(b));
                } else {
                    out.push_str(&format!("%{b:02X}"));
                }
            }
            return Ok(Value::Text(out.into()));
        }
        if id == ids::DATABASE_COMPANYNAME_1 {
            // The company of the session, the same name `Record.CurrentCompany` answers
            // (session/TestDatabase.al:69-75, session/TestSessionFunctions.al:25).
            return Ok(Value::Text(self.cx.company.clone().into()));
        }
        match id {
            // The counter has handed out `next_row_version` so far; the database never reports
            // zero (session/TestSystemExtended.al:635-659, TestBCDatabaseContracts.al:100-117).
            // What the database holds committed: a row inserted in the open transaction does not count yet
            // (record/TestProbeRecordRefContracts.al:LastUsedRowVersion_StartsAndGrows, `grows_after_insert` No).
            ids::DATABASE_LASTUSEDROWVERSION_1 => return Ok(Value::BigInteger(self.tx.committed_row_version.max(1))),
            // After a Commit that ended a transaction without writes, and with no write since, it is LastUsedRowVersion() + 1
            // (platform/lt-platform-members/PlTRowVersionValue.al:MinimumActiveRowVersion_NoWrite_Committed line 40 and
            // _PrefixedCallEqualsBareCall). Inside a write transaction, or after a Commit that made writes durable, the value is not
            // pinned (the file shows it below LastUsedRowVersion() + 1 after a committed insert, line 108, and unchanged by an open insert,
            // line 77, but the first read in each test follows a DeleteAll): refused. platform/lt-platform-members/PlTDatabase.al:250-264
            // pins the bounds 0 <= value <= LastUsedRowVersion() + 1 only.
            ids::DATABASE_MINIMUMACTIVEROWVERSION_1 => {
                if self.tx.tx_phase == crate::transactions::TX_WRITE || self.tx.last_commit_wrote {
                    return unsupported(
                        "Database.MinimumActiveRowVersion in a write transaction or after a Commit that made writes durable (the exact value is not pinned; probe wanted)",
                    );
                }
                return Ok(Value::BigInteger(self.tx.committed_row_version.max(1) + 1));
            }
            // Set when given a value, read otherwise; the default is the documented one
            // (docs/probes-needed.md). session/TestSystemExtended.al:608-630 only calls it.
            ids::DATABASE_LOCKTIMEOUT_1 => {
                match args.first() {
                    None | Some(Value::Null) => {}
                    Some(Value::Boolean(b)) => self.tx.lock_timeout = *b,
                    Some(other) => return unsupported(format!("LockTimeout with {}", other.describe())),
                }
                return Ok(Value::Boolean(self.tx.lock_timeout));
            }
            // Set when given a value, read otherwise. Only "positive" is pinned (corpus types/PVLabelSplitForeach.al
            // on the staging branch: positive, 10 ms on the probe tier, "the exact default is a server setting").
            ids::DATABASE_LOCKTIMEOUTDURATION_1 => {
                match args.first() {
                    None | Some(Value::Null) => {}
                    Some(Value::Duration(d)) => self.tx.lock_timeout_duration = *d,
                    Some(Value::Integer(i)) => self.tx.lock_timeout_duration = i64::from(*i),
                    Some(other) => return unsupported(format!("LockTimeoutDuration with {}", other.describe())),
                }
                // The catalog types the result as an Integer (a Duration in AL); the id is pinned (pinned_ids.tsv).
                return Ok(Value::Integer(
                    i32::try_from(self.tx.lock_timeout_duration).unwrap_or(i32::MAX),
                ));
            }
            // The type of the transaction (session/ProbeSystemEnumMembers.al:60-161, BC 27.5;
            // TestClearAllTransactionTypeContracts.al:224-253):
            // - a test starts as UpdateNoLocks, and a set before any database call takes effect;
            // - after a Commit() the next transaction's type is Update, and a set takes effect again;
            // - once a write call has run (also a DeleteAll that finds no row) the type stays as it was: a
            //   set to Report or Snapshot, or to the type it has, goes through without effect, and a set
            //   to Update raises DB:CannotChangeTransactionType;
            // - once a read call has run, a set of Report reads back as Update.
            // Every other set after the transaction has started is not pinned and refused.
            ids::DATABASE_CURRENTTRANSACTIONTYPE_1 => {
                use crate::transactions::{TX_FRESH, TX_READ, TX_WRITE};
                let members: std::sync::Arc<[String]> = al_builtins::sysenum::option_members("TransactionType")
                    .expect("a system option")
                    .into();
                let new = match args.first() {
                    None | Some(Value::Null) => return Ok(Value::Option(self.tx.transaction_type, members)),
                    Some(Value::Option(t, _)) => *t,
                    Some(other) => return unsupported(format!("CurrentTransactionType set to {}", other.describe())),
                };
                let (update, snapshot, report) = (1, 2, 4);
                match self.tx.tx_phase {
                    TX_FRESH => self.tx.transaction_type = new,
                    TX_WRITE if new == self.tx.transaction_type => {}
                    TX_WRITE if new == snapshot || new == report => {}
                    TX_WRITE if new == update => {
                        let name = |t: i32| members.get(t as usize).cloned().unwrap_or_default();
                        return Err(crate::transactions::al_error_code(
                            &format!(
                                "The transaction type cannot be changed after a transaction has started.\\\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\\\The error occurred because the transaction type was changed from {} to {}.",
                                name(self.tx.transaction_type),
                                name(new)
                            ),
                            "DB:CannotChangeTransactionType",
                        ));
                    }
                    TX_READ if new == report => self.tx.transaction_type = update,
                    _ => {
                        return unsupported(format!(
                            "Database.CurrentTransactionType set to {} after a database call (not pinned)",
                            members.get(new as usize).cloned().unwrap_or_default()
                        ));
                    }
                }
                return Ok(Value::Null);
            }
            _ => {}
        }
        if id == ids::SYSTEM_CODECOVERAGELOG_1 {
            // With a state it records or stops and answers the state it was given; without one it
            // answers whether it is recording (codeunit/TestCodeCoverageStartStop.al:30-102). The
            // MultiSession flag changes nothing observable here.
            return match args.first() {
                None => Ok(Value::Boolean(self.coverage.code_coverage)),
                Some(Value::Boolean(on)) => {
                    if *on && !self.coverage.code_coverage {
                        self.coverage.recorder.reset_refreshed();
                    }
                    self.coverage.code_coverage = *on;
                    Ok(Value::Boolean(*on))
                }
                Some(other) => unsupported(format!("CodeCoverageLog with {}", other.describe())),
            };
        }
        let table_connection = match id {
            ids::DATABASE_REGISTERTABLECONNECTION_1 => Some("Register"),
            ids::DATABASE_UNREGISTERTABLECONNECTION_1 => Some("Unregister"),
            ids::DATABASE_HASTABLECONNECTION_1 => Some("Has"),
            ids::DATABASE_SETDEFAULTTABLECONNECTION_1 => Some("SetDefault"),
            ids::DATABASE_GETDEFAULTTABLECONNECTION_1 => Some("GetDefault"),
            _ => None,
        };
        if let Some(method) = table_connection {
            return self.table_connection_op(method, args);
        }
        if let Some(v) = self.error_collection_op(id, args) {
            return v;
        }
        let isolated = match id {
            ids::ISOLATEDSTORAGE_CONTAINS_1 => Some(("Contains", 1)),
            ids::ISOLATEDSTORAGE_CONTAINS_2 => Some(("Contains", 2)),
            ids::ISOLATEDSTORAGE_DELETE_1 => Some(("Delete", 1)),
            ids::ISOLATEDSTORAGE_GET_1 => Some(("Get", 1)),
            ids::ISOLATEDSTORAGE_GET_2 => Some(("Get", 2)),
            ids::ISOLATEDSTORAGE_GET_3 => Some(("Get", 3)),
            ids::ISOLATEDSTORAGE_GET_4 => Some(("Get", 4)),
            ids::ISOLATEDSTORAGE_SET_1 => Some(("Set", 1)),
            ids::ISOLATEDSTORAGE_SET_2 => Some(("Set", 2)),
            ids::ISOLATEDSTORAGE_SETENCRYPTED_1 => Some(("SetEncrypted", 1)),
            ids::ISOLATEDSTORAGE_SETENCRYPTED_2 => Some(("SetEncrypted", 2)),
            _ => None,
        };
        if let Some((method, n)) = isolated {
            return self.isolated_storage_op(method, n, args);
        }
        let method = match id {
            ids::NUMBERSEQUENCE_CURRENT_1 => Some("Current"),
            ids::NUMBERSEQUENCE_DELETE_1 => Some("Delete"),
            ids::NUMBERSEQUENCE_EXISTS_1 => Some("Exists"),
            ids::NUMBERSEQUENCE_INSERT_1 => Some("Insert"),
            ids::NUMBERSEQUENCE_NEXT_1 => Some("Next"),
            ids::NUMBERSEQUENCE_RANGE_1 | ids::NUMBERSEQUENCE_RANGE_2 => Some("Range"),
            ids::NUMBERSEQUENCE_RESTART_1 => Some("Restart"),
            _ => None,
        };
        let Some(method) = method else {
            return unsupported(format!("builtin {id}"));
        };
        // The position of the optional CompanySpecific argument per overload.
        let scope = match id {
            ids::NUMBERSEQUENCE_INSERT_1 | ids::NUMBERSEQUENCE_RANGE_2 => 3,
            ids::NUMBERSEQUENCE_RANGE_1 | ids::NUMBERSEQUENCE_RESTART_1 => 2,
            _ => 1,
        };
        let key = self.sequence_key(args, scope)?;
        let name = key.1.clone();
        // session/TestNumberSequenceRuntimeContracts.al:NumberSequence_MissingSequence_Texts: the name is
        // quoted and the code is `AL`.
        let missing = || error_code::<Value>(format!("The number sequence '{name}' does not exist."), "AL");
        match method {
            "Insert" => {
                // Seed 0 and Increment 1 when omitted
                // (session/TestNumberSequenceRuntimeContracts.al:NumberSequence_Insert_WithoutArguments_StartsAtZeroAndStepsByOne).
                let seed = big(args, 1).unwrap_or(0);
                let increment = big(args, 2).unwrap_or(1);
                // Inserting a name that exists raises, and the sequence is gone afterwards, in this
                // test and the next (:NumberSequence_Insert_OfAnExistingName_RaisesAndLeavesTheSequenceGone).
                if self.sequences.remove(&key).is_some() {
                    return error_code(format!("The number sequence '{name}' already exists."), "AL");
                }
                // An Increment of 0 raises the SQL error and creates nothing
                // (platform/lt-platform-members/PlTNumberSequence.al:Insert_ZeroIncrement; the name exists already: not pinned).
                if increment == 0 {
                    return error_code(
                        "An unexpected error occurred after a database command was cancelled.".to_string(),
                        "Sql:Sql",
                    );
                }
                self.sequences.insert(
                    key,
                    Sequence {
                        increment,
                        seed,
                        last: None,
                    },
                );
                Ok(Value::Null)
            }
            "Exists" => Ok(Value::Boolean(self.sequences.contains_key(&key))),
            // Deleting a sequence that is not there is not an error
            // (error-handling/TestNumberSequenceTrappableErrors.al:117-126).
            "Delete" => {
                self.sequences.remove(&key);
                Ok(Value::Null)
            }
            "Current" => match self.sequences.get(&key) {
                Some(s) => Ok(Value::BigInteger(s.last.unwrap_or(s.seed))),
                None => missing(),
            },
            "Next" | "Range" => {
                let count = if method == "Range" {
                    match args.get(1) {
                        Some(Value::Integer(n)) if *n >= 1 => i64::from(*n),
                        // :NumberSequence_Range_Zero_ThrowsSqlError (the sequence must exist first).
                        Some(Value::Integer(0)) if self.sequences.contains_key(&key) => {
                            return error_code(
                                "An unexpected error occurred after a database command was cancelled.".to_string(),
                                "Sql:Sql",
                            );
                        }
                        other => {
                            return unsupported(format!(
                                "NumberSequence.Range with Count {} (not pinned)",
                                other.describe()
                            ));
                        }
                    }
                } else {
                    1
                };
                let Some(s) = self.sequences.get_mut(&key) else {
                    return missing();
                };
                let first = match s.last {
                    None => Some(s.seed),
                    Some(last) => last.checked_add(s.increment),
                };
                let last = first.and_then(|f| s.increment.checked_mul(count - 1).and_then(|d| f.checked_add(d)));
                let (Some(first), Some(last)) = (first, last) else {
                    return unsupported("a number sequence past the BigInteger range (error not pinned)");
                };
                s.last = Some(last);
                if id == ids::NUMBERSEQUENCE_RANGE_2 {
                    args[2] = Value::BigInteger(s.increment);
                }
                Ok(Value::BigInteger(first))
            }
            "Restart" => {
                // Without a Seed the sequence restarts at 0 whatever it started at
                // (:NumberSequence_NextRestartRange_Values).
                let seed = big(args, 1).unwrap_or(0);
                match self.sequences.get_mut(&key) {
                    Some(s) => {
                        s.seed = seed;
                        s.last = None;
                        Ok(Value::Null)
                    }
                    None => missing(),
                }
            }
            other => unsupported(format!("NumberSequence.{other}")),
        }
    }
}

fn secret_text(s: String) -> Value {
    Value::Library(
        "SecretText".into(),
        Rc::new(RefCell::new(al_builtins::value::LibraryValue::Text(s))),
    )
}

impl Session {
    /// The key a data scope names. Module is the default and is one app's store; Company adds the
    /// session company, and the app boundary holds there too (session/TestIsolatedStoragePerApp.al:
    /// 100-120). The user scopes need a user identity, which the runtime does not have.
    /// The app that owns the code calling IsolatedStorage, not the app that raised the event it handles
    /// (session/TestIsolatedStoragePerApp.al:140-160).
    fn calling_app(&self) -> u32 {
        self.frames.last().map_or(0, |o| self.engine.program.objects[*o].app)
    }

    fn storage_scope(&self, scope: Option<&Value>) -> Result<String, Signal> {
        let app = self.calling_app();
        match scope {
            None => Ok(format!("app{app}")),
            Some(Value::Option(0, _)) => Ok(format!("app{app}")),
            Some(Value::Option(1, _)) => Ok(format!("app{app}:company:{}", self.cx.company)),
            // The user scopes add the session user (session/TestProbeSessionMiscContracts.al:IsolatedStorage_Behaviour:
            // nothing was set there, a Get answers false).
            Some(Value::Option(2, _)) => Ok(format!("app{app}:user:{}", self.host.identity.user_id())),
            Some(Value::Option(3, _)) => Ok(format!(
                "app{app}:company:{}:user:{}",
                self.cx.company,
                self.host.identity.user_id()
            )),
            Some(Value::Option(..)) => unsupported("IsolatedStorage with a data scope the corpus does not name"),
            Some(other) => unsupported(format!("IsolatedStorage with data scope {}", other.describe())),
        }
    }

    /// `IsolatedStorage.<rest>` (`Get#4`, `SetEncrypted#2`, ...): a text store per scope, part of the transaction
    /// (a rollback undoes it, see `Session::isolated_committed`). Corpus: session/TestIsolatedStorage.al:30-125.
    fn isolated_storage_op(&mut self, method: &str, n: u32, args: &mut [Value]) -> Result<Value, Signal> {
        // A Guid key is its text form `{XXXXXXXX-...}`, for Set, Contains, Get and Delete in every scope
        // (session/lt-isolated-storage-guid/IsoStorageGuidKey.al). Other key types are not pinned (probe wanted).
        let key = match args.first() {
            Some(Value::Guid(g)) => Some(al_builtins::format::format_guid(g)),
            other => other.and_then(al_builtins::native::text_of).map(String::from),
        };
        // A Variant key that holds an Integer, Decimal, Boolean or Date raises a type-conversion error from every
        // method in every scope; nothing is stored (session/lt-isolated-storage-variant-key/IsolatedStorageVariantKey.al:
        // 125-136 Integer, 160-171 Decimal, 175-186 Boolean, 190-201 Date). Other types are not pinned.
        if key.is_none()
            && matches!(
                args.first(),
                Some(Value::Integer(_) | Value::Decimal(_) | Value::Boolean(_) | Value::Date(..))
            )
        {
            return al_builtins::native::raise(
                "The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavText value.",
                "TypeConversion",
            );
        }
        let Some(key) = key else {
            return unsupported("an IsolatedStorage key that is neither text nor a Guid");
        };
        // Where the optional DataScope sits, and where the value goes, per overload.
        let scope_at = match (method, n) {
            ("Get", 3 | 4) => None,
            ("Get", _) => Some(1),
            ("Set" | "SetEncrypted", _) => Some(2),
            _ => Some(1),
        };
        let scope = self.storage_scope(scope_at.and_then(|i| args.get(i)))?;
        let slot = (scope, key);
        let found = self.ret_used;
        match method {
            "Set" | "SetEncrypted" => {
                let Some(arg) = args.get(1) else {
                    return unsupported("IsolatedStorage.Set without a value");
                };
                let secret = al_builtins::native::text_of(arg).is_none();
                let value = al_builtins::textfns::secret_or_text(arg)?;
                if self.pages.bg_worker.is_some() {
                    // A worker runs read-only: a Set of a new key is refused as an Insert into table 2000000107
                    // (session/lt-cn-session-leads/CnIsolatedStorageLeads.al:104-117). A Set over an existing key and a
                    // SetEncrypted are not pinned there.
                    if method != "Set" || self.isolated_storage.contains_key(&slot) {
                        return unsupported(format!(
                            "IsolatedStorage.{method} in a page background task over an existing key or encrypted (the refusal text is not pinned)"
                        ));
                    }
                    return Err(self.bg_read_only_error(2000000107, "Isolated Storage", self.calling_app(), "Insert"));
                }
                if method == "SetEncrypted" {
                    // The limit is on the UTF-8 bytes: 214 ASCII and 107 two-byte characters are kept, 216 ASCII and 108
                    // two-byte characters fail (session/lt-cn-session-leads/CnIsolatedStorageLeads.al:368-424; the header at
                    // :12-13 says 216 and more fail with BC's own length check). 215 bytes fails on the Linux container with
                    // an OpenSSL error (:380-388, Linux suspect), characters of three or four bytes and SecretText values
                    // near the limit are not pinned.
                    let bytes = value.len();
                    if value.chars().any(|c| c.len_utf8() > 2) {
                        return unsupported(
                            "IsolatedStorage.SetEncrypted of characters of three or more UTF-8 bytes (the length limit is pinned for one- and two-byte characters only)",
                        );
                    }
                    if bytes == 215 || (secret && bytes >= 215) {
                        return unsupported(
                            "IsolatedStorage.SetEncrypted of 215 bytes or of a long SecretText (not pinned; 215 is a Linux suspect)",
                        );
                    }
                    if bytes >= 216 {
                        return al_builtins::native::raise(
                            "Unable to encrypt data. The provided string is too long.",
                            "EncryptionDataTooLong",
                        );
                    }
                }
                // Set opens the write transaction in every scope (session/lt-cn-session-leads/CnIsolatedStorageLeads.al:
                // 238-263); a SetEncrypted is undone by a rollback (misc/lt-cu-odd-leads/CuTryTests.al:220-230), so it is
                // a write of the transaction too. Only a Set of a new key is pinned: an overwrite outside a write
                // transaction would have to choose whether it opens one, so it is refused (probe wanted, docs/probes-needed.md,
                // IsolatedStorage row); inside one the phase cannot change.
                if self.isolated_storage.contains_key(&slot) && self.tx.tx_phase != crate::transactions::TX_WRITE {
                    return unsupported(format!(
                        "IsolatedStorage.{method} over an existing key outside a write transaction (whether it opens one is not pinned; probe wanted: IsolatedStorage overwrite and IsInWriteTransaction)"
                    ));
                }
                self.tx.tx_phase = crate::transactions::TX_WRITE;
                self.isolated_set_in_test.insert(slot.clone());
                self.isolated_storage
                    .insert(slot, (value, method == "SetEncrypted", secret));
                Ok(Value::Boolean(true))
            }
            "Contains" => {
                let hit = self.isolated_storage.get(&slot).cloned();
                if n == 2 {
                    args[2] = Value::Boolean(hit.as_ref().is_some_and(|(_, enc, _)| *enc));
                }
                Ok(Value::Boolean(hit.is_some()))
            }
            "Delete" => {
                if self.pages.bg_worker.is_some() {
                    // A worker runs read-only: a Delete of a stored key is refused as a Delete on table 2000000107
                    // (session/lt-cn-session-leads/CnIsolatedStorageLeads.al:119-134). A missing key is not pinned there.
                    if !self.isolated_storage.contains_key(&slot) {
                        return unsupported(
                            "IsolatedStorage.Delete of a missing key in a page background task (not pinned)",
                        );
                    }
                    return Err(self.bg_read_only_error(2000000107, "Isolated Storage", self.calling_app(), "Delete"));
                }
                match self.isolated_storage.remove(&slot) {
                    Some(_) => {
                        // A Delete opens the write transaction (session/lt-cn-session-leads/CnIsolatedStorageLeads.al:254-255).
                        self.tx.tx_phase = crate::transactions::TX_WRITE;
                        Ok(Value::Boolean(true))
                    }
                    None if found => Ok(Value::Boolean(false)),
                    None => {
                        unsupported("IsolatedStorage.Delete of a missing key as a statement (error text not pinned)")
                    }
                }
            }
            "Get" => {
                let target = if n <= 2 { 2 } else { 1 };
                let mut stored = self.isolated_storage.get(&slot);
                if self.pages.bg_worker.is_some() && stored.is_some() {
                    // A worker's Get of a key the caller wrote in this test: not committed yet, the worker reads no key; after
                    // the caller's Commit it waits on the caller's lock and fails (session/lt-cn-session-leads/
                    // CnIsolatedStorageLeads.al:445-522). A key the test did not write is not pinned.
                    let committed = self.isolated_committed.get(&slot);
                    if !self.isolated_set_in_test.contains(&slot) {
                        return unsupported(
                            "IsolatedStorage.Get in a page background task of a key the test did not write (not pinned)",
                        );
                    }
                    match committed {
                        None => stored = None,
                        Some(c) if Some(c) == stored => {
                            return Err(crate::transactions::al_error_code(
                                "We can't save your changes right now, because a record in table 'Isolated Storage' is being updated in a transaction done by another session.\\\\You'll have to wait until the other transaction has completed, which may take a while. Please try again later.",
                                "NDBCS:LockTimeout",
                            ));
                        }
                        Some(_) => {
                            return unsupported(
                                "IsolatedStorage.Get in a page background task of a committed key the caller changed again (not pinned)",
                            );
                        }
                    }
                }
                match stored {
                    // A value stored from a SecretText cannot be read into a Text, also in a TryFunction; the target keeps
                    // its value (misc/lt-cu-odd-leads/CuTryTests.al:181-232).
                    Some((_, _, true)) if n == 2 || n == 4 => al_builtins::native::raise(
                        "The operation was aborted to prevent exposing a secret value.",
                        "ForbiddenSecretAccess",
                    ),
                    Some((value, _, _)) => {
                        args[target] = if n == 1 || n == 3 {
                            secret_text(value.clone())
                        } else {
                            Value::Text(value.clone().into())
                        };
                        Ok(Value::Boolean(true))
                    }
                    // A missing key empties the target (session/TestProbeSessionMiscContracts.al:IsolatedStorage_Behaviour,
                    // `get_missing` shows `No ` with the variable cleared).
                    None if found => {
                        args[target] = if n == 1 || n == 3 {
                            secret_text(String::new())
                        } else {
                            Value::Text(String::new().into())
                        };
                        Ok(Value::Boolean(false))
                    }
                    None => unsupported("IsolatedStorage.Get of a missing key as a statement (error text not pinned)"),
                }
            }
            other => unsupported(format!("IsolatedStorage.{other}")),
        }
    }
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(BASE64[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

impl Session {
    /// `EncryptionEnabled`, `EncryptionKeyExists`, `Encrypt` and `Decrypt`, for a tier that has an
    /// encryption key (types/TestVersionCreateEncryptionContracts.al, BC 27.5 on bc-linux). Encrypted text is
    /// 344 characters (256 bytes in base 64) and differs on every call; this session remembers what it
    /// encrypted, so `Decrypt` returns it. Text that is not encrypted data raises the pinned error;
    /// text that looks like encrypted data this session did not make is refused.
    fn encryption_op(&mut self, id: al_ir::BuiltinId, args: &mut [Value]) -> Option<Result<Value, Signal>> {
        Some(match id {
            ids::SYSTEM_ENCRYPTIONENABLED_1 => match self.encryption_key {
                // A key made after a delete counts as a key (session/lt-encryption-key-lifecycle/EncryptionKeyLifecycle.al:
                // EncryptionKey_Lifecycle_StateAfterEachCall `after_create_enabled`).
                crate::EncryptionKey::Present | crate::EncryptionKey::Recreated => Ok(Value::Boolean(true)),
                crate::EncryptionKey::Deleted => Ok(Value::Boolean(false)),
            },
            ids::SYSTEM_ENCRYPTIONKEYEXISTS_1 => {
                Ok(Value::Boolean(self.encryption_key != crate::EncryptionKey::Deleted))
            }
            // Pinned: a key exists at the start; DeleteEncryptionKey with a key leaves none and EncryptionEnabled No, a second one
            // with no key runs without error; CreateEncryptionKey with no key makes one (session/lt-encryption-key-delete/
            // EncryptionKeyDelete.al:Delete_WithKey_ThenWithout_ThenRestore). Creating while a key exists is not pinned.
            ids::SYSTEM_DELETEENCRYPTIONKEY_1 => {
                self.encryption_key = crate::EncryptionKey::Deleted;
                self.encrypted.clear();
                Ok(Value::Null)
            }
            ids::SYSTEM_CREATEENCRYPTIONKEY_1 => {
                // With a key in place the call raises, and the key stays (EncryptionKeyLifecycle.al `create_when_present`, `create_again`).
                if self.encryption_key != crate::EncryptionKey::Deleted {
                    return Some(Err(crate::transactions::al_error_code(
                        "Unable to create a new encryption key. An encryption key already exists.",
                        "EncryptionCreated",
                    )));
                }
                if self.ret_used {
                    return Some(unsupported("the result of CreateEncryptionKey (not pinned)"));
                }
                self.encryption_key = crate::EncryptionKey::Recreated;
                Ok(Value::Null)
            }
            // Encrypt with no key raises; with a key made after a delete it works (EncryptionKeyLifecycle.al `after_delete_encrypt`,
            // `after_create_encrypt`). Decrypt after a delete is not pinned.
            ids::SYSTEM_ENCRYPT_1 if self.encryption_key == crate::EncryptionKey::Deleted => {
                Err(crate::transactions::al_error_code(
                    "An encryption key is required to complete the request.",
                    "EncryptionNotCreated",
                ))
            }
            ids::SYSTEM_DECRYPT_1 if self.encryption_key != crate::EncryptionKey::Present => {
                unsupported("Decrypt after the encryption key was deleted or made again (not pinned)")
            }
            ids::SYSTEM_ENCRYPT_1 => {
                let Some(plain) = args.first().and_then(al_builtins::native::text_of) else {
                    return Some(unsupported("Encrypt of something that is not text"));
                };
                if plain.is_empty() {
                    return Some(Ok(Value::Text(String::new().into())));
                }
                let mut bytes = Vec::with_capacity(256);
                for _ in 0..16 {
                    bytes.extend_from_slice(&self.fresh_guid());
                }
                let cipher = base64(&bytes);
                self.encrypted.insert(cipher.clone(), plain.to_string());
                Ok(Value::Text(cipher.into()))
            }
            ids::SYSTEM_DECRYPT_1 => {
                let Some(cipher) = args.first().and_then(al_builtins::native::text_of) else {
                    return Some(unsupported("Decrypt of something that is not text"));
                };
                if let Some(plain) = self.encrypted.get(cipher.as_str()) {
                    return Some(Ok(Value::Text(plain.clone().into())));
                }
                let looks_encrypted = cipher.len() == 344
                    && cipher
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='));
                if cipher.is_empty() || looks_encrypted {
                    return Some(unsupported(
                        "Decrypt of text this session did not encrypt (the key and the empty text are not pinned)",
                    ));
                }
                Err(crate::transactions::al_error_code(
                    "Unable to decrypt data. The data was encrypted using a different key.",
                    "Encryption:Format",
                ))
            }
            _ => return None,
        })
    }
}
