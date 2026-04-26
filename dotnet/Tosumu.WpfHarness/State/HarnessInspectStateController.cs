using System.Collections.ObjectModel;
using System.IO;
using System.Windows;
using System.Windows.Media;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessInspectStateController
{
    private readonly HarnessInspectClient inspectClient;
    private readonly HarnessTreeInspectorState treeInspectorState = new();
    private readonly HarnessInspectStateBindings bindings;

    public HarnessInspectStateController(HarnessInspectClient inspectClient, HarnessInspectStateBindings bindings)
    {
        this.inspectClient = inspectClient;
        this.bindings = bindings;
    }

    public void InitializeStartupState()
    {
        ResetHeaderState("Open a database to load the header automatically.");
        ResetVerifyState();
        ResetPagesState();
        ResetPageState();
        ResetProtectorsState();
    }

    public void PrepareForDatabaseSelection(string path)
    {
        bindings.SetDatabasePath(path);
        bindings.SetCurrentDatabaseTitleText(Path.GetFileName(path));
        bindings.SetCurrentDatabaseDetailText("Loading header and resetting stale pane state for the selected database...");
        bindings.SetVerificationBadgeText("Verify pending");
        bindings.SetVerificationBadgeBrush(bindings.ResolveThemeBrush("WarningBrush", Brushes.Khaki));
        bindings.SetVerifyIssueSummaryVisibility(Visibility.Collapsed);
        bindings.SetVerifyIssueSummaryText(string.Empty);
        bindings.SetVerifyIssueSummaryBrush(Brushes.Transparent);
        ResetHeaderState("Loading header for the selected database...");
        ResetVerifyState();
        ResetPagesState();
        ResetPageState();
        ResetWalState();
        ResetProtectorsState();
        ResetTreeInspectorState();
    }

    public async Task<TosumuInspectHeaderPayload> LoadHeaderAsync(string path)
    {
        var header = await ResolveInspectClient().GetHeaderAsync(path);
        var fileName = Path.GetFileName(path);
        treeInspectorState.ApplyHeader(header);

        bindings.HeaderRows.Clear();
        bindings.HeaderRows.Add(new HeaderFieldRow("Format version", header.FormatVersion.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Page size", header.PageSize.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Min reader version", header.MinReaderVersion.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Flags", $"0x{header.Flags:X4}"));
        bindings.HeaderRows.Add(new HeaderFieldRow("Page count", header.PageCount.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Freelist head", header.FreelistHead.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Root page", header.RootPage.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("WAL checkpoint LSN", header.WalCheckpointLsn.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("DEK id", header.DekId.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Keyslot count", header.KeyslotCount.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Keyslot region pages", header.KeyslotRegionPages.ToString()));
        bindings.HeaderRows.Add(new HeaderFieldRow("Slot 0 kind", $"{header.Slot0.Kind} ({header.Slot0.KindByte})"));
        bindings.HeaderRows.Add(new HeaderFieldRow("Slot 0 version", header.Slot0.Version.ToString()));

        bindings.SetCurrentDatabaseTitleText(fileName);
        bindings.SetCurrentDatabaseDetailText($"{header.PageCount} pages | root {header.RootPage} | format v{header.FormatVersion} | page size {header.PageSize}");
        RefreshTreeInspectorViews();
        bindings.LogDebug($"Loaded header for {fileName}: pages={header.PageCount}, root={header.RootPage}, format=v{header.FormatVersion}, page_size={header.PageSize}.");

        return header;
    }

    public async Task<TosumuInspectTreePayload> LoadTreeAsync(string path, TosumuInspectUnlockOptions? unlock)
    {
        var treeSnapshot = await ResolveInspectClient().GetTreeAsync(path, unlock);
        treeInspectorState.ApplyTreeSnapshot(treeSnapshot);
        RefreshTreeInspectorViews();
        bindings.RequestTreeWebViewRender();
        bindings.LogDebug($"Loaded tree snapshot rooted at page {treeSnapshot.RootPgno}.");
        return treeSnapshot;
    }

    public async Task<TosumuInspectVerifyPayload> LoadVerifyAsync(string path, TosumuInspectUnlockOptions? unlock)
    {
        var verify = await ResolveInspectClient().GetVerifyAsync(path, unlock);
        var verifyView = HarnessInspectPanePresenter.BuildVerifyView(verify);

        bindings.SetVerifySummaryText(verifyView.SummaryText);

        bindings.VerifyIssues.Clear();
        foreach (var row in verifyView.Rows)
        {
            bindings.VerifyIssues.Add(row);
        }

        if (verifyView.IsClean)
        {
            bindings.SetVerificationBadgeText(verifyView.BadgeText);
            bindings.SetVerificationBadgeBrush(bindings.ResolveThemeBrush("SuccessBrush", Brushes.Honeydew));
            bindings.SetVerifyIssueSummaryVisibility(Visibility.Visible);
            bindings.SetVerifyIssueSummaryBrush(bindings.ResolveThemeBrush("SuccessSoftBrush", Brushes.Honeydew));
            bindings.SetVerifyIssueSummaryText(verifyView.IssueSummaryText);
            bindings.SetTreeTrustText(verifyView.TreeTrustText);
        }
        else
        {
            bindings.SetVerificationBadgeText(verifyView.BadgeText);
            bindings.SetVerificationBadgeBrush(bindings.ResolveThemeBrush("DangerBrush", Brushes.MistyRose));
            bindings.SetVerifyIssueSummaryVisibility(Visibility.Visible);
            bindings.SetVerifyIssueSummaryBrush(bindings.ResolveThemeBrush("DangerSoftBrush", Brushes.MistyRose));
            bindings.SetVerifyIssueSummaryText(verifyView.IssueSummaryText);
            bindings.SetTreeTrustText(verifyView.TreeTrustText);
        }

        bindings.LogDebug($"Verification completed: pages_checked={verify.PagesChecked}, pages_ok={verify.PagesOk}, issues={verify.IssueCount}, btree_ok={verify.Btree.Ok}.");

        return verify;
    }

    public async Task<TosumuInspectPagesPayload> LoadPagesAsync(string path, TosumuInspectUnlockOptions? unlock)
    {
        var pages = await ResolveInspectClient().GetPagesAsync(path, unlock);
        var pagesView = HarnessInspectPanePresenter.BuildPagesView(pages);

        bindings.PageResults.Clear();
        foreach (var row in pagesView.Rows)
        {
            bindings.PageResults.Add(row);
        }

        bindings.LogDebug($"Loaded pages list: pages={pages.Pages.Count}.");
        return pages;
    }

    public async Task LoadPageAsync(string path, ulong pageNumber, TosumuInspectUnlockOptions? unlock)
    {
        var page = await ResolveInspectClient().GetPageAsync(path, pageNumber, unlock);
        var pageView = HarnessInspectPanePresenter.BuildPageView(page);
        treeInspectorState.SetFocusPage(page.Pgno, page.PageTypeName);
        RefreshTreeInspectorViews();
        bindings.RequestTreeWebViewRender();

        bindings.SetPageSummaryText(pageView.SummaryText);

        bindings.PageRecords.Clear();
        foreach (var row in pageView.Rows)
        {
            bindings.PageRecords.Add(row);
        }

        ApplySelectedPageRecord(pageView.SelectedRecord);

        bindings.LogDebug($"Loaded page {page.Pgno}: type={page.PageTypeName}, version={page.PageVersion}, slots={page.SlotCount}, records={page.Records.Count}.");
    }

    public async Task LoadProtectorsAsync(string path)
    {
        var protectors = await ResolveInspectClient().GetProtectorsAsync(path);
        var protectorsView = HarnessInspectPanePresenter.BuildProtectorsView(protectors);

        bindings.ProtectorSlots.Clear();
        foreach (var row in protectorsView.Rows)
        {
            bindings.ProtectorSlots.Add(row);
        }

        bindings.LogDebug($"Loaded protectors: slots={protectors.SlotCount}.");
    }

    public async Task<TosumuInspectWalPayload> LoadWalAsync(string path)
    {
        var wal = await ResolveInspectClient().GetWalAsync(path);
        var walView = HarnessInspectPanePresenter.BuildWalView(wal);

        bindings.SetWalSummaryText(walView.SummaryText);

        bindings.WalRecords.Clear();
        foreach (var row in walView.Rows)
        {
            bindings.WalRecords.Add(row);
        }

        bindings.LogDebug($"Loaded WAL summary: exists={wal.WalExists}, records={wal.RecordCount}.");
        return wal;
    }

    public void ApplySelectedPageRecord(PageRecordRow? record)
    {
        var selection = HarnessInspectPanePresenter.BuildSelectedRecordView(record);
        bindings.SetSelectedRecordHeadlineText(selection.HeadlineText);
        bindings.SetSelectedRecordDetailText(selection.DetailText);
    }

    public TreeWebViewPayload BuildTreeWebViewPayload(string treeTrustText)
    {
        return treeInspectorState.BuildWebViewPayload(treeTrustText);
    }

    private HarnessInspectClient ResolveInspectClient()
    {
        var executablePath = inspectClient.ExecutablePath;
        if (!string.Equals(bindings.GetExecutableStateText(), executablePath, StringComparison.Ordinal))
        {
            bindings.SetExecutableStateText(executablePath);
            bindings.LogDebug($"CLI resolved to {executablePath}");
        }

        return inspectClient;
    }

    private void ResetHeaderState(string message)
    {
        var headerReset = HarnessInspectPanePresenter.BuildHeaderResetView(message);
        bindings.HeaderRows.Clear();
        foreach (var row in headerReset.Rows)
        {
            bindings.HeaderRows.Add(row);
        }
    }

    private void ResetVerifyState()
    {
        var verifyReset = HarnessInspectPanePresenter.BuildVerifyResetView();
        bindings.SetVerifySummaryText(verifyReset.SummaryText);
        bindings.SetVerifyIssueSummaryVisibility(Visibility.Visible);
        bindings.SetVerifyIssueSummaryBrush(bindings.ResolveThemeBrush("WarningSoftBrush", Brushes.Khaki));
        bindings.SetVerifyIssueSummaryText(verifyReset.IssueSummaryText);
        bindings.SetTreeTrustText(verifyReset.TreeTrustText);
        bindings.VerifyIssues.Clear();
        foreach (var row in verifyReset.Rows)
        {
            bindings.VerifyIssues.Add(row);
        }
    }

    private void ResetPagesState()
    {
        var pagesReset = HarnessInspectPanePresenter.BuildPagesResetView();
        bindings.PageResults.Clear();
        foreach (var row in pagesReset.Rows)
        {
            bindings.PageResults.Add(row);
        }
    }

    private void ResetWalState()
    {
        var walReset = HarnessInspectPanePresenter.BuildWalResetView();
        bindings.SetWalSummaryText(walReset.SummaryText);
        bindings.WalRecords.Clear();
        foreach (var row in walReset.Rows)
        {
            bindings.WalRecords.Add(row);
        }
    }

    private void ResetPageState()
    {
        var pageReset = HarnessInspectPanePresenter.BuildPageResetView();
        bindings.SetPageSummaryText(pageReset.SummaryText);
        bindings.PageRecords.Clear();
        foreach (var row in pageReset.Rows)
        {
            bindings.PageRecords.Add(row);
        }

        ApplySelectedPageRecord(pageReset.SelectedRecord);
    }

    private void ResetTreeInspectorState()
    {
        treeInspectorState.Reset();
        bindings.SetTreeRootText(treeInspectorState.BuildRootText());
        bindings.SetTreeFocusText(treeInspectorState.BuildFocusText());
        bindings.SetTreeTrustText("Trust: verify pending.");
        RefreshTreeInspectorViews();
    }

    private void RefreshTreeInspectorViews()
    {
        bindings.SetTreeRootText(treeInspectorState.BuildRootText());
        bindings.SetTreeFocusText(treeInspectorState.BuildFocusText());

        bindings.TreePageVisits.Clear();
        foreach (var row in treeInspectorState.BuildVisitRows())
        {
            bindings.TreePageVisits.Add(row);
        }
    }

    private void ResetProtectorsState()
    {
        var protectorsReset = HarnessInspectPanePresenter.BuildProtectorsResetView();
        bindings.ProtectorSlots.Clear();
        foreach (var row in protectorsReset.Rows)
        {
            bindings.ProtectorSlots.Add(row);
        }
    }
}

internal sealed record HarnessInspectStateBindings(
    ObservableCollection<HeaderFieldRow> HeaderRows,
    ObservableCollection<VerifyIssueRow> VerifyIssues,
    ObservableCollection<PageSummaryRow> PageResults,
    ObservableCollection<PageRecordRow> PageRecords,
    ObservableCollection<ProtectorSlotRow> ProtectorSlots,
    ObservableCollection<WalRecordRow> WalRecords,
    ObservableCollection<TreePageVisitRow> TreePageVisits,
    Func<string> GetExecutableStateText,
    Action<string> SetExecutableStateText,
    Action<string> SetDatabasePath,
    Action<string> SetCurrentDatabaseTitleText,
    Action<string> SetCurrentDatabaseDetailText,
    Action<string> SetVerifySummaryText,
    Action<string> SetVerificationBadgeText,
    Action<Brush> SetVerificationBadgeBrush,
    Action<Visibility> SetVerifyIssueSummaryVisibility,
    Action<Brush> SetVerifyIssueSummaryBrush,
    Action<string> SetVerifyIssueSummaryText,
    Action<string> SetTreeRootText,
    Action<string> SetTreeFocusText,
    Action<string> SetTreeTrustText,
    Action<string> SetPageSummaryText,
    Action<string> SetWalSummaryText,
    Action<string> SetSelectedRecordHeadlineText,
    Action<string> SetSelectedRecordDetailText,
    Func<string, Brush, Brush> ResolveThemeBrush,
    Action RequestTreeWebViewRender,
    Action<string> LogDebug);