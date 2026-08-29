# Référence de Syntaxe - Fichier de Configuration dynamic-cli

**Version** : 1.0  
**Dernière mise à jour** : 2026-08-29  
**Format** : YAML ou JSON

---

## Table des Matières

- [Vue d'ensemble](#vue-densemble)
- [Format de fichier](#format-de-fichier)
- [Structure racine](#structure-racine)
- [Section metadata](#section-metadata)
- [Options globales](#options-globales)
- [Section commands](#section-commands)
- [Définition de commande](#définition-de-commande)
- [Arguments](#arguments)
- [Options](#options)
- [Options répétables](#options-répétables)
- [Chaînage de commandes](#chaînage-de-commandes)
- [Types d'arguments](#types-darguments)
- [Règles de validation](#règles-de-validation)
- [Exemple complet](#exemple-complet)
- [Bonnes pratiques](#bonnes-pratiques)

---

## Vue d'ensemble

Le fichier de configuration définit toutes les commandes CLI et le comportement REPL pour les applications construites avec `dynamic-cli`.

**Fonctionnalités clés** :
- Définir des commandes sans écrire de code
- Support des arguments positionnels et des options
- Validation et vérification de type automatiques
- Extensible avec des handlers personnalisés

**Formats supportés** :
- YAML (`.yaml`, `.yml`) - Recommandé pour la lisibilité
- JSON (`.json`) - Compatible avec les outils existants

---

## Format de fichier

### Exemple YAML

```yaml
metadata:
  version: "1.0.0"
  prompt: "monapp"
  prompt_suffix: " > "

global_options:
  - name: "verbose"
    # ...

commands:
  - name: "init"
    # ...
```

### Exemple JSON

```json
{
  "metadata": {
    "version": "1.0.0",
    "prompt": "monapp",
    "prompt_suffix": " > "
  },
  "global_options": [
    {
      "name": "verbose"
    }
  ],
  "commands": [
    {
      "name": "init"
    }
  ]
}
```

**Note** : Les exemples YAML sont utilisés dans ce document pour la lisibilité.

---

## Structure racine

Le fichier de configuration comporte **trois sections principales** :

```yaml
metadata:          # Métadonnées de l'application
  # ...

global_options:    # Options disponibles pour TOUTES les commandes
  # ...

commands:          # Liste des commandes disponibles
  # ...
```

**Les trois sections sont obligatoires**, même si `global_options` est vide.

---

## Section metadata

Informations au niveau de l'application.

### Structure

```yaml
metadata:
  version: string        # Obligatoire - Version de la configuration
  prompt: string         # Obligatoire - Texte du prompt REPL
  prompt_suffix: string  # Obligatoire - Suffixe après le prompt (ex : " > ")
```

### Champs

| Champ           | Type   | Obligatoire  | Description                                                            |
|-----------------|--------|--------------|------------------------------------------------------------------------|
| `version`       | string | ✅ Oui        | Version du fichier de configuration (versioning sémantique recommandé) |
| `prompt`        | string | ✅ Oui        | Texte affiché en mode REPL (ex : "monapp", "rpn")                      |
| `prompt_suffix` | string | ✅ Oui        | Texte après le prompt (typiquement `" > "` ou `"$ "`)                  |

### Exemple

```yaml
metadata:
  version: "1.0.0"
  prompt: "rpn"
  prompt_suffix: " > "
```

**Affichage REPL** :
```
rpn > _
```

---

## Options globales

Options qui sont **disponibles pour TOUTES les commandes** de l'application.

### Structure

```yaml
global_options:
  - name: string              # Obligatoire - Identifiant unique
    short: string             # Optionnel - Caractère unique (ex : "v")
    long: string              # Optionnel - Forme longue (ex : "verbose")
    type: ArgumentType        # Obligatoire - Type de données
    required: boolean         # Obligatoire - Doit être fourni ?
    default: string           # Optionnel - Valeur par défaut si non fourni
    description: string       # Obligatoire - Texte d'aide
    choices: [string]         # Optionnel - Liste des valeurs valides
```

### Champs

| Champ         | Type    | Obligatoire  | Description                                                 |
|---------------|---------|--------------|-------------------------------------------------------------|
| `name`        | string  | ✅ Oui        | Identifiant interne (utilisé dans les handlers)             |
| `short`       | string  | ⬜ Non        | Forme courte du flag (caractère unique, ex : "v" pour `-v`) |
| `long`        | string  | ⬜ Non        | Forme longue du flag (ex : "verbose" pour `--verbose`)      |
| `type`        | string  | ✅ Oui        | Un parmi : `string`, `integer`, `float`, `bool`, `path`     |
| `required`    | boolean | ✅ Oui        | Si `true`, doit être fourni par l'utilisateur               |
| `default`     | string  | ⬜ Non        | Valeur par défaut (comme chaîne, sera parsée selon le type) |
| `description` | string  | ✅ Oui        | Texte d'aide pour l'utilisateur                             |
| `choices`     | array   | ⬜ Non        | Restreindre les valeurs à une liste spécifique              |

### Exemples

**Flag verbose** :
```yaml
- name: "verbose"
  short: "v"
  long: "verbose"
  type: "bool"
  required: false
  default: "false"
  description: "Activer la sortie verbeuse"
```

**Utilisation** :
```bash
monapp -v commande
monapp --verbose commande
```

**Répertoire de sortie** :
```yaml
- name: "output"
  short: "o"
  long: "output"
  type: "path"
  required: false
  description: "Répertoire de sortie pour les résultats"
```

**Utilisation** :
```bash
monapp -o /tmp/resultats commande
monapp --output ./sortie commande
```

**Sélection de format** :
```yaml
- name: "format"
  short: "f"
  long: "format"
  type: "string"
  required: false
  default: "text"
  description: "Format de sortie"
  choices: ["text", "json", "yaml"]
```

**Utilisation** :
```bash
monapp --format json commande
monapp -f yaml commande
```

### Notes

- Au moins un de `short` ou `long` doit être fourni
- Les deux peuvent être fournis pour une flexibilité maximale
- Les options globales sont passées à **tous** les handlers de commandes

---

## Section commands

Liste des commandes disponibles dans l'application.

### Structure

```yaml
commands:
  - name: string                    # Obligatoire - Nom de la commande
    aliases: [string]               # Obligatoire - Noms alternatifs (peut être vide)
    description: string             # Obligatoire - Texte d'aide
    required: boolean               # Obligatoire - Doit être exécutée dans le workflow ?
    arguments: [ArgumentDefinition] # Obligatoire - Arguments positionnels (peut être vide)
    options: [OptionDefinition]     # Obligatoire - Options spécifiques à la commande (peut être vide)
    implementation: string          # Obligatoire - Nom de la fonction handler
```

### Champs

| Champ            | Type    | Obligatoire | Description                                                                 |
|------------------|---------|-------------|-----------------------------------------------------------------------------|
| `name`           | string  | ✅ Oui       | Nom principal de la commande (pas d'espaces, minuscules recommandées)       |
| `aliases`        | array   | ✅ Oui       | Noms alternatifs (ex : `["quitter", "q"]` pour exit)                        |
| `description`    | string  | ✅ Oui       | Texte d'aide pour l'utilisateur                                             |
| `required`       | boolean | ✅ Oui       | Si `true`, la commande doit être exécutée dans le workflow de l'application |
| `arguments`      | array   | ✅ Oui       | Liste des arguments positionnels (utiliser `[]` si aucun)                   |
| `options`        | array   | ✅ Oui       | Options spécifiques à la commande (utiliser `[]` si aucune)                 |
| `implementation` | string  | ✅ Oui       | Identifiant pour le handler de commande (référencé dans le code)            |

### Exemple

```yaml
commands:
  - name: "input"
    aliases: ["charger", "ouvrir"]
    description: "Charger un fichier de configuration"
    required: true
    arguments:
      - name: "fichier"
        type: "path"
        required: true
        description: "Chemin vers le fichier d'entrée"
        validation:
          - must_exist: true
          - extensions: [".yaml", ".json"]
    options: []
    implementation: "load_config"
```

**Utilisation** :
```bash
monapp input config.yaml
monapp charger config.yaml
monapp ouvrir config.json
```

---

## Définition de commande

Détail complet d'une commande individuelle.

### Structure complète

```yaml
- name: "nom-commande"
  aliases: ["alt1", "alt2"]
  description: "Ce que fait cette commande"
  required: false
  
  arguments:
    - name: "arg1"
      type: "string"
      required: true
      description: "Premier argument"
      validation: []
    
    - name: "arg2"
      type: "integer"
      required: false
      description: "Deuxième argument optionnel"
      validation:
        - min: 1
          max: 100
  
  options:
    - name: "option1"
      short: "o"
      long: "option"
      type: "bool"
      required: false
      default: "false"
      description: "Activer l'option"
      choices: []
  
  implementation: "nom_handler"
  continue_on_failure: false
  requires_success: false
```

### Détails des champs

**`name`** (chaîne de caractères, obligatoire) :
- Identifiant principal de la commande
- Utilisé en CLI : `monapp <nom>`
- Convention : minuscules, pas d'espaces, utiliser des traits d'union pour les mots multiples (ex : `lancer-simulation`)

**`aliases`** (liste de chaînes de caractères, obligatoire) :
- Noms alternatifs pour la même commande
- Peut être vide : `aliases: []`
- Exemple : `aliases: ["quitter", "q", "sortir"]`
- Tous les alias invoquent le même handler

**`description`** (chaîne de caractères, obligatoire) :
- Texte d'aide pour l'utilisateur
- Affiché dans la sortie de la commande `help`
- Devrait être concis mais clair (une phrase recommandée)

**`required`** (booléen, obligatoire) :
- `true` : La commande doit être exécutée dans le workflow de l'application
- `false` : La commande est optionnelle
- La logique de l'application doit l'appliquer (non automatique)

**`arguments`** (liste, obligatoire) :
- Liste des arguments positionnels
- L'ordre compte (parsés de gauche à droite)
- Peut être vide : `arguments: []`
- Voir section [Arguments](#arguments) pour les détails

**`options`** (liste, obligatoire) :
- Flags/options spécifiques à la commande
- Indépendantes de `global_options`
- Peut être vide : `options: []`
- Voir section [Options](#options) pour les détails

**`implementation`** (chaîne de caractères, obligatoire) :
- Identifiant utilisé dans le code Rust pour lier au handler
- Convention : snake_case
- Exemple : `"load_config"` correspond à `LoadConfigHandler`

**`continue_on_failure`** (booléen, optionnel, défaut `false`) :
- Gouverne le chaînage de commandes (v0.8.0, [DD-026](https://github.com/biface/dcli/issues/52)) — voir la section [Chaînage de commandes](#chaînage-de-commandes) pour la vue d'ensemble
- Sans effet quand cette commande n'appartient pas à une chaîne

**`requires_success`** (booléen, optionnel, défaut `false`) :
- Fait aussi partie du chaînage de commandes ([DD-026](https://github.com/biface/dcli/issues/52)) — voir [Chaînage de commandes](#chaînage-de-commandes)
- **Sans rapport avec `required` ci-dessus** — `required` est un
  contrôle au démarrage (« un handler doit être enregistré »),
  `requires_success` est une garde de chaîne évaluée à chaque
  invocation, au moment du dispatch

> **Sync vs async n'est pas une question de configuration (ref. #8).** Ce
> champ est agnostique quant à la nature sync ou async du handler qui s'y
> rattache — il n'existe pas de drapeau `async: true`, et il n'y en aura
> pas. Le choix entre exécution synchrone ou asynchrone se fait entièrement
> côté Rust, selon la méthode appelée lors du câblage du handler :
> ```rust, ignore
> // Handler synchrone
> .register_sync_handler("load_config", Box::new(LoadConfigHandler))
> // Handler asynchrone — même convention de nom, même YAML
> .register_async_handler("fetch_remote", Box::new(FetchRemoteHandler))
> ```
> `CliBuilder::build()` résout chaque nom `implementation` selon le type
> effectivement enregistré. Ajouter un drapeau au niveau de la config
> créerait une seconde source de vérité susceptible de diverger de
> l'enregistrement Rust réel — seul le câblage Rust fait autorité, et c'est
> voulu.

---

## Arguments

Arguments positionnels pour les commandes.

### Structure

```yaml
arguments:
  - name: string              # Obligatoire - Identifiant
    type: ArgumentType        # Obligatoire - Type de données
    required: boolean         # Obligatoire - Doit être fourni ?
    description: string       # Obligatoire - Texte d'aide
    validation: [Rule]        # Obligatoire - Règles de validation (peut être vide)
```

### Champs

| Champ         | Type    | Obligatoire | Description                                             |
|---------------|---------|-------------|---------------------------------------------------------|
| `name`        | chaîne  | ✅ Oui       | Identifiant interne de l'argument                       |
| `type`        | chaîne  | ✅ Oui       | Un parmi : `string`, `integer`, `float`, `bool`, `path` |
| `required`    | booléen | ✅ Oui       | Si `true`, l'utilisateur doit fournir cet argument      |
| `description` | chaîne  | ✅ Oui       | Texte d'aide pour l'utilisateur                         |
| `validation`  | liste   | ✅ Oui       | Règles de validation (utiliser `[]` si aucune)          |

### Exemples

**Argument chemin de fichier** :
```yaml
- name: "fichier"
  type: "path"
  required: true
  description: "Chemin du fichier d'entrée"
  validation:
    - must_exist: true
    - extensions: [".yaml", ".json", ".yml"]
```

**Utilisation** : `monapp charger config.yaml`

**Plage numérique** :
```yaml
- name: "iterations"
  type: "integer"
  required: true
  description: "Nombre d'itérations"
  validation:
    - min: 1
      max: 10000
```

**Utilisation** : `monapp simuler 5000`

**Chaîne optionnelle** :
```yaml
- name: "nom-sortie"
  type: "string"
  required: false
  description: "Nom du fichier de sortie"
  validation: []
```

**Utilisation** : 
```bash
monapp exporter resultat.csv    # Fournit le nom
monapp exporter                  # Utilise la valeur par défaut
```

### L'ordre compte

Les arguments sont parsés **dans l'ordre** de gauche à droite :

```yaml
arguments:
  - name: "source"      # Premier positionnel
    type: "path"
    required: true
  
  - name: "destination" # Deuxième positionnel
    type: "path"
    required: true
```

**Utilisation** : `monapp copier source.txt dest.txt`

---

## Options

Flags et options spécifiques aux commandes.

### Structure

```yaml
options:
  - name: string              # Obligatoire - Identifiant
    short: string             # Optionnel - Caractère unique
    long: string              # Optionnel - Forme longue
    type: ArgumentType        # Obligatoire - Type de données
    required: boolean         # Obligatoire - Doit être fourni ?
    default: string           # Optionnel - Valeur par défaut
    description: string       # Obligatoire - Texte d'aide
    choices: [string]         # Optionnel - Valeurs valides
```

### Champs

Identiques aux champs des [Options globales](#options-globales).

### Exemples

**Flag booléen** :
```yaml
- name: "recursif"
  short: "r"
  long: "recursif"
  type: "bool"
  required: false
  default: "false"
  description: "Traiter les répertoires récursivement"
```

**Utilisation** :
```bash
monapp traiter -r /chemin
monapp traiter --recursif /chemin
```

**Option entière** :
```yaml
- name: "threads"
  short: "t"
  long: "threads"
  type: "integer"
  required: false
  default: "4"
  description: "Nombre de threads parallèles"
```

**Utilisation** :
```bash
monapp simuler --threads 8
monapp simuler -t 8
monapp simuler              # Utilise la valeur par défaut : 4
```

**Restriction par choix** :
```yaml
- name: "niveau"
  short: "n"
  long: "niveau"
  type: "string"
  required: false
  default: "info"
  description: "Niveau de journalisation"
  choices: ["debug", "info", "warn", "error"]
```

**Utilisation** :
```bash
monapp lancer --niveau debug
monapp lancer -n warn
monapp lancer                   # Utilise la valeur par défaut : info
monapp lancer -n invalide       # ERREUR : choix invalide
```

---

## Options répétables

*Depuis la v0.6.0 ([DD-024](https://github.com/biface/dcli/issues/21)).*

Une option répétable peut apparaître plusieurs fois sur la même ligne de
commande. Chaque occurrence commence par un **discriminant** — un jeton nu
immédiatement après le drapeau, tiré de `choices` — suivi de zéro ou
plusieurs sous-paramètres `clé=valeur`. `dynamic-cli` accumule chaque
occurrence dans l'ordre de la ligne de commande ; le handler les reçoit
toutes.

### Structure

```yaml
options:
  - name: string                          # Requis
    long: string                          # Optionnel
    short: string                         # Optionnel
    option_type: ArgumentType             # Requis
    required: boolean                     # Requis
    description: string                   # Requis
    choices: [string]                     # Requis si repeatable — discriminants valides
    repeatable: true                      # Requis pour activer ce comportement
    option_parameters:                    # Requis — une entrée par discriminant
      <discriminant>:
        - name: string
          arg_type: ArgumentType
          required: boolean
          description: string
          validation: []                  # Optionnel
          secure: false                   # Optionnel
```

### Règles

- `choices` doit être non vide et liste les discriminants valides.
- Les clés de `option_parameters` doivent correspondre **exactement** à
  `choices` — aucune clé orpheline, aucun discriminant non déclaré. Le
  chargement de la configuration échoue sinon.
- Chaque entrée `option_parameters[discriminant]` est une
  [`ArgumentDefinition`](#arguments) classique (mêmes champs que les
  `arguments:` positionnels), mais seuls `name` / `arg_type` / `required` /
  `description` / `validation` / `secure` ont un sens — l'ordre n'a aucun
  effet puisque les sous-paramètres sont associés par clé, pas par
  position.
- `default` n'est **pas autorisé** quand `repeatable: true` : l'absence de
  l'option signifie déjà zéro occurrence, une valeur par défaut implicite
  serait ambiguë.
- `option_parameters` sur une option avec `repeatable: false` (ou absent)
  est une erreur de configuration.
- Deux occurrences du même discriminant avec des paires `clé=valeur`
  différentes sont toutes deux conservées. Deux occurrences **strictement
  identiques** (même discriminant, mêmes paires clé=valeur) sont rejetées
  — c'est le seul chevauchement que le framework peut détecter sans
  connaissance du domaine ; les occurrences partiellement chevauchantes
  (même `file`, `resolution` différente, par exemple) sont acceptées et
  laissées à la charge du handler.

### Exemple

```yaml
- name: "output"
  long: "output"
  option_type: "string"
  required: false
  description: "Écrit les résultats sous une ou plusieurs formes de sortie"
  choices: ["csv", "plot"]
  repeatable: true
  option_parameters:
    csv:
      - name: "file"
        arg_type: "path"
        required: true
        description: "Fichier CSV de destination"
      - name: "resolution"
        arg_type: "integer"
        required: false
        description: "Résolution du pas de temps"
    plot:
      - name: "file"
        arg_type: "path"
        required: true
        description: "Fichier image de destination"
```

**Utilisation** :
```bash
monapp export --output csv file=resultats.csv resolution=100
monapp export --output csv file=resultats.csv --output plot file=graphe.png
monapp export --output csv file=a.csv --output csv file=b.csv   # les deux conservées — `file` différent
monapp export --output csv file=a.csv --output csv file=a.csv   # ERREUR : DuplicateOptionOccurrence
monapp export --output json file=out.json                       # ERREUR : UnknownDiscriminant
monapp export --output csv                                      # ERREUR : MissingRequiredOptionParameter (file)
monapp export --output csv file=a.csv depth=3                   # ERREUR : UnknownOptionParameter (depth)
```

### Côté handler

`CommandHandler::execute` (et `validate`) reçoivent désormais une valeur
`ParsedArgs` (voir la
[documentation du module `executor`](https://docs.rs/dynamic-cli/latest/dynamic_cli/executor/))
plutôt qu'une simple `HashMap<String, String>`, précisément pour pouvoir
représenter les occurrences d'une option répétable aux côtés des options
scalaires :

```rust
let occurrences = args.get_repeated("output").unwrap_or(&[]);
for occ in occurrences {
    match occ.discriminant.as_str() {
        "csv" => { /* occ.params.get("file"), occ.params.get("resolution") */ }
        "plot" => { /* occ.params.get("file") */ }
        _ => unreachable!(), // la validation de config a déjà rejeté les autres discriminants
    }
}
```

Voir `CHANGELOG.md` (v0.6.0) pour le chemin de migration complet
`HashMap<String, String>` → `ParsedArgs` des handlers existants.

---

## Chaînage de commandes

*Depuis la v0.8.0 ([DD-026](https://github.com/biface/dcli/issues/52)).*

Une seule invocation CLI (ou une ligne de
[`run_script()`](https://docs.rs/dynamic-cli/latest/dynamic_cli/interface/struct.CliInterface.html#method.run_script))
peut chaîner plusieurs commandes : une fois l'arité déclarée d'une
commande épuisée, le jeton nu suivant est vérifié auprès du registre
des commandes — s'il correspond à un nom de commande (ou un alias), il
démarre la commande suivante de la chaîne ; sinon, la ligne est rejetée
exactement comme une commande seule trop longue l'a toujours été. Il
n'existe aucun jeton séparateur (`;`, `&&`, `|`) — le chaînage résulte
uniquement de l'épuisement de l'arité, ce n'est pas une syntaxe
distincte à activer.

### Règle de segmentation

Pour chaque commande de la séquence :

1. Ses options (y compris les occurrences d'options répétables, DD-024)
   et ses arguments positionnels sont consommés jusqu'à son arité
   déclarée.
2. Le jeton suivant est recherché dans le registre des commandes.
   - S'il correspond à un nom de commande ou un alias → démarre le
     segment suivant.
   - Sinon → la même erreur « trop d'arguments » qu'une commande seule
     non chaînée aurait levée — aucun changement de comportement pour
     une commande seule réellement trop longue.

### Champs

Deux champs `boolean` additifs sur une définition de commande
gouvernent le comportement de la chaîne quand *cette* commande précise
échoue. Les deux valent `false` par défaut, donc les configurations
existantes ne sont pas affectées — voir [Détails des champs](#détails-des-champs)
ci-dessus pour leur place dans le schéma complet.

```yaml
- name: "nom-commande"
  # ... autres champs ...
  continue_on_failure: false   # optionnel, défaut false
  requires_success: false      # optionnel, défaut false
```

**`continue_on_failure`** :
- `false` (défaut) : si cette commande échoue, la chaîne s'arrête
  immédiatement — identique au comportement actuel d'une commande
  seule, appliqué par segment.
- `true` : si cette commande échoue, l'erreur avec position dans la
  chaîne est signalée et l'exécution se poursuit au segment suivant.
  Destiné aux étapes réellement optionnelles (ex : un export secondaire
  non critique).

**`requires_success`** :
- `false` (défaut) : cette commande s'exécute indépendamment d'un échec
  antérieur dans la chaîne (sans effet sauf si une commande antérieure
  a `continue_on_failure: true`).
- `true` : si une commande antérieure de la chaîne a déjà échoué
  (quelle que soit la valeur de `continue_on_failure` de *cet* échec),
  cette commande est **ignorée** — non exécutée, non comptée comme un
  échec supplémentaire.

### Rapport d'erreur et d'omission

- Une commande en échec est signalée ainsi : `Error in command
  {n}/{total} ('{name}'): <sortie d'erreur habituelle>` — la position
  dans la chaîne est ajoutée en préfixe ; le reste du message est
  inchangé par rapport à l'échec d'une commande seule.
- Une commande ignorée (via `requires_success`) est signalée ainsi :
  `Skipped: command {n}/{total} ('{name}') — a preceding command
  failed`.
- Le code de sortie du processus reflète toujours le **premier** échec
  rencontré dans la chaîne — l'échec « déclencheur » — même si une
  commande ultérieure avec `continue_on_failure: true` échoue aussi, ou
  qu'une commande en aval est ignorée en conséquence.

### Limitation connue

Si la ligne de commande fournit un jeton de plus que l'arité déclarée
d'une commande, et que ce jeton excédentaire correspond par ailleurs à
un nom de commande enregistré, il est silencieusement lu comme le
début du segment suivant plutôt que de lever une erreur — la
segmentation ne peut pas distinguer « une valeur excédentaire isolée »
de « la commande suivante » une fois l'arité épuisée. Aucun
échappement de type `--` n'existe pour forcer une valeur à être
traitée comme positionnelle plutôt que comme une frontière de
commande. Non planifié pour correction ; à revoir seulement si un
besoin concret se présente.

### Portée

- [`CliInterface::run()`](https://docs.rs/dynamic-cli/latest/dynamic_cli/interface/struct.CliInterface.html#method.run)
  (CLI en une fois) et `run_script()` héritent automatiquement du
  chaînage — les deux passent par le même chemin de résolution interne.
- La méta-commande `:load` du REPL ne chaîne **pas** : chaque ligne
  chargée passe toujours par le chemin mono-commande propre au REPL,
  inchangé.

### Exemple

```yaml
commands:
  - name: "configure"
    description: "Enregistre une source d'entrée"
    aliases: []
    required: false
    arguments:
      - name: "path"
        type: "path"
        required: true
        description: "Fichier source"
        validation: []
    options: []
    implementation: "configure_handler"

  - name: "solve"
    description: "Exécute une fois toutes les sources configurées"
    aliases: []
    required: false
    arguments: []
    options: []
    implementation: "solve_handler"
    requires_success: true
```

**Utilisation** :
```bash
monapp configure model.yml configure scenario.yml solve
# "configure" s'exécute deux fois, puis "solve" — seulement si les deux configure ont réussi
```

---

## Types d'arguments

Types de données supportés pour les arguments et options.

### Types disponibles

| Type      | Description               | Exemples de valeurs          | Type Rust  |
|-----------|---------------------------|------------------------------|------------|
| `string`  | Valeur texte              | `"bonjour"`, `"config.yaml"` | `String`   |
| `integer` | Nombre entier             | `42`, `-10`, `0`             | `i64`      |
| `float`   | Nombre décimal            | `3.14`, `-0.5`, `1.0`        | `f64`      |
| `bool`    | Flag booléen              | `true`, `false`              | `bool`     |
| `path`    | Chemin fichier/répertoire | `"./fichier.txt"`, `"/tmp"`  | `PathBuf`  |

### Parsing des types

**`string`** :
- Accepte n'importe quel texte
- Pas de parsing requis
- Exemple : `nom: "Jean Dupont"`

**`integer`** :
- Doit être un entier valide
- Support des valeurs négatives
- Plage : `-9,223,372,036,854,775,808` à `9,223,372,036,854,775,807`
- Exemple : `compte: 42`

**`float`** :
- Doit être un nombre décimal valide
- Support de la notation scientifique
- Exemple : `valeur: 3.14` ou `valeur: 1.5e-3`

**`bool`** :
- Pour les flags : présence = `true`, absence = `false`
- Pour les valeurs : `"true"`, `"false"`, `"yes"`, `"no"`, `"1"`, `"0"`
- Insensible à la casse
- Exemple : `--verbose` (true) vs pas de flag (false)

**`path`** :
- Chemin de fichier ou répertoire
- Peut être relatif ou absolu
- La validation peut vérifier l'existence
- Exemple : `fichier: "./donnees/entree.csv"`

---

## Règles de validation

Contraintes appliquées aux arguments et options.

### Règles disponibles

#### 1. Existence fichier/répertoire

```yaml
validation:
  - must_exist: true
```

**S'applique à** : type `path`  
**Effet** : Vérifie si le fichier/répertoire existe avant l'exécution  
**Erreur si** : Le chemin n'existe pas

#### 2. Extension de fichier

```yaml
validation:
  - extensions: [".txt", ".csv", ".json"]
```

**S'applique à** : type `path` (fichiers uniquement)  
**Effet** : Valide que le fichier a l'une des extensions spécifiées  
**Erreur si** : L'extension du fichier n'est pas dans la liste

#### 3. Plage numérique

```yaml
validation:
  - min: 1
    max: 100
```

**S'applique à** : types `integer`, `float`  
**Effet** : La valeur doit être dans la plage (inclusive)  
**Champs** :
- `min` (optionnel) : Valeur minimale
- `max` (optionnel) : Valeur maximale
- Les deux peuvent être omis pour des bornes unilatérales

**Exemples** :
```yaml
# Seulement minimum
- min: 0

# Seulement maximum
- max: 100

# Les deux
- min: 1
  max: 10
```

### Combinaison de règles

Plusieurs règles peuvent être appliquées à un seul argument :

```yaml
validation:
  - must_exist: true
  - extensions: [".yaml", ".json"]
```

**Effet** : Le fichier doit exister ET avoir l'extension .yaml ou .json

### Pas de validation

Utiliser un tableau vide si aucune validation n'est nécessaire :

```yaml
validation: []
```

---

## Exemple complet

Fichier de configuration complet pour une application de traitement de données.

```yaml
# ═══════════════════════════════════════════════════════════
# Configuration CLI - Traitement de Données
# ═══════════════════════════════════════════════════════════

metadata:
  version: "1.0.0"
  prompt: "dataproc"
  prompt_suffix: " > "

# ───────────────────────────────────────────────────────────
# OPTIONS GLOBALES (disponibles pour toutes les commandes)
# ───────────────────────────────────────────────────────────
global_options:
  - name: "verbose"
    short: "v"
    long: "verbose"
    type: "bool"
    required: false
    default: "false"
    description: "Activer la sortie verbeuse"
  
  - name: "sortie-rep"
    short: "o"
    long: "sortie"
    type: "path"
    required: false
    description: "Répertoire de sortie pour les résultats"
  
  - name: "format"
    short: "f"
    long: "format"
    type: "string"
    required: false
    default: "text"
    description: "Format de sortie"
    choices: ["text", "json", "yaml"]

# ───────────────────────────────────────────────────────────
# COMMANDES
# ───────────────────────────────────────────────────────────
commands:
  
  # ═════════════════════════════════════════════════════════
  # Commande : charger
  # ═════════════════════════════════════════════════════════
  - name: "charger"
    aliases: ["ouvrir", "entree"]
    description: "Charger un fichier de données"
    required: true
    
    arguments:
      - name: "fichier"
        type: "path"
        required: true
        description: "Chemin vers le fichier de données"
        validation:
          - must_exist: true
          - extensions: [".csv", ".json", ".yaml"]
    
    options: []
    
    implementation: "load_data"
  
  # ═════════════════════════════════════════════════════════
  # Commande : traiter
  # ═════════════════════════════════════════════════════════
  - name: "traiter"
    aliases: ["lancer"]
    description: "Traiter les données chargées"
    required: false
    
    arguments: []
    
    options:
      - name: "threads"
        short: "t"
        long: "threads"
        type: "integer"
        required: false
        default: "4"
        description: "Nombre de threads parallèles"
        choices: []
      
      - name: "taille-lot"
        short: "b"
        long: "lot"
        type: "integer"
        required: false
        default: "1000"
        description: "Taille du lot pour le traitement"
        choices: []
    
    implementation: "process_data"
  
  # ═════════════════════════════════════════════════════════
  # Commande : exporter
  # ═════════════════════════════════════════════════════════
  - name: "exporter"
    aliases: ["sauver"]
    description: "Exporter les résultats vers un fichier"
    required: false
    
    arguments:
      - name: "fichier-sortie"
        type: "path"
        required: true
        description: "Chemin du fichier de sortie"
        validation: []
    
    options:
      - name: "compresser"
        short: "c"
        long: "compresser"
        type: "bool"
        required: false
        default: "false"
        description: "Compresser le fichier de sortie"
        choices: []
    
    implementation: "export_results"
  
  # ═════════════════════════════════════════════════════════
  # Commande : aide
  # ═════════════════════════════════════════════════════════
  - name: "aide"
    aliases: ["h", "?"]
    description: "Afficher l'aide"
    required: false
    
    arguments:
      - name: "commande"
        type: "string"
        required: false
        description: "Commande pour laquelle obtenir de l'aide"
        validation: []
    
    options: []
    
    implementation: "show_help"
  
  # ═════════════════════════════════════════════════════════
  # Commande : quitter
  # ═════════════════════════════════════════════════════════
  - name: "quitter"
    aliases: ["sortir", "q"]
    description: "Quitter l'application"
    required: false
    
    arguments: []
    options: []
    
    implementation: "exit_app"
```

### Exemples d'utilisation

**Mode CLI** :
```bash
# Avec options globales
dataproc --verbose charger donnees.csv
dataproc -v -o ./resultats traiter --threads 8
dataproc --format json exporter sortie.json

# Sans options globales
dataproc charger donnees.csv
dataproc traiter --threads 8 --lot 2000
dataproc exporter sortie.csv --compresser
```

**Mode REPL** :
```
dataproc > charger donnees.csv
Données chargées avec succès.

dataproc > traiter --threads 8
Traitement... Terminé.

dataproc > exporter resultats.csv
Résultats exportés vers resultats.csv

dataproc > aide exporter
Commande : exporter
Description : Exporter les résultats vers un fichier
...

dataproc > quitter
Au revoir !
```

---

## Bonnes pratiques

### 1. Conventions de nommage

**Commandes** :
- Utiliser les minuscules
- Utiliser des traits d'union pour les mots multiples : `lancer-simulation` (pas `lancerSimulation` ou `lancer_simulation`)
- Garder les noms concis mais descriptifs
- Fournir des alias significatifs : `["quitter", "q", "sortir"]`

**Arguments/Options** :
- Utiliser les minuscules avec traits d'union : `rep-sortie` (pas `repSortie`)
- Être cohérent dans toute l'application
- Utiliser judicieusement les formes courtes : caractère unique, couramment reconnu (`-v` pour verbose)

### 2. Required vs Optional

**Marquer comme `required: true` quand** :
- La commande DOIT être exécutée pour le workflow de l'application
- L'argument DOIT être fourni pour que la commande fonctionne
- L'option est essentielle (rare - considérer si ce devrait être un argument à la place)

**Marquer comme `required: false` quand** :
- La commande est une fonctionnalité de commodité optionnelle
- L'argument a un comportement par défaut sensé
- L'option améliore mais ne bloque pas la fonctionnalité

### 3. Validation

**Toujours valider quand** :
- Acceptant des chemins de fichiers (vérifier l'existence, les extensions)
- Acceptant des valeurs numériques avec contraintes (utiliser des plages)
- Acceptant des énumérations (utiliser `choices`)

**Exemple** :
```yaml
# Bon : Validation complète
validation:
  - must_exist: true
  - extensions: [".csv", ".json"]
```

```yaml
# Mauvais : Pas de validation pour entrée fichier
validation: []
```

### 4. Descriptions

**Écrire des descriptions claires et concises** :
- ✅ Bon : "Charger un fichier de configuration"
- ❌ Mauvais : "Charge des trucs"
- ✅ Bon : "Nombre de threads parallèles (1-16)"
- ❌ Mauvais : "threads"

### 5. Valeurs par défaut

**Fournir des valeurs par défaut sensées pour les champs optionnels** :
```yaml
# Bon : Valeur par défaut sensée
- name: "threads"
  default: "4"
  
# Mauvais : Requis mais devrait avoir une valeur par défaut
- name: "threads"
  required: true  # L'utilisateur doit toujours spécifier
```

### 6. Options globales vs options de commande

**Utiliser `global_options` pour** :
- Flags verbose/debug
- Sélection de format de sortie
- Chemins de fichiers de configuration
- Niveaux de journalisation

**Utiliser `options` de commande pour** :
- Comportement spécifique à la commande
- Paramètres qui n'ont de sens que pour une commande

### 7. Alias

**Fournir des alias utiles** :
```yaml
# Bon : Raccourcis courants
aliases: ["quitter", "q", "sortir"]
```
```yaml
# Bon : Noms alternatifs
aliases: ["charger", "ouvrir", "entree"]
```
```yaml
# À éviter : Confus ou trop nombreux
aliases: ["a", "b", "c", "xyz123"]
```

### 8. Noms d'implémentation

**Utiliser un nommage clair et cohérent** :
```yaml
# Bon : Objectif clair
implementation: "load_config"
```
```yaml
implementation: "run_simulation"
```
```yaml
# Mauvais : Vague
implementation: "handler1"
```
```yaml
implementation: "do_stuff"
```

### 9. Organisation des fichiers

Pour les grandes applications, considérer :
- Un fichier de configuration par mode (cli.yaml, repl.yaml)
- Commandes partagées dans un fichier inclus séparé
- Surcharges spécifiques à l'environnement (dev.yaml, prod.yaml)

### 10. Documentation

**Commenter les sections complexes** :
```yaml
# ═══════════════════════════════════════════════════════════
# COMMANDES DE SIMULATION
# Ces commandes contrôlent le moteur de simulation numérique
# ═══════════════════════════════════════════════════════════
commands:
  - name: "simuler"
    # Les utilisateurs avancés peuvent surcharger le pas de temps
    options:
      - name: "dt"
        description: "Pas de temps en secondes (experts uniquement)"
```

---

## Équivalent JSON

L'exemple complet ci-dessus au format JSON :

```json
{
  "metadata": {
    "version": "1.0.0",
    "prompt": "dataproc",
    "prompt_suffix": " > "
  },
  "global_options": [
    {
      "name": "verbose",
      "short": "v",
      "long": "verbose",
      "type": "bool",
      "required": false,
      "default": "false",
      "description": "Activer la sortie verbeuse"
    }
  ],
  "commands": [
    {
      "name": "charger",
      "aliases": ["ouvrir", "entree"],
      "description": "Charger un fichier de données",
      "required": true,
      "arguments": [
        {
          "name": "fichier",
          "type": "path",
          "required": true,
          "description": "Chemin vers le fichier de données",
          "validation": [
            {"must_exist": true},
            {"extensions": [".csv", ".json"]}
          ]
        }
      ],
      "options": [],
      "implementation": "load_data"
    }
  ]
}
```

---

## Résumé

Cette référence couvre tous les éléments de syntaxe des fichiers de configuration `dynamic-cli` :

✅ **Metadata** - Informations sur l'application  
✅ **Options globales** - Options pour toutes les commandes  
✅ **Commandes** - Définitions de commandes individuelles  
✅ **Arguments** - Paramètres positionnels  
✅ **Options** - Flags et paramètres nommés  
✅ **Types** - Spécifications de types de données  
✅ **Validation** - Règles de contraintes  
✅ **Exemples** - Utilisation réelle  
✅ **Bonnes pratiques** - Recommandations  

Pour les détails d'implémentation, voir la documentation de l'API `dynamic-cli`.

---

**Version** : 1.0  
**Framework** : dynamic-cli  
**Date** : 2026-01-11
