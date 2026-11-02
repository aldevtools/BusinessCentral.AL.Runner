//! `Microsoft.Dynamics.Nav.AzureKeyVaultClient.AzureKeyVaultClientHelper` and the test provider
//! `MockTest.MockAzureKeyVaultSecret.MockAzureKeyVaultSecretProvider`, as System Application's "Azure Key Vault Impl." and
//! Microsoft's tests use them (corpus tests/al-language-onprem-dotnet/lt-y58-key-vault/Y58AzureKeyVault.al, real BC 27.5,
//! 28.5 and 29.0: KvHelper_Direct :362, KeyVault_Cache :245). The AL below is my own minimal version of those calls.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const SRC: &str = r#"
dotnet {
    assembly("MockTest") { type("MockTest.MockAzureKeyVaultSecret.MockAzureKeyVaultSecretProvider"; "Mock") { } }
    assembly("Microsoft.Dynamics.Nav.AzureKeyVaultClient") {
        type("Microsoft.Dynamics.Nav.AzureKeyVaultClient.AzureKeyVaultClientHelper"; "Helper") { }
    }
}
codeunit 50100 C
{
    procedure F(): Text
    var H: DotNet Helper; P: DotNet Mock; V: Text;
    begin
        __BODY__
    end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    al_testkit::run_in(&SRC.replace("__BODY__", body), "C", "F")
}

fn error(r: Result<Value, Signal>) -> (String, String) {
    match r {
        Err(Signal::Error(e)) => (e.message.clone(), e.code.clone()),
        other => panic!("expected an error, got {other:?}"),
    }
}

const NOT_SET_UP: &str =
    "failed with this message: Azure key vault has not been set up. Please contact your system administrator.";

#[test]
fn a_helper_without_a_provider_is_not_set_up() {
    let (msg, code) = error(run(
        "H := H.AzureKeyVaultClientHelper(); exit(H.GetAzureKeyVaultSecret('x'));",
    ));
    assert_eq!(
        msg,
        format!(
            "A call to Microsoft.Dynamics.Nav.AzureKeyVaultClient.AzureKeyVaultClientHelper.GetAzureKeyVaultSecret {NOT_SET_UP}"
        )
    );
    assert_eq!(code, "DotNetInvoke:InvalidOperation");
    let (msg, _) = error(run(
        "H := H.AzureKeyVaultClientHelper(); exit(H.GetAzureKeyVaultCertificate('x'));",
    ));
    assert!(
        msg.contains("AzureKeyVaultClientHelper.GetAzureKeyVaultCertificate failed"),
        "{msg}"
    );
}

#[test]
fn a_null_provider_is_no_provider() {
    let (msg, _) = error(run(
        "H := H.AzureKeyVaultClientHelper(); H.SetAzureKeyVaultProvider(P); exit(H.GetAzureKeyVaultSecret('x'));",
    ));
    assert!(msg.ends_with(NOT_SET_UP), "{msg}");
}

#[test]
fn mapped_names_answer_their_value_and_unmapped_names_answer_empty() {
    let r = run(
        "P := P.MockAzureKeyVaultSecretProvider(); P.AddSecretMapping('x', 'x-value'); P.AddSecretMapping('e', ''); \
         P.AddCertificateMapping('c', 'c-value'); H := H.AzureKeyVaultClientHelper(); H.SetAzureKeyVaultProvider(P); \
         exit(H.GetAzureKeyVaultSecret('x') + '|' + H.GetAzureKeyVaultSecret('e') + '|' + H.GetAzureKeyVaultSecret('X') + '|' + \
         H.GetAzureKeyVaultSecret('c') + '|' + H.GetAzureKeyVaultCertificate('c') + '|' + H.GetAzureKeyVaultCertificate('x'));",
    );
    // A certificate name read as a secret answers the certificate (KeyVault_Certificates :299, 'certificate name read as
    // secret' = 'true/cert-value'); a secret name read as a certificate answers ''.
    assert_eq!(text(r), "x-value|||c-value|c-value|");
}

#[test]
fn the_provider_is_held_by_reference() {
    let r = run(
        "P := P.MockAzureKeyVaultSecretProvider(); H := H.AzureKeyVaultClientHelper(); H.SetAzureKeyVaultProvider(P); \
         P.AddSecretMapping('late', 'late-value'); exit(H.GetAzureKeyVaultSecret('late'));",
    );
    assert_eq!(text(r), "late-value");
}

#[test]
fn a_second_mapping_of_a_name_is_an_argument_error() {
    let (msg, code) = error(run(
        "P := P.MockAzureKeyVaultSecretProvider(); P.AddSecretMapping('k', 'a'); P.AddSecretMapping('k', 'b'); exit('');",
    ));
    assert_eq!(
        msg,
        "A call to MockTest.MockAzureKeyVaultSecret.MockAzureKeyVaultSecretProvider.AddSecretMapping failed with this message: \
         An item with the same key has already been added. Key: k"
    );
    assert_eq!(code, "DotNetInvoke:Argument");
}

#[test]
fn other_members_are_refused() {
    match run("P := P.MockAzureKeyVaultSecretProvider(); P.RemoveSecretMapping('k'); exit('');") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("RemoveSecretMapping"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn clear_of_a_helper_or_provider_unassigns_the_variable() {
    // "Azure Key Vault Impl.".ClearSecrets clears both (KeyVault_Cache :245, 'read after ClearSecrets' = not set up).
    let r = run(
        "P := P.MockAzureKeyVaultSecretProvider(); H := H.AzureKeyVaultClientHelper(); Clear(H); Clear(P); \
         if IsNull(H) and IsNull(P) then exit('null'); exit('kept');",
    );
    assert_eq!(text(r), "null");
}
