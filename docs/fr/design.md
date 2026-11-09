# TunnelBar — Document de conception

> 🇬🇧 [English version](../en/design.md)

TunnelBar est une application de barre de menus macOS pour démarrer, arrêter
et surveiller des tunnels SSH. Ce document consigne les décisions de
conception prises avant le développement. Il sert de référence aux
contributeurs : toute modification de ces décisions doit mettre ce fichier à
jour dans la même pull request.

## 1. Objectifs

- Démarrer / arrêter un tunnel SSH configuré en un clic depuis la barre de menus.
- Voir d'un coup d'œil si chaque tunnel est actif, en connexion, en
  reconnexion ou en erreur.
- Se reconnecter automatiquement après une perte de réseau ou une mise en veille.
- Rester léger, sûr et prévisible : quitter l'app ferme tous les tunnels.

Tunnel typique (le cas d'usage d'origine) :

```sh
ssh -L 54321:127.0.0.1:5432 203.0.113.10 -N
```

### Hors périmètre (v1)

- Graphiques de trafic ou compteurs d'octets.
- Import des hôtes depuis `~/.ssh/config`.
- Saisie de mot de passe / passphrase (`SSH_ASKPASS`).
- Options `ssh` libres supplémentaires.

## 2. Identité

| Élément | Valeur |
|---|---|
| Nom | TunnelBar |
| Bundle ID | `io.github.gduprez.tunnelbar` |
| Dépôt | `github.com/gduprez/tunnelbar` (privé jusqu'à la v0.1.0, puis public) |
| Crates | `tunnelbar-core` (logique métier), `tunnelbar` (app Tauri) |
| Licence | `MIT OR Apache-2.0` |
| macOS minimum | 13 Ventura, binaire universel (Apple Silicon + Intel) |

## 3. Technologies

- **Tauri v2** pour l'icône de la barre de menus et le panneau.
- **Rust** pour toute la logique métier, dans la crate `tunnelbar-core`,
  indépendante de l'interface.
- **Svelte 5 + TypeScript + Vite** pour l'interface, qui reste « bête » :
  elle affiche l'état poussé par le core et lui renvoie les actions de
  l'utilisateur.
- **Tokio** pour la concurrence (déjà utilisé par Tauri).

## 4. Moteur SSH

TunnelBar lance le `/usr/bin/ssh` du système comme processus enfant. Il
respecte donc `~/.ssh/config`, les clés, l'agent, le trousseau macOS et
`known_hosts`, et se comporte exactement comme dans le terminal.

`ssh` est toujours lancé **avec un vecteur d'arguments, jamais via un shell**.
Options toujours ajoutées :

| Option | Pourquoi |
|---|---|
| `-N` | Aucune commande distante. |
| `-o ExitOnForwardFailure=yes` | Sortie immédiate si une redirection échoue (ex. port local occupé). |
| `-o ServerAliveInterval=15` `-o ServerAliveCountMax=3` | Détection d'une connexion morte en moins d'une minute. |
| `-o BatchMode=yes` | Jamais de blocage sur une question interactive. |
| `-o ConnectTimeout=15` | Échec rapide quand le serveur est injoignable. |
| `-o ControlMaster=no` `-o ControlPath=none` | Posséder la connexion : un client multiplexé quitterait aussitôt en confiant les redirections à un processus maître non surveillé. |
| `--` avant l'hôte | Quelle que soit sa forme, l'hôte ne peut pas être lu comme une option. |

Le panneau peut copier la commande équivalente, protégée pour un shell POSIX.

### Authentification

Par clé uniquement (clé sans passphrase, ou passphrase stockée dans
l'agent SSH / le trousseau macOS via `UseKeychain yes` et
`AddKeysToAgent yes`). Un échec d'authentification place le tunnel dans
l'état d'erreur `AuthFailed`.

### Clés d'hôte

- Par défaut : la politique `StrictHostKeyChecking` de l'utilisateur est
  respectée. Un hôte inconnu échoue avec `HostKeyUnknown` ; l'interface
  explique comment vérifier l'empreinte une fois dans un terminal et propose
  de copier la commande.
- Par tunnel, sur option explicite : `accept_new_host_key = true` ajoute
  `-o StrictHostKeyChecking=accept-new`.
- Une clé d'hôte **modifiée** (`HostKeyMismatch`) est toujours refusée et
  signalée par une alerte visible.

## 5. Configuration

Stockée en TOML dans `~/Library/Application Support/TunnelBar/tunnels.toml`,
créée avec les permissions `0600`, modifiable à la main et rechargée à chaud.

```toml
version = 1

[[tunnel]]
id = "prod"                 # identifiant stable
name = "Serveur prod"       # libellé affiché
host = "203.0.113.10"      # ou un alias de ~/.ssh/config
# user = "deploy"           # facultatif
# port = 22                 # facultatif
autostart = false           # démarrer au lancement de TunnelBar
auto_reconnect = true
accept_new_host_key = false

[[tunnel.forward]]
type = "local"              # -L
local_port = 54321
remote_host = "127.0.0.1"
remote_port = 5432

# [[tunnel.forward]]
# type = "remote"           # -R
# remote_port = 8080
# local_host = "127.0.0.1"
# local_port = 3000

# [[tunnel.forward]]
# type = "dynamic"          # -D (SOCKS)
# local_port = 1080
```

- Un tunnel = une connexion SSH portant une ou plusieurs redirections ; il
  démarre et s'arrête d'un bloc.
- Le modèle gère les redirections `local`, `remote` et `dynamic`. Le
  formulaire de la v1 n'édite que les redirections `local` ; les autres se
  modifient dans le fichier et sont correctement affichées.

### Règles de validation

- Ports compris dans `1..=65535` ; deux tunnels ne peuvent pas utiliser le
  même port local.
- Les `id` sont uniques.
- `host` et `user` ne doivent pas commencer par `-`, ni contenir d'espaces ou
  de caractères de contrôle (empêche l'injection d'options comme
  `-oProxyCommand=…`).
- Pas d'options `ssh` libres ; les réglages avancés passent par un alias de
  `~/.ssh/config`.

## 6. Cycle de vie d'un tunnel

```
Arrêté ──start──► Connexion ──port ouvert──► Actif
   ▲                  │                        │ connexion perdue
   │ stop             ▼                        ▼
   └────────────── Erreur ◄── fatale ── Reconnexion (tentative n)
```

- **Actif** = le processus `ssh` tourne **et** chaque port local (redirections
  `local` et `dynamic`) accepte les connexions TCP. Les ports sont testés
  toutes les 0,5 s **pendant la connexion uniquement** : tester un tunnel
  actif ouvrirait à chaque fois une connexion vers le service distant (bruit
  dans ses journaux) ; une fois actif, les keepalives de `ssh` détectent une
  connexion morte. Un tunnel qui n'a que des redirections distantes est actif
  quand son processus a vécu 5 s.
- Une connexion dont les ports sont toujours fermés au bout de 30 s est tuée
  puis relancée (`ConnectionTimedOut`).
- **Port local déjà pris** : avant chaque démarrage, chaque port local est
  testé ; s'il accepte déjà des connexions, il appartient à un autre programme
  et le tunnel échoue (`PortInUse`) sans lancer `ssh`. Sinon, le test de port
  répondrait pour ce programme et afficherait le tunnel actif. De même, une
  ligne `bind …: Address already in use` arrête `ssh` et fait échouer le
  tunnel : `ssh` continue de tourner s'il écoute sur `[::1]` mais pas sur
  `127.0.0.1` (ou l'inverse), car `ExitOnForwardFailure` ne joue que si aucune
  adresse n'a pu être ouverte.
- **Reconnexion** (`auto_reconnect = true` par défaut) : délai exponentiel
  1 s, 2 s, 4 s… plafonné à 60 s. Les erreurs fatales (`AuthFailed`,
  `PortInUse`, `HostKeyMismatch`, `HostKeyUnknown`) arrêtent les tentatives.
- Au retour de veille ou lors d'un changement de réseau, les tunnels en
  reconnexion réessaient immédiatement. Les deux sont détectés en interrogeant
  le système toutes les 5 s, sans API de notification : un réveil se voit à
  l'horloge murale qui avance plus que l'horloge monotone (arrêtée pendant la
  veille), un changement de réseau à un ensemble d'adresses IP locales différent.
- Une notification macOS s'affiche quand un tunnel tombe définitivement.

### Cycle de vie de l'application

- Barre de menus uniquement : pas d'icône dans le Dock ni dans Cmd+Tab.
- Réglage « Lancer au démarrage » (désactivé par défaut), via `SMAppService`.
- `autostart` par tunnel (désactivé par défaut).
- Quitter l'app arrête tous les processus `ssh`. Les PID et heures de
  démarrage des processus `ssh` en cours sont enregistrés dans `ssh.pids` ; au
  lancement, un processus enregistré n'est tué que s'il est orphelin (son
  parent n'est plus un exécutable `tunnelbar` : launchd sur macOS, PID 1 ou
  `systemd --user` sous Linux), que sa ligne de commande commence par les
  options `ssh` exactes de TunnelBar et qu'il a démarré à l'heure enregistrée.
  Un PID recyclé ne touche jamais un processus sans rapport, ni les tunnels d'un
  TunnelBar encore en cours.
- Sous Linux, chaque `ssh` reçoit aussi un signal de mort du parent
  (`PR_SET_PDEATHSIG`), depuis un thread de lancement dédié : il s'arrête avec
  TunnelBar, même en cas de plantage.
- Le lancement à l'ouverture de session utilise `SMAppService` sur macOS et une
  entrée d'autostart XDG sous Linux. `unsafe` n'est autorisé que dans deux
  modules : `src-tauri/src/login_item/macos.rs` (appels Objective-C) et
  `crates/tunnelbar-core/src/linux.rs` (`pre_exec`).

## 7. Interface

**Menu de l'icône** (clic droit) pour l'accès rapide :

```
● Serveur prod         ✓   ← clic = démarrer/arrêter
○ Redis staging
⚠ Bastion client X
─────────────
Ouvrir le panneau…
Quitter
```

L'icône est une image « template » qui reflète l'état global : aucun tunnel
actif, tous actifs, au moins un en erreur.

**Panneau** (clic gauche), ancré sous l'icône :

- Le panneau prend la hauteur de son contenu (de 160 à 640 px, bornée côté
  Rust) ; le pied de la liste porte **Nouveau tunnel** et les réglages.
- Liste des tunnels : pastille d'état, nom, résumé
  (`localhost:54321 → 127.0.0.1:5432 via 203.0.113.10`), interrupteur et
  menu (modifier, dupliquer, supprimer, copier la commande `ssh` équivalente).
- Détail d'un tunnel : durée d'activité, nombre de reconnexions, dernière
  erreur (traduite), 50 dernières lignes de sortie `ssh` avec bouton Copier.
- Formulaire d'ajout / modification avec validation en direct par le core.
- Réglages : lancer au démarrage, langue (auto / FR / EN), ouvrir le fichier
  de configuration.

Les journaux sont écrits dans `~/Library/Logs/io.github.gduprez.tunnelbar/` avec rotation.

## 8. Internationalisation

- L'interface suit la langue de macOS ; l'anglais est la langue de secours.
- Les traductions sont dans `locales/<langue>.json` ; ajouter une langue
  revient à ajouter un fichier.
- Le core ne renvoie jamais de texte affichable : il renvoie des erreurs
  typées (`PortInUse { port }`, `AuthFailed`, …) que l'interface traduit.
- Un contrôle de CI échoue si une langue a des clés manquantes ou en trop par
  rapport à `en.json`.
- Toute la documentation existe en anglais et en français (`docs/en`,
  `docs/fr`, `README.md` / `README.fr.md`, `CONTRIBUTING`).

## 9. Architecture

```
          commandes (start/stop/reload)
UI ──► commande Tauri ──► TunnelManager ──► Supervisor(tunnel A) ──► ProcessSpawner ──► ssh
                              │             Supervisor(tunnel B)        PortProber
                              │                     │                   Clock (temps tokio)
                              ◄──── événements (broadcast) ◄─┘
UI ◄── emit("tunnel-state") ◄─┘
```

- **`TunnelManager`** : détient la configuration chargée, démarre et arrête
  les superviseurs, refuse les conflits de ports.
- **`Supervisor`** (une tâche Tokio par tunnel actif) : lance `ssh`, teste
  les ports, lit stderr, applique le délai de reconnexion, publie les
  changements d'état.
- **Machine à états** : une fonction pure `(état, événement) -> (état, actions)`,
  sans entrée/sortie.
- **Analyseur d'erreurs `ssh`** : une fonction pure qui transforme le stderr
  brut en erreurs typées (`AuthFailed`, `HostUnreachable`, `PortInUse`,
  `HostKeyUnknown`, `HostKeyMismatch`, …).
- **Ports injectés** (traits) pour chaque effet de bord : `ProcessSpawner`,
  `PortProber`, `ConfigStore`. Le temps est contrôlé en test par l'horloge en
  pause de Tokio.

## 10. Processus de développement

### TDD

Rouge → vert → refactorisation, par petites étapes, chacune terminée par un
commit ([Conventional Commits](https://www.conventionalcommits.org/) :
`test:`, `feat:`, `fix:`, `refactor:`…).

Pyramide de tests :

1. Tests unitaires de `tunnelbar-core` (la majorité) : modèle, validation,
   construction de la commande, machine à états, analyseur d'erreurs.
2. Tests d'intégration du core avec faux processus et faux `ssh` scripté.
3. Tests fins de la couche de commandes Tauri.
4. Tests de composants de l'interface (Vitest + Testing Library, API Tauri
   simulée).
5. Test de bout en bout facultatif contre un `sshd` local, lancé à la demande.

### CI (GitHub Actions, runner macOS)

Sur chaque pull request et chaque push sur `main` (tout est bloquant) :

- `cargo fmt --check`, `prettier --check`
- `cargo clippy -- -D warnings`, `svelte-check`, ESLint
- `cargo nextest run`
- `vitest run`
- contrôle des clés de traduction
- `cargo llvm-cov` sur `tunnelbar-core`, couverture de lignes minimale 85 %
- `cargo deny` (licences, vulnérabilités)
- `cargo tauri build`

Sur un tag `v*` : `.dmg` universel, GitHub Release avec notes générées
(`release-please`, SemVer), mise à jour du cask dans `gduprez/homebrew-tap`.

`main` est protégée : fusion uniquement avec une CI au vert. Les modèles
d'issues et de PR existent en anglais et en français.

### Distribution

- Pas d'identifiant Apple Developer pour l'instant : le `.dmg` est signé
  ad hoc. La documentation explique comment ouvrir l'app la première fois
  (clic droit → Ouvrir, ou `xattr -dr com.apple.quarantine`).
- Installation via le tap personnel `gduprez/homebrew-tap`, ou compilation
  depuis les sources (`cargo tauri build`).
- Le pipeline est conçu pour que la signature Developer ID et la
  notarisation puissent être activées plus tard en ajoutant seulement des
  secrets.

## 11. Jalons

| # | Jalon | Résultat |
|---|---|---|
| 0 | Squelette : workspace, Tauri + Svelte, CI complète, licences, README minimal | Icône vide dans la barre de menus |
| 1 | Modèle, validation (dont règles de sécurité), lecture/écriture TOML, `0600` | — |
| 2 | `Tunnel` → vecteur d'arguments `ssh` | — |
| 3 | Machine à états et délai de reconnexion (pure) | — |
| 4 | Analyseur du stderr `ssh` | — |
| 5 | Superviseur et gestionnaire (acteurs Tokio, faux, faux `ssh` scripté) | — |
| 6 | Commandes Tauri, menu de l'icône avec marche/arrêt, état de l'icône | **Premier usage réel** |
| 7 | Panneau : liste, détail, logs, formulaire, FR/EN | Interface complète |
| 8 | Lancement au démarrage, autostart, veille/réseau, notifications, nettoyage des orphelins | — |
| 9 | Release : `.dmg`, tap Homebrew, docs FR/EN complètes, CONTRIBUTING | **v0.1.0, publique** |
