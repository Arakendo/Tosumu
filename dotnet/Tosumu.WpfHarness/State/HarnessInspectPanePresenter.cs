using System.IO;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal static class HarnessInspectPanePresenter
{
    public static HeaderPaneState BuildHeaderResetView(string message)
    {
        return new HeaderPaneState([new HeaderFieldRow("State", message)]);
    }

    public static VerifyResetPaneState BuildVerifyResetView()
    {
        return new VerifyResetPaneState(
            SummaryText: "Run verification to check page auth and B-tree integrity.",
            IssueSummaryText: "Verification has not run yet. You will be prompted only if credentials are required, then the first auth or integrity problem will surface here.",
            TreeTrustText: "Trust: verify pending.",
            Rows: [new VerifyIssueRow("-", "Run verification to surface integrity or auth problems.", HasIssue: false, IsPlaceholder: true)]);
    }

    public static VerifyPaneState BuildVerifyView(TosumuInspectVerifyPayload verify)
    {
        var summaryText =
            $"Pages checked: {verify.PagesChecked}\n" +
            $"Pages ok: {verify.PagesOk}\n" +
            $"Issue count: {verify.IssueCount}\n" +
            $"B-tree checked: {verify.Btree.Checked}\n" +
            $"B-tree ok: {verify.Btree.Ok}\n" +
            $"B-tree message: {verify.Btree.Message ?? "(none)"}";

        List<VerifyIssueRow> rows = [];
        if (verify.Issues.Count == 0)
        {
            rows.Add(new VerifyIssueRow("-", "Verification passed. No integrity or auth issues were reported.", HasIssue: false, IsPlaceholder: true));
        }
        else
        {
            rows.AddRange(verify.Issues.Select(issue =>
                new VerifyIssueRow(issue.Pgno.ToString(), issue.Description, HasIssue: true, IsPlaceholder: false)));
        }

        if (verify.IssueCount == 0 && verify.Btree.Ok)
        {
            return new VerifyPaneState(
                summaryText,
                rows,
                BadgeText: "Verified clean",
                IsClean: true,
                IssueSummaryText: $"Verified clean across {verify.PagesChecked} pages. No integrity or auth failures were reported.",
                TreeTrustText: $"Trust: verified clean across {verify.PagesChecked} pages.");
        }

        var firstIssue = verify.Issues.FirstOrDefault();
        return new VerifyPaneState(
            summaryText,
            rows,
            BadgeText: verify.IssueCount == 1 ? "1 issue found" : $"{verify.IssueCount} issues found",
            IsClean: false,
            IssueSummaryText: firstIssue is null
                ? $"Verification reported {verify.IssueCount} issue(s). Review the page results below for detail."
                : $"First issue on page {firstIssue.Pgno}: {firstIssue.Description}",
            TreeTrustText: verify.IssueCount == 1
                ? "Trust: verification found 1 issue."
                : $"Trust: verification found {verify.IssueCount} issues.");
    }

    public static PagesPaneState BuildPagesView(TosumuInspectPagesPayload pages)
    {
        List<PageSummaryRow> rows = [];
        if (pages.Pages.Count == 0)
        {
            rows.Add(new PageSummaryRow("-", "-", "-", "-", "No data pages", "The database has no inspectable data pages yet.", HasIssue: false, IsPlaceholder: true));
        }
        else
        {
            rows.AddRange(pages.Pages.Select(page =>
            {
                var hasIssue = !string.Equals(page.State, "ok", StringComparison.Ordinal) || !string.IsNullOrWhiteSpace(page.Issue);
                return new PageSummaryRow(
                    page.Pgno.ToString(),
                    page.PageTypeName ?? "-",
                    page.PageVersion?.ToString() ?? "-",
                    page.SlotCount?.ToString() ?? "-",
                    page.State.Replace('_', ' '),
                    string.IsNullOrWhiteSpace(page.Issue) ? "-" : page.Issue,
                    HasIssue: hasIssue,
                    IsPlaceholder: false);
            }));
        }

        return new PagesPaneState(rows);
    }

    public static PagesPaneState BuildPagesResetView()
    {
        return new PagesPaneState([
            new PageSummaryRow("-", "-", "-", "-", "pending", "Load header for auth-only databases or run an unlockable inspect action to populate page summaries.", HasIssue: false, IsPlaceholder: true),
        ]);
    }

    public static PagePaneState BuildPageView(TosumuInspectPagePayload page)
    {
        var summaryText =
            $"Page {page.Pgno} · {page.PageTypeName} (0x{page.PageType:X2})\n" +
            $"Version: {page.PageVersion} · Slots: {page.SlotCount}\n" +
            $"Free start: {page.FreeStart} · Free end: {page.FreeEnd} · Free bytes: {Math.Max(0, page.FreeEnd - page.FreeStart)}";

        List<PageRecordRow> rows = [];
        PageRecordRow? selectedRecord;
        if (page.Records.Count == 0)
        {
            rows.Add(new PageRecordRow("-", "-", "-", "No decoded records on this page.", "-", "Inspect a different page if you expected payload bytes.", "-", IsPlaceholder: true));
            selectedRecord = rows[0];
        }
        else
        {
            rows.AddRange(page.Records.Select(record => new PageRecordRow(
                record.Kind,
                record.Slot?.ToString() ?? "-",
                record.RecordType is null ? "-" : $"0x{record.RecordType:X2}",
                FormatAsciiPreview(record.KeyHex),
                record.KeyHex ?? "-",
                FormatAsciiPreview(record.ValueHex),
                record.ValueHex ?? "-",
                IsPlaceholder: false)));
            selectedRecord = rows.FirstOrDefault(record => !record.IsPlaceholder);
        }

        return new PagePaneState(summaryText, rows, selectedRecord);
    }

    public static PagePaneState BuildPageResetView()
    {
        var row = new PageRecordRow("-", "-", "-", "Select a page or inspect root to decode records.", "-", "Turn to a different page to compare record payloads.", "-", IsPlaceholder: true);
        return new PagePaneState(
            SummaryText: "Select a page or inspect root to decode the current page.",
            Rows: [row],
            SelectedRecord: null);
    }

    public static ProtectorsPaneState BuildProtectorsView(TosumuInspectProtectorsPayload protectors)
    {
        List<ProtectorSlotRow> rows = [];
        if (protectors.Slots.Count == 0)
        {
            rows.Add(new ProtectorSlotRow("-", "No user-visible protectors reported.", "-"));
        }
        else
        {
            rows.AddRange(protectors.Slots.Select(slot =>
                new ProtectorSlotRow(slot.Slot.ToString(), slot.Kind, slot.KindByte.ToString())));
        }

        return new ProtectorsPaneState(rows);
    }

    public static ProtectorsPaneState BuildProtectorsResetView()
    {
        return new ProtectorsPaneState([
            new ProtectorSlotRow("-", "Load protectors to inspect user-visible keyslots.", "-"),
        ]);
    }

    public static WalPaneState BuildWalView(TosumuInspectWalPayload wal)
    {
        var summaryText = wal.WalExists
            ? $"WAL sidecar present with {wal.RecordCount} record(s)."
            : "No WAL sidecar is present for this database.";

        List<WalRecordRow> rows = [];
        if (!wal.WalExists)
        {
            rows.Add(new WalRecordRow("-", "none", "-", "No WAL sidecar detected.", IsPlaceholder: true));
        }
        else if (wal.Records.Count == 0)
        {
            rows.Add(new WalRecordRow("-", "empty", "-", "WAL exists but contains no readable records.", IsPlaceholder: true));
        }
        else
        {
            rows.AddRange(wal.Records.Select(record =>
            {
                var detail = record.Kind switch
                {
                    "begin" => $"txn {record.TxnId}",
                    "page_write" => $"page {record.Pgno} version {record.PageVersion}",
                    "commit" => $"txn {record.TxnId}",
                    "checkpoint" => $"up to LSN {record.UpToLsn}",
                    _ => "-",
                };

                return new WalRecordRow(
                    record.Lsn.ToString(),
                    record.Kind.Replace('_', ' '),
                    record.Pgno?.ToString() ?? "-",
                    detail,
                    IsPlaceholder: false);
            }));
        }

        return new WalPaneState(summaryText, rows);
    }

    public static WalPaneState BuildWalResetView()
    {
        return new WalPaneState(
            SummaryText: "Load a database to inspect the WAL sidecar if one is present.",
            Rows: [new WalRecordRow("-", "pending", "-", "Open or refresh a database to load WAL state.", IsPlaceholder: true)]);
    }

    public static RecordSelectionState BuildSelectedRecordView(PageRecordRow? record)
    {
        if (record is null)
        {
            return new RecordSelectionState(
                "Select a record",
                "Choose a row in the page records list to inspect its decoded payload.");
        }

        return new RecordSelectionState(
            record.IsPlaceholder ? "No record payload selected" : $"{record.Kind} · slot {record.Slot} · type {record.RecordType}",
            $"Key: {DescribePayload(record.KeyPreview, record.KeyHex)}\n" +
            $"Value: {DescribePayload(record.ValuePreview, record.ValueHex)}");
    }

    public static string BuildVerifyStatusText(string path, TosumuInspectVerifyPayload verify)
    {
        var fileName = Path.GetFileName(path);
        return verify.IssueCount == 0 && verify.Btree.Ok
            ? $"Verified {fileName}: all {verify.PagesChecked} pages passed integrity and auth checks."
            : $"Verified {fileName}: {verify.IssueCount} issue(s) reported across {verify.PagesChecked} pages.";
    }

    private static string DescribePayload(string preview, string hex)
    {
        var byteCount = TryGetHexByteCount(hex);
        if (preview == "-" || preview == "(empty)")
        {
            return byteCount is null ? preview : $"{preview} ({byteCount} bytes)";
        }

        if (preview == "(binary)" || preview == "(invalid)")
        {
            return byteCount is null ? preview : $"{preview} ({byteCount} bytes, {hex})";
        }

        return byteCount is null ? $"\"{preview}\"" : $"\"{preview}\" ({byteCount} bytes, {hex})";
    }

    private static int? TryGetHexByteCount(string hex)
    {
        if (string.IsNullOrWhiteSpace(hex) || hex == "-" || (hex.Length % 2) != 0)
        {
            return null;
        }

        return hex.All(Uri.IsHexDigit) ? hex.Length / 2 : null;
    }

    private static string FormatAsciiPreview(string? hex)
    {
        if (string.IsNullOrWhiteSpace(hex) || hex == "-")
        {
            return "-";
        }

        try
        {
            var bytes = Convert.FromHexString(hex);
            if (bytes.Length == 0)
            {
                return "(empty)";
            }

            foreach (var value in bytes)
            {
                if (value < 0x20 || value > 0x7E)
                {
                    return "(binary)";
                }
            }

            return string.Concat(bytes.Select(value => (char)value));
        }
        catch (FormatException)
        {
            return "(invalid)";
        }
    }
}

internal sealed record VerifyPaneState(
    string SummaryText,
    IReadOnlyList<VerifyIssueRow> Rows,
    string BadgeText,
    bool IsClean,
    string IssueSummaryText,
    string TreeTrustText);

internal sealed record HeaderPaneState(IReadOnlyList<HeaderFieldRow> Rows);

internal sealed record VerifyResetPaneState(
    string SummaryText,
    string IssueSummaryText,
    string TreeTrustText,
    IReadOnlyList<VerifyIssueRow> Rows);

internal sealed record PagesPaneState(IReadOnlyList<PageSummaryRow> Rows);

internal sealed record PagePaneState(
    string SummaryText,
    IReadOnlyList<PageRecordRow> Rows,
    PageRecordRow? SelectedRecord);

internal sealed record ProtectorsPaneState(IReadOnlyList<ProtectorSlotRow> Rows);

internal sealed record WalPaneState(string SummaryText, IReadOnlyList<WalRecordRow> Rows);

internal sealed record RecordSelectionState(string HeadlineText, string DetailText);

public sealed record HeaderFieldRow(string Label, string Value);

public sealed record VerifyIssueRow(string Pgno, string Description, bool HasIssue, bool IsPlaceholder);

public sealed record PageSummaryRow(string Pgno, string PageTypeName, string PageVersionLabel, string SlotCountLabel, string StateLabel, string Issue, bool HasIssue, bool IsPlaceholder);

public sealed record PageRecordRow(string Kind, string Slot, string RecordType, string KeyPreview, string KeyHex, string ValuePreview, string ValueHex, bool IsPlaceholder);

public sealed record ProtectorSlotRow(string Slot, string Kind, string KindByte);

public sealed record WalRecordRow(string Lsn, string Kind, string Pgno, string Detail, bool IsPlaceholder);