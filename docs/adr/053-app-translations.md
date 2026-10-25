# ADR-053: Translations of an app (XLIFF files) under `GlobalLanguage`

- Status: Deferred (owner, 2026-10-25): roadmap item. Step 1 only (refuse translated reads in another language) is built now; the rest waits for more user demand.
- Date: 2026-10-25
- Related: ADR-009 (observable behavior), ADR-013 (dependency apps from source), ADR-022 (compile cache), ADR-033
  (symbolic IR and stable keys), outside-user issue 5772

## Decision in short

1. **Source of translations.** An app's translations are its `Translations/*.xlf` files, except the generated
   `*.g.xlf`: from the app folder for a workspace app, from the `Translations/` entries of the `.app` or
   `*.Source.zip` for a dependency app. Each app is translated only by its own files.
2. **What reads them.** Every text the compiler writes into the generated XLIFF file: `Label` values that are not
   `Locked`, `Caption` of objects, fields, controls, actions and report labels, `OptionCaption`, enum value
   captions, and every AL read of those (`Format`/`Evaluate` of enums and options, `FieldCaption`, `TableCaption`,
   `FieldRef.Caption`, `FieldRef.OptionCaption`, `TestPage` captions, `Error`/`Message`/`ErrorInfo` built from a
   label). The text is looked up when it is read, in the language of the session at that moment.
3. **Language.** `GlobalLanguage` gives a Windows language id. The runtime maps it to a culture name with the
   Windows Language table it already has (2067 → `nl-BE`) and takes the file whose `target-language` is that
   culture. When no file or no unit matches, the source text is used. Any other fallback (for example `nl-BE` to
   `nl-NL`) is refused until a probe pins it.
4. **Build order.** Probes first (section 6), then step 1: refuse every translatable read whose app ships a file for
   the session language (this ends the silent English answers), then step 2: apply the translations.
5. **Recommendation:** option C below (look up by the XLIFF unit id at run time, ids computed at compile time,
   tables kept per app in the compile cache).

## Context

Issue 5772 (an outside user, BC 28.1 W1, checked on a real BC container on 2026-10-23): an app with the
`TranslationFile` feature ships `Translations/AL Runner Repros.nl-BE.xlf`. After `GlobalLanguage(2067)`, real BC
returns `Vrijgegeven` for `Format` of an enum value with `Caption = 'Released'`, accepts `Evaluate(Status,
'Vrijgegeven')`, and returns `Hallo wereld` for a label `'Hello world'`. al-runner returns the English texts and
`Evaluate` returns false. All three tests pass on BC.

What the runtime does today:

- `GlobalLanguage(n)` accepts any positive id (`crates/al-vm/src/hostvalues.rs:156`). Nothing reads `.xlf` files;
  `al-deps` skips `Translations/*.xlf` entries of a package as "not a resource" (`crates/al-deps/src/package.rs:459`).
- `CaptionML` and `OptionCaptionML` are built for 1033, 1031 and 1036 and refused for other languages
  (`crates/al-vm/tests/language/option_caption_ml.rs`, `docs/probes-needed.md` row "OptionCaptionML").
- Labels, captions and enum captions are lowered to the source text (`enum_captions` in `crates/al-ir/src/lib.rs:240`,
  `FormatCtx::enum_name` in `crates/al-vm/src/ctx.rs`).
- So under any language other than 1033 the runtime answers the English text without a refusal. This breaks hard
  rule 4 (a partial behavior run silently), not only the issue's three tests.

What the Microsoft apps ship (checked in the local artifact cache, BC 29.0.54011.56262):

- W1: Base Application's source zip holds only `Base Application.g.xlf`. Most other Microsoft apps (System
  Application, Business Foundation, E-Document Core, ...) hold 26 or 27 translation files each, among them
  `en-US`, `en-GB`, `de-DE`, `nl-BE`.
- US: Base Application ships `Base Application.en-US.xlf` (85 MB, 138 763 units). **5 038 units have a target that
  differs from the source**, for example "The VAT Date is not within the range of allowed VAT dates." becomes
  "The Tax Date is not within the range of allowed tax dates.". If BC applies the `en-US` file under 1033 (the probe
  in 6.2 must show this), then a US run today shows W1 texts where BC shows US texts, also in language 1033. Most
  Microsoft tests compare a label with the same label, so they do not see it. A test that compares with a literal
  does.

The trans-unit ids look like `Codeunit 414859830 - NamedType 868134460` and `Enum 2458498293 - EnumValue
2943620325 - Property 2879900210`: the element kinds and a 32-bit hash of each name. The hash function is the
compiler's. It is not FNV-1, FNV-1a or djb2 over UTF-8 or UTF-16 (checked on the issue's ids). Every generated
`.g.xlf` gives the id and, in the note `Xliff Generator`, the names it was made from (`Codeunit ALR Labels -
NamedType HelloLbl`). The `.g.xlf` files of the Microsoft apps in the local cache give about 139 000 pairs of names
and ids per app to test a hash function against (used locally only, never committed).

## Decision

### 1. Where translations come from

| App kind | Files read | Notes |
|---|---|---|
| Workspace app (folder) | `<app>/Translations/*.xlf`, not `*.g.xlf` | As BC: the compiler packs these files into the `.app` |
| Dependency app (`.app`) | `Translations/*.xlf` entries of the package | `al-deps` keeps them instead of skipping them |
| Microsoft app (`*.Source.zip`, ADR-013) | `Translations/*.xlf` entries of the zip | Cached outside the repository, as the source |
| App with no `TranslationFile` feature | its files are still read if present | Whether BC applies them is a probe (6.2, P7) |

- A file is identified by its `target-language` attribute, not by its file name.
- An app's texts are translated only by that app's files. A `tableextension` caption, an `enumextension` value
  caption or a `pageextension` control caption is translated by the extension's app.
- Two files of one app for the same language are refused at load (which one BC uses is not pinned).
- A unit with `translate="no"`, no `<target>`, or an empty `<target>` gives the source text. A `state` attribute
  (`needs-translation`, `translated`, ...) is ignored unless a probe shows that BC reads it (P8).
- A malformed file: the app is refused at load with the file name and the parser's message. Whether BC refuses
  the publish of such an app is probe P9.

### 2. Which reads use translations

Translated (when the probes listed agree):

- `Label` values without `Locked = true`, where they are read: assignment to a `Text`, `Error`, `Message`,
  `Confirm`, `StrMenu`, `StrSubstNo`, `ErrorInfo.Message`, `ErrorInfo.DetailedMessage`, `Notification.Message`. A
  `Text` variable keeps the text it got; changing the language later does not change it.
- Enum value captions: `Format`, `Evaluate`, `FieldRef.OptionCaption` of an enum field, a page control that shows
  an enum.
- `OptionCaption` of table fields, page fields and option variables: `Format`, `Evaluate`, `FieldRef.OptionCaption`.
- `Caption` of tables and fields: `TableCaption`, `FieldCaption`, `RecordRef.Caption`, `FieldRef.Caption`,
  `Format` of a `RecordId`, platform error texts that name a table or field (only the name part; the rest of a
  platform text keeps today's rules).
- `Caption` of pages, controls and actions as `TestPage` reads them; `InstructionalText`; `ToolTip` where a test
  can read it.
- Report `labels`, `IncludeCaption` captions in the dataset, request page captions.
- Object captions in `AllObjWithCaption` and the other platform tables that show a caption.
- `CaptionClassTranslate` keeps its own rules; its result is not translated again.

Stays as today or refused:

- `CaptionML` and `OptionCaptionML`: unchanged when the app has no translation file for the session language. When
  it has one, the read is refused until probe P6 shows which one BC uses.
- Platform texts that are not from AL (the runtime's own error and message texts in another language): unchanged
  rules, only pinned languages, never invented.
- `Format` of dates and numbers follows the region, not the translations (unchanged).
- Permission set, profile, entitlement and web service captions: refused when a translation exists, until probed.
- A target text longer than the label's `MaxLength`: refused (P10).
- Any language fallback that is not "exact culture, then source text": refused (P3, P4).

### 3. Language resolution

1. The language is the one in force when the text is read: `CurrReport.Language` or the report's language while a
   report runs (as `FormatCtx::language` does today), else `GlobalLanguage`.
2. Map the id to a culture name with the Windows Language table in `crates/al-vm/src/platform/host_tables.rs`.
3. Take that app's file for that culture. No file or no unit: the source text.
4. A page keeps the language it opened in for captions (as for `OptionCaptionML` today,
   `crates/al-vm/src/pages/format.rs:291`), until P5 pins otherwise.
5. Language 1033 also reads an `en-US` file when the app ships one (P2). This changes US runs (see Context).

### 4. Options

**A. Translate at compile time, one program per language.** The lowering replaces each text with the target of the
session's starting language.

- For: no run-time cost, no IR change.
- Against: wrong as soon as AL calls `GlobalLanguage` during a run, which is exactly the issue's case. A cache
  entry per language. Rejected.

**B. Match units by the names in the `Xliff Generator` note.** At load, parse each note (`Enum ALR Status -
EnumValue Rel - Property Caption`) into a path of names; the IR carries the same path for each text.

- For: no need to know the compiler's hash. Works on the issue's file.
- Against: the note is free text for tools that edit XLIFF files. Translation tools (XLIFF Sync, NAB AL Tools) keep
  it, but a hand-edited file can drop or change it, and BC itself matches by id. A file without notes would give the
  source text where BC translates. Names with ` - ` in them make the parse ambiguous.

**C. Match units by id, ids computed at compile time (recommended).** The lowering computes the XLIFF id of every
translatable text (the same id the compiler writes into `.g.xlf`) and stores it next to the source text. At load,
each app's files become a table `(language, id) → target`. A read looks up `(app, language, id)` and falls back to
the source text.

- For: matches what BC does (id lookup). One program for all languages. A `GlobalLanguage` change during a run
  works. A file without notes works.
- Against: needs the compiler's hash function. It must be found from the compiler's own output and checked against
  every `.g.xlf` in the local cache (about 139 000 pairs per Base Application build) and against the corpus app's
  `.g.xlf` from corpus CI. Until the hash is checked on all of them, step 2 stays unbuilt.
- Cross-check: when a unit's note is present and the names in it do not match the element the id points to, refuse
  the app at load (this finds a wrong hash early).

**D. Use the `.g.xlf` as a map from id to names.** Read the app's generated file to learn which id belongs to which
element, then match as in C.

- For: no hash function needed.
- Against: `.g.xlf` is a build output. Many repositories do not commit it, and `.app` packages do not always carry
  it. It also depends on the note, as B does.

**Recommendation: C.** It is the only option that matches BC's lookup and works for files from any tool. B is the
fallback if the hash cannot be found from the compiler's output in reasonable time; the owner decides then.

### 5. Seam and compile-cache impact

- **IR (additive):** a translatable text carries its XLIFF id next to the source text (labels, captions, enum and
  option captions). `Program` gets, per app, a table of translations by language. A `Ctx` lookup method with a
  default that returns the source text.
- **Changed seam (why this is an ADR):** the compile cache (ADR-022). The key of each layer adds the hashes of the
  app's translation files. A changed `.xlf` invalidates only the translation table of that app, not its IR: the
  table is its own block in the layer, decoded per language on first use. The Microsoft layer holds up to 27
  languages per app; only the languages a run reads are decoded.
- **Cost:** a run in 1033 with no `en-US` file reads no translation file. The US Base Application `en-US` file
  (85 MB) is parsed once into the cache, then read as a block.
- **Serve and incremental compile (ADR-028, ADR-029):** a change in a `.xlf` file of the workspace reloads that
  app's table and recompiles nothing.

### 6. Probe plan (before any code)

#### 6.1 Can the corpus ship a translation file?

Yes, as far as the AL tools go: with the `TranslationFile` feature, `alc` writes `Translations/<app>.g.xlf` and packs
every other `Translations/*.xlf` into the `.app`; BC loads them at publish. The writer:

1. Adds a new fixture app `al-language-translations` (no tests of its own), with `TranslationFile` in its features,
   published before `al-language` as the cross-app fixtures `al-language-xapp-*` are (`.github/workflows/ci.yml:279`).
   This is the issue's shape (translations in a dependency app) and leaves `al-language` itself unchanged.
2. Compiles once, takes the ids from the generated `.g.xlf`, and commits hand-made `nl-BE`, `nl-NL`, `de-DE` and
   `en-US` files with those ids. CI uploads the `.g.xlf` as an artifact so the runtime can test its hash against it.
3. Adds the tests to `al-language` (they call the fixture's objects).
4. Checks that `GlobalLanguage(2067)` works on the bc-linux container. If it fails or gives English there, the result
   is verified on the Windows nightly first (a possible container bug).

A later probe adds `TranslationFile` and an `.xlf` to a test app itself (the app under test translating its own
labels), once P1 has shown no side effects of the feature.

#### 6.2 Probes, in order

| Id | What the test pins |
|---|---|
| P1 | The issue's three cases: `Format` and `Evaluate` of a translated enum caption, a translated `Label`, under 2067; and the same reads under 1033 give the source |
| P2 | An `en-US` file whose target differs from the source: what `Format`, a `Label` and `FieldCaption` give under 1033 |
| P3 | Fallback: a file for `nl-NL` only, read under 2067 (`nl-BE`); a file for `nl-BE` only, read under 1043 |
| P4 | A unit with no `<target>`, an empty `<target>`, `translate="no"`; a language with no file |
| P5 | A `TestPage` opened under 1033, then `GlobalLanguage(2067)`: control caption, enum field value, `Caption` of the page |
| P6 | A field with both `CaptionML` and a translated `Caption` unit; an option field with `OptionCaptionML` and a translated `OptionCaption` |
| P7 | An app with an `.xlf` but without the `TranslationFile` feature |
| P8 | A unit with `state="needs-translation"` and a target |
| P9 | A malformed `.xlf`: does publish fail (corpus CI can only show this in a separate job; the writer may skip it and say so) |
| P10 | A target longer than the label's `MaxLength` |
| P11 | `Evaluate` of the source text (`Released`) under 2067; `Evaluate` of the Dutch text under 1033 |
| P12 | A `tableextension` field caption and an `enumextension` value caption translated by the extension's app |
| P13 | `Error(Lbl)` with a translated label: `GetLastErrorText`, and `ErrorInfo.Message` set under 2067 and read under 1033 |
| P14 | `TableCaption`, `FieldCaption`, `Format(RecordId)` and a platform error text that names a translated field (`TestField`) |
| P15 | Report `labels` and an `IncludeCaption` column under 2067 (dataset of `SaveAs`) |

P1, P2, P3, P4 and P11 come first: they decide the lookup rule. The others decide which reads are refused in step 1.

## Consequences

- Step 1 (refuse) turns silent wrong answers into refusals for every test that reads a translated text in another
  language. Microsoft tests run in 1033, so they change only if P2 shows that `en-US` files apply.
- If P2 shows that `en-US` files apply, US runs change at 1033 too: about 5 000 Base Application texts. That needs a
  before and after run of the US buckets when step 2 lands.
- The compile cache format changes (one block per app for translations). Old cache entries are rebuilt once.
- `docs/probes-needed.md` gets one row per open probe until the corpus pins it.

## Open questions for the owner

1. Accept option C (id lookup with the compiler's hash), with B as the fallback if the hash cannot be found?
2. Build step 1 (refuse translated reads in another language) right away, before the probes land?
3. If P2 shows US English texts apply under 1033: build it for the US runs in the same step, or as its own step with
   a measured bucket run?

## Conformance

- Corpus cases from 6.2 (file:line once written), starting with the issue's three tests.
- Unit tests of the hash function against the `.g.xlf` the corpus CI uploads; a local ignored test against every
  `.g.xlf` in the artifact cache.
- `crates/al-vm/tests/language/` tests for the lookup, the fallback and each refusal.
