<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import type { Application, BuildStrategy } from '$lib/types';
	import ConfirmModal from '$lib/components/ConfirmModal.svelte';

	const appId = $page.params.id;

	type Tab = 'configuration' | 'env_vars' | 'deployments' | 'domains' | 'deploy_key' | 'webhooks';
	let activeTab = $state<Tab>('configuration');

	let app = $state<Application | null>(null);
	let servers = $state<any[]>([]);
	let loading = $state(true);
	let error = $state('');
	let saving = $state(false);
	let deploying = $state(false);

	// Configuration form
	let editForm = $state({
		name: '',
		git_url: '',
		git_branch: '',
		build_strategy: 'dockerfile' as BuildStrategy,
		dockerfile_path: '',
		port: undefined as number | undefined,
		auto_deploy: false
	});

	// Env vars
	let appEnvVars = $state<Array<{ key: string; value: string }>>([]);
	let newEnvKey = $state('');
	let newEnvValue = $state('');
	let loadingEnvs = $state(false);

	// Deploy key
	let deployKey = $state<{ public_key: string; created_at: string } | null>(null);
	let loadingDeployKey = $state(false);

	// Deployments
	let deployments = $state<any[]>([]);
	let selectedDeployment = $state<any>(null);
	let deploymentLogs = $state<string[]>([]);
	let showDeploymentLogs = $state(false);
	let loadingDeployments = $state(false);

	// Domains
	let appDomains = $state<any[]>([]);
	let newDomain = $state('');
	let loadingDomains = $state(false);

	// Webhooks
	let webhook = $state<any>(null);
	let webhookDeliveries = $state<any[]>([]);
	let selectedProvider = $state<'github' | 'gitlab'>('github');
	let loadingWebhook = $state(false);

	// Confirm modal
	let confirmModal = $state<{ message: string; confirmLabel?: string; onConfirm: () => void } | null>(null);

	function showConfirm(message: string, onConfirm: () => void, confirmLabel = 'Confirm') {
		confirmModal = { message, confirmLabel, onConfirm };
	}
	function closeConfirm() { confirmModal = null; }
	function handleConfirm() { confirmModal?.onConfirm(); confirmModal = null; }

	onMount(async () => {
		await Promise.all([loadApp(), loadServers()]);
	});

	async function loadApp() {
		loading = true;
		error = '';
		try {
			const response = await api.get<{ application: Application }>(`/applications/${appId}`);
			app = response.application;
			editForm = {
				name: app.name,
				git_url: app.git_url || '',
				git_branch: app.git_branch,
				build_strategy: app.build_strategy,
				dockerfile_path: app.dockerfile_path || '',
				port: app.port ?? undefined,
				auto_deploy: app.auto_deploy
			};
		} catch (e: any) {
			error = e.message || 'Failed to load application';
		} finally {
			loading = false;
		}
	}

	async function loadServers() {
		try {
			const response = await api.get<{ servers: any[] }>('/servers');
			servers = response.servers;
		} catch {}
	}

	async function switchTab(tab: Tab) {
		activeTab = tab;
		if (tab === 'env_vars' && appEnvVars.length === 0) await loadEnvVars();
		if (tab === 'deployments' && deployments.length === 0) await loadDeployments();
		if (tab === 'domains' && appDomains.length === 0) await loadDomains();
		if (tab === 'deploy_key' && !deployKey) await loadDeployKey();
		if (tab === 'webhooks' && !webhook) await loadWebhook();
	}

	// ── Configuration ──────────────────────────────────────────────────────────

	async function saveConfiguration() {
		if (!app) return;
		saving = true;
		error = '';
		try {
			const payload: any = {};
			if (editForm.name !== app.name) payload.name = editForm.name;
			if (editForm.git_url !== (app.git_url || '')) payload.git_url = editForm.git_url || null;
			if (editForm.git_branch !== app.git_branch) payload.git_branch = editForm.git_branch;
			if (editForm.build_strategy !== app.build_strategy) payload.build_strategy = editForm.build_strategy;
			if (editForm.dockerfile_path !== (app.dockerfile_path || '')) payload.dockerfile_path = editForm.dockerfile_path || null;
			if (editForm.port !== (app.port ?? undefined)) payload.port = editForm.port;
			if (editForm.auto_deploy !== app.auto_deploy) payload.auto_deploy = editForm.auto_deploy;

			await api.put(`/applications/${appId}`, payload);
			await loadApp();
		} catch (e: any) {
			error = e.message || 'Failed to save configuration';
		} finally {
			saving = false;
		}
	}

	async function triggerDeploy() {
		if (!app) return;
		showConfirm(`Deploy "${app.name}"? A new deployment will be triggered.`, async () => {
			deploying = true;
			error = '';
			try {
				await api.post(`/applications/${appId}/deploy`, {});
				await loadApp();
			} catch (e: any) {
				error = e.message || 'Failed to trigger deployment';
			} finally {
				deploying = false;
			}
		}, 'Deploy');
	}

	// ── Environment Variables ───────────────────────────────────────────────────

	async function loadEnvVars() {
		loadingEnvs = true;
		try {
			const response = await api.get<{ env_vars: Array<{ key: string; value: string }> }>(
				`/applications/${appId}/envs`
			);
			appEnvVars = response.env_vars;
		} catch (e: any) {
			error = e.message || 'Failed to load environment variables';
		} finally {
			loadingEnvs = false;
		}
	}

	async function addEnvVar() {
		if (!newEnvKey.trim()) return;
		error = '';
		try {
			await api.post(`/applications/${appId}/envs`, { key: newEnvKey.trim(), value: newEnvValue });
			newEnvKey = '';
			newEnvValue = '';
			await loadEnvVars();
		} catch (e: any) {
			error = e.message || 'Failed to add environment variable';
		}
	}

	async function deleteEnvVar(key: string) {
		showConfirm(`Delete environment variable "${key}"?`, async () => {
			error = '';
			try {
				await api.delete(`/applications/${appId}/envs/${key}`);
				await loadEnvVars();
			} catch (e: any) {
				error = e.message || 'Failed to delete environment variable';
			}
		}, 'Delete');
	}

	// ── Deployments ────────────────────────────────────────────────────────────

	async function loadDeployments() {
		loadingDeployments = true;
		try {
			const response = await api.get<{ deployments: any[] }>(`/deployments?application_id=${appId}`);
			deployments = response.deployments;
		} catch (e: any) {
			error = e.message || 'Failed to load deployments';
		} finally {
			loadingDeployments = false;
		}
	}

	function openDeploymentLogs(deployment: any) {
		selectedDeployment = deployment;
		deploymentLogs = deployment.build_log ? deployment.build_log.split('\n').filter((l: string) => l) : [];
		showDeploymentLogs = true;
	}

	function getDeploymentStatusColor(status: string) {
		switch (status) {
			case 'running': return 'green';
			case 'queued':
			case 'cloning':
			case 'building':
			case 'deploying': return 'blue';
			case 'failed': return 'red';
			case 'cancelled': return 'gray';
			default: return 'gray';
		}
	}

	// ── Domains ────────────────────────────────────────────────────────────────

	async function loadDomains() {
		loadingDomains = true;
		try {
			const response = await api.get<{ domains: any[] }>(`/applications/${appId}/domains`);
			appDomains = response.domains;
		} catch (e: any) {
			error = e.message || 'Failed to load domains';
		} finally {
			loadingDomains = false;
		}
	}

	async function addDomain() {
		if (!newDomain.trim()) return;
		error = '';
		try {
			await api.post(`/applications/${appId}/domains`, {
				domain: newDomain.trim(),
				is_primary: appDomains.length === 0
			});
			newDomain = '';
			await loadDomains();
		} catch (e: any) {
			error = e.message || 'Failed to add domain';
		}
	}

	async function removeDomain(domain: string) {
		showConfirm(`Remove domain "${domain}"?`, async () => {
			error = '';
			try {
				await api.delete(`/applications/${appId}/domains/${domain}`);
				await loadDomains();
			} catch (e: any) {
				error = e.message || 'Failed to remove domain';
			}
		}, 'Remove');
	}

	async function setPrimaryDomain(domain: string) {
		error = '';
		try {
			await api.post(`/applications/${appId}/domains/${domain}/primary`, {});
			await loadDomains();
		} catch (e: any) {
			error = e.message || 'Failed to set primary domain';
		}
	}

	async function verifyDomain(domain: string) {
		error = '';
		try {
			const response = await api.post<{ success: boolean; message: string }>(
				`/applications/${appId}/domains/${domain}/verify`, {}
			);
			alert(response.message);
			await loadDomains();
		} catch (e: any) {
			error = e.message || 'Failed to verify domain';
		}
	}

	// ── Deploy Key ─────────────────────────────────────────────────────────────

	async function loadDeployKey() {
		loadingDeployKey = true;
		try {
			const response = await api.get<{ public_key: string; created_at: string }>(
				`/applications/${appId}/deploy-key`
			);
			deployKey = response;
		} catch {
			deployKey = null;
		} finally {
			loadingDeployKey = false;
		}
	}

	async function regenerateDeployKey() {
		showConfirm('Regenerate deploy key? The old key will be invalidated.', async () => {
			error = '';
			try {
				const response = await api.post<{ public_key: string; created_at: string }>(
					`/applications/${appId}/deploy-key`, {}
				);
				deployKey = response;
			} catch (e: any) {
				error = e.message || 'Failed to regenerate deploy key';
			}
		}, 'Regenerate');
	}

	// ── Webhooks ────────────────────────────────────────────────────────────────

	async function loadWebhook() {
		loadingWebhook = true;
		try {
			webhook = await api.get(`/applications/${appId}/webhooks`);
			const deliveries = await api.get<any[]>(`/applications/${appId}/webhooks/deliveries`);
			webhookDeliveries = Array.isArray(deliveries) ? deliveries : [];
		} catch {
			webhook = null;
			webhookDeliveries = [];
		} finally {
			loadingWebhook = false;
		}
	}

	async function createWebhook() {
		error = '';
		try {
			webhook = await api.post(`/applications/${appId}/webhooks`, { provider: selectedProvider });
		} catch (e: any) {
			error = e.message || 'Failed to create webhook';
		}
	}

	async function deleteWebhook() {
		showConfirm('Delete webhook configuration? Auto-deploys will stop working.', async () => {
			error = '';
			try {
				await api.delete(`/applications/${appId}/webhooks`);
				webhook = null;
				webhookDeliveries = [];
			} catch (e: any) {
				error = e.message || 'Failed to delete webhook';
			}
		});
	}

	// ── Helpers ─────────────────────────────────────────────────────────────────

	function getStatusColor(status: string) {
		switch (status) {
			case 'running': return 'green';
			case 'deploying': return 'blue';
			case 'stopped': return 'gray';
			case 'failed': return 'red';
			default: return 'gray';
		}
	}

	function getStrategyLabel(strategy: BuildStrategy) {
		switch (strategy) {
			case 'dockerfile': return 'Dockerfile';
			case 'nixpacks': return 'Nixpacks';
			case 'docker_compose': return 'Docker Compose';
			default: return strategy;
		}
	}

	const tabs: { id: Tab; label: string; icon: string }[] = [
		{ id: 'configuration', label: 'Configuration', icon: '⚙️' },
		{ id: 'env_vars', label: 'Environment Variables', icon: '🔑' },
		{ id: 'deployments', label: 'Deployments', icon: '🚀' },
		{ id: 'domains', label: 'Domains', icon: '🌐' },
		{ id: 'deploy_key', label: 'Deploy Key', icon: '🗝️' },
		{ id: 'webhooks', label: 'Webhooks', icon: '🔗' }
	];
</script>

<div class="detail-page">
	<!-- Back link -->
	<a href="/applications" class="back-link">
		<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5M12 5l-7 7 7 7"/></svg>
		Applications
	</a>

	{#if loading}
		<div class="loading">Loading application...</div>
	{:else if error && !app}
		<div class="error-banner">{error}</div>
	{:else if app}
		<!-- App Header -->
		<div class="app-header">
			<div class="app-header-left">
				<div class="app-avatar">{app.name[0].toUpperCase()}</div>
				<div class="app-header-info">
					<h1>{app.name}</h1>
					<div class="app-header-meta">
						<span class="status-chip status-{getStatusColor(app.status)}">{app.status}</span>
						<span class="header-sep">·</span>
						<span class="header-text">{servers.find(s => s.id === app!.server_id)?.name || 'Unknown server'}</span>
						{#if app.git_url}
							<span class="header-sep">·</span>
							<span class="header-text mono">{app.git_branch}</span>
						{/if}
					</div>
				</div>
			</div>
			<div class="app-header-actions">
				{#if app.git_url}
					<button class="btn-deploy" onclick={triggerDeploy} disabled={deploying}>
						{deploying ? 'Deploying…' : 'Deploy'}
					</button>
				{/if}
			</div>
		</div>

		{#if error}
			<div class="error-banner">{error}</div>
		{/if}

		<!-- Body: sidebar + content -->
		<div class="detail-body">
			<!-- Sidebar -->
			<nav class="detail-sidebar">
				{#each tabs as tab}
					<button
						class="sidebar-item {activeTab === tab.id ? 'active' : ''}"
						onclick={() => switchTab(tab.id)}
					>
						<span class="sidebar-icon">{tab.icon}</span>
						{tab.label}
					</button>
				{/each}
			</nav>

			<!-- Content -->
			<div class="detail-content">

				<!-- Configuration -->
				{#if activeTab === 'configuration'}
					<div class="content-section">
						<div class="section-header">
							<h2>Configuration</h2>
							<p>Manage your application settings and build configuration.</p>
						</div>

						<form onsubmit={(e) => { e.preventDefault(); saveConfiguration(); }}>
							<div class="form-grid">
								<div class="form-group">
									<label for="name">Application Name</label>
									<input id="name" type="text" bind:value={editForm.name} required />
								</div>

								<div class="form-group">
									<label for="git_url">Git Repository URL</label>
									<input id="git_url" type="text" bind:value={editForm.git_url} placeholder="git@github.com:user/repo.git" />
								</div>

								<div class="form-group">
									<label for="git_branch">Git Branch</label>
									<input id="git_branch" type="text" bind:value={editForm.git_branch} />
								</div>

								<div class="form-group">
									<label for="build_strategy">Build Strategy</label>
									<select id="build_strategy" bind:value={editForm.build_strategy}>
										<option value="dockerfile">Dockerfile</option>
										<option value="nixpacks">Nixpacks</option>
										<option value="docker_compose">Docker Compose</option>
									</select>
								</div>

								{#if editForm.build_strategy === 'dockerfile'}
									<div class="form-group">
										<label for="dockerfile_path">Dockerfile Path</label>
										<input id="dockerfile_path" type="text" bind:value={editForm.dockerfile_path} placeholder="./Dockerfile" />
									</div>
								{/if}

								<div class="form-group">
									<label for="port">Port</label>
									<input id="port" type="number" bind:value={editForm.port} placeholder="3000" />
								</div>
							</div>

							<div class="form-group-checkbox">
								<input id="auto_deploy" type="checkbox" bind:checked={editForm.auto_deploy} />
								<label for="auto_deploy">Enable auto-deploy on git push</label>
							</div>

							<div class="form-actions">
								<button type="submit" class="btn-primary" disabled={saving}>
									{saving ? 'Saving…' : 'Save Configuration'}
								</button>
							</div>
						</form>
					</div>
				{/if}

				<!-- Environment Variables -->
				{#if activeTab === 'env_vars'}
					<div class="content-section">
						<div class="section-header">
							<h2>Environment Variables</h2>
							<p>These variables will be injected into your application at runtime.</p>
						</div>

						<div class="env-add-row">
							<input type="text" placeholder="KEY" bind:value={newEnvKey} class="env-key-input" />
							<input type="text" placeholder="Value" bind:value={newEnvValue} class="env-val-input" />
							<button class="btn-primary btn-sm" onclick={addEnvVar}>Add</button>
						</div>

						{#if loadingEnvs}
							<div class="loading-inline">Loading variables...</div>
						{:else if appEnvVars.length === 0}
							<div class="empty-vars">No environment variables defined yet.</div>
						{:else}
							<div class="env-table">
								<div class="env-table-header">
									<span>Key</span>
									<span>Value</span>
									<span></span>
								</div>
								{#each appEnvVars as envVar (envVar.key)}
									<div class="env-table-row">
										<span class="env-key">{envVar.key}</span>
										<span class="env-value">{envVar.value}</span>
										<button class="btn-icon-danger" onclick={() => deleteEnvVar(envVar.key)} title="Delete">
											<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14H6L5 6"/><path d="M10 11v6M14 11v6"/><path d="M9 6V4h6v2"/></svg>
										</button>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<!-- Deployments -->
				{#if activeTab === 'deployments'}
					<div class="content-section">
						<div class="section-header">
							<h2>Deployments</h2>
							<p>History of all deployments for this application.</p>
						</div>

						{#if loadingDeployments}
							<div class="loading-inline">Loading deployments...</div>
						{:else if deployments.length === 0}
							<div class="empty-vars">No deployments yet.</div>
						{:else if showDeploymentLogs && selectedDeployment}
							<button class="btn-back-inline" onclick={() => showDeploymentLogs = false}>
								← Back to deployments
							</button>
							<div class="deploy-log-header">
								<span class="status-chip status-{getDeploymentStatusColor(selectedDeployment.status)}">{selectedDeployment.status}</span>
								<span class="deploy-id-text">{selectedDeployment.id}</span>
							</div>
							<div class="log-viewer">
								{#if deploymentLogs.length === 0}
									<div class="log-line">No logs available yet...</div>
								{:else}
									{#each deploymentLogs as line}
										<div class="log-line">{line}</div>
									{/each}
								{/if}
							</div>
						{:else}
							<div class="deployments-list">
								{#each deployments as deployment (deployment.id)}
									<div class="deployment-item">
										<div class="deployment-row">
											<div class="deployment-row-left">
												<span class="status-chip status-{getDeploymentStatusColor(deployment.status)}">{deployment.status}</span>
												<span class="deploy-id-text">{deployment.id.substring(0, 8)}</span>
												{#if deployment.commit_sha}
													<span class="deploy-meta-text mono">{deployment.commit_sha.substring(0, 7)}</span>
												{/if}
											</div>
											<button class="btn-sm-ghost" onclick={() => openDeploymentLogs(deployment)}>View Logs</button>
										</div>
										{#if deployment.commit_message}
											<div class="deploy-commit-msg">{deployment.commit_message}</div>
										{/if}
										<div class="deploy-times">
											<span>Started: {new Date(deployment.started_at).toLocaleString()}</span>
											{#if deployment.finished_at}
												<span>Finished: {new Date(deployment.finished_at).toLocaleString()}</span>
											{/if}
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}

				<!-- Domains -->
				{#if activeTab === 'domains'}
					<div class="content-section">
						<div class="section-header">
							<h2>Domains</h2>
							<p>Manage custom domains for this application.</p>
						</div>

						<div class="domain-add-row">
							<input type="text" placeholder="example.com" bind:value={newDomain} class="domain-input-field" />
							<button class="btn-primary btn-sm" onclick={addDomain}>Add Domain</button>
						</div>

						{#if loadingDomains}
							<div class="loading-inline">Loading domains...</div>
						{:else if appDomains.length === 0}
							<div class="empty-vars">No custom domains added yet.</div>
							{#if app}
								<p class="hint">Your app is available at: <strong>{app.name}.{servers.find(s => s.id === app!.server_id)?.host || 'localhost'}</strong></p>
							{/if}
						{:else}
							<div class="domains-list">
								{#each appDomains as domain (domain.id)}
									<div class="domain-item">
										<div class="domain-item-left">
											<span class="domain-name">{domain.domain}</span>
											<div class="domain-badges">
												{#if domain.is_primary}<span class="badge badge-primary">Primary</span>{/if}
												{#if domain.ssl_active}
													<span class="badge badge-success">SSL Active</span>
												{:else}
													<span class="badge badge-warning">SSL Pending</span>
												{/if}
											</div>
										</div>
										<div class="domain-item-actions">
											{#if !domain.is_primary}
												<button class="btn-sm-ghost" onclick={() => setPrimaryDomain(domain.domain)}>Set Primary</button>
											{/if}
											{#if !domain.ssl_active}
												<button class="btn-sm-ghost" onclick={() => verifyDomain(domain.domain)}>Verify</button>
											{/if}
											<button class="btn-sm-ghost btn-sm-danger" onclick={() => removeDomain(domain.domain)}>Remove</button>
										</div>
									</div>
								{/each}
							</div>
						{/if}

						<div class="dns-instructions">
							<h4>DNS Configuration</h4>
							<p class="hint">Point your domain to this server by adding an A record:</p>
							<div class="code-box" style="margin-top: 0.5rem;">
								<code>Type: A &nbsp;·&nbsp; Name: @ &nbsp;·&nbsp; Value: {servers.find(s => s.id === app?.server_id)?.host || 'server-ip'}</code>
							</div>
						</div>
					</div>
				{/if}

				<!-- Deploy Key -->
				{#if activeTab === 'deploy_key'}
					<div class="content-section">
						<div class="section-header">
							<h2>Deploy Key</h2>
							<p>Add this public key to your Git repository's deploy keys to allow Ployer to clone your repository.</p>
						</div>

						{#if loadingDeployKey}
							<div class="loading-inline">Loading deploy key...</div>
						{:else if deployKey}
							<div class="deploy-key-box">
								<pre>{deployKey.public_key}</pre>
							</div>
							<p class="key-date">Created: {new Date(deployKey.created_at).toLocaleString()}</p>
							<button class="btn-danger" onclick={regenerateDeployKey}>Regenerate Key</button>
						{:else}
							<p class="empty-vars">No deploy key found.</p>
							<button class="btn-primary" onclick={regenerateDeployKey}>Generate Deploy Key</button>
						{/if}
					</div>
				{/if}

				<!-- Webhooks -->
				{#if activeTab === 'webhooks'}
					<div class="content-section">
						<div class="section-header">
							<h2>Webhooks</h2>
							<p>Configure auto-deploy when you push to your repository.</p>
						</div>

						{#if loadingWebhook}
							<div class="loading-inline">Loading webhook...</div>
						{:else if !webhook}
							<div class="webhook-setup">
								<div class="form-group" style="max-width: 280px;">
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
								<div class="info-block">
									<label class="info-label">Webhook URL</label>
									<div class="code-box"><code>{webhook.webhook_url}</code></div>
									<p class="hint">Add this URL to your {webhook.provider} repository webhook settings.</p>
								</div>

								<div class="info-block">
									<label class="info-label">Secret Token</label>
									<div class="code-box"><code>{webhook.secret}</code></div>
									<p class="hint">Use this secret for webhook signature verification.</p>
								</div>

								<div class="info-block">
									<label class="info-label">Recent Deliveries</label>
									{#if webhookDeliveries.length === 0}
										<p class="empty-vars">No webhook deliveries yet.</p>
									{:else}
										<div class="deliveries-list">
											{#each webhookDeliveries as delivery (delivery.id)}
												<div class="delivery-item">
													<div class="delivery-row">
														<span class="delivery-event">{delivery.event_type}</span>
														<span class="status-chip status-{delivery.status === 'success' ? 'green' : 'red'}">{delivery.status}</span>
													</div>
													{#if delivery.branch}
														<div class="delivery-meta">
															<span>{delivery.branch}</span>
															{#if delivery.commit_sha}<span>{delivery.commit_sha.substring(0, 7)}</span>{/if}
														</div>
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
					</div>
				{/if}

			</div>
		</div>
	{/if}
</div>

{#if confirmModal}
	<ConfirmModal
		message={confirmModal.message}
		confirmLabel={confirmModal.confirmLabel}
		onConfirm={handleConfirm}
		onCancel={closeConfirm}
	/>
{/if}

<style>
	.detail-page {
	}

	/* ── Back link ── */
	.back-link {
		display: inline-flex;
		align-items: center;
		gap: 0.4rem;
		font-size: 0.8125rem;
		color: var(--text-muted);
		text-decoration: none;
		margin-bottom: 1.5rem;
		transition: color 0.15s;
	}

	.back-link:hover {
		color: var(--primary);
	}

	/* ── App header ── */
	.app-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 2rem;
		padding-bottom: 1.5rem;
		border-bottom: 1px solid var(--border);
	}

	.app-header-left {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.app-avatar {
		width: 52px;
		height: 52px;
		border-radius: 12px;
		background: var(--primary);
		color: var(--bg);
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 1.375rem;
		font-weight: 700;
		flex-shrink: 0;
	}

	.app-header-info h1 {
		margin: 0 0 0.375rem;
		font-size: 1.375rem;
		font-weight: 700;
		color: var(--text);
	}

	.app-header-meta {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.header-sep {
		color: var(--border);
	}

	.header-text {
		font-size: 0.8125rem;
		color: var(--text-muted);
	}

	.header-text.mono {
		font-family: 'Courier New', monospace;
		color: var(--primary);
	}

	/* ── Status chip ── */
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

	.status-green { background: rgba(34, 197, 94, 0.15); color: var(--success); }
	.status-blue { background: rgba(50, 130, 184, 0.2); color: var(--primary); }
	.status-gray { background: rgba(126, 137, 172, 0.15); color: var(--text-muted); }
	.status-red { background: rgba(239, 68, 68, 0.15); color: var(--danger); }

	/* ── Deploy button ── */
	.btn-deploy {
		padding: 0.5rem 1.25rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
		font-weight: 600;
		background: var(--primary);
		color: var(--bg);
		border: none;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.btn-deploy:hover { opacity: 0.85; }
	.btn-deploy:disabled { opacity: 0.5; cursor: not-allowed; }

	/* ── Error ── */
	.error-banner {
		background: rgba(239, 68, 68, 0.15);
		border: 1px solid rgba(239, 68, 68, 0.3);
		color: var(--danger);
		padding: 0.75rem 1rem;
		border-radius: var(--radius);
		margin-bottom: 1.25rem;
		font-size: 0.875rem;
	}

	/* ── Loading ── */
	.loading {
		text-align: center;
		padding: 4rem 2rem;
		color: var(--text-muted);
		font-size: 0.875rem;
	}

	.loading-inline {
		color: var(--text-muted);
		font-size: 0.875rem;
		padding: 1rem 0;
	}

	/* ── Body layout ── */
	.detail-body {
		display: flex;
		gap: 1.5rem;
		align-items: flex-start;
	}

	/* ── Sidebar ── */
	.detail-sidebar {
		width: 220px;
		flex-shrink: 0;
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
		position: sticky;
		top: 1.5rem;
	}

	.sidebar-item {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		width: 100%;
		padding: 0.625rem 0.875rem;
		border-radius: 7px;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text-muted);
		background: transparent;
		border: none;
		cursor: pointer;
		text-align: left;
		transition: background 0.15s, color 0.15s;
	}

	.sidebar-item:hover {
		background: var(--bg-tertiary);
		color: var(--text);
	}

	.sidebar-item.active {
		background: rgba(50, 130, 184, 0.12);
		color: var(--primary);
		font-weight: 600;
	}

	.sidebar-icon {
		font-size: 1rem;
		line-height: 1;
	}

	/* ── Content ── */
	.detail-content {
		flex: 1;
		min-width: 0;
	}

	.content-section {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 1.75rem;
	}

	.section-header {
		margin-bottom: 1.75rem;
		padding-bottom: 1.25rem;
		border-bottom: 1px solid var(--border);
	}

	.section-header h2 {
		margin: 0 0 0.375rem;
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--text);
	}

	.section-header p {
		margin: 0;
		font-size: 0.8125rem;
		color: var(--text-muted);
	}

	/* ── Forms ── */
	.form-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 1.25rem;
		margin-bottom: 1.25rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.form-group label {
		font-size: 0.8125rem;
		font-weight: 500;
		color: var(--text-muted);
	}

	.form-group input,
	.form-group select {
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
		color: var(--text);
		outline: none;
		transition: border-color 0.15s;
	}

	.form-group input:focus,
	.form-group select:focus {
		border-color: var(--primary);
	}

	.form-group-checkbox {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.form-group-checkbox label {
		font-size: 0.875rem;
		color: var(--text);
		cursor: pointer;
	}

	.form-group-checkbox input[type='checkbox'] {
		width: 16px;
		height: 16px;
		cursor: pointer;
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
	}

	/* ── Buttons ── */
	.btn-primary {
		padding: 0.5rem 1.25rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
		font-weight: 600;
		background: var(--primary);
		color: var(--bg);
		border: none;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.btn-primary:hover { opacity: 0.85; }
	.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

	.btn-primary.btn-sm {
		padding: 0.375rem 0.875rem;
		font-size: 0.8125rem;
	}

	.btn-danger {
		padding: 0.5rem 1.25rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
		font-weight: 600;
		background: var(--danger);
		color: white;
		border: none;
		cursor: pointer;
		transition: opacity 0.15s;
	}

	.btn-danger:hover { opacity: 0.85; }

	.btn-icon-danger {
		background: transparent;
		border: none;
		cursor: pointer;
		color: var(--text-muted);
		padding: 0.25rem;
		border-radius: 4px;
		display: flex;
		align-items: center;
		transition: color 0.15s, background 0.15s;
	}

	.btn-icon-danger:hover {
		color: var(--danger);
		background: rgba(239, 68, 68, 0.1);
	}

	/* ── Env vars ── */
	.env-add-row {
		display: flex;
		gap: 0.625rem;
		margin-bottom: 1.5rem;
		align-items: center;
	}

	.env-key-input {
		width: 200px;
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
		color: var(--text);
		font-family: 'Courier New', monospace;
		outline: none;
		transition: border-color 0.15s;
	}

	.env-val-input {
		flex: 1;
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
		color: var(--text);
		outline: none;
		transition: border-color 0.15s;
	}

	.env-key-input:focus,
	.env-val-input:focus {
		border-color: var(--primary);
	}

	.empty-vars {
		color: var(--text-muted);
		font-size: 0.875rem;
		padding: 1.5rem 0;
	}

	.env-table {
		border: 1px solid var(--border);
		border-radius: 8px;
		overflow: hidden;
	}

	.env-table-header {
		display: grid;
		grid-template-columns: 1fr 1fr 36px;
		gap: 0;
		padding: 0.5rem 1rem;
		background: var(--bg-tertiary);
		font-size: 0.6875rem;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.env-table-row {
		display: grid;
		grid-template-columns: 1fr 1fr 36px;
		align-items: center;
		padding: 0.625rem 1rem;
		border-top: 1px solid var(--border);
		font-size: 0.8125rem;
	}

	.env-table-row:hover {
		background: rgba(50, 130, 184, 0.04);
	}

	.env-key {
		font-family: 'Courier New', monospace;
		color: var(--primary);
		font-size: 0.8125rem;
	}

	.env-value {
		font-family: 'Courier New', monospace;
		color: var(--text-muted);
		font-size: 0.8125rem;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* ── Deployments ── */
	.deployments-list {
		display: flex;
		flex-direction: column;
		gap: 0.625rem;
	}

	.deployment-item {
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.875rem 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.deployment-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.deployment-row-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.deploy-id-text {
		font-family: 'Courier New', monospace;
		font-size: 0.8125rem;
		color: var(--text-muted);
	}

	.deploy-meta-text {
		font-size: 0.75rem;
		color: var(--text-muted);
	}

	.deploy-meta-text.mono {
		font-family: 'Courier New', monospace;
	}

	.deploy-commit-msg {
		font-size: 0.8125rem;
		color: var(--text);
		padding-left: 0.125rem;
	}

	.deploy-times {
		display: flex;
		gap: 1.25rem;
		font-size: 0.75rem;
		color: var(--text-muted);
		flex-wrap: wrap;
	}

	.btn-sm-ghost {
		padding: 0.25rem 0.625rem;
		border-radius: 5px;
		font-size: 0.75rem;
		font-weight: 500;
		background: transparent;
		color: var(--text-muted);
		border: 1px solid var(--border);
		cursor: pointer;
		transition: background 0.15s, color 0.15s, border-color 0.15s;
		white-space: nowrap;
	}

	.btn-sm-ghost:hover {
		background: rgba(50, 130, 184, 0.1);
		color: var(--primary);
		border-color: var(--primary);
	}

	.btn-sm-ghost.btn-sm-danger:hover {
		background: rgba(239, 68, 68, 0.1);
		color: var(--danger);
		border-color: var(--danger);
	}

	.btn-back-inline {
		background: transparent;
		border: none;
		color: var(--primary);
		font-size: 0.8125rem;
		cursor: pointer;
		padding: 0;
		margin-bottom: 1rem;
		display: inline-flex;
		align-items: center;
		gap: 0.25rem;
	}

	.deploy-log-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		margin-bottom: 0.75rem;
	}

	.log-viewer {
		background: #0d1117;
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 1rem;
		max-height: 480px;
		overflow-y: auto;
		font-family: 'Courier New', monospace;
		font-size: 0.75rem;
		line-height: 1.6;
	}

	.log-line {
		color: #c9d1d9;
		white-space: pre-wrap;
		word-break: break-all;
	}

	/* ── Domains ── */
	.domain-add-row {
		display: flex;
		gap: 0.625rem;
		margin-bottom: 1.5rem;
		align-items: center;
	}

	.domain-input-field {
		flex: 1;
		max-width: 320px;
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		padding: 0.5rem 0.75rem;
		font-size: 0.875rem;
		color: var(--text);
		outline: none;
		transition: border-color 0.15s;
	}

	.domain-input-field:focus {
		border-color: var(--primary);
	}

	.domains-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		margin-bottom: 1.75rem;
	}

	.domain-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.75rem 1rem;
	}

	.domain-item-left {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		flex-wrap: wrap;
		min-width: 0;
	}

	.domain-name {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--text);
	}

	.domain-badges {
		display: flex;
		gap: 0.375rem;
	}

	.domain-item-actions {
		display: flex;
		gap: 0.375rem;
		flex-shrink: 0;
	}

	.badge {
		padding: 0.1rem 0.5rem;
		border-radius: 20px;
		font-size: 0.6875rem;
		font-weight: 600;
	}

	.badge-primary {
		background: rgba(50, 130, 184, 0.15);
		color: var(--primary);
	}

	.badge-success {
		background: rgba(34, 197, 94, 0.15);
		color: var(--success);
	}

	.badge-warning {
		background: rgba(234, 179, 8, 0.15);
		color: #ca8a04;
	}

	.dns-instructions {
		margin-top: 1.75rem;
		padding-top: 1.25rem;
		border-top: 1px solid var(--border);
	}

	.dns-instructions h4 {
		margin: 0 0 0.375rem;
		font-size: 0.875rem;
		font-weight: 600;
		color: var(--text);
	}

	/* ── Deploy key ── */
	.deploy-key-box {
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 1rem;
		margin-bottom: 0.75rem;
		overflow-x: auto;
	}

	.deploy-key-box pre {
		margin: 0;
		font-size: 0.75rem;
		color: var(--text);
		font-family: 'Courier New', monospace;
		white-space: pre-wrap;
		word-break: break-all;
	}

	.key-date {
		font-size: 0.8125rem;
		color: var(--text-muted);
		margin: 0 0 1rem;
	}

	/* ── Webhooks ── */
	.webhook-setup {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.webhook-info {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.info-block {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.info-label {
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.code-box {
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		overflow-x: auto;
	}

	.code-box code {
		font-size: 0.8125rem;
		color: var(--text);
		font-family: 'Courier New', monospace;
		word-break: break-all;
	}

	.hint {
		margin: 0;
		font-size: 0.8125rem;
		color: var(--text-muted);
	}

	.deliveries-list {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.delivery-item {
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.delivery-row {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.delivery-event {
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--text);
	}

	.delivery-meta {
		display: flex;
		gap: 1rem;
		font-size: 0.75rem;
		color: var(--text-muted);
		font-family: 'Courier New', monospace;
	}

	.delivery-time {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
</style>
