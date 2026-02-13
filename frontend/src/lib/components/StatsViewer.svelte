<script lang="ts">
	import { onMount } from 'svelte';
	import { wsClient } from '$lib/stores/websocket';

	interface Props {
		containerId: string;
		initialStats?: ContainerStats;
		onClose: () => void;
	}

	interface ContainerStats {
		cpu_usage: number;
		memory_usage_mb: number;
		memory_limit_mb: number;
		network_rx_bytes: number;
		network_tx_bytes: number;
	}

	let { containerId, initialStats, onClose }: Props = $props();

	let stats = $state<ContainerStats | null>(initialStats || null);
	let lastUpdate = $state<Date>(new Date());

	onMount(() => {
		// Subscribe to container stats channel
		const channel = `container:${containerId}:stats`;
		wsClient.subscribe(channel);

		// Handle incoming stats messages
		const unsubscribe = wsClient.onMessage((message) => {
			if (message.type === 'container_stats' && message.container_id === containerId) {
				stats = {
					cpu_usage: message.cpu_usage,
					memory_usage_mb: message.memory_usage_mb,
					memory_limit_mb: message.memory_limit_mb,
					network_rx_bytes: 0, // WebSocket simplified stats
					network_tx_bytes: 0
				};
				lastUpdate = new Date();
			}
		});

		return () => {
			unsubscribe();
			wsClient.unsubscribe(channel);
		};
	});

	function formatBytes(bytes: number): string {
		if (bytes === 0) return '0 MB';
		const mb = bytes / 1024 / 1024;
		if (mb < 1024) return `${mb.toFixed(2)} MB`;
		const gb = mb / 1024;
		return `${gb.toFixed(2)} GB`;
	}

	function getMemoryPercentage(): number {
		if (!stats || stats.memory_limit_mb === 0) return 0;
		return (stats.memory_usage_mb / stats.memory_limit_mb) * 100;
	}

	function getCpuColor(usage: number): string {
		if (usage < 50) return '#22c55e';
		if (usage < 80) return '#eab308';
		return '#ef4444';
	}

	function getMemoryColor(percentage: number): string {
		if (percentage < 70) return '#22c55e';
		if (percentage < 90) return '#eab308';
		return '#ef4444';
	}
</script>

<div class="modal-overlay" onclick={onClose}>
	<div class="modal-content" onclick={(e) => e.stopPropagation()}>
		<div class="modal-header">
			<div>
				<h3>Container Stats (Live)</h3>
				<p class="last-update">Last updated: {lastUpdate.toLocaleTimeString()}</p>
			</div>
			<button class="btn-close" onclick={onClose}>×</button>
		</div>
		{#if !stats}
			<p class="text-muted">Loading stats...</p>
		{:else}
			<div class="stats-grid">
				<div class="stat-card">
					<div class="stat-label">CPU Usage</div>
					<div class="stat-value" style="color: {getCpuColor(stats.cpu_usage)}">
						{stats.cpu_usage.toFixed(1)}%
					</div>
					<div class="stat-bar">
						<div class="stat-bar-fill" style="width: {Math.min(stats.cpu_usage, 100)}%; background: {getCpuColor(stats.cpu_usage)}"></div>
					</div>
				</div>

				<div class="stat-card">
					<div class="stat-label">Memory Usage</div>
					<div class="stat-value" style="color: {getMemoryColor(getMemoryPercentage())}">
						{stats.memory_usage_mb.toFixed(0)} MB
					</div>
					<div class="stat-bar">
						<div class="stat-bar-fill" style="width: {getMemoryPercentage()}%; background: {getMemoryColor(getMemoryPercentage())}"></div>
					</div>
					<div class="stat-subtext">
						{getMemoryPercentage().toFixed(1)}% of {stats.memory_limit_mb.toFixed(0)} MB
					</div>
				</div>

				<div class="stat-card">
					<div class="stat-label">Network RX</div>
					<div class="stat-value">{formatBytes(stats.network_rx_bytes)}</div>
				</div>

				<div class="stat-card">
					<div class="stat-label">Network TX</div>
					<div class="stat-value">{formatBytes(stats.network_tx_bytes)}</div>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
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
		min-width: 600px;
		max-width: 90vw;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.5rem;
	}

	.last-update {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-top: 0.25rem;
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
		cursor: pointer;
	}

	.btn-close:hover {
		color: var(--text);
	}

	.text-muted {
		color: var(--text-muted);
		text-align: center;
		padding: 2rem;
	}

	.stats-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	.stat-card {
		background: var(--bg-tertiary);
		padding: 1.25rem;
		border-radius: var(--radius);
	}

	.stat-label {
		font-size: 0.875rem;
		color: var(--text-muted);
		margin-bottom: 0.5rem;
		font-weight: 500;
	}

	.stat-value {
		font-size: 2rem;
		font-weight: 700;
		margin-bottom: 0.5rem;
	}

	.stat-bar {
		height: 8px;
		background: var(--bg-secondary);
		border-radius: 4px;
		overflow: hidden;
		margin-top: 0.75rem;
	}

	.stat-bar-fill {
		height: 100%;
		transition: width 0.3s ease;
		border-radius: 4px;
	}

	.stat-subtext {
		font-size: 0.75rem;
		color: var(--text-muted);
		margin-top: 0.5rem;
	}
</style>
