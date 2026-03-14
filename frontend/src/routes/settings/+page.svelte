<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import { toast } from '$lib/stores/toast';

	interface Settings {
		allow_registration: boolean;
	}

	let settings = $state<Settings | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');

	onMount(async () => {
		try {
			const res = await api.get<Settings>('/settings');
			settings = res;
		} catch (e: any) {
			error = e.message || 'Failed to load settings';
		} finally {
			loading = false;
		}
	});

	async function toggleRegistration() {
		if (!settings) return;
		saving = true;
		try {
			const updated = await api.put<Settings>('/settings', {
				allow_registration: !settings.allow_registration
			});
			settings = updated;
			toast.success(
				updated.allow_registration
					? 'Registration enabled'
					: 'Registration disabled'
			);
		} catch (e: any) {
			toast.error(e.message || 'Failed to save settings');
		} finally {
			saving = false;
		}
	}
</script>

<div class="settings-page">
	<div class="page-header">
		<h2>Settings</h2>
		<p>Manage global application settings.</p>
	</div>

	{#if error}
		<div class="error-banner">{error}</div>
	{/if}

	{#if loading}
		<div class="loading">Loading settings...</div>
	{:else if settings}
		<div class="settings-card">
			<div class="settings-section">
				<h3>User Access</h3>

				<label class="toggle-row" class:disabled={saving}>
					<div class="toggle-info">
						<span class="toggle-label">Allow Registration</span>
						<span class="toggle-description">
							When enabled, new users can create an account. When disabled, only existing users can log in.
						</span>
					</div>
					<div class="toggle-control">
						<input
							type="checkbox"
							id="allow-registration"
							checked={settings.allow_registration}
							disabled={saving}
							onchange={toggleRegistration}
						/>
						<span class="toggle-switch"></span>
					</div>
				</label>
			</div>
		</div>
	{/if}
</div>

<style>
	.settings-page {
		max-width: 720px;
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

	.loading {
		color: var(--text-muted);
		font-size: 0.875rem;
		padding: 2rem 0;
	}

	.settings-card {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 12px;
		overflow: hidden;
	}

	.settings-section {
		padding: 1.5rem;
	}

	.settings-section h3 {
		margin: 0 0 1.25rem;
		font-size: 0.9375rem;
		font-weight: 600;
		color: var(--text);
	}

	.toggle-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 2rem;
		cursor: pointer;
		padding: 1rem 1.25rem;
		border-radius: 10px;
		background: var(--bg-tertiary);
		border: 1px solid var(--border);
		transition: border-color 0.15s;
	}

	.toggle-row:hover:not(.disabled) {
		border-color: var(--primary);
	}

	.toggle-row.disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.toggle-info {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}

	.toggle-label {
		font-size: 0.9375rem;
		font-weight: 600;
		color: var(--text);
	}

	.toggle-description {
		font-size: 0.8125rem;
		color: var(--text-muted);
		line-height: 1.5;
	}

	/* Custom toggle switch */
	.toggle-control {
		position: relative;
		flex-shrink: 0;
	}

	.toggle-control input[type='checkbox'] {
		position: absolute;
		opacity: 0;
		width: 0;
		height: 0;
	}

	.toggle-switch {
		display: block;
		width: 44px;
		height: 24px;
		background: var(--border);
		border-radius: 12px;
		transition: background 0.2s;
		position: relative;
		cursor: pointer;
	}

	.toggle-switch::after {
		content: '';
		position: absolute;
		top: 3px;
		left: 3px;
		width: 18px;
		height: 18px;
		border-radius: 50%;
		background: white;
		transition: transform 0.2s;
	}

	.toggle-control input:checked ~ .toggle-switch {
		background: var(--primary);
	}

	.toggle-control input:checked ~ .toggle-switch::after {
		transform: translateX(20px);
	}

	.toggle-control input:disabled ~ .toggle-switch {
		cursor: not-allowed;
	}
</style>
