//! Per-host consent and the deliberately small view-only remote surface.
use super::{
    actions::{self, Remote},
    workspace::Workspace,
};
use rex_linux::connection::{
    HostCommand,
    perform::{MAX_WAIT, Outcome, Pending},
};
use rune::Tx;
use std::time::Duration;

const TYPING: Duration = Duration::from_millis(1500);

impl Workspace {
    pub fn toggle_remote_control(&mut self, host: usize) {
        self.settings.hosts[host].remote_control = !self.settings.hosts[host].remote_control;
        self.save();
    }

    fn remote_policy(&self, host: usize, pending: &Pending) -> Result<(), Outcome> {
        let request = &pending.request;
        let Some(spec) = actions::find(&request.action) else {
            return Err(Outcome::error(
                "not_supported",
                "This remote action is not supported by Rex Linux; nothing was done.",
            ));
        };
        let Remote::View { navigation } = spec.remote() else {
            let Remote::Never(message) = spec.remote() else {
                unreachable!()
            };
            return Err(Outcome::error(
                "not_supported",
                format!(
                    "{} is not available remotely. {message} Nothing was done.",
                    request.action
                ),
            ));
        };
        if !self.settings.hosts[host].remote_control {
            return Err(Outcome::error(
                "refused",
                "Remote control is off for this host in Rex Linux. The person at that computer can turn it on in Settings → Remote Hosts. Nothing was done.",
            ));
        }
        if !spec.accepts(&request.args) {
            return Err(Outcome::error(
                "bad_request",
                format!(
                    "Invalid arguments for {}; nothing was done.",
                    request.action
                ),
            ));
        }
        let refused = if self.panel.is_some() {
            Some("A dialog or popup is open")
        } else if self.active != host {
            Some("This host is not on screen")
        } else if request
            .session_id
            .as_ref()
            .is_some_and(|id| self.model().selected.as_ref() != Some(id))
        {
            Some("The requested session is not on screen")
        } else if request
            .block_id
            .as_deref()
            .is_some_and(|id| self.model().focused() != Some(id))
        {
            Some("The requested pane is not focused")
        } else if navigation && self.last_keypress.is_some_and(|at| at.elapsed() < TYPING) {
            Some("The person is typing; try again after a pause")
        } else if self.navigation.pending.is_some() {
            Some("Another navigation is still in progress")
        } else {
            None
        };
        if let Some(reason) = refused {
            return Err(Outcome::error(
                "refused",
                format!("{reason}; nothing was done."),
            ));
        }
        if !self.hosts[host].model.connected || pending.received.elapsed() >= MAX_WAIT {
            return Err(Outcome::error(
                "internal",
                "Rex Linux did not act in time or the host disconnected; nothing was done.",
            ));
        }
        if request.action != "client.sidebar.toggle" && self.model().window().is_none() {
            return Err(Outcome::error(
                "internal",
                "No tab is on screen; nothing was done.",
            ));
        }
        if request.action == "client.tab.goto" {
            let index = request.args["index"].as_u64().unwrap() as usize;
            if self.model().view().is_none_or(|v| index > v.windows.len()) {
                return Err(Outcome::error(
                    "bad_request",
                    "That tab index is out of range; nothing was done.",
                ));
            }
        }
        if request.action == "pane.scroll" && self.active_terminal().is_none() {
            return Err(Outcome::error(
                "internal",
                "No terminal is on screen; nothing was done.",
            ));
        }
        Ok(())
    }

    pub(super) fn remote_perform(&mut self, host: usize, pending: Pending, tx: &mut Tx) {
        // A sync wake may outlive its socket. Do not execute or answer it on the replacement.
        if pending.epoch != self.hosts[host].model.connection_epoch {
            return;
        }
        let outcome = match self.remote_policy(host, &pending) {
            Err(outcome) => {
                if !self.settings.hosts[host].remote_control
                    && matches!(
                        actions::find(&pending.request.action).map(|s| s.remote()),
                        Some(Remote::View { .. })
                    )
                    && self.remote_off_notices.insert(self.hosts[host].id)
                {
                    self.inform(format!(
                        "{} on {} asked for {}. Remote control is off (Settings → Remote Hosts).",
                        pending.request.requester,
                        self.settings.hosts[host].label,
                        pending.request.action
                    ));
                }
                outcome
            }
            Ok(()) => {
                self.bindings.sequence.clear();
                if actions::run(self, &pending.request.action, &pending.request.args, tx) {
                    Outcome::Performed
                } else {
                    Outcome::error("internal", "The action could not run; nothing was done.")
                }
            }
        };
        self.remote_reply(host, pending, outcome);
    }

    pub(super) fn remote_reply(&mut self, host: usize, pending: Pending, outcome: Outcome) {
        log::info!(
            "Remote control on {}: {} requested {}: {:?}",
            self.settings.hosts[host].label,
            pending.request.requester,
            pending.request.action,
            outcome
        );
        if outcome == Outcome::Performed {
            self.inform(format!(
                "{}: {} performed {}",
                self.settings.hosts[host].label, pending.request.requester, pending.request.action
            ));
        }
        self.send(
            host,
            HostCommand::PerformReply {
                epoch: pending.epoch,
                call_id: pending.request.call_id,
                outcome,
            },
        );
    }
}
