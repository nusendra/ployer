<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { wsClient } from '$lib/stores/websocket';

	interface Props {
		containerId: string;
		initialLogs?: string[];
		onClose: () => void;
	}

	let { containerId, initialLogs = [], onClose }: Props = $props();

	let logs: string[] = $state([...initialLogs]);
	let autoScroll = $state(true);
	let logsContainer: HTMLDivElement;

	const MAX_LINES = 1000;

	onMount(() => {
		// Subscribe to container logs channel
		const channel = `container:${containerId}:logs`;
		wsClient.subscribe(channel);

		// Handle incoming log messages
		const unsubscribe = wsClient.onMessage((message) => {
			if (message.type === 'container_logs' && message.container_id === containerId) {
				logs = [...logs, message.line].slice(-MAX_LINES);
				if (autoScroll && logsContainer) {
					setTimeout(() => {
						logsContainer.scrollTop = logsContainer.scrollHeight;
					}, 0);
				}
			}
		});

		// Auto-scroll to bottom initially
		if (logsContainer) {
			logsContainer.scrollTop = logsContainer.scrollHeight;
		}

		return () => {
			unsubscribe();
			wsClient.unsubscribe(channel);
		};
	});

	function clearLogs() {
		logs = [];
	}

	function handleScroll() {
		if (!logsContainer) return;
		const { scrollTop, scrollHeight, clientHeight } = logsContainer;
		autoScroll = scrollTop + clientHeight >= scrollHeight - 50;
	}
</script>

<div class="modal-overlay" onclick={onClose}>
	<div class="modal-content modal-large" onclick={(e) => e.stopPropagation()}>
		<div class="modal-header">
			<h3>Container Logs (Live)</h3>
			<div class="header-actions">
				<button class="btn-secondary-small" onclick={clearLogs}>Clear</button>
				<button class="btn-close" onclick={onClose}>×</button>
			</div>
		</div>
		<div class="logs-container" bind:this={logsContainer} onscroll={handleScroll}>
			{#if logs.length === 0}
				<p class="text-muted">No logs available. Waiting for output...</p>
			{:else}
				{#each logs as log, i (i)}
					<div class="log-line">{log}</div>
				{/each}
			{/if}
		</div>
		{#if !autoScroll}
			<div class="scroll-hint">
				<button class="btn-secondary-small" onclick={() => {
					logsContainer.scrollTop = logsContainer.scrollHeight;
					autoScroll = true;
				}}>
					↓ Scroll to bottom
				</button>
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
		max-width: 90vw;
		max-height: 90vh;
		display: flex;
		flex-direction: column;
	}

	.modal-large {
		width: 900px;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.header-actions {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	.btn-secondary-small {
		padding: 0.25rem 0.75rem;
		font-size: 0.875rem;
		background: var(--bg-tertiary);
		color: var(--text);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		cursor: pointer;
	}

	.btn-secondary-small:hover {
		background: var(--border);
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

	.logs-container {
		background: #1a1a1a;
		border-radius: var(--radius);
		padding: 1rem;
		height: 500px;
		overflow-y: auto;
		font-family: 'Monaco', 'Menlo', 'Courier New', monospace;
		font-size: 0.875rem;
		flex: 1;
	}

	.log-line {
		color: #e0e0e0;
		margin: 0.25rem 0;
		white-space: pre-wrap;
		word-break: break-all;
		line-height: 1.4;
	}

	.text-muted {
		color: var(--text-muted);
		text-align: center;
		padding: 2rem;
	}

	.scroll-hint {
		margin-top: 0.5rem;
		text-align: center;
	}
</style>
