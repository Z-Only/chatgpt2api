use std::net::{IpAddr, SocketAddr};

use axum::{
    extract::Request,
    http::{
        header::{ACCESS_CONTROL_ALLOW_ORIGIN, ORIGIN, VARY},
        HeaderValue,
    },
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use tokio::{net::TcpListener, sync::oneshot};
use url::Url;

use crate::{
    api::{
        health::{health, root},
        images, openai, ApiState,
    },
    config::AppConfig,
    error::{AppError, AppResult},
};

#[derive(Debug)]
pub struct ServerHandle {
    addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
}

impl ServerHandle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn stop(self) {
        let _ = self.shutdown.send(());
    }
}

pub async fn spawn(config: AppConfig) -> AppResult<ServerHandle> {
    spawn_with_state(ApiState::new(config)).await
}

pub async fn spawn_with_state(state: ApiState) -> AppResult<ServerHandle> {
    let listener = TcpListener::bind(socket_addr(&state.config)?).await?;
    let addr = listener.local_addr()?;
    let (shutdown, shutdown_rx) = oneshot::channel();

    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router_with_state(state))
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
        {
            tracing::error!(%error, "local API server failed");
        }
    });

    Ok(ServerHandle { addr, shutdown })
}

pub fn router() -> Router {
    router_with_state(ApiState::new(AppConfig::default()))
}

pub fn router_with_state(state: ApiState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .merge(openai::routes())
        .merge(images::routes())
        .layer(middleware::from_fn(local_cors))
        .with_state(state)
}

pub fn socket_addr(config: &AppConfig) -> AppResult<SocketAddr> {
    config.validate()?;
    let host: IpAddr =
        config.server.host.parse().map_err(|_| {
            AppError::InvalidConfig("server.host must be an IP address".to_string())
        })?;
    Ok(SocketAddr::new(host, config.server.port))
}

async fn local_cors(request: Request, next: Next) -> Response {
    let origin = request
        .headers()
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok())
        .filter(|origin| is_local_origin(origin))
        .map(str::to_string);
    let mut response = next.run(request).await;

    if let Some(origin) = origin.and_then(|origin| HeaderValue::from_str(&origin).ok()) {
        response
            .headers_mut()
            .insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        response
            .headers_mut()
            .insert(VARY, HeaderValue::from_static("origin"));
    }

    response
}

fn is_local_origin(origin: &str) -> bool {
    let Ok(origin) = Url::parse(origin) else {
        return false;
    };
    matches!(origin.scheme(), "http" | "https" | "tauri")
        && matches!(origin.host_str(), Some("127.0.0.1" | "localhost" | "::1"))
}
