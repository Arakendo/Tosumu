using Tosumu.Cli;
using Xunit;

namespace Tosumu.Cli.IntegrationTests;

public sealed class PackagedCliTests : IDisposable
{
    private readonly string rootDirectory = Path.Combine(Path.GetTempPath(), "tosumu-dotnet", Guid.NewGuid().ToString("N"));
    private readonly TosumuCliTool cli = new();

    public PackagedCliTests()
    {
        Directory.CreateDirectory(rootDirectory);
    }

    [Fact]
    public void PackagedCli_executable_is_copied_to_test_output()
    {
        Assert.True(File.Exists(cli.ExecutablePath), $"expected packaged executable at {cli.ExecutablePath}");
    }

    [Fact]
    public async Task PackagedCli_can_roundtrip_plain_database() 
    {
        var dbPath = Path.Combine(rootDirectory, "roundtrip.tsm");

        (await cli.RunAsync("init", dbPath)).EnsureSuccess();
        (await cli.RunAsync("put", dbPath, "alpha", "one")).EnsureSuccess();
        (await cli.RunAsync("put", dbPath, "beta", "two")).EnsureSuccess();

        var get = await cli.RunAsync("get", dbPath, "alpha");
        get.EnsureSuccess();
        Assert.Equal("one", get.StandardOutput.Trim());

        var scan = await cli.RunAsync("scan", dbPath);
        scan.EnsureSuccess();
        var lines = SplitLines(scan.StandardOutput);
        Assert.Contains("alpha\tone", lines);
        Assert.Contains("beta\ttwo", lines);

        var verify = await cli.RunAsync("verify", dbPath);
        verify.EnsureSuccess();
        Assert.Contains("all pages ok", verify.StandardOutput, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public async Task PackagedCli_can_hammer_many_put_get_cycles()
    {
        var dbPath = Path.Combine(rootDirectory, "hammer.tsm");
        const int Count = 128;

        (await cli.RunAsync("init", dbPath)).EnsureSuccess();

        for (var i = 0; i < Count; i++)
        {
            var key = $"key-{i:D4}";
            var value = $"value-{i:D4}";
            (await cli.RunAsync("put", dbPath, key, value)).EnsureSuccess();

            if (i % 16 == 0)
            {
                var get = await cli.RunAsync("get", dbPath, key);
                get.EnsureSuccess();
                Assert.Equal(value, get.StandardOutput.Trim());
            }
        }

        foreach (var i in new[] { 0, 17, 63, 127 })
        {
            var key = $"key-{i:D4}";
            var expected = $"value-{i:D4}";
            var get = await cli.RunAsync("get", dbPath, key);
            get.EnsureSuccess();
            Assert.Equal(expected, get.StandardOutput.Trim());
        }

        var scan = await cli.RunAsync("scan", dbPath);
        scan.EnsureSuccess();
        Assert.Equal(Count, SplitLines(scan.StandardOutput).Length);

        var stat = await cli.RunAsync("stat", dbPath);
        stat.EnsureSuccess();
        Assert.Contains("page_count:", stat.StandardOutput, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("tree_height:", stat.StandardOutput, StringComparison.OrdinalIgnoreCase);

        var backupPath = Path.Combine(rootDirectory, "hammer-copy.tsm");
        (await cli.RunAsync("backup", dbPath, backupPath)).EnsureSuccess();

        var backupGet = await cli.RunAsync("get", backupPath, "key-0127");
        backupGet.EnsureSuccess();
        Assert.Equal("value-0127", backupGet.StandardOutput.Trim());
    }

    [Fact]
    public async Task PackagedCli_surfaces_dump_hex_verify_and_delete_behaviors()
    {
        var dbPath = Path.Combine(rootDirectory, "inspectable.tsm");

        (await cli.RunAsync("init", dbPath)).EnsureSuccess();
        (await cli.RunAsync("put", dbPath, "alpha", "one")).EnsureSuccess();
        (await cli.RunAsync("put", dbPath, "beta", "two")).EnsureSuccess();

        var dumpHeader = await cli.RunAsync("dump", dbPath);
        dumpHeader.EnsureSuccess();
        Assert.Contains("=== file header:", dumpHeader.StandardOutput, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("keyslot 0", dumpHeader.StandardOutput, StringComparison.OrdinalIgnoreCase);

        var dumpPage = await cli.RunAsync("dump", dbPath, "--page", "1");
        dumpPage.EnsureSuccess();
        Assert.Contains("=== page 1:", dumpPage.StandardOutput, StringComparison.OrdinalIgnoreCase);

        var hexPage0 = await cli.RunAsync("hex", dbPath, "--page", "0");
        hexPage0.EnsureSuccess();
        Assert.Contains("raw frame: page 0", hexPage0.StandardOutput, StringComparison.OrdinalIgnoreCase);

        var verifyExplain = await cli.RunAsync("verify", dbPath, "--explain");
        verifyExplain.EnsureSuccess();
        Assert.Contains("integrity:", verifyExplain.StandardOutput, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("btree:", verifyExplain.StandardOutput, StringComparison.OrdinalIgnoreCase);

        (await cli.RunAsync("delete", dbPath, "beta")).EnsureSuccess();

        var deletedGet = await cli.RunAsync("get", dbPath, "beta");
        Assert.Equal(1, deletedGet.ExitCode);
        Assert.Contains("not found", deletedGet.StandardError, StringComparison.OrdinalIgnoreCase);

        var scan = await cli.RunAsync("scan", dbPath);
        scan.EnsureSuccess();
        var lines = SplitLines(scan.StandardOutput);
        Assert.Contains("alpha\tone", lines);
        Assert.DoesNotContain("beta\ttwo", lines);
    }

    [Fact]
    public async Task PackagedCli_backup_rejects_existing_destination()
    {
        var sourcePath = Path.Combine(rootDirectory, "source.tsm");
        var destinationPath = Path.Combine(rootDirectory, "existing-dest.tsm");

        (await cli.RunAsync("init", sourcePath)).EnsureSuccess();
        (await cli.RunAsync("init", destinationPath)).EnsureSuccess();

        var backup = await cli.RunAsync("backup", sourcePath, destinationPath);
        Assert.Equal(1, backup.ExitCode);
        Assert.Contains("backup destination already exists", backup.StandardError, StringComparison.OrdinalIgnoreCase);
    }

    public void Dispose()
    {
        if (Directory.Exists(rootDirectory))
        {
            Directory.Delete(rootDirectory, recursive: true);
        }
    }

    private static string[] SplitLines(string text) =>
        text.Split(new[] { "\r\n", "\n" }, StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
}