# AL Runner

`al-runner` runs the tests of Business Central AL apps without a BC service tier,
Docker or SQL Server.

This package is version 3: a small .NET tool around the native `al-runner`
binary. The binaries for linux-x64, linux-arm64, osx-arm64 and win-x64 are in the package,
so nothing is downloaded on the first run. Version 3 is an alpha, published as a
prerelease:

```bash
dotnet tool install --global MSDyn365BC.AL.Runner --prerelease
al-runner --version
al-runner test ./MyApp ./MyApp.Test
```

Without `--prerelease`, `dotnet tool install` gives the last version 2 release.

Documentation: https://aldev.tools/al-runner/
Report problems: https://github.com/aldevtools/BusinessCentral.AL.Runner/issues

AL Runner is an independent project. It is not affiliated with, endorsed by or sponsored by
Microsoft. Microsoft, Dynamics 365 and Business Central are trademarks of Microsoft Corporation.
