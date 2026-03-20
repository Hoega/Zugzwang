import type { AuthUser } from '$lib/types';
import { api } from './api';

let user = $state<AuthUser | null>(null);
let checked = $state(false);

export const auth = {
	get user() {
		return user;
	},
	get checked() {
		return checked;
	},
	get isLoggedIn() {
		return user !== null;
	},

	async check() {
		try {
			const res = await api.getMe();
			user = res.user;
		} catch {
			user = null;
		} finally {
			checked = true;
		}
	},

	async login(username: string, password: string) {
		const res = await api.login({ username, password });
		user = res.user;
	},

	async register(username: string, email: string, password: string) {
		const res = await api.register({ username, email, password });
		user = res.user;
	},

	async logout() {
		await api.logout();
		user = null;
		window.location.href = '/login';
	}
};
