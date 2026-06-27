<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { AppEntry, Progress, Settings } from "./lib/types";
  import AppRow from "./lib/AppRow.svelte";
  import SettingsPanel from "./lib/SettingsPanel.svelte";

  let apps = $state<AppEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let query = $state("");
  let showSettings = $state(false);
  let settings = $state<Settings>({ username: "russus87", auto_check_minutes: 60 });
  let progress = $state<Record<string, Progress>>({});
  let lastRefresh = $state<string>("");

  const filtered = $derived(
    apps
      .filter((a) => {
        const q = query.trim().toLowerCase();
        if (!q) return true;
        return (
          a.display_name.toLowerCase().includes(q) ||
          a.description.toLowerCase().includes(q)
        );
      })
      .sort((a, b) => {
        // updatable first, then new, then installed, then the rest
        const rank = (x: AppEntry) =>
          x.status === "update_available"
            ? 0
            : x.is_new
              ? 1
              : x.status === "installed"
                ? 2
                : 3;
        const r = rank(a) - rank(b);
        return r !== 0 ? r : a.display_name.localeCompare(b.display_name);
      }),
  );

  const counts = $derived({
    updates: apps.filter((a) => a.status === "update_available").length,
    installed: apps.filter(
      (a) => a.status === "installed" || a.status === "update_available",
    ).length,
    nuovi: apps.filter((a) => a.is_new).length,
  });

  async function refresh(force = false) {
    loading = true;
    error = null;
    try {
      apps = await invoke<AppEntry[]>("list_apps", { force });
      lastRefresh = new Date().toLocaleTimeString();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function action(app: AppEntry) {
    try {
      await invoke("install_or_update", { name: app.name });
    } catch (e) {
      error = String(e);
    }
  }

  async function uninstall(app: AppEntry) {
    try {
      await invoke("uninstall_app", { name: app.name });
      await refresh(false);
    } catch (e) {
      error = String(e);
    }
  }

  async function launch(app: AppEntry) {
    try {
      await invoke("launch_app", { name: app.name });
    } catch (e) {
      error = String(e);
    }
  }

  async function saveSettings(s: Settings) {
    settings = s;
    await invoke("save_settings", { settings: s });
    showSettings = false;
    await refresh(true);
  }

  onMount(async () => {
    try {
      settings = await invoke<Settings>("get_settings");
    } catch {}
    await refresh(false);

    await listen<Progress>("progress", (e) => {
      const p = e.payload;
      progress = { ...progress, [p.app]: p };
      if (p.phase === "done") {
        // give the row a moment to show "done", then refresh state
        setTimeout(() => {
          const { [p.app]: _, ...rest } = progress;
          progress = rest;
          refresh(false);
        }, 900);
      }
    });

    await listen("apps-changed", () => refresh(false));
  });

  function hide() {
    getCurrentWindow().hide();
  }
</script>

<div class="window">
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <img src="/assets/icon.svg" alt="" class="logo" />
      <div class="brand-text" data-tauri-drag-region>
        <strong>Russus Launcher</strong>
        <span class="sub">
          {#if counts.updates > 0}
            {counts.updates} aggiornament{counts.updates === 1 ? "o" : "i"} disponibil{counts.updates === 1 ? "e" : "i"}
          {:else}
            {counts.installed} installat{counts.installed === 1 ? "a" : "e"} · tutto aggiornato
          {/if}
        </span>
      </div>
    </div>
    <div class="actions">
      <button class="icon-btn" title="Aggiorna elenco" onclick={() => refresh(true)} aria-label="refresh">
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class:spin={loading}>
          <path d="M21 12a9 9 0 1 1-2.64-6.36" /><polyline points="21 3 21 9 15 9" />
        </svg>
      </button>
      <button class="icon-btn" title="Impostazioni" onclick={() => (showSettings = true)} aria-label="settings">
        <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>
      </button>
      <button class="icon-btn close" title="Nascondi" onclick={hide} aria-label="hide">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg>
      </button>
    </div>
  </header>

  <div class="searchbar">
    <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" /></svg>
    <input placeholder="Cerca tra i tuoi progetti…" bind:value={query} />
    {#if counts.nuovi > 0}
      <span class="pill new-pill">{counts.nuovi} nuov{counts.nuovi === 1 ? "o" : "i"}</span>
    {/if}
  </div>

  <main class="list">
    {#if error}
      <div class="banner err">
        <span>{error}</span>
        <button onclick={() => refresh(true)}>Riprova</button>
      </div>
    {/if}

    {#if loading && apps.length === 0}
      <div class="empty">
        <div class="spinner"></div>
        <p>Controllo i progetti su GitHub…</p>
      </div>
    {:else if filtered.length === 0}
      <div class="empty">
        <p>Nessun progetto trovato.</p>
      </div>
    {:else}
      {#each filtered as app (app.name)}
        <AppRow
          {app}
          progress={progress[app.name]}
          onaction={() => action(app)}
          onuninstall={() => uninstall(app)}
          onlaunch={() => launch(app)}
          onopen={() => openUrl(app.html_url)}
        />
      {/each}
    {/if}
  </main>

  <footer class="statusbar">
    <span>@{settings.username}</span>
    <span class="dot">·</span>
    <span>{apps.length} repo</span>
    {#if lastRefresh}
      <span class="dot">·</span>
      <span>agg. {lastRefresh}</span>
    {/if}
  </footer>

  {#if showSettings}
    <SettingsPanel
      {settings}
      onsave={saveSettings}
      oncancel={() => (showSettings = false)}
    />
  {/if}
</div>

<style>
  .window {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 14px;
    overflow: hidden;
    box-shadow: var(--shadow);
  }

  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 12px 12px 14px;
    background: linear-gradient(180deg, #191d2a, #14171f);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .logo {
    width: 30px;
    height: 30px;
    border-radius: 8px;
  }
  .brand-text {
    display: flex;
    flex-direction: column;
    line-height: 1.25;
  }
  .brand-text strong {
    font-size: 13.5px;
    letter-spacing: 0.2px;
  }
  .sub {
    font-size: 11px;
    color: var(--text-dim);
  }
  .actions {
    display: flex;
    gap: 2px;
  }
  .icon-btn {
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    color: var(--text-dim);
    transition: background 0.12s, color 0.12s;
  }
  .icon-btn:hover {
    background: var(--bg-row-hover);
    color: var(--text);
  }
  .icon-btn.close:hover {
    background: #3a2230;
    color: var(--red);
  }
  .spin {
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .searchbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 12px;
    margin: 10px 12px 4px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 10px;
    color: var(--text-faint);
  }
  .searchbar input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: var(--text);
    font-size: 13px;
  }
  .searchbar input::placeholder {
    color: var(--text-faint);
  }
  .pill {
    font-size: 10.5px;
    padding: 2px 8px;
    border-radius: 20px;
    font-weight: 600;
  }
  .new-pill {
    background: rgba(108, 124, 255, 0.18);
    color: #aab4ff;
  }

  .list {
    flex: 1;
    overflow-y: auto;
    padding: 6px 8px 10px;
  }

  .banner {
    margin: 4px 6px 10px;
    padding: 10px 12px;
    border-radius: 10px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    font-size: 12.5px;
  }
  .banner.err {
    background: rgba(255, 93, 115, 0.12);
    color: #ffb0bb;
    border: 1px solid rgba(255, 93, 115, 0.25);
  }
  .banner button {
    color: #fff;
    background: rgba(255, 255, 255, 0.1);
    padding: 4px 10px;
    border-radius: 7px;
    font-size: 12px;
  }

  .empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    color: var(--text-faint);
  }
  .spinner {
    width: 26px;
    height: 26px;
    border: 3px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .statusbar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    font-size: 11px;
    color: var(--text-faint);
    border-top: 1px solid var(--border);
    background: #12141c;
  }
  .dot {
    opacity: 0.5;
  }
</style>
