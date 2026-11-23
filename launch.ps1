$ErrorActionPreference = 'Stop'
$panelData = if ($env:ARK_PANEL_DATA) { $env:ARK_PANEL_DATA } else { Join-Path $env:LOCALAPPDATA 'ArkServerPanel' }
New-Item -ItemType Directory -Force -Path $panelData | Out-Null
try {
    # The bootstrap executable is stable; its Rust supervisor selects updated versions.
    $panelExecutable = Join-Path $panelData 'bootstrap\ark-server-panel.exe'
    $localExecutable = Join-Path $PSScriptRoot 'target\release\ark-server-panel.exe'
    if (Test-Path -LiteralPath $localExecutable) { $panelExecutable = $localExecutable }
    if (-not (Test-Path -LiteralPath $panelExecutable)) {
        $release = Invoke-RestMethod -Uri 'https://api.github.com/repos/oxo140/ark-server-panel/releases/latest' -Headers @{ 'User-Agent'='ark-server-panel-bootstrap' }
        $manifestAsset = $release.assets | Where-Object name -EQ 'release.json' | Select-Object -First 1
        if (-not $manifestAsset) { throw 'La publication ne contient pas release.json.' }
        $manifest = Invoke-RestMethod -Uri $manifestAsset.browser_download_url
        if ($manifest.name -ne 'ark-server-panel' -or $manifest.asset -ne 'ark-server-panel-windows-x64.exe' -or $manifest.sha256 -notmatch '^[a-f0-9]{64}$' -or $manifest.sourceSha -notmatch '^[a-f0-9]{40}$' -or $release.tag_name -ne ('v' + $manifest.version) -or $manifest.size -le 0 -or $manifest.size -gt 209715200) { throw 'Manifeste de publication invalide.' }
        $commit = Invoke-RestMethod -Uri ('https://api.github.com/repos/oxo140/ark-server-panel/commits/' + $release.tag_name) -Headers @{ 'User-Agent'='ark-server-panel-bootstrap' }
        if ($commit.sha -ne $manifest.sourceSha) { throw 'Le manifeste ne correspond pas au commit publié.' }
        $asset = $release.assets | Where-Object name -EQ $manifest.asset | Select-Object -First 1
        if (-not $asset -or -not $asset.browser_download_url.StartsWith('https://github.com/oxo140/ark-server-panel/releases/download/')) { throw 'Adresse de téléchargement invalide.' }
        New-Item -ItemType Directory -Force -Path (Split-Path $panelExecutable) | Out-Null
        $download = $panelExecutable + '.download'
        Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $download
        if ((Get-Item -LiteralPath $download).Length -ne $manifest.size -or (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.sha256) { throw 'Le contrôle SHA-256 a échoué.' }
        Move-Item -LiteralPath $download -Destination $panelExecutable -Force
    }
    # A second launch opens the existing server instead of starting another instance.
    $panelBind = if ($env:ARK_PANEL_BIND) { $env:ARK_PANEL_BIND } else { '127.0.0.1' }
    $panelPort = if ($env:ARK_PANEL_PORT) { $env:ARK_PANEL_PORT } else { '3080' }
    $alreadyRunning = $false
    try { $health = Invoke-RestMethod -Uri "http://${panelBind}:${panelPort}/health" -TimeoutSec 2; $alreadyRunning = $health.ok -eq $true } catch { }
    if ($alreadyRunning) {
        $accessKey = (Get-Content -LiteralPath (Join-Path $panelData 'access-key.txt') -Raw).Trim()
        Start-Process "http://${panelBind}:${panelPort}/#$accessKey"
    } else {
        Start-Process -FilePath $panelExecutable -WorkingDirectory $PSScriptRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $panelData 'launcher.log') -RedirectStandardError (Join-Path $panelData 'launcher-error.log')
    }
} catch {
    $_.Exception.Message | Out-File -LiteralPath (Join-Path $panelData 'bootstrap-error.log') -Encoding utf8
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'ARK Server Panel') | Out-Null
}
