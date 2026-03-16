<script lang="ts">
	import { onMount, onDestroy } from 'svelte';

	let { appId, token }: { appId: string; token: string } = $props();

	let container: HTMLDivElement;
	let statusMsg = $state('Connecting…');
	let connected = $state(false);

	let term: import('xterm').Terminal | null = null;
	let fitAddon: import('xterm-addon-fit').FitAddon | null = null;
	let ws: WebSocket | null = null;
	let resizeObserver: ResizeObserver | null = null;

	onMount(async () => {
		// Dynamic import to avoid SSR issues
		const { Terminal } = await import('xterm');
		const { FitAddon } = await import('xterm-addon-fit');

		term = new Terminal({
			cursorBlink: true,
			fontSize: 14,
			fontFamily: '"Fira Code", "Cascadia Code", "Courier New", monospace',
			theme: {
				background: '#0d1117',
				foreground: '#e6edf3',
				cursor: '#58a6ff',
				selectionBackground: '#264f78',
				black: '#484f58',
				red: '#ff7b72',
				green: '#3fb950',
				yellow: '#d29922',
				blue: '#58a6ff',
				magenta: '#bc8cff',
				cyan: '#39c5cf',
				white: '#b1bac4',
				brightBlack: '#6e7681',
				brightRed: '#ffa198',
				brightGreen: '#56d364',
				brightYellow: '#e3b341',
				brightBlue: '#79c0ff',
				brightMagenta: '#d2a8ff',
				brightCyan: '#56d4dd',
				brightWhite: '#f0f6fc'
			}
		});

		fitAddon = new FitAddon();
		term.loadAddon(fitAddon);
		term.open(container);
		fitAddon.fit();

		// Connect WebSocket — use same host/port as the page so the Vite proxy
		// (ws: true) forwards correctly in dev, and direct in production.
		const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
		const wsUrl = `${proto}//${window.location.host}/api/v1/applications/${appId}/terminal?token=${encodeURIComponent(token)}`;
		ws = new WebSocket(wsUrl);
		ws.binaryType = 'arraybuffer';

		ws.onopen = () => {
			connected = true;
			statusMsg = '';
			// Send initial terminal size
			sendResize();
		};

		ws.onmessage = (e) => {
			if (!term) return;
			if (e.data instanceof ArrayBuffer) {
				term.write(new Uint8Array(e.data));
			} else {
				term.write(e.data);
			}
		};

		ws.onclose = () => {
			connected = false;
			statusMsg = 'Connection closed.';
			term?.write('\r\n\x1b[90mConnection closed.\x1b[0m\r\n');
		};

		ws.onerror = () => {
			statusMsg = 'Connection error.';
			term?.write('\r\n\x1b[31mConnection error.\x1b[0m\r\n');
		};

		// Send keyboard input to the container
		term.onData((data) => {
			if (ws?.readyState === WebSocket.OPEN) {
				ws.send(new TextEncoder().encode(data));
			}
		});

		// Handle terminal resize
		resizeObserver = new ResizeObserver(() => {
			fitAddon?.fit();
			sendResize();
		});
		resizeObserver.observe(container);
	});

	function sendResize() {
		if (!term || !ws || ws.readyState !== WebSocket.OPEN) return;
		ws.send(JSON.stringify({ cols: term.cols, rows: term.rows }));
	}

	onDestroy(() => {
		resizeObserver?.disconnect();
		ws?.close();
		term?.dispose();
	});
</script>

<div class="terminal-wrapper">
	{#if statusMsg && !connected}
		<div class="terminal-status">{statusMsg}</div>
	{/if}
	<div class="terminal-container" bind:this={container}></div>
</div>

<style>
	.terminal-wrapper {
		flex: 1;
		display: flex;
		flex-direction: column;
		background: #0d1117;
		border-radius: 8px;
		overflow: hidden;
		height: 100%;
	}

	.terminal-status {
		padding: 0.75rem 1rem;
		font-size: 0.8125rem;
		color: #6e7681;
		font-family: monospace;
		background: #0d1117;
	}

	.terminal-container {
		flex: 1;
		padding: 0.5rem;
		overflow: hidden;
	}

	:global(.xterm) {
		height: 100%;
	}

	:global(.xterm-viewport) {
		border-radius: 0;
	}
</style>
