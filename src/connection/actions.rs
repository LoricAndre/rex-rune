//! client.action has a server-side 30-second budget, unlike ordinary control requests.
use super::{Client, Shared};
use anyhow::{Context, Result};
use rexrs::api::client::{Action, ClientAction};
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(35);
#[derive(Clone)]
pub struct Queued {
    pub generation: u64,
    pub session: Option<String>,
    pub block: Option<String>,
    pub action: ClientAction,
}

pub(super) async fn run(
    control: &mut Client,
    shared: &Shared,
    generation: u64,
    request: Action,
) -> Result<()> {
    let session = request.session_id.clone();
    let block = request.block_id.clone();
    // This worker's serial control loop queues later input behind the action. Never replay
    // a timed-out action: it may already have changed server state.
    let reply = match tokio::time::timeout(TIMEOUT, control.call(request))
        .await
        .context("Rex client.action timed out after 35 seconds; action was not retried")?
    {
        Ok(reply) => reply,
        Err(error @ (rexrs::Error::Json(_) | rexrs::Error::RequestTooLarge { .. })) => {
            // Request sizing and typed reply decoding leave the transport usable.
            shared.update(|m| m.notice = Some(format!("Rex client.action failed: {error}")));
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    if !reply.client_actions.is_empty() {
        shared.update(|m| {
            m.client_actions
                .extend(reply.client_actions.into_iter().map(|action| Queued {
                    generation,
                    session: session.clone(),
                    block: block.clone(),
                    action,
                }))
        });
    }
    Ok(())
}
