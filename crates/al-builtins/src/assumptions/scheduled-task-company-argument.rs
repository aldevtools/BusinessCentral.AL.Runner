ScheduledTaskCompanyArgument => {
    id: "scheduled-task-company-argument",
    assumed: "The Company of the Scheduled Task row of a task created by TaskScheduler.CreateTask with a company argument, in the same transaction, after the Commit and in a later test: the argument as given when it differs from the session's own company only in ASCII letter case (an upper-cased CompanyName() stays upper case, so SetRange(Company, CompanyName()) does not find the row), and the current company when the argument is blank. A read of Company, or a record method that reads rows with a filter on Company, while such a row exists marks the assumption. Pinned and not part of the assumption: without a company argument Company is the current company (session-system/CnSessionLeads.al:34, :42). A company argument that names another company, existing or not, leaves Company unknown and a read of it refused.",
    tests: "No Microsoft test. A user's report: the guard SetRange(Company, CompanyName()), SetRange(\"Run Codeunit\", ...), IsEmpty, then CreateTask(Cu, 0, true, CompanyName()), measured on real BC with CompanyName(), CompanyName() and NotBefore, a blank text and UpperCase(CompanyName()) (al-runner issue 7589).",
    probe: "session-system/ScheduledTaskCompanyArg.al: CreateTask with CompanyName(), CompanyName() and NotBefore, '', UpperCase and LowerCase of CompanyName(), another company and a company that does not exist; Company of the row and whether SetRange(Company, CompanyName()) finds it, in the same transaction, after Commit() and in a later test.",
    awaiting: Awaiting {
        issues: &[7589],
        files: &["tests/al-language/session-system/ScheduledTaskCompanyArg.al"],
        since: "2026-11-10",
    },
}
