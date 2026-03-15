<script>
	let { children } = $props();
</script>

<svelte:head>
	<style>
		:root {
			--bg: #1a1b26;
			--bg-surface: #24283b;
			--bg-hover: #2f3349;
			--border: #3b4261;
			--text: #c0caf5;
			--text-muted: #565f89;
			--text-bright: #ffffff;
			--accent: #7aa2f7;
			--accent-hover: #89b4fa;
			--success: #9ece6a;
			--warning: #e0af68;
			--error: #f7768e;
			--eagle: #bb9af7;
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
