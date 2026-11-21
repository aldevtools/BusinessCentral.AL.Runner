//! The `.bacpac` container (port of `Bacpac.cs`): an Open Packaging (zip) container
//! holding `model.xml` (the schema), `Origin.xml` (package metadata and a checksum over
//! model.xml), and one folder of native-BCP data streams per table with rows, named
//! `Data/<schema>.<url-encoded table>/TableData-NNN-MMMMM.BCP`.
//!
//! Structure derived by inspection of exports produced by sqlpackage 170.5.76 from SQL
//! Server 2022 and validated by importing the same file back into a SQL Server and
//! comparing full tables (bcdb PROVENANCE.md "bacpac container").

use crate::bcp::{BacpacColumn, BcpRowReader};
use crate::error::{Error, Result};
use crate::io::ByteSource;
use crate::model::ModelScan;
use crate::source::{BcSource, SourceTable};
use crate::types::SysColumn;
use crate::value::Value;
use crate::zip::ZipIndex;
use quick_xml::Reader;
use quick_xml::events::Event;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufReader, Read};
use std::sync::{Mutex, OnceLock};

/// The only Data stream format version whose row framing this reader has derived.
pub const SUPPORTED_DATA_STREAM_VERSION: &str = "2.0.0.0";

/// A table as model.xml declares it: its columns in data-stream order and its primary key.
#[derive(Debug, Clone)]
pub struct BacpacTable {
    pub schema: String,
    pub name: String,
    pub columns: Vec<BacpacColumn>,
    /// Primary-key columns in key order: the clustered key an `$ext` companion joins on.
    pub key_columns: Vec<String>,
    /// Data-stream entries in (batch, sequence) order; empty for a table with no rows.
    pub data_entries: Vec<String>,
}

struct Model {
    tables: Vec<BacpacTable>,
    source_tables: Vec<SourceTable>,
}

pub struct BacpacSource {
    src: Box<dyn ByteSource>,
    zip: ZipIndex,
    database_name: String,
    product_version: String,
    model_checksum: Option<String>,
    model: OnceLock<std::result::Result<Model, Error>>,
    row_counts: Mutex<HashMap<usize, i64>>,
}

fn xml_err(what: &str, e: impl std::fmt::Display) -> Error {
    crate::invalid!("{what}: {e}")
}

/// Local name and attributes of a start tag.
type StartTag = (String, Vec<(String, String)>);

fn start_tag(e: &quick_xml::events::BytesStart<'_>, what: &str) -> Result<StartTag> {
    let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
    let mut attrs = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|x| xml_err(what, x))?;
        let k = String::from_utf8_lossy(a.key.local_name().as_ref()).into_owned();
        let v = a.unescape_value().map_err(|x| xml_err(what, x))?.into_owned();
        attrs.push((k, v));
    }
    Ok((name, attrs))
}

/// A reader that hashes every byte that passes through it.
struct HashingRead<R> {
    inner: R,
    hasher: Sha256,
}

impl<R: Read> Read for HashingRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }
}

/// Elements of a small XML document: local name, attributes, first text content.
struct XmlEl {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
}

fn scan_small(xml: &[u8], what: &str) -> Result<Vec<XmlEl>> {
    let mut reader = Reader::from_reader(xml);
    let mut out: Vec<XmlEl> = Vec::new();
    let mut open: Vec<usize> = Vec::new();
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf).map_err(|e| xml_err(what, e))? {
            Event::Start(e) => {
                let (name, attrs) = start_tag(&e, what)?;
                out.push(XmlEl {
                    name,
                    attrs,
                    text: String::new(),
                });
                open.push(out.len() - 1);
            }
            Event::Empty(e) => {
                let (name, attrs) = start_tag(&e, what)?;
                out.push(XmlEl {
                    name,
                    attrs,
                    text: String::new(),
                });
            }
            Event::Text(t) => {
                if let Some(&i) = open.last() {
                    if out[i].text.is_empty() {
                        out[i].text = t.unescape().map_err(|e| xml_err(what, e))?.into_owned();
                    }
                }
            }
            Event::End(_) => {
                open.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(out)
}

fn unescape_uri(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = ((b[i + 1] as char).to_digit(16), (b[i + 2] as char).to_digit(16)) {
                out.push((h * 16 + l) as u8);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

impl BacpacSource {
    pub fn open(src: Box<dyn ByteSource>) -> Result<BacpacSource> {
        let zip = ZipIndex::parse(&*src)?;
        let origin = scan_small(&zip.read(&*src, "Origin.xml")?, "Origin.xml")?;
        let data_version = origin
            .iter()
            .find(|e| e.name == "Version" && e.attrs.iter().any(|(k, v)| k == "StreamName" && v == "Data"))
            .map(|e| e.text.trim().to_string());
        let Some(data_version) = data_version else {
            return Err(Error::InvalidData(
                "Origin.xml declares no Data stream version — refusing to guess the row format".into(),
            ));
        };
        if data_version != SUPPORTED_DATA_STREAM_VERSION {
            return Err(crate::invalid!(
                "Data stream version {data_version}, but only {SUPPORTED_DATA_STREAM_VERSION} has a derived row format — refusing to guess"
            ));
        }
        let model_checksum = origin
            .iter()
            .find(|e| e.name == "Checksum" && e.attrs.iter().any(|(k, v)| k == "Uri" && v == "/model.xml"))
            .map(|e| e.text.trim().to_string());
        let product_version = origin
            .iter()
            .find(|e| e.name == "ProductVersion")
            .map(|e| e.text.trim().to_string())
            .unwrap_or_else(|| "?".into());
        let database_name = scan_small(&zip.read(&*src, "DacMetadata.xml")?, "DacMetadata.xml")?
            .into_iter()
            .find(|e| e.name == "Name")
            .map(|e| e.text.trim().to_string())
            .unwrap_or_default();
        Ok(BacpacSource {
            src,
            zip,
            database_name,
            product_version,
            model_checksum,
            model: OnceLock::new(),
            row_counts: Mutex::new(HashMap::new()),
        })
    }

    fn model(&self) -> Result<&Model> {
        self.model
            .get_or_init(|| self.load_model())
            .as_ref()
            .map_err(|e| e.clone())
    }

    /// Streams model.xml once (113 MB in a real export, so never as a DOM), keeping only
    /// table and primary-key elements, and verifies Origin.xml's checksum over the same
    /// bytes as they pass. Called on first use, not at open, so the first query pays it.
    fn load_model(&self) -> Result<Model> {
        let entry = self
            .zip
            .find("model.xml")
            .ok_or_else(|| Error::InvalidData("the package has no model.xml — not a bacpac?".into()))?;
        let raw = self.zip.open(&*self.src, entry)?;
        let hashing = HashingRead {
            inner: raw,
            hasher: Sha256::new(),
        };
        let mut reader = Reader::from_reader(BufReader::with_capacity(256 * 1024, hashing));
        reader.config_mut().check_end_names = false;

        let ModelScan {
            tables: tables_cols,
            keys,
        } = crate::model::scan_streaming(&mut reader)?;
        // XmlReader may stop before the last byte; draining completes the hash and proves
        // the entry decompresses in full.
        let mut hashing = reader.into_inner().into_inner();
        std::io::copy(&mut hashing, &mut std::io::sink())?;
        let actual = hex_upper(&hashing.hasher.finalize());
        if let Some(declared) = &self.model_checksum {
            if !actual.eq_ignore_ascii_case(declared) {
                return Err(crate::invalid!(
                    "model.xml checksum {actual} does not match the {declared} Origin.xml declares — the package is damaged or was edited"
                ));
            }
        }

        let mut entries = self.data_entries_by_table()?;
        let mut tables: Vec<BacpacTable> = Vec::new();
        let mut seen: HashMap<String, ()> = HashMap::new();
        for (schema, name, cols) in tables_cols {
            let key = format!("{schema}.{name}");
            let display = if schema == "dbo" { name.clone() } else { key.clone() };
            let files = entries.remove(&key).unwrap_or_default();
            // Ordinal, and duplicates throw: a case-sensitive-collation database can hold two
            // tables that differ only in case, and silently keeping one of them would be a
            // table quietly missing from every listing.
            if seen.insert(display.clone(), ()).is_some() {
                return Err(crate::invalid!("model.xml declares '{display}' twice"));
            }
            tables.push(BacpacTable {
                schema,
                name,
                columns: cols,
                key_columns: keys.get(&key).cloned().unwrap_or_default(),
                data_entries: files,
            });
        }
        if let Some(first) = entries.keys().min() {
            // (zip order in bcdb; any one name identifies the mismatch)
            return Err(crate::invalid!(
                "data stream for '{first}' has no table in model.xml — schema/data mismatch, refusing to guess"
            ));
        }
        let source_tables = tables
            .iter()
            .enumerate()
            .map(|(i, t)| SourceTable {
                name: if t.schema == "dbo" {
                    t.name.clone()
                } else {
                    format!("{}.{}", t.schema, t.name)
                },
                compression: "-".into(), // a bacpac stores logical rows: no storage compression
                handle: i,
            })
            .collect();
        Ok(Model { tables, source_tables })
    }

    /// Data entries grouped by `<schema>.<table>` and ordered by (NNN, MMMMM). NNN is an
    /// export batch, MMMMM its continuation once a batch passes ~4 MiB; both boundaries
    /// fall between rows in every export measured, but concatenating in this order is
    /// correct either way.
    fn data_entries_by_table(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut by: HashMap<String, Vec<(i64, i64, String)>> = HashMap::new();
        for e in &self.zip.entries {
            let n = &e.name;
            let Some(rest) = n.strip_prefix("Data/") else { continue };
            let Some((folder, file)) = rest.split_once('/') else {
                return Err(crate::invalid!("unexpected data entry '{n}'"));
            };
            let Some(nums) = file.strip_prefix("TableData-").and_then(|f| f.strip_suffix(".BCP")) else {
                return Err(crate::invalid!(
                    "unexpected data entry '{n}' — expected TableData-NNN-MMMMM.BCP"
                ));
            };
            let parts: Vec<&str> = nums.split('-').collect();
            let parsed = if parts.len() == 2 {
                (parts[0].parse::<i64>().ok(), parts[1].parse::<i64>().ok())
            } else {
                (None, None)
            };
            let (Some(batch), Some(seq)) = parsed else {
                return Err(crate::invalid!(
                    "cannot read the batch/sequence numbers of data entry '{n}'"
                ));
            };
            let Some((schema, table)) = folder.split_once('.') else {
                return Err(crate::invalid!("data folder '{folder}' is not <schema>.<table>"));
            };
            let key = format!("{}.{}", unescape_uri(schema), unescape_uri(table));
            by.entry(key).or_default().push((batch, seq, n.clone()));
        }
        Ok(by
            .into_iter()
            .map(|(k, mut v)| {
                v.sort();
                (k, v.into_iter().map(|x| x.2).collect())
            })
            .collect())
    }
}

fn hex_upper(b: &[u8]) -> String {
    use std::fmt::Write;
    b.iter().fold(String::with_capacity(b.len() * 2), |mut s, x| {
        let _ = write!(s, "{x:02X}");
        s
    })
}

impl BcSource for BacpacSource {
    fn tables(&self) -> Result<&[SourceTable]> {
        Ok(&self.model()?.source_tables)
    }

    fn row_count(&self, t: &SourceTable) -> Result<i64> {
        if let Some(n) = self.row_counts.lock().unwrap().get(&t.handle) {
            return Ok(*n);
        }
        let mut n = 0i64;
        self.read_rows(t, &[], &mut |_| {
            n += 1;
            Ok(true)
        })?;
        self.row_counts.lock().unwrap().insert(t.handle, n);
        Ok(n)
    }

    fn columns(&self, t: &SourceTable) -> Result<Vec<SysColumn>> {
        Ok(self.model()?.tables[t.handle]
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| c.to_sys_column(i as i32 + 1))
            .collect())
    }

    /// The primary-key constraint model.xml declares.
    fn row_key_columns(&self, t: &SourceTable) -> Result<Vec<String>> {
        Ok(self.model()?.tables[t.handle].key_columns.clone())
    }

    fn read_rows(
        &self,
        t: &SourceTable,
        columns: &[SysColumn],
        sink: &mut dyn FnMut(&[Value]) -> Result<bool>,
    ) -> Result<()> {
        let table = &self.model()?.tables[t.handle];
        let want = columns
            .iter()
            .map(|c| {
                table
                    .columns
                    .iter()
                    .find(|bc| bc.name == c.name)
                    .cloned()
                    .ok_or_else(|| crate::invalid!("bacpac table {} has no column {}", table.name, c.name))
            })
            .collect::<Result<Vec<_>>>()?;
        let reader = BcpRowReader::new(&table.name, &table.columns);
        for entry in &table.data_entries {
            let e = self
                .zip
                .find(entry)
                .ok_or_else(|| crate::invalid!("the package has no {entry} — not a bacpac?"))?;
            let stream = self.zip.open(&*self.src, e)?;
            if !reader.read(stream, &want, sink)? {
                break;
            }
        }
        Ok(())
    }

    fn banner(&self) -> Result<String> {
        let m = self.model()?;
        let with_data = m.tables.iter().filter(|t| !t.data_entries.is_empty()).count();
        Ok(format!(
            "[bacpac \"{}\" exported by DacFx {}: {} tables in model.xml, {} with data]",
            self.database_name,
            self.product_version,
            m.tables.len(),
            with_data
        ))
    }
}
