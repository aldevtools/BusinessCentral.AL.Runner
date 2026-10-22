//! Media and MediaSet fields. The stream source is a Blob field (the corpus uses the "Temp Blob"
//! codeunit, which lives outside the workspace). Each test names the corpus test it follows.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_vm::{Engine, Session};

const TABLE: &str = r#"
table 60980 "ALT Media"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Description; Text[100]) { }
        field(3; Picture; Media) { }
        field(4; Images; MediaSet) { }
        field(5; Src; Blob) { }
    }
    keys { key(PK; "Code") { } }
}
"#;

fn text(vars: &str, body: &str) -> Result<String, Signal> {
    let src = format!(
        "{TABLE}codeunit 50100 C {{ procedure F(): Text var Rec: Record \"ALT Media\"; Rec2: Record \"ALT Media\"; Tmp: Record \"ALT Media\" temporary; a: Text; id: Guid; ins: InStream; outs: OutStream; {vars} begin {body} end; }}"
    );
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l
        .program
        .objects
        .iter()
        .position(|o| o.routines.iter().any(|r| r.name == "F"))
        .expect("object");
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    match s.call(oi, ri, inst, vec![])?.0 {
        Value::Text(t) => Ok(t.to_string()),
        other => panic!("{other:?}"),
    }
}

/// Writes `bytes` into the Src blob of `Rec` and opens `ins` over it.
fn feed(rec: &str, bytes: &str) -> String {
    format!("{rec}.Src.CreateOutStream(outs); outs.WriteText('{bytes}'); {rec}.Src.CreateInStream(ins);")
}

fn yn(b: &str) -> String {
    format!("a += Format({b});")
}

#[test]
fn an_empty_media_and_media_set_answer_a_null_media_id() {
    // media/TestMediaEmptyMediaId.al:22-120: Init without Insert, after Insert, and read back by
    // Get into two variables.
    let body = format!(
        "Rec.Init(); Rec.Code := 'E0'; {} {}
         Rec.Insert(); {} {}
         Rec2.Get('E0'); {} {} {}",
        yn("IsNullGuid(Rec.Picture.MediaId())"),
        yn("IsNullGuid(Rec.Images.MediaId())"),
        yn("IsNullGuid(Rec.Picture.MediaId())"),
        yn("IsNullGuid(Rec.Images.MediaId())"),
        yn("IsNullGuid(Rec2.Picture.MediaId())"),
        yn("IsNullGuid(Rec2.Images.MediaId())"),
        yn("Rec.Picture.MediaId() = Rec2.Picture.MediaId()"),
    ) + " exit(a);";
    assert_eq!(text("", &body).unwrap(), "YesYesYesYesYesYesYes");
}

#[test]
fn a_media_set_nothing_was_added_to_counts_zero_and_removes_nothing() {
    // media/TestMediaSetPersistence.al:59-70 (Count) and :251-262 (Remove of an id never inserted).
    let body = format!(
        "Rec.Code := 'C1'; Rec.Insert(); a += Format(Rec.Images.Count()); {} exit(a);",
        yn("Rec.Images.Remove(CreateGuid())")
    );
    assert_eq!(text("", &body).unwrap(), "0No");
}

#[test]
fn an_imported_media_survives_modify_and_get() {
    // media/TestMediaSetPersistence.al:30-54 (Media_ImportStream_ModifyThenGet_HasValueTrue).
    let body = format!(
        "Rec.Code := 'M1'; Rec.Insert(); {} id := Rec.Picture.ImportStream(ins, 'a picture'); Rec.Modify();
         {} Rec.Get('M1'); {} exit(a);",
        feed("Rec", "bytes"),
        yn("IsNullGuid(id)"),
        yn("Rec.Picture.HasValue()")
    );
    assert_eq!(text("", &body).unwrap(), "NoYes");
}

#[test]
fn an_import_with_a_file_name_fills_the_field_like_the_other_forms() {
    // Base Application's Document Attachment imports with `ImportStream(Stream, '', '', FileName)`
    // ("description and mime-type are set empty and will be automatically set by platform code from
    // the filename"). What the file name changes is visible only in the Tenant Media row, which this
    // runtime does not read (ADR-017); the field and the bytes behave as in the three argument form
    // (media/TestMediaSetPersistence.al:30-54).
    let body = format!(
        "Rec.Code := 'F1'; Rec.Insert(); {} id := Rec.Picture.ImportStream(ins, '', '', 'doc.pdf'); Rec.Modify();
         {} Rec.Get('F1'); {} exit(a);",
        feed("Rec", "bytes"),
        yn("IsNullGuid(id)"),
        yn("Rec.Picture.HasValue()")
    );
    assert_eq!(text("", &body).unwrap(), "NoYes");
}

#[test]
fn an_import_is_not_saved_until_the_record_is() {
    // The field holds the media's id only once Modify wrote it: a second Get without it reads an
    // empty field (media/TestMediaEmptyMediaId.al:80-98 reads an empty field after Insert).
    let body = format!(
        "Rec.Code := 'M2'; Rec.Insert(); {} id := Rec.Picture.ImportStream(ins, 'x'); {} Rec2.Get('M2'); {} exit(a);",
        feed("Rec", "bytes"),
        yn("Rec.Picture.HasValue()"),
        yn("Rec2.Picture.HasValue()")
    );
    assert_eq!(text("", &body).unwrap(), "YesNo");
}

#[test]
fn an_imported_media_set_has_one_item_whose_id_differs_from_the_set_id() {
    // media/TestMediaSetPersistence.al:73-155 (CountIsOne, MediaId_EqualsReturnedId, Item_DiffersFromMediaId)
    // and :100-124 (read through a second record variable).
    let body = format!(
        "Rec.Code := 'MS1'; Rec.Insert(); {} id := Rec.Images.ImportStream(ins, 'sample image'); Rec.Modify();
         {} Rec2.Get('MS1'); a += Format(Rec2.Images.Count()); {} {} {} exit(a);",
        feed("Rec", "bytes"),
        yn("IsNullGuid(id)"),
        yn("Rec2.Images.MediaId() = id"),
        yn("Rec2.Images.Item(1) = id"),
        yn("IsNullGuid(Rec2.Images.Item(1))"),
    );
    assert_eq!(text("", &body).unwrap(), "No1YesNoNo");
}

#[test]
fn removing_an_item_empties_the_set_after_modify_and_get() {
    // media/TestMediaSetPersistence.al:221-248 (MediaSet_Remove_ModifyThenGet_CountIsZero).
    let body = format!(
        "Rec.Code := 'MS6'; Rec.Insert(); {} Rec.Images.ImportStream(ins, 'to be removed'); Rec.Modify();
         Rec.Get('MS6'); id := Rec.Images.Item(1); {} Rec.Modify(); Rec.Get('MS6'); a += Format(Rec.Images.Count()); exit(a);",
        feed("Rec", "bytes"),
        yn("Rec.Images.Remove(id)")
    );
    assert_eq!(text("", &body).unwrap(), "Yes0");
}

#[test]
fn a_temporary_record_keeps_its_media_set() {
    // media/TestMediaSetPersistence.al:265-285.
    let body = format!(
        "Tmp.Code := 'TMP1'; Tmp.Insert(); {} id := Tmp.Images.ImportStream(ins, 'temp record'); Tmp.Modify();
         {} Tmp.Get('TMP1'); a += Format(Tmp.Images.Count()); exit(a);",
        feed("Tmp", "bytes"),
        yn("IsNullGuid(id)")
    );
    assert_eq!(text("", &body).unwrap(), "No1");
}

#[test]
fn a_look_at_the_exported_bytes_is_refused() {
    // types/W7Media.al:141-145 (corpus bump 34): BC re-encodes an imported PNG on every major, into bytes the corpus does not
    // pin, so reading what ExportStream wrote is refused (media/TestMediaPngImport.al:99-168 asserts only the signature and
    // the dimensions, which the runtime cannot produce without the encoder).
    let body = format!(
        "Rec.Code := 'PNG2'; Rec.Insert(); {} Rec.Picture.ImportStream(ins, 'png'); Rec.Modify(); Rec.Get('PNG2');
         Rec2.Src.CreateOutStream(outs); Rec.Picture.ExportStream(outs); Rec2.Src.CreateInStream(ins); ins.ReadText(a); exit(a);",
        feed("Rec", "PNG-signature-and-1x1")
    );
    assert!(matches!(text("", &body), Err(Signal::Unsupported(_))));
}

#[test]
fn what_the_corpus_does_not_pin_is_refused() {
    let refused = |body: String, why: &str| {
        let r = text("", &body);
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains(why)),
            "{body}: {r:?}"
        );
    };
    // A MediaSet that already has members.
    refused(
        format!(
            "Rec.Code := 'R2'; Rec.Insert(); {} Rec.Images.ImportStream(ins, 'a'); {} Rec.Images.ImportStream(ins, 'b'); exit('');",
            feed("Rec", "x"),
            feed("Rec", "y")
        ),
        "already has members",
    );
    // An item index outside the set; exporting an empty field.
    refused(
        "Rec.Code := 'R3'; Rec.Insert(); id := Rec.Images.Item(1); exit('');".into(),
        "outside the set",
    );
    refused(
        "Rec.Code := 'R4'; Rec.Insert(); Rec.Picture.ExportStream(outs); exit('');".into(),
        "empty field",
    );
}

// media/PXMediaFileName.al: importing into a Media field that already holds a value replaces it. The exported bytes cannot
// show it any more (they are refused, types/W7Media.al:141-145); the second import still runs.
#[test]
fn importing_into_a_media_that_holds_a_value_replaces_it() {
    let body = format!(
        "Rec.Code := 'R1'; Rec.Insert(); {} Rec.Picture.ImportStream(ins, 'a'); {} Rec.Picture.ImportStream(ins, 'b');
         exit(Format(Rec.Picture.HasValue()));",
        feed("Rec", "x"),
        feed("Rec", "y")
    );
    assert_eq!(text("", &body).unwrap(), "Yes");
}

// ---- Media.ImportFile of a file that exists --------------------------------------------------------------------------

const TENANT_MEDIA: &str = r#"
table 2000000184 "Tenant Media"
{
    fields
    {
        field(1; ID; Guid) { } field(2; Description; Text[250]) { } field(3; Content; Blob) { }
        field(4; "Mime Type"; Text[100]) { } field(5; Height; Integer) { } field(6; Width; Integer) { }
        field(11; "File Name"; Text[250]) { }
    }
    keys { key(PK; ID) { } }
}
"#;

/// A JPEG of 3 x 2 pixels as far as its size goes: SOI, APP0, SOF0, EOI.
fn jpeg() -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    b.extend_from_slice(b"JFIF\0\x01\x01\x00\x00\x01\x00\x01\x00\x00");
    b.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x03, 0x03]);
    b.extend_from_slice(&[1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    b.extend_from_slice(&[0xFF, 0xD9]);
    b
}

/// A PNG of 4 x 5 pixels as far as its size goes.
fn png() -> Vec<u8> {
    let mut b = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
    b.extend_from_slice(b"IHDR");
    b.extend_from_slice(&[0, 0, 0, 4, 0, 0, 0, 5, 8, 6, 0, 0, 0, 0, 0, 0, 0]);
    b
}

fn import_file(body: &str) -> Result<String, Signal> {
    use al_host::FileSystem;
    let src = format!(
        "{TABLE}{TENANT_MEDIA}codeunit 50101 C {{ procedure F(): Text var Rec: Record \"ALT Media\"; T: Record \"Tenant Media\"; id: Guid; {body} }}"
    );
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    let mut files = al_host::MemFileSystem::default();
    files.write("\\TEMP\\a.jpg", &jpeg()).unwrap();
    files.write("\\TEMP\\b.png", &png()).unwrap();
    files.write("\\TEMP\\c.png", &jpeg()).unwrap();
    files.write("\\TEMP\\d.gif", &png()).unwrap();
    files.write("\\TEMP\\E.PNG", &png()).unwrap();
    let mut s = Session::with_host(al_testkit::engine(&l), al_host::Host::default().with_files(files));
    let inst = s.instance(oi);
    match s.call(oi, ri, inst, vec![])?.0 {
        Value::Text(t) => Ok(t.to_string()),
        other => panic!("{other:?}"),
    }
}

const DESCRIBE: &str = "Rec.Code := 'A'; Rec.Insert(); T.Get(id); exit(Format(Rec.Picture.HasValue()) + '|' + T.Description + '|' + \
                        T.\"Mime Type\" + '|' + Format(T.Width) + 'x' + Format(T.Height) + '|' + T.\"File Name\");";

// onprem media/MediaFiles.al:293-340 QX_MediaAndMediaSet_ImportFile_Jpeg: the returned id is the media's, image/jpeg from a
// .jpg (and .JPG) file, the picture's size, and the File Name is the path as given ('<tmp><name>').
#[test]
fn import_file_of_a_jpeg_file_fills_the_tenant_media_row() {
    let r = import_file(&format!(
        "begin id := Rec.Picture.ImportFile('\\TEMP\\a.jpg', 'Cronus'); {DESCRIBE} end;"
    ));
    assert_eq!(r.unwrap(), "Yes|Cronus|image/jpeg|3x2|\\TEMP\\a.jpg");
}

// Not pinned: a .png file (Microsoft's DemoTool, Create Getting Started Data through Media Resources Mgt.InsertMediaFromFile).
// Answered as a PNG imported from a stream, under the assumption `media-importfile-png`.
#[test]
fn import_file_of_a_png_file_takes_its_assumption() {
    use al_builtins::assumptions::{Assumption, take_hits};
    take_hits();
    let r = import_file(&format!(
        "begin id := Rec.Picture.ImportFile('\\TEMP\\b.png', 'P'); {DESCRIBE} end;"
    ));
    assert_eq!(r.unwrap(), "Yes|P|image/png|4x5|\\TEMP\\b.png");
    assert!(take_hits().contains(&Assumption::MediaImportFilePng));
}

#[test]
fn import_file_of_other_content_or_a_media_set_stays_refused() {
    for body in [
        "begin Rec.Picture.ImportFile('\\TEMP\\c.png', 'x'); exit(''); end;",
        "begin Rec.Picture.ImportFile('\\TEMP\\d.gif', 'x'); exit(''); end;",
        "begin Rec.Images.ImportFile('\\TEMP\\c.png', 'x'); exit(''); end;",
    ] {
        let r = import_file(body);
        assert!(matches!(&r, Err(Signal::Unsupported(_))), "{body}: {r:?}");
    }
}

// onprem media/MediaFiles.al:317-344 QX_MediaAndMediaSet_ImportFile_Jpeg: a .jpg imported into an empty MediaSet gives the set
// one member whose Tenant Media row has the description, image/jpeg and the picture's size ('mediaset count',
// 'mediaset item 1 tenant'). The return value and the row's File Name are not pinned (:340 ignores it): a read of either is refused.
#[test]
fn media_set_import_file_of_a_jpeg_fills_the_set() {
    let r = import_file(
        "begin Rec.Images.ImportFile('\\TEMP\\a.jpg', 'Item picture'); Rec.Code := 'A'; Rec.Insert(); Rec.Get('A'); \
         T.Get(Rec.Images.Item(1)); exit(Format(Rec.Images.Count()) + '|' + T.Description + '|' + T.\"Mime Type\" + '|' + \
         Format(T.Width) + 'x' + Format(T.Height)); end;",
    );
    assert_eq!(r.unwrap(), "1|Item picture|image/jpeg|3x2");
}

// onprem media/MediaFiles.al:394-447 RD_MediaSet_ImportFile_JpegThenUpperCasePng: both ImportFile calls return the set's
// id (MediaId, not an item's), a second file joins the set as Item(2) and a `.PNG` file holding a PNG is image/png with its
// size; Count 2 and the order stay after Modify and Get.
#[test]
fn media_set_import_file_returns_the_set_id_and_a_png_joins_as_the_second_item() {
    use al_builtins::assumptions::take_hits;
    take_hits();
    let r = import_file(
        "id2: Guid; begin id := Rec.Images.ImportFile('\\TEMP\\a.jpg', 'jpeg item'); Rec.Code := 'A'; Rec.Insert(); \
         id2 := Rec.Images.ImportFile('\\TEMP\\E.PNG', 'png item'); Rec.Modify(); Rec.Get('A'); \
         T.Get(Rec.Images.Item(2)); exit(Format(id = id2) + '|' + Format(id = Rec.Images.MediaId()) + '|' + \
         Format(id = Rec.Images.Item(1)) + '|' + Format(Rec.Images.Count()) + '|' + T.Description + '|' + T.\"Mime Type\" + '|' + \
         Format(T.Width) + 'x' + Format(T.Height)); end;",
    );
    assert_eq!(r.unwrap(), "Yes|Yes|No|2|png item|image/png|4x5");
    assert!(take_hits().is_empty());
}

#[test]
fn media_set_import_file_file_name_and_a_third_member_are_refused() {
    for body in [
        "begin Rec.Images.ImportFile('\\TEMP\\a.jpg', 'x'); T.Get(Rec.Images.Item(1)); exit(T.\"File Name\"); end;",
        "begin Rec.Images.ImportFile('\\TEMP\\a.jpg', 'x'); Rec.Images.ImportFile('\\TEMP\\E.PNG', 'y'); \
         Rec.Images.ImportFile('\\TEMP\\a.jpg', 'z'); exit(''); end;",
    ] {
        let r = import_file(body);
        assert!(
            matches!(&r, Err(Signal::Unsupported(w)) if w.contains("not pinned")),
            "{body}: {r:?}"
        );
    }
}
