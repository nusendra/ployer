<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api/client';
	import { setAuth } from '$lib/stores/auth';

	let mode: 'login' | 'register' = 'login';
	let email = '';
	let password = '';
	let name = '';
	let error = '';
	let loading = false;

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;

		try {
			if (mode === 'register') {
				if (!name || !email || !password) {
					error = 'All fields are required';
					return;
				}
				if (password.length < 8) {
					error = 'Password must be at least 8 characters';
					return;
				}
				const res = await api.post<{ user: any; token: string }>('/auth/register', {
					email,
					password,
					name
				});
				setAuth(res.token, res.user);
				goto('/');
			} else {
				if (!email || !password) {
					error = 'Email and password are required';
					return;
				}
				const res = await api.post<{ user: any; token: string }>('/auth/login', {
					email,
					password
				});
				setAuth(res.token, res.user);
				goto('/');
			}
		} catch (e: any) {
			error = e.message || 'Authentication failed';
		} finally {
			loading = false;
		}
	}
</script>

<div class="login-container">
	<div class="login-card">
		<h1>Ployer</h1>
		<p class="subtitle">Lightweight self-hosting PaaS</p>

		<div class="tabs">
			<button class:active={mode === 'login'} onclick={() => (mode = 'login')}>Login</button>
			<button class:active={mode === 'register'} onclick={() => (mode = 'register')}>
				Register
			</button>
		</div>

		<form onsubmit={handleSubmit}>
			{#if mode === 'register'}
				<div class="form-group">
					<label for="name">Name</label>
					<input id="name" type="text" bind:value={name} placeholder="Your name" />
				</div>
			{/if}

			<div class="form-group">
				<label for="email">Email</label>
				<input id="email" type="email" bind:value={email} placeholder="you@example.com" />
			</div>

			<div class="form-group">
				<label for="password">Password</label>
				<input
					id="password"
					type="password"
					bind:value={password}
					placeholder="••••••••"
				/>
			</div>

			{#if error}
				<div class="error">{error}</div>
			{/if}

			<button type="submit" class="btn-primary" disabled={loading}>
				{loading ? 'Loading...' : mode === 'login' ? 'Login' : 'Register'}
			</button>
		</form>
	</div>
</div>

<style>
	.login-container {
		display: flex;
		align-items: center;
		justify-content: center;
		min-height: 100vh;
		background: var(--bg);
	}

	.login-card {
		width: 100%;
		max-width: 400px;
		padding: 2rem;
		background: var(--bg-secondary);
		border: 1px solid var(--border);
		border-radius: var(--radius);
	}

	h1 {
		text-align: center;
		color: var(--primary);
		margin-bottom: 0.5rem;
	}

	.subtitle {
		text-align: center;
		color: var(--text-muted);
		font-size: 0.875rem;
		margin-bottom: 2rem;
	}

	.tabs {
		display: flex;
		gap: 0.5rem;
		margin-bottom: 1.5rem;
	}

	.tabs button {
		flex: 1;
		padding: 0.5rem;
		background: var(--bg-tertiary);
		color: var(--text-muted);
		border: 1px solid var(--border);
	}

	.tabs button.active {
		background: var(--primary);
		color: white;
		border-color: var(--primary);
	}

	.form-group {
		margin-bottom: 1rem;
	}

	label {
		display: block;
		margin-bottom: 0.25rem;
		font-size: 0.875rem;
		color: var(--text-muted);
	}

	.error {
		padding: 0.75rem;
		margin-bottom: 1rem;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid var(--danger);
		border-radius: var(--radius);
		color: var(--danger);
		font-size: 0.875rem;
	}

	button[type='submit'] {
		width: 100%;
		margin-top: 0.5rem;
	}

	button:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
</style>
