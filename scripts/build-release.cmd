@echo off
setlocal

set "PROJECT_ROOT=%~dp0.."
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if not exist "%VSWHERE%" (
  echo Visual Studio Installer was not found.
  exit /b 1
)

for /f "usebackq tokens=*" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VS_PATH=%%i"
if not defined VS_PATH (
  echo MSVC C++ build tools were not found.
  exit /b 1
)

call "%VS_PATH%\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b %errorlevel%

set "RUST_BIN=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
if not exist "%RUST_BIN%\cargo.exe" (
  echo Stable Rust toolchain was not found at %RUST_BIN%.
  exit /b 1
)

set "PATH=%RUST_BIN%;%PATH%"
set "CARGO_HOME=%PROJECT_ROOT%\.cargo-cache"
set "npm_config_cache=%PROJECT_ROOT%\.npm-cache"
cd /d "%PROJECT_ROOT%"

call npm run build
if errorlevel 1 exit /b %errorlevel%

call npm run tauri build -- --no-bundle
if errorlevel 1 exit /b %errorlevel%

if not exist "%PROJECT_ROOT%\release" mkdir "%PROJECT_ROOT%\release"
copy /y "%PROJECT_ROOT%\src-tauri\target\release\codex-account-manager.exe" "%PROJECT_ROOT%\release\CodexAccountManager.exe" >nul
if errorlevel 1 exit /b %errorlevel%

echo Created release\CodexAccountManager.exe
