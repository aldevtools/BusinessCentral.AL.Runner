ExceptionToStringGeneratedFrame => {
    id: "exception-tostring-generated-frame",
    assumed: "Exception.ToString() of the exception GetLastErrorObject answers after an asserterror or in a TryFunction (the shapes Y89ToStringFrames.al pins line by line) holds one more line, a frame of a compiler-generated method whose name contains `b__`. The corpus pins that there is exactly one such line, in every shape on every BC major, but not its text or where it stands: the runtime writes `   at Microsoft.Dynamics.Nav.BusinessApplication.Codeunit<id>.<<procedure>>b__0()` of the calling procedure, just before the frame that ends the trace (the `--- End of stack trace from previous location ---` line, or the TryInvokeAsync frame).",
    tests: "Corpus onprem-dotnet Y89_Asserterror_Plain, _Asserterror_Deep, _Asserterror_Divide, _Try_Deep, _Try_Divide (Y89ToStringFrames.al counts the `b__` lines and expects 1).",
    probe: "Print every line of Exception.ToString() after asserterror and in a TryFunction, the `b__` line included, with its position among the other lines, on BC 27, 28 and 29.",
}
