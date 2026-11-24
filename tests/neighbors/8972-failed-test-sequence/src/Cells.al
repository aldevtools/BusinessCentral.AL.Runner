codeunit 52710 "N72 WriteThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_WriteThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72WTC10'; NumberSequence.Insert(K, 1, 1); Access.Put(K + 'A'); Commit(); end;
    [Test] procedure B_WriteThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72WTC10'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72WTC10')); end;
}

codeunit 52711 "N72 FindFirstThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_FindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72FFTC11'; NumberSequence.Insert(K, 1, 1); Access.ReadFirst(); Commit(); end;
    [Test] procedure B_FindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72FFTC11'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72FFTC11')); end;
}

codeunit 52712 "N72 UpdLockFindFirstThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_UpdLockFindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72ULFFTC12'; NumberSequence.Insert(K, 1, 1); Row.ReadIsolation(IsolationLevel::UpdLock); if Row.FindFirst() then; Commit(); end;
    [Test] procedure B_UpdLockFindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72ULFFTC12'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72ULFFTC12')); end;
}

codeunit 52713 "N72 ReadUncommittedFindFirstTh"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_ReadUncommittedFindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72RUFFTC13'; NumberSequence.Insert(K, 1, 1); Row.ReadIsolation(IsolationLevel::ReadUncommitted); if Row.FindFirst() then; Commit(); end;
    [Test] procedure B_ReadUncommittedFindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72RUFFTC13'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72RUFFTC13')); end;
}

codeunit 52714 "N72 GetThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_GetThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72GTC14'; NumberSequence.Insert(K, 1, 1); if Row.Get('X') then; Commit(); end;
    [Test] procedure B_GetThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72GTC14'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72GTC14')); end;
}

codeunit 52715 "N72 TwoFindFirstThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_TwoFindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72TFFTC15'; NumberSequence.Insert(K, 1, 1); Access.ReadFirst(); Access.ReadFirst(); Commit(); end;
    [Test] procedure B_TwoFindFirstThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72TFFTC15'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72TFFTC15')); end;
}

codeunit 52716 "N72 FindSetThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_FindSetThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72FSTC16'; NumberSequence.Insert(K, 1, 1); if Row.FindSet() then; Commit(); end;
    [Test] procedure B_FindSetThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72FSTC16'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72FSTC16')); end;
}

codeunit 52717 "N72 TempRecordThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_TempRecordThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72TRTC17'; NumberSequence.Insert(K, 1, 1); TempRow."Key" := 'T'; TempRow.Insert(); Access.Put(K + 'A'); Commit(); end;
    [Test] procedure B_TempRecordThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72TRTC17'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72TRTC17')); end;
}

codeunit 52718 "N72 RefLoadFindLastThenCommit"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_RefLoadFindLastThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72RLFLTC18'; NumberSequence.Insert(K, 1, 1); RR.Open(Database::"N72 Row"); RR.ReadIsolation(IsolationLevel::ReadUncommitted); RR.SetLoadFields(1); if RR.FindLast() then; Commit(); end;
    [Test] procedure B_RefLoadFindLastThenCommit() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72RLFLTC18'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72RLFLTC18')); end;
}

codeunit 52719 "N72 RefLoadThenCommitNoRead"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_RefLoadThenCommitNoRead() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72RLTCNR19'; NumberSequence.Insert(K, 1, 1); RR.Open(Database::"N72 Row"); RR.SetLoadFields(1); RR.Close(); Commit(); end;
    [Test] procedure B_RefLoadThenCommitNoRead() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72RLTCNR19'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72RLTCNR19')); end;
}

codeunit 52720 "N72 GuardedRunWritesFails"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_GuardedRunWritesFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72GRWF20'; NumberSequence.Insert(K, 1, 1); if not Codeunit.Run(Codeunit::"N72 Run Writes") then Error('run'); Error('A fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72GRWF20')); end;
}

codeunit 52721 "N72 GuardedRunReadsFails"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_GuardedRunReadsFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72GRRF21'; NumberSequence.Insert(K, 1, 1); if not Codeunit.Run(Codeunit::"N72 Run Reads") then Error('run'); Error('A fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72GRRF21')); end;
}

codeunit 52722 "N72 CommitWithoutAccessThenWri"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_CommitWithoutAccessThenWriteFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72CWATWF22'; NumberSequence.Insert(K, 1, 1); Commit(); end;
    [Test] procedure B_CommitWithoutAccessThenWriteFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72CWATWF22'; Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72CWATWF22')); end;
}

codeunit 52723 "N72 CommitWithoutAccessThenRef"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_CommitWithoutAccessThenRefReadFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72CWATRRF23'; NumberSequence.Insert(K, 1, 1); Commit(); end;
    [Test] procedure B_CommitWithoutAccessThenRefReadFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72CWATRRF23'; RR.Open(Database::"N72 Row"); if RR.FindLast() then; Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72CWATRRF23')); end;
}

codeunit 52724 "N72 OwnCommitThenRefReadFails"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_OwnCommitThenRefReadFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72OCTRRF24'; Access.Put(K + 'A'); Commit(); end;
    [Test] procedure B_OwnCommitThenRefReadFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72OCTRRF24'; NumberSequence.Insert(K, 1, 1); Commit(); RR.Open(Database::"N72 Row"); if RR.FindLast() then; Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72OCTRRF24')); end;
}

codeunit 52725 "N72 OwnCommitThenWriteFails"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_OwnCommitThenWriteFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72OCTWF25'; Access.Put(K + 'A'); Commit(); end;
    [Test] procedure B_OwnCommitThenWriteFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72OCTWF25'; NumberSequence.Insert(K, 1, 1); Commit(); Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72OCTWF25')); end;
}

codeunit 52726 "N72 OwnCommitThenNothingFails"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_OwnCommitThenNothingFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72OCTNF26'; Access.Put(K + 'A'); Commit(); end;
    [Test] procedure B_OwnCommitThenNothingFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72OCTNF26'; NumberSequence.Insert(K, 1, 1); Commit(); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72OCTNF26')); end;
}

codeunit 52727 "N72 InsertInFailedThenWriteFai"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_InsertInFailedThenWriteFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72IIFTWF27'; Access.Put(K + 'A'); Commit(); end;
    [Test] procedure B_InsertInFailedThenWriteFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72IIFTWF27'; NumberSequence.Insert(K, 1, 1); Access.Put(K + 'B'); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72IIFTWF27')); end;
}

codeunit 52728 "N72 InsertInFailedThenFindFirs"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_InsertInFailedThenFindFirstFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72IIFTFFF28'; Access.Put(K + 'A'); Commit(); end;
    [Test] procedure B_InsertInFailedThenFindFirstFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72IIFTFFF28'; NumberSequence.Insert(K, 1, 1); Access.ReadFirst(); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72IIFTFFF28')); end;
}

codeunit 52729 "N72 TwoInsertsAroundCommitFail"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_TwoInsertsAroundCommitFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72TIACF29'; NumberSequence.Insert(K, 1, 1); Access.Put(K + 'A'); Commit(); NumberSequence.Insert(K + 'X', 1, 1); Error('A fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72TIACF29')); end;
}

codeunit 52730 "N72 WriteInOneBodyCommitInNext"
{
    Subtype = Test;
    var Access: Codeunit "N72 Access";
    [Test] procedure A_WriteInOneBodyCommitInNextFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72WIOBCINF30'; NumberSequence.Insert(K, 1, 1); Access.Put(K + 'A'); end;
    [Test] procedure B_WriteInOneBodyCommitInNextFails() var Row: Record "N72 Row"; TempRow: Record "N72 Row" temporary; RR: RecordRef; K: Text; begin K := 'N72WIOBCINF30'; Access.ReadFirst(); Commit(); Error('B fails'); end;
    [Test] procedure C_Reads() begin Error('OBS %1', Access.Seq('N72WIOBCINF30')); end;
}
