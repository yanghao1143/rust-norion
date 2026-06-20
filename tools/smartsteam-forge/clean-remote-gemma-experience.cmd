@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\clean-remote-gemma-experience.ps1" %*
exit /b %ERRORLEVEL%
