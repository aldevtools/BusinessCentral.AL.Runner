//! The rankings at the end of a test run's report: "Top refusals" (`--top-refusals`), "Top failures"
//! (`--top-failures`) and `--refusal-detail`. A test a stop option left out is not a refusal (al-runner issue 8492,
//! `TestResult::is_refusal`).
use super::REPORT_TO_STDERR;
use al_testrun::runtime::TestResult;
use al_testrun::{CaseId, Outcome, refusals};

/// The reason of each refused test.
fn refused<'a>(results: &'a [&TestResult]) -> impl Iterator<Item = (CaseId, String)> + 'a {
    results
        .iter()
        .filter(|r| r.is_refusal())
        .filter_map(|r| match &r.outcome {
            Outcome::NotRun(why) => Some((case(r), why.clone())),
            _ => None,
        })
}

fn case(r: &TestResult) -> CaseId {
    CaseId {
        codeunit: r.codeunit,
        name: r.test.clone(),
    }
}

/// The `n` most common reasons of the refused tests, and how many not-run tests a stop option left out; with no refused
/// test nothing is printed.
pub(crate) fn print_top_refusals(results: &[&TestResult], n: usize) {
    let list = refusals::rank(refused(results));
    if list.is_empty() {
        return;
    }
    let mut text = refusals::render_top("not-run tests", &list, n);
    let left_out = results.iter().filter(|r| r.skipped_by_option).count();
    if left_out > 0 {
        text.push_str(&format!(
            "        {left_out} not-run tests a stop option left out are not ranked\n"
        ));
    }
    let (head, rest) = text.split_once('\n').unwrap_or((&text, ""));
    say!("\n{}\n{rest}", crate::style::bold(head));
}

/// The `n` most common reasons of the failed tests (`--top-failures`).
pub(crate) fn print_top_failures(results: &[&TestResult], n: usize) {
    let list = refusals::rank(results.iter().filter_map(|r| match &r.outcome {
        Outcome::Fail(why) => Some((case(r), why.clone())),
        _ => None,
    }));
    say!("\n{}", refusals::render_top_failures("failed tests", &list, n));
}

/// The reasons of the refused tests that contain `needle` (`--refusal-detail`).
pub(crate) fn print_refusal_detail(results: &[&TestResult], needle: &str) {
    let list = refusals::detail(refused(results), needle);
    say!(
        "\n{}",
        refusals::render_top(&format!("not-run tests containing `{needle}`"), &list, 30)
    );
}
