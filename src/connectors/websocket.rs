//! Shared WebSocket Utilities
//!
//! Common WebSocket connection management for exchange connectors.

use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use crate::connectors::ConnectorError;

/// Configuration for WebSocket connections
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// URL to connect to
    pub url: String,
    /// Ping interval
    pub ping_interval: Duration,
    /// Reconnect delay on disconnect
    pub reconnect_delay: Duration,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            ping_interval: Duration::from_secs(30),
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 10,
        }
    }
}

impl WebSocketConfig {
    pub fn new(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ..Default::default()
        }
    }
}

/// WebSocket connection handle
pub struct WebSocketHandle {
    /// Channel to send messages to the WebSocket
    pub sender: mpsc::Sender<String>,
    /// Receiver for incoming messages
    pub receiver: mpsc::Receiver<Result<String, ConnectorError>>,
}

/// Spawn a WebSocket connection task
/// Returns channels for sending/receiving messages
pub async fn spawn_websocket(config: WebSocketConfig) -> Result<WebSocketHandle, ConnectorError> {
    let (outgoing_tx, mut outgoing_rx) = mpsc::channel::<String>(100);
    let (incoming_tx, incoming_rx) = mpsc::channel::<Result<String, ConnectorError>>(100);
    
    let url = config.url.clone();
    
    tokio::spawn(async move {
        let mut reconnect_attempts = 0;
        
        loop {
            match connect_async(&url).await {
                Ok((ws_stream, _response)) => {
                    reconnect_attempts = 0;
                    info!("WebSocket connected to {}", url);
                    
                    let (mut write, mut read) = ws_stream.split();
                    
                    // Spawn ping task
                    let ping_interval = config.ping_interval;
                    let (ping_tx, mut ping_rx) = mpsc::channel::<()>(1);
                    
                    tokio::spawn(async move {
                        let mut interval = tokio::time::interval(ping_interval);
                        loop {
                            interval.tick().await;
                            if ping_tx.send(()).await.is_err() {
                                break;
                            }
                        }
                    });
                    
                    loop {
                        tokio::select! {
                            // Handle outgoing messages
                            Some(msg) = outgoing_rx.recv() => {
                                debug!("Sending WS message: {}", &msg[..msg.len().min(100)]);
                                if write.send(Message::Text(msg)).await.is_err() {
                                    error!("Failed to send WebSocket message");
                                    break;
                                }
                            }
                            
                            // Handle incoming messages
                            Some(result) = read.next() => {
                                match result {
                                    Ok(Message::Text(text)) => {
                                        debug!("Received WS message: {}", &text[..text.len().min(100)]);
                                        if incoming_tx.send(Ok(text)).await.is_err() {
                                            warn!("Receiver dropped, closing WebSocket");
                                            return;
                                        }
                                    }
                                    Ok(Message::Ping(data)) => {
                                        if write.send(Message::Pong(data)).await.is_err() {
                                            break;
                                        }
                                    }
                                    Ok(Message::Pong(_)) => {
                                        // Pong received, connection is alive
                                    }
                                    Ok(Message::Close(_)) => {
                                        info!("WebSocket closed by server");
                                        break;
                                    }
                                    Ok(Message::Binary(_)) => {
                                        // Some exchanges send binary, might need to handle
                                    }
                                    Ok(Message::Frame(_)) => {}
                                    Err(e) => {
                                        error!("WebSocket error: {}", e);
                                        let _ = incoming_tx.send(Err(ConnectorError::WebSocketError(e.to_string()))).await;
                                        break;
                                    }
                                }
                            }
                            
                            // Handle ping
                            Some(()) = ping_rx.recv() => {
                                if write.send(Message::Ping(vec![])).await.is_err() {
                                    break;
                                }
                            }
                            
                            else => break,
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket connection failed: {}", e);
                    let _ = incoming_tx.send(Err(ConnectorError::WebSocketError(e.to_string()))).await;
                }
            }
            
            // Reconnection logic
            reconnect_attempts += 1;
            if reconnect_attempts >= config.max_reconnect_attempts {
                error!("Max reconnection attempts reached, giving up");
                break;
            }
            
            warn!("Reconnecting in {:?} (attempt {}/{})", 
                  config.reconnect_delay, reconnect_attempts, config.max_reconnect_attempts);
            tokio::time::sleep(config.reconnect_delay).await;
        }
    });
    
    Ok(WebSocketHandle {
        sender: outgoing_tx,
        receiver: incoming_rx,
    })
}

/// Helper to subscribe to a channel on an exchange
pub async fn subscribe(
    sender: &mpsc::Sender<String>,
    subscription_msg: &str,
) -> Result<(), ConnectorError> {
    sender.send(subscription_msg.to_string()).await
        .map_err(|_| ConnectorError::WebSocketError("Failed to send subscription".to_string()))
}
