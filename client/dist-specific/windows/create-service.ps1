# start.ps1
# создаёт/перезапускает сервис на основе turn-proxy-client.exe
# путь к exe сделайте абсолютным (можно из текущего каталога скрипта)
$exeName = "turn-proxy-client.exe"
$serviceName = "turn-proxy-client"
$displayName = "TURN Proxy Client"
$description = "Сервис turn-proxy-client"
$exePath = Join-Path $PSScriptRoot $exeName

if (-not (Test-Path $exePath)) {
    Write-Error "Файл не найден: $exePath"
    exit 1
}

function Remove-ServiceIfExists {
    param([string]$Name)
    $exists = Get-Service -Name $Name -ErrorAction SilentlyContinue
    if ($exists) {
        Write-Host "Останавливаю сервис $Name..."
        Stop-Service -Name $Name -Force -ErrorAction SilentlyContinue
        Write-Host "Удаляю сервис $Name..."
        sc.exe delete $Name | Out-Null
        Start-Sleep -Seconds 1
    }
}

function Create-Service {
    param(
        [string]$Name,
        [string]$DisplayName,
        [string]$BinPath,
        [string]$Description
    )
    Write-Host "Создаю сервис $Name..."
    sc.exe create $Name binPath= "`"$BinPath`"" DisplayName= "`"$DisplayName`"" start= auto obj= "LocalSystem" | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Не удалось создать сервис"; }

    if ($Description) {
        sc.exe description $Name "`"$Description`"" | Out-Null
    }
}

try {
    Remove-ServiceIfExists -Name $serviceName
    Create-Service -Name $serviceName -DisplayName $displayName -BinPath $exePath -Description $description
    Write-Host "Запускаю сервис $serviceName..."
    Start-Service -Name $serviceName
    Write-Host "Сервис установлен и запущен."
} catch {
    Write-Error $_.Exception.Message
    exit 1
}