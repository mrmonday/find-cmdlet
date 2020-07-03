# Base image for find-cmdlet scraping
FROM mcr.microsoft.com/windows/servercore:2004

RUN powershell "Install-PackageProvider Nuget -Force"
RUN powershell "Install-Module PowerShellGet -RequiredVersion 2.2.3 -Force"
