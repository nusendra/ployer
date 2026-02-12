<h2>Dashboard</h2>

<div class="dashboard-grid">
	<div class="card">
		<h3>Applications</h3>
		<p class="stat">0</p>
	</div>
	<div class="card">
		<h3>Servers</h3>
		<p class="stat">0</p>
	</div>
	<div class="card">
		<h3>Deployments</h3>
		<p class="stat">0</p>
	</div>
	<div class="card">
		<h3>System Status</h3>
		<p class="stat status-running">Healthy</p>
	</div>
</div>

<style>
	h2 {
		margin-bottom: 1.5rem;
	}

	.dashboard-grid {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
		gap: 1rem;
	}

	.card h3 {
		color: var(--text-muted);
		font-size: 0.875rem;
		font-weight: 400;
		margin-bottom: 0.5rem;
	}

	.stat {
		font-size: 2rem;
		font-weight: 700;
	}
</style>
