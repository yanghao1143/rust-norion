@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%publish-strict-status-summary.ps1" %*
exit /b %ERRORLEVEL%
