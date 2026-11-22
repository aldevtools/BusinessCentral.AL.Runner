// Enums of the global namespace: NX GKind, the enum of NX Line."G Kind" (Production 1); NX Grade, named like the enum
// of the main app's NX Graded Line.Grade (Production 8 here, 1 there).
enum 50121 "NX GKind"
{
    Extensible = true;

    value(0; " ") { }
    value(1; Production) { }
}

enum 50122 "NX Grade"
{
    value(0; " ") { }
    value(8; Production) { }
}
