//! Which media and media sets the stored rows hold, for `Media.FindOrphans` and `MediaSet.FindOrphans`
//! (al-builtins `media_orphans`; media/lt-y48-find-orphans/Y48FindOrphans.al:229-410). Every table with a Media or
//! MediaSet field is read in every company; a temporary record holds nothing here (:378-391 FO_TemporaryHolder).

use crate::Session;
use al_builtins::library_kind::LibraryKind;
use al_builtins::media_orphans::MediaHolders;
use al_builtins::native::Signal;
use al_builtins::value::{RtType, Value};
use al_data::AccessContext;

impl Session {
    pub(crate) fn media_holders_impl(&mut self) -> Result<MediaHolders, Signal> {
        let mut companies = vec![self.cx.company.to_string()];
        if let Some(ct) = self.company_table() {
            for row in self.all_rows(&self.cx.clone(), &ct)? {
                if let Some(Value::Text(n) | Value::Code(n)) = row.first()
                    && !companies.iter().any(|c| c == &**n)
                {
                    companies.push(n.to_string());
                }
            }
        }
        let tables: Vec<_> = self.engine.tables.iter().flatten().cloned().collect();
        let mut held = MediaHolders::default();
        for t in tables {
            let fields: Vec<(usize, bool)> = t
                .fields
                .iter()
                .enumerate()
                .filter_map(|(i, f)| match f.ty {
                    RtType::Library(LibraryKind::Media) => Some((i, false)),
                    RtType::Library(LibraryKind::MediaSet) => Some((i, true)),
                    _ => None,
                })
                .collect();
            // A platform table nothing provides cannot be written, so it holds nothing (its reads are refused); the rows of a
            // virtual one are the platform's lists (a report layout's media is no Tenant Media row).
            let o = &self.engine.program.objects[t.object as usize];
            let provider = o.id.and_then(al_builtins::platform::provider);
            let unprovided = provider.is_none()
                && self.engine.program.apps.get(o.app as usize).is_some_and(|a| {
                    a.id.eq_ignore_ascii_case(crate::platform::objects::al_sema_platform_app_id())
                });
            let virtual_rows = matches!(provider, Some(al_builtins::platform::Provider::Virtual(_)));
            if fields.is_empty() || unprovided || virtual_rows {
                continue;
            }
            if let Some(why) = self.install_unknown(t.object) {
                return Err(Signal::Unsupported(format!(
                    "FindOrphans: the rows of {} are not known ({why})",
                    t.name
                )));
            }
            if self.is_unseeded(&t) {
                return Err(Signal::Unsupported(format!(
                    "FindOrphans: the rows of {} could not be created (ADR-013 decision 5)",
                    t.name
                )));
            }
            let scan_in: Vec<&String> = if t.per_company {
                companies.iter().collect()
            } else {
                vec![&companies[0]]
            };
            for company in scan_in {
                let cx = AccessContext {
                    company: company.as_str().into(),
                    user: self.cx.user.clone(),
                };
                for row in self.all_rows(&cx, &t)? {
                    for &(i, set) in &fields {
                        if let Some(Value::Guid(g)) = row.get(i)
                            && *g != [0; 16]
                        {
                            if set {
                                held.sets.insert(*g)
                            } else {
                                held.media.insert(*g)
                            };
                        }
                    }
                }
            }
        }
        Ok(held)
    }
}
