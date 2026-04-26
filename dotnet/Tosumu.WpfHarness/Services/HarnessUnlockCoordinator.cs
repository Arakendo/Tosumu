using System.IO;
using System.Windows;
using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessUnlockCoordinator
{
    private readonly Window owner;
    private readonly Func<string> getSelectedUnlockMode;
    private readonly Func<string> getSecret;
    private readonly Func<string> getKeyfilePath;
    private readonly Action<HarnessUnlockSelection?> applyUnlockSelection;
    private readonly Action<bool> setBusy;
    private readonly Action<string> setStatus;
    private readonly Action<string> logDebug;
    private readonly Action<string, TosumuInspectCommandException> logInspectFailure;
    private readonly Action<string, Exception> logException;

    public HarnessUnlockCoordinator(
        Window owner,
        Func<string> getSelectedUnlockMode,
        Func<string> getSecret,
        Func<string> getKeyfilePath,
        Action<HarnessUnlockSelection?> applyUnlockSelection,
        Action<bool> setBusy,
        Action<string> setStatus,
        Action<string> logDebug,
        Action<string, TosumuInspectCommandException> logInspectFailure,
        Action<string, Exception> logException)
    {
        this.owner = owner;
        this.getSelectedUnlockMode = getSelectedUnlockMode;
        this.getSecret = getSecret;
        this.getKeyfilePath = getKeyfilePath;
        this.applyUnlockSelection = applyUnlockSelection;
        this.setBusy = setBusy;
        this.setStatus = setStatus;
        this.logDebug = logDebug;
        this.logInspectFailure = logInspectFailure;
        this.logException = logException;
    }

    public bool TryGetSelection(string operationText, out HarnessUnlockSelection? unlockSelection)
    {
        switch (getSelectedUnlockMode())
        {
            case HarnessUnlockModes.Auto:
                unlockSelection = null;
                return true;
            case HarnessUnlockModes.Passphrase:
            case HarnessUnlockModes.RecoveryKey:
                var secret = getSecret();
                if (string.IsNullOrWhiteSpace(secret))
                {
                    MessageBox.Show(owner, $"Enter the secret before trying to {operationText}.", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Information);
                    unlockSelection = null;
                    return false;
                }

                unlockSelection = new HarnessUnlockSelection(getSelectedUnlockMode(), secret);
                return true;
            case HarnessUnlockModes.Keyfile:
                var keyfilePath = getKeyfilePath().Trim();
                if (string.IsNullOrWhiteSpace(keyfilePath))
                {
                    MessageBox.Show(owner, $"Choose a keyfile path before trying to {operationText}.", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Information);
                    unlockSelection = null;
                    return false;
                }

                if (!File.Exists(keyfilePath))
                {
                    MessageBox.Show(owner, $"Keyfile not found:\n{keyfilePath}", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Warning);
                    unlockSelection = null;
                    return false;
                }

                unlockSelection = new HarnessUnlockSelection(HarnessUnlockModes.Keyfile, keyfilePath);
                return true;
            default:
                unlockSelection = null;
                return true;
        }
    }

    public async Task RunUnlockableInspectActionAsync(
        string operationName,
        HarnessUnlockSelection? unlockSelection,
        Func<TosumuInspectUnlockOptions?, Task> action)
    {
        setBusy(true);
        logDebug($"Starting {operationName} (unlock={DescribeUnlockSelection(unlockSelection)}).");

        try
        {
            while (true)
            {
                try
                {
                    await action(unlockSelection?.ToUnlockOptions());
                    logDebug($"Completed {operationName}.");
                    return;
                }
                catch (TosumuInspectCommandException ex) when (string.Equals(ex.ErrorKind, "wrong_key", StringComparison.Ordinal))
                {
                    setStatus("Unlock required or provided secret was rejected.");
                    logInspectFailure(operationName, ex);
                    setBusy(false);

                    if (!TryPromptForUnlockRetry(out unlockSelection))
                    {
                        setStatus("Unlock retry cancelled.");
                        logDebug($"Cancelled {operationName} after unlock retry prompt.");
                        return;
                    }

                    applyUnlockSelection(unlockSelection);
                    logDebug($"Retrying {operationName} with unlock={DescribeUnlockSelection(unlockSelection)}.");
                    setBusy(true);
                }
            }
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

    private bool TryPromptForUnlockRetry(out HarnessUnlockSelection? unlockSelection)
    {
        var prompt = new UnlockPromptWindow(
            "The last inspect request could not unlock the database. Choose credentials for an immediate retry.",
            getSelectedUnlockMode(),
            getKeyfilePath().Trim())
        {
            Owner = owner,
        };

        var accepted = prompt.ShowDialog() == true;
        unlockSelection = accepted ? prompt.UnlockSelection : null;
        return accepted && unlockSelection is not null;
    }

    private static string DescribeUnlockSelection(HarnessUnlockSelection? unlockSelection)
    {
        return unlockSelection?.Mode switch
        {
            HarnessUnlockModes.Passphrase => "passphrase",
            HarnessUnlockModes.RecoveryKey => "recovery-key",
            HarnessUnlockModes.Keyfile => "keyfile",
            _ => "auto",
        };
    }
}