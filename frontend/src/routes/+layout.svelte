<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<nav>
	<a href="/" class="brand">Zugzwang</a>
	<div class="links">
		<a href="/">Repertoires</a>
		<a href="/openings">Openings</a>
		<a href="/explorer">Explorer</a>
		<a href="/dashboard">Dashboard</a>
	</div>
</nav>

<main>
	{@render children()}
</main>

<style>
	nav {
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

	main {
		max-width: 1200px;
		margin: 0 auto;
		padding: 1.5rem;
	}

	@media (max-width: 768px) {
		nav {
			padding: 0.5rem 1rem;
			gap: 1rem;
		}

		main {
			padding: 0.75rem;
		}
	}
</style>
