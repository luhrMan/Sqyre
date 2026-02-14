; Sqyre Windows Installer — NSIS Script
; Built inside the Docker cross-compile environment via makensis.
;
; Expected staging layout (populated by build.sh):
;   staging/
;     sqyre.exe
;     *.dll          (collected by collect-dlls.sh)
;     tessdata/
;       eng.traineddata

!include "MUI2.nsh"

; ---------------------------------------------------------------------------
; Build-time defines (can be overridden with -D on makensis command line)
; ---------------------------------------------------------------------------
!ifndef APP_NAME
  !define APP_NAME "Sqyre"
!endif
!ifndef APP_VERSION
  !define APP_VERSION "0.5.0"
!endif
!ifndef EXE_NAME
  !define EXE_NAME "Sqyre.exe"
!endif
!ifndef STAGING_DIR
  !define STAGING_DIR "staging"
!endif

Name "${APP_NAME} ${APP_VERSION}"
!ifndef OUT_DIR
  !define OUT_DIR "."
!endif
OutFile "${OUT_DIR}/SqyreSetup-${APP_VERSION}.exe"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallDir"
RequestExecutionLevel admin

; ---------------------------------------------------------------------------
; Modern UI pages
; ---------------------------------------------------------------------------
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ---------------------------------------------------------------------------
; Installer section
; ---------------------------------------------------------------------------
Section "Install"
    SetOutPath "$INSTDIR"

    ; Main executable
    File "${STAGING_DIR}/${EXE_NAME}"

    ; All DLL dependencies
    File "${STAGING_DIR}/*.dll"

    ; Tesseract trained data
    SetOutPath "$INSTDIR\tessdata"
    File "${STAGING_DIR}/tessdata/eng.traineddata"

    ; Reset output path
    SetOutPath "$INSTDIR"

    ; Store install dir in registry
    WriteRegStr HKLM "Software\${APP_NAME}" "InstallDir" "$INSTDIR"

    ; Add/Remove Programs entry
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
        "DisplayName" "${APP_NAME}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
        "DisplayVersion" "${APP_VERSION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
        "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
        "InstallLocation" "$INSTDIR"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
        "Publisher" "Sqyre"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
        "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" \
        "NoRepair" 1

    ; Set TESSDATA_PREFIX environment variable for current user
    WriteRegStr HKCU "Environment" "TESSDATA_PREFIX" "$INSTDIR\tessdata"

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Start Menu shortcuts
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortCut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}"
    CreateShortCut "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk" "$INSTDIR\uninstall.exe"

    ; Desktop shortcut
    CreateShortCut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\${EXE_NAME}"
SectionEnd

; ---------------------------------------------------------------------------
; Uninstaller section
; ---------------------------------------------------------------------------
Section "Uninstall"
    ; Remove files
    Delete "$INSTDIR\${EXE_NAME}"
    Delete "$INSTDIR\*.dll"
    Delete "$INSTDIR\tessdata\eng.traineddata"
    RMDir "$INSTDIR\tessdata"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    ; Remove shortcuts
    Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_NAME}\Uninstall.lnk"
    RMDir "$SMPROGRAMS\${APP_NAME}"
    Delete "$DESKTOP\${APP_NAME}.lnk"

    ; Remove environment variable
    DeleteRegValue HKCU "Environment" "TESSDATA_PREFIX"

    ; Remove registry keys
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
    DeleteRegKey HKLM "Software\${APP_NAME}"
SectionEnd
