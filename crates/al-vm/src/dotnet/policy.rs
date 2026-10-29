//! What is refused before a `DotNet` request is sent (ADR-014 section 8). The rules are data
//! (`policy.txt`) checked in Rust; every refusal names the type or member and the rule.

/// How a request touches a type, for rules that depend on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access<'a> {
    /// A call or property access of this member (`Now`, `NewGuid`).
    Member(&'a str),
    /// A constructor call with this many arguments.
    Construct(usize),
}

#[derive(Debug, Default)]
pub struct Policy {
    deny_assembly: Vec<(String, String)>,
    deny_type: Vec<(String, String)>,
    /// `deny-type-off-windows` rules: kept in `deny_type` unless the host runs .NET on Windows.
    off_windows: Vec<String>,
    allow_type: Vec<String>,
    deny_member: Vec<(String, String, String)>,
    deny_new_no_args: Vec<(String, String)>,
    server_prefix: Vec<String>,
    pure: Vec<String>,
}

fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

impl Policy {
    /// The rules shipped with the runtime, as on a host that does not run .NET on Windows.
    pub fn builtin() -> Policy {
        Policy::builtin_for(false)
    }

    /// The rules shipped with the runtime for a .NET host on Windows (`windows`) or elsewhere. The front end that
    /// starts the sidecar decides (ADR-006: the core does not ask the OS). On Windows the `deny-type-off-windows`
    /// rules are lifted: those types run for real (ADR-027 row 14, amended 2026-10-29).
    pub fn builtin_for(windows: bool) -> Policy {
        let mut p = Policy::parse(include_str!("policy.txt")).expect("policy.txt is well formed");
        if windows {
            let lifted = std::mem::take(&mut p.off_windows);
            p.deny_type
                .retain(|(t, _)| !lifted.iter().any(|l| l.eq_ignore_ascii_case(t)));
        }
        p
    }

    pub fn parse(text: &str) -> Result<Policy, String> {
        let mut p = Policy::default();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let bad = |why: &str| format!("policy line {}: {why}: `{line}`", n + 1);
            let (head, rule) = line.split_once('|').ok_or_else(|| bad("missing `| rule`"))?;
            let (kind, subject) = head.trim().split_once(' ').ok_or_else(|| bad("missing subject"))?;
            let (subject, rule) = (subject.trim().to_string(), rule.trim().to_string());
            match kind {
                "deny-assembly" => p.deny_assembly.push((subject, rule)),
                "allow-type" => p.allow_type.push(subject),
                "deny-type" => p.deny_type.push((subject, rule)),
                "deny-type-off-windows" => {
                    p.off_windows.push(subject.clone());
                    p.deny_type.push((subject, rule));
                }
                "deny-member" => {
                    let (ty, member) = subject.rsplit_once('.').ok_or_else(|| bad("expected Type.Member"))?;
                    p.deny_member.push((ty.to_string(), member.to_string(), rule));
                }
                "deny-new-without-args" => p.deny_new_no_args.push((subject, rule)),
                "server-prefix" => p.server_prefix.push(subject),
                "pure" => p.pure.push(subject),
                other => return Err(bad(&format!("unknown rule kind `{other}`"))),
            }
        }
        Ok(p)
    }

    /// Whether a request may go to .NET. `assembly` is known for a type named in a `dotnet { }`
    /// block and not for a call on an object the host returned. `Err` is the refusal reason.
    pub fn check(&self, assembly: Option<&str>, clr_type: &str, access: Access) -> Result<(), String> {
        if let Some(asm) = assembly {
            if let Some((_, rule)) = self.deny_assembly.iter().find(|(p, _)| starts_with_ci(asm, p)) {
                return Err(format!("refused by policy: assembly '{asm}' ({rule}); {clr_type}"));
            }
        }
        let allowed = self.allow_type.iter().any(|t| t.eq_ignore_ascii_case(clr_type));
        if let Some((_, rule)) = self.deny_type.iter().filter(|_| !allowed).find(|(t, _)| {
            if t.ends_with('.') {
                starts_with_ci(clr_type, t)
            } else {
                clr_type.eq_ignore_ascii_case(t)
            }
        }) {
            return Err(format!("refused by policy: type '{clr_type}' ({rule})"));
        }
        match access {
            Access::Member(m) => {
                if let Some((_, _, rule)) = self
                    .deny_member
                    .iter()
                    .find(|(t, n, _)| t.eq_ignore_ascii_case(clr_type) && n.eq_ignore_ascii_case(m))
                {
                    return Err(format!("refused by policy: member '{clr_type}.{m}' ({rule})"));
                }
            }
            Access::Construct(0) => {
                if let Some((_, rule)) = self
                    .deny_new_no_args
                    .iter()
                    .find(|(t, _)| t.eq_ignore_ascii_case(clr_type))
                {
                    return Err(format!(
                        "refused by policy: constructing '{clr_type}' without arguments ({rule})"
                    ));
                }
            }
            Access::Construct(_) => {}
        }
        let server = assembly.is_some_and(|a| self.server_prefix.iter().any(|p| starts_with_ci(a, p)))
            || self.server_prefix.iter().any(|p| starts_with_ci(clr_type, p));
        if server && !self.pure.iter().any(|t| t.eq_ignore_ascii_case(clr_type)) {
            return Err(format!(
                "refused by policy: server class '{clr_type}' is neither registered nor classified pure"
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p() -> Policy {
        Policy::builtin()
    }

    fn refused(r: Result<(), String>, needles: &[&str]) {
        let e = r.expect_err("must be refused");
        for n in needles {
            assert!(e.contains(n), "`{e}` does not mention `{n}`");
        }
    }

    /// Owner decision 2026-10-18; pinned by corpus tests/al-language-onprem-dotnet/lt-af-dotnet/AfMailAddress.al.
    #[test]
    fn mail_address_is_allowed_and_only_it_under_system_net() {
        let p = p();
        assert!(
            p.check(Some("System"), "System.Net.Mail.MailAddress", Access::Construct(1))
                .is_ok()
        );
        assert!(
            p.check(None, "System.Net.Mail.MailAddress", Access::Member("User"))
                .is_ok()
        );
        for ty in [
            "System.Net.Mail.SmtpClient",
            "System.Net.Mail.MailMessage",
            "System.Net.Mail.MailAddressCollection",
        ] {
            refused(
                p.check(Some("System"), ty, Access::Construct(0)),
                &["refused by policy", ty],
            );
        }
    }

    #[test]
    fn standard_library_types_pass() {
        let p = p();
        assert!(
            p.check(Some("netstandard"), "System.Convert", Access::Member("ToBase64String"))
                .is_ok()
        );
        assert!(
            p.check(Some("netstandard"), "System.Text.Encoding", Access::Member("UTF8"))
                .is_ok()
        );
        assert!(p.check(None, "System.IO.MemoryStream", Access::Construct(0)).is_ok());
        assert!(
            p.check(
                Some("Newtonsoft.Json"),
                "Newtonsoft.Json.Linq.JObject",
                Access::Construct(0)
            )
            .is_ok()
        );
        assert!(p.check(None, "System.IO.Path", Access::Member("Combine")).is_ok());
    }

    #[test]
    fn server_classes_are_refused_unless_classified_pure() {
        let p = p();
        refused(
            p.check(
                Some("Microsoft.Dynamics.Nav.NavUserAccount"),
                "Microsoft.Dynamics.Nav.NavUserAccount.NavTenantSettingsHelper",
                Access::Member("IsSandbox"),
            ),
            &["NavTenantSettingsHelper", "neither registered nor classified pure"],
        );
        // An object the host returned has no assembly known here: the type name decides.
        refused(
            p.check(None, "Microsoft.Dynamics.Nav.Runtime.Foo", Access::Member("Bar")),
            &["server class"],
        );
        assert!(
            p.check(
                Some("Microsoft.Dynamics.Nav.Types"),
                "Microsoft.Dynamics.Nav.Types.Base64UrlHelper",
                Access::Member("EncodeBase64UrlString")
            )
            .is_ok()
        );
    }

    #[test]
    fn add_in_assemblies_that_reach_outside_are_refused() {
        let p = p();
        for (asm, ty) in [
            (
                "Microsoft.Exchange.WebServices.NETStandard",
                "Microsoft.Exchange.WebServices.Data.ExchangeService",
            ),
            ("Microsoft.Dynamics.Nav.CrmCustomizationHelper", "X.Y"),
            ("Microsoft.Dynamics.Nav.PowerBIEmbedded", "X.Y"),
            (
                "Azure.Security.KeyVault.Secrets",
                "Azure.Security.KeyVault.Secrets.SecretClient",
            ),
            ("MailKit", "MailKit.Net.Smtp.SmtpClient"),
        ] {
            refused(p.check(Some(asm), ty, Access::Member("M")), &["refused by policy", asm]);
        }
    }

    #[test]
    fn environment_touching_types_and_namespaces_are_refused_by_name() {
        let p = p();
        for ty in [
            "System.IO.File",
            "System.IO.FileInfo",
            "System.IO.Directory",
            "System.IO.DirectoryInfo",
            "System.IO.FileStream",
            "System.Net.WebClient",
            "System.Net.Http.HttpClient",
            "System.Net.Mail.SmtpClient",
            "System.Diagnostics.Process",
            "System.Environment",
            "System.AppDomain",
            "System.Runtime.InteropServices.Marshal",
            "Microsoft.Win32.Registry",
            "System.DirectoryServices.DirectoryEntry",
            "System.Security.Principal.WindowsIdentity",
            "System.Drawing.Bitmap",
        ] {
            refused(p.check(Some("netstandard"), ty, Access::Member("X")), &[ty]);
        }
        for (ty, m) in [
            ("System.IO.Path", "GetTempPath"),
            ("System.IO.Path", "GetTempFileName"),
            ("System.IO.Path", "GetFullPath"),
            ("System.Reflection.Assembly", "Load"),
            ("System.Reflection.Assembly", "LoadFrom"),
            ("System.Threading.Thread", "Sleep"),
        ] {
            refused(p.check(Some("netstandard"), ty, Access::Member(m)), &[ty, m]);
        }
        // Other members of the same types are fine.
        assert!(
            p.check(None, "System.Reflection.Assembly", Access::Member("GetName"))
                .is_ok()
        );
    }

    #[test]
    fn clock_and_randomness_that_bypass_the_host_are_refused_by_member() {
        let p = p();
        for (ty, m) in [
            ("System.DateTime", "Now"),
            ("System.DateTime", "UtcNow"),
            ("System.DateTime", "Today"),
            ("System.DateTimeOffset", "Now"),
            ("System.DateTimeOffset", "UtcNow"),
            ("System.Guid", "NewGuid"),
            ("System.Diagnostics.Stopwatch", "GetTimestamp"),
            ("System.TimeZoneInfo", "Local"),
        ] {
            refused(p.check(None, ty, Access::Member(m)), &[ty, m]);
        }
        refused(
            p.check(None, "System.Random", Access::Construct(0)),
            &["System.Random", "without arguments"],
        );
        assert!(
            p.check(None, "System.Random", Access::Construct(1)).is_ok(),
            "a seeded Random is deterministic"
        );
        assert!(p.check(None, "System.DateTime", Access::Member("Parse")).is_ok());
    }

    /// ADR-027 row 14 (amended by the owner, 2026-10-29): System.Drawing runs for real through the sidecar on
    /// Windows, where .NET has GDI+ (the same bytes as BC); on any other OS it stays refused with its reason.
    #[test]
    fn system_drawing_is_refused_off_windows_and_sent_to_dotnet_on_windows() {
        let off = Policy::builtin_for(false);
        for ty in [
            "System.Drawing.Bitmap",
            "System.Drawing.Drawing2D.SmoothingMode",
            "System.Drawing.Image",
        ] {
            refused(
                off.check(Some("System.Drawing"), ty, Access::Member("X")),
                &[
                    ty,
                    "needs the Windows drawing stack (AL Runner refuses it for the same reason)",
                ],
            );
            assert!(
                Policy::builtin_for(true)
                    .check(Some("System.Drawing"), ty, Access::Member("X"))
                    .is_ok()
            );
        }
        // The default is the refusing one.
        assert!(p().check(None, "System.Drawing.Bitmap", Access::Construct(2)).is_err());
        // Only the Windows-only rules are lifted on Windows.
        let on = Policy::builtin_for(true);
        refused(
            on.check(None, "System.IO.File", Access::Member("X")),
            &["System.IO.File"],
        );
        refused(
            on.check(None, "Microsoft.Win32.Registry", Access::Member("X")),
            &["Microsoft.Win32"],
        );
    }

    #[test]
    fn rules_ignore_ascii_case_and_a_bad_policy_file_is_an_error() {
        assert!(p().check(None, "system.io.file", Access::Member("x")).is_err());
        assert!(Policy::parse("deny-type X").is_err());
        assert!(Policy::parse("frobnicate X | y").is_err());
        assert!(Policy::parse("deny-member NoDot | y").is_err());
    }
}
