FROM findcmdlet-base:latest

COPY "runner-builtin.ps1" "C:\\runner-builtin.ps1"

VOLUME "C:\\metadata"
VOLUME "C:\\docs"

ENTRYPOINT [ "powershell", ".\\runner-builtin.ps1", "C:\\metadata", "C:\\docs" ]