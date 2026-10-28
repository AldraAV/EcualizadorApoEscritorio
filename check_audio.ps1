Get-Process | Where-Object { ![string]::IsNullOrWhiteSpace($_.MainWindowTitle) } | Select-Object ProcessName, MainWindowTitle
