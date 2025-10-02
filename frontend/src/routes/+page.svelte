<script lang="ts">
  export let data: {
    greeting: string | null;
  };

  const greeting = data.greeting ?? 'Unable to reach backend';
</script>

<h1>{greeting}</h1>
