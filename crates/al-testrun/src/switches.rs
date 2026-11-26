//! The one rule of an on/off environment switch: on when it is set, not empty and not `0`.

/// Whether the switch `name` is on: set, not empty, not `0`.
pub fn on(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty() && v != "0")
}
