@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\reload-remote-gemma-backend.ps1" %*
exit /b %ERRORLEVEL%
