@echo off
cd /d D:\ProjectWhalePet\overlay
call npx --yes @tauri-apps/cli@latest build > D:\ProjectWhalePet\overlay\bundle.log 2>&1
echo EXIT %ERRORLEVEL% >> D:\ProjectWhalePet\overlay\bundle.log
