<#
    .Synopsis

    Find and install all PowerShell Gallery modules

    .Parameter MetaDataDir 

    Directory to store information about the module

    .Parameter ModuleDir

    Directory to save modules to
#>
param (
    [IO.DirectoryInfo]
    $MetaDataDir,

    [IO.DirectoryInfo]
    $ModuleDir
)

Write-Host "Finding PowerShell Gallery modules..."

New-item -Type Directory -Force $MetaDataDir
$MetaDataDir = (Resolve-Path $MetaDataDir).Path

Find-Module | ForEach-Object {
    Write-Host "Processing $($_.Name) [$($_.Version)]"

    $metaPath = Join-Path -Path $MetaDataDir -ChildPath "$($_.Name).json"
    $metaJson = $_ | ConvertTo-Json
    [System.IO.File]::WriteAllLines($metaPath, $metaJson)

    $modPath = Join-Path -Path (Join-Path -Path $ModuleDir -ChildPath "$($_.Name)") -ChildPath "$($_.Version)"
    $donePath = Join-Path -Path $modPath -ChildPath "done.txt"

    if (!(Test-Path $donePath)) {
        Write-Host "Saving $($_.Name) [$($_.Version)]: $modPath"

        Remove-Item -Recurse -Force -LiteralPath $modPath 2>&1 > $null
        New-Item -Type Directory -Force $modPath > $null
        $_ | Save-Module -AcceptLicense -Confirm:$false -Force -Path $modPath
        Set-Content -Path $donePath -Value "done"
    }
}
