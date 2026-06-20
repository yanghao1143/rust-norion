@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\smoke-forge-stack.ps1" %*
exit /b %ERRORLEVEL%
