# Security policy

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately through GitHub: on the **Security** tab of this repository, choose **Report a vulnerability**.
Only the maintainers see the report. I reply there, and we agree on a fix and on when the problem is made public.

Please include:
- the version of AL Runner (the edge or alpha release, or the NuGet package version);
- what an attacker can do, and the steps or AL code that show it.

## Supported versions

AL Runner is in alpha. Only the newest edge release gets fixes. Older releases and v2 do not.

## Scope

AL Runner compiles and runs the AL code you give it, on your machine. Running untrusted AL code is like running any
untrusted program. A report is in scope when AL Runner does something the AL code it runs should not be able to cause, for
example reading or writing files outside what the run is given, or starting processes.
