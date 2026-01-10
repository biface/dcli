# dynamic-cli

[![Crates.io](https://img.shields.io/crates/v/dynamic-cli.svg)](https://crates.io/crates/dynamic-cli)
[![Documentation](https://docs.rs/dynamic-cli/badge.svg)](https://docs.rs/dynamic-cli)
[![License](https://img.shields.io/crates/l/dynamic-cli.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-blue.svg)](https://www.rust-lang.org)

[🇬🇧 English version](README.md)

**dynamic-cli** est un framework Rust pour créer rapidement des applications CLI (Command Line Interface) et REPL (Read-Eval-Print Loop) configurables via des fichiers YAML ou JSON.

Au lieu de coder manuellement chaque commande avec `clap` ou d'autres bibliothèques, vous définissez vos commandes dans un fichier de configuration, et **dynamic-cli** génère automatiquement :
- Le parser d'arguments
- La validation des entrées
- L'aide contextuelle
- Le mode interactif (REPL)
- La gestion d'erreurs avec suggestions intelligentes

## 🎯 Cas d'usage

- **Outils scientifiques** : simulateurs, analyseurs de données, outils de calcul
- **Gestionnaires de fichiers** : opérations batch configurables, navigation
- **Gestionnaires de tâches** : todo lists, suivi de projets, automatisation
- **Clients d'API** : interfaces interactives pour services web
- **Outils de build** : systèmes de compilation personnalisés, scripts de déploiement
- **Applications de tests** : frameworks de test configurables, test runners

## ✨ Fonctionnalités

- ✅ **Configuration déclarative** : définissez vos commandes en YAML/JSON
- ✅ **Double mode** : CLI classique OU REPL interactif (auto-détecté)
- ✅ **Validation automatique** : types, ranges, fichiers, choix multiples
- ✅ **Suggestions intelligentes** : correction de typos avec distance de Levenshtein
- ✅ **Gestion d'erreurs riche** : messages clairs avec contexte et suggestions
- ✅ **Historique REPL** : sauvegarde automatique entre sessions (via rustyline)
- ✅ **Extensible** : contexte personnalisé, validations custom
- ✅ **Type-safe** : traits Rust pour les implémentations
- ✅ **Fonctions utilitaires** : 18+ fonctions helper pour tâches courantes
- ✅ **Sortie colorée** : messages d'erreur user-friendly

## 🚀 Démarrage rapide

### Installation

Ajoutez dans votre `Cargo.toml` :

```toml
[dependencies]
dynamic-cli = "0.1"
```

### Exemple minimal

**1. Créer un fichier `commands.yaml` :**

```yaml
metadata:
  version: "1.0.0"
  prompt: "monapp"
  prompt_suffix: " > "

commands:
  - name: saluer
    aliases: [bonjour, hello]
    description: "Saluer quelqu'un"
    required: true
    arguments:
      - name: nom
        arg_type: string
        required: true
        description: "Nom à saluer"
        validation: []
    options: []
    implementation: "saluer_handler"

global_options: []
```

**2. Implémenter le handler en Rust :**

```rust
use dynamic_cli::prelude::*;
use std::collections::HashMap;

// Définir le contexte d'exécution (état partagé)
#[derive(Default)]
struct MonContexte {
    nombre_salutations: usize,
}

impl ExecutionContext for MonContexte {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// Implémenter le handler de commande
struct CommandeSaluer;

impl CommandHandler for CommandeSaluer {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<MonContexte>(context)
            .ok_or_else(|| /* gestion erreur */)?;
        
        let nom = args.get("nom").unwrap();
        println!("Bonjour, {} !", nom);
        ctx.nombre_salutations += 1;
        
        Ok(())
    }
}

fn main() -> Result<()> {
    CliBuilder::new()
        .config_file("commands.yaml")
        .context(Box::new(MonContexte::default()))
        .register_handler("saluer_handler", Box::new(CommandeSaluer))
        .build()?
        .run()
}
```

**3. Utiliser l'application :**

```bash
# Mode CLI (commande unique)
$ monapp saluer Alice
Bonjour, Alice !

# Mode REPL (interactif)
$ monapp
monapp > saluer Bob
Bonjour, Bob !
monapp > hello Charlie
Bonjour, Charlie !
monapp > exit
```

## 📦 Exemples complets

Le framework inclut trois exemples complets et production-ready démontrant différents niveaux de complexité :

### 1. Simple Calculator (Débutant)

Une calculatrice arithmétique basique avec historique.

```bash
# Lancer la calculatrice
cargo run --example simple_calculator

# Ou en mode CLI
cargo run --example simple_calculator -- add 10 5
```

**Fonctionnalités :**
- Opérations basiques : addition, soustraction, multiplication, division
- Historique des calculs
- Rappel du dernier résultat
- Gestion d'erreurs (division par zéro)

**Commandes :** `add`, `subtract`, `multiply`, `divide`, `history`, `clear`, `last`

---

### 2. File Manager (Intermédiaire)

Outil de navigation et d'information sur les fichiers avec validation de chemins.

```bash
# Lancer le gestionnaire de fichiers
cargo run --example file_manager

# Ou en mode CLI
cargo run --example file_manager -- list .
cargo run --example file_manager -- info Cargo.toml
```

**Fonctionnalités :**
- Lister le contenu des répertoires avec tailles
- Afficher les informations détaillées des fichiers
- Rechercher des fichiers par pattern
- Validation de chemins
- Tailles human-readable (Ko, Mo, Go)
- Suivi des statistiques

**Commandes :** `list`, `info`, `search`, `stats`

---

### 3. Task Runner (Avancé)

Système complet de gestion de tâches avec priorités et statistiques.

```bash
# Lancer le gestionnaire de tâches
cargo run --example task_runner

# Ou en mode CLI
cargo run --example task_runner -- add "Écrire docs" --priority high
cargo run --example task_runner -- list
```

**Fonctionnalités :**
- Ajouter des tâches avec priorités (low, medium, high)
- Lister les tâches en attente ou toutes les tâches
- Marquer les tâches comme complétées
- Supprimer des tâches
- Nettoyer les tâches complétées
- Statistiques avancées avec taux de complétion
- Validation personnalisée

**Commandes :** `add`, `list`, `complete`, `delete`, `clear`, `stats`

**Voir [examples/README.md](examples/README.md) pour la documentation détaillée.**

## 📖 Documentation complète

### Configuration des commandes

Le fichier de configuration définit toutes les commandes disponibles avec leurs arguments, options et règles de validation :

```yaml
commands:
  - name: calculer
    aliases: [calc, compute]
    description: "Effectuer des calculs"
    required: true
    
    arguments:
      - name: operation
        arg_type: string
        required: true
        description: "Opération : add, subtract, multiply, divide"
        validation: []
        
    options:
      - name: precision
        short: p
        long: precision
        option_type: integer
        required: false
        default: "2"
        description: "Nombre de décimales"
        choices: []
        
      - name: verbose
        short: v
        long: verbose
        option_type: bool
        required: false
        description: "Activer le mode verbeux"
        choices: []
    
    implementation: "calculer_handler"
```

### Types supportés

- **`string`** : chaîne de caractères (UTF-8)
- **`integer`** : nombre entier signé (i64)
- **`float`** : nombre à virgule flottante (f64)
- **`bool`** : booléen (accepte : true/false, yes/no, 1/0, on/off)
- **`path`** : chemin de fichier/dossier

### Règles de validation

Dynamic-cli fournit des validateurs intégrés applicables aux arguments :

```yaml
arguments:
  - name: fichier_config
    arg_type: path
    required: true
    validation:
      - must_exist: true
      - extensions: [yaml, yml, json]
      
  - name: pourcentage
    arg_type: float
    required: true
    validation:
      - min: 0.0
        max: 100.0
```

Validateurs disponibles :
- **`must_exist`** : le fichier/dossier doit exister
- **`extensions`** : le fichier doit avoir l'une des extensions spécifiées
- **`range`** : le nombre doit être dans les limites min/max

### Contexte d'exécution

Le contexte permet de partager l'état entre les commandes :

```rust
use dynamic_cli::prelude::*;

#[derive(Default)]
struct ContexteApp {
    fichier_courant: Option<PathBuf>,
    parametres: HashMap<String, String>,
    verbeux: bool,
}

impl ExecutionContext for ContexteApp {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

Utilisez les fonctions helper fournies pour un downcasting sûr :

```rust
// Dans votre handler
let ctx = downcast_mut::<ContexteApp>(context)
    .ok_or_else(|| /* gestion erreur */)?;
```

### Handlers de commandes

Chaque commande est implémentée via le trait `CommandHandler` :

```rust
use dynamic_cli::prelude::*;

struct MaCommande;

impl CommandHandler for MaCommande {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        // Récupérer le contexte typé
        let ctx = downcast_mut::<ContexteApp>(context)?;
        
        // Parser les arguments avec les fonctions utilitaires
        let compte = parse_int(args.get("compte").unwrap(), "compte")?;
        let verbeux = parse_bool(
            args.get("verbeux").unwrap_or(&"false".to_string())
        )?;
        
        // Valider
        if is_blank(args.get("nom").unwrap()) {
            return Err(/* erreur validation */);
        }
        
        // Exécuter la logique
        println!("Traitement de {} éléments", compte);
        
        Ok(())
    }
    
    // Optionnel : validation personnalisée au-delà de la config
    fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
        // Validations supplémentaires
        Ok(())
    }
}
```

### Fonctions utilitaires

Dynamic-cli fournit 18+ fonctions utilitaires pour les tâches courantes :

**Conversion de types :**
```rust
parse_int(value, field_name) -> Result<i64>
parse_float(value, field_name) -> Result<f64>
parse_bool(value) -> Result<bool>  // Accepte : true/false, yes/no, 1/0, on/off
detect_type(value) -> ArgumentType  // Auto-détection du type
```

**Validation de chaînes :**
```rust
is_blank(s) -> bool
normalize(s) -> String  // Trim + minuscules
truncate(s, max_len) -> String
is_valid_email(s) -> bool
```

**Manipulation de chemins :**
```rust
normalize_path(path) -> String  // Multi-plateforme
get_extension(path) -> Option<String>
has_extension(path, extensions) -> bool
```

**Formatage :**
```rust
format_bytes(bytes) -> String  // "2,50 Mo"
format_duration(duration) -> String  // "1h 30m 5s"
format_numbered_list(items) -> String
format_table(headers, rows) -> String
```

**Voir la documentation complète sur [docs.rs/dynamic-cli](https://docs.rs/dynamic-cli)**

## 🏗️ Architecture

```
dynamic-cli/
├── config/       Chargement et validation de la configuration
├── context/      Trait du contexte d'exécution
├── executor/     Logique d'exécution des commandes
├── registry/     Registre des commandes et handlers
├── parser/       Parsing des arguments CLI et REPL
├── validator/    Validation des arguments
├── interface/    Interfaces CLI et REPL
├── builder/      API builder fluide
├── utils/        Fonctions utilitaires
└── error/        Types d'erreur avec suggestions
```

## 🧪 Tests

```bash
# Lancer tous les tests
cargo test

# Lancer les tests avec couverture
cargo test --all-features

# Lancer un exemple spécifique
cargo run --example simple_calculator

# Lancer les benchmarks (si disponibles)
cargo bench
```

## 🔧 Utilisation avancée

### Validateurs personnalisés

Implémentez une validation personnalisée dans vos handlers :

```rust
impl CommandHandler for MaCommande {
    fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
        let valeur = parse_int(args.get("compte").unwrap(), "compte")?;
        if valeur < 1 || valeur > 1000 {
            return Err(ValidationError::OutOfRange {
                arg_name: "compte".to_string(),
                value: valeur as f64,
                min: 1.0,
                max: 1000.0,
            }.into());
        }
        Ok(())
    }
}
```

### Gestion d'erreurs

Dynamic-cli fournit des types d'erreur riches avec contexte :

```rust
use dynamic_cli::error::{DynamicCliError, ParseError};

// Les erreurs incluent des suggestions pour les typos
let error = ParseError::unknown_command_with_suggestions(
    "simulat",
    &["simulate", "validate"]
);
// Erreur : Commande inconnue : 'simulat'
// Vouliez-vous dire : simulate ?
```

### Historique REPL

Le mode REPL sauvegarde automatiquement l'historique des commandes entre les sessions via rustyline :

```bash
# L'historique est sauvegardé dans :
# - Linux/macOS : ~/.local/share/dynamic-cli/history
# - Windows : %APPDATA%\dynamic-cli\history
```

## 🎓 Parcours d'apprentissage

1. **Commencer avec Simple Calculator** (30 min)
   - Apprendre la structure de base des commandes
   - Comprendre la gestion du contexte
   - Parsing d'arguments simple

2. **Explorer File Manager** (45 min)
   - Validation de chemins
   - Opérations sur fichiers
   - Options et flags
   - Sortie formatée

3. **Étudier Task Runner** (1 heure)
   - Gestion d'état complexe
   - Validation personnalisée
   - Logique métier
   - Statistiques et reporting

**Voir [examples/README.md](examples/README.md) pour les guides détaillés.**

## 🤝 Contribution

Les contributions sont les bienvenues ! Merci de :

1. Forker le dépôt
2. Créer une branche de fonctionnalité
3. Ajouter des tests pour les nouvelles fonctionnalités
4. S'assurer que tous les tests passent (`cargo test`)
5. Soumettre une pull request

Pour les changements majeurs, merci d'ouvrir d'abord une issue pour discuter des modifications proposées.

## 📄 License

Ce projet est sous [LICENCE MIT](LICENSE)


## 🔗 Liens

- **Documentation** : [docs.rs/dynamic-cli](https://docs.rs/dynamic-cli)
- **Crates.io** : [crates.io/crates/dynamic-cli](https://crates.io/crates/dynamic-cli)
- **Dépôt** : [github.com/biface/dynamic-cli](https://github.com/biface/dcli)
- **Exemples** : [examples/](examples/)
- **Changelog** : [CHANGELOG.md](CHANGELOG.md)

## 🙏 Remerciements

Ce framework a été développé dans le cadre du projet **chrom-rs** (simulateur de chromatographie) et généralisé pour un usage plus large.

Remerciements particuliers à :
- La communauté Rust pour les excellents crates (serde, thiserror, rustyline)
- Les premiers utilisateurs et testeurs pour leurs retours précieux

