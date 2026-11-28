> 🌐 [English](../../en/deploy/SERVER.md) · **Українська**

# Розгортання сервера Elyra

Цей посібник допоможе перейти від свіжого сервера з Linux до працюючого
розгортання Elyra: вебсайту, API на Rust із автентифікацією, PostgreSQL,
необов'язкового HTTPS і необов'язкового вихідного поштового ретранслятора.
Його розраховано на один невеликий VPS (1–2 vCPU, 1–2 ГБ ОЗП).

Десктопний застосунок зберігає поштові облікові дані та дані IMAP локально.
Сервер зберігає лише облікові записи Elyra, курсори синхронізації, сповіщення
та необов'язкову конфігурацію SMTP-ретранслятора.

## 1. Передумови

- Ubuntu 22.04/24.04 (або будь-який Linux із Docker Engine 24 або новішим).
- Домен або субдомен, наприклад `mail.example.com`.
- Доступ через SSH із користувачем без прав root, який може використовувати sudo.
- Docker Engine і плагін Compose:

```sh
curl -fsSL https://get.docker.com | sudo sh
sudo usermod -aG docker "$USER"
# Log out and back in so the docker group applies.
docker compose version
```

Відкрийте у файрволі лише SSH, HTTP і HTTPS:

```sh
sudo ufw allow OpenSSH
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw enable
```

Не публікуйте PostgreSQL (`5432`), Redis (`6379`) та API (`8787`). Файл Compose
для виробничого середовища залишає їх у приватній мережі Docker.

## 2. Отримання коду

```sh
sudo mkdir -p /opt/elyra
sudo chown "$USER":"$USER" /opt/elyra
git clone https://github.com/imtiredwhenitalk/Elyra.git /opt/elyra
cd /opt/elyra
```

## 3. Налаштування секретів

```sh
cp infrastructure/deployment/.env.prod.example infrastructure/deployment/.env.prod
chmod 600 infrastructure/deployment/.env.prod
```

Відредагуйте `infrastructure/deployment/.env.prod` і замініть усі заповнювачі.
Секрети можна згенерувати так:

```sh
openssl rand -hex 32
```

Обов'язкові значення:

| Змінна | Призначення |
| --- | --- |
| `POSTGRES_PASSWORD` | Пароль PostgreSQL |
| `REDIS_PASSWORD` | Пароль Redis |
| `ELYRA_JWT_SECRET` | Ключ підпису JWT, не менше 32 символів |
| `WEB_BIND` | `127.0.0.1` з Caddy, `0.0.0.0` без нього |
| `WEB_PORT` | Порт хоста для вебсайту, типово `8080` |

| `ELYRA_ACCESS_TOKEN_TTL_HOURS` | Ні | Термін дії токена доступу в годинах, типово `24` |

`.env.prod` ігнорується Git. Ніколи не фіксуйте його в репозиторії.

Щоб розмістити Elyra на власному домені, не потрібно змінювати вихідний код:
домен, порти, секрети й налаштування ретранслятора надходять лише з `.env.prod`,
DNS та налаштування десктопного застосунку **Server URL**. Для інсталяторів
випусків задайте змінну репозиторію `ELYRA_DEFAULT_API_URL`. Якщо робоча копія
залишається незмінною, оновлення зводиться до звичайного `git pull`.

## 4. Розгортання

Без HTTPS (швидка перевірка: відкрийте `http://SERVER_IP:8080`):

```sh
./infrastructure/deployment/deploy.sh
```

Скрипт перевіряє файл Compose, збирає образи API та вебсайту, запускає стек,
чекає на `/health` і виводить URL. Використовуйте `--backup`, щоб спочатку
створити дамп PostgreSQL, і `--check`, щоб лише перевірити конфігурацію. Після
розгортання виконайте [перевірку виробничого розгортання](VERIFY.md): базові
перевірки працездатності, перевірку HTTPS, збереження стану після перезапуску
та підтвердження результатів.

Перевірте стек:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml ps
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs -f api
```

## 5. HTTPS із Caddy

Caddy автоматично отримує та оновлює сертифікати Let's Encrypt.

1. Створіть DNS-запис `A` для `ELYRA_DOMAIN`, що вказує на сервер. Якщо ви
   використовуєте IPv6, додайте також запис `AAAA`.
2. Встановіть такі значення у `.env.prod`:

```env
ELYRA_ENABLE_TLS=1
ELYRA_DOMAIN=mail.example.com
WEB_BIND=127.0.0.1
```

3. Розгорніть із профілем TLS:

```sh
./infrastructure/deployment/deploy.sh --tls
```

Caddy слухає порти `80` і `443`; Nginx і API залишаються недоступними ззовні.
Після видачі сертифіката публічні кінцеві точки такі:

- Вебсайт: `https://mail.example.com`
- API: `https://mail.example.com/api`
- Перевірка стану: `https://mail.example.com/api/health`

Якщо видача сертифіката не вдалася, переконайтеся, що DNS вказує на сервер і
порти 80/443 відкриті, після чого перевірте `docker compose ... logs caddy`.

## 6. Вихідний поштовий ретранслятор

Ретранслятор дозволяє зареєстрованим користувачам Elyra надсилати листи на
будь-яку зовнішню адресу, навіть якщо в них немає локального SMTP-облікового
запису. Він необов'язковий. Щоб вимкнути його, залиште `ELYRA_SMTP_HOST`
порожнім — тоді десктопний застосунок використовує власний поштовий обліковий
запис користувача.

Встановіть такі значення у `.env.prod`:

```env
ELYRA_SMTP_HOST=smtp.example.com
ELYRA_SMTP_PORT=587
ELYRA_SMTP_SECURITY=starttls
ELYRA_SMTP_USERNAME=no-reply@example.com
ELYRA_SMTP_PASSWORD=app-password
ELYRA_SMTP_FROM=no-reply@example.com
ELYRA_SMTP_FROM_NAME=Elyra
```

- Для неявного TLS використовуйте `ELYRA_SMTP_SECURITY=tls` із портом `465`.
- Використовуйте виділену поштову скриньку або сервіс транзакційної пошти, а не
  особисту скриньку.
- Опублікуйте записи SPF, DKIM і DMARC для домену надсилання. Без них зовнішні
  поштові сервіси відхилятимуть листи або поміщатимуть їх до папки «Спам».
- Gmail вимагає пароля застосунку (має бути увімкнено двофакторну
  автентифікацію). Також підходять сервіси транзакційної пошти, як-от Resend,
  Brevo, Mailgun або Postmark.
- Сервер ніколи не зберігає паролі IMAP користувачів. У `.env.prod` містяться
  лише облікові дані ретранслятора.
- Листи надсилаються від `ELYRA_SMTP_FROM`, а в заголовку `Reply-To` вказується
  електронна адреса Elyra користувача, який увійшов у систему, щоб отримувачі
  могли відповісти авторові.
- Перед відкриттям ретранслятора для загального доступу додайте обмеження
  частоти запитів і моніторинг. Надсилати через нього може будь-який
  зареєстрований користувач.

Перевірте ретранслятор після входу в десктопний застосунок:

```sh
curl -fsS https://mail.example.com/api/v1/mail/status \
  -H "Authorization: Bearer <access-token>"
```

## 7. Налаштування десктопного застосунку на сервер

У Elyra: **Налаштування → Elyra Cloud → URL сервера**, введіть
`https://mail.example.com/api`, натисніть **Зберегти сервер**, потім
**Підключити обліковий запис** і зареєструйтеся. Перший зареєстрований
користувач отримує звичайний обліковий запис; окремої ролі адміністратора поки
що немає.

Для інсталяторів випусків задайте змінну репозиторію GitHub
`ELYRA_DEFAULT_API_URL=https://mail.example.com/api`. Робочий процес випуску
вбудовує її до збірки десктопного застосунку, тому новим користувачам потрібно
лише зареєструватися.

## 8. Автоматичне розгортання за допомогою GitHub Actions

Робочий процес `Deploy server` запускається на кожному push до `main`, що
змінює `server/`, `web/` або `infrastructure/deployment/`, і його також можна
запустити вручну. Він вимкнений, доки ви його не увімкнете.

1. Створіть ключ розгортання на сервері:

```sh
ssh-keygen -t ed25519 -C "elyra-deploy" -f ~/.ssh/elyra_deploy -N ""
cat ~/.ssh/elyra_deploy.pub >> ~/.ssh/authorized_keys
cat ~/.ssh/elyra_deploy   # copy the private key
```

2. Додайте секрети репозиторію в **GitHub → Settings → Secrets and variables →
   Actions**:

| Тип | Назва | Значення |
| --- | --- | --- |
| Секрет | `ELYRA_DEPLOY_HOST` | IP-адреса або ім'я хоста сервера |
| Секрет | `ELYRA_DEPLOY_USER` | Користувач SSH, наприклад `ubuntu` |
| Секрет | `ELYRA_DEPLOY_KEY` | Вміст приватного ключа розгортання |
| Секрет | `ELYRA_DEPLOY_PORT` | Порт SSH, необов'язково, типово `22` |
| Секрет | `ELYRA_DEPLOY_PATH` | Абсолютний шлях, наприклад `/opt/elyra` |
| Змінна | `ELYRA_DEPLOY_ENABLED` | `true`, щоб увімкнути робочий процес |
| Змінна | `ELYRA_DEFAULT_API_URL` | `https://mail.example.com/api` |

3. Переконайтеся, що користувач розгортання може запускати Docker без `sudo` і
   володіє робочою копією (або має права на запис до неї). Робочий процес
   виконує `git reset --hard origin/main`, а потім `deploy.sh --backup`.

Робочий процес використовує `appleboy/ssh-action` і ніколи не виводить секрети.
Якщо розгортання не вдалося, попередні контейнери продовжують працювати, доки
нову збірку не буде успішно створено.

<a id="9-backups-and-maintenance"></a>
## 9. Резервні копії та обслуговування

Резервна копія вручну:

```sh
./infrastructure/deployment/deploy.sh --backup
# or
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml \
  exec -T postgres sh -c 'pg_dump --clean --if-exists -U "$POSTGRES_USER" "$POSTGRES_DB"' \
  | gzip > backups/elyra-$(date +%Y%m%d-%H%M%S).sql.gz
```

Нові дампи містять інструкції `DROP ... IF EXISTS`, тому їх можна застосувати
повторно до наявної бази даних. Для копій, створених до додавання цього прапорця,
перед імпортом потрібна порожня база даних (або видалена й повторно створена
схема `public`).

### Порядок відновлення

1. Виберіть перевірений архів і перевірте його цілісність:

   ```sh
   ls -lh backups/
   gzip -t backups/elyra-YYYYMMDD-HHMMSS.sql.gz
   ```

2. Зупиніть API, щоб під час відновлення до бази нічого не записувалося:

   ```sh
   docker compose --env-file infrastructure/deployment/.env.prod \
     -f infrastructure/deployment/docker-compose.prod.yml stop api
   ```

3. За можливості спочатку відпрацюйте відновлення на проміжному екземплярі або
   тимчасовій базі даних. Щоб відновити робочу базу даних:

   ```sh
   gunzip -c backups/elyra-YYYYMMDD-HHMMSS.sql.gz \
     | docker compose --env-file infrastructure/deployment/.env.prod \
         -f infrastructure/deployment/docker-compose.prod.yml \
         exec -T postgres sh -c 'psql -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "$POSTGRES_DB"'
   ```

   `ON_ERROR_STOP=1` гарантує, що відновлення завершиться явною помилкою, а не
   залишить напівімпортовану схему. Не ігноруйте ненульовий код завершення.

4. Запустіть API й переконайтеся, що він повторно застосував свої ідемпотентні
   міграції:

   ```sh
   docker compose --env-file infrastructure/deployment/.env.prod \
     -f infrastructure/deployment/docker-compose.prod.yml start api
   ```

5. Перевірте стан системи, вхід і відновлені дані за допомогою
   [перевірки виробничого розгортання](VERIFY.md). Якщо після створення копії
   змінився `ELYRA_JWT_SECRET`, користувачі мають увійти знову.

Копіюйте резервні копії з сервера (до об'єктного сховища або на інший хост) і
періодично перевіряйте відновлення. Окремих завантажених файлів, які потрібно
відновлювати, на сервері немає.

Оновлення:

```sh
cd /opt/elyra
git pull --ff-only
./infrastructure/deployment/deploy.sh --backup
```

Під час запуску сервер виконує міграції `CREATE TABLE IF NOT EXISTS`.
Руйнівні зміни схеми не автоматизуються; перед оновленням між основними
версіями перегляньте `server/src/database.rs`.

<a id="10-troubleshooting"></a>
## 10. Усунення несправностей

| Симптом | Що перевірити |
| --- | --- |
| `POSTGRES_PASSWORD must be set` | Файл `.env.prod` відсутній або не передано до Compose |
| API несправний | `docker compose ... logs api`, `DATABASE_URL`, стан PostgreSQL |
| Вебсайт завантажується, але API повертає 502 | Контейнер API не працює; Nginx проксіює `/api/` на `api:8787` |
| Помилка сертифіката Caddy | DNS вказує на сервер, порти 80/443 відкриті, `logs caddy` |
| Надсилання листа повертає `mail_not_configured` | `ELYRA_SMTP_HOST` на сервері порожній |
| Надсилання листа повертає `mail_delivery_failed` | Облікові дані SMTP, SPF/DKIM/DMARC, обмеження постачальника |
| Десктопний застосунок не може увійти | URL сервера має закінчуватися на `/api` і використовувати HTTPS у виробничому середовищі |
| 401 після перезапуску сервера | `ELYRA_JWT_SECRET` змінився; увійдіть знову |

Корисні команди:

```sh
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml ps
docker compose --env-file infrastructure/deployment/.env.prod \
  -f infrastructure/deployment/docker-compose.prod.yml logs --tail=100 api
curl -fsS https://mail.example.com/api/health
```