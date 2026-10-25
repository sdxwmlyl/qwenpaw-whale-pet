@echo off
cd /d D:\ProjectWhalePet\overlay
start "" /b D:\Miniconda\pythonw.exe bridge.py
timeout /t 1 >nul
start "" "D:\ProjectWhalePet\overlay\src-tauri\target\debug\whale-pet.exe"
