//! Word packages in Media and MediaSet fields (al-builtins `media_word_package`): what BC stores for a .docx and its neighbors,
//! measured on Windows BC 27.5, 28.5 and 29.0 by corpus PR 1469 (onprem streams/media/MediaDocxRezip.al, not in the pinned corpus yet).
//! The paths run through the assumptions `media-word-package-rezipped`, `media-word-import-not-a-package` and `media-zip-mime-export-kept`.

use super::media::{import_with_files, local_header, word_package};
use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;

const MAJORS: [&str; 3] = ["27.5", "28.5", "29.0"];
const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
const VARS: &str =
    "Fl: File; ins: InStream; Out: OutStream; InS: InStream; B: Record \"ALT Media\" temporary; s: Text; ok: Boolean; ";

/// A zip of the entries `names`, each with `method` (8: its data is one stored deflate block) and the data `<p/>`, with a central
/// directory; the CRC fields stay 0.
fn zip(names: &[&str], method: u16) -> Vec<u8> {
    let data: &[u8] = b"<p/>";
    let body = if method == 8 {
        [&[1, 4, 0, 0xFB, 0xFF][..], data].concat()
    } else {
        data.to_vec()
    };
    let sizes = [body.len(), data.len()].map(|n| u32::try_from(n).unwrap().to_le_bytes());
    let (mut b, mut cd) = (Vec::new(), Vec::new());
    for name in names {
        let offset = u32::try_from(b.len()).unwrap().to_le_bytes();
        let len = u16::try_from(name.len()).unwrap().to_le_bytes();
        b.extend_from_slice(b"PK\x03\x04\x14\0\0\0");
        b.extend_from_slice(&method.to_le_bytes());
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(&sizes.concat());
        b.extend_from_slice(&len);
        b.extend_from_slice(&[0, 0]);
        b.extend_from_slice(name.as_bytes());
        b.extend_from_slice(&body);
        cd.extend_from_slice(b"PK\x01\x02\x14\0\x14\0\0\0");
        cd.extend_from_slice(&method.to_le_bytes());
        cd.extend_from_slice(&[0; 8]);
        cd.extend_from_slice(&sizes.concat());
        cd.extend_from_slice(&len);
        cd.extend_from_slice(&[0; 12]);
        cd.extend_from_slice(&offset);
        cd.extend_from_slice(name.as_bytes());
    }
    let count = u16::try_from(names.len()).unwrap().to_le_bytes();
    let at = u32::try_from(b.len()).unwrap().to_le_bytes();
    let size = u32::try_from(cd.len()).unwrap().to_le_bytes();
    b.extend_from_slice(&cd);
    b.extend_from_slice(b"PK\x05\x06\0\0\0\0");
    for f in [&count[..], &count, &size, &at, &[0, 0]] {
        b.extend_from_slice(f);
    }
    b
}

const PARTS: [&str; 3] = ["[Content_Types].xml", "_rels/.rels", "word/document.xml"];

fn files() -> Vec<(&'static str, Vec<u8>)> {
    let word = word_package();
    let cut = local_header("[Content_Types].xml");
    vec![
        ("\\TEMP\\w.docx", word.clone()),
        ("\\TEMP\\W.DOCX", word.clone()),
        ("\\TEMP\\w.dotx", word.clone()),
        ("\\TEMP\\w.docm", word.clone()),
        ("\\TEMP\\w.Docx", word.clone()),
        ("\\TEMP\\w.zip", word),
        ("\\TEMP\\other.docx", local_header("word/document.xml")),
        ("\\TEMP\\other.dotx", local_header("word/document.xml")),
        ("\\TEMP\\cut.docx", cut.clone()),
        ("\\TEMP\\cut.dotx", cut.clone()),
        ("\\TEMP\\cut.docm", cut),
        ("\\TEMP\\one.docm", zip(&PARTS[..1], 8)),
        ("\\TEMP\\stored.dotx", zip(&PARTS, 0)),
        (
            "\\TEMP\\extra.dotx",
            zip(&[PARTS[0], PARTS[1], PARTS[2], "word/extra.xml"], 8),
        ),
        ("\\TEMP\\reordered.dotx", zip(&[PARTS[1], PARTS[0], PARTS[2]], 8)),
        ("\\TEMP\\empty.bin", Vec::new()),
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

// MediaDocxRezip.al:392 Docx_MediaImportFile_Stored, :754 _UpperCaseExtension, :820 _DotxExtension, :856 _DocmExtension: the import
// gives the field a media; BC stores a re-zipped package whose bytes depend on the host, so a read of the stored or exported bytes
// is refused on every major. A .docx in any letter case with any zip content (a cut package too) was main's answer (MediaFiles.al:489-519 pins the
// import of a lower-case one) and takes no assumption; .dotx and .docm are new answers under `media-word-package-rezipped`.
#[test]
fn import_file_of_a_word_package_has_a_value_and_its_bytes_are_refused_on_every_major() {
    for bc in MAJORS {
        for name in [
            "w.docx",
            "W.DOCX",
            "w.Docx",
            "other.docx",
            "cut.docx",
            "w.dotx",
            "w.docm",
        ] {
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

// MediaDocxRezip.al:995 Docx_MediaSetImportFile_Stored: the package as a .docx into a MediaSet field that was never filled gives one
// item, stored re-zipped.
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

// MediaDocxRezip.al:593 Docx_MediaImportStream_DocxMime, :635 _DocxFileName, :916 _UpperCaseDocxMime, :1034
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
            "id := Rec.Picture.ImportStream(ins, 'Template', 'application/vnd.ms-word.document.macroEnabled.12');".to_string(),
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

// MediaDocxRezip.al:789 Docx_MediaImportFile_NotAPackage and :959 Docx_MediaImportStream_NotAPackageDocxMime: plain text named
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

// MediaDocxRezip.al:677 Docx_MediaImportStream_ZipMime: the package imported by a three-argument call as application/zip into an
// empty field is stored as it is and ExportStream gives its bytes back. Main refused the export (it re-encodes content of other MIME
// types).
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
        assert_eq!(r.unwrap(), word_package().len().to_string(), "{bc}");
        assert!(take_hits().contains(&Assumption::MediaZipMimeExportKept), "{bc}");
    }
}

// Shapes the probe did not measure, each refused. The probe imports only the 911-byte package of three deflated parts, by a call
// into a field or set that was never filled; the application/zip import has three arguments. Refused: plain text named .DOCX, into a
// field that holds a media, into a MediaSet, with a file name or the upper-case MIME type, empty or binary content with the Word MIME
// type; a .dotx or .docm zip that is not that package (another first entry, a cut header with no central directory, one part, the
// parts stored, a fourth part, another order); a .docx into a set that holds an item or held one (a removed member); a cut package or
// a blank fourth argument or a field that held a media for the application/zip export.
#[test]
fn word_document_shapes_no_probe_measured_are_refused() {
    let zip_export =
        |name: &str, import: &str| format!("{} {import} Rec.Code := 'A'; Rec.Insert(); {EXPORTED}", open(name));
    let mut ran = Vec::new();
    for bc in MAJORS {
        for body in [
            "Rec.Picture.ImportFile('\\TEMP\\TEXT.DOCX', 'x'); exit('');".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\other.dotx', 'x'); exit('');".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\cut.dotx', 'x'); exit(Format(Rec.Picture.HasValue()));".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\cut.docm', 'x'); exit(Format(Rec.Picture.HasValue()));".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\one.docm', 'x'); exit(Format(Rec.Picture.HasValue()));".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\stored.dotx', 'x'); exit(Format(Rec.Picture.HasValue()));".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\extra.dotx', 'x'); exit(Format(Rec.Picture.HasValue()));".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\reordered.dotx', 'x'); exit(Format(Rec.Picture.HasValue()));".to_string(),
            "Rec.Images.ImportFile('\\TEMP\\cut.docx', 'x'); exit(Format(Rec.Images.Count()));".to_string(),
            "Rec.Picture.ImportFile('\\TEMP\\w.docx', 'x'); Rec.Picture.ImportFile('\\TEMP\\text.docx', 'x'); exit('');".to_string(),
            "Rec.Images.ImportFile('\\TEMP\\text.docx', 'x'); exit('');".to_string(),
            "Rec.Images.ImportFile('\\TEMP\\w.docx', 'x'); Rec.Images.ImportFile('\\TEMP\\w.docx', 'y'); exit('');".to_string(),
            "Rec.Images.ImportFile('\\TEMP\\b.png', 'x'); Rec.Images.Remove(Rec.Images.Item(1)); \
             Rec.Images.ImportFile('\\TEMP\\w.docx', 'y'); exit(Format(Rec.Images.Count()));"
                .to_string(),
            format!("{} Rec.Picture.ImportStream(ins, 'x', '{DOCX_MIME}', 'a.docx'); exit('');", open("text.bin")),
            format!(
                "{} Rec.Picture.ImportStream(ins, 'x', '{}'); exit('');",
                open("text.bin"),
                DOCX_MIME.to_ascii_uppercase()
            ),
            format!("{} Rec.Picture.ImportStream(ins, 'x', '{DOCX_MIME}'); exit('');", open("bin.bin")),
            format!("{} Rec.Images.ImportStream(ins, 'x', '{DOCX_MIME}'); exit('');", open("text.bin")),
            format!(
                "{} Rec.Images.ImportStream(ins, 'x', '{DOCX_MIME}'); exit(Format(Rec.Images.Count()));",
                open("empty.bin")
            ),
            format!(
                "Rec.Picture.ImportFile('\\TEMP\\a.jpg', 'x'); {} Rec.Picture.ImportStream(ins, 'x', '{DOCX_MIME}'); \
                 exit(Format(Rec.Picture.HasValue()));",
                open("text.bin")
            ),
            zip_export("cut.docx", "id := Rec.Picture.ImportStream(ins, 'Template', 'application/zip');"),
            zip_export("w.docx", "id := Rec.Picture.ImportStream(ins, 'Template', 'application/zip', '');"),
            zip_export(
                "w.docx",
                "Rec.Picture.ImportFile('\\TEMP\\a.jpg', 'x'); id := Rec.Picture.ImportStream(ins, 'Template', 'application/zip');",
            ),
        ] {
            take_hits();
            let r = run(bc, &body);
            if !refused(&r) {
                ran.push(format!("{bc} {body}: {r:?} {:?}", take_hits()));
            }
        }
    }
    assert!(ran.is_empty(), "ran instead of refusing:\n{}", ran.join("\n"));
}

// Neighbors that keep main's answer: a package imported from a stream with no MIME type exports its bytes
// (streams/media/Y44MediaExportStream.al:262-264 Media_Binary_ExportStream; MediaDocxRezip.al:563 Docx_MediaImportStream_NoMime
// measures it too), application/zip over text or with a file name keeps the refusal of a look at the export, and a .zip file keeps
// the refusal of the import. None of them takes an assumption of this area.
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
        assert_eq!(r.unwrap(), word_package().len().to_string(), "{bc}");
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
