@echo off
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Build.ps1"
set "build_exit=%errorlevel%"
if not "%build_exit%"=="0" pause
exit /b %build_exit%
