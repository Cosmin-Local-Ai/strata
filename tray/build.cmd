@echo off
rem Builds strata-tray-hook.dll (x64) with the Visual Studio C++ tools and copies it next to strata.exe.
setlocal
where cl >nul 2>nul || call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" >nul || exit /b 1
cd /d "%~dp0"
if not exist out mkdir out
cl /nologo /O2 /LD /EHsc /MT strata_tray_hook.cpp /Fo:out\ /Fe:out\strata-tray-hook.dll /link user32.lib gdi32.lib kernel32.lib || exit /b 1
if exist ..\src-tauri\target\release copy /y out\strata-tray-hook.dll ..\src-tauri\target\release\ >nul
echo built tray\out\strata-tray-hook.dll
