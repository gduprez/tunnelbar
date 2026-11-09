# TunnelBar sous Ubuntu

> 🇬🇧 [English version](../en/linux.md)

TunnelBar fonctionne sous **Ubuntu 24.04 LTS et 26.04 LTS** (bureau GNOME, en
session Wayland ou X11, architecture amd64). Les autres dérivés d'Ubuntu dotés
d'une zone d'icônes peuvent fonctionner, sans être testés.

## Installation

### Paquet `.deb` (recommandé)

```sh
sudo apt install ./TunnelBar_<version>_amd64.deb
```

Le paquet installe aussi `openssh-client` s'il manque. TunnelBar apparaît dans
la liste des applications.

### AppImage (sans installation)

```sh
chmod +x TunnelBar_<version>_amd64.AppImage
./TunnelBar_<version>_amd64.AppImage
```

## L'icône dans la barre du haut

Ubuntu affiche les icônes d'application grâce à l'extension **Ubuntu
AppIndicators**, active par défaut. Si elle a été désactivée, TunnelBar s'ouvre
en fenêtre au démarrage et explique la marche à suivre :

- activez « Ubuntu AppIndicators » dans l'application **Extensions** ;
- ou installez-la : `sudo apt install gnome-shell-extension-appindicator`,
  puis fermez et rouvrez votre session.

Pour vérifier qu'elle est active :

```sh
gnome-extensions list --enabled | grep -i appindicator
```

## Différences avec macOS

| | macOS | Ubuntu |
|---|---|---|
| Clic sur l'icône | ouvre le panneau | ouvre le menu (tunnels, « Ouvrir le panneau… ») |
| Panneau | popover sous l'icône, se ferme quand on clique ailleurs | petite fenêtre normale, placée par le bureau |
| Fermer le panneau | — | la masque ; TunnelBar reste dans la barre du haut. **Sans icône visible, fermer quitte TunnelBar** (sinon il deviendrait inaccessible) |
| Lancement à l'ouverture de session | Réglages Système › Ouverture | fichier `~/.config/autostart/tunnelbar.desktop`, créé par le réglage de TunnelBar |
| Phrase secrète des clés | trousseau macOS | agent SSH de GNOME (gnome-keyring) |

Tout le reste est identique : tunnels, reconnexion automatique, notifications,
traductions, format du fichier de configuration.

## Authentification SSH

Comme sur macOS, TunnelBar lance `ssh` en arrière-plan, sans possibilité de
saisir un mot de passe. Vérifiez que `ssh mon-serveur` se connecte sans rien
demander. Si votre clé a une phrase secrète, l'agent SSH de GNOME la retient
après la première saisie dans votre session.

## Fichiers

| Fichier | Contenu |
|---|---|
| `~/.config/tunnelbar/tunnels.toml` | Vos tunnels ([référence](configuration.md)) |
| `~/.config/tunnelbar/settings.toml` | Réglages de l'app |
| `~/.local/state/tunnelbar/ssh.pids` | Processus `ssh` en cours |
| `~/.local/share/io.github.gduprez.tunnelbar/logs/` | Journaux |
| `~/.config/autostart/tunnelbar.desktop` | Lancement à l'ouverture de session, si activé |

Les variables `XDG_CONFIG_HOME` et `XDG_STATE_HOME` sont respectées.

## Si TunnelBar plante

Sous Linux, chaque `ssh` lancé par TunnelBar s'arrête automatiquement si
TunnelBar s'arrête, même brutalement : aucun tunnel ne reste ouvert à votre insu.

## Désinstallation

```sh
sudo apt remove tunnelbar
rm -f ~/.config/autostart/tunnelbar.desktop
```

Vos tunnels restent dans `~/.config/tunnelbar/` ; supprimez ce dossier pour
tout effacer.
