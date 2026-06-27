<script lang="ts">
  import type { AppEntry, Progress } from "./types";

  let {
    app,
    progress,
    onaction,
    onuninstall,
    onlaunch,
    onopen,
  }: {
    app: AppEntry;
    progress?: Progress;
    onaction: () => void;
    onuninstall: () => void;
    onlaunch: () => void;
    onopen: () => void;
  } = $props();

  // deterministic colour from the repo name, for the avatar
  function hue(s: string) {
    let h = 0;
    for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
    return h;
  }
  const h = hue(app.name);
  const initials = app.display_name
    .replace(/[^a-zA-Z0-9 ]/g, "")
    .split(" ")
    .map((w) => w[0])
    .slice(0, 2)
    .join("")
    .toUpperCase();

  const busy = $derived(
    !!progress && progress.phase !== "done" && progress.phase !== "error",
  );

  const label = $derived(
    app.status === "update_available"
      ? "Aggiorna"
      : app.status === "installed"
        ? "Apri"
        : app.status === "unsupported"
          ? "Non disp."
          : "Installa",
  );
</script>

<div class="row" class:new={app.is_new}>
  <div class="avatar" style="background: hsl({h} 55% 22%); color: hsl({h} 80% 78%)">
    {initials || "?"}
  </div>

  <div class="info">
    <div class="line1">
      <span class="name">{app.display_name}</span>
      {#if app.status === "installed"}
        <span class="chip ok">v{app.installed_version}</span>
      {:else if app.status === "update_available"}
        <span class="chip upd">{app.installed_version} → {app.latest_version}</span>
      {:else if app.latest_version}
        <span class="chip muted">{app.latest_version}</span>
      {/if}
      {#if app.is_new}<span class="chip new-chip">NUOVO</span>{/if}
    </div>
    <div class="desc">{app.description || "—"}</div>
    {#if busy && progress}
      <div class="prog">
        <div class="bar">
          <div
            class="fill"
            class:indeterminate={progress.percent < 0}
            style={progress.percent >= 0 ? `width:${progress.percent}%` : ""}
          ></div>
        </div>
        <span class="prog-label">{progress.message}</span>
      </div>
    {/if}
  </div>

  <div class="row-actions">
    {#if app.status === "unsupported"}
      <button class="btn ghost" onclick={onopen}>GitHub</button>
    {:else if busy}
      <button class="btn busy" disabled>…</button>
    {:else if app.status === "installed"}
      <button class="btn ghost small" title="Apri su GitHub" onclick={onopen} aria-label="github">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.58 2 12.25c0 4.53 2.87 8.37 6.84 9.73.5.09.68-.22.68-.49v-1.7c-2.78.62-3.37-1.22-3.37-1.22-.45-1.18-1.11-1.5-1.11-1.5-.91-.64.07-.62.07-.62 1 .07 1.53 1.06 1.53 1.06.9 1.57 2.36 1.12 2.94.85.09-.66.35-1.12.63-1.38-2.22-.26-4.56-1.14-4.56-5.06 0-1.12.39-2.03 1.03-2.75-.1-.26-.45-1.3.1-2.71 0 0 .84-.28 2.75 1.05a9.4 9.4 0 0 1 5 0c1.91-1.33 2.75-1.05 2.75-1.05.55 1.41.2 2.45.1 2.71.64.72 1.03 1.63 1.03 2.75 0 3.93-2.34 4.79-4.57 5.05.36.32.68.94.68 1.9v2.81c0 .27.18.59.69.49A10.26 10.26 0 0 0 22 12.25C22 6.58 17.52 2 12 2z" /></svg>
      </button>
      <button class="btn ghost small" title="Disinstalla" onclick={onuninstall} aria-label="uninstall">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="3 6 5 6 21 6" /><path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" /></svg>
      </button>
      <button class="btn primary" onclick={onlaunch}>{label}</button>
    {:else}
      <button
        class="btn"
        class:primary={app.status === "update_available"}
        class:outline={app.status === "not_installed"}
        onclick={onaction}
      >
        {label}
      </button>
    {/if}
  </div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 11px 12px;
    border-radius: 11px;
    transition: background 0.12s;
  }
  .row:hover {
    background: var(--bg-row);
  }
  .row.new {
    background: linear-gradient(90deg, rgba(108, 124, 255, 0.07), transparent 60%);
  }

  .avatar {
    width: 40px;
    height: 40px;
    flex-shrink: 0;
    border-radius: 11px;
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 14px;
    letter-spacing: 0.3px;
  }

  .info {
    flex: 1;
    min-width: 0;
  }
  .line1 {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .name {
    font-weight: 600;
    font-size: 13.5px;
    white-space: nowrap;
  }
  .desc {
    color: var(--text-dim);
    font-size: 12px;
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 320px;
  }

  .chip {
    font-size: 10px;
    padding: 1.5px 7px;
    border-radius: 20px;
    font-weight: 600;
    white-space: nowrap;
  }
  .chip.ok {
    background: rgba(63, 208, 122, 0.14);
    color: var(--green);
  }
  .chip.upd {
    background: rgba(255, 180, 84, 0.16);
    color: var(--amber);
  }
  .chip.muted {
    background: var(--bg-row-hover);
    color: var(--text-faint);
  }
  .chip.new-chip {
    background: rgba(108, 124, 255, 0.2);
    color: #aab4ff;
  }

  .prog {
    margin-top: 7px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .bar {
    flex: 1;
    height: 5px;
    background: var(--bg-row-hover);
    border-radius: 4px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--accent-2));
    border-radius: 4px;
    transition: width 0.2s;
  }
  .fill.indeterminate {
    width: 35%;
    animation: slide 1.1s ease-in-out infinite;
  }
  @keyframes slide {
    0% {
      margin-left: -35%;
    }
    100% {
      margin-left: 100%;
    }
  }
  .prog-label {
    font-size: 10.5px;
    color: var(--text-faint);
    white-space: nowrap;
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
  }
  .btn {
    font-size: 12px;
    font-weight: 600;
    padding: 7px 14px;
    border-radius: 9px;
    background: var(--bg-row-hover);
    color: var(--text);
    transition: filter 0.12s, background 0.12s;
  }
  .btn:hover {
    filter: brightness(1.15);
  }
  .btn.primary {
    background: linear-gradient(135deg, var(--accent), #5566f0);
    color: #fff;
  }
  .btn.outline {
    background: transparent;
    border: 1px solid var(--accent);
    color: #aab4ff;
  }
  .btn.busy {
    opacity: 0.6;
    cursor: default;
  }
  .btn.ghost {
    background: transparent;
    color: var(--text-dim);
  }
  .btn.ghost:hover {
    background: var(--bg-row-hover);
    color: var(--text);
  }
  .btn.small {
    padding: 7px 8px;
  }
</style>
