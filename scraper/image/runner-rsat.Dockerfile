# Image containing RSAT modules
FROM findcmdlet-base:latest

RUN powershell "Install-WindowsFeature -Name Hyper-V-PowerShell"
RUN powershell "Install-WindowsFeature -Name RSAT-System-Insights"
RUN powershell "Install-WindowsFeature -Name RSAT-AD-PowerShell"
RUN powershell "Install-WindowsFeature -Name RSAT-DHCP"
RUN powershell "Install-WindowsFeature -Name RSAT-DNS-Server"
RUN powershell "Install-WindowsFeature -Name UpdateServices-API"

# These features contain cmdlets but don't install in Docker as of WSC2004
#RUN powershell "Install-WindowsFeature -Name RSAT-DataCenterBridging-LLDP-Tools"
#RUN powershell "Install-WindowsFeature -Name RSAT-Clustering-PowerShell"
#RUN powershell "Install-WindowsFeature -Name RSAT-Storage-Replica"
#RUN powershell "Install-WindowsFeature -Name RSAT-RemoteAccess-PowerShell"
#RUN powershell "Install-WindowsFeature -Name WindowsStorageManagementService"
#RUN powershell "Install-WindowsFeature -Name Migration"

COPY "runner-rsat.ps1" "C:\\runner-rsat.ps1"
COPY "runner-installed.ps1" "C:\\runner-installed.ps1"

VOLUME "C:\\metadata"
VOLUME "C:\\docs"

ENTRYPOINT [ "powershell", ".\\runner-rsat.ps1", "C:\\metadata", "C:\\docs" ]
