use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use rexrs::AsyncRexClient;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};

pub trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}
pub type Socket = WebSocketStream<tokio_tungstenite::MaybeTlsStream<Box<dyn Io>>>;

pub fn local_endpoint() -> String {
    std::env::var("REX_SERVER").unwrap_or_else(|_| {
        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".into());
        format!("unix://{runtime}/rex/server.sock")
    })
}

async fn io(endpoint: &str, url: &url::Url) -> Result<Box<dyn Io>> {
    if let Some(path) = endpoint.strip_prefix("unix://") {
        return Ok(Box::new(tokio::net::UnixStream::connect(path).await?));
    }
    let host = url.host_str().context("endpoint needs a host")?;
    let port = url
        .port_or_known_default()
        .context("endpoint needs a port")?;
    let tcp = tokio::net::TcpStream::connect((host, port)).await?;
    tcp.set_nodelay(true)?;
    Ok(Box::new(tcp))
}

pub async fn socket(endpoint: &str, path: &str, protocol: &str) -> Result<Socket> {
    tokio::time::timeout(
        std::time::Duration::from_secs(10),
        open_socket(endpoint, path, protocol),
    )
    .await
    .context("Rex WebSocket connection timed out")?
}

async fn open_socket(endpoint: &str, path: &str, protocol: &str) -> Result<Socket> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut request_url = if endpoint.starts_with("unix://") {
        url::Url::parse("ws://localhost")?
    } else {
        let mut url = url::Url::parse(endpoint)?;
        let scheme = match url.scheme() {
            "https" | "wss" => "wss",
            "http" | "ws" => "ws",
            _ => bail!("unsupported endpoint scheme"),
        };
        url.set_scheme(scheme)
            .map_err(|_| anyhow::anyhow!("invalid scheme"))?;
        url
    };
    request_url.set_path(path);
    request_url.set_query(None);
    let mut request = request_url.as_str().into_client_request()?;
    request
        .headers_mut()
        .insert("Sec-WebSocket-Protocol", protocol.parse()?);
    let stream = io(endpoint, &request_url).await?;
    let (socket, response) =
        tokio_tungstenite::client_async_tls_with_config(request, stream, None, None).await?;
    anyhow::ensure!(
        response
            .headers()
            .get("Sec-WebSocket-Protocol")
            .and_then(|h| h.to_str().ok())
            == Some(protocol),
        "Rex did not negotiate {protocol}"
    );
    Ok(socket)
}

pub struct Transport(pub Socket);
impl rexrs::AsyncTransport for Transport {
    async fn send(&mut self, text: String) -> rexrs::Result<()> {
        self.0
            .send(Message::Text(text.into()))
            .await
            .map_err(rexrs::Error::transport)
    }
    async fn recv(&mut self) -> rexrs::Result<rexrs::transport::Frame> {
        loop {
            match self.0.next().await {
                Some(Ok(Message::Text(text))) => {
                    return Ok(rexrs::transport::Frame::Text(text.to_string()));
                }
                Some(Ok(Message::Close(frame))) => {
                    return Ok(rexrs::transport::Frame::Closed {
                        code: frame.as_ref().map(|f| f.code.into()),
                        reason: frame.map(|f| f.reason.to_string()).unwrap_or_default(),
                    });
                }
                Some(Ok(_)) => self.0.flush().await.map_err(rexrs::Error::transport)?,
                Some(Err(error)) => return Err(rexrs::Error::transport(error)),
                None => {
                    return Ok(rexrs::transport::Frame::Closed {
                        code: None,
                        reason: "connection ended".into(),
                    });
                }
            }
        }
    }
    async fn close(&mut self) -> rexrs::Result<()> {
        self.0.close(None).await.map_err(rexrs::Error::transport)
    }
}

pub type Client = AsyncRexClient<Transport>;
pub const MIN_API_REVISION: u32 = 7;
pub const API_REVISION: u32 = 8;
pub async fn control(endpoint: &str, reconnect: Option<&str>) -> Result<Client> {
    // This is an attachment-only client. Server lifetime and listener configuration
    // belong to the existing upstream Rex server, including its Tailscale identity.
    let socket = socket(endpoint, "/api/control", "rex.control.v1").await?;
    let mut builder = AsyncRexClient::builder()
        .name("Rex Linux")
        .version(env!("CARGO_PKG_VERSION"))
        .api_revisions(MIN_API_REVISION..=API_REVISION)
        .platform("linux")
        .kind(rexrs::ClientKind::App);
    if let Some(token) = reconnect {
        builder = builder.reconnect_token(token);
    }
    let client = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        builder.connect_with(Transport(socket)),
    )
    .await
    .context("Rex control WELCOME timed out")??;
    anyhow::ensure!(
        (MIN_API_REVISION..=API_REVISION).contains(&client.welcome().api_revision),
        "Rex negotiated API revision {}; this client supports {MIN_API_REVISION}–{API_REVISION}",
        client.welcome().api_revision
    );
    Ok(client)
}
