using System.IO;
using System.Text.Json;

namespace Tosumu.WpfHarness;

internal sealed class HarnessSessionState
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = true,
    };

    public string? LastDatabasePath { get; init; }

    public string? LastPageNumber { get; init; }

    public string? UnlockMode { get; init; }

    public string? KeyfilePath { get; init; }

    public List<string> RecentDatabasePaths { get; init; } = [];

    public static HarnessSessionState Load(string path)
    {
        try
        {
            if (!File.Exists(path))
            {
                return new HarnessSessionState();
            }

            var json = File.ReadAllText(path);
            return JsonSerializer.Deserialize<HarnessSessionState>(json, JsonOptions) ?? new HarnessSessionState();
        }
        catch
        {
            return new HarnessSessionState();
        }
    }

    public static void Save(string path, HarnessSessionState state)
    {
        var directory = Path.GetDirectoryName(path);
        if (!string.IsNullOrWhiteSpace(directory))
        {
            Directory.CreateDirectory(directory);
        }

        var json = JsonSerializer.Serialize(state, JsonOptions);
        File.WriteAllText(path, json);
    }
}