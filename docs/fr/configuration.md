# Référence de la configuration

> 🇬🇧 [English version](../en/configuration.md)

TunnelBar enregistre vos tunnels dans un fichier TOML :

```
~/Library/Application Support/TunnelBar/tunnels.toml
```

Vous pouvez le modifier depuis l'app ou à la main. Un exemple complet et testé
se trouve dans [`docs/examples/tunnels.toml`](../examples/tunnels.toml).

## Exemple minimal

La commande `ssh -N -L 54321:127.0.0.1:5432 203.0.113.10` devient :

```toml
version = 1

[[tunnel]]
id = "prod-db"
name = "PostgreSQL prod"
host = "203.0.113.10"

[[tunnel.forward]]
type = "local"
local_port = 54321
remote_host = "127.0.0.1"
remote_port = 5432
```

## Fichier

| Clé | Obligatoire | Description |
|---|---|---|
| `version` | oui | Version du format. Doit valoir `1`. |
| `[[tunnel]]` | non | Zéro, un ou plusieurs tunnels. |

## Tunnel (`[[tunnel]]`)

Un tunnel est **une connexion SSH** qui porte une ou plusieurs redirections.
Il démarre et s'arrête d'un bloc.

| Clé | Obligatoire | Par défaut | Description |
|---|---|---|---|
| `id` | oui | — | Identifiant stable : 1 à 64 caractères parmi `a-z`, `0-9`, `-`, `_`, commençant par une lettre ou un chiffre. Unique. |
| `name` | oui | — | Libellé affiché dans le menu et le panneau. |
| `host` | oui | — | Nom d'hôte, adresse IP ou alias de `~/.ssh/config`. |
| `user` | non | celui de `~/.ssh/config` ou votre login | Utilisateur SSH. |
| `port` | non | celui de `~/.ssh/config` ou `22` | Port SSH. |
| `autostart` | non | `false` | Démarrer le tunnel au lancement de TunnelBar. |
| `auto_reconnect` | non | `true` | Se reconnecter automatiquement après une coupure. |
| `accept_new_host_key` | non | `false` | Accepter la clé d'un serveur jamais vu (`StrictHostKeyChecking=accept-new`). Une clé **modifiée** est toujours refusée. |
| `[[tunnel.forward]]` | oui (au moins une) | — | Redirections portées par la connexion. |

> **Astuce :** les réglages SSH avancés (`ProxyJump`, `IdentityFile`…) se
> placent dans `~/.ssh/config`. Créez-y un alias `Host` et utilisez-le comme
> `host`.

## Redirections (`[[tunnel.forward]]`)

La clé `type` choisit le type de redirection.

### `local` : `-L` (accéder à un service distant depuis votre Mac)

| Clé | Description |
|---|---|
| `local_port` | Port ouvert sur votre Mac (`127.0.0.1`). |
| `remote_host` | Hôte à joindre, **vu depuis le serveur SSH** (souvent `127.0.0.1`). |
| `remote_port` | Port de ce service. |

### `remote` : `-R` (exposer un service de votre Mac sur le serveur)

| Clé | Description |
|---|---|
| `remote_port` | Port ouvert sur le serveur SSH. |
| `local_host` | Hôte à joindre, vu depuis votre Mac (souvent `127.0.0.1`). |
| `local_port` | Port de ce service. |

### `dynamic` : `-D` (proxy SOCKS)

| Clé | Description |
|---|---|
| `local_port` | Port du proxy SOCKS ouvert sur votre Mac. |

## Règles de validation

TunnelBar refuse de charger ou d'enregistrer une configuration qui enfreint
l'une de ces règles, et indique le tunnel et le champ en cause :

| Erreur | Règle |
|---|---|
| `UnsupportedVersion` | `version` doit valoir `1`. |
| `InvalidId` | Voir le format de `id` ci-dessus. |
| `DuplicateId` | Deux tunnels ont le même `id`. |
| `EmptyName` | `name` est vide ou ne contient que des espaces. |
| `InvalidHost`, `InvalidUser`, `InvalidForwardHost` | Vide, commençant par `-`, ou contenant des espaces ou des caractères de contrôle. Cela empêche qu'une valeur soit lue par `ssh` comme une option (ex. `-oProxyCommand=…`). |
| `ZeroPort` | Les ports vont de `1` à `65535` (les valeurs au-delà de `65535` sont refusées dès la lecture du fichier). |
| `NoForward` | Un tunnel doit avoir au moins une redirection. |
| `LocalPortConflict` | Deux redirections (`local` ou `dynamic`) écoutent sur le même port de votre Mac. |

Les clés inconnues sont aussi refusées, pour qu'une faute de frappe
(`hots = …`) ne passe pas inaperçue.

## Sécurité

- Le fichier est écrit avec les permissions `0600` (lisible par vous seul),
  dans un dossier créé avec `0700`.
- Les écritures sont atomiques : le fichier n'est jamais laissé à moitié écrit.
- TunnelBar ne lance jamais `ssh` via un shell, et n'a pas de champ
  « options libres ».
