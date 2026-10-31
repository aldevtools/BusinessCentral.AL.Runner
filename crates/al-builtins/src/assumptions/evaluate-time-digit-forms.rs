EvaluateTimeDigitForms => {
    id: "evaluate-time-digit-forms",
    assumed: "Evaluate into a Time (format 0) of six digits whose minutes or seconds are 60 or more is rejected, as an hour from 24 is. Six digits as hhmmss, one or two digits as an hour, and an hour of 24 rejected are pinned (types/lt-ax-text-format-dates/AxEvaluateText.al:44-118).",
    tests: "Calendar Management CalcTimeDelta tests (Evaluate of '085606') and Evaluate(RandomTime, Format(RandInt(23))); 11 Tests-SCM-Manufacturing tests.",
    probe: "Evaluate(T, '086000') and Evaluate(T, '085960') into a Time: the result and the return value.",
}
