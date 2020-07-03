<#
    .Synopsis

    Run all scraping and indexing
#>
Set-Location $PSScriptRoot

Start-Transcript -Outputdirectory transcripts

# Build images

Push-Location image

Write-Host "Pulling latest server core image..."
docker.exe pull mcr.microsoft.com/windows/servercore:2004

Write-Host "Building base image..."
docker.exe build -f base.Dockerfile -t findcmdlet-base:latest .

Write-Host "Building findmodules-psgallery image..."
docker.exe build -f findmodules-psgallery.Dockerfile -t findcmdlet-findmodules-psgallery:latest .

Write-Host "Building runner-builtin image..."
docker.exe build -f runner-builtin.Dockerfile -t findcmdlet-runner-builtin:latest .

Write-Host "Building runner-rsat image..."
docker.exe build -f runner-rsat.Dockerfile -t findcmdlet-runner-rsat:latest .

Write-Host "Building runner-psgallery image..."
docker.exe build -f runner-psgallery.Dockerfile -t findcmdlet-runner-psgallery:latest .

Pop-Location

# Create data directory
$dataDir = '.\data'
New-Item -Type Directory -Path $dataDir -Force > $null
$dataDir = (Resolve-Path -Path $dataDir).Path
compact.exe /C $dataDir
Set-Location $dataDir

# Meta directory contains module info from PSGallery
$metaPath = Join-Path -Path $dataDir -ChildPath 'metadata'
New-Item -Type Directory -Path $metaPath -Force > $null

# Module path contains the actual installed modules and their dependencies
$modPath = Join-Path -Path $dataDir -ChildPath 'modules'
New-Item -Type Directory -Path $modPath -Force > $null

# Doc path contains the documentation for each module
$rootDocPath = Join-Path -Path $dataDir -ChildPath 'docs'
New-Item -Type Directory -Path $rootDocPath -Force > $null

# Find and save powershell gallery modules
$metaVol = "$($metaPath):C:\metadata"
$modVol = "$($modPath):C:\modules"
docker.exe run --rm -v $metaVol -v $modVol findcmdlet-findmodules-psgallery:latest

# Get documentation for psgallery commands

$numCores = Get-CimInstance Win32_Processor | Select-Object -ExpandProperty NumberOfLogicalProcessors
$files = Get-ChildItem $metaPath

foreach ($file in $files) {
    $i = 1
    while ($(Get-Job -State Running).Count -ge $numCores) {
        Start-Sleep -Seconds ($i * $i + (Get-Random -Minimum 1 -Maximum 5))
        $i += 1
    }

    $completedJobs = Get-Job -State Completed
    $completedJobs | Receive-Job
    $completedJobs | Remove-Job

    Start-Job -ScriptBlock {
        param (
            $dataDir,
            $file
        )

        Set-Location $dataDir

        $mod = Get-Content $file.FullName | ConvertFrom-Json

        $docPath = "$dataDir\docs\$($mod.Name)\$($mod.Version)"
        New-Item -Type Directory $docPath -Force > $null

        $modPath = "$dataDir\modules\$($mod.Name)\$($mod.Version)"
        New-Item -Type Directory $modPath -Force > $null

        $donePath = Join-Path -Path $docPath -ChildPath 'done.txt'
        if (!(Test-Path $donePath)) {
            Remove-Item -Path $docPath -Recurse -Force 
            New-Item -Type Directory $docPath -Force > $null

            $modVol = "$($modPath):C:\modules"
            $docVol = "$($docPath):C:\docs"

            docker.exe run --rm -v $modVol -v $docVol findcmdlet-runner-psgallery:latest $mod.Name $mod.Version
            Set-Content -Path $donePath -Value "done"
        }
    } -ArgumentList @($dataDir, $file) > $null
}

# Get documentation for builtin/snapin commands
$rootDocVol = "$($rootDocPath):C:\docs"
docker.exe run --rm -v $metaVol -v $rootDocVol findcmdlet-runner-builtin:latest

# Get documentation for builtin/RSAT module commands
docker.exe run --rm -v $metaVol -v $rootDocVol findcmdlet-runner-rsat:latest

# TODO: Run indexer

Stop-Transcript
