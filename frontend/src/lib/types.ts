// Server types
export interface Server {
	id: string;
	name: string;
	host: string;
	port: number;
	username: string;
	is_local: boolean;
	status: 'online' | 'offline' | 'unknown';
	last_seen_at: string | null;
	created_at: string;
}

export interface ServerStats {
	total_memory_mb: number;
	used_memory_mb: number;
	cpu_count: number;
	cpu_usage: number;
}

// Application types
export type BuildStrategy = 'dockerfile' | 'nixpacks' | 'docker_compose';
export type AppStatus = 'pending' | 'deploying' | 'running' | 'stopped' | 'failed';

export interface Application {
	id: string;
	name: string;
	server_id: string;
	git_url: string | null;
	git_branch: string;
	build_strategy: BuildStrategy;
	dockerfile_path: string | null;
	port: number | null;
	auto_deploy: boolean;
	status: AppStatus;
	created_at: string;
	updated_at: string;
}

export interface EnvironmentVariable {
	key: string;
	value: string; // Decrypted value
}

export interface DeployKey {
	public_key: string;
	created_at: string;
}

// Container types
export interface ContainerInfo {
	id: string;
	name: string;
	image: string;
	status: string;
	state: string;
	created_at: string;
	ports: Array<{
		private_port: number;
		public_port?: number;
		protocol: string;
	}>;
}

export interface ContainerStats {
	cpu_percent: number;
	memory_mb: number;
	memory_limit_mb: number;
	network_rx_mb: number;
	network_tx_mb: number;
}

// Auth types
export interface LoginResponse {
	token: string;
	refresh_token: string;
	expires_at: string;
}

export interface User {
	id: string;
	email: string;
	created_at: string;
}
