use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use tracing::warn;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use windows::core::PCWSTR;

pub fn load_secret(key: &str) -> Result<String> {
    let path = secret_path(key)?;
    let mut primary_error = None;
    if path.exists() {
        match read_secret_file(&path) {
            Ok(secret) => return Ok(secret),
            Err(error) => {
                warn!(
                    "failed to read migrated secret `{key}` from `{}`; trying legacy locations: {error:#}",
                    path.display()
                );
                primary_error = Some(error);
            }
        }
    }

    for legacy_path in legacy_secret_paths(key)? {
        if !legacy_path.exists() {
            continue;
        }
        let secret = read_secret_file(&legacy_path)?;
        if !path.exists() || primary_error.is_some() {
            if let Err(error) = store_secret(key, &secret) {
                warn!(
                    "failed to migrate secret `{key}` into new directory; using legacy secret: {error:#}"
                );
            }
        }
        return Ok(secret);
    }

    if let Some(error) = primary_error {
        return Err(error);
    }

    Err(anyhow!(
        "failed to read protected secret: {}",
        path.display()
    ))
}

pub fn store_secret(key: &str, value: &str) -> Result<PathBuf> {
    let path = secret_path(key)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create secret directory: {}", parent.display()))?;
    }
    let encrypted = protect_bytes(value.as_bytes())?;
    fs::write(&path, encrypted)
        .with_context(|| format!("failed to write protected secret: {}", path.display()))?;
    Ok(path)
}

pub fn delete_secret(key: &str) -> Result<()> {
    for path in secret_paths_for_delete(key)? {
        if path.exists() {
            fs::remove_file(&path).with_context(|| {
                format!("failed to remove protected secret: {}", path.display())
            })?;
        }
    }
    Ok(())
}

fn secret_path(key: &str) -> Result<PathBuf> {
    secret_path_with_product_dir(key, "remotty")
}

fn legacy_secret_path(key: &str) -> Result<PathBuf> {
    secret_path_with_product_dir(key, "codex-telegram-bridge")
}

fn legacy_secret_paths(key: &str) -> Result<Vec<PathBuf>> {
    let mut paths = vec![legacy_secret_path(key)?];
    if let Some(legacy_key) = legacy_secret_key_alias(key) {
        paths.push(secret_path_with_product_dir(legacy_key, "remotty")?);
        paths.push(secret_path_with_product_dir(
            legacy_key,
            "codex-telegram-bridge",
        )?);
    }
    Ok(paths)
}

fn secret_paths_for_delete(key: &str) -> Result<Vec<PathBuf>> {
    let mut paths = vec![secret_path(key)?];
    paths.extend(legacy_secret_paths(key)?);
    Ok(paths)
}

fn legacy_secret_key_alias(key: &str) -> Option<&'static str> {
    match key {
        "remotty-telegram-bot" => Some("codex-telegram-bot"),
        _ => None,
    }
}

fn secret_path_with_product_dir(key: &str, product_dir: &str) -> Result<PathBuf> {
    if key.trim().is_empty() {
        return Err(anyhow!("secret key must not be empty"));
    }
    let base = std::env::var("LOCALAPPDATA")
        .context("LOCALAPPDATA is not set; cannot resolve secret store path")?;
    Ok(PathBuf::from(base)
        .join(product_dir)
        .join("secrets")
        .join(format!("{key}.bin")))
}

fn read_secret_file(path: &PathBuf) -> Result<String> {
    let encrypted = fs::read(path)
        .with_context(|| format!("failed to read protected secret: {}", path.display()))?;
    let decrypted = unprotect_bytes(&encrypted)?;
    String::from_utf8(decrypted).context("secret is not valid UTF-8")
}

fn protect_bytes(input: &[u8]) -> Result<Vec<u8>> {
    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptProtectData(
            &in_blob,
            PCWSTR::null(),
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
        .context("CryptProtectData failed")?;
    }

    blob_to_vec_and_free(out_blob)
}

fn unprotect_bytes(input: &[u8]) -> Result<Vec<u8>> {
    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB::default();

    unsafe {
        CryptUnprotectData(
            &in_blob,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut out_blob,
        )
        .context("CryptUnprotectData failed")?;
    }

    blob_to_vec_and_free(out_blob)
}

fn blob_to_vec_and_free(blob: CRYPT_INTEGER_BLOB) -> Result<Vec<u8>> {
    if blob.pbData.is_null() || blob.cbData == 0 {
        return Ok(Vec::new());
    }

    let data = unsafe { std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec() };
    unsafe {
        let _ = LocalFree(Some(HLOCAL(blob.pbData as *mut _)));
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::{
        delete_secret, legacy_secret_path, protect_bytes, secret_path,
        secret_path_with_product_dir, store_secret,
    };
    use anyhow::Result;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn load_secret_migrates_legacy_path() -> Result<()> {
        let temp = tempdir()?;
        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
        }
        let legacy_path = legacy_secret_path("bot")?;
        fs::create_dir_all(legacy_path.parent().expect("legacy path parent"))?;
        fs::write(&legacy_path, protect_bytes(b"legacy-token")?)?;

        let secret = super::load_secret("bot")?;
        assert_eq!(secret, "legacy-token");
        assert!(secret_path("bot")?.exists());

        delete_secret("bot")?;
        Ok(())
    }

    #[test]
    #[serial]
    fn store_secret_uses_new_path() -> Result<()> {
        let temp = tempdir()?;
        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
        }

        let path = store_secret("bot", "new-token")?;
        assert!(path.ends_with("remotty\\secrets\\bot.bin"));

        delete_secret("bot")?;
        Ok(())
    }

    #[test]
    #[serial]
    fn load_secret_uses_legacy_value_when_migration_write_fails() -> Result<()> {
        let temp = tempdir()?;
        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
        }
        fs::write(temp.path().join("remotty"), b"not-a-directory")?;
        let legacy_path = legacy_secret_path("bot")?;
        fs::create_dir_all(legacy_path.parent().expect("legacy path parent"))?;
        fs::write(&legacy_path, protect_bytes(b"legacy-token")?)?;

        let secret = super::load_secret("bot")?;
        assert_eq!(secret, "legacy-token");
        assert!(!secret_path("bot")?.exists());
        Ok(())
    }

    #[test]
    #[serial]
    fn load_secret_migrates_legacy_secret_key_name() -> Result<()> {
        let temp = tempdir()?;
        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
        }
        let legacy_path =
            secret_path_with_product_dir("codex-telegram-bot", "codex-telegram-bridge")?;
        fs::create_dir_all(legacy_path.parent().expect("legacy path parent"))?;
        fs::write(&legacy_path, protect_bytes(b"legacy-token")?)?;

        let secret = super::load_secret("remotty-telegram-bot")?;
        assert_eq!(secret, "legacy-token");
        assert!(secret_path("remotty-telegram-bot")?.exists());

        delete_secret("remotty-telegram-bot")?;
        Ok(())
    }

    #[test]
    #[serial]
    fn delete_secret_removes_legacy_alias_key_name() -> Result<()> {
        let temp = tempdir()?;
        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
        }
        let legacy_path =
            secret_path_with_product_dir("codex-telegram-bot", "codex-telegram-bridge")?;
        fs::create_dir_all(legacy_path.parent().expect("legacy path parent"))?;
        fs::write(&legacy_path, protect_bytes(b"legacy-token")?)?;

        delete_secret("remotty-telegram-bot")?;
        assert!(!legacy_path.exists());
        Ok(())
    }

    #[test]
    #[serial]
    fn load_secret_falls_back_to_legacy_when_new_secret_is_corrupt() -> Result<()> {
        let temp = tempdir()?;
        unsafe {
            std::env::set_var("LOCALAPPDATA", temp.path());
        }
        let new_path = secret_path("remotty-telegram-bot")?;
        fs::create_dir_all(new_path.parent().expect("new path parent"))?;
        fs::write(&new_path, b"corrupt")?;
        let legacy_path =
            secret_path_with_product_dir("codex-telegram-bot", "codex-telegram-bridge")?;
        fs::create_dir_all(legacy_path.parent().expect("legacy path parent"))?;
        fs::write(&legacy_path, protect_bytes(b"legacy-token")?)?;

        let secret = super::load_secret("remotty-telegram-bot")?;
        assert_eq!(secret, "legacy-token");
        let repaired_secret = super::load_secret("remotty-telegram-bot")?;
        assert_eq!(repaired_secret, "legacy-token");
        Ok(())
    }
}
