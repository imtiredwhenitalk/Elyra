> 🌐 [English](../en/deployment.md) · **Українська**

# Розгортання

Виробниче розгортання Elyra — це невеликий стек Docker Compose для одного Linux VPS. Він обслуговує публічний вебсайт, проксує автентифікований API на Rust, зберігає дані сервера в PostgreSQL і може завершувати HTTPS за допомогою Caddy.

Покроковий посібник — DNS, брандмауер, секрети, TLS, SMTP-ретранслятор, резервні копії та GitHub Actions — міститься в [інструкції для сервера](deploy/SERVER.md). Ця сторінка дає огляд і швидкий старт.

## Що розгортається

```text
Caddy (optional, profile: tls)
  │  80/443, automatic Let's Encrypt certificates
  ▼
web (Nginx)
  │  static site + /api/ proxy + /health proxy
  ├──────────────▶ api (Rust/Axum, :8787)
  │                   │
  │                   ▼
  │                postgres (:5432)
  │
  └──────────────▶ redis (:6379, reserved for future use)
```

| Служба | Образ/збірка | Опубліковано | Призначення |
| --- | --- | --- | --- |
| `web` | `web/Dockerfile` (Nginx 1.27 Alpine) | `WEB_PORT` (типово 8080) | Вебсайт і проксі API |
| `api` | `server/Dockerfile` (Rust + Debian slim) | Ні | Автентифікація, користувачі, синхронізація, сповіщення, ретранслятор |
| `postgres` | `postgres:16-alpine` | Ні | Постійні дані сервера |
| `redis` | `redis:7-alpine` | Ні | Зарезервовано; сервер поки що не використовує |
| `caddy` | `caddy:2-alpine`, профіль `tls` | 80/443 | Завершення TLS і HSTS |

Файл Compose: `infrastructure/deployment/docker-compose.prod.yml`.
Шаблон змінних середовища: `infrastructure/deployment/.env.prod.example`.
Скрипт розгортання: `infrastructure/deployment/deploy.sh`.
Конфігурація Caddy: `infrastructure/deployment/Caddyfile`.

## Вимоги

- Linux-сервер із Docker Engine 24+ і плагіном Compose.
- Домен або субдомен для HTTPS.
- Доступ через SSH з користувачем без прав root, але з sudo.
- Для невеликого розгортання достатньо 1–2 vCPU та 1–2 ГБ ОЗП.
- Відкриті порти 80 і 443, якщо ви використовуєте Caddy.

Не публікуйте PostgreSQL (5432), Redis (6379) та API (8787).

## Швидкий старт без HTTPS

Використовуйте це лише для першої перевірки або за наявним зворотним проксі.

```sh
git clone https://github.com/imtiredwhenitalk/Elyra.git /opt/elyra
cd /opt/elyra
cp infrastructure/deployment/.env.prod.example infrastructure/deployment/.env.prod
chmod 600 infrastructure/deployment/.env.prod
```

Відредагуйте `.env.prod`:

```env
POSTGRES_PASSWORD=<long-random-value>
REDIS_PASSWORD=<different-long-random-value>
ELYRA_JWT_SECRET=<at-least-32-random-characters>
WEB_BIND=0.0.0.0
WEB_PORT=8080
```

Згенеруйте секрети за допомогою `openssl rand -hex 32`. Потім:

```sh
./infrastructure/deployment/deploy.sh
```

Скрипт перевіряє конфігурацію Compose, збирає образи API та вебсайту, запускає стек, чекає на `/health` і виводить URL-адреси. Відкрийте `http://SERVER_IP:8080`.

## HTTPS із Caddy

1. Направте DNS-запис `A` (за бажанням і `AAAA`) на сервер.
2. Встановіть у `.env.prod`:

```env
ELYRA_ENABLE_TLS=1
ELYRA_DOMAIN=mail.example.com
WEB_BIND=127.0.0.1
```

3. Виконайте розгортання:

```sh
./infrastructure/deployment/deploy.sh --tls
```

Caddy отримує та оновлює сертифікат. Публічні кінцеві точки:

- Вебсайт: `https://mail.example.com`
- Базовий URL API: `https://mail.example.com/api`
- Перевірка стану: `https://mail.example.com/api/health`

`WEB_BIND=127.0.0.1` приховує звичайний HTTP-порт вебсайту від публічного інтерфейсу. Доступні лише порти Caddy 80/443.

## Налаштування десктопного застосунку на сервер

У Elyra: **Налаштування → Elyra Cloud → URL сервера**, введіть `https://mail.example.com/api`, збережіть, потім підключіться та зареєструйтеся.

Для інсталяторів випуску встановіть змінну репозиторію GitHub `ELYRA_DEFAULT_API_URL=https://mail.example.com/api` перед створенням тегу. Робочий процес випуску вбудовує її у збірку.

Базовий URL API має:

- Використовувати HTTPS у виробничому середовищі.
- Містити `/api`.
- Не мати кінцевої похилої риски.

## Необов'язкова поштова ретрансляція

Ретранслятор дозволяє зареєстрованим користувачам надсилати листи на зовнішні адреси без налаштування власного облікового запису SMTP. Він вимкнений, якщо не встановлено `ELYRA_SMTP_HOST`.

```env
ELYRA_SMTP_HOST=smtp.example.com
ELYRA_SMTP_PORT=587
ELYRA_SMTP_SECURITY=starttls
ELYRA_SMTP_USERNAME=no-reply@example.com
ELYRA_SMTP_PASSWORD=app-password
ELYRA_SMTP_FROM=no-reply@example.com
ELYRA_SMTP_FROM_NAME=Elyra
```

- Використовуйте `tls` на порту 465 для неявного TLS; інакше `starttls` на порту 587.
- Опублікуйте записи SPF, DKIM і DMARC для домену відправлення.
- Використовуйте окрему поштову скриньку або провайдера транзакційних листів.
- Будь-який зареєстрований користувач може надсилати листи. Перед відкриттям для широкого доступу додайте обмеження частоти запитів.

Див. [Сервер і API → SMTP-ретранслятор](server.md#smtp-relay) щодо поведінки та кодів помилок.

## Параметри скрипта розгортання

```sh
./infrastructure/deployment/deploy.sh --check     # validate configuration only
./infrastructure/deployment/deploy.sh --backup    # dump PostgreSQL first
./infrastructure/deployment/deploy.sh --no-build  # reuse existing images
./infrastructure/deployment/deploy.sh --tls       # enable Caddy profile
./infrastructure/deployment/deploy.sh --help
```

Скрипт завершується зрозумілою помилкою, якщо бракує Docker, плагіна Compose або файлу змінних середовища. Він ніколи не зберігає секрети поза `.env.prod`.

## Автоматичне розгортання через GitHub Actions

Робочий процес `Deploy server` працює за явним увімкненням (opt-in).

1. Створіть SSH-ключ для розгортання та додайте його публічну частину до `~/.ssh/authorized_keys` на сервері.
2. Додайте секрети репозиторію:
   - `ELYRA_DEPLOY_HOST`
   - `ELYRA_DEPLOY_USER`
   - `ELYRA_DEPLOY_KEY`
   - `ELYRA_DEPLOY_PORT` (необов'язково, типово 22)
   - `ELYRA_DEPLOY_PATH` (наприклад, `/opt/elyra`)
3. Додайте змінну репозиторію `ELYRA_DEPLOY_ENABLED=true`.
4. За бажанням встановіть `ELYRA_DEFAULT_API_URL`.

Робочий процес спрацьовує на push до `main`, що змінює `server/`, `web/`, `packages/`, `infrastructure/deployment/`, `infrastructure/web/`, `Cargo.toml`, `Cargo.lock` або сам файл процесу. Його також можна запустити вручну.

Він виконує:

```sh
git fetch --prune origin main
git reset --hard origin/main
./infrastructure/deployment/deploy.sh --backup
```

Попередні контейнери продовжують працювати, доки нову збірку не буде успішно завершено.

<a id="backups"></a>
## Резервні копії

Ручне створення резервної копії:

```sh
./infrastructure/deployment/deploy.sh --backup
```

Або безпосередньо:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml \
  exec -T postgres sh -c 'pg_dump --clean --if-exists -U "$POSTGRES_USER" "$POSTGRES_DB"' \
  | gzip > backups/elyra-$(date +%Y%m%d-%H%M%S).sql.gz
```

Відновлення (спершу зупиніть API, потім запустіть його знову):

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml stop api

gunzip -c backups/elyra-YYYYMMDD-HHMMSS.sql.gz \
  | docker compose --env-file infrastructure/deployment/.env.prod \
      -f infrastructure/deployment/docker-compose.prod.yml \
      exec -T postgres sh -c 'psql -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB"'

docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml start api
```

Нові дампи використовують `--clean --if-exists`, тому їх можна відтворити в наявній базі даних. Старі дампи потребують порожньої бази. Повну процедуру відновлення, включно з перевіркою архівів і верифікацією, див. у [SERVER.md §9](deploy/SERVER.md#9-backups-and-maintenance).

Копіюйте резервні копії з сервера та періодично перевіряйте відновлення. Сервер не зберігає окремо завантажені файли.

## Оновлення

```sh
cd /opt/elyra
git pull --ff-only
./infrastructure/deployment/deploy.sh --backup
```

Зміни схеми виконуються під час запуску API як ідемпотентні інструкції `CREATE TABLE IF NOT EXISTS`. Перед оновленням між великими версіями перегляньте `server/src/database.rs`; руйнівні міграції не автоматизуються.

## Відкат

Стек збирається з поточного вивантаженого вихідного коду. Щоб виконати відкат:

```sh
cd /opt/elyra
git checkout <previous-good-tag>
./infrastructure/deployment/deploy.sh --backup
```

Якщо схема бази даних змінилася несумісно з попередньою версією, відновіть відповідну резервну копію PostgreSQL. Зберігайте попередні значення `.env.prod`; зміна секрету JWT анулює всі сеанси.

## Контрольний список посилення безпеки

- [ ] Брандмауер дозволяє лише SSH, HTTP і HTTPS.
- [ ] `POSTGRES_PASSWORD`, `REDIS_PASSWORD` та `ELYRA_JWT_SECRET` є унікальними.
- [ ] `.env.prod` має режим `600` і ніколи не комітується.
- [ ] PostgreSQL, Redis і API не опубліковано.
- [ ] `WEB_BIND=127.0.0.1`, коли Caddy увімкнено.
- [ ] DNS вказує на сервер до запиту сертифіката.
- [ ] Резервні копії створюються за розкладом, а відновлення перевіряється.
- [ ] Для домену ретранслятора існують SPF, DKIM і DMARC.
- [ ] До відкриття публічної реєстрації передбачено заходи захисту від зловживань реєстрацією та ретранслятором.
- [ ] Логи та `/health` перебувають під моніторингом.

## Перевірка

Після кожного розгортання виконуйте [контрольний список перевірки виробничого середовища](deploy/VERIFY.md). Він охоплює перевірки конфігурації перед розгортанням, перевірочні запити до публічного та автентифікованого API, перевірки HTTPS і HSTS, збереження стану після перезапуску, тренування резервного копіювання та відновлення, відкат і остаточне підтвердження готовності. Також там наведено команди автоматизованих тестів; CI уже запускає `cargo test -p elyra-server`, а контейнер служби бази даних може увімкнути тести з базою через `ELYRA_TEST_DATABASE_URL`.

Налаштування, специфічні для сайту, ніколи не потребують зміни відстежуваного вихідного коду: використовуйте `.env.prod`, DNS, URL сервера в десктопному застосунку та змінну репозиторію `ELYRA_DEFAULT_API_URL`.

## Усунення несправностей

Повна таблиця симптомів міститься в [інструкції для сервера](deploy/SERVER.md#10-troubleshooting) та [Усуненні несправностей](troubleshooting.md). Швидкі перевірки:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml ps
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs --tail=100 api
curl -fsS https://mail.example.com/api/health
```

## Пов'язані сторінки

- [Інструкція для сервера](deploy/SERVER.md) — повний покроковий посібник для VPS.
- [Сервер і API](server.md) — маршрути, модель даних, ретранслятор.
- [Безпека](security.md) — модель загроз і контрольний список для адміністратора.
- [Довідник конфігурації](reference/configuration.md) — змінні розгортання.
- [Усунення несправностей](troubleshooting.md) — типові проблеми.
