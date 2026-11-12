//! `Txt::push_str` keeps the cached marker answer for every mark (ADR-040 amendment 2026-11-12, A1): a text known to hold no
//! marker that gets a marked part appended must answer `has_marker` true, or every guard that asks it would let the text pass.

use al_builtins::native::TEXT_MARKS;
use al_builtins::text::Txt;

#[test]
fn appending_a_marked_part_to_a_known_unmarked_text_is_marked_for_every_mark() {
    for m in TEXT_MARKS {
        let mut t: Txt = "Signature ".to_string().into();
        assert!(!t.has_marker(), "the start holds no mark");
        t.push_str(&format!("x{m}"));
        assert!(t.has_marker(), "{m:?} appended by push_str");
        let mut u: Txt = "Signature ".to_string().into();
        assert!(!u.has_marker());
        u.push(m);
        assert!(u.has_marker(), "{m:?} appended by push");
    }
}
