<script lang="ts">
	import { onMount } from 'svelte';
	import { Chess } from 'chess.js';
	import Board from '$lib/components/Board.svelte';
	import EvalBar from '$lib/components/EvalBar.svelte';
	import BestMoves from '$lib/components/BestMoves.svelte';
	import { Engine, type MultiPvLine } from '$lib/utils/engine';
	import { makeEngineArrows } from '$lib/utils/shapes';
	import { toDests, turnColor, STARTING_FEN } from '$lib/utils/chess';
	import type { Config } from 'chessground/config';

	interface HistoryEntry {
		fen: string;
		san: string;
		uci: string;
	}

	interface PuzzlePosition {
		fen: string;
		label: string;
		moves: HistoryEntry[];
	}

	let startFen = $state(STARTING_FEN);
	let currentFen = $state(STARTING_FEN);
	let moveHistory = $state<HistoryEntry[]>([]);
	let historyIndex = $state(-1);
	let orientation = $state<'white' | 'black'>('white');
	let pgnText = $state('');
	let evalScore = $state<number | null>(null);
	let evalMate = $state<number | null>(null);
	let showBestMoves = $state(false);
	let showEval = $state(true);
	let engineLines = $state<MultiPvLine[]>([]);
	let engine: Engine | null = null;
	let engineReady = $state(false);
	let computerEnabled = $state(false);
	let computerThinking = $state(false);
	let moveSource = $state<'user' | 'nav' | 'init'>('init');

	let puzzles = $state<PuzzlePosition[]>([]);
	let puzzleIndex = $state(-1);

	let boardConfig = $derived.by(() => {
		const chess = new Chess(currentFen);
		const color = turnColor(currentFen);
		const isComputerTurn = computerEnabled && color !== orientation;
		return {
			fen: currentFen,
			orientation,
			turnColor: color,
			lastMove: historyIndex >= 0 ? [moveHistory[historyIndex].uci.slice(0, 2), moveHistory[historyIndex].uci.slice(2, 4)] : [],
			movable: {
				color: isComputerTurn ? undefined : color,
				free: false,
				dests: isComputerTurn ? new Map() : toDests(chess)
			}
		} as Config;
	});

	let numberedMoves = $derived.by(() => {
		const parts = startFen.split(' ');
		const blackToMove = parts[1] === 'b';
		const startMoveNum = parseInt(parts[5] || '1');

		const result: { moveNum: number; white?: { san: string; idx: number }; black?: { san: string; idx: number } }[] = [];
		for (let i = 0; i < moveHistory.length; i++) {
			// Offset by 1 if start position is black to move
			const adjusted = blackToMove ? i + 1 : i;
			const moveNum = Math.floor(adjusted / 2) + startMoveNum;

			if (adjusted % 2 === 0) {
				// White's move
				result.push({ moveNum, white: { san: moveHistory[i].san, idx: i } });
			} else {
				// Black's move
				if (i === 0 && blackToMove) {
					// First move is black — create row with empty white
					result.push({ moveNum, black: { san: moveHistory[i].san, idx: i } });
				} else {
					result[result.length - 1].black = { san: moveHistory[i].san, idx: i };
				}
			}
		}
		return result;
	});

	onMount(() => {
		engine = new Engine();
		engine.init().then(() => { engineReady = true; }).catch(() => {});

		return () => {
			engine?.destroy();
		};
	});

	$effect(() => {
		if (!showEval && !showBestMoves) {
			evalScore = null;
			evalMate = null;
			engineLines = [];
			return;
		}
		if (engineReady && engine && currentFen) {
			if (showBestMoves) {
				engine.evaluateMultiPv(currentFen, 5, (result) => {
					engineLines = result.lines;
					if (result.lines.length > 0) {
						evalScore = result.lines[0].score;
						evalMate = result.lines[0].mate;
					}
				});
			} else {
				engineLines = [];
				engine.evaluate(currentFen, (result) => {
					evalScore = result.score;
					evalMate = result.mate;
				});
			}
		}
	});

	$effect(() => {
		if (!computerEnabled || !engineReady || !engine) return;
		const fen = currentFen;
		const color = turnColor(fen);
		if (color === orientation) return;
		if (moveSource !== 'user') return;

		computerThinking = true;
		let cancelled = false;
		let bestMoveUci = '';

		engine.evaluate(fen, (result) => {
			if (cancelled) return;
			if (result.pv) bestMoveUci = result.pv.split(' ')[0];
			if (result.depth >= 12 && bestMoveUci) {
				cancelled = true;
				setTimeout(() => {
					if (computerEnabled) {
						handleMove(bestMoveUci.slice(0, 2), bestMoveUci.slice(2, 4));
					}
					computerThinking = false;
				}, 300);
			}
		}, 12);

		return () => {
			cancelled = true;
			computerThinking = false;
		};
	});

	let engineArrows = $derived(makeEngineArrows(engineLines, orientation));

	function handleMove(orig: string, dest: string) {
		const chess = new Chess(currentFen);
		const move = chess.move({ from: orig, to: dest, promotion: 'q' });
		if (!move) return;

		// Truncate forward history if navigating from mid-history
		if (historyIndex < moveHistory.length - 1) {
			moveHistory = moveHistory.slice(0, historyIndex + 1);
		}

		const entry: HistoryEntry = {
			fen: chess.fen(),
			san: move.san,
			uci: orig + dest + (move.promotion || '')
		};

		moveHistory = [...moveHistory, entry];
		historyIndex = moveHistory.length - 1;
		currentFen = chess.fen();
		moveSource = 'user';
		updatePgn();
	}

	function navigateToIndex(idx: number) {
		if (idx < -1 || idx >= moveHistory.length) return;
		historyIndex = idx;
		currentFen = idx === -1 ? startFen : moveHistory[idx].fen;
		moveSource = 'nav';
	}

	function goBack() {
		if (computerThinking) return;
		if (historyIndex >= 0) navigateToIndex(historyIndex - 1);
	}

	function goForward() {
		if (computerThinking) return;
		if (historyIndex < moveHistory.length - 1) navigateToIndex(historyIndex + 1);
	}

	function goToStart() {
		if (computerThinking) return;
		navigateToIndex(-1);
	}

	function goToEnd() {
		if (computerThinking) return;
		if (moveHistory.length > 0) navigateToIndex(moveHistory.length - 1);
	}

	function flipBoard() {
		orientation = orientation === 'white' ? 'black' : 'white';
	}

	function resetBoard() {
		moveHistory = [];
		historyIndex = -1;
		startFen = STARTING_FEN;
		currentFen = STARTING_FEN;
		orientation = 'white';
		pgnText = '';
		puzzles = [];
		puzzleIndex = -1;
		moveSource = 'init';
	}

	function updatePgn() {
		const chess = new Chess(startFen);
		for (const entry of moveHistory) {
			chess.move(entry.san);
		}
		pgnText = chess.pgn();
	}

	function parsePgnGames(text: string): PuzzlePosition[] {
		// Split into individual games by looking for header blocks
		const games: PuzzlePosition[] = [];
		// Split on blank lines before a header tag
		const gameTexts = text.replace(/\r\n/g, '\n').split(/\n\n(?=\[)/);

		for (const gameText of gameTexts) {
			const trimmed = gameText.trim();
			if (!trimmed) continue;

			// Extract FEN from headers
			const fenMatch = trimmed.match(/\[FEN\s+"([^"]+)"\]/);
			const whiteMatch = trimmed.match(/\[White\s+"([^"]+)"\]/);
			const fen = fenMatch ? fenMatch[1] : STARTING_FEN;

			// Extract move text (everything after the last header line)
			const lines = trimmed.split('\n');
			const moveTextLines: string[] = [];
			let pastHeaders = false;
			for (const line of lines) {
				if (pastHeaders) {
					moveTextLines.push(line);
				} else if (!line.startsWith('[')) {
					pastHeaders = true;
					moveTextLines.push(line);
				}
			}
			const moveText = moveTextLines.join(' ').replace(/\*/g, '').trim();

			// Parse moves if any
			const moves: HistoryEntry[] = [];
			if (moveText) {
				try {
					const chess = new Chess(fen);
					chess.loadPgn(moveText);
					const history = chess.history({ verbose: true });
					const replay = new Chess(fen);
					for (const move of history) {
						replay.move(move.san);
						moves.push({
							fen: replay.fen(),
							san: move.san,
							uci: move.from + move.to + (move.promotion || '')
						});
					}
				} catch {
					// No valid moves, just the position
				}
			}

			const label = whiteMatch ? whiteMatch[1] : `Position ${games.length + 1}`;
			games.push({ fen, label, moves });
		}

		return games;
	}

	function loadPuzzle(idx: number) {
		if (idx < 0 || idx >= puzzles.length) return;
		puzzleIndex = idx;
		const puzzle = puzzles[idx];
		startFen = puzzle.fen;
		moveHistory = [...puzzle.moves];
		historyIndex = -1;
		currentFen = puzzle.fen;
		moveSource = 'init';
		// Orient board to the side that moves
		orientation = turnColor(puzzle.fen);
	}

	function prevPuzzle() {
		if (puzzleIndex > 0) loadPuzzle(puzzleIndex - 1);
	}

	function nextPuzzle() {
		if (puzzleIndex < puzzles.length - 1) loadPuzzle(puzzleIndex + 1);
	}

	function loadPgn() {
		const parsed = parsePgnGames(pgnText);

		if (parsed.length > 1 || (parsed.length === 1 && parsed[0].fen !== STARTING_FEN)) {
			// Multi-game or custom FEN — use puzzle mode
			puzzles = parsed;
			loadPuzzle(0);
			return;
		}

		// Single standard game — original behavior
		puzzles = [];
		puzzleIndex = -1;
		const chess = new Chess();
		try {
			chess.loadPgn(pgnText);
		} catch {
			return;
		}

		const history = chess.history({ verbose: true });
		const newHistory: HistoryEntry[] = [];
		const replay = new Chess();

		for (const move of history) {
			replay.move(move.san);
			newHistory.push({
				fen: replay.fen(),
				san: move.san,
				uci: move.from + move.to + (move.promotion || '')
			});
		}

		startFen = STARTING_FEN;
		moveHistory = newHistory;
		historyIndex = newHistory.length - 1;
		currentFen = newHistory.length > 0 ? newHistory[newHistory.length - 1].fen : STARTING_FEN;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.target instanceof HTMLTextAreaElement) return;
		if (e.key === 'ArrowLeft') { e.preventDefault(); goBack(); }
		else if (e.key === 'ArrowRight') { e.preventDefault(); goForward(); }
		else if (e.key === 'ArrowUp' && puzzles.length > 1) { e.preventDefault(); prevPuzzle(); }
		else if (e.key === 'ArrowDown' && puzzles.length > 1) { e.preventDefault(); nextPuzzle(); }
		else if (e.key === 'Home') { e.preventDefault(); goToStart(); }
		else if (e.key === 'End') { e.preventDefault(); goToEnd(); }
		else if (e.key === 'f') { flipBoard(); }
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="explorer">
	<div class="toolbar">
		<h2>Explorer</h2>
		<div class="toolbar-actions">
			<button class:active={showEval} onclick={() => showEval = !showEval}>Eval</button>
			<button class:active={showBestMoves} onclick={() => showBestMoves = !showBestMoves}>Best Moves</button>
			<button class:active={computerEnabled} onclick={() => computerEnabled = !computerEnabled}>Computer</button>
			<button onclick={flipBoard}>Flip Board</button>
			<button onclick={resetBoard}>Reset</button>
		</div>
	</div>

	<div class="layout">
		<div class="sidebar-left">
			{#if puzzles.length > 1}
				<div class="puzzle-nav">
					<button onclick={prevPuzzle} disabled={puzzleIndex <= 0}>&lsaquo;</button>
					<span class="puzzle-counter">Puzzle {puzzleIndex + 1} / {puzzles.length}</span>
					<button onclick={nextPuzzle} disabled={puzzleIndex >= puzzles.length - 1}>&rsaquo;</button>
				</div>
			{/if}
			<h3>Moves</h3>
			<div class="move-list">
				{#each numberedMoves as row}
					<div class="move-row">
						<span class="move-num">{row.moveNum}.</span>
						{#if row.white}
							{@const w = row.white}
							<button
								class="move-btn"
								class:active={historyIndex === w.idx}
								onclick={() => navigateToIndex(w.idx)}
							>{w.san}</button>
						{:else}
							<span class="move-placeholder">...</span>
						{/if}
						{#if row.black}
							{@const b = row.black}
							<button
								class="move-btn"
								class:active={historyIndex === b.idx}
								onclick={() => navigateToIndex(b.idx)}
							>{b.san}</button>
						{/if}
					</div>
				{/each}
			</div>
			<div class="nav-buttons">
				<button onclick={goToStart} title="Start">&laquo;</button>
				<button onclick={goBack} title="Back">&lsaquo;</button>
				<button onclick={goForward} title="Forward">&rsaquo;</button>
				<button onclick={goToEnd} title="End">&raquo;</button>
			</div>
		</div>

		<div class="board-area">
			<div class="board-with-eval" style:grid-template-columns={showEval ? '28px 1fr' : '1fr'}>
				{#if showEval}
					<EvalBar score={evalScore} mate={evalMate} {orientation} />
				{/if}
				<Board config={boardConfig} onMove={handleMove} shapes={engineArrows} />
			</div>
		</div>

		<div class="sidebar-right">
			{#if showBestMoves && engineLines.length > 0}
				<BestMoves lines={engineLines} fen={currentFen} {orientation} onPlayMove={(uci) => handleMove(uci.slice(0, 2), uci.slice(2, 4))} />
			{/if}
			<h3>PGN</h3>
			<div class="pgn-section">
				<textarea
					bind:value={pgnText}
					placeholder="Paste PGN here..."
					rows="12"
				></textarea>
				<button class="load-btn" onclick={loadPgn}>Load PGN</button>
			</div>
			<div class="shortcuts">
				<h4>Shortcuts</h4>
				<ul>
					<li><kbd>&larr;</kbd> <kbd>&rarr;</kbd> Navigate moves</li>
					{#if puzzles.length > 1}
						<li><kbd>&uarr;</kbd> <kbd>&darr;</kbd> Prev/Next puzzle</li>
					{/if}
					<li><kbd>Home</kbd> <kbd>End</kbd> First/Last</li>
					<li><kbd>F</kbd> Flip board</li>
				</ul>
			</div>
		</div>
	</div>
</div>

<style>
	.explorer {
		max-width: 1200px;
	}

	.toolbar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.toolbar-actions {
		display: flex;
		gap: 0.5rem;
	}

	.toolbar-actions button {
		padding: 0.4rem 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.85rem;
		color: var(--color-text);
	}

	.toolbar-actions button.active {
		background: var(--color-primary);
		color: white;
		border-color: var(--color-primary);
	}

	.layout {
		display: grid;
		grid-template-columns: 250px 1fr 250px;
		gap: 1rem;
		align-items: start;
	}

	.sidebar-left,
	.sidebar-right {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		min-height: 400px;
	}

	.sidebar-left h3,
	.sidebar-right h3 {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
		font-size: 0.9rem;
		margin: 0;
	}

	.puzzle-nav {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		padding: 0.5rem;
		border-bottom: 1px solid var(--color-border);
		background: var(--color-bg, #1a1a2e);
	}

	.puzzle-nav button {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 0.2rem 0.6rem;
		cursor: pointer;
		font-size: 1rem;
		color: var(--color-text);
	}

	.puzzle-nav button:disabled {
		opacity: 0.4;
		cursor: default;
	}

	.puzzle-counter {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--color-text);
	}

	.move-placeholder {
		min-width: 3.5rem;
		padding: 0.2rem 0.4rem;
		font-size: 0.85rem;
		color: var(--color-text-muted, #888);
	}

	.move-list {
		max-height: 400px;
		overflow-y: auto;
		padding: 0.5rem;
	}

	.move-row {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-bottom: 1px;
	}

	.move-num {
		color: var(--color-text-muted, #888);
		font-size: 0.8rem;
		min-width: 2rem;
		text-align: right;
	}

	.move-btn {
		background: none;
		border: none;
		padding: 0.2rem 0.4rem;
		cursor: pointer;
		font-size: 0.85rem;
		border-radius: 3px;
		color: var(--color-text);
		min-width: 3.5rem;
		text-align: left;
	}

	.move-btn:hover {
		background: var(--color-border);
	}

	.move-btn.active {
		background: var(--color-primary);
		color: white;
	}

	.nav-buttons {
		display: flex;
		justify-content: center;
		gap: 0.25rem;
		padding: 0.5rem;
		border-top: 1px solid var(--color-border);
	}

	.nav-buttons button {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		padding: 0.3rem 0.75rem;
		cursor: pointer;
		font-size: 1rem;
		color: var(--color-text);
	}

	.nav-buttons button:hover {
		background: var(--color-border);
	}

	.board-area {
		display: flex;
		justify-content: center;
	}

	.board-with-eval {
		display: grid;
		gap: 0.35rem;
		max-width: 600px;
		width: 100%;
	}

	.pgn-section {
		padding: 0.75rem;
	}

	.pgn-section textarea {
		width: 100%;
		font-family: monospace;
		font-size: 0.8rem;
		padding: 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-bg, #1a1a2e);
		color: var(--color-text);
		resize: vertical;
		box-sizing: border-box;
	}

	.load-btn {
		margin-top: 0.5rem;
		width: 100%;
		padding: 0.4rem;
		background: var(--color-primary);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
	}

	.load-btn:hover {
		opacity: 0.9;
	}

	.shortcuts {
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-border);
	}

	.shortcuts h4 {
		font-size: 0.8rem;
		margin: 0 0 0.5rem;
		color: var(--color-text-muted, #888);
	}

	.shortcuts ul {
		list-style: none;
		padding: 0;
		margin: 0;
		font-size: 0.75rem;
		color: var(--color-text-muted, #888);
	}

	.shortcuts li {
		margin-bottom: 0.25rem;
	}

	.shortcuts kbd {
		background: var(--color-border);
		padding: 0.1rem 0.35rem;
		border-radius: 3px;
		font-size: 0.7rem;
	}

	@media (max-width: 768px) {
		.toolbar {
			flex-wrap: wrap;
			gap: 0.5rem;
		}

		.toolbar-actions {
			flex-wrap: wrap;
		}

		.toolbar-actions button {
			min-height: 40px;
		}

		.layout {
			grid-template-columns: 1fr;
		}

		.board-area {
			order: -1;
		}

		.board-with-eval {
			max-width: 100%;
		}

		.sidebar-left,
		.sidebar-right {
			min-height: auto;
		}

		.move-btn {
			padding: 0.4rem 0.6rem;
			min-height: 36px;
		}

		.nav-buttons button {
			min-height: 44px;
			min-width: 44px;
		}
	}
</style>
