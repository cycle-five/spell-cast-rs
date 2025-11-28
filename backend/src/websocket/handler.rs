use crate::{
    auth::AuthenticatedUser,
    db,
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
use dashmap::DashMap;
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

/// Context for a connected player, tracking their lobby membership
struct PlayerContext {
    channel_id: Option<String>,
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

    // Track player's current channel for cleanup on disconnect
    let player_context = Arc::new(tokio::sync::Mutex::new(PlayerContext { channel_id: None }));

    // Handle incoming messages from the client
    let user_for_recv = user.clone();
    let state_for_recv = state.clone();
    let context_for_recv = player_context.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        if let Err(e) = handle_client_message(
                            client_msg,
                            &state_for_recv,
                            &tx,
                            &user_for_recv,
                            &context_for_recv,
                        )
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

    // Remove player from their lobby channel on disconnect
    let context = player_context.lock().await;
    if let Some(channel_id) = &context.channel_id {
        remove_player_from_lobby(&state, channel_id, user.user_id).await;
    }

    tracing::info!(
        "WebSocket connection closed for user: {} ({})",
        user.username,
        user.user_id
    );
}

/// Add a player to a channel lobby and broadcast the update
async fn add_player_to_lobby(
    state: &AppState,
    channel_id: &str,
    guild_id: Option<String>,
    user: &AuthenticatedUser,
    avatar_url: Option<String>,
    tx: mpsc::Sender<ServerMessage>,
) {
    let lobby_player = LobbyPlayer {
        user_id: user.user_id,
        username: user.username.clone(),
        avatar_url,
        channel_id: channel_id.to_string(),
        guild_id,
        tx,
    };

    // Get or create the channel lobby
    let channel_lobby = state
        .channel_lobbies
        .entry(channel_id.to_string())
        .or_insert_with(DashMap::new);

    channel_lobby.insert(user.user_id, lobby_player);

    tracing::info!(
        "Player {} ({}) joined lobby for channel {}",
        user.username,
        user.user_id,
        channel_id
    );

    // Broadcast updated player list to all clients in this channel
    broadcast_channel_lobby_player_list(state, channel_id).await;
}

/// Remove a player from their channel lobby and broadcast the update
async fn remove_player_from_lobby(state: &AppState, channel_id: &str, user_id: i64) {
    if let Some(channel_lobby) = state.channel_lobbies.get(channel_id) {
        channel_lobby.remove(&user_id);

        tracing::info!(
            "Player {} removed from lobby for channel {}",
            user_id,
            channel_id
        );

        // If lobby is empty, remove it entirely
        if channel_lobby.is_empty() {
            drop(channel_lobby); // Release the read lock
            state.channel_lobbies.remove(channel_id);
            tracing::info!("Empty lobby removed for channel {}", channel_id);
        } else {
            // Broadcast updated player list to remaining clients
            drop(channel_lobby); // Release the read lock before broadcast
            broadcast_channel_lobby_player_list(state, channel_id).await;
        }
    }
}

/// Broadcast the current lobby player list to all clients in a specific channel
async fn broadcast_channel_lobby_player_list(state: &AppState, channel_id: &str) {
    if let Some(channel_lobby) = state.channel_lobbies.get(channel_id) {
        let players: Vec<LobbyPlayerInfo> = channel_lobby
            .iter()
            .map(|entry| LobbyPlayerInfo {
                user_id: entry.user_id.to_string(),
                username: entry.username.clone(),
                avatar_url: entry.avatar_url.clone(),
            })
            .collect();

        let message = ServerMessage::LobbyPlayerList { players };

        for entry in channel_lobby.iter() {
            let _ = entry.tx.send(message.clone()).await;
        }
    }
}

/// Handle individual client messages
async fn handle_client_message(
    msg: ClientMessage,
    state: &AppState,
    tx: &mpsc::Sender<ServerMessage>,
    user: &AuthenticatedUser,
    player_context: &Arc<tokio::sync::Mutex<PlayerContext>>,
) -> anyhow::Result<()> {
    match msg {
        ClientMessage::JoinLobby {
            channel_id,
            guild_id,
        } => {
            tracing::info!(
                "User {} ({}) joining lobby for channel: {}, guild: {:?}",
                user.username,
                user.user_id,
                channel_id,
                guild_id
            );

            // Remove from previous lobby if switching channels
            {
                let mut context = player_context.lock().await;
                if let Some(old_channel_id) = &context.channel_id {
                    if old_channel_id != &channel_id {
                        remove_player_from_lobby(state, old_channel_id, user.user_id).await;
                    }
                }
                context.channel_id = Some(channel_id.clone());
            }

            // Fetch avatar from database
            let avatar_url = match db::queries::get_user(
                &state.db,
                user.user_id,
                &state.config.security.encryption_key,
            )
            .await
            {
                Ok(Some(db_user)) => db_user.avatar_url,
                Ok(None) => {
                    tracing::warn!("User {} not found in database", user.user_id);
                    None
                }
                Err(e) => {
                    tracing::error!("Failed to fetch user from database: {}", e);
                    None
                }
            };

            // Add to the new channel lobby
            add_player_to_lobby(state, &channel_id, guild_id, user, avatar_url, tx.clone()).await;
        }
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
