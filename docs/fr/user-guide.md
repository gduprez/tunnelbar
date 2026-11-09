# Guide d'utilisation de TunnelBar

> 🇬🇧 [English version](../en/user-guide.md)

TunnelBar vit dans la barre de menus, à côté de l'heure. Il démarre, arrête et
surveille vos tunnels SSH, et les reconnecte tout seul.

> Sous Ubuntu, voir [TunnelBar sous Ubuntu](linux.md).

## Installation

### Avec Homebrew

```sh
brew install --cask gduprez/tap/tunnelbar
```

### Depuis l'image disque

1. Téléchargez `TunnelBar_<version>_universal.dmg` depuis les
   [releases](https://github.com/gduprez/tunnelbar/releases).
2. Ouvrez-la et glissez **TunnelBar** dans **Applications**.
3. **Premier lancement uniquement :** TunnelBar n'est pas encore notarisé par
   Apple, macOS le bloque donc (« Apple n'a pas pu vérifier… »). Au choix :
   - ouvrez **Réglages Système › Confidentialité et sécurité**, descendez et
     cliquez sur **Ouvrir quand même** à côté de TunnelBar ;
   - ou lancez une fois dans un terminal :

     ```sh
     xattr -dr com.apple.quarantine /Applications/TunnelBar.app
     ```

### Depuis les sources

Voir [Compiler depuis les sources](../../README.fr.md#compiler-depuis-les-sources).

## Prérequis

- macOS 13 Ventura ou plus récent.
- **Authentification SSH par clé.** TunnelBar lance `ssh` en arrière-plan, où
  personne ne peut saisir de mot de passe ni de passphrase. Vérifiez que
  `ssh mon-serveur` se connecte sans rien demander. Si votre clé a une
  passphrase, enregistrez-la une fois dans le trousseau macOS :

  ```sh
  ssh-add --apple-use-keychain ~/.ssh/id_ed25519
  ```

  et ajoutez dans `~/.ssh/config` :

  ```
  Host *
    UseKeychain yes
    AddKeysToAgent yes
  ```

## Utilisation au quotidien

| Action | Comment |
|---|---|
| Démarrer ou arrêter un tunnel | Clic droit sur l'icône, puis clic sur le tunnel. Ou son interrupteur dans le panneau. |
| Ouvrir le panneau | Clic sur l'icône. |
| Comprendre pourquoi un tunnel échoue | Ouvrez le panneau et cliquez sur le tunnel : l'erreur et les dernières lignes affichées par `ssh` y sont. |
| Copier la commande `ssh` équivalente | Dans le détail du tunnel, **⋯ › Copier la commande ssh**. Modifier, Dupliquer et Supprimer sont dans le même menu. |
| Quitter | Clic droit sur l'icône › **Quitter TunnelBar**. Tous les tunnels sont fermés. |

### L'icône

| Icône | Signification |
|---|---|
| Arche en contour | Aucun tunnel en marche. |
| Arche pleine | Des tunnels fonctionnent normalement. |
| Arche avec `!` | Au moins un tunnel est en erreur ou en reconnexion. |

### États d'un tunnel

| État | Signification |
|---|---|
| Arrêté | Il ne tourne pas. |
| Connexion… | `ssh` tourne ; TunnelBar attend que les ports locaux acceptent les connexions. |
| Actif | Le tunnel fonctionne. |
| Reconnexion dans N s | La connexion a été perdue ; TunnelBar réessaie après 1 s, 2 s, 4 s… jusqu'à 60 s. Réveiller le Mac ou changer de réseau relance aussitôt. |
| Erreur | Un problème que réessayer ne réglera pas (clé refusée, port déjà utilisé, clé du serveur inconnue ou modifiée…). Une notification s'affiche. Corrigez la cause, puis redémarrez le tunnel. |

## Ajouter un tunnel

Cliquez sur **+** dans le panneau. Pour la commande
`ssh -L 54321:127.0.0.1:5432 203.0.113.10 -N` :

| Champ | Valeur |
|---|---|
| Nom | PostgreSQL prod |
| Serveur | `203.0.113.10` |
| Port local | `54321` |
| Hôte distant | `127.0.0.1` |
| Port distant | `5432` |

Un tunnel peut porter plusieurs redirections sur la même connexion
(**Ajouter une redirection**). Les redirections distantes (`-R`) et SOCKS
(`-D`) se règlent dans le fichier de configuration : voir la
[référence de la configuration](configuration.md).

Options :

- **Se reconnecter automatiquement** (activée par défaut).
- **Démarrer au lancement de TunnelBar.**
- **Accepter la clé d'un serveur jamais vu** : seulement pour les serveurs
  dont vous ne pouvez pas vérifier l'empreinte dans un terminal. Une clé
  *modifiée* est toujours refusée.

## Réglages

- **Ouvrir TunnelBar à l'ouverture de session.** macOS peut vous demander de
  l'autoriser dans **Réglages Système › Général › Ouverture**.
- **Langue** : comme macOS, ou n'importe quelle langue disponible.
- **Fichier de configuration** : ouvrez `tunnels.toml` pour le modifier à la
  main ; TunnelBar le recharge en moins de deux secondes.

## Dépannage

| Message | Que faire |
|---|---|
| Authentification refusée | `ssh mon-serveur` doit se connecter sans question (voir [Prérequis](#prérequis)). |
| Serveur inconnu | Lancez une fois `ssh mon-serveur` dans un terminal et vérifiez l'empreinte, puis redémarrez le tunnel. |
| L'identité du serveur a CHANGÉ | Ne contournez pas l'alerte. Demandez à l'administrateur du serveur si sa clé a changé ; si oui, supprimez l'ancienne avec `ssh-keygen -R mon-serveur`. |
| Le port local N est déjà utilisé | Un autre programme (ou un autre tunnel) utilise ce port : `lsof -nP -iTCP:N -sTCP:LISTEN` indique lequel. |
| Nom d'hôte inconnu | Vérifiez l'orthographe, ou votre réseau / VPN. |

Journaux de l'app : `~/Library/Logs/io.github.gduprez.tunnelbar/tunnelbar.log`.

## Fichiers

| Fichier | Contenu |
|---|---|
| `~/Library/Application Support/TunnelBar/tunnels.toml` | Vos tunnels. |
| `~/Library/Application Support/TunnelBar/settings.toml` | Réglages de l'app. |
| `~/Library/Application Support/TunnelBar/ssh.pids` | Processus `ssh` en cours, pour faire le ménage après un plantage. |
| `~/Library/Logs/io.github.gduprez.tunnelbar/` | Journaux. |
