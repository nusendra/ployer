<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';

	interface Server {
		id: string;
		name: string;
		host: string;
		port: number;
		username: string;
		is_local: boolean;
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

	let servers: Server[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let showAddForm = $state(false);
	let editingServerId = $state<string | null>(null);
	let selectedServerId = $state<string | null>(null);
	let serverStats = $state<ServerStats | null>(null);
	let testingServer = $state<string | null>(null);

	// Form fields
	let formName = $state('');
	let formHost = $state('');
	let formPort = $state(22);
	let formUsername = $state('root');
	let formSshKey = $state('');
	let formIsLocal = $state(false);
	let formSubmitting = $state(false);

	onMount(() => {
		loadServers();
	});

	async function loadServers() {
		try {
			loading = true;
			error = '';
			const res = await api.get<{ servers: Server[] }>('/servers');
			servers = res.servers;
		} catch (e: any) {
			error = e.message || 'Failed to load servers';
		} finally {
			loading = false;
		}
	}

	async function handleAddServer(e: Event) {
		e.preventDefault();
		if (!formName.trim() || !formHost.trim()) {
			error = 'Name and host are required';
			return;
		}

		try {
			formSubmitting = true;
			error = '';

			if (editingServerId) {
				// Update existing server
				await api.put(`/servers/${editingServerId}`, {
					name: formName,
					host: formHost,
					port: formPort,
					username: formUsername,
					ssh_key: formSshKey || undefined,
					is_local: formIsLocal
				});
			} else {
				// Create new server
				await api.post('/servers', {
					name: formName,
					host: formHost,
					port: formPort,
					username: formUsername,
					ssh_key: formSshKey || undefined,
					is_local: formIsLocal
				});
			}

			// Reset form
			resetForm();

			// Reload servers
			await loadServers();
		} catch (e: any) {
			error = e.message || (editingServerId ? 'Failed to update server' : 'Failed to create server');
		} finally {
			formSubmitting = false;
		}
	}

	function resetForm() {
		formName = '';
		formHost = '';
		formPort = 22;
		formUsername = 'root';
		formSshKey = '';
		formIsLocal = false;
		showAddForm = false;
		editingServerId = null;
	}

	function editServer(server: Server) {
		formName = server.name;
		formHost = server.host;
		formPort = server.port;
		formUsername = server.username;
		formSshKey = '';
		formIsLocal = server.is_local;
		editingServerId = server.id;
		showAddForm = true;
		// Scroll to form
		window.scrollTo({ top: 0, behavior: 'smooth' });
	}

	async function deleteServer(id: string, name: string) {
		if (!confirm(`Delete server "${name}"?`)) return;

		try {
			await api.delete(`/servers/${id}`);
			await loadServers();
		} catch (e: any) {
			error = e.message || 'Failed to delete server';
		}
	}

	async function testConnection(id: string) {
		try {
			testingServer = id;
			error = '';
			const res = await api.post<{ reachable: boolean; status: string }>(`/servers/${id}/validate`, {});
			alert(`Server ${res.reachable ? 'reachable' : 'unreachable'} (${res.status})`);
			await loadServers();
		} catch (e: any) {
			error = e.message || 'Failed to test connection';
		} finally {
			testingServer = null;
		}
	}

	async function viewResources(id: string) {
		try {
			error = '';
			const res = await api.get<{ stats: ServerStats }>(`/servers/${id}/resources`);
			serverStats = res.stats;
			selectedServerId = id;
		} catch (e: any) {
			error = e.message || 'Failed to load resources';
		}
	}

	function closeStats() {
		selectedServerId = null;
		serverStats = null;
	}

	function formatDate(dateStr: string | null): string {
		if (!dateStr) return 'Never';
		return new Date(dateStr).toLocaleString();
	}
</script>

<div class="servers-page">
	<div class="header">
		<h2>Servers</h2>
		<button class="btn-primary" onclick={() => {
			if (showAddForm) {
				resetForm();
			} else {
				showAddForm = true;
			}
		}}>
			{showAddForm ? 'Cancel' : 'Add Server'}
		</button>
	</div>

	{#if error}
		<div class="error">{error}</div>
	{/if}

	{#if showAddForm}
		<div class="card add-form">
			<h3>{editingServerId ? 'Edit Server' : 'Add New Server'}</h3>
			<form onsubmit={handleAddServer}>
				<div class="form-group">
					<label for="name">Name *</label>
					<input id="name" type="text" bind:value={formName} placeholder="My Server" required />
				</div>

				<div class="form-group">
					<label for="host">Host *</label>
					<input id="host" type="text" bind:value={formHost} placeholder="192.168.1.100" required />
				</div>

				<div class="form-row">
					<div class="form-group">
						<label for="port">Port</label>
						<input id="port" type="number" bind:value={formPort} />
					</div>

					<div class="form-group">
						<label for="username">Username</label>
						<input id="username" type="text" bind:value={formUsername} />
					</div>
				</div>

				<div class="form-group">
					<label for="ssh_key">SSH Key (optional)</label>
					<textarea id="ssh_key" bind:value={formSshKey} placeholder="-----BEGIN PRIVATE KEY-----" rows="4"></textarea>
				</div>

				<div class="form-group checkbox-group">
					<label>
						<input type="checkbox" bind:checked={formIsLocal} />
						<span>Mark as local server (enables resource monitoring)</span>
					</label>
				</div>

				<button type="submit" class="btn-primary" disabled={formSubmitting}>
					{#if formSubmitting}
						{editingServerId ? 'Updating...' : 'Creating...'}
					{:else}
						{editingServerId ? 'Update Server' : 'Create Server'}
					{/if}
				</button>
			</form>
		</div>
	{/if}

	{#if loading}
		<p class="text-muted">Loading servers...</p>
	{:else if servers.length === 0}
		<p class="text-muted">No servers configured. Add a server to start deploying.</p>
	{:else}
		<div class="servers-grid">
			{#each servers as server (server.id)}
				<div class="card server-card">
					<div class="server-header">
						<div>
							<h3>{server.name}</h3>
							{#if server.is_local}
								<span class="badge badge-local">Local</span>
							{/if}
						</div>
						<span class="status status-{server.status}">{server.status}</span>
					</div>

					<div class="server-details">
						<p><strong>Host:</strong> {server.host}:{server.port}</p>
						<p><strong>Username:</strong> {server.username}</p>
						<p><strong>Last seen:</strong> {formatDate(server.last_seen_at)}</p>
					</div>

					<div class="server-actions">
						<button class="btn-secondary" onclick={() => editServer(server)}>
							Edit
						</button>
						<button class="btn-secondary" onclick={() => testConnection(server.id)} disabled={testingServer === server.id}>
							{testingServer === server.id ? 'Testing...' : 'Test Connection'}
						</button>
						{#if server.is_local}
							<button class="btn-secondary" onclick={() => viewResources(server.id)}>
								View Resources
							</button>
						{/if}
						<button class="btn-danger" onclick={() => deleteServer(server.id, server.name)}>
							Delete
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}

	{#if selectedServerId && serverStats}
		<div class="modal-overlay" onclick={closeStats}>
			<div class="modal-content" onclick={(e) => e.stopPropagation()}>
				<div class="modal-header">
					<h3>Server Resources</h3>
					<button class="btn-close" onclick={closeStats}>×</button>
				</div>
				<div class="stats-grid">
					<div class="stat-card">
						<div class="stat-label">CPU Cores</div>
						<div class="stat-value">{serverStats.cpu_count}</div>
					</div>
					<div class="stat-card">
						<div class="stat-label">CPU Usage</div>
						<div class="stat-value">{serverStats.cpu_usage.toFixed(1)}%</div>
					</div>
					<div class="stat-card">
						<div class="stat-label">Memory Used</div>
						<div class="stat-value">{serverStats.used_memory_mb} MB</div>
					</div>
					<div class="stat-card">
						<div class="stat-label">Memory Total</div>
						<div class="stat-value">{serverStats.total_memory_mb} MB</div>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>

<style>
	.servers-page {
		max-width: 1200px;
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

	.add-form {
		margin-bottom: 2rem;
	}

	.add-form h3 {
		margin-bottom: 1rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-row {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1rem;
	}

	label {
		display: block;
		margin-bottom: 0.25rem;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	textarea {
		font-family: monospace;
		resize: vertical;
	}

	.checkbox-group {
		margin-bottom: 1.5rem;
	}

	.checkbox-group label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.875rem;
	}

	.checkbox-group input[type="checkbox"] {
		width: auto;
		cursor: pointer;
	}

	.checkbox-group span {
		color: var(--text);
	}

	.servers-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(350px, 1fr));
		gap: 1rem;
	}

	.server-card {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.server-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
	}

	.server-header h3 {
		margin-bottom: 0.25rem;
	}

	.badge {
		display: inline-block;
		padding: 0.25rem 0.5rem;
		font-size: 0.75rem;
		border-radius: var(--radius);
		background: var(--bg-tertiary);
		color: var(--text-muted);
	}

	.badge-local {
		background: var(--primary);
		color: white;
	}

	.status {
		font-weight: 600;
		text-transform: uppercase;
		font-size: 0.75rem;
	}

	.server-details p {
		margin: 0.5rem 0;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	.server-details strong {
		color: var(--text);
	}

	.server-actions {
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
