PermissionsNotEnforced => {
    id: "permissions-not-enforced",
    assumed: "Opt-in, `al-runner test --ignore-permission-mock` (serve: `ignorePermissionMock`): a test that lowers its own permissions (Permissions Mock.Assign or Set, PermissionTestHelper.AddEffectivePermissionSet) runs on with every access granted. Known wrong: BC narrows the session to the assigned sets (permissions/TestPermissionsMockEnforcement.al:28-47), so an access the sets do not grant raises an error there and passes here. A pass under this flag says nothing about permissions.",
    tests: "No Microsoft test needs it: without the flag such a test is not run. User suites whose tests lower their permissions and pass on real BC, run with the flag.",
    probe: "None that removes it: the flag is the user's choice to run without permission checks. It goes away only when permissions are enforced (ADR-027).",
}
