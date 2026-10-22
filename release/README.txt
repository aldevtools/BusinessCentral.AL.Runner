al-runner (alpha)
================

al-runner runs the tests of Business Central AL apps. It is an alpha:
it does not match AL Runner v2 yet, some tests are refused ("not run")
with a reason, and results may change between builds.

Quick start:

    al-runner --version
    al-runner test ./MyApp ./MyApp.Test

Pass both the app and its test app. Microsoft's apps are fetched from
Microsoft's artifact feed on the first run and cached. No Microsoft code is
in this archive.

Optional: tests that use the DotNet type need the .NET 8 SDK. al-runner builds
a small helper from sources inside the binary on first use.

The Windows binary is signed (Azure Trusted Signing). The macOS and Linux
binaries are not signed.
  macOS:   xattr -d com.apple.quarantine ./al-runner
  Windows: if SmartScreen still warns: "More info", then "Run anyway"
           or: Unblock-File .\al-runner.exe

The command was called alrunner up to 3.0.0-alpha.2. AL Runner v2's dotnet
tool is also called al-runner; whichever comes first on PATH runs.
al-runner prints a note (stderr, at most once a day) when it finds another
copy, and al-runner --version lists them. ALRUNNER_NO_PATH_NOTE=1 turns
this off.

Report problems, especially wrong results:
https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/new?template=alpha-feedback.md

License: MIT (see LICENSE).
