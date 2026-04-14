#!/bin/bash
# ================================================
# Тестовый скрипт для Telegram Auth API
# Проверяет полный flow: send-code → verify → refresh → logout
# ================================================

set -euo pipefail

# ================= НАСТРОЙКИ =================
BASE_URL="http://localhost:3001/api/auth"
TELEGRAM_CHAT_ID="123456789"          # ← Замени на свой реальный Telegram Chat ID (или тестовый)
USE_JQ=true                           # Если установлен jq — вывод будет красивым

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }

command -v curl >/dev/null 2>&1 || error "curl не установлен"
if $USE_JQ; then
    command -v jq >/dev/null 2>&1 || { warn "jq не установлен. Вывод будет сырым."; USE_JQ=false; }
fi

echo "=== Тестирование Auth API ==="
echo "Base URL: $BASE_URL"
echo "Telegram Chat ID: $TELEGRAM_CHAT_ID"
echo "========================================"

# 1. Отправка кода в Telegram
echo -e "\n1. Отправка кода подтверждения..."
SEND_RESPONSE=$(curl -s -X POST "$BASE_URL/telegram/send-code" \
  -H "Content-Type: application/json" \
  -d "{\"telegram_chat_id\": $TELEGRAM_CHAT_ID}")

echo "$SEND_RESPONSE" | if $USE_JQ; then jq .; else cat; fi

if echo "$SEND_RESPONSE" | grep -q '"ok":true'; then
    log "Код успешно отправлен в Telegram"
else
    error "Не удалось отправить код"
fi

# Просим пользователя ввести код
echo -e "\n${YELLOW}Проверь Telegram и введи полученный 6-значный код:${NC}"
read -r CODE

if [[ ! $CODE =~ ^[0-9]{6}$ ]]; then
    error "Код должен состоять из 6 цифр"
fi

# 2. Подтверждение кода (login)
echo -e "\n2. Подтверждение кода и получение токенов..."
VERIFY_RESPONSE=$(curl -s -X POST "$BASE_URL/telegram/verify" \
  -H "Content-Type: application/json" \
  -c cookies.txt \
  -d "{\"telegram_chat_id\": $TELEGRAM_CHAT_ID, \"code\": \"$CODE\"}")

echo "$VERIFY_RESPONSE" | if $USE_JQ; then jq .; else cat; fi

ACCESS_TOKEN=$(echo "$VERIFY_RESPONSE" | jq -r '.data.access_token // empty')

if [[ -n "$ACCESS_TOKEN" ]]; then
    log "Успешный вход! Access token получен."
else
    error "Не удалось получить access_token"
fi

# 3. Проверка /me (защищённый маршрут)
echo -e "\n3. Проверка защищённого маршрута /me..."
ME_RESPONSE=$(curl -s -X GET "$BASE_URL/me" \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -b cookies.txt)

echo "$ME_RESPONSE" | if $USE_JQ; then jq .; else cat; fi

if echo "$ME_RESPONSE" | grep -q '"ok":true'; then
    log "Защищённый маршрут /me работает"
else
    warn "Проблема с /me (возможно нужен middleware для JWT)"
fi

# 4. Refresh token
echo -e "\n4. Тест обновления токена (refresh)..."
REFRESH_RESPONSE=$(curl -s -X POST "$BASE_URL/refresh" \
  -b cookies.txt \
  -c cookies.txt)

echo "$REFRESH_RESPONSE" | if $USE_JQ; then jq .; else cat; fi

if echo "$REFRESH_RESPONSE" | grep -q '"ok":true'; then
    log "Refresh token успешно отработал"
else
    warn "Refresh не прошёл"
fi

# 5. Logout (текущая сессия)
echo -e "\n5. Выход из текущей сессии (logout)..."
LOGOUT_RESPONSE=$(curl -s -X POST "$BASE_URL/logout" \
  -b cookies.txt \
  -c cookies.txt)

echo "$LOGOUT_RESPONSE" | if $USE_JQ; then jq .; else cat; fi

log "Тест logout завершён"

echo -e "\n${GREEN}=== Все тесты завершены ===${NC}"
echo "Cookies сохранены в файл: cookies.txt"
echo "Для повторного теста просто запусти скрипт заново."
