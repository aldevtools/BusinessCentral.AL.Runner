//! The render numbers over the life of a kept serve daemon (al-runner issue 8458): every request of the daemon runs in one
//! process, so its saves share the numbers of `rendered_text`, and a marked text can outlive the request that read it (a
//! file under `--files-dir`, the kept install baseline). A test binary of its own: the numbers are process-wide.

use al_builtins::rendered_text::{self, Form, RgbaReplace};

/// One more save of the same report than a number character can name: a daemon whose requests together saved one report
/// as Html 65535 times. Each save gets a number, and its mark names the report.
#[test]
fn a_save_of_the_same_render_gets_a_number_after_0xfffe_saves() {
    for save in 0..=0xFFFEu32 {
        let n = rendered_text::register("Saves Report", "Html", "WordOne")
            .unwrap_or_else(|| panic!("save {save} got no render number"));
        assert_eq!(
            rendered_text::render(n).map(|r| r.report).as_deref(),
            Some("Saves Report")
        );
    }
}

/// A mark read in an earlier request still names its own render after a later request saved other reports more times than a
/// number character can name, and never one of the later renders.
#[test]
fn a_mark_from_an_earlier_request_names_its_own_render_after_later_saves() {
    let earlier = rendered_text::register_with("Earlier Report", "Html", "WordOne", RgbaReplace::Measured)
        .expect("a render for the earlier request");
    let stale = rendered_text::mark_text(earlier, Form::Whole);
    for save in 0..=0xFFFEu32 {
        let n = rendered_text::register("Later Report", "Html", "WordTwo")
            .unwrap_or_else(|| panic!("later save {save} got no render number"));
        assert_ne!(n, earlier, "a later render took the earlier request's number");
    }
    let marks = rendered_text::marks(&stale).expect("a whole mark");
    let named = rendered_text::render(marks[0].0).expect("the earlier render");
    assert_eq!(
        (named.report.as_str(), named.layout.as_str(), named.rgba_replace),
        ("Earlier Report", "WordOne", RgbaReplace::Measured)
    );
    assert_eq!(
        rendered_text::describe(&stale),
        "<rendered content of report Earlier Report>"
    );
}
