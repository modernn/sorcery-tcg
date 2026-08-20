#requires -Version 7.0

[CmdletBinding()]
param(
    [string]$BackupRoot,
    [switch]$AcknowledgePrivateUseRisk
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ProductionSourceDescriptors {
    @(
        [pscustomobject]@{
            relativePath = 'rulebook/rulebook-current.pdf'
            provenanceUrl = 'https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update'
            requestUrl = 'https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update'
            mediaType = 'application/pdf'
            sourceMarker = 'Sorcery: Contested Realm December 2025 Rulebook Update'
            effectiveDatePolicy = 'fixed'
            effectiveDate = '2025-12-19'
            maxBytes = 268435456
            allowedHosts = @('sorcerytcg.com', 'www.sorcerytcg.com')
            allowedRedirectHosts = @('drive.google.com', 'drive.usercontent.google.com')
            rulebookLocatorHosts = @('drive.google.com')
            rulebookDownloadBaseUrl = 'https://drive.google.com/uc'
        }
        [pscustomobject]@{
            relativePath = 'formats/constructed-current.html'
            provenanceUrl = 'https://sorcerytcg.com/constructed'
            requestUrl = 'https://sorcerytcg.com/constructed'
            mediaType = 'text/html'
            sourceMarker = 'Constructed Format'
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            maxBytes = 268435456
            allowedHosts = @('sorcerytcg.com', 'www.sorcerytcg.com')
            allowedRedirectHosts = @('sorcerytcg.com', 'www.sorcerytcg.com')
        }
        [pscustomobject]@{
            relativePath = 'codex/codex-current.html'
            provenanceUrl = 'https://curiosa.io/codex'
            requestUrl = 'https://curiosa.io/codex'
            mediaType = 'text/html'
            sourceMarker = 'Welcome to the Codex'
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            maxBytes = 268435456
            allowedHosts = @('curiosa.io', 'www.curiosa.io')
            allowedRedirectHosts = @('curiosa.io', 'www.curiosa.io')
        }
        [pscustomobject]@{
            relativePath = 'codex/faqs-current.html'
            provenanceUrl = 'https://curiosa.io/faqs'
            requestUrl = 'https://curiosa.io/faqs'
            mediaType = 'text/html'
            sourceMarker = 'FAQs'
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            maxBytes = 268435456
            allowedHosts = @('curiosa.io', 'www.curiosa.io')
            allowedRedirectHosts = @('curiosa.io', 'www.curiosa.io')
        }
        [pscustomobject]@{
            relativePath = 'codex/changelog-current.html'
            provenanceUrl = 'https://curiosa.io/codex/changelog'
            requestUrl = 'https://curiosa.io/codex/changelog'
            mediaType = 'text/html'
            sourceMarker = 'Codex Changelog'
            effectiveDatePolicy = 'changelog'
            effectiveDate = $null
            maxBytes = 268435456
            allowedHosts = @('curiosa.io', 'www.curiosa.io')
            allowedRedirectHosts = @('curiosa.io', 'www.curiosa.io')
        }
        [pscustomobject]@{
            relativePath = 'updates/card-updates-2025.html'
            provenanceUrl = 'https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025'
            requestUrl = 'https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025'
            mediaType = 'text/html'
            sourceMarker = 'Sorcery: Contested Realm Card Updates 2025'
            effectiveDatePolicy = 'fixed'
            effectiveDate = '2025-11-25'
            maxBytes = 268435456
            allowedHosts = @('sorcerytcg.com', 'www.sorcerytcg.com')
            allowedRedirectHosts = @('sorcerytcg.com', 'www.sorcerytcg.com')
        }
        [pscustomobject]@{
            relativePath = 'cards/cards.raw.json'
            provenanceUrl = 'https://api.sorcerytcg.com/api/cards'
            requestUrl = 'https://api.sorcerytcg.com/api/cards'
            mediaType = 'application/json'
            sourceMarker = $null
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            maxBytes = 10000000
            allowedHosts = @('api.sorcerytcg.com')
            allowedRedirectHosts = @('api.sorcerytcg.com')
        }
    )
}

function Test-LoopbackHost {
    param([Parameter(Mandatory)][string]$HostName)

    if ($HostName -eq 'localhost') { return $true }
    $address = $null
    return [Net.IPAddress]::TryParse($HostName, [ref]$address) -and [Net.IPAddress]::IsLoopback($address)
}

function Assert-RequestUri {
    param(
        [Parameter(Mandatory)][Uri]$Uri,
        [Parameter(Mandatory)][string[]]$AllowedHosts,
        [Parameter(Mandatory)][bool]$LoopbackOnly
    )

    if (-not $Uri.IsAbsoluteUri) { throw "Request URI must be absolute: $Uri" }
    if ($Uri.UserInfo) { throw "Request URI credentials are forbidden: $Uri" }
    if ($LoopbackOnly) {
        if (-not (Test-LoopbackHost $Uri.DnsSafeHost)) { throw "Loopback test request escaped loopback: $Uri" }
        if ($Uri.Scheme -notin @('http', 'https')) { throw "Loopback test request uses an invalid scheme: $Uri" }
    }
    elseif ($Uri.Scheme -ne 'https') {
        throw "Production request must use HTTPS: $Uri"
    }
    if ($Uri.DnsSafeHost -notin $AllowedHosts) { throw "Request host is not allowed: $Uri" }
}

function Get-StrictUtf8Text {
    param([Parameter(Mandatory)][string]$Path)

    $encoding = [Text.UTF8Encoding]::new($false, $true)
    try {
        return $encoding.GetString([IO.File]::ReadAllBytes($Path))
    }
    catch {
        throw "Content is not valid UTF-8: $Path"
    }
}

function Get-NormalizedVisibleText {
    param([Parameter(Mandatory)][string]$Html)

    $withoutTags = [Text.RegularExpressions.Regex]::Replace($Html, '(?is)<[^>]+>', ' ')
    $decoded = [Net.WebUtility]::HtmlDecode($withoutTags)
    return [Text.RegularExpressions.Regex]::Replace($decoded, '\s+', ' ').Trim()
}

function Test-ChallengePage {
    param([Parameter(Mandatory)][string]$Html)

    return $Html -match '(?is)(captcha|cf-chl-|challenge-platform|<title[^>]*>\s*(?:Access Denied|Request Blocked|Attention Required)|unusual traffic)'
}

function Invoke-BoundedHttpToFile {
    param(
        [Parameter(Mandatory)][Net.Http.HttpClient]$Client,
        [Parameter(Mandatory)][Uri]$RequestUri,
        [Parameter(Mandatory)][string[]]$AllowedHosts,
        [Parameter(Mandatory)][string]$DestinationPath,
        [Parameter(Mandatory)][long]$MaximumBytes,
        [Parameter(Mandatory)][int]$HeaderTimeoutSeconds,
        [Parameter(Mandatory)][int]$BodyTimeoutSeconds,
        [Parameter(Mandatory)][bool]$LoopbackOnly
    )

    $current = $RequestUri
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
    for ($hop = 0; $hop -le 5; $hop++) {
        Assert-RequestUri $current $AllowedHosts $LoopbackOnly
        if (-not $seen.Add($current.AbsoluteUri)) { throw "Redirect loop detected: $current" }

        $request = [Net.Http.HttpRequestMessage]::new([Net.Http.HttpMethod]::Get, $current)
        $headerCancellation = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds($HeaderTimeoutSeconds))
        $response = $null
        try {
            $response = $Client.SendAsync(
                $request,
                [Net.Http.HttpCompletionOption]::ResponseHeadersRead,
                $headerCancellation.Token
            ).GetAwaiter().GetResult()
        }
        catch [OperationCanceledException] {
            throw "Response header timeout: $current"
        }
        finally {
            $headerCancellation.Dispose()
            $request.Dispose()
        }

        $status = [int]$response.StatusCode
        if ($status -in @(301, 302, 303, 307, 308)) {
            try {
                if ($hop -eq 5) { throw "Redirect limit exceeded: $current" }
                $location = $response.Headers.Location
                if ($null -eq $location) { throw "Redirect response omitted Location: $current" }
                $next = if ($location.IsAbsoluteUri) { $location } else { [Uri]::new($current, $location) }
                Assert-RequestUri $next $AllowedHosts $LoopbackOnly
                $current = $next
                continue
            }
            finally {
                $response.Dispose()
            }
        }
        if ($status -in @(401, 403, 429)) {
            $response.Dispose()
            throw "Request stopped on HTTP $status without retry: $current"
        }
        if ($status -ne 200) {
            $response.Dispose()
            throw "Request failed on HTTP $status without retry: $current"
        }

        $declaredLength = $response.Content.Headers.ContentLength
        if ($null -ne $declaredLength -and $declaredLength -gt $MaximumBytes) {
            $response.Dispose()
            throw "Declared response size exceeds the byte limit: $current"
        }

        [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($DestinationPath)) | Out-Null
        $output = $null
        $input = $null
        $bodyCancellation = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds($BodyTimeoutSeconds))
        $written = 0L
        try {
            $output = [IO.FileStream]::new(
                $DestinationPath,
                [IO.FileMode]::CreateNew,
                [IO.FileAccess]::Write,
                [IO.FileShare]::None
            )
            $input = $response.Content.ReadAsStream($bodyCancellation.Token)
            $buffer = [byte[]]::new(65536)
            while (($count = $input.ReadAsync($buffer, 0, $buffer.Length, $bodyCancellation.Token).GetAwaiter().GetResult()) -gt 0) {
                $written += $count
                if ($written -gt $MaximumBytes) { throw "Streamed response size exceeds the byte limit: $current" }
                $output.Write($buffer, 0, $count)
            }
            if ($written -eq 0) { throw "Response body is empty: $current" }
            $output.Flush($true)
        }
        catch [OperationCanceledException] {
            throw "Response body timeout: $current"
        }
        finally {
            if ($null -ne $input) { $input.Dispose() }
            if ($null -ne $output) { $output.Dispose() }
            $bodyCancellation.Dispose()
            $response.Dispose()
        }

        $contentType = if ($null -eq $response.Content.Headers.ContentType) { '' } else { $response.Content.Headers.ContentType.MediaType }
        $disposition = $response.Content.Headers.ContentDisposition
        $fileName = if ($null -eq $disposition) { $null } else { $disposition.FileNameStar }
        if ($null -ne $disposition -and [string]::IsNullOrWhiteSpace($fileName)) { $fileName = $disposition.FileName }
        if ($null -ne $fileName) { $fileName = $fileName.Trim('"') }
        return [pscustomobject]@{
            byteLength = $written
            contentType = $contentType
            fileName = $fileName
            finalUri = $current.AbsoluteUri
        }
    }
    throw "Redirect limit exceeded: $RequestUri"
}

function Assert-HtmlSource {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$ContentType,
        [Parameter(Mandatory)][string]$Marker
    )

    if ($ContentType -ne 'text/html') { throw "Expected text/html content: $Path" }
    $html = Get-StrictUtf8Text $Path
    if ($html -notmatch '(?is)^\s*(?:<!doctype\s+html|<html\b)') { throw "HTML signature is missing: $Path" }
    if (-not $html.Contains($Marker, [StringComparison]::Ordinal)) { throw "Required source marker is missing: $Marker" }
    if (Test-ChallengePage $html) { throw "Challenge or block page detected: $Path" }
    return $html
}

function Assert-CardJsonSource {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$ContentType
    )

    if ($ContentType -ne 'application/json') { throw "Expected application/json content: $Path" }
    $text = Get-StrictUtf8Text $Path
    try { $cards = $text | ConvertFrom-Json -NoEnumerate }
    catch { throw "Card source contains malformed JSON: $Path" }
    if ($cards -isnot [array] -or $cards.Count -eq 0) { throw "Card JSON must have a nonempty array root: $Path" }
    foreach ($card in $cards) {
        if ($null -eq $card -or $card -isnot [psobject] -or
            $null -eq $card.PSObject.Properties['name'] -or
            $card.name -isnot [string] -or [string]::IsNullOrWhiteSpace($card.name)) {
            throw "Every card must be an object with a nonempty string name: $Path"
        }
    }
}

function Get-ChangelogDate {
    param(
        [Parameter(Mandatory)][string]$Html,
        [Parameter(Mandatory)][string]$Marker
    )

    $markerIndex = $Html.IndexOf($Marker, [StringComparison]::Ordinal)
    if ($markerIndex -lt 0) { throw "Changelog marker is missing: $Marker" }
    $visible = Get-NormalizedVisibleText $Html.Substring($markerIndex)
    $match = [Text.RegularExpressions.Regex]::Match(
        $visible,
        '\b(?<date>\d{4}-\d{2}-\d{2}|(?:January|February|March|April|May|June|July|August|September|October|November|December) \d{1,2}, \d{4})\b',
        [Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    if (-not $match.Success) { throw 'The first visible changelog date is missing or invalid' }
    $dateText = $match.Groups['date'].Value
    $format = if ($dateText -match '^\d{4}-') { 'yyyy-MM-dd' } else { 'MMMM d, yyyy' }
    try {
        $date = [DateTime]::ParseExact(
            $dateText,
            $format,
            [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::None
        )
    }
    catch { throw 'The first visible changelog date is invalid' }
    return $date.ToString('yyyy-MM-dd', [Globalization.CultureInfo]::InvariantCulture)
}

function Receive-Rulebook {
    param(
        [Parameter(Mandatory)][Net.Http.HttpClient]$Client,
        [Parameter(Mandatory)][psobject]$Descriptor,
        [Parameter(Mandatory)][string]$DestinationPath,
        [Parameter(Mandatory)][int]$HeaderTimeoutSeconds,
        [Parameter(Mandatory)][int]$BodyTimeoutSeconds,
        [Parameter(Mandatory)][bool]$LoopbackOnly
    )

    $releasePath = "$DestinationPath.release-page"
    $release = Invoke-BoundedHttpToFile $Client ([Uri]$Descriptor.requestUrl) @($Descriptor.allowedHosts) $releasePath ([long]$Descriptor.maxBytes) $HeaderTimeoutSeconds $BodyTimeoutSeconds $LoopbackOnly
    try {
        $html = Assert-HtmlSource $releasePath $release.contentType ([string]$Descriptor.sourceMarker)
        $visible = Get-NormalizedVisibleText $html
        if ($visible -notmatch '\b2025-12-19\b') { throw 'Rulebook release date 2025-12-19 is missing' }
        $anchorPattern = '(?is)<a\b[^>]*\bhref\s*=\s*(?:"(?<double>[^"]*)"|''(?<single>[^'']*)'')[^>]*>(?<text>.*?)</a>'
        $matches = @([Text.RegularExpressions.Regex]::Matches($html, $anchorPattern) | Where-Object {
            (Get-NormalizedVisibleText $_.Groups['text'].Value) -eq 'Sorcery: Contested Realm Rulebook (December 2025)'
        })
        if ($matches.Count -ne 1) { throw 'Expected exactly one standard December 2025 rulebook anchor' }
        $href = if ($matches[0].Groups['double'].Success) { $matches[0].Groups['double'].Value } else { $matches[0].Groups['single'].Value }
        $locator = [Uri]::new([Uri]$Descriptor.requestUrl, [Net.WebUtility]::HtmlDecode($href))
        Assert-RequestUri $locator @($Descriptor.rulebookLocatorHosts) $LoopbackOnly
        $locatorMatch = [Text.RegularExpressions.Regex]::Match($locator.AbsolutePath, '^/file/d/(?<id>[^/]+)/view/?$')
        if (-not $locatorMatch.Success) { throw 'Rulebook locator is not a Drive file viewer path' }
        $id = [Uri]::EscapeDataString($locatorMatch.Groups['id'].Value)
        $downloadBuilder = [UriBuilder]::new([string]$Descriptor.rulebookDownloadBaseUrl)
        $downloadBuilder.Query = "export=download&id=$id"
        $download = Invoke-BoundedHttpToFile $Client $downloadBuilder.Uri @($Descriptor.allowedRedirectHosts) $DestinationPath ([long]$Descriptor.maxBytes) $HeaderTimeoutSeconds $BodyTimeoutSeconds $LoopbackOnly
        if ($download.contentType -notin @('application/pdf', 'application/octet-stream')) { throw 'Rulebook response has the wrong media type' }
        if ($download.fileName -ne 'SorceryRulebook.pdf') { throw 'Rulebook response filename must be SorceryRulebook.pdf' }
        $bytes = [IO.File]::ReadAllBytes($DestinationPath)
        if ($bytes.Length -lt 10 -or [Text.Encoding]::ASCII.GetString($bytes, 0, 5) -ne '%PDF-' -or
            [Text.Encoding]::ASCII.GetString($bytes, $bytes.Length - 5, 5) -ne '%%EOF') {
            throw 'Rulebook PDF signature or EOF marker is invalid'
        }
        return [pscustomobject]@{
            transfer = $download
            locator = $locator.AbsoluteUri
            observedFilename = $download.fileName
        }
    }
    finally {
        if ([IO.File]::Exists($releasePath)) { [IO.File]::Delete($releasePath) }
    }
}

function New-SourceEntry {
    param(
        [Parameter(Mandatory)][psobject]$Descriptor,
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][long]$ByteLength,
        [AllowNull()][object]$EffectiveDate
    )

    [pscustomobject]@{
        relativePath = [string]$Descriptor.relativePath
        url = [string]$Descriptor.provenanceUrl
        retrievedAt = [DateTime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", [Globalization.CultureInfo]::InvariantCulture)
        effectiveDate = $EffectiveDate
        mediaType = [string]$Descriptor.mediaType
        byteLength = $ByteLength
        byteHash = 'sha256:' + (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

function Invoke-PrivateAuthorityTransport {
    param(
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][object[]]$Descriptors,
        [Parameter(Mandatory)][int]$HeaderTimeoutSeconds,
        [Parameter(Mandatory)][int]$BodyTimeoutSeconds,
        [Parameter(Mandatory)][long]$MaxTotalBytes,
        [Parameter(Mandatory)][bool]$LoopbackOnly
    )

    if ([IO.Directory]::Exists($PrimaryRoot) -or [IO.File]::Exists($PrimaryRoot)) { throw "Primary destination already exists: $PrimaryRoot" }
    $handler = [Net.Http.HttpClientHandler]::new()
    $handler.AllowAutoRedirect = $false
    $client = [Net.Http.HttpClient]::new($handler)
    $client.Timeout = [Threading.Timeout]::InfiniteTimeSpan
    $entries = [Collections.Generic.List[object]]::new()
    $totalBytes = 0L
    $rulebookEvidence = $null
    [IO.Directory]::CreateDirectory($PrimaryRoot) | Out-Null
    try {
        foreach ($descriptor in $Descriptors) {
            $destination = [IO.Path]::GetFullPath((Join-Path $PrimaryRoot ([string]$descriptor.relativePath)))
            if (-not $destination.StartsWith([IO.Path]::GetFullPath($PrimaryRoot) + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
                throw "Source destination escapes primary root: $($descriptor.relativePath)"
            }
            if ([string]$descriptor.relativePath -eq 'rulebook/rulebook-current.pdf') {
                $rulebook = Receive-Rulebook $client $descriptor $destination $HeaderTimeoutSeconds $BodyTimeoutSeconds $LoopbackOnly
                $transfer = $rulebook.transfer
                $rulebookEvidence = [pscustomobject]@{
                    sourceUrl = [string]$descriptor.provenanceUrl
                    relativePath = [string]$descriptor.relativePath
                    observedFilename = $rulebook.observedFilename
                    privateLocatorEvidence = $rulebook.locator
                    privateLocatorIsNormative = $false
                }
            }
            else {
                $transfer = Invoke-BoundedHttpToFile $client ([Uri]$descriptor.requestUrl) @($descriptor.allowedRedirectHosts) $destination ([long]$descriptor.maxBytes) $HeaderTimeoutSeconds $BodyTimeoutSeconds $LoopbackOnly
                if ([string]$descriptor.mediaType -eq 'text/html') {
                    $html = Assert-HtmlSource $destination $transfer.contentType ([string]$descriptor.sourceMarker)
                }
                elseif ([string]$descriptor.mediaType -eq 'application/json') {
                    Assert-CardJsonSource $destination $transfer.contentType
                }
                else { throw "Unsupported source media type: $($descriptor.mediaType)" }
            }
            $totalBytes += [long]$transfer.byteLength
            if ($totalBytes -gt $MaxTotalBytes) { throw 'Source set exceeds the aggregate byte limit' }
            $effectiveDate = switch ([string]$descriptor.effectiveDatePolicy) {
                'fixed' { [string]$descriptor.effectiveDate }
                'none' { $null }
                'changelog' { Get-ChangelogDate $html ([string]$descriptor.sourceMarker) }
                default { throw "Unknown effective-date policy: $($descriptor.effectiveDatePolicy)" }
            }
            $entries.Add((New-SourceEntry $descriptor $destination ([long]$transfer.byteLength) $effectiveDate))
        }
        return [pscustomobject]@{
            entries = @($entries)
            rulebookAcquisitionEvidence = $rulebookEvidence
        }
    }
    catch {
        if ([IO.Directory]::Exists($PrimaryRoot)) { [IO.Directory]::Delete($PrimaryRoot, $true) }
        throw
    }
    finally {
        $client.Dispose()
        $handler.Dispose()
    }
}

function Assert-LoopbackDescriptors {
    param([Parameter(Mandatory)][object[]]$Descriptors)

    if ($Descriptors.Count -ne 7) { throw 'Loopback test requires exactly seven descriptors' }
    $expectedPaths = @(
        'rulebook/rulebook-current.pdf',
        'formats/constructed-current.html',
        'codex/codex-current.html',
        'codex/faqs-current.html',
        'codex/changelog-current.html',
        'updates/card-updates-2025.html',
        'cards/cards.raw.json'
    )
    $actualPaths = @($Descriptors | ForEach-Object { [string]$_.relativePath })
    if (@($actualPaths | Sort-Object -Unique).Count -ne 7 -or
        @(Compare-Object ($expectedPaths | Sort-Object) ($actualPaths | Sort-Object)).Count -ne 0) {
        throw 'Loopback test descriptors must use exactly the seven locked source paths'
    }
    foreach ($descriptor in $Descriptors) {
        $hostSets = @(@($descriptor.allowedHosts), @($descriptor.allowedRedirectHosts))
        if ($null -ne $descriptor.PSObject.Properties['rulebookLocatorHosts']) { $hostSets += ,@($descriptor.rulebookLocatorHosts) }
        foreach ($hostSet in $hostSets) {
            foreach ($hostName in $hostSet) {
                if (-not (Test-LoopbackHost ([string]$hostName))) { throw "Loopback test host is not loopback: $hostName" }
            }
        }
        Assert-RequestUri ([Uri]$descriptor.requestUrl) @($descriptor.allowedHosts) $true
        if ($null -ne $descriptor.PSObject.Properties['rulebookDownloadBaseUrl']) {
            Assert-RequestUri ([Uri]$descriptor.rulebookDownloadBaseUrl) @($descriptor.allowedRedirectHosts) $true
        }
    }
}

function Get-NormalizedPath {
    param([Parameter(Mandatory)][string]$Path)

    $fullPath = [IO.Path]::GetFullPath($Path).TrimEnd([IO.Path]::DirectorySeparatorChar, [IO.Path]::AltDirectorySeparatorChar)
    if ($IsWindows) { return $fullPath.ToLowerInvariant() }
    return $fullPath
}

function Test-PathWithin {
    param(
        [Parameter(Mandatory)][string]$Parent,
        [Parameter(Mandatory)][string]$Candidate
    )

    $normalizedParent = Get-NormalizedPath $Parent
    $normalizedCandidate = Get-NormalizedPath $Candidate
    return $normalizedCandidate -eq $normalizedParent -or
        $normalizedCandidate.StartsWith($normalizedParent + [IO.Path]::DirectorySeparatorChar, [StringComparison]::Ordinal)
}

function Assert-NoReparseAncestors {
    param([Parameter(Mandatory)][string]$Path)

    $current = [IO.Path]::GetFullPath($Path)
    while ($null -ne $current) {
        if ([IO.Directory]::Exists($current) -or [IO.File]::Exists($current)) {
            $attributes = [IO.File]::GetAttributes($current)
            if (($attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Destination path may not traverse a symlink or junction: $Path"
            }
        }
        $parent = [IO.Directory]::GetParent($current)
        $current = if ($null -eq $parent) { $null } else { $parent.FullName }
    }
}

function Resolve-CollectionPaths {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][string]$BackupRoot,
        [Parameter(Mandatory)][string]$LockPath
    )

    if (-not [IO.Path]::IsPathFullyQualified($RepositoryRoot) -or
        -not [IO.Path]::IsPathFullyQualified($PrimaryRoot) -or
        -not [IO.Path]::IsPathFullyQualified($BackupRoot) -or
        -not [IO.Path]::IsPathFullyQualified($LockPath)) {
        throw 'Repository, primary, backup, and lock paths must be absolute'
    }
    $repository = [IO.Path]::GetFullPath($RepositoryRoot)
    $primary = [IO.Path]::GetFullPath($PrimaryRoot)
    $backup = [IO.Path]::GetFullPath($BackupRoot)
    $lock = [IO.Path]::GetFullPath($LockPath)
    if (-not [IO.Directory]::Exists($repository)) { throw "Repository root does not exist: $repository" }
    foreach ($path in @($repository, $primary, $backup, $lock)) { Assert-NoReparseAncestors $path }
    if (-not (Test-PathWithin $repository $primary)) { throw 'Primary destination must be inside the repository' }
    if (-not (Test-PathWithin $repository $lock)) { throw 'Lock destination must be inside the repository' }
    if ((Test-PathWithin $repository $backup) -or (Test-PathWithin $backup $repository)) {
        throw 'Backup destination must be outside and may not contain the repository'
    }
    if ((Test-PathWithin $primary $backup) -or (Test-PathWithin $backup $primary)) {
        throw 'Primary and backup destinations may not overlap or be equal'
    }
    if (Test-PathWithin $primary $lock) { throw 'Lock destination may not be inside the primary tree' }
    foreach ($destination in @($primary, $backup, $lock)) {
        if (Test-Path -LiteralPath $destination) { throw "Destination already exists: $destination" }
    }
    return [pscustomobject]@{
        repositoryRoot = $repository
        primaryRoot = $primary
        backupRoot = $backup
        lockPath = $lock
    }
}

function Write-NewUtf8Json {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][object]$Value
    )

    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($Path)) | Out-Null
    $stream = [IO.FileStream]::new($Path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    $writer = [IO.StreamWriter]::new($stream, [Text.UTF8Encoding]::new($false, $true))
    try {
        $writer.Write(($Value | ConvertTo-Json -Depth 32))
        $writer.Flush()
        $stream.Flush($true)
    }
    finally {
        $writer.Dispose()
        $stream.Dispose()
    }
}

function Copy-SourceTree {
    param(
        [Parameter(Mandatory)][string]$SourceRoot,
        [Parameter(Mandatory)][string]$DestinationRoot,
        [Parameter(Mandatory)][object[]]$Descriptors
    )

    [IO.Directory]::CreateDirectory($DestinationRoot) | Out-Null
    foreach ($descriptor in $Descriptors) {
        $relativePath = [string]$descriptor.relativePath
        $source = Join-Path $SourceRoot $relativePath
        $destination = Join-Path $DestinationRoot $relativePath
        [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($destination)) | Out-Null
        [IO.File]::Copy($source, $destination, $false)
    }
}

function Invoke-PrivateSourceVerifier {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][string]$BackupRoot,
        [Parameter(Mandatory)][object[]]$Entries,
        [Parameter(Mandatory)][string]$DraftPath
    )

    Write-NewUtf8Json $DraftPath ([pscustomobject]@{
        repositoryRoot = $RepositoryRoot
        primaryRoot = $PrimaryRoot
        backupRoot = $BackupRoot
        entries = $Entries
    })
    $bridgeSource = @'
import { readFile } from 'node:fs/promises';
import { verifyPrivateSourceSet } from './src/authority/private-source-set.ts';
const input = JSON.parse(await readFile(process.argv[1], 'utf8'));
const result = await verifyPrivateSourceSet(input);
process.stdout.write(JSON.stringify(result));
'@
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = 'node'
    $startInfo.WorkingDirectory = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.ArgumentList.Add('--input-type=module')
    $startInfo.ArgumentList.Add('--eval')
    $startInfo.ArgumentList.Add($bridgeSource)
    $startInfo.ArgumentList.Add($DraftPath)
    try {
        $process = [Diagnostics.Process]::Start($startInfo)
        $standardOutput = $process.StandardOutput.ReadToEnd()
        $standardError = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Private source verifier failed: $standardError" }
        try { return $standardOutput | ConvertFrom-Json -Depth 32 -DateKind String }
        catch { throw "Private source verifier returned invalid JSON: $standardOutput" }
    }
    finally {
        if (Test-Path -LiteralPath $DraftPath) { [IO.File]::Delete($DraftPath) }
    }
}

function Assert-SameVerification {
    param(
        [Parameter(Mandatory)][object]$Expected,
        [Parameter(Mandatory)][object]$Actual
    )

    $expectedEntries = $Expected.entries | ConvertTo-Json -Depth 16 -Compress
    $actualEntries = $Actual.entries | ConvertTo-Json -Depth 16 -Compress
    if ($Expected.sourceSetRootHash -ne $Actual.sourceSetRootHash -or $expectedEntries -ne $actualEntries) {
        throw "Private source verification result changed between publication gates (expected $($Expected.sourceSetRootHash), actual $($Actual.sourceSetRootHash))"
    }
}

function Invoke-TestFault {
    param(
        [AllowNull()][string]$FaultPoint,
        [Parameter(Mandatory)][string]$Expected
    )

    if ($FaultPoint -eq $Expected) { throw "Controlled loopback test fault: $Expected" }
}

function Invoke-PrivateAuthorityCollectionCore {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][string]$BackupRoot,
        [Parameter(Mandatory)][string]$LockPath,
        [Parameter(Mandatory)][object[]]$Descriptors,
        [Parameter(Mandatory)][int]$HeaderTimeoutSeconds,
        [Parameter(Mandatory)][int]$BodyTimeoutSeconds,
        [Parameter(Mandatory)][long]$MaxTotalBytes,
        [Parameter(Mandatory)][bool]$LoopbackOnly,
        [AllowNull()][string]$FaultPoint
    )

    $paths = Resolve-CollectionPaths $RepositoryRoot $PrimaryRoot $BackupRoot $LockPath
    $runId = [Guid]::NewGuid().ToString('N')
    $primaryStage = "$($paths.primaryRoot).collecting-$runId"
    $backupStage = "$($paths.backupRoot).collecting-$runId"
    $lockCandidate = "$($paths.lockPath).collecting-$runId"
    $draftPath = "$($paths.lockPath).verifier-$runId.json"
    $primaryPublished = $false
    $backupPublished = $false
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($paths.primaryRoot)) | Out-Null
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($paths.backupRoot)) | Out-Null
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($paths.lockPath)) | Out-Null
    try {
        $transport = Invoke-PrivateAuthorityTransport $primaryStage $Descriptors $HeaderTimeoutSeconds $BodyTimeoutSeconds $MaxTotalBytes $LoopbackOnly
        Copy-SourceTree $primaryStage $backupStage $Descriptors
        $staged = Invoke-PrivateSourceVerifier $paths.repositoryRoot $primaryStage $backupStage @($transport.entries) $draftPath
        Invoke-TestFault $FaultPoint 'after-staged-verification'

        [IO.Directory]::Move($backupStage, $paths.backupRoot)
        $backupPublished = $true
        Invoke-TestFault $FaultPoint 'after-backup-move'
        [IO.Directory]::Move($primaryStage, $paths.primaryRoot)
        $primaryPublished = $true
        Invoke-TestFault $FaultPoint 'after-primary-move'
        Invoke-TestFault $FaultPoint 'during-final-verifier'
        $final = Invoke-PrivateSourceVerifier $paths.repositoryRoot $paths.primaryRoot $paths.backupRoot @($transport.entries) $draftPath
        Assert-SameVerification $staged $final

        $rulebookEntry = @($final.entries | Where-Object { $_.relativePath -eq 'rulebook/rulebook-current.pdf' })[0]
        $rulebookEvidence = [ordered]@{
            sourceUrl = $transport.rulebookAcquisitionEvidence.sourceUrl
            relativePath = $rulebookEntry.relativePath
            byteHash = $rulebookEntry.byteHash
            observedFilename = $transport.rulebookAcquisitionEvidence.observedFilename
            retrievedAt = $rulebookEntry.retrievedAt
            privateLocatorEvidence = $transport.rulebookAcquisitionEvidence.privateLocatorEvidence
            privateLocatorIsNormative = $false
        }
        $lock = [ordered]@{
            schemaVersion = 1
            acquisitionMethod = 'user-run-one-shot-powershell'
            primaryRoot = $paths.primaryRoot
            backupRoot = $paths.backupRoot
            entries = @($final.entries)
            sourceSetRootHash = $final.sourceSetRootHash
            operatingAcknowledgment = [ordered]@{
                scope = 'private-local-noncommercial'
                noRedistributionReleaseHostingUploadOrArtwork = $true
                apiTermsRobotsConflictAndPrivateUseRiskAccepted = $true
                establishesLegalPermission = $false
                stopOnBlockedStatusCaptchaOrPublisherObjection = $true
                retryOrEvasion = $false
            }
            rulebookAcquisitionEvidence = $rulebookEvidence
        }
        Write-NewUtf8Json $lockCandidate $lock
        $candidate = Get-Content -Raw -LiteralPath $lockCandidate | ConvertFrom-Json -Depth 32 -DateKind String
        if ($candidate.acquisitionMethod -ne 'user-run-one-shot-powershell') { throw 'Lock candidate has an invalid acquisition method' }
        $candidateVerification = Invoke-PrivateSourceVerifier $paths.repositoryRoot $candidate.primaryRoot $candidate.backupRoot @($candidate.entries) $draftPath
        Assert-SameVerification $final $candidateVerification
        if ($candidate.sourceSetRootHash -ne $candidateVerification.sourceSetRootHash) { throw 'Lock candidate root hash does not match final verification' }
        Invoke-TestFault $FaultPoint 'before-lock-move'
        [IO.File]::Move($lockCandidate, $paths.lockPath, $false)
        return $lock
    }
    catch {
        $originalError = $_
        $quarantineErrors = [Collections.Generic.List[string]]::new()
        foreach ($stage in @($primaryStage, $backupStage)) {
            if ([IO.Directory]::Exists($stage)) {
                try { [IO.Directory]::Delete($stage, $true) }
                catch { $quarantineErrors.Add("Could not remove owned staging path $stage") }
            }
        }
        if ($primaryPublished -and [IO.Directory]::Exists($paths.primaryRoot)) {
            try { [IO.Directory]::Move($paths.primaryRoot, "$($paths.primaryRoot).failed-$runId") }
            catch { $quarantineErrors.Add("Could not quarantine $($paths.primaryRoot)") }
        }
        if ($backupPublished -and [IO.Directory]::Exists($paths.backupRoot)) {
            try { [IO.Directory]::Move($paths.backupRoot, "$($paths.backupRoot).failed-$runId") }
            catch { $quarantineErrors.Add("Could not quarantine $($paths.backupRoot)") }
        }
        if ([IO.File]::Exists($lockCandidate)) {
            try { [IO.File]::Move($lockCandidate, "$($paths.lockPath).failed-$runId", $false) }
            catch { $quarantineErrors.Add("Could not quarantine $lockCandidate") }
        }
        if ([IO.File]::Exists($draftPath)) {
            try { [IO.File]::Delete($draftPath) }
            catch { $quarantineErrors.Add("Could not remove owned verifier draft $draftPath") }
        }
        if ($quarantineErrors.Count -gt 0) {
            throw "$($originalError.Exception.Message) Quarantine failures: $($quarantineErrors -join '; ')"
        }
        throw $originalError
    }
}

function Invoke-PrivateAuthorityCollectionForLoopbackTest {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][string]$BackupRoot,
        [Parameter(Mandatory)][string]$LockPath,
        [Parameter(Mandatory)][object[]]$Descriptors,
        [Parameter(Mandatory)][int]$HeaderTimeoutSeconds,
        [Parameter(Mandatory)][int]$BodyTimeoutSeconds,
        [Parameter(Mandatory)][long]$MaxTotalBytes,
        [AllowNull()][string]$FaultPoint
    )

    Assert-LoopbackDescriptors $Descriptors
    return Invoke-PrivateAuthorityCollectionCore $RepositoryRoot $PrimaryRoot $BackupRoot $LockPath $Descriptors $HeaderTimeoutSeconds $BodyTimeoutSeconds $MaxTotalBytes $true $FaultPoint
}

function Invoke-PrivateAuthorityCollection {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][string]$BackupRoot,
        [Parameter(Mandatory)][string]$LockPath,
        [Parameter(Mandatory)][object[]]$Descriptors
    )

    return Invoke-PrivateAuthorityCollectionCore $RepositoryRoot $PrimaryRoot $BackupRoot $LockPath $Descriptors 30 900 805306368 $false $null
}

if ($MyInvocation.InvocationName -ne '.') {
    if ([string]::IsNullOrWhiteSpace($BackupRoot)) { throw 'BackupRoot is required and must be an absolute path outside the repository.' }
    if (-not $AcknowledgePrivateUseRisk) { throw 'AcknowledgePrivateUseRisk is required before collection.' }
    $repositoryRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
    $primaryRoot = Join-Path $repositoryRoot '.local/authority/inputs/official-2026-08-20/primary'
    $lockPath = Join-Path $repositoryRoot '.local/authority/locks/official-2026-08-20/source-set-lock.json'
    Invoke-PrivateAuthorityCollection $repositoryRoot $primaryRoot $BackupRoot $lockPath (Get-ProductionSourceDescriptors)
}
