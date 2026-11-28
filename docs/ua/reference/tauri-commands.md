> 🌐 [English](../../en/reference/tauri-commands.md) · **Українська**

# Довідник команд Tauri

Ця сторінка містить перелік усіх команд, зареєстрованих у
`apps/desktop/src-tauri/src/lib.rs` для поточної збірки Elyra для десктопу (0.4.0
та ще не випущеного етапу безпеки). Вебперегляд може викликати лише ці
команди. Усе інше Tauri відхиляє.

## Як викликаються команди

Фронтенд використовує обгортку з `apps/desktop/src/services/tauri.js`:

```js
import { invoke } from "./services/tauri.js";

const page = await invoke("list_mailbox", {
  filter: "inbox",
  quickFilter: "all",
  query: "",
  limit: 100,
  offset: 0,
});
```

Правила, які стосуються кожної команди:

- **Назви аргументів у JavaScript мають camelCase** навіть тоді, коли параметр Rust має
  `snake_case`. Tauri перетворює їх автоматично.
- **Необов'язкові аргументи** опускають або передають як `null`. Параметри Rust `Option<T>`
  мають значення за замовчуванням, наведені нижче.
- **Повернуті значення** — це серіалізовані у JSON структури Rust. Структури з
  `#[serde(rename_all = "camelCase")]` повертають поля у camelCase; решта повертає
  поля у `snake_case`. Для кожної команди зазначено, який саме варіант використовується.
- **Помилки** — це звичайні рядки. `invoke` відхиляє обіцянку з повідомленням, яке
  інтерфейс показує у сповіщенні (toast) або як вбудовану помилку.
- **Блокувальна робота з мережею** позначається в Rust як `#[tauri::command(async)]`, щоб
  вона виконувалася поза потоком інтерфейсу. З JavaScript це й надалі звичайна обіцянка
  (promise).
- **Секрети ніколи не перетинають межу.** Жодна команда не повертає токен доступу до
  хмари або пароль пошти.

Попередній перегляд у браузері в `services/tauri.js` реалізує підмножину цих команд
за допомогою `localStorage`. Збірки для десктопу завжди викликають реалізації Rust.

## Перелік команд

| Модуль | Команди |
| --- | --- |
| [Сховище](#storage) | `initialize_local_storage`, `cache_email`, `list_cached_emails`, `set_local_setting`, `get_local_settings`, `set_local_settings`, `delete_local_setting` |
| [Поштова скринька](#mailbox) | `mailbox_stats`, `seed_demo_mailbox`, `reset_demo_mailbox`, `refresh_mailbox`, `list_mailbox`, `get_message`, `update_message`, `delete_message`, `bulk_update_messages`, `mark_all_read`, `empty_trash`, `save_draft`, `delete_draft`, `search_suggestions`, `export_mailbox`, `refresh_priority_suggestions` |
| [Відправники](#senders) | `list_sender_rules`, `save_sender_rule`, `delete_sender_rule`, `apply_sender_rules` |
| [Вкладення](#attachments) | `scan_attachment`, `scan_all_attachments`, `scan_attachment_external`, `get_attachment_content` |
| [Репутація відправника](#sender-reputation) | `get_sender_reputation`, `refresh_domain_reputation`, `report_reputation_issue` |
| [Провайдери](#providers) | `provider_presets`, `list_email_accounts`, `save_email_account`, `delete_email_account`, `test_email_account`, `test_email_settings`, `sync_email_account`, `sync_all_accounts` |
| [OAuth](#oauth) | `oauth_providers`, `oauth_start`, `oauth_status`, `oauth_revoke` |
| [Надсилання](#sending) | `send_email`, `list_sent_emails` |
| [Безпека](#security) | `get_security_status`, `list_audit_events` |
| [Інтеграції](#integrations) | `list_integrations`, `connect_integration`, `test_integration`, `set_integration_enabled`, `remove_integration_key` |
| [Shield і сканування](#shield-and-scanning) | `scan_links`, `analyze_message`, `get_message_security`, `list_security_alerts`, `get_shield_summary`, `report_false_positive` |
| [Ключі ідентичності](#identity-keys) | `list_crypto_keys`, `generate_crypto_key`, `rotate_crypto_key`, `revoke_crypto_key`, `export_public_key`, `import_public_key`, `list_crypto_contacts`, `delete_crypto_contact` |
| [Криптографія](#crypto) | `generate_encryption_key`, `encrypt_text`, `decrypt_text`, `fingerprint` |
| [Діагностика](#diagnostics) | `get_changelog`, `get_app_diagnostics` |
| [Звітність](#reporting) | `log_diagnostic_event`, `list_diagnostic_events`, `export_diagnostic_report`, `get_crash_reporting_status`, `set_crash_reporting_enabled`, `get_diagnostic_logging_status`, `set_diagnostic_logging_enabled`, `clear_diagnostic_log` |
| [Розширений пошук](#advanced-search) | `advanced_search` |
| [Синхронізація та автономний режим](#sync-and-offline) | `queue_outgoing_message`, `list_outgoing_queue`, `flush_outgoing_queue`, `retry_outgoing_message`, `discard_outgoing_message`, `get_sync_status`, `set_offline_mode` |
| [Оновлення](#updates) | `get_update_preferences`, `set_update_preferences`, `check_for_updates`, `download_and_install_update` |
| [Хмара](#cloud) | `cloud_register`, `cloud_login`, `cloud_session`, `cloud_logout`, `cloud_list_notifications`, `cloud_send_notification`, `cloud_mark_notification_read`, `cloud_sync_cursor`, `cloud_update_cursor`, `cloud_health`, `cloud_api_url`, `cloud_set_api_url`, `cloud_mail_status` |

`secrets.rs` містить лише допоміжні функції; він не реєструє жодних команд.

<a id="storage"></a>
## Сховище

Джерело: `apps/desktop/src-tauri/src/storage.rs`.

### `initialize_local_storage`

```ts
() => Promise<void>
```

Створює каталог даних застосунку, відкриває SQLite, застосовує прагми та створює
базові таблиці. Під час запуску це вже викликається через `storage::initialize`, тож
інтерфейсу зазвичай не потрібно робити це самостійно.

### `cache_email`

```ts
(accountId: number, providerId: string, sender: string, subject: string,
 bodyText: string, folder: string, receivedAt: string) => Promise<void>
```

Оновлює або вставляє один кешований лист за `(account_id, provider_id)`. Папка має
вже існувати. Використовується для шляхів кешування без IMAP; синхронізація IMAP
використовує власний, розширений варіант upsert.

### `list_cached_emails`

```ts
() => Promise<CachedEmail[]>
```

Повертає кешовані рядки у форматі `snake_case`:

```json
[
  {
    "id": 1,
    "provider_id": "imap-42",
    "sender": "Ada <ada@example.com>",
    "subject": "Hello",
    "body_text": "Plain text body",
    "folder": "Inbox",
    "is_read": false,
    "is_starred": false
  }
]
```

### `set_local_setting`

```ts
(key: string, value: string) => Promise<void>
```

Оновлює або вставляє один рядок `settings`. Значення зберігаються як рядки. Для
типізованих значень краще використовувати `set_local_settings`.

### `get_local_settings`

```ts
() => Promise<Record<string, unknown>>
```

Повертає всі рядки `settings`, окрім ключів, що починаються з `cloud_`. Значення
розбираються як JSON, коли це можливо, інакше повертаються як рядки.

### `set_local_settings`

```ts
(values: Record<string, unknown>) => Promise<void>
```

Оновлює або вставляє кожен запис. Ключі, що починаються з `cloud_`, ігноруються.
Об'єкти й масиви зберігаються як текст JSON.

### `delete_local_setting`

```ts
(key: string) => Promise<void>
```

Видаляє один рядок `settings`. Не видаляє записів у сховищі ключів.

<a id="mailbox"></a>
## Поштова скринька

Джерело: `apps/desktop/src-tauri/src/mailbox.rs`.

### `mailbox_stats`

```ts
() => Promise<MailboxStats>
```

Повертає лічильники поштової скриньки у форматі `snake_case`:

```json
{
  "total": 42,
  "unread": 7,
  "starred": 3,
  "important": 2,
  "high_priority": 4,
  "attachments": 5,
  "trash": 2,
  "spam": 1,
  "drafts": 4,
  "snoozed": 0,
  "accounts": 2,
  "storage_bytes": 1048576,
  "last_sync_at": "2026-11-16 10:00:00",
  "folder_counts": { "Inbox": 20, "Sent": 8 },
  "folder_unread": { "Inbox": 5 }
}
```

### `seed_demo_mailbox`

```ts
(force?: boolean) => Promise<MailboxStats>
```

Заповнює демо-поштову скриньку, якщо вона порожня. За `force: true` спочатку
видаляє всі вкладення й листи. Типово `false`.

### `reset_demo_mailbox`

```ts
() => Promise<MailboxStats>
```

Еквівалентно `seed_demo_mailbox(true)`. Знищує всі локальні листи, включно з поштою,
синхронізованою через IMAP, чернетками та надісланими записами.

### `refresh_mailbox`

```ts
() => Promise<MailboxStats>
```

Оновлює локальне налаштування `last_sync_at` і повертає свіжу статистику. Він **не**
звертається до IMAP.

### `list_mailbox`

```ts
(filter?: string, quickFilter?: string, query?: string,
 limit?: number, offset?: number) => Promise<MailboxPage>
```

Типові значення: `filter = "inbox"`, `quickFilter = "all"`, `query = ""`,
`limit = 100` (обмежено 1–500), `offset = 0`.

`filter` приймає `inbox`, `starred`, `snoozed`, `sent`, `drafts`, `archive`,
`spam`, `trash`, а також подання міток `important`, `updates`, `security`,
`personal`. Невідомі значення обробляються як пошук папки або мітки.

`quickFilter` приймає `all`, `unread`, `starred`, `important`, `high`,
`urgent`, `attachments`. `high` відповідає пріоритетам `high` і `urgent`.

Повертає:

```json
{
  "messages": [
    {
      "id": 12,
      "sender": "Ada Lovelace",
      "initials": "AL",
      "subject": "Notes",
      "preview": "First lines of the body",
      "time": "2h",
      "received_at": "2026-11-16 08:00:00",
      "folder": "Inbox",
      "tag": "Important",
      "color": "coral",
      "is_read": false,
      "is_starred": false,
      "is_important": true,
      "priority": "high",
      "priority_source": "auto",
      "priority_reason": "Matches important or time-sensitive signals.",
      "blocked_by": null,
      "has_attachments": true,
      "snoozed_until": null
    }
  ],
  "total": 1,
  "unread": 1,
  "starred": 0,
  "has_more": false,
  "filter": "inbox",
  "quick_filter": "all"
}
```

`MessageSummary` не має перейменування через serde, тож його поля мають формат
`snake_case`.

### `get_message`

```ts
(id: number) => Promise<MessageDetail>
```

Повертає повний лист і позначає його як прочитаний. Формат:

```json
{
  "message": { "...": "MessageSummary" },
  "recipient": "you@example.com",
  "body_text": "Plain text body",
  "body_html": null,
  "attachments": [
    {
      "id": 3,
      "name": "report.pdf",
      "mime_type": "application/pdf",
      "size_bytes": 42000,
      "detected_mime": "application/pdf",
      "kind": "document",
      "risk": "safe",
      "score": 0,
      "reasons": [],
      "sha256": "…",
      "content_available": true,
      "scan_source": "local",
      "scanned_at": "2026-11-17 10:00:00",
      "consent_required": true
    }
  ]
}
```

`body_html` є в базі даних, але читач показує екранований звичайний текст, а не
сирий HTML.

### `update_message`

```ts
(id: number, isRead?: boolean, isStarred?: boolean, isImportant?: boolean,
 priority?: string, folder?: string) => Promise<void>
```

Оновлює лише ті поля, які передано. `folder` — це назва папки, наприклад `Archive`,
`Trash`, `Spam`, `Snoozed` або `Inbox`. `priority` приймає `low`, `normal`, `high`
або `urgent` і записує `priority_source = "manual"`, тож вибір ніколи не
перезаписується автоматичними пропозиціями. Ручне переміщення у `folder`
також очищає `blocked_by`, тому подальші зміни правил не перемістять лист повторно.
Без полів команда нічого не робить.

### `delete_message`

```ts
(id: number, permanent?: boolean) => Promise<void>
```

Типово виконує м'яке видалення (встановлює `deleted_at`). За `permanent: true`
видаляє рядок і його вкладення. Постійне видалення неможливо скасувати.

### `bulk_update_messages`

```ts
(ids: number[], action: string, until?: string) => Promise<number>
```

Повертає кількість змінених рядків. Максимум 500 ідентифікаторів за виклик; порожній
список повертає `0`.

| Дія | Ефект |
| --- | --- |
| `read` | Встановлює `is_read = 1` |
| `unread` | Встановлює `is_read = 0` |
| `star` | Встановлює `is_starred = 1` |
| `unstar` | Встановлює `is_starred = 0` |
| `important` | Встановлює `is_important = 1` |
| `unimportant` | Встановлює `is_important = 0` |
| `priority-low`, `priority-normal`, `priority-high`, `priority-urgent` | Встановлює `priority` і записує ручний вибір |
| `trash` | Переміщує до **Trash** |
| `archive` | Переміщує до **Archive** |
| `spam` | Переміщує до **Spam** |
| `inbox` / `restore` | Переміщує до **Inbox** |
| `snooze` | Переміщує до **Snoozed** і встановлює `snoozed_until` (типово поточний час + 1 день) |
| `delete` | Назавжди видаляє рядки |

Невідомі дії повертають `Unsupported action: <name>`.

### `mark_all_read`

```ts
(filter?: string) => Promise<number>
```

Позначає як прочитані всі листи, що відповідають поточному поданню скриньки, і
повертає їх кількість. Типово `filter = "inbox"`.

### `empty_trash`

```ts
() => Promise<number>
```

Назавжди видаляє всі листи в **Trash** і повертає їх кількість.

### `save_draft`

```ts
(id?: number, recipient: string, subject: string, body: string) => Promise<number>
```

Оновлює наявну чернетку, якщо `id` належить папці **Drafts**, інакше додає нову
чернетку. Повертає ідентифікатор чернетки.

### `delete_draft`

```ts
(id: number) => Promise<boolean>
```

Видаляє рядок лише тоді, коли він перебуває в **Drafts**. Повертає, чи було видалено
рядок.

### `search_suggestions`

```ts
(query: string, limit?: number) => Promise<string[]>
```

Повертає до `limit` пропозицій (типово 6, обмежено 1–20), отриманих на основі
відповідних відправників і тем листів.

### `export_mailbox`

```ts
() => Promise<unknown>
```

Повертає об'єкт JSON із папками, листами та метаданими для всіх невидалених листів.
Об'єкт може бути великим і містить тіла листів. Вважайте експорт конфіденційним.

### `refresh_priority_suggestions`

```ts
() => Promise<number>
```

Перераховує автоматичний пріоритет і причину для кожного невидаленого листа, у якого
`priority_source` дорівнює `auto`, і повертає кількість розглянутих рядків. Ручні
пріоритети та прапорець `is_important` залишаються незмінними. Запускайте цю команду
після зміни налаштування автоматичного пріоритету або локальних правил.

<a id="senders"></a>
## Відправники

Джерело: `apps/desktop/src-tauri/src/senders.rs`.

Правила — це локальні рядки SQLite. `kind` дорівнює `block` або `allow`, `scope` —
`address` або `domain`, а `value` — нормалізована адреса або домен. Правила дозволу
обробляються перед правилами блокування, тож довірена адреса чи домен ніколи не
фільтрується. Заблоковані листи переміщуються до **Spam** або **Trash** залежно від
налаштування `blocked_sender_action`, а відповідне правило записується в `blocked_by`
як `address:<value>` або `domain:<value>`.

Формат `SenderRule`:

```json
{
  "id": 3,
  "kind": "block",
  "scope": "domain",
  "value": "example.com",
  "note": "Repeated phishing",
  "created_at": "2026-11-17 09:00:00"
}
```

### `list_sender_rules`

```ts
() => Promise<SenderRule[]>
```

Повертає всі правила, упорядковані за видом, областю та значенням.

### `save_sender_rule`

```ts
(kind: string, scope: string, value: string, note?: string) => Promise<SenderRule>
```

Створює або оновлює правило. `kind` має бути `block` або `allow`; `scope` має бути
`address` або `domain`. Значення домену може починатися з `@` і переводиться в
нижній регістр. Значення адреси може містити відображуване ім'я, яке відкидається.
Збереження правила повторно застосовує всі правила до локальної пошти.

### `delete_sender_rule`

```ts
(id: number) => Promise<boolean>
```

Видаляє правило й повторно застосовує решту правил. Листи, які були автоматично
відфільтровані видаленим правилом, повертаються до **Inbox**, якщо вони ще перебувають
у **Spam** або **Trash**. Повертає, чи було видалено рядок.

### `apply_sender_rules`

```ts
() => Promise<number>
```

Переоцінює кожен невидалений лист і повертає кількість заблокованих листів. Ручне
переміщення до іншої папки очищає `blocked_by`, тому такі листи не відновлюються при
зміні правил.

<a id="attachments"></a>
## Вкладення

Джерело: `apps/desktop/src-tauri/src/attachments.rs`.

Аналіз вкладень перевіряє збережені байти, а не лише назву файлу чи оголошений
MIME-тип. Для кожного вкладення зберігаються `detected_mime`, `kind`, `risk`,
`score`, `reasons`, `sha256`, `content_available`, `scan_source`, `scanned_at`
і `consent_required`. Ризик дорівнює `safe` нижче 25, `suspicious` від 25 та
`malicious` від 60. Локальний аналіз завжди виконується; ClamAV і VirusTotal
увімковуються за бажанням. Файли ніколи не виконуються й не відкриваються.

### `scan_attachment`

```ts
(id: number) => Promise<AttachmentScan>
```

Повторно виконує локальний аналіз одного вкладення, а також ClamAV, якщо увімкнено
`attachment_clamav` і вміст збережено. Повертає оновлений результат сканування.

```json
{
  "attachment_id": 3,
  "email_id": 12,
  "name": "invoice.pdf.exe",
  "mime_type": "application/pdf",
  "detected_mime": "application/pdf",
  "kind": "executable",
  "size_bytes": 1204224,
  "sha256": null,
  "risk": "malicious",
  "score": 80,
  "reasons": ["The attachment is an executable program (exe) ..."],
  "content_available": false,
  "scan_source": "local",
  "scanned_at": "2026-11-17 10:00:00",
  "consent_required": false
}
```

### `scan_all_attachments`

```ts
() => Promise<number>
```

Повторно сканує всі вкладення й повертає кількість тих, що відсканувалися успішно.
Це лежить в основі дії налаштувань **Rescan attachments**.

### `scan_attachment_external`

```ts
(id: number, consent: boolean) => Promise<AttachmentScan>
```

Завантажує одне вкладення до VirusTotal для аналізу файлу. Потрібні `consent: true`,
`attachment_virustotal = "true"`, збережений ключ VirusTotal, збережений вміст і файл
у межах `attachment_scan_max_mb`. Використовується спільне 16-секундне обмеження
частоти запитів VirusTotal. Чистий результат ніколи не знижує локальне попередження.
Без згоди команда повертає помилку, і нічого не завантажується.

### `get_attachment_content`

```ts
(id: number) => Promise<AttachmentContent>
```

Повертає збережені байти у форматі base64 для контрольованого завантаження. Відхиляє
файли понад 50 МБ і вкладення, чий вміст не збережено локально.

```json
{
  "id": 3,
  "name": "report.pdf",
  "mime_type": "application/pdf",
  "detected_mime": "application/pdf",
  "size_bytes": 42000,
  "data_base64": "..."
}
```

<a id="sender-reputation"></a>
## Репутація відправника

Джерело: `apps/desktop/src-tauri/src/reputation.rs`.

Репутація кешується для кожного домену відправника в `domain_reputation`. Оцінки
поєднують правила дозволу й блокування, офіційні домени брендів, імітацію брендів і
punycode, результати SPF/DKIM/DMARC з `emails.auth_results` та кешовані вердикти
розвідки загроз. Невідомі домени починають із Neutral. Оцінка 25 або більше — це
Suspicious; 60 або більше — Malicious. Відправники зі списку дозволу мають Trusted.

`AuthResults` зберігається як JSON у листі:

```json
{
  "spf": "pass",
  "dkim": "pass",
  "dmarc": "pass",
  "source": "Authentication-Results"
}
```

Формат `SenderReputation`:

```json
{
  "emailId": 12,
  "domain": "example.com",
  "address": "ada@example.com",
  "classification": "neutral",
  "score": 0,
  "reasons": ["SPF, DKIM and DMARC did not report any problems."],
  "auth": { "spf": "pass", "dkim": "pass", "dmarc": "pass", "source": "Authentication-Results" },
  "allowlisted": false,
  "blocklisted": false,
  "disputed": false,
  "feedbackNote": null,
  "updatedAt": "2026-11-17 10:00:00"
}
```

`classification` дорівнює `trusted`, `neutral`, `suspicious` або `malicious`.

### `get_sender_reputation`

```ts
(id: number) => Promise<SenderReputation>
```

Перераховує репутацію одного листа, оновлює кеш домену й повертає результат.
Відхиляє запит, якщо лист не існує.

### `refresh_domain_reputation`

```ts
(domain?: string) => Promise<number>
```

Перераховує кешовані репутації на основі поточної локальної пошти та даних розвідки.
Передайте домен, щоб обмежити оновлення; якщо його не передати, оновлюються всі
домени відправників. Повертає кількість оновлених доменів.

### `report_reputation_issue`

```ts
(id: number, note?: string) => Promise<SenderReputation>
```

Позначає оцінку домену як оскаржену й зберігає необов'язкову примітку в
`reputation_feedback`. Класифікація стає Neutral і залишається такою, доки нові дані
розвідки не дадуть оцінку, вищу за відхилену.

<a id="providers"></a>
## Провайдери

Джерело: `apps/desktop/src-tauri/src/providers.rs`.

### `provider_presets`

```ts
() => Promise<ProviderPreset[]>
```

Повертає вбудовані пресети у форматі `camelCase`:

```json
[
  {
    "id": "gmail",
    "label": "Gmail",
    "imapHost": "imap.gmail.com",
    "imapPort": 993,
    "imapSecurity": "tls",
    "smtpHost": "smtp.gmail.com",
    "smtpPort": 465,
    "smtpSecurity": "tls",
    "hint": "Use an app password"
  }
]
```

Ідентифікатори пресетів: `gmail`, `outlook`, `icloud`, `yahoo`, `fastmail`, `proton`,
`custom`.

### `list_email_accounts`

```ts
() => Promise<EmailAccount[]>
```

Повертає метадані облікових записів у форматі `camelCase`:

```json
[
  {
    "id": 1,
    "provider": "gmail",
    "email": "you@gmail.com",
    "displayName": "Personal",
    "username": "you@gmail.com",
    "imapHost": "imap.gmail.com",
    "imapPort": 993,
    "imapSecurity": "tls",
    "smtpHost": "smtp.gmail.com",
    "smtpPort": 465,
    "smtpSecurity": "tls",
    "hasPassword": true,
    "authMethod": "password",
    "oauthConnected": false,
    "lastSyncAt": "2026-11-16 10:00:00",
    "syncError": null
  }
]
```

`hasPassword` та `oauthConnected` повідомляють про наявність ключів у сховищі
ключів; самі секрети ніколи не повертаються. `authMethod` має значення `password`
або `oauth2`.

### `save_email_account`

```ts
(account: EmailAccountInput) => Promise<EmailAccount>
```

Створює або оновлює обліковий запис. Вхідні дані мають формат `camelCase`:

```json
{
  "id": null,
  "provider": "custom",
  "email": "you@example.com",
  "displayName": "Work",
  "username": "you@example.com",
  "password": "app-password",
  "imapHost": "imap.example.com",
  "imapPort": 993,
  "imapSecurity": "tls",
  "smtpHost": "smtp.example.com",
  "smtpPort": 587,
  "smtpSecurity": "starttls",
  "authMethod": "password"
}
```

Поведінка:

- Адресу електронної пошти обрізають і переводять у нижній регістр; вона має містити
  `@` і бути не коротшою за 3 символи.
- Порожній `provider` стає `custom`.
- Якщо `imapPort`/`smtpPort` не вказано, використовуються 993 і 465.
- Значення безпеки, відмінні від `tls`/`starttls`, нормалізуються до `starttls`.
- Непорожній пароль записується до сховища ключів під ключем `imap-account-<id>`.
- Порожній пароль під час оновлення зберігає наявне значення у сховищі ключів.
- `authMethod: "oauth2"` зберігає наявний сеанс OAuth, якщо пароль не вказано;
  будь-яке інше значення перемикає обліковий запис на автентифікацію за паролем.
- Повертає збережені метадані облікового запису.

### `delete_email_account`

```ts
(id: number) => Promise<void>
```

Видаляє рядок облікового запису, його кешовані листи (каскадно) та запис у сховищі
ключів.

### `test_email_account`

```ts
(id: number) => Promise<AccountTest>
```

Перевіряє збережений обліковий запис, використовуючи пароль зі сховища ключів
або сеанс OAuth через `AUTHENTICATE XOAUTH2`, якщо `authMethod` дорівнює `oauth2`.
Для облікових записів лише з OAuth SMTP позначається як помилка, доки не додано
пароль застосунку. Повертає:

```json
{ "imapOk": true, "smtpOk": true, "detail": "IMAP inbox reachable (128 messages); SMTP ready" }
```

### `test_email_settings`

```ts
(account: EmailAccountInput) => Promise<AccountTest>
```

Перевіряє незбережені налаштування. Якщо `password` порожній і вказано `id`,
використовується збережений пароль зі сховища ключів. Інакше пароль обов'язковий.
`authMethod: "oauth2"` перевіряє IMAP через збережений сеанс OAuth для наявного
облікового запису. Нічого не зберігається.

### `sync_email_account`

```ts
(accountId: number, limit?: number) => Promise<SyncAccountResult>
```

Отримує найновіші `limit` листів із **INBOX** (типово 40, обмежено 1–200) і
виконує upsert. Повертає:

```json
{ "accountId": 1, "email": "you@gmail.com", "imported": 12, "error": null }
```

Помилка синхронізації записується в `syncError` і повертається в `error`; сама
команда завершується успішно, щоб інтерфейс міг показати стан кожного облікового
запису.

### `sync_all_accounts`

```ts
(limit?: number) => Promise<SyncSummary>
```

Виконує таку саму синхронізацію для кожного облікового запису. Повертає:

```json
{
  "accounts": [
    { "accountId": 1, "email": "you@gmail.com", "imported": 12, "error": null }
  ],
  "imported": 12
}
```

Синхронізується лише INBOX. У 0.4.0 немає планової фонової синхронізації.

<a id="oauth"></a>
## OAuth

Джерело: `apps/desktop/src-tauri/src/oauth.rs`. Потік, змінні ідентифікатора
клієнта та обмеження описано на сторінці [OAuth провайдерів](../oauth.md).

### `oauth_providers`

```ts
() => Promise<OAuthProviderView[]>
```

Повертає постачальників із полями `id`, `label`, `configured`, `scopes` та
`docsUrl`. `configured` дорівнює `true`, лише якщо ідентифікатор клієнта доступний
під час збирання або виконання та працює ключниця ОС. У браузерному прев'ю та
збірках не для Windows значення дорівнює `false`.

### `oauth_start`

```ts
(provider: string, email: string) => Promise<OAuthSessionView>
```

Запускає нативний потік Authorization Code + PKCE: створює тимчасовий
loopback-слухач, відкриває браузер, перевіряє `state`, обмінює код і зберігає
токени в ключниці ОС. `provider` приймає `google`/`microsoft`, а також пресети
облікових записів `gmail`/`outlook`. Повертає лише метадані:

```json
{
  "accountId": 1,
  "provider": "gmail",
  "email": "you@gmail.com",
  "connected": true,
  "expiresAt": "2026-11-28T12:00:00+00:00",
  "hasRefreshToken": true,
  "needsAttention": false
}
```

### `oauth_status`

```ts
(accountId: number) => Promise<OAuthSessionView>
```

Повідомляє, чи збережено сеанс OAuth для облікового запису. Токен доступу чи
оновлення ніколи не повертаються.

### `oauth_revoke`

```ts
(accountId: number) => Promise<OAuthSessionView>
```

Вилучає локальний запис із ключниці й скидає `authMethod` до `password`.
Відкликання на сервері Google відбувається на найкращих зусиллях; Microsoft не
надає публічної кінцевої точки відкликання для цього потоку.

<a id="sending"></a>
## Надсилання

Джерело: `apps/desktop/services/sendemail.rs`.

### `send_email`

```ts
(to: string, subject: string, body: string, accountId?: number) => Promise<void>
```

Надсилає лист із звичайним текстом і за успіху записує його до **Sent**. Порядок:

1. Якщо вказано `accountId`, використовувати конфігурацію SMTP цього облікового
   запису та пароль зі сховища ключів.
2. Інакше спробувати повну конфігурацію змінних середовища `ELYRA_SMTP_*`.
3. Інакше, якщо виконано вхід до Elyra Cloud, використовувати ретранслятор сервера.

Помилка відхиляє запит із комбінованим повідомленням, що містить проблему зі
змінними середовища та проблему з ретранслятором. Невдалі надсилання не
записуються.

### `list_sent_emails`

```ts
() => Promise<SendEmail[]>
```

Повертає записи надісланих листів у форматі `snake_case`:

```json
[{ "to": "friend@example.com", "subject": "Hello", "body": "Plain text body" }]
```

<a id="security"></a>
## Безпека

Джерело: `apps/desktop/src-tauri/src/security.rs`.

### `get_security_status`

```ts
() => Promise<SecurityStatus>
```

Виконує перевірку стану SQLite й записує подію аудиту. Повертає поля у форматі
`snake_case`:

```json
{
  "status": "protected",
  "checks_passed": 4,
  "checks_total": 4,
  "foreign_keys": true,
  "journal_mode": "wal",
  "secure_delete": true,
  "audit_events": 12
}
```

`status` дорівнює `protected` лише тоді, коли перевірки пройдено. Інтерфейс має
показувати реальний результат і не підміняти його удаваним успіхом.

### `list_audit_events`

```ts
(limit?: number) => Promise<AuditEvent[]>
```

Повертає рядки аудиту, найновіші спочатку. Типово 20, обмежено 1–100.

```json
[
  {
    "id": 12,
    "created_at": "2026-11-16 10:00:00",
    "event_type": "security.check",
    "outcome": "success",
    "details": "SQLite security policy check completed"
  }
]
```

`outcome` — один із `success`, `failure`, `warning`. Таблиця обмежена 500 рядками.

<a id="integrations"></a>
## Інтеграції

Джерело: `apps/desktop/src-tauri/src/integrations.rs`. Ключі API записуються до
сховища ключів ОС як `integration-<provider>` і ніколи не повертаються до
вебперегляду. SQLite зберігає лише метадані стану.

### `list_integrations`

```ts
() => Promise<IntegrationView[]>
```

Повертає по одному запису для кожного вбудованого провайдера (`virustotal`,
`google_safe_browsing`) у форматі `camelCase`:

```json
[
  {
    "provider": "virustotal",
    "label": "VirusTotal",
    "description": "Checks links against 90+ security vendors and their URL reputation data.",
    "docsUrl": "https://www.virustotal.com/gui/my-apikey",
    "keyHint": "Paste the API key from your VirusTotal profile.",
    "enabled": true,
    "configured": true,
    "status": "connected",
    "lastCheckedAt": "2026-11-17 10:00:00",
    "lastError": null
  }
]
```

`status` дорівнює `not_configured`, `unverified`, `connected`, `disabled` або
`error`. `configured` повідомляє про наявність ключа у сховищі ключів.

### `connect_integration`

```ts
(provider: string, key: string) => Promise<IntegrationView>
```

Перевіряє ключ у провайдера, а потім зберігає його у сховищі ключів. Ключ, коротший за
8 символів, відхиляється до будь-якого мережевого запиту. Коли провайдер відхиляє
ключ, команда все одно завершується з `status: "error"` і встановленим `lastError`;
помилки сховища ключів або бази даних призводять до відхилення. Успішне
підключення встановлює `enabled: true`.

### `test_integration`

```ts
(provider: string) => Promise<IntegrationView>
```

Повторно перевіряє збережений ключ. Відхиляє, якщо ключ не збережено. За успіху
позначає інтеграцію як `connected`; за помилки провайдера зберігає `error` і
`lastError` та повертає представлення.

### `set_integration_enabled`

```ts
(provider: string, enabled: boolean) => Promise<IntegrationView>
```

Вимкнення зберігає ключ у сховищі ключів і встановлює `status: "disabled"`.
Увімкнення повторно перевіряє збережений ключ і відхиляє запит, якщо провайдер його
відхилив або ключа немає.

### `remove_integration_key`

```ts
(provider: string) => Promise<IntegrationView>
```

Видаляє запис зі сховища ключів і скидає рядок SQLite до `not_configured`,
вимкненого стану без останньої помилки.

<a id="shield-and-scanning"></a>
## Shield і сканування

Джерела: `apps/desktop/src-tauri/src/shield.rs` та
`apps/desktop/src-tauri/src/threats.rs`. Локальні евристики завжди виконуються в Rust.
Виклики зовнішніх провайдерів є необов'язковими, вимикаються через
`external_scanning=false` і відкладаються до відкриття листа.

### `scan_links`

```ts
(text: string) => Promise<ScanReport>
```

Видобуває до 40 унікальних посилань із тексту й перевіряє їх увімкненими
провайдерами. Повертає поля у форматі `camelCase`:

```json
{
  "urls": [
    {
      "url": "https://example.com/login",
      "host": "example.com",
      "verdict": "safe",
      "score": 0,
      "providers": [
        {
          "provider": "virustotal",
          "label": "VirusTotal",
          "verdict": "safe",
          "detail": "0 malicious, 0 suspicious, 71 clean vendors"
        }
      ],
      "cached": true,
      "checkedAt": "2026-11-17 10:00:00"
    }
  ],
  "scanned": true,
  "disabled": false,
  "providers": ["VirusTotal"],
  "note": "Link checks completed with your connected providers."
}
```

Вердикти URL: `safe`, `suspicious`, `malicious`, `unknown`, `unscanned` або
`skipped`. `scanned` дорівнює false, коли не вдалося виконати жодного запиту до
провайдера. `disabled` відображає значення `external_scanning`.

### `analyze_message`

```ts
(id: number, rescan?: boolean) => Promise<MessageSecurity>
```

Виконує локальний аналіз Shield і, коли провайдери увімкнено, зовнішню перевірку
посилань. За `rescan: false` (типово) наявний зовнішній результат повертається без
повторного мережевого запиту. Повертає дані у форматі `camelCase`:

```json
{
  "emailId": 12,
  "risk": "suspicious",
  "score": 46,
  "reasons": ["The sender claims to be Microsoft, but the address is not on an official Microsoft domain."],
  "urls": [{ "...": "UrlScan" }],
  "externalScanned": true,
  "falsePositive": false,
  "falsePositiveNote": null,
  "analyzedAt": "2026-11-17 10:00:00"
}
```

`risk` дорівнює `safe`, `suspicious` або `malicious`. Оцінка становить 0–100.

### `get_message_security`

```ts
(id: number) => Promise<MessageSecurity>
```

Повертає збережений аналіз або виконує локальний аналіз, якщо його немає. Не
звертається до зовнішніх провайдерів.

### `list_security_alerts`

```ts
(limit?: number) => Promise<SecurityAlert[]>
```

Повертає підозрілі та шкідливі листи, спочатку з найвищою оцінкою. Типово 25,
обмежено 1–200. Кожен рядок має `emailId`, `risk`, `score`, `reasons`,
`falsePositive`, `analyzedAt`, `sender` і `subject`.

### `get_shield_summary`

```ts
() => Promise<ShieldSummary>
```

Повертає `analyzed`, `safe`, `suspicious`, `malicious`, `falsePositives`,
`providersConnected` і `externalScanning`.

### `report_false_positive`

```ts
(id: number, note?: string) => Promise<MessageSecurity>
```

Позначає наявний аналіз як хибнопозитивний і зберігає необов'язкову примітку.
Відхиляє запит, якщо лист не було проаналізовано.

<a id="identity-keys"></a>
## Ключі ідентичності

Джерело: `apps/desktop/src-tauri/src/keys.rs`. Приватні ключі зберігаються лише у
сховищі ключів ОС як `crypto-key-<id>`. Публічні ключі, відбитки та стан зберігаються
в SQLite.

### `list_crypto_keys`

```ts
() => Promise<CryptoKeyView[]>
```

Повертає ключі, упорядковані за станом: active, retired, потім revoked. Кожне
представлення має формат `camelCase`:

```json
[
  {
    "id": 1,
    "label": "Primary identity",
    "algorithm": "Ed25519",
    "publicKey": "base64…",
    "fingerprint": "ab12:cd34:…",
    "status": "active",
    "createdAt": "2026-11-17 10:00:00",
    "rotatedAt": null,
    "revokedAt": null,
    "hasPrivateKey": true
  }
]
```

`status` дорівнює `active`, `retired` або `revoked`. `hasPrivateKey` повідомляє про
наявність ключа у сховищі ключів, а не про сам ключ.

### `generate_crypto_key`

```ts
(label?: string) => Promise<CryptoKeyView>
```

Генерує пару ключів Ed25519, зберігає приватний ключ у сховищі ключів і повертає
публічне представлення. Відхиляє запит, якщо активний ключ уже існує. Типова
назва: `Primary identity`.

### `rotate_crypto_key`

```ts
(id: number, label?: string) => Promise<CryptoKeyView>
```

Створює новий активний ключ, позначає старий ключ як `retired` і видаляє його запис
зі сховища ключів. Відхиляє запит для скасованого ключа.

### `revoke_crypto_key`

```ts
(id: number) => Promise<CryptoKeyView>
```

Позначає ключ як `revoked` і видаляє його приватний ключ зі сховища ключів.
Скасування неможливо відмінити.

### `export_public_key`

```ts
(id: number) => Promise<string>
```

Повертає публічний ключ у форматі armored:

```text
-----BEGIN ELYRA PUBLIC KEY-----
<base64>
-----END ELYRA PUBLIC KEY-----
```

### `import_public_key`

```ts
(email: string, label: string | null, publicKey: string) => Promise<CryptoContactView>
```

Перевіряє корисне навантаження base64 (32 байти) і точку Ed25519, потім додає або
оновлює ключ контакту. Повертає `id`, `email`, `label`, `publicKey`,
`fingerprint` і `importedAt`.

### `list_crypto_contacts`

```ts
() => Promise<CryptoContactView[]>
```

Повертає публічні ключі контактів, упорядковані за назвою та адресою електронної
пошти.

### `delete_crypto_contact`

```ts
(id: number) => Promise<void>
```

Видаляє один публічний ключ контакту. Відхиляє запит, якщо ідентифікатор невідомий.

<a id="crypto"></a>
## Криптографія

Джерело: `apps/desktop/src-tauri/src/crypto.rs`.

### `generate_encryption_key`

```ts
() => Promise<string>
```

Повертає 32-байтовий ключ у кодуванні base64, отриманий із випадкових даних ОС.
Security Lab зберігає його лише в пам'яті.

### `encrypt_text`

```ts
(key: string, plaintext: string) => Promise<string>
```

Шифрує за допомогою AES-256-GCM із новим 96-бітовим nonce. Повертає base64 від
`nonce || ciphertext || tag`.

### `decrypt_text`

```ts
(key: string, payload: string) => Promise<string>
```

Перевіряє корисне навантаження та тег автентифікації. Відхиляє некоректний base64,
короткі дані, неправильні ключі, змінений шифротекст і відкритий текст, що не є
UTF-8.

### `fingerprint`

```ts
(value: string) => Promise<string>
```

Повертає відбиток SHA-256 із розділовими двокрапками лише для показу та порівняння.
Це не механізм автентифікації.

<a id="diagnostics"></a>
## Діагностика

Джерело: `apps/desktop/src-tauri/src/diagnostics.rs`.

### `get_changelog`

```ts
() => Promise<ChangeEntry[]>
```

Повертає статичні нотатки про випуски, які відображаються на вкладці **Changes**:

```json
[
  {
    "version": "0.4.0",
    "date": "2026-11-26",
    "title": "Local diagnostics",
    "detail": "Redacted error events are stored in SQLite and exported only when you choose...",
    "status": "ready"
  }
]
```

`status` дорівнює `shipped`, `ready` або `next`. Авторитетним журналом є
[`changes.md`](../../../changes.md).

### `get_app_diagnostics`

```ts
() => Promise<AppDiagnostics>
```

Повертає діагностичні дані у форматі `snake_case`:

```json
{
  "app_version": "0.4.0",
  "engine": "Rust + Tauri + SQLite",
  "os": "windows",
  "arch": "x86_64",
  "sqlite_version": "3.46.0",
  "database_path": "C:\\Users\\you\\AppData\\Local\\com.elyra.desktop\\elyra.sqlite3",
  "database_bytes": 1048576,
  "mailbox": { "...": "MailboxStats" },
  "cloud_configured": true,
  "keychain_available": true
}
```

`cloud_configured` дорівнює true, коли визначений базовий URL API не є локальним
типовим. `keychain_available` повідомляє, чи доступне сховище облікових даних ОС.

<a id="reporting"></a>
## Звітність

Джерело: `apps/desktop/src-tauri/src/reporting.rs`. Події проходять маскування перед
записом до SQLite, обмежені 500 рядками й очищаються через 30 днів. Кінцевої точки
телеметрії немає: `crash_reporting_enabled` зберігає лише налаштування, а
`automatic_upload` завжди дорівнює `false`.

### `log_diagnostic_event`

```ts
(input: {
  message: string;
  level?: "info" | "warning" | "error";
  category?: string;
  code?: string;
}) => Promise<void>
```

Фронтенд викликає це з глобальних обробників `error`/`unhandledrejection`. Адреси
електронної пошти, секрети, рядки запитів URL і довгі непрозорі токени маскуються.

### `list_diagnostic_events`

```ts
(options?: { limit?: number }) => Promise<DiagnosticEvent[]>
```

`limit` типово дорівнює 50 і обмежений 500. Події повертаються найновішими
спочатку з полями `id`, `created_at`, `level`, `category`, `code` і `message`.

### `export_diagnostic_report`

```ts
() => Promise<DiagnosticReport>
```

Повертає звіт у форматі `snake_case` із полями `format`, `app_version`,
`generated_at`, `crash_reporting_enabled`, `diagnostic_logging_enabled`, `diagnostics`
і `events`. Фронтенд завантажує його як JSON; Elyra ніколи його нікуди не надсилає.

### `get_crash_reporting_status` / `set_crash_reporting_enabled`

```ts
() => Promise<CrashReportingStatus>
(options: { enabled: boolean }) => Promise<CrashReportingStatus>
```

Стан містить `enabled`, `automatic_upload: false` і `note`.

### `get_diagnostic_logging_status` / `set_diagnostic_logging_enabled`

```ts
() => Promise<DiagnosticLoggingStatus>
(options: { enabled: boolean }) => Promise<DiagnosticLoggingStatus>
```

Коли ведення журналу вимкнено, нові події ігноруються; наявні зберігаються, доки їх
не очищать або не вилучать автоматично.

### `clear_diagnostic_log`

```ts
() => Promise<number>
```

Видаляє всі локальні події й повертає кількість видалених.

<a id="advanced-search"></a>
## Розширений пошук

Джерело: `apps/desktop/src-tauri/src/search.rs`.

### `advanced_search`

```ts
(options: {
  query?: string;
  from?: string;
  to?: string;
  subject?: string;
  dateFrom?: string;
  dateTo?: string;
  hasAttachments?: boolean;
  important?: boolean;
  priorities?: string[];
  security?: string[];
  reputation?: string[];
  folder?: string;
  limit?: number;
  offset?: number;
}) => Promise<MailboxPage>
```

Усі фільтри необов'язкові й застосовуються через параметризовані SQL-запити до
локальної бази даних. `dateFrom`/`dateTo` мають використовувати формат `YYYY-MM-DD`.
Дозволені значення: `low|normal|high|urgent` для пріоритетів,
`safe|suspicious|malicious|unscanned` для безпеки та
`trusted|neutral|suspicious|malicious` для репутації; невідомі значення
ігноруються. `limit` типово дорівнює 100 (максимум 500), а `offset` — 0.

<a id="sync-and-offline"></a>
## Синхронізація та автономний режим

Джерело: `apps/desktop/src-tauri/src/sync.rs`. Черга — це постійний стан SQLite;
нічого не надсилається автоматично.

### `queue_outgoing_message`

```ts
(options: {
  to: string;
  subject: string;
  body: string;
  accountId?: number | null;
}) => Promise<number>
```

Перевіряє отримувача й зберігає лист зі `status: "queued"`.

### `list_outgoing_queue`

```ts
() => Promise<OutgoingMessage[]>
```

Повертає до 100 записів, найновіші спочатку, включно з `attempts`, `status` і
замаскованим `last_error`.

### `flush_outgoing_queue`

```ts
() => Promise<QueueFlushResult>
```

Виконується у фоновому потоці. Повертає `sent`, `failed`, `blocked`, `skipped` і
`errors`. Успішні записи видаляються. Помилки збільшують `attempts`; після п'яти
помилок статус стає `blocked`. В автономному режимі все пропускається.

### `retry_outgoing_message` / `discard_outgoing_message`

```ts
(options: { id: number }) => Promise<void>
```

Повторна спроба скидає `status`, `attempts` і `last_error`; скасування видаляє рядок.

### `get_sync_status` / `set_offline_mode`

```ts
() => Promise<SyncStatus>
(options: { enabled: boolean }) => Promise<SyncStatus>
```

`SyncStatus` містить `offline_mode`, `last_sync_at`, `last_error`, `queued`,
`failed`, `blocked`, `accounts` і `updated_at`.

<a id="updates"></a>
## Оновлення

Джерело: `apps/desktop/src-tauri/src/updates.rs`. Ці команди використовують підписаний
канал оновлень Tauri; див. [Оновлення для десктопу](../desktop-updates.md) щодо
ключів, етапів випуску та поведінки у разі помилки. Фронтенд отримує повідомлення
через події `elyra-update-available` і `elyra-update-progress`.

### `get_update_preferences`

```ts
() => Promise<{
  autoCheck: boolean;
  notifyAvailable: boolean;
  lastCheckedAt: string | null;
  lastResult: UpdateStatus | null;
}>
```

Зчитує збережені налаштування сповіщень і результат останньої перевірки з локальної
таблиці `settings`. Обидва перемикачі типово увімкнено (`true`).

### `set_update_preferences`

```ts
(options: { autoCheck?: boolean; notifyAvailable?: boolean }) => Promise<UpdatePreferences>
```

Зберігає лише передані перемикачі й повертає оновлені налаштування. Ручні перевірки
залишаються доступними, коли автоматичні вимкнено.

### `check_for_updates`

```ts
() => Promise<UpdateStatus>
```

Отримує й перевіряє `latest.json`, повертаючи `status` (`available`, `upToDate`,
`error` або `busy`), поточну й найновішу версії, дату випуску, нотатки, цільову
платформу, час перевірки та зрозуміле для людини повідомлення. Паралельні перевірки
повертають `busy`, не починаючи друге завантаження.

### `download_and_install_update`

```ts
() => Promise<{ installed: true; version: string; restartRecommended: boolean }>
```

Знову виконує перевірку, завантажує підписаний інсталятор, надсилаючи події
`elyra-update-progress` із полями `stage`, `downloaded`, `total` і `percent`,
перевіряє підпис і встановлює його. Помилки повертаються без зміни наявної
інсталяції. На Windows застосунок завершується, коли запускається підтверджений
інсталятор, тож інтерфейс сприймає розрив команди як очікуване перезавантаження.

<a id="cloud"></a>
## Хмара

Джерело: `apps/desktop/src-tauri/src/cloud.rs`. Усі мережеві команди мають анотацію
`#[tauri::command(async)]`.

### `cloud_register`

```ts
(email: string, password: string, displayName: string) => Promise<CloudSessionView>
```

Викликає `POST {api}/v1/auth/register`, зберігає сеанс у сховищі ключів під
`cloud_session` і повертає подання сеансу:

```json
{
  "expiresAt": "2026-11-17T12:00:00Z",
  "user": {
    "id": "0f3f6c1e-...",
    "email": "you@example.com",
    "displayName": "You",
    "createdAt": "2026-11-16T12:00:00Z"
  },
  "signedIn": true
}
```

Токен доступу **не** повертається.

### `cloud_login`

```ts
(email: string, password: string) => Promise<CloudSessionView>
```

Викликає `POST {api}/v1/auth/login`, зберігає сеанс і повертає таке саме подання.

### `cloud_session`

```ts
() => Promise<CloudSessionView | null>
```

Завантажує збережений сеанс. Повертає `null`, якщо виконано вихід. Під час першого
читання переносить застарілий рядок `settings.cloud_session` до сховища ключів.

### `cloud_logout`

```ts
() => Promise<void>
```

Видаляє запис `cloud_session` зі сховища ключів і будь-який застарілий рядок сеансу
SQLite. Не видаляє обліковий запис на сервері, локальну пошту чи паролі пошти.

### `cloud_list_notifications`

```ts
() => Promise<unknown>
```

Викликає `GET {api}/v1/notifications` із токеном bearer і повертає масив JSON із
сервера (до 100 елементів, найновіші спочатку). Помилка, якщо виконано вихід.

### `cloud_send_notification`

```ts
(recipientEmail: string, title: string, body: string) => Promise<unknown>
```

Викликає `POST {api}/v1/notifications`. Отримувач уже має мати обліковий запис
Elyra. Повертає JSON створеного сповіщення.

### `cloud_mark_notification_read`

```ts
(id: string) => Promise<void>
```

Викликає `PATCH {api}/v1/notifications/{id}/read`. Лише отримувач може позначити
сповіщення як прочитане; інші ідентифікатори повертають `404`.

### `cloud_sync_cursor`

```ts
() => Promise<unknown>
```

Викликає `GET {api}/v1/sync/cursor` і повертає `{ cursor, synced_at }`. Курсор є
непрозорим; у 0.4.0 немає планового циклу синхронізації.

### `cloud_update_cursor`

```ts
(cursor?: string | null) => Promise<void>
```

Викликає `PUT {api}/v1/sync/cursor` із `{ cursor }`. `null` очищає курсор.

### `cloud_health`

```ts
() => Promise<unknown>
```

Викликає `GET {api}/health` без автентифікації. Повертає `{ status, service }`.

### `cloud_api_url`

```ts
() => Promise<string>
```

Повертає визначений базовий URL API (без кінцевої похилої риски) згідно з
документованим порядком: `ELYRA_API_URL` → `settings.api_url` →
`ELYRA_DEFAULT_API_URL` → `http://127.0.0.1:8787`.

### `cloud_set_api_url`

```ts
(url: string) => Promise<string>
```

Перевіряє й зберігає URL сервера в `settings.api_url` або очищає його, якщо введення
порожнє. URL має починатися з `http://` або `https://`. Повертає чинний базовий URL
після збереження.

### `cloud_mail_status`

```ts
() => Promise<unknown>
```

Викликає `GET {api}/v1/mail/status` і повертає
`{ configured: boolean, from: string | null }`. Потрібен активний сеанс.

## Пов'язані сторінки

- [Архітектура](../architecture.md) — обов'язки модулів.
- [Сервер і API](../server.md) — маршрути, що лежать в основі команд хмари.
- [Посібник користувача](../user-guide.md) — як інтерфейс використовує ці команди.
- [Розробка](../development.md) — додавання та реєстрація команди.