<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<template>
  <div class="auth-layout">
    <div class="auth-container">
      <h1 class="auth-brand">Conf-Ops</h1>
      <RouterView />
    </div>
  </div>
</template>

<style scoped>
.auth-layout {
  min-height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
}

.auth-container {
  width: 100%;
  max-width: 400px;
  padding: 2rem;
}

.auth-brand {
  text-align: center;
  font-size: 2rem;
  margin-bottom: 2rem;
}
</style>
