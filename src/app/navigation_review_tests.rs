//! Regressions adapted from the Opus review of PR #6.
use super::*;

#[test]
fn review_failed_navigation_preserves_the_origin_occurrence() {
    for cancel in ["failure", "timeout", "activate"] {
        for replay in [false, true] {
            let (mut ws, rx) = tabbed_workspace("single");
            for w in ["single", "split", "single", "split"] {
                ws.hosts[0]
                    .model
                    .views
                    .get_mut("s")
                    .unwrap()
                    .active_window_id = Some(w.into());
                ws.observe_navigation();
            }
            let mut rt = runtime(ws);
            if replay {
                rt.update(|s, tx| s.navigate_history(-1, tx));
            } else {
                cycle(&mut rt, false);
            }
            requests(&rx);
            rt.update(|s, tx| {
                let p = s.navigation.pending.as_ref().unwrap();
                let (id, deadline) = (p.id, p.deadline);
                match cancel {
                    "failure" => {
                        s.hosts[0].model.navigation_result = Some((id, false));
                        s.observe_navigation();
                    }
                    "timeout" => {
                        s.poll_navigation(deadline, tx);
                    }
                    _ => s.activate_host(0, tx),
                }
                s.navigate_history(1, tx);
            });
            assert!(
                requests(&rx).is_empty(),
                "{cancel}, replay={replay}: Forward must stay at the newest occurrence"
            );
            std::fs::remove_file(&rt.state.settings_path).ok();
        }
    }
}

#[test]
fn forward_can_cancel_an_unacknowledged_back_to_the_origin() {
    let (mut rt, rx) = recorded_history();
    rt.update(|s, tx| s.navigate_history(-1, tx));
    assert_eq!(requests(&rx)[0].2["window_id"], "single");
    rt.update(|s, tx| s.navigate_history(1, tx));
    assert_eq!(requests(&rx)[0].2["window_id"], "split");
    rt.update(|s, _| acknowledge(s));
    rt.update(|s, tx| s.navigate_history(1, tx));
    assert!(requests(&rx).is_empty());
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn review_click_session_on_reconnecting_host_keeps_the_selection() {
    for via_switcher in [false, true] {
        let (seed, _) = tabbed_workspace("single");
        let mut ws = workspace_hosts(2);
        for host in &mut ws.hosts {
            host.model = seed.hosts[0].model.clone();
        }
        ws.hosts[1].model.connected = false;
        ws.observe_navigation();
        let rx1 = capture_host(&mut ws, 1);
        let mut rt = runtime(ws);
        rt.update(|s, tx| {
            if via_switcher {
                s.picker(true);
                s.perform(Intent::Select(1, "s".into()), tx);
            } else {
                s.select(1, "s".into(), tx);
            }
        });
        let commands: Vec<_> = rx1.try_iter().collect();
        assert!(
            commands
                .iter()
                .any(|c| matches!(c, HostCommand::Select(id) if id == "s")),
            "offline selection must retain the worker's preferred-session path"
        );
        assert!(
            !commands
                .iter()
                .any(|c| matches!(c, HostCommand::Navigate { .. }))
        );
        assert!(rt.state.navigation.pending.is_none());
        rt.update(|s, tx| {
            s.hosts[1].model.navigation_result = Some((1, false));
            s.observe_navigation();
            s.poll_navigation(Instant::now() + Duration::from_secs(31), tx);
            // The reconnect bootstrap then selects the preferred session.
            s.hosts[1].model.connected = true;
            s.hosts[1].model.selected = Some("s".into());
            s.observe_navigation();
        });
        assert_eq!(rt.state.active, 1, "the offline click must not bounce back");
        std::fs::remove_file(&rt.state.settings_path).ok();
        std::fs::remove_file(&seed.settings_path).ok();
    }
}

fn three_hosts() -> Runtime<Workspace> {
    let (seed, _) = tabbed_workspace("single");
    let mut ws = workspace_hosts(3);
    for host in &mut ws.hosts {
        host.model = seed.hosts[0].model.clone();
    }
    ws.observe_navigation();
    ws.active = 1;
    ws.observe_navigation();
    // Keep command receivers alive; otherwise sending is a synchronous failure.
    // The callers use the normal fake host workers, which target nonexistent sockets.
    std::fs::remove_file(&seed.settings_path).ok();
    runtime(ws)
}

#[test]
fn review_success_on_a_different_current_host_finishes_pending() {
    let mut rt = three_hosts();
    let _rx = capture_host(&mut rt.state, 0);
    rt.update(|s, tx| s.navigate_history(-1, tx));
    let p = rt.state.navigation.pending.as_ref().unwrap();
    let (id, deadline) = (p.id, p.deadline);
    rt.update(|s, tx| {
        s.active = 2;
        s.hosts[0].model.navigation_result = Some((id, true));
        s.observe_navigation();
        assert!(
            s.navigation.pending.is_none(),
            "a successful result is final even after focus moved"
        );
        s.poll_navigation(deadline, tx);
    });
    assert_eq!(rt.state.active, 2);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn review_create_session_elsewhere_cancels_pending_before_late_success() {
    let mut rt = three_hosts();
    let _rx0 = capture_host(&mut rt.state, 0);
    let _rx2 = capture_host(&mut rt.state, 2);
    rt.update(|s, tx| s.navigate_history(-1, tx));
    let p = rt.state.navigation.pending.as_ref().unwrap();
    let (id, deadline) = (p.id, p.deadline);
    rt.update(|s, tx| {
        s.create_session(2, None, tx);
        assert!(
            s.navigation.pending.is_none(),
            "creating elsewhere supersedes navigation"
        );
        s.hosts[0].model.navigation_result = Some((id, true));
        s.observe_navigation();
        s.poll_navigation(deadline, tx);
    });
    assert_eq!(rt.state.active, 2);
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn review_settled_dispatch_cannot_override_a_later_workspace_action() {
    for action in ["create", "add", "remove"] {
        let mut rt = three_hosts();
        let _rx0 = capture_host(&mut rt.state, 0);
        let rx1 = capture_host(&mut rt.state, 1);
        rt.update(|s, _| {
            s.active = 0;
            s.hosts[0]
                .model
                .views
                .get_mut("s")
                .unwrap()
                .active_window_id = Some("split".into());
        });
        cycle(&mut rt, false); // host0 split -> host1 single, deferred
        let at = rt
            .state
            .navigation
            .pending
            .as_ref()
            .unwrap()
            .dispatch_at
            .unwrap();
        rt.update(|s, tx| {
            match action {
                "create" => s.create_session(0, None, tx),
                // Exercise the existing-endpoint path without starting a connection.
                "add" => s.add_host(s.settings.hosts[0].clone(), tx),
                _ => s.remove_host(2, tx),
            }
            assert!(
                s.navigation.pending.is_none(),
                "{action} must cancel the settled target"
            );
            s.poll_navigation(at, tx);
        });
        assert_eq!(
            rt.state.active, 0,
            "{action}: settled navigation stole focus"
        );
        assert!(requests(&rx1).is_empty());
        std::fs::remove_file(&rt.state.settings_path).ok();
    }
}

#[test]
fn review_closed_window_does_not_leave_a_noop_back() {
    let (mut ws, rx) = tabbed_workspace("single");
    for w in ["single", "split", "single"] {
        ws.hosts[0]
            .model
            .views
            .get_mut("s")
            .unwrap()
            .active_window_id = Some(w.into());
        ws.observe_navigation();
    }
    ws.hosts[0]
        .model
        .views
        .get_mut("s")
        .unwrap()
        .windows
        .retain(|w| w.window_id.as_deref() != Some("split"));
    let mut rt = runtime(ws);
    rt.update(|s, tx| s.navigate_history(-1, tx));
    assert!(
        requests(&rx).is_empty(),
        "Back must not reselect the current tab after pruning a closed one"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}

#[test]
fn review_skipping_an_offline_host_does_not_reselect_the_origin() {
    let mut rt = three_hosts(); // history: host0 single, host1 single
    rt.update(|s, _| {
        s.active = 0;
        s.observe_navigation();
        s.hosts[1].model.connected = false;
    });
    let rx0 = capture_host(&mut rt.state, 0);
    rt.update(|s, tx| s.navigate_history(-1, tx));
    assert!(
        requests(&rx0).is_empty(),
        "skipping offline visits must also skip the current tab's earlier occurrence"
    );
    std::fs::remove_file(&rt.state.settings_path).ok();
}
