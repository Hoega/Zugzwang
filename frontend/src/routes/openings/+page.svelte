<script lang="ts">
	import { browser } from '$app/environment';
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { Chess } from 'chess.js';
	import Board from '$lib/components/Board.svelte';
	import EvalBar from '$lib/components/EvalBar.svelte';
	import BestMoves from '$lib/components/BestMoves.svelte';
	import { Engine, type MultiPvLine } from '$lib/utils/engine';
	import { makeEngineArrows } from '$lib/utils/shapes';
	import { toDests, turnColor, STARTING_FEN } from '$lib/utils/chess';
	import type { Config } from 'chessground/config';
	import ecoData from '$lib/data/eco.json';
	import AddToRepertoireModal from '$lib/components/AddToRepertoireModal.svelte';

	interface EcoOpening {
		eco: string;
		name: string;
		name_fr: string;
		pgn: string;
		fen: string;
	}

	interface HistoryEntry {
		fen: string;
		san: string;
		uci: string;
	}

	const openings: EcoOpening[] = ecoData as EcoOpening[];
	const categories = ['A', 'B', 'C', 'D', 'E'];

	// Pre-compute parsed SAN move arrays for all openings (lazy, computed once on first access)
	let _openingMoves: string[][] | null = null;
	function getOpeningMoves(): string[][] {
		if (!_openingMoves) {
			_openingMoves = openings.map((o) => {
				const chess = new Chess();
				try {
					chess.loadPgn(o.pgn);
					return chess.history();
				} catch {
					return [];
				}
			});
		}
		return _openingMoves;
	}

	let language = $state<'en' | 'fr'>(
		(browser && localStorage.getItem('openings-lang') as 'en' | 'fr') || 'en'
	);

	function toggleLanguage(lang: 'en' | 'fr') {
		language = lang;
		localStorage.setItem('openings-lang', lang);
	}

	function openingName(opening: EcoOpening): string {
		return language === 'fr' ? opening.name_fr : opening.name;
	}

	let searchQuery = $state('');
	let activeCategory = $state('A');
	let selectedOpening = $state<EcoOpening | null>(null);
	let browseHistory = $state<HistoryEntry[]>([]);
	let viewHistory = $state<HistoryEntry[]>([]);
	let historyIndex = $state(-1);
	let orientation = $state<'white' | 'black'>('white');

	let evalScore = $state<number | null>(null);
	let evalMate = $state<number | null>(null);
	let showBestMoves = $state(false);
	let showEval = $state(false);
	let engineLines = $state<MultiPvLine[]>([]);
	let engine: Engine | null = null;
	let engineReady = $state(false);
	let showAddModal = $state(false);

	onMount(() => {
		engine = new Engine();
		engine.init().then(() => { engineReady = true; }).catch(() => {});

		// Pre-select opening from URL params (e.g. from MoveLines eco-tag links)
		const eco = $page.url.searchParams.get('eco');
		const name = $page.url.searchParams.get('name');
		if (eco && name) {
			const match = openings.find((o) => o.eco === eco && o.name === name);
			if (match) {
				activeCategory = eco[0];
				selectOpening(match);
			}
		}

		return () => { engine?.destroy(); };
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

	let engineArrows = $derived(makeEngineArrows(engineLines, orientation));

	// Active move history depends on mode
	let moveHistory = $derived(selectedOpening ? viewHistory : browseHistory);

	let currentFen = $derived(
		historyIndex === -1 ? STARTING_FEN : moveHistory[historyIndex]?.fen ?? STARTING_FEN
	);

	let browsing = $derived(selectedOpening === null);

	function normalize(s: string): string {
		return s.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase();
	}

	let filteredOpenings = $derived.by(() => {
		// When browsing with moves played, filter by move prefix
		if (browseHistory.length > 0) {
			const browseSans = browseHistory.map((e) => e.san);
			const allMoves = getOpeningMoves();
			return openings.filter((_, i) => {
				const moves = allMoves[i];
				if (moves.length < browseSans.length) return false;
				for (let j = 0; j < browseSans.length; j++) {
					if (moves[j] !== browseSans[j]) return false;
				}
				return true;
			});
		}

		let filtered = openings;
		if (searchQuery.trim()) {
			const q = normalize(searchQuery);
			filtered = openings.filter(
				(o) =>
					normalize(o.name).includes(q) ||
					normalize(o.name_fr).includes(q) ||
					o.eco.toLowerCase().includes(q)
			);
		} else {
			filtered = openings.filter((o) => o.eco.startsWith(activeCategory));
		}
		return filtered;
	});

	let boardConfig = $derived.by(() => {
		const chess = new Chess(currentFen);
		const color = turnColor(currentFen);
		const interactive = browsing;
		return {
			fen: currentFen,
			orientation,
			turnColor: color,
			lastMove:
				historyIndex >= 0 && moveHistory[historyIndex]
					? [
							moveHistory[historyIndex].uci.slice(0, 2),
							moveHistory[historyIndex].uci.slice(2, 4)
						]
					: [],
			movable: {
				color: interactive ? color : undefined,
				free: false,
				dests: interactive ? toDests(chess) : new Map()
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

	function handleMove(orig: string, dest: string) {
		const chess = new Chess(currentFen);
		const move = chess.move({ from: orig, to: dest, promotion: 'q' });
		if (!move) return;

		// Truncate forward history if navigating from mid-history
		const newHistory = browseHistory.slice(0, historyIndex + 1);

		const entry: HistoryEntry = {
			fen: chess.fen(),
			san: move.san,
			uci: orig + dest + (move.promotion || '')
		};

		browseHistory = [...newHistory, entry];
		historyIndex = browseHistory.length - 1;
		// Ensure we're in browse mode
		selectedOpening = null;
	}

	function selectOpening(opening: EcoOpening) {
		selectedOpening = opening;
		const chess = new Chess();
		try {
			chess.loadPgn(opening.pgn);
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

		viewHistory = newHistory;
		historyIndex = newHistory.length - 1;
	}

	function backToBrowsing() {
		selectedOpening = null;
		historyIndex = browseHistory.length > 0 ? browseHistory.length - 1 : -1;
	}

	function resetBrowse() {
		browseHistory = [];
		selectedOpening = null;
		historyIndex = -1;
	}

	function navigateToIndex(idx: number) {
		if (idx < -1 || idx >= moveHistory.length) return;
		historyIndex = idx;
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
		} else if (e.key === 'Escape' && selectedOpening) {
			backToBrowsing();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="openings-page">
	<div class="toolbar">
		<h2>Opening Database</h2>
		<div class="toolbar-actions">
			<div class="lang-toggle">
				<button class:active={language === 'en'} onclick={() => toggleLanguage('en')}>EN</button>
				<button class:active={language === 'fr'} onclick={() => toggleLanguage('fr')}>FR</button>
			</div>
			<button class:active={showEval} onclick={() => showEval = !showEval}>Eval</button>
			<button class:active={showBestMoves} onclick={() => showBestMoves = !showBestMoves}>Best Moves</button>
			<a href="/openings/explorer" class="btn">Lichess Explorer</a>
			<button onclick={flipBoard}>Flip Board</button>
			{#if browseHistory.length > 0 || selectedOpening}
				<button onclick={resetBrowse}>Reset</button>
			{/if}
		</div>
	</div>

	<div class="layout">
		<div class="sidebar-left">
			<div class="search-box">
				<input
					type="text"
					placeholder="Search openings..."
					bind:value={searchQuery}
				/>
			</div>
			{#if !searchQuery.trim() && browseHistory.length === 0}
				<div class="category-tabs">
					{#each categories as cat}
						<button
							class="cat-tab"
							class:active={activeCategory === cat}
							onclick={() => (activeCategory = cat)}
						>{cat}</button>
					{/each}
				</div>
			{/if}
			<div class="opening-list">
				{#each filteredOpenings as opening, i (opening.eco + opening.name + i)}
					<button
						class="opening-item"
						class:active={selectedOpening === opening}
						onclick={() => selectOpening(opening)}
					>
						<span class="eco-code">{opening.eco}</span>
						<span class="opening-name">{openingName(opening)}</span>
					</button>
				{/each}
				{#if filteredOpenings.length === 0}
					<div class="no-results">No openings found</div>
				{/if}
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
				<BestMoves lines={engineLines} fen={currentFen} {orientation} onPlayMove={(uci) => { selectedOpening = null; handleMove(uci.slice(0, 2), uci.slice(2, 4)); }} />
			{/if}
			{#if selectedOpening}
				<div class="opening-info">
					<div class="opening-eco">{selectedOpening.eco}</div>
					<div class="opening-title">{openingName(selectedOpening)}</div>
					<button class="add-btn" onclick={() => (showAddModal = true)}>Add to Repertoire</button>
					<button class="back-btn" onclick={backToBrowsing}>Back to browsing</button>
				</div>
			{:else if browseHistory.length > 0}
				<div class="browse-info">
					<span class="browse-label">Filtering by moves</span>
					<span class="browse-count">{filteredOpenings.length} openings</span>
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

{#if selectedOpening}
	<AddToRepertoireModal bind:open={showAddModal} pgn={selectedOpening.pgn} openingName={openingName(selectedOpening)} />
{/if}

<style>
	.openings-page {
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

	.lang-toggle {
		display: flex;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		overflow: hidden;
	}

	.lang-toggle button {
		border: none;
		border-radius: 0;
		border-right: 1px solid var(--color-border);
		padding: 0.4rem 0.6rem;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--color-muted);
	}

	.lang-toggle button:last-child {
		border-right: none;
	}

	.lang-toggle button.active {
		background: var(--color-primary);
		color: white;
	}

	.toolbar-actions button,
	.toolbar-actions .btn {
		padding: 0.4rem 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.85rem;
		color: var(--color-text);
		text-decoration: none;
	}

	.toolbar-actions .btn:hover,
	.toolbar-actions button:hover {
		background: var(--color-hover);
		text-decoration: none;
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

	.search-box {
		padding: 0.75rem;
		border-bottom: 1px solid var(--color-border);
	}

	.search-box input {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		font-size: 0.85rem;
		background: var(--color-bg);
		color: var(--color-text);
	}

	.category-tabs {
		display: flex;
		border-bottom: 1px solid var(--color-border);
	}

	.cat-tab {
		flex: 1;
		padding: 0.5rem;
		background: none;
		border: none;
		border-bottom: 2px solid transparent;
		cursor: pointer;
		font-size: 0.9rem;
		font-weight: 600;
		color: var(--color-muted);
	}

	.cat-tab.active {
		color: var(--color-primary);
		border-bottom-color: var(--color-primary);
	}

	.cat-tab:hover {
		color: var(--color-text);
	}

	.opening-list {
		max-height: 500px;
		overflow-y: auto;
	}

	.opening-item {
		display: flex;
		gap: 0.5rem;
		align-items: baseline;
		width: 100%;
		padding: 0.4rem 0.75rem;
		background: none;
		border: none;
		border-bottom: 1px solid var(--color-border);
		cursor: pointer;
		text-align: left;
		color: var(--color-text);
		font-size: 0.82rem;
	}

	.opening-item:hover {
		background: var(--color-hover);
	}

	.opening-item.active {
		background: var(--color-selected);
	}

	.eco-code {
		font-weight: 700;
		font-size: 0.75rem;
		color: var(--color-primary);
		min-width: 2rem;
	}

	.opening-name {
		flex: 1;
	}

	.no-results {
		padding: 1rem;
		text-align: center;
		color: var(--color-muted);
		font-size: 0.85rem;
	}

	.toolbar-actions button.active {
		background: var(--color-primary);
		color: white;
		border-color: var(--color-primary);
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

	.sidebar-right h3 {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
		font-size: 0.9rem;
		margin: 0;
	}

	.opening-info {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
	}

	.opening-eco {
		font-weight: 700;
		font-size: 0.8rem;
		color: var(--color-primary);
	}

	.opening-title {
		font-weight: 600;
		font-size: 0.9rem;
		margin-top: 0.25rem;
	}

	.add-btn {
		margin-top: 0.5rem;
		width: 100%;
		padding: 0.4rem;
		background: var(--color-primary);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.8rem;
		font-weight: 600;
		min-height: 44px;
	}

	.add-btn:hover {
		opacity: 0.9;
	}

	.back-btn {
		margin-top: 0.5rem;
		width: 100%;
		padding: 0.3rem;
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.8rem;
		color: var(--color-muted);
	}

	.back-btn:hover {
		background: var(--color-hover);
		color: var(--color-text);
	}

	.browse-info {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 1rem;
		border-bottom: 1px solid var(--color-border);
		font-size: 0.8rem;
	}

	.browse-label {
		font-weight: 600;
		color: var(--color-primary);
	}

	.browse-count {
		color: var(--color-muted);
	}

	.move-list {
		max-height: 300px;
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

	@media (max-width: 768px) {
		.toolbar {
			flex-wrap: wrap;
			gap: 0.5rem;
		}

		.toolbar-actions {
			flex-wrap: wrap;
			gap: 0.75rem;
		}

		.toolbar-actions button,
		.toolbar-actions .btn {
			min-height: 44px;
			padding: 0.5rem 0.75rem;
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
