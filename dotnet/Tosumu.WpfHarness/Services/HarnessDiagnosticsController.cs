using System.Collections.Concurrent;
using System.Text;
using System.Windows.Controls;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessDiagnosticsController
{
    private readonly ConcurrentQueue<string> debugLogBuffer = new();
    private readonly TextBox debugLogTextBox;
    private readonly ScrollViewer debugLogScrollViewer;
    private readonly int maxDebugLogChars;

    public HarnessDiagnosticsController(TextBox debugLogTextBox, ScrollViewer debugLogScrollViewer, int maxDebugLogChars)
    {
        this.debugLogTextBox = debugLogTextBox;
        this.debugLogScrollViewer = debugLogScrollViewer;
        this.maxDebugLogChars = maxDebugLogChars;
    }

    public void LogDebug(string message)
    {
        debugLogBuffer.Enqueue($"{DateTime.Now:HH:mm:ss.fff}  {message}");
    }

    public void FlushDebugLog()
    {
        if (debugLogBuffer.IsEmpty)
        {
            return;
        }

        var text = new StringBuilder();
        while (debugLogBuffer.TryDequeue(out var line))
        {
            text.AppendLine(line);
        }

        if (debugLogTextBox.Text.Length > maxDebugLogChars)
        {
            debugLogTextBox.Clear();
            debugLogTextBox.AppendText($"{DateTime.Now:HH:mm:ss.fff}  [debug] Cleared previous output after reaching {maxDebugLogChars} characters.{Environment.NewLine}");
        }

        debugLogTextBox.AppendText(text.ToString());
        debugLogScrollViewer.ScrollToEnd();
    }

    public void ClearDebugLog()
    {
        debugLogTextBox.Clear();
        while (debugLogBuffer.TryDequeue(out _))
        {
        }

        LogDebug("Debug console cleared.");
    }

    public void LogInspectFailure(string operationName, TosumuInspectCommandException ex)
    {
        var pgnoText = ex.Pgno is ulong pgno ? $", pgno={pgno}" : string.Empty;
        LogDebug($"{operationName} failed: command={ex.Command}, kind={ex.ErrorKind ?? "unknown"}, exit={ex.ExitCode}{pgnoText}, message={FlattenForLog(ex.Message)}");

        if (!string.IsNullOrWhiteSpace(ex.StandardError))
        {
            LogDebug($"stderr: {FlattenForLog(ex.StandardError)}");
        }

        if (!string.IsNullOrWhiteSpace(ex.StandardOutput))
        {
            LogDebug($"stdout: {FlattenForLog(ex.StandardOutput)}");
        }
    }

    public void LogException(string operationName, Exception ex)
    {
        LogDebug($"{operationName} failed with {ex.GetType().Name}: {FlattenForLog(ex.Message)}");
    }

    private static string FlattenForLog(string? text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return "(empty)";
        }

        var flattened = text.Replace("\r", string.Empty).Replace("\n", " | ").Trim();
        return flattened.Length <= 800 ? flattened : flattened[..800] + "...";
    }
}