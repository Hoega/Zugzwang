<script lang="ts">
	import { api } from '$lib/stores/api';
	import { auth } from '$lib/stores/auth.svelte';

	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let error = $state('');
	let success = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		success = '';

		if (newPassword.length < 8) {
			error = 'New password must be at least 8 characters';
			return;
		}

		if (newPassword !== confirmPassword) {
			error = 'New passwords do not match';
			return;
		}

		loading = true;

		try {
			await api.changePassword({
				current_password: currentPassword,
				new_password: newPassword
			});
			success = 'Password changed successfully';
			currentPassword = '';
			newPassword = '';
			confirmPassword = '';
		} catch (err: any) {
			error = err.message || 'Something went wrong';
		} finally {
			loading = false;
		}
	}
</script>

<div class="settings-page">
	<div class="settings-card">
		<h1>Settings</h1>

		<section class="section">
			<h2>Account</h2>
			<p class="info">Logged in as <strong>{auth.user?.username}</strong></p>
		</section>

		<section class="section">
			<h2>Change Password</h2>

			<form onsubmit={handleSubmit}>
				<label class="field">
					<span>Current Password</span>
					<input
						type="password"
						bind:value={currentPassword}
						autocomplete="current-password"
						required
					/>
				</label>

				<label class="field">
					<span>New Password</span>
					<input
						type="password"
						bind:value={newPassword}
						autocomplete="new-password"
						minlength={8}
						required
					/>
				</label>

				<label class="field">
					<span>Confirm New Password</span>
					<input
						type="password"
						bind:value={confirmPassword}
						autocomplete="new-password"
						minlength={8}
						required
					/>
				</label>

				{#if error}
					<p class="error">{error}</p>
				{/if}

				{#if success}
					<p class="success">{success}</p>
				{/if}

				<button type="submit" class="submit" disabled={loading}>
					{loading ? '...' : 'Change Password'}
				</button>
			</form>
		</section>
	</div>
</div>

<style>
	.settings-page {
		display: flex;
		justify-content: center;
		padding: 1rem;
	}

	.settings-card {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 12px;
		padding: 2rem;
		width: 100%;
		max-width: 480px;
	}

	h1 {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--color-text);
		margin-bottom: 1.5rem;
	}

	.section {
		margin-bottom: 2rem;
	}

	h2 {
		font-size: 1.1rem;
		font-weight: 600;
		color: var(--color-text);
		margin-bottom: 0.75rem;
	}

	.info {
		font-size: 0.9rem;
		color: var(--color-muted);
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

	.success {
		color: var(--color-success, #22c55e);
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
