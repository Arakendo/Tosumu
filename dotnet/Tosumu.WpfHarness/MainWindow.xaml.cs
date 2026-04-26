using System.Collections.ObjectModel;
using System.ComponentModel;
using System.IO;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Threading;
using Microsoft.Win32;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

public partial class MainWindow : Window, INotifyPropertyChanged
{
    private const int MaxRecentDatabaseCount = 8;
    private const int MaxDebugLogChars = 50_000;
    private const double KeyHexColumnVisibleWidth = 320;
    private const double ValueHexColumnVisibleWidth = 420;

    private string databasePath = string.Empty;
    private string currentDatabaseDetailText = "Browse to a .tsm file to enter inspection mode. The header will load automatically.";
    private string currentDatabaseTitleText = "No database selected";
    private string executableStateText = "Packaged CLI will be resolved on first command.";
    private string pageNumberText = "1";
    private string selectedRecordDetailText = "Select a non-placeholder record to inspect the current key/value payloads.";
    private string selectedRecordHeadlineText = "No record selected";
    private string pageSummaryText = "Select a page or inspect root to decode the current page.";
    private bool showHexColumns;
    private string statusText = "Activity updates appear here while you browse, verify, and inspect pages.";
    private string treeFocusText = "Focus page: inspect root or another page to begin tree navigation.";
    private string treeRootText = "Root page: load a database header to discover the tree root.";
    private string treeTrustText = "Trust: verify pending";
    private string unlockModeHintText = "You will be prompted only if the current database actually requires credentials.";
    private string walSummaryText = "Load a database to inspect the WAL sidecar if one is present.";
    private Brush verifyIssueSummaryBrush = Brushes.Transparent;
    private string verifyIssueSummaryText = string.Empty;
    private Visibility verifyIssueSummaryVisibility = Visibility.Collapsed;
    private Brush verificationBadgeBrush = Brushes.Khaki;
    private string verificationBadgeText = "Verify pending";
    private string verifySummaryText = "Run verification to check page auth and B-tree integrity.";
    private readonly string sessionStatePath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
        "Tosumu",
        "WpfHarness",
        "session.json");
    private DispatcherTimer? debugLogFlushTimer;
    private bool restoreDatabaseOnLoad;
    private readonly HarnessDatabaseLoadCoordinator databaseLoadCoordinator;
    private readonly HarnessDiagnosticsController diagnosticsController;
    private readonly HarnessInspectClient inspectClient = new();
    private readonly HarnessInspectStateController inspectStateController;
    private readonly HarnessInspectWorkflowCoordinator inspectWorkflowCoordinator;
    private readonly HarnessOperationRunner operationRunner;
    private readonly HarnessRecentDatabaseSelectionController recentDatabaseSelectionController;
    private readonly HarnessSessionCoordinator sessionCoordinator;
    private readonly HarnessUnlockPanelController unlockPanelController;
    private readonly HarnessUnlockCoordinator unlockCoordinator;
    private HarnessTreeWebViewController? treeWebViewController;

    public MainWindow()
    {
        InitializeComponent();
        Closing += MainWindow_OnClosing;
        Loaded += MainWindow_OnLoaded;
        DataContext = this;
        diagnosticsController = new HarnessDiagnosticsController(DebugLogTextBox, DebugLogScrollViewer, MaxDebugLogChars);
        inspectStateController = new HarnessInspectStateController(
            inspectClient,
            new HarnessInspectStateBindings(
                HeaderRows,
                VerifyIssues,
                PageResults,
                PageRecords,
                ProtectorSlots,
                WalRecords,
                TreePageVisits,
                () => ExecutableStateText,
                value => ExecutableStateText = value,
                value => DatabasePath = value,
                value => CurrentDatabaseTitleText = value,
                value => CurrentDatabaseDetailText = value,
                value => VerifySummaryText = value,
                value => VerificationBadgeText = value,
                value => VerificationBadgeBrush = value,
                value => VerifyIssueSummaryVisibility = value,
                value => VerifyIssueSummaryBrush = value,
                value => VerifyIssueSummaryText = value,
                value => TreeRootText = value,
                value => TreeFocusText = value,
                value => TreeTrustText = value,
                value => PageSummaryText = value,
                value => WalSummaryText = value,
                value => SelectedRecordHeadlineText = value,
                value => SelectedRecordDetailText = value,
                ResolveThemeBrush,
                QueueTreeWebViewRender,
                diagnosticsController.LogDebug));
        operationRunner = new HarnessOperationRunner(
            this,
            SetBusy,
            status => StatusText = status,
            diagnosticsController.LogDebug,
            diagnosticsController.LogInspectFailure,
            diagnosticsController.LogException);
        unlockPanelController = new HarnessUnlockPanelController(
            UnlockModeComboBox,
            SecretLabelTextBlock,
            SecretPasswordBox,
            KeyfilePathTextBox,
            BrowseKeyfileButton,
            HarnessUnlockModes.Auto,
            value => UnlockModeHintText = value);
        databaseLoadCoordinator = new HarnessDatabaseLoadCoordinator(
            inspectStateController.PrepareForDatabaseSelection,
            AddRecentDatabasePath,
            status => StatusText = status,
            inspectStateController.LoadHeaderAsync,
            inspectStateController.LoadWalAsync,
            inspectStateController.LoadPagesAsync,
            inspectStateController.LoadTreeAsync,
            ShouldAutoLoadTreeWithoutUnlock,
            operationRunner.RunAsync);
        recentDatabaseSelectionController = new HarnessRecentDatabaseSelectionController(RecentDatabasePaths, RecentDatabasesComboBox);
        sessionCoordinator = new HarnessSessionCoordinator(sessionStatePath, MaxRecentDatabaseCount);
        unlockCoordinator = new HarnessUnlockCoordinator(
            this,
            unlockPanelController.GetSelectedUnlockMode,
            () => SecretPasswordBox.Password,
            () => KeyfilePathTextBox.Text,
            unlockPanelController.ApplyUnlockSelection,
            SetBusy,
            status => StatusText = status,
            diagnosticsController.LogDebug,
            diagnosticsController.LogInspectFailure,
            diagnosticsController.LogException);
        inspectWorkflowCoordinator = new HarnessInspectWorkflowCoordinator(
            unlockCoordinator,
            GetValidDatabasePathOrNull,
            GetRequestedPageNumberOrNull,
            () => PageNumberText,
            AddRecentDatabasePath,
            status => StatusText = status,
            pageNumber => PageNumberText = pageNumber,
            diagnosticsController.LogDebug,
            inspectStateController.LoadHeaderAsync,
            inspectStateController.LoadTreeAsync,
            inspectStateController.LoadVerifyAsync,
            inspectStateController.LoadPagesAsync,
            inspectStateController.LoadPageAsync,
            inspectStateController.LoadProtectorsAsync,
            inspectStateController.LoadWalAsync,
            ShouldAutoLoadTreeWithoutUnlock,
            operationRunner.RunAsync);
        UnlockModeComboBox.SelectedIndex = 0;
        debugLogFlushTimer = new DispatcherTimer(
            TimeSpan.FromMilliseconds(150),
            DispatcherPriority.Background,
            (_, _) => diagnosticsController.FlushDebugLog(),
            Dispatcher);
        debugLogFlushTimer.Start();
        unlockPanelController.UpdateModeInputs();
        UpdateHexColumnVisibility();
        inspectStateController.InitializeStartupState();
        LoadSessionState();
        diagnosticsController.LogDebug("Harness initialized. Browse to a .tsm file or open a recent database to begin.");
    }

    public event PropertyChangedEventHandler? PropertyChanged;

    public ObservableCollection<HeaderFieldRow> HeaderRows { get; } = [];

    public ObservableCollection<VerifyIssueRow> VerifyIssues { get; } = [];

    public ObservableCollection<PageSummaryRow> PageResults { get; } = [];

    public ObservableCollection<WalRecordRow> WalRecords { get; } = [];

    public ObservableCollection<PageRecordRow> PageRecords { get; } = [];

    public ObservableCollection<ProtectorSlotRow> ProtectorSlots { get; } = [];

    public ObservableCollection<string> RecentDatabasePaths { get; } = [];

    public ObservableCollection<TreePageVisitRow> TreePageVisits { get; } = [];

    public string DatabasePath
    {
        get => databasePath;
        set => SetProperty(ref databasePath, value);
    }

    public string ExecutableStateText
    {
        get => executableStateText;
        set => SetProperty(ref executableStateText, value);
    }

    public string CurrentDatabaseTitleText
    {
        get => currentDatabaseTitleText;
        set => SetProperty(ref currentDatabaseTitleText, value);
    }

    public string CurrentDatabaseDetailText
    {
        get => currentDatabaseDetailText;
        set => SetProperty(ref currentDatabaseDetailText, value);
    }

    public string PageNumberText
    {
        get => pageNumberText;
        set => SetProperty(ref pageNumberText, value);
    }

    public string SelectedRecordDetailText
    {
        get => selectedRecordDetailText;
        set => SetProperty(ref selectedRecordDetailText, value);
    }

    public string SelectedRecordHeadlineText
    {
        get => selectedRecordHeadlineText;
        set => SetProperty(ref selectedRecordHeadlineText, value);
    }

    public string PageSummaryText
    {
        get => pageSummaryText;
        set => SetProperty(ref pageSummaryText, value);
    }

    public bool ShowHexColumns
    {
        get => showHexColumns;
        set => SetProperty(ref showHexColumns, value);
    }

    public string StatusText
    {
        get => statusText;
        set => SetProperty(ref statusText, value);
    }

    public string TreeFocusText
    {
        get => treeFocusText;
        set => SetProperty(ref treeFocusText, value);
    }

    public string TreeRootText
    {
        get => treeRootText;
        set => SetProperty(ref treeRootText, value);
    }

    public string TreeTrustText
    {
        get => treeTrustText;
        set => SetProperty(ref treeTrustText, value);
    }

    public string UnlockModeHintText
    {
        get => unlockModeHintText;
        set => SetProperty(ref unlockModeHintText, value);
    }

    public string WalSummaryText
    {
        get => walSummaryText;
        set => SetProperty(ref walSummaryText, value);
    }

    public Brush VerifyIssueSummaryBrush
    {
        get => verifyIssueSummaryBrush;
        set => SetProperty(ref verifyIssueSummaryBrush, value);
    }

    public string VerifyIssueSummaryText
    {
        get => verifyIssueSummaryText;
        set => SetProperty(ref verifyIssueSummaryText, value);
    }

    public Visibility VerifyIssueSummaryVisibility
    {
        get => verifyIssueSummaryVisibility;
        set => SetProperty(ref verifyIssueSummaryVisibility, value);
    }

    public Brush VerificationBadgeBrush
    {
        get => verificationBadgeBrush;
        set => SetProperty(ref verificationBadgeBrush, value);
    }

    public string VerificationBadgeText
    {
        get => verificationBadgeText;
        set => SetProperty(ref verificationBadgeText, value);
    }

    public string VerifySummaryText
    {
        get => verifySummaryText;
        set => SetProperty(ref verifySummaryText, value);
    }

    private async void LoadHeaderButton_OnClick(object sender, RoutedEventArgs e)
    {
        await inspectWorkflowCoordinator.ReloadHeaderAsync();
    }

    private async void VerifyButton_OnClick(object sender, RoutedEventArgs e)
    {
        await inspectWorkflowCoordinator.VerifyAsync();
    }

    private async void InspectPageButton_OnClick(object sender, RoutedEventArgs e)
    {
        await inspectWorkflowCoordinator.InspectRequestedPageAsync();
    }

    private async void InspectProtectorsButton_OnClick(object sender, RoutedEventArgs e)
    {
        await inspectWorkflowCoordinator.LoadProtectorsAsync();
    }

    private async void InspectRootPageButton_OnClick(object sender, RoutedEventArgs e)
    {
        await inspectWorkflowCoordinator.InspectRootPageAsync();
    }

    private async void RefreshAllButton_OnClick(object sender, RoutedEventArgs e)
    {
        await inspectWorkflowCoordinator.RefreshAllAsync();
    }

    private void BrowseButton_OnClick(object sender, RoutedEventArgs e)
    {
        var dialog = new OpenFileDialog
        {
            Title = "Choose a Tosumu database",
            Filter = "Tosumu database (*.tsm)|*.tsm|All files (*.*)|*.*",
            CheckFileExists = true,
            Multiselect = false,
        };

        if (dialog.ShowDialog(this) == true)
        {
            diagnosticsController.LogDebug($"Selected database path: {dialog.FileName}");
            _ = databaseLoadCoordinator.AutoLoadSelectedDatabaseAsync(dialog.FileName, "Opened database and loaded header.");
        }
    }

    private void RecentDatabasesComboBox_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        var path = recentDatabaseSelectionController.GetSelectedPath();
        if (string.IsNullOrWhiteSpace(path))
        {
            return;
        }

        diagnosticsController.LogDebug($"Selected recent database: {path}");
    _ = databaseLoadCoordinator.AutoLoadSelectedDatabaseAsync(path, "Loaded header from recent database.");
    }

    private void ShowHexColumnsCheckBox_OnChanged(object sender, RoutedEventArgs e)
    {
        ShowHexColumns = ShowHexColumnsCheckBox.IsChecked == true;
        UpdateHexColumnVisibility();
    }

    private void TreePagesListView_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (sender is ListView { SelectedItem: TreePageVisitRow row } && !row.IsPlaceholder)
        {
            inspectWorkflowCoordinator.SelectPageNumberFromSource(row.Page, "tree history");
        }
    }

    private async void TreePagesListView_OnMouseDoubleClick(object sender, MouseButtonEventArgs e)
    {
        if (sender is ListView { SelectedItem: TreePageVisitRow row } && !row.IsPlaceholder)
        {
            await inspectWorkflowCoordinator.InspectSelectedPageFromSourceAsync(row.Page, "tree history");
        }
    }

    private async void TreePagesListView_OnKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter || sender is not ListView { SelectedItem: TreePageVisitRow row } || row.IsPlaceholder)
        {
            return;
        }

        await inspectWorkflowCoordinator.InspectSelectedPageFromSourceAsync(row.Page, "tree history");
        e.Handled = true;
    }

    private void Window_OnPreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.F5 || (Keyboard.Modifiers == ModifierKeys.Control && e.Key == Key.R))
        {
            RefreshAllButton_OnClick(RefreshAllButton, e);
            e.Handled = true;
            return;
        }

        if (Keyboard.Modifiers == ModifierKeys.Control && e.Key == Key.Enter)
        {
            InspectRootPageButton_OnClick(InspectRootPageButton, e);
            e.Handled = true;
        }
    }

    private void PageNumberTextBox_OnKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter)
        {
            return;
        }

        InspectPageButton_OnClick(InspectPageButton, e);
        e.Handled = true;
    }

    private void VerifyIssuesListView_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (VerifyIssuesListView.SelectedItem is VerifyIssueRow row)
        {
            inspectWorkflowCoordinator.SelectPageNumberFromSource(row.Pgno, "issue");
        }
    }

    private async void VerifyIssuesListView_OnKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter || VerifyIssuesListView.SelectedItem is not VerifyIssueRow row)
        {
            return;
        }

        await inspectWorkflowCoordinator.InspectSelectedPageFromSourceAsync(row.Pgno, "verification issue");
        e.Handled = true;
    }

    private async void VerifyIssuesListView_OnMouseDoubleClick(object sender, MouseButtonEventArgs e)
    {
        if (VerifyIssuesListView.SelectedItem is VerifyIssueRow row)
        {
            await inspectWorkflowCoordinator.InspectSelectedPageFromSourceAsync(row.Pgno, "verification issue");
        }
    }

    private void PageResultsListView_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (PageResultsListView.SelectedItem is PageSummaryRow row)
        {
            inspectWorkflowCoordinator.SelectPageNumberFromSource(row.Pgno, "pages list");
        }
    }

    private async void PageResultsListView_OnKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key != Key.Enter || PageResultsListView.SelectedItem is not PageSummaryRow row)
        {
            return;
        }

        await inspectWorkflowCoordinator.InspectSelectedPageFromSourceAsync(row.Pgno, "pages list");
        e.Handled = true;
    }

    private async void PageResultsListView_OnMouseDoubleClick(object sender, MouseButtonEventArgs e)
    {
        if (PageResultsListView.SelectedItem is PageSummaryRow row)
        {
            await inspectWorkflowCoordinator.InspectSelectedPageFromSourceAsync(row.Pgno, "pages list");
        }
    }

    private void PageRecordsListView_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        inspectStateController.ApplySelectedPageRecord(PageRecordsListView.SelectedItem as PageRecordRow);
    }

    private async void MainWindow_OnLoaded(object sender, RoutedEventArgs e)
    {
        if (TreeWebView is not null)
        {
            await ResolveTreeWebViewController().InitializeAsync();
        }

        if (!restoreDatabaseOnLoad)
        {
            return;
        }

        restoreDatabaseOnLoad = false;
        await databaseLoadCoordinator.AutoLoadSelectedDatabaseAsync(DatabasePath.Trim(), "Restored last database and loaded header.");
    }

    private void BrowseKeyfileButton_OnClick(object sender, RoutedEventArgs e)
    {
        var dialog = new OpenFileDialog
        {
            Title = "Choose a Tosumu keyfile",
            Filter = "All files (*.*)|*.*",
            CheckFileExists = true,
            Multiselect = false,
        };

        if (dialog.ShowDialog(this) == true)
        {
            KeyfilePathTextBox.Text = dialog.FileName;
            diagnosticsController.LogDebug($"Selected keyfile path: {dialog.FileName}");
        }
    }

    private void MainWindow_OnClosing(object? sender, CancelEventArgs e)
    {
        debugLogFlushTimer?.Stop();
        SaveSessionState();
    }

    private void ClearDebugConsoleButton_OnClick(object sender, RoutedEventArgs e)
    {
        diagnosticsController.ClearDebugLog();
    }

    private void UnlockModeComboBox_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        unlockPanelController.UpdateModeInputs();
    }

    private bool TryGetValidDatabasePath(out string path)
    {
        path = DatabasePath.Trim();

        if (string.IsNullOrWhiteSpace(path))
        {
            MessageBox.Show(this, "Choose a database file first.", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Information);
            return false;
        }

        if (!System.IO.File.Exists(path))
        {
            MessageBox.Show(this, $"Database file not found:\n{path}", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Warning);
            return false;
        }

        return true;
    }

    private void LoadSessionState()
    {
        var session = sessionCoordinator.Load();

        recentDatabaseSelectionController.ApplyRecentDatabasePaths(session.RecentDatabasePaths, session.LastDatabasePath);

        if (!string.IsNullOrWhiteSpace(session.LastDatabasePath))
        {
            DatabasePath = session.LastDatabasePath;
            CurrentDatabaseTitleText = session.CurrentDatabaseTitleText ?? CurrentDatabaseTitleText;
            CurrentDatabaseDetailText = session.CurrentDatabaseDetailText ?? CurrentDatabaseDetailText;
            restoreDatabaseOnLoad = session.RestoreDatabaseOnLoad;
            StatusText = session.StatusText ?? StatusText;
        }

        if (!string.IsNullOrWhiteSpace(session.LastPageNumber))
        {
            PageNumberText = session.LastPageNumber;
        }

        if (!string.IsNullOrWhiteSpace(session.UnlockMode))
        {
            unlockPanelController.SelectUnlockMode(session.UnlockMode);
        }

        if (!string.IsNullOrWhiteSpace(session.KeyfilePath))
        {
            KeyfilePathTextBox.Text = session.KeyfilePath;
        }
    }

    private void SaveSessionState()
    {
        sessionCoordinator.Save(new HarnessSessionSaveRequest(
            LastDatabasePath: DatabasePath,
            LastPageNumber: PageNumberText,
            UnlockMode: unlockPanelController.GetSelectedUnlockMode(),
            KeyfilePath: KeyfilePathTextBox.Text,
            RecentDatabasePaths: RecentDatabasePaths.ToList()));
    }

    private void AddRecentDatabasePath(string path)
    {
        var updatedPaths = sessionCoordinator.PushRecentDatabasePath(RecentDatabasePaths, path);
        recentDatabaseSelectionController.ApplyRecentDatabasePaths(updatedPaths, updatedPaths.FirstOrDefault());
    }

    private void UpdateHexColumnVisibility()
    {
        if (KeyHexColumn is null || ValueHexColumn is null)
        {
            return;
        }

        KeyHexColumn.Width = ShowHexColumns ? KeyHexColumnVisibleWidth : 0;
        ValueHexColumn.Width = ShowHexColumns ? ValueHexColumnVisibleWidth : 0;
    }

    private void QueueTreeWebViewRender()
    {
        if (TreeWebView is null)
        {
            return;
        }

        _ = ResolveTreeWebViewController().RenderAsync(BuildTreeWebViewPayload());
    }

    private HarnessTreeWebViewController ResolveTreeWebViewController()
    {
        return treeWebViewController ??= new HarnessTreeWebViewController(
            TreeWebView,
            diagnosticsController.LogDebug,
            diagnosticsController.LogException,
            HandleTreeWebViewSelectPage,
            HandleTreeWebViewInspectPageAsync);
    }

    private TreeWebViewPayload BuildTreeWebViewPayload()
    {
        return inspectStateController.BuildTreeWebViewPayload(TreeTrustText);
    }

    private void HandleTreeWebViewSelectPage(ulong pageNumber)
    {
        PageNumberText = pageNumber.ToString();
        StatusText = $"Selected page {pageNumber} from the D3 tree view. Double-click to inspect it immediately.";
        diagnosticsController.LogDebug($"Selected page {pageNumber} from the D3 tree view.");
    }

    private async Task HandleTreeWebViewInspectPageAsync(ulong pageNumber)
    {
        diagnosticsController.LogDebug($"Inspect request for page {pageNumber} came from the D3 tree view.");
        await inspectWorkflowCoordinator.InspectSelectedPageFromSourceAsync(pageNumber.ToString(), "D3 tree view");
    }

    private Brush ResolveThemeBrush(string resourceKey, Brush fallback)
    {
        return TryFindResource(resourceKey) as Brush ?? fallback;
    }

    private static bool ShouldAutoLoadTreeWithoutUnlock(TosumuInspectHeaderPayload header)
    {
        return string.Equals(header.Slot0.Kind, "Sentinel", StringComparison.Ordinal);
    }

    private string? GetValidDatabasePathOrNull()
    {
        return TryGetValidDatabasePath(out var path) ? path : null;
    }

    private ulong? GetRequestedPageNumberOrNull()
    {
        return TryGetPageNumber(out var pageNumber) ? pageNumber : null;
    }

    private bool TryGetPageNumber(out ulong pageNumber)
    {
        if (!ulong.TryParse(PageNumberText.Trim(), out pageNumber))
        {
            MessageBox.Show(this, "Enter a valid non-negative page number.", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Information);
            return false;
        }

        return true;
    }

    private Task RunBusyActionAsync(string operationName, Func<Task> action)
        => operationRunner.RunAsync(operationName, action);

    private void SetBusy(bool isBusy)
    {
        BrowseButton.IsEnabled = !isBusy;
        BrowseKeyfileButton.IsEnabled = !isBusy;
        InspectProtectorsButton.IsEnabled = !isBusy;
        InspectRootPageButton.IsEnabled = !isBusy;
        RefreshAllButton.IsEnabled = !isBusy;
        InspectPageButton.IsEnabled = !isBusy;
        LoadHeaderButton.IsEnabled = !isBusy;
        VerifyButton.IsEnabled = !isBusy;
        DatabasePathTextBox.IsEnabled = !isBusy;
        UnlockModeComboBox.IsEnabled = !isBusy;
        KeyfilePathTextBox.IsEnabled = !isBusy;
        PageNumberTextBox.IsEnabled = !isBusy;
        SecretPasswordBox.IsEnabled = !isBusy;
    }

    private void SetProperty<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return;
        }

        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }
}
