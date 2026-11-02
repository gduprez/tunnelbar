# Contribuer à TunnelBar

> 🇬🇧 [English version](CONTRIBUTING.md)

Merci de votre intérêt ! Les rapports de bug, traductions et pull requests sont
les bienvenus.

## Avant de commencer

- Lisez le [document de conception](docs/fr/design.md) : il consigne les
  décisions que le code suit. Une modification de l'une d'elles met ce document
  à jour dans la même pull request.
- Pour tout ce qui dépasse une correction, ouvrez d'abord une issue pour
  convenir de l'approche.

## Installation

Prérequis : macOS 13+, Rust (stable, 1.93 ou plus), Node.js 24 (`.nvmrc`),
outils en ligne de commande Xcode.

```sh
npm ci
npm run tauri dev      # lancer l'app
```

Pour travailler le panneau dans un navigateur, avec des tunnels d'exemple et
sans la partie Rust :

```sh
npm run dev            # puis ouvrir http://localhost:1420/?preview
```

## Développement piloté par les tests (TDD)

TunnelBar se développe tests d'abord : **rouge → vert → refactorisation**.

1. Écrire un test qui échoue pour la bonne raison.
2. Écrire le code le plus simple qui le fait passer.
3. Nettoyer, tests toujours au vert.

Où placer les tests :

| Couche | Emplacement | Outils |
|---|---|---|
| Domaine (modèle, validation, commande `ssh`, machine à états, diagnostic des erreurs, supervision) | `crates/tunnelbar-core` | `cargo nextest` ; faux dans `src/fake.rs` ; temps Tokio en pause |
| Vrais processus | `crates/tunnelbar-core/tests/real_processes.rs` | faux `ssh` scripté |
| Couche Tauri (logique du menu, traductions, commandes) | `src-tauri/src` | fonctions pures à côté de leurs tests |
| Panneau | `src/**/*.test.ts` | Vitest, Testing Library, `src/lib/api.ts` simulé |
| Bout en bout (à la demande) | `crates/tunnelbar-core/tests/end_to_end.rs` | un vrai serveur SSH |

```sh
TUNNELBAR_E2E_HOST=mon-serveur cargo test -p tunnelbar-core --test end_to_end -- --ignored
```

Le core garde au moins **85 %** de couverture des lignes (vérifié par la CI).

## Contrôles

La CI les exécute tous ; lancez-les avant de pousser :

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace
cargo llvm-cov -p tunnelbar-core --fail-under-lines 85
cargo deny check
npm run format:check && npm run lint && npm run check && npm test
```

## Commits et pull requests

- [Conventional Commits](https://www.conventionalcommits.org/) : `feat:`,
  `fix:`, `docs:`, `test:`, `refactor:`, `chore:`… Ils produisent le changelog
  et le numéro de version (release-please).
- Un sujet par pull request, avec ses tests.
- Tout texte visible passe par les traductions (voir ci-dessous), en anglais
  et en français.

## Ajouter une langue

1. Copiez `locales/en.json` vers `locales/<code>.json` (`de`, `es`, `pt-BR` → `pt`…).
2. Traduisez les valeurs ; gardez les clés et les `{paramètres}`.
3. Lancez `npm test` : un test échoue si une clé manque ou est en trop.

C'est tout : le panneau et le menu prennent le fichier en compte
automatiquement, et la langue apparaît dans les réglages sous son propre nom.

## Sécurité

Signalez les vulnérabilités en privé : voir [SECURITY.md](SECURITY.md).

## Licence

En contribuant, vous acceptez que vos contributions soient distribuées sous
`MIT OR Apache-2.0`, comme le projet.
