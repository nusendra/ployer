import { writable } from 'svelte/store';

interface WsMessage {
	type: string;
	[key: string]: any;
}

type MessageHandler = (message: WsMessage) => void;

class WebSocketClient {
	private ws: WebSocket | null = null;
	private reconnectTimer: number | null = null;
	private messageHandlers: Set<MessageHandler> = new Set();
	private subscriptions: Set<string> = new Set();
	public connected = writable(false);

	connect(token: string) {
		if (this.ws?.readyState === WebSocket.OPEN) {
			return; // Already connected
		}

		// In development, connect directly to backend. In production, use same host.
		const isDev = import.meta.env.DEV;
		const wsHost = isDev ? 'localhost:3001' : window.location.host;
		const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		const wsUrl = `${protocol}//${wsHost}/api/v1/ws?token=${encodeURIComponent(token)}`;

		console.log('Connecting to WebSocket:', wsUrl);
		this.ws = new WebSocket(wsUrl);

		this.ws.onopen = () => {
			console.log('WebSocket connected');
			this.connected.set(true);

			// Resubscribe to all channels after reconnect
			this.subscriptions.forEach(channel => {
				this.send({ type: 'subscribe', channel });
			});
		};

		this.ws.onmessage = (event) => {
			try {
				const message = JSON.parse(event.data) as WsMessage;
				this.messageHandlers.forEach(handler => handler(message));
			} catch (e) {
				console.error('Failed to parse WebSocket message:', e);
			}
		};

		this.ws.onerror = (error) => {
			console.error('WebSocket error:', error);
		};

		this.ws.onclose = () => {
			console.log('WebSocket disconnected');
			this.connected.set(false);
			this.scheduleReconnect(token);
		};
	}

	disconnect() {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}

		if (this.ws) {
			this.ws.close();
			this.ws = null;
		}

		this.connected.set(false);
		this.subscriptions.clear();
	}

	private scheduleReconnect(token: string) {
		if (this.reconnectTimer) return;

		this.reconnectTimer = setTimeout(() => {
			console.log('Attempting to reconnect...');
			this.reconnectTimer = null;
			this.connect(token);
		}, 3000) as unknown as number;
	}

	subscribe(channel: string) {
		this.subscriptions.add(channel);
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.send({ type: 'subscribe', channel });
		}
	}

	unsubscribe(channel: string) {
		this.subscriptions.delete(channel);
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.send({ type: 'unsubscribe', channel });
		}
	}

	onMessage(handler: MessageHandler) {
		this.messageHandlers.add(handler);
		return () => {
			this.messageHandlers.delete(handler);
		};
	}

	private send(data: any) {
		if (this.ws?.readyState === WebSocket.OPEN) {
			this.ws.send(JSON.stringify(data));
		}
	}

	ping() {
		this.send({ type: 'ping' });
	}
}

export const wsClient = new WebSocketClient();
