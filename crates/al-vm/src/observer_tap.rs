//! The cell tap: per test, how many `Expect` cells of the corpus fixture codeunit 69160 "Probe Observer" matched what the
//! test observed. Report only: it reads the fixture's state and changes nothing AL can see or any outcome.
//!
//! A test calls `Observe(name, value)` per question and `Expect(name, value)` per answer of real BC, and `Verify()` compares the
//! two lists position by position and stops at the first wrong cell (`_fixtures/ProbeObserver.Codeunit.al` `Verify`). The tap
//! counts the `Expect` calls, and at the entry of `Verify` reads the instance's `Pairs`, `Expected` and `Ignored` variables and
//! compares them cell by cell the way `Verify` does, past the first wrong cell. A cell is one `Expect` pair; "same position" is
//! the fixture's own rule, so after one missing observation the later cells are wrong too.
//!
//! Off unless asked for (`Session::enable_cell_tap`): then a routine call costs one branch.

use al_builtins::value::{Instance, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// Pair separator (U+241E) and name/value separator (U+241F) of the fixture.
const RS: char = '\u{241E}';
const US: char = '\u{241F}';

/// One `Expect` cell that did not match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellMismatch {
    pub key: String,
    pub expected: String,
    /// The pair observed at the same position as `key=value`, `None` when the test observed fewer pairs (never reached).
    pub actual: Option<String>,
}

/// What the cell tap saw in one test.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellRecord {
    /// `Expect` calls the test made.
    pub expected: u32,
    /// Cells a `Verify` found equal.
    pub matched: u32,
    /// Cells a `Verify` found wrong or missing, in order.
    pub mismatched: Vec<CellMismatch>,
    /// Observations beyond the expected ones, summed over the `Verify` calls.
    pub extra: u32,
    /// `Verify` calls that were reached.
    pub verifies: u32,
}

impl CellRecord {
    /// Cells no `Verify` looked at: the test ended or failed before it.
    pub fn unreached(&self) -> u32 {
        self.expected
            .saturating_sub(self.matched + u32::try_from(self.mismatched.len()).unwrap_or(u32::MAX))
    }

    /// `key: expected X, actual Y` of the first wrong cell.
    pub fn first_mismatch(&self) -> Option<String> {
        self.mismatched.first().map(|m| match &m.actual {
            Some(a) => format!("{}: expected [{}] actual [{}]", m.key, m.expected, a),
            None => format!("{}: expected [{}] never observed", m.key, m.expected),
        })
    }
}

fn pairs(text: &str) -> Vec<&str> {
    let mut v: Vec<&str> = text.split(RS).collect();
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

fn name_of(pair: &str) -> &str {
    pair.split(US).next().unwrap_or("")
}

fn show(pair: &str) -> String {
    pair.replacen(US, "=", 1)
}

/// What `Verify` compares: `got` is the observed pairs, `want` the expected ones, `ignored` the names not asserted.
pub fn compare(got: &str, want: &str, ignored: &[String]) -> CellRecord {
    let want_pairs = pairs(want);
    let got_pairs: Vec<&str> = pairs(got)
        .into_iter()
        .filter(|p| ignored.is_empty() || !ignored.iter().any(|i| i == name_of(p)))
        .collect();
    let mut r = CellRecord {
        verifies: 1,
        ..CellRecord::default()
    };
    for (i, w) in want_pairs.iter().enumerate() {
        match got_pairs.get(i) {
            Some(g) if g == w => r.matched += 1,
            other => {
                let value = w.split_once(US).map_or("", |(_, v)| v);
                r.mismatched.push(CellMismatch {
                    key: name_of(w).to_string(),
                    expected: value.to_string(),
                    actual: other.map(|g| show(g)),
                });
            }
        }
    }
    r.extra = u32::try_from(got_pairs.len().saturating_sub(want_pairs.len())).unwrap_or(u32::MAX);
    r
}

/// The tap's state in a session.
#[derive(Debug, Default)]
pub(crate) struct Tap {
    /// Objects of the program that are the fixture (id 69160, name "Probe Observer").
    objects: Vec<usize>,
    pub(crate) record: CellRecord,
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(t) | Value::Code(t) => t.as_str().to_string(),
        _ => String::new(),
    }
}

impl crate::Session {
    /// Starts counting cells of the fixture codeunit (`al-test-cli --cell-report`, `--junit`).
    pub fn enable_cell_tap(&mut self) {
        let objects = self
            .engine
            .program
            .objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.id == Some(69160) && o.name == "Probe Observer")
            .map(|(i, _)| i)
            .collect();
        self.coverage.cells = Some(Box::new(Tap {
            objects,
            record: CellRecord::default(),
        }));
    }

    /// What the tap saw since the last call; `None` when it is off or the test did not use the fixture.
    pub fn take_cell_record(&mut self) -> Option<CellRecord> {
        let tap = self.coverage.cells.as_mut()?;
        let r = std::mem::take(&mut tap.record);
        (r.expected > 0 || r.verifies > 0).then_some(r)
    }

    /// Called at the entry of every routine while the tap is on.
    pub(crate) fn tap_cells(&mut self, object: usize, routine: usize, instance: &Rc<RefCell<Instance>>) {
        let Some(tap) = self.coverage.cells.as_mut() else {
            return;
        };
        if !tap.objects.contains(&object) {
            return;
        }
        let obj = &self.engine.program.objects[object];
        let name = obj.routines[routine].name.as_str();
        if name.eq_ignore_ascii_case("Expect") {
            tap.record.expected += 1;
        } else if name.eq_ignore_ascii_case("Verify") {
            let inst = instance.borrow();
            let global = |n: &str| {
                obj.globals
                    .iter()
                    .position(|s| s.name == n)
                    .and_then(|i| inst.globals.get(i))
            };
            let (Some(got), Some(want)) = (global("Pairs"), global("Expected")) else {
                return;
            };
            let ignored: Vec<String> = match global("Ignored") {
                Some(Value::List(l)) => l.borrow().iter().map(text_of).collect(),
                _ => Vec::new(),
            };
            let one = compare(&text_of(got), &text_of(want), &ignored);
            let rec = &mut tap.record;
            rec.verifies += 1;
            rec.matched += one.matched;
            rec.extra += one.extra;
            rec.mismatched.extend(one.mismatched);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(items: &[(&str, &str)]) -> String {
        items.iter().map(|(k, v)| format!("{k}{US}{v}{RS}")).collect()
    }

    #[test]
    fn all_equal_cells_match() {
        let s = p(&[("a", "1"), ("b", "2")]);
        let r = compare(&s, &s, &[]);
        assert_eq!((r.matched, r.mismatched.len(), r.extra), (2, 0, 0));
    }

    #[test]
    fn it_goes_past_the_first_wrong_cell() {
        let got = p(&[("a", "1"), ("b", "x"), ("c", "3"), ("d", "y")]);
        let want = p(&[("a", "1"), ("b", "2"), ("c", "3"), ("d", "4")]);
        let r = compare(&got, &want, &[]);
        assert_eq!(r.matched, 2);
        assert_eq!(r.mismatched.len(), 2);
        assert_eq!(r.first_mismatch().unwrap(), "b: expected [2] actual [b=x]");
    }

    #[test]
    fn missing_and_extra_observations() {
        let got = p(&[("a", "1")]);
        let want = p(&[("a", "1"), ("b", "2")]);
        let r = compare(&got, &want, &[]);
        assert_eq!(r.mismatched[0].actual, None);
        let r = compare(&want, &got, &[]);
        assert_eq!((r.matched, r.extra), (1, 1));
    }

    #[test]
    fn ignored_names_are_not_asserted() {
        let got = p(&[("dump", "zzz"), ("a", "1")]);
        let want = p(&[("a", "1")]);
        let r = compare(&got, &want, &["dump".to_string()]);
        assert_eq!((r.matched, r.mismatched.len()), (1, 0));
    }

    #[test]
    fn unreached_cells_are_the_expected_ones_no_verify_saw() {
        let r = CellRecord {
            expected: 5,
            matched: 2,
            mismatched: vec![CellMismatch {
                key: "k".into(),
                expected: "v".into(),
                actual: None,
            }],
            ..CellRecord::default()
        };
        assert_eq!(r.unreached(), 2);
    }
}
