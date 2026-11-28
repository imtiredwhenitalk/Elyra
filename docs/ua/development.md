> 🌐 [English](../en/development.md) · **Українська**

# Розробка

Ця сторінка пояснює, як збирати, тестувати та випускати Elyra з вихідного коду. Передбачається, що ви працюєте в корені репозиторію.

## Передумови

| Інструмент | Версія | Для чого потрібен |
| --- | --- | --- |
| Node.js | 20 або новіша | Скрипти пакета десктопу, перевірки JavaScript, Tauri CLI |
| npm | У комплекті з Node | Встановлення залежностей десктопу |
| Rust | Стабільний toolchain | Ядро десктопу, сервер, криптографія, лаунчер |
| Cargo | У комплекті з Rust | Збірка й тестування робочого простору |
| Передумови Tauri 2 | Див. нижче | Збірка десктопу на Windows |
| Docker | Docker Engine 24+ із плагіном Compose | Розробка та розгортання сервера |
| PostgreSQL | 16 (або Docker) | Локальний запуск сервера |
| Git | Будь-яка актуальна версія | Керування вихідним кодом |

### Передумови Tauri для Windows

Встановіть:

- Microsoft C++ Build Tools із робочим навантаженням **Desktop development with C++**.
- WebView2 Runtime (входить до складу поточних Windows 10/11).
- Стабільний Rust для цільової платформи `x86_64-pc-windows-msvc`.

Перевірте:

```powershell
rustc --version
cargo --version
node --version
npm.cmd --version
```

### Передумови сервера на Linux

Для роботи лише з сервером достатньо стабільного Rust і PostgreSQL. Рекомендується Docker, оскільки стек розробки надає PostgreSQL, Redis і Mailpit.

## Отримання вихідного коду

```powershell
git clone https://github.com/imtiredwhenitalk/Elyra.git
cd Elyra
```

Репозиторій — це робочий простір Cargo з чотирма учасниками:

- `packages/crypto`
- `server`
- `apps/launcher`
- `apps/desktop/src-tauri`

Фронтенд десктопу не є учасником робочого простору; це статичні файли.

## Встановлення залежностей десктопу

```powershell
npm.cmd install --prefix apps/desktop
```

Пакет десктопу має лише Tauri CLI як залежність для розробки та `@tauri-apps/api` як залежність під час виконання. Немає ні збирача, ні React, ні Vite.

## Загальні команди

З кореня репозиторію:

| Команда | Призначення |
| --- | --- |
| `npm.cmd run check` | Виконати `node --check` для вхідної точки десктопу |
| `npm.cmd run build` | Зібрати пакети MSI та NSIS для Windows |
| `npm.cmd run tauri dev` | Запустити десктопний застосунок у режимі розробки |
| `npm.cmd run server:check` | `cargo check -p elyra-server` |
| `npm.cmd run crypto:test` | `cargo test -p elyra-crypto` |
| `npm.cmd run launcher:build` | Зібрати лаунчер випуску |
| `npm.cmd run release:desktop` | Псевдонім збірки пакетів десктопу |
| `npm.cmd run release:all` | Перевірити JS, зібрати десктоп, зібрати лаунчер |

З `apps/desktop`:

```powershell
npm.cmd run check
npm.cmd run tauri dev
npm.cmd run build
```

### Перевірка JavaScript

`npm.cmd run check` перевіряє лише `src/main.js`. Для повної перевірки синтаксису всіх скриптів десктопу та вебсайту виконайте:

```powershell
Get-ChildItem apps\desktop\src,web -Recurse -Filter *.js |
  ForEach-Object { node --check $_.FullName }
```

Фронтенд десктопу завантажується безпосередньо з `apps/desktop/src` через `frontendDist`. Зміни в HTML, CSS або JS з'являються після перезавантаження; окремий етап стеження за змінами у збирачі не потрібен.

### Перевірка Rust

```powershell
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
cargo check -p tauri-app
```

CI-процес запускає тести на Linux і збирає десктопний застосунок на Windows. Для повної локальної збірки десктопу потрібні передумови Tauri для Windows.

### Перевірка сервера

```powershell
cargo check -p elyra-server
cargo test -p elyra-server
```

<a id="automated-tests"></a>
### Автоматизовані тести

Набір тестів сервера містить чисті тести та тести, що працюють із базою даних, у трьох розташуваннях:

| Рівень | Розташування | Без PostgreSQL |
| --- | --- | --- |
| Чисті модульні тести | `server/src/auth.rs`, `server/src/mailer.rs`, `server/src/api.rs` | Виконуються звичайно |
| Інтеграційні тести API | `server/src/api.rs` (`#[cfg(test)]`) | Тести з базою даних пропускаються з повідомленням |
| Інтеграційні тести схеми | `server/tests/database_integration.rs` | Пропускаються з повідомленням |

Тести з базою даних охоплюють реєстрацію/вхід, безпеку bearer-токенів, цикли збереження та зчитування курсора синхронізації, приватність сповіщень і право позначати їх прочитаними, поведінку з відмовою за замовчуванням (fail-closed) для вимкненого ретранслятора, ідемпотентність схеми, унікальність адрес електронної пошти, upsert курсорів і каскадне видалення. Жоден тест не надсилає справжній SMTP-трафік.

Тому звичайний `cargo test -p elyra-server` працює в будь-якому середовищі. Щоб запустити повний набір тестів, спрямуйте їх на тимчасову базу даних:

```sh
ELYRA_TEST_DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra_test \
  cargo test -p elyra-server
```

Еквівалент у PowerShell:

```powershell
$env:ELYRA_TEST_DATABASE_URL = "postgres://elyra:elyra@127.0.0.1:5432/elyra_test"
cargo test -p elyra-server
```

`ELYRA_TEST_DATABASE_URL` — це явний дозвіл (opt-in): некоректна URL-адреса або невдала міграція спричиняє помилку тестового запуску. `DATABASE_URL` використовується як резервний лише тоді, коли він доступний; інакше тести з базою даних виводять повідомлення про пропуск. CI-процес `Rust` уже запускає `cargo test -p elyra-crypto -p elyra-server -p
elyra-launcher` на Linux, тому чисті тести виконуються автоматично. Контейнер служби CI може встановити `ELYRA_TEST_DATABASE_URL`, щоб також обов'язково виконувати тести з базою даних; файл робочого процесу не потрібно змінювати, щоб підхопити нові тести.

### Процедури тестування

Локальний перевірочний запуск (smoke test) сервера після `cargo run -p elyra-server` (див. [Запуск сервера локально](#run-the-server-locally)):

```sh
curl -fsS http://127.0.0.1:8787/health
curl -sS -X POST http://127.0.0.1:8787/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"dev@example.test","password":"local-dev-passphrase","display_name":"Dev"}'
# Use the returned access_token as: -H "Authorization: Bearer <token>"
```

Для виробничих розгортань натомість дотримуйтеся [контрольного списку перевірки виробничого середовища](deploy/VERIFY.md): він містить перевірки перед розгортанням, перевірку HTTPS, збереження стану після перезапуску, тренування резервного копіювання та відновлення й остаточне підтвердження готовності.

### Форматування та лінтинг

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

Перед комітуванням змін у Rust очікується `cargo fmt`. У версії 0.4.0 Clippy не перевіряється примусово в CI, але його рекомендується використовувати.

## Запуск десктопного застосунку

### Режим десктопу

```powershell
npm.cmd run tauri dev
```

Це запускає вікно Tauri з ядром Rust. SQLite, сховище ключів, IMAP, SMTP, хмарні виклики та криптографічні команди активні.

### Попередній перегляд у браузері

Відкрийте `apps/desktop/src/index.html` у браузері або обслуговуйте цей каталог:

```powershell
npx --yes serve apps/desktop/src
```

`services/tauri.js` виявляє відсутність середовища виконання Tauri і використовує `localStorage` як резерв для даних поштової скриньки та налаштувань. Попередній перегляд корисний для роботи з макетом, але не залучає Rust, SQLite, сховище ключів, IMAP, SMTP чи криптографію. Деякі дії навмисно показують повідомлення про попередній перегляд.

<a id="run-the-server-locally"></a>
## Запуск сервера локально

### З Docker

```sh
cp infrastructure/docker/.env.example infrastructure/docker/.env
./infrastructure/build/program.sh check
./infrastructure/build/program.sh up
```

Це запускає PostgreSQL, Redis і Mailpit. Вебінтерфейс Mailpit доступний за адресою `http://localhost:8025`.

Зупинка:

```sh
./infrastructure/build/program.sh down
```

### Без Docker

Спочатку самостійно запустіть PostgreSQL 16, потім:

```sh
cp server/.env.example server/.env
# edit DATABASE_URL and ELYRA_JWT_SECRET
cargo run -p elyra-server
```

Сервер застосовує свою схему під час запуску та слухає на `ELYRA_SERVER_ADDR` (типово `127.0.0.1:8787`).

### Налаштування десктопу на локальний сервер

У застосунку: **Налаштування → Elyra Cloud → URL сервера**, введіть `http://127.0.0.1:8787`, збережіть, потім зареєструйтеся або увійдіть. Також можна встановити `ELYRA_API_URL=http://127.0.0.1:8787` перед запуском.

## Правила репозиторію

### Rust

- Залишайте привілейовану логіку в Rust. Не переносьте логіку поштової скриньки, секретів або мережі у webview.
- Використовуйте параметризовані SQL-запити. Ніколи не підставляйте дані, введені користувачем, до рядка запиту.
- Повертайте `Result<T, String>` із команд Tauri та показуйте зрозумілі помилки, які можна виправити.
- Використовуйте `#[tauri::command(async)]` для блокуючих мережевих або довготривалих операцій, щоб вони виконувалися поза головним потоком.
- Не записуйте секрети до логів, помилок і серіалізованих даних, що передаються у webview.
- Дотримуйтеся наявної структури модулів і найменувань.

### JavaScript

- Використовуйте наявну обгортку `invoke` у `services/tauri.js`.
- Екрануйте вміст від користувача або провайдера за допомогою `escapeHtml` перед вставкою в HTML.
- Де це можливо, зберігайте працездатність інтерфейсу в браузерному попередньому перегляді, але не імітуйте результати перевірок безпеки на сервері.
- Віддавайте перевагу невеликим функціям і наявному підходу до стану та рендерингу.
- Використовуйте вбудований SVG-спрайт (`icon(name)`) замість зображень піктограм.

### Документація

- Оновлюйте `docs/`, коли змінюється поведінка.
- Додавайте запис до `changes.md` із маркером `[200 OK]`, який використовує проєкт.
- Оновлюйте `task.md` відповідно до стану дорожньої карти.
- Підтримуйте `README.md` та `ai.md` у відповідності до реального стану.

### Зміни архітектури

Правила проєкту в [`ai.md`](../../ai.md) вимагають явного дозволу перед зміною архітектури. Додавання можливості в межах наявних меж дозволено; додавання служби, бази даних, фреймворку або нової межі довіри — ні. Фіксуйте додатки в `changes.md`.

## Додавання можливості

Типовий вертикальний зріз:

1. **Дані** — додайте стовпець або таблицю у відповідному модулі Rust з ідемпотентною міграцією (`ensure_column` або `CREATE TABLE IF NOT EXISTS`).
2. **Домен** — реалізуйте запит або зміну даних у Rust із валідацією.
3. **Команда** — додайте `#[tauri::command]` і зареєструйте його в `lib.rs`.
4. **Місток** — відкрийте її через `services/tauri.js`, якщо інтерфейс використовує обгортку.
5. **Інтерфейс** — додайте елемент керування, стан, рендеринг і обробку помилок.
6. **Документація** — за потреби оновіть довідник команд, посібник користувача та архітектуру.
7. **Валідація** — виконайте перевірки JavaScript, перевірки й тести Rust і ручний перевірочний запуск.
8. **Журнал** — додайте запис до `changes.md` і оновіть `task.md`.

### Додавання команди Tauri

```rust
#[tauri::command]
pub fn example_command(app: AppHandle, value: String) -> Result<String, String> {
    // validate, access SQLite/keychain/network, return a safe result
    Ok(value)
}
```

Зареєструйте її:

```rust
.invoke_handler(tauri::generate_handler![
    // ...
    module::example_command,
])
```

Опишіть її в [`docs/reference/tauri-commands.md`](reference/tauri-commands.md).

### Додавання маршруту сервера

Додайте маршрут у `server/src/api.rs`, перевіряйте кожні вхідні дані, використовуйте наявні варіанти `ApiError` і опишіть корисне навантаження та коди помилок у [Сервері й API](server.md).

## Процес випуску

### 1. Оновлення версій

Оновіть версію в:

- `Cargo.toml` → `[workspace.package] version`
- `apps/desktop/package.json`
- `package.json`
- `apps/desktop/src-tauri/tauri.conf.json`

`release.yml` зчитує версію з `tauri.conf.json` для назви випуску та резервного значення тегу.

### 2. Оновлення журналів

- Позначте завершені пункти в `task.md` маркером `[200 OK]`.
- Додайте докладний запис у `changes.md` із примітками про валідацію.
- Оновіть тексти про випуск у `README.md`, `docs/` та `web/`.

### 3. Валідація

```powershell
npm.cmd run check
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
cargo check -p tauri-app
npm.cmd run build
```

Також виконайте перевірку синтаксису JavaScript для всіх файлів `.js` і вручну перевірте десктопний застосунок: початкове налаштування, локальну поштову скриньку, створення листа/чернетки, додавання та перевірку облікового запису, синхронізацію, збереження налаштувань, Security Lab і діагностику.

### 4. Тегування та надсилання змін

```powershell
git tag v0.4.0
git push origin main --tags
```

Робочий процес `Release Windows` збирає MSI, інсталятор NSIS і лаунчер, перейменовує їх на стабільні імена активів, обчислює контрольні суми й публікує випуск GitHub. Якщо налаштовано `TAURI_SIGNING_PRIVATE_KEY`, він також підписує обидва інсталятори та публікує `latest.json` для оновлень у застосунку; див. [Оновлення десктопу](desktop-updates.md) щодо налаштування підпису та кроків перевірки.

Очікувані активи:

- `Elyra-latest-x64.msi`
- `Elyra-latest-x64-setup.exe`
- `Elyra-Launcher-x64.exe`
- `Elyra-latest-x64.msi.sha256`
- `Elyra-latest-x64-setup.exe.sha256`
- `Elyra-latest-x64.msi.sig` і `Elyra-latest-x64-setup.exe.sig` (підписані випуски)
- `latest.json` (підписані випуски)
- `Elyra-latest-x64.AppImage`
- `Elyra-latest-x64.deb`
- `Elyra-latest-x64.AppImage.sha256`
- `Elyra-latest-x64.deb.sha256`

Пакування для Linux, залежності під час виконання, збірки з вихідного коду та статус Flatpak описано в [посібнику з встановлення на Linux](linux.md).

### 5. Необов'язково: вбудувати URL виробничого API

Перед створенням тегу встановіть змінну репозиторію `ELYRA_DEFAULT_API_URL` на публічну базову адресу API, наприклад `https://mail.example.com/api`. Робочий процес випуску передає її до збірки десктопу, щоб нові встановлення типово використовували ваш сервер.

## CI-процеси

| Робочий процес | Тригер | Що він робить |
| --- | --- | --- |
| `Rust` | Push/PR до `main` | Тестує crypto, server і launcher на Linux; встановлює залежності npm і збирає десктопний застосунок на Windows |
| `Release Windows` | Тег `v*` або ручний запуск | Збирає інсталятори та лаунчер, публікує випуск GitHub |
| `Release Linux` | Тег `v*` або ручний запуск | Збирає AppImage + .deb із контрольними сумами, завантажує `elyra-linux-x64`; публікація випуску за бажанням через `ELYRA_LINUX_PUBLISH_RELEASE` |
| `Deploy GitHub Pages` | Push до `main` або ручний запуск | Публікує `web/` на GitHub Pages |
| `Deploy server` | Push до `main` зі змінами у server/web/infrastructure або ручний запуск | Розгортання через SSH за явним дозволом, контрольоване `ELYRA_DEPLOY_ENABLED` |

Робочий процес розгортання виконує `git reset --hard origin/main` і `deploy.sh --backup` на сервері. Він вимкнений, якщо змінна репозиторію `ELYRA_DEPLOY_ENABLED` не дорівнює точно `true`.

## Примітки щодо залежностей

- Десктоп використовує `rusqlite` із можливістю `bundled`, тому SQLite компілюється до бінарного файлу. Системний SQLite не потрібен.
- `reqwest` і `lettre` використовують rustls, що дає змогу уникнути залежності від OpenSSL.
- `imap` використовує `native-tls` для TLS платформи.
- `keyring` включається лише для цілей Windows із можливістю `windows-native`.
- `Cargo.lock` може відставати від змін залежностей робочого простору. CI та Docker не використовують `--locked`, тому Cargo розв'язує залежності й оновлює файл блокувань під час збірки. Якщо потрібні відтворювані збірки, згенеруйте файл блокувань заново через `cargo update -w` і закомітьте його.

## Усунення несправностей

| Симптом | Як діяти |
| --- | --- |
| Порожнє вікно десктопу | Перевірте консоль розробника Tauri; перевірте шляхи `frontendDist` і `index.html` |
| Команду не знайдено у webview | Переконайтеся, що її зареєстровано в `lib.rs` і що в `invoke` вона написана точно |
| Помилка SQLite | Перевірте шлях до бази даних у Діагностиці; закрийте інші процеси; перевірте pragmas |
| Сховище ключів недоступне | Перевірте службу Windows Credential Manager і можливість `windows-native` |
| Не вдається увійти через IMAP | Використовуйте пароль застосунку, перевірте хост/порт/безпеку та налаштування доступу провайдера |
| Хмарні виклики не працюють | Перевірте `ELYRA_API_URL`, URL сервера в налаштуваннях, `/health` і термін дії bearer-токена |
| У збірці випуску бракує активів | Перевірте крок `Collect release assets` і шляхи виводу пакетів |

Корисні локальні перевірки:

```powershell
Get-ChildItem apps\desktop\src-tauri\target\release\bundle -Recurse -File
Get-Content apps\desktop\src-tauri\tauri.conf.json
```

## Структура проєкту

```text
Elyra/
в”њв”Ђв”Ђ .github/workflows/        CI, release, Pages, server deploy
в”њв”Ђв”Ђ apps/
в”‚   в”њв”Ђв”Ђ desktop/
в”‚   в”‚   в”њв”Ђв”Ђ src/              static HTML/CSS/JS frontend
в”‚   в”‚   в”њв”Ђв”Ђ src-tauri/        Rust desktop core and Tauri config
в”‚   в”‚   в”њв”Ђв”Ђ services/         sendemail.rs compiled into the desktop crate
в”‚   в”‚   в””в”Ђв”Ђ package.json
в”‚   в””в”Ђв”Ђ launcher/             portable installer/updater CLI
в”њв”Ђв”Ђ packages/
в”‚   в”њв”Ђв”Ђ crypto/               Rust AES-256-GCM primitives
в”‚   в”њв”Ђв”Ђ types/                shared TypeScript contracts
в”‚   в””в”Ђв”Ђ ui/                   framework-neutral UI primitives
в”њв”Ђв”Ђ server/                   Axum API, PostgreSQL schema, auth, relay
в”њв”Ђв”Ђ web/                      static marketing/download site
в”њв”Ђв”Ђ infrastructure/           Docker, deploy script, Caddyfile, Nginx
в”њв”Ђв”Ђ docs/                     this documentation set
в”њв”Ђв”Ђ ai.md                     project rules and intended stack
в”њв”Ђв”Ђ task.md                   feature checklist
в””в”Ђв”Ђ changes.md                implementation log
```

## Пов'язані сторінки

- [Архітектура](architecture.md) — як частини системи поєднуються.
- [Довідник команд Tauri](reference/tauri-commands.md) — перелік команд.
- [Довідник конфігурації](reference/configuration.md) — змінні середовища.
- [Розгортання](deployment.md) — робота з виробничим середовищем.
- [Перевірка виробничого середовища](deploy/VERIFY.md) — контрольний список розгортання та перевірочні запуски.
- [Усунення несправностей](troubleshooting.md) — виправлення помилок збірки та роботи.
