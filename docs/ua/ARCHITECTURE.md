> 🌐 [English](../en/ARCHITECTURE.md) · **Українська**

# Архітектура

Цей каталог містить набір документації Elyra. Канонічна сторінка архітектури —
[`architecture.md`](architecture.md); вона описує систему версії 0.5.0, що
постачається, потоки даних, межі довіри та відомі обмеження.

## Карта документації

| Область | Сторінка |
| --- | --- |
| Індекс документації | [`README.md`](README.md) |
| Встановлення та перший запуск | [`getting-started.md`](getting-started.md) |
| Інтерфейс і можливості | [`user-guide.md`](user-guide.md) |
| Системна архітектура | [`architecture.md`](architecture.md) |
| Сервер і API | [`server.md`](server.md) |
| Модель безпеки | [`security.md`](security.md) |
| Розробка та випуск | [`development.md`](development.md) |
| Огляд розгортання | [`deployment.md`](deployment.md) |
| Повний посібник VPS | [`deploy/SERVER.md`](deploy/SERVER.md) |
| Контрольний список перевірки продакшну | [`deploy/VERIFY.md`](deploy/VERIFY.md) |
| Усунення несправностей | [`troubleshooting.md`](troubleshooting.md) |
| Довідка з конфігурації | [`reference/configuration.md`](reference/configuration.md) |
| Довідка з команд Tauri | [`reference/tauri-commands.md`](reference/tauri-commands.md) |

## Примітки щодо обсягу каталогів

Це короткі позначки для учасників, які працюють усередині каталогу, а не
описова документація:

- [`architecture/ARCHITECTURE.md`](architecture/ARCHITECTURE.md) — обсяг
  каталогу архітектури.
- [`crypto/README.md`](crypto/README.md) — гарантії криптографічної межі.
- [`security/ARCHITECTURE.md`](security/ARCHITECTURE.md) — базовий рівень
  безпеки.

Зміни архітектури потребують явного схвалення відповідно до правил у
[`ai.md`](../../ai.md). Зміни документації — ні, але вони мають описувати
реалізацію такою, якою вона є, і бути зафіксованими у
[`changes.md`](../../changes.md).
