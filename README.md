# DockerOps CLI

Outil CLI en Rust pour gérer les stacks Docker Swarm depuis des répertoires GitHub : déploiement, synchronisation, volumes NFS et secrets.

## Installation

```bash
curl -sSL https://raw.githubusercontent.com/TomBedinoVT/DockerOps/main/dockerops.sh | sudo bash -s install
```

## Commandes

| Commande | Description |
|----------|-------------|
| `watch <url>` | Déployer un repository GitHub |
| `reconcile` | Synchroniser les repositories (`--force` pour forcer le redéploiement) |
| `stop` | Arrêter toutes les stacks et nettoyer |
| `version` | Afficher la version |
| `debug-cache` | Afficher le cache des repositories |
| `run` | Mode daemon (conteneur/Swarm) : synchronisation automatique à intervalle |

Exécuter avec `sudo` (ex. `sudo dockerops watch "https://github.com/user/repo"`).

## Docker et déploiement dans le Swarm

Vous pouvez dockeriser DockerOps et le déployer comme un service dans le Swarm (mode type ArgoCD) : voir **[DOCUMENTATION.md#14-exécution-en-conteneur-et-dans-le-swarm](DOCUMENTATION.md#14-exécution-en-conteneur-et-dans-le-swarm)** (build image, variables d'environnement, fichier [deploy/dockerops-stack.yml](deploy/dockerops-stack.yml)).

## Documentation

Documentation complète (installation, configuration, volumes, secrets, authentification GitHub, dépannage, exécution en conteneur) : **[DOCUMENTATION.md](DOCUMENTATION.md)**.

## Licence

Voir [LICENSE](LICENSE).
