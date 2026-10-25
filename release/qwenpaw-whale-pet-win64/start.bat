@echo off
set DIR=%~dp0
cd /d "%DIR%"
tasklist /fi "imagename eq bridge.exe" | find /i "bridge.exe" >nul
if errorlevel 1 start "" /b "%DIR%bridge.exe"
timeout /t 2 >nul
tasklist /fi "imagename eq whale-pet.exe" | find /i "whale-pet.exe" >nul
if errorlevel 1 start "" "%DIR%whale-pet.exe"
