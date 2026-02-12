import { writable } from 'svelte/store';

interface User {
	id: string;
	email: string;
	name: string;
	role: string;
}

export const user = writable<User | null>(null);
export const isAuthenticated = writable(false);

export function setAuth(token: string, userData: User) {
	localStorage.setItem('token', token);
	user.set(userData);
	isAuthenticated.set(true);
}

export function clearAuth() {
	localStorage.removeItem('token');
	user.set(null);
	isAuthenticated.set(false);
}
