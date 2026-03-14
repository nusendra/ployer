<script lang="ts">
	import '../app.css';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { browser } from '$app/environment';
	import { clearAuth, user } from '$lib/stores/auth';
	import { wsClient } from '$lib/stores/websocket';
	import { onDestroy } from 'svelte';
	import Toast from '$lib/components/Toast.svelte';

	let { children } = $props();
	let isAuthenticated = $state(false);
	let isLoginPage = $state(false);
	let isChecking = $state(true);
	$effect(() => {
		if (!browser) return;

		const currentPath = $page.url.pathname;

		if (currentPath === '/login') {
			isLoginPage = true;
			isAuthenticated = false;
			isChecking = false;
			return;
		}

		isLoginPage = false;

		const token = localStorage.getItem('token');
		if (!token) {
			clearAuth();
			goto('/login');
			isAuthenticated = false;
			wsClient.disconnect();
		} else {
			isAuthenticated = true;
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

	const navItems = [
		{
			label: 'Dashboard',
			href: '/',
			icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="1" y="1" width="6" height="6" rx="1.5" fill="currentColor"/><rect x="9" y="1" width="6" height="6" rx="1.5" fill="currentColor"/><rect x="1" y="9" width="6" height="6" rx="1.5" fill="currentColor"/><rect x="9" y="9" width="6" height="6" rx="1.5" fill="currentColor"/></svg>`
		},
		{
			label: 'Applications',
			href: '/applications',
			icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M2 4C2 2.9 2.9 2 4 2H12C13.1 2 14 2.9 14 4V12C14 13.1 13.1 14 12 14H4C2.9 14 2 13.1 2 12V4Z" stroke="currentColor" stroke-width="1.5"/><path d="M5 8H11M8 5V11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`
		},
		{
			label: 'Servers',
			href: '/servers',
			icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><rect x="2" y="2" width="12" height="5" rx="1.5" stroke="currentColor" stroke-width="1.5"/><rect x="2" y="9" width="12" height="5" rx="1.5" stroke="currentColor" stroke-width="1.5"/><circle cx="11.5" cy="4.5" r="1" fill="currentColor"/><circle cx="11.5" cy="11.5" r="1" fill="currentColor"/></svg>`
		},
		{
			label: 'Containers',
			href: '/containers',
			icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M8 2L14 5.5V10.5L8 14L2 10.5V5.5L8 2Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M8 2V14M2 5.5L8 9L14 5.5" stroke="currentColor" stroke-width="1.5"/></svg>`
		},
	];

	const secondaryNavItems = [
		{
			label: 'Settings',
			href: '/settings',
			icon: `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="8" cy="8" r="2.5" stroke="currentColor" stroke-width="1.5"/><path d="M8 1.5V3M8 13V14.5M1.5 8H3M13 8H14.5M3.4 3.4L4.5 4.5M11.5 11.5L12.6 12.6M3.4 12.6L4.5 11.5M11.5 4.5L12.6 3.4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>`
		},
	];
</script>

<svelte:head>
	<title>Ployer</title>
</svelte:head>

<Toast />

{#if isChecking}
	<div class="loading-screen">
		<p>Loading...</p>
	</div>
{:else if isLoginPage}
	{@render children()}
{:else if isAuthenticated}
	<div class="app-shell">
		<nav class="sidebar">
			<!-- Logo -->
			<div class="sidebar-logo">
				<div class="logo-icon">
					<svg width="20" height="20" viewBox="0 0 20 20" fill="none" xmlns="http://www.w3.org/2000/svg">
						<path d="M10 2L18 6.5V13.5L10 18L2 13.5V6.5L10 2Z" fill="var(--primary)" stroke="var(--primary)" stroke-width="1" stroke-linejoin="round"/>
						<path d="M10 2V18M2 6.5L10 11L18 6.5" stroke="var(--bg-secondary)" stroke-width="1.5"/>
					</svg>
				</div>
				<span class="logo-text">Ployer</span>
			</div>

			<!-- Primary nav -->
			<ul class="nav-links">
				{#each navItems as item}
					<li>
						<a
							href={item.href}
							class:active={$page.url.pathname === item.href || ($page.url.pathname.startsWith(item.href) && item.href !== '/')}
						>
							<span class="nav-icon">{@html item.icon}</span>
							<span>{item.label}</span>
						</a>
					</li>
				{/each}
			</ul>

			<div class="nav-divider"></div>

			<!-- Secondary nav -->
			<ul class="nav-links">
				{#each secondaryNavItems as item}
					<li>
						<a href={item.href} class:active={$page.url.pathname === item.href}>
							<span class="nav-icon">{@html item.icon}</span>
							<span>{item.label}</span>
						</a>
					</li>
				{/each}
			</ul>

			<!-- User profile -->
			<div class="sidebar-footer">
				<div class="user-profile">
					<div class="user-avatar">
						{($user?.name ?? 'U')[0].toUpperCase()}
					</div>
					<div class="user-info">
						<span class="user-name">{$user?.name ?? 'User'}</span>
						<span class="user-email">{$user?.email ?? ''}</span>
					</div>
				</div>
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
		height: 100vh;
		overflow: hidden;
	}

	.sidebar {
		width: 260px;
		background: var(--bg-secondary);
		border-right: 1px solid var(--border);
		padding: 1.5rem 1rem;
		flex-shrink: 0;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		box-shadow: 4px 0 20px rgba(0, 0, 0, 0.2);
		height: 100vh;
		overflow-y: auto;
	}

	/* Logo */
	.sidebar-logo {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		padding: 0.5rem 0.75rem;
		margin-bottom: 0.5rem;
	}

	.logo-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
	}

	.logo-text {
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text);
		letter-spacing: -0.02em;
	}

	/* Nav */
	.nav-links {
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.nav-links a {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.625rem 0.75rem;
		border-radius: 7px;
		color: #ffffff;
		font-size: 0.875rem;
		font-weight: 500;
		transition: background 0.15s, color 0.15s;
		text-decoration: none;
	}

	.nav-links a:hover {
		background: var(--bg-tertiary);
		color: var(--text);
	}

	.nav-links a.active {
		background: var(--bg-tertiary);
		color: var(--primary);
	}

	.nav-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		opacity: 0.85;
	}

	.nav-links a.active .nav-icon {
		opacity: 1;
	}

	/* Divider */
	.nav-divider {
		height: 1px;
		background: var(--border);
		margin: 0.5rem 0;
	}

	/* Footer */
	.sidebar-footer {
		margin-top: auto;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
	}

	.user-profile {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 0.75rem;
		border-radius: 7px;
		cursor: pointer;
		transition: background 0.15s;
	}

	.user-profile:hover {
		background: var(--bg-tertiary);
	}

	.user-avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		background: var(--primary);
		color: var(--bg);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.8125rem;
		font-weight: 600;
		flex-shrink: 0;
	}

	.user-info {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		overflow: hidden;
	}

	.user-name {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.user-email {
		font-size: 0.75rem;
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.btn-logout {
		width: 100%;
		background: transparent;
		color: var(--text-muted);
		border: 1px solid var(--border);
		padding: 0.5rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
		transition: background 0.15s, color 0.15s;
	}

	.btn-logout:hover {
		background: var(--bg-tertiary);
		color: var(--text);
	}

	.content {
		flex: 1;
		padding: 2rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
	}
</style>
