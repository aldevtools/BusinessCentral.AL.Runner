ForCharLiteralBounds => {
    id: "for-char-literal-bounds",
    assumed: "A `for` loop over a local Char variable whose bounds are, to or downto, two one-character literals, a one-character literal and a local Char variable (either order), or the integer 52 after a one-character literal or the integer 48 before one (`to` only, not `downto`), all literals within the Basic Multilingual Plane, reads each one-character literal as that Char (code point 0 to 65535, 'ä' is 228): the loop runs the characters between the bounds, an empty range runs zero times and leaves the variable at the start value, `'a' to 'a'` runs once, and the variable keeps its last value after the loop; and a `for` loop over a local Integer variable with two one-character literals as bounds iterates their codes (48 to 52 for '0' to '4'). A multi-character, empty or non-BMP literal bound, any other integer literal bound, value or position (including 0, 1, 65534 and 65535, which the probe measured only between integers), a Text loop variable and every other loop variable type, bound kind (an expression, a global or a field, a Text or Integer variable next to a literal) or pair keep the refusal.",
    tests: "No Microsoft test. al-runner issue 7872 (the reporter's repro: `for c := '0' to '4'` over a Char variable).",
    probe: "Open corpus pull request 1525, tests/al-language/language/ForCharBounds.al (13 tests, plus must-not-compile 3801 to 3803), green on all three pinned majors of a local Business Central with no version difference (corpus CI not run yet).",
    awaiting: Awaiting {
        issues: &[7872],
        files: &["tests/al-language/language/ForCharBounds.al"],
        since: "2026-11-22",
    },
}
