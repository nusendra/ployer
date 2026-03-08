<script lang="ts">
	interface Props {
		message: string;
		confirmLabel?: string;
		onConfirm: () => void;
		onCancel: () => void;
	}

	let { message, confirmLabel = 'Delete', onConfirm, onCancel }: Props = $props();
</script>

<div class="modal-overlay" onclick={onCancel}>
	<div class="modal-content" onclick={(e) => e.stopPropagation()}>
		<div class="confirm-icon">
			<svg width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
				<path d="M12 9V13M12 17H12.01M10.29 3.86L1.82 18A2 2 0 0 0 3.64 21H20.36A2 2 0 0 0 22.18 18L13.71 3.86A2 2 0 0 0 10.29 3.86Z" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
			</svg>
		</div>
		<h3>Are you sure?</h3>
		<p>{message}</p>
		<div class="confirm-actions">
			<button class="btn-cancel" onclick={onCancel}>Cancel</button>
			<button class="btn-confirm" onclick={onConfirm}>{confirmLabel}</button>
		</div>
	</div>
</div>

<style>
	.modal-overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.8);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 1100;
		padding: 1rem;
	}

	.modal-content {
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: 10px;
		padding: 2rem;
		max-width: 400px;
		width: 100%;
		text-align: center;
	}

	.confirm-icon {
		width: 52px;
		height: 52px;
		border-radius: 50%;
		background: rgba(239, 68, 68, 0.12);
		color: var(--danger);
		display: flex;
		align-items: center;
		justify-content: center;
		margin: 0 auto 1rem;
	}

	h3 {
		margin: 0 0 0.5rem;
		font-size: 1.125rem;
		font-weight: 600;
		color: var(--text);
	}

	p {
		margin: 0 0 1.5rem;
		font-size: 0.875rem;
		color: var(--text-muted);
		line-height: 1.5;
	}

	.confirm-actions {
		display: flex;
		gap: 0.75rem;
		justify-content: center;
	}

	.btn-cancel {
		padding: 0.5rem 1.25rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
		font-weight: 500;
		background: var(--bg-tertiary);
		color: var(--text);
		border: 1px solid var(--border);
		cursor: pointer;
		transition: background 0.15s;
	}

	.btn-cancel:hover {
		background: rgba(50, 130, 184, 0.1);
	}

	.btn-confirm {
		padding: 0.5rem 1.25rem;
		border-radius: var(--radius);
		font-size: 0.875rem;
		font-weight: 500;
		background: var(--danger);
		color: white;
		border: none;
		cursor: pointer;
		transition: background 0.15s;
	}

	.btn-confirm:hover {
		background: #dc2626;
	}
</style>
