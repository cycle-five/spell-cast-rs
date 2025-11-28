import { initDiscord } from './discord-sdk.js';
import { GameClient } from './websocket.js';
import { GameUI } from './game-ui.js';

class App {
  constructor() {
    this.gameClient = null;
    this.gameUI = null;
    this.currentScreen = 'loading';
    this.channelId = null;
    this.guildId = null;
    this.currentLobbyCode = null;
    this.currentLobbyType = null;
  }

  async init() {
    try {
      console.log('Initializing Spell Cast...');

      // Initialize Discord SDK
      const discordResult = await initDiscord();
      console.log('Discord SDK initialized:', discordResult);

      // Store channel/guild context for lobby scoping
      this.channelId = discordResult.channelId;
      this.guildId = discordResult.guildId;

      // Initialize WebSocket connection with JWT token for authentication
      const wsUrl = this.getWebSocketUrl(discordResult.access_token);
      this.gameClient = new GameClient(wsUrl);

      // Initialize UI
      this.gameUI = new GameUI(this.gameClient);

      // Set up event listeners
      this.setupEventListeners();

      // Show lobby screen
      this.showScreen('lobby');

      console.log('App initialized successfully');
    } catch (error) {
      console.error('Failed to initialize app:', error);
      this.showError('Failed to connect to game server. Please try again.');
    }
  }

  getWebSocketUrl(token) {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;

    // When running inside Discord Activity (discordsays.com), use the proxy path directly
    // Don't rely on patchUrlMappings for WebSocket to avoid double-patching
    const isDiscordActivity = host.includes('discordsays.com');
    const wsPath = isDiscordActivity ? '/.proxy/ws' : '/ws';

    // Append JWT token as query parameter for authentication
    return `${protocol}//${host}${wsPath}?token=${encodeURIComponent(token)}`;
  }

  setupEventListeners() {
    // Lobby - Game mode selection
    document.querySelectorAll('.mode-button').forEach(button => {
      button.addEventListener('click', (e) => {
        const mode = e.currentTarget.dataset.mode;
        this.gameClient.createGame(mode);
      });
    });

    // Custom lobby button - create a new custom lobby
    document.getElementById('create-custom-lobby-btn')?.addEventListener('click', () => {
      console.log('Creating custom lobby...');
      this.gameClient.createCustomLobby();
    });

    // Join custom lobby button
    document.getElementById('join-custom-lobby-btn')?.addEventListener('click', () => {
      const codeInput = document.getElementById('lobby-code-input');
      const code = codeInput?.value?.trim();
      if (code) {
        console.log('Joining custom lobby with code:', code);
        this.gameClient.joinCustomLobby(code);
      } else {
        this.showError('Please enter a lobby code');
      }
    });

    // Game controls
    document.getElementById('clear-btn')?.addEventListener('click', () => {
      this.gameUI.clearSelection();
    });

    document.getElementById('submit-btn')?.addEventListener('click', () => {
      this.gameUI.submitWord();
    });

    document.getElementById('pass-btn')?.addEventListener('click', () => {
      this.gameClient.passTurn();
    });

    // Play again
    document.getElementById('play-again-btn')?.addEventListener('click', () => {
      this.showScreen('lobby');
    });

    // Join lobby when WebSocket connects
    this.gameClient.on('connected', () => {
      console.log('WebSocket connected');
      // If we have a channel context (Discord activity), auto-join the channel lobby
      if (this.channelId) {
        console.log('Auto-joining channel lobby:', this.channelId);
        this.gameClient.joinChannelLobby(this.channelId, this.guildId);
      } else {
        // No channel context (e.g., web client or DM with bot)
        // User can manually create or join a custom lobby
        console.log('No channel context - user can create or join a custom lobby');
        this.showCustomLobbyControls();
      }
    });

    // Handle lobby created response
    this.gameClient.on('lobby_created', (data) => {
      console.log('Custom lobby created with code:', data.lobby_code);
      this.currentLobbyCode = data.lobby_code;
      this.displayLobbyCode(data.lobby_code);
    });

    // Handle lobby joined confirmation
    this.gameClient.on('lobby_joined', (data) => {
      console.log('Joined lobby:', data.lobby_id, 'type:', data.lobby_type);
      this.currentLobbyType = data.lobby_type;
      this.currentLobbyCode = data.lobby_code;

      if (data.lobby_code) {
        this.displayLobbyCode(data.lobby_code);
      } else {
        this.hideLobbyCode();
      }
    });

    // Listen for lobby player list updates
    this.gameClient.on('lobby_player_list', (data) => {
      this.displayLobbyPlayers(data.players);
      // Update lobby code display if provided
      if (data.lobby_code) {
        this.currentLobbyCode = data.lobby_code;
        this.displayLobbyCode(data.lobby_code);
      }
    });

    // Listen for errors
    this.gameClient.on('error', (data) => {
      console.error('Server error:', data.message);
      this.showError(data.message);
    });

    // Listen for game state changes
    this.gameClient.on('game_started', () => {
      this.showScreen('game');
    });

    this.gameClient.on('game_over', () => {
      this.showScreen('results');
    });
  }

  showScreen(screenName) {
    document.querySelectorAll('.screen').forEach(screen => {
      screen.classList.remove('active');
    });

    const screen = document.getElementById(`${screenName}-screen`);
    if (screen) {
      screen.classList.add('active');
      this.currentScreen = screenName;
    }
  }

  showError(message) {
    const toast = document.getElementById('error-toast');
    toast.textContent = message;
    toast.classList.remove('hidden');

    setTimeout(() => {
      toast.classList.add('hidden');
    }, 5000);
  }

  showCustomLobbyControls() {
    const controls = document.getElementById('custom-lobby-controls');
    if (controls) {
      controls.classList.remove('hidden');
    }
  }

  displayLobbyCode(code) {
    const codeDisplay = document.getElementById('lobby-code-display');
    if (codeDisplay) {
      codeDisplay.textContent = `Lobby Code: ${code}`;
      codeDisplay.classList.remove('hidden');
    }
  }

  hideLobbyCode() {
    const codeDisplay = document.getElementById('lobby-code-display');
    if (codeDisplay) {
      codeDisplay.classList.add('hidden');
    }
  }

  displayLobbyPlayers(players) {
    const container = document.getElementById('players-container');
    if (!container) return;

    // Clear container
    container.innerHTML = '';

    // Create a player card for each player
    players.forEach(player => {
      // Use actual avatar_url from server, fallback to Discord default avatar
      const avatarUrl = player.avatar_url ||
        `https://cdn.discordapp.com/embed/avatars/${parseInt(player.user_id) % 5}.png`;

      const playerCard = document.createElement('div');
      playerCard.className = 'player-card';

      const img = document.createElement('img');
      img.src = avatarUrl;
      img.alt = player.username;
      img.className = 'player-avatar';
      // Handle image load errors by falling back to default avatar, only once
      img.onerror = () => {
        if (!img.dataset.fallback) {
          img.dataset.fallback = 'true';
          img.src = `https://cdn.discordapp.com/embed/avatars/${parseInt(player.user_id) % 5}.png`;
        }
      };

      const span = document.createElement('span');
      span.className = 'player-name';
      span.textContent = player.username;

      playerCard.appendChild(img);
      playerCard.appendChild(span);
      container.appendChild(playerCard);
    });
  }
}

// Initialize app when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  const app = new App();
  app.init();
});
