git # Documentation DockerOps

Guide utilisateur et référence technique pour DockerOps, outil CLI de gestion des stacks Docker Swarm depuis des répertoires GitHub.

## Table des matières

1. [Introduction](#1-introduction)
2. [Installation](#2-installation)
3. [Configuration](#3-configuration)
4. [Authentification GitHub](#4-authentification-github)
5. [Commandes](#5-commandes)
6. [Structure du repository](#6-structure-du-repository)
7. [Volumes](#7-volumes)
8. [Secrets](#8-secrets)
9. [Gestion des images](#9-gestion-des-images)
10. [Référence technique](#10-référence-technique)
11. [Workflows et exemples](#11-workflows-et-exemples)
12. [Dépannage](#12-dépannage)
13. [Développement](#13-développement)
14. [Exécution en conteneur et dans le Swarm](#14-exécution-en-conteneur-et-dans-le-swarm)

---

## 1. Introduction

### Vue d'ensemble

DockerOps est un outil CLI en Rust qui automatise la gestion des stacks Docker Swarm depuis des répertoires GitHub. Il permet de :

- Cloner automatiquement des repositories GitHub
- Déployer des stacks Docker Swarm
- Gérer les volumes (Docker volumes et bindings NFS)
- Injecter des secrets comme variables d'environnement
- Synchroniser les changements via la commande `reconcile`
- Nettoyer automatiquement les images non utilisées

### Cas d'usage typiques

- **Déploiement continu** : Déployer automatiquement des stacks depuis GitHub
- **Gestion centralisée** : Gérer plusieurs stacks depuis un seul repository
- **Environnements distribués** : Utiliser NFS pour partager des volumes entre nœuds Swarm
- **Sécurité** : Gérer les secrets de manière centralisée sur NFS

### Prérequis

- **Docker** : Version 20.10 ou supérieure
- **Docker Swarm** : Cluster Swarm initialisé (`docker swarm init`)
- **Droits root** : DockerOps doit être exécuté avec sudo
- **NFS** (optionnel) : Serveur NFS monté si vous utilisez des bindings
- **GitHub** : Accès aux repositories (token pour repositories privés)
- **Rust** (pour compilation manuelle) : Version 1.70 ou supérieure

---

## 2. Installation

### Installation rapide (recommandée)

```bash
# Installation en une ligne
curl -sSL https://raw.githubusercontent.com/TomBedinoVT/DockerOps/main/dockerops.sh | sudo bash -s install
```

### Script manager (dockerops.sh)

Le script unifié permet d'installer, mettre à jour, désinstaller et gérer DockerOps.

**Prérequis du script :** Python 3.6+, curl, droits root (sudo).

**Commandes principales :**

```bash
# Installer la dernière version
sudo ./dockerops.sh install

# Installer une version spécifique
sudo ./dockerops.sh install -v v1.0.0

# Installer avec nettoyage complet
sudo ./dockerops.sh install --clean-all

# Désinstaller complètement
sudo ./dockerops.sh uninstall --clean-all

# Vérifier le statut
./dockerops.sh status

# Afficher l'aide
./dockerops.sh help
```

**Options d'installation :**

```bash
sudo ./dockerops.sh install              # Dernière version
sudo ./dockerops.sh install -v v1.2.0    # Version spécifique
sudo ./dockerops.sh install --clean-db   # + nettoyage base de données
sudo ./dockerops.sh install --clean-dirs # + nettoyage dossiers
sudo ./dockerops.sh install --clean-all  # + tout nettoyer
```

**Options de désinstallation :**

```bash
sudo ./dockerops.sh uninstall            # Supprimer le binaire
sudo ./dockerops.sh uninstall --clean-all # Binaire + toutes les données
```

**Fonctionnalités du script :** détection d'architecture, téléchargement depuis GitHub, installation dans `/usr/local/bin`, sauvegarde de l'ancienne version, restauration en cas d'échec, nettoyage base/dossiers, statut et diagnostic.

**Structure d'installation :**

```
/usr/local/bin/
└── dockerops                    # Binaire principal
    └── dockerops.backup         # Sauvegarde (si existe)

~/.dockerops/
├── dockerops.db                 # Base de données SQLite
└── logs/                        # Logs (si configuré)
```

**Exemples :**

```bash
# Installation propre
curl -O https://raw.githubusercontent.com/TomBedinoVT/DockerOps/main/dockerops.sh
chmod +x dockerops.sh
sudo ./dockerops.sh install --clean-all

# Mise à jour
sudo ./dockerops.sh install
sudo ./dockerops.sh install -v v1.3.0

# Désinstallation
sudo ./dockerops.sh uninstall --clean-all

# Diagnostic
./dockerops.sh status
```

**Résolution de problèmes script :** vérifier les permissions (`ls -la /usr/local/bin/dockerops`, `sudo chmod +x /usr/local/bin/dockerops`), la connectivité (`curl -I https://api.github.com`), la version (`dockerops version`), Python 3 (`python3 --version`).

### Installation manuelle

```bash
git clone https://github.com/TomBedinoVT/DockerOps.git
cd DockerOps
cargo build --release
sudo cp target/release/dockerops /usr/local/bin/
```

L'exécutable est aussi disponible dans `target/release/dockerops` sans copie.

---

## 3. Configuration

### Variables d'environnement

- **DOCKEROPS_DB_PATH** : Chemin de la base SQLite (défaut : `~/.dockerops/dockerops.db`)
- **DOCKEROPS_IMAGE_PULL_POLICY** : Politique de pull (`always` ou `ifnotpresent`, défaut : `ifnotpresent`)
- **GITHUB_TOKEN** : Token GitHub pour repositories privés (voir [Authentification GitHub](#4-authentification-github))

```bash
export DOCKEROPS_DB_PATH="/var/lib/dockerops/dockerops.db"
export DOCKEROPS_IMAGE_PULL_POLICY="always"   # ou ifnotpresent
export GITHUB_TOKEN="ghp_votre_token_ici"
```

Exemple de fichier de config : `~/.dockerops/config.sh` avec les exports ci-dessus, puis `source ~/.dockerops/config.sh` avant d'utiliser DockerOps.

### Permissions requises

DockerOps doit être exécuté avec les privilèges root (sudo) : exécution de commandes Docker, gestion des stacks Swarm, pull/suppression d'images, accès au daemon Docker.

```bash
sudo dockerops <command>
```

### Base de données

La base SQLite est créée automatiquement dans `~/.dockerops/` par défaut, ou à l'emplacement donné par `DOCKEROPS_DB_PATH`. Pour un chemin personnalisé :

```bash
export DOCKEROPS_DB_PATH="/var/lib/dockerops/dockerops.db"
sudo mkdir -p /var/lib/dockerops
sudo chown $USER:$USER /var/lib/dockerops
```

### Configuration NFS (si volumes bindings)

Si vous utilisez des volumes de type `binding`, configurez le montage NFS (le chemin doit correspondre à `nfs.yaml`) :

```bash
sudo mount -t nfs nfs-server:/path/to/share /mnt/nfs/dockerops

# Montage permanent dans /etc/fstab
nfs-server:/path/to/share /mnt/nfs/dockerops nfs defaults 0 0
```

---

## 4. Authentification GitHub

### Problème

Erreur typique pour un repository privé :

```
Error: Failed to clone repository: remote authentication required but no callback set; class=Http (34); code=Auth (-16)
```

### Solution 1 : Token GitHub (recommandée)

1. **Créer un token** : GitHub.com → Settings → Developer settings → Personal access tokens → Tokens (classic) → Generate new token (classic). Nom (ex. "DockerOps"), permissions : `repo` (repositories privés), `read:org` (si org). Copier le token.
2. **Configurer le token :**
   - Windows PowerShell : `$env:GITHUB_TOKEN="ghp_votre_token_ici"`
   - Windows CMD : `set GITHUB_TOKEN=ghp_votre_token_ici`
   - Linux/macOS : `export GITHUB_TOKEN="ghp_votre_token_ici"`
3. **Utiliser DockerOps :** `sudo dockerops watch https://github.com/username/repository`

### Solution 2 : Configuration permanente

- **Windows** : Paramètres système → Variables d'environnement → ajouter variable utilisateur `GITHUB_TOKEN`.
- **Linux/macOS** : Ajouter `export GITHUB_TOKEN="ghp_votre_token_ici"` dans `~/.bashrc` ou `~/.zshrc`.

### Solution 3 : Repository public

Si le repository est public, l'authentification n'est pas nécessaire.

### Sécurité

- Ne jamais committer le token
- Permissions minimales
- Régénérer régulièrement les tokens
- Supprimer les tokens inutilisés

### Dépannage authentification

1. Vérifier que le token est défini : `echo $GITHUB_TOKEN` (Linux/macOS), `echo %GITHUB_TOKEN%` (Windows CMD), `$env:GITHUB_TOKEN` (PowerShell).
2. Vérifier les permissions du token.
3. Vérifier l'URL du repository.
4. Tester le clonage manuel : `git clone https://github.com/username/repository.git`

---

## 5. Commandes

### Tableau récapitulatif

| Commande | Description | Options |
|----------|-------------|---------|
| `watch <url>` | Surveiller et déployer un repository GitHub | - |
| `reconcile` | Synchroniser les repositories et afficher l'état | `--force` : forcer le redéploiement |
| `stop` | Arrêter toutes les stacks et nettoyer | - |
| `version` | Afficher la version | - |
| `debug-cache` | Afficher les infos de debug du cache | - |
| `run` | Mode daemon : init. DOCKEROPS_REPOS puis boucle reconcile (conteneur/Swarm) | - |

### watch

```bash
sudo dockerops watch "https://github.com/user/repo"
```

Vérifie que le repository n'est pas déjà en cache, clone le repository, lit `stacks.yaml`, traite volumes et secrets, calcule les hashes, déploie chaque stack, traite les images, ajoute le repository au cache, nettoie le répertoire cloné.

### reconcile

```bash
sudo dockerops reconcile
sudo dockerops reconcile --force
```

Affiche l'état (repositories, stacks, images), clone et synchronise chaque repository en cache, met à jour les stacks modifiées, nettoie les images non utilisées. Nécessite au moins un `watch` préalable. `--force` : redéploie toutes les stacks même sans changement détecté.

### stop

```bash
sudo dockerops stop
```

Supprime toutes les stacks Swarm, toutes les images, nettoie la base de données et le cache. Commande destructive.

### version

```bash
dockerops version
```

Affiche la version et les infos du repository (pas besoin de sudo).

### debug-cache

```bash
sudo dockerops debug-cache
```

Affiche les repositories en cache et leur dernier watch (utile pour le dépannage).

### run (mode daemon)

```bash
sudo dockerops run
```

Lance DockerOps en mode daemon (comme ArgoCD) : initialise les repositories listés dans `DOCKEROPS_REPOS`, puis exécute `reconcile` en boucle à l’intervalle `DOCKEROPS_SYNC_INTERVAL` (en secondes). Utilisé notamment lorsque DockerOps est déployé dans un conteneur ou dans le Swarm.

- **DOCKEROPS_REPOS** (optionnel) : URLs des repositories GitHub à surveiller, séparées par des virgules ou des points-virgules. Au démarrage, chaque URL est ajoutée au cache (équivalent d’un `watch`) ; si une URL est déjà en cache, elle est ignorée.
- **DOCKEROPS_SYNC_INTERVAL** (optionnel) : intervalle en secondes entre deux reconciles (défaut : 300).

Voir la section [Exécution en conteneur et dans le Swarm](#14-exécution-en-conteneur-et-dans-le-swarm).

---

## 6. Structure du repository

Le repository GitHub doit contenir :

```
repository/
├── stacks.yaml          # Liste des stacks (requis)
├── volumes.yaml         # Volumes (optionnel)
├── nfs.yaml             # NFS (requis si volumes.yaml)
├── stack1/
│   ├── docker-compose.yml
│   └── secrets.yaml     # Secrets pour ce stack (optionnel)
└── stack2/
    └── docker-compose.yml
```

### stacks.yaml

À la racine, liste des stacks (un dossier par stack avec un `docker-compose.yml`) :

```yaml
- name: web-stack
- name: api-stack
```

Chaque nom doit correspondre à un dossier contenant `docker-compose.yml` (ou `docker-compose.yaml`, `compose.yml`, `compose.yaml`).

---

## 7. Volumes

DockerOps gère deux types de volumes : **Volumes Docker** (gérés par Docker) et **Bindings** (fichiers/dossiers copiés vers NFS pour compatibilité Swarm).

### volumes.yaml

À la racine du repository :

```yaml
- id: "app_data"
  type: "volume"
  path: "app_data_volume"

- id: "config_files"
  type: "binding"
  path: "config"
```

- **id** : Identifiant utilisé dans les services (docker-compose).
- **type** : `volume` ou `binding`.
- **path** : pour `volume` = nom du volume Docker ; pour `binding` = chemin relatif dans le repository.

### nfs.yaml

Requis si `volumes.yaml` est présent :

```yaml
path: "/mnt/nfs/dockerops"
```

Ce chemin doit être monté et accessible depuis tous les nœuds Swarm.

### Utilisation dans docker-compose

Les volumes sont déclarés dans les services (pas de section `volumes` top-level obligatoire) :

```yaml
services:
  web:
    image: nginx:alpine
    volumes:
      - "config_files:/etc/nginx/conf.d"
      - "static_files:/usr/share/nginx/html"
```

### Fonctionnement

- **watch / reconcile** : DockerOps lit `volumes.yaml` et `nfs.yaml`. Pour chaque `volume` : le volume Docker est géré par Docker. Pour chaque `binding` : copie du contenu du repo vers NFS, suppression de l'ancien contenu sur NFS si besoin, puis modification du docker-compose pour remplacer l'id par le chemin NFS.
- **Transformation** : Avant `"config_files:/etc/nginx/conf.d"` → Après `"/mnt/nfs/dockerops/config:/etc/nginx/conf.d"` (le chemin NFS est `{nfs.path}/{path}` où `path` est le champ du volumes.yaml pour le binding).
- **Workflow binding** : lecture du dossier local, suppression ancien contenu NFS, copie récursive, permissions (755 dossiers, 644 fichiers), propriétaire pour Docker, mise à jour du compose.

Structure de répertoires typique :

```
repository/
├── stacks.yaml
├── volumes.yaml
├── nfs.yaml
├── config/
├── static/
├── logs/
└── my-stack/
    └── docker-compose.yml
```

### Configuration requise volumes

- Serveur NFS accessible depuis tous les nœuds Swarm
- Droits d'écriture pour DockerOps sur NFS
- Montage NFS sur tous les nœuds

### Dépannage volumes

- **Erreur de copie NFS** : accessibilité NFS, permissions, existence du chemin local dans le repo.
- **Volume Docker non créé** : Docker accessible, permissions Docker.
- **Binding non transformé** : id dans docker-compose cohérent avec `volumes.yaml`, présence de `volumes.yaml`.

---

## 8. Secrets

DockerOps n'utilise que les **secrets natifs Docker Swarm**. Aucune valeur n'est lue depuis le disque (plus de NFS pour les secrets). Vous déclarez dans `secrets.yaml` quels secrets Swarm exposer en variables d'environnement ; DockerOps génère un script d'entrypoint qui fait `export VAR=$(cat /run/secrets/nom)` au démarrage du conteneur.

### Principe

- **secrets.yaml** : déclaration uniquement (pas de valeurs). Chaque entrée associe le **nom** d'un secret Docker Swarm à la **variable d'environnement** à remplir dans le conteneur.
- Les secrets sont créés par l'administrateur avec `docker secret create <nom> -` (ou depuis un fichier) **avant** `watch` ou `reconcile`.
- DockerOps lit `secrets.yaml`, génère un script **entrypoint-secrets.sh** dans le dossier du stack, et injecte dans le compose : section `secrets: <nom>: external: true`, montage du script, et entrypoint qui exporte les variables puis lance la commande du service. Le contenu des fichiers montés sous `/run/secrets/<nom>` est ainsi exposé automatiquement en variables d'environnement.

### Format secrets.yaml

Fichier dans le dossier de chaque stack (à côté du docker-compose) :

```yaml
- secret: db_password
  env: DB_PASSWORD

- secret: api_key
  env: API_SECRET_KEY
```

- **secret** : nom du secret Docker Swarm (créé avec `docker secret create <nom> -`).
- **env** : nom de la variable d'environnement à définir dans le conteneur (contenu du fichier `/run/secrets/<secret>`).

Le champ **id** est accepté comme alias de **secret** pour la rétrocompatibilité.

### Création des secrets (Swarm)

Avant le premier déploiement, créez les secrets sur le Swarm :

```bash
echo -n "mon_mot_de_passe_secret" | docker secret create db_password -
echo -n "ma_cle_api" | docker secret create api_key -
```

Ou depuis un fichier :

```bash
docker secret create db_password ./db_password.txt
```

Vérification : `docker secret ls`

### Comportement dans le conteneur

DockerOps génère **entrypoint-secrets.sh** dans le dossier du stack. Ce script :

1. Exporte chaque variable : `export DB_PASSWORD=$(cat /run/secrets/db_password 2>/dev/null || true)` (etc.)
2. Lance la commande du service : `exec "$@"`

Le compose modifié contient un volume montant ce script et un entrypoint qui l'exécute. Votre application reçoit donc les variables d'environnement sans lire les fichiers sous `/run/secrets/` elle-même.

### Utilisation dans docker-compose

Vous pouvez référencer les variables dans `environment` (sans valeur) ; elles seront définies par l'entrypoint :

```yaml
services:
  app:
    image: myapp:latest
    environment:
      - DB_PASSWORD
      - API_SECRET_KEY
```

La section `secrets` (external) et l'entrypoint sont **injectés automatiquement** par DockerOps à partir de `secrets.yaml`.

### Bonnes pratiques

- Ne jamais committer de valeurs de secrets dans le repository.
- Créer les secrets sur le Swarm avant d'exécuter `watch` ou `reconcile`.
- Rotation : créer un nouveau secret (ex. `db_password_v2`), mettre à jour `secrets.yaml` et le compose si besoin, redéployer.
- En cas d'erreur de déploiement liée aux secrets : vérifier que les secrets existent (`docker secret ls`) et que les noms dans `secrets.yaml` correspondent exactement.

---

## 9. Gestion des images

### Politique de pull

- **always** : toujours pull depuis le registry.
- **ifnotpresent** (défaut) : pull seulement si l'image n'est pas présente localement.

Configuré par `DOCKEROPS_IMAGE_PULL_POLICY`. Avantages/inconvénients : always = à jour mais plus lent ; ifnotpresent = plus rapide mais peut rester sur une image ancienne.

### Nettoyage automatique

À chaque `watch` ou `reconcile` : réinitialisation des compteurs de références, comptage des images présentes dans les compose, suppression des images à 0 référence, nettoyage de la base.

### Vérification SHA

DockerOps peut comparer les SHA locaux au registry (API Docker Hub) et mettre à jour selon la politique configurée.

---

## 10. Référence technique

### Base de données (SQLite)

- **Table `images`** : `id` (INTEGER PRIMARY KEY), `name` (TEXT UNIQUE), `reference_count` (INTEGER).
- **Table `stacks`** : `id`, `name`, `repository_url`, `compose_path`, `hash`, `status` ("deployed", "stopped", "error") ; UNIQUE(name, repository_url).
- **Table `repository_cache`** : `id`, `url` (TEXT UNIQUE), `last_watch` (timestamp).

Fichier par défaut : `~/.dockerops/dockerops.db` ou `DOCKEROPS_DB_PATH`.

### Dépendances Rust (résumé)

clap, tokio, sqlx, git2, walkdir, md5, serde, serde_yaml, reqwest, anyhow, thiserror, chrono, bollard, octocrab, futures.

---

## 11. Workflows et exemples

### Premiers pas

1. Créer un repository avec `stacks.yaml` et un dossier par stack avec `docker-compose.yml`.
2. `sudo dockerops watch "https://github.com/username/repo"`.
3. Vérifier : `docker stack ls`, `docker service ls`.
4. Synchroniser : `sudo dockerops reconcile`.

### Exemple : stack simple

Repository : `stacks.yaml` avec `- name: my-app`, dossier `my-app/docker-compose.yml` (ex. nginx:alpine, port 80, replicas 2). Puis `watch` puis `reconcile`.

### Exemple : structure complète avec volumes

Structure :

```
my-dockerops-repo/
├── stacks.yaml
├── volumes.yaml
├── nfs.yaml
├── config/
│   ├── nginx.conf
│   └── app.conf
├── static/
│   ├── index.html
│   └── style.css
├── logs/
└── web-stack/
    └── docker-compose.yml
```

**stacks.yaml :** `- name: web-stack`

**volumes.yaml :**

```yaml
- id: "app_data"
  type: "volume"
  path: "app_data_volume"
- id: "config_files"
  type: "binding"
  path: "config"
- id: "static_files"
  type: "binding"
  path: "static"
- id: "logs"
  type: "binding"
  path: "logs"
```

**nfs.yaml :** `path: "/mnt/nfs/dockerops"`

**web-stack/docker-compose.yml :** services avec volumes `config_files`, `static_files`, `logs`, `app_data`. Après traitement : bindings remplacés par chemins NFS, volume Docker inchangé.

### Exemple : stack avec secrets

Structure : `stacks.yaml`, `api-stack/docker-compose.yml`, `api-stack/secrets.yaml`. Créer les secrets sur le Swarm avant déploiement : `echo -n "valeur" | docker secret create db_password -` (et idem pour `jwt_secret`). Dans `secrets.yaml` : `- secret: db_password` / `env: DB_PASSWORD` et `- secret: jwt_secret` / `env: JWT_SECRET`. DockerOps génère `entrypoint-secrets.sh` et injecte la section `secrets` (external) et l'entrypoint ; les variables sont disponibles dans le conteneur.

### Exemple : multi-stacks avec volumes et secrets

Plusieurs stacks (ex. web-stack, api-stack), `volumes.yaml` et `nfs.yaml` communs si besoin de volumes, `secrets.yaml` par stack. Secrets créés avec `docker secret create` ; déclaration dans chaque `secrets.yaml` (secret + env). Même principe : volumes et secrets déclarés, DockerOps injecte NFS pour les bindings et entrypoint + secrets externes pour les secrets.

### Workflows avancés

- **Développement** : modifier le repo, push, sur le serveur `sudo dockerops reconcile`, vérifier avec `docker stack services` et `docker service logs`.
- **Production** : déploiement initial avec `watch`, cron pour `reconcile` (ex. `0 * * * * root /usr/local/bin/dockerops reconcile`).
- **Mise à jour d'une stack** : modifier le compose, commit/push, `reconcile` ; le changement de hash déclenche le redéploiement.
- **Nouvelle stack** : créer le dossier et le compose, ajouter le nom dans `stacks.yaml`, push, `reconcile`.
- **Multi-repositories** : plusieurs `watch` avec des URLs différentes, un seul `reconcile` synchronise tous les repositories en cache.

---

## 12. Dépannage

### Erreurs courantes

- **"Repository is already being watched"** : utiliser `reconcile` au lieu de `watch`, ou vérifier avec `debug-cache`.
- **"remote authentication required"** : configurer `GITHUB_TOKEN` (voir [Authentification GitHub](#4-authentification-github)).
- **"DockerOps must be run with root privileges"** : exécuter avec `sudo dockerops <command>`.
- **Stack non déployée** : vérifier présence du dossier et du docker-compose, lancer `reconcile`, puis `docker stack ls` et `docker stack ps <stack-name>`.
- **Images non pullées** : vérifier `DOCKEROPS_IMAGE_PULL_POLICY`, essayer `always` et `reconcile --force`.
- **Erreur liée aux secrets** : vérifier que les secrets existent dans le Swarm (`docker secret ls`) et que les noms dans `secrets.yaml` correspondent exactement à ceux créés.
- **Problèmes NFS** (volumes uniquement) : `mount | grep nfs`, `ls -la /mnt/nfs/dockerops`, cohérence avec `nfs.yaml`.

### Commandes de debug

```bash
sudo dockerops debug-cache
sudo dockerops reconcile
docker stack ls
docker stack services <stack-name>
docker service ls
docker service ps <service-name>
docker service logs <service-name>
docker secret ls   # vérifier les secrets Swarm
```

Logs et diagnostics : `docker stack ps <stack-name> --no-trunc`, `docker service inspect <service-name>`, `docker images`, et si besoin `sqlite3 ~/.dockerops/dockerops.db "SELECT * FROM stacks;"` (et `images`).

### Réinitialisation complète

```bash
sudo dockerops stop
docker stack ls   # vide
docker images     # vide ou seulement système
sudo dockerops watch "https://github.com/username/repo"
```

---

## 13. Développement

```bash
cargo build
cargo test
sudo cargo run -- watch "https://github.com/example/repo"
```

Process de release et publication des binaires : voir [RELEASE.md](RELEASE.md).

---

## 14. Exécution en conteneur et dans le Swarm

DockerOps peut être dockerisé et déployé comme un service dans le Swarm, en mode autonome (type ArgoCD) : le conteneur exécute la commande `run`, initialise les repositories depuis `DOCKEROPS_REPOS` puis enchaîne des `reconcile` à intervalle régulier.

### Build de l’image

À la racine du repository :

```bash
docker build -t dockerops:latest .
```

L’image contient le binaire DockerOps et le CLI Docker (pour `docker stack deploy`). Le point d’entrée par défaut est `dockerops run`.

### Variables d’environnement en conteneur

- **DOCKEROPS_DB_PATH** : chemin de la base SQLite (défaut dans l’image : `/data/dockerops.db`). À placer sur un volume monté pour persister.
- **DOCKEROPS_REPOS** : liste d’URLs GitHub à surveiller, séparées par des virgules ou des points-virgules. Au démarrage, chaque URL est ajoutée au cache (watch) si elle n’y est pas déjà.
- **DOCKEROPS_SYNC_INTERVAL** : intervalle en secondes entre deux reconciles (défaut : 300).
- **GITHUB_TOKEN** : token GitHub pour les repositories privés. En Swarm, peut être fourni via un secret monté en fichier (voir ci-dessous).

### Exécution locale en conteneur

Montage du socket Docker et d’un volume pour la base :

```bash
docker run -d \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v dockerops-data:/data \
  -e DOCKEROPS_REPOS="https://github.com/org/repo1" \
  -e DOCKEROPS_SYNC_INTERVAL=300 \
  -e GITHUB_TOKEN="ghp_xxx" \
  --name dockerops \
  dockerops:latest
```

### Déploiement dans le Swarm

Un exemple de stack est fourni dans [deploy/dockerops-stack.yml](deploy/dockerops-stack.yml).

1. **Créer le secret Swarm pour le token GitHub :**
   ```bash
   echo -n "ghp_votre_token" | docker secret create github_token -
   ```

2. **Adapter le fichier** `deploy/dockerops-stack.yml` : mettre à jour `DOCKEROPS_REPOS` avec vos URLs, et éventuellement l’image (registry si besoin).

3. **Déployer le stack :**
   ```bash
   docker stack deploy -c deploy/dockerops-stack.yml dockerops
   ```

Le service DockerOps doit s’exécuter sur un nœud **manager** (contrainte `node.role == manager` dans l’exemple) pour accéder au socket Docker et déployer les stacks sur le même Swarm. L’image inclut un entrypoint qui exporte `GITHUB_TOKEN` depuis `/run/secrets/github_token` si ce fichier est présent (secret Swarm monté).

### Persistance

Le volume `dockerops-data` (ou le chemin défini par `DOCKEROPS_DB_PATH`) doit être persistant pour conserver le cache des repositories et l’état des stacks. Sans volume, les données sont perdues au redémarrage du conteneur.

---

Pour toute question ou problème, ouvrir une issue sur [GitHub](https://github.com/TomBedinoVT/DockerOps).
