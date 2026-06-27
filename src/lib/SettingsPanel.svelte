<script lang="ts">
  import type { Settings } from "./types";

  let {
    settings,
    onsave,
    oncancel,
  }: {
    settings: Settings;
    onsave: (s: Settings) => void;
    oncancel: () => void;
  } = $props();

  let username = $state(settings.username);
  let auto = $state(settings.auto_check_minutes);
</script>

<div
  class="overlay"
  onclick={oncancel}
  onkeydown={(e) => e.key === "Escape" && oncancel()}
  role="button"
  tabindex="-1"
>
  <div class="panel" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
    <h2>Impostazioni</h2>

    <label>
      <span>Utente / organizzazione GitHub</span>
      <input bind:value={username} placeholder="russus87" spellcheck="false" />
    </label>

    <label>
      <span>Controllo automatico aggiornamenti</span>
      <select bind:value={auto}>
        <option value={0}>Disattivato</option>
        <option value={30}>Ogni 30 minuti</option>
        <option value={60}>Ogni ora</option>
        <option value={360}>Ogni 6 ore</option>
        <option value={1440}>Una volta al giorno</option>
      </select>
    </label>

    <p class="hint">
      Il launcher legge le release pubbliche del profilo e installa il pacchetto
      adatto alla piattaforma (<code>.pkg.tar.zst</code> su Arch, <code>.dmg</code> su macOS).
    </p>

    <div class="buttons">
      <button class="ghost" onclick={oncancel}>Annulla</button>
      <button
        class="primary"
        onclick={() => onsave({ username: username.trim() || "russus87", auto_check_minutes: auto })}
      >
        Salva
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(6, 8, 12, 0.6);
    backdrop-filter: blur(2px);
    display: grid;
    place-items: center;
    z-index: 20;
  }
  .panel {
    width: 320px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 18px;
    box-shadow: var(--shadow);
  }
  h2 {
    margin: 0 0 14px;
    font-size: 15px;
  }
  label {
    display: block;
    margin-bottom: 14px;
  }
  label span {
    display: block;
    font-size: 11.5px;
    color: var(--text-dim);
    margin-bottom: 6px;
  }
  input,
  select {
    width: 100%;
    padding: 9px 10px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 9px;
    color: var(--text);
    font-size: 13px;
    outline: none;
  }
  input:focus,
  select:focus {
    border-color: var(--accent);
  }
  .hint {
    font-size: 11px;
    color: var(--text-faint);
    line-height: 1.5;
    margin: 2px 0 16px;
  }
  .hint code {
    background: var(--bg-row);
    padding: 1px 5px;
    border-radius: 5px;
    font-size: 10.5px;
  }
  .buttons {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
  .buttons button {
    padding: 8px 16px;
    border-radius: 9px;
    font-size: 12.5px;
    font-weight: 600;
  }
  .ghost {
    background: var(--bg-row-hover);
    color: var(--text);
  }
  .primary {
    background: linear-gradient(135deg, var(--accent), #5566f0);
    color: #fff;
  }
</style>
