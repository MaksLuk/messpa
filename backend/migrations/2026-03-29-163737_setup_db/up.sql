CREATE TYPE roles AS ENUM (
    'unverified',
    'client',
    'executor',
    'moderator',
    'support',
    'admin',
    'devops',
    'dispatcher'
);

CREATE TYPE languages AS ENUM (
    'ru',
    'en'
);

CREATE TYPE currencies AS ENUM (
    'RUB'
);

-- Пользователи
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE,
    role roles NOT NULL DEFAULT 'unverified',
    telegram_id BIGINT UNIQUE,
    display_name VARCHAR(100),
    avatar_url TEXT,
    banner_url TEXT,
    description TEXT,
    language languages NOT NULL DEFAULT 'ru',
    currency currencies NOT NULL DEFAULT 'RUB',
    is_executor BOOLEAN DEFAULT FALSE,
    register_at TIMESTAMPTZ DEFAULT NOW()
);

-- Специализации жестко заданы
-- Можно убрать и хранить их текстом в таблице user_info_executor
CREATE TABLE specializations (
    id SERIAL PRIMARY KEY ,
    name_ru VARCHAR(25) NOT NULL,           -- название специальности на русском и на английском
    name_en VARCHAR(25) NOT NULL
);

-- Информация о пользователе-исполнителе 
CREATE TABLE user_info_executor (
    user_id UUID PRIMARY KEY REFERENCES users(id),
    specialization INTEGER REFERENCES specializations(id),
    rating NUMERIC(3,2) DEFAULT 0,
    review_count INTEGER DEFAULT 0,
    completed_orders INTEGER DEFAULT 0,
    timezone VARCHAR(50),
    work_schedule JSONB,                    -- график работы
    contact_rules JSONB                     -- разрешённые способы связи
);

-- Команды/кланы
CREATE TABLE teams (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    banner_url TEXT,
    logo_url TEXT,
    specializations INTEGER[],              -- массив id из таблицы specializations
    public_contacts JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TYPE team_roles AS ENUM (
    'owner',
    'admin',
    'manager',
    'executor'
);

-- Члены команды/клана
CREATE TABLE team_members (
    team_id UUID REFERENCES teams(id),
    user_id UUID REFERENCES users(id),
    role team_roles NOT NULL,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (team_id, user_id)
);

