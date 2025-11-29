export class GameUI {
  constructor(gameClient) {
    this.gameClient = gameClient;
    this.selectedTiles = [];
    this.currentGrid = null;
    this.gameId = null;
    this.currentPlayerId = null;
    this.setupListeners();
  }

  setupListeners() {
    // Listen for game state updates
    this.gameClient.on('game_state', (data) => {
      this.handleGameState(data);
    });

    this.gameClient.on('game_started', (data) => {
      this.initializeGame({
        gameId: data.game_id,
        grid: data.grid,
        players: data.players,
        currentPlayerId: data.current_player_id,
        totalRounds: data.total_rounds
      });
    });

    this.gameClient.on('word_scored', (data) => {
      this.handleWordScored(data);
    });

    this.gameClient.on('invalid_word', (data) => {
      this.handleInvalidWord(data);
    });

    this.gameClient.on('turn_update', (data) => {
      this.handleTurnUpdate(data);
    });

    this.gameClient.on('round_end', (data) => {
      this.handleRoundEnd(data);
    });

    this.gameClient.on('game_over', (data) => {
      this.handleGameOver(data);
    });
  }

  handleGameState(data) {
    console.log('Game state received:', data);

    this.currentGrid = data.grid;
    this.renderGrid(data.grid);
    this.renderPlayers(data.players);
    this.renderUsedWords(data.used_words);

    document.getElementById('current-round').textContent = `Round ${data.round}`;
    document.getElementById('max-rounds').textContent = data.max_rounds;
  }

  /**
   * Initialize the game UI with data from the server
   * @param {Object} data - Game initialization data
   * @param {string} data.gameId - Unique identifier for the game
   * @param {Array} data.grid - The 5x5 letter grid
   * @param {Array} data.players - List of players in the game
   * @param {string} data.currentPlayerId - ID of the player who goes first
   * @param {number} data.totalRounds - Total number of rounds
   */
  initializeGame(data) {
    console.log('Initializing game:', data);

    this.gameId = data.gameId;
    this.currentPlayerId = data.currentPlayerId;

    // Hide lobby, show game screen
    document.getElementById('lobby-screen').classList.remove('active');
    document.getElementById('game-screen').classList.add('active');

    this.currentGrid = data.grid;
    this.renderGrid(data.grid);

    // Map GamePlayerInfo to format expected by renderPlayers (needs score)
    const playersWithScore = data.players.map(p => ({
      ...p,
      score: 0 // Initial score
    }));
    this.renderPlayers(playersWithScore);

    document.getElementById('current-round').textContent = 'Round 1';
    document.getElementById('max-rounds').textContent = data.totalRounds;

    // Reset other UI elements
    document.getElementById('used-words-list').innerHTML = '';
    document.getElementById('current-word').textContent = '';
    document.getElementById('word-score').textContent = '0 pts';
    this.selectedTiles = [];
  }

  renderGrid(grid) {
    const boardElement = document.getElementById('game-board');
    boardElement.innerHTML = '';

    grid.forEach((row, rowIdx) => {
      row.forEach((cell, colIdx) => {
        const tile = this.createTile(cell, rowIdx, colIdx);
        boardElement.appendChild(tile);
      });
    });
  }

  createTile(cell, row, col) {
    const tile = document.createElement('div');
    tile.className = 'tile';
    tile.dataset.row = row;
    tile.dataset.col = col;
    tile.dataset.letter = cell.letter;

    if (cell.multiplier) {
      tile.classList.add(cell.multiplier);
    }

    const letterSpan = document.createElement('span');
    letterSpan.className = 'letter';
    letterSpan.textContent = cell.letter;
    tile.appendChild(letterSpan);

    const valueSpan = document.createElement('span');
    valueSpan.className = 'value';
    valueSpan.textContent = cell.value;
    tile.appendChild(valueSpan);

    if (cell.multiplier) {
      const multiplierSpan = document.createElement('span');
      multiplierSpan.className = 'multiplier';
      multiplierSpan.textContent = cell.multiplier;
      tile.appendChild(multiplierSpan);
    }

    tile.addEventListener('click', () => this.selectTile(row, col, tile));

    return tile;
  }

  selectTile(row, col, tileElement) {
    const position = { row, col };

    // Check if already selected
    const alreadySelected = this.selectedTiles.some(
      t => t.row === row && t.col === col
    );

    if (alreadySelected) {
      // Deselect if clicking the last tile
      const lastTile = this.selectedTiles[this.selectedTiles.length - 1];
      if (lastTile.row === row && lastTile.col === col) {
        this.selectedTiles.pop();
        tileElement.classList.remove('selected');
        this.updateWordDisplay();
      }
      return;
    }

    // Check if adjacent to last selected tile
    if (this.selectedTiles.length > 0) {
      const lastTile = this.selectedTiles[this.selectedTiles.length - 1];
      if (!this.isAdjacent(lastTile, position)) {
        console.log('Tile not adjacent');
        return;
      }
    }

    // Add to selection
    this.selectedTiles.push(position);
    tileElement.classList.add('selected');
    this.updateWordDisplay();
  }

  isAdjacent(pos1, pos2) {
    const rowDiff = Math.abs(pos1.row - pos2.row);
    const colDiff = Math.abs(pos1.col - pos2.col);
    return rowDiff <= 1 && colDiff <= 1 && (rowDiff + colDiff > 0);
  }

  updateWordDisplay() {
    const word = this.selectedTiles
      .map(pos => this.currentGrid[pos.row][pos.col].letter)
      .join('');

    document.getElementById('current-word').textContent = word || '';

    // Calculate estimated score
    const score = this.calculateScore();
    document.getElementById('word-score').textContent = `${score} pts`;
  }

  calculateScore() {
    let score = 0;

    this.selectedTiles.forEach(pos => {
      const cell = this.currentGrid[pos.row][pos.col];
      let value = cell.value;

      if (cell.multiplier === 'DL') {
        value *= 2;
      } else if (cell.multiplier === 'TL') {
        value *= 3;
      }

      score += value;
    });

    // Add length bonus
    const length = this.selectedTiles.length;
    if (length >= 4) score += 5;
    if (length >= 5) score += 5;
    if (length >= 6) score += 5;
    if (length >= 7) score += 10;
    if (length >= 8) score += 25;

    return score;
  }

  clearSelection() {
    this.selectedTiles = [];
    document.querySelectorAll('.tile.selected').forEach(tile => {
      tile.classList.remove('selected');
    });
    this.updateWordDisplay();
  }

  submitWord() {
    if (this.selectedTiles.length === 0) {
      return;
    }

    const word = this.selectedTiles
      .map(pos => this.currentGrid[pos.row][pos.col].letter)
      .join('');

    this.gameClient.submitWord(word, this.selectedTiles);
  }

  renderPlayers(players) {
    const container = document.getElementById('player-scores');
    if (!container) return;

    container.innerHTML = players.map(player => `
      <div class="player-score">
        <div class="player-name">${player.username}</div>
        <div class="player-points">${player.score} pts</div>
      </div>
    `).join('');
  }

  renderUsedWords(words) {
    const list = document.getElementById('used-words-list');
    if (!list) return;

    list.innerHTML = words.map(word => `<li>${word}</li>`).join('');
  }

  handleWordScored(data) {
    console.log('Word scored:', data);
    this.clearSelection();
    // TODO: Add animation for scored word
  }

  handleInvalidWord(data) {
    console.log('Invalid word:', data.reason);
    // TODO: Show error message
    this.showError(data.reason);
  }

  handleTurnUpdate(data) {
    const indicator = document.getElementById('turn-indicator');
    if (indicator) {
      indicator.textContent = data.current_player === 'me' ? 'Your Turn!' : "Opponent's Turn";
    }
  }

  handleRoundEnd(data) {
    console.log('Round ended:', data);
    // TODO: Show round results
  }

  handleGameOver(data) {
    console.log('Game over:', data);
    this.renderFinalResults(data);
  }

  renderFinalResults(data) {
    const container = document.getElementById('final-scores');
    const announcement = document.getElementById('winner-announcement');

    if (announcement && data.winner) {
      const winner = data.final_scores.find(s => s.user_id === data.winner);
      announcement.textContent = `🎉 ${winner?.username || 'Player'} wins! 🎉`;
    }

    if (container) {
      container.innerHTML = data.final_scores
        .sort((a, b) => b.score - a.score)
        .map((player, index) => `
          <div class="final-score-row">
            <span class="rank">${index + 1}.</span>
            <span class="name">${player.username}</span>
            <span class="score">${player.score} pts</span>
          </div>
        `).join('');
    }
  }

  showError(message) {
    const toast = document.getElementById('error-toast');
    if (toast) {
      toast.textContent = message;
      toast.classList.remove('hidden');
      setTimeout(() => toast.classList.add('hidden'), 3000);
    }
  }
}
