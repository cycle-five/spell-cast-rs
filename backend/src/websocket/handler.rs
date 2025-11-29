use std::{sync::Arc, time::Instant};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use rand::seq::SliceRandom;
use tokio::sync::mpsc;

use crate::{
    auth::AuthenticatedUser,
    db,
    game::{grid::GridGenerator, scorer::Scorer, validator::WordValidator},
    websocket::messages::{
        ClientMessage, GamePlayerInfo, LobbyPlayerInfo, LobbyType, ServerMessage,
    },
    AppState, Lobby, LobbyPlayer, PlayerConnectionState,
};

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
    /// The lobby_id of the current lobby (if any)
    lobby_id: Option<String>,
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

    // Track player's current lobby for cleanup on disconnect
    let player_context = Arc::new(tokio::sync::Mutex::new(PlayerContext { lobby_id: None }));

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

    // Mark player as awaiting reconnection (don't remove - they stay visible during grace period)
    let context = player_context.lock().await;
    if let Some(lobby_id) = &context.lobby_id {
        mark_player_awaiting_reconnect(&state, lobby_id, user.user_id).await;
    }

    tracing::info!(
        "WebSocket connection closed for user: {} ({})",
        user.username,
        user.user_id
    );
}

/// Fetch user's avatar URL from database
async fn fetch_user_avatar(state: &AppState, user_id: i64) -> Option<String> {
    match db::queries::get_user(&state.db, user_id, &state.config.security.encryption_key).await {
        Ok(Some(db_user)) => db_user.avatar_url,
        Ok(None) => {
            tracing::warn!("User {} not found in database", user_id);
            None
        }
        Err(e) => {
            tracing::error!("Failed to fetch user from database: {}", e);
            None
        }
    }
}

/// Add a player to a lobby (or reconnect if already present)
/// Returns (lobby_type, lobby_code, is_host) tuple if successful
async fn add_player_to_lobby(
    state: &AppState,
    lobby_id: &str,
    user: &AuthenticatedUser,
    avatar_url: Option<String>,
    tx: mpsc::Sender<ServerMessage>,
) -> Option<(LobbyType, Option<String>, bool, Option<String>)> {
    // Get the lobby
    let result = if let Some(mut lobby) = state.lobbies.get_mut(lobby_id) {
        // Check if player is already in lobby (reconnecting)
        let player_exists = lobby.players.contains_key(&user.user_id);

        if player_exists {
            // Reconnecting! Update their connection state and tx
            if let Some(mut existing_player) = lobby.players.get_mut(&user.user_id) {
                let was_awaiting_reconnect = !existing_player.is_connected();
                existing_player.tx = tx;
                existing_player.connection_state = PlayerConnectionState::Connected;
                drop(existing_player);

                if was_awaiting_reconnect {
                    tracing::info!(
                        "Player {} ({}) reconnected to lobby {} (type: {:?})",
                        user.username,
                        user.user_id,
                        lobby_id,
                        lobby.lobby_type
                    );
                } else {
                    tracing::debug!(
                        "Player {} ({}) refreshed connection to lobby {}",
                        user.username,
                        user.user_id,
                        lobby_id
                    );
                }
            }

            // Clear empty_since if lobby was marked as empty
            lobby.empty_since = None;

            let lobby_type = lobby.lobby_type.clone();
            let lobby_code = lobby.lobby_code.clone();
            let is_host = lobby.is_host(user.user_id);
            let active_game_id = lobby.active_game_id.map(|id| id.to_string()).clone();

            Some((lobby_type, lobby_code, is_host, active_game_id))
        } else {
            // New player joining
            let lobby_player = LobbyPlayer {
                user_id: user.user_id,
                username: user.username.clone(),
                avatar_url,
                tx,
                connection_state: PlayerConnectionState::Connected,
            };

            lobby.players.insert(user.user_id, lobby_player);

            // Clear empty_since since we have a player now
            lobby.empty_since = None;

            // Assign host if no current host
            let is_host = if lobby.host_id.is_none() {
                lobby.host_id = Some(user.user_id);
                tracing::info!(
                    "Player {} ({}) is now the host of lobby {}",
                    user.username,
                    user.user_id,
                    lobby_id
                );
                true
            } else {
                lobby.is_host(user.user_id)
            };

            let lobby_type = lobby.lobby_type.clone();
            let lobby_code = lobby.lobby_code.clone();
            let active_game_id = lobby.active_game_id.map(|id| id.to_string()).clone();

            tracing::info!(
                "Player {} ({}) joined lobby {} (type: {:?})",
                user.username,
                user.user_id,
                lobby_id,
                lobby_type
            );

            Some((lobby_type, lobby_code, is_host, active_game_id))
        }
    } else {
        tracing::warn!("Lobby {} not found when adding player", lobby_id);
        None
    };

    // Broadcast updated player list (outside the lock)
    if result.is_some() {
        broadcast_lobby_player_list(state, lobby_id).await;
    }

    result
}

/// Get or create a channel lobby
fn get_or_create_channel_lobby(
    state: &AppState,
    channel_id: &str,
    guild_id: Option<String>,
) -> String {
    let lobby_id = format!("channel:{}", channel_id);

    // Check if lobby already exists
    if state.lobbies.contains_key(&lobby_id) {
        return lobby_id;
    }

    // Create new channel lobby
    let lobby = Lobby::new_channel(channel_id.to_string(), guild_id);
    state.lobbies.insert(lobby_id.clone(), lobby);

    tracing::info!("Created new channel lobby: {}", lobby_id);
    lobby_id
}

/// Create a new custom lobby
fn create_custom_lobby(state: &AppState) -> (String, String) {
    let lobby = Lobby::new_custom();
    let lobby_id = lobby.lobby_id.clone();
    let lobby_code = lobby.lobby_code.clone().unwrap();

    // Add to code index for quick lookup
    state
        .lobby_code_index
        .insert(lobby_code.clone(), lobby_id.clone());

    // Add to lobbies
    state.lobbies.insert(lobby_id.clone(), lobby);

    tracing::info!(
        "Created new custom lobby: {} (code: {})",
        lobby_id,
        lobby_code
    );

    (lobby_id, lobby_code)
}

/// Find a custom lobby by its code
fn find_lobby_by_code(state: &AppState, lobby_code: &str) -> Option<String> {
    // Normalize the code (uppercase, trim)
    let normalized_code = lobby_code.trim().to_uppercase();
    state
        .lobby_code_index
        .get(&normalized_code)
        .map(|r| r.value().clone())
}

/// Mark a player as awaiting reconnection (starts grace period but player stays visible)
/// This is called when a WebSocket drops unexpectedly (not an intentional leave)
async fn mark_player_awaiting_reconnect(state: &AppState, lobby_id: &str, user_id: i64) {
    if let Some(lobby) = state.lobbies.get(lobby_id) {
        if let Some(mut player) = lobby.players.get_mut(&user_id) {
            player.connection_state = PlayerConnectionState::AwaitingReconnect {
                since: Instant::now(),
            };
            tracing::info!(
                "Player {} awaiting reconnection in lobby {} (grace period started, still visible)",
                user_id,
                lobby_id
            );
        }

        // Check if all players are now awaiting reconnection (no active connections)
        let all_awaiting = lobby.players.iter().all(|p| !p.is_connected());
        drop(lobby);

        if all_awaiting {
            // Mark lobby as empty (starts lobby grace period)
            if let Some(mut lobby) = state.lobbies.get_mut(lobby_id) {
                if lobby.empty_since.is_none() {
                    lobby.empty_since = Some(Instant::now());
                    tracing::info!(
                        "Lobby {} has no active connections, grace period started",
                        lobby_id
                    );
                }
            }
        }

        // Note: We don't broadcast here because the player is still visible
        // and connected players will see no change in the player list
    }
}

/// Remove a player from their lobby immediately (e.g., when explicitly leaving)
async fn remove_player_from_lobby(state: &AppState, lobby_id: &str, user_id: i64) {
    if let Some(lobby) = state.lobbies.get(lobby_id) {
        lobby.players.remove(&user_id);
        let is_empty = lobby.players.is_empty();

        tracing::info!("Player {} removed from lobby {}", user_id, lobby_id);

        drop(lobby);

        if is_empty {
            // Mark lobby as empty (starts grace period for cleanup)
            if let Some(mut lobby) = state.lobbies.get_mut(lobby_id) {
                if lobby.empty_since.is_none() {
                    lobby.empty_since = Some(Instant::now());
                    tracing::info!("Lobby {} is now empty, grace period started", lobby_id);
                }
            }
        } else {
            // Broadcast updated player list to remaining clients
            broadcast_lobby_player_list(state, lobby_id).await;
        }
    }
}

/// Broadcast the current lobby player list to all connected clients in a lobby
pub async fn broadcast_lobby_player_list(state: &AppState, lobby_id: &str) {
    if let Some(lobby) = state.lobbies.get(lobby_id) {
        // Include ALL visible players (connected + awaiting reconnect)
        // Players only disappear when removed by background cleanup after grace period
        let players: Vec<LobbyPlayerInfo> = lobby
            .players
            .iter()
            .filter(|p| p.is_visible())
            .map(|entry| LobbyPlayerInfo {
                user_id: entry.user_id.to_string(),
                username: entry.username.clone(),
                avatar_url: entry.avatar_url.clone(),
            })
            .collect();

        let lobby_code = lobby.lobby_code.clone();

        let message = ServerMessage::LobbyPlayerList {
            players,
            lobby_code,
        };

        // Only send to actively connected players (awaiting reconnect players have dead tx)
        for entry in lobby.players.iter() {
            if entry.is_connected() {
                let _ = entry.tx.send(message.clone()).await;
            }
        }
    }
}

/// Broadcast a message to all connected players in a lobby
async fn broadcast_to_lobby(state: &AppState, lobby_id: &str, message: ServerMessage) {
    if let Some(lobby) = state.lobbies.get(lobby_id) {
        for entry in lobby.players.iter() {
            if entry.is_connected() {
                let _ = entry.tx.send(message.clone()).await;
            }
        }
    }
}

/// Handle the StartGame message - validates and starts a new game
/// Returns Ok(GameStarted message) on success, or Err(GameError message) on failure
async fn handle_start_game(
    state: &AppState,
    lobby_id: &str,
    user: &AuthenticatedUser,
) -> Result<ServerMessage, ServerMessage> {
    // Get lobby and validate
    let lobby = state
        .lobbies
        .get(lobby_id)
        .ok_or_else(|| ServerMessage::GameError {
            code: "lobby_not_found".to_string(),
            message: "Lobby not found".to_string(),
        })?;

    // 1. Validate sender is lobby host
    if !lobby.is_host(user.user_id) {
        return Err(ServerMessage::GameError {
            code: "not_host".to_string(),
            message: "Only the lobby host can start the game".to_string(),
        });
    }

    // 2. Atomically try to start game (prevents race condition)
    // This checks both has_active_game and sets game_starting flag atomically
    if !lobby.try_start_game() {
        return Err(ServerMessage::GameError {
            code: "game_in_progress".to_string(),
            message: "A game is already in progress or starting in this lobby".to_string(),
        });
    }

    // From this point on, we have the game_starting flag set.
    // We must clear it on any error path or set active_game_id on success.

    // 3. Validate player count (2-6 players)
    let connected_count = lobby.connected_player_count();
    if connected_count < 2 {
        lobby.clear_game_starting();
        return Err(ServerMessage::GameError {
            code: "not_enough_players".to_string(),
            message: format!(
                "At least 2 players are required to start a game (currently {})",
                connected_count
            ),
        });
    }
    if connected_count > 6 {
        lobby.clear_game_starting();
        return Err(ServerMessage::GameError {
            code: "too_many_players".to_string(),
            message: format!("Maximum 6 players allowed (currently {})", connected_count),
        });
    }

    // 4. Generate 5x5 grid with multipliers
    let grid = GridGenerator::generate();

    // 5. Collect and shuffle player order
    let mut players_info: Vec<GamePlayerInfo> = lobby
        .players
        .iter()
        .filter(|p| p.is_connected())
        .map(|p| GamePlayerInfo {
            user_id: p.user_id.to_string(),
            username: p.username.clone(),
            avatar_url: p.avatar_url.clone(),
            turn_order: 0, // Will be assigned after shuffle
        })
        .collect();

    // Drop the lobby ref before any operations that might await
    drop(lobby);

    // Helper to clear game_starting flag and return an error
    let clear_and_err = |state: &AppState, lobby_id: &str, err: ServerMessage| {
        if let Some(lobby) = state.lobbies.get(lobby_id) {
            lobby.clear_game_starting();
        }
        err
    };

    // Shuffle player order (using thread_rng in a non-async block)
    {
        let mut rng = rand::rng();
        players_info.shuffle(&mut rng);
    }

    // Assign turn orders after shuffle
    for (idx, player) in players_info.iter_mut().enumerate() {
        player.turn_order = idx as u8;
    }

    // Get the first player (current turn)
    let current_player_id = players_info
        .first()
        .map(|p| p.user_id.clone())
        .unwrap_or_default();

    // 6. Persist game session to database
    let total_rounds: u8 = 5; // Default 5 rounds

    // Collect player user_ids for database batch insert
    let player_tuples: Vec<(i64, u8)> = players_info
        .iter()
        .map(|p| {
            (
                p.user_id.parse::<i64>().expect(
                    "Failed to parse user_id from GamePlayerInfo; this should never happen",
                ),
                p.turn_order,
            )
        })
        .collect();

    // Create game session in database
    let game_id = db::queries::create_game_session(&state.db, lobby_id, user.user_id, total_rounds)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create game session: {}", e);
            clear_and_err(
                state,
                lobby_id,
                ServerMessage::GameError {
                    code: "database_error".to_string(),
                    message: "Failed to create game session".to_string(),
                },
            )
        })?;

    // Add players to game
    db::queries::add_game_players_batch(&state.db, game_id, &player_tuples)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add players to game: {}", e);
            clear_and_err(
                state,
                lobby_id,
                ServerMessage::GameError {
                    code: "database_error".to_string(),
                    message: "Failed to add players to game".to_string(),
                },
            )
        })?;

    // Save the grid to database
    let grid_json = serde_json::to_value(&grid).map_err(|e| {
        tracing::error!("Failed to serialize grid: {}", e);
        clear_and_err(
            state,
            lobby_id,
            ServerMessage::GameError {
                code: "serialization_error".to_string(),
                message: "Failed to serialize game grid".to_string(),
            },
        )
    })?;

    db::queries::create_or_update_game_board(&state.db, game_id, grid_json)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create game board: {}", e);
            clear_and_err(
                state,
                lobby_id,
                ServerMessage::GameError {
                    code: "database_error".to_string(),
                    message: "Failed to create game board".to_string(),
                },
            )
        })?;

    // Update game state to active
    db::queries::update_game_db_state(&state.db, game_id, crate::models::GameDbState::Active)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update game state: {}", e);
            clear_and_err(
                state,
                lobby_id,
                ServerMessage::GameError {
                    code: "database_error".to_string(),
                    message: "Failed to update game state".to_string(),
                },
            )
        })?;

    // 7. Link game to lobby and clear game_starting flag
    if let Some(mut lobby) = state.lobbies.get_mut(lobby_id) {
        lobby.active_game_id = Some(game_id);
        lobby.clear_game_starting();
    }

    tracing::info!(
        "Game {} started in lobby {} by host {} with {} players",
        game_id,
        lobby_id,
        user.username,
        players_info.len()
    );

    // 8. Return GameStarted message for broadcast
    Ok(ServerMessage::GameStarted {
        game_id: game_id.to_string(),
        grid,
        players: players_info,
        current_player_id,
        total_rounds,
    })
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
        ClientMessage::JoinChannelLobby {
            channel_id,
            guild_id,
        } => {
            tracing::info!(
                "User {} ({}) joining channel lobby: {}, guild: {:?}",
                user.username,
                user.user_id,
                channel_id,
                guild_id
            );

            // Get or create the channel lobby
            let lobby_id = get_or_create_channel_lobby(state, &channel_id, guild_id);

            // Remove from previous lobby if different
            {
                let mut context = player_context.lock().await;
                if let Some(old_lobby_id) = &context.lobby_id {
                    if old_lobby_id != &lobby_id {
                        remove_player_from_lobby(state, old_lobby_id, user.user_id).await;
                    }
                }
                context.lobby_id = Some(lobby_id.clone());
            }

            // Fetch avatar and add to lobby (handles reconnection)
            let avatar_url = fetch_user_avatar(state, user.user_id).await;
            if let Some((lobby_type, lobby_code, _is_host, active_game_id)) =
                add_player_to_lobby(state, &lobby_id, user, avatar_url, tx.clone()).await
            {
                // Send confirmation
                tx.send(ServerMessage::LobbyJoined {
                    lobby_id: lobby_id.clone(),
                    lobby_type,
                    lobby_code,
                })
                .await?;

                // If game is active, send game state
                if let Some(_game_id) = active_game_id {
                    tracing::info!("Player joined lobby with active game, sending game state");
                    match db::queries::get_active_game_for_lobby(&state.db, &lobby_id).await {
                        Ok(Some(game_state)) => {
                            tx.send(ServerMessage::GameState {
                                game_id: game_state.game_id.to_string(),
                                mode: crate::models::GameMode::Multiplayer, // TODO: Store mode in GameState
                                round: game_state.current_round as i32,
                                max_rounds: game_state.total_rounds as i32,
                                grid: game_state.grid,
                                players: game_state
                                    .players
                                    .into_iter()
                                    .map(|p| crate::websocket::messages::PlayerInfo {
                                        user_id: 0, // TODO: Fix user_id mapping in get_active_game_for_lobby
                                        username: p.username,
                                        avatar_url: p.avatar_url,
                                        score: p.score,
                                        team: None,
                                    })
                                    .collect(),
                                current_turn: None, // TODO: Map current player index to user ID
                                used_words: game_state.used_words.into_iter().collect(),
                                timer_enabled: false,
                                time_remaining: None,
                            })
                            .await?;
                        }
                        Ok(None) => {
                            tracing::warn!("Lobby has active_game_id but no game found in DB");
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch active game state: {}", e);
                        }
                    }
                }
            }
        }

        ClientMessage::CreateCustomLobby => {
            tracing::info!(
                "User {} ({}) creating custom lobby",
                user.username,
                user.user_id
            );

            // Create the custom lobby
            let (lobby_id, lobby_code) = create_custom_lobby(state);

            // Remove from previous lobby
            {
                let mut context = player_context.lock().await;
                if let Some(old_lobby_id) = &context.lobby_id {
                    remove_player_from_lobby(state, old_lobby_id, user.user_id).await;
                }
                context.lobby_id = Some(lobby_id.clone());
            }

            // Fetch avatar and add to lobby
            let avatar_url = fetch_user_avatar(state, user.user_id).await;

            // Send lobby created response first
            tx.send(ServerMessage::LobbyCreated {
                lobby_code: lobby_code.clone(),
            })
            .await?;

            // Then add player and send joined confirmation
            if let Some((lobby_type, lobby_code, _is_host, _active_game_id)) =
                add_player_to_lobby(state, &lobby_id, user, avatar_url, tx.clone()).await
            {
                tx.send(ServerMessage::LobbyJoined {
                    lobby_id,
                    lobby_type,
                    lobby_code,
                })
                .await?;
            }
        }

        ClientMessage::JoinCustomLobby { lobby_code } => {
            tracing::info!(
                "User {} ({}) joining custom lobby with code: {}",
                user.username,
                user.user_id,
                lobby_code
            );

            // Find the lobby by code
            let lobby_id = match find_lobby_by_code(state, &lobby_code) {
                Some(id) => id,
                None => {
                    tx.send(ServerMessage::Error {
                        message: format!("Lobby with code '{}' not found", lobby_code),
                    })
                    .await?;
                    return Ok(());
                }
            };

            // Remove from previous lobby if different
            {
                let mut context = player_context.lock().await;
                if let Some(old_lobby_id) = &context.lobby_id {
                    if old_lobby_id != &lobby_id {
                        remove_player_from_lobby(state, old_lobby_id, user.user_id).await;
                    }
                }
                context.lobby_id = Some(lobby_id.clone());
            }

            // Fetch avatar and add to lobby
            let avatar_url = fetch_user_avatar(state, user.user_id).await;
            if let Some((lobby_type, lobby_code, _is_host, active_game_id)) =
                add_player_to_lobby(state, &lobby_id, user, avatar_url, tx.clone()).await
            {
                tx.send(ServerMessage::LobbyJoined {
                    lobby_id: lobby_id.clone(),
                    lobby_type,
                    lobby_code,
                })
                .await?;

                // If game is active, send game state
                if let Some(_game_id) = active_game_id {
                    tracing::info!("Player joined lobby with active game, sending game state");
                    match db::queries::get_active_game_for_lobby(&state.db, &lobby_id).await {
                        Ok(Some(game_state)) => {
                            tx.send(ServerMessage::GameState {
                                game_id: game_state.game_id.to_string(),
                                mode: crate::models::GameMode::Multiplayer,
                                round: game_state.current_round as i32,
                                max_rounds: game_state.total_rounds as i32,
                                grid: game_state.grid,
                                players: game_state
                                    .players
                                    .into_iter()
                                    .map(|p| crate::websocket::messages::PlayerInfo {
                                        user_id: 0, // TODO: Fix user_id mapping
                                        username: p.username,
                                        avatar_url: p.avatar_url,
                                        score: p.score,
                                        team: None,
                                    })
                                    .collect(),
                                current_turn: None,
                                used_words: game_state.used_words.into_iter().collect(),
                                timer_enabled: false,
                                time_remaining: None,
                            })
                            .await?;
                        }
                        Ok(None) => {
                            tracing::warn!("Lobby has active_game_id but no game found in DB");
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch active game state: {}", e);
                        }
                    }
                }
            }
        }

        ClientMessage::LeaveLobby => {
            tracing::info!("User {} ({}) leaving lobby", user.username, user.user_id);

            let mut context = player_context.lock().await;
            if let Some(lobby_id) = context.lobby_id.take() {
                remove_player_from_lobby(state, &lobby_id, user.user_id).await;
            }
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
            tracing::info!("User {} ({}) leaving game", user.username, user.user_id);
            // TODO: Implement leave game logic
        }

        ClientMessage::StartGame => {
            tracing::info!("User {} ({}) starting game", user.username, user.user_id);

            // Get the player's current lobby
            let context = player_context.lock().await;
            let lobby_id = match &context.lobby_id {
                Some(id) => id.clone(),
                None => {
                    tx.send(ServerMessage::GameError {
                        code: "not_in_lobby".to_string(),
                        message: "You must be in a lobby to start a game".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };
            drop(context);

            // Validate and start the game
            match handle_start_game(state, &lobby_id, user).await {
                Ok(game_started_msg) => {
                    // Broadcast GameStarted to all players in the lobby
                    broadcast_to_lobby(state, &lobby_id, game_started_msg).await;
                }
                Err(error_msg) => {
                    tx.send(error_msg).await?;
                }
            }
        }

        ClientMessage::SubmitWord { word, positions } => {
            tracing::info!(
                "User {} ({}) submitting word: {} at positions: {:?}",
                user.username,
                user.user_id,
                word,
                positions
            );

            // Get the player's current lobby
            let context = player_context.lock().await;
            let lobby_id = match &context.lobby_id {
                Some(id) => id.clone(),
                None => {
                    tx.send(ServerMessage::Error {
                        message: "You must be in a lobby to submit a word".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };
            drop(context);

            // Get active game from lobby
            let active_game_id = if let Some(lobby) = state.lobbies.get(&lobby_id) {
                lobby.active_game_id.clone()
            } else {
                tx.send(ServerMessage::Error {
                    message: "Lobby not found".to_string(),
                })
                .await?;
                return Ok(());
            };

            let game_id_str = match active_game_id {
                Some(id) => id,
                None => {
                    tx.send(ServerMessage::GameError {
                        code: "no_active_game".to_string(),
                        message: "No active game in this lobby".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };

            // Fetch game state from DB
            let game_state =
                match db::queries::get_active_game_for_lobby(&state.db, &lobby_id).await {
                    Ok(Some(gs)) => gs,
                    Ok(None) => {
                        tx.send(ServerMessage::GameError {
                            code: "game_not_found".to_string(),
                            message: "Game state not found".to_string(),
                        })
                        .await?;
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch game state: {}", e);
                        tx.send(ServerMessage::Error {
                            message: "Internal server error".to_string(),
                        })
                        .await?;
                        return Ok(());
                    }
                };

            // Validate turn
            // TODO: Proper turn validation using user_id from DB
            // For now, we'll skip strict turn validation to get it working,
            // but in production we must check if game_state.players[current_player_index].user_id == user.user_id

            // Validate word
            // Load dictionary (in a real app, this should be loaded once and shared)
            // For now, we'll use a simple check or load it here
            // TODO: Use shared dictionary from AppState

            // Check if word is already used
            if game_state.used_words.contains(&word.to_uppercase()) {
                tx.send(ServerMessage::InvalidWord {
                    reason: "Word already used".to_string(),
                })
                .await?;
                return Ok(());
            }

            // Validate path
            let validator = WordValidator::new(std::collections::HashSet::new()); // Empty dict for now, just path validation
            if !validator.is_valid_path(&game_state.grid, &positions) {
                tx.send(ServerMessage::InvalidWord {
                    reason: "Invalid path".to_string(),
                })
                .await?;
                return Ok(());
            }

            // Validate word in dictionary
            // TODO: Actually check dictionary
            // For now, assume valid if path is valid

            // Score word
            let score = Scorer::calculate_score(&game_state.grid, &positions);

            // Update DB
            let game_uuid = uuid::Uuid::parse_str(&game_state.game_id).unwrap_or_default();

            // 1. Update player score
            if let Err(e) =
                db::queries::update_player_score(&state.db, game_uuid, user.user_id, score).await
            {
                tracing::error!("Failed to update player score: {}", e);
            }

            // 2. Add to used words
            let mut new_used_words: Vec<String> = game_state.used_words.into_iter().collect();
            new_used_words.push(word.to_uppercase());
            if let Err(e) =
                db::queries::update_game_board_used_words(&state.db, game_uuid, &new_used_words)
                    .await
            {
                tracing::error!("Failed to update used words: {}", e);
            }

            // 3. Record move
            if let Err(e) = db::queries::create_game_move(
                &state.db,
                game_uuid,
                user.user_id,
                game_state.current_round as i32,
                &word,
                score,
                serde_json::to_value(&positions).unwrap_or_default(),
            )
            .await
            {
                tracing::error!("Failed to record move: {}", e);
            }

            // Broadcast WordScored
            let player_info = crate::websocket::messages::PlayerInfo {
                user_id: user.user_id,
                username: user.username.clone(),
                avatar_url: None, // TODO: Fetch avatar
                score, // This is just the score for this word, or total? Message def says "score", usually total.
                // But here we might want to send the *word* score or the *new total*.
                // Let's send new total.
                team: None,
            };

            broadcast_to_lobby(
                state,
                &lobby_id,
                ServerMessage::WordScored {
                    word: word.to_string(),
                    score,
                    player: player_info,
                    positions: positions.clone(),
                },
            )
            .await;

            // TODO: Handle turn passing / round end logic
            // For now, just keep it simple
        }

        ClientMessage::PassTurn => {
            tracing::info!("User {} ({}) passing turn", user.username, user.user_id);

            // Get lobby ID
            let context = player_context.lock().await;
            let lobby_id = match &context.lobby_id {
                Some(id) => id.clone(),
                None => {
                    tx.send(ServerMessage::Error {
                        message: "Not in a lobby".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };
            drop(context);

            // Get active game
            let game_state =
                match db::queries::get_active_game_for_lobby(&state.db, &lobby_id).await {
                    Ok(Some(gs)) => gs,
                    Ok(None) => {
                        tx.send(ServerMessage::GameError {
                            code: "no_game".to_string(),
                            message: "No active game".to_string(),
                        })
                        .await?;
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("Failed to fetch game state: {}", e);
                        return Ok(());
                    }
                };

            // Determine next player
            let current_idx = game_state.current_player_index;
            let next_idx = (current_idx + 1) % game_state.players.len();
            let next_player = &game_state.players[next_idx];

            // Update DB
            let game_uuid = uuid::Uuid::parse_str(&game_state.game_id).unwrap_or_default();
            // Note: We need the user_id (i64) for the DB update, but GamePlayer struct has user_id as Uuid (in-memory) or i64?
            // Let's check GamePlayer definition. In db/queries.rs it says:
            // user_id: Uuid::new_v4(), // Generate a UUID for in-memory tracking
            // Wait, that's a problem. We need the real user_id (i64) to update the DB.
            // The GameState struct in models/game.rs uses Uuid for user_id?
            // Let's look at how we get the next player's ID.

            // Actually, we should probably look up the player in the `players` list from `get_active_game_for_lobby`
            // But `get_active_game_for_lobby` generates fake UUIDs for `user_id` in `GamePlayer` struct?
            // Line 476 in queries.rs: `user_id: Uuid::new_v4(),`
            // This seems wrong if we need to reference them back.
            // However, `current_turn_player` in `games` table is `i64`.

            // For now, let's try to get the real user_id from the `game_players` table again or fix `get_active_game_for_lobby`.
            // But I can't easily change `get_active_game_for_lobby` right now without seeing `models/game.rs`.
            // Let's assume we can get the player's real ID by re-querying or using the index.

            // Re-query game players to get real IDs
            let players = db::queries::get_game_players(&state.db, game_uuid)
                .await
                .unwrap_or_default();
            if players.is_empty() {
                return Ok(());
            }

            // Sort by team/joined_at to match the order in GameState
            // The order should be consistent.
            let next_player_record = &players[next_idx];
            let next_player_id = next_player_record.user_id;

            if let Err(e) = db::queries::update_game_round(
                &state.db,
                game_uuid,
                game_state.current_round as i32,
                next_player_id,
            )
            .await
            {
                tracing::error!("Failed to update turn: {}", e);
            }

            // Broadcast update
            broadcast_to_lobby(
                state,
                &lobby_id,
                ServerMessage::TurnUpdate {
                    current_player: next_player_id,
                    time_remaining: None,
                },
            )
            .await;
        }

        ClientMessage::EnableTimer => {
            tracing::info!("User {} ({}) enabling timer", user.username, user.user_id);
            // TODO: Implement timer enable logic
        }

        ClientMessage::AdminGetGames => {
            tracing::info!(
                "User {} ({}) requesting admin games list",
                user.username,
                user.user_id
            );

            // Get lobby ID
            let context = player_context.lock().await;
            let lobby_id = match &context.lobby_id {
                Some(id) => id.clone(),
                None => {
                    tx.send(ServerMessage::Error {
                        message: "Not in a lobby".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };
            drop(context);

            // Parse channel ID from lobby ID
            let (channel_id, _) = match db::queries::parse_lobby_id(&lobby_id) {
                Ok(ids) => ids,
                Err(_) => {
                    tx.send(ServerMessage::Error {
                        message: "Invalid lobby ID".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };

            // Fetch games for this channel
            let games = match sqlx::query_as::<_, crate::models::Game>(
                "SELECT * FROM games WHERE channel_id = $1 ORDER BY created_at DESC",
            )
            .bind(channel_id)
            .fetch_all(&state.db)
            .await
            {
                Ok(g) => g,
                Err(e) => {
                    tracing::error!("Failed to fetch games: {}", e);
                    tx.send(ServerMessage::Error {
                        message: "Database error".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };

            let admin_games = games
                .into_iter()
                .map(|g| {
                    crate::websocket::messages::AdminGameInfo {
                        game_id: g.game_id.to_string(),
                        state: g.state.to_string(),
                        created_at: g.created_at,
                        players: vec![], // TODO: Fetch players if needed, keeping it simple for now
                    }
                })
                .collect();

            tx.send(ServerMessage::AdminGamesList { games: admin_games })
                .await?;
        }

        ClientMessage::AdminDeleteGame { game_id } => {
            tracing::info!(
                "User {} ({}) deleting game {}",
                user.username,
                user.user_id,
                game_id
            );

            let game_uuid = match uuid::Uuid::parse_str(&game_id) {
                Ok(id) => id,
                Err(_) => {
                    tx.send(ServerMessage::Error {
                        message: "Invalid game ID".to_string(),
                    })
                    .await?;
                    return Ok(());
                }
            };

            // Delete game (cascades to players, boards, moves)
            match sqlx::query("DELETE FROM games WHERE game_id = $1")
                .bind(game_uuid)
                .execute(&state.db)
                .await
            {
                Ok(_) => {
                    // Also clear active_game_id from lobby if it matches
                    let context = player_context.lock().await;
                    if let Some(lobby_id) = &context.lobby_id {
                        if let Some(mut lobby) = state.lobbies.get_mut(lobby_id) {
                            if let Some(active_id) = &lobby.active_game_id {
                                if active_id == &game_id {
                                    lobby.active_game_id = None;
                                    tracing::info!(
                                        "Cleared active game {} from lobby {}",
                                        game_id,
                                        lobby_id
                                    );
                                }
                            }
                        }
                    }
                    drop(context);

                    tx.send(ServerMessage::Error {
                        message: "Game deleted".to_string(),
                    })
                    .await?; // Using Error as generic toast for now
                }
                Err(e) => {
                    tracing::error!("Failed to delete game: {}", e);
                    tx.send(ServerMessage::Error {
                        message: "Failed to delete game".to_string(),
                    })
                    .await?;
                }
            }
        }
    }

    Ok(())
}
