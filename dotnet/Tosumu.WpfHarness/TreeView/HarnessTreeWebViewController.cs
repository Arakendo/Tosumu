using System.IO;
using System.Text.Json;
using Microsoft.Web.WebView2.Core;
using Microsoft.Web.WebView2.Wpf;

namespace Tosumu.WpfHarness;

internal sealed class HarnessTreeWebViewController
{
    private static readonly JsonSerializerOptions TreeWebViewJsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    private readonly WebView2 treeWebView;
    private readonly Action<string> logDebug;
    private readonly Action<string, Exception> logException;
    private readonly Action<ulong> selectPage;
    private readonly Func<ulong, Task> inspectPageAsync;
    private Task? initializationTask;

    public HarnessTreeWebViewController(
        WebView2 treeWebView,
        Action<string> logDebug,
        Action<string, Exception> logException,
        Action<ulong> selectPage,
        Func<ulong, Task> inspectPageAsync)
    {
        this.treeWebView = treeWebView;
        this.logDebug = logDebug;
        this.logException = logException;
        this.selectPage = selectPage;
        this.inspectPageAsync = inspectPageAsync;
    }

    public Task RenderAsync(TreeWebViewPayload payload)
    {
        return RenderCoreAsync(payload);
    }

    public Task InitializeAsync()
    {
        return EnsureInitializedAsync();
    }

    private async Task EnsureInitializedAsync()
    {
        initializationTask ??= InitializeCoreAsync();
        await initializationTask;
    }

    private async Task InitializeCoreAsync()
    {
        var htmlPath = Path.Combine(AppContext.BaseDirectory, "Assets", "TreeView", "tree-view.html");
        if (!File.Exists(htmlPath))
        {
            logDebug($"Tree view asset missing: {htmlPath}");
            return;
        }

        await treeWebView.EnsureCoreWebView2Async();

        if (treeWebView.CoreWebView2 is null)
        {
            logDebug("Tree WebView2 runtime was not available.");
            return;
        }

        treeWebView.CoreWebView2.Settings.AreDefaultContextMenusEnabled = false;
        treeWebView.CoreWebView2.Settings.AreDevToolsEnabled = false;
        treeWebView.CoreWebView2.Settings.IsStatusBarEnabled = false;
        treeWebView.CoreWebView2.WebMessageReceived += OnWebMessageReceived;

        var navigationCompletion = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);

        void HandleNavigationCompleted(object? _, CoreWebView2NavigationCompletedEventArgs args)
        {
            treeWebView.NavigationCompleted -= HandleNavigationCompleted;
            navigationCompletion.TrySetResult(args.IsSuccess);
        }

        treeWebView.NavigationCompleted += HandleNavigationCompleted;
        treeWebView.Source = new Uri(htmlPath);

        var navigationSucceeded = await navigationCompletion.Task;
        logDebug(navigationSucceeded
            ? $"Initialized D3 tree view from {htmlPath}."
            : $"Tree view navigation reported failure for {htmlPath}.");
    }

    private async Task RenderCoreAsync(TreeWebViewPayload payload)
    {
        try
        {
            await EnsureInitializedAsync();

            if (treeWebView.CoreWebView2 is null)
            {
                return;
            }

            var json = JsonSerializer.Serialize(payload, TreeWebViewJsonOptions);
            await treeWebView.ExecuteScriptAsync($"window.renderTree({json});");
        }
        catch (Exception ex)
        {
            logException("render D3 tree view", ex);
        }
    }

    private async void OnWebMessageReceived(object? sender, CoreWebView2WebMessageReceivedEventArgs e)
    {
        try
        {
            using var document = JsonDocument.Parse(e.WebMessageAsJson);
            var root = document.RootElement;
            if (!root.TryGetProperty("type", out var typeElement))
            {
                return;
            }

            var messageType = typeElement.GetString();
            if (!root.TryGetProperty("pageNumber", out var pageElement) || pageElement.ValueKind != JsonValueKind.Number)
            {
                return;
            }

            if (!pageElement.TryGetUInt64(out var pageNumber))
            {
                return;
            }

            switch (messageType)
            {
                case "selectPage":
                    selectPage(pageNumber);
                    break;
                case "inspectPage":
                    await inspectPageAsync(pageNumber);
                    break;
            }
        }
        catch (Exception ex)
        {
            logException("handle D3 tree selection", ex);
        }
    }
}