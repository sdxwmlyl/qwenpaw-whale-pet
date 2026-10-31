@echo off
chcp 65001 >nul
cd /d D:\ProjectWhalePet\overlay
rem bridge is built into whale-pet.exe now; do NOT start python bridge (it grabs port 7799)
tasklist /fi "imagename eq whale-pet.exe" | find /i "whale-pet.exe" >nul
if errorlevel 1 (
  start "" "D:\ProjectWhalePet\overlay\src-tauri\target\release\whale-pet.exe"
)
exit
