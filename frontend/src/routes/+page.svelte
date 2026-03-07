<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { user } from '$lib/stores/auth';

	let applications = $state<any[]>([]);
	let servers = $state<any[]>([]);
	let deployments = $state<any[]>([]);
	let loading = $state(true);

	onMount(async () => {
		await loadDashboardData();
	});

	async function loadDashboardData() {
		loading = true;
		try {
			const [appsRes, serversRes, deploymentsRes] = await Promise.all([
				api.get<{ applications: any[] }>('/applications'),
				api.get<{ servers: any[] }>('/servers'),
				api.get<{ deployments: any[] }>('/deployments')
			]);

			applications = appsRes.applications;
			servers = serversRes.servers;
			deployments = deploymentsRes.deployments.slice(0, 10);
		} catch (e) {
			console.error('Failed to load dashboard data:', e);
		} finally {
			loading = false;
		}
	}

	function getHealthyAppsCount() {
		return applications.filter((app) => app.status === 'running').length;
	}

	function getOnlineServersCount() {
		return servers.filter((s) => s.status === 'online').length;
	}

	function getRecentDeploymentsCount() {
		const oneDayAgo = new Date(Date.now() - 24 * 60 * 60 * 1000);
		return deployments.filter((d) => new Date(d.created_at) > oneDayAgo).length;
	}

	function getDeploymentStatusColor(status: string) {
		switch (status) {
			case 'running': return 'green';
			case 'failed': return 'red';
			case 'building':
			case 'deploying': return 'blue';
			default: return 'gray';
		}
	}

	function timeAgo(dateStr: string) {
		const diff = Date.now() - new Date(dateStr).getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 60) return `${mins}m ago`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h ago`;
		const days = Math.floor(hours / 24);
		return `${days}d ago`;
	}
</script>

<div class="dashboard">
	<!-- Header -->
	<div class="dashboard-header">
		<div>
			<h2>Welcome back, {$user?.name ?? 'User'}</h2>
			<p class="subtitle">Manage your applications and monitor deployments.</p>
		</div>
	</div>

	{#if loading}
		<div class="loading">
			<div class="loading-spinner"></div>
			<span>Loading dashboard...</span>
		</div>
	{:else}
		<!-- Stats Cards -->
		<div class="stats-grid">
			<div class="stat-card">
				<div class="stat-header">
					<div class="stat-icon icon-apps">
						<svg width="18" height="18" viewBox="0 0 16 16" fill="none"><rect x="1" y="1" width="6" height="6" rx="1.5" fill="currentColor"/><rect x="9" y="1" width="6" height="6" rx="1.5" fill="currentColor"/><rect x="1" y="9" width="6" height="6" rx="1.5" fill="currentColor"/><rect x="9" y="9" width="6" height="6" rx="1.5" fill="currentColor"/></svg>
					</div>
					<span class="stat-label">Applications</span>
				</div>
				<div class="stat-body">
					<span class="stat-number">{applications.length}</span>
					<span class="stat-badge badge-success">{getHealthyAppsCount()} running</span>
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-header">
					<div class="stat-icon icon-servers">
						<svg width="18" height="18" viewBox="0 0 16 16" fill="none"><rect x="2" y="2" width="12" height="5" rx="1.5" stroke="currentColor" stroke-width="1.5"/><rect x="2" y="9" width="12" height="5" rx="1.5" stroke="currentColor" stroke-width="1.5"/><circle cx="11.5" cy="4.5" r="1" fill="currentColor"/><circle cx="11.5" cy="11.5" r="1" fill="currentColor"/></svg>
					</div>
					<span class="stat-label">Servers</span>
				</div>
				<div class="stat-body">
					<span class="stat-number">{servers.length}</span>
					<span class="stat-badge badge-success">{getOnlineServersCount()} online</span>
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-header">
					<div class="stat-icon icon-deploys">
						<svg width="18" height="18" viewBox="0 0 16 16" fill="none"><path d="M8 2L14 8L8 14" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/><path d="M14 8H2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
					</div>
					<span class="stat-label">Deployments (24h)</span>
				</div>
				<div class="stat-body">
					<span class="stat-number">{getRecentDeploymentsCount()}</span>
					<span class="stat-badge badge-info">Last 24 hours</span>
				</div>
			</div>

			<div class="stat-card">
				<div class="stat-header">
					<div class="stat-icon icon-health">
						<svg width="18" height="18" viewBox="0 0 16 16" fill="none"><path d="M8 14S2 10 2 6.5C2 4 4 2 6 2C7.1 2 7.6 2.5 8 3C8.4 2.5 8.9 2 10 2C12 2 14 4 14 6.5C14 10 8 14 8 14Z" fill="currentColor"/></svg>
					</div>
					<span class="stat-label">System Status</span>
				</div>
				<div class="stat-body">
					<span class="stat-number stat-healthy">Healthy</span>
					<span class="stat-badge badge-success">All systems</span>
				</div>
			</div>
		</div>

		<!-- Main content grid -->
		<div class="content-grid">
			<!-- Applications Status -->
			<div class="card">
				<div class="card-header">
					<h3>Applications Status</h3>
					<a href="/applications" class="card-link">View all →</a>
				</div>
				{#if applications.length === 0}
					<div class="empty-state">
						<p>No applications yet</p>
						<a href="/applications" class="btn-primary-sm">Create Application</a>
					</div>
				{:else}
					<div class="item-list">
						{#each applications as app (app.id)}
							<div class="item-row">
								<div class="item-left">
									<span class="item-dot dot-{app.status}"></span>
									<div class="item-info">
										<span class="item-name">{app.name}</span>
										<span class="item-sub">{app.git_branch || 'No branch'}</span>
									</div>
								</div>
								<span class="status-pill status-{app.status}">{app.status}</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Recent Deployments -->
			<div class="card">
				<div class="card-header">
					<h3>Recent Deployments</h3>
				</div>
				{#if deployments.length === 0}
					<div class="empty-state">
						<p>No deployments yet</p>
					</div>
				{:else}
					<div class="table-wrapper">
						<table class="data-table">
							<thead>
								<tr>
									<th>Application</th>
									<th>Time</th>
									<th>Status</th>
								</tr>
							</thead>
							<tbody>
								{#each deployments as deployment (deployment.id)}
									<tr>
										<td class="cell-name">{deployment.application_id.slice(0, 8)}...</td>
										<td class="cell-time">{timeAgo(deployment.created_at)}</td>
										<td>
											<span class="status-pill status-{getDeploymentStatusColor(deployment.status)}">
												{deployment.status}
											</span>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</div>
		</div>

		<!-- Server Status -->
		<div class="card" style="margin-top: 1.5rem;">
			<div class="card-header">
				<h3>Server Status</h3>
				<a href="/servers" class="card-link">View all →</a>
			</div>
			{#if servers.length === 0}
				<div class="empty-state">
					<p>No servers configured</p>
					<a href="/servers" class="btn-primary-sm">Add Server</a>
				</div>
			{:else}
				<div class="server-grid">
					{#each servers as server (server.id)}
						<div class="server-card">
							<div class="server-card-header">
								<span class="item-dot dot-{server.status === 'online' ? 'running' : 'stopped'}"></span>
								<span class="server-name">{server.name}</span>
							</div>
							<span class="server-host">{server.host}</span>
							<span class="status-pill status-{server.status === 'online' ? 'running' : 'stopped'}">{server.status}</span>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.dashboard {
		max-width: 1200px;
		margin: 0 auto;
	}

	/* Header */
	.dashboard-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.5rem;
	}

	h2 {
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--text);
		margin: 0;
	}

	.subtitle {
		font-size: 0.875rem;
		color: var(--text-muted);
		margin: 0.25rem 0 0 0;
	}

	/* Loading */
	.loading {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 4rem;
		color: var(--text-muted);
	}

	.loading-spinner {
		width: 20px;
		height: 20px;
		border: 2px solid var(--border);
		border-top-color: var(--primary);
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	/* Stats Grid */
	.stats-grid {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.stat-card {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.stat-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.stat-icon {
		width: 28px;
		height: 28px;
		border-radius: 6px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
	}

	.icon-apps { background: var(--primary); }
	.icon-servers { background: var(--success); }
	.icon-deploys { background: #00C2FF; }
	.icon-health { background: #ef4444; }

	.stat-label {
		font-size: 0.8125rem;
		color: var(--text-muted);
		font-weight: 500;
	}

	.stat-body {
		display: flex;
		align-items: baseline;
		gap: 0.75rem;
	}

	.stat-number {
		font-size: 1.75rem;
		font-weight: 700;
		color: var(--text);
	}

	.stat-healthy {
		color: var(--success);
		font-size: 1.25rem;
	}

	.stat-badge {
		font-size: 0.6875rem;
		font-weight: 600;
		padding: 0.125rem 0.5rem;
		border-radius: 10px;
	}

	.badge-success {
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
	}

	.badge-info {
		background: rgba(50, 130, 184, 0.2);
		color: var(--primary);
	}

	/* Content Grid */
	.content-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.5rem;
	}

	/* Cards */
	.card {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.25rem;
	}

	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.card-header h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		color: var(--text);
	}

	.card-link {
		font-size: 0.8125rem;
		color: var(--primary);
		text-decoration: none;
		font-weight: 500;
	}

	.card-link:hover {
		color: var(--primary-hover);
	}

	/* Empty State */
	.empty-state {
		text-align: center;
		padding: 2rem 1rem;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.75rem;
	}

	.empty-state p {
		margin: 0;
		color: var(--text-muted);
		font-size: 0.875rem;
	}

	.btn-primary-sm {
		font-size: 0.8125rem;
		padding: 0.375rem 0.75rem;
		background: var(--primary);
		color: var(--bg);
		border-radius: var(--radius);
		text-decoration: none;
		font-weight: 500;
	}

	/* Item list */
	.item-list {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.item-row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.625rem 0.75rem;
		border-radius: 6px;
		transition: background 0.15s;
	}

	.item-row:hover {
		background: var(--bg-tertiary);
	}

	.item-left {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.item-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		flex-shrink: 0;
	}

	.dot-running { background: var(--success); box-shadow: 0 0 6px rgba(34, 197, 94, 0.4); }
	.dot-stopped { background: var(--danger); }
	.dot-building, .dot-deploying { background: #00C2FF; }
	.dot-pending { background: var(--warning); }

	.item-info {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.item-name {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text);
	}

	.item-sub {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	/* Status pills */
	.status-pill {
		font-size: 0.6875rem;
		font-weight: 600;
		padding: 0.1875rem 0.625rem;
		border-radius: 10px;
		text-transform: capitalize;
	}

	.status-running {
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
	}

	.status-stopped {
		background: rgba(239, 68, 68, 0.15);
		color: var(--danger);
	}

	.status-green {
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
	}

	.status-red {
		background: rgba(239, 68, 68, 0.15);
		color: var(--danger);
	}

	.status-blue {
		background: rgba(0, 194, 255, 0.15);
		color: #00C2FF;
	}

	.status-gray {
		background: rgba(126, 137, 172, 0.15);
		color: var(--text-muted);
	}

	.status-pending {
		background: rgba(245, 158, 11, 0.15);
		color: var(--warning);
	}

	/* Table */
	.table-wrapper {
		overflow-x: auto;
	}

	.data-table {
		width: 100%;
		border-collapse: collapse;
	}

	.data-table th {
		text-align: left;
		font-size: 0.75rem;
		font-weight: 600;
		color: var(--text-muted);
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid var(--border);
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.data-table td {
		padding: 0.625rem 0.75rem;
		font-size: 0.8125rem;
		border-bottom: 1px solid var(--border);
	}

	.data-table tr:last-child td {
		border-bottom: none;
	}

	.data-table tbody tr {
		transition: background 0.15s;
	}

	.data-table tbody tr:hover {
		background: var(--bg-tertiary);
	}

	.cell-name {
		color: var(--text);
		font-weight: 500;
	}

	.cell-time {
		color: var(--text-muted);
	}

	/* Server grid */
	.server-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 0.75rem;
	}

	.server-card {
		background: var(--bg);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.server-card-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.server-name {
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text);
	}

	.server-host {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.server-card .status-pill {
		align-self: flex-start;
	}

	/* Responsive */
	@media (max-width: 900px) {
		.stats-grid {
			grid-template-columns: repeat(2, 1fr);
		}
		.content-grid {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 500px) {
		.stats-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
