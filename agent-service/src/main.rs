//! Servicio Windows del agente de InnoApp.
//!
//! Corre `node.exe app.mjs` (el print-capture-agent) como `LocalSystem`, de
//! modo que:
//!  - arranca solo con Windows y sobrevive reboot / cierre de sesión
//!  - tiene permiso para leer `C:\Windows\System32\spool\PRINTERS` (la
//!    captura de spool lo necesita — ver print-capture-agent)
//!
//! Sin depender de que alguien abra la app de bandeja ni de "ejecutar como
//! administrador".
//!
//! La app de bandeja (`agent-status-viewer`) pasa a ser solo un visor: se
//! conecta al named pipe que levanta el agente y muestra el estado. Al
//! activar, escribe `credentials.json` en `C:\ProgramData\InnoApp Agent\`;
//! este servicio vigila ese archivo y reinicia el agente cuando cambia.

use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::mpsc,
    time::{Duration, SystemTime},
};

use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

const SERVICE_NAME: &str = "InnoAppAgent";
const SERVICE_DISPLAY: &str = "InnoApp Agent";
const CLOUD_UPLOAD_URL: &str = "https://uqa4ti7fwi.execute-api.us-east-1.amazonaws.com/prod/tickets";

/// Carpeta compartida entre el servicio (LocalSystem) y la app de bandeja
/// (usuario). El instalador la crea con permiso de modificación para
/// `BUILTIN\Users` para que la activación pueda escribir `credentials.json`.
fn data_dir() -> PathBuf {
    PathBuf::from(r"C:\ProgramData\InnoApp Agent")
}

fn credentials_path() -> PathBuf {
    data_dir().join("credentials.json")
}

fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

fn log_line(file: &str, msg: &str) {
    let _ = fs::create_dir_all(logs_dir());
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(logs_dir().join(file)) {
        let _ = writeln!(f, "[{}] {}", now_iso(), msg);
    }
}

fn now_iso() -> String {
    // Sin dependencias de fecha: segundos desde epoch alcanza para ordenar.
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("t+{}", d.as_secs()),
        Err(_) => "t+0".to_string(),
    }
}

/// Ubica `resources\agent\` (node.exe + app.mjs) relativo al exe del servicio.
/// El instalador deja el servicio en `<install>\` y el runtime en
/// `<install>\resources\agent\`.
fn runtime_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let base = exe.parent()?;
    let candidates = [
        base.join("resources").join("agent"),
        base.join("agent"),
        base.parent().map(|p| p.join("resources").join("agent")).unwrap_or_default(),
    ];
    candidates.into_iter().find(|c| c.join("node.exe").exists())
}

fn credentials_mtime() -> Option<SystemTime> {
    fs::metadata(credentials_path()).ok()?.modified().ok()
}

fn spawn_agent(runtime: &Path) -> std::io::Result<Child> {
    let _ = fs::create_dir_all(logs_dir());
    // Append, no truncate: si el agente se cae y se relanza, no se pierde
    // el log del intento anterior.
    let open_log = |name: &str| fs::OpenOptions::new().create(true).append(true).open(logs_dir().join(name));
    let stdout = open_log("agent.log")?;
    let stderr = open_log("agent-error.log")?;

    // La captura viene prendida; overrideable con ENABLE_CAPTURE=false en el
    // entorno de la MÁQUINA para dejar una instalación en modo diagnóstico.
    let enable_capture = std::env::var("ENABLE_CAPTURE").unwrap_or_else(|_| "true".to_string());

    let data = data_dir();
    let mut command = Command::new(runtime.join("node.exe"));
    command
        .arg("app.mjs")
        .current_dir(runtime)
        .env("CLOUD_UPLOAD_URL", CLOUD_UPLOAD_URL)
        .env("AGENT_CREDENTIALS_FILE", data.join("credentials.json"))
        .env("QUEUE_FILE", data.join("queue.json"))
        .env("ENABLE_CAPTURE", enable_capture)
        .stdout(stdout)
        .stderr(stderr);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command.spawn()
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_args: Vec<OsString>) {
    if let Err(err) = run_service() {
        log_line("service-error.log", &format!("run_service falló: {err}"));
    }
}

fn run_service() -> windows_service::Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();

    let event_handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = shutdown_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    let report = |state: ServiceState, accept: ServiceControlAccept, exit: u32| {
        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: state,
            controls_accepted: accept,
            exit_code: ServiceExitCode::Win32(exit),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
    };

    report(
        ServiceState::Running,
        ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        0,
    )?;
    log_line("service.log", "servicio arrancado");

    let runtime = match runtime_dir() {
        Some(dir) => dir,
        None => {
            log_line("service-error.log", "no se encontró resources\\agent\\node.exe — el servicio no puede correr el agente");
            report(ServiceState::Stopped, ServiceControlAccept::empty(), 1)?;
            return Ok(());
        }
    };
    log_line("service.log", &format!("runtime del agente: {}", runtime.display()));

    let mut child: Option<Child> = None;
    let mut last_creds_mtime = credentials_mtime();
    let mut backoff = Duration::from_secs(2);
    const MAX_BACKOFF: Duration = Duration::from_secs(60);
    const TICK: Duration = Duration::from_secs(3);

    loop {
        // Señal de stop/shutdown del SCM.
        match shutdown_rx.recv_timeout(TICK) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(mut c) = child.take() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
                log_line("service.log", "servicio detenido");
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        // credentials.json cambió (activación / reactivación) → reiniciar el agente.
        let mtime = credentials_mtime();
        if mtime != last_creds_mtime {
            last_creds_mtime = mtime;
            log_line("service.log", "credentials.json cambió — reiniciando el agente");
            if let Some(mut c) = child.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }

        // ¿Hace falta (re)lanzar el agente?
        let should_spawn = match child.as_mut() {
            // Sin agente: solo si ya hay credenciales (si no, esperamos a que se active).
            None => credentials_mtime().is_some(),
            // Con agente: relanzar si terminó (crash, o salida por error).
            Some(c) => match c.try_wait() {
                Ok(Some(status)) => {
                    log_line("service.log", &format!("el agente terminó ({status}) — relanzando"));
                    child = None;
                    true
                }
                Ok(None) => false,
                Err(err) => {
                    log_line("service-error.log", &format!("try_wait falló: {err}"));
                    false
                }
            },
        };

        if should_spawn {
            match spawn_agent(&runtime) {
                Ok(c) => {
                    log_line("service.log", &format!("agente lanzado (pid {})", c.id()));
                    child = Some(c);
                    backoff = Duration::from_secs(2);
                }
                Err(err) => {
                    log_line("service-error.log", &format!("no se pudo lanzar el agente: {err} — reintenta en {}s", backoff.as_secs()));
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            }
        }
    }

    report(ServiceState::Stopped, ServiceControlAccept::empty(), 0)?;
    Ok(())
}

// --- instalación/desinstalación (para dev; el instalador usa sc.exe) ---

fn install() -> windows_service::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CREATE_SERVICE)?;
    let exe = std::env::current_exe().expect("current_exe");
    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // LocalSystem
        account_password: None,
    };
    let service = manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START)?;
    let _ = service.set_description("Captura tickets impresos en esta PC y los sube a InnoApp.");
    service.start(&[] as &[&str])?;
    println!("servicio '{SERVICE_NAME}' instalado y arrancado");
    Ok(())
}

fn uninstall() -> windows_service::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    let service = manager.open_service(SERVICE_NAME, ServiceAccess::STOP | ServiceAccess::DELETE)?;
    let _ = service.stop();
    service.delete()?;
    println!("servicio '{SERVICE_NAME}' eliminado");
    Ok(())
}

fn main() -> windows_service::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("install") => install(),
        Some("uninstall") => uninstall(),
        _ => service_dispatcher::start(SERVICE_NAME, ffi_service_main),
    }
}
