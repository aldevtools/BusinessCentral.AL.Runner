//! Data-driven tests (BC 29, ADR-059 decision 2): a procedure with `[TestDataSource(Codeunit, DataSet)]` runs once per
//! test case that the data source's `ITestDataSource.ListTestCases` returns, and each case is a result of its own named
//! `Procedure[identifier]`.
//!
//! Pins: corpus al-language-testlibraries/lt-bi-test-framework/BiDataDriven.al (corpus PR 861). Its header (:9-41) is the
//! reference run on BC 29; the asserting tests passed on the corpus CI of BC 29 with the result names used here (for
//! example `Rows[alpha]`, `Id[ padded ]`), and the red part (:542-703) holds the results of the cases that cannot pass.
use super::*;
use al_builtins::value::{Instance, LibraryValue, Value};
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;

/// What the run loop does next with one data-driven procedure or case.
pub(crate) enum Step {
    /// A plain `[Test]` procedure.
    Plain,
    /// A data-driven procedure whose cases are not listed yet: `ListTestCases` is called when the loop reaches it
    /// (BiDataDriven.al:13-16: list once, right before the first case).
    Source,
    /// One case: `GetTestCase(DataSet, index, identifier)` and then the procedure with the context it returns.
    Case { source: Source, index: i32, id: String },
    /// A result decided without running anything.
    Decided(Outcome),
}

/// The run loop's work item: the test routine, the result name and what to do.
pub(crate) struct Item {
    pub routine: usize,
    pub name: String,
    pub step: Step,
}

/// The data source codeunit of a data-driven procedure.
#[derive(Clone)]
pub(crate) struct Source {
    pub object: usize,
    pub list: usize,
    pub get: usize,
    pub data_set: String,
}

/// Why a session ends on BC: the data-driven procedures of the codeunit report nothing there (BiDataDriven.al:29-34,
/// :669-702). The runner does not model a session that ends; the procedures are not run.
pub(crate) const SESSION_ENDS: &str = "BC ends the test session here and reports no data-driven result of the codeunit \
     (BiDataDriven.al:29-34); a session that ends is not modelled";

/// The order BC runs the procedures of a test codeunit in: the plain tests as given (declaration order), then the
/// data-driven procedures in alphabetical order of their names (BiDataDriven.al:11-12, codeunit 81607 at :244-306:
/// declared Zeta, Alpha, Mid, run Alpha, Mid, Zeta). Only names whose order does not depend on letter case are pinned:
/// otherwise every data-driven procedure is not run.
pub(crate) fn queue(lowered: &Lowered, object: usize, plain: &[usize], data_driven: &[usize]) -> VecDeque<Item> {
    let obj = &lowered.program.objects[object];
    let name = |r: usize| obj.routines[r].name.clone();
    let mut items: VecDeque<Item> = plain
        .iter()
        .map(|&r| Item {
            routine: r,
            name: name(r),
            step: Step::Plain,
        })
        .collect();
    let mut by_name = data_driven.to_vec();
    by_name.sort_by_key(|&r| obj.routines[r].name.to_lowercase());
    let mut ordinal = data_driven.to_vec();
    ordinal.sort_by(|&a, &b| obj.routines[a].name.cmp(&obj.routines[b].name));
    let pinned = by_name == ordinal;
    items.extend(by_name.into_iter().map(|r| Item {
        routine: r,
        name: name(r),
        step: if pinned {
            Step::Source
        } else {
            Step::Decided(Outcome::NotRun(
                "the order of data-driven procedures whose names differ in letter case is not pinned".into(),
            ))
        },
    }));
    items
}

/// The data source of procedure `routine`, or why it cannot be run.
fn source_of(lowered: &Lowered, object: usize, routine: usize) -> Result<Source, String> {
    let r = &lowered.program.objects[object].routines[routine];
    let attr = r
        .attributes
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("TestDataSource"))
        .ok_or("no TestDataSource attribute")?;
    let (id, data_set) = match attr.args.as_slice() {
        [al_ir::Const::Integer(id), al_ir::Const::Text(set)] => (*id, set.clone()),
        _ => {
            return Err(format!(
                "a data-driven test whose attribute is not a codeunit id and a text: {}",
                attr.text
            ));
        }
    };
    // BiDataDriven.al:146-149 and every data-driven procedure of the file: one parameter, `Interface ITestContext`.
    if r.params != 1 {
        return Err("a data-driven test procedure without exactly one parameter".into());
    }
    let program = &lowered.program;
    let object = program
        .objects
        .heads()
        .iter()
        .position(|o| o.is_kind(al_ir::ObjKind::Codeunit) && o.id == Some(id as u32))
        .ok_or_else(|| format!("the data source codeunit {id} is not in the compiled apps"))?;
    let src = &program.objects[object];
    if src.uncompiled {
        return Err(format!("the data source codeunit {id} is not compiled"));
    }
    let find = |n: &str| {
        src.routines
            .iter()
            .position(|r| !r.is_trigger && r.name.eq_ignore_ascii_case(n))
            .ok_or_else(|| format!("the data source codeunit {id} has no {n}"))
    };
    let (list, get) = (find("ListTestCases")?, find("GetTestCase")?);
    // BiDataDriven.al:29-34: a source whose ListTestCases contains a DataSourceContext member call, even one it never
    // reaches, ends the session of every codeunit that uses it; so does a call in GetTestCase. Any call in the source
    // codeunit counts here.
    let calls_context = src.routines.iter().any(|r| {
        let mut found = false;
        al_ir::types::for_each_expr_post(&r.body, &mut |e| {
            if let al_ir::Expr::Builtin { id, .. } = e {
                found |= id.name().starts_with("DataSourceContext.");
            }
        });
        found
    });
    if calls_context {
        return Err(SESSION_ENDS.into());
    }
    // Whether the framework keeps one instance of the source codeunit between its calls is not pinned (the sources of
    // the corpus have no globals); a source with globals of its own that is not SingleInstance is not run.
    let single = src
        .properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.eq_ignore_ascii_case("true"));
    if !src.globals.is_empty() && !single {
        return Err(format!(
            "the data source codeunit {id} has global variables: whether they keep their values between the calls of the test framework is not pinned"
        ));
    }
    for r in [list, get] {
        if let Some(b) = blocked_site(lowered, object, r) {
            return Err(b.text);
        }
    }
    Ok(Source {
        object,
        list,
        get,
        data_set,
    })
}

/// A `DataSourceContext` as the test framework hands it to a data source.
fn context(lowered: &Lowered, source: &Source, routine: usize, at: usize) -> Value {
    let ty = &lowered.program.objects[source.object].routines[routine].locals[at].ty;
    match Value::default_for(ty) {
        Value::Library(kind, cell) => {
            *cell.borrow_mut() = LibraryValue::HandedContext;
            Value::Library(kind, cell)
        }
        other => other,
    }
}

/// The rules for identifiers, checked when the list is read (BiDataDriven.al:24-26): not blank (empty or spaces only), no
/// '[' or ']', no duplicate ('same' and 'Same' differ, :123-137). A list that breaks a rule is one failed result named
/// after the procedure; the texts are the reference results of codeunits 81635-81637 (BiDataDriven.al:636, :649, :662),
/// where BC puts `Data-driven test '<procedure>': ` before them (the reference run's log). Only one broken rule per list
/// is pinned: a list that breaks more is not run.
fn check_ids(proc_name: &str, ids: &[String]) -> Result<(), Outcome> {
    let mut broken = Vec::new();
    let mut seen = HashSet::new();
    for id in ids {
        if id.chars().all(|c| c == ' ') {
            broken.push("ListTestCases returned a blank identifier. Every test case must have a non-blank, distinct identifier that contains no square brackets.".to_string());
        } else if id.trim().is_empty() {
            return Err(Outcome::NotRun(
                "a test case identifier of white space other than spaces: whether it counts as blank is not pinned"
                    .into(),
            ));
        } else if id.contains(['[', ']']) {
            broken.push(format!("ListTestCases returned the identifier '{id}', which contains a square bracket ('[' or ']'). Test case identifiers must not contain square brackets; they delimit the case name in 'Method[identifier]'."));
        } else if !seen.insert(id.as_str()) {
            broken.push(format!(
                "ListTestCases returned the duplicate identifier '{id}'. Every test case must have a distinct identifier."
            ));
        }
    }
    match broken.as_slice() {
        [] => Ok(()),
        [one] => Err(Outcome::Fail(format!("Data-driven test '{proc_name}': {one}"))),
        _ => Err(Outcome::NotRun(
            "a list of test cases that breaks more than one identifier rule: which one BC reports is not pinned".into(),
        )),
    }
}

/// What listing the cases of a data-driven procedure gave.
pub(crate) enum Listed {
    /// The items to run next, in order (none for an empty list, BiDataDriven.al:179-195).
    Items(Vec<Item>),
    /// BC ends the session (an error in ListTestCases, BiDataDriven.al:29-30, :669-680).
    SessionEnds,
}

/// Calls `ListTestCases` for data-driven procedure `routine` and turns its answer into the cases to run.
pub(crate) fn list(session: &mut Session, lowered: &Lowered, object: usize, routine: usize, name: &str) -> Listed {
    let decided = |o: Outcome| {
        Listed::Items(vec![Item {
            routine,
            name: name.to_string(),
            step: Step::Decided(o),
        }])
    };
    let source = match source_of(lowered, object, routine) {
        Ok(s) => s,
        Err(why) if why == SESSION_ENDS => return Listed::SessionEnds,
        Err(why) => return decided(Outcome::NotRun(why)),
    };
    let instance = session.codeunit_instance(source.object);
    let args = vec![
        Value::Text(source.data_set.clone().into()),
        context(lowered, &source, source.list, 1),
    ];
    let changes = session.store_mut().change_counter();
    let called = session.call(source.object, source.list, instance, args);
    if session.store_mut().change_counter() != changes {
        session.rollback_transaction();
        return decided(Outcome::NotRun(
            "ListTestCases wrote to the database: when the test framework commits that is not pinned".into(),
        ));
    }
    let list = match called {
        Ok((Value::List(items), _)) => items,
        Ok((other, _)) => {
            return decided(Outcome::NotRun(format!(
                "ListTestCases returned {}",
                al_builtins::describe::Describe::describe(&other)
            )));
        }
        Err(Signal::Error(_)) => {
            session.rollback_transaction();
            return Listed::SessionEnds;
        }
        Err(Signal::Unsupported(why)) => {
            session.rollback_transaction();
            return decided(Outcome::NotRun(why));
        }
        Err(other) => return decided(Outcome::NotRun(format!("ListTestCases: {}", other.describe()))),
    };
    let mut ids = Vec::new();
    for v in list.borrow().iter() {
        match al_builtins::native::text_of(v) {
            Some(t) => ids.push(t.to_string()),
            None => {
                return decided(Outcome::NotRun(
                    "ListTestCases returned a value that is not text".into(),
                ));
            }
        }
    }
    if let Err(o) = check_ids(name, &ids) {
        return decided(o);
    }
    Listed::Items(
        ids.into_iter()
            .enumerate()
            .map(|(i, id)| Item {
                routine,
                // BiDataDriven.al:19 and the corpus CI of BC 29: `Rows[alpha]`, identifiers unchanged.
                name: format!("{name}[{id}]"),
                step: Step::Case {
                    source: source.clone(),
                    index: i as i32 + 1,
                    id,
                },
            })
            .collect(),
    )
}

/// One case: `GetTestCase` (BiDataDriven.al:13-18: called right before the case runs, with the 1-based index and the
/// identifier unchanged), then the test procedure with the context it returned. An error in GetTestCase fails the case
/// and the procedure does not run for it (:27-28, :623-629).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_case(
    session: &mut Session,
    lowered: &Lowered,
    object: usize,
    routine: usize,
    test_instance: Rc<RefCell<Instance>>,
    source: &Source,
    index: i32,
    id: &str,
) -> Result<(Value, Vec<Value>), Signal> {
    let instance = session.codeunit_instance(source.object);
    let args = vec![
        Value::Text(source.data_set.clone().into()),
        Value::Integer(index),
        Value::Text(id.to_string().into()),
        context(lowered, source, source.get, 3),
    ];
    let changes = session.store_mut().change_counter();
    let (got, _) = session.call(source.object, source.get, instance, args)?;
    if session.store_mut().change_counter() != changes {
        return Err(Signal::Unsupported(
            "GetTestCase wrote to the database: whether that belongs to the test case is not pinned".into(),
        ));
    }
    // BiDataDriven.al:20: the ITestContext GetTestCase returns is what the procedure receives.
    if !matches!(got, Value::Codeunit(_)) {
        return Err(Signal::Unsupported(format!(
            "GetTestCase returned {} instead of a codeunit",
            al_builtins::describe::Describe::describe(&got)
        )));
    }
    session.call(object, routine, test_instance, vec![got])
}
