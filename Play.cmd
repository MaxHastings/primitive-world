@echo off
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Play.ps1" %*
set "play_exit=%errorlevel%"
if not "%play_exit%"=="0" pause
exit /b %play_exit%
