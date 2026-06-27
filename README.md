<div align="center">
  <img src="assets/icon.svg" width="120" alt="Russus Launcher" />
  <h1>Russus Launcher</h1>
  <p><em>Un launcher in stile JetBrains Toolbox per le tue app pubblicate su GitHub.</em></p>
  <p>Vive nell'area di notifica, controlla le release dei tuoi repo e installa / aggiorna con un click.</p>
</div>

---

## Cos'è

Russus Launcher legge le **release pubbliche** del tuo profilo GitHub e si comporta come Toolbox:

- 📦 **Elenca** i tuoi progetti che hanno una release con un pacchetto installabile.
- ⬇️ **Installa** scaricando l'asset giusto per la piattaforma:
  - **Arch Linux** → `*.pkg.tar.zst` via `pacman -U` (con prompt grafico `pkexec`/polkit).
  - **macOS** → `*.dmg` montato e copiato in `/Applications`.
- ⬆️ **Aggiorna** quando esce una nuova versione (confronto versione locale ↔ ultima release).
- 🆕 **Segnala i progetti nuovi**: quando compare un repo mai visto prima, ricevi una notifica.
- 🔔 **Vive nel tray**: click sull'icona per aprire/chiudere il pannello; controllo periodico in background.
- 🚀 **Avvio automatico** al login.

È pensato per chi pubblica tante app (es. tutto lo stack Tauri: `oxiterm`, `oops`, `charon`, `archmind`, …)
e vuole un unico posto da cui tenerle installate e aggiornate.

## Come funziona il rilevamento

Per ogni repo viene letta l'ultima release **non draft / non prerelease**. Il pacchetto è scelto così:

| Piattaforma | Asset cercato            | Installazione        |
|-------------|--------------------------|----------------------|
| Arch Linux  | `*-x86_64.pkg.tar.zst`   | `pkexec pacman -U`   |
| macOS arm   | `*aarch64*.dmg`          | copia in `/Applications` |
| macOS Intel | `*x64*.dmg` / qualsiasi `.dmg` | copia in `/Applications` |

La versione installata su Arch è letta da `pacman -Q`, quindi lo stato resta corretto anche se
installi/disinstalli da fuori dal launcher.

## Sviluppo

```bash
npm install
npm run tauri dev      # dev con hot-reload
npm run tauri build    # build locale
```

Stack: **Tauri 2 + Svelte 5 + TypeScript** (frontend) e **Rust** (backend).

### Autenticazione GitHub

Le release pubbliche si leggono senza token. Per alzare il rate limit dell'API, il launcher riusa
automaticamente la sessione della **GitHub CLI** (`gh auth token`) se presente, oppure la variabile
`GITHUB_TOKEN`.

### Impostazioni

Dal pannello (icona ingranaggio):

- **Utente / organizzazione GitHub** — di default `russus87`.
- **Controllo automatico** — intervallo del check in background (off / 30m / 1h / 6h / giornaliero).

## Build & distribuzione (CI)

Il workflow [`.github/workflows/build.yml`](.github/workflows/build.yml) parte su tag `v*` e produce:

- **Arch Linux**: `russus-launcher-<ver>-1-x86_64.pkg.tar.zst` (in un container `archlinux`, via `makepkg`).
- **macOS**: `.dmg` + `.app.tar.gz` per **Apple Silicon** e **Intel**.

Gli artefatti vengono allegati automaticamente alla GitHub Release del tag.

```bash
# pubblicare una nuova versione
git tag v0.1.0
git push origin v0.1.0
```

## Note sui window manager tiling (niri/Sway/Hyprland)

La finestra è senza decorazioni e `always-on-top`. Su un compositor *tiling* potrebbe essere
agganciata al layout: aggiungi una regola per farla fluttuare, es. su **niri**:

```kdl
window-rule {
    match app-id="com.russus.launcher"
    open-floating true
}
```

## Licenza

[MIT](LICENSE)
