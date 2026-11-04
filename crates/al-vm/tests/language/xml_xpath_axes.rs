//! XPath axes of `SelectNodes`: following, preceding and namespace. Each test names the corpus test that pins it
//! (xml/AyXPathAxesVariables.al).

use al_builtins::native::Signal;
use al_builtins::value::Value;

const LOAD: &str = "XmlDocument.ReadFrom('<r xmlns:p=\"urn:p\"><a id=\"1\"><b id=\"2\"><c id=\"3\"/><d id=\"4\"/></b><e id=\"5\"/></a><f id=\"6\"><g id=\"7\"/></f></r>', D); D.GetRoot(R); R.SelectSingleNode('//b', N); B := N.AsXmlElement();";

/// `Format(Ok) Count:names,` for `SelectNodes(path)` from element b, as the corpus test's `Q` builds it.
fn select(path: &str) -> Result<Value, Signal> {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var D: XmlDocument; R: XmlElement; B: XmlElement; N: XmlNode; L: XmlNodeList; Ok: Boolean; T: Text;
         begin {LOAD} Ok := B.SelectNodes('{path}', L); T := Format(Ok) + ' ' + Format(L.Count()) + ':';
         foreach N in L do if N.IsXmlElement() then T += N.AsXmlElement().Name() + ',' else if N.IsXmlAttribute() then T += '@' + N.AsXmlAttribute().Name() + ',' else T += '#other,';
         exit(T); end; }}"
    );
    al_testkit::run(&src, "F")
}

fn text(path: &str) -> String {
    match select(path) {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn following_axis_is_every_later_node_but_descendants() {
    // AyXPathAxesVariables.al:101 XPath_Axes_3: following::* of b is e, f, g.
    assert_eq!(text("following::*"), "Yes 3:e,f,g,");
}

#[test]
fn preceding_axis_is_every_earlier_node_but_ancestors() {
    // AyXPathAxesVariables.al:116 XPath_Axes_4: //g/preceding::* is a, b, c, d, e in document order.
    assert_eq!(text("//g/preceding::*"), "Yes 5:a,b,c,d,e,");
}

#[test]
fn preceding_axis_position_counts_from_the_nearest_node() {
    // AyXPathAxesVariables.al:176 XPath_Axes_8: //g/preceding::*[1] is e.
    assert_eq!(text("//g/preceding::*[1]"), "Yes 1:e,");
}

#[test]
fn namespace_axis_is_refused() {
    // AyXPathAxesVariables.al:161 XPath_Axes_7 pins only the names (xmlns:p, xmlns:xml); what the nodes are
    // (value, owner, identity with the declaring attribute) is not pinned.
    assert!(matches!(select("namespace::*"), Err(Signal::Unsupported(_))));
}

#[test]
fn following_and_preceding_from_an_attribute_are_refused() {
    // Not pinned: the corpus starts these axes only from elements.
    assert!(matches!(select("@id/following::*"), Err(Signal::Unsupported(_))));
    assert!(matches!(select("@id/preceding::*"), Err(Signal::Unsupported(_))));
}
