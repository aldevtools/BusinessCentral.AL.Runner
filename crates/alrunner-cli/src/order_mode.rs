//! The order mode a run states (ADR-070, "The modes"): `isolated` by default, each test codeunit from the install
//! baseline with fresh SingleInstance state, in parallel; `bc` with `--strict-order`, each BC session in BC's order.
//! The header line `Order mode: ...`, the JSON field `orderMode` and the JUnit property `orderMode` on every
//! `testsuite`. Entry: `OrderMode::of`.

/// The two order modes of ADR-070.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderMode {
    Isolated,
    Bc,
}

impl OrderMode {
    /// The mode of a run whose test codeunits run in BC's order when `bc_order` holds: `--strict-order`, a test driver
    /// that runs in one session, or a stability run, whose base variant runs in BC's order.
    pub fn of(bc_order: bool) -> Self {
        match bc_order {
            true => OrderMode::Bc,
            false => OrderMode::Isolated,
        }
    }

    /// The value of `orderMode`.
    pub fn text(self) -> &'static str {
        match self {
            OrderMode::Isolated => "isolated",
            OrderMode::Bc => "bc",
        }
    }

    /// The header line.
    pub fn line(self) -> String {
        format!("Order mode: {}", self.text())
    }

    /// `orderMode` at the top of an `--output-json` document.
    pub fn json(self, doc: String) -> String {
        match serde_json::from_str::<serde_json::Value>(&doc) {
            Ok(mut d) if d.is_object() => {
                d["orderMode"] = self.text().into();
                serde_json::to_string_pretty(&d).expect("a JSON value serializes")
            }
            _ => doc,
        }
    }

    /// `<property name="orderMode" value="..." />` on every `testsuite` of a JUnit file.
    pub fn junit(self, xml: &str) -> String {
        crate::seed::with_properties(xml, &format!("<property name=\"orderMode\" value=\"{}\" />", self.text()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_goes_into_json_and_every_testsuite() {
        let j: serde_json::Value = serde_json::from_str(&OrderMode::Bc.json("{\"a\":1}".into())).unwrap();
        assert_eq!(j["orderMode"], "bc");
        let x = OrderMode::Isolated.junit("<testsuites><testsuite name=\"a\"></testsuite><testsuite name=\"b\"/></testsuites>");
        assert_eq!(x.matches("<property name=\"orderMode\" value=\"isolated\" />").count(), 2, "{x}");
    }
}
