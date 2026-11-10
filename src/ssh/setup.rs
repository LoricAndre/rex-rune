//! A one-shot, explicitly authorized native Rex SSH auto-start. Never used by attachment.
use super::{Target, last_line};
use anyhow::{Context, Result, ensure};
use std::{ffi::OsStr, path::PathBuf, process::Stdio, time::Duration};

/// The exact invocation the confirmation previews. No shell and no session creation.
pub fn start_args(target: &Target) -> Result<Vec<String>> {
    ensure!(
        target.socket.is_none(),
        "A custom socket must be started by its owner; edit the endpoint to use a personal server instead"
    );
    Ok([
        "--autostart=true",
        "--server",
        &target.endpoint(),
        "--timeout",
        "20s",
        "api",
        "call",
        "server.status",
        "{}",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect())
}

pub fn command_preview(target: &Target) -> Result<String> {
    Ok(format!(
        "rex {}",
        start_args(target)?
            .iter()
            .map(|arg| format!("'{}'", arg.replace('\'', "'\\''")))
            .collect::<Vec<_>>()
            .join(" ")
    ))
}

fn local_rex() -> PathBuf {
    if std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join("rex").is_file()))
    {
        return "rex".into();
    }
    std::env::var_os("XDG_BIN_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/bin")))
        .map(|dir| dir.join("rex"))
        .filter(|p| p.is_file())
        .unwrap_or_else(|| "rex".into())
}

async fn output(program: &OsStr, args: &[String]) -> Result<String> {
    let output = tokio::time::timeout(Duration::from_secs(45), tokio::process::Command::new(program)
        .args(args).stdin(Stdio::null()).kill_on_drop(true).output()).await
        .context("Setup timed out. It will not be retried; check the host before trying again")?
        .with_context(|| format!("Couldn't run {}. Install Rex preview.14 or newer locally and on the SSH host, then try again", program.to_string_lossy()))?;
    ensure!(
        output.status.success(),
        "{}",
        last_line(&String::from_utf8_lossy(&output.stderr))
            .unwrap_or_else(|| format!("Command refused setup ({})", output.status))
    );
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn require_api12(version: &str, location: &str) -> Result<()> {
    // Local Rex can pretty-print JSON; SSH login banners may precede a compact reply.
    let value: serde_json::Value = serde_json::from_str(version)
        .or_else(|_| {
            serde_json::from_str(
                version
                    .lines()
                    .rfind(|l| !l.trim().is_empty())
                    .unwrap_or(""),
            )
        })
        .with_context(|| {
            format!(
                "{location} Rex didn't report its version; preview.14 (API 12) or newer is required"
            )
        })?;
    ensure!(
        value["api_revision"].as_u64().is_some_and(|n| n >= 12),
        "{location} Rex is too old for this setup. Update to preview.14 (API 12) or newer; nothing was started"
    );
    Ok(())
}

/// Call only after the person confirms. Every invocation authorizes one attempt, never a
/// persistent auto-start setting. SSH config/aliases remain OpenSSH's responsibility.
pub async fn start_personal(endpoint: &str) -> Result<()> {
    let target = Target::parse(endpoint)?.context("Setup requires an ssh:// host")?;
    start_with(&target, local_rex().as_os_str(), OsStr::new("ssh")).await
}

async fn start_with(target: &Target, rex: &OsStr, ssh: &OsStr) -> Result<()> {
    let args = start_args(target)?;
    let local = output(rex, &["version".into(), "--json".into()]).await?;
    require_api12(&local, "Local")?;
    let mut remote = target.options();
    remote.extend([
        "--".into(),
        target.destination(),
        "rex version --json".into(),
    ]);
    let version = output(ssh, &remote).await.context("Couldn't check remote Rex. Install preview.14 or newer there, or check the SSH login; nothing was started")?;
    require_api12(&version, "Remote")?;
    let reply = output(rex, &args)
        .await
        .context("Personal server start failed; no automatic retry")?;
    let _: rexrs::api::server::StatusResponse = serde_json::from_str(&reply)
        .context("Rex didn't confirm the server status. It may have started; use Retry connection before starting again")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn old_missing_and_refusing_rex_never_start_or_retry() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("rex-setup-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let rex = root.join("rex");
        let ssh = root.join("ssh");
        let log = root.join("attempts");
        let script = |path: &std::path::Path, body: &str| {
            std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        };
        script(
            &rex,
            &format!(
                "if [ \"$1\" = version ]; then echo '{{\"api_revision\":12}}'; exit 0; fi\nprintf '%s\\n' \"$@\" >> '{}'; echo 'remote refused to start' >&2; exit 1",
                log.display()
            ),
        );
        let target = Target::parse("ssh://work-alias").unwrap().unwrap();
        for remote in [
            "echo '{\"api_revision\":11}'",
            "echo 'rex: command not found' >&2; exit 127",
            "echo 'SSH login refused' >&2; exit 255",
        ] {
            script(&ssh, remote);
            assert!(
                start_with(&target, rex.as_os_str(), ssh.as_os_str())
                    .await
                    .is_err()
            );
            assert!(!log.exists(), "preflight failed, no native auto-start");
        }
        script(&ssh, "echo '{\"api_revision\":12}'");
        let error = start_with(&target, rex.as_os_str(), ssh.as_os_str())
            .await
            .unwrap_err();
        assert!(format!("{error:#}").contains("remote refused to start"));
        let attempts = std::fs::read_to_string(&log).unwrap();
        assert_eq!(attempts.matches("--autostart=true").count(), 1);
        assert!(attempts.contains("ssh://work-alias"));
        std::fs::remove_file(&log).unwrap();
        script(&rex, "echo '{\"api_revision\":11}'");
        assert!(
            start_with(&target, rex.as_os_str(), ssh.as_os_str())
                .await
                .unwrap_err()
                .to_string()
                .contains("Local Rex is too old")
        );
        assert!(
            start_with(&target, root.join("missing").as_os_str(), ssh.as_os_str())
                .await
                .is_err()
        );
        assert!(!log.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn setup_keeps_alias_user_port_and_ipv6_and_rejects_custom_sockets() {
        for endpoint in [
            "ssh://work-alias",
            "ssh://ops@box:2222",
            "ssh://me@[fd7a::1]:22",
        ] {
            let target = Target::parse(endpoint).unwrap().unwrap();
            let args = start_args(&target).unwrap();
            assert_eq!(
                args,
                [
                    "--autostart=true",
                    "--server",
                    endpoint,
                    "--timeout",
                    "20s",
                    "api",
                    "call",
                    "server.status",
                    "{}"
                ]
            );
            assert!(command_preview(&target).unwrap().contains(endpoint));
        }
        assert!(start_args(&Target::parse("ssh://box/custom.sock").unwrap().unwrap()).is_err());
        for old in [r#"{"api_revision":11}"#, "{}", "old rex"] {
            assert!(require_api12(old, "Remote").is_err());
        }
        require_api12(r#"{"api_revision":12}"#, "Remote").unwrap();
        require_api12("Welcome to the host!\n{\"api_revision\":12}\n\n", "Remote").unwrap();
        require_api12("{\n  \"api_revision\":12\n}", "Local").unwrap();
    }
}
