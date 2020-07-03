# Find and install all PowerShell Gallery modules
FROM findcmdlet-base:latest

COPY "findmodules-psgallery.ps1" "C:\\findmodules-psgallery.ps1"

VOLUME "C:\\metadata"
VOLUME "C:\\modules"

ENTRYPOINT [ "powershell", ".\\findmodules-psgallery.ps1", "C:\\metadata", "C:\\modules" ]