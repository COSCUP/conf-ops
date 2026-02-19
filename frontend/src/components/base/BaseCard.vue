<script setup lang="ts">
defineProps<{
  title?: string
}>()
</script>

<template>
  <div class="base-card">
    <div v-if="title" class="base-card__header">
      <h3 class="base-card__title">{{ title }}</h3>
    </div>
    <div class="base-card__body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.base-card {
  border: 1px solid #e5e7eb;
  border-radius: 0.5rem;
  background: white;
  overflow: hidden;
}

.base-card__header {
  padding: 1rem 1.5rem;
  border-bottom: 1px solid #e5e7eb;
}

.base-card__title {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
}

.base-card__body {
  padding: 1.5rem;
}
</style>
