; Hooks del instalador NSIS de InnoApp Agent (Tauri).
; El instalador corre elevado (installMode = perMachine), asi que aca se
; puede registrar el servicio Windows y preparar la carpeta de datos.

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Preparando InnoApp Agent..."

  ; Carpeta de datos compartida entre el servicio (LocalSystem) y la app de
  ; bandeja (usuario). Se le da modificacion a BUILTIN\Users (*S-1-5-32-545)
  ; para que la activacion pueda escribir credentials.json sin elevar.
  CreateDirectory "C:\ProgramData\InnoApp Agent"
  CreateDirectory "C:\ProgramData\InnoApp Agent\logs"
  nsExec::ExecToLog 'icacls "C:\ProgramData\InnoApp Agent" /grant "*S-1-5-32-545:(OI)(CI)M" /T /C'

  ; Servicio Windows: corre resources\innoapp-agent-service.exe como
  ; LocalSystem (cuenta por defecto de sc create), arranque automatico. Se
  ; borra primero por si es un upgrade.
  nsExec::ExecToLog 'sc.exe stop "InnoAppAgent"'
  nsExec::ExecToLog 'sc.exe delete "InnoAppAgent"'
  Sleep 1500
  nsExec::ExecToLog 'sc.exe create "InnoAppAgent" binPath= "\"$INSTDIR\resources\innoapp-agent-service.exe\"" start= auto DisplayName= "InnoApp Agent"'
  nsExec::ExecToLog 'sc.exe description "InnoAppAgent" "Captura los tickets impresos en esta PC y los sube a InnoApp."'
  nsExec::ExecToLog 'sc.exe failure "InnoAppAgent" reset= 86400 actions= restart/5000/restart/5000/restart/60000'
  nsExec::ExecToLog 'sc.exe start "InnoAppAgent"'

  ; La app de bandeja (visor de estado) arranca al iniciar sesion cualquier usuario.
  CreateShortcut "$SMSTARTUP\InnoApp Agent.lnk" "$INSTDIR\InnoApp Agent.exe"
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'sc.exe stop "InnoAppAgent"'
  Sleep 2000
  nsExec::ExecToLog 'sc.exe delete "InnoAppAgent"'
  Delete "$SMSTARTUP\InnoApp Agent.lnk"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; credentials.json / queue.json / logs quedan en C:\ProgramData\InnoApp Agent
  ; a proposito — una reinstalacion los reutiliza. Borrar a mano si se quiere
  ; una desinstalacion total.
!macroend
