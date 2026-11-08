//! `al-runner test --profile PATH [--profile-mode instrumentation|sampling] [--profile-interval US]` and the serve
//! field `profile` (`{"path": ..., "mode": ..., "intervalUs": ...}`): starts al-testrun's profiler before the run,
//! then writes the `.alcpuprofile` files and the summary, with `file://` links to the sources of the APPs given as
//! folders. Manual: `docs/manual/profiling.md`.

use al_testrun::profiling::{self, Mode, Node, Options};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

/// What a run asks of the profiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// A folder (one file per test codeunit and `summary.txt`) or a file ending in `.alcpuprofile` (one merged file).
    pub path: PathBuf,
    pub mode: Mode,
    pub interval_us: u64,
}

/// What the parser collects before it can build a [`Request`] (`--profile-mode` may come before `--profile`).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Flags {
    pub path: Option<PathBuf>,
    pub mode: Option<String>,
    pub interval: Option<String>,
}

impl Flags {
    /// Takes `flag` with its value when it is a profile flag; `false` otherwise.
    pub fn take(&mut self, flag: &str, value: impl FnOnce() -> Result<String, String>) -> Result<bool, String> {
        match flag {
            "--profile" => self.path = Some(value()?.into()),
            "--profile-mode" => self.mode = Some(value()?),
            "--profile-interval" => self.interval = Some(value()?),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// The request, or why the flags do not make one.
    pub fn request(&self) -> Result<Option<Request>, String> {
        let Some(path) = &self.path else {
            if self.mode.is_some() {
                return Err("--profile-mode needs --profile PATH".into());
            }
            if self.interval.is_some() {
                return Err("--profile-interval needs --profile PATH".into());
            }
            return Ok(None);
        };
        let mode = match &self.mode {
            Some(m) => Mode::parse(m).map_err(|e| format!("--profile-mode: {e}"))?,
            None => Mode::Instrumentation,
        };
        let interval_us = match &self.interval {
            Some(v) => v
                .parse::<u64>()
                .ok()
                .filter(|&n| n >= 50)
                .ok_or_else(|| format!("--profile-interval needs microseconds (50 or more), got `{v}`"))?,
            None => 1000,
        };
        Ok(Some(Request {
            path: path.clone(),
            mode,
            interval_us,
        }))
    }
}

/// The serve field `profile`.
pub fn from_json(v: &serde_json::Value) -> Result<Option<Request>, String> {
    if v.is_null() {
        return Ok(None);
    }
    let s = |k: &str| v.get(k).and_then(serde_json::Value::as_str).map(str::to_string);
    let path = s("path").ok_or("`profile` needs `path` (a folder, or a file ending in .alcpuprofile)")?;
    Flags {
        path: Some(path.into()),
        mode: s("mode"),
        interval: v.get("intervalUs").map(|n| n.to_string().trim_matches('"').to_string()),
    }
    .request()
    .map_err(|e| {
        e.replace("--profile-mode", "`profile.mode`")
            .replace("--profile-interval", "`profile.intervalUs`")
    })
}

thread_local! {
    static WRITTEN: RefCell<Option<Vec<PathBuf>>> = const { RefCell::new(None) };
}

/// Starts the profiler for a run.
pub fn begin(r: &Request) {
    profiling::start(Options {
        mode: r.mode,
        interval_us: r.interval_us,
        statement_times: true,
    });
}

/// Writes the profile of the run that [`begin`] started; `apps` are the APPs of the command line (their folders give
/// the source links). Prints where the files went.
pub fn end(r: &Request, apps: &[PathBuf]) {
    let Some(c) = profiling::finish() else { return };
    let roots = app_roots(apps);
    let url_of = |n: &Node| -> Option<String> {
        let dir = roots
            .iter()
            .find(|(id, _)| id.eq_ignore_ascii_case(&n.app.id))?
            .1
            .as_path();
        let file = n.file.as_ref()?;
        Some(file_url(&dir.join(file.replace('\\', "/"))))
    };
    match profiling::write::write(&r.path, &c, &url_of) {
        Ok(files) => {
            eprintln!(
                "profile ({}): {} file(s) in {}",
                r.mode.name(),
                files.len(),
                r.path.display()
            );
            WRITTEN.with(|w| *w.borrow_mut() = Some(files));
        }
        Err(e) => eprintln!("al-runner: profile not written: {e}"),
    }
}

/// The files the last [`end`] of this thread wrote (the serve response lists them).
pub fn take_written() -> Option<Vec<PathBuf>> {
    WRITTEN.with(|w| w.borrow_mut().take())
}

/// (app id, folder) of each APP given as a folder with an `app.json`.
fn app_roots(apps: &[PathBuf]) -> Vec<(String, PathBuf)> {
    apps.iter()
        .filter(|p| p.is_dir())
        .filter_map(|p| {
            let text = std::fs::read_to_string(p.join("app.json")).ok()?;
            let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
            let id = v.get("id")?.as_str()?.to_string();
            Some((id, std::fs::canonicalize(p).unwrap_or_else(|_| p.clone())))
        })
        .collect()
}

/// `file:///abs/path` with forward slashes; spaces and `#`/`%`/`?` encoded.
fn file_url(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !s.starts_with('/') {
        out.push('/');
    }
    for c in s.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '%' => out.push_str("%25"),
            '?' => out.push_str("%3F"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_make_a_request() {
        let mut f = Flags::default();
        assert!(f.take("--profile-mode", || Ok("sampling".into())).unwrap());
        assert!(f.take("--profile", || Ok("out".into())).unwrap());
        assert!(!f.take("--coverage", || unreachable!()).unwrap());
        let r = f.request().unwrap().unwrap();
        assert_eq!(r.mode, Mode::Sampling);
        assert_eq!(r.interval_us, 1000);
        assert_eq!(Flags::default().request(), Ok(None));
        let bad = Flags {
            path: Some("x".into()),
            interval: Some("10".into()),
            ..Flags::default()
        };
        assert!(bad.request().unwrap_err().contains("50 or more"));
    }

    #[test]
    fn serve_field() {
        let r = from_json(&serde_json::json!({"path": "p.alcpuprofile", "mode": "sampling", "intervalUs": 200}))
            .unwrap()
            .unwrap();
        assert_eq!((r.mode, r.interval_us), (Mode::Sampling, 200));
        assert!(from_json(&serde_json::json!({"mode": "sampling"})).is_err());
        assert!(
            from_json(&serde_json::json!({"path": "p", "mode": "x"}))
                .unwrap_err()
                .contains("`profile.mode`")
        );
        assert_eq!(from_json(&serde_json::Value::Null), Ok(None));
    }

    #[test]
    fn file_urls() {
        assert_eq!(file_url(Path::new("/a b/c#.al")), "file:///a%20b/c%23.al");
        assert_eq!(file_url(Path::new("C:\\x\\y.al")), "file:///C:/x/y.al");
    }
}
