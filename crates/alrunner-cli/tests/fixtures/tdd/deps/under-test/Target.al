// The app under test: it declares no test codeunit, so its own call of a missing procedure stays a compile error
// with --tdd. (The folder is not named `target`: the repository ignores every `target/` folder.)
codeunit 50200 "Tdd App Target"
{
    procedure Existing(): Integer
    begin
        exit(1);
    end;
}

codeunit 50201 "Tdd Uses Missing"
{
    procedure CallsMissing()
    var
        Target: Codeunit "Tdd App Target";
    begin
        Target.NotYet();
    end;
}
