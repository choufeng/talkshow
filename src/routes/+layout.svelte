<script lang="ts">
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';

  let activeMenu = $derived(
    $page.url.pathname === '/settings' ? 'settings' :
    $page.url.pathname === '/models' ? 'models' : 'home'
  );

  function navigateTo(path: string) {
    goto(path);
  }
</script>

<div class="app-container">
  <aside class="sidebar">
    <div class="logo">TalkShow</div>
    <nav class="menu">
      <button
        class="menu-item"
        class:active={activeMenu === 'home'}
        onclick={() => navigateTo('/')}
      >
        <span class="icon">🏠</span>
        <span class="label">首页</span>
      </button>
      <button
        class="menu-item"
        class:active={activeMenu === 'models'}
        onclick={() => navigateTo('/models')}
      >
        <span class="icon">🤖</span>
        <span class="label">模型</span>
      </button>
      <button
        class="menu-item"
        class:active={activeMenu === 'settings'}
        onclick={() => navigateTo('/settings')}
      >
        <span class="icon">⚙️</span>
        <span class="label">设置</span>
      </button>
    </nav>
  </aside>
  <main class="content">
    <slot />
  </main>
</div>

<style>
  .app-container {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .sidebar {
    width: 160px;
    background-color: #f5f5f5;
    border-right: 1px solid #e0e0e0;
    display: flex;
    flex-direction: column;
  }

  .logo {
    padding: 16px 20px;
    font-weight: 600;
    font-size: 14px;
    color: #333;
    border-bottom: 1px solid #e0e0e0;
  }

  .menu {
    padding: 8px 0;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 20px;
    width: 100%;
    border: none;
    background: none;
    cursor: pointer;
    font-size: 14px;
    color: #333;
    text-align: left;
  }

  .menu-item:hover {
    background-color: #e8e8e8;
  }

  .menu-item.active {
    background-color: #e8e8e8;
    border-left: 3px solid #396cd8;
  }

  .icon {
    font-size: 16px;
  }

  .content {
    flex: 1;
    padding: 24px;
    overflow-y: auto;
    background-color: #fff;
  }
</style>