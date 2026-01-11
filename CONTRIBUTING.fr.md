# Contribuer à dynamic-cli

Tout d'abord, merci d'envisager de contribuer à dynamic-cli ! 🎉

**[English](CONTRIBUTING.md)** | **Français**

## 📋 Table des Matières

- [Code de Conduite](#code-de-conduite)
- [Premiers Pas](#premiers-pas)
- [Configuration du Développement](#configuration-du-développement)
- [Comment Puis-je Contribuer ?](#comment-puis-je-contribuer)
- [Flux de Travail de Développement](#flux-de-travail-de-développement)
- [Standards de Codage](#standards-de-codage)
- [Directives de Test](#directives-de-test)
- [Documentation](#documentation)
- [Processus de Pull Request](#processus-de-pull-request)
- [Communauté](#communauté)

---

## 📜 Code de Conduite

Ce projet et tous ceux qui y participent sont régis par notre Code de Conduite. En participant, vous vous engagez à respecter ce code. Veuillez signaler tout comportement inacceptable aux mainteneurs du projet.

### Nos Standards

**Les comportements positifs incluent :**
- Utiliser un langage courtois et bienveillant
- Être respectueux des points de vue et expériences différents
- Les critiques constructives permettent d'avancer et de progresser, écoutons-les...
- Se concentrer sur ce qui est le mieux pour la communauté
- Faire preuve d'empathie envers les autres membres de la communauté

**Les comportements inacceptables incluent :**
- Le trolling, les commentaires insultants/dérogatoires et les attaques personnelles
- Le harcèlement public ou privé
- Publier les informations privées d'autrui sans permission
- Toute autre conduite qui pourrait raisonnablement être considérée comme inappropriée

---

## 🚀 Premiers Pas

### Prérequis

Avant de commencer, assurez-vous d'avoir installé :

```bash
# Rust (dernière version stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Outils essentiels
rustup component add rustfmt clippy
```

**Versions recommandées :**
- Rust : 1.75.0 ou ultérieur
- Cargo : Dernière version stable

### Démarrage Rapide

```bash
# 1. Forkez le dépôt sur GitHub
# 2. Clonez votre fork
git clone https://github.com/biface/dcli.git
cd dynamic-cli

# 3. Ajoutez le remote upstream
git remote add upstream https://github.com/biface/dcli.git

# 4. Créez une branche
git checkout -b feature/ma-fonctionnalite

# 5. Faites vos modifications
# ...

# 6. Exécutez les tests
cargo test --all-features

# 7. Commitez et poussez
git commit -am "Ajout d'une fonctionnalité"
git push origin feature/ma-fonctionnalite

# 8. Créez une Pull Request sur GitHub
```

---

## 🛠 Configuration du Développement

### Configuration Initiale

```bash
# Clonez le dépôt
git clone https://github.com/biface/dcli.git
cd dynamic-cli

# Installez les dépendances et compilez
cargo build

# Exécutez les tests pour vérifier la configuration
cargo test --all-features

# Exécutez les exemples pour voir en action
cargo run --example simple_calculator
```

### Outils de Développement

Nous utilisons plusieurs outils pour maintenir la qualité du code :

```bash
# Formater le code
cargo fmt

# Vérifier les erreurs courantes
cargo clippy --all-features -- -D warnings

# Exécuter tous les tests
cargo test --all-features

# Générer la documentation
cargo doc --no-deps --open

# Exécuter les benchmarks
cargo bench
```

### Structure du Projet

```
dynamic-cli/
├── src/
│   ├── lib.rs              # Point d'entrée de la bibliothèque
│   ├── error/              # Types d'erreurs et gestion
│   ├── config/             # Chargement et validation de configuration
│   ├── context/            # Traits de contexte d'exécution
│   ├── executor/           # Exécution de commandes
│   ├── registry/           # Registre de commandes
│   ├── parser/             # Parsing CLI et REPL
│   ├── validator/          # Validation d'arguments
│   ├── interface/          # Interfaces CLI et REPL
│   ├── builder.rs          # API Builder
│   └── utils.rs            # Fonctions utilitaires
├── examples/               # Exemples d'applications
├── tests/                  # Tests d'intégration
├── benches/                # Benchmarks
└── docs/                   # Documentation supplémentaire
```

---

## 💡 Comment Puis-je Contribuer ?

### Signaler des Bugs

**Avant de soumettre un rapport de bug :**
- Vérifiez le [suivi des issues](https://github.com/biface/dcli/issues) pour voir s'il est déjà signalé
- Essayez de reproduire le problème avec la dernière version
- Collectez les informations pertinentes (OS, version Rust, messages d'erreur)

**Lors de la soumission d'un rapport de bug, incluez :**
- Un titre clair et descriptif
- Les étapes détaillées pour reproduire le problème
- Le comportement attendu vs. le comportement réel
- Des échantillons de code ou cas de test (si applicable)
- Les détails de votre environnement

**Modèle de rapport de bug :**
```markdown
**Description :**
Une description claire du bug.

**Étapes pour Reproduire :**
1. Étape 1
2. Étape 2
3. ...

**Comportement Attendu :**
Ce que vous attendiez qu'il se passe.

**Comportement Réel :**
Ce qui s'est réellement passé.

**Environnement :**
- OS : [ex., Ubuntu 22.04]
- Version Rust : [ex., 1.75.0]
- Version dynamic-cli : [ex., 0.1.0]

**Contexte Supplémentaire :**
Toute autre information pertinente.
```

### Suggérer des Fonctionnalités

**Avant de suggérer une fonctionnalité :**
- Vérifiez si elle n'est pas déjà suggérée ou en développement
- Considérez si elle correspond à la portée et aux objectifs du projet
- Pensez au bénéfice qu'elle apportera à la majorité des utilisateurs

**Lors de la suggestion d'une fonctionnalité, incluez :**
- Un titre clair et descriptif
- Le problème que votre fonctionnalité résout
- Votre solution proposée
- Les solutions alternatives que vous avez envisagées
- Tout exemple ou cas d'usage pertinent

**Modèle de demande de fonctionnalité :**
```markdown
**Problème :**
Décrivez le problème que vous essayez de résoudre.

**Solution Proposée :**
Décrivez votre solution proposée.

**Alternatives :**
Autres solutions que vous avez envisagées.

**Cas d'Usage :**
Scénarios réels où cela serait utile.
```

### Améliorer la Documentation

Les améliorations de documentation sont toujours les bienvenues ! Cela inclut :

- Corriger les fautes de frappe ou erreurs grammaticales
- Clarifier des explications confuses
- Ajouter de la documentation manquante
- Améliorer les exemples de code
- Traduire la documentation

**Emplacements de documentation :**
- Documentation API : Commentaires Rustdoc dans les fichiers sources
- Guide utilisateur : Répertoire `docs/`
- Exemples : Répertoire `examples/`
- README : `README.md` et `README.fr.md`
- Ce fichier : `CONTRIBUTING.md` et `CONTRIBUTING.fr.md`

### Contribuer du Code

Nous accueillons les contributions de code ! Voici les types de contributions que nous recherchons :

**Corrections de bugs :**
- Corriger les issues signalées
- Améliorer la gestion des erreurs
- Améliorer la gestion des cas limites

**Fonctionnalités :**
- Implémenter les fonctionnalités demandées
- Ajouter de nouvelles fonctionnalités (après discussion)
- Améliorer les fonctionnalités existantes

**Refactoring :**
- Améliorer la qualité du code
- Optimiser les performances
- Améliorer la maintenabilité

**Tests :**
- Ajouter des tests manquants
- Améliorer la couverture de tests
- Ajouter des tests d'intégration

---

## 🔄 Flux de Travail de Développement

### 1. Trouver ou Créer une Issue

- Vérifiez les issues existantes
- Créez une nouvelle issue si nécessaire
- Discutez de votre approche avant de coder (pour les gros changements)

### 2. Fork et Branche

```bash
# Forkez sur GitHub, puis :
git clone https://github.com/biface/dynamic-cli.git
cd dynamic-cli

# Ajoutez upstream
git remote add upstream https://github.com/biface/dynamic-cli.git

# Créez une branche de fonctionnalité
git checkout -b feature/nom-descriptif
# ou
git checkout -b fix/numero-issue
```

**Conventions de nommage des branches :**
- `feature/description` - Nouvelles fonctionnalités
- `fix/numero-issue` - Corrections de bugs
- `docs/description` - Documentation
- `refactor/description` - Refactoring de code
- `test/description` - Améliorations de tests

### 3. Faites Vos Modifications

**Suivez ces pratiques :**
- Écrivez du code propre et lisible
- Suivez les standards de codage (voir ci-dessous)
- Ajoutez des tests pour les nouvelles fonctionnalités
- Mettez à jour la documentation au besoin
- Gardez les commits atomiques et ciblés

### 4. Testez Vos Modifications

```bash
# Exécuter tous les tests
cargo test --all-features

# Exécuter clippy
cargo clippy --all-features -- -D warnings

# Formater le code
cargo fmt

# Vérifier la documentation
cargo doc --no-deps

# Exécuter un test spécifique
cargo test nom_test

# Exécuter avec sortie
cargo test -- --nocapture
```

### 5. Commitez Vos Modifications

**Bons messages de commit :**
```bash
# Format : <type>: <sujet>

# Exemples :
git commit -m "feat: ajout du support pour validateurs personnalisés"
git commit -m "fix: résolution du problème de parsing avec guillemets échappés"
git commit -m "docs: amélioration documentation module executor"
git commit -m "test: ajout tests d'intégration mode REPL"
git commit -m "refactor: simplification gestion erreurs dans parser"
```

**Types de commit :**
- `feat` : Nouvelle fonctionnalité
- `fix` : Correction de bug
- `docs` : Documentation
- `test` : Tests
- `refactor` : Refactoring de code
- `perf` : Amélioration de performance
- `style` : Changements de style de code
- `chore` : Changements build/outils

### 6. Poussez et Créez une Pull Request

```bash
# Poussez vers votre fork
git push origin feature/ma-fonctionnalite

# Créez une Pull Request sur GitHub
# Remplissez le template de PR
```

---

## 📏 Standards de Codage

### Principes Généraux

- **Clarté plutôt que ruse** : Écrivez du code facile à comprendre
- **Cohérence** : Suivez les patterns existants dans le code
- **Documentation** : Documentez les APIs publiques et la logique complexe
- **Tests** : Visez 80-90% de couverture de tests
- **Performance** : Optimisez quand nécessaire, mais priorisez la correction

### Directives Spécifiques à Rust

**Style de code :**
- Suivez les défaults de `rustfmt` (exécutez `cargo fmt`)
- Suivez les suggestions de `clippy` (exécutez `cargo clippy`)
- Utilisez des noms de variables et fonctions significatifs
- Gardez les fonctions ciblées et petites

**Gestion des erreurs :**
- Utilisez `Result<T>` pour les opérations faillibles
- Fournissez du contexte dans les messages d'erreur
- Utilisez `thiserror` pour les types d'erreurs
- Utilisez `anyhow` pour les erreurs au niveau application

**Documentation :**
- Documentez tous les éléments publics avec des commentaires `///`
- Incluez des exemples dans la documentation
- Expliquez le "pourquoi", pas seulement le "quoi"
- Utilisez le formatage Markdown approprié

**Exemple :**
```rust
/// Parse un argument de ligne de commande dans le type spécifié
///
/// Cette fonction tente de parser une valeur chaîne dans le type cible
/// spécifié par `arg_type`. Elle gère tous les types d'arguments supportés
/// et fournit des messages d'erreur détaillés en cas d'échec.
///
/// # Arguments
///
/// * `value` - La valeur chaîne à parser
/// * `arg_type` - Le type cible pour le parsing
///
/// # Returns
///
/// Un `Result` contenant la valeur parsée comme chaîne, ou une erreur
/// si le parsing échoue.
///
/// # Errors
///
/// Retourne [`ParseError::TypeParseError`] si la valeur ne peut pas être
/// parsée dans le type spécifié.
///
/// # Exemples
///
/// ```
/// use dynamic_cli::parser::parse_value;
/// use dynamic_cli::config::ArgumentType;
///
/// let result = parse_value("42", ArgumentType::Integer)?;
/// assert_eq!(result, "42");
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
pub fn parse_value(
    value: &str,
    arg_type: ArgumentType,
) -> Result<String> {
    // Implémentation
}
```

### Organisation du Code

**Structure des modules :**
- Un module par responsabilité majeure
- API publique dans `mod.rs`
- Implémentation privée dans fichiers séparés
- Tests dans modules `#[cfg(test)]`

**Conventions de nommage :**
- `snake_case` pour fonctions et variables
- `PascalCase` pour types et traits
- `SCREAMING_SNAKE_CASE` pour constantes
- Préfixez les éléments privés avec underscore si inutilisés

### Performance

**Directives d'optimisation :**
- Profilez avant d'optimiser
- Documentez les sections critiques pour les performances
- Utilisez les structures de données appropriées
- Évitez les allocations inutiles
- Clonez seulement quand nécessaire

---

## 🧪 Directives de Test

### Objectifs de Couverture de Tests

- **Tests unitaires** : Couverture 80-90%
- **Tests d'intégration** : Couvrir les workflows principaux
- **Tests de documentation** : Tous les exemples publics fonctionnent
- **Cas limites** : Tester les conditions d'erreur

### Écrire des Tests

**Tests unitaires :**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_integer_valide() {
        let result = parse_integer("42").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_parse_integer_invalide() {
        let result = parse_integer("pas un nombre");
        assert!(result.is_err());
    }

    #[test]
    #[should_panic(expected = "overflow")]
    fn test_parse_integer_overflow() {
        parse_integer("999999999999999999999").unwrap();
    }
}
```

**Tests d'intégration :**
```rust
// tests/cli_integration.rs
use dynamic_cli::prelude::*;

#[test]
fn test_workflow_cli_complet() {
    // Tester le workflow CLI complet
}
```

**Tests de documentation :**
```rust
/// Parse un entier depuis une chaîne
///
/// # Exemples
///
/// ```
/// use dynamic_cli::parser::parse_integer;
///
/// let value = parse_integer("42")?;
/// assert_eq!(value, 42);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse_integer(s: &str) -> Result<i64> {
    // Implémentation
}
```

### Exécuter les Tests

```bash
# Tous les tests
cargo test --all-features

# Test spécifique
cargo test nom_test

# Avec sortie
cargo test -- --nocapture

# Tests de documentation
cargo test --doc

# Tests d'intégration uniquement
cargo test --test '*'

# Avec couverture (nécessite cargo-tarpaulin)
cargo tarpaulin --out Html
```

### Organisation des Tests

**Organisation des fichiers :**
- Tests unitaires : Même fichier que le code dans module `#[cfg(test)]`
- Tests d'intégration : Répertoire `tests/`
- Benchmarks : Répertoire `benches/`

**Nommage des tests :**
- Noms descriptifs : `test_parse_integer_valide`
- Groupez les tests liés dans des modules
- Utilisez `#[ignore]` pour les tests lents

---

## 📚 Documentation

### Standards de Documentation

**Tous les éléments publics doivent avoir :**
- Ligne de résumé
- Description détaillée
- Arguments (pour fonctions)
- Valeur de retour (pour fonctions)
- Erreurs (pour fonctions faillibles)
- Exemples
- Liens vers éléments liés

**Exemple :**
```rust
/// Charge la configuration depuis un fichier YAML ou JSON
///
/// Détecte automatiquement le format du fichier basé sur l'extension
/// (`.yaml`, `.yml`, ou `.json`) et parse le contenu en conséquence.
///
/// # Arguments
///
/// * `path` - Chemin vers le fichier de configuration
///
/// # Returns
///
/// La [`CommandsConfig`] parsée en cas de succès.
///
/// # Errors
///
/// - [`ConfigError::FileNotFound`] si le fichier n'existe pas
/// - [`ConfigError::UnsupportedFormat`] si l'extension n'est pas supportée
/// - [`ConfigError::YamlParse`] ou [`ConfigError::JsonParse`] en cas d'erreurs de parsing
///
/// # Exemples
///
/// ```no_run
/// use dynamic_cli::config::load_config;
///
/// let config = load_config("commands.yaml")?;
/// println!("Chargé {} commandes", config.commands.len());
/// # Ok::<(), dynamic_cli::error::DynamicCliError>(())
/// ```
///
/// # Voir Aussi
///
/// - [`load_yaml`] - Parser directement du contenu YAML
/// - [`load_json`] - Parser directement du contenu JSON
pub fn load_config<P: AsRef<Path>>(path: P) -> Result<CommandsConfig> {
    // Implémentation
}
```

### Meilleures Pratiques de Documentation

- Écrivez en anglais (audience internationale) pour le code
- Utilisez grammaire et orthographe appropriées
- Soyez concis mais complet
- Incluez des exemples pratiques
- Liez vers documentation liée
- Mettez à jour la doc en changeant le code

### Générer la Documentation

```bash
# Générer et ouvrir la documentation
cargo doc --no-deps --open

# Vérifier les liens cassés
cargo doc --no-deps 2>&1 | grep warning

# Générer avec toutes les fonctionnalités
cargo doc --all-features --no-deps
```

---

## 🔀 Processus de Pull Request

### Avant de Soumettre

**Checklist :**
- [ ] Le code suit les directives de style (`cargo fmt`)
- [ ] Aucun avertissement clippy (`cargo clippy --all-features -- -D warnings`)
- [ ] Tous les tests passent (`cargo test --all-features`)
- [ ] La documentation est mise à jour
- [ ] Nouveaux tests ajoutés pour nouvelles fonctionnalités
- [ ] Messages de commit clairs
- [ ] Branche à jour avec main

### Template de PR

```markdown
## Description

Brève description des changements.

## Type de Changement

- [ ] Correction de bug (changement non-cassant corrigeant une issue)
- [ ] Nouvelle fonctionnalité (changement non-cassant ajoutant une fonctionnalité)
- [ ] Changement cassant (correction ou fonctionnalité causant un changement de fonctionnalité existante)
- [ ] Mise à jour de documentation

## Issues Liées

Corrige #(numéro d'issue)

## Tests

Décrivez comment vous avez testé vos changements :
- Cas de test ajoutés
- Tests manuels effectués
- Cas limites considérés

## Checklist

- [ ] Le code suit les directives de style
- [ ] Auto-revue complétée
- [ ] Le code est commenté où nécessaire
- [ ] Documentation mise à jour
- [ ] Pas de nouveaux avertissements
- [ ] Tests ajoutés
- [ ] Tous les tests passent
```

### Processus de Revue

1. **Vérifications automatiques** : CI doit passer
2. **Revue de code** : Au moins une approbation requise
3. **Discussion** : Répondre aux retours des reviewers
4. **Mise à jour** : Faire les changements demandés
5. **Approbation** : Obtenir l'approbation finale
6. **Merge** : Le mainteneur merge la PR

### Après le Merge

- Supprimez votre branche de fonctionnalité
- Mettez à jour votre fork :
  ```bash
  git checkout main
  git pull upstream main
  git push origin main
  ```

---

## 🤝 Communauté

### Obtenir de l'Aide

**Si vous avez besoin d'aide :**
- Vérifiez la documentation existante
- Cherchez dans les issues existantes
- Posez dans les discussions
- Créez une nouvelle issue

**Soyez respectueux et patient :**
- Les mainteneurs sont bénévoles
- Fournissez des informations complètes
- Soyez ouvert aux retours
- Faites un suivi des réponses

### Canaux de Communication

- **GitHub Issues** : Rapports de bugs et demandes de fonctionnalités
- **GitHub Discussions** : Questions et discussion générale
- **Pull Requests** : Contributions de code

### Reconnaissance

Nous valorisons toutes les contributions ! Les contributeurs sont reconnus dans :
- README du projet
- Notes de version
- Page des contributeurs GitHub

---

## 📜 Licence

En contribuant à dynamic-cli, vous acceptez que vos contributions soient sous licence MIT/Apache-2.0 double licence.

Sauf indication contraire explicite de votre part, toute contribution intentionnellement soumise pour inclusion dans le projet par vous, telle que définie dans la licence Apache-2.0, sera sous double licence comme ci-dessus, sans termes ou conditions supplémentaires.

---

## 🙏 Merci !

Vos contributions ont pour objet d'améliorer dynamic-cli. Que vous corrigiez une faute de frappe, signaliez un bug ou implémentiez une fonctionnalité majeure, nous apprécions votre effort et votre temps.

Bon codage ! 🚀

---

## 📖 Ressources Supplémentaires

**Apprendre Rust :**
- [Le Livre Rust](https://jimskapt.github.io/rust-book-fr/) (français)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rustlings](https://github.com/rust-lang/rustlings)

**Meilleures Pratiques Rust :**
- [Directives API Rust](https://rust-lang.github.io/api-guidelines/)
- [Livre Performance Rust](https://nnethercote.github.io/perf-book/)
- [Effective Rust](https://www.lurklurk.org/effective-rust/)

**Spécifique au Projet :**
- [Documentation API](https://docs.rs/dynamic-cli)
- [Exemples](./examples)
- [Journal des modifications](CHANGELOG.md)
---

**Dernière Mise à Jour** : 2026-01-11  
**Version** : 0.1.0
