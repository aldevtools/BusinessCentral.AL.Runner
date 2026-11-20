//! al-runner issue 8320: AL0288 for a subscriber's var parameter on a by-value publisher parameter (corpus
//! tests/must-not-compile/cases/1685-subscriber-var-on-by-value-parameter.al:1). A var publisher parameter with a var or
//! a by-value subscriber parameter compiles (1685:3).

use super::*;

fn al0288(sub_var: &str, pub_var: &str) -> Vec<String> {
    let src = format!(
        "codeunit 50000 Pub {{ [IntegrationEvent(false, false)] procedure OnH(var Handled: Boolean; {pub_var}Plain: Boolean) begin end; }}
         codeunit 50001 Sub {{ [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnH', '', false, false)]
           local procedure S({sub_var}Plain: Boolean) begin end; }}"
    );
    let ws = load(&[&src]);
    check_workspace(&ws)
        .into_iter()
        .filter(|d| d.code == "AL0288")
        .map(|d| d.message)
        .collect()
}

#[test]
fn a_var_subscriber_parameter_on_a_by_value_publisher_parameter_is_al0288() {
    assert_eq!(
        al0288("var ", ""),
        vec!["Parameter 'Plain' is only allowed to be 'var' if the publisher parameter is 'var'.".to_string()]
    );
}

#[test]
fn a_var_publisher_parameter_compiles_either_way() {
    assert!(al0288("var ", "var ").is_empty());
    assert!(al0288("", "var ").is_empty());
}
