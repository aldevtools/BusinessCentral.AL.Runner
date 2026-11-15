//! The measured hash cells of the Html of a Word layout (ADR-040 amendment 2026-11-15, decision 4): one row per report shape
//! whose Html, made by the runner's render process, has the SHA-256 BC writes on each listed major. Until a corpus bump pins
//! the probe, each row runs through the assumption `report-html-rendered-by-bc-converter`.

/// One measured render; each row's comment cites the probe test that measured it (corpus file:line).
#[derive(Debug)]
pub(crate) struct HtmlCell {
    /// The BC majors whose hash the runner's render equals.
    pub(crate) majors: &'static [u16],
    /// SHA-256 of the layout file's bytes.
    pub(crate) layout_sha256: &'static str,
    /// SHA-256 of the bound dataset part (`super::bound_part`).
    pub(crate) data_sha256: &'static str,
    /// The report caption the merge writes as the title.
    pub(crate) title: &'static str,
}

/// The two-row data part of the RX Row reports: RowId and RowTxt of rows 1 and 2 (BC 27; BC 28 and 29 write it differently).
const TWO_ROWS_27: &str = "3a3126860bb485a439f7e376c1f6982f7f1d652150931a827d22c0c059223e04";
const TWO_ROWS_28_29: &str = "63197f3063e02466c4ce60f8765423c6ab03e14d2d961cddf89f2b25617c8685";
const WORD_ALPHA: &str = "cf5f8283ccaf5498c5dc2281a3bd7843641e24da8062b72c3de9f661994c5f9e";
const WORD_RGBA: &str = "29b1be5b3ea88195d785d3f917f86274d7e1e3c6b4627340105feab2b3dd24cb";

/// The measured cells (corpus pull request 1415, reports/html-email-body/RxHtmlHashTests.al). Candidates until the runner's
/// render is shown equal to BC's hash on each listed major.
pub(crate) const CELLS: &[HtmlCell] = &[
    // Hash_WordAlpha_TwoRows (RxHtmlHashTests.al:163): `RX Word Alpha`, two rows.
    HtmlCell {
        majors: &[27],
        layout_sha256: WORD_ALPHA,
        data_sha256: TWO_ROWS_27,
        title: "RX Word Alpha",
    },
    HtmlCell {
        majors: &[28, 29],
        layout_sha256: WORD_ALPHA,
        data_sha256: TWO_ROWS_28_29,
        title: "RX Word Alpha",
    },
    // Hash_WordRgba_TwoRows (:189) and the own-layout save of Hash_TenantGuid_WordRgba_SameAsOwnLayout (:216): `RX Word
    // Rgba`, two rows.
    HtmlCell {
        majors: &[27],
        layout_sha256: WORD_RGBA,
        data_sha256: TWO_ROWS_27,
        title: "RX Word Rgba",
    },
    HtmlCell {
        majors: &[28, 29],
        layout_sha256: WORD_RGBA,
        data_sha256: TWO_ROWS_28_29,
        title: "RX Word Rgba",
    },
];
