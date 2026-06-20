@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\repl-gemma-lab.ps1" %*
exit /b %ERRORLEVEL%
