@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\sync-remote-gemma-model-cache.ps1" %*
exit /b %ERRORLEVEL%
