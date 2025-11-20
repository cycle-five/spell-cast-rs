use crate::{
    websocket::messages::{ClientMessage, ServerMessage},
    AppState,
};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

/// WebSocket upgrade handler
pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    // Spawn a task to send messages to the client
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Handle incoming messages from the client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        if let Err(e) = handle_client_message(client_msg, &state, &tx).await {
                            tracing::error!("Error handling message: {}", e);
                            let error_msg = ServerMessage::Error {
                                message: e.to_string(),
                            };
                            let _ = tx.send(error_msg).await;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse message: {}", e);
                        let error_msg = ServerMessage::Error {
                            message: format!("Invalid message format: {}", e),
                        };
                        let _ = tx.send(error_msg).await;
                    }
                },
                Message::Close(_) => {
                    tracing::info!("Client disconnected");
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
        }
    }
}

/// Handle individual client messages
async fn handle_client_message(
    msg: ClientMessage,
    _state: &AppState,
    tx: &mpsc::Sender<ServerMessage>,
) -> anyhow::Result<()> {
    match msg {
        ClientMessage::CreateGame { mode } => {
            tracing::info!("Creating game with mode: {:?}", mode);
            // TODO: Implement game creation
            let game_id = uuid::Uuid::new_v4().to_string();
            tx.send(ServerMessage::GameCreated { game_id }).await?;
        }
        ClientMessage::JoinGame { game_id } => {
            tracing::info!("Joining game: {}", game_id);
            // TODO: Implement join game logic
        }
        ClientMessage::LeaveGame => {
            tracing::info!("Leaving game");
            // TODO: Implement leave game logic
        }
        ClientMessage::StartGame => {
            tracing::info!("Starting game");
            // TODO: Implement start game logic
        }
        ClientMessage::SubmitWord { word, positions } => {
            tracing::info!("Submitting word: {} at positions: {:?}", word, positions);
            // TODO: Implement word submission logic
        }
        ClientMessage::PassTurn => {
            tracing::info!("Passing turn");
            // TODO: Implement pass turn logic
        }
        ClientMessage::EnableTimer => {
            tracing::info!("Enabling timer");
            // TODO: Implement timer enable logic
        }
    }

    Ok(())
}
