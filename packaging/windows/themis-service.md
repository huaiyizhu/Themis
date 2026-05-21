# Windows Service Installation

Install the Themis background service (requires Administrator):

```powershell
# From directory containing themis-service.exe and themis-cli.exe
themis-cli service install
themis-cli service start
```

Or using `sc.exe` directly:

```powershell
sc create ThemisService binPath= "C:\Path\To\themis-service.exe" start= auto DisplayName= "Themis Audio Capture Service"
sc start ThemisService
```

Uninstall:

```powershell
themis-cli service uninstall
```

For quick trials without admin rights, use **portable mode**: run `themis-tray` only; it will spawn `themis-service` in user context.
