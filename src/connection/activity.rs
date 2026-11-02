//! What programs in a terminal report about themselves: OSC 7501 status records, the
//! foreground process, and OSC 9 / OSC 777 desktop notifications. All of it arrives as the
//! terminal creator's `block_event`s, or from its `program_status` and `process` methods.
use serde_json::Value;

pub const TERMINAL: &str = "com.superlogical.terminal";

/// A record's state, in priority order: a tab shows the highest state among its records.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgramState {
    #[default]
    Idle,
    Done,
    Working,
    Error,
    Blocked,
}

impl ProgramState {
    /// Unknown states read as idle, so a newer server's states never raise an alarm.
    fn parse(value: &str) -> Self {
        match value {
            "working" => Self::Working,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            "error" => Self::Error,
            _ => Self::Idle,
        }
    }
}

/// Why a blocked program is waiting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockedKind {
    Permission,
    Question,
    Auth,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusRecord {
    pub state: ProgramState,
    pub kind: Option<BlockedKind>,
    /// The program that reported, as `effective_app` (or `app`) names it.
    pub app: Option<String>,
    /// The program's own words; shown only as plain text.
    pub title: Option<String>,
    pub msg: Option<String>,
    /// Reported while this client was listening, not read back by a query: only live
    /// changes are news worth a notification.
    pub live: bool,
}

impl StatusRecord {
    pub fn parse(value: &Value) -> Self {
        let text = |key: &str| {
            value[key]
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
        };
        Self {
            state: ProgramState::parse(value["state"].as_str().unwrap_or_default()),
            kind: match value["kind"].as_str() {
                Some("permission") => Some(BlockedKind::Permission),
                Some("question") => Some(BlockedKind::Question),
                Some("auth") => Some(BlockedKind::Auth),
                _ => None,
            },
            app: text("effective_app").or_else(|| text("app")),
            title: text("title"),
            msg: text("msg"),
            live: false,
        }
    }
}

/// The name to show for a foreground process: the basename of the first of `script`,
/// `argv0` and `name` that is present, without a login shell's leading `-`.
pub fn program_name(foreground: &Value) -> Option<String> {
    ["script", "argv0", "name"]
        .iter()
        .filter_map(|key| foreground[*key].as_str())
        .find(|s| !s.is_empty())
        .map(|s| s.rsplit('/').next().unwrap_or(s).trim_start_matches('-'))
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockEvent {
    StatusChanged {
        id: String,
        record: StatusRecord,
    },
    StatusRemoved {
        ids: Vec<String>,
    },
    /// The foreground program's name; none when the server couldn't tell.
    Process(Option<String>),
    Notification {
        title: String,
        body: String,
    },
}

/// A terminal `block_event`'s block and meaning; none for other creators and other events.
pub fn parse_block_event(payload: &Value) -> Option<(String, BlockEvent)> {
    if payload["creator_name"] != TERMINAL {
        return None;
    }
    let block = payload["block_id"].as_str()?.to_owned();
    let body = &payload["payload"];
    let event = match payload["name"].as_str()? {
        "program_status_changed" => BlockEvent::StatusChanged {
            id: body["id"].as_str().unwrap_or_default().to_owned(),
            record: StatusRecord {
                live: true,
                ..StatusRecord::parse(&body["record"])
            },
        },
        "program_status_removed" => BlockEvent::StatusRemoved {
            ids: body["ids"]
                .as_array()?
                .iter()
                .filter_map(|id| id.as_str().map(str::to_owned))
                .collect(),
        },
        "process_changed" => BlockEvent::Process(program_name(&body["foreground"])),
        "desktop_notification" => BlockEvent::Notification {
            title: body["title"].as_str().unwrap_or_default().to_owned(),
            body: body["body"].as_str().unwrap_or_default().to_owned(),
        },
        _ => return None,
    };
    Some((block, event))
}

/// A `program_status` reply's records by id; the root record's id is empty.
pub fn parse_status(reply: &Value) -> Vec<(String, StatusRecord)> {
    reply["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| {
            (
                r["id"].as_str().unwrap_or_default().to_owned(),
                StatusRecord::parse(r),
            )
        })
        .collect()
}

/// An OSC 9 or OSC 777 notification a program sent, with where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct DesktopNotice {
    pub session: String,
    pub block: String,
    pub title: String,
    pub body: String,
}
