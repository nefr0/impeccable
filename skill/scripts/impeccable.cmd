@echo off
setlocal
rem Impeccable launcher (Windows). Runs a local engine binary; never downloads.
rem Order: IMPECCABLE_BIN, bin\windows-<arch>\impeccable.exe next to this file,
rem %USERPROFILE%\.impeccable\bin\impeccable.exe, then impeccable-engine /
rem impeccable on PATH. Home and PATH candidates must answer the engine-probe
rem handshake so the retired 3.x npm CLI (also named impeccable) is never run.
if not defined IMPECCABLE_SKILL_DIR set "IMPECCABLE_SKILL_DIR=%~dp0.."
if not defined IMPECCABLE_SELF set "IMPECCABLE_SELF=%~f0"
set "arch=x64"
if /I "%PROCESSOR_ARCHITECTURE%"=="ARM64" set "arch=arm64"

if not defined IMPECCABLE_BIN goto no_env_bin
if not exist "%IMPECCABLE_BIN%" goto no_env_bin
set "run=%IMPECCABLE_BIN%"
goto run
:no_env_bin

set "bin=%~dp0bin\windows-%arch%\impeccable.exe"
if not exist "%bin%" goto no_sibling
set "run=%bin%"
goto run
:no_sibling

set "home_bin=%USERPROFILE%\.impeccable\bin\impeccable.exe"
if not exist "%home_bin%" goto no_home_bin
if defined IMPECCABLE_LAUNCHER_PROBE goto no_home_bin
call :probe "%home_bin%"
if not "%probe_ok%"=="1" goto no_home_bin
set "run=%home_bin%"
goto run
:no_home_bin

if defined IMPECCABLE_LAUNCHER_PROBE exit /b 127
where impeccable-engine >nul 2>nul
if errorlevel 1 goto no_path_engine
call :probe impeccable-engine
if not "%probe_ok%"=="1" goto no_path_engine
impeccable-engine %*
exit /b %errorlevel%
:no_path_engine
where impeccable >nul 2>nul
if errorlevel 1 goto fail
call :probe impeccable
if not "%probe_ok%"=="1" goto fail
impeccable %*
exit /b %errorlevel%

:fail
echo impeccable: no engine binary found (looked in IMPECCABLE_BIN, %bin%, %home_bin%, PATH). 1>&2
echo build it: cargo build --release -p impeccable, then set IMPECCABLE_BIN 1>&2
exit /b 127

:run
"%run%" %*
exit /b %errorlevel%

:probe
rem Sets probe_ok=1 when %1 answers the engine handshake: prints
rem "impeccable-engine <version>" and exits 0.
set "probe_ok="
set "probe_tmp=%TEMP%\impeccable-probe-%RANDOM%%RANDOM%.txt"
set "IMPECCABLE_LAUNCHER_PROBE=1"
"%~1" engine-probe >"%probe_tmp%" 2>nul
set "probe_err=%ERRORLEVEL%"
set "IMPECCABLE_LAUNCHER_PROBE="
if not "%probe_err%"=="0" goto probe_done
findstr /b /c:"impeccable-engine" "%probe_tmp%" >nul 2>nul
if errorlevel 1 goto probe_done
set "probe_ok=1"
:probe_done
del "%probe_tmp%" >nul 2>nul
exit /b 0
