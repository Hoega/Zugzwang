<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';

	let mode = $state<'login' | 'register'>('login');
	let username = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		try {
			if (mode === 'login') {
				await auth.login(username, password);
			} else {
				await auth.register(username, password);
			}
			goto('/');
		} catch (err: any) {
			error = err.message || 'Something went wrong';
		} finally {
			loading = false;
		}
	}
</script>

<div class="login-page">
	<div class="login-card">
		<h1 class="logo">Zugzwang</h1>
		<p class="subtitle">Chess Opening Trainer</p>

		<div class="tabs">
			<button
				class="tab"
				class:active={mode === 'login'}
				onclick={() => { mode = 'login'; error = ''; }}
			>Log In</button>
			<button
				class="tab"
				class:active={mode === 'register'}
				onclick={() => { mode = 'register'; error = ''; }}
			>Register</button>
		</div>

		<form onsubmit={handleSubmit}>
			<label class="field">
				<span>Username</span>
				<input
					type="text"
					bind:value={username}
					autocomplete="username"
					minlength={3}
					maxlength={32}
					required
				/>
			</label>

			<label class="field">
				<span>Password</span>
				<input
					type="password"
					bind:value={password}
					autocomplete={mode === 'login' ? 'current-password' : 'new-password'}
					minlength={8}
					required
				/>
			</label>

			{#if error}
				<p class="error">{error}</p>
			{/if}

			<button type="submit" class="submit" disabled={loading}>
				{loading ? '...' : mode === 'login' ? 'Log In' : 'Create Account'}
			</button>
		</form>
	</div>
</div>

<style>
	.login-page {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 80vh;
		padding: 1rem;
	}

	.login-card {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		padding: 2rem;
		width: 100%;
		max-width: 400px;
	}

	.logo {
		font-size: 1.75rem;
		font-weight: 700;
		text-align: center;
		color: var(--color-text);
	}

	.subtitle {
		text-align: center;
		color: var(--color-muted);
		margin-bottom: 1.5rem;
		font-size: 0.9rem;
	}

	.tabs {
		display: flex;
		gap: 0;
		margin-bottom: 1.5rem;
		border: 1px solid var(--color-border);
		border-radius: 8px;
		overflow: hidden;
	}

	.tab {
		flex: 1;
		padding: 0.75rem;
		min-height: var(--tap-target);
		border: none;
		background: var(--color-bg);
		color: var(--color-muted);
		font-size: 0.95rem;
		font-weight: 500;
		cursor: pointer;
	}

	.tab.active {
		background: var(--color-primary);
		color: white;
	}

	.field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
		margin-bottom: 1rem;
	}

	.field span {
		font-size: 0.85rem;
		font-weight: 500;
		color: var(--color-muted);
	}

	.field input {
		padding: 0.75rem;
		min-height: var(--tap-target);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		font-size: 1rem;
		background: var(--color-bg);
		color: var(--color-text);
	}

	.field input:focus {
		outline: 2px solid var(--color-primary);
		outline-offset: -1px;
		border-color: var(--color-primary);
	}

	.error {
		color: var(--color-danger);
		font-size: 0.9rem;
		margin-bottom: 1rem;
		text-align: center;
	}

	.submit {
		width: 100%;
		padding: 0.75rem;
		min-height: var(--tap-target);
		background: var(--color-primary);
		color: white;
		border: none;
		border-radius: 8px;
		font-size: 1rem;
		font-weight: 600;
		cursor: pointer;
	}

	.submit:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.submit:hover:not(:disabled) {
		filter: brightness(1.1);
	}
</style>
