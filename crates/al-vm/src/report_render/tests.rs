//! The gate of the measured Html shapes: every fact by equality; a neighbor that differs in one fact keeps Part A.

use super::layout_facts::LayoutFacts;
use super::save_is_measured;
use super::shape::{
    BUILDS, BindingKind, ColumnKind, SHAPES, SaveShape, build_is_measured, content_is_measured, shape_is_measured,
};
use crate::report_save::Renders;

#[test]
fn the_save_facts_are_the_cells_save_on_a_measured_major() {
    assert!(save_is_measured(true, Renders::Default, Some(29)));
    // Another save form, target or selection; the layout a temporary selection named.
    assert!(!save_is_measured(false, Renders::Default, Some(29)));
    assert!(!save_is_measured(true, Renders::Selected, Some(29)));
    assert!(!save_is_measured(true, Renders::StoredRdlc, Some(29)));
    assert!(save_is_measured(true, Renders::Default, Some(27)));
    assert!(save_is_measured(true, Renders::Default, Some(28)));
    // A major no cell measured, or none known.
    assert!(!save_is_measured(true, Renders::Default, Some(26)));
    assert!(!save_is_measured(true, Renders::Default, None));
}

fn html(generator: &str) -> Vec<u8> {
    format!("<html><head><meta name=\"generator\" content=\"{generator}\" /></head></html>").into_bytes()
}

#[test]
fn only_the_converter_version_a_cell_measured_on_that_major_is_bcs_answer() {
    assert!(build_is_measured(BUILDS, 29, &html("Aspose.Words for .NET 26.8.0")));
    assert!(build_is_measured(BUILDS, 27, &html("Aspose.Words for .NET 26.10.0")));
    // The version of the probe's 27 and 28 builds on 29; the version of the pinned 27 build (26.8.0, not measured on 27); no
    // generator.
    assert!(!build_is_measured(BUILDS, 29, &html("Aspose.Words for .NET 26.10.0")));
    assert!(!build_is_measured(BUILDS, 27, &html("Aspose.Words for .NET 26.8.0")));
    assert!(!build_is_measured(BUILDS, 29, b"<html></html>"));
}

fn cell_shape() -> SaveShape {
    SaveShape {
        columns: vec![ColumnKind::IntegerField, ColumnKind::TextField],
        bindings: vec![BindingKind::RowSection, BindingKind::Column(ColumnKind::TextField)],
        rows: 2,
    }
}

#[test]
fn a_shape_that_differs_in_one_fact_is_not_measured() {
    assert!(shape_is_measured(SHAPES, &cell_shape()));
    for change in [
        // One or three rows (corpus pull request 1415 Bound_OneRow / Bound_ThreeRows are not asserted yet).
        |s: &mut SaveShape| s.rows = 1,
        |s: &mut SaveShape| s.rows = 3,
        // An Integer column bound, no binding at all, a second Text binding, one column less.
        |s: &mut SaveShape| s.bindings[1] = BindingKind::Column(ColumnKind::IntegerField),
        |s: &mut SaveShape| s.bindings.clear(),
        |s: &mut SaveShape| s.bindings.push(BindingKind::Column(ColumnKind::TextField)),
        |s: &mut SaveShape| s.columns.truncate(1),
    ] {
        let mut s = cell_shape();
        change(&mut s);
        assert!(!shape_is_measured(SHAPES, &s), "{s:?}");
    }
}

#[test]
fn a_part_or_element_no_measured_layout_has_is_not_measured() {
    let measured = || LayoutFacts {
        parts: ["word/document.xml", "word/settings.xml", "[Content_Types].xml"]
            .map(String::from)
            .into(),
        document: ["w:document", "w:body", "w:p", "w:r", "w:t"].map(String::from).into(),
        settings: ["w:settings"].map(String::from).into(),
        bindings: Vec::new(),
    };
    assert!(content_is_measured(&measured()));
    let mut styles = measured();
    styles.parts.insert("word/styles.xml".into());
    let mut tab = measured();
    tab.document.insert("w:tab".into());
    let mut picture = measured();
    picture.document.insert("w:drawing".into());
    let mut header = measured();
    header.parts.insert("word/header1.xml".into());
    let mut setting = measured();
    setting.settings.insert("w:defaultTabStop".into());
    for f in [styles, tab, picture, header, setting] {
        assert!(!content_is_measured(&f), "{f:?}");
    }
}
