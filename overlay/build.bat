@echo off
cd /d D:\ProjectWhalePet\overlay\src-tauri
cargo build > D:\ProjectWhalePet\overlay\build.log 2>&1
echo BUILD_EXIT %ERRORLEVEL% >> D:\ProjectWhalePet\overlay\build.log
