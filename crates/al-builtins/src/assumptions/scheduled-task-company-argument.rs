ScheduledTaskCompanyArgument => {
    id: "scheduled-task-company-argument",
    assumed: "The Company of the Scheduled Task row of a task created by TaskScheduler.CreateTask with a company argument, read in the same transaction: the argument as given when it names the session's own company in any spelling (an upper-cased CompanyName() stays upper case, so SetRange(Company, CompanyName()) does not find the row), and the current company when the argument is blank. Pinned and not part of the assumption: without a company argument Company is the current company (session-system/CnSessionLeads.al:34, :42). A company argument that names another company, existing or not, leaves Company unknown and a read of it refused.",
    tests: "No Microsoft test. A user's report: the guard SetRange(Company, CompanyName()), SetRange(\"Run Codeunit\", ...), IsEmpty, then CreateTask(Cu, 0, true, CompanyName()), measured on real BC with CompanyName(), CompanyName() and NotBefore, a blank text and UpperCase(CompanyName()) (al-runner issue 7589).",
    probe: "session-system/ScheduledTaskCompanyArg.al: CreateTask with CompanyName(), CompanyName() and NotBefore, '', UpperCase(CompanyName()) and a company that does not exist; Company of the row and whether SetRange(Company, CompanyName()) finds it.",
    awaiting: Awaiting {
        issues: &[7589],
        files: &["tests/al-language/session-system/ScheduledTaskCompanyArg.al"],
        since: "2026-11-10",
    },
}
