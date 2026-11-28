> 🌐 [English](../../en/deploy/VERIFY.md) · **Українська**

# Перевірка продакшн-розгортання

Використовуйте цей контрольний список після розгортання Elyra на приватному
сервері. Він підтверджує, що стек працює справно, API доступний через публічну
HTTPS-кінцеву точку, дані зберігаються після перезапуску, а шлях резервного
копіювання та відновлення є робочим.

Для власного домену або приватного розгортання Elyra не потребує **жодних змін у вихідному коді**.
Значення, специфічні для сайту, містяться у `infrastructure/deployment/.env.prod`,
у DNS та у полі десктопного застосунку **Налаштування → Elyra Cloud → URL сервера**.
Для інсталяторів встановіть змінну репозиторію GitHub `ELYRA_DEFAULT_API_URL`.
Не змінюйте робочу копію, щоб оновлення залишалося звичайним `git pull`.

Повний посібник з налаштування міститься у [SERVER.md](SERVER.md); ця сторінка
є супровідним документом для перевірок і smoke-тестів.

## 1. Перед розгортанням

- [ ] Робоча копія чиста й перебуває на потрібному коміті або тезі.
- [ ] `.env.prod` існує, має режим доступу `600` і не відстежується Git.
- [ ] `POSTGRES_PASSWORD`, `REDIS_PASSWORD` та `ELYRA_JWT_SECRET` — унікальні
      випадкові значення; `ELYRA_JWT_SECRET` має щонайменше 32 символи.
- [ ] `ELYRA_DOMAIN` вказує на цей сервер (записи `A`/`AAAA`), коли TLS увімкнено.
- [ ] Файрвол дозволяє SSH і, за використання Caddy, лише порти 80/443.
      PostgreSQL, Redis та API не опубліковані зовні.
- [ ] `WEB_BIND=127.0.0.1`, коли профіль `tls` увімкнено.
- [ ] Налаштування SMTP-релею є навмисними або `ELYRA_SMTP_HOST` порожній.
- [ ] Перед будь-яким оновленням або зміною конфігурації існує резервна копія.

Перевірте конфігурацію, не запускаючи нічого:

```sh
./infrastructure/deployment/deploy.sh --check
```

Запустіть автоматичні тести (див. [Автоматичні тести](#2-automated-tests)).
Потім розгорніть, наприклад:

```sh
./infrastructure/deployment/deploy.sh --backup --tls
```

<a id="2-automated-tests"></a>
## 2. Автоматичні тести

Тести без зовнішніх залежностей завжди запускаються і не потребують жодних
сервісів:

```sh
cargo test -p elyra-server
```

Тести API та схеми, що працюють із базою даних, використовують тимчасову базу
PostgreSQL. Вони пропускаються, якщо база даних недосяжна, тому звичайний
`cargo test` ніколи не падає лише через відсутність Docker:

```sh
ELYRA_TEST_DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra_test \
  cargo test -p elyra-server
```

У Windows PowerShell:

```powershell
$env:ELYRA_TEST_DATABASE_URL = "postgres://elyra:elyra@127.0.0.1:5432/elyra_test"
cargo test -p elyra-server
```

`ELYRA_TEST_DATABASE_URL` — це явна згода на участь: у такому разі помилки
підключення або міграції призводять до невдалого запуску. `DATABASE_URL`
використовується як резервний варіант лише коли він доступний.
Робочий процес CI для `Rust` уже виконує `cargo test -p elyra-crypto -p
elyra-server -p elyra-launcher`, тому ці команди не потребують зміни робочого
процесу; сервісний контейнер CI може задати `ELYRA_TEST_DATABASE_URL`, щоб
увімкнути тести з базою даних.

Покриття:

| Область | Файл |
| --- | --- |
| Автентифікація (хешування паролів, підпис JWT, термін дії, неправильний секрет) | `server/src/auth.rs` |
| Валідація вхідних даних API, розбір bearer-токена, відображення помилок | `server/src/api.rs` |
| Реєстрація/вхід, курсор синхронізації, поштовий релей у режимі fail-closed | `server/src/api.rs` (з базою даних) |
| Конфіденційність сповіщень і перевірка прав власника на читання | `server/src/api.rs` (з базою даних) |
| Ідемпотентність схеми, унікальність, upsert, каскадні видалення | `server/tests/database_integration.rs` |
| Нормалізація конфігурації SMTP і поведінка при вимкненому релеї | `server/src/mailer.rs` |

## 3. Публічні smoke-тести

Встановіть базову адресу публічного API (вона має закінчуватися на `/api`) і
підтвердьте, що сервіс активний:

```sh
BASE=https://mail.example.com/api
curl -fsS "$BASE/health"
# {"service":"elyra-server","status":"ok"}
```

Зареєструйте тимчасового користувача з немаршрутизованою адресою:

```sh
VERIFY_EMAIL="verify-$(date +%s)@example.invalid"
RESPONSE=$(curl -sS -X POST "$BASE/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$VERIFY_EMAIL\",\"password\":\"verification-passphrase\",\"display_name\":\"Verifier\"}")
printf '%s\n' "$RESPONSE"
```

Скопіюйте `access_token` з відповіді або скористайтеся `jq`, якщо його
встановлено:

```sh
TOKEN=$(printf '%s' "$RESPONSE" | jq -r .access_token)
test -n "$TOKEN" && echo "token acquired"
```

Перевірки з автентифікацією:

```sh
curl -fsS "$BASE/v1/users/me" -H "Authorization: Bearer $TOKEN"

curl -fsS -X PUT "$BASE/v1/sync/cursor" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"cursor":"verify-1"}'
curl -fsS "$BASE/v1/sync/cursor" -H "Authorization: Bearer $TOKEN"
# {"cursor":"verify-1","synced_at":"..."}

CREATED=$(curl -sS -X POST "$BASE/v1/notifications" \
  -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d "{\"recipient_email\":\"$VERIFY_EMAIL\",\"title\":\"Verify\",\"body\":\"Smoke test\"}")
printf '%s\n' "$CREATED"
NOTE_ID=$(printf '%s' "$CREATED" | jq -r .id)

curl -fsS "$BASE/v1/notifications" -H "Authorization: Bearer $TOKEN"
curl -sS -o /dev/null -w '%{http_code}\n' -X PATCH \
  "$BASE/v1/notifications/$NOTE_ID/read" -H "Authorization: Bearer $TOKEN"
# 204

curl -fsS "$BASE/v1/mail/status" -H "Authorization: Bearer $TOKEN"
```

Очікувані результати:

- [ ] `/health` повертає `status: ok` через проксі `/api`.
- [ ] Реєстрація повертає сесію та `user.id`.
- [ ] `me` повертає той самий ідентифікатор користувача.
- [ ] Повний цикл курсора зберігає значення й встановлює `synced_at`.
- [ ] Створення сповіщення повертає `201`, список містить його, позначення як
      прочитане повертає `204`.
- [ ] Відсутні або недійсні bearer-токени повертають `401` (перевірка нижче).
- [ ] `mail/status` повідомляє значення `configured`, що відповідає вашому
      `.env.prod`.

Негативні перевірки безпеки:

```sh
curl -sS -o /dev/null -w '%{http_code}\n' "$BASE/v1/users/me"                    # 401
curl -sS -o /dev/null -w '%{http_code}\n' "$BASE/v1/users/me" \
  -H 'Authorization: Bearer not-a-jwt'                                           # 401
curl -sS -o /dev/null -w '%{http_code}\n' -X POST "$BASE/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$VERIFY_EMAIL\",\"password\":\"verification-passphrase\"}"    # 409
```

Якщо релей повідомляє `configured: true`, надішліть рівно один лист на поштову
скриньку, якою ви керуєте, і підтвердьте доставку, заголовок `Reply-To`,
результати SPF/DKIM/DMARC та відсутність обмеження частоти запитів у релеї
0.4.0. Не використовуйте публічного користувача перевірки для масового
надсилання.

## 4. Перевірки HTTPS (профіль TLS)

```sh
curl -fsSI https://mail.example.com/ | head -n 1
curl -sSI https://mail.example.com/ | grep -i strict-transport-security
echo | openssl s_client -servername mail.example.com -connect mail.example.com:443 2>/dev/null \
  | openssl x509 -noout -dates
curl -sSI http://mail.example.com/ | head -n 1
```

- [ ] Сайт HTTPS і `/api/health` завантажуються з дійсним сертифікатом.
- [ ] Терміни дії сертифіката актуальні, а оновлення не зазнають помилок у
      `docker compose ... logs caddy`.
- [ ] `Strict-Transport-Security` присутній; звичайний HTTP перенаправляється
      на HTTPS.
- [ ] Десктопний застосунок виконує вхід із URL сервера
      `https://mail.example.com/api`.

## 5. Збереження даних і перезапуск

- [ ] Запишіть ідентифікатор сповіщення або значення курсора з кроку 3.
- [ ] Перезапустіть контейнер API:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml restart api
```

- [ ] `/health` знову повертає `ok`.
- [ ] Той самий `TOKEN` все ще автентифікує, а збережені курсор/сповіщення
      присутні. Токени зберігаються після перезапусків, доки
      `ELYRA_JWT_SECRET` не змінено.

## 6. Тренування резервного копіювання та відновлення

- [ ] `deploy.sh --backup` створює непорожній стиснутий gzip-дамп у `backups/`.
- [ ] `gzip -t backups/<file>.sql.gz` виконується успішно.
- [ ] Копія існує поза сервером (об'єктне сховище або інший хост).
- [ ] Відновлення було відпрацьовано на проміжному екземплярі за процедурою з
      [SERVER.md §9](SERVER.md#9-backups-and-maintenance), після чого перевірено
      вхід і дані.
- [ ] Відомий шлях відкату: перейти на попередній робочий тег, повторно
      розгорнути з `--backup` і відновити відповідний дамп бази даних, якщо
      схема змінилася.

## 7. Очищення та підтвердження результатів

Видаліть тимчасового користувача (підставте значення бази даних з `.env.prod`):

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml \
  exec -T postgres psql -U elyra -d elyra \
  -c "DELETE FROM users WHERE email LIKE 'verify-%@example.invalid';"
```

- [ ] Користувача перевірки видалено.
- [ ] У журналах немає неочікуваних помилок під час smoke-тестів.
- [ ] Моніторинг охоплює `/health`, використання диска та перезапуски
      контейнерів.
- [ ] Результат зафіксовано: дата, коміт/тег, домен, відповідальний і подальші
      дії.

## Пов'язані сторінки

- [Посібник із сервера](SERVER.md) — налаштування, секрети, TLS, резервні
  копії, усунення несправностей.
- [Огляд розгортання](../deployment.md) — стек і швидкий старт.
- [Сервер і API](../server.md) — маршрути, тіла запитів і коди помилок.
- [Розробка](../development.md) — локальне середовище та процедури
  тестування.