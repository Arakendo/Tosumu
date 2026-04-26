using System.IO;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessDatabaseLoadCoordinator
{
    private readonly Action<string> prepareForDatabaseSelection;
    private readonly Action<string> addRecentDatabasePath;
    private readonly Action<string> setStatus;
    private readonly Func<string, Task<TosumuInspectHeaderPayload>> loadHeaderAsync;
    private readonly Func<string, Task> loadWalAsync;
    private readonly Func<string, TosumuInspectUnlockOptions?, Task> loadPagesAsync;
    private readonly Func<string, TosumuInspectUnlockOptions?, Task> loadTreeAsync;
    private readonly Func<TosumuInspectHeaderPayload, bool> shouldAutoLoadTreeWithoutUnlock;
    private readonly Func<string, Func<Task>, Task> runBusyActionAsync;

    public HarnessDatabaseLoadCoordinator(
        Action<string> prepareForDatabaseSelection,
        Action<string> addRecentDatabasePath,
        Action<string> setStatus,
        Func<string, Task<TosumuInspectHeaderPayload>> loadHeaderAsync,
        Func<string, Task> loadWalAsync,
        Func<string, TosumuInspectUnlockOptions?, Task> loadPagesAsync,
        Func<string, TosumuInspectUnlockOptions?, Task> loadTreeAsync,
        Func<TosumuInspectHeaderPayload, bool> shouldAutoLoadTreeWithoutUnlock,
        Func<string, Func<Task>, Task> runBusyActionAsync)
    {
        this.prepareForDatabaseSelection = prepareForDatabaseSelection;
        this.addRecentDatabasePath = addRecentDatabasePath;
        this.setStatus = setStatus;
        this.loadHeaderAsync = loadHeaderAsync;
        this.loadWalAsync = loadWalAsync;
        this.loadPagesAsync = loadPagesAsync;
        this.loadTreeAsync = loadTreeAsync;
        this.shouldAutoLoadTreeWithoutUnlock = shouldAutoLoadTreeWithoutUnlock;
        this.runBusyActionAsync = runBusyActionAsync;
    }

    public async Task AutoLoadSelectedDatabaseAsync(string path, string completionStatus)
    {
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path))
        {
            return;
        }

        prepareForDatabaseSelection(path);

        await runBusyActionAsync("open selected database", async () =>
        {
            setStatus($"Opening {Path.GetFileName(path)}...");
            addRecentDatabasePath(path);
            var header = await loadHeaderAsync(path);
            await loadWalAsync(path);
            if (shouldAutoLoadTreeWithoutUnlock(header))
            {
                await loadPagesAsync(path, null);
                await loadTreeAsync(path, null);
            }

            setStatus(completionStatus);
        });
    }
}