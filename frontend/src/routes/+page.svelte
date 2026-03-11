<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/stores/api';
	import type { Repertoire, BundleExport } from '$lib/types';

	let repertoires = $state<Repertoire[]>([]);
	let loading = $state(true);
	let showCreate = $state(false);
	let newName = $state('');
	let newColor = $state<'white' | 'black'>('white');
	let creating = $state(false);
	let editingId = $state<string | null>(null);
	let editName = $state('');
	let exporting = $state(false);
	let importing = $state(false);
	let bundleError = $state<string | null>(null);

	onMount(async () => {
		await loadRepertoires();
	});

	async function loadRepertoires() {
		loading = true;
		try {
			repertoires = await api.listRepertoires();
		} catch (e) {
			console.error('Failed to load repertoires:', e);
		}
		loading = false;
	}

	async function createRepertoire() {
		if (!newName.trim()) return;
		creating = true;
		try {
			await api.createRepertoire({ name: newName.trim(), color: newColor });
			newName = '';
			showCreate = false;
			await loadRepertoires();
		} catch (e) {
			console.error('Failed to create repertoire:', e);
		}
		creating = false;
	}

	function startEditing(rep: Repertoire) {
		editingId = rep.id;
		editName = rep.name;
	}

	async function saveEdit(id: string) {
		const trimmed = editName.trim();
		if (!trimmed) { editingId = null; return; }
		try {
			await api.updateRepertoire(id, trimmed);
			const rep = repertoires.find(r => r.id === id);
			if (rep) rep.name = trimmed;
		} catch (e) {
			console.error('Failed to rename:', e);
		}
		editingId = null;
	}

	async function exportAll() {
		exporting = true;
		bundleError = null;
		try {
			const bundle = await api.exportBundle();
			const json = JSON.stringify(bundle, null, 2);
			const blob = new Blob([json], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `zugzwang-backup-${new Date().toISOString().slice(0, 10)}.json`;
			a.click();
			URL.revokeObjectURL(url);
		} catch (e) {
			bundleError = e instanceof Error ? e.message : 'Export failed';
		}
		exporting = false;
	}

	async function importBundle() {
		const input = document.createElement('input');
		input.type = 'file';
		input.accept = '.json';
		input.onchange = async () => {
			const file = input.files?.[0];
			if (!file) return;
			importing = true;
			bundleError = null;
			try {
				const text = await file.text();
				const data: BundleExport = JSON.parse(text);
				await api.importBundle(data);
				await loadRepertoires();
			} catch (e) {
				bundleError = e instanceof Error ? e.message : 'Import failed';
			}
			importing = false;
		};
		input.click();
	}

	async function deleteRepertoire(id: string) {
		if (!confirm('Delete this repertoire?')) return;
		try {
			await api.deleteRepertoire(id);
			await loadRepertoires();
		} catch (e) {
			console.error('Failed to delete:', e);
		}
	}
</script>

<div class="home">
	<div class="header">
		<h1>Your Repertoires</h1>
		<div class="header-actions">
			<button class="btn" onclick={importBundle} disabled={importing}>
				{importing ? 'Importing...' : 'Import'}
			</button>
			<button class="btn" onclick={exportAll} disabled={exporting}>
				{exporting ? 'Exporting...' : 'Export All'}
			</button>
			<button class="primary" onclick={() => (showCreate = !showCreate)}>
				{showCreate ? 'Cancel' : '+ New Repertoire'}
			</button>
		</div>
	</div>

	{#if bundleError}
		<p class="error">{bundleError}</p>
	{/if}

	{#if showCreate}
		<form class="create-form" onsubmit={(e) => { e.preventDefault(); createRepertoire(); }}>
			<input
				bind:value={newName}
				placeholder="Repertoire name"
				required
			/>
			<select bind:value={newColor}>
				<option value="white">White</option>
				<option value="black">Black</option>
			</select>
			<button type="submit" class="primary" disabled={creating}>
				{creating ? 'Creating...' : 'Create'}
			</button>
		</form>
	{/if}

	{#if loading}
		<p class="muted">Loading...</p>
	{:else if repertoires.length === 0}
		<p class="muted">No repertoires yet. Create one to get started.</p>
	{:else}
		<div class="grid">
			{#each repertoires as rep}
				<div class="card">
					<div class="card-header">
						<span class="color-badge" class:white={rep.color === 'white'} class:black={rep.color === 'black'}>
							{rep.color}
						</span>
						{#if editingId === rep.id}
							<!-- svelte-ignore a11y_autofocus -->
							<input
								class="edit-name"
								bind:value={editName}
								autofocus
								onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') saveEdit(rep.id); if (e.key === 'Escape') editingId = null; }}
								onblur={() => saveEdit(rep.id)}
							/>
						{:else}
							<button class="editable-name" onclick={() => startEditing(rep)}><h3>{rep.name}</h3></button>
						{/if}
					</div>
					<div class="card-actions">
						<a href="/repertoire/{rep.id}" class="btn">Edit</a>
						<a href="/train/{rep.id}" class="btn">Drill</a>
						<button class="btn danger" onclick={() => deleteRepertoire(rep.id)}>
							Delete
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.home {
		max-width: 800px;
		margin: 0 auto;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.header-actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.error {
		color: var(--color-danger);
		background: #fed7d7;
		padding: 0.5rem 1rem;
		border-radius: 4px;
		margin-bottom: 1rem;
	}

	.create-form {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
		padding: 1rem;
		background: var(--color-surface);
		border-radius: 8px;
		border: 1px solid var(--color-border);
	}

	.create-form input {
		flex: 1;
		padding: 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
	}

	.create-form select {
		padding: 0.5rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
	}

	.grid {
		display: grid;
		gap: 1rem;
	}

	.card {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 1rem;
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.card-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.color-badge {
		padding: 2px 8px;
		border-radius: 4px;
		font-size: 0.75rem;
		text-transform: uppercase;
		font-weight: 600;
	}

	.color-badge.white {
		background: #f0f0f0;
		color: #333;
		border: 1px solid #ccc;
	}

	.color-badge.black {
		background: #333;
		color: #fff;
	}

	.card-actions {
		display: flex;
		gap: 0.5rem;
	}

	.btn {
		padding: 0.4rem 0.75rem;
		border-radius: 4px;
		border: 1px solid var(--color-border);
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.85rem;
		color: var(--color-text);
		text-decoration: none;
	}

	.btn:hover {
		background: var(--color-hover);
		text-decoration: none;
	}

	.btn.danger:hover {
		background: #fed7d7;
		border-color: var(--color-danger);
		color: var(--color-danger);
	}

	button.primary {
		padding: 0.5rem 1rem;
		background: var(--color-primary);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	button.primary:disabled {
		opacity: 0.6;
	}

	.editable-name {
		all: unset;
		cursor: pointer;
	}

	.editable-name h3 {
		border-bottom: 1px dashed transparent;
	}

	.editable-name:hover h3 {
		border-bottom-color: var(--color-muted);
	}

	.edit-name {
		font-size: inherit;
		font-weight: 600;
		padding: 0 0.25rem;
		border: 1px solid var(--color-primary);
		border-radius: 4px;
		outline: none;
		background: transparent;
		color: var(--color-text);
	}

	.muted {
		color: var(--color-muted);
	}
</style>
