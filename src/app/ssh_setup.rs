//! Explicit personal-server setup, separate from automatic attachment and reconnect.
use super::{panel::PanelKind, workspace::Workspace};
use rex_linux::{
    connection::HostCommand,
    ssh::{Target, setup},
};
use rune::Tx;

#[derive(Default)]
pub struct Setup {
    pub running: Option<String>,
    /// One-shot starts headless tests would have run.
    pub ran: Vec<String>,
}

impl Workspace {
    pub fn can_start_ssh(&self, host: usize) -> bool {
        self.settings
            .hosts
            .get(host)
            .zip(self.hosts.get(host))
            .is_some_and(|(saved, state)| {
                !state.model.connected
                    && state.model.ssh_server_missing
                    && Target::parse(&saved.endpoint)
                        .ok()
                        .flatten()
                        .is_some_and(|t| t.socket.is_none())
            })
    }

    pub fn preview_ssh_start(&mut self, host: usize) {
        if !self.can_start_ssh(host) {
            return;
        }
        let endpoint = self.settings.hosts[host].endpoint.clone();
        let label = self.settings.hosts[host].label.clone();
        self.open_panel(
            PanelKind::StartSsh { endpoint },
            &format!("Start a personal server on {label}?"),
            &[],
        );
    }

    pub fn start_ssh(&mut self, endpoint: String) {
        if self.ssh_setup.running.is_some()
            || !self.panel.as_ref().is_some_and(
                |p| matches!(&p.kind, PanelKind::StartSsh { endpoint: e } if e == &endpoint),
            )
        {
            return;
        }
        // The preview captures the endpoint. An edit/removal or successful attachment must
        // not turn a stale confirmation into a start on a different host.
        if !self
            .settings
            .hosts
            .iter()
            .enumerate()
            .any(|(i, h)| h.endpoint == endpoint && self.can_start_ssh(i))
        {
            if let Some(panel) = &mut self.panel {
                panel.error = Some("This host changed or already connected. Close this dialog and check its connection.".into());
            }
            return;
        }
        self.ssh_setup.running = Some(endpoint.clone());
        if let Some(panel) = &mut self.panel {
            panel.error = None;
        }
        if cfg!(test) {
            self.ssh_setup.ran.push(endpoint);
            return;
        }
        let handle = self.handle.clone();
        let task_endpoint = endpoint.clone();
        let spawned = std::thread::Builder::new()
            .name("rex-ssh-setup".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(anyhow::Error::from)
                    .and_then(|runtime| runtime.block_on(setup::start_personal(&task_endpoint)))
                    .map_err(|e| format!("{e:#}"));
                handle.update(move |s, tx| s.ssh_start_finished(&task_endpoint, result, tx));
            });
        if let Err(error) = spawned {
            self.ssh_setup.running = None;
            if let Some(panel) = &mut self.panel {
                panel.error = Some(format!("Couldn't start setup: {error}"));
            }
        }
    }

    pub fn ssh_start_finished(&mut self, endpoint: &str, result: Result<(), String>, tx: &mut Tx) {
        if self.ssh_setup.running.as_deref() != Some(endpoint) {
            return;
        }
        self.ssh_setup.running = None;
        let showing = self.panel.as_ref().is_some_and(
            |p| matches!(&p.kind, PanelKind::StartSsh { endpoint: e } if e == endpoint),
        );
        match result {
            Ok(()) => {
                if showing {
                    self.close_panel(tx);
                }
                if let Some(host) = self
                    .settings
                    .hosts
                    .iter()
                    .position(|h| h.endpoint == endpoint)
                {
                    self.send(host, HostCommand::Reconnect);
                }
                self.inform(format!("Personal server answered on {endpoint}. Connecting through SSH; no session was created."));
            }
            Err(error) => {
                let message = format!(
                    "{error}\nNo automatic retry. Check the host or use Retry connection before trying setup again."
                );
                if showing {
                    self.panel.as_mut().unwrap().error = Some(message);
                } else {
                    self.notify(message);
                }
            }
        }
    }
}

pub fn dialog(s: &Workspace, endpoint: &str) -> rune::El {
    use super::{
        theme::*,
        widgets::{Pill, pill},
    };
    use rune::{col, row, text};
    let panel = s.panel.as_ref().expect("setup panel");
    let busy = s.ssh_setup.running.is_some();
    let target = Target::parse(endpoint).ok().flatten();
    let command = target
        .as_ref()
        .and_then(|t| setup::command_preview(t).ok())
        .unwrap_or_default();
    let mut body = col().gap(12.)
        .child(text(panel.title.clone()).font(strong(17.)).color(rgb(ink())).wrap())
        .child(text(format!("{endpoint}\n\nUses your SSH configuration and the Rex executable already installed on both machines (preview.14 or newer). Starts a personal server as the SSH login user only if none is running. No packages, system services or sessions are created. The server may create its own user config/state files and keeps running when this app closes.\n\nThis is one attempt, not permission to auto-start on reconnect. SSH must already work without a password prompt."))
            .font(ui(12.)).color(rgb(muted())).wrap())
        .child(text(command).font(ui(12.)).color(rgb(ink())).wrap());
    if let Some(running) = &s.ssh_setup.running {
        body = body.child(
            text(format!(
                "Starting {running}… Closing this dialog does not cancel the requested start."
            ))
            .font(ui(12.))
            .color(rgb(muted()))
            .wrap(),
        );
    }
    if let Some(error) = &panel.error {
        body = body.child(
            col()
                .max_h(140.)
                .scroll(rune::ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    text(error.clone())
                        .font(ui(12.))
                        .color(rgb(danger()))
                        .wrap(),
                ),
        );
    }
    let start = pill(
        "confirm-ssh-start",
        "Start personal server",
        Pill::Primary,
        32.,
        |s, tx| s.submit(tx),
    )
    .tab_index(2);
    let start = if busy {
        start.opacity(0.45).inert()
    } else {
        start
    };
    body.child(
        row()
            .gap(8.)
            .justify_end()
            .child(
                pill(
                    "cancel-ssh-start",
                    if busy { "Close" } else { "Not now" },
                    Pill::Neutral,
                    32.,
                    |s, tx| s.cancel_panel(tx),
                )
                .default_button()
                .tab_index(1),
            )
            .child(start),
    )
}
