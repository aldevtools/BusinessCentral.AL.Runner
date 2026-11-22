//! A refusal that names a placeholder Html does not name its render number. The number is process-wide (ADR-040 amendment
//! 2026-11-12, A1), so it depends on which tests ran before in the same process: the same test would refuse with a different
//! reason in an eager-compile run and in a demand-driven run (the nightly comparison of the two).

use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::{placeholder_output, rendered_text};

fn text(s: Signal) -> String {
    match s {
        Signal::Unsupported(why) => why.to_string(),
        other => panic!("not a refusal: {other:?}"),
    }
}

#[test]
fn a_content_read_refusal_is_the_same_for_every_render_number() {
    let one = rendered_text::html_mark("RX Word Default", Some(1));
    let other = rendered_text::html_mark("RX Word Default", Some(56));
    let none = rendered_text::html_mark("RX Word Default", None);
    let want = text(placeholder_output::refusal(&none));
    assert_eq!(text(placeholder_output::refusal(&one)), want);
    assert_eq!(text(placeholder_output::refusal(&other)), want);
    assert!(
        want.contains("a read of the content of a placeholder Html of report RX Word Default ("),
        "{want}"
    );
}

#[test]
fn a_size_refusal_and_a_description_are_the_same_for_every_render_number() {
    let one = rendered_text::html_mark("RX Word Default", Some(3));
    let none = rendered_text::html_mark("RX Word Default", None);
    assert_eq!(
        placeholder_output::size_refusal_text(&one, "Format"),
        placeholder_output::size_refusal_text(&none, "Format")
    );
    assert_eq!(
        placeholder_output::size(&one).describe(),
        placeholder_output::size(&none).describe()
    );
}
