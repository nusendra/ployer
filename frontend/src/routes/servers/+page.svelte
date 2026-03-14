<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';

	interface Server {
		id: string;
		name: string;
		host: string;
		status: 'online' | 'offline' | 'unknown';
		last_seen_at: string | null;
		created_at: string;
	}

	interface ServerStats {
		total_memory_mb: number;
		used_memory_mb: number;
		cpu_count: number;
		cpu_usage: number;
	}

	let server = $state<Server | null>(null);
	let stats = $state<ServerStats | null>(null);
	let loading = $state(true);
	let loadingStats = $state(false);
	let error = $state('');

	onMount(async () => {
		await loadServer();
	});

	async function loadServer() {
		loading = true;
		error = '';
		try {
			const res = await api.get<{ servers: Server[] }>('/servers');
			server = res.servers.find((s: any) => s.is_local) ?? res.servers[0] ?? null;
			if (server) await loadStats(server.id);
		} catch (e: any) {
			error = e.message || 'Failed to load server';
		} finally {
			loading = false;
		}
	}

	async function loadStats(id: string) {
		loadingStats = true;
		try {
			const res = await api.get<{ stats: ServerStats }>(`/servers/${id}/resources`);
			stats = res.stats;
		} catch {
			stats = null;
		} finally {
			loadingStats = false;
		}
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return 'Never';
		return new Date(dateStr).toLocaleString();
	}

	function memPercent(): number {
		if (!stats) return 0;
		return Math.round((stats.used_memory_mb / stats.total_memory_mb) * 100);
	}
</script>

<div class="servers-page">
	<div class="page-header">
		<h2>Server</h2>
		<p>Local machine running Ployer.</p>
	</div>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	{#if loading}
		<div class="loading">Loading server...</div>
	{:else if !server}
		<div class="empty">No local server found.</div>
	{:else}
		<div class="server-card">
			<!-- Header -->
			<div class="card-header">
				<div class="server-avatar">
					<svg width="20" height="20" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
						<rect x="2" y="2" width="12" height="5" rx="1.5" stroke="currentColor" stroke-width="1.5"/>
						<rect x="2" y="9" width="12" height="5" rx="1.5" stroke="currentColor" stroke-width="1.5"/>
						<circle cx="11.5" cy="4.5" r="1" fill="currentColor"/>
						<circle cx="11.5" cy="11.5" r="1" fill="currentColor"/>
					</svg>
				</div>
				<div class="server-info">
					<div class="server-name-row">
						<h3>{server.name}</h3>
						<span class="badge-local">Local</span>
					</div>
					<span class="status-chip status-{server.status}">{server.status}</span>
				</div>
				<div class="server-meta">
					<div class="meta-item">
						<span class="meta-label">Host</span>
						<span class="meta-value">{server.host}</span>
					</div>
					<div class="meta-item">
						<span class="meta-label">Last Seen</span>
						<span class="meta-value">{formatDate(server.last_seen_at)}</span>
					</div>
				</div>
			</div>

			<!-- Resource stats -->
			<div class="stats-section">
				<div class="stats-header">
					<h4>Resources</h4>
					<button class="btn-refresh" onclick={() => loadStats(server!.id)} disabled={loadingStats}>
						{loadingStats ? 'Refreshing…' : 'Refresh'}
					</button>
				</div>

				{#if loadingStats}
					<div class="loading-inline">Loading stats...</div>
				{:else if stats}
					<div class="stats-grid">
						<div class="stat-card">
							<div class="stat-label">CPU Cores</div>
							<div class="stat-value">{stats.cpu_count}</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">CPU Usage</div>
							<div class="stat-value">{stats.cpu_usage.toFixed(1)}%</div>
							<div class="stat-bar">
								<div class="stat-bar-fill" style="width: {Math.min(stats.cpu_usage, 100)}%; background: {stats.cpu_usage > 80 ? 'var(--danger)' : 'var(--primary)'}"></div>
							</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Memory Used</div>
							<div class="stat-value">{stats.used_memory_mb.toLocaleString()} MB <span class="stat-sub">/ {stats.total_memory_mb.toLocaleString()} MB</span></div>
							<div class="stat-bar">
								<div class="stat-bar-fill" style="width: {memPercent()}%; background: {memPercent() > 80 ? 'var(--danger)' : 'var(--primary)'}"></div>
							</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Memory Free</div>
							<div class="stat-value">{(stats.total_memory_mb - stats.used_memory_mb).toLocaleString()} MB</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.servers-page {
		max-width: 860px;
	}

	.page-header {
		margin-bottom: 1.75rem;
	}

	.page-header h2 {
		margin: 0 0 0.25rem;
		font-size: 1.375rem;
		font-weight: 700;
		color: var(--text);
	}

	.page-header p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	.error-banner {
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--danger);
		padding: 0.75rem 1rem;
		border-radius: var(--radius);
		margin-bottom: 1.25rem;
		font-size: 0.875rem;
	}

	.loading, .empty {
		color: var(--text-muted);
		font-size: 0.875rem;
		padding: 2rem 0;
	}

	.loading-inline {
		color: var(--text-muted);
		font-size: 0.875rem;
		padding: 1rem 0;
	}

	/* ── Server card ── */
	.server-card {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 12px;
		overflow: hidden;
	}

	.card-header {
		display: flex;
		align-items: center;
		gap: 1rem;
		padding: 1.5rem;
		border-bottom: 1px solid var(--border);
		flex-wrap: wrap;
	}

	.server-avatar {
		width: 44px;
		height: 44px;
		border-radius: 10px;
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.server-info {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		min-width: 0;
	}

	.server-name-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.server-name-row h3 {
		margin: 0;
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--text);
	}

	.badge-local {
		padding: 0.1rem 0.45rem;
		font-size: 0.6875rem;
		font-weight: 600;
		border-radius: 20px;
		background: rgba(50, 130, 184, 0.2);
		color: var(--primary);
	}

	.status-chip {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.15rem 0.6rem;
		border-radius: 20px;
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: capitalize;
		width: fit-content;
	}

	.status-chip::before {
		content: '';
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: currentColor;
	}

	.status-online  { background: rgba(34, 197, 94, 0.15); color: var(--success); }
	.status-offline { background: rgba(239, 68, 68, 0.15); color: var(--danger); }
	.status-unknown { background: rgba(126, 137, 172, 0.15); color: var(--text-muted); }

	.server-meta {
		display: flex;
		gap: 2rem;
		margin-left: auto;
	}

	.meta-item {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.meta-label {
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.meta-value {
		font-size: 0.875rem;
		color: var(--text);
		font-weight: 500;
	}

	/* ── Stats ── */
	.stats-section {
		padding: 1.5rem;
	}

	.stats-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1.25rem;
	}

	.stats-header h4 {
		margin: 0;
		font-size: 0.9375rem;
		font-weight: 600;
		color: var(--text);
	}

	.btn-refresh {
		padding: 0.3125rem 0.75rem;
		border-radius: 6px;
		font-size: 0.75rem;
		font-weight: 500;
		background: transparent;
		color: var(--text-muted);
		border: 1px solid var(--border);
		cursor: pointer;
		transition: color 0.15s, border-color 0.15s;
	}

	.btn-refresh:hover:not(:disabled) {
		color: var(--primary);
		border-color: var(--primary);
	}

	.btn-refresh:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	.stat-card {
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.125rem 1.25rem;
	}

	.stat-label {
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		margin-bottom: 0.5rem;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text);
		margin-bottom: 0.625rem;
	}

	.stat-sub {
		font-size: 0.8125rem;
		font-weight: 400;
		color: var(--text-muted);
	}

	.stat-bar {
		height: 4px;
		background: var(--border);
		border-radius: 2px;
		overflow: hidden;
	}

	.stat-bar-fill {
		height: 100%;
		border-radius: 2px;
		transition: width 0.4s ease;
	}
</style>
