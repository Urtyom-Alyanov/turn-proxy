#!/bin/bash
set -e

REPO="Urtyom-Alyanov/turn-proxy"
GITHUB_API="https://api.github.com/repos/$REPO/releases/latest"

echo "Поиск последней версии $REPO..."

LATEST_RELEASE=$(curl -s $GITHUB_API)
VERSION=$(echo "$LATEST_RELEASE" | grep -Po '"tag_name": "\K.*?(?=")')

echo "Найдена версия: $VERSION"

if [ -f /etc/debian_version ]; then
    PKG_EXT="deb"
    INSTALL_CMD="sudo apt install"
    echo "Определен дистрибутив: Debian/Ubuntu"
elif [ -f /etc/redhat-release ] || [ -f /etc/fedora-release ]; then
    PKG_EXT="rpm"
    INSTALL_CMD="sudo rpm -Uvh"
    echo "Определен дистрибутив: RHEL/Fedora/CentOS"
else
    echo "Ошибка: Дистрибутив не поддерживается установщиком (нужен .deb или .rpm), если вы на NixOS или Arch Linux следуйте инструкции из README.md"
    exit 1
fi

DOWNLOAD_URL=$(echo "$LATEST_RELEASE" | grep -Po '"browser_download_url": "\K.*?'"$PKG_EXT"'(?=")' | head -n 1)

if [ -z "$DOWNLOAD_URL" ]; then
    echo "Ошибка: Не удалось найти пакет .$PKG_EXT в релизе."
    exit 1
fi

FILE_NAME=$(basename "$DOWNLOAD_URL")

echo "Скачиваем $FILE_NAME..."
curl -L -o "/tmp/$FILE_NAME" "$DOWNLOAD_URL"

echo "Установка..."
$INSTALL_CMD "/tmp/$FILE_NAME"

sudo systemctl daemon-reload

echo "--- Установка завершена! ---"
echo "Теперь вы можете запустить сервис:"
echo "sudo systemctl enable --now turn-proxy"
