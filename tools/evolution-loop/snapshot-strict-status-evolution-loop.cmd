@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
set "REPO_ROOT=%SCRIPT_DIR%..\.."
set "OUT=%REPO_ROOT%\target\evolution\strict-status.json"
if not "%~1"=="" (
  set "OUT=%~1"
  shift /1
)
if not exist "%REPO_ROOT%\target\evolution" mkdir "%REPO_ROOT%\target\evolution" >nul 2>nul
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%status-evolution-loop.ps1" -JsonStatus -StrictUnattendedEvolution -FailOnNotReady -SkipProcess %* > "%OUT%"
set "STATUS_CODE=%ERRORLEVEL%"
echo strict_status_snapshot=%OUT%
exit /b %STATUS_CODE%
