// The scans of production code (readable_messages, no_version_literals) stop at the first `#[cfg(test)]` of a file,
// so a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;
use crate::value::Value;

/// A key for these tests only, made with the hidden constructor (the registry macro is the only other way).
const TEST_KEY: RefusalKey = RefusalKey::new(&KeyInfo {
    name: "TEST_KEY",
    key: "xml.test.only-key",
    probe: Probe::Wanted,
});

#[test]
fn a_free_text_refusal_has_the_key_legacy_and_reads_as_before() {
    let text = "a sibling operation on an attribute or declaration";
    for r in [Refusal::from(text), Refusal::from(text.to_string())] {
        assert_eq!(r.key(), LEGACY);
        assert!(r.is_legacy());
        assert_eq!(r.reason(), text);
        assert_eq!(r.to_string(), text);
        assert_eq!(format!("{r:?}"), format!("{text:?}"));
        assert!(r.contains("sibling") && r.starts_with("a "), "Deref to str");
        let owned = String::from(text);
        assert!(r == text && r == *text && r == owned);
        assert_eq!(r.clone().into_reason(), text);
    }
    // A quote prints as the Debug of the String did.
    assert_eq!(format!("{:?}", Refusal::from("a \"b\"")), "\"a \\\"b\\\"\"");
}

#[test]
fn refuse_builds_the_reason_from_described_arguments() {
    let v = Value::Integer(5);
    let r = crate::refuse!(TEST_KEY, "assigning {} to {}", v, "a Code[10]");
    assert_eq!(r.reason(), "assigning Integer 5 to a Code[10]");
    assert_eq!(r.key(), "xml.test.only-key");
    assert!(!r.is_legacy());
    // Width and positions are kept; `{{` is an escape.
    let r = crate::refuse!(TEST_KEY, "[{1:>4}|{0}] {{x}}", 7, 12);
    assert_eq!(r.reason(), "[  12|7] {x}");
}

#[test]
fn a_keyed_refusal_keeps_its_key_through_clone_and_map_reason() {
    let r = Refusal::keyed(TEST_KEY, "why".into());
    let c = r.clone();
    assert_eq!(c, r);
    let m = c.map_reason(|t| format!("OnRun: {t}"));
    assert_eq!((m.key(), m.reason()), ("xml.test.only-key", "OnRun: why"));
    assert_eq!(format!("{m:?}"), "xml.test.only-key: \"OnRun: why\"");
    assert_eq!(m.to_string(), "OnRun: why");
    // Same text, different key: not equal.
    assert_ne!(Refusal::from("why"), r);
    let legacy = Refusal::from("why").map_reason(|t| t + "!");
    assert_eq!((legacy.key(), legacy.reason()), (LEGACY, "why!"));
}

#[test]
fn the_area_is_the_key_up_to_the_first_dot() {
    assert_eq!(Refusal::keyed(TEST_KEY, String::new()).area(), "xml");
    assert_eq!(Refusal::from("x").area(), LEGACY);
}

#[test]
fn positional_only_rejects_a_named_placeholder() {
    for ok in [
        "",
        "plain",
        "{}",
        "{} and {}",
        "{0} {1:>5}",
        "{:?}",
        "{{name}}",
        "a {{ b }} c {}",
    ] {
        assert!(positional_only(ok), "{ok}");
    }
    for bad in ["{name}", "{x:?}", "{} {why}", "{{{name}}}", "{_x}"] {
        assert!(!positional_only(bad), "{bad}");
    }
}

#[test]
fn a_legacy_refusal_dumps_as_a_plain_string() {
    let r = Refusal::from("not pinned: \"x\"");
    let json = serde_json::to_string(&r).unwrap();
    assert_eq!(json, serde_json::to_string("not pinned: \"x\"").unwrap());
    let back: Refusal = serde_json::from_str(&json).unwrap();
    assert_eq!(back, r);
    assert!(back.is_legacy());
    let keyed = Refusal::keyed(TEST_KEY, "why".into());
    let json = serde_json::to_string(&keyed).unwrap();
    assert_eq!(json, r#"{"key":"xml.test.only-key","reason":"why"}"#);
    let back: Refusal = serde_json::from_str(&json).unwrap();
    assert_eq!((back.key(), back.reason()), ("xml.test.only-key", "why"));
}

#[test]
fn a_refusal_is_one_pointer() {
    assert_eq!(std::mem::size_of::<Refusal>(), std::mem::size_of::<usize>());
}
