using System.Diagnostics;

namespace Tosumu.Cli;

public sealed class TosumuCliTool
{
    public TosumuCliTool(string? executablePath = null)
    {
        ExecutablePath = executablePath ?? ResolveExecutablePath();
    }

    public string ExecutablePath { get; }

    public static string ResolveExecutablePath()
    {
        string[] candidates =
        {
            Path.Combine(AppContext.BaseDirectory, "tosumu.exe"),
            Path.Combine(Path.GetDirectoryName(typeof(TosumuCliTool).Assembly.Location) ?? AppContext.BaseDirectory, "tosumu.exe"),
        };

        foreach (var candidate in candidates)
        {
            if (File.Exists(candidate))
            {
                return candidate;
            }
        }

        throw new FileNotFoundException(
            "Could not locate tosumu.exe next to the consuming application's output. Pack Tosumu.Cli and ensure the package build targets ran.",
            candidates[0]);
    }

    public async Task<TosumuCommandResult> RunAsync(IEnumerable<string> arguments, CancellationToken cancellationToken = default)
    {
        using var process = new Process
        {
            StartInfo = BuildStartInfo(arguments),
        };

        process.Start();

        var stdoutTask = process.StandardOutput.ReadToEndAsync(cancellationToken);
        var stderrTask = process.StandardError.ReadToEndAsync(cancellationToken);

        await process.WaitForExitAsync(cancellationToken).ConfigureAwait(false);

        var stdout = await stdoutTask.ConfigureAwait(false);
        var stderr = await stderrTask.ConfigureAwait(false);

        return new TosumuCommandResult(process.ExitCode, stdout, stderr);
    }

    public Task<TosumuCommandResult> RunAsync(params string[] arguments) =>
        RunAsync((IEnumerable<string>)arguments, CancellationToken.None);

    public Task<TosumuCommandResult> RunAsync(CancellationToken cancellationToken = default, params string[] arguments) =>
        RunAsync((IEnumerable<string>)arguments, cancellationToken);

    private ProcessStartInfo BuildStartInfo(IEnumerable<string> arguments)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = ExecutablePath,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
        };

        foreach (var argument in arguments)
        {
            startInfo.ArgumentList.Add(argument);
        }

        return startInfo;
    }
}

public sealed record TosumuCommandResult(int ExitCode, string StandardOutput, string StandardError)
{
    public void EnsureSuccess()
    {
        if (ExitCode == 0)
        {
            return;
        }

        throw new InvalidOperationException(
            $"tosumu exited with code {ExitCode}{Environment.NewLine}stdout:{Environment.NewLine}{StandardOutput}{Environment.NewLine}stderr:{Environment.NewLine}{StandardError}");
    }
}