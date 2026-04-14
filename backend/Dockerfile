# ==================== BUILDER STAGE ====================
FROM rust:1.88-slim AS builder

WORKDIR /build

# Устанавливаем зависимости для сборки
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Теперь копируем весь исходный код и собираем настоящий бинарник
COPY . .

RUN cargo build --release

# ==================== RUNTIME STAGE ====================
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libpq5 \
    libssl3 \
    libc6 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /build/target/release/freelance-backend ./freelance-backend
COPY --from=builder /build/migrations ./migrations

CMD ["./freelance-backend"]
