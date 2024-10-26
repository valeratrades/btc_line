#!/bin/sh

if [ "$(id -u)" -ne 0 ]; then
    echo "This script requires root privileges. Please run as root or use sudo."
    exit 1
fi

cat <<EOF > /etc/systemd/system/btc_line.service
[Unit]
Description=btc_line https://github.com/Valera6/btc_line
After=network.target

[Service]
ExecStart=/usr/local/bin/btc_line
User=$(whoami)
Group=root
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

systemctl daemon-reload
systemctl enable btc_line.service
systemctl start btc_line.service

echo "btc_line service has been installed and started successfully."
