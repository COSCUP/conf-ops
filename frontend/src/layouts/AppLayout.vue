<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<template>
  <div class="app-layout">
    <header class="app-header">
      <nav class="app-nav">
        <RouterLink to="/" class="app-brand">Conf-Ops</RouterLink>
        <div class="nav-links">
          <RouterLink to="/">Dashboard</RouterLink>
          <RouterLink to="/projects">Projects</RouterLink>
          <RouterLink to="/settings">Settings</RouterLink>
        </div>
      </nav>
    </header>
    <main class="app-main">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.app-header {
  border-bottom: 1px solid var(--color-border, #e5e7eb);
  padding: 0 1rem;
}

.app-nav {
  display: flex;
  align-items: center;
  gap: 2rem;
  max-width: 1200px;
  margin: 0 auto;
  height: 3.5rem;
}

.app-brand {
  font-weight: 700;
  font-size: 1.25rem;
  text-decoration: none;
}

.nav-links {
  display: flex;
  gap: 1rem;
}

.nav-links a {
  text-decoration: none;
  padding: 0.25rem 0.5rem;
  border-radius: 0.25rem;
}

.app-main {
  flex: 1;
  max-width: 1200px;
  margin: 0 auto;
  padding: 1.5rem 1rem;
  width: 100%;
}
</style>
