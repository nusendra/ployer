<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import type { Application, BuildStrategy } from '$lib/types';

	let applications = $state<Application[]>([]);
	let servers = $state<any[]>([]);
	let loading = $state(true);
	let error = $state('');

	// Create modal state
	let showCreateModal = $state(false);
	let createForm = $state({
		name: '',
		server_id: '',
		git_url: '',
		git_branch: 'main',
		build_strategy: 'dockerfile' as BuildStrategy,
		dockerfile_path: '',
		port: undefined as number | undefined,
		auto_deploy: false,
		env_vars: {} as Record<string, string>
	});
	let envVarKey = $state('');
	let envVarValue = $state('');

	// Detail/Edit modal state
	let showDetailModal = $state(false);
	let selectedApp = $state<Application | null>(null);
	let editForm = $state({
		name: '',
		git_url: '',
		git_branch: '',
		build_strategy: 'dockerfile' as BuildStrategy,
		dockerfile_path: '',
		port: undefined as number | undefined,
		auto_deploy: false
	});

	// Environment variables modal state
	let showEnvModal = $state(false);
	let appEnvVars = $state<Array<{ key: string; value: string }>>([]);
	let newEnvKey = $state('');
	let newEnvValue = $state('');

	// Deploy key modal state
	let showDeployKeyModal = $state(false);
	let deployKey = $state<{ public_key: string; created_at: string } | null>(null);

	onMount(async () => {
		await loadApplications();
		await loadServers();
	});

	async function loadApplications() {
		loading = true;
		error = '';
		try {
			const response = await api.get<{ applications: Application[] }>('/api/v1/applications');
			applications = response.applications;
		} catch (e: any) {
			error = e.message || 'Failed to load applications';
		} finally {
			loading = false;
		}
	}

	async function loadServers() {
		try {
			const response = await api.get<{ servers: any[] }>('/api/v1/servers');
			servers = response.servers;
		} catch (e: any) {
			console.error('Failed to load servers:', e);
		}
	}

	async function createApplication() {
		error = '';
		try {
			await api.post('/api/v1/applications', createForm);
			showCreateModal = false;
			resetCreateForm();
			await loadApplications();
		} catch (e: any) {
			error = e.message || 'Failed to create application';
		}
	}

	function resetCreateForm() {
		createForm = {
			name: '',
			server_id: '',
			git_url: '',
			git_branch: 'main',
			build_strategy: 'dockerfile',
			dockerfile_path: '',
			port: undefined,
			auto_deploy: false,
			env_vars: {}
		};
		envVarKey = '';
		envVarValue = '';
	}

	function addEnvVarToCreate() {
		if (envVarKey && envVarValue) {
			createForm.env_vars[envVarKey] = envVarValue;
			envVarKey = '';
			envVarValue = '';
		}
	}

	function removeEnvVarFromCreate(key: string) {
		delete createForm.env_vars[key];
		createForm.env_vars = { ...createForm.env_vars };
	}

	async function openDetailModal(app: Application) {
		selectedApp = app;
		editForm = {
			name: app.name,
			git_url: app.git_url || '',
			git_branch: app.git_branch,
			build_strategy: app.build_strategy,
			dockerfile_path: app.dockerfile_path || '',
			port: app.port,
			auto_deploy: app.auto_deploy
		};
		showDetailModal = true;
	}

	async function updateApplication() {
		if (!selectedApp) return;
		error = '';
		try {
			const payload: any = {};
			if (editForm.name !== selectedApp.name) payload.name = editForm.name;
			if (editForm.git_url !== (selectedApp.git_url || '')) payload.git_url = editForm.git_url || null;
			if (editForm.git_branch !== selectedApp.git_branch) payload.git_branch = editForm.git_branch;
			if (editForm.build_strategy !== selectedApp.build_strategy)
				payload.build_strategy = editForm.build_strategy;
			if (editForm.dockerfile_path !== (selectedApp.dockerfile_path || ''))
				payload.dockerfile_path = editForm.dockerfile_path || null;
			if (editForm.port !== selectedApp.port) payload.port = editForm.port;
			if (editForm.auto_deploy !== selectedApp.auto_deploy) payload.auto_deploy = editForm.auto_deploy;

			await api.put(`/api/v1/applications/${selectedApp.id}`, payload);
			showDetailModal = false;
			await loadApplications();
		} catch (e: any) {
			error = e.message || 'Failed to update application';
		}
	}

	async function deleteApplication(id: string) {
		if (!confirm('Are you sure you want to delete this application?')) return;
		error = '';
		try {
			await api.delete(`/api/v1/applications/${id}`);
			await loadApplications();
		} catch (e: any) {
			error = e.message || 'Failed to delete application';
		}
	}

	async function openEnvModal(app: Application) {
		selectedApp = app;
		showEnvModal = true;
		await loadEnvVars(app.id);
	}

	async function loadEnvVars(appId: string) {
		try {
			const response = await api.get<{ env_vars: Array<{ key: string; value: string }> }>(
				`/api/v1/applications/${appId}/envs`
			);
			appEnvVars = response.env_vars;
		} catch (e: any) {
			error = e.message || 'Failed to load environment variables';
		}
	}

	async function addEnvVar() {
		if (!selectedApp || !newEnvKey) return;
		error = '';
		try {
			await api.post(`/api/v1/applications/${selectedApp.id}/envs`, {
				key: newEnvKey,
				value: newEnvValue
			});
			newEnvKey = '';
			newEnvValue = '';
			await loadEnvVars(selectedApp.id);
		} catch (e: any) {
			error = e.message || 'Failed to add environment variable';
		}
	}

	async function deleteEnvVar(key: string) {
		if (!selectedApp) return;
		if (!confirm(`Delete environment variable ${key}?`)) return;
		error = '';
		try {
			await api.delete(`/api/v1/applications/${selectedApp.id}/envs/${key}`);
			await loadEnvVars(selectedApp.id);
		} catch (e: any) {
			error = e.message || 'Failed to delete environment variable';
		}
	}

	async function openDeployKeyModal(app: Application) {
		selectedApp = app;
		showDeployKeyModal = true;
		await loadDeployKey(app.id);
	}

	async function loadDeployKey(appId: string) {
		try {
			const response = await api.get<{ public_key: string; created_at: string }>(
				`/api/v1/applications/${appId}/deploy-key`
			);
			deployKey = response;
		} catch (e: any) {
			deployKey = null;
		}
	}

	async function regenerateDeployKey() {
		if (!selectedApp) return;
		if (!confirm('Regenerate deploy key? The old key will be deleted.')) return;
		error = '';
		try {
			const response = await api.post<{ public_key: string; created_at: string }>(
				`/api/v1/applications/${selectedApp.id}/deploy-key`,
				{}
			);
			deployKey = response;
		} catch (e: any) {
			error = e.message || 'Failed to regenerate deploy key';
		}
	}

	function getStatusColor(status: string) {
		switch (status) {
			case 'running':
				return 'green';
			case 'deploying':
				return 'blue';
			case 'stopped':
				return 'gray';
			case 'failed':
				return 'red';
			default:
				return 'gray';
		}
	}

	function getStrategyLabel(strategy: BuildStrategy) {
		switch (strategy) {
			case 'dockerfile':
				return 'Dockerfile';
			case 'nixpacks':
				return 'Nixpacks';
			case 'docker_compose':
				return 'Docker Compose';
			default:
				return strategy;
		}
	}
</script>

<div class="applications-page">
	<div class="header">
		<div>
			<h1>Applications</h1>
			<p>Manage your deployed applications</p>
		</div>
		<button class="btn-primary" onclick={() => (showCreateModal = true)}>+ Create Application</button>
	</div>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	{#if loading}
		<div class="loading">Loading applications...</div>
	{:else if applications.length === 0}
		<div class="empty-state">
			<p>No applications yet</p>
			<button class="btn-primary" onclick={() => (showCreateModal = true)}>Create Your First Application</button>
		</div>
	{:else}
		<div class="applications-grid">
			{#each applications as app (app.id)}
				<div class="app-card">
					<div class="app-header">
						<div>
							<h3>{app.name}</h3>
							<span class="status status-{getStatusColor(app.status)}">{app.status}</span>
						</div>
						<div class="app-actions">
							<button class="btn-sm" onclick={() => openDetailModal(app)}>Edit</button>
							<button class="btn-sm" onclick={() => openEnvModal(app)}>Env Vars</button>
							{#if app.git_url}
								<button class="btn-sm" onclick={() => openDeployKeyModal(app)}>Deploy Key</button>
							{/if}
							<button class="btn-sm btn-danger" onclick={() => deleteApplication(app.id)}>Delete</button>
						</div>
					</div>
					<div class="app-details">
						<div class="detail-row">
							<span class="label">Server:</span>
							<span>{servers.find((s) => s.id === app.server_id)?.name || app.server_id}</span>
						</div>
						{#if app.git_url}
							<div class="detail-row">
								<span class="label">Git:</span>
								<span class="git-url">{app.git_url} ({app.git_branch})</span>
							</div>
						{/if}
						<div class="detail-row">
							<span class="label">Build:</span>
							<span>{getStrategyLabel(app.build_strategy)}</span>
						</div>
						{#if app.port}
							<div class="detail-row">
								<span class="label">Port:</span>
								<span>{app.port}</span>
							</div>
						{/if}
						<div class="detail-row">
							<span class="label">Auto Deploy:</span>
							<span>{app.auto_deploy ? 'Yes' : 'No'}</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Create Application Modal -->
{#if showCreateModal}
	<div class="modal-overlay" onclick={() => (showCreateModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Create Application</h2>
			<form
				onsubmit={(e) => {
					e.preventDefault();
					createApplication();
				}}
			>
				<div class="form-group">
					<label for="name">Application Name</label>
					<input id="name" type="text" bind:value={createForm.name} required />
				</div>

				<div class="form-group">
					<label for="server">Server</label>
					<select id="server" bind:value={createForm.server_id} required>
						<option value="">Select a server</option>
						{#each servers as server (server.id)}
							<option value={server.id}>{server.name}</option>
						{/each}
					</select>
				</div>

				<div class="form-group">
					<label for="git_url">Git Repository URL (optional)</label>
					<input id="git_url" type="text" bind:value={createForm.git_url} placeholder="git@github.com:user/repo.git" />
				</div>

				<div class="form-group">
					<label for="git_branch">Git Branch</label>
					<input id="git_branch" type="text" bind:value={createForm.git_branch} />
				</div>

				<div class="form-group">
					<label for="build_strategy">Build Strategy</label>
					<select id="build_strategy" bind:value={createForm.build_strategy}>
						<option value="dockerfile">Dockerfile</option>
						<option value="nixpacks">Nixpacks</option>
						<option value="docker_compose">Docker Compose</option>
					</select>
				</div>

				{#if createForm.build_strategy === 'dockerfile'}
					<div class="form-group">
						<label for="dockerfile_path">Dockerfile Path (optional)</label>
						<input id="dockerfile_path" type="text" bind:value={createForm.dockerfile_path} placeholder="./Dockerfile" />
					</div>
				{/if}

				<div class="form-group">
					<label for="port">Port (optional)</label>
					<input id="port" type="number" bind:value={createForm.port} placeholder="3000" />
				</div>

				<div class="form-group-checkbox">
					<input id="auto_deploy" type="checkbox" bind:checked={createForm.auto_deploy} />
					<label for="auto_deploy">Enable auto-deploy on git push</label>
				</div>

				<div class="form-group">
					<label>Environment Variables</label>
					<div class="env-vars-input">
						<input type="text" placeholder="KEY" bind:value={envVarKey} />
						<input type="text" placeholder="Value" bind:value={envVarValue} />
						<button type="button" class="btn-sm" onclick={addEnvVarToCreate}>Add</button>
					</div>
					<div class="env-vars-list">
						{#each Object.entries(createForm.env_vars) as [key, value] (key)}
							<div class="env-var-item">
								<span class="env-key">{key}</span>
								<span class="env-value">{value}</span>
								<button type="button" class="btn-sm btn-danger" onclick={() => removeEnvVarFromCreate(key)}>
									Remove
								</button>
							</div>
						{/each}
					</div>
				</div>

				<div class="modal-actions">
					<button type="button" class="btn-secondary" onclick={() => (showCreateModal = false)}>Cancel</button>
					<button type="submit" class="btn-primary">Create</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<!-- Edit Application Modal -->
{#if showDetailModal && selectedApp}
	<div class="modal-overlay" onclick={() => (showDetailModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Edit Application</h2>
			<form
				onsubmit={(e) => {
					e.preventDefault();
					updateApplication();
				}}
			>
				<div class="form-group">
					<label for="edit_name">Application Name</label>
					<input id="edit_name" type="text" bind:value={editForm.name} required />
				</div>

				<div class="form-group">
					<label for="edit_git_url">Git Repository URL</label>
					<input id="edit_git_url" type="text" bind:value={editForm.git_url} />
				</div>

				<div class="form-group">
					<label for="edit_git_branch">Git Branch</label>
					<input id="edit_git_branch" type="text" bind:value={editForm.git_branch} />
				</div>

				<div class="form-group">
					<label for="edit_build_strategy">Build Strategy</label>
					<select id="edit_build_strategy" bind:value={editForm.build_strategy}>
						<option value="dockerfile">Dockerfile</option>
						<option value="nixpacks">Nixpacks</option>
						<option value="docker_compose">Docker Compose</option>
					</select>
				</div>

				{#if editForm.build_strategy === 'dockerfile'}
					<div class="form-group">
						<label for="edit_dockerfile_path">Dockerfile Path</label>
						<input id="edit_dockerfile_path" type="text" bind:value={editForm.dockerfile_path} />
					</div>
				{/if}

				<div class="form-group">
					<label for="edit_port">Port</label>
					<input id="edit_port" type="number" bind:value={editForm.port} />
				</div>

				<div class="form-group-checkbox">
					<input id="edit_auto_deploy" type="checkbox" bind:checked={editForm.auto_deploy} />
					<label for="edit_auto_deploy">Enable auto-deploy on git push</label>
				</div>

				<div class="modal-actions">
					<button type="button" class="btn-secondary" onclick={() => (showDetailModal = false)}>Cancel</button>
					<button type="submit" class="btn-primary">Update</button>
				</div>
			</form>
		</div>
	</div>
{/if}

<!-- Environment Variables Modal -->
{#if showEnvModal && selectedApp}
	<div class="modal-overlay" onclick={() => (showEnvModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Environment Variables - {selectedApp.name}</h2>

			<div class="env-vars-section">
				<div class="env-vars-input">
					<input type="text" placeholder="KEY" bind:value={newEnvKey} />
					<input type="text" placeholder="Value" bind:value={newEnvValue} />
					<button class="btn-primary" onclick={addEnvVar}>Add</button>
				</div>

				<div class="env-vars-list">
					{#each appEnvVars as envVar (envVar.key)}
						<div class="env-var-item">
							<span class="env-key">{envVar.key}</span>
							<span class="env-value">{envVar.value}</span>
							<button class="btn-sm btn-danger" onclick={() => deleteEnvVar(envVar.key)}>Delete</button>
						</div>
					{/each}
				</div>
			</div>

			<div class="modal-actions">
				<button class="btn-secondary" onclick={() => (showEnvModal = false)}>Close</button>
			</div>
		</div>
	</div>
{/if}

<!-- Deploy Key Modal -->
{#if showDeployKeyModal && selectedApp}
	<div class="modal-overlay" onclick={() => (showDeployKeyModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Deploy Key - {selectedApp.name}</h2>

			{#if deployKey}
				<div class="deploy-key-section">
					<p class="deploy-key-info">Add this public key to your Git repository's deploy keys.</p>
					<div class="deploy-key-box">
						<pre>{deployKey.public_key}</pre>
					</div>
					<p class="deploy-key-date">Created: {new Date(deployKey.created_at).toLocaleString()}</p>
					<button class="btn-danger" onclick={regenerateDeployKey}>Regenerate Key</button>
				</div>
			{:else}
				<div class="deploy-key-section">
					<p>No deploy key found. A deploy key should have been generated when you created this application.</p>
					<button class="btn-primary" onclick={regenerateDeployKey}>Generate Deploy Key</button>
				</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-secondary" onclick={() => (showDeployKeyModal = false)}>Close</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.applications-page {
		padding: 2rem;
		max-width: 1400px;
		margin: 0 auto;
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
	}

	.header h1 {
		margin: 0 0 0.5rem 0;
		font-size: 2rem;
		font-weight: 600;
	}

	.header p {
		margin: 0;
		color: #6b7280;
	}

	.loading,
	.empty-state {
		text-align: center;
		padding: 3rem;
		color: #6b7280;
	}

	.empty-state button {
		margin-top: 1rem;
	}

	.applications-grid {
		display: grid;
		gap: 1.5rem;
		grid-template-columns: repeat(auto-fill, minmax(500px, 1fr));
	}

	.app-card {
		background: white;
		border: 1px solid #e5e7eb;
		border-radius: 8px;
		padding: 1.5rem;
	}

	.app-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1rem;
		gap: 1rem;
	}

	.app-header h3 {
		margin: 0 0 0.5rem 0;
		font-size: 1.25rem;
		font-weight: 600;
	}

	.app-actions {
		display: flex;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.status {
		display: inline-block;
		padding: 0.25rem 0.75rem;
		border-radius: 12px;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.status-green {
		background: #d1fae5;
		color: #065f46;
	}

	.status-blue {
		background: #dbeafe;
		color: #1e40af;
	}

	.status-gray {
		background: #f3f4f6;
		color: #374151;
	}

	.status-red {
		background: #fee2e2;
		color: #991b1b;
	}

	.app-details {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.detail-row {
		display: flex;
		gap: 0.5rem;
		font-size: 0.875rem;
	}

	.detail-row .label {
		font-weight: 600;
		color: #6b7280;
		min-width: 100px;
	}

	.git-url {
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: #4b5563;
	}

	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
	}

	.modal {
		background: white;
		border-radius: 8px;
		padding: 2rem;
		max-width: 600px;
		width: 100%;
		max-height: 90vh;
		overflow-y: auto;
	}

	.modal h2 {
		margin: 0 0 1.5rem 0;
		font-size: 1.5rem;
		font-weight: 600;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 500;
		color: #374151;
	}

	.form-group input,
	.form-group select {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #d1d5db;
		border-radius: 4px;
		font-size: 0.875rem;
	}

	.form-group-checkbox {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.form-group-checkbox input {
		width: auto;
	}

	.env-vars-section {
		margin-bottom: 1.5rem;
	}

	.env-vars-input {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1rem;
	}

	.env-vars-input input {
		flex: 1;
		padding: 0.5rem;
		border: 1px solid #d1d5db;
		border-radius: 4px;
		font-size: 0.875rem;
	}

	.env-vars-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		max-height: 300px;
		overflow-y: auto;
	}

	.env-var-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem;
		background: #f9fafb;
		border-radius: 4px;
	}

	.env-key {
		font-weight: 600;
		color: #374151;
		min-width: 120px;
	}

	.env-value {
		flex: 1;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: #6b7280;
	}

	.deploy-key-section {
		margin-bottom: 1.5rem;
	}

	.deploy-key-info {
		margin-bottom: 1rem;
		color: #6b7280;
	}

	.deploy-key-box {
		background: #f9fafb;
		border: 1px solid #e5e7eb;
		border-radius: 4px;
		padding: 1rem;
		margin-bottom: 1rem;
		overflow-x: auto;
	}

	.deploy-key-box pre {
		margin: 0;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.deploy-key-date {
		font-size: 0.875rem;
		color: #6b7280;
		margin-bottom: 1rem;
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 1.5rem;
		padding-top: 1.5rem;
		border-top: 1px solid #e5e7eb;
	}

	.btn-primary,
	.btn-secondary,
	.btn-danger,
	.btn-sm {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-weight: 500;
		transition: all 0.2s;
	}

	.btn-primary {
		background: #3b82f6;
		color: white;
	}

	.btn-primary:hover {
		background: #2563eb;
	}

	.btn-secondary {
		background: #f3f4f6;
		color: #374151;
	}

	.btn-secondary:hover {
		background: #e5e7eb;
	}

	.btn-danger {
		background: #ef4444;
		color: white;
	}

	.btn-danger:hover {
		background: #dc2626;
	}

	.btn-sm {
		padding: 0.375rem 0.75rem;
		font-size: 0.875rem;
		background: #f3f4f6;
		color: #374151;
	}

	.btn-sm:hover {
		background: #e5e7eb;
	}

	.btn-sm.btn-danger {
		background: #fee2e2;
		color: #991b1b;
	}

	.btn-sm.btn-danger:hover {
		background: #fecaca;
	}

	.error-banner {
		background: #fee2e2;
		color: #991b1b;
		padding: 1rem;
		border-radius: 4px;
		margin-bottom: 1rem;
	}
</style>
