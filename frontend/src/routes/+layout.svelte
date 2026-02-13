<script lang="ts">
	import '../app.css';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { clearAuth } from '$lib/stores/auth';
	import { wsClient } from '$lib/stores/websocket';
	import { onDestroy } from 'svelte';

	let { children } = $props();
	let isAuthenticated = $state(false);
	let isChecking = $state(true);

	// Check auth on every route change
	$effect(() => {
		if (!browser) return;

		const currentPath = $page.url.pathname;

		// Skip auth check for login page
		if (currentPath === '/login') {
			isAuthenticated = true;
			isChecking = false;
			return;
		}

		// Check if user has a token
		const token = localStorage.getItem('token');
		if (!token) {
			clearAuth();
			goto('/login');
			isAuthenticated = false;
			wsClient.disconnect();
		} else {
			isAuthenticated = true;
			// Connect to WebSocket with token
			wsClient.connect(token);
		}
		isChecking = false;
	});

	onDestroy(() => {
		wsClient.disconnect();
	});

	function handleLogout() {
		wsClient.disconnect();
		clearAuth();
		goto('/login');
	}
</script>

<svelte:head>
	<title>Ployer</title>
</svelte:head>

{#if isChecking}
	<div class="loading-screen">
		<p>Loading...</p>
	</div>
{:else if isAuthenticated}
	<div class="app-shell">
		<nav class="sidebar">
			<div class="logo">
				<h1>Ployer</h1>
			</div>
			<ul class="nav-links">
				<li><a href="/">Dashboard</a></li>
				<li><a href="/applications">Applications</a></li>
				<li><a href="/servers">Servers</a></li>
				<li><a href="/containers">Containers</a></li>
				<li><a href="/settings">Settings</a></li>
			</ul>
			<div class="sidebar-footer">
				<button class="btn-logout" onclick={handleLogout}>Logout</button>
			</div>
		</nav>
		<main class="content">
			{@render children()}
		</main>
	</div>
{/if}

<style>
	.loading-screen {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		background: var(--bg);
		color: var(--text-muted);
	}

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
		display: flex;
		flex-direction: column;
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

	.sidebar-footer {
		margin-top: auto;
		padding-top: 2rem;
	}

	.btn-logout {
		width: 100%;
		background: transparent;
		color: var(--text-muted);
		border: 1px solid var(--border);
		padding: 0.5rem;
	}

	.btn-logout:hover {
		background: var(--bg-tertiary);
		color: var(--text);
	}
</style>
