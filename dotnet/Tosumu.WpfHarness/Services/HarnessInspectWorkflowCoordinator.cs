using System.IO;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessInspectWorkflowCoordinator
{
    private readonly HarnessUnlockCoordinator unlockCoordinator;
    private readonly Func<string?> getValidDatabasePathOrNull;
    private readonly Func<ulong?> getRequestedPageNumberOrNull;
    private readonly Func<string> getCurrentPageNumberText;
    private readonly Action<string> addRecentDatabasePath;
    private readonly Action<string> setStatus;
    private readonly Action<string> setPageNumberText;
    private readonly Action<string> logDebug;
    private readonly Func<string, Task<TosumuInspectHeaderPayload>> loadHeaderAsync;
    private readonly Func<string, TosumuInspectUnlockOptions?, Task<TosumuInspectTreePayload>> loadTreeAsync;
    private readonly Func<string, TosumuInspectUnlockOptions?, Task<TosumuInspectVerifyPayload>> loadVerifyAsync;
    private readonly Func<string, TosumuInspectUnlockOptions?, Task<TosumuInspectPagesPayload>> loadPagesAsync;
    private readonly Func<string, ulong, TosumuInspectUnlockOptions?, Task> loadPageAsync;
    private readonly Func<string, Task> loadProtectorsAsync;
    private readonly Func<string, Task<TosumuInspectWalPayload>> loadWalAsync;
    private readonly Func<TosumuInspectHeaderPayload, bool> shouldAutoLoadTreeWithoutUnlock;
    private readonly Func<string, Func<Task>, Task> runBusyActionAsync;

    public HarnessInspectWorkflowCoordinator(
        HarnessUnlockCoordinator unlockCoordinator,
        Func<string?> getValidDatabasePathOrNull,
        Func<ulong?> getRequestedPageNumberOrNull,
        Func<string> getCurrentPageNumberText,
        Action<string> addRecentDatabasePath,
        Action<string> setStatus,
        Action<string> setPageNumberText,
        Action<string> logDebug,
        Func<string, Task<TosumuInspectHeaderPayload>> loadHeaderAsync,
        Func<string, TosumuInspectUnlockOptions?, Task<TosumuInspectTreePayload>> loadTreeAsync,
        Func<string, TosumuInspectUnlockOptions?, Task<TosumuInspectVerifyPayload>> loadVerifyAsync,
        Func<string, TosumuInspectUnlockOptions?, Task<TosumuInspectPagesPayload>> loadPagesAsync,
        Func<string, ulong, TosumuInspectUnlockOptions?, Task> loadPageAsync,
        Func<string, Task> loadProtectorsAsync,
        Func<string, Task<TosumuInspectWalPayload>> loadWalAsync,
        Func<TosumuInspectHeaderPayload, bool> shouldAutoLoadTreeWithoutUnlock,
        Func<string, Func<Task>, Task> runBusyActionAsync)
    {
        this.unlockCoordinator = unlockCoordinator;
        this.getValidDatabasePathOrNull = getValidDatabasePathOrNull;
        this.getRequestedPageNumberOrNull = getRequestedPageNumberOrNull;
        this.getCurrentPageNumberText = getCurrentPageNumberText;
        this.addRecentDatabasePath = addRecentDatabasePath;
        this.setStatus = setStatus;
        this.setPageNumberText = setPageNumberText;
        this.logDebug = logDebug;
        this.loadHeaderAsync = loadHeaderAsync;
        this.loadTreeAsync = loadTreeAsync;
        this.loadVerifyAsync = loadVerifyAsync;
        this.loadPagesAsync = loadPagesAsync;
        this.loadPageAsync = loadPageAsync;
        this.loadProtectorsAsync = loadProtectorsAsync;
        this.loadWalAsync = loadWalAsync;
        this.shouldAutoLoadTreeWithoutUnlock = shouldAutoLoadTreeWithoutUnlock;
        this.runBusyActionAsync = runBusyActionAsync;
    }

    public async Task ReloadHeaderAsync()
    {
        var path = getValidDatabasePathOrNull();
        if (path is null)
        {
            return;
        }

        await runBusyActionAsync("reload header", async () =>
        {
            setStatus("Loading header...");
            addRecentDatabasePath(path);
            var header = await loadHeaderAsync(path);
            await loadWalAsync(path);
            if (shouldAutoLoadTreeWithoutUnlock(header))
            {
                await loadPagesAsync(path, null);
                await loadTreeAsync(path, null);
            }

            setStatus($"Loaded {Path.GetFileName(path)}: {header.PageCount} pages, root page {header.RootPage}.");
        });
    }

    public async Task VerifyAsync()
    {
        var path = getValidDatabasePathOrNull();
        if (path is null)
        {
            return;
        }

        if (!unlockCoordinator.TryGetSelection("verify the database", out var unlockSelection))
        {
            return;
        }

        await unlockCoordinator.RunUnlockableInspectActionAsync("verify database", unlockSelection, async unlock =>
        {
            setStatus("Running verification...");
            addRecentDatabasePath(path);
            var verify = await loadVerifyAsync(path, unlock);
            await loadPagesAsync(path, unlock);
            await loadTreeAsync(path, unlock);

            setStatus(HarnessInspectPanePresenter.BuildVerifyStatusText(path, verify));
        });
    }

    public async Task InspectRequestedPageAsync()
    {
        var path = getValidDatabasePathOrNull();
        var pageNumber = getRequestedPageNumberOrNull();
        if (path is null || pageNumber is null)
        {
            return;
        }

        if (!unlockCoordinator.TryGetSelection("inspect the page", out var unlockSelection))
        {
            return;
        }

        await unlockCoordinator.RunUnlockableInspectActionAsync($"inspect page {pageNumber}", unlockSelection, async unlock =>
        {
            setStatus($"Inspecting page {pageNumber}...");
            addRecentDatabasePath(path);
            await loadPageAsync(path, pageNumber.Value, unlock);
            await loadPagesAsync(path, unlock);
            await loadTreeAsync(path, unlock);

            setStatus($"Loaded page {pageNumber} from {Path.GetFileName(path)}.");
        });
    }

    public async Task LoadProtectorsAsync()
    {
        var path = getValidDatabasePathOrNull();
        if (path is null)
        {
            return;
        }

        await runBusyActionAsync("load protectors", async () =>
        {
            setStatus("Loading protectors...");
            addRecentDatabasePath(path);
            await loadProtectorsAsync(path);
            await loadWalAsync(path);

            setStatus($"Loaded protectors for {Path.GetFileName(path)}.");
        });
    }

    public async Task InspectRootPageAsync()
    {
        var path = getValidDatabasePathOrNull();
        if (path is null)
        {
            return;
        }

        if (!unlockCoordinator.TryGetSelection("inspect the root page", out var unlockSelection))
        {
            return;
        }

        await unlockCoordinator.RunUnlockableInspectActionAsync("inspect root page", unlockSelection, async unlock =>
        {
            setStatus("Loading header and root page...");
            addRecentDatabasePath(path);

            var header = await loadHeaderAsync(path);
            setPageNumberText(header.RootPage.ToString());
            await loadPageAsync(path, header.RootPage, unlock);
            await loadPagesAsync(path, unlock);
            await loadTreeAsync(path, unlock);

            setStatus($"Loaded root page {header.RootPage} from {Path.GetFileName(path)}.");
        });
    }

    public async Task RefreshAllAsync()
    {
        var path = getValidDatabasePathOrNull();
        if (path is null)
        {
            return;
        }

        if (!unlockCoordinator.TryGetSelection("refresh inspect data", out var unlockSelection))
        {
            return;
        }

        var hasPageNumber = ulong.TryParse(getCurrentPageNumberText().Trim(), out var pageNumber);

        await unlockCoordinator.RunUnlockableInspectActionAsync("refresh all panes", unlockSelection, async unlock =>
        {
            setStatus("Refreshing header, pages, verify, protectors, and page...");
            addRecentDatabasePath(path);

            await loadHeaderAsync(path);
            await loadProtectorsAsync(path);
            await loadWalAsync(path);
            await loadPagesAsync(path, unlock);
            await loadVerifyAsync(path, unlock);
            await loadTreeAsync(path, unlock);

            if (hasPageNumber)
            {
                await loadPageAsync(path, pageNumber, unlock);
                setStatus($"Refreshed all views for {Path.GetFileName(path)}.");
            }
            else
            {
                setStatus($"Refreshed header, verify, and protectors for {Path.GetFileName(path)}. Page refresh skipped because the page number is invalid.");
            }
        });
    }

    public void SelectPageNumberFromSource(string pgnoText, string sourceLabel)
    {
        if (!ulong.TryParse(pgnoText, out var pageNumber))
        {
            return;
        }

        setPageNumberText(pageNumber.ToString());
        setStatus($"Selected page {pageNumber} from {sourceLabel}. Double-click to inspect it.");
        logDebug($"Selected page {pageNumber} from {sourceLabel}.");
    }

    public async Task InspectSelectedPageFromSourceAsync(string pgnoText, string sourceLabel)
    {
        if (!ulong.TryParse(pgnoText, out var pageNumber))
        {
            return;
        }

        var path = getValidDatabasePathOrNull();
        if (path is null)
        {
            return;
        }

        if (!unlockCoordinator.TryGetSelection($"inspect page {pageNumber} from the {sourceLabel}", out var unlockSelection))
        {
            return;
        }

        setPageNumberText(pageNumber.ToString());

        await unlockCoordinator.RunUnlockableInspectActionAsync($"inspect page {pageNumber} from {sourceLabel}", unlockSelection, async unlock =>
        {
            setStatus($"Inspecting page {pageNumber} from the {sourceLabel}...");
            addRecentDatabasePath(path);
            await loadPageAsync(path, pageNumber, unlock);
            setStatus($"Loaded page {pageNumber} from the {sourceLabel}.");
        });
    }
}