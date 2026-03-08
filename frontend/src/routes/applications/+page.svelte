<script lang="ts">
	import { onMount } from 'svelte';
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

	// Deployment modal state
	let showDeploymentsModal = $state(false);
	let deployments = $state<any[]>([]);
	let showDeploymentLogsModal = $state(false);
	let selectedDeployment = $state<any>(null);
	let deploymentLogs = $state<string[]>([]);
	let deploying = $state(false);

	// Domain modal state
	let showDomainsModal = $state(false);
	let appDomains = $state<any[]>([]);
	let newDomain = $state('');

	// Webhook modal state
	let showWebhookModal = $state(false);
	let webhook = $state<any>(null);
	let webhookDeliveries = $state<any[]>([]);
	let selectedProvider = $state<'github' | 'gitlab'>('github');

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

			await api.put(`/applications/${selectedApp.id}`, payload);
			showDetailModal = false;
			await loadApplications();
		} catch (e: any) {
			error = e.message || 'Failed to update application';
		}
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

	async function openEnvModal(app: Application) {
		selectedApp = app;
		showEnvModal = true;
		await loadEnvVars(app.id);
	}

	async function loadEnvVars(appId: string) {
		try {
			const response = await api.get<{ env_vars: Array<{ key: string; value: string }> }>(
				`/applications/${appId}/envs`
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
			await api.post(`/applications/${selectedApp.id}/envs`, {
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
		showConfirm(`Delete environment variable "${key}"?`, async () => {
			error = '';
			try {
				await api.delete(`/applications/${selectedApp!.id}/envs/${key}`);
				await loadEnvVars(selectedApp!.id);
			} catch (e: any) {
				error = e.message || 'Failed to delete environment variable';
			}
		});
	}

	async function openDeployKeyModal(app: Application) {
		selectedApp = app;
		showDeployKeyModal = true;
		await loadDeployKey(app.id);
	}

	async function loadDeployKey(appId: string) {
		try {
			const response = await api.get<{ public_key: string; created_at: string }>(
				`/applications/${appId}/deploy-key`
			);
			deployKey = response;
		} catch (e: any) {
			deployKey = null;
		}
	}

	async function regenerateDeployKey() {
		if (!selectedApp) return;
		showConfirm('Regenerate deploy key? The old key will be invalidated.', async () => {
			error = '';
			try {
				const response = await api.post<{ public_key: string; created_at: string }>(
					`/applications/${selectedApp!.id}/deploy-key`,
					{}
				);
				deployKey = response;
			} catch (e: any) {
				error = e.message || 'Failed to regenerate deploy key';
			}
		}, 'Regenerate');
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

	async function openDeploymentsModal(app: Application) {
		selectedApp = app;
		showDeploymentsModal = true;
		await loadDeployments(app.id);
	}

	// Domain management functions
	async function openDomainsModal(app: Application) {
		selectedApp = app;
		showDomainsModal = true;
		await loadDomains(app.id);
	}

	async function loadDomains(appId: string) {
		try {
			const response = await api.get<{ domains: any[] }>(`/applications/${appId}/domains`);
			appDomains = response.domains;
		} catch (e: any) {
			console.error('Failed to load domains:', e);
			appDomains = [];
		}
	}

	async function addDomain() {
		if (!selectedApp || !newDomain.trim()) return;
		error = '';
		try {
			await api.post(`/applications/${selectedApp.id}/domains`, {
				domain: newDomain.trim(),
				is_primary: appDomains.length === 0
			});
			newDomain = '';
			await loadDomains(selectedApp.id);
		} catch (e: any) {
			error = e.message || 'Failed to add domain';
		}
	}

	async function removeDomain(domain: string) {
		if (!selectedApp) return;
		showConfirm(`Remove domain "${domain}"?`, async () => {
			error = '';
			try {
				await api.delete(`/applications/${selectedApp!.id}/domains/${domain}`);
				await loadDomains(selectedApp!.id);
			} catch (e: any) {
				error = e.message || 'Failed to remove domain';
			}
		}, 'Remove');
	}

	async function setPrimaryDomain(domain: string) {
		if (!selectedApp) return;
		error = '';
		try {
			await api.post(`/applications/${selectedApp.id}/domains/${domain}/primary`, {});
			await loadDomains(selectedApp.id);
		} catch (e: any) {
			error = e.message || 'Failed to set primary domain';
		}
	}

	async function verifyDomain(domain: string) {
		if (!selectedApp) return;
		error = '';
		try {
			const response = await api.post<{ success: boolean; message: string }>(
				`/applications/${selectedApp.id}/domains/${domain}/verify`,
				{}
			);
			alert(response.message);
			await loadDomains(selectedApp.id);
		} catch (e: any) {
			error = e.message || 'Failed to verify domain';
		}
	}

	async function openWebhookModal(app: Application) {
		selectedApp = app;
		showWebhookModal = true;
		await loadWebhook(app.id);
		await loadWebhookDeliveries(app.id);
	}

	async function loadWebhook(appId: string) {
		try {
			webhook = await api.get(`/applications/${appId}/webhooks`);
		} catch (e: any) {
			webhook = null;
		}
	}

	async function loadWebhookDeliveries(appId: string) {
		try {
			const deliveries = await api.get<any[]>(`/applications/${appId}/webhooks/deliveries`);
			webhookDeliveries = deliveries;
		} catch (e: any) {
			webhookDeliveries = [];
		}
	}

	async function createWebhook() {
		if (!selectedApp) return;
		error = '';
		try {
			webhook = await api.post(`/applications/${selectedApp.id}/webhooks`, {
				provider: selectedProvider
			});
		} catch (e: any) {
			error = e.message || 'Failed to create webhook';
		}
	}

	async function deleteWebhook() {
		if (!selectedApp) return;
		showConfirm('Delete webhook configuration? Auto-deploys will stop working.', async () => {
			error = '';
			try {
				await api.delete(`/applications/${selectedApp!.id}/webhooks`);
				webhook = null;
				webhookDeliveries = [];
			} catch (e: any) {
				error = e.message || 'Failed to delete webhook';
			}
		});
	}

	async function loadDeployments(appId: string) {
		try {
			const response = await api.get<{ deployments: any[] }>(
				`/deployments?application_id=${appId}`
			);
			deployments = response.deployments;
		} catch (e: any) {
			error = e.message || 'Failed to load deployments';
		}
	}

	async function openDeploymentLogs(deployment: any) {
		selectedDeployment = deployment;
		deploymentLogs = deployment.build_log ? deployment.build_log.split('\n').filter((l: string) => l) : [];
		showDeploymentLogsModal = true;
	}

	function getDeploymentStatusColor(status: string) {
		switch (status) {
			case 'running':
				return 'green';
			case 'queued':
			case 'cloning':
			case 'building':
			case 'deploying':
				return 'blue';
			case 'failed':
				return 'red';
			case 'cancelled':
				return 'gray';
			default:
				return 'gray';
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
					<!-- Card top: avatar + name + status -->
					<div class="app-card-top">
						<div class="app-avatar">{app.name[0].toUpperCase()}</div>
						<div class="app-title">
							<h3>{app.name}</h3>
							<span class="status-chip status-{getStatusColor(app.status)}">{app.status}</span>
						</div>
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
							<button class="btn-action" onclick={() => openDeploymentsModal(app)}>History</button>
							<button class="btn-action" onclick={() => openDomainsModal(app)}>Domains</button>
							<button class="btn-action" onclick={() => openDetailModal(app)}>Edit</button>
							<button class="btn-action" onclick={() => openEnvModal(app)}>Env</button>
							{#if app.git_url}
								<button class="btn-action" onclick={() => openDeployKeyModal(app)}>Key</button>
								<button class="btn-action" onclick={() => openWebhookModal(app)}>Webhook</button>
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

<!-- Deployments Modal -->
{#if showDeploymentsModal && selectedApp}
	<div class="modal-overlay" onclick={() => (showDeploymentsModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Deployments - {selectedApp.name}</h2>

			{#if deployments.length === 0}
				<p>No deployments yet. Click the "Deploy" button to create your first deployment.</p>
			{:else}
				<div class="deployments-list">
					{#each deployments as deployment (deployment.id)}
						<div class="deployment-item">
							<div class="deployment-header">
								<div>
									<span class="status status-{getDeploymentStatusColor(deployment.status)}">
										{deployment.status}
									</span>
									<span class="deployment-id">{deployment.id.substring(0, 8)}</span>
								</div>
								<button class="btn-sm" onclick={() => openDeploymentLogs(deployment)}>
									View Logs
								</button>
							</div>
							<div class="deployment-details">
								{#if deployment.commit_sha}
									<div>Commit: {deployment.commit_sha.substring(0, 7)}</div>
								{/if}
								{#if deployment.commit_message}
									<div>Message: {deployment.commit_message}</div>
								{/if}
								<div>Started: {new Date(deployment.started_at).toLocaleString()}</div>
								{#if deployment.finished_at}
									<div>Finished: {new Date(deployment.finished_at).toLocaleString()}</div>
								{/if}
							</div>
						</div>
					{/each}
				</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-secondary" onclick={() => (showDeploymentsModal = false)}>Close</button>
			</div>
		</div>
	</div>
{/if}

<!-- Deployment Logs Modal -->
{#if showDeploymentLogsModal && selectedDeployment}
	<div class="modal-overlay" onclick={() => (showDeploymentLogsModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Deployment Logs</h2>
			<p>
				<span class="status status-{getDeploymentStatusColor(selectedDeployment.status)}">
					{selectedDeployment.status}
				</span>
				<span class="deployment-id">{selectedDeployment.id}</span>
			</p>

			<div class="log-viewer">
				{#if deploymentLogs.length === 0}
					<div class="log-line">No logs available yet...</div>
				{:else}
					{#each deploymentLogs as line}
						<div class="log-line">{line}</div>
					{/each}
				{/if}
			</div>

			<div class="modal-actions">
				<button class="btn-secondary" onclick={() => (showDeploymentLogsModal = false)}>Close</button>
			</div>
		</div>
	</div>
{/if}

<!-- Domains Modal -->
{#if showDomainsModal && selectedApp}
	<div class="modal-overlay" onclick={() => (showDomainsModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Domains - {selectedApp.name}</h2>

			<div class="domains-section">
				<div class="domain-input">
					<input type="text" placeholder="example.com" bind:value={newDomain} />
					<button class="btn-primary" onclick={addDomain}>Add Domain</button>
				</div>

				{#if appDomains.length === 0}
					<p class="empty-message">No custom domains added yet.</p>
					<p class="domain-hint">
						Your app will be available at: <strong>{selectedApp.name}.{servers.find((s) => s.id === selectedApp.server_id)?.host || 'localhost'}</strong>
					</p>
				{:else}
					<div class="domains-list">
						{#each appDomains as domain (domain.id)}
							<div class="domain-item">
								<div class="domain-info">
									<span class="domain-name">{domain.domain}</span>
									{#if domain.is_primary}
										<span class="badge badge-primary">Primary</span>
									{/if}
									{#if domain.ssl_active}
										<span class="badge badge-success">SSL Active</span>
									{:else}
										<span class="badge badge-warning">SSL Pending</span>
									{/if}
								</div>
								<div class="domain-actions">
									{#if !domain.is_primary}
										<button class="btn-sm" onclick={() => setPrimaryDomain(domain.domain)}>
											Set Primary
										</button>
									{/if}
									{#if !domain.ssl_active}
										<button class="btn-sm" onclick={() => verifyDomain(domain.domain)}>
											Verify
										</button>
									{/if}
									<button class="btn-sm btn-danger" onclick={() => removeDomain(domain.domain)}>
										Remove
									</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}

				<div class="dns-instructions">
					<h4>DNS Configuration</h4>
					<p>Point your domain to this server by adding an A record:</p>
					<div class="dns-record">
						<code>
							Type: A<br />
							Name: @ (or your subdomain)<br />
							Value: {servers.find((s) => s.id === selectedApp?.server_id)?.host || 'server-ip'}
						</code>
					</div>
				</div>
			</div>

			<div class="modal-actions">
				<button class="btn-secondary" onclick={() => (showDomainsModal = false)}>Close</button>
			</div>
		</div>
	</div>
{/if}

<!-- Webhook Modal -->
{#if showWebhookModal && selectedApp}
	<div class="modal-overlay" onclick={() => (showWebhookModal = false)}>
		<div class="modal" onclick={(e) => e.stopPropagation()}>
			<h2>Webhooks - {selectedApp.name}</h2>

			{#if !webhook}
				<div class="webhook-setup">
					<p>Configure webhook to auto-deploy when you push to your repository.</p>

					<div class="form-group">
						<label>Git Provider</label>
						<select bind:value={selectedProvider}>
							<option value="github">GitHub</option>
							<option value="gitlab">GitLab</option>
						</select>
					</div>

					<button class="btn-primary" onclick={createWebhook}>Create Webhook</button>
				</div>
			{:else}
				<div class="webhook-info">
					<div class="info-section">
						<h3>Webhook URL</h3>
						<div class="code-box">
							<code>{webhook.webhook_url}</code>
						</div>
						<p class="hint">Add this URL to your {webhook.provider} repository webhook settings</p>
					</div>

					<div class="info-section">
						<h3>Secret Token</h3>
						<div class="code-box">
							<code>{webhook.secret}</code>
						</div>
						<p class="hint">Use this secret for webhook signature verification</p>
					</div>

					{#if webhook.provider === 'github'}
						<div class="info-section">
							<h3>GitHub Configuration</h3>
							<ol>
								<li>Go to your repository → Settings → Webhooks → Add webhook</li>
								<li>Paste the Webhook URL above</li>
								<li>Set Content type to: <code>application/json</code></li>
								<li>Paste the Secret Token above</li>
								<li>Select event: <code>Push events</code></li>
								<li>Click "Add webhook"</li>
							</ol>
						</div>
					{:else}
						<div class="info-section">
							<h3>GitLab Configuration</h3>
							<ol>
								<li>Go to your repository → Settings → Webhooks</li>
								<li>Paste the Webhook URL above</li>
								<li>Paste the Secret Token</li>
								<li>Check "Push events"</li>
								<li>Click "Add webhook"</li>
							</ol>
						</div>
					{/if}

					<div class="info-section">
						<h3>Recent Deliveries</h3>
						{#if webhookDeliveries.length === 0}
							<p class="empty-message">No webhook deliveries yet</p>
						{:else}
							<div class="deliveries-list">
								{#each webhookDeliveries as delivery (delivery.id)}
									<div class="delivery-item">
										<div class="delivery-header">
											<span class="delivery-event">{delivery.event_type}</span>
											<span class="delivery-status status-{delivery.status}">{delivery.status}</span>
										</div>
										{#if delivery.branch}
											<div class="delivery-details">
												<span><strong>Branch:</strong> {delivery.branch}</span>
												{#if delivery.commit_sha}
													<span><strong>Commit:</strong> {delivery.commit_sha.substring(0, 7)}</span>
												{/if}
												{#if delivery.author}
													<span><strong>Author:</strong> {delivery.author}</span>
												{/if}
											</div>
										{/if}
										{#if delivery.commit_message}
											<div class="delivery-message">{delivery.commit_message}</div>
										{/if}
										<div class="delivery-time">{new Date(delivery.delivered_at).toLocaleString()}</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>

					<button class="btn-danger" onclick={deleteWebhook}>Delete Webhook</button>
				</div>
			{/if}

			<div class="modal-actions">
				<button class="btn-secondary" onclick={() => (showWebhookModal = false)}>Close</button>
			</div>
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
