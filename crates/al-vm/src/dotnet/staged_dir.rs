//! `System.IO.DirectoryInfo` over a folder AL Runner staged itself for the company build (ADR-014 section 1). Microsoft's
//! DemoTool (Create Data Exch. Column Def) lists `TemporaryPath() + '\..\PostingExchangeDefinitions'` this way:
//! `DirectoryInfo.DirectoryInfo(Path)`, `Exists`, `GetFiles('*.xml').GetEnumerator()`, `MoveNext`, `Current`, and of each
//! `FileInfo` its `Length` and `FullName`, which it opens with AL `File.Open`.
//!
//! The rule is the one of the staged-folder `Directory.GetFiles` (router step 1c, `mod.rs`): which files a staged folder
//! holds is a detail of the host's set-up that no test sees, so the listing is answered from the staged folder as
//! Windows BC answers it. `Exists` of a folder that exists is Yes (corpus
//! tests/al-language-onprem-dotnet/lt-qx-dotnet/QxDotNetDemoShapes.al:97, :127). Windows lists '*.xml' without case, in
//! name order, and leaves out `e.xml.bak` and `d.txt`; FullName joins folder and name with a backslash
//! (windows-check-results 2026-10-22, QX 14, the same test). Linux BC lists nothing there (bc-linux draft E).
//!
//! Refused: any other folder (also a staged folder AL code has written to, which the host no longer answers), any other
//! pattern, a file whose extension only starts with `.xml` (whether '*.xml' takes it is not pinned), every other member,
//! `Current` outside the listing.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{DIRECTORY_INFO, STAGED_FILE_ARRAY, STAGED_FILE_ENUMERATOR, STAGED_FILE_INFO};
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

const PIN: &str = "tests/al-language-onprem-dotnet/lt-qx-dotnet/QxDotNetDemoShapes.al:97 (Exists); \
    the listing: windows-check-results 2026-10-22 QX 14, a staged folder only (no test sees it)";

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn object(type_name: &str, state: Vec<Value>) -> Value {
    Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: type_name.into(),
        state: RefCell::new(state),
    })))
}

fn this_state<'a>(this: Option<&'a Rc<DotNetObject>>, type_name: &str) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { type_name: t, state }) if t == type_name => Ok(state),
        _ => unsupported(format!(
            "DotNet {type_name} member on something that is not a {type_name} of a staged folder"
        )),
    }
}

fn first_text(state: &RefCell<Vec<Value>>) -> String {
    match state.borrow().first() {
        Some(Value::Text(t)) => t.to_string(),
        _ => String::new(),
    }
}

fn text_arg(args: &[Value]) -> Option<String> {
    match args {
        [Value::Text(t) | Value::Code(t)] => Some(t.to_string()),
        _ => None,
    }
}

fn no_args(args: &[Value], what: &str) -> Result<(), Signal> {
    if args.is_empty() {
        Ok(())
    } else {
        unsupported(format!("DotNet {what} takes no arguments here"))
    }
}

fn new_directory_info(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let Some(path) = text_arg(args) else {
        return unsupported(format!("DotNet new {DIRECTORY_INFO}: only a path argument is served"));
    };
    if s.host.files.staged_folder_files(&path).is_none() {
        return unsupported(format!(
            "DotNet new {DIRECTORY_INFO}: touches the file system; only a folder AL Runner staged for the company build is served"
        ));
    }
    Ok(object(DIRECTORY_INFO, vec![Value::Text(path.into())]))
}

fn exists(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, &format!("{DIRECTORY_INFO}.Exists"))?;
    this_state(this, DIRECTORY_INFO)?;
    // Only a staged folder makes a DirectoryInfo, and it exists (QxDotNetDemoShapes.al:97).
    Ok(Value::Boolean(true))
}

fn get_files(s: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let dir = first_text(this_state(this, DIRECTORY_INFO)?);
    if text_arg(args).as_deref() != Some("*.xml") {
        return unsupported(format!(
            "DotNet {DIRECTORY_INFO}.GetFiles: only the pattern '*.xml' is pinned (windows-check-results QX 14)"
        ));
    }
    let Some(paths) = s.host.files.staged_folder_files(&dir) else {
        return unsupported(format!(
            "DotNet {DIRECTORY_INFO}.GetFiles: the folder is no longer the one AL Runner staged (AL code wrote to it)"
        ));
    };
    let folder = dir.trim_end_matches(['\\', '/']);
    let mut files = Vec::new();
    // The host lists the files in Windows name order (company_data.rs staged_folder_files).
    for p in paths {
        let name = p.rsplit(['\\', '/']).next().unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        let Some(dot) = lower.rfind('.') else { continue };
        let ext = &lower[dot..];
        if ext == ".xml" {
            files.push(object(
                STAGED_FILE_INFO,
                vec![Value::Text(format!("{folder}\\{name}").into())],
            ));
        } else if ext.starts_with(".xml") {
            return unsupported(format!(
                "DotNet {DIRECTORY_INFO}.GetFiles('*.xml'): whether it lists '{name}' is not pinned"
            ));
        }
    }
    Ok(object(STAGED_FILE_ARRAY, files))
}

fn get_enumerator(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, &format!("{STAGED_FILE_ARRAY}.GetEnumerator"))?;
    let files = this_state(this, STAGED_FILE_ARRAY)?.borrow().clone();
    // State: the position (-1 before the first MoveNext), then the files.
    let mut state = vec![Value::Integer(-1)];
    state.extend(files);
    Ok(object(STAGED_FILE_ENUMERATOR, state))
}

fn position(state: &RefCell<Vec<Value>>) -> i32 {
    match state.borrow().first() {
        Some(Value::Integer(i)) => *i,
        _ => -1,
    }
}

fn move_next(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, &format!("{STAGED_FILE_ENUMERATOR}.MoveNext"))?;
    let state = this_state(this, STAGED_FILE_ENUMERATOR)?;
    let count = state.borrow().len() as i32 - 1;
    let next = (position(state) + 1).min(count);
    state.borrow_mut()[0] = Value::Integer(next);
    Ok(Value::Boolean(next < count))
}

fn current(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, &format!("{STAGED_FILE_ENUMERATOR}.Current"))?;
    let state = this_state(this, STAGED_FILE_ENUMERATOR)?;
    let at = position(state);
    let items = state.borrow();
    match usize::try_from(at).ok().and_then(|i| items.get(i + 1)) {
        Some(v) => Ok(v.clone()),
        None => unsupported(format!(
            "DotNet {STAGED_FILE_ENUMERATOR}.Current before the first or after the last file (not pinned)"
        )),
    }
}

fn full_name(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, &format!("{STAGED_FILE_INFO}.FullName"))?;
    Ok(Value::Text(first_text(this_state(this, STAGED_FILE_INFO)?).into()))
}

fn length(s: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    no_args(args, &format!("{STAGED_FILE_INFO}.Length"))?;
    let path = first_text(this_state(this, STAGED_FILE_INFO)?);
    match s.host.files.read(&path) {
        Some(bytes) => Ok(Value::BigInteger(bytes.len() as i64)),
        None => unsupported(format!("DotNet {STAGED_FILE_INFO}.Length: the staged file is gone")),
    }
}

fn member(name: &str, kind: MemberKind, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind,
        pin: Pin::Corpus(PIN.into()),
        call: f,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    use MemberKind::{Call, Construct};
    vec![
        RegistryType {
            clr_name: DIRECTORY_INFO.into(),
            members: vec![
                member("DirectoryInfo", Construct, new_directory_info),
                member("Exists", Call, exists),
                member("GetFiles", Call, get_files),
            ],
        },
        RegistryType {
            clr_name: STAGED_FILE_ARRAY.into(),
            members: vec![member("GetEnumerator", Call, get_enumerator)],
        },
        RegistryType {
            clr_name: STAGED_FILE_ENUMERATOR.into(),
            members: vec![member("MoveNext", Call, move_next), member("Current", Call, current)],
        },
        RegistryType {
            clr_name: STAGED_FILE_INFO.into(),
            members: vec![member("FullName", Call, full_name), member("Length", Call, length)],
        },
    ]
}
