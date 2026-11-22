// Dependency app: its sources must be compiled as a dependency only, never into the app under test.
codeunit 60920 "ND Lib"
{
    procedure Answer(): Integer
    begin
        exit(42);
    end;
}
