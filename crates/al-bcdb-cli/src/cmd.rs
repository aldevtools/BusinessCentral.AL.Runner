//! The al-bcdb commands. Entry points: `run`.
use crate::opts;
use al_bcdb::bacpac::BacpacSource;
use al_bcdb::bak::BakSource;
use al_bcdb::io::ByteSource;
use al_bcdb::query::{self, ReadRequest, TableSelector, bc_tables, plan_read};
use al_bcdb::source::{BcSource, is_bacpac};
use al_bcdb::symbols::SymbolStore;
use al_bcdb::value::json_string;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// The part of the usage text bcdb prints, with what this port adds and drops.
const USAGE: &str = "usage:  <file> is a SQL Server backup (.bak) or a BC cloud export (.bacpac)
  al-bcdb tables <file> [--symbols <apps>]               list readable BC tables
  al-bcdb companies <file>                               list the companies in the database
  al-bcdb describe <file> --table <name> --symbols <apps>   AL schema of a table (field ids, AL types, SQL columns)
  al-bcdb check  <file.bak>                              cross-check the structural page map against page self-identification
  al-bcdb validate <file.bak> --against <restored.mdf>   byte-compare every mapped page against a restored copy
  al-bcdb read   <file> --table <name> [--company <c>] [--app <id-prefix>] [--top N] [--select \"A,B\"] [--merge-extensions] [--format tsv|json]
  al-bcdb serve  <file> [--symbols <apps>]               open once, answer JSON requests over stdin/stdout
  al-bcdb verify <file> --fixture <fixture.tsv> --table <name> --select \"A,B\"
  al-bcdb export <file> [--company <c>] [--out <dir>] [--format tsv|json] [--jobs N] [--all-tables]
                                                         read every table with rows, in parallel (new in this port;
                                                         without --out the rows are formatted and discarded)
  al-bcdb --version
restore is not ported: it needs a SQL Server as the destination.
An option the command does not accept fails the command instead of being ignored.
Table name may be the AL table name (e.g. \"No. Series\") or the raw SQL object name.
--symbols takes a comma-separated list of .app packages or SymbolReference.json files.";

enum Opened {
    Bak(Box<BakSource>),
    Bacpac(Box<BacpacSource>),
}

impl Opened {
    fn src(&self) -> &dyn BcSource {
        match self {
            Opened::Bak(b) => &**b,
            Opened::Bacpac(b) => &**b,
        }
    }
}

fn open_file(path: &str) -> Result<Opened, String> {
    let src = al_bcdb_std::FileSource::open(path)?;
    let mut head = [0u8; 4];
    if src.len() >= 4 {
        src.read_at(0, &mut head).map_err(|e| e.to_string())?;
    }
    if is_bacpac(&head) {
        BacpacSource::open(Box::new(src))
            .map(|b| Opened::Bacpac(Box::new(b)))
            .map_err(|e| e.to_string())
    } else {
        BakSource::open(Box::new(src))
            .map(|b| Opened::Bak(Box::new(b)))
            .map_err(|e| e.to_string())
    }
}

fn load_symbols(o: &HashMap<String, String>) -> Result<Option<SymbolStore>, String> {
    let Some(list) = o.get("symbols") else { return Ok(None) };
    let mut inputs = Vec::new();
    for p in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let bytes = std::fs::read(p).map_err(|_| format!("symbols file not found: {p}"))?;
        inputs.push((p.to_string(), bytes));
    }
    SymbolStore::load(inputs.iter().map(|(p, b)| (p.as_str(), b.clone())))
        .map(Some)
        .map_err(|e| e.to_string())
}

pub fn run(args: &[String]) -> Result<u8, String> {
    if args.len() == 1 && args[0] == "--version" {
        println!("al-bcdb {} (al-bcdb core, rust)", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    if args.len() < 2 || opts::command_keys(&args[0]).is_none() {
        eprintln!("{USAGE}");
        return Ok(64);
    }
    let cmd = args[0].as_str();
    let path = &args[1];
    if !std::path::Path::new(path).is_file() {
        eprintln!("error: file not found: {path}");
        return Ok(2);
    }
    let o = opts::parse(&args[2..], cmd)?;
    if cmd == "restore" {
        return Err("restore is not ported: it writes into a SQL Server, which this port does not talk to".into());
    }
    let opened = open_file(path)?;
    let src = opened.src();
    let stdout = std::io::stdout();
    let mut out = BufWriter::with_capacity(1 << 20, stdout.lock());
    let r = match cmd {
        "check" | "validate" => {
            let Opened::Bak(bak) = &opened else {
                eprintln!("error: {cmd} works on a .bak: a .bacpac has no page map to check");
                return Ok(2);
            };
            if cmd == "check" {
                check(bak, &mut out)
            } else {
                validate(bak, &o, &mut out)
            }
        }
        "tables" => tables(src, &o, &mut out),
        "companies" => companies(src, &mut out),
        "describe" => describe(src, &o, &mut out),
        "read" => read(src, &o, &mut out),
        "verify" => verify(src, &o),
        "serve" => {
            let sym = load_symbols(&o)?;
            src.tables().map_err(|e| e.to_string())?; // a bacpac parses model.xml here
            let stdin = std::io::stdin();
            al_bcdb::serve::serve(src, sym.as_ref(), &mut stdin.lock(), &mut out)
                .map(|_| 0)
                .map_err(|e| e.to_string())
        }
        "export" => export(src, &o),
        _ => unreachable!(),
    };
    match out.flush() {
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => return Ok(0),
        Err(e) => return Err(e.to_string()),
        Ok(()) => {}
    }
    r
}

fn io(e: std::io::Error) -> String {
    e.to_string()
}

fn pad(s: &str, width: usize) -> String {
    let n = s.encode_utf16().count();
    format!("{s}{}", " ".repeat(width.saturating_sub(n)))
}

fn lpad(s: &str, width: usize) -> String {
    let n = s.encode_utf16().count();
    format!("{}{s}", " ".repeat(width.saturating_sub(n)))
}

fn selector(o: &HashMap<String, String>) -> TableSelector {
    TableSelector {
        table: o.get("table").cloned(),
        company: o.get("company").cloned(),
        app: o.get("app").cloned(),
    }
}

fn read_request(o: &HashMap<String, String>) -> ReadRequest {
    ReadRequest {
        sel: selector(o),
        top: o.get("top").cloned(),
        select: o.get("select").cloned(),
        sha256: o.get("sha256").cloned(),
        merge_extensions: o.contains_key("merge-extensions"),
    }
}

fn check(bak: &BakSource, out: &mut dyn Write) -> Result<u8, String> {
    let pf = bak.page_file();
    let cc = pf.cross_check().map_err(|e| e.to_string())?;
    let w = |out: &mut dyn Write, s: String| writeln!(out, "{s}").map_err(io);
    w(out, format!("data regions:        {}", pf.mtf().mqda_regions.len()))?;
    w(out, format!("GAM intervals:       {}", pf.gam_interval_count()))?;
    w(out, format!("mapped pages:        {}", pf.page_count()))?;
    w(out, format!("superseded by re-read: {}", pf.superseded_page_count()))?;
    w(
        out,
        format!("log stream bytes (not replayed): {}", pf.mtf().log_stream_bytes),
    )?;
    w(out, format!("self-id agreeing blocks:  {}", cc.agree))?;
    w(out, format!("stale self-id headers:    {}", cc.stale_headers))?;
    w(out, format!("unidentifiable blocks:    {}", cc.unidentified))?;
    w(
        out,
        format!(
            "pages where last-image-wins would differ: {}",
            cc.last_wins_disagreements.len()
        ),
    )?;
    for p in cc.last_wins_disagreements.iter().take(50) {
        w(out, format!("  page 1:{p}"))?;
    }
    Ok(0)
}

fn validate(bak: &BakSource, o: &HashMap<String, String>, out: &mut dyn Write) -> Result<u8, String> {
    let mdf = o.get("against").ok_or("--against <restored.mdf> is required")?;
    let (exact, hdr, body) = bak
        .page_file()
        .compare_against(&al_bcdb_std::FileSource::open(mdf)?)
        .map_err(|e| e.to_string())?;
    let w = |out: &mut dyn Write, s: String| writeln!(out, "{s}").map_err(io);
    w(out, format!("mapped pages:     {}", bak.page_file().page_count()))?;
    w(out, format!("byte-identical:   {exact}"))?;
    w(out, format!("header-only diff: {hdr}"))?;
    w(out, format!("body diff:        {}", body.len()))?;
    for pid in body.iter().take(2000) {
        w(out, format!("  page 1:{pid}"))?;
    }
    if body.len() > 2000 {
        w(out, format!("  ... {} more", body.len() - 2000))?;
    }
    Ok(if body.is_empty() { 0 } else { 1 })
}

fn tables(src: &dyn BcSource, o: &HashMap<String, String>, out: &mut dyn Write) -> Result<u8, String> {
    let sym = load_symbols(o)?;
    let mut ts = bc_tables(src).map_err(|e| e.to_string())?;
    query::sort_tables(&mut ts);
    for t in &ts {
        let al = sym
            .as_ref()
            .map(|s| s.find_for_sql_table(query::strip_ext(&t.table_name), t.app_id.as_deref()));
        let alcol = match al {
            None => String::new(),
            Some(None) => "\t-".to_string(),
            Some(Some(a)) => format!("\t{} \"{}\" ({})", a.id, a.name, a.app_name),
        };
        let rows = src.row_count(&t.table).map_err(|e| e.to_string())?;
        writeln!(
            out,
            "{}  {}  {}\t{}{}",
            lpad(&rows.to_string(), 8),
            pad(&t.table.compression, 4),
            t.company.as_deref().unwrap_or("-"),
            t.table_name,
            alcol
        )
        .map_err(io)?;
    }
    eprintln!("{}", src.banner().map_err(|e| e.to_string())?);
    Ok(0)
}

fn companies(src: &dyn BcSource, out: &mut dyn Write) -> Result<u8, String> {
    for c in query::companies(src).map_err(|e| e.to_string())? {
        writeln!(out, "{c}").map_err(io)?;
    }
    Ok(0)
}

fn describe(src: &dyn BcSource, o: &HashMap<String, String>, out: &mut dyn Write) -> Result<u8, String> {
    let sym = load_symbols(o)?.ok_or("describe requires --symbols (a .app package or SymbolReference.json)")?;
    let d = query::describe(src, &sym, &selector(o)).map_err(|e| e.to_string())?;
    let mut w = |s: String| writeln!(out, "{s}").map_err(io);
    w(format!(
        "Table {} \"{}\" — app \"{}\" ({})",
        d.al.id, d.al.name, d.al.app_name, d.al.app_id
    ))?;
    w(format!("SQL object: {}", d.sql_object))?;
    w(format!(
        "{}  {} {} {} SQL type",
        lpad("Id", 6),
        pad("AL name", 40),
        pad("AL type", 28),
        pad("SQL column", 40)
    ))?;
    let row = |id: &str, name: &str, ty: &str, col: &str, tail: &str| {
        format!(
            "{}  {} {} {} {}",
            lpad(id, 6),
            pad(name, 40),
            pad(ty, 28),
            pad(col, 40),
            tail
        )
    };
    for f in &d.fields {
        let id = f.field.id.to_string();
        if f.field.field_class != "Normal" {
            w(row(
                &id,
                &f.field.name,
                &f.field.type_name,
                "-",
                &format!("({}: computed, not stored)", f.field.field_class),
            ))?;
        } else if let (Some(c), Some(t)) = (&f.sql_column, &f.sql_type) {
            w(row(&id, &f.field.name, &f.field.type_name, c, t))?;
        } else {
            w(row(
                &id,
                &f.field.name,
                &f.field.type_name,
                "-",
                "(no SQL column — removed or disabled)",
            ))?;
        }
    }
    for e in &d.ext_fields {
        match (&e.field, &e.ext_name) {
            (Some(f), Some(en)) => w(row(
                &f.id.to_string(),
                &f.name,
                &f.type_name,
                &e.column,
                &format!(
                    "{} (tableextension \"{}\", {})",
                    e.sql_type,
                    en,
                    e.ext_app_name.as_deref().unwrap_or("")
                ),
            ))?,
            _ => w(row(
                "-",
                "-",
                "-",
                &e.column,
                &format!(
                    "{} (extension field of app {} — not in the provided symbols)",
                    e.sql_type, e.ext_app_id
                ),
            ))?,
        }
    }
    if let Some(c) = &d.companion {
        w(format!(
            "Companion:  {c} (extension fields; read together with --merge-extensions)"
        ))?;
    }
    for (name, ty) in &d.system_columns {
        w(row("-", "-", "-", name, &format!("{ty} (system column)")))?;
    }
    Ok(0)
}

/// Rows as tsv: header then one `|`-joined line per row.
fn write_tsv(plan: &query::ReadPlan, src: &dyn BcSource, out: &mut dyn Write) -> Result<(), String> {
    out.write_all(plan.headers.join("|").as_bytes()).map_err(io)?;
    out.write_all(b"\n").map_err(io)?;
    let mut line = String::with_capacity(512);
    let mut err = None;
    let r = plan.run(src, &mut |row| {
        line.clear();
        for (i, v) in row.iter().enumerate() {
            if i > 0 {
                line.push('|');
            }
            v.write_tsv(&mut line);
        }
        line.push('\n');
        match out.write_all(line.as_bytes()) {
            Ok(()) => Ok(true),
            Err(e) => {
                err = Some(e);
                Ok(false)
            }
        }
    });
    if let Some(e) = err {
        return Err(e.to_string());
    }
    r.map_err(|e| e.to_string())
}

fn write_json_rows(plan: &query::ReadPlan, src: &dyn BcSource, out: &mut dyn Write) -> Result<(), String> {
    out.write_all(b"[\n").map_err(io)?;
    let mut heads: Vec<String> = Vec::new();
    for h in &plan.headers {
        let mut s = String::new();
        json_string(h, &mut s);
        s.push_str(": ");
        heads.push(s);
    }
    let mut first = true;
    let mut line = String::with_capacity(512);
    let mut err = None;
    let r = plan.run(src, &mut |row| {
        line.clear();
        if !first {
            line.push_str(",\n");
        }
        first = false;
        line.push_str("  {");
        for (i, v) in row.iter().enumerate() {
            if i > 0 {
                line.push_str(", ");
            }
            line.push_str(&heads[i]);
            v.write_json(&mut line);
        }
        line.push('}');
        match out.write_all(line.as_bytes()) {
            Ok(()) => Ok(true),
            Err(e) => {
                err = Some(e);
                Ok(false)
            }
        }
    });
    if let Some(e) = err {
        return Err(e.to_string());
    }
    r.map_err(|e| e.to_string())?;
    out.write_all(if first { b"]\n" } else { b"\n]\n" }).map_err(io)?;
    Ok(())
}

fn read(src: &dyn BcSource, o: &HashMap<String, String>, out: &mut dyn Write) -> Result<u8, String> {
    let sym = load_symbols(o)?;
    let plan = plan_read(src, sym.as_ref(), &read_request(o)).map_err(|e| e.to_string())?;
    if let Some(n) = &plan.note {
        eprintln!("{n}");
    }
    // All or nothing, as bcdb: it decodes every row before printing the first, so a row
    // that cannot be decoded leaves stdout empty rather than holding a prefix of the answer.
    let mut buf: Vec<u8> = Vec::with_capacity(1 << 16);
    if o.get("format").is_some_and(|f| f == "json") {
        write_json_rows(&plan, src, &mut buf)?;
    } else {
        write_tsv(&plan, src, &mut buf)?;
    }
    out.write_all(&buf).map_err(io)?;
    Ok(0)
}

/// Compare decoded rows against a fixture exported from a restored SQL Server (the
/// oracle). Order-insensitive.
fn verify(src: &dyn BcSource, o: &HashMap<String, String>) -> Result<u8, String> {
    let fix = o.get("fixture").ok_or("--fixture is required")?;
    // Fixture lines may end with "|#" (a sentinel the oracle export appends so that trailing
    // spaces inside the last real column survive).
    let text = std::fs::read_to_string(fix).map_err(|e| format!("{fix}: {e}"))?;
    let mut expected: Vec<String> = text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.strip_suffix("|#").unwrap_or(l).to_string())
        .collect();
    expected.sort();
    let sym = load_symbols(o)?;
    let plan = plan_read(src, sym.as_ref(), &read_request(o)).map_err(|e| e.to_string())?;
    let mut actual: Vec<String> = Vec::new();
    plan.run(src, &mut |row| {
        actual.push(row.iter().map(|v| v.to_tsv()).collect::<Vec<_>>().join("|"));
        Ok(true)
    })
    .map_err(|e| e.to_string())?;
    actual.sort();
    if expected == actual {
        let name = std::path::Path::new(fix)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        println!("OK: {} rows match fixture {name}", actual.len());
        return Ok(0);
    }
    eprintln!("FAIL: {} decoded rows vs {} fixture rows", actual.len(), expected.len());
    for m in expected.iter().filter(|e| !actual.contains(e)).take(5) {
        eprintln!("  missing: {m}");
    }
    for m in actual.iter().filter(|a| !expected.contains(a)).take(5) {
        eprintln!("  extra:   {m}");
    }
    Ok(1)
}

/// Read every table that has rows, in parallel. Not in bcdb: the whole-file read the
/// performance comparison measures, and the shape a store loader needs.
fn export(src: &dyn BcSource, o: &HashMap<String, String>) -> Result<u8, String> {
    let started = std::time::Instant::now();
    let sym = load_symbols(o)?;
    let all = bc_tables(src).map_err(|e| e.to_string())?;
    let company = o.get("company");
    let json = o.get("format").is_some_and(|f| f == "json");
    let out_dir = o.get("out").map(std::path::PathBuf::from);
    if let Some(d) = &out_dir {
        std::fs::create_dir_all(d).map_err(io)?;
    }
    let jobs: usize = match o.get("jobs") {
        Some(j) => j.parse().map_err(|_| format!("--jobs expects a number, got '{j}'"))?,
        None => 0,
    };
    let mut todo = Vec::new();
    for t in &all {
        let wanted = match (&t.company, company) {
            (Some(c), Some(want)) => c.to_uppercase().starts_with(&want.to_uppercase()),
            (None, _) => true,
            (Some(_), None) => true,
        };
        if wanted && src.row_count(&t.table).map_err(|e| e.to_string())? > 0 {
            todo.push(t);
        }
    }
    let rows_total = Arc::new(AtomicU64::new(0));
    let bytes_total = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(AtomicU64::new(0));
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build()
        .map_err(|e| e.to_string())?;
    pool.install(|| {
        todo.par_iter().for_each(|t| {
            let req = ReadRequest::default();
            let result = (|| -> Result<(u64, u64), String> {
                let plan = query::plan_read_table(src, sym.as_ref(), t, &req).map_err(|e| e.to_string())?;
                let mut counter = CountingWriter;
                let mut file;
                let sink: &mut dyn Write = match &out_dir {
                    Some(d) => {
                        let safe: String = t
                            .sql_name()
                            .chars()
                            .map(|c| {
                                if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                                    c
                                } else {
                                    '_'
                                }
                            })
                            .collect();
                        let p = d.join(format!("{safe}.{}", if json { "json" } else { "tsv" }));
                        file = BufWriter::with_capacity(1 << 20, File::create(p).map_err(io)?);
                        &mut file
                    }
                    None => &mut counter,
                };
                let mut sink = CountingSink { inner: sink, bytes: 0 };
                if json {
                    write_json_rows(&plan, src, &mut sink)?
                } else {
                    write_tsv(&plan, src, &mut sink)?
                }
                sink.flush().map_err(io)?;
                Ok((src.row_count(&t.table).map_err(|e| e.to_string())? as u64, sink.bytes))
            })();
            match result {
                Ok((rows, bytes)) => {
                    rows_total.fetch_add(rows, Ordering::Relaxed);
                    bytes_total.fetch_add(bytes, Ordering::Relaxed);
                }
                Err(e) => {
                    failed.fetch_add(1, Ordering::Relaxed);
                    eprintln!("{}: {e}", t.sql_name());
                }
            }
        });
    });
    eprintln!(
        "exported {} tables ({} rows by catalog count, {} bytes), {} failed, in {:.3} s",
        todo.len() as u64 - failed.load(Ordering::Relaxed),
        rows_total.load(Ordering::Relaxed),
        bytes_total.load(Ordering::Relaxed),
        failed.load(Ordering::Relaxed),
        started.elapsed().as_secs_f64()
    );
    Ok(if failed.load(Ordering::Relaxed) == 0 { 0 } else { 1 })
}

struct CountingWriter;

impl Write for CountingWriter {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct CountingSink<'a> {
    inner: &'a mut dyn Write,
    bytes: u64,
}

impl Write for CountingSink<'_> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(b)?;
        self.bytes += n as u64;
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}
