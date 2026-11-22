//! The header of a report's dataset: the report, its app, the session, the clock and, on BC 29 and later, the
//! company block of the Company Information row (its address lines and the other company elements).

use super::{Rec, Schema};
use crate::dataset;
use crate::{Session, Signal};
use al_builtins::describe::Describe;
use al_builtins::native::unsupported;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;

impl Session {
    /// The CompanyAddressLine elements of the BC 29 dataset header for the Company Information row `rec`: Name, Name 2,
    /// Address, Address 2, the city line, County and the name of the country, each empty one left out; the Contact Person is
    /// never written (reports/dataset/AhDatasetHeader.al:98-151, writer RB :260-460).
    ///
    /// The city line follows the address format: of the Country/Region row, of the General Ledger Setup's Local Address Format
    /// without a country (:390-430, no country line then), and City+Post Code for a code without a row, whose country line is the
    /// code (:366-388). With City and Post Code both set:
    /// - Post Code+City and Blank Line+Post Code+City: the post code, a space and the city, no blank line (:309, :345);
    /// - City+Post Code: the city, a comma and the post code (:327);
    /// - City+County+Post Code: the city, a comma, the county, a space and the post code, and no County line; without a county
    ///   the city, a comma and the post code (:335);
    /// - County is a line of its own in every other format;
    /// - Custom with the rows Validate writes from City+Post Code: the city, a space and the post code, no County (:366-380).
    ///
    /// With only one of City and Post Code, or neither, only these are pinned: no country and the local format Post Code+City
    /// (the post code, a space and the city, leading spaces cut: :125-126, :152-153), and a country of the City+Post Code format
    /// with neither (:155). Everything else is refused: other partial addresses, Custom rows of another shape, an empty Name with
    /// address lines, a country row without a name, country translations.
    fn company_address_lines(&mut self, rec: &Rec, t: &al_data::TableDef) -> Result<Vec<String>, Signal> {
        let get = |name: &str| -> String {
            t.fields
                .iter()
                .position(|f| f.name.eq_ignore_ascii_case(name))
                .and_then(|i| al_builtins::native::text_of(&rec.borrow().fields[i]).map(|v| v.to_string()))
                .unwrap_or_default()
        };
        let refuse = |what: String| -> Result<Vec<String>, Signal> {
            unsupported(format!(
                "the company address lines of the dataset header: {what} (not pinned by AhDatasetHeader.al)"
            ))
        };
        let name = get("Name");
        let [name2, address, address2, city, post_code, county, country] = [
            "Name 2",
            "Address",
            "Address 2",
            "City",
            "Post Code",
            "County",
            "Country/Region Code",
        ]
        .map(get);
        const POST_CODE_CITY: i32 = 0;
        const CITY_POST_CODE: i32 = 1;
        const CITY_COUNTY_POST_CODE: i32 = 2;
        const BLANK_LINE_POST_CODE_CITY: i32 = 3;
        const CUSTOM: i32 = 13;
        let (format, country_line) =
            if country.is_empty() && city.is_empty() && post_code.is_empty() && county.is_empty() {
                // Nothing the format places: no city line in any format, as before the local format was read (AhDatasetHeader.al:
                // 155-156 `all_empty`, `only_country`).
                (POST_CODE_CITY, None)
            } else if country.is_empty() {
                match self.header_row(98, None, 57)? {
                    None => return refuse("the General Ledger Setup has no row".into()),
                    Some(Value::Option(n, _)) if (0..=3).contains(&n) => (n, None),
                    Some(v) => return refuse(format!("the Local Address Format {}", v.describe())),
                }
            } else {
                let key = || Some(Value::Code(country.clone().into()));
                match self.header_row(9, key(), 8)? {
                    // A code without a row: Country/Region's InitValue City+Post Code, the code as the country line (:382-388).
                    None => (CITY_POST_CODE, Some(country.clone())),
                    Some(Value::Enum(_, n)) if [0, 1, 2, 3, CUSTOM].contains(&n) => {
                        let country_name = match self.header_row(9, key(), 2)? {
                            Some(v) => al_builtins::native::text_of(&v).unwrap_or_default().to_string(),
                            None => String::new(),
                        };
                        if country_name.is_empty() {
                            return refuse(format!("country \"{country}\" has no name"));
                        }
                        if self.compiled_table(11).is_some() && self.header_row(11, None, 1)?.is_some() {
                            return refuse("Country/Region Translation has rows".into());
                        }
                        (n, Some(country_name))
                    }
                    Some(v) => return refuse(format!("country \"{country}\" has the Address Format {}", v.describe())),
                }
            };
        let both = !city.is_empty() && !post_code.is_empty();
        let partial_pinned = match format {
            POST_CODE_CITY => country_line.is_none(),
            CITY_POST_CODE => country_line.is_some() && city.is_empty() && post_code.is_empty(),
            _ => false,
        };
        if !both && !partial_pinned {
            return refuse(format!(
                "the address format {format} with City \"{city}\" and Post Code \"{post_code}\" (only both set is pinned)"
            ));
        }
        let (city_line, county_line) = match format {
            POST_CODE_CITY | BLANK_LINE_POST_CODE_CITY => {
                (format!("{post_code} {city}").trim_start().to_string(), county)
            }
            CITY_POST_CODE if both => (format!("{city}, {post_code}"), county),
            CITY_POST_CODE => (String::new(), county),
            CITY_COUNTY_POST_CODE if county.is_empty() => (format!("{city}, {post_code}"), String::new()),
            CITY_COUNTY_POST_CODE => (format!("{city}, {county} {post_code}"), String::new()),
            _ => {
                self.custom_address_format_pinned(&country)?;
                (format!("{city} {post_code}"), String::new())
            }
        };
        let rest: Vec<String> = [
            name2,
            address,
            address2,
            city_line,
            county_line,
            country_line.unwrap_or_default(),
        ]
        .into_iter()
        .filter(|l| !l.is_empty())
        .collect();
        if name.is_empty() && !rest.is_empty() {
            return refuse("an empty Name with address fields set".into());
        }
        Ok(std::iter::once(name).chain(rest).collect())
    }

    /// The Custom Address Format rows of `country` are the ones Validate("Address Format", Custom) writes for a country of the
    /// City+Post Code format (AhDatasetHeader.al:366-380): lines 1-7 Name, Name 2, Contact Person, Address, Address 2, a line of
    /// City and Post Code without separators, and the country. Any other shape is refused.
    fn custom_address_format_pinned(&mut self, country: &str) -> Result<(), Signal> {
        let refuse = || {
            unsupported(format!(
                "the company address lines of the dataset header: the Custom Address Format rows of country \"{country}\" are not the ones Validate writes from City+Post Code (not pinned by AhDatasetHeader.al)"
            ))
        };
        let int = |v: &Value| match v {
            Value::Integer(i) => Some(*i),
            Value::Option(i, _) => Some(*i),
            _ => None,
        };
        let code = |v: &Value| al_builtins::native::text_of(v).unwrap_or_default().to_string();
        // Field ids of Company Information: Name 2, 3 Name 2, 51 Contact Person, 4 Address, 5 Address 2, 6 City, 30 Post Code,
        // 36 Country/Region Code.
        let mut lines: Vec<(i32, i32, i32)> = self
            .header_rows(725, &[1, 2, 4, 5])?
            .iter()
            .filter(|r| code(&r[0]).eq_ignore_ascii_case(country))
            .filter_map(|r| Some((int(&r[1])?, int(&r[2])?, int(&r[3])?)))
            .collect();
        lines.sort();
        let fields: Vec<i32> = lines.iter().map(|l| l.2).collect();
        if lines.iter().map(|l| l.0).ne(0..7) || fields != [2, 3, 51, 4, 5, 0, 36] {
            return refuse();
        }
        let line_no = lines[5].1;
        let mut parts: Vec<(i32, i32, String)> = self
            .header_rows(726, &[1, 7, 3, 5, 4])?
            .iter()
            .filter(|r| code(&r[0]).eq_ignore_ascii_case(country) && int(&r[1]) == Some(line_no))
            .filter_map(|r| Some((int(&r[2])?, int(&r[3])?, code(&r[4]))))
            .collect();
        parts.sort();
        if parts != [(0, 6, String::new()), (1, 30, String::new())] {
            return refuse();
        }
        Ok(())
    }

    /// The values of fields `fields` of every row of table `table`, in key order. A table that is not compiled is refused.
    fn header_rows(&mut self, table: u32, fields: &[u32]) -> Result<Vec<Vec<Value>>, Signal> {
        let Some(obj) = self.compiled_table(table) else {
            return unsupported(format!(
                "the company address lines of the dataset header need table {table}, which is not compiled"
            ));
        };
        let t = self.engine.tables[obj].clone().expect("a compiled table has a schema");
        let mut pos = Vec::with_capacity(fields.len());
        for field in fields {
            let Some(p) = t.fields.iter().position(|f| f.id == *field) else {
                return unsupported(format!(
                    "the company address lines of the dataset header: table {table} has no field {field}"
                ));
            };
            pos.push(p);
        }
        let Value::Record(rec) = self.default_value(&al_builtins::value::RtType::Record(obj as u32, false)) else {
            unreachable!("a record type defaults to a record")
        };
        let mut rows = Vec::new();
        if al_builtins::ops::truthy(&self.record_op(RecordOp::FindSet, &rec, &mut [])?)? {
            loop {
                rows.push(pos.iter().map(|&p| rec.borrow().fields[p].clone()).collect());
                if matches!(self.record_op(RecordOp::Next, &rec, &mut [])?, Value::Integer(0)) {
                    break;
                }
            }
        }
        Ok(rows)
    }

    /// The value of field `field` of a row of table `table`: the row with primary key `key`, or the first row. `None` when
    /// the table has no such row. A table that is not compiled is refused.
    fn header_row(&mut self, table: u32, key: Option<Value>, field: u32) -> Result<Option<Value>, Signal> {
        let Some(obj) = self.compiled_table(table) else {
            return unsupported(format!(
                "the company address lines of the dataset header need table {table}, which is not compiled"
            ));
        };
        let t = self.engine.tables[obj].clone().expect("a compiled table has a schema");
        let Some(pos) = t.fields.iter().position(|f| f.id == field) else {
            return unsupported(format!(
                "the company address lines of the dataset header: table {table} has no field {field}"
            ));
        };
        let Value::Record(rec) = self.default_value(&al_builtins::value::RtType::Record(obj as u32, false)) else {
            unreachable!("a record type defaults to a record")
        };
        let found = match key {
            Some(k) => self.record_op(RecordOp::Get, &rec, &mut [k])?,
            None => self.record_op(RecordOp::FindFirst, &rec, &mut [])?,
        };
        if !al_builtins::ops::truthy(&found)? {
            return Ok(None);
        }
        let v = rec.borrow().fields[pos].clone();
        Ok(Some(v))
    }

    /// The header of the dataset: what the report, its app, the session and the clock say.
    pub(super) fn dataset_header(&mut self, schema: &Schema) -> Result<dataset::Header, Signal> {
        let engine = &schema.engine;
        let object = &engine.program.objects[schema.object];
        let report = schema.report();
        let app = engine.program.apps.get(object.app as usize);
        let ms = self.now_millis();
        let day = ms.div_euclid(86_400_000) + i64::from(al_builtins::format::day_from_civil(1970, 1, 1));
        let (y, mo, d) = al_builtins::format::civil_from_day(day as i32);
        let in_day = ms.rem_euclid(86_400_000);
        let id: String = {
            let g = self.host.identity.company_id();
            let h: String = g.iter().map(|b| format!("{b:02x}")).collect();
            format!(
                "{}-{}-{}-{}-{}",
                &h[0..8],
                &h[8..12],
                &h[12..16],
                &h[16..20],
                &h[20..32]
            )
        };
        // BC 29 and later: the company address lines of the header, all empty while Company Information has no row
        // (reports/W3ReportDataset.al:174 Header_CompanyInformation_NameAndAddress); see `company_address_lines`.
        let mut company_lines = Vec::new();
        let mut company_values: Vec<(&'static str, String)> = Vec::new();
        if self.has(al_version::Gate::ReportSaveAsXmlHeaderHasCompanyBlock) {
            if let Some(obj) = self.compiled_table(79) {
                let t = self.engine.tables[obj].clone().expect("a compiled table has a schema");
                if t.fields.iter().any(|f| f.id == 2) {
                    if let Value::Record(rec) =
                        self.default_value(&al_builtins::value::RtType::Record(obj as u32, false))
                    {
                        if al_builtins::ops::truthy(&self.record_op(RecordOp::FindFirst, &rec, &mut [])?)? {
                            company_lines = self.company_address_lines(&rec, &t)?;
                            let value = |i: usize| al_builtins::native::text_of(&rec.borrow().fields[i]);
                            // The other elements write the value of their field (text/PnBReportSaveAs.al:568-592).
                            // The logo is the Picture field's bytes in base64 as they were written, a PNG and bytes that are no
                            // picture alike, with no line breaks; empty without a picture (reports/dataset/
                            // AhDatasetHeader.al:218-248 QV_Header_CompanyLogo_PictureVariants, BC 29; AL Runner issue 5681).
                            for (element, field) in dataset::COMPANY_FIELDS {
                                let Some(i) = t.fields.iter().position(|f| f.name.eq_ignore_ascii_case(field)) else {
                                    continue;
                                };
                                if element == "CompanyLogo" {
                                    self.load_blobs(&rec, &[i])?;
                                    let logo = match &rec.borrow().fields[i] {
                                        Value::Blob(b) => {
                                            let b = b.borrow();
                                            if let Some(what) = b.render_pending.as_deref()
                                                && al_builtins::stream::is_unloaded_write_mark(what)
                                            {
                                                return Err(al_builtins::stream::render_pending_error(what));
                                            }
                                            if b.render_pending.is_some() {
                                                return unsupported(
                                                    "the CompanyLogo of the dataset header: the Picture holds content that was never produced",
                                                );
                                            }
                                            crate::sessionops::base64(&b.bytes)
                                        }
                                        other => {
                                            return unsupported(format!(
                                                "the CompanyLogo of the dataset header from {}",
                                                other.describe()
                                            ));
                                        }
                                    };
                                    if !logo.is_empty() {
                                        company_values.push((element, logo));
                                    }
                                    continue;
                                }
                                if let Some(v) = value(i).filter(|v| !v.is_empty()) {
                                    company_values.push((element, v.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(dataset::Header {
            company_lines,
            company_values,
            extension_id: app.map(|a| a.id.clone()).unwrap_or_default(),
            extension_name: app
                .map(|a| header_extension_name(&a.name, &a.publisher))
                .unwrap_or_default(),
            extension_publisher: app.map(|a| a.publisher.clone()).unwrap_or_default(),
            extension_version: app.map(|a| a.version.clone()).unwrap_or_default(),
            report_id: object.id.map_or(String::new(), |i| i.to_string()),
            report_name: object.name.clone(),
            report_caption: report.caption.clone().unwrap_or_else(|| object.name.clone()),
            about_title: report.about_title.clone().unwrap_or_default(),
            about_text: report.about_text.clone().unwrap_or_default(),
            // The manifest of both apps the corpus measured states no contextSensitiveHelpUrl, and the
            // page then states none either (xml/TestReportDatasetHelpLink.al).
            help_link: String::new(),
            company_name: self.cx.company.to_string(),
            company_id: id,
            user_name: self.host.identity.user_id(),
            now: (y, mo, d, (in_day / 3_600_000) as u32, (in_day / 60_000 % 60) as u32),
            company_block: self.has(al_version::Gate::ReportSaveAsXmlHeaderHasCompanyBlock),
        })
    }
}

/// The ExtensionName of the dataset header: the app's name, " by " and its publisher (reports/dataset/
/// ReportDatasetHeaderRaw.al:174, text/PnBReportSaveAs.al:162). An app without a publisher (only the
/// runtime's own tests build one) keeps the bare name.
pub(super) fn header_extension_name(name: &str, publisher: &str) -> String {
    if publisher.is_empty() {
        name.to_string()
    } else {
        format!("{name} by {publisher}")
    }
}
