//! The oracle: the JUnit report the corpus CI produced against real BC.
//!
//! Every `<testcase>` in it passed on real BC, so it is the denominator of the
//! pass rate. A case is identified by its codeunit id and procedure name.

pub use al_testrun::CaseId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleCase {
    pub id: CaseId,
    /// True when real BC reported this case as failed, errored or skipped.
    pub bc_failed: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OracleError {
    Xml(String),
    BadCodeunit(String),
    /// The document is the other format: its root element (`assemblies` for XUnit, `testsuites` for JUnit).
    WrongFormat(&'static str),
}

impl std::fmt::Display for OracleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OracleError::Xml(why) => write!(f, "not valid XML: {why}"),
            OracleError::BadCodeunit(what) => write!(f, "bad codeunit in the oracle: {what}"),
            // ADR-074: the format names the test driver the oracle was recorded with, so a missing or wrong `format` of a
            // suite is an error, not a run under the other driver.
            OracleError::WrongFormat(root) => write!(
                f,
                "the oracle is {} (root element <{root}>): its suite in oracle.toml needs format = \"{}\"",
                if *root == "assemblies" { "XUnit" } else { "JUnit" },
                if *root == "assemblies" { "xunit" } else { "junit" }
            ),
        }
    }
}

/// The BC 28 and 29 corpus legs run through a hybrid test runner that reports one
/// pseudo-case with this name per test codeunit. It is not a test procedure: no
/// corpus codeunit declares one, so it is skipped by rule (ADR-015 1.6).
pub const RUNNER_PSEUDO_CASE: &str = "OnRun";

pub fn parse_junit(xml: &str) -> Result<Vec<OracleCase>, OracleError> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(xml);
    let mut cases = Vec::new();
    // The case whose element is still open, waiting for a failure child or its end tag.
    let mut open: Option<OracleCase> = None;
    loop {
        let ev = reader.read_event().map_err(|e| OracleError::Xml(e.to_string()))?;
        match ev {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"assemblies" => {
                return Err(OracleError::WrongFormat("assemblies"));
            }
            Event::Empty(e) if e.name().as_ref() == b"testcase" => cases.push(testcase(&e)?),
            Event::Start(e) if e.name().as_ref() == b"testcase" => open = Some(testcase(&e)?),
            Event::Start(e) | Event::Empty(e) if matches!(e.name().as_ref(), b"failure" | b"error" | b"skipped") => {
                if let Some(c) = open.as_mut() {
                    c.bc_failed = true;
                }
            }
            Event::End(e) if e.name().as_ref() == b"testcase" => {
                cases.extend(open.take());
            }
            Event::Eof => {
                if open.is_some() {
                    return Err(OracleError::Xml("unterminated <testcase>".into()));
                }
                cases.retain(|c| c.id.name != RUNNER_PSEUDO_CASE);
                return Ok(cases);
            }
            _ => {}
        }
    }
}

/// When real BC ran each test codeunit: the `timestamp` of its `<testsuite name="Codeunit N">` in a JUnit oracle, as Unix
/// milliseconds (UTC; the container BC ran in used UTC, as the oracle's days show: values derived from `Today` on the
/// CI day, which a rerun on another day cannot reproduce). A suite without a readable timestamp is left out.
pub fn parse_clocks(xml: &str) -> Result<std::collections::HashMap<u32, i64>, OracleError> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(xml);
    let mut out = std::collections::HashMap::new();
    loop {
        match reader.read_event().map_err(|e| OracleError::Xml(e.to_string()))? {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"testsuite" => {
                let id =
                    attr(&e, b"name")?.and_then(|n| n.strip_prefix("Codeunit ").and_then(|n| n.trim().parse().ok()));
                let at = attr(&e, b"timestamp")?.and_then(|t| unix_millis(&t));
                if let (Some(id), Some(at)) = (id, at) {
                    out.insert(id, at);
                }
            }
            Event::Eof => return Ok(out),
            _ => {}
        }
    }
}

/// `2026-10-15T22:00:23Z` (UTC, optional fraction of a second) as Unix milliseconds.
pub fn unix_millis(text: &str) -> Option<i64> {
    let t = text.trim().strip_suffix('Z')?;
    let (date, time) = t.split_once('T')?;
    let mut d = date.split('-');
    let (y, m, day): (i64, i64, i64) = (
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
    );
    let (hms, frac) = match time.split_once('.') {
        Some((h, f)) => (h, f),
        None => (time, ""),
    };
    let mut p = hms.split(':');
    let (h, mi, sec): (i64, i64, i64) = (
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
        p.next()?.parse().ok()?,
    );
    if d.next().is_some() || p.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&day) {
        return None;
    }
    let ms: i64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<3}")[..3].parse().ok()?
    };
    // Days since 1970-01-01 (Howard Hinnant's days_from_civil).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + h) * 60 + mi) * 60_000 + sec * 1000 + ms)
}

/// One `<test>` of an XUnit report, with the failure message real BC gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XunitCase {
    pub case: OracleCase,
    /// The text of the `<failure><message>`, when the test did not pass.
    pub message: Option<String>,
}

/// The XUnit report of BcContainerHelper's `Run-TestsInBcContainer`, which is what the Windows
/// nightly writes (the isolation probe runs under a custom test runner there, never in the Linux
/// legs). One `<assembly name="<codeunit id> <name>">` per test codeunit, a `<test method=..
/// result=..>` per method; anything but `Pass` is a BC failure.
pub fn parse_xunit(xml: &str) -> Result<Vec<XunitCase>, OracleError> {
    use quick_xml::events::Event;

    let xml = xml.trim_start_matches('\u{feff}');
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut cases: Vec<XunitCase> = Vec::new();
    let mut codeunit: Option<u32> = None;
    // Inside a `<test>` that has a body: collect the message of its `<failure>`.
    let mut open = false;
    let mut in_message = false;
    loop {
        match reader.read_event().map_err(|e| OracleError::Xml(e.to_string()))? {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"testsuites" => {
                return Err(OracleError::WrongFormat("testsuites"));
            }
            Event::Start(e) if e.name().as_ref() == b"assembly" => {
                let name = attr(&e, b"name")?.unwrap_or_default();
                codeunit = Some(
                    name.split_whitespace()
                        .next()
                        .and_then(|n| n.parse().ok())
                        .ok_or_else(|| OracleError::BadCodeunit(name.clone()))?,
                );
            }
            Event::End(e) if e.name().as_ref() == b"assembly" => codeunit = None,
            Event::Start(e) if e.name().as_ref() == b"test" => {
                cases.push(xunit_test(codeunit, &e)?);
                open = true;
            }
            Event::Empty(e) if e.name().as_ref() == b"test" => cases.push(xunit_test(codeunit, &e)?),
            Event::End(e) if e.name().as_ref() == b"test" => open = false,
            Event::Start(e) if open && e.name().as_ref() == b"message" => in_message = true,
            Event::End(e) if e.name().as_ref() == b"message" => in_message = false,
            Event::Text(t) if in_message => {
                if let Some(c) = cases.last_mut() {
                    let text = t.unescape().map_err(|e| OracleError::Xml(e.to_string()))?;
                    c.message.get_or_insert_with(String::new).push_str(&text);
                }
            }
            Event::Eof => return Ok(cases),
            _ => {}
        }
    }
}

fn xunit_test(codeunit: Option<u32>, e: &quick_xml::events::BytesStart) -> Result<XunitCase, OracleError> {
    let codeunit = codeunit.ok_or_else(|| OracleError::BadCodeunit("test outside an assembly".into()))?;
    let name = attr(e, b"method")?.ok_or_else(|| OracleError::Xml("test without method".into()))?;
    let result = attr(e, b"result")?.unwrap_or_default();
    Ok(XunitCase {
        case: OracleCase {
            id: CaseId { codeunit, name },
            bc_failed: !result.eq_ignore_ascii_case("pass"),
        },
        message: None,
    })
}

fn attr(e: &quick_xml::events::BytesStart, key: &[u8]) -> Result<Option<String>, OracleError> {
    for a in e.attributes() {
        let a = a.map_err(|e| OracleError::Xml(e.to_string()))?;
        if a.key.as_ref() == key {
            return Ok(Some(
                a.unescape_value()
                    .map_err(|e| OracleError::Xml(e.to_string()))?
                    .into_owned(),
            ));
        }
    }
    Ok(None)
}

fn testcase(e: &quick_xml::events::BytesStart) -> Result<OracleCase, OracleError> {
    let mut classname = None;
    let mut name = None;
    for a in e.attributes() {
        let a = a.map_err(|e| OracleError::Xml(e.to_string()))?;
        let v = a
            .unescape_value()
            .map_err(|e| OracleError::Xml(e.to_string()))?
            .into_owned();
        match a.key.as_ref() {
            b"classname" => classname = Some(v),
            b"name" => name = Some(v),
            _ => {}
        }
    }
    let classname = classname.ok_or_else(|| OracleError::Xml("testcase without classname".into()))?;
    let name = name.ok_or_else(|| OracleError::Xml("testcase without name".into()))?;
    let codeunit = classname
        .strip_prefix("Codeunit ")
        .and_then(|n| n.trim().parse().ok())
        .ok_or_else(|| OracleError::BadCodeunit(classname.clone()))?;
    Ok(OracleCase {
        id: CaseId { codeunit, name },
        bc_failed: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version='1.0' encoding='utf-8'?>
<testsuites name="DEFAULT" tests="3" failures="1">
  <testsuite name="Codeunit 60002" tests="2">
    <testcase classname="Codeunit 60002" name="A_Works" time="0.004" />
    <testcase classname="Codeunit 60002" name="B_Fails" time="0.003"><failure message="x">boom</failure></testcase>
  </testsuite>
  <testsuite name="Codeunit 60006" tests="1">
    <testcase classname="Codeunit 60006" name="C_Works" time="0.040" />
  </testsuite>
</testsuites>"#;

    /// ADR-074: an oracle read in the other format is an error that names the `format` to set (a suite without `format`
    /// is read as JUnit and run under the bc-linux driver).
    #[test]
    fn an_oracle_in_the_other_format_is_an_error() {
        let xunit = r#"<assemblies><assembly name="61302 Isol Probe Fixture"><collection name="X"><test name="T" result="Pass" /></collection></assembly></assemblies>"#;
        let e = parse_junit(xunit).unwrap_err().to_string();
        assert!(e.contains("format = \"xunit\""), "{e}");
        let e = parse_xunit(SAMPLE).unwrap_err().to_string();
        assert!(e.contains("format = \"junit\""), "{e}");
    }

    fn id(cu: u32, n: &str) -> CaseId {
        CaseId {
            codeunit: cu,
            name: n.into(),
        }
    }

    #[test]
    fn the_clock_of_a_codeunit_is_the_timestamp_of_its_suite() {
        let xml = "<testsuites><testsuite name=\"Codeunit 60002\" tests=\"1\" timestamp=\"2026-10-15T22:00:23Z\"><testcase classname=\"Codeunit 60002\" name=\"A\" /></testsuite><testsuite name=\"Codeunit 7\" timestamp=\"bad\" /><testsuite name=\"Other\" timestamp=\"2026-10-15T22:00:23Z\" /></testsuites>";
        let clocks = parse_clocks(xml).unwrap();
        assert_eq!(clocks.len(), 1, "{clocks:?}");
        assert_eq!(clocks[&60002], 1_792_101_623_000);
        assert_eq!(unix_millis("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(unix_millis("2024-02-29T23:59:59.5Z"), Some(1_709_251_199_500));
        assert_eq!(unix_millis("2026-10-15T22:00:23"), None);
    }

    #[test]
    fn parses_every_testcase_with_its_codeunit() {
        let cases = parse_junit(SAMPLE).unwrap();
        let ids: Vec<_> = cases.iter().map(|c| c.id.clone()).collect();
        assert_eq!(
            ids,
            vec![id(60002, "A_Works"), id(60002, "B_Fails"), id(60006, "C_Works")]
        );
    }

    #[test]
    fn marks_a_case_with_a_failure_child_as_bc_failed() {
        let cases = parse_junit(SAMPLE).unwrap();
        let failed: Vec<_> = cases
            .iter()
            .filter(|c| c.bc_failed)
            .map(|c| c.id.name.as_str())
            .collect();
        assert_eq!(failed, vec!["B_Fails"]);
    }

    #[test]
    fn skips_the_runner_onrun_pseudo_case() {
        // The BC 28 and 29 legs run through the hybrid test runner, which reports one `OnRun`
        // case per test codeunit. It is not a test procedure (ADR-015 1.6).
        let xml = r#"<testsuites name="hybrid"><testsuite name="Codeunit 60002">
            <testcase classname="Codeunit 60002" name="OnRun" time="0.010" />
            <testcase classname="Codeunit 60002" name="A_Works" time="0.004" />
        </testsuite></testsuites>"#;
        let ids: Vec<_> = parse_junit(xml).unwrap().into_iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![id(60002, "A_Works")]);
        // A failed OnRun is still not a case; only the exact name is skipped.
        let xml = r#"<testsuites><testsuite name="x">
            <testcase classname="Codeunit 1" name="OnRun"><failure message="x"/></testcase>
            <testcase classname="Codeunit 1" name="OnRunSomething"/>
        </testsuite></testsuites>"#;
        let ids: Vec<_> = parse_junit(xml).unwrap().into_iter().map(|c| c.id).collect();
        assert_eq!(ids, vec![id(1, "OnRunSomething")]);
    }

    // oracle/28/xunit-isolation-*.xml: the Windows nightly's XUnit shape (BcContainerHelper's
    // Run-TestsInBcContainer), with a byte-order mark, one `assembly` per test codeunit named
    // `<id> <name>`, and a `test` per method.
    const XUNIT: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<assemblies>
  <assembly name=\"61302 Isol Probe Fixture\" total=\"2\">
    <collection name=\"Isol Probe Fixture\" total=\"2\">
      <test name=\"Isol Probe Fixture:T1_A\" method=\"T1_A\" time=\"0.02\" result=\"Pass\" />
      <test name=\"Isol Probe Fixture:T5_B\" method=\"T5_B\" time=\"0.007\" result=\"Fail\">
        <failure><message>the row is still there</message></failure>
      </test>
    </collection>
  </assembly>
</assemblies>";

    #[test]
    fn parses_the_nightlys_xunit_with_its_failure_messages() {
        let cases = parse_xunit(XUNIT).unwrap();
        let got: Vec<_> = cases.iter().map(|c| (c.case.id.clone(), c.case.bc_failed)).collect();
        assert_eq!(got, vec![(id(61302, "T1_A"), false), (id(61302, "T5_B"), true)]);
        assert_eq!(cases[0].message, None);
        assert_eq!(cases[1].message.as_deref(), Some("the row is still there"));
    }

    #[test]
    fn an_xunit_result_other_than_pass_is_a_bc_failure() {
        let xml = r#"<assemblies><assembly name="1 X"><collection name="X">
            <test name="X:A" method="A" result="Skip"/>
            <test name="X:B" method="B" result="Pass"/></collection></assembly></assemblies>"#;
        let failed: Vec<_> = parse_xunit(xml)
            .unwrap()
            .into_iter()
            .map(|c| c.case.bc_failed)
            .collect();
        assert_eq!(failed, vec![true, false]);
    }

    #[test]
    fn an_xunit_assembly_without_a_codeunit_id_is_rejected() {
        let xml = r#"<assemblies><assembly name="Isol Probe Fixture"><collection name="X">
            <test name="X:A" method="A" result="Pass"/></collection></assembly></assemblies>"#;
        assert!(matches!(parse_xunit(xml), Err(OracleError::BadCodeunit(_))));
        assert!(matches!(parse_xunit("<assemblies><test"), Err(OracleError::Xml(_))));
    }

    #[test]
    fn rejects_a_classname_that_is_not_a_codeunit() {
        let xml = r#"<testsuites><testsuite name="x"><testcase classname="Page 1" name="n"/></testsuite></testsuites>"#;
        assert!(matches!(parse_junit(xml), Err(OracleError::BadCodeunit(_))));
    }

    #[test]
    fn rejects_malformed_xml() {
        assert!(matches!(parse_junit("<testsuites><testcase"), Err(OracleError::Xml(_))));
    }
}
