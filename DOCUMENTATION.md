# Documentation DockerOps

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

Les secrets sont lus depuis le NFS et injectés comme variables d'environnement lors de `docker stack deploy`.

### Chemin des secrets

Les secrets sont stockés sous **`{nfs.path}/secret/{id}`**, où `nfs.path` est celui défini dans `nfs.yaml`. Exemple avec `path: "/mnt/nfs/dockerops"` :

- `database_password` → `/mnt/nfs/dockerops/secret/database_password`
- `api_key` → `/mnt/nfs/dockerops/secret/api_key`

(Il ne s'agit pas de `/secrets/` au niveau système : tout est sous le chemin NFS.)

### Format secrets.yaml

Fichier dans le dossier de chaque stack (à côté du docker-compose) :

```yaml
- id: database_password
  env: DB_PASSWORD

- id: api_key
  env: API_SECRET_KEY
```

- **id** : identifiant du secret (nom du fichier sous `{nfs.path}/secret/`).
- **env** : nom de la variable d'environnement injectée.

### Création des secrets sur NFS

```bash
sudo mkdir -p /mnt/nfs/dockerops/secret
echo "mon_mot_de_passe_secret" | sudo tee /mnt/nfs/dockerops/secret/database_password
sudo chmod 600 /mnt/nfs/dockerops/secret/database_password
sudo chown root:root /mnt/nfs/dockerops/secret/database_password
```

### Utilisation dans docker-compose

Les variables sont injectées automatiquement ; les référencer dans `environment` sans valeur :

```yaml
services:
  app:
    image: myapp:latest
    environment:
      - DB_PASSWORD
      - API_SECRET_KEY
```

Le fichier docker-compose reste inchangé ; DockerOps passe les paires env/valeur au processus `docker stack deploy`. Si un fichier secret est manquant, le déploiement échoue.

### Bonnes pratiques sécurité

- Permissions restrictives (600), propriétaire root.
- Ne jamais stocker les secrets dans le repository GitHub.
- Rotation régulière, accès limité au répertoire NFS des secrets.
- Les valeurs ne sont pas loggées.

Script de création sécurisée (exemple) :

```bash
SECRET_DIR="/mnt/nfs/dockerops/secret"
create_secret() {
    local id=$1
    local file="$SECRET_DIR/$id"
    read -s -p "Enter secret for $id: " secret
    echo
    echo -n "$secret" | sudo tee "$file" > /dev/null
    sudo chmod 600 "$file"
    sudo chown root:root "$file"
}
create_secret "database_password"
```

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

Structure : `stacks.yaml`, `nfs.yaml`, `api-stack/docker-compose.yml`, `api-stack/secrets.yaml`. Secrets sous `/mnt/nfs/dockerops/secret/` (ex. `db_password`, `jwt_secret`). Variables d'environnement déclarées sans valeur dans le compose.

### Exemple : multi-stacks avec volumes et secrets

Plusieurs stacks (ex. web-stack, api-stack), `volumes.yaml` et `nfs.yaml` communs, `secrets.yaml` par stack si besoin. Même principe : volumes et secrets définis une fois, référencés dans chaque compose.

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
- **Problèmes NFS** : `mount | grep nfs`, `ls -la /mnt/nfs/dockerops`, cohérence avec `nfs.yaml`.

### Commandes de debug

```bash
sudo dockerops debug-cache
sudo dockerops reconcile
docker stack ls
docker stack services <stack-name>
docker service ls
docker service ps <service-name>
docker service logs <service-name>
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

Pour toute question ou problème, ouvrir une issue sur [GitHub](https://github.com/TomBedinoVT/DockerOps).
