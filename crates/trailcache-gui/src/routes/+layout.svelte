<script>
	let { children } = $props();
</script>

<svelte:head>
	<style>
		:root {
			--bg: #16181e;
			--bg-surface: #1e2028;
			--bg-hover: #282a34;
			--border: #363840;
			--text: #d0d4dc;
			--text-muted: #808080;
			--text-bright: #ffffff;
			--accent: #4080c0;
			--accent-hover: #5090d0;
			--gold: #c0a040;
			--success: #60a060;
			--warning: #c0a040;
			--error: #c04040;
			--eagle: #a080c0;
			font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
			color-scheme: dark;
		}

		* {
			margin: 0;
			padding: 0;
			box-sizing: border-box;
		}

		body {
			background: var(--bg);
			color: var(--text);
			overflow: hidden;
			height: 100vh;
			height: 100dvh;
			padding-top: env(safe-area-inset-top, 0);
		}
	</style>
</svelte:head>

{@render children()}
