//! The report renderer seam (ADR-040 amendment 2026-11-13b): BC's own Word merger and document converter, run by a
//! front end (al-testrun's render process). Plain data and one trait; nothing here touches the OS.
//!
//! The renderer only makes bytes. Whether a save is answered, its error and its events stay with the save table of the
//! VM; which saves ask the renderer at all is decided there too.

use std::fmt;

/// What a caller needs the renderer for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderNeed {
    /// A Word layout merged with a dataset, without picture controls.
    Merge,
    /// A Word layout with picture controls merged with a dataset (needs `System.Drawing` on the platform).
    MergePictures,
    /// A .docx converted to PDF.
    Pdf,
    /// A .docx converted to Html.
    Html,
}

impl RenderNeed {
    pub const ALL: [RenderNeed; 4] = [
        RenderNeed::Merge,
        RenderNeed::MergePictures,
        RenderNeed::Pdf,
        RenderNeed::Html,
    ];

    /// The name the render process uses for this need.
    pub fn wire(self) -> &'static str {
        match self {
            RenderNeed::Merge => "merge",
            RenderNeed::MergePictures => "merge_pictures",
            RenderNeed::Pdf => "pdf",
            RenderNeed::Html => "html",
        }
    }
}

/// Why a need cannot be served on this machine: the first missing piece, by name (`libgdiplus`, `.NET 8 runtime`,
/// `libSkiaSharp.so 3.119.0`, ...), and what the renderer says about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderUnavailable {
    /// The missing piece, by name.
    pub piece: String,
    /// The detail the renderer gave (where it looked, what it found instead).
    pub detail: String,
}

impl fmt::Display for RenderUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.detail.is_empty() {
            write!(f, "{} not found", self.piece)
        } else {
            write!(f, "{} not found ({})", self.piece, self.detail)
        }
    }
}

/// One merge: the layout, the dataset XML the runner wrote for the report, and the two texts the merger writes into the
/// document's properties (the title into core properties, the company into `docProps/app.xml`). Which values BC passes
/// is decided by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WordMerge {
    pub layout: Vec<u8>,
    pub dataset: Vec<u8>,
    pub title: String,
    pub company: String,
    /// The layout has picture controls: the merge needs [`RenderNeed::MergePictures`], not only [`RenderNeed::Merge`].
    pub pictures: bool,
}

/// Why a render call gave no bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderFailure {
    /// The call could not be made: a piece is missing, the entry point's overload is not in the table, the process died.
    Refused(String),
    /// BC's code threw: the CLR type of the exception (the innermost of a reflection call) and its message.
    Exception { clr_type: String, message: String },
}

impl fmt::Display for RenderFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderFailure::Refused(why) => f.write_str(why),
            RenderFailure::Exception { clr_type, message } => write!(f, "{clr_type}: {message}"),
        }
    }
}

/// The renderer a run has. Shared by every session of the run.
pub trait ReportRenderer: Send + Sync {
    /// Cheap after the first call. `Err` names the missing piece.
    fn availability(&self, need: RenderNeed) -> Result<(), RenderUnavailable>;
    fn merge_word(&self, req: &WordMerge) -> Result<Vec<u8>, RenderFailure>;
    fn word_to_pdf(&self, docx: &[u8]) -> Result<Vec<u8>, RenderFailure>;
    fn word_to_html(&self, docx: &[u8]) -> Result<Vec<u8>, RenderFailure>;
}

/// The renderer of a run that has none: every need is unavailable, every call refused.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoRenderer;

/// The piece [`NoRenderer`] names.
pub const NO_RENDERER: &str = "a report renderer";

impl NoRenderer {
    fn refused() -> RenderFailure {
        RenderFailure::Refused(format!("{NO_RENDERER} is not configured"))
    }
}

impl ReportRenderer for NoRenderer {
    fn availability(&self, _need: RenderNeed) -> Result<(), RenderUnavailable> {
        Err(RenderUnavailable {
            piece: NO_RENDERER.into(),
            detail: "none is configured".into(),
        })
    }
    fn merge_word(&self, _req: &WordMerge) -> Result<Vec<u8>, RenderFailure> {
        Err(NoRenderer::refused())
    }
    fn word_to_pdf(&self, _docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
        Err(NoRenderer::refused())
    }
    fn word_to_html(&self, _docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
        Err(NoRenderer::refused())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A renderer for tests: one need available, the calls recorded.
    struct Fake {
        available: RenderNeed,
        calls: Mutex<Vec<String>>,
    }

    impl ReportRenderer for Fake {
        fn availability(&self, need: RenderNeed) -> Result<(), RenderUnavailable> {
            if need == self.available {
                Ok(())
            } else {
                Err(RenderUnavailable {
                    piece: "libgdiplus".into(),
                    detail: String::new(),
                })
            }
        }
        fn merge_word(&self, req: &WordMerge) -> Result<Vec<u8>, RenderFailure> {
            self.calls.lock().unwrap().push(format!("merge {}", req.title));
            Ok(req.layout.clone())
        }
        fn word_to_pdf(&self, _docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
            Err(RenderFailure::Exception {
                clr_type: "System.InvalidOperationException".into(),
                message: "x".into(),
            })
        }
        fn word_to_html(&self, _docx: &[u8]) -> Result<Vec<u8>, RenderFailure> {
            Err(RenderFailure::Refused("no".into()))
        }
    }

    #[test]
    fn no_renderer_refuses_every_need_and_every_call() {
        let r: Arc<dyn ReportRenderer> = Arc::new(NoRenderer);
        for need in RenderNeed::ALL {
            let e = r.availability(need).unwrap_err();
            assert_eq!(e.to_string(), "a report renderer not found (none is configured)");
        }
        assert_eq!(
            r.merge_word(&WordMerge::default()).unwrap_err(),
            RenderFailure::Refused("a report renderer is not configured".into())
        );
        assert!(r.word_to_pdf(b"x").is_err());
        assert!(r.word_to_html(b"x").is_err());
    }

    #[test]
    fn a_renderer_behind_the_trait_answers_per_need_and_its_failures_print_their_kind() {
        let f = Fake {
            available: RenderNeed::Merge,
            calls: Mutex::new(Vec::new()),
        };
        let r: &dyn ReportRenderer = &f;
        assert!(r.availability(RenderNeed::Merge).is_ok());
        assert_eq!(
            r.availability(RenderNeed::MergePictures).unwrap_err().to_string(),
            "libgdiplus not found"
        );
        let req = WordMerge {
            layout: b"PK".to_vec(),
            title: "R".into(),
            ..WordMerge::default()
        };
        assert_eq!(r.merge_word(&req).unwrap(), b"PK");
        assert_eq!(*f.calls.lock().unwrap(), vec!["merge R".to_string()]);
        assert_eq!(
            r.word_to_pdf(b"").unwrap_err().to_string(),
            "System.InvalidOperationException: x"
        );
        assert_eq!(r.word_to_html(b"").unwrap_err().to_string(), "no");
    }

    #[test]
    fn the_wire_names_are_distinct() {
        let mut names: Vec<_> = RenderNeed::ALL.iter().map(|n| n.wire()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), RenderNeed::ALL.len());
    }
}
