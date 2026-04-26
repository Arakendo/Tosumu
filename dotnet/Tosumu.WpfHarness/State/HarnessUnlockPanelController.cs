using System.Windows;
using System.Windows.Controls;

namespace Tosumu.WpfHarness;

internal sealed class HarnessUnlockPanelController
{
    private readonly ComboBox unlockModeComboBox;
    private readonly TextBlock secretLabelTextBlock;
    private readonly PasswordBox secretPasswordBox;
    private readonly TextBox keyfilePathTextBox;
    private readonly Button browseKeyfileButton;
    private readonly Action<string>? setHintText;
    private readonly bool keepLabelVisible;
    private readonly string defaultMode;

    public HarnessUnlockPanelController(
        ComboBox unlockModeComboBox,
        TextBlock secretLabelTextBlock,
        PasswordBox secretPasswordBox,
        TextBox keyfilePathTextBox,
        Button browseKeyfileButton,
        string defaultMode,
        Action<string>? setHintText = null,
        bool keepLabelVisible = false)
    {
        this.unlockModeComboBox = unlockModeComboBox;
        this.secretLabelTextBlock = secretLabelTextBlock;
        this.secretPasswordBox = secretPasswordBox;
        this.keyfilePathTextBox = keyfilePathTextBox;
        this.browseKeyfileButton = browseKeyfileButton;
        this.defaultMode = defaultMode;
        this.setHintText = setHintText;
        this.keepLabelVisible = keepLabelVisible;
    }

    public string GetSelectedUnlockMode()
    {
        return (unlockModeComboBox.SelectedItem as ComboBoxItem)?.Tag as string ?? defaultMode;
    }

    public void SelectUnlockMode(string mode)
    {
        var desiredMode = mode == HarnessUnlockModes.Auto && defaultMode != HarnessUnlockModes.Auto
            ? defaultMode
            : mode;

        foreach (var item in unlockModeComboBox.Items)
        {
            if (item is ComboBoxItem comboBoxItem && string.Equals(comboBoxItem.Tag as string, desiredMode, StringComparison.Ordinal))
            {
                unlockModeComboBox.SelectedItem = comboBoxItem;
                return;
            }
        }

        unlockModeComboBox.SelectedIndex = 0;
    }

    public void UpdateModeInputs()
    {
        var selectedMode = GetSelectedUnlockMode();
        var usesSecret = selectedMode is HarnessUnlockModes.Passphrase or HarnessUnlockModes.RecoveryKey;
        var usesKeyfile = selectedMode == HarnessUnlockModes.Keyfile;

        secretLabelTextBlock.Visibility = keepLabelVisible || usesSecret ? Visibility.Visible : Visibility.Collapsed;
        secretPasswordBox.Visibility = usesSecret ? Visibility.Visible : Visibility.Collapsed;
        keyfilePathTextBox.Visibility = usesKeyfile ? Visibility.Visible : Visibility.Collapsed;
        browseKeyfileButton.Visibility = usesKeyfile ? Visibility.Visible : Visibility.Collapsed;

        if (usesKeyfile)
        {
            secretPasswordBox.Password = string.Empty;
            secretLabelTextBlock.Text = "Keyfile path";
        }
        else
        {
            secretLabelTextBlock.Text = selectedMode == HarnessUnlockModes.RecoveryKey ? "Recovery key" : "Passphrase";
        }

        if (!usesKeyfile)
        {
            keyfilePathTextBox.Text = string.Empty;
        }

        if (!usesSecret && !keepLabelVisible)
        {
            secretPasswordBox.Password = string.Empty;
        }

        setHintText?.Invoke(selectedMode switch
        {
            HarnessUnlockModes.Auto => "You will be prompted only if the current inspect action actually requires credentials.",
            HarnessUnlockModes.Passphrase => "Use this when the database should unlock with a passphrase piped to the CLI.",
            HarnessUnlockModes.RecoveryKey => "Use this when you need the recovery key instead of a passphrase.",
            HarnessUnlockModes.Keyfile => "Use this when the database should unlock from a keyfile path instead of typed secret input.",
            _ => "Choose how inspect commands should unlock the current database."
        });
    }

    public void ApplyUnlockSelection(HarnessUnlockSelection? unlockSelection)
    {
        SelectUnlockMode(unlockSelection?.Mode ?? HarnessUnlockModes.Auto);

        if (unlockSelection is null)
        {
            secretPasswordBox.Password = string.Empty;
            keyfilePathTextBox.Text = string.Empty;
            return;
        }

        switch (unlockSelection.Mode)
        {
            case HarnessUnlockModes.Passphrase:
            case HarnessUnlockModes.RecoveryKey:
                secretPasswordBox.Password = unlockSelection.Value;
                break;
            case HarnessUnlockModes.Keyfile:
                keyfilePathTextBox.Text = unlockSelection.Value;
                break;
        }

        UpdateModeInputs();
    }
}