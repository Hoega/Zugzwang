<script lang="ts">
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { Chess } from 'chess.js';
	import Board from '$lib/components/Board.svelte';
	import EvalBar from '$lib/components/EvalBar.svelte';
	import BestMoves from '$lib/components/BestMoves.svelte';
	import { Engine, type MultiPvLine } from '$lib/utils/engine';
	import { makeEngineArrows } from '$lib/utils/shapes';
	import { toDests, turnColor, STARTING_FEN } from '$lib/utils/chess';
	import {
		fetchExplorerDebounced,
		type LichessExplorerResponse,
		type LichessMove,
		type DatabaseType
	} from '$lib/stores/lichess';
	import { api } from '$lib/stores/api';
	import type { Repertoire } from '$lib/types';
	import type { Config } from 'chessground/config';

	interface HistoryEntry {
		fen: string;
		san: string;
		uci: string;
	}

	let currentFen = $state(STARTING_FEN);
	let moveHistory = $state<HistoryEntry[]>([]);
	let historyIndex = $state(-1);
	let orientation = $state<'white' | 'black'>('white');

	let explorerData = $state<LichessExplorerResponse | null>(null);
	let explorerLoading = $state(false);
	let explorerError = $state<string | null>(null);
	let database = $state<DatabaseType>('masters');

	let evalScore = $state<number | null>(null);
	let evalMate = $state<number | null>(null);
	let showEval = $state(false);
	let showBestMoves = $state(false);
	let engineLines = $state<MultiPvLine[]>([]);
	let engine: Engine | null = null;
	let engineReady = $state(false);

	let repertoires = $state<Repertoire[]>([]);
	let selectedRepertoireId = $state('');
	let addingLine = $state(false);
	let addLineResult = $state<{ success: boolean; message: string } | null>(null);

	let boardConfig = $derived.by(() => {
		const chess = new Chess(currentFen);
		const color = turnColor(currentFen);
		return {
			fen: currentFen,
			orientation,
			turnColor: color,
			lastMove:
				historyIndex >= 0
					? [
							moveHistory[historyIndex].uci.slice(0, 2),
							moveHistory[historyIndex].uci.slice(2, 4)
						]
					: [],
			movable: {
				color,
				free: false,
				dests: toDests(chess)
			}
		} as Config;
	});

	let numberedMoves = $derived.by(() => {
		const result: {
			moveNum: number;
			white?: { san: string; idx: number };
			black?: { san: string; idx: number };
		}[] = [];
		for (let i = 0; i < moveHistory.length; i++) {
			const moveNum = Math.floor(i / 2) + 1;
			if (i % 2 === 0) {
				result.push({ moveNum, white: { san: moveHistory[i].san, idx: i } });
			} else {
				result[result.length - 1].black = {
					san: moveHistory[i].san,
					idx: i
				};
			}
		}
		return result;
	});

	let totalGames = $derived(
		explorerData
			? explorerData.white + explorerData.draws + explorerData.black
			: 0
	);

	let engineArrows = $derived(makeEngineArrows(engineLines, orientation));

	onMount(() => {
		engine = new Engine();
		engine
			.init()
			.then(() => {
				engineReady = true;
			})
			.catch(() => {});
		api.listRepertoires().then(r => { repertoires = r; }).catch(() => {});

		return () => {
			engine?.destroy();
		};
	});

	// Fetch explorer data when position changes
	$effect(() => {
		if (!browser) return;
		const fen = currentFen;
		const db = database;
		explorerLoading = true;
		explorerError = null;
		fetchExplorerDebounced(
			fen,
			db,
			(data) => {
				explorerData = data;
				explorerLoading = false;
			},
			(err) => {
				explorerError = err.message;
				explorerLoading = false;
			}
		);
	});

	// Stockfish eval
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

	function handleMove(orig: string, dest: string) {
		const chess = new Chess(currentFen);
		const move = chess.move({ from: orig, to: dest, promotion: 'q' });
		if (!move) return;

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
	}

	function playExplorerMove(move: LichessMove) {
		handleMove(move.uci.slice(0, 2), move.uci.slice(2, 4));
	}

	function navigateToIndex(idx: number) {
		if (idx < -1 || idx >= moveHistory.length) return;
		historyIndex = idx;
		currentFen = idx === -1 ? STARTING_FEN : moveHistory[idx].fen;
	}

	function goBack() {
		if (historyIndex >= 0) navigateToIndex(historyIndex - 1);
	}

	function goForward() {
		if (historyIndex < moveHistory.length - 1)
			navigateToIndex(historyIndex + 1);
	}

	function goToStart() {
		navigateToIndex(-1);
	}

	function goToEnd() {
		if (moveHistory.length > 0) navigateToIndex(moveHistory.length - 1);
	}

	function flipBoard() {
		orientation = orientation === 'white' ? 'black' : 'white';
	}

	function resetBoard() {
		moveHistory = [];
		historyIndex = -1;
		currentFen = STARTING_FEN;
		orientation = 'white';
	}

	function formatNumber(n: number): string {
		if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
		if (n >= 1000) return (n / 1000).toFixed(1) + 'k';
		return n.toString();
	}

	function winPercent(move: LichessMove): {
		white: number;
		draws: number;
		black: number;
	} {
		const total = move.white + move.draws + move.black;
		if (total === 0) return { white: 0, draws: 0, black: 0 };
		return {
			white: (move.white / total) * 100,
			draws: (move.draws / total) * 100,
			black: (move.black / total) * 100
		};
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.target instanceof HTMLInputElement) return;
		if (e.key === 'ArrowLeft') {
			e.preventDefault();
			goBack();
		} else if (e.key === 'ArrowRight') {
			e.preventDefault();
			goForward();
		} else if (e.key === 'Home') {
			e.preventDefault();
			goToStart();
		} else if (e.key === 'End') {
			e.preventDefault();
			goToEnd();
		} else if (e.key === 'f') {
			flipBoard();
		}
	}

	async function addLineToRepertoire() {
		if (addingLine || !selectedRepertoireId || moveHistory.length === 0) return;
		addingLine = true;
		addLineResult = null;

		const movesToAdd = moveHistory.slice(0, historyIndex + 1);
		if (movesToAdd.length === 0) {
			addingLine = false;
			return;
		}

		let parentId: string | null = null;
		let parentFen = STARTING_FEN;
		let addedCount = 0;

		try {
			for (const move of movesToAdd) {
				const newNode = await api.addMove(selectedRepertoireId, {
					parent_id: parentId,
					fen: parentFen,
					uci_move: move.uci
				});
				parentId = newNode.id;
				parentFen = newNode.fen;
				addedCount++;
			}
			addLineResult = { success: true, message: `Added ${addedCount} move${addedCount !== 1 ? 's' : ''} to repertoire` };
		} catch (e) {
			addLineResult = { success: false, message: e instanceof Error ? e.message : 'Failed to add line' };
		}
		addingLine = false;
	}

	$effect(() => {
		if (addLineResult) {
			const timer = setTimeout(() => { addLineResult = null; }, 3000);
			return () => clearTimeout(timer);
		}
	});
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="explorer-page">
	<div class="toolbar">
		<h2>Opening Explorer</h2>
		<div class="toolbar-actions">
			<div class="db-toggle">
				<button
					class:active={database === 'masters'}
					onclick={() => (database = 'masters')}
				>Masters</button>
				<button
					class:active={database === 'lichess'}
					onclick={() => (database = 'lichess')}
				>Lichess</button>
			</div>
			<button class:active={showEval} onclick={() => (showEval = !showEval)}>Eval</button>
			<button
				class:active={showBestMoves}
				onclick={() => (showBestMoves = !showBestMoves)}
			>Best Moves</button>
			<button onclick={flipBoard}>Flip Board</button>
			<button onclick={resetBoard}>Reset</button>
		</div>
	</div>

	<div class="layout">
		<div class="sidebar-left">
			{#if explorerData?.opening}
				<div class="current-opening">
					<span class="opening-eco">{explorerData.opening.eco}</span>
					<span class="opening-name">{explorerData.opening.name}</span>
				</div>
			{/if}

			{#if totalGames > 0}
				<div class="total-stats">
					<div class="stat-bar-large">
						<div
							class="bar-white"
							style="width: {(explorerData!.white / totalGames) * 100}%"
						>
							{((explorerData!.white / totalGames) * 100).toFixed(0)}%
						</div>
						<div
							class="bar-draws"
							style="width: {(explorerData!.draws / totalGames) * 100}%"
						>
							{((explorerData!.draws / totalGames) * 100).toFixed(0)}%
						</div>
						<div
							class="bar-black"
							style="width: {(explorerData!.black / totalGames) * 100}%"
						>
							{((explorerData!.black / totalGames) * 100).toFixed(0)}%
						</div>
					</div>
					<div class="total-count">
						{formatNumber(totalGames)} games
					</div>
				</div>
			{/if}

			<div class="moves-table-header">
				<span class="col-move">Move</span>
				<span class="col-games">Games</span>
				<span class="col-bar">Result</span>
			</div>

			<div class="moves-table">
				{#if explorerLoading && !explorerData}
					<div class="loading">Loading...</div>
				{:else if explorerError}
					<div class="error">{explorerError}</div>
				{:else if explorerData}
					{#each explorerData.moves as move (move.uci)}
						{@const total = move.white + move.draws + move.black}
						{@const pct = winPercent(move)}
						<button class="move-row" onclick={() => playExplorerMove(move)}>
							<span class="col-move move-san">{move.san}</span>
							<span class="col-games">{formatNumber(total)}</span>
							<span class="col-bar">
								<div class="result-bar">
									<div
										class="bar-white"
										style="width: {pct.white}%"
									></div>
									<div
										class="bar-draws"
										style="width: {pct.draws}%"
									></div>
									<div
										class="bar-black"
										style="width: {pct.black}%"
									></div>
								</div>
							</span>
						</button>
					{/each}
					{#if explorerData.moves.length === 0}
						<div class="no-data">No games in this position</div>
					{/if}
				{/if}
			</div>

			{#if explorerData?.topGames && explorerData.topGames.length > 0}
				<div class="top-games">
					<h4>Notable Games</h4>
					{#each explorerData.topGames.slice(0, 5) as game (game.id)}
						<a
							class="game-row"
							href="https://lichess.org/{game.id}"
							target="_blank"
							rel="noopener"
						>
							<div class="game-players">
								<span class="player-white">{game.white.name} ({game.white.rating})</span>
								<span class="vs">vs</span>
								<span class="player-black">{game.black.name} ({game.black.rating})</span>
							</div>
							<div class="game-meta">
								<span class="game-result">
									{game.winner === 'white'
										? '1-0'
										: game.winner === 'black'
											? '0-1'
											: '1/2'}
								</span>
								<span class="game-year">{game.year}</span>
							</div>
						</a>
					{/each}
				</div>
			{/if}
		</div>

		<div class="board-area">
			<div
				class="board-with-eval"
				style:grid-template-columns={showEval ? '28px 1fr' : '1fr'}
			>
				{#if showEval}
					<EvalBar score={evalScore} mate={evalMate} {orientation} />
				{/if}
				<Board
					config={boardConfig}
					onMove={handleMove}
					shapes={engineArrows}
				/>
			</div>
			<div class="nav-buttons mobile-nav">
				<button onclick={goToStart} title="Start">&laquo;</button>
				<button onclick={goBack} title="Back">&lsaquo;</button>
				<button onclick={goForward} title="Forward">&rsaquo;</button>
				<button onclick={goToEnd} title="End">&raquo;</button>
			</div>
		</div>

		<div class="sidebar-right">
			{#if showBestMoves && engineLines.length > 0}
				<BestMoves
					lines={engineLines}
					fen={currentFen}
					{orientation}
					onPlayMove={(uci) =>
						handleMove(uci.slice(0, 2), uci.slice(2, 4))}
				/>
			{/if}
			<h3>Moves</h3>
			<div class="move-list">
				{#each numberedMoves as row}
					<div class="move-row-nav">
						<span class="move-num">{row.moveNum}.</span>
						{#if row.white}
							{@const w = row.white}
							<button
								class="move-btn"
								class:active={historyIndex === w.idx}
								onclick={() => navigateToIndex(w.idx)}
							>{w.san}</button>
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
			<div class="nav-buttons sidebar-nav">
				<button onclick={goToStart} title="Start">&laquo;</button>
				<button onclick={goBack} title="Back">&lsaquo;</button>
				<button onclick={goForward} title="Forward">&rsaquo;</button>
				<button onclick={goToEnd} title="End">&raquo;</button>
			</div>
			{#if moveHistory.length > 0 && repertoires.length > 0}
				<div class="add-to-repertoire">
					<h4>Add to Repertoire</h4>
					<div class="add-controls">
						<select bind:value={selectedRepertoireId}>
							<option value="">Select repertoire...</option>
							{#each repertoires as rep}
								<option value={rep.id}>{rep.name} ({rep.color})</option>
							{/each}
						</select>
						<button
							class="add-line-btn"
							onclick={addLineToRepertoire}
							disabled={!selectedRepertoireId || addingLine}
						>
							{addingLine ? 'Adding...' : 'Add line'}
						</button>
					</div>
					{#if addLineResult}
						<div class="add-result" class:success={addLineResult.success} class:error={!addLineResult.success}>
							{addLineResult.message}
						</div>
					{/if}
				</div>
			{/if}
			<div class="shortcuts">
				<h4>Shortcuts</h4>
				<ul>
					<li><kbd>&larr;</kbd> <kbd>&rarr;</kbd> Navigate moves</li>
					<li><kbd>Home</kbd> <kbd>End</kbd> First/Last</li>
					<li><kbd>F</kbd> Flip board</li>
				</ul>
			</div>
		</div>
	</div>
</div>

<style>
	.explorer-page {
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
		align-items: center;
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

	.db-toggle {
		display: flex;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		overflow: hidden;
	}

	.db-toggle button {
		border: none;
		border-radius: 0;
		border-right: 1px solid var(--color-border);
	}

	.db-toggle button:last-child {
		border-right: none;
	}

	.layout {
		display: grid;
		grid-template-columns: 300px 1fr 250px;
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

	.current-opening {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
	}

	.opening-eco {
		font-weight: 700;
		font-size: 0.75rem;
		color: var(--color-primary);
		margin-right: 0.5rem;
	}

	.opening-name {
		font-weight: 600;
		font-size: 0.85rem;
	}

	.total-stats {
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid var(--color-border);
	}

	.stat-bar-large {
		display: flex;
		height: 20px;
		border-radius: 3px;
		overflow: hidden;
		font-size: 0.7rem;
		font-weight: 600;
	}

	.total-count {
		font-size: 0.75rem;
		color: var(--color-muted);
		margin-top: 0.25rem;
		text-align: center;
	}

	.bar-white {
		background: #f0f0f0;
		color: #333;
		display: flex;
		align-items: center;
		justify-content: center;
		min-width: 0;
		overflow: hidden;
	}

	.bar-draws {
		background: #a0a0a0;
		color: white;
		display: flex;
		align-items: center;
		justify-content: center;
		min-width: 0;
		overflow: hidden;
	}

	.bar-black {
		background: #333;
		color: white;
		display: flex;
		align-items: center;
		justify-content: center;
		min-width: 0;
		overflow: hidden;
	}

	.moves-table-header {
		display: grid;
		grid-template-columns: 3rem 3.5rem 1fr;
		gap: 0.5rem;
		padding: 0.4rem 0.75rem;
		border-bottom: 1px solid var(--color-border);
		font-size: 0.7rem;
		font-weight: 600;
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.moves-table {
		max-height: 300px;
		overflow-y: auto;
	}

	.moves-table .move-row {
		display: grid;
		grid-template-columns: 3rem 3.5rem 1fr;
		gap: 0.5rem;
		padding: 0.35rem 0.75rem;
		border: none;
		border-bottom: 1px solid var(--color-border);
		background: none;
		cursor: pointer;
		width: 100%;
		text-align: left;
		color: var(--color-text);
		font-size: 0.82rem;
		align-items: center;
	}

	.moves-table .move-row:hover {
		background: var(--color-hover);
	}

	.move-san {
		font-weight: 600;
	}

	.col-games {
		font-size: 0.75rem;
		color: var(--color-muted);
	}

	.result-bar {
		display: flex;
		height: 10px;
		border-radius: 2px;
		overflow: hidden;
	}

	.loading,
	.error,
	.no-data {
		padding: 1rem;
		text-align: center;
		color: var(--color-muted);
		font-size: 0.85rem;
	}

	.error {
		color: var(--color-danger);
	}

	.top-games {
		border-top: 1px solid var(--color-border);
		padding: 0.5rem 0;
	}

	.top-games h4 {
		padding: 0.25rem 0.75rem 0.5rem;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--color-muted);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.game-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.3rem 0.75rem;
		font-size: 0.75rem;
		text-decoration: none;
		color: var(--color-text);
		border-bottom: 1px solid var(--color-border);
	}

	.game-row:hover {
		background: var(--color-hover);
		text-decoration: none;
	}

	.game-players {
		display: flex;
		gap: 0.25rem;
		flex-wrap: wrap;
	}

	.player-white {
		font-weight: 600;
	}

	.vs {
		color: var(--color-muted);
	}

	.game-meta {
		display: flex;
		gap: 0.5rem;
		flex-shrink: 0;
	}

	.game-result {
		font-weight: 600;
	}

	.game-year {
		color: var(--color-muted);
	}

	.board-area {
		display: flex;
		flex-direction: column;
		align-items: center;
	}

	.board-with-eval {
		display: grid;
		gap: 0.35rem;
		max-width: 600px;
		width: 100%;
	}

	.sidebar-right h3 {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
		font-size: 0.9rem;
		margin: 0;
	}

	.move-list {
		max-height: 400px;
		overflow-y: auto;
		padding: 0.5rem;
	}

	.move-row-nav {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		margin-bottom: 1px;
	}

	.move-num {
		color: var(--color-muted);
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

	.shortcuts {
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-border);
	}

	.shortcuts h4 {
		font-size: 0.8rem;
		margin: 0 0 0.5rem;
		color: var(--color-muted);
	}

	.shortcuts ul {
		list-style: none;
		padding: 0;
		margin: 0;
		font-size: 0.75rem;
		color: var(--color-muted);
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

	.mobile-nav {
		display: none;
	}

	.add-to-repertoire {
		padding: 0.75rem 1rem;
		border-top: 1px solid var(--color-border);
	}

	.add-to-repertoire h4 {
		font-size: 0.8rem;
		margin: 0 0 0.5rem;
		color: var(--color-muted);
	}

	.add-controls {
		display: flex;
		gap: 0.5rem;
	}

	.add-controls select {
		flex: 1;
		padding: 0.3rem 0.4rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-bg, #1a1a2e);
		color: var(--color-text);
		font-size: 0.8rem;
	}

	.add-line-btn {
		padding: 0.3rem 0.6rem;
		border: none;
		border-radius: 4px;
		background: var(--color-primary);
		color: white;
		cursor: pointer;
		font-size: 0.8rem;
		font-weight: 600;
		white-space: nowrap;
	}

	.add-line-btn:hover:not(:disabled) {
		opacity: 0.9;
	}

	.add-line-btn:disabled {
		opacity: 0.5;
		cursor: default;
	}

	.add-result {
		margin-top: 0.4rem;
		font-size: 0.75rem;
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
	}

	.add-result.success {
		color: #16a34a;
		background: #dcfce7;
	}

	.add-result.error {
		color: #dc2626;
		background: #fee2e2;
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
			min-height: 44px;
		}

		.layout {
			grid-template-columns: 1fr;
		}

		.board-area {
			order: -1;
		}

		.mobile-nav {
			display: flex;
		}

		.sidebar-nav {
			display: none;
		}

		.board-with-eval {
			max-width: 100%;
		}

		.sidebar-left,
		.sidebar-right {
			min-height: auto;
		}

		.add-controls {
			flex-direction: column;
		}

		.add-controls select,
		.add-line-btn {
			min-height: 44px;
		}

		.moves-table .move-row {
			min-height: 44px;
		}

		.move-btn {
			padding: 0.4rem 0.6rem;
			min-height: 44px;
		}

		.nav-buttons {
			gap: 0.5rem;
			padding: 0.75rem;
		}

		.nav-buttons button {
			min-height: 44px;
			min-width: 44px;
			padding: 0.4rem 1rem;
		}
	}
</style>
