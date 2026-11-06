//! Translate authoritative Rex split trees to normalized native pane geometry.
use rexrs::api::{
    session::{ViewLayoutNode, Window},
    terminal::SetTheme,
};
use serde_json::{Value, json};

/// One new terminal block for a layout, starting `shell` when one is set (else the server
/// user's login shell), with this client's declared `theme` when it has one. Rex counts a
/// layout's theme as its creator's declaration.
pub fn shell_layout(shell: Option<&str>, theme: Option<&SetTheme>) -> Value {
    let mut block = json!({"flavor": "com.superlogical.terminal.shell"});
    if let Some(shell) = shell.map(str::trim).filter(|s| !s.is_empty()) {
        block["options"]["shell"] = json!(shell);
    }
    if let Some(theme) = theme {
        block["options"]["theme"] = json!(theme);
    }
    json!({"block": block})
}

#[cfg(test)]
#[test]
fn shell_layout_sets_the_shell_and_theme_only_when_given() {
    assert_eq!(
        shell_layout(None, None),
        json!({"block":{"flavor":"com.superlogical.terminal.shell"}})
    );
    assert_eq!(shell_layout(Some("  "), None), shell_layout(None, None));
    assert_eq!(
        shell_layout(Some("zsh"), None)["block"]["options"],
        json!({"shell":"zsh"})
    );
    let theme = SetTheme::new().background("#001528");
    assert_eq!(
        shell_layout(Some("zsh"), Some(&theme))["block"]["options"],
        json!({"shell":"zsh","theme":{"background":"#001528"}})
    );
    assert_eq!(
        shell_layout(None, Some(&SetTheme::new()))["block"]["options"],
        json!({"theme":{}}),
        "Rex's own colours"
    );
}

#[derive(Clone, Debug)]
pub struct Placement {
    pub block: String,
    pub rect: [f64; 4],
}
#[derive(Clone, Debug)]
pub struct Divider {
    pub before: String,
    pub after: String,
    pub horizontal: bool,
    pub rect: [f64; 4],
    pub ratio: f64,
}
impl Divider {
    pub fn key(&self) -> String {
        format!("{}:{}", self.before, self.after)
    }
}
pub fn placements(window: &Window, drag: Option<(&str, f64)>) -> (Vec<Placement>, Vec<Divider>) {
    if let Some(block) = &window.zoomed_block_id {
        return (
            vec![Placement {
                block: block.clone(),
                rect: [0., 0., 1., 1.],
            }],
            vec![],
        );
    }
    let mut panes = Vec::new();
    let mut dividers = Vec::new();
    for layer in &window.layers {
        // Its blocks keep running, but a minimized layer isn't drawn. Unknown states draw.
        if layer.state.as_deref() == Some("minimized") {
            continue;
        }
        if let Some(node) = &layer.layout {
            let rect = layer
                .bounds
                .as_slice()
                .try_into()
                .unwrap_or([0., 0., 1., 1.]);
            walk(node, rect, drag, &mut panes, &mut dividers);
        }
    }
    (panes, dividers)
}
fn first(node: &ViewLayoutNode) -> Option<&str> {
    node.block_id
        .as_deref()
        .or_else(|| first(node.split.as_ref()?.before.as_deref()?))
}
fn walk(
    node: &ViewLayoutNode,
    rect: [f64; 4],
    drag: Option<(&str, f64)>,
    panes: &mut Vec<Placement>,
    dividers: &mut Vec<Divider>,
) {
    if let Some(block) = &node.block_id {
        panes.push(Placement {
            block: block.clone(),
            rect,
        });
        return;
    }
    let Some(split) = &node.split else {
        return;
    };
    let (Some(before), Some(after)) = (&split.before, &split.after) else {
        return;
    };
    let horizontal = split.direction != rexrs::api::Direction::Vertical;
    let key = format!(
        "{}:{}",
        first(before).unwrap_or(""),
        first(after).unwrap_or("")
    );
    let ratio = drag
        .filter(|(id, _)| *id == key)
        .map_or(split.ratio, |(_, r)| r);
    if let (Some(before), Some(after)) = (first(before), first(after)) {
        dividers.push(Divider {
            before: before.into(),
            after: after.into(),
            horizontal,
            rect,
            ratio,
        });
    }
    let [x, y, w, h] = rect;
    let (a, b) = if horizontal {
        (
            [x, y, w * ratio, h],
            [x + w * ratio, y, w * (1. - ratio), h],
        )
    } else {
        (
            [x, y, w, h * ratio],
            [x, y + h * ratio, w, h * (1. - ratio)],
        )
    };
    walk(before, a, drag, panes, dividers);
    walk(after, b, drag, panes, dividers);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn nested_layout_and_drag_keep_server_order_and_area() {
        let window: Window = serde_json::from_value(json!({"layers":[{"kind":"tiled","bounds":[0,0,1,1],"layout":{"split":{"direction":"horizontal","ratio":0.6,"before":{"block_id":"a"},"after":{"split":{"direction":"vertical","ratio":0.4,"before":{"block_id":"b"},"after":{"block_id":"c"}}}}},"blocks":[]}]})).unwrap();
        let (panes, borders) = placements(&window, None);
        assert_eq!(
            panes.iter().map(|p| p.block.as_str()).collect::<Vec<_>>(),
            ["a", "b", "c"]
        );
        assert_eq!(panes[1].rect, [0.6, 0., 0.4, 0.4]);
        let key = borders[0].key();
        let (panes, _) = placements(&window, Some((&key, 0.7)));
        assert_eq!(panes[0].rect, [0., 0., 0.7, 1.]);
        assert!((panes.iter().map(|p| p.rect[2] * p.rect[3]).sum::<f64>() - 1.).abs() < 1e-9);
    }
    #[test]
    fn zoom_and_floating_layers_use_window_coordinates() {
        let mut window: Window = serde_json::from_value(json!({"layers":[{"kind":"floating","bounds":[0.2,0.1,0.5,0.6],"layout":{"block_id":"floating"}}]})).unwrap();
        assert_eq!(placements(&window, None).0[0].rect, [0.2, 0.1, 0.5, 0.6]);
        window.zoomed_block_id = Some("floating".into());
        assert_eq!(placements(&window, None).0[0].rect, [0., 0., 1., 1.]);
        assert!(placements(&window, None).1.is_empty());
    }
    #[test]
    fn minimized_layers_are_not_drawn_and_unknown_states_are() {
        let window: Window = serde_json::from_value(json!({"layers":[
            {"kind":"tiled","bounds":[0,0,1,1],"layout":{"block_id":"tiled"}},
            {"kind":"floating","state":"minimized","bounds":[0.2,0.1,0.5,0.6],"layout":{"block_id":"hidden"}},
            {"kind":"floating","state":"docked","bounds":[0.2,0.1,0.5,0.6],"layout":{"block_id":"later"}}
        ]}))
        .unwrap();
        let (panes, _) = placements(&window, None);
        assert_eq!(
            panes.iter().map(|p| p.block.as_str()).collect::<Vec<_>>(),
            ["tiled", "later"]
        );
    }
}
