//! `Microsoft.Dynamics.Nav.AzureKeyVaultClient.AzureKeyVaultClientHelper` and Microsoft's test provider
//! `MockTest.MockAzureKeyVaultSecret.MockAzureKeyVaultSecretProvider` (ADR-014 section 1; ADR-027 row 9 amendment: Key Vault
//! is served only through a provider a test sets, never a real vault). System Application's "Azure Key Vault Impl." constructs
//! the helper, hands it the provider "Azure Key Vault Test Library" set (or none) and asks it for secrets and certificates;
//! the cache, the "missing or empty" error and ClearSecrets are that codeunit's own AL.
//!
//! Pinned by tests/al-language-onprem-dotnet/lt-y58-key-vault/Y58AzureKeyVault.al (corpus master 7330f181, PR 1019; real BC
//! 27.5, 28.5 and 29.0):
//! - no provider (never set, or a null one): both getters raise "Azure key vault has not been set up. Please contact your
//!   system administrator." as a failed call of the helper, code `DotNetInvoke:` on 27 and 28 and `DotNetInvoke:InvalidOperation`
//!   on 29 (KeyVault_NoProvider :71, KvHelper_Direct :362);
//! - with a provider: a secret name answers its mapped value, compared exactly (another case is another name), and '' when
//!   unmapped (KvHelper_Direct :362, KeyVault_Mapped_NoAllowedList :125); AllowedApplicationSecrets is not checked
//!   (KeyVault_Mapped_AllowedList :153); a certificate name answers its certificate mapping, '' when unmapped, and a secret read
//!   also finds a certificate mapping (KeyVault_Certificates :299, 'certificate name read as secret' = 'cert-value');
//! - the provider is held by reference: a name mapped after the helper got it is found (KeyVault_Cache :245);
//! - a second AddSecretMapping of a name fails with the .NET Dictionary text, code `DotNetInvoke:Argument` (KeyVault_Cache :245).
//!
//! Refused: every other member, a certificate mapping added twice (not pinned), and arguments that are not Text.
//!
//! State of a provider: `[Text(kind), Text(name), Text(value), ...]` in insertion order, kind "s" (secret) or "c" (certificate).
//! State of a helper: `[]` without a provider, `[provider]` with one.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, Value};
use std::cell::RefCell;
use std::rc::Rc;

use al_builtins::dotnet_sigs::{KEY_VAULT_HELPER as HELPER, KEY_VAULT_MOCK_PROVIDER as MOCK_PROVIDER};

const PIN: &str =
    "tests/al-language-onprem-dotnet/lt-y58-key-vault/Y58AzureKeyVault.al:71-404 (corpus master 7330f181)";

const SECRET: &str = "s";
const CERTIFICATE: &str = "c";

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn object(type_name: &str, state: Vec<Value>) -> Value {
    Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: type_name.into(),
        state: RefCell::new(state),
    })))
}

fn state<'a>(this: Option<&'a Rc<DotNetObject>>, type_name: &str) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { type_name: t, state }) if t == type_name => Ok(state),
        _ => unsupported(format!(
            "DotNet {type_name} member on something the runtime did not make"
        )),
    }
}

fn texts<'a, const N: usize>(args: &'a [Value], what: &str) -> Result<[&'a str; N], Signal> {
    let found: Vec<&str> = args
        .iter()
        .filter_map(|a| match a {
            Value::Text(t) => Some(t.as_str()),
            _ => None,
        })
        .collect();
    match <[&str; N]>::try_from(found) {
        Ok(a) if args.len() == N => Ok(a),
        _ => unsupported(format!("DotNet {what} with arguments other than {N} Text (not pinned)")),
    }
}

fn new_provider(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!("DotNet new {MOCK_PROVIDER} with arguments (not pinned)"));
    }
    Ok(object(MOCK_PROVIDER, Vec::new()))
}

fn add_mapping(this: Option<&Rc<DotNetObject>>, args: &[Value], kind: &str, member: &str) -> Result<Value, Signal> {
    let [name, value] = texts::<2>(args, &format!("{MOCK_PROVIDER}.{member}"))?;
    let mut st = state(this, MOCK_PROVIDER)?.borrow_mut();
    let taken = st
        .chunks(3)
        .any(|e| matches!(e, [Value::Text(k), Value::Text(n), _] if k.as_str() == kind && n.as_str() == name));
    if taken {
        if kind == CERTIFICATE {
            return unsupported(format!(
                "DotNet {MOCK_PROVIDER}.{member} of a name mapped before (not pinned)"
            ));
        }
        return al_builtins::native::raise(
            format!(
                "A call to {MOCK_PROVIDER}.{member} failed with this message: An item with the same key has already been added. Key: {name}"
            ),
            "DotNetInvoke:Argument",
        );
    }
    st.extend([
        Value::Text(kind.into()),
        Value::Text(name.into()),
        Value::Text(value.into()),
    ]);
    Ok(Value::Null)
}

fn add_secret(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    add_mapping(this, args, SECRET, "AddSecretMapping")
}

fn add_certificate(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    add_mapping(this, args, CERTIFICATE, "AddCertificateMapping")
}

fn new_helper(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    if !args.is_empty() {
        return unsupported(format!("DotNet new {HELPER} with arguments (not pinned)"));
    }
    Ok(object(HELPER, Vec::new()))
}

fn set_provider(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let provider = match args {
        [Value::Null] => None,
        [Value::DotNet(p)] if matches!(&p.kind, DotNetKind::Internal { type_name, .. } if type_name == MOCK_PROVIDER) => {
            Some(Value::DotNet(p.clone()))
        }
        _ => {
            return unsupported(format!(
                "DotNet {HELPER}.SetAzureKeyVaultProvider with a provider other than {MOCK_PROVIDER} (only that test provider is served, ADR-027 row 9)"
            ));
        }
    };
    *state(this, HELPER)?.borrow_mut() = provider.into_iter().collect();
    Ok(Value::Null)
}

/// What the provider maps `name` to: the first mapping of a kind in `kinds`, in that order, or ''.
fn lookup(provider: &Value, name: &str, kinds: &[&str]) -> Result<String, Signal> {
    let Value::DotNet(p) = provider else {
        return unsupported("DotNet Key Vault provider that is not an object");
    };
    let st = state(Some(p), MOCK_PROVIDER)?.borrow();
    for kind in kinds {
        for e in st.chunks(3) {
            if let [Value::Text(k), Value::Text(n), Value::Text(v)] = e {
                if k.as_str() == *kind && n.as_str() == name {
                    return Ok(v.as_str().to_string());
                }
            }
        }
    }
    Ok(String::new())
}

fn get(
    s: &mut Session,
    this: Option<&Rc<DotNetObject>>,
    args: &[Value],
    member: &str,
    kinds: &[&str],
) -> Result<Value, Signal> {
    let [name] = texts::<1>(args, &format!("{HELPER}.{member}"))?;
    let provider = state(this, HELPER)?.borrow().first().cloned();
    let Some(provider) = provider else {
        // The code names the exception from BC 29 only (KeyVault_NoProvider :71, the BC29PLUS guard).
        let code = if s.has(al_version::Gate::ProbedValuesDifferFrom29) {
            "DotNetInvoke:InvalidOperation"
        } else {
            "DotNetInvoke:"
        };
        return al_builtins::native::raise(
            format!(
                "A call to {HELPER}.{member} failed with this message: Azure key vault has not been set up. Please contact your system administrator."
            ),
            code,
        );
    };
    Ok(Value::Text(lookup(&provider, name, kinds)?.into()))
}

fn get_secret(s: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    get(s, this, args, "GetAzureKeyVaultSecret", &[SECRET, CERTIFICATE])
}

fn get_certificate(s: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    get(s, this, args, "GetAzureKeyVaultCertificate", &[CERTIFICATE])
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
            clr_name: MOCK_PROVIDER.into(),
            members: vec![
                member("MockAzureKeyVaultSecretProvider", Construct, new_provider),
                member("AddSecretMapping", Call, add_secret),
                member("AddCertificateMapping", Call, add_certificate),
            ],
        },
        RegistryType {
            clr_name: HELPER.into(),
            members: vec![
                member("AzureKeyVaultClientHelper", Construct, new_helper),
                member("SetAzureKeyVaultProvider", Call, set_provider),
                member("GetAzureKeyVaultSecret", Call, get_secret),
                member("GetAzureKeyVaultCertificate", Call, get_certificate),
            ],
        },
    ]
}
