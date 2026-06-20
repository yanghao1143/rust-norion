@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\run-remote-gemma-unattended.ps1" %*
exit /b %ERRORLEVEL%
