//! The `[Native]` procedures of System.app's codeunit "MCP Utilities" that list the system tools of the MCP server features.
//!
//! Pinned through the System Application's public MCP surface (the codeunit is Internal): platform/lt-y71-mcp-config-card/
//! Y71McpConfigCard.al:76-155 (BC 29) reads the "MCP System Tool" rows the features insert from these answers. The System
//! Application is the only caller and keeps the first 250 characters of a description (`MCPDynToolModeFeature.LoadSystemTools`,
//! `MCPDataQueryToolsFeature`), so a description is pinned up to that length: `bc_actions_search`, whose stored text is 250
//! characters, is answered with those. The answer is the same on every call (:140-155). `GetParentMCPTools` reads the
//! `Api Web Service` table and stays refused.

use crate::native::{Ctx, Signal};
use crate::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

const DYNAMIC_MODE: [(&str, &str); 3] = [
    (
        "bc_actions_describe",
        "Retrieves the complete schema for a specific Business Central API action. Use this tool to understand the schema properties an action accepts before calling 'bc_actions_invoke'.",
    ),
    (
        "bc_actions_invoke",
        "Executes a Business Central API action. Use 'bc_actions_describe' first to get the action's schema, then pass matching parameters here.",
    ),
    (
        "bc_actions_search",
        "Searches for available Business Central API actions. Use this tool when the user asks about capabilities, data operations, or what actions are available. Start with this tool to discover actions, then use 'bc_actions_describe' tool to understand thei",
    ),
];

const DATA_QUERY: [(&str, &str); 4] = [
    (
        "bc_data_find_tables",
        "Find Business Central tables by name or by semantic similarity. Returns table IDs and names.",
    ),
    (
        "bc_data_get_table_relations",
        "Get outgoing or incoming relations for a Business Central table.",
    ),
    (
        "bc_data_get_table_schema",
        "Get the fields for a Business Central table. Returns compact pipe-delimited text.",
    ),
    (
        "bc_data_query",
        "Compile and validate an AL query. By default returns only the schema; set 'returnData' to true to return data rows.",
    ),
];

/// Measured on BC 29; the System Application of earlier majors has no MCP configuration (the probe is guarded with BC29PLUS).
fn dictionary(rows: &[(&str, &str)]) -> Result<Value, Signal> {
    let pairs = rows
        .iter()
        .map(|(k, v)| (Value::Text((*k).into()), Value::Text((*v).into())))
        .collect();
    Ok(Value::Dictionary(Rc::new(RefCell::new(pairs))))
}

pub(crate) fn system_tools_in_dynamic_mode(
    _: &mut dyn Ctx,
    _: Option<&mut Value>,
    _: &mut [Value],
) -> Result<Value, Signal> {
    dictionary(&DYNAMIC_MODE)
}

pub(crate) fn system_tools_in_data_query(
    _: &mut dyn Ctx,
    _: Option<&mut Value>,
    _: &mut [Value],
) -> Result<Value, Signal> {
    dictionary(&DATA_QUERY)
}
