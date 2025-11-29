export class GameUI {
  constructor(gameClient, userId) {
    this.gameClient = gameClient;
    this.userId = userId;
    this.selectedTiles = [];
    this.currentGrid = null;
    this.gameId = null;
    this.currentPlayerId = null;
    this.setupListeners();
  }

  setupListeners() {
    // Listen for game state updates
    this.gameClient.on('game_state', (data) => {
      this.safeHandle(() => this.handleGameState(data), 'game_state');
    });

    this.gameClient.on('game_started', (data) => {
      this.safeHandle(() => this.initializeGame({
        gameId: data.game_id,
        grid: data.grid,
        players: data.players,
        currentPlayerId: data.current_player_id,
        totalRounds: data.total_rounds
      }), 'game_started');
    });

    this.gameClient.on('word_scored', (data) => {
      this.safeHandle(() => this.handleWordScored(data), 'word_scored');
    });

    this.gameClient.on('invalid_word', (data) => {
      this.safeHandle(() => this.handleInvalidWord(data), 'invalid_word');
    });

    this.gameClient.on('turn_update', (data) => {
      this.safeHandle(() => this.handleTurnUpdate(data), 'turn_update');
    });

    this.gameClient.on('round_end', (data) => {
      this.safeHandle(() => this.handleRoundEnd(data), 'round_end');
    });

    this.gameClient.on('game_over', (data) => {
      this.safeHandle(() => this.handleGameOver(data), 'game_over');
    });

    this.gameClient.on('grid_update', (data) => {
      this.safeHandle(() => this.handleGridUpdate(data), 'grid_update');
    });

    // Listen for errors from the server
    this.gameClient.on('error', (data) => {
      console.error('Server error:', data);
      this.showError(data.message || 'An error occurred');
    });

    this.gameClient.on('game_error', (data) => {
      console.error('Game error:', data);
      this.showError(data.message || 'Game error occurred');
      this.setSubmitLoading(false);
    });
  }

  /**
   * Safely execute a handler with error catching
   */
  safeHandle(handler, eventName) {
    try {
      handler();
    } catch (error) {
      console.error(`Error handling ${eventName}:`, error);
      this.showError(`Failed to process ${eventName}`);
    }
  }

  handleGameState(data) {
    console.log('Game state received:', data);

    // Clear any pending submit loading state - GameState is the acknowledgement
    this.setSubmitLoading(false);

    this.currentGrid = data.grid;
    this.gameId = data.game_id;
    this.renderGrid(data.grid);
    this.renderPlayers(data.players);
    this.renderUsedWords(data.used_words);

    document.getElementById('current-round').textContent = `Round ${data.round}`;
    document.getElementById('max-rounds').textContent = data.max_rounds;

    // Update turn indicator if current_turn is provided
    if (data.current_turn !== undefined && data.current_turn !== null) {
      this.currentPlayerId = data.current_turn;
      this.updateTurnIndicator(data.current_turn);
    }

    // Clear any existing selection since grid may have changed
    this.selectedTiles = [];
    this.updateWordDisplay();
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
    try {
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

      // Update turn indicator
      this.updateTurnIndicator(this.currentPlayerId);

      // Reset other UI elements
      document.getElementById('used-words-list').innerHTML = '';
      document.getElementById('current-word').textContent = '';
      document.getElementById('word-score').textContent = '0 pts';
      this.selectedTiles = [];
    } catch (error) {
      console.error('Failed to initialize game:', error);
      // We can't easily show a toast here if the UI is broken, but logging helps
    }
  }

  updateTurnIndicator(currentPlayerId) {
    const indicator = document.getElementById('turn-indicator');
    if (indicator) {
      // Check if it's my turn
      // Note: userId might be string or number depending on source, so use loose equality or string conversion
      const isMyTurn = String(currentPlayerId) === String(this.userId);
      indicator.textContent = isMyTurn ? 'Your Turn!' : "Opponent's Turn";

      // Toggle controls based on turn
      const controls = document.querySelector('.game-controls');
      if (controls) {
        if (isMyTurn) {
          controls.classList.remove('disabled');
          controls.style.opacity = '1';
          controls.style.pointerEvents = 'auto';
        } else {
          controls.classList.add('disabled');
          controls.style.opacity = '0.5';
          controls.style.pointerEvents = 'none';
        }
      }
    }
  }

  renderGrid(grid) {
    const boardElement = document.getElementById('game-board');
    boardElement.innerHTML = '';

    // Create SVG overlay for selection path lines
    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.id = 'selection-path-svg';
    svg.classList.add('selection-path-svg');
    boardElement.appendChild(svg);

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

    // Add gem class if cell has a gem
    if (cell.has_gem) {
      tile.classList.add('has-gem');
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

    // Add gem indicator if cell has a gem
    if (cell.has_gem) {
      const gemSpan = document.createElement('span');
      gemSpan.className = 'gem';
      gemSpan.textContent = '💎';
      tile.appendChild(gemSpan);
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
        this.shakeInvalidTile(tileElement);
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

  shakeInvalidTile(tileElement) {
    tileElement.classList.add('shake');
    // Remove shake class after animation completes
    setTimeout(() => {
      tileElement.classList.remove('shake');
    }, 400);
  }

  updateWordDisplay() {
    const word = this.selectedTiles
      .map(pos => this.currentGrid[pos.row][pos.col].letter)
      .join('');

    document.getElementById('current-word').textContent = word || '';

    // Calculate estimated score
    const score = this.calculateScore();
    document.getElementById('word-score').textContent = `${score} pts`;

    // Update SVG path lines
    this.updateSelectionPath();
  }

  updateSelectionPath() {
    const svg = document.getElementById('selection-path-svg');
    if (!svg) return;

    // Clear existing paths
    svg.innerHTML = '';

    if (this.selectedTiles.length < 2) return;

    const boardElement = document.getElementById('game-board');
    const boardRect = boardElement.getBoundingClientRect();

    // Draw lines between consecutive selected tiles
    for (let i = 0; i < this.selectedTiles.length - 1; i++) {
      const from = this.selectedTiles[i];
      const to = this.selectedTiles[i + 1];

      const fromTile = boardElement.querySelector(`[data-row="${from.row}"][data-col="${from.col}"]`);
      const toTile = boardElement.querySelector(`[data-row="${to.row}"][data-col="${to.col}"]`);

      if (!fromTile || !toTile) continue;

      const fromRect = fromTile.getBoundingClientRect();
      const toRect = toTile.getBoundingClientRect();

      // Calculate center points relative to the board
      const x1 = fromRect.left - boardRect.left + fromRect.width / 2;
      const y1 = fromRect.top - boardRect.top + fromRect.height / 2;
      const x2 = toRect.left - boardRect.left + toRect.width / 2;
      const y2 = toRect.top - boardRect.top + toRect.height / 2;

      const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      line.setAttribute('x1', x1);
      line.setAttribute('y1', y1);
      line.setAttribute('x2', x2);
      line.setAttribute('y2', y2);
      line.classList.add('selection-path-line');
      svg.appendChild(line);
    }
  }

  /**
   * Calculate score using SpellCast rules:
   * - DL (Double Letter) multiplies letter value by 2
   * - TL (Triple Letter) multiplies letter value by 3
   * - DW (Double Word) multiplies entire word score by 2
   * - +10 flat bonus for 6+ letter words (not multiplied by DW)
   */
  calculateScore() {
    let letterTotal = 0;
    let hasDoubleWord = false;
    let gemsCollected = 0;

    this.selectedTiles.forEach(pos => {
      const cell = this.currentGrid[pos.row][pos.col];
      let value = cell.value;

      if (cell.multiplier === 'DL') {
        value *= 2;
      } else if (cell.multiplier === 'TL') {
        value *= 3;
      } else if (cell.multiplier === 'DW') {
        hasDoubleWord = true;
        // Letter itself is not multiplied for DW, just the word total
      }

      letterTotal += value;

      // Count gems
      if (cell.has_gem) {
        gemsCollected++;
      }
    });

    // Apply double word multiplier if present
    let wordScore = hasDoubleWord ? letterTotal * 2 : letterTotal;

    // Add length bonus (+10 for 6+ letters, NOT multiplied by DW)
    const length = this.selectedTiles.length;
    if (length >= 6) {
      wordScore += 10;
    }

    // Store gems collected for display (could show this in UI later)
    this.lastGemsCollected = gemsCollected;

    return wordScore;
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

    // Validate minimum word length (3 characters required)
    if (this.selectedTiles.length < 3) {
      this.showError('Word must be at least 3 letters');
      return;
    }

    const word = this.selectedTiles
      .map(pos => this.currentGrid[pos.row][pos.col].letter)
      .join('');

    // Show loading state
    this.setSubmitLoading(true);

    this.gameClient.submitWord(word, this.selectedTiles);
  }

  setSubmitLoading(isLoading) {
    const submitBtn = document.getElementById('submit-btn');
    if (submitBtn) {
      if (isLoading) {
        submitBtn.disabled = true;
        submitBtn.dataset.originalText = submitBtn.textContent;
        submitBtn.textContent = 'Submitting...';
        submitBtn.classList.add('loading');
      } else {
        submitBtn.disabled = false;
        submitBtn.textContent = submitBtn.dataset.originalText || 'Submit';
        submitBtn.classList.remove('loading');
      }
    }
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
    this.setSubmitLoading(false);
    this.clearSelection();
    // TODO: Add animation for scored word
  }

  handleInvalidWord(data) {
    console.log('Invalid word:', data.reason);
    this.setSubmitLoading(false);
    this.showError(data.reason);
  }

  handleTurnUpdate(data) {
    this.currentPlayerId = data.current_player;
    this.updateTurnIndicator(data.current_player);
  }

  handleGridUpdate(data) {
    console.log('Grid update received:', data);

    // Update the current grid
    this.currentGrid = data.grid;

    // Re-render the grid with new letters
    this.renderGrid(data.grid);

    // Clear any selection since tiles have changed
    this.selectedTiles = [];
    this.updateWordDisplay();

    // Optionally animate the replaced positions
    if (data.replaced_positions && data.replaced_positions.length > 0) {
      this.animateReplacedTiles(data.replaced_positions);
    }
  }

  animateReplacedTiles(positions) {
    const boardElement = document.getElementById('game-board');
    positions.forEach(pos => {
      const tile = boardElement.querySelector(`[data-row="${pos.row}"][data-col="${pos.col}"]`);
      if (tile) {
        tile.classList.add('tile-replaced');
        setTimeout(() => {
          tile.classList.remove('tile-replaced');
        }, 600);
      }
    });
  }

  handleRoundEnd(data) {
    console.log('Round ended:', data);
    // TODO: Show round results
  }

  handleGameOver(data) {
    console.log('Game over:', data);
    // Clear any pending submit loading state
    this.setSubmitLoading(false);
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
