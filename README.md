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

Exécuter avec `sudo` (ex. `sudo dockerops watch "https://github.com/user/repo"`).

## Documentation

Documentation complète (installation, configuration, volumes, secrets, authentification GitHub, dépannage) : **[DOCUMENTATION.md](DOCUMENTATION.md)**.

## Licence

Voir [LICENSE](LICENSE).
