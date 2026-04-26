using System.Collections.ObjectModel;
using System.Windows.Controls;

namespace Tosumu.WpfHarness;

internal sealed class HarnessRecentDatabaseSelectionController
{
    private readonly ObservableCollection<string> recentDatabasePaths;
    private readonly ComboBox recentDatabasesComboBox;
    private bool isUpdatingSelection;

    public HarnessRecentDatabaseSelectionController(ObservableCollection<string> recentDatabasePaths, ComboBox recentDatabasesComboBox)
    {
        this.recentDatabasePaths = recentDatabasePaths;
        this.recentDatabasesComboBox = recentDatabasesComboBox;
    }

    public string? GetSelectedPath()
    {
        if (isUpdatingSelection)
        {
            return null;
        }

        return recentDatabasesComboBox.SelectedItem as string;
    }

    public void ApplyRecentDatabasePaths(IEnumerable<string> paths, string? selectedPath = null)
    {
        recentDatabasePaths.Clear();
        foreach (var path in paths)
        {
            recentDatabasePaths.Add(path);
        }

        if (recentDatabasesComboBox is null)
        {
            return;
        }

        isUpdatingSelection = true;
        try
        {
            recentDatabasesComboBox.SelectedItem = selectedPath;
        }
        finally
        {
            isUpdatingSelection = false;
        }
    }
}