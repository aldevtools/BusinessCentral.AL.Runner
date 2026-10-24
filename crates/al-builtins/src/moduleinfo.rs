//! `NavApp.GetCurrentModuleInfo` and friends, and the `ModuleInfo` type: what an app's manifest
//! says about it (corpus session/TestNavAppExtended.al, TestNavAppModuleInfoCallerIdentity.al,
//! TestNavAppCallerCallstackModuleInfos.al, TestBCPlatformContracts.al, TestMiscTypes.al).
//!
//! Resources, entitlements and archive data of an app are not here.

use crate::describe::Describe;
use crate::native::{AlError, Ctx, ModuleLookup, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{LibraryValue, ModuleInfoState, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Out = Result<Value, Signal>;

/// The id of the Microsoft "Application" app, which every app under test depends on through `application` in its manifest.
pub const APPLICATION_APP_ID: &str = "c1335042-3002-4257-bf8a-75c898ccb1b8";

pub const REGISTRY: &[(&str, NativeFn)] = &[
    ("NavApp.GetCurrentModuleInfo#1", get_current),
    ("NavApp.GetCallerModuleInfo#1", get_caller),
    ("NavApp.GetModuleInfo#1", get_module),
    ("NavApp.GetCallstackModuleInfos#1", |c, _, _| callstack(c, 0)),
    ("NavApp.GetCallerCallstackModuleInfos#1", |c, _, _| callstack(c, 1)),
    ("NavApp.GetResource#1", get_resource),
    ("NavApp.GetResourceAsText#1", get_resource_as_text),
    ("NavApp.GetResourceAsJson#1", get_resource_as_json),
    ("NavApp.ListResources#1", list_resources),
    // True while an install codeunit runs (session/TestNavAppExtended.al, TestBCPlatformContracts.al: false in a test).
    ("NavApp.IsInstalling#1", |c, _, _| Ok(Value::Boolean(c.install_scope()))),
];

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn module_value(m: ModuleInfoState) -> Value {
    Value::Library("ModuleInfo".into(), Rc::new(RefCell::new(LibraryValue::ModuleInfo(m))))
}

fn set_info(args: &mut [Value], m: ModuleInfoState) -> Out {
    match args.first_mut() {
        Some(slot) => {
            *slot = module_value(m);
            Ok(Value::Boolean(true))
        }
        None => unsupported("a module info call without its var parameter"),
    }
}

fn get_current(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    match c.module_at_frame(0) {
        Some(m) => set_info(a, m),
        None => unsupported("the app of the running code has no readable manifest id"),
    }
}

/// The immediate caller's app, even when it is the same app
/// (session/TestNavAppModuleInfoCallerIdentity.al:15-20).
fn get_caller(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    match c.module_at_frame(1) {
        Some(m) => set_info(a, m),
        None => unsupported("GetCallerModuleInfo with no caller in the workspace (the test runner calls a test)"),
    }
}

fn guid_arg(v: &Value) -> Option<[u8; 16]> {
    match v {
        Value::Guid(g) => Some(*g),
        other => crate::native::text_of(other).and_then(|t| crate::ops::parse_guid(&t)),
    }
}

fn plain_guid(g: &[u8; 16]) -> String {
    let h: String = g.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[0..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..32]
    )
}

/// `GetModuleInfo(AppId, var Info)`: an installed app, else false when the result is used, else an
/// error that names the id (session/TestNavAppExtended.al:118-150: "No installed extension was
/// found with ID" and the id). The null id answers with a Microsoft module there: refused.
fn get_module(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Some(id) = a.first().and_then(guid_arg) else {
        return unsupported(format!("GetModuleInfo with {} as the app id", a.first().describe()));
    };
    if id == [0; 16] {
        // True, with a module whose data version is the installed application's (at least 27):
        // session/TestEnvironmentUserTenantContracts.al:195-201. Its other properties are not pinned.
        if a.len() < 2 {
            return unsupported("GetModuleInfo without its var parameter");
        }
        a[1] = module_value(c.application_module());
        return Ok(Value::Boolean(true));
    }
    match c.module_by_id(&id) {
        ModuleLookup::Found(m) => {
            if a.len() < 2 {
                return unsupported("GetModuleInfo without its var parameter");
            }
            a[1] = module_value(m);
            Ok(Value::Boolean(true))
        }
        ModuleLookup::OutsideWorkspace => {
            unsupported("GetModuleInfo of an app outside the workspace (Microsoft apps, ADR-013)")
        }
        ModuleLookup::NotInstalled if c.ret_used() => Ok(Value::Boolean(false)),
        ModuleLookup::NotInstalled => Err(Signal::Error(Box::new(AlError {
            message: format!("No installed extension was found with ID '{}'.", plain_guid(&id)),
            code: "App".into(),
            call_stack: String::new(),
            ..Default::default()
        }))),
    }
}

/// The apps on the call stack, each once, innermost first
/// (session/TestNavAppCallerCallstackModuleInfos.al).
fn callstack(c: &mut dyn Ctx, skip: usize) -> Out {
    let items = c.module_callstack(skip).into_iter().map(module_value).collect();
    Ok(Value::List(Rc::new(RefCell::new(items))))
}

fn state(r: Option<&mut Value>) -> Result<ModuleInfoState, Signal> {
    state_of(r, false)
}

/// The module behind a `ModuleInfo`; the module BC makes up for the null id is only readable where
/// `synthesized_ok` (its data version).
fn state_of(r: Option<&mut Value>, _synthesized_ok: bool) -> Result<ModuleInfoState, Signal> {
    match r {
        Some(Value::Library(_, cell)) => match &*cell.borrow() {
            LibraryValue::ModuleInfo(m) => Ok(m.clone()),
            other => unsupported(format!("a ModuleInfo nobody filled ({})", other.describe())),
        },
        other => unsupported(format!("a ModuleInfo method on {}", other.describe())),
    }
}

fn version_value(v: [i32; 4]) -> Value {
    Value::Library("Version".into(), Rc::new(RefCell::new(LibraryValue::Version(v))))
}

fn app_version(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    version_of(state(r)?)
}

fn version_of(m: ModuleInfoState) -> Out {
    match m.version {
        Some(v) => Ok(version_value(v)),
        None => unsupported("the manifest of this app has no version the runtime can read"),
    }
}

/// A freshly installed app's data version is its app version (not pinned further: the corpus asks
/// only that a version comes back, session/TestMiscTypes.al).
///
/// While its install code runs, the app's own data version is 0.0.0.0, per company and per database: the app was
/// not installed before (install/lt-cm-install-leads/CmInstallLeads.al:Item43_ModuleInfoVersions_DuringInstall; this
/// runtime always installs fresh).
fn data_version(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let m = state_of(r, true)?;
    if c.install_without_client() && c.module_at_frame(0).is_some_and(|own| own.id == m.id) {
        return Ok(version_value([0, 0, 0, 0]));
    }
    version_of(m)
}

/// The package id of the app with id `app` (16 bytes in text order): a service tier assigns it when it
/// publishes the app, so the runtime makes one up that follows the app id. The corpus pins that it is not empty
/// for a real app, differs from the app id in its first characters (session/W5ModuleInfo.al:
/// ModuleInfo_PackageId_ComparedToId), is one value for all objects of the app (AllObj, AllObjWithCaption) and
/// differs between apps (record/TestAllObjOwningAppPerKind.al:109-132). The null id has the null package id
/// (W5ModuleInfo.al:ModuleInfo_NullId_Properties).
pub fn package_id_of(app: [u8; 16]) -> [u8; 16] {
    if app == [0; 16] {
        return [0; 16];
    }
    // Two 64-bit FNV-1a lanes over the id and a fixed salt, laid out as a version 4 guid.
    let salt = b"al-runtime/package-id/v1";
    let mut out = [0u8; 16];
    for (lane, chunk) in out.chunks_mut(8).enumerate() {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (lane as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        for b in app.iter().chain(salt) {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        chunk.copy_from_slice(&h.to_be_bytes());
    }
    out[6] = (out[6] & 0x0f) | 0x40;
    out[8] = (out[8] & 0x3f) | 0x80;
    out
}

fn package_id(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    Ok(Value::Guid(package_id_of(state(r)?.id)))
}

fn dependencies(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let items = state(r)?
        .dependencies
        .iter()
        .map(|d| {
            Value::Library(
                "ModuleDependencyInfo".into(),
                Rc::new(RefCell::new(LibraryValue::Text(plain_guid(d)))),
            )
        })
        .collect();
    Ok(Value::List(Rc::new(RefCell::new(items))))
}

/// The guid a `ModuleDependencyInfo` value holds.
fn dependency_id(r: Option<&mut Value>) -> Result<[u8; 16], Signal> {
    match r {
        Some(Value::Library(_, cell)) => match &*cell.borrow() {
            LibraryValue::Text(t) => crate::ops::parse_guid(t)
                .ok_or_else(|| Signal::Unsupported("a ModuleDependencyInfo with a broken id".into())),
            other => unsupported(format!("a ModuleDependencyInfo nobody filled ({})", other.describe())),
        },
        other => unsupported(format!("a ModuleDependencyInfo method on {}", other.describe())),
    }
}

/// The app a dependency names, from the apps of the workspace (session/TestProbeSessionMiscContracts.al:ModuleInfo_CurrentModule:
/// `Dep.Name`, `Dep.Id`, `Dep.Publisher`).
fn dependency_app(c: &mut dyn Ctx, r: Option<&mut Value>) -> Result<ModuleInfoState, Signal> {
    let id = dependency_id(r)?;
    if crate::ops::parse_guid(APPLICATION_APP_ID) == Some(id) {
        return Ok(ModuleInfoState {
            id,
            name: "Application".into(),
            publisher: "Microsoft".into(),
            version: None,
            dependencies: Vec::new(),
            synthesized: false,
        });
    }
    match c.module_by_id(&id) {
        ModuleLookup::Found(m) => Ok(m),
        _ => unsupported("the name or publisher of a dependency whose app is not among the apps of the workspace"),
    }
}

pub fn native_for_dependency(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver != "ModuleDependencyInfo" || !sig.instance {
        return None;
    }
    let f: NativeFn = match sig.name.as_str() {
        "Id" => |_, r, _| Ok(Value::Guid(dependency_id(r)?)),
        "Name" => |c, r, _| Ok(Value::Text(dependency_app(c, r)?.name.into())),
        "Publisher" => |c, r, _| Ok(Value::Text(dependency_app(c, r)?.publisher.into())),
        _ => return None,
    };
    Some(f)
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver == "ModuleDependencyInfo" {
        return native_for_dependency(sig);
    }
    if sig.receiver != "ModuleInfo" || !sig.instance {
        return None;
    }
    let f: NativeFn = match sig.name.as_str() {
        "Id" => |_, r, _| Ok(Value::Guid(state(r)?.id)),
        "Name" => |_, r, _| Ok(Value::Text(state(r)?.name.into())),
        "Publisher" => |_, r, _| Ok(Value::Text(state(r)?.publisher.into())),
        "AppVersion" => app_version,
        "DataVersion" => data_version,
        "PackageId" => package_id,
        "Dependencies" => dependencies,
        _ => return None,
    };
    Some(f)
}

// ---- packaged resources ----------------------------------------------------------------------

/// The resource `name` of the app that owns the running code, else the error BC raises
/// (session/TestNavAppGetResource.al:62-85: "A resource matching '{0}' could not be found in app
/// '{1}'."; the corpus asserts the part up to the app name).
fn resource(c: &dyn Ctx, name: &str) -> Result<Vec<u8>, Signal> {
    let Some((app, resources)) = c.app_resources() else {
        return unsupported("resources of an app the runtime does not know");
    };
    match resources.into_iter().find(|(n, _)| n == name) {
        Some((_, bytes)) => Ok(bytes),
        None => Err(Signal::Error(Box::new(AlError {
            message: format!("A resource matching '{name}' could not be found in app '{app}'."),
            code: String::new(),
            call_stack: String::new(),
            ..Default::default()
        }))),
    }
}

fn name_arg(a: &[Value]) -> Result<String, Signal> {
    match a.first().and_then(crate::native::text_of) {
        Some(n) => Ok(n.to_string()),
        None => unsupported(format!("a resource name that is not text: {}", a.first().describe())),
    }
}

/// The `TextEncoding` argument at `at`: UTF-8 when omitted (ordinals as documented: MSDos 0, UTF8 1,
/// UTF16 2, Windows 3); only UTF-8 is pinned for resources.
fn encoding_arg(a: &[Value], at: usize) -> Result<crate::stream::Encoding, Signal> {
    match a.get(at) {
        None | Some(Value::Option(1, _)) => Ok(crate::stream::Encoding::Utf8),
        Some(other) => unsupported(format!(
            "a resource read with text encoding {other} (only UTF8 is pinned)",
            other = other.describe()
        )),
    }
}

fn get_resource(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = name_arg(a)?;
    let enc = encoding_arg(a, 2)?;
    let data = resource(c, &name)?;
    match a.get(1) {
        Some(Value::Library(_, cell)) => {
            *cell.borrow_mut() = LibraryValue::InStream(crate::stream::InState::snapshot(data, enc));
            Ok(Value::Null)
        }
        other => unsupported(format!("GetResource into {}", other.describe())),
    }
}

fn get_resource_as_text(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = name_arg(a)?;
    let enc = encoding_arg(a, 1)?;
    Ok(Value::Text(enc.decode(&resource(c, &name)?).into()))
}

fn get_resource_as_json(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = name_arg(a)?;
    let enc = encoding_arg(a, 1)?;
    let text = enc.decode(&resource(c, &name)?);
    match crate::json::object_from_text(&text)? {
        Some(v) => Ok(v),
        None => unsupported(format!(
            "resource `{name}` is not a JSON object (error text not pinned)"
        )),
    }
}

/// `*` and `?` wildcards; the filter language is the documented one, the corpus only calls it
/// (session/TestMiscComplete.al:139-165).
fn glob(pattern: &str, text: &str) -> bool {
    fn go(p: &[char], t: &[char]) -> bool {
        match p.split_first() {
            None => t.is_empty(),
            Some(('*', rest)) => (0..=t.len()).any(|i| go(rest, &t[i..])),
            Some(('?', rest)) => !t.is_empty() && go(rest, &t[1..]),
            Some((c, rest)) => t.first().is_some_and(|x| x.eq_ignore_ascii_case(c)) && go(rest, &t[1..]),
        }
    }
    go(&pattern.chars().collect::<Vec<_>>(), &text.chars().collect::<Vec<_>>())
}

fn list_resources(c: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Some((_, resources)) = c.app_resources() else {
        return unsupported("resources of an app the runtime does not know");
    };
    let filter = match a.first() {
        None => None,
        Some(v) => Some(
            crate::native::text_of(v)
                .ok_or_else(|| Signal::Unsupported("a resource filter that is not text".into()))?,
        ),
    };
    let items = resources
        .into_iter()
        .map(|(n, _)| n)
        .filter(|n| filter.as_deref().is_none_or(|f| glob(f, n)))
        .map(|s| Value::Text(s.into()))
        .collect();
    Ok(Value::List(Rc::new(RefCell::new(items))))
}
