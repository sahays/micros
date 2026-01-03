#!/bin/bash

# Install systemd service for managing Docker Compose
install_systemd_service() {
    cat > /etc/systemd/system/app.service <<'EOF'
[Unit]
Description=Application Containers
After=docker.service
Requires=docker.service

[Service]
Type=oneshot
RemainAfterExit=yes
WorkingDirectory=/app/current
ExecStart=/usr/bin/docker compose up -d
ExecStop=/usr/bin/docker compose down
User=deploy

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable app.service
}
