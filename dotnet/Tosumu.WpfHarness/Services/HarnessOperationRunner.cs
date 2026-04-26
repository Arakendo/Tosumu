using System.Windows;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessOperationRunner
{
    private readonly Window owner;
    private readonly Action<bool> setBusy;
    private readonly Action<string> setStatus;
    private readonly Action<string> logDebug;
    private readonly Action<string, TosumuInspectCommandException> logInspectFailure;
    private readonly Action<string, Exception> logException;

    public HarnessOperationRunner(
        Window owner,
        Action<bool> setBusy,
        Action<string> setStatus,
        Action<string> logDebug,
        Action<string, TosumuInspectCommandException> logInspectFailure,
        Action<string, Exception> logException)
    {
        this.owner = owner;
        this.setBusy = setBusy;
        this.setStatus = setStatus;
        this.logDebug = logDebug;
        this.logInspectFailure = logInspectFailure;
        this.logException = logException;
    }

    public async Task RunAsync(string operationName, Func<Task> action)
    {
        setBusy(true);
        logDebug($"Starting {operationName}.");

        try
        {
            await action();
            logDebug($"Completed {operationName}.");
        }
        catch (TosumuInspectCommandException ex)
        {
            setStatus("Last command failed.");
            logInspectFailure(operationName, ex);
            MessageBox.Show(owner, ex.Message, "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Error);
        }
        catch (Exception ex)
        {
            setStatus("Last command failed.");
            logException(operationName, ex);
            MessageBox.Show(owner, ex.Message, "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Error);
        }
        finally
        {
            setBusy(false);
        }
    }
}