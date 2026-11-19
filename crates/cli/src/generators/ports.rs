//! The ports a project listens on in development, read from its `.env`, and a warning for each
//! one another process already holds (Fase 27.1): two projects on the same machine clash.

use std::fs;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::path::Path;

/// The `.env` variables `puerto nuevo` checks; one that the project lacks is skipped.
pub const PROJECT_PORTS: &[&str] = &["DB_HOST_PORT", "SERVICE_PORT", "WORKER_PORT"];
/// The ones the first mail adds (Mailpit).
pub const MAIL_PORTS: &[&str] = &["SMTP_HOST_PORT", "MAILPIT_UI_PORT"];
/// Vite's port: fixed in `web/vite.config.ts`, which takes the next free one by itself.
pub const WEB_PORT: u16 = 5173;

/// True when something already listens on `port` (IPv4 or IPv6, any address).
pub fn is_taken(port: u16) -> bool {
    ["0.0.0.0", "::"].into_iter().any(|host| {
        matches!(TcpListener::bind((host, port)), Err(e) if e.kind() == ErrorKind::AddrInUse)
    })
}

fn env_port(env: &str, key: &str) -> Option<u16> {
    env.lines().find_map(|line| {
        line.strip_prefix(key)?
            .strip_prefix('=')?
            .trim()
            .parse()
            .ok()
    })
}

/// One warning per variable of `keys` whose port in `.env` is taken, naming what to change.
pub fn taken(base: &Path, keys: &[&str]) -> Vec<String> {
    let env = fs::read_to_string(base.join(".env")).unwrap_or_default();
    keys.iter()
        .filter_map(|&key| {
            let port = env_port(&env, key)?;
            // The URLs repeat the port: they are read by tools that do not expand variables.
            let url = match key {
                "DB_HOST_PORT" => " y el puerto de DATABASE_URL",
                "SMTP_HOST_PORT" => " y el puerto de SMTP_URL",
                _ => "",
            };
            is_taken(port)
                .then(|| format!("aviso: el puerto {port} está ocupado; cambia {key}{url} en .env"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project_with_env(name: &str, env: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("puerto_ports_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".env"), env).unwrap();
        dir
    }

    fn free_port() -> u16 {
        TcpListener::bind(("0.0.0.0", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[test]
    fn should_warn_about_the_taken_port_and_not_the_free_one() {
        let busy = TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let busy_port = busy.local_addr().unwrap().port();
        let free = free_port();
        let dir = project_with_env(
            "taken",
            &format!("SERVICE_PORT={busy_port}\nWORKER_PORT={free}\n"),
        );

        let warnings = taken(&dir, PROJECT_PORTS);

        assert_eq!(
            warnings,
            [format!(
                "aviso: el puerto {busy_port} está ocupado; cambia SERVICE_PORT en .env"
            )]
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn should_name_the_url_that_repeats_the_port() {
        let busy = TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let port = busy.local_addr().unwrap().port();
        let dir = project_with_env(
            "url",
            &format!("DB_HOST_PORT={port}\nSMTP_HOST_PORT={port}\n"),
        );

        let warnings = taken(&dir, &["DB_HOST_PORT", "SMTP_HOST_PORT"]);

        assert!(warnings[0].ends_with("cambia DB_HOST_PORT y el puerto de DATABASE_URL en .env"));
        assert!(warnings[1].ends_with("cambia SMTP_HOST_PORT y el puerto de SMTP_URL en .env"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn should_skip_a_variable_the_env_lacks_or_a_prefix_of_another() {
        let busy = TcpListener::bind(("0.0.0.0", 0)).unwrap();
        let port = busy.local_addr().unwrap().port();
        let dir = project_with_env("missing", &format!("SERVICE_PORT_X={port}\n"));

        assert!(taken(&dir, PROJECT_PORTS).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
