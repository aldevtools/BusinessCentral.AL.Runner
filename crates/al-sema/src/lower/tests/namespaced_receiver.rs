//! `Ns.Sub."Enum".FromInteger(n)` without `Enum::` (al-runner issue 7920, the reporter's alc 18.1 repro; assumption
//! `namespaced-enum-receiver`).
use super::*;

const KIND: &str = "namespace Contoso.Foundation.Enums; enum 50100 \"Probe Kind\" { Extensible = true; value(0; A) { } value(1; B) { } }";

fn user(head: &str, body: &str) -> String {
    format!("{head} codeunit 50101 \"Probe User\" {{ procedure Pick(Kind: Integer): Integer {body} }}")
}

const CASE: &str = "begin case Contoso.Foundation.Enums.\"Probe Kind\".FromInteger(Kind) of \"Probe Kind\"::B: exit(2); end; exit(1); end;";

fn al_codes(l: &Lowered) -> Vec<&str> {
    l.al_diagnostics.iter().map(|d| d.code).collect()
}

fn assumes(l: &Lowered) -> bool {
    format!("{:?}", l.program.objects).contains("Assume(NamespacedEnumReceiver)")
}

#[test]
fn a_qualified_enum_receiver_of_from_integer_compiles_and_is_flagged() {
    // The reporter's shapes: with the using, without it, and from another namespace.
    for head in [
        "namespace Contoso.Utilities; using Contoso.Foundation.Enums;",
        "namespace Contoso.Utilities;",
    ] {
        let src = user(head, CASE).replace("\"Probe Kind\"::B", "Contoso.Foundation.Enums.\"Probe Kind\"::B");
        let (_, l) = lower_files(&[("k.al", KIND), ("u.al", &src)], false);
        assert!(al_codes(&l).is_empty(), "{head}: {:?}", al_codes(&l));
        assert!(assumes(&l), "{head}");
    }
}

#[test]
fn other_receivers_keep_mains_result() {
    // Names() on the qualified receiver is not taken: AL0118 on the first name, as on main.
    let names = user(
        "namespace Contoso.Utilities;",
        "begin exit(Contoso.Foundation.Enums.\"Probe Kind\".Names().Count()); end;",
    );
    let (_, l) = lower_files(&[("k.al", KIND), ("u.al", &names)], false);
    assert_eq!(al_codes(&l), vec!["AL0118"]);
    assert!(!assumes(&l));
    // The bare and the Enum:: forms do not take the assumption.
    for body in [
        "begin exit(Enum::Contoso.Foundation.Enums.\"Probe Kind\".FromInteger(Kind).AsInteger()); end;",
        "begin exit(\"Probe Kind\".FromInteger(Kind).AsInteger()); end;",
    ] {
        let src = user("namespace Contoso.Utilities; using Contoso.Foundation.Enums;", body);
        let (_, l) = lower_files(&[("k.al", KIND), ("u.al", &src)], false);
        assert!(al_codes(&l).is_empty(), "{body}: {:?}", al_codes(&l));
        assert!(!assumes(&l), "{body}");
    }
    // A variable named like the first name is that variable: the receiver is not taken.
    let shadow = user(
        "namespace Contoso.Utilities;",
        "var Contoso: Integer; begin exit(Contoso.Foundation.Enums.\"Probe Kind\".FromInteger(Kind).AsInteger()); end;",
    );
    let (_, l) = lower_files(&[("k.al", KIND), ("u.al", &shadow)], false);
    assert!(!assumes(&l));
}
