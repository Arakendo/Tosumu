using System.IO;

namespace Tosumu.WpfHarness;

internal sealed class HarnessSessionCoordinator
{
    private readonly string sessionStatePath;
    private readonly int maxRecentDatabaseCount;

    public HarnessSessionCoordinator(string sessionStatePath, int maxRecentDatabaseCount)
    {
        this.sessionStatePath = sessionStatePath;
        this.maxRecentDatabaseCount = maxRecentDatabaseCount;
    }

    public HarnessSessionLoadResult Load()
    {
        var sessionState = HarnessSessionState.Load(sessionStatePath);
        var recentDatabasePaths = NormalizeRecentDatabasePaths(sessionState.RecentDatabasePaths);

        if (string.IsNullOrWhiteSpace(sessionState.LastDatabasePath))
        {
            return new HarnessSessionLoadResult(
                RecentDatabasePaths: recentDatabasePaths,
                LastDatabasePath: null,
                LastPageNumber: sessionState.LastPageNumber,
                UnlockMode: sessionState.UnlockMode,
                KeyfilePath: sessionState.KeyfilePath,
                CurrentDatabaseTitleText: null,
                CurrentDatabaseDetailText: null,
                StatusText: null,
                RestoreDatabaseOnLoad: false);
        }

        var lastDatabasePath = sessionState.LastDatabasePath.Trim();
        recentDatabasePaths = PushRecentDatabasePath(recentDatabasePaths, lastDatabasePath);

        var fileName = Path.GetFileName(lastDatabasePath);
        var restoreDatabaseOnLoad = File.Exists(lastDatabasePath);
        var statusText = restoreDatabaseOnLoad
            ? $"Restored {fileName} from the last session."
            : $"Restored last database path {fileName}, but the file is no longer present.";

        return new HarnessSessionLoadResult(
            RecentDatabasePaths: recentDatabasePaths,
            LastDatabasePath: lastDatabasePath,
            LastPageNumber: sessionState.LastPageNumber,
            UnlockMode: sessionState.UnlockMode,
            KeyfilePath: sessionState.KeyfilePath,
            CurrentDatabaseTitleText: fileName,
            CurrentDatabaseDetailText: "Restored from the last session. Loading header on startup...",
            StatusText: statusText,
            RestoreDatabaseOnLoad: restoreDatabaseOnLoad);
    }

    public void Save(HarnessSessionSaveRequest request)
    {
        var sessionState = new HarnessSessionState
        {
            LastDatabasePath = NormalizeOptionalValue(request.LastDatabasePath),
            LastPageNumber = NormalizeOptionalValue(request.LastPageNumber),
            UnlockMode = NormalizeOptionalValue(request.UnlockMode),
            KeyfilePath = NormalizeOptionalValue(request.KeyfilePath),
            RecentDatabasePaths = NormalizeRecentDatabasePaths(request.RecentDatabasePaths),
        };

        HarnessSessionState.Save(sessionStatePath, sessionState);
    }

    public List<string> PushRecentDatabasePath(IEnumerable<string> recentDatabasePaths, string path)
    {
        var normalizedPath = NormalizeOptionalValue(path);
        if (normalizedPath is null)
        {
            return NormalizeRecentDatabasePaths(recentDatabasePaths);
        }

        var updatedPaths = NormalizeRecentDatabasePaths(recentDatabasePaths);
        updatedPaths.RemoveAll(item => string.Equals(item, normalizedPath, StringComparison.OrdinalIgnoreCase));
        updatedPaths.Insert(0, normalizedPath);

        if (updatedPaths.Count > maxRecentDatabaseCount)
        {
            updatedPaths.RemoveRange(maxRecentDatabaseCount, updatedPaths.Count - maxRecentDatabaseCount);
        }

        return updatedPaths;
    }

    private List<string> NormalizeRecentDatabasePaths(IEnumerable<string> recentDatabasePaths)
    {
        List<string> normalizedPaths = [];
        foreach (var path in recentDatabasePaths)
        {
            var normalizedPath = NormalizeOptionalValue(path);
            if (normalizedPath is null || normalizedPaths.Any(item => string.Equals(item, normalizedPath, StringComparison.OrdinalIgnoreCase)))
            {
                continue;
            }

            normalizedPaths.Add(normalizedPath);
            if (normalizedPaths.Count == maxRecentDatabaseCount)
            {
                break;
            }
        }

        return normalizedPaths;
    }

    private static string? NormalizeOptionalValue(string? value)
    {
        return string.IsNullOrWhiteSpace(value) ? null : value.Trim();
    }
}

internal sealed record HarnessSessionLoadResult(
    IReadOnlyList<string> RecentDatabasePaths,
    string? LastDatabasePath,
    string? LastPageNumber,
    string? UnlockMode,
    string? KeyfilePath,
    string? CurrentDatabaseTitleText,
    string? CurrentDatabaseDetailText,
    string? StatusText,
    bool RestoreDatabaseOnLoad);

internal sealed record HarnessSessionSaveRequest(
    string? LastDatabasePath,
    string? LastPageNumber,
    string? UnlockMode,
    string? KeyfilePath,
    IReadOnlyList<string> RecentDatabasePaths);