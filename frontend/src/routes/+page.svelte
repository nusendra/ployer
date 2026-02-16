<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';

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
			case 'running':
				return 'green';
			case 'failed':
				return 'red';
			case 'building':
			case 'deploying':
				return 'blue';
			default:
				return 'gray';
		}
	}
</script>

<div class="dashboard">
	<h2>Dashboard</h2>

	{#if loading}
		<div class="loading">Loading...</div>
	{:else}
		<div class="stats-grid">
			<div class="stat-card">
				<h3>Applications</h3>
				<p class="stat-number">{applications.length}</p>
				<p class="stat-detail">{getHealthyAppsCount()} running</p>
			</div>

			<div class="stat-card">
				<h3>Servers</h3>
				<p class="stat-number">{servers.length}</p>
				<p class="stat-detail">{getOnlineServersCount()} online</p>
			</div>

			<div class="stat-card">
				<h3>Deployments (24h)</h3>
				<p class="stat-number">{getRecentDeploymentsCount()}</p>
				<p class="stat-detail">Last 24 hours</p>
			</div>

			<div class="stat-card">
				<h3>System Status</h3>
				<p class="stat-number status-healthy">Healthy</p>
				<p class="stat-detail">All systems operational</p>
			</div>
		</div>

		<div class="dashboard-sections">
			<!-- Applications Overview -->
			<div class="section">
				<h3>Applications Status</h3>
				{#if applications.length === 0}
					<p class="empty">No applications yet</p>
				{:else}
					<div class="app-list">
						{#each applications as app (app.id)}
							<div class="app-item">
								<div class="app-info">
									<span class="app-name">{app.name}</span>
									<span class="app-branch">{app.git_branch || 'N/A'}</span>
								</div>
								<span class="status status-{app.status}">{app.status}</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Recent Deployments -->
			<div class="section">
				<h3>Recent Deployments</h3>
				{#if deployments.length === 0}
					<p class="empty">No deployments yet</p>
				{:else}
					<div class="deployments-list">
						{#each deployments as deployment (deployment.id)}
							<div class="deployment-item">
								<div class="deployment-info">
									<span class="deployment-app">{deployment.application_id}</span>
									<span class="deployment-time">{new Date(deployment.created_at).toLocaleString()}</span>
								</div>
								<span class="status status-{getDeploymentStatusColor(deployment.status)}">
									{deployment.status}
								</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Server Status -->
			<div class="section">
				<h3>Server Status</h3>
				{#if servers.length === 0}
					<p class="empty">No servers configured</p>
				{:else}
					<div class="server-list">
						{#each servers as server (server.id)}
							<div class="server-item">
								<div class="server-info">
									<span class="server-name">{server.name}</span>
									<span class="server-host">{server.host}</span>
								</div>
								<span class="status status-{server.status === 'online' ? 'green' : 'red'}">
									{server.status}
								</span>
							</div>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.dashboard {
		padding: 2rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	h2 {
		margin-bottom: 1.5rem;
		color: #1f2937;
	}

	.loading {
		text-align: center;
		padding: 2rem;
		color: #6b7280;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
		margin-bottom: 2rem;
	}

	.stat-card {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		border: 1px solid #e5e7eb;
	}

	.stat-card h3 {
		color: #6b7280;
		font-size: 0.875rem;
		font-weight: 500;
		margin: 0 0 0.5rem 0;
	}

	.stat-number {
		font-size: 2rem;
		font-weight: 700;
		color: #1f2937;
		margin: 0;
	}

	.stat-number.status-healthy {
		color: #10b981;
	}

	.stat-detail {
		font-size: 0.875rem;
		color: #9ca3af;
		margin: 0.5rem 0 0 0;
	}

	.dashboard-sections {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
		gap: 1.5rem;
	}

	.section {
		background: white;
		padding: 1.5rem;
		border-radius: 8px;
		border: 1px solid #e5e7eb;
	}

	.section h3 {
		margin: 0 0 1rem 0;
		color: #1f2937;
		font-size: 1rem;
		font-weight: 600;
	}

	.empty {
		color: #9ca3af;
		text-align: center;
		padding: 2rem;
		margin: 0;
	}

	.app-list,
	.deployments-list,
	.server-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.app-item,
	.deployment-item,
	.server-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background: #f9fafb;
		border-radius: 6px;
	}

	.app-info,
	.deployment-info,
	.server-info {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.app-name,
	.deployment-app,
	.server-name {
		font-weight: 600;
		color: #1f2937;
		font-size: 0.875rem;
	}

	.app-branch,
	.deployment-time,
	.server-host {
		font-size: 0.75rem;
		color: #6b7280;
	}

	.status {
		padding: 0.25rem 0.75rem;
		border-radius: 12px;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
	}

	.status-green {
		background: #d1fae5;
		color: #065f46;
	}

	.status-red {
		background: #fee2e2;
		color: #991b1b;
	}

	.status-blue {
		background: #dbeafe;
		color: #1e40af;
	}

	.status-gray {
		background: #f3f4f6;
		color: #4b5563;
	}

	.status-running {
		background: #d1fae5;
		color: #065f46;
	}

	.status-stopped {
		background: #fee2e2;
		color: #991b1b;
	}

	.status-pending {
		background: #fef3c7;
		color: #92400e;
	}
</style>
