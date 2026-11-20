//! Word packages in Media and MediaSet fields (al-builtins `media_word_package`): what BC stores for a .docx and its neighbors,
//! measured on Windows BC 27.5, 28.5 and 29.0 by corpus PR 1469 (onprem streams/media/MediaDocxRezip.al, not in the pinned corpus yet).
//! The paths run through the assumptions `media-word-package-rezipped`, `media-word-import-not-a-package` and `media-zip-mime-export-kept`.

use super::media::{import_with_files, package};
use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;

const MAJORS: [&str; 3] = ["27.5", "28.5", "29.0"];
const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const VARS: &str =
    "Fl: File; ins: InStream; Out: OutStream; InS: InStream; B: Record \"ALT Media\" temporary; s: Text; ok: Boolean; ";

fn files() -> Vec<(&'static str, Vec<u8>)> {
    let word = package("[Content_Types].xml");
    vec![
        ("\\TEMP\\w.docx", word.clone()),
        ("\\TEMP\\W.DOCX", word.clone()),
        ("\\TEMP\\w.dotx", word.clone()),
        ("\\TEMP\\w.docm", word.clone()),
        ("\\TEMP\\w.Docx", word.clone()),
        ("\\TEMP\\w.zip", word),
        ("\\TEMP\\other.docx", package("word/document.xml")),
        ("\\TEMP\\other.dotx", package("word/document.xml")),
        ("\\TEMP\\TEXT.DOCX", b"hello docx".to_vec()),
        ("\\TEMP\\text.docx", b"hello docx".to_vec()),
        ("\\TEMP\\text.bin", b"hello docx".to_vec()),
        ("\\TEMP\\bin.bin", vec![0xFF, 0xFE, 0, 1, 2]),
    ]
}

fn run(bc: &str, body: &str) -> Result<String, Signal> {
    import_with_files(Some(bc), &files(), &format!("{VARS}begin {body} end;"))
}

/// Refused while it ran, not by a call the test source got wrong.
fn refused(r: &Result<String, Signal>) -> bool {
    matches!(r, Err(Signal::Unsupported(w)) if !w.contains("no callee"))
}

/// Opens `ins` over the file `name` of `files()`.
fn open(name: &str) -> String {
    format!("Fl.Open('\\TEMP\\{name}'); Fl.CreateInStream(ins);")
}

/// Reads the Tenant Media Content of `id`.
const STORED: &str = "T.Get(id); T.CalcFields(Content); T.Content.CreateInStream(InS); InS.ReadText(s); exit(s);";
/// Exports the Picture of the inserted row A into B.Src and reads it.
const EXPORTED: &str = "Rec.Get('A'); B.Code := 'B'; B.Insert(); B.Src.CreateOutStream(Out); Rec.Picture.ExportStream(Out); B.Modify(); \
                        B.Get('B'); B.CalcFields(Src); B.Src.CreateInStream(InS); InS.ReadText(s); exit(s);";

// MediaDocxRezip.al:393 Docx_MediaImportFile_Stored, :755 _UpperCaseExtension, :821 _DotxExtension, :857 _DocmExtension: the import
// gives the field a media; BC stores a re-zipped package whose bytes depend on the host, so a read of the stored or exported bytes
// is refused on every major. A .docx in any letter case with any zip content was main's answer (MediaFiles.al:489-519 pins the
// import of a lower-case one) and takes no assumption; .dotx and .docm are new answers under `media-word-package-rezipped`.
#[test]
fn import_file_of_a_word_package_has_a_value_and_its_bytes_are_refused_on_every_major() {
    for bc in MAJORS {
        for name in ["w.docx", "W.DOCX", "w.Docx", "other.docx", "w.dotx", "w.docm"] {
            take_hits();
            let r = run(
                bc,
                &format!(
                    "id := Rec.Picture.ImportFile('\\TEMP\\{name}', 'Template'); Rec.Code := 'A'; Rec.Insert(); \
                     exit(Format(IsNullGuid(id)) + Format(Rec.Picture.HasValue()));"
                ),
            );
            assert_eq!(r.unwrap(), "NoYes", "{bc} {name}");
            assert_eq!(
                take_hits().contains(&Assumption::MediaWordPackageRezipped),
                matches!(name, "w.dotx" | "w.docm"),
                "{bc} {name}"
            );
            for read in [STORED, EXPORTED] {
                let r = run(
                    bc,
                    &format!(
                        "id := Rec.Picture.ImportFile('\\TEMP\\{name}', 'Template'); Rec.Code := 'A'; Rec.Insert(); {read}"
                    ),
                );
                assert!(refused(&r), "{bc} {name} {read}: {r:?}");
            }
        }
    }
}

// MediaDocxRezip.al:996 Docx_MediaSetImportFile_Stored: a .docx into an empty MediaSet gives one item, stored re-zipped.
#[test]
fn media_set_import_file_of_a_word_package_counts_one_and_refuses_its_bytes() {
    for bc in MAJORS {
        take_hits();
        let r = run(
            bc,
            "Rec.Images.ImportFile('\\TEMP\\w.docx', 'Template'); exit(Format(Rec.Images.Count()));",
        );
        assert_eq!(r.unwrap(), "1", "{bc}");
        assert!(take_hits().contains(&Assumption::MediaWordPackageRezipped), "{bc}");
        let r = run(
            bc,
            &format!("Rec.Images.ImportFile('\\TEMP\\w.docx', 'Template'); id := Rec.Images.Item(1); {STORED}"),
        );
        assert!(refused(&r), "{bc}: {r:?}");
    }
}

// MediaDocxRezip.al:594 Docx_MediaImportStream_DocxMime, :636 _DocxFileName, :917 _UpperCaseDocxMime, :1035
// Docx_MediaSetImportStream_Stored: a Word package imported from a stream with the Word document MIME type is stored re-zipped.
// Main stored and exported the stream's bytes. The import itself was main's answer: no assumption. The Word template MIME type is
// not measured; a Word MIME type re-zips (facts header :10-11), so its bytes are refused too.
#[test]
fn import_stream_of_a_word_package_with_the_word_mime_type_refuses_its_bytes() {
    let upper = DOCX_MIME.to_ascii_uppercase();
    for bc in MAJORS {
        for import in [
            format!("id := Rec.Picture.ImportStream(ins, 'Template', '{DOCX_MIME}');"),
            format!("id := Rec.Picture.ImportStream(ins, 'Template', '{upper}');"),
            format!("id := Rec.Picture.ImportStream(ins, 'Template', '{DOCX_MIME}', 'template.docx');"),
            "id := Rec.Picture.ImportStream(ins, 'Template', 'application/vnd.openxmlformats-officedocument.wordprocessingml.template');"
                .to_string(),
        ] {
            take_hits();
            let r = run(
                bc,
                &format!("{} {import} Rec.Code := 'A'; Rec.Insert(); exit(Format(Rec.Picture.HasValue()));", open("w.docx")),
            );
            assert_eq!(r.unwrap(), "Yes", "{bc} {import}");
            assert!(take_hits().is_empty(), "{bc} {import}");
            for read in [STORED, EXPORTED] {
                let r = run(bc, &format!("{} {import} Rec.Code := 'A'; Rec.Insert(); {read}", open("w.docx")));
                assert!(refused(&r), "{bc} {import} {read}: {r:?}");
            }
        }
        let r = run(
            bc,
            &format!(
                "{} Rec.Images.ImportStream(ins, 'Template', '{DOCX_MIME}'); id := Rec.Images.Item(1); {STORED}",
                open("w.docx")
            ),
        );
        assert!(refused(&r), "{bc}: {r:?}");
    }
}

// MediaDocxRezip.al:790 Docx_MediaImportFile_NotAPackage and :960 Docx_MediaImportStream_NotAPackageDocxMime: plain text named
// .docx, or imported with the Word document MIME type, raises nothing; the call returns the null Guid, the field has no value and
// no Tenant Media row is made; ExportStream writes nothing.
#[test]
fn plain_text_as_a_word_document_imports_nothing() {
    for bc in MAJORS {
        for import in [
            "id := Rec.Picture.ImportFile('\\TEMP\\text.docx', 'Template');".to_string(),
            format!(
                "{} id := Rec.Picture.ImportStream(ins, 'Template', '{DOCX_MIME}');",
                open("text.bin")
            ),
        ] {
            take_hits();
            let r = run(
                bc,
                &format!(
                    "{import} Rec.Code := 'A'; Rec.Insert(); Rec.Get('A'); ok := T.Get(id); B.Code := 'B'; B.Insert(); B.Src.CreateOutStream(Out); \
                     Rec.Picture.ExportStream(Out); B.Modify(); B.Get('B'); B.CalcFields(Src); \
                     exit(Format(IsNullGuid(id)) + Format(IsNullGuid(Rec.Picture.MediaId())) + Format(Rec.Picture.HasValue()) + \
                     Format(ok) + Format(B.Src.Length()));"
                ),
            );
            assert_eq!(r.unwrap(), "YesYesNoNo0", "{bc} {import}");
            assert!(
                take_hits().contains(&Assumption::MediaWordImportNotAPackage),
                "{bc} {import}"
            );
        }
    }
}

// MediaDocxRezip.al:678 Docx_MediaImportStream_ZipMime: the package imported from a stream as application/zip is stored as it is
// and ExportStream gives its bytes back. Main refused the export (it re-encodes content of other MIME types).
#[test]
fn import_stream_of_a_package_as_application_zip_exports_its_bytes() {
    for bc in MAJORS {
        take_hits();
        let r = run(
            bc,
            &format!(
                "{} id := Rec.Picture.ImportStream(ins, 'Template', 'application/zip'); Rec.Code := 'A'; Rec.Insert(); Rec.Get('A'); \
                 B.Code := 'B'; B.Insert(); B.Src.CreateOutStream(Out); Rec.Picture.ExportStream(Out); B.Modify(); B.Get('B'); \
                 B.CalcFields(Src); exit(Format(B.Src.Length()));",
                open("w.docx")
            ),
        );
        assert_eq!(r.unwrap(), package("[Content_Types].xml").len().to_string(), "{bc}");
        assert!(take_hits().contains(&Assumption::MediaZipMimeExportKept), "{bc}");
    }
}

// Shapes the probe did not measure: plain text named .DOCX, a .dotx zip whose first entry is not [Content_Types].xml, plain text
// into a field that holds a media or into a MediaSet, plain text with a file name or the upper-case MIME type, binary content that
// is no zip with the Word MIME type, a .docx into a MediaSet that holds an item. Each is refused.
#[test]
fn word_document_shapes_no_probe_measured_are_refused() {
    for bc in MAJORS {
        for body in [
            "Rec.Picture.ImportFile('\\TEMP\\TEXT.DOCX', 'x'); exit('');".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\other.dotx', 'x'); exit('');".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\w.docx', 'x'); Rec.Picture.ImportFile('\\TEMP\\text.docx', 'x'); exit('');".to_string(),
            "Rec.Images.ImportFile('\\TEMP\\text.docx', 'x'); exit('');".to_string(),
            "Rec.Images.ImportFile('\\TEMP\\w.docx', 'x'); Rec.Images.ImportFile('\\TEMP\\w.docx', 'y'); exit('');".to_string(),
            format!("{} Rec.Picture.ImportStream(ins, 'x', '{DOCX_MIME}', 'a.docx'); exit('');", open("text.bin")),
            format!(
                "{} Rec.Picture.ImportStream(ins, 'x', '{}'); exit('');",
                open("text.bin"),
                DOCX_MIME.to_ascii_uppercase()
            ),
            format!("{} Rec.Picture.ImportStream(ins, 'x', '{DOCX_MIME}'); exit('');", open("bin.bin")),
            format!("{} Rec.Images.ImportStream(ins, 'x', '{DOCX_MIME}'); exit('');", open("text.bin")),
        ] {
            let r = run(bc, &body);
            assert!(refused(&r), "{bc} {body}: {r:?}");
        }
    }
}

// Neighbors that keep main's answer: a package imported from a stream with no MIME type exports its bytes
// (streams/media/Y44MediaExportStream.al:262-264 Media_Binary_ExportStream; MediaDocxRezip.al:564 measures it too), application/zip
// over text or with a file name keeps the refusal of a look at the export, and a .zip file keeps the refusal of the import. None of them takes an assumption of this area.
#[test]
fn neighbors_of_the_word_package_shapes_keep_mains_answer() {
    for bc in MAJORS {
        take_hits();
        let r = run(
            bc,
            &format!(
                "{} id := Rec.Picture.ImportStream(ins, 'Template'); Rec.Code := 'A'; Rec.Insert(); Rec.Get('A'); B.Code := 'B'; \
                 B.Insert(); B.Src.CreateOutStream(Out); Rec.Picture.ExportStream(Out); B.Modify(); B.Get('B'); B.CalcFields(Src); \
                 exit(Format(B.Src.Length()));",
                open("w.docx")
            ),
        );
        assert_eq!(r.unwrap(), package("[Content_Types].xml").len().to_string(), "{bc}");
        for import in [
            format!(
                "{} id := Rec.Picture.ImportStream(ins, 'Template', 'application/zip');",
                open("text.bin")
            ),
            format!(
                "{} id := Rec.Picture.ImportStream(ins, 'Template', 'application/zip', 'a.zip');",
                open("w.docx")
            ),
        ] {
            let r = run(bc, &format!("{import} Rec.Code := 'A'; Rec.Insert(); {EXPORTED}"));
            assert!(refused(&r), "{bc} {import}: {r:?}");
        }
        let r = run(bc, "Rec.Picture.ImportFile('\\TEMP\\w.zip', 'x'); exit('');");
        assert!(refused(&r), "{bc}: {r:?}");
        let hits = take_hits();
        for a in [
            Assumption::MediaWordPackageRezipped,
            Assumption::MediaWordImportNotAPackage,
            Assumption::MediaZipMimeExportKept,
        ] {
            assert!(!hits.contains(&a), "{bc} {a:?}");
        }
    }
}
