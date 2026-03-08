<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import type { Application, BuildStrategy } from '$lib/types';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';

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

	let deploying = $state(false);

	// Confirm modal
	let confirmModal = $state<{ message: string; confirmLabel?: string; onConfirm: () => void } | null>(null);

	function showConfirm(message: string, onConfirm: () => void, confirmLabel = 'Confirm') {
		confirmModal = { message, confirmLabel, onConfirm };
	}

	function closeConfirm() {
		confirmModal = null;
	}

	function handleConfirm() {
		confirmModal?.onConfirm();
		confirmModal = null;
	}

	onMount(async () => {
		await loadApplications();
		await loadServers();
	});

	async function loadApplications() {
		loading = true;
		error = '';
		try {
			const response = await api.get<{ applications: Application[] }>('/applications');
			applications = response.applications;
		} catch (e: any) {
			error = e.message || 'Failed to load applications';
		} finally {
			loading = false;
		}
	}

	async function loadServers() {
		try {
			const response = await api.get<{ servers: any[] }>('/servers');
			servers = response.servers;
		} catch (e: any) {
			console.error('Failed to load servers:', e);
		}
	}

	async function createApplication() {
		error = '';
		try {
			await api.post('/applications', createForm);
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

	async function deleteApplication(id: string) {
		showConfirm('Delete this application? All deployments and configuration will be lost.', async () => {
			error = '';
			try {
				await api.delete(`/applications/${id}`);
				await loadApplications();
			} catch (e: any) {
				error = e.message || 'Failed to delete application';
			}
		});
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

	async function triggerDeploy(app: Application) {
		showConfirm(`Deploy "${app.name}"? A new deployment will be triggered.`, async () => {
			deploying = true;
			error = '';
			try {
				await api.post(`/applications/${app.id}/deploy`, {});
				await loadApplications();
			} catch (e: any) {
				error = e.message || 'Failed to trigger deployment';
			} finally {
				deploying = false;
			}
		}, 'Deploy');
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
					<!-- Card top: avatar + name + status -->
					<div class="app-card-top">
						<div class="app-avatar">{app.name[0].toUpperCase()}</div>
						<div class="app-title">
							<h3>{app.name}</h3>
							<span class="status-chip status-{getStatusColor(app.status)}">{app.status}</span>
						</div>
						<a class="btn-view" href="/applications/{app.id}">View</a>
					</div>

					<!-- Meta info grid -->
					<div class="app-meta">
						<div class="meta-item">
							<span class="meta-label">Server</span>
							<span class="meta-value">{servers.find((s) => s.id === app.server_id)?.name || '—'}</span>
						</div>
						<div class="meta-item">
							<span class="meta-label">Build</span>
							<span class="meta-value">{getStrategyLabel(app.build_strategy)}</span>
						</div>
						{#if app.git_url}
							<div class="meta-item">
								<span class="meta-label">Branch</span>
								<span class="meta-value branch">{app.git_branch}</span>
							</div>
						{/if}
						{#if app.port}
							<div class="meta-item">
								<span class="meta-label">Port</span>
								<span class="meta-value">{app.port}</span>
							</div>
						{/if}
						<div class="meta-item">
							<span class="meta-label">Auto Deploy</span>
							<span class="meta-value">{app.auto_deploy ? 'On' : 'Off'}</span>
						</div>
					</div>

					{#if app.git_url}
						<div class="app-git-url">
							<svg width="12" height="12" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M2 2.5A2.5 2.5 0 0 1 4.5 0 2.5 2.5 0 0 1 7 2.5a2.5 2.5 0 0 1-1.5 2.29v1.27l3 1.5 3-1.5V4.79A2.5 2.5 0 0 1 13.5 0 2.5 2.5 0 0 1 16 2.5a2.5 2.5 0 0 1-2.5 2.5 2.49 2.49 0 0 1-1-.21v1.75l-4 2-4-2V4.79A2.49 2.49 0 0 1 3.5 5 2.5 2.5 0 0 1 1 2.5H2z" fill="currentColor"/><path d="M5.5 12.21A2.5 2.5 0 0 0 4.5 12a2.5 2.5 0 0 0-2.5 2.5A2.5 2.5 0 0 0 4.5 17a2.5 2.5 0 0 0 2.5-2.5v-1.75l-1-.54z" fill="currentColor"/></svg>
							<span>{app.git_url}</span>
						</div>
					{/if}

					<!-- Action footer -->
					<div class="app-card-footer">
						<div class="app-actions-left">
							{#if app.git_url}
								<button class="btn-action btn-deploy" onclick={() => triggerDeploy(app)} disabled={deploying}>
									{deploying ? 'Deploying…' : 'Deploy'}
								</button>
							{/if}
						</div>
						<button class="btn-action btn-delete" onclick={() => deleteApplication(app.id)}>Delete</button>
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

{#if confirmModal}
	<ConfirmModal
		message={confirmModal.message}
		confirmLabel={confirmModal.confirmLabel}
		onConfirm={handleConfirm}
		onCancel={closeConfirm}
	/>
{/if}

<style>
	.applications-page {
		max-width: 1400px;
		margin: 0 auto;
	}

	/* ── Header ── */
	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 2rem;
	}

	.header h1 {
		margin: 0 0 0.25rem 0;
		font-size: 1.5rem;
		font-weight: 600;
		color: var(--text);
	}

	.header p {
		margin: 0;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	/* ── State messages ── */
	.loading,
	.empty-state {
		text-align: center;
		padding: 4rem 2rem;
		color: var(--text-muted);
		font-size: 0.875rem;
	}

	.empty-state button {
		margin-top: 1rem;
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

	/* ── Grid ── */
	.applications-grid {
		display: grid;
		gap: 1.25rem;
		grid-template-columns: repeat(auto-fill, minmax(480px, 1fr));
	}

	/* ── App Card ── */
	.app-card {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
		transition: border-color 0.15s;
	}

	.app-card:hover {
		border-color: var(--primary);
	}

	.app-card-top {
		display: flex;
		align-items: center;
		gap: 0.875rem;
		justify-content: space-between;
	}

	.btn-view {
		padding: 0.3125rem 0.875rem;
		border-radius: 6px;
		font-size: 0.75rem;
		font-weight: 600;
		background: var(--primary);
		color: var(--bg);
		border: none;
		cursor: pointer;
		text-decoration: none;
		display: inline-flex;
		align-items: center;
		flex-shrink: 0;
		transition: opacity 0.15s;
	}

	.btn-view:hover {
		opacity: 0.85;
	}

	.app-avatar {
		width: 40px;
		height: 40px;
		border-radius: 10px;
		background: var(--primary);
		color: var(--bg);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.125rem;
		font-weight: 700;
		flex-shrink: 0;
	}

	.app-title {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		min-width: 0;
		flex: 1;
	}

	.app-title h3 {
		margin: 0;
		font-size: 1rem;
		font-weight: 600;
		color: var(--text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Status chips on card */
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

	.status-green {
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
	}

	.status-blue {
		background: rgba(50, 130, 184, 0.2);
		color: var(--primary);
	}

	.status-gray {
		background: rgba(126, 137, 172, 0.15);
		color: var(--text-muted);
	}

	.status-red {
		background: rgba(239, 68, 68, 0.15);
		color: var(--danger);
	}

	/* ── Meta grid ── */
	.app-meta {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.625rem 1rem;
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
		font-size: 0.8125rem;
		color: var(--text);
		font-weight: 500;
	}

	.meta-value.branch {
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: var(--primary);
	}

	/* ── Git URL strip ── */
	.app-git-url {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.4rem 0.625rem;
		background: var(--bg-tertiary);
		border-radius: 6px;
		color: var(--text-muted);
		font-size: 0.6875rem;
		font-family: 'Courier New', monospace;
		overflow: hidden;
	}

	.app-git-url span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* ── Card footer actions ── */
	.app-card-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		padding-top: 0.875rem;
		border-top: 1px solid var(--border);
	}

	.app-actions-left {
		display: flex;
		gap: 0.375rem;
		flex-wrap: wrap;
	}

	.btn-action {
		padding: 0.3125rem 0.625rem;
		border-radius: 6px;
		font-size: 0.75rem;
		font-weight: 500;
		background: var(--bg-tertiary);
		color: var(--text);
		border: 1px solid var(--border);
		cursor: pointer;
		transition: background 0.15s, border-color 0.15s;
		text-decoration: none;
		display: inline-flex;
		align-items: center;
	}

	.btn-action:hover {
		background: rgba(50, 130, 184, 0.15);
		border-color: var(--primary);
		color: var(--primary);
	}

	.btn-action.btn-deploy {
		background: var(--primary);
		color: var(--bg);
		border-color: var(--primary);
	}

	.btn-action.btn-deploy:hover:not(:disabled) {
		background: var(--primary-hover);
		border-color: var(--primary-hover);
		color: var(--bg);
	}

	.btn-action.btn-deploy:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.btn-action.btn-delete {
		color: var(--danger);
		border-color: rgba(239, 68, 68, 0.3);
	}

	.btn-action.btn-delete:hover {
		background: rgba(239, 68, 68, 0.15);
		border-color: var(--danger);
	}

	/* ── Modals ── */
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(2px);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1000;
		padding: 1rem;
	}

	.modal {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.75rem;
		max-width: 600px;
		width: 100%;
		max-height: 90vh;
		overflow-y: auto;
	}

	.modal h2 {
		margin: 0 0 1.5rem 0;
		font-size: 1.25rem;
		font-weight: 600;
		color: var(--text);
	}

	.modal p {
		color: var(--text-muted);
		margin: 0.75rem 0;
		font-size: 0.875rem;
	}

	.form-group {
		margin-bottom: 1rem;
	}

	.form-group label {
		display: block;
		margin-bottom: 0.375rem;
		font-size: 0.8125rem;
		font-weight: 500;
		color: var(--text-muted);
	}

	.form-group-checkbox {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1rem;
		font-size: 0.875rem;
		color: var(--text);
	}

	.form-group-checkbox input {
		width: auto;
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
		margin-top: 1.5rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.btn-primary {
		background: var(--primary);
		color: var(--bg);
		padding: 0.5rem 1rem;
		border: none;
		border-radius: var(--radius);
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: background 0.15s;
	}

	.btn-primary:hover {
		background: var(--primary-hover);
	}

	.btn-secondary {
		background: var(--bg-tertiary);
		color: var(--text);
		padding: 0.5rem 1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: background 0.15s;
	}

	.btn-secondary:hover {
		background: rgba(50, 130, 184, 0.1);
	}

	.btn-danger {
		background: var(--danger);
		color: white;
		padding: 0.5rem 1rem;
		border: none;
		border-radius: var(--radius);
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: background 0.15s;
	}

	.btn-danger:hover {
		background: #dc2626;
	}

	.btn-sm {
		padding: 0.375rem 0.75rem;
		border-radius: 6px;
		font-size: 0.8125rem;
		font-weight: 500;
		background: var(--bg-tertiary);
		color: var(--text);
		border: 1px solid var(--border);
		cursor: pointer;
		transition: background 0.15s;
	}

	.btn-sm:hover {
		background: rgba(50, 130, 184, 0.1);
	}

	.btn-sm.btn-danger {
		background: rgba(239, 68, 68, 0.15);
		color: var(--danger);
		border-color: rgba(239, 68, 68, 0.3);
	}

	.btn-sm.btn-danger:hover {
		background: rgba(239, 68, 68, 0.25);
	}

	.btn-sm.btn-primary {
		background: var(--primary);
		color: var(--bg);
		border-color: var(--primary);
	}

	.btn-sm.btn-primary:hover:not(:disabled) {
		background: var(--primary-hover);
	}

	.btn-sm:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	/* ── Env vars ── */
	.env-vars-section {
		margin-bottom: 1.5rem;
	}

	.env-vars-input {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	.env-vars-input input {
		flex: 1;
	}

	.env-vars-list {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
		max-height: 300px;
		overflow-y: auto;
	}

	.env-var-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		padding: 0.5rem 0.75rem;
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 6px;
	}

	.env-key {
		font-weight: 600;
		font-size: 0.8125rem;
		color: var(--primary);
		min-width: 120px;
		font-family: 'Courier New', monospace;
	}

	.env-value {
		flex: 1;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	/* ── Deploy key ── */
	.deploy-key-section {
		margin-bottom: 1.5rem;
	}

	.deploy-key-info {
		margin-bottom: 1rem;
		color: var(--text-muted);
		font-size: 0.875rem;
	}

	.deploy-key-box {
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 1rem;
		margin-bottom: 0.75rem;
		overflow-x: auto;
	}

	.deploy-key-box pre {
		margin: 0;
		font-family: 'Courier New', monospace;
		font-size: 0.6875rem;
		white-space: pre-wrap;
		word-break: break-all;
		color: var(--text);
	}

	.deploy-key-date {
		font-size: 0.8125rem;
		color: var(--text-muted);
		margin-bottom: 1rem;
	}

	/* ── Deployments ── */
	.deployments-list {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		max-height: 400px;
		overflow-y: auto;
	}

	.deployment-item {
		padding: 0.875rem;
		background: var(--bg-tertiary);
		border-radius: var(--radius);
		border: 1px solid var(--border);
	}

	.deployment-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.deployment-id {
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.deployment-details {
		font-size: 0.8125rem;
		color: var(--text-muted);
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	/* ── Status on modals (reuse from cards via class) ── */
	.status {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		padding: 0.15rem 0.6rem;
		border-radius: 20px;
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: capitalize;
	}

	/* ── Log viewer ── */
	.log-viewer {
		background: var(--bg);
		border: 1px solid var(--border);
		color: var(--text);
		font-family: 'Courier New', monospace;
		font-size: 0.6875rem;
		padding: 1rem;
		border-radius: var(--radius);
		max-height: 500px;
		overflow-y: auto;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.log-line {
		margin-bottom: 0.2rem;
		line-height: 1.4;
	}

	/* ── Domains ── */
	.domains-section {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.domain-input {
		display: flex;
		gap: 0.5rem;
	}

	.domain-input input {
		flex: 1;
	}

	.domains-list {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.domain-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.875rem;
		background: var(--bg-tertiary);
		border-radius: var(--radius);
		border: 1px solid var(--border);
		gap: 0.5rem;
	}

	.domain-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.domain-name {
		font-family: 'Courier New', monospace;
		font-size: 0.875rem;
		color: var(--text);
		font-weight: 600;
	}

	.domain-actions {
		display: flex;
		gap: 0.375rem;
		flex-shrink: 0;
	}

	.badge {
		padding: 0.15rem 0.5rem;
		border-radius: 12px;
		font-size: 0.6875rem;
		font-weight: 600;
	}

	.badge-primary {
		background: rgba(50, 130, 184, 0.2);
		color: var(--primary);
	}

	.badge-success {
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
	}

	.badge-warning {
		background: rgba(245, 158, 11, 0.15);
		color: var(--warning);
	}

	.empty-message {
		color: var(--text-muted);
		text-align: center;
		font-size: 0.875rem;
		margin: 1rem 0;
	}

	.domain-hint {
		color: var(--text-muted);
		text-align: center;
		font-size: 0.8125rem;
	}

	.domain-hint strong {
		color: var(--text);
		font-family: 'Courier New', monospace;
	}

	.dns-instructions {
		background: var(--bg-tertiary);
		padding: 1rem;
		border-radius: var(--radius);
		border-left: 3px solid var(--primary);
	}

	.dns-instructions h4 {
		margin: 0 0 0.5rem 0;
		color: var(--text);
		font-size: 0.875rem;
		font-weight: 600;
	}

	.dns-instructions p {
		margin: 0 0 0.5rem 0;
		font-size: 0.8125rem;
	}

	.dns-record {
		background: var(--bg);
		padding: 0.75rem;
		border-radius: 6px;
		border: 1px solid var(--border);
	}

	.dns-record code {
		color: var(--text);
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		line-height: 1.6;
	}

	/* ── Webhooks ── */
	.webhook-setup {
		padding: 0.5rem 0;
	}

	.webhook-info {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.info-section h3 {
		margin: 0 0 0.625rem 0;
		color: var(--text);
		font-size: 0.9375rem;
		font-weight: 600;
	}

	.code-box {
		background: var(--bg);
		border: 1px solid var(--border);
		padding: 0.75rem;
		border-radius: var(--radius);
		margin-bottom: 0.375rem;
	}

	.code-box code {
		color: var(--text);
		font-family: 'Courier New', monospace;
		font-size: 0.8125rem;
		word-break: break-all;
	}

	.hint {
		color: var(--text-muted);
		font-size: 0.8125rem;
		margin: 0;
	}

	.info-section ol {
		margin: 0.5rem 0;
		padding-left: 1.5rem;
		color: var(--text-muted);
	}

	.info-section ol li {
		margin: 0.375rem 0;
		font-size: 0.875rem;
	}

	.info-section ol code {
		background: var(--bg-tertiary);
		padding: 0.125rem 0.375rem;
		border-radius: 3px;
		font-family: 'Courier New', monospace;
		font-size: 0.8125rem;
		color: var(--primary);
		border: 1px solid var(--border);
	}

	/* ── Deliveries ── */
	.deliveries-list {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
		max-height: 300px;
		overflow-y: auto;
	}

	.delivery-item {
		background: var(--bg-tertiary);
		padding: 0.75rem;
		border-radius: var(--radius);
		border-left: 3px solid var(--border);
		border: 1px solid var(--border);
	}

	.delivery-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.delivery-event {
		font-weight: 600;
		color: var(--text);
		font-size: 0.875rem;
	}

	.delivery-status {
		padding: 0.125rem 0.5rem;
		border-radius: 12px;
		font-size: 0.6875rem;
		font-weight: 600;
	}

	.delivery-status.status-success {
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
	}

	.delivery-status.status-failed {
		background: rgba(239, 68, 68, 0.15);
		color: var(--danger);
	}

	.delivery-status.status-skipped {
		background: rgba(245, 158, 11, 0.15);
		color: var(--warning);
	}

	.delivery-details {
		display: flex;
		gap: 1rem;
		margin-bottom: 0.5rem;
		flex-wrap: wrap;
	}

	.delivery-details span {
		font-size: 0.8125rem;
		color: var(--text-muted);
	}

	.delivery-message {
		background: var(--bg);
		padding: 0.375rem 0.625rem;
		border-radius: 4px;
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
		font-family: 'Courier New', monospace;
		border: 1px solid var(--border);
	}

	.delivery-time {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
</style>
