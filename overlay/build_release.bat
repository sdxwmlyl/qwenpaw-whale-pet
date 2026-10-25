@echo off
cd /d D:\ProjectWhalePet\overlay\src-tauri
cargo build --release > D:\ProjectWhalePet\overlay\build_release.log 2>&1
echo BUILD_EXIT %ERRORLEVEL% >> D:\ProjectWhalePet\overlay\build_release.log
