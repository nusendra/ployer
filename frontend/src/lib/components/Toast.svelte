<script lang="ts">
	import { toast } from '$lib/stores/toast';
</script>

<div class="toast-container">
	{#each $toast as t (t.id)}
		<div class="toast toast-{t.type}" role="alert">
			<span class="toast-icon">
				{#if t.type === 'success'}✓{:else if t.type === 'error'}✕{:else if t.type === 'warning'}⚠{:else}ℹ{/if}
			</span>
			<span class="toast-message">{t.message}</span>
			<button class="toast-close" onclick={() => toast.remove(t.id)} aria-label="Dismiss">✕</button>
		</div>
	{/each}
</div>

<style>
	.toast-container {
		position: fixed;
		bottom: 1.5rem;
		right: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		z-index: 9999;
		max-width: 380px;
		width: 100%;
	}

	.toast {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.75rem 1rem;
		border-radius: var(--radius);
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
		animation: slide-in 0.2s ease;
		font-size: 0.875rem;
	}

	@keyframes slide-in {
		from {
			transform: translateX(110%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}

	.toast-success { border-left: 3px solid var(--success); }
	.toast-error   { border-left: 3px solid var(--danger); }
	.toast-warning { border-left: 3px solid var(--warning); }
	.toast-info    { border-left: 3px solid var(--primary); }

	.toast-icon {
		font-size: 1rem;
		flex-shrink: 0;
	}
	.toast-success .toast-icon { color: var(--success); }
	.toast-error   .toast-icon { color: var(--danger); }
	.toast-warning .toast-icon { color: var(--warning); }
	.toast-info    .toast-icon { color: var(--primary); }

	.toast-message {
		flex: 1;
		color: var(--text);
		line-height: 1.4;
	}

	.toast-close {
		background: none;
		border: none;
		color: var(--text-muted);
		padding: 0;
		font-size: 0.75rem;
		cursor: pointer;
		line-height: 1;
		flex-shrink: 0;
	}
	.toast-close:hover { color: var(--text); }
</style>
