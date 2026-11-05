//! Apply / rollback de `app.mjs` (Fase 1 OTA). Sin deps de Windows service
//! para poder unit-testear en CI no-Windows.
//!
//! Modelo de confianza: `C:\ProgramData\InnoApp Agent\` es modificable por
//! `BUILTIN\Users`, así que nada que viva ahí se acepta sin más. `updates\`
//! se deja escribible solo por SYSTEM/Administradores (`secure_updates_dir`) y
//! además el servicio verifica el SHA-256 del bundle antes de instalarlo.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use sha2::{Digest, Sha256};

/// Gracia para que el agente nuevo confirme que arrancó bien (primer
/// heartbeat exitoso) antes de que el servicio lo dé por malo. Cubre los
/// reintentos del heartbeat inicial (~110 s en el peor caso).
pub const OTA_BOOT_GRACE: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingApply {
    pub version: String,
    /// SHA-256 (hex minúscula) que debe tener `updates\staging\app.mjs`.
    pub sha256: String,
}

/// Único archivo que el servicio acepta aplicar. El `stagedPath` del marker
/// se ignora a propósito: el marker lo escribe el agente, pero la ruta la
/// decide el servicio.
pub fn staging_path(updates_dir: &Path) -> PathBuf {
    updates_dir.join("staging").join("app.mjs")
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Versión tipo semver (`0.6.1`, `0.6.1-rc.1`): evita comillas o barras en
/// los JSON que este servicio arma a mano.
fn is_safe_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
}

/// Parseo mínimo del JSON que escribe el agent (`pending-apply.json`).
pub fn parse_pending_apply(raw: &str) -> Option<PendingApply> {
    let version = json_string_field(raw, "version")?;
    let sha256 = json_string_field(raw, "sha256")?.to_ascii_lowercase();
    if !is_safe_version(&version) || !is_sha256_hex(&sha256) {
        return None;
    }
    Some(PendingApply { version, sha256 })
}

/// ¿Hay que revertir un update que el agente nuevo todavía no confirmó?
///
/// `boot-check.json` existe desde el swap hasta que el agente nuevo lo borra
/// tras su primer heartbeat exitoso. Sigue ahí si el agente murió (a cualquier
/// hora) o si está vivo pero no confirmó dentro de la gracia (colgado).
pub fn should_rollback(boot_check_exists: bool, child_alive: bool, since_spawn: Duration, grace: Duration) -> bool {
    boot_check_exists && (!child_alive || since_spawn >= grace)
}

/// Deja `updates\` escribible solo por SYSTEM y Administradores.
///
/// Si `updates\` heredara el permiso de Users de `ProgramData\InnoApp Agent`,
/// cualquier usuario local podría plantar un `app.mjs` y un
/// `pending-apply.json` y el servicio (LocalSystem) lo ejecutaría. El agente
/// corre como SYSTEM, así que no pierde acceso. Falla cerrado: si no se puede,
/// el llamador no debe aplicar updates.
pub fn secure_updates_dir(updates_dir: &Path) -> Result<(), String> {
    wipe_stale(updates_dir);
    fs::create_dir_all(updates_dir).map_err(|e| format!("crear updates: {e}"))?;
    lock_down(updates_dir)
}

/// Restos de antes del candado: nada plantado por un usuario sobrevive.
/// `boot-check.json` se conserva (solo puede provocar un rollback a un
/// `.prev` que vive en el directorio del runtime, no escribible por Users).
fn wipe_stale(updates_dir: &Path) {
    let _ = fs::remove_file(updates_dir.join("pending-apply.json"));
    let _ = fs::remove_file(updates_dir.join("app.mjs.prev"));
    let _ = fs::remove_dir_all(updates_dir.join("staging"));
}

#[cfg(windows)]
fn lock_down(dir: &Path) -> Result<(), String> {
    use std::{ffi::OsStr, process::Command};

    let run = |args: &[&OsStr]| -> Result<(), String> {
        let out = Command::new("icacls")
            .args(args)
            .output()
            .map_err(|e| format!("no se pudo ejecutar icacls: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "icacls salió con {:?}: {}",
                out.status.code(),
                String::from_utf8_lossy(&out.stdout).trim()
            ))
        }
    };
    let d = dir.as_os_str();
    run(&[d, OsStr::new("/inheritance:r")])?;
    run(&[
        d,
        OsStr::new("/grant:r"),
        OsStr::new("*S-1-5-18:(OI)(CI)F"),
        OsStr::new("*S-1-5-32-544:(OI)(CI)F"),
    ])?;
    // Borra ACEs explícitas en lo que ya hubiera adentro (p. ej. el
    // `icacls /grant Users:M /T` del instalador) y re-hereda del directorio.
    let mut children = d.to_os_string();
    children.push("\\*");
    run(&[children.as_os_str(), OsStr::new("/reset"), OsStr::new("/T"), OsStr::new("/C")])
}

#[cfg(not(windows))]
fn lock_down(_dir: &Path) -> Result<(), String> {
    Ok(())
}

/// Hora para los markers. Segundos Unix con prefijo `t+` (no es ISO-8601).
fn now_marker() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("t+{}", d.as_secs()),
        Err(_) => "t+0".to_string(),
    }
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

pub fn write_json(path: &Path, body: &str) {
    let _ = fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")));
    let tmp = path.with_extension("json.tmp");
    if fs::write(&tmp, body).is_ok() {
        let _ = fs::rename(&tmp, path);
    }
}

/// Verifica el hash de `updates\staging\app.mjs`, copia `app.mjs` actual →
/// `.prev`, instala el staging y escribe boot-check.
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

    // Se lee una sola vez y se escriben esos mismos bytes: lo que se verifica
    // es exactamente lo que queda instalado (sin ventana entre check y copia).
    let staged = staging_path(updates_dir);
    let staged_bytes = fs::read(&staged).map_err(|e| format!("leer staging {}: {e}", staged.display()))?;
    let actual = sha256_hex(&staged_bytes);
    if actual != pending.sha256 {
        return Err(format!(
            "sha256 del staging no coincide (esperado {}, actual {actual})",
            pending.sha256
        ));
    }

    if runtime_app.is_file() {
        fs::copy(&runtime_app, &runtime_prev).map_err(|e| format!("backup runtime .prev: {e}"))?;
        fs::copy(&runtime_app, &updates_prev).map_err(|e| format!("backup updates .prev: {e}"))?;
    }
    let tmp = runtime.join("app.mjs.ota-new");
    fs::write(&tmp, &staged_bytes).map_err(|e| format!("escribir tmp: {e}"))?;
    fs::rename(&tmp, &runtime_app).map_err(|e| format!("swap app.mjs: {e}"))?;

    let boot = format!(
        "{{\n  \"version\": \"{}\",\n  \"appliedAt\": \"{}\"\n}}\n",
        pending.version.replace('"', ""),
        now_marker()
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
        now_marker()
    );
    write_json(&rolled_back, &marker);
    let _ = fs::remove_file(boot_check);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const HASH: &str = "11d3b4d1b2e0d4a1d1f4c3a0e5b7a9c2d4e6f8a0b1c3d5e7f9a1b3c5d7e9f0a2";

    fn tmp_root(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("ota-svc-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn parses_pending_apply_marker_and_ignores_staged_path() {
        let raw = format!(
            "{{\n  \"version\": \"0.5.1\",\n  \"sha256\": \"{HASH}\",\n  \"stagedPath\": \"C:\\\\Windows\\\\evil.mjs\"\n}}"
        );
        let pending = parse_pending_apply(&raw).expect("parse");
        assert_eq!(pending.version, "0.5.1");
        assert_eq!(pending.sha256, HASH);
    }

    #[test]
    fn rejects_marker_without_valid_sha256_or_version() {
        assert!(parse_pending_apply(r#"{"version":"0.5.1","stagedPath":"x"}"#).is_none());
        assert!(parse_pending_apply(r#"{"version":"0.5.1","sha256":"abc"}"#).is_none());
        let slash = format!(r#"{{"version":"..\\x","sha256":"{HASH}"}}"#);
        assert!(parse_pending_apply(&slash).is_none());
        let empty = format!(r#"{{"version":"","sha256":"{HASH}"}}"#);
        assert!(parse_pending_apply(&empty).is_none());
    }

    #[test]
    fn apply_and_rollback_roundtrip() {
        let root = tmp_root("rt");
        let runtime = root.join("runtime");
        let updates = root.join("updates");
        fs::create_dir_all(updates.join("staging")).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("app.mjs"), b"old").unwrap();
        fs::write(staging_path(&updates), b"new").unwrap();

        let pending = PendingApply { version: "0.5.1".into(), sha256: sha256_hex(b"new") };
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

    #[test]
    fn apply_refuses_staging_that_does_not_match_the_marker_hash() {
        let root = tmp_root("bad");
        let runtime = root.join("runtime");
        let updates = root.join("updates");
        fs::create_dir_all(updates.join("staging")).unwrap();
        fs::create_dir_all(&runtime).unwrap();
        fs::write(runtime.join("app.mjs"), b"old").unwrap();
        fs::write(staging_path(&updates), b"evil").unwrap();

        // Hash del contenido legítimo, archivo sustituido.
        let pending = PendingApply { version: "0.5.1".into(), sha256: sha256_hex(b"new") };
        let err = apply_pending_update(&runtime, &updates, &pending).unwrap_err();
        assert!(err.contains("sha256"), "{err}");
        assert_eq!(fs::read(runtime.join("app.mjs")).unwrap(), b"old");
        assert!(!runtime.join("app.mjs.prev").exists());
        assert!(!updates.join("boot-check.json").exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn apply_only_reads_the_canonical_staging_file() {
        let root = tmp_root("canon");
        let runtime = root.join("runtime");
        let updates = root.join("updates");
        fs::create_dir_all(&runtime).unwrap();
        fs::create_dir_all(&updates).unwrap();
        fs::write(runtime.join("app.mjs"), b"old").unwrap();
        // Archivo con hash correcto pero fuera de staging\: no se usa.
        fs::write(root.join("elsewhere.mjs"), b"new").unwrap();

        let pending = PendingApply { version: "0.5.1".into(), sha256: sha256_hex(b"new") };
        assert!(apply_pending_update(&runtime, &updates, &pending).is_err());
        assert_eq!(fs::read(runtime.join("app.mjs")).unwrap(), b"old");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn rollback_decision_covers_crash_after_start_and_hang() {
        let g = OTA_BOOT_GRACE;
        let s = Duration::from_secs;
        // Confirmado (sin boot-check): nunca se revierte, ni muerto ni viejo.
        assert!(!should_rollback(false, false, s(5), g));
        assert!(!should_rollback(false, true, s(9999), g));
        // Murió sin confirmar, aunque haya pasado mucho desde el spawn.
        assert!(should_rollback(true, false, s(5), g));
        assert!(should_rollback(true, false, s(600), g));
        // Vivo pero sin confirmar: se espera la gracia, luego se revierte.
        assert!(!should_rollback(true, true, s(60), g));
        assert!(should_rollback(true, true, g, g));
    }

    #[test]
    fn wipe_stale_removes_planted_files_but_keeps_boot_check() {
        let root = tmp_root("wipe");
        let updates = root.join("updates");
        fs::create_dir_all(updates.join("staging")).unwrap();
        fs::write(updates.join("pending-apply.json"), b"{}").unwrap();
        fs::write(updates.join("app.mjs.prev"), b"planted").unwrap();
        fs::write(updates.join("staging").join("app.mjs"), b"planted").unwrap();
        fs::write(updates.join("boot-check.json"), b"{}").unwrap();

        wipe_stale(&updates);
        assert!(!updates.join("pending-apply.json").exists());
        assert!(!updates.join("app.mjs.prev").exists());
        assert!(!updates.join("staging").exists());
        assert!(updates.join("boot-check.json").exists());

        let _ = fs::remove_dir_all(&root);
    }

    /// En Windows ejecuta icacls de verdad. Un test sin elevar no puede leer
    /// el directorio después (el candado deja solo SYSTEM/Administradores y
    /// el token filtrado no usa el grupo), así que solo se comprueba que
    /// termine bien y se restaura la herencia para limpiar.
    #[test]
    fn secure_updates_dir_succeeds_on_a_fresh_directory() {
        let root = tmp_root("lock");
        let updates = root.join("updates");
        secure_updates_dir(&updates).unwrap();
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("icacls").arg(&root).args(["/reset", "/T", "/C"]).output();
        }
        let _ = fs::remove_dir_all(&root);
    }
}
