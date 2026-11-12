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
                    // `en-US` after 1031, 1036 or 1033: the run goes on, the region reads back empty and Format keeps
                    // following the language (reports/runtime/BdRegionTests.al:942, :1036, :1065
                    // `after=... language=N region=[]`).
                    Some(1031 | 1036 | 1033) if t == "en-US" => self.report.format_region = None,
                    // `de-DE` after `CurrReport.Language := 1031` (PnC06ReportLanguage.al:58-65 `region1:de-DE`), and the pairs of
                    // `language_region_pinned`: the region is kept and reads back.
                    Some(l) if language_region_pinned(l, &t) => self.report.format_region = Some(t),
                    Some(l) => {
                        return unsupported(format!(
                            "CurrReport.FormatRegion := '{t}' after CurrReport.Language := {l} (only 'en-US' after 1031, 1036 and 1033 and the pairs BdRegionTests.al measures are pinned)"
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
            // (reports/runtime/BdRegionTests.al:582-925 BdSetter_*: `after=... region=[nl-NL]`, header, columns; :1109-1200 in a report with the property).
            // After a language a trigger set, only the pairs of `language_region_pinned` (BdSetterLanguage_*, Y89_Lang_*_D).
            Some(Value::Text(t) | Value::Code(t))
                if (t.as_str() != "en-US" || self.report.region_property)
                    && self.report.report_preview_pinned.last() == Some(&crate::state::ReportStart::SaveAs)
                    && self
                        .report
                        .curr_report_language
                        .is_none_or(|l| language_region_pinned(l, t))
                    && self.report.run_language.is_none()
                    && al_builtins::format::Region::parse_setter(t).is_some()
                    && self.report.dataset.as_ref().is_some_and(|d| d.nothing_written()) =>
            {
                let t = t.to_string();
                let r = al_builtins::format::Region::parse_setter(&t).expect("checked");
                let ds = self.report.dataset.as_mut().expect("checked");
                ds.region = Some((t.clone(), r));
                // A language that a trigger set before is pinned with this region (BdRegionTests.al:1712-1912 Y89_Lang_*_D).
                ds.language_wants_region = None;
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

    /// `CurrReport.Language := n` in a trigger of a SaveAs: the dataset names the language's culture on its top data items and in
    /// its header: `de-DE` for 1031 (reports/W3ReportObjectIdPreviewLanguage.al:605 Language_Set1031_FormatAndColumns `item
    /// language=de-DE`; reports/runtime/BdRegionTests.al:928 the header), and the cultures of [`language_culture`], whose datasets
    /// are pinned only with a region a trigger sets after the language (BdRegionTests.al:1350-1460, :1712-1912: `language="es-ES"`
    /// for 1034, `da-DK` for 1030). Another language, a language set after the first data item was written, a language of that
    /// list without its region and the Labels block in another language are not pinned: the dataset is refused when it is written.
    pub(super) fn saveas_language_set(&mut self, n: i32, running: &Inst) -> Result<(), Signal> {
        if self.report.report_preview_pinned.last() != Some(&crate::state::ReportStart::SaveAs) {
            return Ok(());
        }
        let schema = self.report_schema(running)?;
        let labels = !schema.report().labels.is_empty()
            || schema
                .report()
                .items
                .iter()
                .any(|i| i.columns.iter().any(|c| c.include_caption));
        let Some(ds) = self.report.dataset.as_mut() else {
            return Ok(());
        };
        let target = match n {
            1033 => None,
            1031 => Some("de-DE"),
            // 2067 without a region: `language="nl-BE" formatRegion="en-US"` on the data item and the column values in en-US
            // (reports/runtime/ReportLanguageNotRunFormats.al:243-247, SaveAs of a report variable and the static SaveAs).
            2067 => Some("nl-BE"),
            _ => match language_culture(n) {
                Some(c) => Some(c),
                None => {
                    ds.culture_unpinned.get_or_insert(format!(
                        "the SaveAs dataset after CurrReport.Language := {n} (only 1031 is pinned, and {} with a region set after it)",
                        "1030, 1034, 1043, 1044, 1053, 2057 and 3079"
                    ));
                    return Ok(());
                }
            },
        };
        let why = if ds.rows > 0 {
            // The same language again, row after row (streams/TestReportCurrReportLanguage.al:44 sets 1031 in every
            // OnAfterGetRecord), changes nothing.
            (ds.language.as_deref() != target)
                .then(|| format!("CurrReport.Language := {n} after the dataset's first data item was written"))
        } else {
            ds.language = target.map(str::to_string);
            ds.language_wants_region = (n != 1031 && n != 2067 && target.is_some()).then(|| {
                format!("the SaveAs dataset after CurrReport.Language := {n} without a FormatRegion set after it (not pinned)")
            });
            // Kept, not pinned: on BC 29 the company captions of the header stay en-US (refusing them would lose
            // streams/TestReportCurrReportLanguage.al:44 TestReport_SettingCurrReportLanguage_RunsAndIsReadableBack, which does
            // not read the dataset).
            (labels && target.is_some()).then(|| {
                format!("CurrReport.Language := {n} with a Labels block (its texts in that language are not pinned)")
            })
        };
        if let Some(why) = why {
            ds.culture_unpinned
                .get_or_insert(format!("the SaveAs dataset after {why}"));
        }
        Ok(())
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

/// The culture name a dataset writes for language `n` set by a trigger, for the languages pinned only together with a region set
/// after them (reports/runtime/BdRegionTests.al: :1086 `language="da-DK"`, :1350 `es-ES` for 1034, :1388 `en-GB`, :1422 `nl-NL`,
/// :1803 `sv-SE`, :1832 `nb-NO`, :1890 `de-AT`).
fn language_culture(n: i32) -> Option<&'static str> {
    Some(match n {
        1030 => "da-DK",
        1034 => "es-ES",
        1043 => "nl-NL",
        1044 => "nb-NO",
        1053 => "sv-SE",
        2057 => "en-GB",
        3079 => "de-AT",
        _ => return None,
    })
}

/// Whether `CurrReport.FormatRegion := region` after `CurrReport.Language := language` is pinned, in a report started by Run and in
/// a SaveAs (reports/runtime/BdRegionTests.al: :955 de-DE after 1031, :1082 fr-FR after 1030, :1346-1480 es-ES_tradnl after
/// 1034, en-GB after 2057, nl-NL after 1043, da-DK after 1030, :1712-1912 Y89_Lang_* da-DK, nl-NL, es-ES, sv-SE, nb-NO, en-GB and
/// de-AT after their own language). What Format writes then is decided in al-builtins format.rs `format_in_region`.
pub(crate) fn language_region_pinned(language: i32, region: &str) -> bool {
    matches!(
        (language, region),
        (1031, "de-DE")
            | (1030, "fr-FR" | "da-DK")
            | (1034, "es-ES_tradnl" | "es-ES")
            | (2057, "en-GB")
            | (1043, "nl-NL")
            | (1053, "sv-SE")
            | (1044, "nb-NO")
            | (3079, "de-AT")
    )
}
