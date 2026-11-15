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
        elements: [
            ("word/document.xml", &["w:document", "w:body", "w:p", "w:r", "w:t"][..]),
            ("word/settings.xml", &["w:settings"][..]),
        ]
        .map(|(p, e)| (p.to_string(), e.iter().map(|e| e.to_string()).collect()))
        .into(),
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
    let doc = |f: &mut LayoutFacts, e: &str| {
        f.elements.get_mut("word/document.xml").unwrap().insert(e.into());
    };
    // The font table part (styles and header parts are measured since corpus pull request 1419).
    let mut styles = measured();
    styles.parts.insert("word/fontTable.xml".into());
    let mut tab = measured();
    doc(&mut tab, "w:tab");
    let mut picture = measured();
    doc(&mut picture, "w:drawing");
    // A footer part (a header part `word/header1.xml` is measured since corpus pull request 1419).
    let mut header = measured();
    header.parts.insert("word/footer1.xml".into());
    let mut setting = measured();
    setting
        .elements
        .get_mut("word/settings.xml")
        .unwrap()
        .insert("w:defaultTabStop".into());
    let mut external = measured();
    external.relationships[0].external = true;
    let mut image = measured();
    image.relationships[0].kind = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image".into();
    let mut field = measured();
    doc(&mut field, "w:fldSimple");
    let mut footer = measured();
    doc(&mut footer, "w:footerReference");
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
fn the_four_columns_are_measured_with_one_to_three_rows_only() {
    assert!(shape_is_measured(SHAPES, &bound_shape(1)));
    assert!(shape_is_measured(SHAPES, &bound_shape(3)));
    // Two rows: every cell of RxHtmlWidenTests.al but BlankZero_IntegerZero (corpus pull request 1419).
    assert!(shape_is_measured(SHAPES, &bound_shape(2)));
    for change in [
        // Four rows: the cells of this dataset have one, two and three.
        |s: &mut SaveShape| s.rows = 4,
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

/// The facts of the bound layout plus the part `part` with element names `names`, its relationship `rel`, the document
/// elements `doc` and the namespace declarations `ns`, as RxWdStyles.docx, RxWdTheme.docx, RxWdNumbering.docx and
/// RxWdHeader.docx add them to RxHashBound.docx (corpus pull request 1419, RxHtmlWidenTests.al).
fn with_part(part: &str, names: &[&str], rel: &str, doc: &[&str], ns: (&str, &str)) -> LayoutFacts {
    let mut f = LayoutFacts {
        parts: [
            "[Content_Types].xml",
            "_rels/.rels",
            "word/document.xml",
            "word/settings.xml",
        ]
        .map(String::from)
        .into(),
        elements: [
            ("word/document.xml", &["w:document", "w:body", "w:p", "w:r", "w:t"][..]),
            ("word/settings.xml", &["w:settings"][..]),
        ]
        .map(|(p, e)| (p.to_string(), e.iter().map(|e| e.to_string()).collect()))
        .into(),
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
    f.parts.insert(part.into());
    f.elements
        .insert(part.into(), names.iter().map(|e| e.to_string()).collect());
    f.relationships.push(Relationship {
        kind: format!("http://schemas.openxmlformats.org/officeDocument/2006/relationships/{rel}"),
        external: false,
    });
    f.elements
        .get_mut("word/document.xml")
        .unwrap()
        .extend(doc.iter().map(|e| e.to_string()));
    f.namespaces.insert((ns.0.into(), ns.1.into()));
    f
}

const W: (&str, &str) = ("w", "http://schemas.openxmlformats.org/wordprocessingml/2006/main");
const A: (&str, &str) = ("a", "http://schemas.openxmlformats.org/drawingml/2006/main");

fn styles() -> LayoutFacts {
    let names = [
        "w:styles",
        "w:docDefaults",
        "w:rPrDefault",
        "w:rPr",
        "w:sz",
        "w:style",
        "w:name",
        "w:basedOn",
    ];
    let more = ["w:qFormat", "w:rFonts", "w:b"];
    let all: Vec<&str> = names.into_iter().chain(more).collect();
    with_part("word/styles.xml", &all, "styles", &["w:pPr", "w:pStyle"], W)
}

fn theme() -> LayoutFacts {
    let names = [
        "a:theme",
        "a:themeElements",
        "a:clrScheme",
        "a:dk1",
        "a:sysClr",
        "a:srgbClr",
        "a:fontScheme",
        "a:majorFont",
        "a:minorFont",
        "a:latin",
        "a:ea",
        "a:cs",
        "a:fmtScheme",
        "a:fillStyleLst",
        "a:solidFill",
        "a:schemeClr",
    ];
    with_part("word/theme/theme1.xml", &names, "theme", &["w:rPr", "w:rFonts"], A)
}

fn numbering() -> LayoutFacts {
    let names = [
        "w:numbering",
        "w:abstractNum",
        "w:multiLevelType",
        "w:lvl",
        "w:start",
        "w:numFmt",
        "w:lvlText",
        "w:lvlJc",
        "w:pPr",
        "w:ind",
        "w:num",
        "w:abstractNumId",
    ];
    with_part(
        "word/numbering.xml",
        &names,
        "numbering",
        &["w:pPr", "w:numPr", "w:ilvl", "w:numId"],
        W,
    )
}

fn header() -> LayoutFacts {
    let names = ["w:hdr", "w:p", "w:r", "w:t"];
    with_part(
        "word/header1.xml",
        &names,
        "header",
        &["w:sectPr", "w:headerReference"],
        W,
    )
}

#[test]
fn each_part_corpus_pull_request_1419_adds_but_numbering_is_measured_with_its_elements() {
    for f in [styles(), theme(), header()] {
        assert!(content_is_measured(&f), "{f:?}");
    }
}

/// The converter pads a list label to the indent by the label's width in its font, measured with the fonts of the host
/// (Numbering_BulletList_NoTabs, RxHtmlWidenTests.al:524, gives other bytes on a host without a Times New Roman metric
/// font): a numbering part, and a paragraph that names a numbering, keep Part A.
#[test]
fn a_numbering_part_or_a_numbered_paragraph_keeps_part_a() {
    assert!(!content_is_measured(&numbering()));
    let mut numbered = styles();
    numbered
        .elements
        .get_mut("word/document.xml")
        .unwrap()
        .extend(["w:numPr", "w:ilvl", "w:numId"].map(String::from));
    assert!(!content_is_measured(&numbered));
    let mut related = styles();
    related.relationships.push(Relationship {
        kind: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering".into(),
        external: false,
    });
    assert!(!content_is_measured(&related));
}

#[test]
fn a_tab_a_footer_or_an_element_the_measured_part_does_not_have_keeps_part_a() {
    let part = |mut f: LayoutFacts, name: &str, e: &str| {
        f.elements.get_mut(name).unwrap().insert(e.into());
        f
    };
    let neighbors = [
        // Tabs: a tab stop of a style, a tab character in the header and in the document.
        part(styles(), "word/styles.xml", "w:tabs"),
        part(header(), "word/header1.xml", "w:tab"),
        part(styles(), "word/document.xml", "w:tab"),
        // A picture or a content control in the header; an element of another kind of theme content.
        part(header(), "word/header1.xml", "w:drawing"),
        part(header(), "word/header1.xml", "w:sdt"),
        part(theme(), "word/theme/theme1.xml", "a:extLst"),
        // A footer reference beside the header.
        part(header(), "word/document.xml", "w:footerReference"),
    ];
    for f in neighbors {
        assert!(!content_is_measured(&f), "{f:?}");
    }
    // A second header part, a footer part with its relationship, the measured elements under another part name.
    let mut second = header();
    second.parts.insert("word/header2.xml".into());
    let mut footer = header();
    footer.relationships[1].kind = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer".into();
    let mut renamed = header();
    let names = renamed.elements.remove("word/header1.xml").unwrap();
    renamed.elements.insert("word/footer1.xml".into(), names);
    // The theme's prefix bound to another namespace.
    let mut rebound = theme();
    rebound.namespaces.remove(&(A.0.to_string(), A.1.to_string()));
    rebound.namespaces.insert(("a".into(), "urn:other".into()));
    for f in [second, footer, renamed, rebound] {
        assert!(!content_is_measured(&f), "{f:?}");
    }
}

const ZERO_ROW: &str = "table 50100 \"Z Row\" { fields { field(1; Id; Integer) { } field(2; Txt; Text[50]) { } \
    field(3; Qty; Integer) { } field(4; QtyField; Integer) { BlankZero = true; } field(5; QtyNot; Integer) { BlankZero = false; } \
    field(6; QtyCap; Integer) { Caption = 'Q'; } field(7; QtyTwo; Integer) { BlankZero = true; BlankNumbers = BlankNeg; } \
    field(8; TxtZero; Text[50]) { BlankZero = true; } } keys { key(PK; Id) { Clustered = true; } } }";

fn report_src(props: &str) -> String {
    format!(
        "{ZERO_ROW} report 50101 \"Z Rep\" {{ {props} UsageCategory = None; UseRequestPage = false; DefaultRenderingLayout = L; \
         dataset {{ dataitem(Row; \"Z Row\") {{ column(RowTxt; Txt) {{ }} column(Qty; Qty) {{ }} column(QtyField; QtyField) {{ }} \
         column(QtyNot; QtyNot) {{ }} column(QtyCap; QtyCap) {{ }} column(QtyTwo; QtyTwo) {{ }} column(TxtZero; TxtZero) {{ }} }} }} \
         rendering {{ layout(L) {{ Type = Word; LayoutFile = 'L.docx'; }} }} }}"
    )
}

fn title_of(props: &str) -> Option<super::shape::Title> {
    let l = al_testkit::compile(&report_src(props));
    let o = l.program.objects.iter().find(|o| o.name == "Z Rep").unwrap();
    super::shape::report_title(&l.program, o, o.report.as_ref().unwrap())
}

#[test]
fn the_title_is_the_plain_caption_or_the_name() {
    use super::shape::Title;
    // RX Wd Caption (RxHtmlWidenTests.al:190) and every report without a Caption.
    assert_eq!(
        title_of("Caption = 'Rx Cap';"),
        Some(Title {
            text: "Rx Cap".into(),
            caption: true
        })
    );
    assert_eq!(
        title_of(""),
        Some(Title {
            text: "Z Rep".into(),
            caption: false
        })
    );
    // CaptionML, an empty Caption, a Caption with a label attribute (no cell measured one), another property.
    for props in [
        "Caption = 'Rx Cap', Locked = true;",
        "Caption = 'Rx Cap', Comment = 'c';",
        "Caption = 'Rx Cap', MaxLength = 10;",
        "CaptionML = ENU = 'Rx Cap';",
        "Caption = '';",
        "Caption = 'Rx Cap'; CaptionML = ENU = 'Rx Cap';",
        "Caption = 'Rx Cap'; ProcessingOnly = false;",
    ] {
        assert_eq!(title_of(props), None, "{props}");
    }
    // The label attribute signal is a Comma token of the Caption property: a comma inside the literal or inside a
    // comment after it is not one, so the Caption stays the title.
    for (props, text) in [
        ("Caption = 'Rx, Cap';", "Rx, Cap"),
        ("Caption = 'Rx Cap' /* a, b */;", "Rx Cap"),
        ("Caption = 'Rx Cap'; // a, b\n", "Rx Cap"),
    ] {
        assert_eq!(
            title_of(props),
            Some(Title {
                text: text.into(),
                caption: true
            }),
            "{props}"
        );
    }
}

#[test]
fn a_field_column_is_measured_only_over_a_field_that_states_no_property() {
    let l = al_testkit::compile(&report_src(""));
    let o = l.program.objects.iter().find(|o| o.name == "Z Rep").unwrap();
    let item = &o.report.as_ref().unwrap().items[0];
    let table = l.program.objects[item.table.unwrap()].table.as_ref().unwrap();
    let kinds: Vec<Option<ColumnKind>> = item
        .columns
        .iter()
        .map(|c| super::shape::column_kind(table, c, &[]))
        .collect();
    use ColumnKind::*;
    // RowTxt and Qty; then BlankZero = true (BlankZero_IntegerZero, RxHtmlWidenTests.al:664, binds another namespace than
    // its report's, so its save keeps Part A), BlankZero = false, a Caption, BlankZero with BlankNumbers, BlankZero on a
    // Text field.
    assert_eq!(
        kinds,
        vec![Some(TextField), Some(IntegerField), None, None, None, None, None]
    );
}

#[test]
fn an_app_with_a_translation_file_of_the_call_language_may_translate_the_caption() {
    let src = report_src("Caption = 'Rx Cap';");
    let l = al_testkit::compile(&src);
    let object = l.program.objects.iter().position(|o| o.name == "Z Rep").unwrap();
    let app = l.program.objects[object].app as usize;
    let session = |l: al_sema::lower::Lowered| {
        let blocked = al_testkit::blocked(&l);
        crate::Session::new(crate::Engine::new(l.program, blocked))
    };
    let s = session(l);
    assert!(!s.app_translates_into(object, 1033));
    let mut l = al_testkit::compile(&src);
    // A translation file whose language is not known (no head read) may be of 1033.
    std::sync::Arc::make_mut(&mut l.program).apps[app].translations = true;
    let s = session(l);
    assert!(s.app_translates_into(object, 1033));
}
