@echo off
cd /d D:\ProjectWhalePet\overlay
start "" /b D:\Miniconda\python.exe bridge.py
timeout /t 2 >nul
start "" "D:\ProjectWhalePet\overlay\src-tauri\target\debug\whale-pet.exe"
