import { initDiscord } from './discord-sdk.js';
import { GameClient } from './websocket.js';
import { GameUI } from './game-ui.js';

class App {
  constructor() {
    this.gameClient = null;
    this.gameUI = null;
    this.currentScreen = 'loading';
  }

  async init() {
    try {
      console.log('Initializing Spell Cast...');

      // Initialize Discord SDK
      const discordSdk = await initDiscord();
      console.log('Discord SDK initialized:', discordSdk);

      // Initialize WebSocket connection
      const wsUrl = this.getWebSocketUrl();
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

  getWebSocketUrl() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host;
    return `${protocol}//${host}/ws`;
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
}

// Initialize app when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  const app = new App();
  app.init();
});
