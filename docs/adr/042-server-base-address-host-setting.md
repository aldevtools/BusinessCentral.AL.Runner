# ADR-042: The server base address behind `GetUrl` is a host setting

- Status: Accepted (owner, 2026-10-17, with the default `http://localhost:8080/BC/`)
- Date: 2026-10-17

## Context

`GetUrl` is refused today ("the server address is not a host value"). Microsoft's code needs it in places no test
chooses. With a company built from the RapidStart package (`--company-data package`), Company-Initialize calls
`Doc. Exch. Service Setup.SetURLsToDefault`, then `OAuth2Impl.GetDefaultRedirectUrl`, then `GetUrl(ClientType::Web)`
and stores the result in `Redirect URL`. The install baseline stops there, `Application Area Setup` and `Workflow`
stay unseeded and about 14000 Microsoft tests are refused (full BC 27.5 run, 2026-10-17). The backup company hides
this because it already holds the row.

The address of a server is environmental, like the user id or `--environment` (ADR-006, ADR-014 amendment
2026-10-15). It is not a BC behaviour a test can pin. The shape of the string around it is.
Owner decision 2026-10-17: "host setting + probe".

## Decision

1. **The host owns the base address.** A new defaulted method on the `al-host` identity capability, `server_base_url()`,
   returns the base address of the web client (scheme, host, port, instance path, no query). Core crates build
   nothing from the OS (ADR-006); they ask the host.
2. **Setting.** CLI: `--server-url URL` on `alrunner test`, `al-test-cli` and `alrunner serve`. Serve and embedding:
   the same value as the `serverUrl` field of the run request (additive field, protocol v2 unchanged otherwise).
   Environment variable `ALRUNNER_SERVER_URL` is the fallback. The value must be an absolute `http` or `https` URL;
   anything else is a start-up error (exit 2).
3. **Default.** `http://localhost:8080/BC/` when nothing is set. It is a stated host value, not a claim about BC: the
   run summary prints `server address: <value> (default)` or `(--server-url)`. A test whose result depends on the
   host value is, like any other, free to differ from real BC only in that value.
4. **What `GetUrl` builds from it.** Only the part that is pinned. Until the probe below lands, `GetUrl(ClientType::Web)`
   without company and without object returns the base address; every form with a company, an object, a record or
   another client type stays refused with the reason "not pinned (docs/probes-needed.md)". The base address is
   returned verbatim, so code that cuts the path (`OAuth2Impl.GetDefaultRedirectUrl`) works on a value the user chose.
5. **Marked as an assumption.** The path through `GetUrl` is the registered assumption `geturl-web-base-address` (split on 2026-11-04, see below)
   (`al-builtins/src/assumptions.rs`), so a test that passes through it is a flagged pass.

## Pinned and to be probed

| Fact | State |
|---|---|
| `GetUrl` returns a non-empty text starting with `http` for `ClientType::Web` | observed only (corpus lt-system-database-members/GetUrl*.al, `Observer`, no assertion) |
| Query parts (`company=`, `page=`, `report=`, `codeunit=`, `table=`) per call form | to be probed |
| Result without company, with an empty company, with a name needing escaping | to be probed |
| Other client types (Phone, Tablet, Desktop, Default, Background) | to be probed; stay refused |
| `GetUrl` inside a per-database install trigger (no company) | to be probed; the baseline needs it |
| Trailing `/` of the base address and `GetBaseUrl` on SaaS | to be probed; the runtime does not normalise the setting |

## Consequences

- The package company can pass Company-Initialize's address step. Other later stops are separate work.
- The default is visible in the summary, so a parity report names the address it ran with.
- The compile cache and the install-baseline cache key must include the address (it changes `Redirect URL`).
- Embedding hosts (ALchemist, LethAL) can pass their real service address.

## Conformance

- Unit test: `--server-url` parsing (valid, relative, wrong scheme, repeated).
- Test: `GetUrl(ClientType::Web)` returns exactly the configured address; a form with a company is refused.
- Test: the baseline-cache key differs for two addresses.
- Probe topic in `docs/probes-needed.md` (row `GetUrl(ClientType::Web)`).

## As built (2026-10-17)

`al_host::Identity::server_base_url()` (default `DEFAULT_SERVER_URL`), `Host::with_server_url`, `check_server_url`.
`alrunner test --server-url`, `ALRUNNER_SERVER_URL`, the `serverUrl` field of `runTests`/`execute` (set per request).
`al-test-cli` has no flag (it is the corpus tool; it uses the default). The summary prints `server address: <url> (default|--server-url|ALRUNNER_SERVER_URL)`.
The baseline cache key holds the address. `GetUrl#1` with `ClientType::Web` and one argument is built; every other form is refused.
Tests: `al-host` (default, wrappers, check), `al-vm/tests/store/hostvalues.rs` (the value, the mark, the refusals), `alrunner-cli` (arguments, summary, serve request).

## Decided without owner review (2026-10-18)

Corpus bump 22 pins the forms with a company, an object, a record and its filters (session-system/session/GetUrlShape.al,
platform/lt-system-database-members/GetUrl*.al). They build `<base>/?query`, where `<base>` is `GetUrl(ClientType::Web)` cut at
its first `?` and has no trailing slash. So the default is now `http://localhost:8080/BC` (no trailing slash), and the forms
with arguments cut a trailing slash of a user-set address before adding `/?`. `GetUrl(ClientType::Web)` alone still returns the
address as set. Other client types stay refused: their host part is not pinned (only the scheme and the query are).

## Decided without owner review (2026-10-20)

Corpus bump 32 pins `GetUrl(ClientType::Web)` alone with a host part that is not `localhost`
(session-system/session/GetUrlShape.al:283), and `GetUrl(ClientType::Default)` alone as `dynamicsnav://<host>/BC//`
(GetUrlShape.al:270). So `al-test-cli` (the corpus tool) now sets the server address `http://bcserver:8080/BC`
(`CORPUS_SERVER_URL` in `crates/al-test-cli/src/main.rs`) instead of the default. It is a host setting, as this ADR
decides; `alrunner` keeps the default `http://localhost:8080/BC`. The host name itself stays under the assumption
`geturl-web-base-address`: no corpus test pins which host BC reports, only that it is not `localhost`.

## Owner decision (2026-11-04, al-runner issue 6924)

`GetUrl(ClientType::Web)` and `ClientType::Current` carry no assumption mark: the value is the user's own `--server-url`
input, not a BC rule. The assumption `geturl-web-base-address` is split. The service forms (SOAP, OData, ODataV4, Api)
are marked `geturl-service-host-port` until corpus branch al-runtime-rs-staging-p6924c (GetUrlShape.al, BC 27.5, 28.5,
29.0: same scheme, host and instance as Web, port 7047 for SOAP and 7048 for the others) is in the pinned corpus. Current
is marked `geturl-current-is-web` until the same file pins that it answers as Web (that equality is a BC fact, the host
value is not). Default, Windows, Phone, Tablet and Desktop are marked `geturl-client-host-port`: the probe shows that
Default has the Web host with another port than Web, 7047 and 7048, so Default now uses 7046 (BC's default client
services port); the port value itself is not pinned.

Note (2026-11-04, corpus pin e89f0e53): GetUrlShape.al is now in the pinned corpus. The service forms (`:374-405`) and
Current alone and with a page (`:424`, `:429`) carry no mark any more; `geturl-service-host-port` is gone. Default and
Windows use port 7085, not 7046 (`:465-468`), with no mark. `geturl-client-host-port` now covers only Phone, Tablet and
Desktop, and `geturl-current-is-web` only Current with a company or a record.
