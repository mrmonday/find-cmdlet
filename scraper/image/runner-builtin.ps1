<#
    .Synopsis

    Pull metadata and cmdlet documentation for built-in commands

    .Parameter MetaDataDir

    Directory for generated module metadata

    .Parameter RootDocDir

    Root directory for documentation
#>
param (
    [IO.DirectoryInfo]
    $MetaDataDir,

    [IO.DirectoryInfo]
    $RootDocDir
)

$snapins = Get-PSSnapin
foreach ($snapin in $snapins)
{
    Write-Host "Processing snap-in $($snapin.Name) [$($snapin.PSVersion)]..."
    $assembly = [System.Reflection.Assembly]::Load($snapin.AssemblyName);

    $metadata = @{
        Name = $snapin.Name
        Version = $snapin.PSVersion
    }
    $metaPath = Join-Path -Path $MetaDataDir -ChildPath "$($snapin.Name).json"
    $metaJson = $metadata | ConvertTo-Json
    [System.IO.File]::WriteAllLines($metaPath, $metaJson)

    $docDir = Join-Path -Path $RootDocDir -ChildPath $snapin.Name
    $docDir = Join-Path -Path $docDir -ChildPath $snapin.PSVersion
    New-Item -Type Directory $docDir -Force > $null

    $module = @{
        ProjectUri = 'https://docs.microsoft.com/en-us/powershell/scripting/overview'
        Tags = @('Builtin')
    }
    $modJson = $module | ConvertTo-Json
    $modPath = Join-Path -Path $docDir -ChildPath "mod.json"
    [System.IO.File]::WriteAllLines($modPath, $modJson)

    Update-Help -Module $snapin.Name -Force

    $commandDir = Join-Path -Path $docDir -ChildPath "commands"
    $helpDir = Join-Path -Path $docDir -ChildPath "help"

    New-Item -Type Directory $commandDir -Force > $null
    New-Item -Type Directory $helpDir -Force > $null

    $commands = $assembly.ExportedTypes | Where-Object {
        [System.Management.Automation.PSCmdlet].IsAssignableFrom($_.BaseType)
    }

    foreach ($command in $commands)
    {
        $cmdletAttr = $command.GetCustomAttributes([System.Management.Automation.CmdletAttribute], $true)
        if ($null -eq $cmdletAttr -or $cmdletAttr.Length -eq 0)
        {
            Write-Host "Skipping $($command.Name): No CmdletAttribute"
            continue
        }
        $cmdletAttr = $cmdletAttr[0]

        $commandName = "$($cmdletAttr.VerbName)-$($cmdletAttr.NounName)"

        $jsonName = "$($commandName).json"
        $cmdJson = Get-Command -Name $commandName | ConvertTo-Json
        $cmdPath = Join-Path -Path $commandDir -ChildPath $jsonName
        [System.IO.File]::WriteAllLines($cmdPath, $cmdJson)

        $helpJson = Get-Help -Full $commandName | ConvertTo-Json
        $helpPath = Join-Path -Path $helpDir -ChildPath $jsonName
        [System.IO.File]::WriteAllLines($helpPath, $helpJson)
    }
}
