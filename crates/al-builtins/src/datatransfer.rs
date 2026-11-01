//! `DataTransfer`: copies table data during upgrade and install code only. The corpus pins what it
//! does anywhere else (install/TestDataTransferContracts.al): `SetTables` is accepted and staged;
//! a call before it asks for `SetTables`; a call after it says the type is for upgrade and install
//! code. Inside install code each staging and copying call goes to the VM (`Ctx::ref_op`, al-vm `datatransfer`), which keeps
//! the staged [`Plan`] in the variable and runs the copy (record/lt-as-records/AsDtInstaller.al, AsDtInstallRead.al).

use crate::native::{AlError, Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{LibraryValue, Value};

type Out = Result<Value, Signal>;

/// What a `DataTransfer` staged inside install code: the tables by number, and each Add call in order of its kind.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub source: i32,
    pub destination: i32,
    /// `AddSourceFilter`: field number, filter text, its `%n` values.
    pub source_filters: Vec<(i32, String, Vec<Value>)>,
    /// `AddDestinationFilter`: field number, filter text, its `%n` values.
    pub destination_filters: Vec<(i32, String, Vec<Value>)>,
    /// `AddFieldValue`: (source field, destination field).
    pub fields: Vec<(i32, i32)>,
    /// `AddConstantValue`: (value, destination field).
    pub constants: Vec<(Value, i32)>,
    /// `AddJoin`: (source field, destination field).
    pub joins: Vec<(i32, i32)>,
}

/// The native of method `$id` inside install code: the VM's, else `$outside`.
macro_rules! in_install {
    ($id:literal, $outside:expr) => {
        (|c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]| {
            if c.install_scope() {
                c.ref_op($id, r, a)
            } else {
                ($outside)(c, r, a)
            }
        }) as NativeFn
    };
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn error<T>(message: &str) -> Result<T, Signal> {
    Err(Signal::Error(Box::new(AlError {
        message: message.to_string(),
        // xmlport/lt-xmlport-query-datatransfer/XpTDataTransfer.al, misc/W5DataTransferOutside.al: the code is DB:Ok.
        code: "DB:Ok".into(),
        call_stack: String::new(),
        ..Default::default()
    })))
}

fn set_tables(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (Some(Value::Integer(src)), Some(Value::Integer(dst))) = (a.first(), a.get(1)) else {
        return unsupported("DataTransfer.SetTables with table numbers that are not Integers");
    };
    let (src, dst) = (*src, *dst);
    if !c.workspace_table(src) || !c.workspace_table(dst) {
        return unsupported("DataTransfer.SetTables on a table outside the workspace (validation not pinned)");
    }
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("DataTransfer method without a receiver");
    };
    *cell.borrow_mut() = LibraryValue::DataTransfer(Some((src, dst)));
    Ok(Value::Null)
}

/// Every staging and copying call: refused, with the text that says why.
fn other_call(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("DataTransfer method without a receiver");
    };
    let staged = matches!(&*cell.borrow(), LibraryValue::DataTransfer(Some(_)));
    if staged {
        error("DataTransfer is only usable during upgrade and installation code.")
    } else {
        error("SetTables must first be called before calling other methods on DataTransfer.")
    }
}

/// `UpdateAuditFields(Value)` and `UpdateAuditFields := Value`: accepted after `SetTables` even outside upgrade and install code,
/// where the other staging calls raise (misc/W5DataTransferOutside.al:SetTables_Twice_AndUpdateAuditFields,
/// misc/lt-probes-needed-2/PnBErrorsCollections.al:DataTransfer_OutsideInstall `op6`). Before `SetTables` and inside install code
/// it is not pinned.
fn update_audit_fields(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("DataTransfer method without a receiver");
    };
    let staged = matches!(&*cell.borrow(), LibraryValue::DataTransfer(Some(_)));
    if c.install_scope() || !staged {
        return unsupported("DataTransfer.UpdateAuditFields before SetTables or inside install code (not pinned)");
    }
    Ok(Value::Null)
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    if sig.receiver != "DataTransfer" {
        return None;
    }
    Some(match sig.name.as_str() {
        "SetTables" => in_install!("DataTransfer.SetTables#1", set_tables),
        "UpdateAuditFields" => update_audit_fields,
        "AddConstantValue" => in_install!("DataTransfer.AddConstantValue#1", other_call),
        "AddDestinationFilter" => in_install!("DataTransfer.AddDestinationFilter#1", other_call),
        "AddFieldValue" => in_install!("DataTransfer.AddFieldValue#1", other_call),
        "AddJoin" => in_install!("DataTransfer.AddJoin#1", other_call),
        "AddSourceFilter" => in_install!("DataTransfer.AddSourceFilter#1", other_call),
        "CopyFields" => in_install!("DataTransfer.CopyFields#1", other_call),
        "CopyRows" => in_install!("DataTransfer.CopyRows#1", other_call),
        _ => return None,
    })
}
