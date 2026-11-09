# Plan : TunnelBar pour Ubuntu

> Statut : **en cours de réalisation** sur la branche `feat/linux` (décision du
> 9 novembre 2026). `main` et la version macOS ne sont pas touchées tant que le
> port n'est pas prêt.

## 1. Objectif et périmètre

Offrir TunnelBar aux utilisateurs d'**Ubuntu Desktop**, avec les mêmes
fonctionnalités que sur macOS : démarrer, arrêter et surveiller des tunnels SSH
depuis la barre du haut, reconnexion automatique, notifications, lancement à
l'ouverture de session.

| Cible | Niveau de support |
|---|---|
| Ubuntu 24.04 LTS et 26.04 LTS, GNOME (Wayland et X11), **amd64** | **Supporté et testé** |
| Les mêmes en arm64 | Paquets publiés, testés en CI uniquement |
| Autres dérivés Debian/Ubuntu avec un indicateur de zone de notification (Kubuntu, Xubuntu, Linux Mint) | Best effort, non testé en CI |
| Autres distributions, Flatpak, Snap | Hors périmètre de la première version |

Hors périmètre : nouvelles fonctionnalités. Le port reprend la v0.2.x telle
quelle ; les évolutions viendront ensuite pour les deux systèmes à la fois.

## 2. Point de départ mesuré

Tests réalisés le 3 novembre 2026 dans des conteneurs Ubuntu 24.04 (**arm64**,
Rust 1.99.1, OpenSSH 9.6, procps 4.0.4). L'architecture de référence étant
amd64, le jalon L0 refait ces mesures en amd64 :

| Élément | Résultat |
|---|---|
| `tunnelbar-core` : 129 tests, dont vrais processus `ssh` simulés, ports, `ps` | ✅ tous passent |
| `tunnelbar-core` : clippy | ✅ propre |
| `tunnelbar` (app Tauri) : compilation | ❌ une seule erreur : `login_item.rs` (bibliothèque macOS) |
| `tunnelbar` avec ce module remplacé par un bouchon | ✅ compile, 35 tests passent |
| Nettoyage des orphelins avec un « subreaper » (comportement de `systemd --user`) | ⚠️ l'orphelin a pour parent le subreaper, pas le PID 1 : le critère actuel ne le reconnaît pas (sans danger, mais inopérant) |

Conclusion : **le domaine est déjà portable** ; le travail porte sur
l'intégration au bureau et sur la distribution.

## 3. Ce qui change, point par point

### 3.1 Emplacements des fichiers (faible)

| Donnée | macOS | Ubuntu (spécification XDG) |
|---|---|---|
| Tunnels | `~/Library/Application Support/TunnelBar/tunnels.toml` | `$XDG_CONFIG_HOME/tunnelbar/tunnels.toml` (défaut `~/.config/tunnelbar/`) |
| Réglages | `…/TunnelBar/settings.toml` | `~/.config/tunnelbar/settings.toml` |
| PID des `ssh` | `…/TunnelBar/ssh.pids` | `$XDG_STATE_HOME/tunnelbar/ssh.pids` (défaut `~/.local/state/tunnelbar/`) |
| Journaux | `~/Library/Logs/io.github.gduprez.tunnelbar/` | `$XDG_STATE_HOME/tunnelbar/logs/` (via le plugin de logs de Tauri) |

`default_config_path(home)` devient une fonction par système, testée sur les
deux. Mêmes règles de sécurité : dossiers en `0700`, fichiers en `0600`,
écriture atomique.

### 3.2 Lancement à l'ouverture de session (faible)

Fichier `~/.config/autostart/tunnelbar.desktop` (standard freedesktop,
respecté par GNOME) créé ou supprimé par l'app. Remplace `SMAppService` ;
l'état « nécessite une approbation » n'existe pas sous Linux. Le contenu du
fichier est généré par une fonction pure testée ; le chemin de l'exécutable est
celui de l'app installée, échappé selon la spécification Desktop Entry.

### 3.3 Ouvrir le fichier de configuration (faible)

`xdg-open <fichier>` au lieu de `/usr/bin/open -t`, toujours sans shell, chemin
absolu fixe (`/usr/bin/xdg-open`).

### 3.4 Icône et menu dans la barre du haut (moyen, principal chantier UX)

- **Ubuntu affiche les indicateurs** grâce à l'extension « Ubuntu
  AppIndicators » installée et activée par défaut (contrairement à GNOME
  standard). Tauri s'appuie sur `libayatana-appindicator`.
- **Le clic sur l'icône ouvre le menu**, pas le panneau : sous Linux, Tauri ne
  transmet pas les clics sur l'icône. Le menu garde les tunnels (cases à cocher)
  et gagne une entrée **« Ouvrir TunnelBar »** en premier.
- **Icônes en couleur** : les icônes « template » de macOS n'existent pas. La
  barre d'Ubuntu étant sombre dans les deux thèmes, on fournit les trois icônes
  (inactif / actif / problème) en blanc, avec une variante sombre si besoin.
- **Détecter l'absence d'indicateur plutôt que la deviner** : au lancement,
  l'app demande via D-Bus si un service d'icônes est présent
  (`org.kde.StatusNotifierWatcher`, fourni par l'extension). S'il manque, elle
  s'ouvre en fenêtre normale et affiche une explication avec la marche à suivre
  (activer l'extension, ou installer `gnome-shell-extension-appindicator`).
  Cas réel rencontré : un poste Ubuntu 24.04.5 où l'extension n'était pas
  active.

### 3.5 Le panneau devient une petite fenêtre (moyen)

- Sous **Wayland**, session par défaut d'Ubuntu, une application ne peut pas
  choisir la position de sa fenêtre : le panneau ne peut pas s'afficher sous
  l'icône. Il devient une **petite fenêtre normale** (380 px de large, hauteur
  ajustée au contenu comme aujourd'hui), placée par le gestionnaire de fenêtres.
- Elle garde un titre et un bouton de fermeture ; elle ne se masque plus à la
  perte de focus (comportement déroutant pour une fenêtre normale). Fermer la
  fenêtre la masque, l'app reste dans la barre ; « Quitter » est dans le menu.
- Le code d'interface (Svelte) ne change pas. Seule la couche Rust distingue
  « popover » (macOS) et « fenêtre » (Linux).

### 3.6 Processus orphelins (moyen, point de sécurité)

Deux mécanismes complémentaires :

1. **Prévention (nouveau, Linux uniquement)** : chaque `ssh` est lancé avec un
   signal de mort du parent (`PR_SET_PDEATHSIG` = `SIGTERM`) : si TunnelBar
   plante ou est tué, le noyau termine ses `ssh` immédiatement. Ce réglage se
   fait entre `fork` et `exec` ; il demande un bloc `unsafe` minimal
   (`Command::pre_exec` + `nix::sys::prctl::set_pdeathsig`), confiné dans un
   module Linux dédié, comme `login_item.rs` sur macOS, avec commentaire
   `SAFETY` et test sur vrai processus.
   **Piège connu** : ce signal est lié au *thread* qui a lancé le processus,
   pas au processus TunnelBar entier. Avec le runtime multithread de Tokio, un
   thread qui s'arrête tuerait ses `ssh` alors que l'app tourne encore. Les
   lancements passent donc par un thread dédié qui vit autant que l'app, et un
   test vérifie qu'un `ssh` survit à l'arrêt d'un autre thread.
2. **Nettoyage au démarrage (adapté)** : le critère « parent = PID 1 » devient
   « le parent n'est pas une instance de TunnelBar en cours d'exécution », en
   plus de la signature de la ligne de commande et de l'heure de démarrage déjà
   vérifiées. Testé avec un subreaper simulé (le cas qui échoue aujourd'hui).

### 3.7 Ce qui ne change pas

Le moteur `ssh` (`/usr/bin/ssh`, mêmes options, `--` avant l'hôte), la
validation, la machine à états, le diagnostic des erreurs (messages d'OpenSSH
identiques), la détection de veille (l'horloge monotone de Linux s'arrête aussi
pendant la suspension) et de changement de réseau, les notifications (D-Bus via
le plugin Tauri), l'interface et les traductions.

L'authentification reste par clé : l'agent SSH de GNOME (gnome-keyring) joue le
rôle du trousseau macOS ; la documentation l'explique.

## 4. Organisation du code

```
crates/tunnelbar-core/src/
  paths.rs          (nouveau) chemins par système, fonctions pures testées
  orphans.rs        critère de parent adapté, testé sur les deux systèmes
  process.rs        appelle platform::prepare_child() avant le lancement
  platform/
    mod.rs
    linux.rs        (nouveau) PR_SET_PDEATHSIG, seul unsafe du core, Linux
src-tauri/src/
  login_item.rs     → login_item/{macos.rs, linux.rs}, même interface
  panel.rs          (nouveau) popover sous l'icône (macOS) / fenêtre (Linux)
  open.rs           (nouveau) open -t / xdg-open
```

Règle : un seul point d'entrée par fonctionnalité, choisi par
`#[cfg(target_os = …)]`, sans dupliquer la logique. Les fonctions pures
(chemins, contenu `.desktop`, critère d'orphelin) sont testées sur les deux
systèmes en CI. La règle `unsafe_code = "deny"` reste ; l'exception Linux est
documentée comme celle de macOS.

## 5. Distribution

| Format | Usage | Remarques |
|---|---|---|
| **`.deb`** (principal) | `sudo apt install ./TunnelBar_x.y.z_amd64.deb` | Généré par Tauri ; déclare ses dépendances (`libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1`, `openssh-client`) ; amd64 et arm64 |
| **AppImage** | Sans installation | Pratique pour essayer ; le lancement au démarrage pointe vers son emplacement |
| Dépôt APT signé | Plus tard | Mises à jour via `apt upgrade` ; demande une clé de signature à gérer |
| Flatpak / Snap | Écartés pour l'instant | Leurs bacs à sable compliquent l'accès à `~/.ssh`, à l'agent SSH et à `/usr/bin/ssh` |

Chaque version publie les paquets Linux à côté du `.dmg`, avec leurs empreintes
SHA-256.

## 6. CI et tests

- **Nouveau job `ubuntu`** (machines `ubuntu-24.04` et `ubuntu-24.04-arm`) :
  clippy, tests Rust du core et de l'app, couverture du core, build des paquets.
  Les minutes Linux coûtent 10 fois moins que les minutes macOS.
- `scripts/check.sh` fonctionne aussi sous Linux.
- **Test manuel avant chaque version Linux**, sur machines virtuelles et sur un
  poste réel (Ubuntu 24.04.5, GNOME 46, X11, amd64, noyau 7.0) :

| Ubuntu | Session | Vérifications |
|---|---|---|
| 24.04 LTS | Wayland, puis X11 | icône visible, menu, ouverture de la fenêtre, démarrage/arrêt d'un vrai tunnel, reconnexion après suspension, notification d'échec, lancement au démarrage, désinstallation propre |
| 26.04 LTS | Wayland | idem |

## 7. Sécurité

Les règles de `CONTRIBUTING` s'appliquent sans exception. Points spécifiques :

- Le bloc `unsafe` Linux (`pre_exec`) est revu à part : il ne doit appeler que
  des fonctions sûres entre `fork` et `exec` (`prctl` l'est).
- Fichiers `.desktop` générés sans interpolation de données utilisateur, chemin
  d'exécutable échappé.
- Le paquet `.deb` n'exécute aucun script d'installation au-delà de ce que
  génère Tauri.
- À vérifier tôt : l'interaction entre WebKitGTK et les restrictions AppArmor
  d'Ubuntu 24.04 sur les espaces de noms utilisateur (risque connu pour
  certaines applications web embarquées, surtout en AppImage).
- **Audit de sécurité complet** (`/security-audit`) avant la première version
  Linux, puis à chaque version comme aujourd'hui.

## 8. Jalons (en TDD, chacun livrable)

| # | Jalon | Contenu | Estimation |
|---|---|---|---|
| L0 | CI Linux | Job Ubuntu : le core compile et ses tests passent ; l'app compile avec bouchons | 0,5 j |
| L1 | Fichiers | `paths.rs` XDG, migration transparente des chemins par système | 0,5 j |
| L2 | Orphelins | `PR_SET_PDEATHSIG` + critère de parent adapté, tests sur vrais processus | 1 j |
| L3 | Intégration | Lancement au démarrage (`.desktop`), `xdg-open`, icônes couleur | 1 j |
| L4 | Panneau | Fenêtre normale sous Linux, entrée « Ouvrir TunnelBar », comportement sans indicateur | 1 à 1,5 j |
| L5 | Paquets | `.deb` et AppImage amd64/arm64 dans le workflow de publication | 0,5 j |
| L6 | Recette | Tests manuels sur 24.04 et 26.04, Wayland/X11, corrections | 1 j |
| L7 | Documentation | Guides FR/EN (installation, agent SSH, différences avec macOS), README | 0,5 j |
| L8 | Version | Audit de sécurité, publication de la première version Linux | 0,5 j |

**Total estimé : 6 à 7 jours de travail**, dont la recette sur de vrais bureaux
constitue la principale incertitude.

## 9. Risques

| Risque | Probabilité | Parade |
|---|---|---|
| Indicateur invisible (extension désactivée, autre bureau) | Moyenne | Ouverture en fenêtre au lancement si aucun indicateur ; documentation |
| WebKitGTK et AppArmor (Ubuntu 24.04) | Faible à moyenne | Tester dès L0 ; privilégier le `.deb` |
| `PR_SET_PDEATHSIG` lié au thread lanceur (tunnels coupés à tort) | Moyenne si on l'ignore | Thread de lancement dédié et test spécifique (voir 3.6) |
| Comportements Wayland différents selon les versions de GNOME | Moyenne | Recette sur 24.04 et 26.04 ; fenêtre normale plutôt que popover |
| Charge de maintenance de deux systèmes | Certaine | Logique commune unique, différences confinées à quelques modules, CI sur les deux |

## 10. Décisions

| Question | Décision |
|---|---|
| Lancer le port | **Oui**, le 9 novembre 2026, en TDD sur `feat/linux` |
| Formats | **`.deb`** (principal) + AppImage |
| Architectures | **amd64** de référence ; arm64 en complément |
| Dépôt public | Reste privé pendant le développement ; à décider avant la première publication Linux (les paquets doivent être téléchargeables) |
