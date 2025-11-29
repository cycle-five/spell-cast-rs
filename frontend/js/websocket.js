export class GameClient {
  constructor(wsUrl) {
    this.wsUrl = wsUrl;
    this.ws = null;
    this.listeners = new Map();
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    // Heartbeat properties for keeping connection alive through proxies
    this.heartbeatInterval = null;
    this.heartbeatTimeout = null;
    this.lastPongTime = Date.now();
    this.connect();
  }

  connect() {
    console.log('Connecting to WebSocket:', this.wsUrl);

    try {
      this.ws = new WebSocket(this.wsUrl);

      this.ws.onopen = () => {
        console.log('WebSocket connected');
        this.reconnectAttempts = 0;
        this.startHeartbeat();
        this.emit('connected');
      };

      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data);
          console.log('Received message:', message);
          this.handleMessage(message);
        } catch (error) {
          console.error('Failed to parse message:', error);
        }
      };

      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        this.emit('error', error);
      };

      this.ws.onclose = () => {
        console.log('WebSocket closed');
        this.stopHeartbeat();
        this.emit('disconnected');
        this.attemptReconnect();
      };
    } catch (error) {
      console.error('Failed to create WebSocket:', error);
    }
  }

  attemptReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnect attempts reached');
      this.emit('max_reconnect_attempts');
      return;
    }

    this.reconnectAttempts++;
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 10000);

    console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

    setTimeout(() => {
      this.connect();
    }, delay);
  }

  send(message) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    } else {
      console.error('WebSocket not connected');
    }
  }

  handleMessage(message) {
    const { type } = message;
    // Handle heartbeat acknowledgment specially
    if (type === 'heartbeat_ack') {
      this.handleHeartbeatAck();
      return;
    }
    this.emit(type, message);
  }

  // Heartbeat methods to keep connection alive through proxies
  startHeartbeat() {
    this.stopHeartbeat(); // Clear any existing timers
    this.lastPongTime = Date.now();

    this.heartbeatInterval = setInterval(() => {
      this.send({ type: 'heartbeat' });
      // Set timeout to detect if server doesn't respond
      this.heartbeatTimeout = setTimeout(() => {
        console.warn('Heartbeat timeout - connection may be dead');
        this.ws.close(); // Will trigger reconnection
      }, 10000); // 10 second timeout for ack
    }, 20000); // Send heartbeat every 20 seconds
  }

  handleHeartbeatAck() {
    clearTimeout(this.heartbeatTimeout);
    this.heartbeatTimeout = null;
    this.lastPongTime = Date.now();
  }

  stopHeartbeat() {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
    if (this.heartbeatTimeout) {
      clearTimeout(this.heartbeatTimeout);
      this.heartbeatTimeout = null;
    }
  }

  on(event, callback) {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event).push(callback);
  }

  emit(event, data) {
    const callbacks = this.listeners.get(event);
    if (callbacks) {
      callbacks.forEach(callback => callback(data));
    }
  }

  // Lobby actions

  // Join a channel-based lobby (default for Discord activities)
  joinChannelLobby(channelId, guildId = null) {
    this.send({
      type: 'join_channel_lobby',
      channel_id: channelId,
      guild_id: guildId,
    });
  }

  // Create a new custom lobby with a shareable code
  createCustomLobby() {
    this.send({
      type: 'create_custom_lobby',
    });
  }

  // Join an existing custom lobby by its code
  joinCustomLobby(lobbyCode) {
    this.send({
      type: 'join_custom_lobby',
      lobby_code: lobbyCode,
    });
  }

  // Leave the current lobby
  leaveLobby() {
    this.send({
      type: 'leave_lobby',
    });
  }

  // Game actions
  createGame(mode) {
    this.send({
      type: 'create_game',
      mode,
    });
  }

  joinGame(gameId) {
    this.send({
      type: 'join_game',
      game_id: gameId,
    });
  }

  startGame() {
    this.send({
      type: 'start_game',
    });
  }

  submitWord(word, positions) {
    this.send({
      type: 'submit_word',
      word,
      positions,
    });
  }

  passTurn() {
    this.send({
      type: 'pass_turn',
    });
  }

  enableTimer() {
    this.send({
      type: 'enable_timer',
    });
  }

  // Admin actions
  getAdminGames() {
    this.send({
      type: 'admin_get_games',
    });
  }

  deleteGame(gameId) {
    this.send({
      type: 'admin_delete_game',
      game_id: gameId,
    });
  }
}
