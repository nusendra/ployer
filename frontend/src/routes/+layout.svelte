<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Ployer</title>
</svelte:head>

<div class="app-shell">
	<nav class="sidebar">
		<div class="logo">
			<h1>Ployer</h1>
		</div>
		<ul class="nav-links">
			<li><a href="/">Dashboard</a></li>
			<li><a href="/apps">Applications</a></li>
			<li><a href="/servers">Servers</a></li>
			<li><a href="/settings">Settings</a></li>
		</ul>
	</nav>
	<main class="content">
		{@render children()}
	</main>
</div>

<style>
	.app-shell {
		display: flex;
		min-height: 100vh;
	}

	.sidebar {
		width: 240px;
		background: var(--bg-secondary);
		border-right: 1px solid var(--border);
		padding: 1.5rem;
		flex-shrink: 0;
	}

	.logo h1 {
		font-size: 1.5rem;
		color: var(--primary);
		margin-bottom: 2rem;
	}

	.nav-links {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.nav-links a {
		display: block;
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius);
		color: var(--text-muted);
		transition: all 0.15s;
	}

	.nav-links a:hover {
		background: var(--bg-tertiary);
		color: var(--text);
	}

	.content {
		flex: 1;
		padding: 2rem;
		overflow-y: auto;
	}
</style>
