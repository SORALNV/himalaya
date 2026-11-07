//! # Setup HTTP boundary
//!
//! Uses Hyper's HTTP/1 parser with exact-origin capability authorization,
//! bounded connections and bodies, and locally bundled static assets.

use std::{convert::Infallible, future::Future, io, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{
    Request, Response, StatusCode,
    body::{Bytes, Incoming},
    header::{CONTENT_LENGTH, CONTENT_TYPE, HOST, ORIGIN, TRANSFER_ENCODING},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use serde_json::json;
use tokio::{
    net::{TcpListener, TcpStream},
    signal::unix::{SignalKind, signal},
    sync::Semaphore,
    task::JoinSet,
    time::{interval, timeout},
};
use zeroize::Zeroizing;

use crate::wizard::web::{Services, Session};

const MAX_BODY: usize = 8192;
const MAX_SETTINGS_BODY: usize = 131072;
const TOKEN_HEADER: &str = "x-himalaya-setup-token";

/// Validates requests before parsing account fields or invoking side effects.
pub fn authorize<E: Services, B>(
    request: &Request<B>,
    session: &Session<E>,
) -> std::result::Result<(), StatusCode> {
    let headers = request.headers();
    if request.uri().scheme().is_some()
        || request.uri().authority().is_some()
        || headers.get_all(HOST).iter().count() != 1
        || headers.get(HOST).and_then(|v| v.to_str().ok()) != Some(session.authority.as_str())
        || request.uri().query().is_some()
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let method = request.method().as_str();
    let origin = format!("http://{}", session.authority);
    let origins = headers.get_all(ORIGIN).iter().count();
    if origins > 1
        || (origins == 1
            && headers.get(ORIGIN).and_then(|v| v.to_str().ok()) != Some(origin.as_str()))
        || (method == "POST" && origins != 1)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    if headers.contains_key(TRANSFER_ENCODING) || headers.get_all(CONTENT_LENGTH).iter().count() > 1
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let length = headers
        .get(CONTENT_LENGTH)
        .map(|v| v.to_str().ok().and_then(|s| s.parse::<usize>().ok()));
    let limit = if request.uri().path().starts_with("/api/settings/") {
        MAX_SETTINGS_BODY
    } else {
        MAX_BODY
    };
    if length.is_some_and(|n| n.is_none() || n.is_some_and(|n| n > limit)) {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }
    if method == "GET" && length.flatten().unwrap_or(0) != 0 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = request.uri().path();
    let public = method == "GET" && matches!(path, "/" | "/app.js" | "/style.css");
    if !public {
        if headers.get_all(TOKEN_HEADER).iter().count() != 1
            || !session.authorized(
                headers
                    .get(TOKEN_HEADER)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or(""),
            )
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if !matches!(
            (method, path),
            ("GET", "/api/status")
                | (
                    "POST",
                    "/api/test"
                        | "/api/save"
                        | "/api/finish"
                        | "/api/wizard/start"
                        | "/api/wizard/poll"
                        | "/api/wizard/answer"
                        | "/api/wizard/save"
                        | "/api/wizard/cancel"
                        | "/api/settings/discover"
                        | "/api/settings/check"
                        | "/api/settings/poll"
                        | "/api/settings/cancel"
                        | "/api/settings/save"
                        | "/api/settings/result"
                )
        ) {
            return Err(StatusCode::METHOD_NOT_ALLOWED);
        }
        if method == "POST"
            && (headers.get(CONTENT_TYPE).and_then(|v| v.to_str().ok()) != Some("application/json")
                || headers.get_all(CONTENT_TYPE).iter().count() != 1
                || length.flatten().is_none())
        {
            return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        }
    }
    if !session.live() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(())
}

async fn handle<E: Services>(
    request: Request<Incoming>,
    session: Arc<Session<E>>,
) -> std::result::Result<Response<Full<Bytes>>, Infallible> {
    if let Err(status) = authorize(&request, &session) {
        return Ok(reply(
            status,
            "application/json",
            "{\"error\":\"Request rejected\"}",
        ));
    }
    let method = request.method().as_str().to_owned();
    let path = request.uri().path().to_owned();
    let limit = if path.starts_with("/api/settings/") {
        MAX_SETTINGS_BODY
    } else {
        MAX_BODY
    };
    let asset = match path.as_str() {
        "/" => Some((
            "text/html; charset=utf-8",
            include_str!("assets/index.html"),
        )),
        "/app.js" => Some((
            "text/javascript; charset=utf-8",
            include_str!("assets/app.js"),
        )),
        "/style.css" => Some(("text/css; charset=utf-8", include_str!("assets/style.css"))),
        _ => None,
    };
    if let Some((kind, text)) = asset {
        return Ok(reply(StatusCode::OK, kind, text));
    }
    let body = match timeout(
        Duration::from_secs(5),
        Limited::new(request.into_body(), limit).collect(),
    )
    .await
    {
        Ok(Ok(body)) => Zeroizing::new(body.to_bytes().to_vec()),
        _ => {
            return Ok(reply(
                StatusCode::BAD_REQUEST,
                "application/json",
                "{\"error\":\"Invalid or oversized request body\"}",
            ));
        }
    };
    let work = session.begin_work();
    let result = tokio::task::spawn_blocking(move || {
        let _work = work;
        session.dispatch(&method, &path, &body)
    })
    .await;
    let (status, value) = match result {
        Ok(Ok(value)) => (StatusCode::OK, value),
        Ok(Err(error)) => (StatusCode::BAD_REQUEST, json!({"error": error.to_string()})),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({"error": "Setup operation failed"}),
        ),
    };
    Ok(reply(status, "application/json", &value.to_string()))
}

fn reply(status: StatusCode, kind: &str, body: &str) -> Response<Full<Bytes>> {
    Response::builder().status(status)
        .header(CONTENT_TYPE, kind).header("Cache-Control", "no-store")
        .header("Content-Security-Policy", "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'")
        .header("X-Content-Type-Options", "nosniff").header("X-Frame-Options", "DENY")
        .header("Referrer-Policy", "no-referrer").header("Cross-Origin-Resource-Policy", "same-origin")
        .header("Cross-Origin-Opener-Policy", "same-origin").header("Connection", "close")
        .body(Full::new(Bytes::copy_from_slice(body.as_bytes()))).expect("static response headers")
}

/// The accept boundary, injectable only by compiled fixture implementations.
pub trait SetupListener: Send + Sync + 'static {
    /// Accepts a connection or reports a listener failure for common teardown.
    fn accept(&self) -> impl Future<Output = io::Result<(TcpStream, SocketAddr)>> + Send;
}

impl SetupListener for TcpListener {
    fn accept(&self) -> impl Future<Output = io::Result<(TcpStream, SocketAddr)>> + Send {
        TcpListener::accept(self)
    }
}

/// Serves at most eight bounded HTTP/1 connections until revocation or expiry.
pub async fn serve<E: Services>(
    listener: impl SetupListener,
    session: Arc<Session<E>>,
) -> Result<()> {
    let permits = Arc::new(Semaphore::new(8));
    let mut connections = JoinSet::new();
    let mut clock = interval(Duration::from_millis(200));
    let mut accept_error = None;
    let signals = signal(SignalKind::terminate())
        .and_then(|terminate| signal(SignalKind::hangup()).map(|hangup| (terminate, hangup)));
    let (mut terminate, mut hangup) = match signals {
        Ok(signals) => signals,
        Err(error) => {
            session.revoke();
            tokio::task::spawn_blocking(move || session.settle()).await??;
            return Err(error.into());
        }
    };
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = terminate.recv() => break,
            _ = hangup.recv() => break,
            _ = clock.tick() => { session.watchdog(); if !session.live() { break; } },
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            accepted = listener.accept() => {
                let (socket, peer) = match accepted {
                    Ok(connection) => connection,
                    Err(error) => { accept_error = Some(error); break; }
                };
                if !peer.ip().is_loopback() { continue; }
                let Ok(permit) = permits.clone().try_acquire_owned() else { continue; };
                let session = session.clone();
                connections.spawn(async move {
                    let _permit = permit;
                    let service = service_fn(move |request| handle(request, session.clone()));
                    let mut builder = http1::Builder::new();
                    builder.keep_alive(false).max_headers(32).max_buf_size(16384)
                        .timer(TokioTimer::new()).header_read_timeout(Duration::from_secs(5));
                    let _ = timeout(Duration::from_secs(40), builder.serve_connection(TokioIo::new(socket), service)).await;
                });
            }
        }
    }
    session.revoke();
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    tokio::task::spawn_blocking(move || session.settle()).await??;
    if let Some(error) = accept_error {
        return Err(error.into());
    }
    Ok(())
}
