//! `BigText.Write(OutStream)` as the probes of corpus tests/al-language/streams/StTBigText.al pin it
//! (WriteToStreamAndRead :267-296, WriteReadLargeText :382-400) and streams/lt-bigtext-write/BigTextWriteEdges.al.

use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
table 69180 \"BW Rec\" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
";

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS} codeunit 69181 T {{ Subtype = Test; {tests} }}");
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

// StTBigText.al:WriteToStreamAndRead
#[test]
fn write_puts_the_ascii_text_in_the_blob_and_read_gives_it_back() {
    let r = run("[Test] procedure W() var Rec: Record \"BW Rec\"; Big: BigText; Big2: BigText; Out: OutStream; Ins: InStream; T: Text; begin
            Rec.Init(); Rec.Id := 1; Rec.Insert();
            Big.AddText('big text content');
            Rec.Data.CreateOutStream(Out);
            Big.Write(Out);
            Rec.Modify();
            Rec.CalcFields(Data);
            if Rec.Data.Length() <> 16 then Error('blob len %1', Rec.Data.Length());
            Rec.Data.CreateInStream(Ins);
            Big2.Read(Ins);
            if Big2.Length() <> 16 then Error('read len');
            Big2.GetSubText(T, 1);
            if T <> 'big text content' then Error('text %1', T);
            if not Ins.EOS() then Error('eos');
         end;");
    assert_eq!(r[0].1, Outcome::Pass, "{:?}", r[0]);
}

// StTBigText.al:WriteReadLargeText
#[test]
fn write_of_a_large_text() {
    let r = run("[Test] procedure W() var Rec: Record \"BW Rec\"; Big: BigText; Big2: BigText; Out: OutStream; Ins: InStream; i: Integer; R: Text; begin
            for i := 1 to 10000 do R += '0123456789';
            Rec.Init(); Rec.Id := 1; Rec.Insert();
            Big.AddText(R);
            Rec.Data.CreateOutStream(Out);
            Big.Write(Out);
            Rec.Modify();
            Rec.CalcFields(Data);
            if Rec.Data.Length() <> 100000 then Error('blob len %1', Rec.Data.Length());
            Rec.Data.CreateInStream(Ins);
            Big2.Read(Ins);
            if Big2.Length() <> 100000 then Error('read len');
         end;");
    assert_eq!(r[0].1, Outcome::Pass, "{:?}", r[0]);
}

// BigTextWriteEdges.al:Write_NonAscii_DefaultStream: a-umlaut is one byte on a default stream.
#[test]
fn write_of_a_pinned_non_ascii_character_is_one_byte_on_a_default_stream() {
    let r = run(
        "[Test] procedure W() var Rec: Record \"BW Rec\"; Big: BigText; Out: OutStream; T: Text; begin
            Rec.Init(); Rec.Id := 1; Rec.Insert();
            T[1] := 228; Big.AddText(T);
            Rec.Data.CreateOutStream(Out);
            Big.Write(Out);
            Rec.Modify();
            Rec.CalcFields(Data);
            if Rec.Data.Length() <> 1 then Error('blob len %1', Rec.Data.Length());
         end;",
    );
    assert_eq!(r[0].1, Outcome::Pass, "{:?}", r[0]);
}

// A character no probe shows (a CJK character on a default stream) is refused.
#[test]
fn write_of_an_unpinned_character_is_refused() {
    let r = run(
        "[Test] procedure W() var Rec: Record \"BW Rec\"; Big: BigText; Out: OutStream; T: Text; begin
            Rec.Init(); Rec.Id := 1; Rec.Insert();
            T[1] := 20013; Big.AddText(T);
            Rec.Data.CreateOutStream(Out);
            Big.Write(Out);
         end;",
    );
    assert!(
        matches!(&r[0].1, Outcome::NotRun(m) if m.contains("not pinned")),
        "{:?}",
        r[0]
    );
}

// BigTextWriteEdges.al:WriteText_AfterBigTextWrite_SameStream: a later write goes after the text.
#[test]
fn a_write_after_bigtext_write_on_the_same_stream_appends() {
    let r = run(
        "[Test] procedure W() var Rec: Record \"BW Rec\"; Big: BigText; Out: OutStream; begin
            Rec.Init(); Rec.Id := 1; Rec.Insert();
            Big.AddText('abc');
            Rec.Data.CreateOutStream(Out);
            Big.Write(Out);
            Out.WriteText('def');
            Rec.Modify();
            Rec.CalcFields(Data);
            if Rec.Data.Length() <> 6 then Error('blob len %1', Rec.Data.Length());
         end;
",
    );
    assert_eq!(r[0].1, Outcome::Pass, "{:?}", r[0]);
    // A new CreateOutStream starts at 0 again, and the blob reads back.
    let r = run(
        "[Test] procedure R() var Rec: Record \"BW Rec\"; Big: BigText; Out: OutStream; Out2: OutStream; begin
            Rec.Init(); Rec.Id := 1; Rec.Insert();
            Big.AddText('abc');
            Rec.Data.CreateOutStream(Out);
            Big.Write(Out);
            Rec.Data.CreateOutStream(Out2);
            Out2.WriteText('xy');
            Rec.CalcFields(Data);
            if Rec.Data.Length() = 0 then Error('empty');
         end;",
    );
    assert_eq!(r[0].1, Outcome::Pass, "{:?}", r[0]);
}
