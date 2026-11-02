//! Three BC server classes with no state of their own, served here (ADR-014 section 1; owner 2026-10-29: pure BC server
//! classes are registered natively). Pinned by tests/al-language-onprem-dotnet/lt-y58-server-helpers/Y58ServerHelpers.al
//! (corpus master 7330f181, PR 1019; real BC 27.5, 28.5 and 29.0) and, through System Application's "Password Handler",
//! tests/al-language/misc/lt-y58-password/Y58PasswordHandler.al:
//!
//! - `PasswordGenerator.GeneratePassword(Length, MinNonAlphanumeric)` (:207 PasswordGenerator_Arguments, :112-205 the
//!   shapes): a length from 1 to 128 ("Password length incorrect" otherwise) and a minimum from 0 to the length ("Number of
//!   non alphanumeric characters is incorrect" otherwise), both `DotNetInvoke:Argument`; the password has exactly that length,
//!   at least that many of the 25 characters `!#$%&()*+-./:;=>?@[]^_{|}`, the rest letters and digits, and no class is
//!   guaranteed (an 8-character password may lack upper case, lower case or a digit). Each character is drawn evenly from the
//!   87 characters, then random positions get a non-alphanumeric until the minimum is met. The draws come from a generator
//!   of this module seeded per test method (from the session's clock key), so AL's `Random` is not disturbed and a run is
//!   repeatable; real BC draws from a cryptographic source, which AL cannot tell apart.
//! - `VSCodeRequestHelper.GetLaunchInformationQueryPart()` (:250): a query part without a leading `?` or `&` naming server,
//!   serverInstance, port, authentication, runtime and appVersion, in that order, none empty. The values are the server's
//!   and are not pinned; the runner answers its own (instance `BC` as in Active Session, the BC major as the versions).
//! - `ALSearch.IsItemSearchReady()` (:322): false in a test session. Every other ALSearch member (FindItems,
//!   EnableItemSearch, ...) is refused; FindItems is held as a Linux suspect by the corpus (Suspect_ALSearch_FindItems).

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::{DotNetObject, Value};
use std::rc::Rc;

use al_builtins::dotnet_sigs::{AL_SEARCH, PASSWORD_GENERATOR, VSCODE_REQUEST_HELPER};

const FILE: &str = "tests/al-language-onprem-dotnet/lt-y58-server-helpers/Y58ServerHelpers.al";

/// The non-alphanumeric characters a password draws from (:112-205, 'non-alphanumerics seen (sorted)').
const SYMBOLS: &[u8; 25] = b"!#$%&()*+-./:;=>?@[]^_{|}";
const ALPHANUMERIC: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn argument_error<T>(message: &str) -> Result<T, Signal> {
    al_builtins::native::raise(
        format!("A call to {PASSWORD_GENERATOR}.GeneratePassword failed with this message: {message}"),
        "DotNetInvoke:Argument",
    )
}

fn draw(s: &mut Session, below: i32) -> i32 {
    let key = s.clock_key;
    let rng = match &mut s.dotnet.password_rng {
        Some((k, rng)) if *k == key => rng,
        slot => {
            let seed = (key ^ (key >> 32)) as i32;
            &mut slot.insert((key, al_host::random::NetRandom::new(seed))).1
        }
    };
    rng.next_below(below)
}

fn generate_password(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let [Value::Integer(length), Value::Integer(min)] = args else {
        return unsupported(format!(
            "DotNet {PASSWORD_GENERATOR}.GeneratePassword with arguments other than two Integers (not pinned)"
        ));
    };
    let (length, min) = (*length, *min);
    if !(1..=128).contains(&length) {
        return argument_error("Password length incorrect");
    }
    if min < 0 || min > length {
        return argument_error("Number of non alphanumeric characters is incorrect");
    }
    let all = (ALPHANUMERIC.len() + SYMBOLS.len()) as i32;
    let mut chars: Vec<u8> = (0..length)
        .map(|_| {
            let i = draw(s, all) as usize;
            ALPHANUMERIC
                .get(i)
                .copied()
                .unwrap_or_else(|| SYMBOLS[i - ALPHANUMERIC.len()])
        })
        .collect();
    let mut symbols = chars.iter().filter(|c| SYMBOLS.contains(c)).count() as i32;
    while symbols < min {
        let at = draw(s, length) as usize;
        if !SYMBOLS.contains(&chars[at]) {
            chars[at] = SYMBOLS[draw(s, SYMBOLS.len() as i32) as usize];
            symbols += 1;
        }
    }
    Ok(Value::Text(String::from_utf8(chars).expect("ASCII").into()))
}

fn launch_query_part(s: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!(
            "DotNet {VSCODE_REQUEST_HELPER}.GetLaunchInformationQueryPart with arguments (not pinned)"
        ));
    }
    let major = s.engine.target.as_ref().map_or(29, |t| t.bc.major);
    Ok(Value::Text(
        format!(
            "server=http%3A%2F%2Flocalhost&serverInstance=BC&port=7049&authentication=UserPassword&runtime={major}.0&appVersion={major}.0"
        )
        .into(),
    ))
}

fn is_item_search_ready(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!(
            "DotNet {AL_SEARCH}.IsItemSearchReady with arguments (not pinned)"
        ));
    }
    Ok(Value::Boolean(false))
}

fn member(name: &str, line: u32, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind: MemberKind::Call,
        pin: Pin::Corpus(format!("{FILE}:{line} (corpus master 7330f181)")),
        call: f,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    vec![
        RegistryType {
            clr_name: PASSWORD_GENERATOR.into(),
            members: vec![member("GeneratePassword", 207, generate_password)],
        },
        RegistryType {
            clr_name: VSCODE_REQUEST_HELPER.into(),
            members: vec![member("GetLaunchInformationQueryPart", 250, launch_query_part)],
        },
        RegistryType {
            clr_name: AL_SEARCH.into(),
            members: vec![member("IsItemSearchReady", 322, is_item_search_ready)],
        },
    ]
}
