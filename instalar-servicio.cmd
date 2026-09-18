@echo off
setlocal enabledelayedexpansion
title Instalar servicio de Windows - node-winsvc

rem ============================================================================
rem  Instalador de doble clic.
rem  Se eleva solo a administrador, comprueba las dependencias ANTES de
rem  registrar nada, instala, arranca y muestra el estado.
rem
rem  Copiar este archivo a la carpeta del proyecto que tiene winsvc.config.json.
rem ============================================================================

rem --- 1. Elevarse a administrador si no lo esta -------------------------------
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo Solicitando permisos de administrador...
    powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
    exit /b
)

cd /d "%~dp0"
echo.
echo ============================================================
echo   node-winsvc - Instalacion del servicio
echo   Carpeta: %CD%
echo ============================================================
echo.

rem --- 2. Comprobar que existe la configuracion --------------------------------
if not exist "winsvc.config.json" (
    echo [FALLO] No se encuentra winsvc.config.json en esta carpeta.
    echo         Ejecuta primero:  npx node-winsvc init
    goto :fin
)
echo [ok]    winsvc.config.json encontrado

rem --- 3. Comprobar que Node esta instalado ------------------------------------
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [FALLO] node.exe no esta en el PATH. Instala Node.js 18 o superior.
    goto :fin
)
for /f "delims=" %%v in ('node --version') do echo [ok]    Node %%v

rem --- 4. Diagnostico: script, binario, permisos -------------------------------
echo.
echo --- Comprobando dependencias ---
call npx node-winsvc doctor
if %errorlevel% neq 0 (
    echo.
    echo [FALLO] El diagnostico fallo. No se instala nada.
    echo         Corrige lo de arriba y vuelve a ejecutar este archivo.
    goto :fin
)

rem --- 5. Si el servicio ya existe, desinstalarlo primero ----------------------
echo.
echo --- Instalando ---
call npx node-winsvc uninstall >nul 2>&1

call npx node-winsvc install
if %errorlevel% neq 0 (
    echo.
    echo [FALLO] No se pudo registrar el servicio.
    goto :fin
)

rem --- 6. Arrancar y mostrar el estado -----------------------------------------
echo.
echo --- Arrancando ---
call npx node-winsvc start
if %errorlevel% neq 0 (
    echo.
    echo [FALLO] El servicio quedo instalado pero no arranco.
    echo         Revisa el log:  npx node-winsvc logs
    goto :fin
)

timeout /t 3 /nobreak >nul
echo.
echo --- Estado ---
call npx node-winsvc status

echo.
echo ============================================================
echo   Listo. El servicio arranca solo al reiniciar Windows.
echo.
echo   Ver el log en vivo:   npx node-winsvc logs -f
echo   Reiniciarlo:          npx node-winsvc restart
echo   Quitarlo:             npx node-winsvc uninstall
echo ============================================================

:fin
echo.
pause
endlocal
