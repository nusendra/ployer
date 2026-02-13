<script lang="ts">
	import '../../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Login - Ployer</title>
</svelte:head>

{@render children()}
