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
      console.log('WebSocket connected, joining lobby for channel:', this.channelId);
      if (this.channelId) {
        this.gameClient.joinLobby(this.channelId, this.guildId);
      } else {
        console.warn('No channel ID available, cannot join lobby');
      }
    });

    // Listen for lobby player list updates
    this.gameClient.on('lobby_player_list', (data) => {
      this.displayLobbyPlayers(data.players);
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
      // Handle image load errors by falling back to default avatar
      img.onerror = () => {
        img.src = `https://cdn.discordapp.com/embed/avatars/${parseInt(player.user_id) % 5}.png`;
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
