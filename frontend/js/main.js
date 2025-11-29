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
      this.gameUI = new GameUI(this.gameClient, discordResult.user.id);

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

    // Start game button (host only)
    document.getElementById('start-game-btn')?.addEventListener('click', () => {
      console.log('Starting game...');
      this.gameClient.startGame();
    });

    // Play again
    document.getElementById('play-again-btn')?.addEventListener('click', () => {
      this.showScreen('lobby');
    });

    // Admin controls
    document.getElementById('toggle-admin-btn')?.addEventListener('click', () => {
      const panel = document.getElementById('admin-panel');
      panel.classList.toggle('hidden');
    });

    document.getElementById('refresh-games-btn')?.addEventListener('click', () => {
      this.gameClient.getAdminGames();
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

    this.gameClient.on('game_error', (data) => {
      console.error('Game error:', data.message);
      this.showError(data.message);
    });

    // Admin games list
    this.gameClient.on('admin_games_list', (data) => {
      this.renderAdminGamesList(data.games);
    });

    // Listen for game state changes
    // game_started is handled by GameUI, which transitions the screen
    // this.gameClient.on('game_started', () => {
    //   this.showScreen('game');
    // });

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

    // Show/hide start game button based on player count and host status
    // Note: We don't have explicit "is_host" flag in player list yet, 
    // but usually the first player or the one who created the lobby is host.
    // For now, let's show it if we have enough players (>= 2).
    // TODO: Add proper host check when backend provides it in player list or separate message
    const startBtn = document.getElementById('start-game-btn');
    if (startBtn) {
      if (players.length >= 2) {
        startBtn.classList.remove('hidden');
      } else {
        startBtn.classList.add('hidden');
      }
    }
  }

  renderAdminGamesList(games) {
    const list = document.getElementById('admin-games-list');
    if (!list) return;

    list.innerHTML = '';
    if (games.length === 0) {
      list.innerHTML = '<p style="color: #aaa; font-size: 0.9rem;">No games found.</p>';
      return;
    }

    games.forEach(game => {
      const item = document.createElement('div');
      item.style.cssText = 'display: flex; justify-content: space-between; align-items: center; padding: 5px; border-bottom: 1px solid #444; font-size: 0.9rem;';

      const info = document.createElement('span');
      info.textContent = `${new Date(game.created_at).toLocaleTimeString()} - ${game.state}`;

      const deleteBtn = document.createElement('button');
      deleteBtn.textContent = 'Delete';
      deleteBtn.style.cssText = 'background: #ED4245; color: white; border: none; padding: 2px 6px; border-radius: 4px; cursor: pointer; font-size: 0.8rem;';
      deleteBtn.onclick = () => {
        this.showConfirmationModal(
          'Delete Game',
          'Are you sure you want to delete this game? This cannot be undone.',
          () => {
            this.gameClient.deleteGame(game.game_id);
            // Refresh list after a short delay
            setTimeout(() => this.gameClient.getAdminGames(), 500);
          }
        );
      };

      item.appendChild(info);
      item.appendChild(deleteBtn);
      list.appendChild(item);
    });
  }

  showConfirmationModal(title, message, onConfirm) {
    const modal = document.getElementById('confirmation-modal');
    const modalTitle = document.getElementById('modal-title');
    const modalMessage = document.getElementById('modal-message');
    const confirmBtn = document.getElementById('modal-confirm-btn');
    const cancelBtn = document.getElementById('modal-cancel-btn');

    if (!modal || !modalTitle || !modalMessage || !confirmBtn || !cancelBtn) return;

    modalTitle.textContent = title;
    modalMessage.textContent = message;

    const closeModal = () => {
      modal.classList.add('hidden');
      // Remove event listeners to prevent memory leaks/multiple firings
      confirmBtn.onclick = null;
      cancelBtn.onclick = null;
    };

    confirmBtn.onclick = () => {
      onConfirm();
      closeModal();
    };

    cancelBtn.onclick = () => {
      closeModal();
    };

    modal.classList.remove('hidden');
  }
}

// Initialize app when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  const app = new App();
  app.init();
});
