// The scans of production code stop at the first `#[cfg(test)]` of a file; a test module in a file of its own starts with one.
#[cfg(test)]
use super::*;

/// A member call on a `[RunOnClient]` variable that no corpus test pins (Microsoft's Office Host Provider.OnGetHostType:
/// `HostType := OfficeHost.HostType()`) is refused when it runs, but its reach is known: the call runs on the client, which a
/// test session does not have, so it changes nothing on the service tier except what its arguments and its target hold
/// (corpus onprem-dotnet/runonclient/BgRunOnClient.al:122-150, :186-209, :282-302: every call on such a variable raises
/// DotNetFromAlNotSupported and the variable never holds an object). A later failure after an earlier test refused at such a
/// call is therefore a verdict, not "not trusted: part of its reach is not known" (al-runner issue 6924).
#[test]
fn a_failure_after_a_test_refused_at_a_runonclient_member_call_is_trusted() {
    let src = "dotnet { assembly(\"System.Drawing\") { type(\"System.Drawing.Printing.PrinterSettings\"; \"PrinterSettings\") { } } }
        table 61422 Row { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 61423 Host {
            [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnGetName', '', false, false)]
            local procedure OnGetName(var Name: Text) var [RunOnClient] Settings: DotNet PrinterSettings; Other: DotNet PrinterSettings; Ready: Boolean;
            begin if Ready then begin Settings := Other; Name := Settings.PrinterName(); end; end; }
        codeunit 61424 Pub { procedure Name(): Text var N: Text; begin OnGetName(N); exit(N); end;
            [IntegrationEvent(false, false)] local procedure OnGetName(var Name: Text) begin end; }
        codeunit 61421 Shared { Subtype = Test;
            [Test] procedure Refused() var [RunOnClient] Settings: DotNet PrinterSettings; T: Text; P: Codeunit Pub;
            begin T := P.Name(); T := Settings.PrinterName(); end;
            [Test] procedure Fails() var R: Record Row; begin R.Id := 1; R.Insert(); Error('x'); end; }";
    let o = super::tests::outcomes_of(src, "Shared");
    let get = |n: &str| o.iter().find(|(name, _)| name == n).unwrap().1.clone();
    assert!(
        matches!(get("Refused"), Outcome::NotRun(r) if r.contains("RunOnClient")),
        "{:?}",
        get("Refused")
    );
    assert!(matches!(get("Fails"), Outcome::Fail(_)), "{:?}", get("Fails"));
}
