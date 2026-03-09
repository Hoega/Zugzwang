<script lang="ts">
	import { api } from '$lib/stores/api';

	let { repertoireId, open = $bindable(false), onImported = () => {} }: {
		repertoireId: string;
		open?: boolean;
		onImported?: () => void;
	} = $props();

	let pgnText = $state('');
	let importing = $state(false);
	let error = $state('');
	let result = $state('');

	async function handleImport() {
		if (!pgnText.trim()) return;
		importing = true;
		error = '';
		result = '';
		try {
			const res = await api.importPgn(repertoireId, pgnText);
			result = `Imported ${res.imported_moves} moves`;
			pgnText = '';
			onImported();
		} catch (e: any) {
			error = e.message || 'Import failed';
		}
		importing = false;
	}

	function handleFile(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		const reader = new FileReader();
		reader.onload = () => {
			pgnText = reader.result as string;
		};
		reader.readAsText(file);
	}

	function close() {
		open = false;
		error = '';
		result = '';
	}
</script>

{#if open}
	<div class="modal-backdrop" onclick={close} onkeydown={close} role="button" tabindex="-1">
		<div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()} role="dialog">
			<h2>Import PGN</h2>

			<textarea
				bind:value={pgnText}
				rows={10}
				placeholder="Paste PGN here..."
			></textarea>

			<div class="file-upload">
				<span>or</span>
				<input type="file" accept=".pgn" onchange={handleFile} />
			</div>

			{#if error}
				<p class="error">{error}</p>
			{/if}

			{#if result}
				<p class="success">{result}</p>
			{/if}

			<div class="actions">
				<button class="secondary" onclick={close}>Cancel</button>
				<button onclick={handleImport} disabled={importing || !pgnText.trim()}>
					{importing ? 'Importing...' : 'Import'}
				</button>
			</div>
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
		background: white;
		border-radius: 8px;
		padding: 1.5rem;
		width: 90%;
		max-width: 500px;
	}

	h2 {
		margin: 0 0 1rem;
	}

	textarea {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ccc;
		border-radius: 4px;
		font-family: monospace;
		resize: vertical;
	}

	.file-upload {
		margin: 0.75rem 0;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.9rem;
		color: #666;
	}

	.error {
		color: #e53e3e;
		font-size: 0.9rem;
	}

	.success {
		color: #38a169;
		font-size: 0.9rem;
	}

	.actions {
		display: flex;
		gap: 0.5rem;
		justify-content: flex-end;
		margin-top: 1rem;
	}

	button {
		padding: 0.5rem 1rem;
		background: #4a90d9;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	button.secondary {
		background: #e2e8f0;
		color: #333;
	}

	button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
