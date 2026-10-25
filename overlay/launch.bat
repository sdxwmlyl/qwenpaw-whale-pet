@echo off
chcp 65001 >nul
cd /d D:\ProjectWhalePet\overlay

curl -s -m 2 -o nul http://127.0.0.1:7799/health
if errorlevel 1 (
  start "" /b D:\Miniconda\pythonw.exe bridge.py
  timeout /t 2 >nul
)

tasklist /fi "imagename eq whale-pet.exe" | find /i "whale-pet.exe" >nul
if errorlevel 1 (
  start "" "D:\ProjectWhalePet\overlay\src-tauri\target\release\whale-pet.exe"
)

exit
