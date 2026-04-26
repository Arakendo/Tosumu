using Tosumu.Cli;

namespace Tosumu.WpfHarness;

internal sealed class HarnessInspectClient
{
    private TosumuCliTool? cli;

    public string ExecutablePath => GetCli().ExecutablePath;

    public Task<TosumuInspectHeaderPayload> GetHeaderAsync(string path)
        => GetCli().GetHeaderAsync(path);

    public Task<TosumuInspectTreePayload> GetTreeAsync(string path, TosumuInspectUnlockOptions? unlock)
        => GetCli().GetTreeAsync(path, unlock);

    public Task<TosumuInspectVerifyPayload> GetVerifyAsync(string path, TosumuInspectUnlockOptions? unlock)
        => GetCli().GetVerifyAsync(path, unlock);

    public Task<TosumuInspectPagesPayload> GetPagesAsync(string path, TosumuInspectUnlockOptions? unlock)
        => GetCli().GetPagesAsync(path, unlock);

    public Task<TosumuInspectPagePayload> GetPageAsync(string path, ulong pageNumber, TosumuInspectUnlockOptions? unlock)
        => GetCli().GetPageAsync(path, pageNumber, unlock);

    public Task<TosumuInspectProtectorsPayload> GetProtectorsAsync(string path)
        => GetCli().GetProtectorsAsync(path);

    public Task<TosumuInspectWalPayload> GetWalAsync(string path)
        => GetCli().GetWalAsync(path);

    private TosumuCliTool GetCli()
    {
        cli ??= new TosumuCliTool();
        return cli;
    }
}