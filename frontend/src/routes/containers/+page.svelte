<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';

	interface Container {
		id: string;
		name: string;
		image: string;
		state: string;
		status: string;
		created: number;
		ports: PortInfo[];
	}

	interface PortInfo {
		container_port: number;
		host_port: number | null;
		protocol: string;
	}

	interface ContainerStats {
		cpu_usage: number;
		memory_usage_mb: number;
		memory_limit_mb: number;
		network_rx_bytes: number;
		network_tx_bytes: number;
	}

	interface EnvVar {
		key: string;
		value: string;
	}

	interface PortMapping {
		containerPort: string;
		hostPort: string;
	}

	interface VolumeMount {
		hostPath: string;
		containerPath: string;
	}

	let containers: Container[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let showCreateForm = $state(false);

	// Form state
	let formImage = $state('');
	let formName = $state('');
	let formEnvVars: EnvVar[] = $state([]);
	let formPorts: PortMapping[] = $state([]);
	let formVolumes: VolumeMount[] = $state([]);
	let formCmd = $state('');
	let formSubmitting = $state(false);

	// Action states
	let actioningContainer = $state<string | null>(null);

	// Modal states
	let viewingLogsId = $state<string | null>(null);
	let viewingStatsId = $state<string | null>(null);
	let containerLogs: string[] = $state([]);
	let containerStats = $state<ContainerStats | null>(null);
	let logsLoading = $state(false);
	let statsLoading = $state(false);

	onMount(() => {
		loadContainers();
	});

	async function loadContainers() {
		try {
			loading = true;
			error = '';
			const res = await api.get<{ containers: Container[] }>('/containers?all=true');
			containers = res.containers;
		} catch (e: any) {
			error = e.message || 'Failed to load containers';
		} finally {
			loading = false;
		}
	}

	function addEnvVar() {
		formEnvVars = [...formEnvVars, { key: '', value: '' }];
	}

	function removeEnvVar(index: number) {
		formEnvVars = formEnvVars.filter((_, i) => i !== index);
	}

	function addPort() {
		formPorts = [...formPorts, { containerPort: '', hostPort: '' }];
	}

	function removePort(index: number) {
		formPorts = formPorts.filter((_, i) => i !== index);
	}

	function addVolume() {
		formVolumes = [...formVolumes, { hostPath: '', containerPath: '' }];
	}

	function removeVolume(index: number) {
		formVolumes = formVolumes.filter((_, i) => i !== index);
	}

	function resetForm() {
		formImage = '';
		formName = '';
		formEnvVars = [];
		formPorts = [];
		formVolumes = [];
		formCmd = '';
		showCreateForm = false;
	}

	async function handleCreateContainer(e: Event) {
		e.preventDefault();
		if (!formImage.trim()) {
			error = 'Image name is required';
			return;
		}

		try {
			formSubmitting = true;
			error = '';

			// Build env array
			const env = formEnvVars
				.filter(e => e.key.trim() && e.value.trim())
				.map(e => `${e.key}=${e.value}`);

			// Build ports object
			const ports: Record<string, string> = {};
			formPorts
				.filter(p => p.containerPort && p.hostPort)
				.forEach(p => {
					ports[`${p.containerPort}/tcp`] = p.hostPort;
				});

			// Build volumes object
			const volumes: Record<string, string> = {};
			formVolumes
				.filter(v => v.hostPath && v.containerPath)
				.forEach(v => {
					volumes[v.hostPath] = v.containerPath;
				});

			// Build cmd array
			const cmd = formCmd.trim() ? formCmd.trim().split(' ') : undefined;

			await api.post('/containers', {
				image: formImage,
				name: formName || undefined,
				env: env.length > 0 ? env : undefined,
				ports: Object.keys(ports).length > 0 ? ports : undefined,
				volumes: Object.keys(volumes).length > 0 ? volumes : undefined,
				cmd
			});

			resetForm();
			await loadContainers();
		} catch (e: any) {
			error = e.message || 'Failed to create container';
		} finally {
			formSubmitting = false;
		}
	}

	async function startContainer(id: string) {
		try {
			actioningContainer = id;
			error = '';
			await api.post(`/containers/${id}/start`, {});
			await loadContainers();
		} catch (e: any) {
			error = e.message || 'Failed to start container';
		} finally {
			actioningContainer = null;
		}
	}

	async function stopContainer(id: string) {
		try {
			actioningContainer = id;
			error = '';
			await api.post(`/containers/${id}/stop`, {});
			await loadContainers();
		} catch (e: any) {
			error = e.message || 'Failed to stop container';
		} finally {
			actioningContainer = null;
		}
	}

	async function restartContainer(id: string) {
		try {
			actioningContainer = id;
			error = '';
			await api.post(`/containers/${id}/restart`, {});
			await loadContainers();
		} catch (e: any) {
			error = e.message || 'Failed to restart container';
		} finally {
			actioningContainer = null;
		}
	}

	async function deleteContainer(id: string, name: string) {
		if (!confirm(`Delete container "${name}"?`)) return;

		try {
			actioningContainer = id;
			error = '';
			await api.delete(`/containers/${id}`);
			await loadContainers();
		} catch (e: any) {
			error = e.message || 'Failed to delete container';
		} finally {
			actioningContainer = null;
		}
	}

	async function viewLogs(id: string) {
		try {
			logsLoading = true;
			viewingLogsId = id;
			const res = await api.get<{ logs: string[] }>(`/containers/${id}/logs?tail=100`);
			containerLogs = res.logs;
		} catch (e: any) {
			error = e.message || 'Failed to load logs';
			viewingLogsId = null;
		} finally {
			logsLoading = false;
		}
	}

	async function viewStats(id: string) {
		try {
			statsLoading = true;
			viewingStatsId = id;
			const res = await api.get<{ stats: ContainerStats }>(`/containers/${id}/stats`);
			containerStats = res.stats;
		} catch (e: any) {
			error = e.message || 'Failed to load stats';
			viewingStatsId = null;
		} finally {
			statsLoading = false;
		}
	}

	function closeLogs() {
		viewingLogsId = null;
		containerLogs = [];
	}

	function closeStats() {
		viewingStatsId = null;
		containerStats = null;
	}

	function formatPorts(ports: PortInfo[]): string {
		if (!ports || ports.length === 0) return '-';
		return ports
			.map(p => p.host_port ? `${p.host_port}→${p.container_port}` : `${p.container_port}`)
			.join(', ');
	}

	function formatDate(timestamp: number): string {
		if (!timestamp) return 'Unknown';
		return new Date(timestamp * 1000).toLocaleString();
	}

	function getStatusColor(state: string): string {
		const s = state.toLowerCase();
		if (s === 'running') return 'status-running';
		if (s === 'exited' || s === 'dead') return 'status-stopped';
		return 'status-unknown';
	}
</script>

<div class="containers-page">
	<div class="header">
		<h2>Containers</h2>
		<button class="btn-primary" onclick={() => {
			if (showCreateForm) {
				resetForm();
			} else {
				showCreateForm = true;
			}
		}}>
			{showCreateForm ? 'Cancel' : 'Create Container'}
		</button>
	</div>

	{#if error}
		<div class="error">{error}</div>
	{/if}

	{#if showCreateForm}
		<div class="card create-form">
			<h3>Create Container</h3>
			<form onsubmit={handleCreateContainer}>
				<div class="form-group">
					<label for="image">Image *</label>
					<input id="image" type="text" bind:value={formImage} placeholder="nginx:latest" required />
				</div>

				<div class="form-group">
					<label for="name">Container Name (optional)</label>
					<input id="name" type="text" bind:value={formName} placeholder="my-container" />
				</div>

				<div class="form-section">
					<div class="section-header">
						<label>Environment Variables</label>
						<button type="button" class="btn-small" onclick={addEnvVar}>+ Add</button>
					</div>
					{#each formEnvVars as envVar, i}
						<div class="form-row">
							<input type="text" bind:value={envVar.key} placeholder="KEY" />
							<input type="text" bind:value={envVar.value} placeholder="value" />
							<button type="button" class="btn-danger-small" onclick={() => removeEnvVar(i)}>×</button>
						</div>
					{/each}
				</div>

				<div class="form-section">
					<div class="section-header">
						<label>Port Mappings</label>
						<button type="button" class="btn-small" onclick={addPort}>+ Add</button>
					</div>
					{#each formPorts as port, i}
						<div class="form-row">
							<input type="text" bind:value={port.hostPort} placeholder="Host Port (e.g., 8080)" />
							<span class="arrow">→</span>
							<input type="text" bind:value={port.containerPort} placeholder="Container Port (e.g., 80)" />
							<button type="button" class="btn-danger-small" onclick={() => removePort(i)}>×</button>
						</div>
					{/each}
				</div>

				<div class="form-section">
					<div class="section-header">
						<label>Volume Mounts</label>
						<button type="button" class="btn-small" onclick={addVolume}>+ Add</button>
					</div>
					{#each formVolumes as volume, i}
						<div class="form-row">
							<input type="text" bind:value={volume.hostPath} placeholder="Host Path (e.g., /data)" />
							<span class="arrow">→</span>
							<input type="text" bind:value={volume.containerPath} placeholder="Container Path (e.g., /app/data)" />
							<button type="button" class="btn-danger-small" onclick={() => removeVolume(i)}>×</button>
						</div>
					{/each}
				</div>

				<div class="form-group">
					<label for="cmd">Command (optional)</label>
					<input id="cmd" type="text" bind:value={formCmd} placeholder="npm start" />
					<small class="hint">Space-separated command arguments</small>
				</div>

				<button type="submit" class="btn-primary" disabled={formSubmitting}>
					{formSubmitting ? 'Creating...' : 'Create Container'}
				</button>
			</form>
		</div>
	{/if}

	{#if loading}
		<p class="text-muted">Loading containers...</p>
	{:else if containers.length === 0}
		<p class="text-muted">No containers found. Create one to get started.</p>
	{:else}
		<div class="containers-grid">
			{#each containers as container (container.id)}
				<div class="card container-card">
					<div class="container-header">
						<div>
							<h3>{container.name || container.id.substring(0, 12)}</h3>
							<span class="status {getStatusColor(container.state)}">{container.state}</span>
						</div>
					</div>

					<div class="container-details">
						<p><strong>Image:</strong> {container.image}</p>
						<p><strong>Ports:</strong> {formatPorts(container.ports)}</p>
						<p><strong>Status:</strong> {container.status}</p>
						<p><strong>Created:</strong> {formatDate(container.created)}</p>
					</div>

					<div class="container-actions">
						{#if container.state.toLowerCase() === 'running'}
							<button
								class="btn-secondary"
								onclick={() => stopContainer(container.id)}
								disabled={actioningContainer === container.id}
							>
								{actioningContainer === container.id ? 'Stopping...' : 'Stop'}
							</button>
							<button
								class="btn-secondary"
								onclick={() => restartContainer(container.id)}
								disabled={actioningContainer === container.id}
							>
								{actioningContainer === container.id ? 'Restarting...' : 'Restart'}
							</button>
						{:else}
							<button
								class="btn-secondary"
								onclick={() => startContainer(container.id)}
								disabled={actioningContainer === container.id}
							>
								{actioningContainer === container.id ? 'Starting...' : 'Start'}
							</button>
						{/if}

						<button class="btn-secondary" onclick={() => viewLogs(container.id)}>
							View Logs
						</button>
						<button class="btn-secondary" onclick={() => viewStats(container.id)}>
							View Stats
						</button>
						<button
							class="btn-danger"
							onclick={() => deleteContainer(container.id, container.name || container.id)}
							disabled={actioningContainer === container.id}
						>
							Delete
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<!-- Logs Modal -->
	{#if viewingLogsId}
		<div class="modal-overlay" onclick={closeLogs}>
			<div class="modal-content modal-large" onclick={(e) => e.stopPropagation()}>
				<div class="modal-header">
					<h3>Container Logs</h3>
					<button class="btn-close" onclick={closeLogs}>×</button>
				</div>
				{#if logsLoading}
					<p class="text-muted">Loading logs...</p>
				{:else}
					<div class="logs-container">
						{#if containerLogs.length === 0}
							<p class="text-muted">No logs available</p>
						{:else}
							{#each containerLogs as log}
								<div class="log-line">{log}</div>
							{/each}
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Stats Modal -->
	{#if viewingStatsId && containerStats}
		<div class="modal-overlay" onclick={closeStats}>
			<div class="modal-content" onclick={(e) => e.stopPropagation()}>
				<div class="modal-header">
					<h3>Container Stats</h3>
					<button class="btn-close" onclick={closeStats}>×</button>
				</div>
				{#if statsLoading}
					<p class="text-muted">Loading stats...</p>
				{:else}
					<div class="stats-grid">
						<div class="stat-card">
							<div class="stat-label">CPU Usage</div>
							<div class="stat-value">{containerStats.cpu_usage.toFixed(1)}%</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Memory Used</div>
							<div class="stat-value">{containerStats.memory_usage_mb.toFixed(0)} MB</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Memory Limit</div>
							<div class="stat-value">{containerStats.memory_limit_mb.toFixed(0)} MB</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Network RX</div>
							<div class="stat-value">{(containerStats.network_rx_bytes / 1024 / 1024).toFixed(2)} MB</div>
						</div>
						<div class="stat-card">
							<div class="stat-label">Network TX</div>
							<div class="stat-value">{(containerStats.network_tx_bytes / 1024 / 1024).toFixed(2)} MB</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<style>
	.containers-page {
		max-width: 1400px;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.error {
		padding: 0.75rem;
		margin-bottom: 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--danger);
		border-radius: var(--radius);
		color: var(--danger);
	}

	.text-muted {
		color: var(--text-muted);
	}

	.create-form {
		margin-bottom: 2rem;
	}

	.create-form h3 {
		margin-bottom: 1rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-section {
		margin-bottom: 1.5rem;
		padding: 1rem;
		background: var(--bg-tertiary);
		border-radius: var(--radius);
	}

	.section-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.75rem;
	}

	.section-header label {
		margin-bottom: 0;
		font-weight: 600;
	}

	.form-row {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.5rem;
		align-items: center;
	}

	.form-row input {
		flex: 1;
	}

	.form-row .arrow {
		color: var(--text-muted);
		font-weight: bold;
	}

	.btn-small {
		padding: 0.25rem 0.75rem;
		font-size: 0.875rem;
		background: var(--primary);
		color: white;
		border: none;
		border-radius: var(--radius);
		cursor: pointer;
	}

	.btn-small:hover {
		opacity: 0.9;
	}

	.btn-danger-small {
		padding: 0.25rem 0.5rem;
		font-size: 1rem;
		background: var(--danger);
		color: white;
		border: none;
		border-radius: var(--radius);
		cursor: pointer;
		line-height: 1;
	}

	.btn-danger-small:hover {
		opacity: 0.9;
	}

	label {
		display: block;
		margin-bottom: 0.25rem;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	.hint {
		display: block;
		margin-top: 0.25rem;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.containers-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(400px, 1fr));
		gap: 1rem;
	}

	.container-card {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.container-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.container-header h3 {
		margin-bottom: 0.5rem;
		font-size: 1rem;
		word-break: break-all;
	}

	.status {
		padding: 0.25rem 0.75rem;
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		border-radius: var(--radius);
	}

	.status-running {
		background: rgba(34, 197, 94, 0.2);
		color: #22c55e;
	}

	.status-stopped {
		background: rgba(239, 68, 68, 0.2);
		color: var(--danger);
	}

	.status-unknown {
		background: var(--bg-tertiary);
		color: var(--text-muted);
	}

	.container-details p {
		margin: 0.5rem 0;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	.container-details strong {
		color: var(--text);
	}

	.container-actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.btn-secondary {
		background: var(--bg-tertiary);
		color: var(--text);
		border: 1px solid var(--border);
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
	}

	.btn-secondary:hover {
		background: var(--border);
	}

	.btn-secondary:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.modal-overlay {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.8);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
	}

	.modal-content {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 1.5rem;
		min-width: 500px;
		max-width: 90vw;
		max-height: 90vh;
		overflow-y: auto;
	}

	.modal-large {
		min-width: 800px;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.5rem;
	}

	.btn-close {
		background: transparent;
		border: none;
		font-size: 2rem;
		line-height: 1;
		padding: 0;
		width: 2rem;
		height: 2rem;
		color: var(--text-muted);
	}

	.btn-close:hover {
		color: var(--text);
	}

	.logs-container {
		background: #1a1a1a;
		border-radius: var(--radius);
		padding: 1rem;
		max-height: 500px;
		overflow-y: auto;
		font-family: 'Monaco', 'Courier New', monospace;
		font-size: 0.875rem;
	}

	.log-line {
		color: #e0e0e0;
		margin: 0.25rem 0;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	.stat-card {
		background: var(--bg-tertiary);
		padding: 1rem;
		border-radius: var(--radius);
		text-align: center;
	}

	.stat-label {
		font-size: 0.875rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
	}

	.stat-value {
		font-size: 1.5rem;
		font-weight: 700;
		color: var(--text);
	}
</style>
