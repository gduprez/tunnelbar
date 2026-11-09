# Recette TunnelBar sous Ubuntu

Checklist de test manuel avant la première version Linux (jalon L6 du
[plan](plan-ubuntu.md)). Durée : 30 à 45 minutes. Notez pour chaque étape ✅ ou
❌, avec une capture d'écran ou le message en cas de problème.

> ⚠️ Faites ces tests **sur votre propre machine, jamais vers un serveur de
> production**. L'étape 0 vous fait créer un tunnel vers votre propre poste.

## 0. Préparation

1. Récupérez `TunnelBar_<version>_amd64.deb` et
   `TunnelBar_<version>_amd64.AppImage`.
2. Préparez un serveur SSH local pour tester sans risque :

   ```sh
   sudo apt install openssh-server
   [ -f ~/.ssh/id_ed25519 ] || ssh-keygen -t ed25519 -N "" -f ~/.ssh/id_ed25519
   cat ~/.ssh/id_ed25519.pub >> ~/.ssh/authorized_keys
   ssh -o BatchMode=yes localhost true && echo "SSH local OK"
   ```

   (La première fois, acceptez l'empreinte de `localhost` en lançant
   `ssh localhost true` sans `BatchMode`.)
3. Notez votre configuration : `lsb_release -d`, `echo $XDG_SESSION_TYPE`,
   `gnome-extensions list --enabled | grep -i appindicator` (vide = extension
   inactive).

## 1. Installation

| # | Action | Attendu |
|---|---|---|
| 1.1 | `sudo apt install ./TunnelBar_<version>_amd64.deb` | Installation sans erreur |
| 1.2 | `dpkg -s tunnel-bar \| grep -E 'Status\|Depends'` | `install ok installed` ; dépendances `openssh-client`, `libglib2.0-bin`, `xdg-utils`… |
| 1.3 | Chercher « TunnelBar » dans les applications | L'icône et le nom apparaissent |

## 2. Sans extension d'icônes (votre cas actuel)

| # | Action | Attendu |
|---|---|---|
| 2.1 | Lancer TunnelBar | Une fenêtre s'ouvre **avec un bandeau** expliquant que l'icône ne peut pas apparaître, avec la commande `sudo apt install gnome-shell-extension-appindicator` |
| 2.2 | Fermer la fenêtre (croix) | L'app **se ferme** : `pgrep -x tunnelbar` ne renvoie rien |

## 3. Un vrai tunnel

Relancez TunnelBar.

| # | Action | Attendu |
|---|---|---|
| 3.1 | « Nouveau tunnel » : nom `Test local`, serveur `localhost`, port local `15022`, hôte distant `127.0.0.1`, port distant `22` → Enregistrer | Le tunnel apparaît, arrêté |
| 3.2 | Activer l'interrupteur | Passe à « Connexion… » puis **Actif** (pastille verte, `localhost:15022 · actif depuis…`) |
| 3.3 | `ssh -o BatchMode=yes -p 15022 localhost echo via-tunnel` | Affiche `via-tunnel` |
| 3.4 | Ouvrir le détail du tunnel, menu « ⋯ › Copier la commande ssh », coller dans un terminal (sans l'exécuter) | Commande `/usr/bin/ssh -N -o … -- localhost` |
| 3.5 | Désactiver l'interrupteur | « Arrêté » ; `pgrep -af "/usr/bin/ssh -N"` ne renvoie rien |

## 4. Erreurs et reconnexion

| # | Action | Attendu |
|---|---|---|
| 4.1 | Lancer `python3 -m http.server 15022` dans un terminal, puis démarrer le tunnel | **Erreur** : « Le port local 15022 est déjà utilisé… », expliquée dans la liste. Arrêter le serveur Python ensuite |
| 4.2 | Créer un tunnel vers l'hôte `hote-inexistant.invalid`, le démarrer | « Nom d'hôte inconnu », puis tentatives de reconnexion espacées (1 s, 2 s, 4 s…) |
| 4.3 | Tunnel `Test local` actif, puis `pkill -f "/usr/bin/ssh -N"` | « Reconnexion dans 1 s » puis de nouveau **Actif** |
| 4.4 | (Optionnel) Mettre le PC en veille 1 minute avec le tunnel actif, le réveiller | Le tunnel redevient actif en quelques secondes |
| 4.5 | Faire échouer un tunnel définitivement (ex. : 4.1) | Une **notification** du bureau s'affiche |

## 5. Plantage

| # | Action | Attendu |
|---|---|---|
| 5.1 | Tunnel `Test local` actif, puis `pkill -9 -x tunnelbar` | TunnelBar disparaît **et** `pgrep -af "/usr/bin/ssh -N"` ne renvoie rien (le tunnel s'est arrêté avec lui) |
| 5.2 | Relancer TunnelBar | Démarre normalement, tunnels arrêtés |

## 6. Avec l'extension d'icônes

| # | Action | Attendu |
|---|---|---|
| 6.1 | `gnome-extensions enable ubuntu-appindicators@ubuntu.com` (ou `sudo apt install gnome-shell-extension-appindicator`), puis fermer et rouvrir la session | `gnome-extensions list --enabled \| grep -i appindicator` affiche l'extension |
| 6.2 | Lancer TunnelBar | **Pas** de fenêtre au démarrage ; une **icône blanche** apparaît dans la barre du haut |
| 6.3 | Cliquer sur l'icône | Un **menu** s'ouvre : les tunnels, « Ouvrir le panneau… », « Quitter TunnelBar » |
| 6.4 | Cocher `Test local` dans le menu | Le tunnel démarre ; l'icône passe à l'arche pleine |
| 6.5 | « Ouvrir le panneau… », puis fermer la fenêtre | La fenêtre se ferme, **l'icône reste** : l'app tourne toujours |
| 6.6 | Menu › « Quitter TunnelBar » | L'app et ses tunnels s'arrêtent |

## 7. Réglages

| # | Action | Attendu |
|---|---|---|
| 7.1 | Réglages › « Ouvrir TunnelBar à l'ouverture de session » : activer | `cat ~/.config/autostart/tunnelbar.desktop` montre `Exec=/usr/bin/tunnelbar` |
| 7.2 | Fermer puis rouvrir la session | TunnelBar démarre tout seul |
| 7.3 | Désactiver le réglage | Le fichier `~/.config/autostart/tunnelbar.desktop` a disparu |
| 7.4 | Réglages › « Ouvrir le fichier de configuration » | `~/.config/tunnelbar/tunnels.toml` s'ouvre dans l'éditeur de texte |
| 7.5 | Changer la langue (Français / English) | Toute l'interface et le menu changent de langue |
| 7.6 | Modifier à la main le nom d'un tunnel dans `tunnels.toml`, enregistrer | Le nom change dans TunnelBar en moins de 2 secondes |

## 8. AppImage

| # | Action | Attendu |
|---|---|---|
| 8.1 | `chmod +x TunnelBar_<version>_amd64.AppImage && ./TunnelBar_<version>_amd64.AppImage` (après avoir quitté la version `.deb`) | L'app démarre, mêmes tunnels |
| 8.2 | Activer « Ouvrir à l'ouverture de session » | `~/.config/autostart/tunnelbar.desktop` pointe vers le fichier `.AppImage` |
| 8.3 | Désactiver le réglage, quitter | Fichier supprimé |

## 9. Sécurité du moteur web (AppArmor)

Après avoir utilisé l'app (étapes 2 à 8) :

```sh
journalctl --user -b | grep -iE "apparmor|bwrap|sandbox" | grep -i -E "webkit|tunnelbar" | tail
sudo dmesg | grep -i apparmor | grep -iE "tunnelbar|webkit" | tail
```

Copiez la sortie, même vide.

## 10. Désinstallation

```sh
sudo apt remove tunnel-bar
```

| # | Attendu |
|---|---|
| 10.1 | L'app disparaît du menu des applications |
| 10.2 | `~/.config/tunnelbar/` est conservé (vos tunnels) |

## À renvoyer

- Le tableau des résultats (✅ / ❌ et commentaires).
- Les sorties de l'étape 0.3 et de l'étape 9.
- Le journal : `~/.local/share/io.github.gduprez.tunnelbar/logs/tunnelbar.log`.
- Des captures de la fenêtre (étape 2.1), du menu (6.3) et d'un tunnel actif.

Pour nettoyer le serveur SSH de test ensuite : `sudo apt remove openssh-server`
et retirer la ligne ajoutée à `~/.ssh/authorized_keys`.
