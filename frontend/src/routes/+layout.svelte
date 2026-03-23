<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.svelte';

	let { children } = $props();

	onMount(() => {
		auth.check();
	});

	$effect(() => {
		if (auth.checked && !auth.isLoggedIn && $page.url.pathname !== '/login') {
			goto('/login');
		}
	});

	let isLoginPage = $derived($page.url.pathname === '/login');
</script>

{#if isLoginPage}
	<main class="login-main">
		{@render children()}
	</main>
{:else if !auth.checked}
	<div class="loading">Loading...</div>
{:else if auth.isLoggedIn}
	<nav class="top-nav">
		<a href="/" class="brand">Zugzwang</a>
		<div class="links">
			<a href="/">Repertoires</a>
			<a href="/openings">Openings</a>
			<a href="/explorer">Explorer</a>
			<a href="/dashboard">Dashboard</a>
		</div>
		<div class="user-menu">
			<a href="/settings" class="username">{auth.user?.username}</a>
			<button class="logout-btn" onclick={() => auth.logout()}>Log out</button>
		</div>
	</nav>

	<main>
		{@render children()}
	</main>

	<nav class="bottom-nav">
		<a href="/" class="bottom-tab">
			<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 19V9l8-6 8 6v10a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"/></svg>
			<span>Repertoires</span>
		</a>
		<a href="/openings" class="bottom-tab">
			<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>
			<span>Openings</span>
		</a>
		<a href="/explorer" class="bottom-tab">
			<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/></svg>
			<span>Explorer</span>
		</a>
		<a href="/dashboard" class="bottom-tab">
			<svg viewBox="0 0 24 24" width="22" height="22" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 20V10"/><path d="M12 20V4"/><path d="M6 20v-6"/></svg>
			<span>Dashboard</span>
		</a>
	</nav>
{/if}

<style>
	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 50vh;
		color: var(--color-muted);
		font-size: 1.1rem;
	}

	.login-main {
		max-width: 1200px;
		margin: 0 auto;
		padding: 1.5rem;
	}

	.top-nav {
		background: var(--color-surface);
		border-bottom: 1px solid var(--color-border);
		padding: 0.75rem 1.5rem;
		display: flex;
		align-items: center;
		gap: 2rem;
	}

	.brand {
		font-weight: 700;
		font-size: 1.25rem;
		color: var(--color-text);
	}

	.links {
		display: flex;
		gap: 1rem;
	}

	.user-menu {
		margin-left: auto;
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.username {
		font-size: 0.9rem;
		color: var(--color-muted);
		font-weight: 500;
		text-decoration: none;
	}

	.username:hover {
		color: var(--color-text);
	}

	.logout-btn {
		padding: 0.4rem 0.75rem;
		min-height: 36px;
		background: transparent;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		color: var(--color-muted);
		font-size: 0.85rem;
		cursor: pointer;
	}

	.logout-btn:hover {
		background: var(--color-hover);
		color: var(--color-text);
	}

	.bottom-nav {
		display: none;
	}

	main {
		max-width: 1200px;
		margin: 0 auto;
		padding: 1.5rem;
	}

	@media (max-width: 768px) {
		.top-nav {
			padding: 0.5rem 1rem;
			gap: 0.75rem;
		}

		.brand {
			font-size: 1.1rem;
		}

		.links {
			display: none;
		}

		.username {
			display: none;
		}

		.logout-btn {
			min-height: var(--tap-target);
			padding: 0.5rem 1rem;
		}

		main {
			padding: 0.75rem;
			padding-bottom: 64px;
		}

		.bottom-nav {
			display: flex;
			position: fixed;
			bottom: 0;
			left: 0;
			right: 0;
			background: var(--color-surface);
			border-top: 1px solid var(--color-border);
			z-index: 100;
		}

		.bottom-tab {
			flex: 1;
			display: flex;
			flex-direction: column;
			align-items: center;
			justify-content: center;
			min-height: 56px;
			gap: 0.2rem;
			color: var(--color-muted);
			font-size: 0.65rem;
			text-decoration: none;
		}

		.bottom-tab:hover {
			color: var(--color-primary);
			text-decoration: none;
		}
	}
</style>
