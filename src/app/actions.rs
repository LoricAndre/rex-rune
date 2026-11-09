//! Named workspace actions shared by keyboard, pointer and Commands dispatch.
//! The registry structure follows Andrea Zito (nivox), commit 8037d78.
use super::{panel::Intent, workspace::Workspace};
use rex_linux::connection::Command;
use rune::Tx;
use serde_json::{Value, json};

pub const SEARCH: &str = "client.search";
pub const NEXT_GLOBAL: &str = "client.tab.next_global";
pub const PREVIOUS_GLOBAL: &str = "client.tab.previous_global";
pub const HISTORY_BACK: &str = "client.history.back";
pub const HISTORY_FORWARD: &str = "client.history.forward";

#[derive(Clone, Copy)]
enum Handler {
    Palette,
    Settings,
    Sidebar,
    Close,
    Search,
    NewTab,
    NextTab,
    PreviousTab,
    NextGlobal,
    PreviousGlobal,
    Back,
    Forward,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    SwitchSession,
    NewSession,
    Split,
    ZoomPane,
    FocusPane,
    PaneActions,
    Copy,
    Paste,
    Reconnect,
    Scroll,
    GotoTab,
    CloseTab,
    RenameTab,
    RenameSession,
    ClosePane,
    SendKey,
}

pub struct Spec {
    pub name: &'static str,
    handler: Handler,
}

macro_rules! specs {
    ($($name:expr => $handler:ident),* $(,)?) => {
        pub static SPECS: &[Spec] = &[$(Spec {name: $name, handler: Handler::$handler}),*];
    };
}

specs! {
    "client.palette.toggle" => Palette,
    "client.settings" => Settings,
    "client.sidebar.toggle" => Sidebar,
    "client.close" => Close,
    SEARCH => Search,
    "client.tab.new" => NewTab,
    "client.tab.next" => NextTab,
    "client.tab.previous" => PreviousTab,
    NEXT_GLOBAL => NextGlobal,
    PREVIOUS_GLOBAL => PreviousGlobal,
    HISTORY_BACK => Back,
    HISTORY_FORWARD => Forward,
    "client.zoom.in" => ZoomIn,
    "client.zoom.out" => ZoomOut,
    "client.zoom.reset" => ZoomReset,
    "session.switch" => SwitchSession,
    "session.new" => NewSession,
    "pane.split" => Split,
    "pane.zoom" => ZoomPane,
    "pane.focus" => FocusPane,
    "pane.actions" => PaneActions,
    "pane.copy" => Copy,
    "pane.paste" => Paste,
    "pane.reconnect" => Reconnect,
    "pane.scroll" => Scroll,
    "client.tab.goto" => GotoTab,
    "client.tab.close" => CloseTab,
    "client.tab.rename" => RenameTab,
    "session.rename" => RenameSession,
    "pane.close" => ClosePane,
    "pane.send_key" => SendKey,
}

pub fn find(name: &str) -> Option<&'static Spec> {
    SPECS.iter().find(|spec| spec.name == name)
}

impl Spec {
    pub fn global(&self) -> bool {
        matches!(
            self.handler,
            Handler::ZoomIn | Handler::ZoomOut | Handler::ZoomReset
        )
    }
    pub fn repeats(&self) -> bool {
        self.global()
            || matches!(
                self.handler,
                Handler::FocusPane
                    | Handler::Scroll
                    | Handler::NextGlobal
                    | Handler::PreviousGlobal
            )
    }

    /// Validate before dispatch: invalid arguments must not partially perform an action.
    pub fn accepts(&self, args: &Value) -> bool {
        let Some(object) = args.as_object() else {
            return args.is_null()
                && !matches!(
                    self.handler,
                    Handler::Split
                        | Handler::FocusPane
                        | Handler::Scroll
                        | Handler::GotoTab
                        | Handler::SendKey
                );
        };
        match self.handler {
            Handler::Split => {
                object.len() == 1 && matches!(args["direction"].as_str(), Some("right" | "down"))
            }
            Handler::FocusPane => {
                object.len() == 1
                    && matches!(
                        args["direction"].as_str(),
                        Some("left" | "right" | "up" | "down")
                    )
            }
            Handler::Scroll => {
                object.len() == 1
                    && args["lines"]
                        .as_i64()
                        .is_some_and(|n| i32::try_from(n).is_ok())
            }
            Handler::GotoTab => {
                object.len() == 1
                    && args["index"]
                        .as_u64()
                        .is_some_and(|n| n > 0 && usize::try_from(n).is_ok())
            }
            Handler::SendKey => {
                object.len() == 1
                    && args["key"]
                        .as_str()
                        .is_some_and(|key| super::keys::synthesize(key).is_some())
            }
            _ => object.is_empty(),
        }
    }
}

/// False means unknown action or invalid args, not that an offline action failed.
pub fn run(s: &mut Workspace, name: &str, args: &Value, tx: &mut Tx) -> bool {
    let Some(spec) = find(name).filter(|spec| spec.accepts(args)) else {
        return false;
    };
    match spec.handler {
        Handler::Palette => s.picker(false),
        Handler::Settings => s.perform(Intent::Settings, tx),
        Handler::Sidebar => s.perform(Intent::Sidebar, tx),
        Handler::Close => s.close_focused(tx),
        Handler::Search => s.open_search(),
        Handler::NewTab => s.perform(Intent::NewWindow, tx),
        Handler::NextTab => s.request("session.focus_next_window", json!({})),
        Handler::PreviousTab => s.request("session.focus_previous_window", json!({})),
        Handler::NextGlobal => s.cycle_tab(1, tx),
        Handler::PreviousGlobal => s.cycle_tab(-1, tx),
        Handler::Back => s.navigate_history(-1, tx),
        Handler::Forward => s.navigate_history(1, tx),
        Handler::ZoomIn => s.change_ui_scale(1, tx),
        Handler::ZoomOut => s.change_ui_scale(-1, tx),
        Handler::ZoomReset => s.change_ui_scale(0, tx),
        Handler::SwitchSession => s.picker(true),
        Handler::NewSession => s.perform(Intent::NewSession, tx),
        Handler::Split => s.perform(
            Intent::Split(if args["direction"] == "right" {
                "horizontal"
            } else {
                "vertical"
            }),
            tx,
        ),
        Handler::ZoomPane => s.perform(Intent::Zoom, tx),
        Handler::FocusPane => s.request("session.focus_direction", args.clone()),
        Handler::PaneActions => {
            if let Some(block) = s.focused_block() {
                s.pane_menu(block, None);
            }
        }
        Handler::Copy => {
            if let Some(text) = s.active_terminal().and_then(|t| t.copy_text()) {
                tx.copy(text);
            }
        }
        Handler::Paste => s.paste_clipboard(tx),
        Handler::Reconnect => {
            if let Some(term) = s.active_terminal_mut() {
                term.send(Command::Reconnect);
            }
        }
        Handler::Scroll => {
            if let Some(term) = s.active_terminal_mut() {
                term.send(Command::Scroll(args["lines"].as_i64().unwrap() as isize));
            }
        }
        Handler::GotoTab => {
            let index = args["index"].as_u64().unwrap() as usize - 1;
            if let Some(id) = s
                .model()
                .view()
                .and_then(|v| v.windows.get(index))
                .and_then(|w| w.window_id.clone())
            {
                s.perform(Intent::Tab(id), tx);
            }
        }
        Handler::CloseTab | Handler::RenameTab => {
            if let Some(id) = s.model().window().and_then(|w| w.window_id.clone()) {
                s.perform(
                    if matches!(spec.handler, Handler::CloseTab) {
                        Intent::CloseWindow(id)
                    } else {
                        Intent::RenameWindow(id)
                    },
                    tx,
                );
            }
        }
        Handler::RenameSession => s.perform(Intent::RenameSession, tx),
        Handler::ClosePane => {
            if let Some(target) = s.focused_block().and_then(|b| s.target(&b)) {
                s.perform(Intent::Pane(target, super::panel::PaneAction::Close), tx);
            }
        }
        Handler::SendKey => {
            if let Some(payloads) = args["key"].as_str().and_then(super::keys::synthesize)
                && let Some(term) = s.active_terminal_mut()
            {
                for payload in payloads {
                    term.send(Command::SendKey(payload));
                }
            }
        }
    }
    true
}

/// Resolve once per file generation, including argument diagnostics. Invalid bindings
/// are skipped at match time, so a lower valid action can still be selected.
pub fn resolve(overrides: &rex_linux::keymap::Overrides) -> rex_linux::keymap::Resolved {
    let actions: Vec<_> = SPECS
        .iter()
        .map(|s| {
            rexrs::api::client::InspectResponseAction::new(s.name)
                .kind("client")
                .repeats(s.repeats())
        })
        .collect();
    let mut map = rex_linux::keymap::resolve(&[rex_linux::keymap::defaults()], overrides, &actions);
    for binding in &map.bindings {
        let binding = &binding.binding;
        if find(&binding.action)
            .is_some_and(|spec| !spec.accepts(binding.args.as_ref().unwrap_or(&Value::Null)))
        {
            map.diagnostics.push(rexrs::api::client::Diagnostic {
                severity: "warning".into(),
                source: binding.source.clone(),
                message: format!("invalid arguments for {:?}", binding.action),
                action: Some(binding.action.clone()),
                key: Some(binding.keys.join(" ")),
                mode: None,
            });
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_unique_and_covers_every_default() {
        let names: std::collections::HashSet<_> = SPECS.iter().map(|s| s.name).collect();
        assert_eq!(names.len(), SPECS.len());
        for binding in rex_linux::keymap::defaults().bindings {
            let spec = find(&binding.action).expect("default action is registered");
            assert!(
                spec.accepts(binding.args.as_ref().unwrap_or(&Value::Null)),
                "{}",
                binding.action
            );
        }
    }

    #[test]
    fn arguments_are_checked_before_dispatch() {
        for name in ["pane.split", "pane.focus", "pane.scroll"] {
            let spec = find(name).unwrap();
            assert!(!spec.accepts(&Value::Null));
            assert!(!spec.accepts(&json!({})));
        }
        assert!(
            !find("pane.split")
                .unwrap()
                .accepts(&json!({"direction":"left"}))
        );
        assert!(!find("pane.scroll").unwrap().accepts(&json!({"lines":1.5})));
        assert!(
            !find("pane.scroll")
                .unwrap()
                .accepts(&json!({"lines":i64::MAX}))
        );
        assert!(!find(SEARCH).unwrap().accepts(&json!({"typo":true})));
        assert!(find(SEARCH).unwrap().accepts(&Value::Null));
        assert!(find(SEARCH).unwrap().accepts(&json!({})));
        assert!(find("client.nothing").is_none());
    }
}
