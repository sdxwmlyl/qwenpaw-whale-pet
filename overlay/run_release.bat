@echo off
cd /d D:\ProjectWhalePet\overlay
start "" /min "D:\ProjectWhalePet\overlay\dist\bridge.exe"
timeout /t 1 >nul
start "" "D:\ProjectWhalePet\overlay\src-tauri\target\release\whale-pet.exe"
