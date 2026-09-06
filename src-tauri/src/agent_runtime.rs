use std::{
    fs::{self, File},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

const ACTIVATION_URL: &str = "https://uqa4ti7fwi.execute-api.us-east-1.amazonaws.com/prod/agents/activate";
const UPLOAD_URL: &str = "https://uqa4ti7fwi.execute-api.us-east-1.amazonaws.com/prod/tickets";

#[derive(Default)]
pub struct AgentProcess(pub Mutex<Option<Child>>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationStatus {
    pub activated: bool,
    pub agent_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentCredentials {
    agent_id: String,
    name: String,
    api_key: String,
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|err| err.to_string())
}

fn credentials_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("credentials.json"))
}

fn load_credentials(app: &AppHandle) -> Result<Option<AgentCredentials>, String> {
    let path = credentials_path(app)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map(Some).map_err(|err| err.to_string())
}

fn runtime_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let resources = app.path().resource_dir().map_err(|err| err.to_string())?;
    for candidate in [resources.join("resources/agent"), resources.join("agent")] {
        if candidate.join("node.exe").exists() {
            return Ok(candidate);
        }
    }
    Err(format!("no se encontró el runtime del agente en {}", resources.display()))
}

pub fn start_agent(app: &AppHandle, process: &AgentProcess) -> Result<(), String> {
    if load_credentials(app)?.is_none() {
        return Ok(());
    }

    let mut current = process.0.lock().map_err(|_| "no se pudo bloquear el proceso del agente".to_string())?;
    if let Some(child) = current.as_mut() {
        if child.try_wait().map_err(|err| err.to_string())?.is_none() {
            return Ok(());
        }
    }

    let runtime = runtime_dir(app)?;
    let data = data_dir(app)?;
    fs::create_dir_all(&data).map_err(|err| err.to_string())?;
    let logs = data.join("logs");
    fs::create_dir_all(&logs).map_err(|err| err.to_string())?;
    let stdout = File::create(logs.join("agent.log")).map_err(|err| err.to_string())?;
    let stderr = File::create(logs.join("agent-error.log")).map_err(|err| err.to_string())?;

    // Build del piloto: la captura viene PRENDIDA. Ya se validó instalación,
    // activación, detección de puertos y heartbeat contra PCs reales, así que
    // esta versión abre los puertos detectados para leer tickets. Sigue
    // siendo overrideable desde el entorno del SO (ENABLE_CAPTURE=false) para
    // poder dejar una instalación en modo "solo diagnóstico" sin recompilar.
    let enable_capture = std::env::var("ENABLE_CAPTURE").unwrap_or_else(|_| "true".to_string());

    let mut command = Command::new(runtime.join("node.exe"));
    command
        // El directorio de instalación contiene espacios ("InnoApp Agent").
        // Ejecutar el entrypoint de forma relativa evita que Node reciba una
        // ruta truncada al crear el proceso en Windows.
        .arg("app.mjs")
        .current_dir(&runtime)
        .env("CLOUD_UPLOAD_URL", UPLOAD_URL)
        .env("AGENT_CREDENTIALS_FILE", data.join("credentials.json"))
        .env("QUEUE_FILE", data.join("queue.json"))
        .env("ENABLE_CAPTURE", enable_capture)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    *current = Some(command.spawn().map_err(|err| format!("no se pudo arrancar el agente: {err}"))?);
    Ok(())
}

pub fn stop_agent(process: &AgentProcess) {
    if let Ok(mut current) = process.0.lock() {
        if let Some(child) = current.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        *current = None;
    }
}

#[tauri::command]
pub fn get_activation_status(app: AppHandle) -> Result<ActivationStatus, String> {
    let credentials = load_credentials(&app)?;
    Ok(ActivationStatus {
        activated: credentials.is_some(),
        agent_name: credentials.map(|value| value.name),
    })
}

#[tauri::command]
pub async fn activate_agent(
    app: AppHandle,
    process: State<'_, AgentProcess>,
    code: String,
    name: String,
) -> Result<ActivationStatus, String> {
    let response = reqwest::Client::new()
        .post(ACTIVATION_URL)
        .json(&serde_json::json!({ "code": code.trim().to_uppercase(), "name": name.trim() }))
        .send()
        .await
        .map_err(|err| format!("no se pudo conectar con InnoApp: {err}"))?;
    let status = response.status();
    let body = response.text().await.map_err(|err| err.to_string())?;
    if !status.is_success() {
        let message = serde_json::from_str::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| value.get("error")?.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("la activación respondió HTTP {status}"));
        return Err(message);
    }

    let credentials: AgentCredentials = serde_json::from_str(&body).map_err(|_| "la activación devolvió una respuesta incompleta".to_string())?;
    let path = credentials_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(&path, serde_json::to_vec_pretty(&credentials).map_err(|err| err.to_string())?).map_err(|err| err.to_string())?;
    start_agent(&app, &process)?;

    Ok(ActivationStatus { activated: true, agent_name: Some(credentials.name) })
}

#[tauri::command]
pub fn restart_agent(app: AppHandle, process: State<'_, AgentProcess>) -> Result<(), String> {
    stop_agent(&process);
    start_agent(&app, &process)
}
