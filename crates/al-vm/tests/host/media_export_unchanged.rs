//! Media.ExportStream gives back the bytes of non-picture content unchanged, whatever MIME type the import named
//! (streams/lt-y44-media-export/Y44MediaExportStream.al, corpus PR 962).

const TABLE: &str = r#"
table 60981 "ALT Media Export"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Pic; Media) { }
        field(3; Src; Blob) { }
        field(4; Dst; Blob) { }
    }
    keys { key(PK; Id) { } }
}
"#;

/// Imports what `fill` writes into Src with `import`, exports it into Dst and gives `<answer>:<bytes as decimals>`.
fn export(fill: &str, import: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let src = format!(
        "{TABLE}codeunit 50100 C {{ procedure F(): Text var R: Record \"ALT Media Export\"; ins: InStream; outs: OutStream;
            B: Byte; I: Integer; a: Text; begin
            R.Id := 1; R.Src.CreateOutStream(outs); {fill} R.Src.CreateInStream(ins); {import} R.Insert(); R.Get(1);
            R.Dst.CreateOutStream(outs); a := Format(R.Pic.ExportStream(outs)) + ':'; R.Dst.CreateInStream(ins);
            while not ins.EOS() do begin ins.Read(B); I := B; a += Format(I) + ','; end; exit(a); end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

/// Y44MediaExportStream.al Media_TextWithMime_ExportStream (MIME 'text/plain' and '').
#[test]
fn text_imported_with_a_mime_type_exports_unchanged() {
    for mime in ["text/plain", ""] {
        let got = al_testkit::text(export(
            "outs.WriteText('Hi');",
            &format!("R.Pic.ImportStream(ins, 'd', '{mime}');"),
        ));
        assert_eq!(got, "Yes:72,105,", "{mime}");
    }
}

/// Y44MediaExportStream.al Media_Binary_ExportStream (16 bytes 0x00-0x0F, two-argument import).
#[test]
fn binary_content_that_is_no_picture_exports_unchanged() {
    let got = al_testkit::text(export(
        "for I := 0 to 15 do begin B := I; outs.Write(B); end;",
        "R.Pic.ImportStream(ins, 'd');",
    ));
    assert_eq!(got, "Yes:0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,");
}

/// A MIME type no corpus case pins for text keeps the refusal of a look at the exported bytes.
#[test]
fn text_imported_with_another_mime_type_stays_refused() {
    let r = export(
        "outs.WriteText('Hi');",
        "R.Pic.ImportStream(ins, 'd', 'application/pdf');",
    );
    assert!(format!("{r:?}").contains("not pinned"), "{r:?}");
}
