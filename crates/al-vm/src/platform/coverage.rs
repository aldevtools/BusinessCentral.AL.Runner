//! The three code coverage tables: Code Coverage (2000000049), Code Coverage Test Lookup
//! (2000000288) and Code Coverage Tests Run (2000000289).
//!
//! While `CodeCoverageLog(true, ...)` is on, the session records which statements ran (hits per
//! source position), which objects were entered, which `[Test]` procedures started and which
//! methods ran inside each of them. The tables are stored tables that start empty (what real BC
//! holds until something is recorded: codeunits/TestCodeCoverageTable.al:`DeleteAll_AfterRecording_
//! LeavesTableEmpty`). A read of one of them first writes what was recorded into the three tables
//! ([`Session::coverage_flush`]); `DeleteAll` without filters forgets what the table showed.
//!
//! What the corpus pins:
//! - A Code Coverage row's `Line No.` is a line of the object's source file; the line of a
//!   procedure's declaration is a `Trigger/Function` row; a statement is a `Code` row with
//!   `No. of Hits`, 0 for one that did not run (codeunits/TestCodeCoverageTable.al:73-104, lines 33,
//!   36, 37). A tableextension, pageextension and reportextension have `Code` rows with hits
//!   (codeunits/TestCodeCoverageTableExtension.al:201-239).
//! - A `[Test]` procedure that starts while recording has one Tests Run row (its codeunit's id,
//!   the method's name, the owning app) and every method that runs inside it, the test procedure
//!   itself excluded, one Test Lookup row, a codeunit's `OnRun` trigger included
//!   (codeunits/TestCodeCoveragePerTest.al:54-168). A procedure that runs outside any test
//!   leaves no per-test row (same file, `PerTestTables_NoTestStartedWhileRecording_AreEmpty...`).
//! - `Test ID` and `Method ID` are only compared between rows, never to a literal. The Test ID is the same for every test
//!   recorded in one recording and in two recordings of one test, and is not 1 (test-framework/code-coverage/W6CodeCoverageIds.al:96-97,
//!   :111-112, :167-168). Its value is not pinned ([`Session::RUN_TEST_ID`]): a read of it outside those shapes is refused
//!   (al-sema lower/coverage_ids.rs; a FieldRef read here). Tests Run is keyed by Test ID alone, so the
//!   runtime cannot hold two recorded tests at once: a read of the per-test tables is refused then.
//!
//! - Of an app compiled without its source in the symbols, every row's `Line` holds the platform's
//!   "source cannot be viewed" text; the `begin` and `end` lines are `Empty` rows; the status is
//!   Covered, Not Covered or Non Applicable (codeunits/TestCodeCoverageLineTextContracts.al).
//!
//! Not pinned, and set to a stated default (docs/probes-needed.md, "Code coverage rows"): `Line` of an
//! app that shows its source, the `Method Id` column, the hits of a `Trigger/Function` row.

use crate::Session;
use al_builtins::filter::FieldFilter;
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_data::{Row, TableDef};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CODE_COVERAGE: u32 = 2_000_000_049;
pub(crate) const TEST_LOOKUP: u32 = 2_000_000_288;
pub(crate) const TESTS_RUN: u32 = 2_000_000_289;
pub(crate) const COVERAGE_TABLES: [u32; 3] = [CODE_COVERAGE, TEST_LOOKUP, TESTS_RUN];

/// What `Line` holds for an app compiled without its source in the symbols, the default
/// (codeunits/TestCodeCoverageLineTextContracts.al). An app with `ShowMyCode` or
/// `IncludeSourceInSymbolFile` would show the source text: not pinned (docs/probes-needed.md).
const NO_SOURCE: &str = "The extension that declares this object has its manifest property ShowMyCode (being deprecated) or IncludeSourceInSymbolFile set to false. This means that the source code cannot be viewed.";

/// What recording noted since the tables were last emptied.
#[derive(Default, PartialEq)]
pub(crate) struct Recorder {
    /// Hits per position id (`Program::positions`), grown when the first statement runs.
    hits: Vec<u32>,
    /// Objects (indexes into `Program::objects`) with a routine that ran while recording.
    entered: BTreeSet<usize>,
    /// `[Test]` procedures that started while recording (each row gets the one Test ID, `Session::RUN_TEST_ID`).
    tests: Vec<(usize, usize)>,
    /// (test id, object, routine) of the methods that ran inside a recorded test.
    lookups: BTreeSet<(i32, usize, usize)>,
    /// The recorded test that is running now.
    current_test: Option<i32>,
    /// Something was recorded since the rows were last written.
    dirty: bool,
    /// CodeCoverageRefresh ran while logging is on (cleared when logging starts): a later read shows that snapshot.
    refreshed_while_logging: bool,
}

/// What a routine entry noted: the test that was running before it started one.
pub(crate) struct TestScope(Option<i32>);

impl Recorder {
    pub(crate) fn reset_refreshed(&mut self) {
        self.refreshed_while_logging = false;
    }
}

impl Session {
    /// Counts one run of the statement at position `id`.
    #[inline]
    pub(crate) fn coverage_hit(&mut self, id: u32) {
        let hits = &mut self.coverage.recorder.hits;
        let id = id as usize;
        if hits.len() <= id {
            hits.resize(self.engine.program.positions.len().max(id + 1), 0);
        }
        hits[id] += 1;
        self.coverage.recorder.dirty = true;
    }

    /// A routine starts while recording. Returns what to restore when it ends, if it started a test.
    ///
    /// Only a `[Test]` procedure that AL code calls while no recorded test runs is noted as a test
    /// (codeunits/TestCodeCoveragePerTest.al:RecordedTest_RunsTripled is called directly). A test the
    /// runner starts while recording is on (codeunits/PWCodeCoverageRows.al:Outlives_2_...) is not, nor is
    /// a test procedure that runs inside a recorded one (NestedTest_TestsRunAndLookupRows: one Tests
    /// Run row for the outer test, no Test Lookup row for the inner `[Test]` procedure).
    pub(crate) fn coverage_enter(&mut self, object: usize, routine: usize) -> Option<TestScope> {
        let engine = self.engine.clone();
        let r = &engine.program.objects[object].routines[routine];
        // Under a test runner codeunit (ADR-066) its frames below the test do not count as AL calling the test.
        let from_al = self.frames.len() > self.test_frame_base();
        let rec = &mut self.coverage.recorder;
        rec.entered.insert(object);
        rec.dirty = true;
        if r.attributes.iter().any(|a| a.name.eq_ignore_ascii_case("Test")) {
            if from_al && rec.current_test.is_none() {
                rec.tests.push((object, routine));
                let previous = rec.current_test.replace(rec.tests.len() as i32);
                return Some(TestScope(previous));
            }
            return None;
        }
        if let Some(test) = rec.current_test {
            rec.lookups.insert((test, object, routine));
        }
        None
    }

    /// A rollback took the rows written by the last read: write what was recorded again at the next read.
    pub(crate) fn coverage_rolled_back(&mut self) {
        let rec = &mut self.coverage.recorder;
        if !rec.entered.is_empty() || !rec.tests.is_empty() {
            rec.dirty = true;
        }
    }

    pub(crate) fn coverage_leave(&mut self, scope: TestScope) {
        self.coverage.recorder.current_test = scope.0;
    }

    /// A read of a coverage table: what was recorded is in the tables first.
    pub(crate) fn coverage_read(&mut self, t: &TableDef, temporary: bool) -> Result<(), Signal> {
        if temporary {
            return Ok(());
        }
        let Some(table) = self.coverage_table(t) else {
            return Ok(());
        };
        if table != CODE_COVERAGE && self.several_recorded_tests() {
            return Err(Signal::unsupported(format!(
                "a read of {}: several [Test] procedures were recorded and BC gives them one Test ID, which the Tests Run \
                 table keys its rows by (test-framework/code-coverage/W6CodeCoverageIds.al:111-112); the runtime cannot hold those rows",
                t.name
            )));
        }
        // While logging is on the tables hold nothing new: what was recorded lands in them when logging stops or at
        // CodeCoverageRefresh (metadata/code-coverage/CodeCoverageRefreshLogged.al: `rows before nonzero` No).
        if self.coverage.code_coverage {
            // After a refresh the table stays the snapshot the refresh wrote; whether BC shows later hits live is not pinned
            // (the test cannot tell the two apart): assumption code-coverage-snapshot-while-logging.
            if self.coverage.recorder.refreshed_while_logging {
                al_builtins::assumptions::assume(
                    al_builtins::assumptions::Assumption::CodeCoverageSnapshotWhileLogging,
                );
            }
            return Ok(());
        }
        self.coverage_flush()
    }

    /// `CodeCoverageRefresh()`.
    pub(crate) fn coverage_refresh(&mut self) -> Result<(), Signal> {
        if self.coverage.code_coverage {
            self.coverage.recorder.refreshed_while_logging = true;
        }
        self.coverage_flush()
    }

    /// More than one `[Test]` procedure was recorded since the per-test tables were emptied.
    fn several_recorded_tests(&self) -> bool {
        self.coverage.recorder.tests.len() > 1
    }

    /// Whether field `field` of `t` is the Test ID of a per-test coverage table.
    pub(crate) fn is_coverage_test_id(&self, t: &TableDef, field: u32) -> bool {
        self.engine.program.objects[t.object as usize]
            .id
            .is_some_and(|id| al_builtins::platform::COVERAGE_TEST_ID.contains(&(id, field)))
    }

    fn coverage_table(&self, t: &TableDef) -> Option<u32> {
        let id = self.engine.program.objects[t.object as usize].id?;
        COVERAGE_TABLES.contains(&id).then_some(id)
    }

    /// `DeleteAll` without filters leaves the table empty, and what it showed is forgotten.
    pub(crate) fn coverage_cleared(&mut self, t: &TableDef, temporary: bool, filters: &[FieldFilter]) {
        if temporary || !filters.is_empty() {
            return;
        }
        let rec = &mut self.coverage.recorder;
        match self.engine.program.objects[t.object as usize].id {
            Some(CODE_COVERAGE) => {
                rec.hits.clear();
                rec.entered.clear();
            }
            Some(TEST_LOOKUP) => rec.lookups.clear(),
            // The lookups name the tests: they go with them.
            Some(TESTS_RUN) => {
                rec.tests.clear();
                rec.lookups.clear();
                rec.current_test = None;
            }
            _ => {}
        }
    }

    /// Writes the recorded data into the three tables, replacing what they held.
    fn coverage_flush(&mut self) -> Result<(), Signal> {
        if !self.coverage.recorder.dirty {
            return Ok(());
        }
        self.coverage.recorder.dirty = false;
        let engine = self.engine.clone();
        let program = &engine.program;
        for (oi, o) in program.objects.heads().iter().enumerate() {
            let (Some(id), al_ir::ObjKind::Table) = (o.id, if o.uncompiled { al_ir::ObjKind::Other } else { o.kind })
            else {
                continue;
            };
            if !COVERAGE_TABLES.contains(&id) {
                continue;
            }
            let Some(t) = engine.tables[oi].as_ref() else { continue };
            let rows = match id {
                CODE_COVERAGE => self.coverage_rows(t),
                // Their reads are refused (`coverage_read`).
                _ if self.several_recorded_tests() => Vec::new(),
                TEST_LOOKUP => self.lookup_rows(t),
                _ => self.tests_run_rows(t),
            };
            let cx = self.cx.clone();
            // One DeleteAll and one Insert per row, in one pass (al-data `memory/replace.rs`).
            self.store
                .replace_all(&cx, t, rows)
                .map_err(|e| Signal::unsupported(format!("{e}")))?;
        }
        Ok(())
    }

    /// The option value of field `id` named `member` (ignoring case and spaces).
    fn option_member(t: &TableDef, id: u32, member: &str) -> Value {
        let blank = super::option(t, id, 0);
        let Value::Option(_, names) = &blank else { return blank };
        let want: String = member.chars().filter(|c| !c.is_whitespace()).collect();
        match names.iter().position(|n| {
            n.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .eq_ignore_ascii_case(&want)
        }) {
            Some(i) => Value::Option(i as i32, names.clone()),
            None => blank,
        }
    }

    /// The Code Coverage rows of every object a routine of which ran while recording
    /// (codeunits/PWCodeCoverageRows.al:Rows_AllLinesOfSmallCodeunit_OneProcedureRun and the `Hits_*` tests,
    /// BC 27.5; onprem test-framework/code-coverage/TestCodeCoverageSourceLinesContracts.al for the lines inside a routine):
    /// - one `Object` row at line 0, status Non Applicable;
    /// - for each routine, one row for every line from its declaration to its `end`: the declaration is
    ///   `Trigger/Function`, a line where a statement starts is `Code`, every other line (`var`, a variable,
    ///   `begin`, a blank line, a comment, `end`) is `Empty`. Nothing outside the routines has a row;
    /// - the hits of a `Code` line are the largest count of the statements that start on it, its status is
    ///   Covered when all of them ran, Partially Covered when some did and Not Covered when none did.
    fn coverage_rows(&self, t: &TableDef) -> Vec<Row> {
        let program = &self.engine.program;
        let rec = &self.coverage.recorder;
        let mut out = Vec::new();
        // What every row shares, made once per write (a table of about 200,000 rows after a posting test): the blank row,
        // the column positions, the option values and the text of an app without its source.
        let template = super::blank(t);
        let [p_type, p_id, p_line, p_kind, p_text, p_hits, p_status] =
            [1, 2, 3, 4, 5, 6, 7].map(|id| super::pos(t, id));
        let no_source = Value::Text(NO_SOURCE.into());
        let mut types: BTreeMap<&str, Value> = BTreeMap::new();
        let kind = |name: &str| Self::option_member(t, 4, name);
        let (object_kind, header_kind, code_kind, empty_kind) =
            (kind("Object"), kind("Trigger/Function"), kind("Code"), kind("Empty"));
        let status = |name: &str| Self::option_member(t, 7, name);
        let (not_applicable, not_covered, covered, partially_covered) = (
            status("NonApplicable"),
            status("NotCovered"),
            status("Covered"),
            status("PartiallyCovered"),
        );
        // The positions of each object, found once per engine: a scan of all positions per entered object, on every read
        // of the Code Coverage table, was positions x objects. The positions a cache supplies are not read for it
        // (ADR-054): only those of the entered objects are, below.
        let by_object = self
            .engine
            .positions_by_object
            .get_or_init(|| program.positions.ids_by_object(program.objects.len()));
        // In primary key order (Object Type, Object ID; the lines of an object come in order), so the store's sort of the
        // rows finds them sorted. A stable sort: of two objects with one key the first written stays first.
        let mut entered: Vec<(i32, i32, usize)> = rec
            .entered
            .iter()
            .filter_map(|&oi| {
                let o = &program.objects[oi];
                let ty = match types
                    .entry(o.kind.label())
                    .or_insert_with(|| Self::option_member(t, 1, o.kind.label()))
                {
                    Value::Option(n, _) => *n,
                    _ => 0,
                };
                Some((ty, o.id? as i32, oi))
            })
            .collect();
        entered.sort_by_key(|&(ty, id, _)| (ty, id));
        let mut row = |oi: usize, line: u32, text: Option<&str>, kind: &Value, hits: u32, status: &Value| {
            let o = &program.objects[oi];
            let mut r = template.clone();
            r[p_type] = types[o.kind.label()].clone();
            r[p_id] = Value::Integer(o.id.unwrap_or(0) as i32);
            r[p_line] = Value::Integer(line as i32);
            r[p_kind] = kind.clone();
            r[p_text] = text.map_or_else(|| no_source.clone(), |s| Value::Text(s.into()));
            r[p_hits] = Value::Integer(hits as i32);
            r[p_status] = status.clone();
            out.push(r);
        };
        for &(_, _, oi) in &entered {
            let o = &program.objects[oi];
            // The positions that are no statement: the declaration, `begin` and `end` of each routine.
            let mut frame: BTreeSet<u32> = BTreeSet::new();
            for r in &o.routines {
                frame.extend([r.pos, r.begin_pos, r.end_pos]);
            }
            // Per line: (statements, statements that ran, largest count).
            let mut statements: BTreeMap<u32, (u32, u32, u32)> = BTreeMap::new();
            for &id in &by_object[oi] {
                let id = id as usize;
                let p = &program.positions[id];
                if frame.contains(&(id as u32)) {
                    continue;
                }
                let hits = rec.hits.get(id).copied().unwrap_or(0);
                let e = statements.entry(p.line).or_default();
                e.0 += 1;
                e.1 += u32::from(hits > 0);
                e.2 = e.2.max(hits);
            }
            // The lines from each routine's declaration to its `end`, in order and each once.
            let mut headers: Vec<u32> = Vec::with_capacity(o.routines.len());
            let mut spans: Vec<(u32, u32)> = Vec::with_capacity(o.routines.len());
            for r in &o.routines {
                let Some(first) = program.pos(r.pos).map(|p| p.line) else {
                    continue;
                };
                let last = program.pos(r.end_pos).map_or(first, |p| p.line).max(first);
                headers.push(first);
                spans.push((first, last));
            }
            headers.sort_unstable();
            spans.sort_unstable();
            let mut lines: Vec<u32> = Vec::new();
            for (first, last) in spans {
                let from = lines.last().map_or(first, |&l| first.max(l + 1));
                lines.extend(from..=last);
            }
            // An app that includes its source in the symbols shows the line as written
            // (onprem test-framework/code-coverage/TestCodeCoverageSourceLinesContracts.al:60-74); any other shows the platform's text.
            let source: Option<&Vec<String>> = o
                .routines
                .iter()
                .find_map(|r| program.pos(r.pos))
                .and_then(|p| program.sources.get(p.file as usize))
                .filter(|sf| {
                    program.apps.get(sf.app as usize).is_some_and(|a| a.include_source) && !sf.lines.is_empty()
                })
                .map(|sf| &sf.lines);
            let text = |line: u32| source.map(|l| l.get(line as usize - 1).map_or("", String::as_str));
            row(oi, 0, None, &object_kind, 0, &not_applicable);
            for &line in &lines {
                let text = text(line);
                match statements.get(&line) {
                    _ if headers.binary_search(&line).is_ok() => row(oi, line, text, &header_kind, 0, &not_applicable),
                    Some(&(total, ran, max)) => {
                        let status = match (ran, total) {
                            (0, _) => &not_covered,
                            (r, t) if r == t => &covered,
                            _ => &partially_covered,
                        };
                        row(oi, line, text, &code_kind, max, status)
                    }
                    None => row(oi, line, text, &empty_kind, 0, &not_applicable),
                }
            }
        }
        out
    }

    /// The "Method ID" of a Test Lookup row: the routine's index plus one. Kept, not pinned: the corpus pins one
    /// literal (31658110, codeunits/PWCodeCoverageRows.al:309) and that it differs from the test's own Tests Run id
    /// (codeunits/TestCodeCoveragePerTest.al:121), not the rule (docs/probes-needed.md).
    fn method_id(routine: usize) -> Value {
        Value::Integer(routine as i32 + 1)
    }

    /// The "Method ID" of a Tests Run row: one number for every test procedure (codeunits/PWCodeCoverageRows.al:308;
    /// test-framework/code-coverage/W6CodeCoverageIds.al:105-107 for four tests of different bodies, :139 not below 1000, :168 the same in
    /// two recordings).
    const TEST_METHOD_ID: i32 = 469_735_107;

    /// The "Test ID" of every Tests Run and Test Lookup row. Pinned: one value for the tests of a recording and for two
    /// recordings of one test, and not 1 (test-framework/code-coverage/W6CodeCoverageIds.al:111-112, :167-168). The number itself is not
    /// pinned (BC computes it per run, codeunits/TestCodeCoveragePerTest.al:14): every read that would show it is refused
    /// (al-sema lower/coverage_ids.rs, `FieldRef.Value`), so no outcome depends on it.
    const RUN_TEST_ID: i32 = 2;

    fn lookup_rows(&self, t: &TableDef) -> Vec<Row> {
        let program = &self.engine.program;
        self.coverage
            .recorder
            .lookups
            .iter()
            .filter_map(|&(_, oi, ri)| {
                let o = &program.objects[oi];
                let mut r = super::blank(t);
                r[super::pos(t, 1)] = Self::option_member(t, 1, o.kind.label());
                r[super::pos(t, 2)] = Value::Integer(o.id? as i32);
                r[super::pos(t, 3)] = Self::method_id(ri);
                r[super::pos(t, 4)] = Value::Integer(Self::RUN_TEST_ID);
                Some(r)
            })
            .collect()
    }

    fn tests_run_rows(&self, t: &TableDef) -> Vec<Row> {
        let program = &self.engine.program;
        self.coverage
            .recorder
            .tests
            .iter()
            .filter_map(|&(oi, ri)| {
                let o = &program.objects[oi];
                let mut r = super::blank(t);
                r[super::pos(t, 1)] = Value::Integer(Self::RUN_TEST_ID);
                r[super::pos(t, 2)] = Value::Integer(o.id? as i32);
                r[super::pos(t, 3)] = Value::Integer(Self::TEST_METHOD_ID);
                r[super::pos(t, 4)] = Value::Text(o.routines[ri].name.clone().into());
                if let Some(m) = self.module_of_app(o.app as usize) {
                    r[super::pos(t, 5)] = Value::Guid(m.id);
                }
                Some(r)
            })
            .collect()
    }
}
