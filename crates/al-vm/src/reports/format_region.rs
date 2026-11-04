//! `CurrReport.FormatRegion` and `CurrReport.FormatRegion := t` (moved out of `report_builtin`, which the function length ratchet caps).

use super::*;

impl Session {
    /// The getter and the setter of `FormatRegion` on the running report. Each arm cites the corpus test that pins it.
    pub(super) fn report_format_region(&mut self, args: &[Value]) -> Result<Value, Signal> {
        // A region the platform does not know raises at the assignment, in a report started by Run and in a SaveAs: the blank
        // text (`Invalid language ID supplied: 0.`, Argument), `zz-ZZ` and the neutral culture `de` (`An unsupported language was
        // selected`, ReportFormatCultureNotFound); the log stops after `before=` (reports/runtime/BdRegionTests.al:
        // 773-801 BdSetter_ZzZZ and BdSetter_Blank, :910-925 BdSetter_Neutral). `zz-ZZ` is fine as the property
        // (YReportPropsTests.al:FormatRegion_UnknownRegion_XmlOrError); other texts are not pinned.
        if let Some(Value::Text(t) | Value::Code(t)) = args.first()
            && let Some(error) = region_setter_error(t)
            && matches!(
                self.report.report_preview_pinned.last(),
                Some(crate::state::ReportStart::Run | crate::state::ReportStart::SaveAs)
            )
            && !self.report.variable_region
            && self.report.curr_report_language.is_none()
        {
            return Err(error);
        }
        match args.first() {
            Some(Value::Text(t) | Value::Code(t))
                if self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::Run) =>
            {
                let t = t.to_string();
                match self.report.curr_report_language {
                    None if t == "en-US" => self.report.format_region = None,
                    None => self.report.format_region = Some(t),
                    // `de-DE` after `CurrReport.Language := 1031` (PnC06ReportLanguage.al:58-65 `region1:de-DE`).
                    Some(1031) if t == "de-DE" => self.report.format_region = Some(t),
                    // `en-US` after 1031, 1036 or 1033: the run goes on, the region reads back empty and Format keeps
                    // following the language (reports/runtime/BdRegionTests.al:942, :1036, :1065
                    // `after=... language=N region=[]`).
                    Some(1031 | 1036 | 1033) if t == "en-US" => self.report.format_region = None,
                    // A region after its own language is kept and reads back: `es-ES_tradnl` after 1034, `en-GB` after 2057,
                    // `nl-NL` after 1043, `da-DK` after 1030 (reports/runtime/BdRegionTests.al:1346-1440, corpus
                    // master: `language=1034 region=[es-ES_tradnl]`). What Format writes then is decided in al-builtins format.rs
                    // `format_in_region`, which refuses the regions and shapes it has no pin for.
                    Some(1034) if t == "es-ES_tradnl" => self.report.format_region = Some(t),
                    Some(2057) if t == "en-GB" => self.report.format_region = Some(t),
                    Some(1043) if t == "nl-NL" => self.report.format_region = Some(t),
                    Some(1030) if t == "da-DK" => self.report.format_region = Some(t),
                    Some(l) => {
                        return unsupported(format!(
                            "CurrReport.FormatRegion := '{t}' after CurrReport.Language := {l} (only 'de-DE' after 1031, 'en-US' after 1031, 1036 and 1033, and a region after its own language for 1034, 2057, 1043 and 1030 are pinned)"
                        ));
                    }
                }
                Ok(Value::Null)
            }
            // A SaveAs that has the region of its variable: setting another one there is not pinned.
            Some(_) if self.report.variable_region => unsupported(
                "CurrReport.FormatRegion := in a SaveAs whose report variable set FormatRegion (not pinned)",
            ),
            // `CurrReport.FormatRegion :=` in a trigger of a SaveAs Xml (OnPreReport) before the first data item opens: the region of
            // the formats from then on, the `formatRegion` of the header and the data items and the culture of the columns
            // (reports/runtime/BdRegionTests.al:582-925 BdSetter_*: `after=... region=[nl-NL]`, header, columns; :1109-1200 in a report with the property). A
            // language set as well is not built (BdSetterLanguage_*).
            Some(Value::Text(t) | Value::Code(t))
                if (t.as_str() != "en-US" || self.report.region_property)
                    && self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::SaveAs)
                    && self.report.curr_report_language.is_none()
                    && self.report.run_language.is_none()
                    && al_builtins::format::Region::parse_setter(t).is_some()
                    && self.report.dataset.as_ref().is_some_and(|d| d.nothing_written()) =>
            {
                let t = t.to_string();
                let r = al_builtins::format::Region::parse_setter(&t).expect("checked");
                self.report.dataset.as_mut().expect("checked").region = Some((t.clone(), r));
                self.report.format_region = Some(t);
                // In a report with the property the new region replaces it, `en-US` included, which then reads back as typed
                // (BdRegionTests.al:1134-1150 BdSetterInPropertyReport_EnUS `region=[en-US]`): it reads like a variable's.
                if std::mem::take(&mut self.report.region_property) {
                    self.report.variable_region = true;
                }
                Ok(Value::Null)
            }
            // Outside a report started by Run a region changes the formats and the dataset of a SaveAs
            // (reports/W3ReportObjectIdPreviewLanguage.al:559-570 FormatRegion_SetDeDE_FormatAndColumns), which is not built;
            // the session's own region changes nothing.
            Some(Value::Text(t) | Value::Code(t)) if t.as_str() != "en-US" => unsupported(format!(
                "CurrReport.FormatRegion := '{t}' in a report not started by Run (its formats and dataset in that region are not built)"
            )),
            Some(Value::Text(_) | Value::Code(_)) => Ok(Value::Null),
            None if self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::Run)
                || (self.report.region_property && self.report.region_in_init.is_none()) =>
            {
                // `en-US` as the property reads back empty (BdRegionTests.al:256-271 `region=[]`).
                let region = self.report.format_region.clone().filter(|r| r != "en-US");
                Ok(Value::Text(region.unwrap_or_default().into()))
            }
            // The region a SaveAs got from its variable reads back in its triggers (W3ReportObjectIdPreviewLanguage.al:912
            // `pre ... region=[de-DE]`).
            None if self.report.variable_region || self.report.format_region.is_some() => Ok(Value::Text(
                self.report.format_region.clone().unwrap_or_default().into(),
            )),
            None => Ok(Value::Text(String::new().into())),
            Some(other) => unsupported(format!("CurrReport.FormatRegion with {}", other.describe())),
        }
    }
}

/// The error `CurrReport.FormatRegion := t` raises for a text the platform rejects, `None` for any other text.
fn region_setter_error(t: &str) -> Option<Signal> {
    match t {
        "" => Some(crate::transactions::al_error_code(
            "Invalid language ID supplied: 0.",
            "Argument",
        )),
        "zz-ZZ" | "de" => Some(crate::transactions::al_error_code(
            &format!(
                "An unsupported language was selected ('{t}' - 'Requested culture is not supported by the current platform')."
            ),
            "ReportFormatCultureNotFound",
        )),
        _ => None,
    }
}
