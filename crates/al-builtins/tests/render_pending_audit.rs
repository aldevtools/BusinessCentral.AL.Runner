//! ADR-040, "How the guard is proven complete", item 3: every builtin that takes or gives a stream, a blob or a file
//! is listed here with what it does to the output of a report that was saved in a rendered format and never produced.
//! A new builtin of that kind that is not in the table fails the test, so the list of observation paths cannot go
//! stale as builtins are added. The behaviour itself is tested in `al-vm/tests/reporting/report_no_output.rs`.
//!
//! The kinds:
//! - `Guard`: looks at the bytes, the size, the presence or the file. A pending target is refused (the mark is checked
//!   in the stream, the blob and the file system layers, so one rule covers every builtin that reads through them).
//! - `Produces`: writes data. It does not look at a pending target; creating the stream again, or a write by AL over a
//!   path, ends the mark.
//! - `Pure`: names or flags only; no content.
//! - `Client`: implemented as a refusal, because it needs a browser on the other side.
//! - `Refused`: not implemented, so a run that reaches it is refused anyway. The test checks that it stays so: when one
//!   of these is implemented it has to move to another kind, with its test.

use al_builtins::catalog::Catalog;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Guard,
    Produces,
    Pure,
    Refused,
    /// Implemented as a refusal: it needs a client (a browser round trip) and a headless run has none.
    Client,
}
use Kind::*;

/// `Receiver.Method` and what it does to a pending target.
const TABLE: &[(&str, Kind)] = &[
    ("BigText.Read", Guard),
    ("BigText.Write", Produces),
    ("Blob.CreateInStream", Guard),
    ("Blob.CreateOutStream", Produces),
    ("Blob.Export", Refused),
    ("Blob.HasValue", Guard),
    ("Blob.Import", Guard),
    ("Blob.Length", Guard),
    ("File.Close", Produces),
    ("File.Copy", Guard),
    ("File.Create", Produces),
    ("File.CreateInStream", Guard),
    ("File.CreateOutStream", Produces),
    ("File.CreateTempFile", Produces),
    ("File.Download", Guard),
    ("File.DownloadFromStream", Guard),
    ("File.Erase", Guard),
    ("File.Exists", Guard),
    ("File.GetStamp", Refused),
    ("File.IsPathTemporary", Pure),
    ("File.Len", Guard),
    ("File.Name", Pure),
    ("File.Open", Guard),
    ("File.Pos", Refused),
    ("File.Read", Refused),
    ("File.Rename", Guard),
    ("File.Seek", Refused),
    ("File.SetStamp", Refused),
    ("File.TextMode", Pure),
    ("File.Trunc", Refused),
    ("File.Upload", Client),
    ("File.UploadIntoStream", Client),
    ("File.View", Guard),
    ("File.ViewFromStream", Guard),
    ("File.Write", Produces),
    ("File.WriteMode", Pure),
    ("FileUpload.CreateInStream", Pure),
    ("HttpContent.ReadAs", Pure),
    ("HttpContent.WriteFrom", Guard),
    ("InStream.EOS", Guard),
    ("InStream.Length", Guard),
    ("InStream.Position", Guard),
    ("InStream.Read", Guard),
    ("InStream.ReadText", Guard),
    ("InStream.ResetPosition", Guard),
    ("JsonArray.ReadFrom", Guard),
    ("JsonArray.WriteTo", Produces),
    ("JsonObject.ReadFrom", Guard),
    ("JsonObject.ReadFromYaml", Guard),
    ("JsonObject.WriteTo", Produces),
    ("JsonObject.WriteToYaml", Produces),
    ("JsonToken.ReadFrom", Guard),
    ("JsonToken.WriteTo", Produces),
    ("JsonValue.ReadFrom", Guard),
    ("JsonValue.WriteTo", Produces),
    ("Media.ExportFile", Refused),
    ("Media.ExportStream", Produces),
    ("Media.FindOrphans", Refused),
    ("Media.HasValue", Pure),
    ("Media.ImportFile", Guard),
    ("Media.ImportStream", Guard),
    ("Media.MediaId", Pure),
    ("MediaSet.Count", Pure),
    ("MediaSet.ExportFile", Refused),
    ("MediaSet.FindOrphans", Refused),
    ("MediaSet.ImportFile", Guard),
    ("MediaSet.ImportStream", Guard),
    ("MediaSet.Insert", Refused),
    ("MediaSet.Item", Pure),
    ("MediaSet.MediaId", Pure),
    ("MediaSet.Remove", Pure),
    ("NavApp.GetResource", Pure),
    ("OutStream.Write", Produces),
    ("OutStream.WriteText", Produces),
    ("Query.SaveAsCsv", Produces),
    ("Query.SaveAsJson", Produces),
    ("Query.SaveAsXml", Produces),
    ("Report.ExcelLayout", Refused),
    ("Report.RDLCLayout", Refused),
    ("Report.RdlcLayout", Refused),
    ("Report.SaveAs", Produces),
    ("Report.ValidateAndPrepareLayout", Refused),
    ("Report.WordLayout", Refused),
    ("System.CopyStream", Guard),
    ("System.ImportStreamWithUrlAccess", Refused),
    ("XmlAttribute.WriteTo", Produces),
    ("XmlCData.WriteTo", Produces),
    ("XmlComment.WriteTo", Produces),
    ("XmlDeclaration.WriteTo", Produces),
    ("XmlDocument.ReadFrom", Guard),
    ("XmlDocument.WriteTo", Produces),
    ("XmlDocumentType.WriteTo", Produces),
    ("XmlElement.WriteTo", Produces),
    ("XmlNode.WriteTo", Produces),
    ("XmlProcessingInstruction.WriteTo", Produces),
    ("XmlText.WriteTo", Produces),
    ("Xmlport.Export", Produces),
    ("Xmlport.Import", Guard),
    ("Xmlport.SetDestination", Produces),
    ("Xmlport.SetSource", Guard),
];

/// Receivers that hold or give a stream, a blob or a file, and the parameter types that take one.
const RECEIVERS: &[&str] = &["Blob", "InStream", "OutStream", "File", "TempBlob", "Media", "MediaSet"];
const PARAM_TYPES: &[&str] = &["InStream", "OutStream", "Blob", "File"];

fn touches_data(s: &al_builtins::signature::Signature) -> bool {
    RECEIVERS.contains(&s.receiver.as_str()) || s.params.iter().any(|p| PARAM_TYPES.contains(&p.ty.trim()))
}

#[test]
fn every_builtin_that_takes_or_gives_a_stream_blob_or_file_is_classified() {
    let found: BTreeSet<String> = Catalog::load()
        .signatures
        .iter()
        .filter(|s| touches_data(s))
        .map(|s| format!("{}.{}", s.receiver, s.name))
        .collect();
    let table: BTreeMap<&str, Kind> = TABLE.iter().copied().collect();
    assert_eq!(table.len(), TABLE.len(), "a builtin is listed twice");
    let missing: Vec<&String> = found.iter().filter(|k| !table.contains_key(k.as_str())).collect();
    assert!(
        missing.is_empty(),
        "these builtins take or give a stream, a blob or a file and are not classified in render_pending_audit.rs \
         (decide whether they look at a pending output, and test it): {missing:?}"
    );
    let stale: Vec<&&str> = table.keys().filter(|k| !found.contains(**k)).collect();
    assert!(stale.is_empty(), "classified but no longer in the catalog: {stale:?}");
}

#[test]
fn a_refused_builtin_stays_refused_and_every_other_kind_is_implemented() {
    let registered = al_builtins::native::registered_ids();
    let implemented = |key: &str| registered.iter().any(|id| id.starts_with(&format!("{key}#")));
    let mut wrong = Vec::new();
    for (key, kind) in TABLE {
        match (kind, implemented(key)) {
            (Refused, true) => wrong.push(format!(
                "{key} is implemented now: classify it (Guard, Produces or Pure) and test it"
            )),
            // Not every overload has an implementation of its own (the VM runs some); at least one has.
            (Guard | Produces | Pure | Client, false) => wrong.push(format!(
                "{key} is classified {kind:?} but has no implementation: Refused?"
            )),
            _ => {}
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
