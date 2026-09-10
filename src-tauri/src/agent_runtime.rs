//! Activación del agente desde la app de bandeja.
//!
//! Desde la versión `0.3.0` el agente lo corre un **servicio Windows**
//! (`innoapp-agent-service`, como LocalSystem) — ver el crate `agent-service`.
//! La app de bandeja ya no lanza ningún proceso: solo escribe
//! `credentials.json` en `C:\ProgramData\InnoApp Agent\` al activar. El
//! servicio vigila ese archivo y (re)arranca el agente cuando cambia.

use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

const ACTIVATION_URL: &str = "https://uqa4ti7fwi.execute-api.us-east-1.amazonaws.com/prod/agents/activate";

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

/// Carpeta compartida con el servicio (LocalSystem). El instalador la crea
/// con permiso de modificación para `BUILTIN\Users`, así esta app —que corre
/// sin privilegios— puede escribir `credentials.json`.
fn data_dir() -> PathBuf {
    PathBuf::from(r"C:\ProgramData\InnoApp Agent")
}

fn credentials_path() -> PathBuf {
    data_dir().join("credentials.json")
}

fn load_credentials() -> Result<Option<AgentCredentials>, String> {
    let path = credentials_path();
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&raw).map(Some).map_err(|err| err.to_string())
}

fn write_credentials(credentials: &AgentCredentials) -> Result<(), String> {
    fs::create_dir_all(data_dir()).map_err(|err| err.to_string())?;
    let bytes = serde_json::to_vec_pretty(credentials).map_err(|err| err.to_string())?;
    fs::write(credentials_path(), bytes).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_activation_status() -> Result<ActivationStatus, String> {
    let credentials = load_credentials()?;
    Ok(ActivationStatus {
        activated: credentials.is_some(),
        agent_name: credentials.map(|value| value.name),
    })
}

#[tauri::command]
pub async fn activate_agent(code: String, name: String) -> Result<ActivationStatus, String> {
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

    let credentials: AgentCredentials =
        serde_json::from_str(&body).map_err(|_| "la activación devolvió una respuesta incompleta".to_string())?;
    write_credentials(&credentials)?;
    // El servicio detecta el cambio de `credentials.json` y arranca el agente.

    Ok(ActivationStatus { activated: true, agent_name: Some(credentials.name) })
}

/// "Reintentar" desde la UI cuando el agente no responde: reescribe
/// `credentials.json` (mismo contenido, mtime nuevo) para que el servicio
/// reinicie el agente. No hace nada si todavía no se activó.
#[tauri::command]
pub fn restart_agent() -> Result<(), String> {
    if let Some(credentials) = load_credentials()? {
        write_credentials(&credentials)?;
    }
    Ok(())
}
