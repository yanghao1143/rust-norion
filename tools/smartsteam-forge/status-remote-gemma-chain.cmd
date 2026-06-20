@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\start-remote-gemma-chain.ps1" -Status %*
exit /b %ERRORLEVEL%
