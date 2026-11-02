//! Platform tables whose writes raise "The <table> table is read-only." (code DB:Ok).

use al_builtins::record_op::RecordOp;

/// Whether `method` on a (non-temporary) row of platform table `id` raises "The <table> table is read-only.", code DB:Ok.
/// - Windows Language, Language Selection and License Permission: Insert with and without triggers, Modify and Delete of a row
///   that exists, and DeleteAll (platform/lt-ag-contoso/AgPlatformReadOnly.al:14-55; platform/lt-platform-tables/
///   PtCloudTables.al:38-81). A Modify or Delete of a row that does not exist is not pinned (PtCloudTables.al:50-52 is the
///   error of the helper's FindFirst): the caller refuses it.
/// - Feature Key: Insert with and without triggers, of an unknown ID and of a listed one in any casing, and Delete of a listed
///   row; nothing changes (record/lt-y48-feature-key-writes/Y48FeatureKeyWrites.al:126-200). A Modify of Enabled is allowed
///   (records.rs `feature_key_read_only_change`); DeleteAll is not pinned.
/// - Metadata Permission Set Rel.: Insert, Modify and Delete of a row that exists, and DeleteAll, each through Codeunit.Run;
///   nothing changes (permissions/lt-y59-metadata-permission/Y59PermissionSetRel.al:290-306, corpus master 9cf26907).
pub(crate) fn raises(id: u32, method: RecordOp) -> bool {
    match id {
        2_000_000_045 | 2_000_000_050 | 2_000_000_043 => true,
        2_000_000_211 => matches!(method, RecordOp::Insert | RecordOp::Delete),
        2_000_000_252 => matches!(
            method,
            RecordOp::Insert | RecordOp::Modify | RecordOp::Delete | RecordOp::DeleteAll
        ),
        _ => false,
    }
}
