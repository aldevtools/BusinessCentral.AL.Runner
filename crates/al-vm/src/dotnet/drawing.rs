//! The eight `System.Drawing` members Microsoft's Base App uses to make the sample picture of a brand color
//! (`O365 Brand Color`.MakePicture, O365BrandColor.Table.al:113-137), served by the runtime (ADR-044, option 3,
//! accepted by the owner on 2026-10-18). The same pattern as ADR-040: the call runs, nothing is drawn, and AL
//! that looks at the picture is refused.
//!
//! - `new Bitmap(Integer, Integer)`, `Graphics.FromImage(Bitmap)`, `ColorTranslator.FromHtml('#RRGGBB')`,
//!   `new SolidBrush(Color)`, `Graphics.FillEllipse(SolidBrush, Integer x4)`, `Graphics.Dispose()`,
//!   `Bitmap.Save(InStream, ImageFormat.Png)` and `Bitmap.Dispose()` are accepted.
//! - `Save` marks the buffer its InStream reads as "picture not produced" (`BlobBuf::render_pending`, the mark
//!   ADR-040 uses for report output). The buffer then has a value in the sense of `Media.ImportStream`
//!   (`HasValue` true) and every look at its content is refused with
//!   `picture drawn by System.Drawing.Bitmap is not produced`.
//! - Everything else on these types is refused as a member the runtime does not implement, and so is any
//!   argument other than the kinds above (`Bitmap(0, 10)`, `FromHtml('red')`, `Save` to an OutStream).
//!
//! Pinned by no corpus test and no probe. The only evidence is Microsoft's own test, which passes on real BC:
//! O365BrandColorTests.Codeunit.al:74 and :158 (`Sample Picture.HasValue` after MakePicture). That is the
//! hard rule 3 exception of 2026-10-15; the assumption `bitmap-picture-not-drawn` is marked each time the code
//! runs and a probe row waits in `docs/probes-needed.md`.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::dotnet_sigs::{BITMAP, COLOR, COLOR_TRANSLATOR, GRAPHICS, IMAGE_FORMAT, SOLID_BRUSH};
use al_builtins::library_kind::LibraryKind;
use al_builtins::native::{Signal, unsupported};
use al_builtins::value::{DotNetKind, DotNetObject, LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PIN: &str =
    "Microsoft tests/Tests-Invoicing O365BrandColorTests.Codeunit.al:74 (HasValue after MakePicture; ADR-044)";

fn object(type_name: &str, state: Vec<Value>) -> Value {
    Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: type_name.into(),
        state: RefCell::new(state),
    })))
}

/// The state of `v` when it is a runtime object of `type_name`.
fn state_of<'a>(v: &'a Value, type_name: &str) -> Option<&'a RefCell<Vec<Value>>> {
    match v {
        Value::DotNet(o) => match &o.kind {
            DotNetKind::Internal { type_name: t, state } if t == type_name => Some(state),
            _ => None,
        },
        _ => None,
    }
}

fn this_state<'a>(this: Option<&'a Rc<DotNetObject>>, type_name: &str) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { type_name: t, state }) if t == type_name => Ok(state),
        _ => unsupported(format!(
            "DotNet {type_name} member on something that is not a {type_name}"
        )),
    }
}

fn disposed(state: &RefCell<Vec<Value>>) -> bool {
    matches!(state.borrow().first(), Some(Value::Boolean(true)))
}

/// A whole-number argument. The members take `Integer`; nothing else is accepted.
fn int(v: &Value) -> Option<i32> {
    match v {
        Value::Integer(n) => Some(*n),
        _ => None,
    }
}

// ---- Bitmap -----------------------------------------------------------------------------------------------

fn bitmap_new(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let [w, h] = args else {
        return unsupported("DotNet new System.Drawing.Bitmap takes (Integer, Integer) here (ADR-044)");
    };
    match (int(w), int(h)) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {}
        _ => {
            return unsupported(
                "DotNet new System.Drawing.Bitmap takes two positive Integers here; what BC does for another size is not pinned (ADR-044)",
            );
        }
    }
    assume(Assumption::BitmapPictureNotDrawn);
    Ok(object(BITMAP, vec![Value::Boolean(false)]))
}

fn bitmap_save(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = this_state(this, BITMAP)?;
    if disposed(state) {
        return unsupported("DotNet System.Drawing.Bitmap.Save after Dispose: BC's answer is not pinned (ADR-044)");
    }
    let [Value::Library(LibraryKind::InStream, stream), format] = args else {
        return unsupported(
            "DotNet System.Drawing.Bitmap.Save takes (InStream, ImageFormat.Png) here; another target is not pinned (ADR-044)",
        );
    };
    match state_of(format, IMAGE_FORMAT) {
        Some(s) if matches!(s.borrow().first(), Some(Value::Text(t)) if &**t == "Png") => {}
        _ => {
            return unsupported(
                "DotNet System.Drawing.Bitmap.Save with an image format other than Png: not pinned (ADR-044)",
            );
        }
    }
    assume(Assumption::BitmapPictureNotDrawn);
    let marked = match &*stream.borrow() {
        LibraryValue::InStream(s) => s.mark_render_pending(BITMAP.into()),
        _ => false,
    };
    if !marked {
        return unsupported("DotNet System.Drawing.Bitmap.Save into an InStream that reads no live buffer (ADR-044)");
    }
    Ok(Value::Null)
}

fn bitmap_dispose(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    dispose(this, args, BITMAP)
}

fn dispose(this: Option<&Rc<DotNetObject>>, args: &[Value], type_name: &str) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!("DotNet {type_name}.Dispose takes no argument"));
    }
    this_state(this, type_name)?.borrow_mut()[0] = Value::Boolean(true);
    Ok(Value::Null)
}

// ---- Graphics ---------------------------------------------------------------------------------------------

fn graphics_from_image(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let [image] = args else {
        return unsupported("DotNet System.Drawing.Graphics.FromImage takes one Bitmap here (ADR-044)");
    };
    match state_of(image, BITMAP) {
        Some(s) if !disposed(s) => Ok(object(GRAPHICS, vec![Value::Boolean(false)])),
        _ => unsupported("DotNet System.Drawing.Graphics.FromImage takes a Bitmap that is not disposed here (ADR-044)"),
    }
}

fn graphics_fill_ellipse(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = this_state(this, GRAPHICS)?;
    let [brush, rest @ ..] = args else {
        return unsupported("DotNet System.Drawing.Graphics.FillEllipse takes (SolidBrush, Integer x4) here (ADR-044)");
    };
    if disposed(state)
        || state_of(brush, SOLID_BRUSH).is_none()
        || rest.len() != 4
        || rest.iter().any(|v| int(v).is_none())
    {
        return unsupported(
            "DotNet System.Drawing.Graphics.FillEllipse takes (SolidBrush, Integer x4) on a Graphics that is not disposed here (ADR-044)",
        );
    }
    // Nothing is drawn (ADR-044).
    Ok(Value::Null)
}

fn graphics_dispose(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    dispose(this, args, GRAPHICS)
}

// ---- Color, brush, image format ---------------------------------------------------------------------------

/// `ColorTranslator.FromHtml('#RRGGBB')`. Names ('red'), short forms ('#F00') and numbers are other code paths of
/// .NET; none is used by MakePicture, so they are refused.
fn color_from_html(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let text = match args {
        [Value::Text(t)] => Some(t.to_string()),
        [Value::Code(t)] => Some(t.to_string()),
        _ => None,
    };
    match text {
        Some(t) if t.len() == 7 && t.starts_with('#') && t[1..].bytes().all(|b| b.is_ascii_hexdigit()) => {
            Ok(object(COLOR, vec![Value::Text(t.into())]))
        }
        _ => unsupported(
            "DotNet System.Drawing.ColorTranslator.FromHtml takes text of the form #RRGGBB here; another form is not pinned (ADR-044)",
        ),
    }
}

fn brush_new(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    match args {
        [c] if state_of(c, COLOR).is_some() => Ok(object(SOLID_BRUSH, vec![c.clone()])),
        _ => unsupported("DotNet new System.Drawing.SolidBrush takes one Color here (ADR-044)"),
    }
}

fn format_png(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    Ok(object(IMAGE_FORMAT, vec![Value::Text("Png".into())]))
}

fn member(name: &str, kind: MemberKind, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind,
        pin: Pin::Microsoft(PIN.into()),
        call: f,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    use MemberKind::{Call, Construct, Get};
    vec![
        RegistryType {
            clr_name: BITMAP.into(),
            members: vec![
                member("Bitmap", Construct, bitmap_new),
                member("Save", Call, bitmap_save),
                member("Dispose", Call, bitmap_dispose),
            ],
        },
        RegistryType {
            clr_name: GRAPHICS.into(),
            members: vec![
                member("FromImage", Call, graphics_from_image),
                member("FillEllipse", Call, graphics_fill_ellipse),
                member("Dispose", Call, graphics_dispose),
            ],
        },
        RegistryType {
            clr_name: COLOR_TRANSLATOR.into(),
            members: vec![member("FromHtml", Call, color_from_html)],
        },
        // The color is only passed on to the brush; none of its members is served.
        RegistryType {
            clr_name: COLOR.into(),
            members: Vec::new(),
        },
        RegistryType {
            clr_name: SOLID_BRUSH.into(),
            members: vec![member("SolidBrush", Construct, brush_new)],
        },
        RegistryType {
            clr_name: IMAGE_FORMAT.into(),
            members: vec![member("Png", Get, format_png)],
        },
    ]
}
