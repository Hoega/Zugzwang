<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { Chess } from 'chess.js';
	import Board from '$lib/components/Board.svelte';
	import EvalBar from '$lib/components/EvalBar.svelte';
	import MoveTree from '$lib/components/MoveTree.svelte';
	import MoveLines from '$lib/components/MoveLines.svelte';
	import AnnotationEditor from '$lib/components/AnnotationEditor.svelte';
	import BestMoves from '$lib/components/BestMoves.svelte';
	import PgnImportModal from '$lib/components/PgnImportModal.svelte';
	import { api } from '$lib/stores/api';
	import { Engine, type MultiPvLine } from '$lib/utils/engine';
	import { makeEngineArrows } from '$lib/utils/shapes';
	import { toDests, turnColor, STARTING_FEN } from '$lib/utils/chess';
	import { lookupOpening, ecoColor } from '$lib/utils/eco';
	import type { Repertoire, MoveTreeNode, MoveNode } from '$lib/types';
	import type { Config } from 'chessground/config';
	import type { DrawShape } from 'chessground/draw';
	import type { Key } from 'chessground/types';

	let repertoire = $state<Repertoire | null>(null);
	let tree = $state<MoveTreeNode[]>([]);
	let currentFen = $state(STARTING_FEN);
	let currentNodeId = $state<string | null>(null);
	let selectedMove = $state<MoveNode | null>(null);
	let showImport = $state(false);
	let loading = $state(true);
	let editingName = $state(false);
	let editName = $state('');
	let transpositionIds = $state<Set<string>>(new Set());
	let gapIds = $state<Set<string>>(new Set());
	let viewMode = $state<'tree' | 'lines'>('tree');
	let flipped = $state(false);
	let currentLineIndex = $state<number | null>(null);
	let evalScore = $state<number | null>(null);
	let evalMate = $state<number | null>(null);
	let showBestMoves = $state(false);
	let engineLines = $state<MultiPvLine[]>([]);
	let engine: Engine | null = null;

	let repertoireId = $derived($page.params.id);

	let boardConfig = $derived.by(() => {
		const chess = new Chess(currentFen);
		const color = turnColor(currentFen);
		const defaultOrientation = repertoire?.color ?? 'white';
		const orientation = flipped ? (defaultOrientation === 'white' ? 'black' : 'white') : defaultOrientation;
		return {
			fen: currentFen,
			orientation,
			turnColor: color,
			movable: {
				color,
				free: false,
				dests: toDests(chess)
			}
		} as Config;
	});

	let engineReady = $state(false);

	onMount(async () => {
		engine = new Engine();
		engine.init().then(() => { engineReady = true; }).catch(() => {});
		await loadData();

		return () => {
			engine?.destroy();
		};
	});

	$effect(() => {
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

	async function loadData() {
		loading = true;
		try {
			repertoire = await api.getRepertoire(repertoireId);
			tree = await api.getMoveTree(repertoireId);
			await loadAnalysis();
		} catch (e) {
			console.error('Failed to load:', e);
		}
		loading = false;
	}

	async function loadAnalysis() {
		try {
			const [transpositions, gaps] = await Promise.all([
				api.getTranspositions(repertoireId),
				api.getCoverageGaps(repertoireId)
			]);
			transpositionIds = new Set(transpositions.flatMap((g) => g.move_ids));
			gapIds = new Set(gaps);
		} catch (e) {
			console.error('Failed to load analysis:', e);
		}
	}

	async function handleMove(orig: string, dest: string) {
		const chess = new Chess(currentFen);
		const move = chess.move({ from: orig, to: dest, promotion: 'q' });
		if (!move) return;

		const uciMove = orig + dest + (move.promotion || '');

		try {
			const newNode = await api.addMove(repertoireId, {
				parent_id: currentNodeId,
				fen: currentFen,
				uci_move: uciMove
			});

			currentFen = newNode.fen;
			currentNodeId = newNode.id;
			tree = await api.getMoveTree(repertoireId);
		} catch (e) {
			console.error('Failed to add move:', e);
		}
	}

	function navigateToNode(node: MoveTreeNode, lineIndex?: number) {
		currentFen = node.fen;
		currentNodeId = node.id;
		selectedMove = node;
		if (lineIndex !== undefined) currentLineIndex = lineIndex;
	}

	function navigateToStart() {
		currentFen = STARTING_FEN;
		currentNodeId = null;
		selectedMove = null;
		currentLineIndex = null;
	}

	async function handleDeleteMove() {
		if (!selectedMove || !confirm('Delete this move and all continuations?')) return;
		try {
			await api.deleteMove(repertoireId, selectedMove.id);
			navigateToStart();
			tree = await api.getMoveTree(repertoireId);
		} catch (e) {
			console.error('Failed to delete move:', e);
		}
	}

	async function handleLineDelete() {
		navigateToStart();
		tree = await api.getMoveTree(repertoireId);
		await loadAnalysis();
	}

	async function handleExport() {
		try {
			const res = await api.exportPgn(repertoireId);
			const blob = new Blob([res.pgn], { type: 'text/plain' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `${repertoire?.name || 'repertoire'}.pgn`;
			a.click();
			URL.revokeObjectURL(url);
		} catch (e) {
			console.error('Export failed:', e);
		}
	}

	function updateNodeInTree(nodes: MoveTreeNode[], updated: MoveNode): MoveTreeNode[] {
		return nodes.map(n => {
			if (n.id === updated.id) {
				return { ...n, ...updated, children: n.children };
			}
			if (n.children.length > 0) {
				return { ...n, children: updateNodeInTree(n.children, updated) };
			}
			return n;
		});
	}

	const nagColors: Record<string, string> = {
		'!': '#22c55e',
		'!!': '#22c55e',
		'?': '#ef4444',
		'??': '#ef4444',
		'!?': '#f59e0b',
		'?!': '#f59e0b'
	};

	let nagShapes = $derived.by((): DrawShape[] => {
		if (!selectedMove?.nag || !selectedMove.uci_move) return [];
		const color = nagColors[selectedMove.nag];
		if (!color) return [];
		const dest = selectedMove.uci_move.slice(2, 4) as Key;
		const symbol = selectedMove.nag;
		const fontSize = symbol.length > 1 ? 17 : 22;
		return [{
			orig: dest,
			customSvg: {
				html: `<circle cx="80" cy="20" r="18" fill="${color}" stroke="white" stroke-width="2"/><text x="80" y="20" text-anchor="middle" dominant-baseline="central" fill="white" font-size="${fontSize}" font-weight="bold" font-family="sans-serif">${symbol}</text>`,
				center: 'orig'
			}
		}];
	});

	let currentOpening = $derived(lookupOpening(currentFen));

	let boardOrientation = $derived(boardConfig.orientation as 'white' | 'black');
	let engineArrows = $derived(makeEngineArrows(engineLines, boardOrientation));

	let allShapes = $derived([...nagShapes, ...engineArrows]);

	function findNode(nodes: MoveTreeNode[], id: string): MoveTreeNode | null {
		for (const n of nodes) {
			if (n.id === id) return n;
			const found = findNode(n.children, id);
			if (found) return found;
		}
		return null;
	}

	function findParent(nodes: MoveTreeNode[], id: string): MoveTreeNode | null {
		for (const n of nodes) {
			for (const c of n.children) {
				if (c.id === id) return n;
			}
			const found = findParent(n.children, id);
			if (found) return found;
		}
		return null;
	}

	function collectLines(nodes: MoveTreeNode[]): MoveTreeNode[][] {
		const result: MoveTreeNode[][] = [];
		function dfs(path: MoveTreeNode[], children: MoveTreeNode[]) {
			if (children.length === 0) {
				if (path.length > 0) result.push([...path]);
				return;
			}
			for (const child of children) {
				path.push(child);
				dfs(path, child.children);
				path.pop();
			}
		}
		dfs([], nodes);
		return result;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

		if (e.key === 'ArrowLeft') {
			e.preventDefault();
			if (!currentNodeId) return;
			if (viewMode === 'lines') {
				const lines = collectLines(tree);
				const lineIdx = currentLineIndex != null && currentLineIndex < lines.length ? currentLineIndex : lines.findIndex(l => l.some(m => m.id === currentNodeId));
				const line = lineIdx >= 0 ? lines[lineIdx] : null;
				if (line) {
					const idx = line.findIndex(m => m.id === currentNodeId);
					if (idx > 0) navigateToNode(line[idx - 1], lineIdx);
					else navigateToStart();
				} else {
					navigateToStart();
				}
			} else {
				const parent = findParent(tree, currentNodeId);
				if (parent) navigateToNode(parent);
				else navigateToStart();
			}
		} else if (e.key === 'ArrowRight') {
			e.preventDefault();
			if (viewMode === 'lines') {
				const lines = collectLines(tree);
				if (!currentNodeId) {
					if (lines.length > 0 && lines[0].length > 0) navigateToNode(lines[0][0], 0);
				} else {
					const lineIdx = currentLineIndex != null && currentLineIndex < lines.length ? currentLineIndex : lines.findIndex(l => l.some(m => m.id === currentNodeId));
					const line = lineIdx >= 0 ? lines[lineIdx] : null;
					if (line) {
						const idx = line.findIndex(m => m.id === currentNodeId);
						if (idx < line.length - 1) navigateToNode(line[idx + 1], lineIdx);
					}
				}
			} else {
				if (!currentNodeId) {
					if (tree.length > 0) navigateToNode(tree[0]);
				} else {
					const node = findNode(tree, currentNodeId);
					if (node && node.children.length > 0) navigateToNode(node.children[0]);
				}
			}
		}
	}

	function handleMoveUpdate(updated: MoveNode) {
		selectedMove = updated;
		tree = updateNodeInTree(tree, updated);
	}

	function startEditingName() {
		if (!repertoire) return;
		editName = repertoire.name;
		editingName = true;
	}

	async function saveNameEdit() {
		const trimmed = editName.trim();
		if (!trimmed || !repertoire) { editingName = false; return; }
		try {
			await api.updateRepertoire(repertoireId, trimmed);
			repertoire.name = trimmed;
		} catch (e) {
			console.error('Failed to rename:', e);
		}
		editingName = false;
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if loading}
	<p>Loading...</p>
{:else if repertoire}
	<div class="builder">
		<div class="toolbar">
			{#if editingName}
				<!-- svelte-ignore a11y_autofocus -->
				<input
					class="edit-name"
					bind:value={editName}
					autofocus
					onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') saveNameEdit(); if (e.key === 'Escape') editingName = false; }}
					onblur={() => saveNameEdit()}
				/>
			{:else}
				<button class="editable-name" onclick={startEditingName}><h2>{repertoire.name}</h2></button>
			{/if}
			<div class="toolbar-actions">
				<button class:active={showBestMoves} onclick={() => showBestMoves = !showBestMoves}>Best Moves</button>
				<button onclick={() => flipped = !flipped}>Flip Board</button>
				<button onclick={navigateToStart}>Start Position</button>
				<button onclick={() => (showImport = true)}>Import PGN</button>
				<button onclick={handleExport}>Export PGN</button>
				<a href="/train/{repertoireId}" class="btn">Drill</a>
			</div>
		</div>

		<div class="layout">
			<div class="sidebar-left">
				<div class="sidebar-header">
					<h3>Moves</h3>
					<div class="view-toggle">
						<button class:active={viewMode === 'tree'} onclick={() => viewMode = 'tree'}>Tree</button>
						<button class:active={viewMode === 'lines'} onclick={() => viewMode = 'lines'}>Lines</button>
					</div>
				</div>
				<div class="tree-container">
					{#if viewMode === 'tree'}
						<MoveTree
							nodes={tree}
							selectedId={currentNodeId}
							onSelect={navigateToNode}
							{transpositionIds}
							{gapIds}
						/>
					{:else}
						<MoveLines
							nodes={tree}
							selectedId={currentNodeId}
							onSelect={navigateToNode}
							{transpositionIds}
							{gapIds}
							{repertoireId}
							onRename={handleMoveUpdate}
							onDelete={handleLineDelete}
						/>
					{/if}
				</div>
			</div>

			<div class="board-area">
				<div class="board-with-eval">
					<EvalBar score={evalScore} mate={evalMate} orientation={boardOrientation} />
					<Board config={boardConfig} onMove={handleMove} shapes={allShapes} />
				</div>
			</div>

			<div class="sidebar-right">
				{#if currentOpening}
					<div class="opening-tag" style:border-left-color={ecoColor(currentOpening.eco)}>
						<span class="eco-code" style:background={ecoColor(currentOpening.eco)}>{currentOpening.eco}</span>
						<span class="opening-name">{currentOpening.name}</span>
					</div>
				{/if}
				{#if showBestMoves && engineLines.length > 0}
					<BestMoves lines={engineLines} fen={currentFen} orientation={boardOrientation} onPlayMove={(uci) => handleMove(uci.slice(0, 2), uci.slice(2, 4))} />
				{/if}
				<AnnotationEditor
					move={selectedMove}
					{repertoireId}
					onUpdate={handleMoveUpdate}
				/>
				{#if selectedMove}
					<a href="/train/{repertoireId}?from_move_id={selectedMove.id}" class="drill-from-btn">
						Drill from here
					</a>
					<button class="delete-btn" onclick={handleDeleteMove}>
						Delete Move
					</button>
				{/if}
			</div>
		</div>
	</div>

	<PgnImportModal
		{repertoireId}
		bind:open={showImport}
		onImported={loadData}
	/>
{/if}

<style>
	.builder {
		max-width: 1300px;
	}

	.editable-name {
		all: unset;
		cursor: pointer;
	}

	.editable-name h2 {
		border-bottom: 1px dashed transparent;
	}

	.editable-name:hover h2 {
		border-bottom-color: var(--color-muted);
	}

	.edit-name {
		font-size: 1.5rem;
		font-weight: 600;
		padding: 0 0.25rem;
		border: 1px solid var(--color-primary);
		border-radius: 4px;
		outline: none;
		background: transparent;
		color: var(--color-text);
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

	.toolbar-actions button.active {
		background: var(--color-primary);
		color: white;
		border-color: var(--color-primary);
	}

	.layout {
		display: grid;
		grid-template-columns: 320px 1fr 250px;
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

	.sidebar-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 1rem;
		border-bottom: 1px solid var(--color-border);
	}

	.sidebar-header h3 {
		font-size: 0.9rem;
		margin: 0;
	}

	.view-toggle {
		display: flex;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		overflow: hidden;
	}

	.view-toggle button {
		padding: 0.2rem 0.5rem;
		border: none;
		background: transparent;
		cursor: pointer;
		font-size: 0.75rem;
		color: var(--color-text);
	}

	.view-toggle button + button {
		border-left: 1px solid var(--color-border);
	}

	.view-toggle button.active {
		background: var(--color-primary);
		color: white;
	}

	.tree-container {
		max-height: 600px;
		overflow-y: auto;
	}

	.board-area {
		display: flex;
		justify-content: center;
	}

	.board-with-eval {
		display: grid;
		grid-template-columns: 28px 1fr;
		gap: 0.35rem;
		max-width: 600px;
		width: 100%;
	}

	.drill-from-btn {
		display: block;
		margin: 1rem 1rem 0;
		padding: 0.4rem 0.75rem;
		background: var(--color-primary);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
		text-align: center;
		text-decoration: none;
	}

	.drill-from-btn:hover {
		opacity: 0.9;
	}

	.delete-btn {
		margin: 1rem;
		padding: 0.4rem 0.75rem;
		background: none;
		border: 1px solid var(--color-danger);
		color: var(--color-danger);
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
	}

	.delete-btn:hover {
		background: #fed7d7;
	}

	.opening-tag {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.6rem 0.75rem;
		border-left: 3px solid;
		background: var(--color-surface);
		border-bottom: 1px solid var(--color-border);
	}

	.eco-code {
		font-size: 0.7rem;
		font-weight: 700;
		color: white;
		padding: 1px 6px;
		border-radius: 3px;
		flex-shrink: 0;
	}

	.opening-name {
		font-size: 0.8rem;
		color: var(--color-text);
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

		.view-toggle button {
			padding: 0.4rem 0.75rem;
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

		.delete-btn {
			min-height: 44px;
			padding: 0.5rem 0.75rem;
		}

		.drill-from-btn {
			min-height: 44px;
			padding: 0.5rem 0.75rem;
			display: flex;
			align-items: center;
			justify-content: center;
		}
	}
</style>
