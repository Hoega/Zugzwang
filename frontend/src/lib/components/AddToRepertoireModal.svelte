<script lang="ts">
	import { api } from '$lib/stores/api';
	import type { Repertoire } from '$lib/types';

	let { open = $bindable(false), pgn = '', openingName = '' }: {
		open?: boolean;
		pgn?: string;
		openingName?: string;
	} = $props();

	let repertoires = $state<Repertoire[]>([]);
	let loading = $state(false);
	let importing = $state(false);
	let error = $state('');
	let success = $state<{ repertoireId: string; moves: number } | null>(null);
	let colorFilter = $state<'white' | 'black'>('white');

	// Create new repertoire
	let creating = $state(false);
	let newName = $state('');

	let filtered = $derived(repertoires.filter((r) => r.color === colorFilter));

	$effect(() => {
		if (open) {
			loading = true;
			error = '';
			success = null;
			creating = false;
			newName = '';
			api.listRepertoires().then((r) => {
				repertoires = r;
				loading = false;
			}).catch((e) => {
				error = e.message || 'Failed to load repertoires';
				loading = false;
			});
		}
	});

	async function importToRepertoire(repertoireId: string) {
		importing = true;
		error = '';
		try {
			const res = await api.importPgn(repertoireId, pgn);
			success = { repertoireId, moves: res.imported_moves };
		} catch (e: any) {
			error = e.message || 'Import failed';
		}
		importing = false;
	}

	async function createAndImport() {
		if (!newName.trim()) return;
		importing = true;
		error = '';
		try {
			const rep = await api.createRepertoire({ name: newName.trim(), color: colorFilter });
			repertoires = [...repertoires, rep];
			const res = await api.importPgn(rep.id, pgn);
			success = { repertoireId: rep.id, moves: res.imported_moves };
		} catch (e: any) {
			error = e.message || 'Failed to create repertoire';
		}
		importing = false;
	}

	function close() {
		open = false;
		error = '';
		success = null;
	}
</script>

{#if open}
	<div class="modal-backdrop" onclick={close} onkeydown={close} role="button" tabindex="-1">
		<div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog">
			<h2>Add to Repertoire</h2>
			<p class="opening-label">{openingName}</p>

			{#if success}
				<p class="success">Imported {success.moves} moves</p>
				<div class="actions">
					<a href="/repertoire/{success.repertoireId}" class="btn primary">Open in Builder</a>
					<button class="secondary" onclick={close}>Close</button>
				</div>
			{:else}
				<div class="color-toggle">
					<button class:active={colorFilter === 'white'} onclick={() => (colorFilter = 'white')}>White</button>
					<button class:active={colorFilter === 'black'} onclick={() => (colorFilter = 'black')}>Black</button>
				</div>

				{#if loading}
					<p class="muted">Loading repertoires...</p>
				{:else}
					<div class="repertoire-list">
						{#each filtered as rep (rep.id)}
							<button class="rep-item" onclick={() => importToRepertoire(rep.id)} disabled={importing}>
								<span class="rep-name">{rep.name}</span>
								{#if rep.move_count != null}
									<span class="rep-count">{rep.move_count} moves</span>
								{/if}
							</button>
						{/each}
						{#if filtered.length === 0 && !creating}
							<p class="muted">No {colorFilter} repertoires yet</p>
						{/if}
					</div>

					{#if creating}
						<div class="create-form">
							<input
								type="text"
								placeholder="Repertoire name"
								bind:value={newName}
								onkeydown={(e) => { if (e.key === 'Enter') createAndImport(); }}
							/>
							<div class="create-actions">
								<button class="secondary" onclick={() => (creating = false)}>Cancel</button>
								<button onclick={createAndImport} disabled={importing || !newName.trim()}>
									{importing ? 'Creating...' : 'Create & Add'}
								</button>
							</div>
						</div>
					{:else}
						<button class="create-btn" onclick={() => { creating = true; newName = openingName; }}>
							+ Create new {colorFilter} repertoire
						</button>
					{/if}
				{/if}

				{#if error}
					<p class="error">{error}</p>
				{/if}

				<div class="actions">
					<button class="secondary" onclick={close}>Cancel</button>
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		background: var(--color-surface, white);
		border-radius: 8px;
		padding: 1.5rem;
		width: 90%;
		max-width: 450px;
		color: var(--color-text, #333);
	}

	h2 {
		margin: 0 0 0.25rem;
	}

	.opening-label {
		font-size: 0.85rem;
		color: var(--color-muted, #666);
		margin: 0 0 1rem;
	}

	.color-toggle {
		display: flex;
		border: 1px solid var(--color-border, #ccc);
		border-radius: 4px;
		overflow: hidden;
		margin-bottom: 0.75rem;
	}

	.color-toggle button {
		flex: 1;
		padding: 0.5rem;
		border: none;
		border-radius: 0;
		background: var(--color-surface, white);
		cursor: pointer;
		font-size: 0.85rem;
		font-weight: 600;
		color: var(--color-muted, #666);
		min-height: 44px;
	}

	.color-toggle button.active {
		background: var(--color-primary, #4a90d9);
		color: white;
	}

	.repertoire-list {
		max-height: 250px;
		overflow-y: auto;
		margin-bottom: 0.75rem;
	}

	.rep-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		width: 100%;
		padding: 0.6rem 0.75rem;
		background: none;
		border: none;
		border-bottom: 1px solid var(--color-border, #eee);
		cursor: pointer;
		text-align: left;
		color: var(--color-text, #333);
		font-size: 0.9rem;
		min-height: 44px;
	}

	.rep-item:hover {
		background: var(--color-hover, #f5f5f5);
	}

	.rep-item:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.rep-name {
		font-weight: 500;
	}

	.rep-count {
		font-size: 0.8rem;
		color: var(--color-muted, #999);
	}

	.muted {
		font-size: 0.85rem;
		color: var(--color-muted, #999);
		text-align: center;
		padding: 1rem 0;
	}

	.create-btn {
		width: 100%;
		padding: 0.6rem;
		background: none;
		border: 1px dashed var(--color-border, #ccc);
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
		color: var(--color-primary, #4a90d9);
		min-height: 44px;
		margin-bottom: 0.75rem;
	}

	.create-btn:hover {
		background: var(--color-hover, #f5f5f5);
	}

	.create-form {
		margin-bottom: 0.75rem;
	}

	.create-form input {
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: 1px solid var(--color-border, #ccc);
		border-radius: 4px;
		font-size: 0.85rem;
		background: var(--color-bg, white);
		color: var(--color-text, #333);
		margin-bottom: 0.5rem;
		min-height: 44px;
	}

	.create-actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
	}

	.error {
		color: #e53e3e;
		font-size: 0.9rem;
	}

	.success {
		color: #38a169;
		font-size: 0.95rem;
		margin-bottom: 1rem;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
		margin-top: 1rem;
	}

	button {
		padding: 0.5rem 1rem;
		background: var(--color-primary, #4a90d9);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		min-height: 44px;
	}

	button.secondary {
		background: var(--color-border, #e2e8f0);
		color: var(--color-text, #333);
	}

	button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.btn {
		display: inline-flex;
		align-items: center;
		padding: 0.5rem 1rem;
		background: var(--color-primary, #4a90d9);
		color: white;
		border: none;
		border-radius: 4px;
		text-decoration: none;
		font-size: 0.9rem;
		min-height: 44px;
	}

	.btn:hover {
		opacity: 0.9;
		text-decoration: none;
	}
</style>
