# Tosumu.Cli

This package ships the `tosumu` command-line executable together with a small .NET wrapper that can locate and run the packaged tool.

Current scope:

- Windows packaging only
- CLI-backed integration, not direct native interop
- Intended for local integration / smoke testing while the Rust engine matures

Typical local workflow from the repository root:

```powershell
.\dotnet\Invoke-TosumuDotNetChecks.ps1
```

Manual equivalent:

```powershell
dotnet pack .\dotnet\Tosumu.Cli\Tosumu.Cli.csproj -c Release -o .\dotnet\.artifacts\packages
dotnet test .\dotnet\Tosumu.Cli.IntegrationTests\Tosumu.Cli.IntegrationTests.csproj -c Release -p:RestoreForce=true -p:RestoreNoCache=true
```