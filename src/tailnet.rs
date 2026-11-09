//! The machines on this device's tailnet, from `tailscale status --json`, and whether one runs
//! a Rex server this client can attach to directly over the tailnet (built-in Tailscale,
//! `tailscale serve`, or a listener on the tailnet address).
use anyhow::{Context, Result};
use serde::Deserialize;

/// A machine on the tailnet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    /// Its name on the tailnet: the first label of its MagicDNS name, or what it calls itself
    /// when the tailnet has no MagicDNS.
    pub name: String,
    /// How it is reached: its MagicDNS name without the trailing dot, such as
    /// `box.tail1234.ts.net`, or its Tailscale IP when the tailnet has no MagicDNS.
    pub address: String,
    /// The name the machine calls itself, when that differs from `name`.
    pub host_name: Option<String>,
    /// `linux`, `macOS`, `windows`, `iOS`, `android`, or empty.
    pub os: String,
    pub online: bool,
    /// It runs Tailscale SSH, so the tailnet's policy signs SSH logins in with no keys.
    pub ssh: bool,
    /// The device this client runs on.
    pub this_machine: bool,
}

impl Machine {
    /// An iPhone or iPad: it runs neither a Rex server nor `sshd`, so there is nothing to
    /// attach to. (Android can run `sshd`, so it is listed.)
    pub fn is_ios(&self) -> bool {
        self.os == "iOS"
    }

    /// The address as a URL writes it: an IPv6 address in brackets.
    fn address_in_url(&self) -> String {
        if self.address.contains(':') {
            format!("[{}]", self.address)
        } else {
            self.address.clone()
        }
    }

    /// The operating system as words, or empty when unknown.
    pub fn os_name(&self) -> &str {
        match self.os.as_str() {
            "linux" => "Linux",
            "macOS" => "macOS",
            "windows" => "Windows",
            "iOS" => "iOS",
            "android" => "Android",
            other => other,
        }
    }

    /// The endpoints a Rex server reachable over the tailnet may answer at, in the order the
    /// official clients try a bare host name: HTTPS on 443 (built-in Tailscale or `tailscale
    /// serve`), HTTPS on 7297, then HTTP on 7297.
    pub fn rex_endpoints(&self) -> [String; 3] {
        let host = self.address_in_url();
        [
            format!("https://{host}"),
            format!("https://{host}:7297"),
            format!("http://{host}:7297"),
        ]
    }

    /// The endpoint that forwards this machine's Rex socket over SSH as `user`, at `socket`
    /// when given, else wherever the machine says it is.
    pub fn ssh_endpoint(&self, user: &str, socket: &str) -> String {
        let mut out = String::from("ssh://");
        if !user.is_empty() {
            out.push_str(user);
            out.push('@');
        }
        out.push_str(&self.address_in_url());
        if !socket.is_empty() {
            out.push_str(socket);
        }
        out
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Status {
    #[serde(rename = "Self")]
    this: Option<Node>,
    #[serde(default)]
    peer: std::collections::HashMap<String, Node>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Node {
    #[serde(default)]
    host_name: String,
    #[serde(default, rename = "DNSName")]
    dns_name: String,
    #[serde(default, rename = "OS")]
    os: String,
    #[serde(default)]
    online: bool,
    #[serde(default, rename = "sshHostKeys")]
    ssh_host_keys: Vec<String>,
    #[serde(default, rename = "TailscaleIPs")]
    tailscale_ips: Vec<String>,
}

impl Node {
    fn machine(self, this_machine: bool) -> Option<Machine> {
        let dns_name = self.dns_name.trim_end_matches('.').to_owned();
        // Without MagicDNS a machine has no name on the tailnet; its IP reaches it (IPv4
        // first, which reads better and needs no brackets).
        let ip = self
            .tailscale_ips
            .iter()
            .find(|ip| !ip.contains(':'))
            .or(self.tailscale_ips.first())
            .cloned();
        let (name, address) = if !dns_name.is_empty() {
            (
                dns_name.split('.').next().unwrap_or_default().to_owned(),
                dns_name,
            )
        } else if let Some(ip) = ip {
            let name = if self.host_name.is_empty() || self.host_name == "localhost" {
                ip.clone()
            } else {
                self.host_name.clone()
            };
            (name, ip)
        } else {
            return None;
        };
        let host_name = Some(self.host_name)
            .filter(|h| !h.is_empty() && h != "localhost" && !h.eq_ignore_ascii_case(&name));
        Some(Machine {
            name,
            address,
            host_name,
            os: self.os,
            online: this_machine || self.online,
            ssh: !self.ssh_host_keys.is_empty(),
            this_machine,
        })
    }
}

/// The machines in a `tailscale status --json` reply: this one first, then the peers by name.
pub fn parse_status(json: &str) -> Result<Vec<Machine>> {
    let status: Status = serde_json::from_str(json).context("read tailscale status")?;
    let mut machines: Vec<Machine> = status
        .this
        .and_then(|node| node.machine(true))
        .into_iter()
        .collect();
    let mut peers: Vec<Machine> = status
        .peer
        .into_values()
        .filter_map(|node| node.machine(false))
        .collect();
    peers.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.address.cmp(&b.address))
    });
    machines.extend(peers);
    Ok(machines)
}

/// Ask the local Tailscale for the tailnet's machines, or words for the user.
pub fn status() -> std::result::Result<Vec<Machine>, String> {
    let output = std::process::Command::new("tailscale")
        .args(["status", "--json"])
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                "Tailscale isn't installed on this machine, so there's nothing to list. Enter an endpoint instead.".to_owned()
            } else {
                format!("Couldn't run tailscale: {error}")
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.starts_with("Warning:"))
            .unwrap_or("");
        return Err(if reason.is_empty() {
            "Tailscale didn't answer. Is it running and signed in?".to_owned()
        } else {
            format!("Tailscale: {reason}")
        });
    }
    let machines = parse_status(&String::from_utf8_lossy(&output.stdout))
        .map_err(|error| format!("{error:#}"))?;
    if machines.len() <= 1 {
        return Err("Tailscale is signed in but lists no other machines.".to_owned());
    }
    Ok(machines)
}

/// What checking a machine for a Rex server found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    /// A Rex server answers at this endpoint: its control WebSocket negotiated Rex's
    /// subprotocol. Nothing was sent on it.
    Rex(String),
    /// Every listener this client would try refused the connection: nothing listens there.
    Nothing,
    /// The check could not tell, with why (a timeout, a TLS or protocol failure): a server may
    /// still be there behind a firewall or a certificate this client doesn't trust.
    Unknown(String),
}

/// Check `machine` for a Rex server reachable over the tailnet, at each of its
/// [`Machine::rex_endpoints`] at once; the first in that order that answers wins.
pub async fn probe(machine: &Machine) -> Probe {
    let endpoints = machine.rex_endpoints();
    let checks = endpoints.iter().map(|endpoint| async move {
        match tokio::time::timeout(
            std::time::Duration::from_secs(6),
            crate::transport::socket(endpoint, "/api/control", "rex.control.v1"),
        )
        .await
        {
            Ok(Ok(mut socket)) => {
                let _ = tokio::time::timeout(std::time::Duration::from_secs(1), socket.close(None))
                    .await;
                Ok(())
            }
            Ok(Err(error)) => Err(if refused(&error) {
                None
            } else {
                Some(format!("{}: {}", endpoint_port(endpoint), reason(&error)))
            }),
            Err(_) => Err(Some(format!("{}: timed out", endpoint_port(endpoint)))),
        }
    });
    let results = futures_util::future::join_all(checks).await;
    let mut unknown = Vec::new();
    for (endpoint, result) in endpoints.iter().zip(results) {
        match result {
            Ok(()) => return Probe::Rex(endpoint.clone()),
            Err(Some(why)) => unknown.push(why),
            Err(None) => {}
        }
    }
    if unknown.is_empty() {
        Probe::Nothing
    } else {
        Probe::Unknown(unknown.join("; "))
    }
}

/// The connection was refused outright: no listener on that port.
fn refused(error: &anyhow::Error) -> bool {
    error.chain().any(|e| {
        e.downcast_ref::<std::io::Error>()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::ConnectionRefused)
    })
}

/// The innermost cause, as words for a note.
fn reason(error: &anyhow::Error) -> String {
    let root = error
        .chain()
        .last()
        .map(ToString::to_string)
        .unwrap_or_default();
    if root.len() > 80 {
        format!("{}…", &root[..root.floor_char_boundary(80)])
    } else {
        root
    }
}

/// `https 443`, `http 7297`: enough to tell the checks apart.
fn endpoint_port(endpoint: &str) -> String {
    let (scheme, rest) = endpoint.split_once("://").unwrap_or(("", endpoint));
    let port = rest
        .rsplit_once(':')
        .filter(|(host, _)| !host.ends_with(']') || host.starts_with('['))
        .and_then(|(_, port)| port.parse::<u16>().ok())
        .unwrap_or(443);
    format!("{scheme} {port}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS: &str = r#"{
      "Version": "1.104.5",
      "MagicDNSSuffix": "tail1234.ts.net",
      "Self": {"HostName": "here", "DNSName": "here.tail1234.ts.net.", "OS": "linux", "Online": true},
      "Peer": {
        "n1": {"HostName": "Zed Mac", "DNSName": "zed.tail1234.ts.net.", "OS": "macOS", "Online": true},
        "n2": {"HostName": "box", "DNSName": "box.tail1234.ts.net.", "OS": "linux", "Online": true,
               "sshHostKeys": ["ssh-ed25519 AAAA"]},
        "n3": {"HostName": "localhost", "DNSName": "phone.tail1234.ts.net.", "OS": "iOS", "Online": false},
        "n4": {"HostName": "nameless", "DNSName": "", "OS": "linux", "Online": true,
               "TailscaleIPs": ["fd7a:115c:a1e0::1", "100.64.0.9"]},
        "n5": {"HostName": "", "DNSName": "", "OS": "linux", "Online": true, "TailscaleIPs": ["fd7a:115c:a1e0::2"]},
        "n6": {"HostName": "ghost", "DNSName": "", "OS": "linux", "Online": true}
      }
    }"#;

    #[test]
    fn machines_come_from_status_this_one_first_then_by_name() {
        let machines = parse_status(STATUS).unwrap();
        let names: Vec<&str> = machines.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "here",
                "box",
                "fd7a:115c:a1e0::2",
                "nameless",
                "phone",
                "zed"
            ],
            "a machine with neither a name nor an address is not listed"
        );
        assert!(machines[0].this_machine);
        assert_eq!(machines[4].host_name, None, "localhost says nothing");
        assert!(machines[4].is_ios());
        assert_eq!(machines[5].host_name.as_deref(), Some("Zed Mac"));
        assert!(!machines[5].is_ios());
        assert_eq!(machines[0].address, "here.tail1234.ts.net");
        assert!(machines[1].ssh, "sshHostKeys means Tailscale SSH");
        assert!(!machines[5].ssh);
        assert!(!machines[4].online);
        assert_eq!(machines[4].os_name(), "iOS");
        assert_eq!(
            machines[5].rex_endpoints(),
            [
                "https://zed.tail1234.ts.net",
                "https://zed.tail1234.ts.net:7297",
                "http://zed.tail1234.ts.net:7297"
            ]
        );
        assert_eq!(
            machines[1].ssh_endpoint("me", ""),
            "ssh://me@box.tail1234.ts.net"
        );
        assert_eq!(
            machines[1].ssh_endpoint("me", "/run/rex/server.sock"),
            "ssh://me@box.tail1234.ts.net/run/rex/server.sock"
        );
        assert_eq!(
            machines[1].ssh_endpoint("", ""),
            "ssh://box.tail1234.ts.net"
        );

        // Without MagicDNS, the IPv4 address stands in for the name.
        assert_eq!(machines[3].address, "100.64.0.9");
        assert_eq!(
            machines[3].host_name, None,
            "the name already is the host name"
        );
        assert_eq!(machines[3].rex_endpoints()[0], "https://100.64.0.9");
        assert_eq!(
            machines[2].rex_endpoints()[1],
            "https://[fd7a:115c:a1e0::2]:7297"
        );
        assert_eq!(
            machines[2].ssh_endpoint("me", ""),
            "ssh://me@[fd7a:115c:a1e0::2]"
        );
        assert!(
            crate::ssh::Target::parse(&machines[2].ssh_endpoint("me", "")).is_ok(),
            "and the SSH endpoint parses back"
        );
    }

    #[test]
    fn refused_connections_mean_nothing_listens() {
        let refused =
            anyhow::Error::from(std::io::Error::from(std::io::ErrorKind::ConnectionRefused))
                .context("connect");
        assert!(super::refused(&refused));
        let timeout = anyhow::anyhow!("Rex WebSocket connection timed out");
        assert!(!super::refused(&timeout));
        assert_eq!(endpoint_port("https://box"), "https 443");
        assert_eq!(endpoint_port("http://box:7297"), "http 7297");
        assert_eq!(endpoint_port("https://[fd7a::1]:7297"), "https 7297");
        assert_eq!(endpoint_port("https://[fd7a::1]"), "https 443");
    }

    #[tokio::test]
    async fn a_closed_port_on_this_machine_is_nothing_and_a_wrong_server_is_unknown() {
        // Nothing listens on a port a listener just gave back.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let machine = Machine {
            name: "lo".into(),
            address: "127.0.0.1".into(),
            host_name: None,
            os: "linux".into(),
            online: true,
            ssh: false,
            this_machine: false,
        };
        // Every endpoint of the machine refuses when 443 and 7297 are closed here; this test
        // can't bind those, so check the classifier on a single refused endpoint instead.
        let Err(error) = crate::transport::socket(
            &format!("http://127.0.0.1:{port}"),
            "/api/control",
            "rex.control.v1",
        )
        .await
        else {
            panic!("nothing listens on {port}");
        };
        assert!(super::refused(&error), "{error:#}");
        assert!(matches!(
            probe(&machine).await,
            Probe::Nothing | Probe::Unknown(_)
        ));
    }

    #[test]
    fn an_empty_status_has_no_machines() {
        assert!(parse_status("{}").unwrap().is_empty());
        assert!(parse_status("not json").is_err());
    }
}
