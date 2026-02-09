# dynamic-cli

[![Crates.io](https://img.shields.io/crates/v/dynamic-cli.svg)](https://crates.io/crates/dynamic-cli)
[![codecov](https://codecov.io/gh/biface/dcli/graph/badge.svg?token=58T5WKC802)](https://codecov.io/gh/biface/dcli)[![Documentation](https://docs.rs/dynamic-cli/badge.svg)](https://docs.rs/dynamic-cli)
[![Licence](https://img.shields.io/badge/licence-MIT%20OU%20Apache--2.0-blue.svg)](LICENSE-MIT)

Un framework Rust puissant pour créer des applications CLI et REPL configurables via des fichiers YAML/JSON.

**Définissez votre interface en ligne de commande dans un fichier de configuration, pas dans le code.** ✨

---

**[English](README.md)** | **Français**

---

## 🎯 Fonctionnalités

- **📝 Piloté par Configuration** : Définissez commandes, arguments et options en YAML/JSON
- **🔄 Modes CLI & REPL** : Support des modes ligne de commande et interactif
- **✅ Validation Automatique** : Vérification de type et validation de contraintes intégrées
- **🎨 Messages d'Erreur Riches** : Messages colorés et informatifs avec suggestions
- **🔌 Extensible** : Ajout facile de gestionnaires de commandes personnalisés
- **📚 Bien Documenté** : Documentation API complète et exemples
- **🧪 Testé Exhaustivement** : Couverture de tests >80% avec 345+ tests
- **⚡ Performance** : Abstractions sans coût avec parsing efficace

---

## 🚀 Démarrage Rapide

### Installation

Ajoutez à votre `Cargo.toml` :

```toml
[dependencies]
dynamic-cli = "0.1.1"
```

### Exemple Basique

**1. Créez un fichier de configuration** (`commands.yaml`) :

```yaml
metadata:
  version: "1.0.0"
  prompt: "monapp"
  prompt_suffix: " > "

commands:
  - name: saluer
    aliases: [bonjour, salut]
    description: "Saluer quelqu'un"
    required: false
    arguments:
      - name: nom
        arg_type: string
        required: true
        description: "Nom à saluer"
        validation: []
    options:
      - name: fort
        short: f
        long: fort
        option_type: bool
        required: false
        description: "Utiliser les majuscules"
        choices: []
    implementation: "saluer_handler"

global_options: []
```

> Note :
> 
>  La syntaxe du fichier de configuration est disponible dans [cet espace projet](CONFIG_SYNTAX_REFERENCE.fr.md) 

**2. Implémentez vos gestionnaires de commandes** :

```rust
use dynamic_cli::prelude::*;
use std::collections::HashMap;

// Définissez le contexte de votre application
#[derive(Default)]
struct MonContexte {
    // L'état de votre application
}

impl ExecutionContext for MonContexte {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

// Implémentez le gestionnaire de commande
struct CommandeSaluer;

impl CommandHandler for CommandeSaluer {
    fn execute(
        &self,
        _context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> dynamic_cli::Result<()> {
        let nom = args.get("nom").unwrap();
        let fort = args.get("fort").map(|v| v == "true").unwrap_or(false);
        
        let salutation = format!("Bonjour, {} !", nom);
        println!("{}", if fort { salutation.to_uppercase() } else { salutation });
        
        Ok(())
    }
}

fn main() -> dynamic_cli::Result<()> {
    CliBuilder::new()
        .config_file("commands.yaml")
        .context(Box::new(MonContexte::default()))
        .register_handler("saluer_handler", Box::new(CommandeSaluer))
        .build()?
        .run()
}
```

**3. Exécutez votre application** :

```bash
# Mode CLI
$ monapp saluer Alice
Bonjour, Alice !

$ monapp saluer Bob --fort
BONJOUR, BOB !

# Mode REPL
$ monapp
monapp > saluer Alice
Bonjour, Alice !
monapp > help
Commandes disponibles :
  saluer [nom] - Saluer quelqu'un
monapp > exit
```

---

## 📖 Documentation

- **[Référence API](https://docs.rs/dynamic-cli)** - Documentation API complète
- **[Exemples](examples/README.md)** - Exemples fonctionnels et échantillons de code
- **[Guide de Contribution](CONTRIBUTING.fr.md)** - Comment contribuer au projet

---

## 🎓 Exemples

Le [répertoire d'exemples](examples) contient des exemples complets :

- **[simple_calculator.rs](examples/simple_calculator.rs)** - Calculatrice arithmétique basique
- **[file_manager.rs](examples/file_manager.rs)** - Opérations sur fichiers avec validation
- **[task_runner.rs](examples/task_runner.rs)** - Application de gestion de tâches

Exécutez n'importe quel exemple :
```bash
cargo run --example simple_calculator
```

---

## 🏗 Architecture

dynamic-cli est organisé en modules ciblés :

- **config** - Chargement et validation de configuration
- **context** - Trait de contexte d'exécution
- **executor** - Moteur d'exécution de commandes
- **registry** - Registre de commandes et gestionnaires
- **parser** - Parsing d'arguments CLI et REPL
- **validator** - Validation d'arguments
- **interface** - Interfaces CLI et REPL
- **error** - Types d'erreurs et affichage
- **builder** - API fluide pour construire des applications

---

## 🧪 Tests

```bash
# Exécuter tous les tests
cargo test --all-features

# Exécuter avec couverture
cargo tarpaulin --out Html

# Vérifier la qualité du code
cargo clippy --all-features -- -D warnings
```

**Statistiques de tests actuelles :**
- **345+ tests unitaires** ✅
- **126+ tests de documentation**
- **Couverture de code 80-90%**
- **Zéro avertissement clippy**

---

## 🤝 Contribuer

Nous accueillons les contributions de tous ! Voici comment vous pouvez aider :

### Façons de Contribuer

- 🐛 **Signaler des bugs** - Trouvé un bug ? [Ouvrez une issue](https://github.com/biface/dcli/issues)
- 💡 **Suggérer des fonctionnalités** - Vous avez une idée ? [Démarrez une discussion](https://github.com/biface/dcli/discussions)
- 📝 **Améliorer la documentation** - Corrigez des fautes, clarifiez, ajoutez des exemples
- 🔧 **Soumettre du code** - Corrigez des bugs, implémentez des fonctionnalités, améliorez les performances
- 🧪 **Ajouter des tests** - Augmentez la couverture, ajoutez des cas limites

### Démarrage

```bash
# Forkez et clonez
git clone https://github.com/biface/dcli.git
cd dynamic-cli

# Créez une branche
git checkout -b feature/ma-fonctionnalite

# Faites vos modifications et testez
cargo test --all-features
cargo clippy --all-features

# Commitez et poussez
git commit -am "Ajout d'une super fonctionnalité"
git push origin feature/ma-fonctionnalite
```

### Directives de Développement

**Avant de soumettre une `pull request` :**

- [ ] Le code suit les directives de style Rust (`cargo fmt`)
- [ ] Tous les tests passent (`cargo test --all-features`)
- [ ] Aucun avertissement clippy (`cargo clippy --all-features -- -D warnings`)
- [ ] La documentation est mise à jour
- [ ] De nouveaux tests sont ajoutés pour les nouvelles fonctionnalités
- [ ] Les messages de commit sont clairs et descriptifs

### Code de Conduite

Ce projet suit un Code de Conduite pour assurer un environnement accueillant :

- ✅ Soyez respectueux avec autrui
- ✅ Accueillez les nouveaux venus et aidez-les à apprendre
- ✅ Acceptez gracieusement les critiques constructives
- ✅ Concentrez-vous sur ce qui est le mieux pour la communauté
- ❌ Pas de harcèlement, trolling ou attaques personnelles

**[Lisez le guide complet de contribution →](CONTRIBUTING.fr.md)**

---

## 📜 Licence

Sous licence au choix :

 * Licence Apache, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) ou http://www.apache.org/licenses/LICENSE-2.0)
 * Licence MIT
   ([LICENSE-MIT](LICENSE-MIT) ou http://opensource.org/licenses/MIT)

### Licence de Contribution

Sauf indication contraire explicite de votre part, toute contribution intentionnellement soumise pour inclusion dans le projet par vous, telle que définie dans la licence Apache-2.0, sera sous double licence comme ci-dessus, sans termes ou conditions supplémentaires.

---

## 🙏 Remerciements

- **Communauté Rust** - Pour les outils et bibliothèques développées
- **Contributeurs** - Tous ceux qui ont contribué à ce projet
- **[clap](https://github.com/clap-rs/clap)** - Inspiration pour la conception CLI
- **[rustyline](https://github.com/kkawakam/rustyline)** - Fonctionnalité REPL
- **[serde](https://github.com/serde-rs/serde)** - Support de sérialisation

---

## 📞 Support

**Besoin d'aide ?**

- 📖 Consultez la [documentation API](https://docs.rs/dynamic-cli)
- 💬 Ouvrez une [discussion](https://github.com/biface/dcli/discussions)
- 🐛 Signalez une [issue](https://github.com/biface/dcli/issues)
- 📧 Contactez les mainteneurs

**Trouvé une vulnérabilité de sécurité ?**  
Veuillez la signaler en privé aux mainteneurs.

---

## 🌟 Montrez Votre Support

Si vous trouvez dynamic-cli utile, veuillez :

- ⭐ **Étoiler le dépôt** sur GitHub
- 📢 **Partager** avec d'autres qui pourraient le trouver utile
- 📝 **Écrire** un article de blog ou un tutoriel !

**Dernière mise à jour** : 2026-01-12
