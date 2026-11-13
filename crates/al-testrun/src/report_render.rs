//! The report render process (ADR-040 amendment 2026-11-13b): the .NET helper started with `--render`, one per run,
//! shared by every session, calls one at a time. It implements al-host's [`ReportRenderer`] for front ends.
//!
//! The process is started on the first call that needs it, never at run start. Its pieces (probe folders, the Linux
//! drawing pair and Skia native) are made present by the run's [`PrepareFn`] then; the first `render_probe` answer is
//! kept for the run. A process that dies is started again for the next call: [`MAX_STARTS`] starts a run (2 restarts).
//!
//! Requests and replies use the frames of the session sidecar (`crate::dotnet`); bytes travel as Base64.

use crate::dotnet::{
    PROTOCOL, Proc, SidecarConfig, base64, check_hello, resolve, spawn_helper, unbase64, wire_probe_dirs,
};
use al_deps::render_set::{RenderSet, set_files};
use al_host::render::{RenderFailure, RenderNeed, RenderUnavailable, ReportRenderer, WordMerge};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How often a run starts the render process at most (the first start and the restarts after a crash).
pub const MAX_STARTS: usize = 3;

/// What the render process needs on disk, made present by the run on first use.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderPieces {
    /// Folders the BC assemblies are found in (the service folder of ADR-014 section 3; its `Add-ins` is searched too).
    pub dirs: Vec<PathBuf>,
    /// Linux: `System.Drawing.Common.dll` and `Microsoft.Win32.SystemEvents.dll` 6.0.0.
    pub drawing: Vec<PathBuf>,
    /// Linux: the `libSkiaSharp.so` of the build's SkiaSharp version.
    pub skia: Option<PathBuf>,
    /// What could not be fetched, one line each (reported once on stderr; the probe then names the missing piece).
    pub notes: Vec<String>,
}

/// Fetches or finds the pieces; called once, on the first call that needs the process.
pub type PrepareFn = dyn Fn() -> RenderPieces + Send + Sync;

/// Why a request gave no value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CallError {
    /// The process answered with a refusal or BC's exception.
    Failed(RenderFailure),
    /// The process is gone (it ended, or gave no answer in time and was stopped); the text says which.
    Gone(String),
}

/// One request, one reply (internal; a fake in the tests).
pub(crate) trait Transport: Send {
    /// The reply's `value`, or why there is none.
    fn request(&mut self, req: Value) -> Result<Value, CallError>;
}

/// The `render_probe` answer per need.
pub type ProbeAnswer = HashMap<RenderNeed, Result<(), RenderUnavailable>>;

type StartFn = dyn Fn() -> Result<Box<dyn Transport>, String> + Send + Sync;

struct State {
    proc: Option<Box<dyn Transport>>,
    starts: usize,
    pieces: Option<RenderPieces>,
    /// The first `render_probe` answer per need, or why no process could answer it.
    probe: Option<Result<ProbeAnswer, String>>,
}

/// The render host of a run.
pub struct RenderHost {
    start: Box<StartFn>,
    prepare: Box<PrepareFn>,
    state: Mutex<State>,
}

impl RenderHost {
    /// A host that starts the helper of `cfg` (its .NET, helper path, BC major and call deadline) with `--render`.
    pub fn new(cfg: SidecarConfig, prepare: Box<PrepareFn>) -> RenderHost {
        let cfg = Arc::new(cfg);
        let start = move || -> Result<Box<dyn Transport>, String> { Ok(Box::new(ProcTransport::start(&cfg)?)) };
        RenderHost::with_transport(Box::new(start), prepare)
    }

    pub(crate) fn with_transport(start: Box<StartFn>, prepare: Box<PrepareFn>) -> RenderHost {
        RenderHost {
            start,
            prepare,
            state: Mutex::new(State {
                proc: None,
                starts: 0,
                pieces: None,
                probe: None,
            }),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A running, configured process; starts one (and sends it the pieces) when none runs.
    fn running<'a>(&self, st: &'a mut State) -> Result<&'a mut Box<dyn Transport>, String> {
        if st.proc.is_none() {
            if let Some(why) = st.used_up() {
                return Err(why);
            }
            st.starts += 1;
            let mut t = (self.start)()?;
            let pieces = st.pieces.get_or_insert_with(|| {
                let p = (self.prepare)();
                for n in &p.notes {
                    eprintln!("warning: report rendering: {n}");
                }
                p
            });
            let answer = t.request(probe_request(pieces)).map_err(|f| match f {
                CallError::Failed(f) => f.to_string(),
                CallError::Gone(why) => format!("{why} (render_probe)"),
            })?;
            if st.probe.is_none() {
                st.probe = Some(parse_probe(&answer));
            }
            st.proc = Some(t);
        }
        Ok(st.proc.as_mut().expect("just set"))
    }

    /// The `render_probe` answer of this run, per need: what is available and the first missing piece of the rest.
    pub fn probe(&self) -> Result<ProbeAnswer, String> {
        let mut st = self.lock();
        if st.probe.is_none()
            && let Err(why) = self.running(&mut st)
        {
            st.probe = Some(Err(why));
        }
        st.probe.clone().expect("set above")
    }

    fn call(&self, need: RenderNeed, req: Value) -> Result<Vec<u8>, RenderFailure> {
        self.availability(need)
            .map_err(|u| RenderFailure::Refused(u.to_string()))?;
        let mut st = self.lock();
        let t = self.running(&mut st).map_err(RenderFailure::Refused)?;
        match t.request(req) {
            Ok(v) => match v["bytes"].as_str() {
                Some(b) if !b.is_empty() => Ok(unbase64(b)),
                _ => Err(RenderFailure::Refused("the render process answered no bytes".into())),
            },
            Err(CallError::Failed(f)) => Err(f),
            Err(CallError::Gone(why)) => {
                st.proc = None;
                Err(RenderFailure::Refused(why))
            }
        }
    }
}

impl State {
    /// Why no process may be started any more: the run used its [`MAX_STARTS`] starts and none runs.
    fn used_up(&self) -> Option<String> {
        (self.proc.is_none() && self.starts >= MAX_STARTS).then(|| format!("started {MAX_STARTS} times in this run"))
    }
}

/// The name a refusal about the process itself uses as the missing piece.
const THE_PROCESS: &str = "the render process";

fn probe_request(p: &RenderPieces) -> Value {
    let path = |b: &PathBuf| {
        std::path::absolute(b)
            .unwrap_or_else(|_| b.clone())
            .to_string_lossy()
            .into_owned()
    };
    // Every file of each set, by name: the helper answers the first one it cannot find as the missing piece.
    let sets: serde_json::Map<String, Value> = [RenderSet::Merge, RenderSet::Convert]
        .into_iter()
        .map(|s| (s.name().to_string(), json!(set_files(s, cfg!(windows)))))
        .collect();
    json!({
        "op": "render_probe",
        "dirs": wire_probe_dirs(&p.dirs),
        "sets": sets,
        "drawing": p.drawing.iter().map(path).collect::<Vec<_>>(),
        "skia": p.skia.as_ref().map(path),
    })
}

fn parse_probe(v: &Value) -> Result<ProbeAnswer, String> {
    let mut out = HashMap::new();
    for need in RenderNeed::ALL {
        let e = &v[need.wire()];
        let r = match e["ok"].as_bool() {
            Some(true) => Ok(()),
            Some(false) => Err(RenderUnavailable {
                piece: e["piece"].as_str().unwrap_or("an unnamed piece").to_string(),
                detail: e["detail"].as_str().unwrap_or("").to_string(),
            }),
            None => {
                return Err(format!(
                    "the render process answered no '{}' in render_probe",
                    need.wire()
                ));
            }
        };
        out.insert(need, r);
    }
    Ok(out)
}

impl ReportRenderer for RenderHost {
    fn availability(&self, need: RenderNeed) -> Result<(), RenderUnavailable> {
        if let Some(why) = self.lock().used_up() {
            return Err(RenderUnavailable {
                piece: THE_PROCESS.into(),
                detail: why,
            });
        }
        match self.probe() {
            Ok(m) => m.get(&need).cloned().unwrap_or_else(|| {
                Err(RenderUnavailable {
                    piece: need.wire().into(),
                    detail: "not probed".into(),
                })
            }),
            Err(why) => Err(RenderUnavailable {
                piece: THE_PROCESS.into(),
                detail: why,
            }),
        }
    }

    fn merge_word(&self, req: &WordMerge) -> Result<Vec<u8>, RenderFailure> {
        let need = if req.pictures {
            RenderNeed::MergePictures
        } else {
            RenderNeed::Merge
        };
        self.call(
            need,
            json!({"op": "word_merge", "layout": base64(&req.layout), "dataset": base64(&req.dataset),
                   "title": req.title, "company": req.company}),
        )
    }

    fn word_to_pdf(&self, docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
        self.call(RenderNeed::Pdf, json!({"op": "word_to_pdf", "docx": base64(docx)}))
    }

    fn word_to_html(&self, docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
        self.call(RenderNeed::Html, json!({"op": "word_to_html", "docx": base64(docx)}))
    }
}

// ---- the real process ---------------------------------------------------------------------

struct ProcTransport {
    proc: Proc,
    next_id: u64,
    timeout: Duration,
}

impl Drop for ProcTransport {
    fn drop(&mut self) {
        // `Proc`'s own drop kills the process and removes its folder; say goodbye first so it ends cleanly.
        let _ = self.proc.send(&json!({"op": "bye", "id": 0}));
    }
}

impl ProcTransport {
    fn start(cfg: &SidecarConfig) -> Result<ProcTransport, String> {
        let r = resolve(cfg)?;
        static STARTS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = STARTS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        let folder = format!("al-runtime-render-{}-{n}", std::process::id());
        let proc = spawn_helper(cfg, &r, &folder, &["--render"])?;
        // Starting a process is not a call: it gets at least 30 s however short the call deadline.
        let mut t = ProcTransport {
            proc,
            next_id: 1,
            timeout: cfg.timeout.max(Duration::from_secs(30)),
        };
        let v = t
            .request(json!({"op": "hello", "protocol": PROTOCOL}))
            .map_err(|f| match f {
                CallError::Failed(f) => f.to_string(),
                CallError::Gone(why) => format!("{why} (hello)"),
            })?;
        check_hello(&v, &r, "render process", Some("render"))?;
        t.timeout = cfg.timeout;
        Ok(t)
    }

    /// The process is gone: how it ended, with its last stderr line.
    fn gone(&mut self, what: &str) -> CallError {
        CallError::Gone(format!("{what}{}", self.proc.exit_detail()))
    }
}

impl Transport for ProcTransport {
    fn request(&mut self, mut req: Value) -> Result<Value, CallError> {
        let id = self.next_id;
        self.next_id += 1;
        req["id"] = json!(id);
        if self.proc.send(&req).is_err() {
            return Err(self.gone("the pipe to the render process is closed"));
        }
        let bytes = match self.proc.replies.recv_timeout(self.timeout) {
            Ok(Some(b)) => b,
            Ok(None) | Err(RecvTimeoutError::Disconnected) => {
                return Err(self.gone("the render process ended during the call"));
            }
            Err(RecvTimeoutError::Timeout) => {
                let _ = self.proc.child.kill();
                return Err(CallError::Gone(no_answer_within(self.timeout)));
            }
        };
        let v: Value = serde_json::from_slice(&bytes).map_err(|e| {
            CallError::Failed(RenderFailure::Refused(format!(
                "the render process wrote invalid JSON: {e}"
            )))
        })?;
        reply_of(v, id)
    }
}

/// The reason of a call that hit its deadline (the process is then stopped).
fn no_answer_within(timeout: Duration) -> String {
    format!(
        "the render process gave no answer within {} s and was stopped",
        timeout.as_secs()
    )
}

/// A reply frame to the request `id`'s value or failure.
fn reply_of(v: Value, id: u64) -> Result<Value, CallError> {
    let failed = |f| Err(CallError::Failed(f));
    if v["id"] != json!(id) {
        return failed(RenderFailure::Refused(format!(
            "the render process answered out of order (expected {id}, got {})",
            v["id"]
        )));
    }
    if v["ok"] == json!(true) {
        return Ok(v["value"].clone());
    }
    if v["kind"] == "exception" {
        return failed(RenderFailure::Exception {
            clr_type: v["type"].as_str().unwrap_or("").to_string(),
            message: v["message"].as_str().unwrap_or("").to_string(),
        });
    }
    failed(RenderFailure::Refused(
        v["reason"]
            .as_str()
            .unwrap_or("refused by the render process")
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Answers like the render process: a probe with merge available and pdf missing a piece; a merge echoes the layout;
    /// `die_on_merge` merges end the process.
    struct Fake {
        log: Arc<Mutex<Vec<String>>>,
        die_on_merge: bool,
    }

    impl Transport for Fake {
        fn request(&mut self, req: Value) -> Result<Value, CallError> {
            let op = req["op"].as_str().unwrap().to_string();
            self.log.lock().unwrap().push(op.clone());
            match op.as_str() {
                "render_probe" => {
                    assert_eq!(req["dirs"][0]["recursive"], json!(false));
                    // Every file of both sets, by name, from the one list (al_deps::render_set::set_files).
                    assert_eq!(req["sets"]["merge"][0], "Microsoft.Dynamics.Nav.OpenXml");
                    assert_eq!(
                        req["sets"]["conversion"][0],
                        "Microsoft.BusinessCentral.DocumentProcessor"
                    );
                    Ok(json!({
                        "merge": {"ok": true},
                        "merge_pictures": {"ok": false, "piece": "libgdiplus", "detail": "install it"},
                        "pdf": {"ok": false, "piece": "libSkiaSharp 4.152.1", "detail": "x is Skia milestone 119.0"},
                        "html": {"ok": true},
                    }))
                }
                "word_merge" if self.die_on_merge => Err(CallError::Gone(
                    "the render process ended during the call (signal: 9; last stderr: boom)".into(),
                )),
                "word_merge" if req["title"] == "empty" => Ok(json!({"bytes": ""})),
                "word_merge" => Ok(json!({"bytes": req["layout"]})),
                "word_to_html" => reply_of(
                    json!({"id": 1, "ok": false, "kind": "exception", "type": "System.IO.InvalidDataException",
                           "message": "bad"}),
                    1,
                ),
                _ => panic!("unexpected {op}"),
            }
        }
    }

    type Counted = (RenderHost, Arc<Mutex<Vec<String>>>, Arc<AtomicUsize>, Arc<AtomicUsize>);

    fn host(die_on_merge: bool) -> Counted {
        let log = Arc::new(Mutex::new(Vec::new()));
        let (starts, prepares) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let (l, s, p) = (log.clone(), starts.clone(), prepares.clone());
        let h = RenderHost::with_transport(
            Box::new(move || {
                s.fetch_add(1, Ordering::SeqCst);
                Ok(Box::new(Fake {
                    log: l.clone(),
                    die_on_merge,
                }) as Box<dyn Transport>)
            }),
            Box::new(move || {
                p.fetch_add(1, Ordering::SeqCst);
                RenderPieces {
                    dirs: vec![PathBuf::from("/svc")],
                    ..RenderPieces::default()
                }
            }),
        );
        (h, log, starts, prepares)
    }

    #[test]
    fn nothing_starts_before_the_first_need_and_the_probe_is_asked_once() {
        let (h, log, starts, prepares) = host(false);
        assert_eq!(starts.load(Ordering::SeqCst), 0);
        assert!(h.availability(RenderNeed::Merge).is_ok());
        assert_eq!(
            h.availability(RenderNeed::MergePictures).unwrap_err().to_string(),
            "libgdiplus not found (install it)"
        );
        assert_eq!(
            h.availability(RenderNeed::Pdf).unwrap_err().piece,
            "libSkiaSharp 4.152.1"
        );
        assert_eq!(
            h.merge_word(&WordMerge {
                layout: b"PK".to_vec(),
                ..WordMerge::default()
            })
            .unwrap(),
            b"PK"
        );
        assert_eq!(*log.lock().unwrap(), vec!["render_probe", "word_merge"]);
        assert_eq!((starts.load(Ordering::SeqCst), prepares.load(Ordering::SeqCst)), (1, 1));
    }

    #[test]
    fn a_call_whose_need_is_unavailable_is_refused_without_a_request() {
        let (h, log, _, _) = host(false);
        let e = h.word_to_pdf(b"PK").unwrap_err();
        assert_eq!(
            e,
            RenderFailure::Refused("libSkiaSharp 4.152.1 not found (x is Skia milestone 119.0)".into())
        );
        assert_eq!(*log.lock().unwrap(), vec!["render_probe"]);
    }

    #[test]
    fn an_exception_of_bcs_code_comes_back_with_its_clr_type() {
        let (h, _, _, _) = host(false);
        assert_eq!(
            h.word_to_html(b"PK").unwrap_err(),
            RenderFailure::Exception {
                clr_type: "System.IO.InvalidDataException".into(),
                message: "bad".into()
            }
        );
    }

    #[test]
    fn a_process_that_dies_is_started_again_with_its_pieces_and_at_most_max_starts_times() {
        let (h, log, starts, prepares) = host(true);
        for _ in 0..MAX_STARTS {
            let e = h.merge_word(&WordMerge::default()).unwrap_err();
            assert_eq!(
                e,
                RenderFailure::Refused(
                    "the render process ended during the call (signal: 9; last stderr: boom)".into()
                )
            );
        }
        let e = h.merge_word(&WordMerge::default()).unwrap_err();
        assert!(e.to_string().contains("started 3 times"), "{e}");
        assert_eq!(starts.load(Ordering::SeqCst), MAX_STARTS);
        // The pieces are prepared once; each new process is configured again.
        assert_eq!(prepares.load(Ordering::SeqCst), 1);
        let probes = log.lock().unwrap().iter().filter(|o| *o == "render_probe").count();
        assert_eq!(probes, MAX_STARTS);
    }

    #[test]
    fn an_answer_without_bytes_is_refused_not_an_empty_document() {
        let (h, _, _, _) = host(false);
        let e = h
            .merge_word(&WordMerge {
                title: "empty".into(),
                ..WordMerge::default()
            })
            .unwrap_err();
        assert_eq!(e, RenderFailure::Refused("the render process answered no bytes".into()));
    }

    #[test]
    fn after_the_last_start_every_need_is_unavailable_by_name() {
        let (h, _, _, _) = host(true);
        for _ in 0..MAX_STARTS {
            let _ = h.merge_word(&WordMerge::default());
        }
        let e = h.availability(RenderNeed::Merge).unwrap_err();
        assert_eq!(
            e.to_string(),
            "the render process not found (started 3 times in this run)"
        );
    }

    #[test]
    fn a_start_that_fails_makes_every_need_unavailable_and_names_why() {
        let h = RenderHost::with_transport(
            Box::new(|| Err("needs the .NET 8 runtime; 'dotnet' not found".to_string())),
            Box::new(RenderPieces::default),
        );
        let e = h.availability(RenderNeed::Merge).unwrap_err();
        assert_eq!(e.piece, "the render process");
        assert!(e.detail.contains("'dotnet' not found"), "{e}");
        assert!(h.merge_word(&WordMerge::default()).is_err());
    }

    #[test]
    fn a_merge_with_pictures_asks_for_the_picture_pieces_first() {
        let (h, log, _, _) = host(false);
        let e = h
            .merge_word(&WordMerge {
                pictures: true,
                ..WordMerge::default()
            })
            .unwrap_err();
        assert_eq!(e, RenderFailure::Refused("libgdiplus not found (install it)".into()));
        assert_eq!(*log.lock().unwrap(), vec!["render_probe"]);
    }

    #[test]
    fn a_deadline_is_named_as_such() {
        assert_eq!(
            no_answer_within(Duration::from_secs(120)),
            "the render process gave no answer within 120 s and was stopped"
        );
    }

    #[test]
    fn a_probe_answer_without_a_need_is_an_error_not_a_guess() {
        let e = parse_probe(&json!({"merge": {"ok": true}})).unwrap_err();
        assert!(e.contains("merge_pictures"), "{e}");
    }

    #[test]
    fn replies_out_of_order_or_refused_are_refusals() {
        assert!(matches!(
            reply_of(json!({"id": 2, "ok": true}), 1),
            Err(CallError::Failed(RenderFailure::Refused(_)))
        ));
        assert_eq!(
            reply_of(
                json!({"id": 1, "ok": false, "kind": "refused", "reason": "MergeWordDocument overload not found"}),
                1
            ),
            Err(CallError::Failed(RenderFailure::Refused(
                "MergeWordDocument overload not found".into()
            )))
        );
    }
}
