# Docker Compose Operations Guide

Quick reference for managing the Micros infrastructure with docker-compose.

## Prerequisites

- Ensure `.env` file exists at project root
- Run all commands from the project root directory

## Running Infrastructure

```bash
# Start all services
docker-compose up -d

# Start only infrastructure (no apps)
docker-compose up -d prometheus loki grafana promtail mongo redis

# Start with logs visible
docker-compose up

# View logs
docker-compose logs -f
docker-compose logs -f auth-service    # specific service
```

## Updating Services

```bash
# After code changes - rebuild and restart
docker-compose up -d --build

# Rebuild specific service
docker-compose up -d --build auth-service

# After .env changes - restart
docker-compose down
docker-compose up -d

# Restart without rebuilding
docker-compose restart auth-service
```

## Destroying/Cleanup

```bash
# Stop services (keep data)
docker-compose stop

# Stop and remove containers (keep data)
docker-compose down

# Remove containers AND data
docker-compose down -v

# Remove everything
docker-compose down -v --rmi all
```

## Monitoring

```bash
# Check service status
docker-compose ps

# View resource usage
docker stats

# Access container shell
docker-compose exec auth-service sh
docker-compose exec mongo mongosh
```

## Common Workflows

### Daily Operations
```bash
# Start
docker-compose start

# Stop
docker-compose stop
```

### Development Cycle
```bash
# After code changes
docker-compose up -d --build

# Clean restart (keep data)
docker-compose down && docker-compose up -d

# Fresh start (delete data)
docker-compose down -v && docker-compose up -d
```

## Service Access URLs

| Service | URL |
|---------|-----|
| Auth Service | http://localhost:9096 |
| Secure Frontend | http://localhost:9097 |
| Prometheus | http://localhost:9090 |
| Grafana | http://localhost:9092 |
| Loki | http://localhost:9091 |
| MongoDB | mongodb://localhost:9094 |
| Redis | redis://localhost:9095 |

## Important Notes

- `.env` changes require: `docker-compose down && docker-compose up -d`
- Code changes require: `docker-compose up -d --build`
- `-v` flag deletes volumes (MongoDB data will be lost)
- Always check logs if services fail: `docker-compose logs -f`
