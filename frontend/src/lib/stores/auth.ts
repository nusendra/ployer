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
	localStorage.setItem('user', JSON.stringify(userData));
	user.set(userData);
	isAuthenticated.set(true);
}

export function clearAuth() {
	localStorage.removeItem('token');
	localStorage.removeItem('user');
	user.set(null);
	isAuthenticated.set(false);
}

export function restoreAuth() {
	const token = localStorage.getItem('token');
	const userData = localStorage.getItem('user');
	if (token && userData) {
		try {
			user.set(JSON.parse(userData));
			isAuthenticated.set(true);
		} catch {
			// ignore malformed data
		}
	}
}
