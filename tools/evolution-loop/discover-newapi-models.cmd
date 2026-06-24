@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0discover-newapi-models.ps1" %*
exit /b %ERRORLEVEL%
