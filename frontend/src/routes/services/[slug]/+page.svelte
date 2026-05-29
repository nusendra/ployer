<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api/client';
	import { toast } from '$lib/stores/toast';

	type InputSpec = {
		key: string;
		label: string;
		type: 'string' | 'password' | 'number' | 'bool';
		default?: string;
		generate?: { kind: 'password' | 'hex'; length?: number };
		required?: boolean;
	};

	type Template = {
		slug: string;
		name: string;
		description: string;
		category: string;
		tags: string[];
		inputs: InputSpec[];
		post_install?: { message?: string };
	};

	type InstallResult = {
		compose: string;
		resolved_inputs: Record<string, string>;
		post_install_message: string | null;
		outputs: { label: string; value: string }[];
		note: string;
	};

	let slug = $derived($page.params.slug);

	let template = $state<Template | null>(null);
	let loading = $state(true);
	let error = $state('');

	let appName = $state('');
	let values = $state<Record<string, string>>({});
	let showSecrets = $state<Record<string, boolean>>({});

	let installing = $state(false);
	let result = $state<InstallResult | null>(null);

	onMount(async () => {
		try {
			template = await api.get<Template>(`/templates/${slug}`);
			appName = template.slug;
			for (const inp of template.inputs) {
				values[inp.key] = inp.default ?? '';
			}
		} catch (e: any) {
			error = e.message || 'Failed to load service';
		} finally {
			loading = false;
		}
	});

	async function install() {
		if (!template) return;
		if (!appName.trim()) {
			toast.error('Name is required');
			return;
		}
		installing = true;
		error = '';
		try {
			result = await api.post<InstallResult>(`/templates/${slug}/install`, {
				app_name: appName.trim(),
				inputs: values
			});
		} catch (e: any) {
			error = e.message || 'Install failed';
		} finally {
			installing = false;
		}
	}

	function copy(value: string) {
		navigator.clipboard.writeText(value).then(
			() => toast.success('Copied'),
			() => toast.error('Copy failed')
		);
	}
</script>

<svelte:head><title>{template?.name ?? 'Service'} · Ployer</title></svelte:head>

<div class="page">
	<a href="/services" class="back">← All services</a>

	{#if loading}
		<p class="muted">Loading...</p>
	{:else if error && !template}
		<div class="error">{error}</div>
	{:else if template}
		<header class="hero">
			<h1>{template.name}</h1>
			<p>{template.description}</p>
			<div class="tags">
				<span class="tag category">{template.category}</span>
				{#each template.tags as t}
					<span class="tag">{t}</span>
				{/each}
			</div>
		</header>

		{#if !result}
			<form
				class="card form"
				onsubmit={(e) => {
					e.preventDefault();
					install();
				}}
			>
				<h2>Configure</h2>

				<label class="field">
					<span>Name</span>
					<input bind:value={appName} required minlength="1" maxlength="63" />
					<small>Used as the container/service name on the Ployer network.</small>
				</label>

				{#each template.inputs as inp}
					<label class="field">
						<span>{inp.label}</span>
						{#if inp.type === 'password'}
							<div class="password-row">
								<input
									type={showSecrets[inp.key] ? 'text' : 'password'}
									bind:value={values[inp.key]}
									placeholder={inp.generate ? 'Auto-generated if blank' : ''}
								/>
								<button
									type="button"
									class="ghost"
									onclick={() => (showSecrets[inp.key] = !showSecrets[inp.key])}
								>
									{showSecrets[inp.key] ? 'Hide' : 'Show'}
								</button>
							</div>
						{:else if inp.type === 'number'}
							<input type="number" bind:value={values[inp.key]} />
						{:else if inp.type === 'bool'}
							<select bind:value={values[inp.key]}>
								<option value="true">true</option>
								<option value="false">false</option>
							</select>
						{:else}
							<input type="text" bind:value={values[inp.key]} />
						{/if}
						<small class="key">{inp.key}</small>
					</label>
				{/each}

				{#if error}
					<div class="error">{error}</div>
				{/if}

				<div class="actions">
					<button type="submit" class="btn-primary" disabled={installing}>
						{installing ? 'Installing...' : 'Install'}
					</button>
				</div>
			</form>
		{:else}
			<section class="card result">
				<h2>Ready to deploy</h2>
				<p class="muted preview-note">{result.note}</p>

				{#if result.post_install_message}
					<pre class="message">{result.post_install_message}</pre>
				{/if}

				{#if result.outputs.length > 0}
					<h3>Connection details</h3>
					<div class="outputs">
						{#each result.outputs as out}
							<div class="output">
								<div class="output-label">{out.label}</div>
								<div class="output-row">
									<code>{out.value}</code>
									<button type="button" class="ghost" onclick={() => copy(out.value)}>Copy</button>
								</div>
							</div>
						{/each}
					</div>
				{/if}

				<h3>docker-compose.yml</h3>
				<div class="compose-wrap">
					<button type="button" class="ghost copy-compose" onclick={() => copy(result?.compose ?? '')}>
						Copy
					</button>
					<pre class="compose"><code>{result.compose}</code></pre>
				</div>

				<div class="actions">
					<button type="button" class="ghost" onclick={() => (result = null)}>
						Back to form
					</button>
				</div>
			</section>
		{/if}
	{/if}
</div>

<style>
	.page {
		padding: 2rem;
		max-width: 800px;
		margin: 0 auto;
	}
	.back {
		display: inline-block;
		margin-bottom: 1rem;
		font-size: 0.875rem;
	}
	.hero {
		margin-bottom: 2rem;
	}
	.hero h1 {
		font-size: 1.75rem;
		margin: 0 0 0.5rem;
	}
	.hero p {
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}
	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
	}
	.tag {
		font-size: 0.7rem;
		padding: 0.15rem 0.55rem;
		background: var(--bg-tertiary);
		border-radius: 999px;
		color: var(--text-muted);
	}
	.tag.category {
		background: var(--primary);
		color: var(--bg);
		font-weight: 600;
	}
	.form,
	.result {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}
	h2 {
		font-size: 1.1rem;
		margin: 0;
	}
	h3 {
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
		margin: 0.5rem 0 0;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}
	.field > span {
		font-size: 0.875rem;
		font-weight: 500;
	}
	.field small {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.key {
		font-family: monospace;
	}
	.password-row {
		display: flex;
		gap: 0.5rem;
	}
	.password-row input {
		flex: 1;
	}
	.ghost {
		background: var(--bg-tertiary);
		color: var(--text);
		border: 1px solid var(--border);
	}
	.ghost:hover {
		border-color: var(--primary);
	}
	.actions {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}
	.error {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--danger);
		color: var(--danger);
		padding: 0.6rem 0.85rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
	}
	.preview-note {
		font-size: 0.8rem;
		font-style: italic;
	}
	.message {
		background: var(--bg-tertiary);
		border-radius: var(--radius);
		padding: 0.75rem;
		font-family: monospace;
		font-size: 0.8rem;
		white-space: pre-wrap;
		color: var(--text);
	}
	.outputs {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.output {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}
	.output-label {
		font-size: 0.75rem;
		color: var(--text-muted);
	}
	.output-row {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}
	.output-row code {
		flex: 1;
		background: var(--bg-tertiary);
		border-radius: var(--radius);
		padding: 0.5rem 0.75rem;
		font-size: 0.8rem;
		overflow-x: auto;
	}
	.compose-wrap {
		position: relative;
	}
	.copy-compose {
		position: absolute;
		top: 0.5rem;
		right: 0.5rem;
		font-size: 0.75rem;
		padding: 0.3rem 0.6rem;
	}
	.compose {
		background: var(--bg-tertiary);
		border-radius: var(--radius);
		padding: 1rem;
		overflow-x: auto;
		font-family: monospace;
		font-size: 0.8rem;
		max-height: 360px;
		overflow-y: auto;
		white-space: pre;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
