# dynamic-cli

[![Crates.io](https://img.shields.io/crates/v/dynamic-cli.svg)](https://crates.io/crates/dynamic-cli)
[![Documentation](https://docs.rs/dynamic-cli/badge.svg)](https://docs.rs/dynamic-cli)
[![License](https://img.shields.io/crates/l/dynamic-cli.svg)](LICENSE-MIT)

[🇬🇧 English version](README.md)

**dynamic-cli** est un framework Rust pour créer rapidement des applications CLI (Command Line Interface) et REPL (Read-Eval-Print Loop) configurables via des fichiers YAML ou JSON.

Au lieu de coder manuellement chaque commande avec `clap` ou d'autres bibliothèques, vous définissez vos commandes dans un fichier de configuration, et **dynamic-cli** génère automatiquement :
- Le parser d'arguments
- La validation des entrées
- L'aide contextuelle
- Le mode interactif (REPL)
- La gestion d'erreurs avec suggestions

## 🎯 Cas d'usage

- **Outils scientifiques** : simulateurs, analyseurs de données
- **Gestionnaires de fichiers** : opérations batch configurables
- **Clients d'API** : interfaces interactives pour services web
- **Outils de build** : systèmes de compilation personnalisés
- **Applications de tests** : frameworks de test configurables

## ✨ Fonctionnalités

- ✅ **Configuration déclarative** : définissez vos commandes en YAML/JSON
- ✅ **Double mode** : CLI classique OU REPL interactif (auto-détecté)
- ✅ **Validation automatique** : types, ranges, fichiers, choix multiples
- ✅ **Suggestions intelligentes** : correction de typos avec distance de Levenshtein
- ✅ **Gestion d'erreurs riche** : messages clairs avec contexte
- ✅ **Historique REPL** : sauvegarde automatique entre sessions
- ✅ **Extensible** : contexte personnalisé, validations custom
- ✅ **Type-safe** : traits Rust pour les implémentations

## 🚀 Démarrage rapide

### Installation

```toml
[dependencies]
dynamic-cli = "0.1"
```

### Exemple minimal

**1. Créer un fichier `commands.yaml` :**

```yaml
metadata:
  version: "1.0"
  prompt: "my-app"
  prompt_suffix: " > "

commands:
  - name: "greet"
    description: "Greet someone"
    arguments:
      - name: "name"
        type: "string"
        required: true
        description: "Name to greet"
    options: []
    implementation: "greet_handler"

global_options: []
```

**2. Implémenter le handler en Rust :**

```rust
use dynamic_cli::{
    CliBuilder, CommandHandler, ExecutionContext,
    Result,
};
use std::collections::HashMap;

// Définir le contexte d'exécution (état partagé)
#[derive(Default)]
struct MyContext {
    greeting_count: usize,
}

impl ExecutionContext for MyContext {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// Implémenter le handler de commande
struct GreetCommand;

impl CommandHandler for GreetCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = context.downcast_mut::<MyContext>().unwrap();
        let name = args.get("name").unwrap();
        
        println!("Hello, {}!", name);
        ctx.greeting_count += 1;
        
        Ok(())
    }
}

fn main() -> Result<()> {
    // Construire et lancer l'application
    CliBuilder::new()
        .config_file("commands.yaml")
        .context(Box::new(MyContext::default()))
        .register_handler("greet_handler", Box::new(GreetCommand))
        .build()?
        .run()
}
```

**3. Utiliser l'application :**

```bash
# Mode CLI
$ my-app greet Alice
Hello, Alice!

# Mode REPL (interactif)
$ my-app
my-app > greet Bob
Hello, Bob!
my-app > exit
```

## 📖 Documentation complète

### Configuration des commandes

Le fichier de configuration définit toutes les commandes disponibles :

```yaml
commands:
  - name: "calculate"
    aliases: ["calc", "compute"]
    description: "Perform calculations"
    
    arguments:
      - name: "operation"
        type: "string"
        required: true
        description: "Operation to perform (+, -, *, /)"
        
    options:
      - name: "precision"
        short: "p"
        long: "precision"
        type: "integer"
        required: false
        default: "2"
        description: "Number of decimal places"
        
      - name: "verbose"
        short: "v"
        long: "verbose"
        type: "bool"
        default: "false"
        description: "Enable verbose output"
    
    implementation: "calculate_handler"
```

### Types supportés

- **`string`** : chaîne de caractères
- **`integer`** : nombre entier (i64)
- **`float`** : nombre décimal (f64)
- **`bool`** : booléen (true/false)
- **`path`** : chemin de fichier/dossier

### Validation

```yaml
arguments:
  - name: "config_file"
    type: "path"
    required: true
    validation:
      - must_exist: true
      - extensions: [".yaml", ".yml", ".json"]
      
  - name: "percentage"
    type: "float"
    validation:
      - range:
          min: 0.0
          max: 100.0
```

### Contexte d'exécution

Le contexte permet de partager l'état entre les commandes :

```rust
#[derive(Default)]
struct AppContext {
    current_file: Option<PathBuf>,
    settings: HashMap<String, String>,
    verbose: bool,
}

impl ExecutionContext for AppContext {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}
```

### Handlers de commandes

Chaque commande est implémentée via le trait `CommandHandler` :

```rust
struct MyCommand;

impl CommandHandler for MyCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        // Récupérer le contexte typé
        let ctx = context.downcast_mut::<AppContext>().unwrap();
        
        // Récupérer les arguments
        let value = args.get("some_arg").unwrap();
        
        // Exécuter la logique
        println!("Doing something with: {}", value);
        
        Ok(())
    }
    
    // Validation personnalisée (optionnel)
    fn validate(&self, args: &HashMap<String, String>) -> Result<()> {
        // Validations supplémentaires au-delà de la config
        Ok(())
    }
}
```

## 🧪 Tests

```bash
# Tests unitaires
cargo test

# Tests d'intégration
cargo test --test '*'

# Benchmarks
cargo bench

# Couverture de code (avec tarpaulin)
cargo tarpaulin --out Html
```

## 📦 Exemples

Le dépôt contient plusieurs exemples complets :

```bash
# Calculatrice simple
cargo run --example simple_calculator

# Gestionnaire de fichiers
cargo run --example file_manager

# Task runner
cargo run --example task_runner
```

## 🤝 Contribution

Les contributions sont les bienvenues ! Consultez [CONTRIBUTING.md](CONTRIBUTING.md) pour les guidelines.

## 📄 License

Ce projet est sous licence MIT. Voir [LICENSE-MIT](LICENSE-MIT).

## 🔗 Liens

- [Documentation API](https://docs.rs/dynamic-cli)
- [Crates.io](https://crates.io/crates/dynamic-cli)
- [Dépôt GitHub](https://github.com/votre-org/dynamic-cli)
- [Exemples](https://github.com/votre-org/dynamic-cli/tree/main/examples)

## 🙏 Remerciements

Inspiré par les besoins du projet **chrom-rs** (simulateur de chromatographie).
