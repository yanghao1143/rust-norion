@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\test-remote-gemma-forge-status.ps1" %*
exit /b %ERRORLEVEL%
