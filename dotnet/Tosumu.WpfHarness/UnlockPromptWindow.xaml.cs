using System.Windows;
using System.Windows.Controls;
using Microsoft.Win32;

namespace Tosumu.WpfHarness;

public partial class UnlockPromptWindow : Window
{
    private readonly HarnessUnlockPanelController unlockPanelController;

    public UnlockPromptWindow(string promptMessage, string initialMode, string initialKeyfilePath)
    {
        InitializeComponent();
        unlockPanelController = new HarnessUnlockPanelController(
            UnlockModeComboBox,
            SecretLabelTextBlock,
            SecretPasswordBox,
            KeyfilePathTextBox,
            BrowseKeyfileButton,
            HarnessUnlockModes.Passphrase,
            keepLabelVisible: true);
        PromptTextBlock.Text = promptMessage;
        unlockPanelController.SelectUnlockMode(initialMode);
        KeyfilePathTextBox.Text = initialKeyfilePath;
        unlockPanelController.UpdateModeInputs();
    }

    internal HarnessUnlockSelection? UnlockSelection { get; private set; }

    private void UnlockModeComboBox_OnSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        unlockPanelController.UpdateModeInputs();
    }

    private void BrowseKeyfileButton_OnClick(object sender, RoutedEventArgs e)
    {
        var dialog = new OpenFileDialog
        {
            Title = "Choose a Tosumu keyfile",
            Filter = "All files (*.*)|*.*",
            CheckFileExists = true,
            Multiselect = false,
        };

        if (dialog.ShowDialog(this) == true)
        {
            KeyfilePathTextBox.Text = dialog.FileName;
        }
    }

    private void CancelButton_OnClick(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
    }

    private void RetryButton_OnClick(object sender, RoutedEventArgs e)
    {
        switch (unlockPanelController.GetSelectedUnlockMode())
        {
            case HarnessUnlockModes.Passphrase:
            case HarnessUnlockModes.RecoveryKey:
                var secret = SecretPasswordBox.Password;
                if (string.IsNullOrWhiteSpace(secret))
                {
                    MessageBox.Show(this, "Enter the secret before retrying.", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Information);
                    return;
                }

                UnlockSelection = new HarnessUnlockSelection(unlockPanelController.GetSelectedUnlockMode(), secret);
                DialogResult = true;
                return;
            case HarnessUnlockModes.Keyfile:
                var keyfilePath = KeyfilePathTextBox.Text.Trim();
                if (string.IsNullOrWhiteSpace(keyfilePath))
                {
                    MessageBox.Show(this, "Choose a keyfile path before retrying.", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Information);
                    return;
                }

                if (!System.IO.File.Exists(keyfilePath))
                {
                    MessageBox.Show(this, $"Keyfile not found:\n{keyfilePath}", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Warning);
                    return;
                }

                UnlockSelection = new HarnessUnlockSelection(HarnessUnlockModes.Keyfile, keyfilePath);
                DialogResult = true;
                return;
            default:
                MessageBox.Show(this, "Choose an unlock mode before retrying.", "Tosumu Harness", MessageBoxButton.OK, MessageBoxImage.Information);
                return;
        }
    }
}