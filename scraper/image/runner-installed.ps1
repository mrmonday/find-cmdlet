<#
    .Synopsis

    Generate cmdlet and module documentation json for a module

    .Parameter ModuleDir

    Directory where the module is saved. $null if the module is in the default path

    .Parameter DocDir

    Directory to output documentation to

    .Parameter ModuleName

    Name of the module to generate documentation for

    .Parameter ModuleVersion

    Version of the module to generate documentation for
#>
param (
    [IO.DirectoryInfo]
    $ModuleDir,

    [IO.DirectoryInfo]
    $DocDir,

    [string]
    $ModuleName,

    [string]
    $ModuleVersion
)

Write-Host "Processing $ModuleName [$ModuleVersion]"

Set-ExecutionPolicy -ExecutionPolicy Bypass -Scope CurrentUser -Force

if ($null -ne $ModuleDir) {
    $Env:PSModulePath = "$ModuleDir;$($Env:PSModulePath)"
}

$mod = Get-Module -Name $ModuleName -ListAvailable | Where-Object Version -eq $ModuleVersion

if ($mod.HelpInfoUri) {
    Update-Help -Module $ModuleName -Force
}

$modJson = $mod | ConvertTo-Json
$modPath = Join-Path -Path $DocDir -ChildPath "mod.json"
[System.IO.File]::WriteAllLines($modPath, $modJson)

$commandDir = Join-Path -Path $DocDir -ChildPath "commands"
$helpDir = Join-Path -Path $DocDir -ChildPath "help"

New-Item -Type Directory $commandDir -Force > $null
New-Item -Type Directory $helpDir -Force > $null

$mod.ExportedCommands.Values | ForEach-Object {
    $jsonName = "$($_.Name).json"
    $cmdJson = $_ | ConvertTo-Json
    $cmdPath = Join-Path -Path $commandDir -ChildPath $jsonName
    [System.IO.File]::WriteAllLines($cmdPath, $cmdJson)

    $helpJson = Get-Help -Full $_ | ConvertTo-Json
    $helpPath = Join-Path -Path $helpDir -ChildPath $jsonName
    [System.IO.File]::WriteAllLines($helpPath, $helpJson)
}