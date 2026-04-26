using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessTreeInspectorState
{
    private sealed record TreePageVisitState(ulong PageNumber, string PageTypeName);

    private const int MaxTreePageHistoryCount = 10;

    private readonly List<TreePageVisitState> treePageVisitStates = [];

    public TosumuInspectTreePayload? Snapshot { get; private set; }

    public ulong? RootPageNumber { get; private set; }

    public ulong? FocusPageNumber { get; private set; }

    public string FocusPageTypeName { get; private set; } = string.Empty;

    public void ApplyHeader(TosumuInspectHeaderPayload header)
    {
        RootPageNumber = header.RootPage;
    }

    public void ApplyTreeSnapshot(TosumuInspectTreePayload snapshot)
    {
        Snapshot = snapshot;
        RootPageNumber = snapshot.RootPgno;
    }

    public void SetFocusPage(ulong pageNumber, string pageTypeName)
    {
        FocusPageNumber = pageNumber;
        FocusPageTypeName = pageTypeName;

        treePageVisitStates.RemoveAll(entry => entry.PageNumber == pageNumber);
        treePageVisitStates.Insert(0, new TreePageVisitState(pageNumber, pageTypeName));

        while (treePageVisitStates.Count > MaxTreePageHistoryCount)
        {
            treePageVisitStates.RemoveAt(treePageVisitStates.Count - 1);
        }
    }

    public void Reset()
    {
        Snapshot = null;
        RootPageNumber = null;
        FocusPageNumber = null;
        FocusPageTypeName = string.Empty;
        treePageVisitStates.Clear();
    }

    public string BuildRootText()
    {
        return RootPageNumber is ulong rootPage
            ? $"Root page {rootPage} · {DescribeTreeNode(rootPage, rootPage == FocusPageNumber ? FocusPageTypeName : null)}"
            : "Root page: load a database header to discover the tree root.";
    }

    public string BuildFocusText()
    {
        if (FocusPageNumber is ulong focusPage)
        {
            return $"Focus page {focusPage} · {DescribeTreeNode(focusPage, FocusPageTypeName)}";
        }

        if (RootPageNumber is ulong rootPage)
        {
            return $"Focus page: inspect root {rootPage} or another page to begin tree navigation.";
        }

        return "Focus page: inspect root or another page to begin tree navigation.";
    }

    public IReadOnlyList<TreePageVisitRow> BuildVisitRows()
    {
        if (treePageVisitStates.Count == 0)
        {
            return [new TreePageVisitRow("-", "No tree pages inspected yet.", "Inspect root to begin.", true)];
        }

        return treePageVisitStates
            .Select(entry => new TreePageVisitRow(
                entry.PageNumber.ToString(),
                DescribeTreeNode(entry.PageNumber, entry.PageTypeName),
                GetTreeRelationLabel(entry.PageNumber),
                false))
            .ToList();
    }

    public TreeWebViewPayload BuildWebViewPayload(string trustText)
    {
        if (Snapshot is not null)
        {
            return new TreeWebViewPayload(
                Snapshot.RootPgno,
                FocusPageNumber,
                trustText,
                BuildTreeWebViewNode(Snapshot.Root, relationLabel: null, separatorKeyHex: null));
        }

        var rootNode = RootPageNumber is ulong rootPage
            ? new TreeWebViewNode(
                $"Page {rootPage}",
                DescribeTreeNode(rootPage, rootPage == FocusPageNumber ? FocusPageTypeName : null),
                rootPage == FocusPageNumber ? "root-focus" : "root",
                rootPage,
                rootPage == FocusPageNumber ? FocusPageTypeName : null,
                null,
                null,
                null,
                null,
                null,
                0,
                [])
            : new TreeWebViewNode(
                "Root pending",
                "Load a header to discover the root page.",
                "synthetic",
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                0,
                []);

        var observedNodes = treePageVisitStates
            .Where(entry => entry.PageNumber != RootPageNumber)
            .Select(entry => new TreeWebViewNode(
                $"Page {entry.PageNumber}",
                $"{GetTreeRelationLabel(entry.PageNumber)} · {DescribeTreeNode(entry.PageNumber, entry.PageTypeName)}",
                entry.PageNumber == FocusPageNumber ? "focus" : "visited",
                entry.PageNumber,
                entry.PageTypeName,
                null,
                null,
                GetTreeRelationLabel(entry.PageNumber),
                null,
                null,
                0,
                []))
            .ToList();

        var topLevelNodes = new List<TreeWebViewNode> { rootNode };
        if (observedNodes.Count > 0)
        {
            topLevelNodes.Add(new TreeWebViewNode(
                "Observed pages",
                $"{observedNodes.Count} inspected page{(observedNodes.Count == 1 ? string.Empty : "s")}",
                "synthetic",
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                observedNodes.Count,
                observedNodes));
        }

        return new TreeWebViewPayload(
            RootPageNumber,
            FocusPageNumber,
            trustText,
            new TreeWebViewNode(
                "Tree Inspector",
                "Observed root, focus, and inspected pages",
                "synthetic",
                null,
                null,
                null,
                null,
                null,
                null,
                null,
                topLevelNodes.Count,
                topLevelNodes));
    }

    private TreeWebViewNode BuildTreeWebViewNode(
        TosumuInspectTreeNodePayload node,
        string? relationLabel,
        string? separatorKeyHex)
    {
        var relationPrefix = string.IsNullOrWhiteSpace(relationLabel) ? string.Empty : relationLabel + " · ";
        var separatorSuffix = string.IsNullOrWhiteSpace(separatorKeyHex) ? string.Empty : $" · sep {ShortHex(separatorKeyHex)}";
        var nextLeafSuffix = node.NextLeaf is ulong nextLeaf ? $" · next {nextLeaf}" : string.Empty;
        var meta = $"{relationPrefix}{node.PageTypeName} · slots {node.SlotCount}{separatorSuffix}{nextLeafSuffix}";

        return new TreeWebViewNode(
            $"Page {node.Pgno}",
            meta,
            GetTreeVisualKind(node.Pgno),
            node.Pgno,
            node.PageTypeName,
            node.PageVersion,
            node.SlotCount,
            relationLabel,
            separatorKeyHex,
            node.NextLeaf,
            node.Children.Count,
            node.Children.Select(child => BuildTreeWebViewNode(
                child.Child,
                child.Relation,
                child.SeparatorKeyHex)).ToList());
    }

    private string GetTreeVisualKind(ulong pageNumber)
    {
        if (pageNumber == RootPageNumber && pageNumber == FocusPageNumber)
        {
            return "root-focus";
        }

        if (pageNumber == RootPageNumber)
        {
            return "root";
        }

        if (pageNumber == FocusPageNumber)
        {
            return "focus";
        }

        return "visited";
    }

    private string GetTreeRelationLabel(ulong pageNumber)
    {
        return pageNumber == FocusPageNumber
            ? pageNumber == RootPageNumber ? "Root focus" : "Focus"
            : pageNumber == RootPageNumber ? "Root" : "Visited";
    }

    private string DescribeTreeNode(ulong pageNumber, string? pageTypeName)
    {
        var label = pageTypeName switch
        {
            "Leaf" => "leaf node",
            "Internal" => "internal node",
            "Overflow" => "overflow page",
            "Free" => "free page",
            null or "" => pageNumber == RootPageNumber ? "root page" : "page",
            _ => $"{pageTypeName.ToLowerInvariant()} page",
        };

        if (pageNumber != RootPageNumber)
        {
            return char.ToUpperInvariant(label[0]) + label[1..];
        }

        return label switch
        {
            "leaf node" => "Root leaf node",
            "internal node" => "Root internal node",
            "overflow page" => "Root overflow page",
            "free page" => "Root free page",
            _ => "Root page",
        };
    }

    private static string ShortHex(string hex)
    {
        return hex.Length <= 12 ? hex : hex[..12] + "...";
    }
}

public sealed record TreePageVisitRow(string Page, string Node, string Relation, bool IsPlaceholder);

public sealed record TreeWebViewPayload(
    ulong? RootPageNumber,
    ulong? FocusPageNumber,
    string TrustText,
    TreeWebViewNode Hierarchy);

public sealed record TreeWebViewNode(
    string Label,
    string Meta,
    string VisualKind,
    ulong? PageNumber,
    string? PageTypeName,
    ulong? PageVersion,
    ushort? SlotCount,
    string? RelationLabel,
    string? SeparatorKeyHex,
    ulong? NextLeaf,
    int ChildCount,
    IReadOnlyList<TreeWebViewNode> Children);