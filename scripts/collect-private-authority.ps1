#requires -Version 7.0

[CmdletBinding()]
param(
    [switch]$ImportManualInbox,
    [switch]$AcknowledgePrivateUseRisk,
    [switch]$VerifyExistingRoots,
    [AllowNull()][string]$LockPath
)


$collectorModule = New-Module -Name 'Sorcery.PrivateAuthorityCollector' -ArgumentList $PSScriptRoot -ScriptBlock {
param([Parameter(Mandatory)][string]$CollectorScriptRoot)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$script:CollectorScriptRoot = [IO.Path]::GetFullPath($CollectorScriptRoot)

function Get-ProductionSourceDescriptors {
    @(
        [pscustomobject]@{
            relativePath = 'rulebook/rulebook-current.pdf'
            provenanceUrl = 'https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update'
            mediaType = 'application/pdf'
            sourceMarker = 'Sorcery: Contested Realm December 2025 Rulebook Update'
            effectiveDatePolicy = 'fixed'
            effectiveDate = '2025-12-19'
            minBytes = 32768
            maxBytes = 268435456
        }
        [pscustomobject]@{
            relativePath = 'formats/constructed-current.html'
            provenanceUrl = 'https://sorcerytcg.com/constructed'
            mediaType = 'text/html'
            sourceMarker = 'Constructed Format'
            visibleBodyMarkers = @('Constructed Format', 'Deckbuilding')
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            minBytes = 512
            maxBytes = 268435456
        }
        [pscustomobject]@{
            relativePath = 'codex/codex-current.html'
            provenanceUrl = 'https://curiosa.io/codex'
            mediaType = 'text/html'
            sourceMarker = 'Welcome to the Codex'
            visibleBodyMarkers = @('Welcome to the Codex', 'Golden Rule')
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            minBytes = 512
            maxBytes = 268435456
        }
        [pscustomobject]@{
            relativePath = 'codex/faqs-current.html'
            provenanceUrl = 'https://curiosa.io/faqs'
            mediaType = 'text/html'
            sourceMarker = 'FAQs'
            visibleBodyMarkers = @('FAQs')
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            minBytes = 512
            maxBytes = 268435456
        }
        [pscustomobject]@{
            relativePath = 'codex/changelog-current.html'
            provenanceUrl = 'https://curiosa.io/codex/changelog'
            mediaType = 'text/html'
            sourceMarker = 'Codex Changelog'
            visibleBodyMarkers = @('Codex Changelog')
            effectiveDatePolicy = 'changelog'
            effectiveDate = $null
            minBytes = 512
            maxBytes = 268435456
        }
        [pscustomobject]@{
            relativePath = 'updates/card-updates-2025.html'
            provenanceUrl = 'https://sorcerytcg.com/news/sorcery-contested-realm-card-updates-2025'
            mediaType = 'text/html'
            sourceMarker = 'Sorcery: Contested Realm Card Updates 2025'
            visibleBodyMarkers = @('Sorcery: Contested Realm Card Updates 2025', 'Card Updates')
            effectiveDatePolicy = 'fixed'
            effectiveDate = '2025-11-25'
            minBytes = 512
            maxBytes = 268435456
        }
        [pscustomobject]@{
            relativePath = 'cards/cards.raw.json'
            provenanceUrl = 'https://api.sorcerytcg.com/api/cards'
            mediaType = 'application/json'
            sourceMarker = $null
            expectedCardCount = 1100
            effectiveDatePolicy = 'none'
            effectiveDate = $null
            minBytes = 65536
            maxBytes = 10000000
        }
    )
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

function Get-VisibleHtml {
    param([Parameter(Mandatory)][string]$Html)

    $withoutComments = [Text.RegularExpressions.Regex]::Replace($Html, '(?is)<!--.*?-->', ' ')
    return [Text.RegularExpressions.Regex]::Replace(
        $withoutComments,
        '(?is)<(?<hidden>head|script|style|template|noscript|title|nav|header|footer|aside)\b[^>]*>.*?</\k<hidden>\s*>',
        ' '
    )
}

function Get-NormalizedVisibleText {
    param([Parameter(Mandatory)][string]$Html)

    $withoutTags = [Text.RegularExpressions.Regex]::Replace((Get-VisibleHtml $Html), '(?is)<[^>]+>', ' ')
    $decoded = [Net.WebUtility]::HtmlDecode($withoutTags)
    return [Text.RegularExpressions.Regex]::Replace($decoded, '\s+', ' ').Trim()
}

function Test-ChallengePage {
    param([Parameter(Mandatory)][string]$Html)

    return $Html -match '(?is)(captcha|cf-chl-|challenge-platform|<title[^>]*>\s*(?:Access Denied|Request Blocked|Attention Required)|unusual traffic)'
}

function Assert-HtmlSource {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$ContentType,
        [Parameter(Mandatory)][string]$Marker,
        [Parameter(Mandatory)][AllowEmptyCollection()][string[]]$VisibleBodyMarkers
    )

    if ($ContentType -ne 'text/html') { throw "Expected text/html content: $Path" }
    $html = Get-StrictUtf8Text $Path
    if ($html -notmatch '(?is)^\s*(?:<!doctype\s+html|<html\b)') { throw "HTML signature is missing: $Path" }
    if ($html -notmatch '(?is)</html\s*>\s*$') { throw 'HTML closing structure is missing' }
    if (Test-ChallengePage $html) { throw "Challenge or block page detected: $Path" }
    if ($VisibleBodyMarkers.Count -eq 0) {
        if (-not $html.Contains($Marker, [StringComparison]::Ordinal)) { throw 'Required source marker is missing' }
        return $html
    }
    $visible = Get-NormalizedVisibleText $html
    if ($visible.Length -lt 128) { throw 'Visible source content is too short to be complete' }
    foreach ($requiredMarker in @($Marker) + @($VisibleBodyMarkers) | Select-Object -Unique) {
        if ([string]::IsNullOrWhiteSpace($requiredMarker) -or
            -not $visible.Contains($requiredMarker, [StringComparison]::Ordinal)) {
            throw 'Required visible source structure is missing'
        }
    }
    return $html
}

function Assert-PdfSource {
    param([Parameter(Mandatory)][string]$Path)

    $bytes = [IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 5 -or [Text.Encoding]::ASCII.GetString($bytes, 0, 5) -ne '%PDF-') {
        throw 'Rulebook PDF prefix is invalid'
    }
    $ending = if ($bytes.Length -ge 2 -and $bytes[$bytes.Length - 2] -eq 13 -and $bytes[$bytes.Length - 1] -eq 10) {
        "%%EOF`r`n"
    }
    elseif ($bytes[$bytes.Length - 1] -eq 10) { "%%EOF`n" }
    elseif ($bytes[$bytes.Length - 1] -eq 13) { "%%EOF`r" }
    else { '%%EOF' }
    if ($bytes.Length -lt $ending.Length -or
        [Text.Encoding]::ASCII.GetString($bytes, $bytes.Length - $ending.Length, $ending.Length) -cne $ending) {
        throw 'Rulebook PDF EOF marker or trailing bytes are invalid'
    }
    $ascii = [Text.Encoding]::ASCII.GetString($bytes)
    $startXref = [Text.RegularExpressions.Regex]::Match($ascii, '(?s)startxref\s+(?<offset>\d+)\s+%%EOF\s*$')
    $xrefOffset = [long]0
    if ($ascii -cnotmatch '(?s)/Type\s*/Page\b' -or
        -not $startXref.Success -or
        -not [long]::TryParse($startXref.Groups['offset'].Value, [ref]$xrefOffset) -or
        $xrefOffset -lt 0 -or
        $xrefOffset -ge $ascii.Length) {
        throw 'Rulebook PDF structure is incomplete'
    }
    $xref = $ascii.Substring([int]$xrefOffset)
    if ($xref -cnotmatch '(?s)^xref\b.*?\btrailer\b' -and
        $xref -cnotmatch '(?s)^\d+\s+\d+\s+obj\b.*?/Type\s*/XRef\b') {
        throw 'Rulebook PDF cross-reference structure is invalid'
    }
}

function Assert-PrivateAuthorityContentSet {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][object[]]$Descriptors
    )

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
    if ($Descriptors.Count -ne 7 -or
        @($actualPaths | Sort-Object -Unique).Count -ne 7 -or
        @(Compare-Object ($expectedPaths | Sort-Object) ($actualPaths | Sort-Object)).Count -ne 0) {
        throw 'Private content verification requires exactly the seven authority descriptors'
    }
    $resolvedRoot = [IO.Path]::GetFullPath($Root)
    if (-not [IO.Directory]::Exists($resolvedRoot)) { throw 'Private authority root does not exist' }
    $rootPrefix = $resolvedRoot + [IO.Path]::DirectorySeparatorChar
    $effectiveDates = @{}
    foreach ($descriptor in $Descriptors) {
        $relativePath = [string]$descriptor.relativePath
        $path = [IO.Path]::GetFullPath((Join-Path $resolvedRoot $relativePath))
        if (-not $path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase) -or
            -not [IO.File]::Exists($path)) {
            throw 'Private authority content path is missing or escapes its root'
        }
        $file = [IO.FileInfo]::new($path)
        if ($file.Length -lt [long]$descriptor.minBytes -or $file.Length -gt [long]$descriptor.maxBytes) {
            throw 'Private authority content size is invalid'
        }
        $html = $null
        switch ([string]$descriptor.mediaType) {
            'application/pdf' { Assert-PdfSource $path }
            'text/html' {
                $html = Assert-HtmlSource $path 'text/html' ([string]$descriptor.sourceMarker) @($descriptor.visibleBodyMarkers)
            }
            'application/json' {
                Assert-CardJsonSource $path 'application/json' ([int]$descriptor.expectedCardCount)
            }
            default { throw 'Unsupported private authority content media type' }
        }
        $effectiveDates[$relativePath] = switch ([string]$descriptor.effectiveDatePolicy) {
            'fixed' { [string]$descriptor.effectiveDate }
            'none' { $null }
            'changelog' { Get-ChangelogDate $html ([string]$descriptor.sourceMarker) }
            default { throw 'Unknown private authority effective-date policy' }
        }
    }
    return $effectiveDates
}

function Invoke-BoundedProcess {
    param(
        [Parameter(Mandatory)][string]$FileName,
        [Parameter(Mandatory)][string[]]$ArgumentList,
        [Parameter(Mandatory)][string]$WorkingDirectory,
        [Parameter(Mandatory)][int]$TimeoutSeconds,
        [int]$MaximumOutputCharacters = 4096
    )

    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $FileName
    $start.WorkingDirectory = $WorkingDirectory
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in $ArgumentList) { $start.ArgumentList.Add($argument) }

    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
        if (-not $process.Start()) { throw 'Subprocess did not start' }
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit([int][TimeSpan]::FromSeconds($TimeoutSeconds).TotalMilliseconds)) {
            try { $process.Kill($true) } catch { }
            $process.WaitForExit()
            $stdoutTask.GetAwaiter().GetResult() | Out-Null
            $stderrTask.GetAwaiter().GetResult() | Out-Null
            throw 'Subprocess timed out'
        }
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        return [pscustomobject]@{
            exitCode = $process.ExitCode
            stdout = if ($stdout.Length -le $MaximumOutputCharacters) { $stdout } else { $stdout.Substring(0, $MaximumOutputCharacters) }
            stderr = if ($stderr.Length -le $MaximumOutputCharacters) { $stderr } else { $stderr.Substring(0, $MaximumOutputCharacters) }
        }
    }
    finally {
        $process.Dispose()
    }
}

function Assert-CardJsonSource {
    param(
        [Parameter(Mandatory)][string]$Path,
        [Parameter(Mandatory)][string]$ContentType,
        [Parameter(Mandatory)][ValidateRange(1, 2000)][int]$ExpectedCardCount
    )

    if ($ContentType -ne 'application/json') { throw 'Card source has the wrong media type' }
    $adapterPath = [IO.Path]::GetFullPath((Join-Path $script:CollectorScriptRoot '../src/authority/official-card-api-adapter.ts'))
    $adapterUrl = [Uri]::new($adapterPath).AbsoluteUri
    $bridge = @'
import { readFile } from 'node:fs/promises';
try {
  const { adaptOfficialCardApiSnapshot } = await import(process.argv[3]);
  const input = JSON.parse(await readFile(process.argv[1], 'utf8'));
  const snapshot = adaptOfficialCardApiSnapshot(input);
  if (snapshot.cards.length !== Number(process.argv[2])) process.exitCode = 1;
} catch {
  process.exitCode = 1;
}
'@
    try {
        $result = Invoke-BoundedProcess 'node' @(
            '--input-type=module',
            '--eval',
            $bridge,
            $Path,
            [string]$ExpectedCardCount,
            $adapterUrl
        ) ([IO.Path]::GetDirectoryName($adapterPath)) 30
    }
    catch { throw 'Card source validation did not complete' }
    if ($result.exitCode -ne 0) { throw 'Card source failed strict shape or cardinality validation' }
}

function Get-ChangelogDate {
    param(
        [Parameter(Mandatory)][string]$Html,
        [Parameter(Mandatory)][string]$Marker
    )

    $visibleHtml = Get-VisibleHtml $Html
    $markerIndex = $visibleHtml.IndexOf($Marker, [StringComparison]::Ordinal)
    if ($markerIndex -lt 0) { throw "Changelog marker is missing: $Marker" }
    $datePattern = '\b(?<date>\d{4}-\d{2}-\d{2}|(?:January|February|March|April|May|June|July|August|September|October|November|December) \d{1,2}, \d{4}|\d{1,2} (?:January|February|March|April|May|June|July|August|September|October|November|December) \d{4})\b'
    $visibleEntry = $null
    foreach ($headingMatch in [Text.RegularExpressions.Regex]::Matches(
        $visibleHtml.Substring($markerIndex),
        '(?is)<h[1-6]\b[^>]*>(?<entry>.*?)</h[1-6]\s*>'
    )) {
        $entryHtml = $headingMatch.Groups['entry'].Value
        if ([string]::IsNullOrWhiteSpace($entryHtml)) { continue }
        $candidate = Get-NormalizedVisibleText $entryHtml
        if ([Text.RegularExpressions.Regex]::IsMatch(
            $candidate,
            "^(?:$datePattern)$",
            [Text.RegularExpressions.RegexOptions]::CultureInvariant
        )) {
            $visibleEntry = $candidate
            break
        }
    }
    if ($null -eq $visibleEntry) { throw 'The first visible changelog date heading is missing' }
    $matches = [Text.RegularExpressions.Regex]::Matches(
        $visibleEntry,
        $datePattern,
        [Text.RegularExpressions.RegexOptions]::CultureInvariant
    )
    $dates = [Collections.Generic.List[DateTime]]::new()
    foreach ($match in $matches) {
        $dateText = $match.Groups['date'].Value
        $format = if ($dateText -match '^\d{4}-') { 'yyyy-MM-dd' }
            elseif ($dateText -match '^\d') { 'd MMMM yyyy' }
            else { 'MMMM d, yyyy' }
        $parsed = [DateTime]::MinValue
        if ([DateTime]::TryParseExact(
            $dateText,
            $format,
            [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::None,
            [ref]$parsed
        )) { $dates.Add($parsed) }
    }
    if ($dates.Count -ne 1) { throw 'The first visible changelog entry must contain exactly one valid date' }
    return $dates[0].ToString('yyyy-MM-dd', [Globalization.CultureInfo]::InvariantCulture)
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
        [Parameter(Mandatory)][string]$DraftPath,
        [Parameter(Mandatory)][ValidateRange(1, 30)][int]$TimeoutSeconds,
        [switch]$ForceBridgeFailure,
        [switch]$ForceBridgeHang
    )

    Write-NewUtf8Json $DraftPath ([pscustomobject]@{
        repositoryRoot = $RepositoryRoot
        primaryRoot = $PrimaryRoot
        backupRoot = $BackupRoot
        entries = $Entries
    })
    $bridgeSource = if ($ForceBridgeHang) { @'
process.stderr.write('CANARY_VERIFIER_PIPE'.repeat(65536));
setInterval(() => {}, 1000);
'@ } else { @'
import { readFile } from 'node:fs/promises';
import { verifyPrivateSourceSet } from './src/authority/private-source-set.ts';
const input = JSON.parse(await readFile(process.argv[1], 'utf8'));
const result = await verifyPrivateSourceSet(input);
process.stdout.write(JSON.stringify(result));
'@
    }
    $verifierInputPath = if ($ForceBridgeFailure) { "$DraftPath.missing" } else { $DraftPath }
    try {
        try {
            $result = Invoke-BoundedProcess 'node' @(
                '--input-type=module',
                '--eval',
                $bridgeSource,
                $verifierInputPath
            ) ([IO.Path]::GetFullPath((Join-Path $script:CollectorScriptRoot '..'))) $TimeoutSeconds
        }
        catch {
            if ($_.Exception.Message -eq 'Subprocess timed out') { throw 'Private source verifier timed out' }
            throw 'Private source verifier could not run'
        }
        if ($result.exitCode -ne 0) { throw 'Private source verifier failed' }
        try { return $result.stdout | ConvertFrom-Json -Depth 32 -DateKind String }
        catch { throw 'Private source verifier returned invalid output' }
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

function Get-RepositoryRootFromAuthorityLockPath {
    param([Parameter(Mandatory)][string]$LockPath)

    $current = [IO.Directory]::GetParent($LockPath)
    while ($null -ne $current) {
        if ($current.Name -ceq 'locks' -and
            $null -ne $current.Parent -and $current.Parent.Name -ceq 'authority' -and
            $null -ne $current.Parent.Parent -and $current.Parent.Parent.Name -ceq '.local' -and
            $null -ne $current.Parent.Parent.Parent) {
            return $current.Parent.Parent.Parent.FullName
        }
        $current = $current.Parent
    }
    throw 'Lock path is outside the private authority lock hierarchy'
}

function Assert-HistoricalAgentAuthorizationRecord {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$AuthorizationReference
    )

    $recordPath = [IO.Path]::GetFullPath((Join-Path $RepositoryRoot ".local/authority/authorizations/$AuthorizationReference.consumed.json"))
    Assert-NoReparseAncestors $recordPath
    $file = [IO.FileInfo]::new($recordPath)
    if (-not $file.Exists -or $file.Length -lt 1 -or $file.Length -gt 4096) {
        throw 'Historical agent authorization record is missing or invalid'
    }
    try { $record = Get-Content -Raw -LiteralPath $recordPath | ConvertFrom-Json -Depth 8 -DateKind String }
    catch { throw 'Historical agent authorization record is invalid' }
    $fields = @('schemaVersion', 'revisionId', 'acquisitionMethod', 'authorizationReference', 'consumedAt')
    $consumedAt = [DateTime]::MinValue
    if (@(Compare-Object ($fields | Sort-Object) (@($record.PSObject.Properties.Name) | Sort-Object)).Count -ne 0 -or
        $record.schemaVersion -ne 1 -or
        $record.revisionId -cne 'official-2026-08-20' -or
        $record.acquisitionMethod -cne 'user-authorized-agent-run-one-shot-powershell' -or
        $record.authorizationReference -cne $AuthorizationReference -or
        -not [DateTime]::TryParseExact(
            [string]$record.consumedAt,
            "yyyy-MM-dd'T'HH:mm:ss.fff'Z'",
            [Globalization.CultureInfo]::InvariantCulture,
            [Globalization.DateTimeStyles]::AssumeUniversal,
            [ref]$consumedAt
        )) {
        throw 'Historical agent authorization record is invalid'
    }
}

function Read-ConsumedPrivateAuthorityLock {
    param([Parameter(Mandatory)][string]$LockPath)

    if ([string]::IsNullOrWhiteSpace($LockPath)) { throw 'Lock path is required' }
    $resolvedPath = [IO.Path]::GetFullPath($LockPath)
    Assert-NoReparseAncestors $resolvedPath
    if (-not [IO.File]::Exists($resolvedPath)) { throw 'Lock file does not exist' }
    try { $lock = Get-Content -Raw -LiteralPath $resolvedPath | ConvertFrom-Json -Depth 32 -DateKind String }
    catch { throw 'Lock file is not valid JSON' }
    $baseFields = @(
        'schemaVersion',
        'acquisitionMethod',
        'primaryRoot',
        'backupRoot',
        'entries',
        'sourceSetRootHash',
        'operatingAcknowledgment',
        'rulebookAcquisitionEvidence'
    )
    $actualFields = @($lock.PSObject.Properties.Name)
    $hasAuthorizationReference = $actualFields -ccontains 'authorizationReference'
    $legacyIdentity = $lock.acquisitionMethod -ceq 'user-run-one-shot-powershell' -and -not $hasAuthorizationReference
    $historicalAgentIdentity = $hasAuthorizationReference -and
        $lock.acquisitionMethod -ceq 'user-authorized-agent-run-one-shot-powershell' -and
        @('quick-260825-mhh', 'quick-260825-mhh-retry-1') -ccontains [string]$lock.authorizationReference
    $manualIdentity = $hasAuthorizationReference -and
        $lock.acquisitionMethod -ceq 'user-provided-manual-download' -and
        [string]$lock.authorizationReference -ceq 'phase-01-20260827-manual-provision-1'
    $requiredFields = if ($legacyIdentity) { $baseFields } else { @($baseFields) + 'authorizationReference' }
    if (@(Compare-Object ($requiredFields | Sort-Object) ($actualFields | Sort-Object)).Count -ne 0 -or
        $lock.schemaVersion -ne 1 -or
        (-not $legacyIdentity -and -not $historicalAgentIdentity -and -not $manualIdentity)) {
        throw 'Private lock fields or acquisition identity are invalid'
    }
    $repositoryRoot = Get-RepositoryRootFromAuthorityLockPath $resolvedPath
    $revisionId = if ($manualIdentity) { 'official-2026-08-27-v3' } else { 'official-2026-08-20' }
    $expectedLockPath = [IO.Path]::GetFullPath((Join-Path $repositoryRoot ".local/authority/locks/$revisionId/source-set-lock.json"))
    $expectedPrimaryRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot ".local/authority/inputs/$revisionId/primary"))
    if ((Get-NormalizedPath $resolvedPath) -cne (Get-NormalizedPath $expectedLockPath) -or
        (Get-NormalizedPath ([string]$lock.primaryRoot)) -cne (Get-NormalizedPath $expectedPrimaryRoot)) {
        throw 'Private lock identity does not match its revision paths'
    }
    if ($manualIdentity) {
        $expectedBackupRoot = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $repositoryRoot) 'sorcery-tcg-authority-backup-official-2026-08-27-v3'))
        if ((Get-NormalizedPath ([string]$lock.backupRoot)) -cne (Get-NormalizedPath $expectedBackupRoot)) {
            throw 'Manual lock backup path is invalid'
        }
    }
    if ($historicalAgentIdentity) {
        Assert-HistoricalAgentAuthorizationRecord $repositoryRoot ([string]$lock.authorizationReference)
    }
    if ([string]::IsNullOrWhiteSpace([string]$lock.primaryRoot) -or
        [string]::IsNullOrWhiteSpace([string]$lock.backupRoot) -or
        -not [IO.Path]::IsPathFullyQualified([string]$lock.primaryRoot) -or
        -not [IO.Path]::IsPathFullyQualified([string]$lock.backupRoot) -or
        @($lock.entries).Count -ne 7 -or
        [string]$lock.sourceSetRootHash -cnotmatch '^sha256:[0-9a-f]{64}$') {
        throw 'Consumed lock source-set fields are invalid'
    }
    $acknowledgment = $lock.operatingAcknowledgment
    if ($acknowledgment.scope -cne 'private-local-noncommercial' -or
        $acknowledgment.noRedistributionReleaseHostingUploadOrArtwork -ne $true -or
        $acknowledgment.apiTermsRobotsConflictAndPrivateUseRiskAccepted -ne $true -or
        $acknowledgment.establishesLegalPermission -ne $false -or
        $acknowledgment.stopOnBlockedStatusCaptchaOrPublisherObjection -ne $true -or
        $acknowledgment.retryOrEvasion -ne $false) {
        throw 'Consumed lock operating acknowledgment is invalid'
    }
    $evidence = $lock.rulebookAcquisitionEvidence
    $rulebookEntries = @($lock.entries | Where-Object { $_.relativePath -ceq 'rulebook/rulebook-current.pdf' })
    if ($rulebookEntries.Count -ne 1 -or
        $evidence.sourceUrl -cne 'https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update' -or
        $evidence.relativePath -cne 'rulebook/rulebook-current.pdf' -or
        $evidence.byteHash -cne $rulebookEntries[0].byteHash -or
        $evidence.retrievedAt -cne $rulebookEntries[0].retrievedAt -or
        [string]::IsNullOrWhiteSpace([string]$evidence.observedFilename) -or
        [string]::IsNullOrWhiteSpace([string]$evidence.privateLocatorEvidence) -or
        $evidence.privateLocatorIsNormative -ne $false) {
        throw 'Private lock rulebook evidence is invalid'
    }
    return [pscustomobject]@{
        path = $resolvedPath
        repositoryRoot = $repositoryRoot
        lock = $lock
    }
}
function Invoke-ExistingPrivateSourceVerifier {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$LockPath
    )

    $verifierPath = [IO.Path]::GetFullPath((Join-Path $script:CollectorScriptRoot '../src/authority/private-source-set.ts'))
    $verifierUrl = [Uri]::new($verifierPath).AbsoluteUri
    $bridge = @'
import { readFile } from 'node:fs/promises';
const lock = JSON.parse(await readFile(process.argv[1], 'utf8'));
const { verifyPrivateSourceSet } = await import(process.argv[3]);
const result = await verifyPrivateSourceSet({
  primaryRoot: lock.primaryRoot,
  backupRoot: lock.backupRoot,
  repositoryRoot: process.argv[2],
  entries: lock.entries,
});
process.stdout.write(JSON.stringify(result));
'@
    try {
        $result = Invoke-BoundedProcess 'node' @(
            '--input-type=module',
            '--eval',
            $bridge,
            $LockPath,
            $RepositoryRoot,
            $verifierUrl
        ) ([IO.Path]::GetDirectoryName($verifierPath)) 30
    }
    catch { throw 'Existing private source verifier did not complete' }
    if ($result.exitCode -ne 0) { throw 'Existing private source verifier failed' }
    try { return $result.stdout | ConvertFrom-Json -Depth 32 -DateKind String }
    catch { throw 'Existing private source verifier returned invalid output' }
}

function Invoke-PrivateAuthorityExistingRootsVerification {
    param([Parameter(Mandatory)][string]$LockPath)

    $record = Read-ConsumedPrivateAuthorityLock $LockPath
    $lock = $record.lock
    $descriptors = @(Get-ProductionSourceDescriptors)
    foreach ($entry in @($lock.entries)) {
        $descriptor = @($descriptors | Where-Object { $_.relativePath -ceq $entry.relativePath })
        if ($descriptor.Count -ne 1 -or
            $entry.url -cne [string]$descriptor[0].provenanceUrl -or
            $entry.mediaType -cne [string]$descriptor[0].mediaType) {
            throw 'Consumed lock entry provenance is invalid'
        }
    }
    $structural = Invoke-ExistingPrivateSourceVerifier $record.repositoryRoot $record.path
    Assert-SameVerification $lock $structural
    $primaryDates = Assert-PrivateAuthorityContentSet ([string]$lock.primaryRoot) $descriptors
    $backupDates = Assert-PrivateAuthorityContentSet ([string]$lock.backupRoot) $descriptors
    foreach ($entry in @($lock.entries)) {
        $relativePath = [string]$entry.relativePath
        if ($entry.effectiveDate -cne $primaryDates[$relativePath] -or
            $entry.effectiveDate -cne $backupDates[$relativePath]) {
            throw 'Consumed lock effective date does not match verified content'
        }
    }
}
function Get-ProductionManualIntakeConfiguration {
    $repositoryRoot = [IO.Path]::GetFullPath((Join-Path $script:CollectorScriptRoot '..'))
    return [pscustomobject]@{
        repositoryRoot = $repositoryRoot
        inboxRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot '.local/authority/manual-inbox/official-2026-08-27-v3'))
        primaryRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot '.local/authority/inputs/official-2026-08-27-v3/primary'))
        backupRoot = [IO.Path]::GetFullPath((Join-Path (Split-Path -Parent $repositoryRoot) 'sorcery-tcg-authority-backup-official-2026-08-27-v3'))
        lockPath = [IO.Path]::GetFullPath((Join-Path $repositoryRoot '.local/authority/locks/official-2026-08-27-v3/source-set-lock.json'))
        descriptors = @(Get-ProductionSourceDescriptors)
    }
}

function Assert-ManualInboxTree {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$InboxRoot,
        [Parameter(Mandatory)][object[]]$Descriptors
    )

    if (-not [IO.Path]::IsPathFullyQualified($InboxRoot)) { throw 'Manual inbox path must be absolute' }
    $repository = [IO.Path]::GetFullPath($RepositoryRoot)
    $inbox = [IO.Path]::GetFullPath($InboxRoot)
    Assert-NoReparseAncestors $inbox
    if (-not (Test-PathWithin $repository $inbox) -or -not [IO.Directory]::Exists($inbox)) {
        throw 'Manual inbox is missing or outside the repository'
    }

    $expectedFiles = @($Descriptors | ForEach-Object { [string]$_.relativePath } | Sort-Object)
    if ($Descriptors.Count -ne 7 -or @($expectedFiles | Sort-Object -Unique).Count -ne 7) {
        throw 'Manual intake requires exactly seven descriptors'
    }
    $expectedDirectories = @(
        $expectedFiles |
            ForEach-Object { [IO.Path]::GetDirectoryName($_).Replace('\', '/') } |
            Sort-Object -Unique
    )
    $actualFiles = [Collections.Generic.List[string]]::new()
    $actualDirectories = [Collections.Generic.List[string]]::new()
    $items = @(Get-ChildItem -LiteralPath $inbox -Recurse -Force | Select-Object -First 13)
    if ($items.Count -gt 12) { throw 'Manual inbox contains too many entries' }
    foreach ($item in $items) {
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            throw 'Manual inbox may not contain links or junctions'
        }
        $relativePath = [IO.Path]::GetRelativePath($inbox, $item.FullName).Replace('\', '/')
        if ($item.PSIsContainer) {
            $actualDirectories.Add($relativePath)
        }
        elseif ($item -is [IO.FileInfo]) {
            $actualFiles.Add($relativePath)
        }
        else {
            throw 'Manual inbox contains a non-ordinary entry'
        }
    }
    if (@(Compare-Object $expectedFiles @($actualFiles | Sort-Object)).Count -ne 0 -or
        @(Compare-Object $expectedDirectories @($actualDirectories | Sort-Object)).Count -ne 0) {
        throw 'Manual inbox must contain exactly the seven fixed files and directories'
    }
    foreach ($descriptor in $Descriptors) {
        $file = [IO.FileInfo]::new((Join-Path $inbox ([string]$descriptor.relativePath)))
        if (-not $file.Exists -or
            $file.Length -lt [long]$descriptor.minBytes -or
            $file.Length -gt [long]$descriptor.maxBytes) {
            throw 'Manual source file size is invalid'
        }
    }
    return $inbox
}

function New-ManualSourceEntries {
    param(
        [Parameter(Mandatory)][string]$Root,
        [Parameter(Mandatory)][object[]]$Descriptors,
        [Parameter(Mandatory)][hashtable]$EffectiveDates,
        [Parameter(Mandatory)][string]$RetrievedAt
    )

    $timestamp = [DateTime]::MinValue
    if (-not [DateTime]::TryParseExact(
        $RetrievedAt,
        "yyyy-MM-dd'T'HH:mm:ss.fff'Z'",
        [Globalization.CultureInfo]::InvariantCulture,
        [Globalization.DateTimeStyles]::AssumeUniversal,
        [ref]$timestamp
    )) { throw 'Manual intake timestamp is invalid' }

    return @($Descriptors | ForEach-Object {
        $descriptor = $_
        $relativePath = [string]$descriptor.relativePath
        $path = Join-Path $Root $relativePath
        $file = [IO.FileInfo]::new($path)
        if (-not $file.Exists -or
            $file.Length -lt [long]$descriptor.minBytes -or
            $file.Length -gt [long]$descriptor.maxBytes) {
            throw 'Manual source file size is invalid'
        }
        [pscustomobject][ordered]@{
            relativePath = $relativePath
            url = [string]$descriptor.provenanceUrl
            retrievedAt = $RetrievedAt
            effectiveDate = $EffectiveDates[$relativePath]
            mediaType = [string]$descriptor.mediaType
            byteLength = $file.Length
            byteHash = "sha256:$((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant())"
        }
    })
}

function Invoke-TestFault {
    param(
        [AllowNull()][string]$FaultPoint,
        [Parameter(Mandatory)][string]$Expected
    )

    if ($FaultPoint -eq $Expected) { throw "Controlled manual-intake test fault: $Expected" }
}

function Invoke-PrivateAuthorityManualIntakeCore {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$InboxRoot,
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][string]$BackupRoot,
        [Parameter(Mandatory)][string]$LockPath,
        [Parameter(Mandatory)][object[]]$Descriptors,
        [Parameter(Mandatory)][bool]$AcknowledgePrivateUseRisk,
        [Parameter(Mandatory)][string]$RetrievedAt,
        [AllowNull()][string]$FaultPoint
    )

    if (-not $AcknowledgePrivateUseRisk) { throw 'AcknowledgePrivateUseRisk is required before manual intake.' }
    $paths = Resolve-CollectionPaths $RepositoryRoot $PrimaryRoot $BackupRoot $LockPath
    $inbox = Assert-ManualInboxTree $paths.repositoryRoot $InboxRoot $Descriptors
    foreach ($destination in @($paths.primaryRoot, $paths.backupRoot, $paths.lockPath)) {
        if ((Test-PathWithin $inbox $destination) -or (Test-PathWithin $destination $inbox)) {
            throw 'Manual inbox and publication destinations may not overlap'
        }
    }
    $effectiveDates = Assert-PrivateAuthorityContentSet $inbox $Descriptors
    $entries = @(New-ManualSourceEntries $inbox $Descriptors $effectiveDates $RetrievedAt)

    $runId = [Guid]::NewGuid().ToString('N')
    $primaryStage = "$($paths.primaryRoot).intake-$runId"
    $backupStage = "$($paths.backupRoot).intake-$runId"
    $lockCandidate = "$($paths.lockPath).intake-$runId"
    $draftPath = "$($paths.lockPath).verifier-$runId.json"
    $primaryPublished = $false
    $backupPublished = $false
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($paths.primaryRoot)) | Out-Null
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($paths.backupRoot)) | Out-Null
    [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($paths.lockPath)) | Out-Null
    try {
        Copy-SourceTree $inbox $primaryStage $Descriptors
        Copy-SourceTree $primaryStage $backupStage $Descriptors
        $primaryDates = Assert-PrivateAuthorityContentSet $primaryStage $Descriptors
        $backupDates = Assert-PrivateAuthorityContentSet $backupStage $Descriptors
        foreach ($entry in $entries) {
            $relativePath = [string]$entry.relativePath
            if ($entry.effectiveDate -cne $primaryDates[$relativePath] -or
                $entry.effectiveDate -cne $backupDates[$relativePath]) {
                throw 'Manual source effective date changed during staging'
            }
        }
        $staged = Invoke-PrivateSourceVerifier $paths.repositoryRoot $primaryStage $backupStage $entries $draftPath 30
        Invoke-TestFault $FaultPoint 'after-staged-verification'

        [IO.Directory]::Move($backupStage, $paths.backupRoot)
        $backupPublished = $true
        Invoke-TestFault $FaultPoint 'after-backup-move'
        [IO.Directory]::Move($primaryStage, $paths.primaryRoot)
        $primaryPublished = $true
        Invoke-TestFault $FaultPoint 'after-primary-move'

        $final = Invoke-PrivateSourceVerifier $paths.repositoryRoot $paths.primaryRoot $paths.backupRoot $entries $draftPath 30
        Assert-SameVerification $staged $final
        $finalPrimaryDates = Assert-PrivateAuthorityContentSet $paths.primaryRoot $Descriptors
        $finalBackupDates = Assert-PrivateAuthorityContentSet $paths.backupRoot $Descriptors
        foreach ($entry in @($final.entries)) {
            $relativePath = [string]$entry.relativePath
            if ($entry.effectiveDate -cne $finalPrimaryDates[$relativePath] -or
                $entry.effectiveDate -cne $finalBackupDates[$relativePath]) {
                throw 'Manual source effective date changed after publication'
            }
        }

        $rulebookEntry = @($final.entries | Where-Object { $_.relativePath -ceq 'rulebook/rulebook-current.pdf' })[0]
        $lock = [ordered]@{
            schemaVersion = 1
            acquisitionMethod = 'user-provided-manual-download'
            authorizationReference = 'phase-01-20260827-manual-provision-1'
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
            rulebookAcquisitionEvidence = [ordered]@{
                sourceUrl = 'https://sorcerytcg.com/news/sorcery-contested-realm-december-2025-rulebook-update'
                relativePath = $rulebookEntry.relativePath
                byteHash = $rulebookEntry.byteHash
                observedFilename = 'rulebook-current.pdf'
                retrievedAt = $rulebookEntry.retrievedAt
                privateLocatorEvidence = 'user-provided-' + 'manual-local-file'
                privateLocatorIsNormative = $false
            }
        }
        Write-NewUtf8Json $lockCandidate $lock
        $candidate = Get-Content -Raw -LiteralPath $lockCandidate | ConvertFrom-Json -Depth 32 -DateKind String
        if ($candidate.acquisitionMethod -cne 'user-provided-manual-download' -or
            $candidate.authorizationReference -cne 'phase-01-20260827-manual-provision-1') {
            throw 'Manual lock identity is invalid'
        }
        $candidateVerification = Invoke-PrivateSourceVerifier $paths.repositoryRoot $candidate.primaryRoot $candidate.backupRoot @($candidate.entries) $draftPath 30
        Assert-SameVerification $final $candidateVerification
        if ($candidate.sourceSetRootHash -cne $candidateVerification.sourceSetRootHash) {
            throw 'Manual lock root hash does not match final verification'
        }
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

function Invoke-PrivateAuthorityManualIntakeForTest {
    param(
        [Parameter(Mandatory)][string]$RepositoryRoot,
        [Parameter(Mandatory)][string]$InboxRoot,
        [Parameter(Mandatory)][string]$PrimaryRoot,
        [Parameter(Mandatory)][string]$BackupRoot,
        [Parameter(Mandatory)][string]$LockPath,
        [Parameter(Mandatory)][object[]]$Descriptors,
        [switch]$AcknowledgePrivateUseRisk,
        [Parameter(Mandatory)][string]$RetrievedAt,
        [AllowNull()][string]$FaultPoint
    )

    $null = Invoke-PrivateAuthorityManualIntakeCore -RepositoryRoot $RepositoryRoot -InboxRoot $InboxRoot -PrimaryRoot $PrimaryRoot -BackupRoot $BackupRoot -LockPath $LockPath -Descriptors $Descriptors -AcknowledgePrivateUseRisk:$AcknowledgePrivateUseRisk -RetrievedAt $RetrievedAt -FaultPoint $FaultPoint
}

function Invoke-PrivateAuthorityManualIntake {
    param([switch]$AcknowledgePrivateUseRisk)

    try {
        $configuration = Get-ProductionManualIntakeConfiguration
        $retrievedAt = [DateTime]::UtcNow.ToString("yyyy-MM-dd'T'HH:mm:ss.fff'Z'", [Globalization.CultureInfo]::InvariantCulture)
        $null = Invoke-PrivateAuthorityManualIntakeCore -RepositoryRoot $configuration.repositoryRoot -InboxRoot $configuration.inboxRoot -PrimaryRoot $configuration.primaryRoot -BackupRoot $configuration.backupRoot -LockPath $configuration.lockPath -Descriptors @($configuration.descriptors) -AcknowledgePrivateUseRisk:$AcknowledgePrivateUseRisk -RetrievedAt $retrievedAt -FaultPoint $null
    }
    catch { throw 'Private authority manual intake failed.' }
}

Export-ModuleMember -Function @(
    'Get-ProductionSourceDescriptors',
    'Invoke-PrivateAuthorityManualIntakeForTest'
)
}

Import-Module -ModuleInfo $collectorModule -Scope Local

if ($MyInvocation.InvocationName -ne '.') {
    try {
        if ($VerifyExistingRoots) {
            if ($ImportManualInbox -or
                [string]::IsNullOrWhiteSpace($LockPath) -or
                $AcknowledgePrivateUseRisk) {
                throw 'Offline verification parameters are invalid'
            }
            & $collectorModule { param($ExistingLockPath) Invoke-PrivateAuthorityExistingRootsVerification $ExistingLockPath } $LockPath
            Write-Output 'Private authority existing roots verified.'
        }
        elseif ($ImportManualInbox) {
            if (-not [string]::IsNullOrWhiteSpace($LockPath) -or -not $AcknowledgePrivateUseRisk) {
                throw 'Manual intake parameters are invalid'
            }
            & $collectorModule { param($Acknowledged) Invoke-PrivateAuthorityManualIntake -AcknowledgePrivateUseRisk:$Acknowledged } $AcknowledgePrivateUseRisk
            Write-Output 'Private authority manual intake completed.'
        }
        else {
            throw 'Exactly one of ImportManualInbox or VerifyExistingRoots is required'
        }
    }
    catch {
        $failureMessage = if ($VerifyExistingRoots) {
            'Private authority existing-root verification failed.'
        }
        else { 'Private authority manual intake failed.' }
        [Console]::Error.WriteLine($failureMessage)
        exit 1
    }
}

Remove-Variable -Name collectorModule
