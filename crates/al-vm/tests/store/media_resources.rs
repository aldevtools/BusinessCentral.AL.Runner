//! The platform table Media Resources (2000000182): an ordinary stored table
//! (platform/lt-platform-tables/PtCloudTables.al:303-360 QX_MediaResources_ImportStreamAndBlob).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 2000000184 "Tenant Media"
{
    fields
    {
        field(1; ID; Guid) { } field(2; Description; Text[250]) { } field(3; Content; Blob) { }
        field(4; "Mime Type"; Text[100]) { } field(5; Height; Integer) { } field(6; Width; Integer) { }
    }
    keys { key(PK; ID) { } }
}
table 2000000182 "Media Resources"
{
    fields
    {
        field(1; Code; Code[50]) { } field(2; Blob; Blob) { } field(3; "Media Reference"; Media) { }
        field(4; "MediaSet Reference"; MediaSet) { }
    }
    keys { key(PK; Code) { } }
}
codeunit 69988 R
{
    procedure Rows(): Text
    var MediaResources: Record "Media Resources"; Src: Record "Media Resources" temporary; InS: InStream; OutS: OutStream; s: Text;
    begin
        MediaResources.SetFilter(Code, 'LTQX*');
        MediaResources.DeleteAll();
        MediaResources.Reset();
        s := Format(MediaResources.Get('LTQX-MEDIA'));
        MediaResources.Init();
        MediaResources.Validate(Code, 'LTQX-MEDIA');
        Src.Blob.CreateOutStream(OutS); OutS.WriteText('png'); Src.Blob.CreateInStream(InS);
        MediaResources."Media Reference".ImportStream(InS, 'LTQX-MEDIA');
        s += Format(MediaResources.Insert(true));
        Clear(MediaResources);
        s += Format(MediaResources.Get('ltqx-media')) + Format(MediaResources."Media Reference".HasValue());
        MediaResources.Init();
        MediaResources.Code := 'LTQX-MEDIA';
        s += Format(MediaResources.Insert(true));
        MediaResources.SetFilter(Code, 'LTQX*');
        s += Format(MediaResources.Count());
        MediaResources.DeleteAll(true);
        exit(s + Format(MediaResources.Count()));
    end;

    procedure TenantMediaOfAResource(): Text
    var MediaResources: Record "Media Resources"; Src: Record "Media Resources" temporary; TenantMedia: Record "Tenant Media"; InS: InStream; OutS: OutStream;
    begin
        MediaResources.Code := 'LTQX-MEDIA';
        Src.Blob.CreateOutStream(OutS); OutS.WriteText('png'); Src.Blob.CreateInStream(InS);
        MediaResources."Media Reference".ImportStream(InS, 'LTQX-MEDIA');
        MediaResources.Insert();
        exit(Format(TenantMedia.Get(MediaResources."Media Reference".MediaId())));
    end;

    procedure ReadFirst(): Text
    var MediaResources: Record "Media Resources";
    begin
        exit(Format(MediaResources.FindFirst()));
    end;

    procedure WriteThenRead(): Text
    var MediaResources: Record "Media Resources";
    begin
        MediaResources.Code := 'A';
        MediaResources.Insert();
        exit(Format(MediaResources.Get('A')));
    end;
}
"#;

#[test]
fn media_resources_rows_are_stored() {
    assert_eq!(text(al_testkit::run_in(SRC, "R", "Rows")), "NoYesYesYesNo10");
}

// BC keeps no Tenant Media row for the media of a Media Resources row (PtCloudTables.al:352 'tenant media' =
// 'no tenant media row'); where it keeps it, and what Tenant Media shows before the Insert, is not pinned.
#[test]
fn the_tenant_media_row_of_a_media_resources_media_is_refused() {
    let r = al_testkit::run_in(SRC, "R", "TenantMediaOfAResource");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("Media Resources")),
        "{r:?}"
    );
}

// Without test data the corpus database holds 46 Media Resources rows the runtime does not have
// (PtCloudTables.al:269 QX_MediaResources_FreshTenantCodes): a read before the session's code wrote to the table is refused.
#[test]
fn a_read_of_media_resources_before_a_write_is_refused_where_the_database_holds_demo_rows() {
    let l = al_testkit::compile(SRC);
    let mut engine = al_testkit::engine(&l);
    let t = l
        .program
        .objects
        .iter()
        .position(|o| o.name == "Media Resources")
        .unwrap();
    std::sync::Arc::get_mut(&mut engine).unwrap().demo_tables.insert(t);
    let oi = l.program.objects.iter().position(|o| o.name == "R").unwrap();
    let call = |routine: &str| {
        let mut s = al_vm::Session::new(engine.clone());
        let ri = l.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == routine)
            .unwrap();
        let inst = s.instance(oi);
        s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
    };
    let r = call("ReadFirst");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("demo rows")),
        "{r:?}"
    );
    assert_eq!(text(call("WriteThenRead")), "Yes");
}
