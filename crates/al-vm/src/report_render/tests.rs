//! The gate of the measured Html shapes: every fact by equality; a neighbor that differs in one fact keeps Part A.

use super::call::{MEASURED_CALL, SaveCall, Stream};
use super::layout_facts::{LayoutFacts, Relationship};
use super::save_is_measured;
use super::shape::{
    BUILDS, ColumnKind, SHAPES, SaveShape, build_is_measured, content_is_measured, converter_is_measured,
    shape_is_measured,
};
use crate::report_save::Renders;

#[test]
fn the_save_facts_are_the_cells_save_on_a_measured_major() {
    let m = MEASURED_CALL;
    assert!(save_is_measured(true, m, Renders::Default, Some(29)));
    // Another fact vector after the run; the layout a selection named.
    assert!(!save_is_measured(false, m, Renders::Default, Some(29)));
    assert!(!save_is_measured(true, m, Renders::Selected, Some(29)));
    assert!(!save_is_measured(true, m, Renders::StoredRdlc, Some(29)));
    assert!(save_is_measured(true, m, Renders::Default, Some(27)));
    assert!(save_is_measured(true, m, Renders::Default, Some(28)));
    // A major no cell measured, or none known.
    assert!(!save_is_measured(true, m, Renders::Default, Some(26)));
    assert!(!save_is_measured(true, m, Renders::Default, None));
}

#[test]
fn a_call_that_differs_from_the_cells_call_in_one_fact_is_not_measured() {
    let changes: [fn(&mut SaveCall); 5] = [
        |c| c.by_number = false,
        |c| c.parameters = true,
        |c| c.record_ref = true,
        |c| c.stream = Stream::Other,
        |c| c.language = 1031,
    ];
    for change in changes {
        let mut c = MEASURED_CALL;
        change(&mut c);
        assert!(!save_is_measured(true, c, Renders::Default, Some(29)), "{c:?}");
    }
}

#[test]
fn only_the_converter_assembly_a_cell_measured_on_that_major_is_rendered() {
    assert!(converter_is_measured(BUILDS, 29, [26, 8, 0, 0]));
    assert!(converter_is_measured(BUILDS, 27, [26, 10, 0, 0]));
    // The pinned 27 and 28 builds carry 26.8.0.0; another revision; 26.10.0 on 29.
    assert!(!converter_is_measured(BUILDS, 27, [26, 8, 0, 0]));
    assert!(!converter_is_measured(BUILDS, 28, [26, 8, 0, 0]));
    assert!(!converter_is_measured(BUILDS, 29, [26, 8, 0, 1]));
    assert!(!converter_is_measured(BUILDS, 29, [26, 10, 0, 0]));
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
        rows: 2,
    }
}

#[test]
fn a_shape_that_differs_in_one_fact_is_not_measured() {
    assert!(shape_is_measured(SHAPES, &cell_shape()));
    for change in [
        // One or three rows: measured only for the layout that binds four columns.
        |s: &mut SaveShape| s.rows = 1,
        |s: &mut SaveShape| s.rows = 3,
        // One column less, a second Text column.
        |s: &mut SaveShape| s.columns.truncate(1),
        |s: &mut SaveShape| s.columns.push(ColumnKind::TextField),
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
        namespaces: [(
            "w".to_string(),
            "http://schemas.openxmlformats.org/wordprocessingml/2006/main".to_string(),
        )]
        .into(),
        relationships: vec![Relationship {
            kind: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/settings".into(),
            external: false,
        }],
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
    let mut external = measured();
    external.relationships[0].external = true;
    let mut image = measured();
    image.relationships[0].kind = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image".into();
    let mut field = measured();
    field.document.insert("w:fldSimple".into());
    let mut footer = measured();
    footer.document.insert("w:footerReference".into());
    // L6: the prefix `w` bound to another namespace, a default namespace.
    let mut rebound = measured();
    rebound.namespaces = [("w".to_string(), "urn:other".to_string())].into();
    let mut default = measured();
    default.namespaces.insert((String::new(), "urn:d".to_string()));
    for f in [
        styles, tab, picture, header, setting, external, image, field, footer, rebound, default,
    ] {
        assert!(!content_is_measured(&f), "{f:?}");
    }
}

fn bound_shape(rows: usize) -> SaveShape {
    use ColumnKind::*;
    SaveShape {
        columns: vec![IntegerField, TextField, DecimalExpression, DateExpression],
        rows,
    }
}

#[test]
fn the_four_columns_are_measured_with_one_and_three_rows_only() {
    assert!(shape_is_measured(SHAPES, &bound_shape(1)));
    assert!(shape_is_measured(SHAPES, &bound_shape(3)));
    for change in [
        // Two rows: the cells of this report have one and three.
        |s: &mut SaveShape| s.rows = 2,
        // A column less, a second Date column.
        |s: &mut SaveShape| {
            s.columns.pop();
        },
        |s: &mut SaveShape| s.columns.push(ColumnKind::DateExpression),
    ] {
        let mut s = bound_shape(1);
        change(&mut s);
        assert!(!shape_is_measured(SHAPES, &s), "{s:?}");
    }
}
