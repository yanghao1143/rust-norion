@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\test-chat-gemma-lab-client.ps1" %*
exit /b %ERRORLEVEL%
