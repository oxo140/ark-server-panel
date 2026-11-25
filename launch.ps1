param([switch]$PrepareUpdate)
$ErrorActionPreference = 'Stop'
function Get-PublicRelease {
    Add-Type -AssemblyName System.Net.Http
    $client = New-Object System.Net.Http.HttpClient
    $client.DefaultRequestHeaders.UserAgent.ParseAdd('ark-server-panel-bootstrap')
    try {
        $repo = 'oxo140/ark-server-panel'
        $response = $client.GetAsync("https://github.com/$repo/releases/latest").GetAwaiter().GetResult()
        $response.EnsureSuccessStatusCode() | Out-Null
        $url = $response.RequestMessage.RequestUri.AbsoluteUri
        $prefix = "https://github.com/$repo/releases/tag/"
        if (-not $url.StartsWith($prefix)) { throw 'Redirection GitHub inattendue.' }
        $tag = $url.Substring($prefix.Length)
        if ($tag -notmatch '^v[0-9A-Za-z._-]+$') { throw 'Tag invalide.' }
        $base = "https://github.com/$repo/releases/download/$tag"
        $manifest = Invoke-RestMethod -Uri "$base/release.json"
        if ($manifest.name -ne 'ark-server-panel' -or $manifest.asset -ne 'ark-server-panel-windows-x64.exe' -or $manifest.sha256 -notmatch '^[a-f0-9]{64}$' -or $manifest.sourceSha -notmatch '^[a-f0-9]{40}$' -or $tag -ne ('v' + $manifest.version) -or $manifest.size -le 0 -or $manifest.size -gt 209715200) { throw 'Manifeste de publication invalide.' }
        $bytes = $client.GetByteArrayAsync("https://github.com/$repo.git/info/refs?service=git-upload-pack").GetAwaiter().GetResult()
        if ($bytes.Length -gt 2097152) { throw 'Références Git trop volumineuses.' }
        $refs = @{}; $at = 0
        while ($at -lt $bytes.Length) {
            if ($at + 4 -gt $bytes.Length) { throw 'Références Git tronquées.' }
            $length = [Convert]::ToInt32([Text.Encoding]::ASCII.GetString($bytes,$at,4),16); $at += 4
            if ($length -eq 0) { continue }
            if ($length -lt 4 -or $at + $length - 4 -gt $bytes.Length) { throw 'Paquet Git invalide.' }
            $line = [Text.Encoding]::UTF8.GetString($bytes,$at,$length-4).Split([char]0)[0].Trim(); $at += $length - 4
            if ($line -match '^([a-f0-9]{40}) (refs/\S+)$') { $refs[$Matches[2]] = $Matches[1] }
        }
        $ref = "refs/tags/$tag"; $sha = $refs[$ref]
        if ($refs.ContainsKey("$ref^{}")) { $sha = $refs["$ref^{}"] }
        if ($sha -ne $manifest.sourceSha) { throw 'Le manifeste ne correspond pas au commit publié.' }
        return @{ manifest=$manifest; url="$base/$($manifest.asset)" }
    } finally { $client.Dispose() }
}
$panelData = if ($env:ARK_PANEL_DATA) { $env:ARK_PANEL_DATA } else { Join-Path $env:LOCALAPPDATA 'ArkServerPanel' }
New-Item -ItemType Directory -Force -Path $panelData | Out-Null
try {
    # The bootstrap executable is stable; its Rust supervisor selects updated versions.
    $panelExecutable = Join-Path $panelData 'bootstrap\ark-server-panel.exe'
    $localExecutable = Join-Path $PSScriptRoot 'target\release\ark-server-panel.exe'
    if (Test-Path -LiteralPath $localExecutable) { $panelExecutable = $localExecutable }
    if ($PrepareUpdate -or -not (Test-Path -LiteralPath $panelExecutable)) {
        $stageUpdate = $PrepareUpdate -and (Test-Path -LiteralPath $panelExecutable)
        $release = Get-PublicRelease
        $manifest = $release.manifest
        $destination = $panelExecutable
        if ($stageUpdate) { $destination = Join-Path $panelData ("versions\" + [guid]::NewGuid().ToString('N') + '\ark-server-panel.exe') }
        New-Item -ItemType Directory -Force -Path (Split-Path $destination) | Out-Null
        $download = $destination + '.download'
        Invoke-WebRequest -Uri $release.url -OutFile $download
        if ((Get-Item -LiteralPath $download).Length -ne $manifest.size -or (Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash.ToLowerInvariant() -ne $manifest.sha256) { throw 'Le contrôle SHA-256 a échoué.' }
        Move-Item -LiteralPath $download -Destination $destination -Force
        if ($stageUpdate) {
            $json = @{executable=$destination;sha=$manifest.sourceSha} | ConvertTo-Json
            $pendingFile = Join-Path $panelData 'pending-update.json'
            $temporary = $pendingFile + '.' + [guid]::NewGuid().ToString('N') + '.tmp'
            [IO.File]::WriteAllText($temporary,$json,(New-Object Text.UTF8Encoding($false)))
            Move-Item -LiteralPath $temporary -Destination $pendingFile -Force
        }
    }
    # A second launch opens the existing server instead of starting another instance.
    $panelBind = if ($env:ARK_PANEL_BIND) { $env:ARK_PANEL_BIND } else { '127.0.0.1' }
    $panelPort = if ($env:ARK_PANEL_PORT) { $env:ARK_PANEL_PORT } else { '3080' }
    $alreadyRunning = $false
    try { $health = Invoke-RestMethod -Uri "http://${panelBind}:${panelPort}/health" -TimeoutSec 2; $alreadyRunning = $health.ok -eq $true } catch { }
    if ($alreadyRunning) {
        $accessKey = (Get-Content -LiteralPath (Join-Path $panelData 'access-key.txt') -Raw).Trim()
        if ($env:ARK_PANEL_NO_BROWSER -ne '1') { Start-Process "http://${panelBind}:${panelPort}/#$accessKey" }
    } else {
        Start-Process -FilePath $panelExecutable -WorkingDirectory $PSScriptRoot -WindowStyle Hidden -RedirectStandardOutput (Join-Path $panelData 'launcher.log') -RedirectStandardError (Join-Path $panelData 'launcher-error.log')
    }
} catch {
    $_.Exception.Message | Out-File -LiteralPath (Join-Path $panelData 'bootstrap-error.log') -Encoding utf8
    Add-Type -AssemblyName System.Windows.Forms
    [System.Windows.Forms.MessageBox]::Show($_.Exception.Message, 'ARK Server Panel') | Out-Null
}
