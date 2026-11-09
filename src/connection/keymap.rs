//! Best-effort layered keymap registration. Never changes host connectivity.
use super::{Client, Shared};
use crate::keymap::{Overrides, Resolved};
use rexrs::api::client::{Keymap, KeymapResponse};
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct Declaration {
    pub generation: u64,
    pub request: Keymap,
    pub overrides: Overrides,
}
#[derive(Clone)]
pub struct Snapshot {
    pub generation: u64,
    pub reply: KeymapResponse,
    pub resolved: Resolved,
}

/// A config load counter alone is not a change. A restarted instance always is.
pub(super) fn changed(snapshot: Option<&Snapshot>, event: &serde_json::Value) -> bool {
    let Some(snapshot) = snapshot else {
        return true;
    };
    event["hash"]
        .as_str()
        .is_none_or(|hash| hash != snapshot.reply.config.hash)
        || event["instance_id"]
            .as_str()
            .is_some_and(|id| id != snapshot.reply.config.instance_id)
}

pub(super) async fn refresh(control: &mut Client, shared: &Shared, declaration: &Declaration) {
    let result = tokio::time::timeout(
        Duration::from_secs(8),
        control.call(declaration.request.clone()),
    )
    .await;
    let result = match result {
        Ok(Ok(reply)) => Ok(reply),
        Ok(Err(error)) => Err(error.to_string()),
        Err(error) => Err(error.to_string()),
    };
    match result {
        Ok(reply) => {
            let resolved = Resolved::from_server(&reply, &declaration.overrides);
            shared.update(|m| {
                m.keymap = Some(Arc::new(Snapshot {
                    generation: declaration.generation,
                    reply,
                    resolved,
                }));
                m.keymap_error = None;
                m.keymap_revision += 1;
            });
        }
        Err(error) => {
            let message = format!("client.keymap failed; using local shortcuts: {error}");
            eprintln!("warning: {message}");
            shared.update(|m| {
                m.keymap = None;
                m.keymap_error = Some(message);
                m.keymap_revision += 1;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn refresh_checks_hash_and_instance_not_load_counter() {
        let mut snapshot = Snapshot {
            generation: 0,
            reply: KeymapResponse::default(),
            resolved: Resolved::default(),
        };
        snapshot.reply.config.hash = "hash".into();
        snapshot.reply.config.instance_id = "instance".into();
        assert!(!changed(
            Some(&snapshot),
            &json!({"hash":"hash","generation":999})
        ));
        assert!(changed(Some(&snapshot), &json!({"hash":"new"})));
        assert!(changed(
            Some(&snapshot),
            &json!({"hash":"hash","instance_id":"new"})
        ));
        assert!(changed(None, &json!({"hash":"hash"})));
    }
}
