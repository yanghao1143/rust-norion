@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0test-newapi-model-discovery.ps1" %*
exit /b %ERRORLEVEL%
