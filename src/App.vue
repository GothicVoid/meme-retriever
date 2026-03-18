<template>
  <nav class="nav">
    <RouterLink to="/">
      搜索
    </RouterLink>
    <RouterLink to="/library">
      图库
    </RouterLink>
    <RouterLink to="/settings">
      设置
    </RouterLink>
  </nav>
  <RouterView />
</template>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  color: #0f0f0f;
  background-color: #f6f6f6;
}
@media (prefers-color-scheme: dark) {
  :root { color: #f6f6f6; background-color: #2f2f2f; }
}
* { box-sizing: border-box; margin: 0; padding: 0; }
</style>

<style scoped>
.nav {
  display: flex;
  gap: 1rem;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #eee;
}
.nav a { text-decoration: none; color: #555; }
.nav a.router-link-active { color: #000; font-weight: 600; }
</style>
