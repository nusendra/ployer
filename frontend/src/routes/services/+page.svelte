<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';

	type CatalogEntry = {
		slug: string;
		name: string;
		description: string;
		category: string;
		icon: string | null;
		tags: string[];
	};

	let templates = $state<CatalogEntry[]>([]);
	let loading = $state(true);
	let error = $state('');
	let query = $state('');

	onMount(async () => {
		try {
			const res = await api.get<{ templates: CatalogEntry[] }>('/templates');
			templates = res.templates;
		} catch (e: any) {
			error = e.message || 'Failed to load services';
		} finally {
			loading = false;
		}
	});

	const filtered = $derived.by(() => {
		const q = query.trim().toLowerCase();
		if (!q) return templates;
		return templates.filter(
			(t) =>
				t.name.toLowerCase().includes(q) ||
				t.slug.toLowerCase().includes(q) ||
				t.description.toLowerCase().includes(q) ||
				t.tags.some((tag) => tag.toLowerCase().includes(q))
		);
	});

	const categories = $derived(
		Array.from(new Set(filtered.map((t) => t.category))).sort()
	);

	function byCategory(cat: string): CatalogEntry[] {
		return filtered.filter((t) => t.category === cat);
	}
</script>

<svelte:head><title>Services · Ployer</title></svelte:head>

<div class="page">
	<header class="page-header">
		<div>
			<h1>Services</h1>
			<p class="subtitle">One-click install for databases, queues, and more.</p>
		</div>
		<input
			type="search"
			placeholder="Search services..."
			bind:value={query}
			class="search"
		/>
	</header>

	{#if loading}
		<p class="muted">Loading...</p>
	{:else if error}
		<div class="error">{error}</div>
	{:else if filtered.length === 0}
		<p class="muted">No services match "{query}".</p>
	{:else}
		{#each categories as cat}
			<section class="category">
				<h2>{cat}</h2>
				<div class="grid">
					{#each byCategory(cat) as t}
						<a href="/services/{t.slug}" class="card service-card">
							<div class="service-head">
								<span class="service-name">{t.name}</span>
								<span class="service-slug">{t.slug}</span>
							</div>
							<p class="service-desc">{t.description}</p>
							<div class="tags">
								{#each t.tags as tag}
									<span class="tag">{tag}</span>
								{/each}
							</div>
						</a>
					{/each}
				</div>
			</section>
		{/each}
	{/if}
</div>

<style>
	.page {
		padding: 2rem;
		max-width: 1200px;
		margin: 0 auto;
	}
	.page-header {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1rem;
		margin-bottom: 2rem;
	}
	.page-header h1 {
		font-size: 1.5rem;
		margin: 0;
	}
	.subtitle {
		color: var(--text-muted);
		margin-top: 0.25rem;
	}
	.search {
		max-width: 280px;
	}
	.category {
		margin-bottom: 2rem;
	}
	.category h2 {
		font-size: 0.875rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--text-muted);
		margin-bottom: 0.75rem;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
		gap: 1rem;
	}
	.service-card {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		color: var(--text);
		transition: border-color 0.15s;
	}
	.service-card:hover {
		border-color: var(--primary);
	}
	.service-head {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 0.5rem;
	}
	.service-name {
		font-weight: 600;
	}
	.service-slug {
		font-size: 0.75rem;
		color: var(--text-muted);
		font-family: monospace;
	}
	.service-desc {
		color: var(--text-muted);
		font-size: 0.875rem;
		line-height: 1.4;
	}
	.tags {
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem;
		margin-top: auto;
	}
	.tag {
		font-size: 0.7rem;
		padding: 0.1rem 0.5rem;
		background: var(--bg-tertiary);
		border-radius: 999px;
		color: var(--text-muted);
	}
	.muted {
		color: var(--text-muted);
	}
	.error {
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--danger);
		color: var(--danger);
		padding: 0.75rem 1rem;
		border-radius: var(--radius);
	}
</style>
