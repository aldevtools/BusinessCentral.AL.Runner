//! The bytes of a placeholder output, one small valid file per format. Only the kind is meant to match BC: the file
//! signature a test can tell (reports/layouts-saveas/AMReportOutputTests.al:41-52 `Head`: `%PDF-`, `PK`, `<`) and that it is
//! not empty. The content is never BC's, and a read of it is refused (`al_builtins::placeholder_output`).

/// A rendered format a placeholder stands in for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Format {
    Pdf,
    Word,
    Excel,
    Html,
}

impl Format {
    /// The `ReportFormat` member name.
    pub fn from_name(name: &str) -> Option<Format> {
        match name {
            "Pdf" => Some(Format::Pdf),
            "Word" => Some(Format::Word),
            "Excel" => Some(Format::Excel),
            "Html" => Some(Format::Html),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Format::Pdf => "Pdf",
            Format::Word => "Word",
            Format::Excel => "Excel",
            Format::Html => "Html",
        }
    }

    /// The bytes of the placeholder file.
    pub fn bytes(self) -> Vec<u8> {
        match self {
            Format::Pdf => placeholder_pdf(),
            Format::Word => placeholder_docx(),
            Format::Excel => placeholder_xlsx(),
            Format::Html => placeholder_html(),
        }
    }
}

/// A small valid PDF: the `%PDF-1.4` header, a catalog, a page tree and one blank Letter page, with a correct
/// cross-reference table.
pub fn placeholder_pdf() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> >>",
    ];
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.push_str(&format!("{} 0 obj\n{body}\nendobj\n", i + 1));
    }
    let xref = out.len();
    out.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for at in offsets {
        out.push_str(&format!("{at:010} 00000 n \n"));
    }
    out.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    ));
    out.into_bytes()
}

const XML_HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#;

/// A minimal valid .docx: a zip with `[Content_Types].xml`, the package relationships and `word/document.xml` holding one
/// empty paragraph (AMReportOutputTests.al:187 `main_part` Yes/No: the main part is `word/document.xml`).
pub fn placeholder_docx() -> Vec<u8> {
    let types = format!(
        "{XML_HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
         <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
         <Override PartName=\"/word/document.xml\" \
         ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/></Types>"
    );
    let rels = package_rels(
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
        "word/document.xml",
    );
    let document = format!(
        "{XML_HEAD}<w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
         <w:body><w:p/></w:body></w:document>"
    );
    stored_zip(&[
        ("[Content_Types].xml", types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("word/document.xml", document.as_bytes()),
    ])
}

/// A minimal valid .xlsx: a zip with `[Content_Types].xml`, the package relationships, `xl/workbook.xml`, its relationships
/// and one empty sheet (AMReportOutputTests.al:235 `main_part` No/Yes: the main part is `xl/workbook.xml`).
pub fn placeholder_xlsx() -> Vec<u8> {
    let types = format!(
        "{XML_HEAD}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
         <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
         <Override PartName=\"/xl/workbook.xml\" \
         ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>\
         <Override PartName=\"/xl/worksheets/sheet1.xml\" \
         ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>"
    );
    let rels = package_rels(
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument",
        "xl/workbook.xml",
    );
    let workbook = format!(
        "{XML_HEAD}<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" \
         xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\">\
         <sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
    );
    let workbook_rels = format!(
        "{XML_HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
         <Relationship Id=\"rId1\" \
         Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" \
         Target=\"worksheets/sheet1.xml\"/></Relationships>"
    );
    let sheet = format!(
        "{XML_HEAD}<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData/></worksheet>"
    );
    stored_zip(&[
        ("[Content_Types].xml", types.as_bytes()),
        ("_rels/.rels", rels.as_bytes()),
        ("xl/workbook.xml", workbook.as_bytes()),
        ("xl/_rels/workbook.xml.rels", workbook_rels.as_bytes()),
        ("xl/worksheets/sheet1.xml", sheet.as_bytes()),
    ])
}

/// A minimal valid html page (AMReportOutputTests.al:205 `kind` markup: it starts with `<`).
pub fn placeholder_html() -> Vec<u8> {
    b"<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title></title></head><body></body></html>\n".to_vec()
}

fn package_rels(kind: &str, target: &str) -> String {
    format!(
        "{XML_HEAD}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
         <Relationship Id=\"rId1\" Type=\"{kind}\" Target=\"{target}\"/></Relationships>"
    )
}

/// A zip archive of `entries`, each stored without compression.
fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let mut crc = flate2::Crc::new();
        crc.update(data);
        let (crc, size, at) = (crc.sum(), data.len() as u32, out.len() as u32);
        // Local file header: version 2.0, no flags, method 0 (stored), no time stamp.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for v in [crc, size, size] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        // Central directory header.
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        for v in [crc, size, size] {
            central.extend_from_slice(&v.to_le_bytes());
        }
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        // Extra field, comment, disk number, internal and external attributes.
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&at.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let (dir_at, dir_len, count) = (out.len() as u32, central.len() as u32, entries.len() as u16);
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&dir_len.to_le_bytes());
    out.extend_from_slice(&dir_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The entries of a stored zip, read back with `report_layout_export`'s reader.
    fn entries(bytes: &[u8]) -> Vec<String> {
        crate::report_layout_export::zip_entries(bytes)
            .expect("a zip")
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }

    #[test]
    fn the_docx_is_a_zip_whose_main_part_is_the_word_document() {
        let bytes = placeholder_docx();
        assert!(bytes.starts_with(b"PK"));
        assert_eq!(
            entries(&bytes),
            ["[Content_Types].xml", "_rels/.rels", "word/document.xml"]
        );
    }

    #[test]
    fn the_xlsx_is_a_zip_whose_main_part_is_the_workbook() {
        let bytes = placeholder_xlsx();
        assert!(bytes.starts_with(b"PK"));
        assert!(entries(&bytes).contains(&"xl/workbook.xml".to_string()));
        assert!(entries(&bytes).contains(&"xl/worksheets/sheet1.xml".to_string()));
    }

    #[test]
    fn the_html_is_markup() {
        assert!(placeholder_html().starts_with(b"<"));
    }
}
