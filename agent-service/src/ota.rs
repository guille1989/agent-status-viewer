//! Apply / rollback de `app.mjs` (Fase 1 OTA). Sin deps de Windows service
//! para poder unit-testear en CI no-Windows.

use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingApply {
    pub version: String,
    pub staged_path: PathBuf,
}

/// Parseo mínimo del JSON que escribe el agent (`pending-apply.json`).
pub fn parse_pending_apply(raw: &str) -> Option<PendingApply> {
    let version = json_string_field(raw, "version")?;
    let staged = json_string_field(raw, "stagedPath")?;
    if version.is_empty() || staged.is_empty() {
        return None;
    }
    Some(PendingApply {
        version,
        staged_path: PathBuf::from(staged),
    })
}

pub fn json_string_field(raw: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let idx = raw.find(&needle)?;
    let after_key = &raw[idx + needle.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = after_colon[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => break,
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

fn now_iso() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("t+{}", d.as_secs()),
        Err(_) => "t+0".to_string(),
    }
}

pub fn write_json(path: &Path, body: &str) {
    let _ = fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")));
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, body).is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}

/// Copia `app.mjs` actual → `.prev`, staging → `app.mjs`, escribe boot-check.
pub fn apply_pending_update(
    runtime: &Path,
    updates_dir: &Path,
    pending: &PendingApply,
) -> Result<(), String> {
    let runtime_app = runtime.join("app.mjs");
    let runtime_prev = runtime.join("app.mjs.prev");
    let updates_prev = updates_dir.join("app.mjs.prev");
    let pending_apply = updates_dir.join("pending-apply.json");
    let boot_check = updates_dir.join("boot-check.json");
    let _ = fs::create_dir_all(updates_dir);

    if !pending.staged_path.is_file() {
        return Err(format!("staging ausente: {}", pending.staged_path.display()));
    }
    if runtime_app.is_file() {
        fs::copy(&runtime_app, &runtime_prev).map_err(|e| format!("backup runtime .prev: {e}"))?;
        fs::copy(&runtime_app, &updates_prev).map_err(|e| format!("backup updates .prev: {e}"))?;
    }
    let staged_bytes = fs::read(&pending.staged_path).map_err(|e| format!("leer staging: {e}"))?;
    let tmp = runtime.join("app.mjs.ota-new");
    fs::write(&tmp, &staged_bytes).map_err(|e| format!("escribir tmp: {e}"))?;
    fs::rename(&tmp, &runtime_app).map_err(|e| format!("swap app.mjs: {e}"))?;

    let boot = format!(
        "{{\n  \"version\": \"{}\",\n  \"appliedAt\": \"{}\"\n}}\n",
        pending.version.replace('"', ""),
        now_iso()
    );
    write_json(&boot_check, &boot);
    let _ = fs::remove_file(pending_apply);
    Ok(())
}

pub fn rollback_from_prev(runtime: &Path, updates_dir: &Path, reason: &str) {
    let runtime_app = runtime.join("app.mjs");
    let runtime_prev = runtime.join("app.mjs.prev");
    let updates_prev = updates_dir.join("app.mjs.prev");
    let boot_check = updates_dir.join("boot-check.json");
    let rolled_back = updates_dir.join("rolled-back.json");

    let src = if runtime_prev.is_file() {
        runtime_prev
    } else if updates_prev.is_file() {
        updates_prev
    } else {
        let _ = fs::remove_file(&boot_check);
        return;
    };
    let _ = fs::copy(&src, &runtime_app);
    let from_version = fs::read_to_string(&boot_check)
        .ok()
        .and_then(|raw| json_string_field(&raw, "version"))
        .unwrap_or_else(|| "unknown".into());
    let marker = format!(
        "{{\n  \"fromVersion\": \"{}\",\n  \"toVersion\": null,\n  \"reason\": \"{}\",\n  \"at\": \"{}\"\n}}\n",
        from_version.replace('"', ""),
        reason.replace('"', "'"),
        now_iso()
    );
    write_json(&rolled_back, &marker);
    let _ = fs::remove_file(boot_check);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_pending_apply_marker() {
        let raw = r#"{
  "version": "0.5.1",
  "sha256": "abc",
  "stagedPath": "C:\\ProgramData\\InnoApp Agent\\updates\\staging\\app.mjs",
  "requestedAt": "2026-11-05T00:00:00.000Z"
}"#;
        let pending = parse_pending_apply(raw).expect("parse");
        assert_eq!(pending.version, "0.5.1");
        assert!(pending.staged_path.to_string_lossy().contains("staging"));
    }

    #[test]
    fn apply_and_rollback_roundtrip() {
        let root = std::env::temp_dir().join(format!("ota-svc-{}", std::process::id()));
        let runtime = root.join("runtime");
        let updates = root.join("updates");
        let staging = updates.join("staging");
        fs::create_dir_all(&staging).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("app.mjs"), b"old").unwrap();
        fs::write(staging.join("app.mjs"), b"new").unwrap();

        let pending = PendingApply {
            version: "0.5.1".into(),
            staged_path: staging.join("app.mjs"),
        };
        apply_pending_update(&runtime, &updates, &pending).unwrap();
        assert_eq!(fs::read(runtime.join("app.mjs")).unwrap(), b"new");
        assert_eq!(fs::read(runtime.join("app.mjs.prev")).unwrap(), b"old");
        assert!(updates.join("boot-check.json").is_file());

        rollback_from_prev(&runtime, &updates, "test");
        assert_eq!(fs::read(runtime.join("app.mjs")).unwrap(), b"old");
        assert!(updates.join("rolled-back.json").is_file());
        assert!(!updates.join("boot-check.json").is_file());

        let _ = fs::remove_dir_all(&root);
    }
}
