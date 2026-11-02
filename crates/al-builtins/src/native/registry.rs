use super::*;

pub type NativeFn = fn(&mut dyn Ctx, Option<&mut Value>, &mut [Value]) -> Result<Value, Signal>;

pub(super) const REGISTRY: &[(&str, NativeFn)] = &[
    ("System.Format#1", format_1),
    ("System.Format#2", format_2),
    ("System.FormatVariant#1", format_variant),
    ("TextBuilder.AppendVariant#1", crate::textfns::tb_append_variant),
    // `RecordRef.GetTable(V)` with a Variant argument (lowering picks it): the VM drops the record's company.
    ("RecordRef.GetTableVariant#1", |c, r, a| {
        c.ref_op("RecordRef.GetTableVariant#1", r, a)
    }),
    ("TextBuilder.AppendFieldValue#1", crate::textfns::tb_append_field_value),
    // `TestField.SetValue(V)` with a Variant argument (lowering picks it): an Enum or Option control reads a Text in it by
    // ordinal digits (pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:532-597).
    ("TestField.SetValueVariant#1", |c, r, a| {
        c.set_value_from_variant(true);
        let out = c.page(crate::page_op::PageOp::TestFieldSetValue, r, a);
        c.set_value_from_variant(false);
        out
    }),
    ("System.FormatDecimalField#1", format_decimal_field),
    ("System.FormatAutoFormatField#1", format_auto_format_field),
    ("System.OptionCaptionML#1", option_caption_ml),
    ("System.Evaluate#1", crate::evaluate::evaluate),
    (
        "System.EvaluateVariableOption#1",
        crate::evaluate::evaluate_variable_option,
    ),
    ("System.EvaluateBounded#1", crate::evaluate::evaluate_bounded),
    ("System.EvaluateOptionField#1", crate::evaluate::evaluate_option_field),
    ("System.Abs#1", abs),
    ("System.GetLastErrorText#1", get_last_error_text),
    (
        "System.GetLastErrorText#2",
        get_last_error_text_without_customer_content,
    ),
    ("System.GetLastErrorCode#1", get_last_error_code),
    ("System.GetLastErrorCallStack#1", get_last_error_call_stack),
    ("System.GetLastErrorObject#1", get_last_error_object),
    ("System.ClearLastError#1", clear_last_error),
    ("System.CodeCoverageRefresh#1", code_coverage_refresh),
    ("Dialog.Error#2", error_text),
    ("Dialog.ErrorNotLiteral#1", error_text_not_literal),
    ("Dialog.ErrorConstant#1", error_text_constant),
    ("Dialog.ErrorComputed#1", error_text_computed),
    ("Record.FieldErrorConstant#1", field_error_constant),
    ("Record.FieldErrorComputed#1", field_error_computed),
    ("Dialog.Error#1", crate::errorinfo::raise),
    ("Session.GetExecutionContext#1", execution_context),
    ("Session.GetCurrentModuleExecutionContext#1", execution_context),
    ("Session.GetModuleExecutionContext#1", module_execution_context),
    ("Dialog.Message#1", message),
    ("Dialog.Confirm#1", confirm),
    ("Dialog.StrMenu#1", str_menu),
    ("Text.StrLen#1", strlen),
    ("Text.StrPos#1", strpos),
    ("Text.StrCheckSum#1", strchecksum),
    ("Session.LogMessage#1", log_message),
    ("Session.LogMessage#2", log_message),
    ("Session.LogSecurityAudit#1", log_message),
    ("Session.LogAuditMessage#1", log_audit_message),
    ("System.Hyperlink#1", hyperlink),
    ("System.Variant2Date#1", variant_2_date),
    ("System.Variant2Time#1", variant_2_time),
    // Whether the host can create tasks is not pinned (corpus session/TestTaskSchedulerCloudBehavior.al
    // pins only that CreateTask returns a task id): a run that reaches the question is refused.
    // The URL of the web client depends on the server's address, which a host has not been asked for: a run that builds
    // one is refused (session/TestFinalCoverage.al:105-125 asks only that GetUrl does not throw).
    // Measured on real BC: session/TestEnvironmentUserTenantContracts.al:237-241.
    ("Session.CurrentExecutionMode#1", |_, _, _| {
        let members: std::sync::Arc<[String]> = crate::sysenum::option_members("ExecutionMode")
            .expect("a system option")
            .into();
        Ok(Value::Option(0, members))
    }),
    ("Session.ApplicationIdentifier#1", |_, _, _| {
        Ok(Value::Text("NAV".into()))
    }),
    ("ProductName.Full#1", |_, _, _| {
        Ok(Value::Text("Dynamics 365 Business Central".into()))
    }),
    ("ProductName.Short#1", |_, _, _| {
        Ok(Value::Text("Business Central".into()))
    }),
    ("ProductName.Marketing#1", |_, _, _| {
        Ok(Value::Text("Microsoft Dynamics 365 Business Central".into()))
    }),
    // False in a test: measured on real BC by session/TestEnvironmentUserTenantContracts.al:238
    // (Assert.IsFalse(TaskScheduler.CanCreateTask())).
    ("TaskScheduler.CanCreateTask#1", |_, _, _| Ok(Value::Boolean(false))),
    // CreateTask, TaskExists, CancelTask and SetTaskReady keep the session's tasks: `Ctx::session_op` (al-vm tasks.rs).
    ("Session.StartSession#1", start_session),
    ("Session.StartSession#2", start_session_timeout),
    ("Session.StartSession#3", start_session_timeout_first),
    ("Text.CopyStr#1", copystr),
    ("Text.StrSubstNo#1", strsubstno),
    ("Text.UpperCase#1", uppercase),
    ("Text.LowerCase#1", lowercase),
    ("Database.Commit#1", |c, _, _| c.commit().map(|_| Value::Null)),
    ("Database.SID#1", database_sid),
    ("System.Clear#1", clear),
    ("System.ArrayLen#1", array_len),
    // A progress dialog is a headless no-op in a test (handlers/TestHandlersDialogProgressNoOp.al:24-110).
    ("Dialog.Open#1", dialog_open),
    ("Dialog.Update#1", dialog_update),
    ("Dialog.Close#1", dialog_close),
    ("Dialog.HideSubsequentDialogs#1", dialog_hide_subsequent),
    // Reading the latest committed data is what this runtime always does (session/TestBCDatabaseContracts.al:170-187).
    // Telemetry goes to the server's log, which AL cannot read (ADR-009).
    ("Session.LogMessage#1", |_, _, _| Ok(Value::Null)),
    ("Session.LogMessage#2", |_, _, _| Ok(Value::Null)),
    // Unique and not null is all the corpus asks (session/TestSystemExtended.al:306-328).
    ("Guid.CreateSequentialGuid#1", |c, _, _| Ok(Value::Guid(c.new_guid()))),
    // A TextBuilder is bounded by the Integer range; the corpus needs at least a million
    // (session/TestSystemExtended.al:450-473).
    ("TextBuilder.MaxCapacity#1", |_, _, _| Ok(Value::Integer(i32::MAX))),
    ("Enum.AsInteger#1", enum_as_integer),
    ("Enum.Names#1", enum_names),
    ("Enum.Ordinals#1", enum_ordinals),
    ("Enum.FromInteger#1", enum_from_integer),
    ("Record.Init#1", |c, r, a| c.record(RecordOp::Init, r, a)),
    ("Record.Insert#1", |c, r, a| c.record(RecordOp::Insert, r, a)),
    ("Record.Insert#2", |c, r, a| c.record(RecordOp::Insert, r, a)),
    ("Record.Modify#1", |c, r, a| c.record(RecordOp::Modify, r, a)),
    ("Record.Delete#1", |c, r, a| c.record(RecordOp::Delete, r, a)),
    ("Record.DeleteAll#1", |c, r, a| c.record(RecordOp::DeleteAll, r, a)),
    ("Record.Get#1", |c, r, a| c.record(RecordOp::Get, r, a)),
    ("Record.Find#1", |c, r, a| c.record(RecordOp::Find, r, a)),
    ("Record.FindFirst#1", |c, r, a| c.record(RecordOp::FindFirst, r, a)),
    ("Record.FindLast#1", |c, r, a| c.record(RecordOp::FindLast, r, a)),
    ("Record.FindSet#1", |c, r, a| c.record(RecordOp::FindSet, r, a)),
    ("Record.FindSet#2", |c, r, a| c.record(RecordOp::FindSet, r, a)),
    ("Record.Next#1", |c, r, a| c.record(RecordOp::Next, r, a)),
    ("Record.Count#1", |c, r, a| c.record(RecordOp::Count, r, a)),
    ("Record.IsEmpty#1", |c, r, a| c.record(RecordOp::IsEmpty, r, a)),
    ("Record.SetRange#1", |c, r, a| c.record(RecordOp::SetRange, r, a)),
    ("Record.SetFilter#1", |c, r, a| c.record(RecordOp::SetFilter, r, a)),
    // Event subscriptions are session state; the VM intercepts these ids
    // (al-vm dispatch.rs). Registered so lowering accepts them.
    ("Session.ApplicationArea#1", vm_only),
    // ClearAll resets the running object's globals: the VM runs it (al-vm lib.rs).
    ("System.ClearAll#1", vm_only),
    ("Session.BindSubscription#1", vm_only),
    ("Session.UnbindSubscription#1", vm_only),
    // Codeunit.Run runs a codeunit and `IsInWriteTransaction` reads the transaction: both are
    // session state, run by the VM (al-vm transactions.rs).
    ("Codeunit.Run#1", vm_only),
    ("Codeunit.Run#2", vm_only),
    ("Codeunit.Run#3", vm_only),
    ("Database.IsInWriteTransaction#1", vm_only),
    // XmlPort import and export run triggers and write records: the VM runs them (al-vm xmlports.rs).
    ("Xmlport.Export#1", vm_only),
    ("Xmlport.Export#2", vm_only),
    ("Xmlport.Import#1", vm_only),
    ("Xmlport.Import#2", vm_only),
    ("Xmlport.Run#1", vm_only),
    ("Xmlport.Run#2", vm_only),
    ("Xmlport.SetSource#1", vm_only),
    ("Xmlport.SetDestination#1", vm_only),
    ("Xmlport.SetTableView#1", vm_only),
    ("Xmlport.FieldDelimiter#1", vm_only),
    ("Xmlport.FieldSeparator#1", vm_only),
    ("Xmlport.RecordSeparator#1", vm_only),
    ("Xmlport.TableSeparator#1", vm_only),
    ("Xmlport.TextEncoding#1", vm_only),
    ("Xmlport.Filename#1", vm_only),
    ("Xmlport.ImportFile#1", vm_only),
    ("Xmlport.CurrentPath#1", vm_only),
    ("Xmlport.Break#1", vm_only),
    ("Xmlport.Skip#1", vm_only),
    ("Xmlport.BreakUnbound#1", vm_only),
    ("Xmlport.Quit#1", vm_only),
    // Report runs execute triggers and write records: the VM runs them (al-vm reports.rs). The
    // ids not listed (SaveAsPdf, SaveAsXml and the other file forms, Preview, the layout accessors, Skip, Break, ...) need
    // output or behaviour the corpus does not pin and are refused in lowering.
    // A FilterPageBuilder keeps record buffers and renders views: the VM runs it (al-vm filterpage.rs).
    ("FilterPageBuilder.AddTable#1", vm_only),
    ("FilterPageBuilder.AddRecord#1", vm_only),
    ("FilterPageBuilder.AddRecordRef#1", vm_only),
    ("FilterPageBuilder.AddField#1", vm_only),
    ("FilterPageBuilder.AddField#2", vm_only),
    ("FilterPageBuilder.AddFieldNo#1", vm_only),
    ("FilterPageBuilder.Count#1", vm_only),
    ("FilterPageBuilder.GetView#1", vm_only),
    ("FilterPageBuilder.SetView#1", vm_only),
    ("FilterPageBuilder.Name#1", vm_only),
    ("FilterPageBuilder.PageCaption#1", vm_only),
    ("FilterPageBuilder.RunModal#1", vm_only),
    ("Report.Run#1", vm_only),
    ("Report.Run#3", vm_only),
    ("Report.RunModal#1", vm_only),
    ("Report.RunModal#3", vm_only),
    ("Report.Execute#1", vm_only),
    ("Report.Execute#3", vm_only),
    ("Report.WordXmlPart#1", vm_only),
    ("Report.WordXmlPart#2", vm_only),
    ("Report.Print#1", vm_only),
    ("Report.Print#2", vm_only),
    ("Report.RunRequestPage#1", vm_only),
    ("Report.RunRequestPage#2", vm_only),
    ("Report.SetTableView#1", vm_only),
    ("Report.UseRequestPage#1", vm_only),
    ("Report.ObjectId#1", vm_only),
    ("Report.IsReadOnly#1", vm_only),
    ("Report.Preview#1", vm_only),
    ("RequestPage.Update#1", vm_only),
    ("RequestPage.Caption#1", vm_only),
    ("RequestPage.Editable#1", vm_only),
    ("RequestPage.LookupMode#1", vm_only),
    ("RequestPage.ObjectId#1", vm_only),
    ("RequestPage.Activate#1", vm_only),
    ("Report.GetSubstituteReportId#1", vm_only),
    ("Report.DefaultLayout#1", vm_only),
    ("Report.DefaultLayout#2", vm_only),
    ("Report.RDLCLayout#1", vm_only),
    ("Report.RdlcLayout#2", vm_only),
    ("Report.WordLayout#1", vm_only),
    ("Report.WordLayout#2", vm_only),
    ("Report.ExcelLayout#1", vm_only),
    ("Report.ExcelLayout#2", vm_only),
    ("Report.Language#1", vm_only),
    ("Report.FormatRegion#1", vm_only),
    // SaveAs with a stream runs the report and writes its dataset (al-vm reports.rs); SaveAs to a file
    // name (SaveAsXml, SaveAsPdf, ...) writes a file no corpus test pins and stays refused.
    ("Report.SaveAs#1", vm_only),
    ("Report.SaveAs#2", vm_only),
    // The rendering saves with a file name run only for the reports on the allow-list of ADR-040 and are refused for the
    // others when they run. SaveAsXml with a file name stays refused in lowering.
    ("Report.SaveAsPdf#1", vm_only),
    ("Report.SaveAsPdf#2", vm_only),
    ("Report.SaveAsExcel#1", vm_only),
    ("Report.SaveAsExcel#2", vm_only),
    ("Report.SaveAsWord#1", vm_only),
    ("Report.SaveAsWord#2", vm_only),
    ("Report.SaveAsHtml#1", vm_only),
    ("Report.SaveAsHtml#2", vm_only),
    ("Report.Quit#1", vm_only),
    ("Report.ShowOutput#1", vm_only),
    ("Report.ShowOutput#2", vm_only),
    ("Report.TotalsCausedBy#1", vm_only),
    ("Report.NewPage#1", vm_only),
    ("Report.PageNo#1", vm_only),
    ("Report.PrintOnlyIfDetail#1", vm_only),
    ("Report.PaperSource#1", vm_only),
    ("Report.CreateTotals#1", vm_only),
    ("Report.CreateTotals#2", vm_only),
    ("Report.Skip#1", vm_only),
    ("Report.Break#1", vm_only),
    ("System.CreateGuid#1", |c, _, _| Ok(Value::Guid(c.new_guid()))),
    // `IsNull(DotNet)`: an unassigned DotNet variable, or a .NET call that returned null, holds
    // `Null` (ADR-014). Microsoft's Azure AD Tenant Impl. and Json Impl. ask it
    // (session/TestEnvironmentUserTenantContracts.al:204-232 reaches them).
    ("System.IsNull#1", |_, _, a| {
        Ok(Value::Boolean(match a.first() {
            Some(Value::Null) => true,
            // GetLastErrorObject without a last error (GetLastErrorObject.al:290-296).
            // A `var` parameter that stands for a [RunOnClient] variable (`dotnet_sigs::RUN_ON_CLIENT_NULL`).
            Some(Value::DotNet(o)) => matches!(
                &o.kind,
                crate::value::DotNetKind::Internal { type_name, .. }
                    if type_name == crate::dotnet_sigs::NULL_EXCEPTION || type_name == crate::dotnet_sigs::RUN_ON_CLIENT_NULL
            ),
            _ => false,
        }))
    }),
    ("System.IsNullGuid#1", |_, _, a| {
        Ok(Value::Boolean(matches!(a[0], Value::Guid(g) if g == [0; 16])))
    }),
    ("Record.Reset#1", |c, r, a| c.record(RecordOp::Reset, r, a)),
    ("Record.CalcFields#1", |c, r, a| c.record(RecordOp::CalcFields, r, a)),
    ("Record.SetCurrentKey#1", |c, r, a| {
        c.record(RecordOp::SetCurrentKey, r, a)
    }),
    ("Record.Validate#1", |c, r, a| c.record(RecordOp::Validate, r, a)),
    ("Record.Rename#1", |c, r, a| c.record(RecordOp::Rename, r, a)),
    ("Record.Truncate#1", |c, r, a| c.record(RecordOp::Truncate, r, a)),
    ("Record.CalcSums#1", |c, r, a| c.record(RecordOp::CalcSums, r, a)),
    ("Record.SetLoadFields#1", |c, r, a| {
        c.record(RecordOp::SetLoadFields, r, a)
    }),
    ("Record.AddLoadFields#1", |c, r, a| {
        c.record(RecordOp::AddLoadFields, r, a)
    }),
    ("Record.LoadFields#1", |c, r, a| c.record(RecordOp::LoadFields, r, a)),
    ("Record.SetBaseLoadFields#1", |c, r, a| {
        c.record(RecordOp::SetBaseLoadFields, r, a)
    }),
    ("Record.AreFieldsLoaded#1", |c, r, a| {
        c.record(RecordOp::AreFieldsLoaded, r, a)
    }),
    ("Record.SetAutoCalcFields#1", |c, r, a| {
        c.record(RecordOp::SetAutoCalcFields, r, a)
    }),
    // `ToText` of an Integer, a BigInteger, a Guid and a Byte: no thousands separator (`1000`, `2147483647`, `-9223372036854775808`),
    // a Guid with braces in upper case whatever the argument, a Byte as `Format` shows it (types/lt-totext-bigtext-codeunit-run/
    // P93ScalarToText.al:180-300, P93Guid.al:28-102; record/PZFindDirections.al:106).
    ("Integer.ToText#1", |_, r, _| match r.as_deref() {
        Some(Value::Integer(n)) => Ok(Value::Text(n.to_string().into())),
        other => Err(Signal::Unsupported(format!("Integer.ToText on {}", other.describe()))),
    }),
    ("BigInteger.ToText#1", |_, r, _| match r.as_deref() {
        Some(Value::BigInteger(n)) => Ok(Value::Text(n.to_string().into())),
        other => Err(Signal::Unsupported(format!(
            "BigInteger.ToText on {}",
            other.describe()
        ))),
    }),
    ("Guid.ToText#1", |_, r, _| guid_to_text(r)),
    ("Guid.ToText#2", |_, r, _| guid_to_text(r)),
    ("Byte.ToText#1", |c, r, _| byte_to_text(c, r)),
    ("Byte.ToText#2", |c, r, _| byte_to_text(c, r)),
    // `Date.ToText()` and `Boolean.ToText()` (Base Application's Customer Mgt. CalcLastPaymentInfo puts them in a Dictionary
    // of [Text, Text]): Format(value) with the default format, pinned by types/lt-conversions-hidden-parts-doctype/
    // P142ToTextLang.al:P142_Date_Language and P142_Boolean_Language.
    ("Date.ToText#1", |c, r, a| primitive_to_text(c, r, "Date", a)),
    ("Boolean.ToText#1", |c, r, a| primitive_to_text(c, r, "Boolean", a)),
    ("Decimal.ToText#1", |c, r, a| primitive_to_text(c, r, "Decimal", a)),
    ("Decimal.ToText#2", |c, r, a| primitive_to_text(c, r, "Decimal", a)),
    ("Boolean.ToText#2", |c, r, a| primitive_to_text(c, r, "Boolean", a)),
    ("Time.ToText#1", |c, r, a| primitive_to_text(c, r, "Time", a)),
    ("DateTime.ToText#1", |c, r, a| primitive_to_text(c, r, "DateTime", a)),
    ("Duration.ToText#1", |c, r, a| primitive_to_text(c, r, "Duration", a)),
    ("Variant.IsDefaultLayout#1", |_, r, _| is(r, "defaultlayout")),
    ("Variant.IsTestPermissions#1", |_, r, _| is(r, "testpermissions")),
    ("Variant.IsDotNet#1", |_, r, _| is(r, "DotNet")),
    ("Variant.IsAction#1", |_, r, _| is(r, "action")),
    ("Variant.IsClientType#1", |_, r, _| is(r, "clienttype")),
    ("Variant.IsDataClassification#1", |_, r, _| is(r, "dataclassification")),
    ("Variant.IsExecutionMode#1", |_, r, _| is(r, "executionmode")),
    ("Variant.IsObjectType#1", |_, r, _| is(r, "objecttype")),
    ("Variant.IsPromptMode#1", |_, r, _| is(r, "promptmode")),
    ("Variant.IsReportFormat#1", |_, r, _| is(r, "reportformat")),
    ("Variant.IsTableConnectionType#1", |_, r, _| {
        is(r, "tableconnectiontype")
    }),
    ("Variant.IsTextEncoding#1", |_, r, _| is(r, "textencoding")),
    // A Char is also a wide char; nothing else is (types/lt-variant-is-text-streams/LTP89IsMisc.al:WideChar_Variants: Char values
    // `WideChar,Char,`, a Byte, an Integer, a BigInteger, a Text and a two-character literal not).
    ("Variant.IsWideChar#1", |_, r, _| is(r, "Char")),
    ("Variant.IsFilterPageBuilder#1", |_, r, _| is(r, "FilterPageBuilder")),
    ("Variant.IsInStream#1", |_, r, _| is(r, "InStream")),
    ("Variant.IsOutStream#1", |_, r, _| is(r, "OutStream")),
    // `IsBinary` is No for every value the corpus puts in a Variant: numbers, text, a record, a Blob field's value, an InStream,
    // an OutStream, a TextBuilder, a FieldRef and an empty Variant (types/lt-variant-is-text-streams/LTP89IsMisc.al:Binary_*).
    ("Variant.IsBinary#1", |_, _, _| Ok(Value::Boolean(false))),
    ("Variant.IsBoolean#1", |_, r, _| is(r, "Boolean")),
    ("Variant.IsInteger#1", |_, r, _| is(r, "Integer")),
    ("Variant.IsBigInteger#1", |_, r, _| is(r, "BigInteger")),
    ("Variant.IsDecimal#1", |_, r, _| is(r, "Decimal")),
    ("Variant.IsByte#1", |_, r, _| is(r, "Byte")),
    ("Variant.IsChar#1", |_, r, _| is(r, "Char")),
    ("Variant.IsText#1", |_, r, _| is(r, "Text")),
    ("Variant.IsCode#1", |_, r, _| is(r, "Code")),
    ("Variant.IsDate#1", |_, r, _| is(r, "Date")),
    ("Variant.IsTime#1", |_, r, _| is(r, "Time")),
    ("Variant.IsDateTime#1", |_, r, _| is(r, "DateTime")),
    ("Variant.IsDuration#1", |_, r, _| is(r, "Duration")),
    ("Variant.IsGuid#1", |_, r, _| is(r, "Guid")),
    // A Media or MediaSet value put into a Variant: the media id, as a value no Is... test answers Yes for.
    ("Media.ToVariant#1", |_, _, a| {
        let Some(Value::Guid(g)) = a.first() else {
            return unsupported("a Media value that is not a media id");
        };
        let text = crate::format::format_guid(g);
        Ok(Value::Library(
            crate::library_kind::LibraryKind::Media,
            std::rc::Rc::new(std::cell::RefCell::new(crate::value::LibraryValue::Text(text))),
        ))
    }),
    // The routine that created this InStream on its own local hands it back and ends: it reads as empty
    // (streams/lt-dc-stream-leads/DcInStreamScope.al:107-166).
    ("InStream.OwnerOutOfScope#1", crate::stream::owner_out_of_scope),
    // A Label or an OptionCaption list of an app (value, app id, what is read): the value as it is, refused where a translation
    // file of the app could change it in the language in force (ADR-053 step 1, al-runner issue 5772). With a fourth argument,
    // the Label's XLIFF unit id: the text of that unit in the language in force (ADR-053 step 2).
    ("System.TranslatedText#1", |c, _, a| {
        match a {
            [value, Value::Text(app), Value::Text(what)] => {
                c.translated_app_read(app, what)?;
                return Ok(value.clone());
            }
            [value, Value::Text(app), Value::Text(what), Value::Text(xliff)] => {
                return Ok(match c.translated_app_text(app, what, xliff)? {
                    Some(text) => Value::Text(text.into()),
                    None => value.clone(),
                });
            }
            _ => {}
        }
        unsupported("System.TranslatedText with other arguments than a value, an app id, a text and an XLIFF id")
    }),
    // A TextConst whose text depends on the language in force (language code, text, ... in declaration order).
    ("System.TextConstText#1", |c, _, a| textconst_text(c.language(), a)),
    // An interface assigned from another interface variable: one that holds no implementation raises
    // (codeunit/lt-enums-interfaces/OjT71047.al:250-251, Uninitialized_AssignedFromUninitialized `af224`).
    ("Interface.AssertAssigned#1", |_, _, a| match a.first() {
        Some(Value::Null) | None => raise("Interface not initialized", "InvalidOperation"),
        Some(v) => Ok(v.clone()),
    }),
    // A Variant assigned to an Integer: a BigInteger or a Duration beyond the Integer range raises TypeConversion:Overflow
    // (types/lt-variant/VaTConvert.al:183-184; types/lt-filter-enum-relation-duration-guid/D_Duration.al:226). Everything
    // else converts as any value does.
    ("Variant.IntoInteger#1", |_, _, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        if let Value::BigInteger(n) | Value::Duration(n) = v {
            if i32::try_from(n).is_err() {
                return raise(
                    "The expression Variant cannot be type-converted to a Integer value.",
                    "TypeConversion:Overflow",
                );
            }
        }
        crate::ops::convert(v, &crate::value::RtType::Integer)
    }),
    // A Variant assigned to an Option variable or field: a BigInteger beyond the Integer range raises TypeConversion:Overflow
    // (types/lt-bigint-enum-option/BigIntToEnumOption.al:353-383 QO_Variant_ToOptionVariable, QO_Variant_ToOptionField_Assign).
    // The value is handed back unchanged; the assignment converts it.
    ("Variant.IntoOption#1", |_, _, a| {
        let v = a.first().cloned().unwrap_or(Value::Null);
        if let Value::BigInteger(n) = v {
            if i32::try_from(n).is_err() {
                return raise(
                    "The expression Variant cannot be type-converted to a Integer value.",
                    "TypeConversion:Overflow",
                );
            }
        }
        Ok(v)
    }),
    ("Variant.IsDateFormula#1", |_, r, _| is(r, "DateFormula")),
    ("Variant.IsOption#1", |_, r, _| is(r, "Option")),
    ("Variant.IsList#1", |_, r, _| is(r, "List")),
    ("Variant.IsDictionary#1", |_, r, _| is(r, "Dictionary")),
    ("Variant.IsCodeunit#1", |_, r, _| is(r, "Codeunit")),
    // RecordId, RecordRef and FieldRef variables are library handles named after their type.
    ("Variant.IsNotification#1", |_, r, _| is(r, "Notification")),
    ("Variant.IsTextBuilder#1", |_, r, _| is(r, "TextBuilder")),
    ("Variant.IsRecordId#1", |_, r, _| is(r, "RecordId")),
    ("Variant.IsRecord#1", |_, r, _| is(r, "Record")),
    ("Variant.IsRecordRef#1", |_, r, _| is(r, "RecordRef")),
    ("Variant.IsFieldRef#1", |_, r, _| is(r, "FieldRef")),
    // Permission enforcement is not modelled (`Permissions Mock`, docs/probes-needed.md): the session
    // user reads every row, so the permission filter filters nothing. Record: record/TestRecordFilter.al:
    // 406-413 pins that it does not throw. RecordRef is the same call on the same model, not pinned itself.
    ("Record.SetPermissionFilter#1", |_, _, _| Ok(Value::Null)),
    ("RecordRef.SetPermissionFilter#1", |_, _, _| Ok(Value::Null)),
    // Kinds the value model has no variant for yet are never true.
    ("Variant.IsTextConstant#1", |_, _, _| Ok(Value::Boolean(false))),
    ("Variant.IsAutomation#1", |_, _, _| Ok(Value::Boolean(false))),
    ("Variant.IsFile#1", |_, _, _| Ok(Value::Boolean(false))),
];

/// Builtins that are refused on purpose, with the reason. No corpus test pins what they answer, so
/// the lowering says so instead of "is not implemented" (hard rule 3; each has a line in
/// `docs/probes-needed.md`, "Builtins refused until a probe pins them").
/// None is refused this way now (the last one, `Xmlport.Run#1`, is pinned by corpus master ac3499d6,
/// xmlport/lt-y40-xmlport-request-page/Y40XmlPortRequestPage.al).
pub fn refusal(_id: &str) -> Option<&'static str> {
    None
}

pub(super) fn all() -> impl Iterator<Item = &'static (&'static str, NativeFn)> {
    REGISTRY
        .iter()
        .chain(crate::collections::REGISTRY)
        .chain(crate::pagebuiltins::REGISTRY)
        .chain(crate::querybuiltins::REGISTRY)
        .chain(crate::refs::REGISTRY)
        .chain(crate::refs::RECORD_REGISTRY)
        .chain(crate::datefns::REGISTRY)
        .chain(crate::mathfns::REGISTRY)
        .chain(crate::sessionfns::REGISTRY)
        .chain(crate::media::REGISTRY)
        .chain(crate::media_orphans::REGISTRY)
        .chain(crate::moduleinfo::REGISTRY)
        .chain(crate::conv::REGISTRY)
}

/// Implementations chosen per catalog signature instead of being listed by id
/// (families with many near-identical overloads: JSON, XML, streams). Built once
/// from the catalog, so a lookup is a hash probe.
pub(super) fn derived() -> &'static std::collections::HashMap<String, NativeFn> {
    static TABLE: std::sync::OnceLock<std::collections::HashMap<String, NativeFn>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let cat = crate::catalog::Catalog::shared();
        cat.signatures
            .iter()
            .filter_map(|sig| {
                crate::json::native_for(sig)
                    .or_else(|| crate::xml::native_for(sig))
                    .or_else(|| crate::stream::native_for(sig))
                    .or_else(|| crate::file::native_for(sig))
                    .or_else(|| crate::textfns::native_for(sig))
                    .or_else(|| crate::wsac::native_for(sig))
                    .or_else(|| crate::testcontexts::native_for(sig))
                    .or_else(|| crate::errorinfo::native_for(sig))
                    .or_else(|| crate::version::native_for(sig))
                    .or_else(|| crate::datatransfer::native_for(sig))
                    .or_else(|| crate::moduleinfo::native_for(sig))
                    .or_else(|| crate::sessiontypes::native_for(sig))
                    .or_else(|| crate::http::native_for(sig))
                    .map(|f| (cat.id(sig), f))
            })
            .collect()
    })
}

/// The implementations by name: the registry lists in order (the first entry for an id wins), then the
/// implementations chosen per catalog signature, then the platform's. Built once.
pub(super) fn by_name() -> &'static std::collections::HashMap<String, (Source, NativeFn)> {
    static TABLE: std::sync::OnceLock<std::collections::HashMap<String, (Source, NativeFn)>> =
        std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut t = std::collections::HashMap::new();
        for (n, (i, f)) in all().enumerate() {
            t.entry((*i).to_string()).or_insert((Source::Registry(n), *f));
        }
        for (i, f) in derived() {
            t.entry(i.clone()).or_insert((Source::Derived, *f));
        }
        for (n, (i, f)) in crate::platform_native::entries().enumerate() {
            t.entry(i.to_string()).or_insert((Source::Platform(n), f));
        }
        t
    })
}

/// Where a registered native comes from: its position in [`all`], the catalog-derived family table (one entry per id),
/// or its position in the platform-native table. The test compares the table with the scan by this, not by function
/// address: a release build may give one function several addresses (a local copy per codegen unit) or fold two
/// identical functions into one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Source {
    Registry(usize),
    Derived,
    Platform(usize),
}

/// Ids the lowering writes that are no catalog overload and have no native of their own (the VM runs them).
pub const INVENTED_IDS: &[&str] = &[
    "Record.SetCurrFieldNo#1",
    "Record.CurrFieldNo#1",
    "Media.ToVariant#1",
    "TextBuilder.AppendFieldValue#1",
    "Interface.AssertAssigned#1",
    "Variant.IntoInteger#1",
    "Variant.IntoOption#1",
    "Variant.Into#1",
    "Text.MaxStrLenOfVarParameter#1",
    "InStream.OwnerOutOfScope#1",
    "System.TranslatedText#1",
    "System.TextConstText#1",
    "Report.LayoutListExportStream#1",
];

/// Every name that gets a [`BuiltinId`]: the catalog's overloads of every version, the registered natives
/// and the invented ids. Sorted, unique. This is what `builtin_ids.rs` is generated from.
pub fn all_builtin_names() -> Vec<String> {
    let cat = crate::catalog::Catalog::shared();
    let mut names: std::collections::BTreeSet<String> = cat.signatures.iter().map(|s| cat.id(s)).collect();
    names.extend(by_name().keys().cloned());
    names.extend(INVENTED_IDS.iter().map(|s| (*s).to_string()));
    names.into_iter().collect()
}

/// The implementation of builtin `id`, if any: one array index into a table built once (the VM's own
/// builtins have no entry).
pub fn by_id(id: crate::builtin_id::BuiltinId) -> Option<NativeFn> {
    table().get(id.index()).copied().flatten().map(|(_, f)| f)
}

/// The table [`by_id`] reads: one slot per [`BuiltinId`], with where its native comes from.
pub(super) fn table() -> &'static [Option<(Source, NativeFn)>] {
    static TABLE: std::sync::OnceLock<Vec<Option<(Source, NativeFn)>>> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let names = by_name();
        crate::builtin_id::BuiltinId::all()
            .map(|i| names.get(i.name()).copied())
            .collect()
    })
}

/// The implementation registered for overload `id`, if any.
pub fn lookup(id: &str) -> Option<NativeFn> {
    crate::builtin_id::BuiltinId::of(id).and_then(by_id)
}

/// Where [`lookup`] finds the native of `id`, read from the table.
#[cfg(test)]
pub(super) fn source_in_table(id: &str) -> Option<Source> {
    crate::builtin_id::BuiltinId::of(id)
        .and_then(|i| table().get(i.index()).copied().flatten())
        .map(|(s, _)| s)
}

/// Where [`lookup`] found the native of `id` before the table: a scan in the same order. The test compares the two.
#[cfg(test)]
pub(super) fn source_by_scan(id: &str) -> Option<Source> {
    all()
        .position(|(i, _)| *i == id)
        .map(Source::Registry)
        .or_else(|| derived().contains_key(id).then_some(Source::Derived))
        .or_else(|| {
            crate::platform_native::entries()
                .position(|(i, _)| i == id)
                .map(Source::Platform)
        })
}

/// Every registered id, for the conformance test.
pub fn registered_ids() -> Vec<&'static str> {
    all()
        .map(|(i, _)| *i)
        .chain(derived().keys().map(String::as_str))
        .collect()
}
