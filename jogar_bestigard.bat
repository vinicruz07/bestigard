@echo off
title Bestigard - Santuario das Bestas
cd /d "%~dp0"
if exist "target\debug\mobile_entry.exe" (
    "target\debug\mobile_entry.exe" --window
) else (
    cargo run -p mobile_entry -- --window
)

