FROM findcmdlet-base:latest

COPY "runner-installed.ps1" "C:\\runner-installed.ps1"

VOLUME "C:\\modules"
VOLUME "C:\\docs"

ENTRYPOINT [ "powershell", ".\\runner-installed.ps1", "C:\\modules", "C:\\docs" ]