param (
    [IO.DirectoryInfo]
    $MetaDataDir,

    [IO.DirectoryInfo]
    $RootDocDir
)

# Exclude modules which  also exist on PSGallery
$exclude = @('PackageManagement', 'Pester', 'PowerShellGet', 'PSReadline')

$allModules = Get-Module -ListAvailable | Where-Object Name -notin $exclude

foreach ($module in $allModules) {
    $moduleVersion = $module.Version.ToString()

    $metadata = @{
        Name = $module.Name
        Version = $moduleVersion
    }
    $metaPath = Join-Path -Path $MetaDataDir -ChildPath "$($module.Name).json"
    $metaJson = $metadata | ConvertTo-Json
    [System.IO.File]::WriteAllLines($metaPath, $metaJson)

    $docDir = Join-Path -Path $RootDocDir -ChildPath $module.Name
    $docDir = Join-Path -Path $docDir -ChildPath $moduleVersion
    New-Item -Type Directory $docDir -Force > $null

    .\runner-installed.ps1 -ModuleDir $null -DocDir $docDir -ModuleName $module.Name -ModuleVersion $moduleVersion
}
