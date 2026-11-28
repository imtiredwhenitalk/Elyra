> 🌐 [English](../en/server.md) · **Українська**

# Сервер і API

Сервер Elyra — це невеликий сервіс на Rust із автентифікацією. Він забезпечує реєстрацію та вхід до облікових записів, метадані користувачів, курсори синхронізації, сповіщення всередині застосунку та необов'язкову вихідну SMTP-ретрансляцію. Він **не** зберігає поштові скриньки, паролі IMAP, ключі шифрування десктопного застосунку або тіла повідомлень.

Ця сторінка є технічною довідкою. Для покрокового розгортання на VPS див. [Розгортання](deployment.md) та [інструкцію для сервера](deploy/SERVER.md).

## Що зберігає сервер

| Що зберігається | Що не зберігається |
| --- | --- |
| Ідентифікатор користувача, електронна адреса, відображуване ім'я, хеш пароля Argon2id | Паролі IMAP/SMTP |
| Курсор синхронізації для кожного користувача | Вміст поштової скриньки або вкладення |
| Сповіщення всередині застосунку | Ключі шифрування десктопного застосунку |
| Необов'язкова конфігурація ретрансляції (лише через змінні оточення) | Тіла повідомлень, надісланих через ретранслятор |

Десктопний застосунок взаємодіє з сервером через HTTPS. Трафік IMAP і SMTP надходить безпосередньо від десктопного застосунку до поштового провайдера й ніколи не проходить через сервер Elyra.

## Локальна розробка

### 1. Запустіть PostgreSQL

Використайте стек Compose для розробки:

```sh
./infrastructure/build/program.sh up
```

Або запустіть лише PostgreSQL із файлу `infrastructure/docker/docker-compose.yaml`. Стек для розробки також надає Redis і Mailpit.

### 2. Налаштуйте оточення

```sh
cp server/.env.example server/.env
```

Вкажіть принаймні:

```env
DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra
ELYRA_JWT_SECRET=replace-with-at-least-32-random-characters
ELYRA_SERVER_ADDR=127.0.0.1:8787
```

Згенеруйте секрет за допомогою `openssl rand -hex 32`.

### 3. Запуск

```sh
cargo run -p elyra-server
```

Сервер завантажує `server/.env` через `dotenvy`, виконує міграції `CREATE TABLE IF NOT EXISTS`, прив'язується до налаштованої адреси та виводить у журнал `Elyra server listening`.

### 4. Перевірка працездатності

```sh
curl -fsS http://127.0.0.1:8787/health
# {"service":"elyra-server","status":"ok"}

curl -fsS -X POST http://127.0.0.1:8787/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"you@example.com","password":"correct horse battery","display_name":"You"}'
```

Використовуйте отриманий `access_token` у заголовку `Authorization: Bearer <token>` для захищених маршрутів.

## Конфігурація

Усі змінні сервера перелічено в [довідці з конфігурації](reference/configuration.md#server). Обов'язкові з них:

| Змінна | Обов'язкова | Примітки |
| --- | --- | --- |
| `DATABASE_URL` | Так | Рядок підключення до PostgreSQL; максимум 10 з'єднань у пулі |
| `ELYRA_JWT_SECRET` | Так | Щонайменше 32 символи; інакше сервер відмовляється запускатися |
| `ELYRA_SERVER_ADDR` | Ні | Типово `127.0.0.1:8787`; Docker встановлює `0.0.0.0:8787` |
| `ELYRA_ACCESS_TOKEN_TTL_HOURS` | Ні | Типово `24` |
| `RUST_LOG` | Ні | Типово `elyra_server=info,tower_http=info` |
| `ELYRA_SMTP_HOST` | Ні | Порожнє значення вимикає ретрансляцію |
| `ELYRA_SMTP_FROM` | Якщо ретрансляція увімкнена | Неправильне або відсутнє значення блокує запуск |
| Інші `ELYRA_SMTP_*` | Ні | Порт, режим захисту, ім'я користувача, пароль, ім'я відправника |

Сервер не має типових значень для секретів у продакшені. Продакшен-файл Compose використовує `${VAR:?message}`, щоб відсутні значення виявлялись одразу.

## Запуск і робота

```text
dotenvy::dotenv
  → tracing subscriber
  → Database::connect_from_env
  → Database::migrate
  → AuthService::from_env
  → MailService::from_env
  → axum::serve(router)
  → graceful shutdown on Ctrl+C
```

Проміжні обробники:

- `CorsLayer::permissive()` — дозволяє браузерному прев'ю та локальним інструментам викликати API. У продакшені вебсайт проксує запити з того самого походження (same-origin); сервер усе одно вмикає ліберальну конфігурацію CORS.
- `TraceLayer::new_for_http()` — трасування запитів через `tower-http`.

Окрім PostgreSQL і конфігурації оточення, сервер не зберігає стану. JWT залишаються чинними після перезапусків, доки не зміниться `ELYRA_JWT_SECRET`.

## Модель даних

```sql
users (
  id UUID PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  display_name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)

sync_cursors (
  user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  cursor TEXT,
  synced_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)

notifications (
  id UUID PRIMARY KEY,
  sender_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  recipient_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  body TEXT NOT NULL,
  is_read BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
)
```

Міграції — це ідемпотентні інструкції `CREATE TABLE IF NOT EXISTS`, які виконуються під час запуску. Таблиці версій міграцій немає. Руйнівні зміни схеми не автоматизуються; перед оновленням між великими версіями перевірте `server/src/database.rs`.

## Автентифікація

### Реєстрація та вхід

`POST /v1/auth/register` і `POST /v1/auth/login` приймають:

```json
{
  "email": "you@example.com",
  "password": "correct horse battery",
  "display_name": "You"
}
```

Поле `display_name` необов'язкове під час входу; якщо його немає в будь-якому з запитів, використовується `Elyra user`.

Валідація:

- Електронна адреса має містити `@` і бути не довшою за 320 символів.
- Пароль має містити щонайменше 12 символів.
- Відображуване ім'я має бути непорожнім після обрізання пробілів і не довшим за 120 символів.
- Реєстрація повертає `409 conflict`, якщо такий самий рядок електронної адреси вже зареєстровано.

Відповідь:

```json
{
  "access_token": "<jwt>",
  "expires_at": "2026-11-17T12:00:00Z",
  "user": {
    "id": "0f3f6c1e-...",
    "email": "you@example.com",
    "display_name": "You",
    "created_at": "2026-11-16T12:00:00Z"
  }
}
```

### Токени

- JWT HS256 із твердженнями `{ "sub": "<user uuid>", "exp": <unix seconds> }`.
- Час життя (TTL) визначається `ELYRA_ACCESS_TOKEN_TTL_HOURS`, типово 24 години.
- Надсилайте як `Authorization: Bearer <token>`.
- Токенів оновлення немає. Після завершення терміну дії токену увійдіть знову.
- Зміна `ELYRA_JWT_SECRET` анулює всі наявні токени.

### Хешування паролів

Паролі хешуються за допомогою Argon2id через `argon2::Argon2::default()` із випадковою сіллю, отриманою від ОС. Хеш зберігається в `users.password_hash`; паролі у відкритому вигляді ніколи не зберігаються й не потрапляють до журналів.

## Довідка API

Базова URL-адреса під час локальної розробки: `http://127.0.0.1:8787`.
Базова URL-адреса у продакшені: `https://<domain>/api` (проксі Nginx вебсайту
видаляє префікс `/api` перед пересиланням запиту до контейнера API).

Усі тіла запитів і відповідей мають формат JSON. Захищені маршрути вимагають заголовка bearer.

### `GET /health`

Публічна кінцева точка перевірки доступності (liveness).

```json
{ "status": "ok", "service": "elyra-server" }
```

Перевірка стану в продакшен-Compose шукає `"status":"ok"` усередині контейнера.

### `POST /v1/auth/register`

Публічний. Створює користувача й повертає сесію.

Запит:

```json
{
  "email": "you@example.com",
  "password": "correct horse battery",
  "display_name": "You"
}
```

Відповідь `200`:

```json
{
  "access_token": "<jwt>",
  "expires_at": "2026-11-17T12:00:00Z",
  "user": {
    "id": "0f3f6c1e-...",
    "email": "you@example.com",
    "display_name": "You",
    "created_at": "2026-11-16T12:00:00Z"
  }
}
```

Помилки: `400 bad_request`, `409 conflict`, `500 internal_error`.

### `POST /v1/auth/login`

Публічний. Перевіряє облікові дані й повертає сесію такої самої структури, як під час реєстрації.

Помилки: `400 bad_request`, `401 auth_error` із `invalid credentials`, `500 internal_error`.

### `GET /v1/users/me`

Захищений. Повертає поточний `UserRecord`.

```json
{
  "id": "0f3f6c1e-...",
  "email": "you@example.com",
  "display_name": "You",
  "created_at": "2026-11-16T12:00:00Z"
}
```

Помилки: `401 unauthorized`, `401 auth_error`, `404 not_found`, `500 internal_error`.

### `GET /v1/sync/cursor`

Захищений. Повертає курсор поточного користувача.

```json
{ "cursor": "opaque-value-or-null", "synced_at": "2026-11-16T12:00:00Z" }
```

Якщо курсора не існує:

```json
{ "cursor": null, "synced_at": null }
```

Курсор — це непрозорий рядок. Десктопна версія 0.4.0 надає команди отримання та оновлення, але не запускає запланований цикл синхронізації.

### `PUT /v1/sync/cursor`

Захищений. Замінює курсор поточного користувача та оновлює `synced_at`.

Запит:

```json
{ "cursor": "opaque-value" }
```

Щоб очистити курсор, можна вказати `cursor` як `null`. Відповідь: `204 No Content`.

Помилки: `401 unauthorized`, `500 internal_error`.

### `GET /v1/notifications`

Захищений. Повертає до 100 сповіщень для поточного користувача, спочатку найновіші.

```json
[
  {
    "id": "7a1c...",
    "sender_id": "0f3f...",
    "recipient_id": "9b2d...",
    "title": "Welcome",
    "body": "Your Elyra account is ready.",
    "is_read": false,
    "created_at": "2026-11-16T12:00:00Z"
  }
]
```

Сповіщення обмежені умовою `recipient_id = caller`. Сповіщення інших користувачів ніколи не повертаються.

### `POST /v1/notifications`

Захищений. Надсилає сповіщення всередині застосунку іншому зареєстрованому користувачеві.

Запит:

```json
{
  "recipient_email": "friend@example.com",
  "title": "Hello",
  "body": "This is an in-app notification, not email."
}
```

Валідація:

| Поле | Правило |
| --- | --- |
| `recipient_email` | Містить `@`, не більше 320 символів |
| `title` | Не порожнє після обрізання пробілів, не більше 160 символів |
| `body` | Не порожнє після обрізання пробілів, не більше 4000 символів |

Відповідь `201`:

```json
{
  "id": "7a1c...",
  "sender_id": "0f3f...",
  "recipient_id": "9b2d...",
  "title": "Hello",
  "body": "This is an in-app notification, not email.",
  "is_read": false,
  "created_at": "2026-11-16T12:00:00Z"
}
```

Помилки: `400 bad_request`, `401 unauthorized`, `404 not_found`, якщо одержувача не існує, `500 internal_error`.

### `PATCH /v1/notifications/:id/read`

Захищений. Позначає одне сповіщення прочитаним **лише якщо** воно належить поточному користувачеві.

Відповідь: `204 No Content`.

Помилки: `401 unauthorized`, `404 not_found`, якщо ідентифікатор не існує або належить іншому користувачеві.

### `GET /v1/mail/status`

Захищений. Повідомляє, чи налаштовано ретрансляцію.

```json
{ "configured": true, "from": "no-reply@example.com" }
```

Коли ретрансляцію вимкнено:

```json
{ "configured": false, "from": null }
```

### `POST /v1/mail/send`

Захищений. Надсилає повідомлення у простому тексті через серверну ретрансляцію.

Запит:

```json
{
  "to": "friend@example.com",
  "subject": "Hello from Elyra",
  "body": "Plain text body."
}
```

Валідація:

| Поле | Правило |
| --- | --- |
| `to` | Містить `@`, не більше 320 символів |
| `subject` | Не порожнє після обрізання пробілів, не більше 200 символів |
| `body` | Не порожнє після обрізання пробілів, не більше 100 000 символів |

Відповідь `200`:

```json
{ "status": "sent" }
```

Повідомлення надсилається з адреси `ELYRA_SMTP_FROM` (з ім'ям `ELYRA_SMTP_FROM_NAME`), а заголовок `Reply-To` встановлюється на електронну адресу та відображуване ім'я користувача, який увійшов у систему. Сервер не зберігає тіло повідомлення.

Помилки:

| Статус | Код | Значення |
| --- | --- | --- |
| 400 | `bad_request` | Неправильний одержувач, тема або тіло повідомлення |
| 400 | `invalid_recipient` | `lettre` відхилив адресу одержувача |
| 400 | `invalid_reply_to` | Електронну адресу користувача, який увійшов у систему, не можна використати як Reply-To |
| 401 | `unauthorized` / `auth_error` | Відсутній або неправильний токен |
| 503 | `mail_not_configured` | `ELYRA_SMTP_HOST` порожній |
| 502 | `mail_delivery_failed` | Помилка підключення SMTP, автентифікації або доставки |

## Формат помилок

Кожна помилка API має однакову структуру:

```json
{ "code": "bad_request", "message": "password must contain at least 12 characters" }
```

| Статус | Код | Коли |
| --- | --- | --- |
| 400 | `bad_request` | Помилка валідації |
| 401 | `unauthorized` | Відсутній або некоректний заголовок bearer |
| 401 | `auth_error` | Неправильні облікові дані або неправильний/прострочений токен |
| 404 | `not_found` | Користувача, сповіщення або ресурс не знайдено |
| 409 | `conflict` | Електронна адреса вже зареєстрована |
| 500 | `internal_error` | Помилка бази даних або неочікуваний збій; деталі записуються в журнал, але не повертаються клієнту |
| 502 | `mail_delivery_failed` | Ретранслятору не вдалося доставити повідомлення |
| 503 | `mail_not_configured` | Ретрансляцію вимкнено |

Помилки бази даних записуються за допомогою `tracing::error!` і повертаються як загальне повідомлення `internal_error`, щоб внутрішні деталі не потрапляли до клієнтів.

<a id="smtp-relay"></a>
## SMTP-ретрансляція

Ретрансляція необов'язкова й вимкнена, доки не встановлено `ELYRA_SMTP_HOST`.

```env
ELYRA_SMTP_HOST=smtp.example.com
ELYRA_SMTP_PORT=587
ELYRA_SMTP_SECURITY=starttls
ELYRA_SMTP_USERNAME=no-reply@example.com
ELYRA_SMTP_PASSWORD=app-password
ELYRA_SMTP_FROM=no-reply@example.com
ELYRA_SMTP_FROM_NAME=Elyra
```

Поведінка:

- `ELYRA_SMTP_SECURITY=tls` використовує неявний TLS (зазвичай порт 465).
- Будь-яке інше значення, зокрема типове, використовує STARTTLS (зазвичай порт 587).
- Якщо хост вказано, але `ELYRA_SMTP_FROM` відсутній або неправильний, сервер **не запускається**, а не працює з несправною ретрансляцією.
- Ретрансляція використовує `lettre` із rustls і перевіркою сертифікатів.
- Тіло повідомлення не зберігається.
- Ретрансляцією може користуватися будь-який зареєстрований користувач. У версії 0.4.0 немає обмежень частоти, квот або контролю зловживань для окремих користувачів.

Перед відкриттям доступу до ретрансляції опублікуйте записи SPF, DKIM і DMARC для домену відправника та додайте обмеження частоти. Використовуйте окрему поштову скриньку для надсилання або транзакційного провайдера, а не особисту пошту.

## Інтеграція з десктопним застосунком

Десктопний клієнт викликає сервер через `cloud.rs` та `services/sendemail.rs`.

| Команда десктопного застосунку | Маршрут сервера |
| --- | --- |
| `cloud_register` | `POST /v1/auth/register` |
| `cloud_login` | `POST /v1/auth/login` |
| `cloud_session` | Лише локальне сховище ключів |
| `cloud_logout` | Локальне сховище ключів + очищення `settings` |
| `cloud_list_notifications` | `GET /v1/notifications` |
| `cloud_send_notification` | `POST /v1/notifications` |
| `cloud_mark_notification_read` | `PATCH /v1/notifications/:id/read` |
| `cloud_sync_cursor` | `GET /v1/sync/cursor` |
| `cloud_update_cursor` | `PUT /v1/sync/cursor` |
| `cloud_health` | `GET /health` |
| `cloud_mail_status` | `GET /v1/mail/status` |
| Резервний варіант `send_email` | `POST /v1/mail/send` |

### Визначення URL API

Десктопний застосунок визначає базову адресу API в такому порядку:

1. Змінна оточення `ELYRA_API_URL`.
2. Запис SQLite `settings.api_url`.
3. `ELYRA_DEFAULT_API_URL`, вбудований в інсталятор.
4. `http://127.0.0.1:8787`.

Кінцеві похилі риски вилучаються. Продакшен-адреси обов'язково мають містити `/api`, оскільки публічний сайт під Nginx проксує `/api/` до контейнера API.

## Тестування

Сервер має чисті тести, які запускаються будь-де, та тести з базою даних, які коректно пропускаються, якщо не надано тимчасовий екземпляр PostgreSQL:

```sh
cargo test -p elyra-server

ELYRA_TEST_DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra_test \
  cargo test -p elyra-server
```

`ELYRA_TEST_DATABASE_URL` — явне ввімкнення; непридатна адреса спричиняє помилку запуску тестів. `DATABASE_URL` використовується лише як резервний варіант, якщо вона доступна. Тести охоплюють автентифікацію та роботу з JWT, валідацію API, обробку заголовка bearer і помилок, реєстрацію та вхід, курсори синхронізації, конфіденційність сповіщень, поведінку з вимкненою SMTP-ретрансляцією та обмеження схеми. Жоден тест не надсилає справжній SMTP-трафік. Детальніше див. [Розробка → Автоматичні тести](development.md#automated-tests), а для перевірок розгортання — [Перевірка у продакшені](deploy/VERIFY.md).

## Експлуатаційні нотатки

### Стан і журнали

```sh
curl -fsS http://127.0.0.1:8787/health
RUST_LOG=elyra_server=debug,tower_http=debug cargo run -p elyra-server
```

У продакшені:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs -f api
```

### Доступ до бази даних

Сервер підключається з пулом до 10 з'єднань. PostgreSQL не опубліковано в продакшен-стеку. Для обслуговування використовуйте `docker compose exec`.

<a id="backups"></a>
### Резервні копії

Створюйте резервні копії PostgreSQL за допомогою `pg_dump`; див. [інструкцію для сервера](deploy/SERVER.md#9-backups-and-maintenance). Сервер не зберігає файлів, які потребують окремого резервування.

### CORS

API вмикає ліберальну конфігурацію CORS. Продакшен-вебсайт проксує запити з того самого походження, тому для сайту CORS не потрібен. Він призначений для браузерного прев'ю та локальних інструментів. Якщо ви надаєте прямий доступ до API, обмежте CORS довіреними походженнями.

### Відомі обмеження сервера

- Адреси електронної пошти обрізаються та переводяться в нижній регістр на межі API, а унікальний індекс `lower(email)` забороняє варіанти з іншим регістром. Бази даних, що містять дублікати з різним регістром, потрібно спочатку об'єднати: сервер повідомить конфліктну адресу замість запуску з пошкодженою схемою.
- Немає токенів оновлення, відкликання токенів, скидання пароля, підтвердження електронної адреси та багатофакторної автентифікації.
- Немає ролі адміністратора, кодів запрошення, обмеження частоти реєстрації чи квот ретрансляції.
- Немає WebSocket або каналу реального часу.
- Redis розгорнуто, але сервер його не використовує.
- Приймаються лише тіла вихідних повідомлень у простому тексті; кінцева точка ретрансляції не підтримує вкладень.

Ці обмеження відстежуються у [`task.md`](../../task.md).

## Пов'язані сторінки

- [Архітектура](architecture.md) — як сервер вписується в систему.
- [Безпека](security.md) — межі довіри та модель загроз.
- [Довідка з конфігурації](reference/configuration.md) — усі змінні.
- [Розгортання](deployment.md) — продакшен-стек і HTTPS.
- [Вирішення проблем](troubleshooting.md) — симптоми та способи усунення проблем сервера.
