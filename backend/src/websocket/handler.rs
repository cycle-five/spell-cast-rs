use crate::{
    auth::AuthenticatedUser,
    websocket::messages::{ClientMessage, LobbyPlayerInfo, ServerMessage},
    AppState, LobbyPlayer,
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

/// WebSocket upgrade handler with authentication
pub async fn handle_websocket(
    user: AuthenticatedUser,
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    tracing::info!(
        "WebSocket connection authenticated for user: {} ({})",
        user.username,
        user.user_id
    );
    ws.on_upgrade(move |socket| handle_socket(socket, state, user))
}

/// Handle individual WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>, user: AuthenticatedUser) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    tracing::info!(
        "WebSocket connection established for user: {} ({})",
        user.username,
        user.user_id
    );

    // Add player to lobby
    let lobby_player = LobbyPlayer {
        user_id: user.user_id,
        username: user.username.clone(),
        tx: tx.clone(),
    };
    state.lobby_players.insert(user.user_id, lobby_player);

    // Broadcast updated player list to all connected clients
    broadcast_lobby_player_list(&state).await;

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
    let user_for_recv = user.clone();
    let state_for_recv = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        if let Err(e) =
                            handle_client_message(client_msg, &state_for_recv, &tx, &user_for_recv)
                                .await
                        {
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
                    tracing::info!(
                        "Client disconnected: {} ({})",
                        user_for_recv.username,
                        user_for_recv.user_id
                    );
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

    // Remove player from lobby
    state.lobby_players.remove(&user.user_id);

    // Broadcast updated player list after disconnect
    broadcast_lobby_player_list(&state).await;

    tracing::info!(
        "WebSocket connection closed for user: {} ({})",
        user.username,
        user.user_id
    );
}

/// Broadcast the current lobby player list to all connected clients
async fn broadcast_lobby_player_list(state: &AppState) {
    let players: Vec<LobbyPlayerInfo> = state
        .lobby_players
        .iter()
        .map(|entry| LobbyPlayerInfo {
            user_id: entry.user_id.to_string(),
            username: entry.username.clone(),
        })
        .collect();

    let message = ServerMessage::LobbyPlayerList { players };

    for entry in state.lobby_players.iter() {
        let _ = entry.tx.send(message.clone()).await;
    }
}

/// Handle individual client messages
async fn handle_client_message(
    msg: ClientMessage,
    _state: &AppState,
    tx: &mpsc::Sender<ServerMessage>,
    user: &AuthenticatedUser,
) -> anyhow::Result<()> {
    match msg {
        ClientMessage::CreateGame { mode } => {
            tracing::info!(
                "User {} ({}) creating game with mode: {:?}",
                user.username,
                user.user_id,
                mode
            );
            // TODO: Implement game creation
            let game_id = uuid::Uuid::new_v4().to_string();
            tx.send(ServerMessage::GameCreated { game_id }).await?;
        }
        ClientMessage::JoinGame { game_id } => {
            tracing::info!(
                "User {} ({}) joining game: {}",
                user.username,
                user.user_id,
                game_id
            );
            // TODO: Implement join game logic
        }
        ClientMessage::LeaveGame => {
            tracing::info!(
                "User {} ({}) leaving game",
                user.username,
                user.user_id
            );
            // TODO: Implement leave game logic
        }
        ClientMessage::StartGame => {
            tracing::info!(
                "User {} ({}) starting game",
                user.username,
                user.user_id
            );
            // TODO: Implement start game logic
        }
        ClientMessage::SubmitWord { word, positions } => {
            tracing::info!(
                "User {} ({}) submitting word: {} at positions: {:?}",
                user.username,
                user.user_id,
                word,
                positions
            );
            // TODO: Implement word submission logic
        }
        ClientMessage::PassTurn => {
            tracing::info!(
                "User {} ({}) passing turn",
                user.username,
                user.user_id
            );
            // TODO: Implement pass turn logic
        }
        ClientMessage::EnableTimer => {
            tracing::info!(
                "User {} ({}) enabling timer",
                user.username,
                user.user_id
            );
            // TODO: Implement timer enable logic
        }
    }

    Ok(())
}
