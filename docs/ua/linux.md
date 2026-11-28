> 🌐 [English](../en/linux.md) · **Українська**

# Linux

Elyra 0.5.0 містить 64-бітну збірку для Linux-робочих столів, створену з того самого вихідного коду Tauri 2, що й застосунок для Windows. Робочий процес `Release Linux` (`.github/workflows/linux-release.yml`) збирає **AppImage** та **Debian-пакет** для `x86_64` на кожному тегу `v*` і при ручному запуску, зі стабільними назвами артефактів і контрольними сумами SHA-256. Рецепт у стилі AUR — [`PKGBUILD`](../../packaging/linux/PKGBUILD) — входить до репозиторію для Arch Linux. Flatpak **не постачається**; дослідження та залишок роботи [описані нижче](#flatpak-investigation).

> **Примітка щодо конфігурації.** Робочий процес передає `--bundles appimage deb` до
> CLI Tauri, тому він збирає пакети для Linux навіть якщо спільний
> `apps/desktop/src-tauri/tauri.conf.json` і надалі містить лише цільові платформи Windows. Для
> звичайного локального `npm run build`, щоб він створював пакети для Linux, `bundle.targets` має
> включати `appimage` і `deb` (див. [Збірка з вихідного коду](#build-from-source)).

## Встановлення

Усі артефакти додаються до [останнього випуску GitHub](https://github.com/imtiredwhenitalk/Elyra/releases/latest), коли для репозиторію увімкнено публікацію для Linux; інакше вони доступні як артефакт `elyra-linux-x64` у запуску робочого процесу. Див. [Конвеєр випуску](#release-pipeline).

| Дистрибутив | Рекомендований метод | Команда |
| :--- | :--- | :--- |
| Debian, Ubuntu, Linux Mint, Pop!_OS | `.deb` | `sudo apt install ./Elyra-latest-x64.deb` |
| Будь-який дистрибутив x86_64 на базі glibc | AppImage | `chmod +x Elyra-latest-x64.AppImage && ./Elyra-latest-x64.AppImage` |
| Arch, Manjaro, EndeavourOS | PKGBUILD | `makepkg -si` з `packaging/linux` |

### Перевірка завантаження

Кожен опублікований артефакт має файл `<name>.sha256`, створений завданням випуску:

```sh
sha256sum -c Elyra-latest-x64.AppImage.sha256
sha256sum -c Elyra-latest-x64.deb.sha256
```

Файл контрольної суми містить `<64 hex digits>  <asset name>`, тому `sha256sum -c` має виконуватися в тому самому каталозі, що й артефакт. Невідповідність означає, що завантаження неповне або його було замінено; не запускайте його.

### AppImage

```sh
# Download the stable asset and its checksum from the latest release.
curl -fL -O https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/Elyra-latest-x64.AppImage
curl -fL -O https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/Elyra-latest-x64.AppImage.sha256
sha256sum -c Elyra-latest-x64.AppImage.sha256
chmod +x Elyra-latest-x64.AppImage
./Elyra-latest-x64.AppImage
```

AppImage є портативним: він не потребує встановлення й записує дані до звичайних розташувань XDG. Файли, розміщені в каталозі, доступному для запису, запускаються з відносними шляхами, що розраховуються відносно самого AppImage. Якщо FUSE 2 недоступний, запустіть його без змонтованого loop-пристрою:

```sh
./Elyra-latest-x64.AppImage --appimage-extract-and-run
```

Для постійної інтеграції з робочим столом використовуйте пакет `.deb` або PKGBUILD: обидва встановлюють запис `.desktop` і піктограму hicolor. Додаткові інструменти, як-от AppImageLauncher, також можуть реєструвати завантажений AppImage.

### Debian і Ubuntu

```sh
sudo apt install ./Elyra-latest-x64.deb
```

`apt` вирішує та встановлює залежності середовища виконання WebKitGTK 4.1 і GTK 3, оголошені в пакеті. Двійковий файл встановлюється в `/usr/bin`, а засіб запуску з'являється в меню застосунків як **Elyra**.

### Arch Linux (PKGBUILD)

Репозиторій містить рецепт у стилі AUR за адресою
[`packaging/linux/PKGBUILD`](../../packaging/linux/PKGBUILD):

```sh
git clone https://github.com/imtiredwhenitalk/Elyra.git
cd Elyra/packaging/linux
makepkg -si
```

Він завантажує стабільний AppImage та його файл `.sha256`, перевіряє хеш у `prepare()`, встановлює AppImage до `/opt/elyra/elyra.AppImage`, створює символічне посилання `/usr/bin/elyra` і встановлює запис для робочого стола та піктограму 256x256.

Артефакт `latest` змінюється з кожним випуском, тому `sha256sums[0]` має значення `SKIP`, а опублікована контрольна сума отримується під час збірки. Це перевіряється, але не є прив'язаним джерелом довіри. Для незмінної збірки або подання до AUR натомість вкажіть артефакт із версією:

```sh
ELYRA_APPIMAGE_URL=https://github.com/imtiredwhenitalk/Elyra/releases/download/v0.5.0/Elyra-0.5.0-x64.AppImage \
  makepkg -si
```

Потім перенесіть справжню контрольну суму до `sha256sums[0]`. `ELYRA_ALLOW_UNVERIFIED=1` пропускає перевірку лише для локальних експериментів; інакше відсутня або некоректна контрольна сума призводить до помилки збірки. Докладніше див. [`packaging/linux/README.md`](../../packaging/linux/README.md).

## Залежності

### Залежності для збірки

На Debian або Ubuntu (передумови Tauri 2, включно з WebKitGTK **4.1**):

```sh
sudo apt update
sudo apt install -y \
  build-essential pkg-config curl wget file \
  libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libxdo-dev libssl-dev dpkg-dev libfuse2
```

На Arch: `sudo pacman -S --needed base-devel webkit2gtk-4.1 libsoup3 gtk3 libappindicator-gtk3 librsvg curl wget file`.

### Залежності середовища виконання

- `.deb` оголошує свої залежності WebKitGTK 4.1, GTK 3 та appindicator; `apt` встановлює їх за вас.
- AppImage містить більшість бібліотек, але йому все одно потрібні системний стек WebKitGTK 4.1 / GTK 3 і glibc сумісної базової системи. На Ubuntu 24.04 встановіть `libfuse2`, щоб монтувати AppImage без `--appimage-extract-and-run`.
- PKGBUILD оголошує відповідні залежності середовища виконання для Arch. `fuse2` є необов'язковим і потрібен лише якщо ви не використовуєте резервний варіант extract-and-run.
- SQLite скомпільовано до двійкового файлу (`bundled`-функція `rusqlite`); системний пакет SQLite не потрібен.

<a id="glibc-baseline"></a>
### Мінімальна версія glibc

Робочий процес використовує `ubuntu-latest`, наразі Ubuntu 24.04. Власні рекомендації Tauri радять збирати на найстаршій базі, яку ви плануєте підтримувати: двійкові файли, зібрані там, вимагають версії glibc цього раннера, тому AppImage або `.deb` із цього робочого процесу може повідомляти `version 'GLIBC_2.39' not found` на Ubuntu 22.04, Debian 12 або старіших дистрибутивах. Таким користувачам слід збирати з вихідного коду у своєму дистрибутиві (або в контейнері 22.04). Перехід робочого процесу на старіший образ раннера є варіантом, якщо потрібна ширша сумісність двійкових файлів.

<a id="build-from-source"></a>
## Збірка з вихідного коду

Вимоги: Node.js 20+, стабільна версія Rust, наведені вище залежності для збірки та Git.

```sh
git clone https://github.com/imtiredwhenitalk/Elyra.git
cd Elyra
npm ci --prefix apps/desktop
cd apps/desktop
npm run check
npm run build -- --bundles appimage deb
```

Прапорець `--bundles` перевизначає `bundle.targets` для цього виклику. Щоб звичайний `npm run build` створював пакети для Linux, додайте цільові платформи Linux до
`apps/desktop/src-tauri/tauri.conf.json`:

```json
"targets": ["msi", "nsis", "appimage", "deb"]
```

Пакети записуються до:

```text
apps/desktop/src-tauri/target/release/bundle/appimage/
apps/desktop/src-tauri/target/release/bundle/deb/
```

Встановіть `ELYRA_DEFAULT_API_URL` перед збіркою, щоб вбудувати базову URL-адресу виробничого API до застосунку, так само як у випуску для Windows. Встановлюйте `TAURI_SIGNING_PRIVATE_KEY` і `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` лише якщо увімкнено артефакти оновлювача; робочий процес для Linux вимикає `createUpdaterArtifacts`, коли ключ підпису відсутній, щоб непідписані збірки все одно завершувалися успішно.

Локальний інструментарій Rust не потрібен, щоб проходити конвеєр випуску; сама збірка для Linux перевіряється в GitHub Actions.

## Записи робочого стола та піктограми

| Артефакт | Розташування | Примітки |
| :--- | :--- | :--- |
| Засіб запуску | `packaging/linux/elyra.desktop` | `Name=Elyra`, `Exec=/opt/elyra/elyra.AppImage %F`, `Icon=elyra`, `Categories=Office;Email;Network;`, обробник `mailto`. |
| Піктограма | `packaging/linux/elyra.png` | PNG 256x256, скопійований з `apps/desktop/src-tauri/icons/128x128@2x.png`. |
| Скрипт оновлення | `packaging/linux/copy-icon.sh` | Повторно копіює піктограму й завершується помилкою, якщо вона не має розміру 256x256; після цього оновіть закріплений хеш у `PKGBUILD`. |

Пошук у Freedesktop hicolor очікує растрову піктограму 256x256 за адресою
`usr/share/icons/hicolor/256x256/apps/elyra.png`, яку встановлює PKGBUILD, щоб `Icon=elyra` у записі робочого стола розв'язувався. Конфігурація пакета Tauri також містить `32x32.png`, `128x128.png` і `128x128@2x.png`; останнє — це джерело 256x256, яке використовується тут. Перевіряйте змінений засіб запуску за допомогою
`desktop-file-validate packaging/linux/elyra.desktop`. Якщо вікно, що працює, не групується зі своїм засобом запуску, порівняйте `xprop WM_CLASS` із
`StartupWMClass=Elyra` і скоригуйте файл робочого стола.

<a id="release-pipeline"></a>
## Конвеєр випуску

| Файл | Призначення |
| :--- | :--- |
| `Elyra-latest-x64.AppImage` | Стабільний AppImage, який використовують документація та PKGBUILD. |
| `Elyra-latest-x64.deb` | Стабільний пакет Debian. |
| `Elyra-<version>-x64.AppImage` | Версійна копія тієї самої збірки. |
| `Elyra-<version>-x64.deb` | Версійна копія тієї самої збірки. |
| `<name>.sha256` | Хеш SHA-256 кожного з наведених вище артефактів. |

`<version>` зчитується з `apps/desktop/src-tauri/tauri.conf.json`, тому назви артефактів завжди відповідають версії застосунку. Завдання збірки завантажує їх усі як артефакт GitHub Actions `elyra-linux-x64` із дозволом `contents` лише на читання. Завдання публікації керується змінною репозиторію
`ELYRA_LINUX_PUBLISH_RELEASE`. Станом на 0.5.0 вона увімкнена, тому той самий випуск GitHub містить інсталятори для Windows і ці артефакти для Linux разом із їхніми контрольними сумами. Завдання використовує `contents: write` лише для публікації; завдання збірки залишається доступним лише для читання. Якщо змінну колись буде вимкнено, файли все одно будуть доступні з артефакту робочого процесу `elyra-linux-x64`.

<a id="flatpak-investigation"></a>
## Дослідження Flatpak

**Рішення: не постачається у 0.5.0.** AppImage та `.deb` покривають поточні потреби Linux за наявного пакувальника Tauri й конвеєра випуску; додавання Flatpak вимагало б окремої збірки в пісочниці, вибору залежності середовища виконання та відповідального за подання до Flathub. Це прогалина в пакуванні, а не обмеження середовища виконання.

Що знадобиться:

1. Маніфест `com.github.imtiredwhenitalk.Elyra.yml` із використанням `flatpak-builder` і SDK/середовища виконання GNOME або Freedesktop, яке надає WebKitGTK 4.1 та libsoup 3 (наприклад `org.gnome.Sdk//46`), плюс розширення з інструментарієм Rust або архів вихідного коду з vendored-залежностями Cargo.
2. Застосунок має збиратися всередині пісочниці з вихідного коду. AppImage Tauri не є хорошим джерелом для Flatpak: його середовище виконання на базі FUSE, вбудований завантажник і шлях `Exec` не підходять для пісочниці Flatpak. Натомість використовуйте збірку з вихідного коду Tauri з цільовими платформами Linux.
3. Дозволи пісочниці для доступу до мережі IMAP, SMTP і HTTPS; сеансова шина `org.freedesktop.secrets` для секретів у сховищі ключів; каталоги даних і конфігурації XDG для локальної бази даних SQLite; desktop portals, Wayland/X11 та IPC для інтеграції webview і вікон.
4. Запис `.desktop`, названий на честь ідентифікатора застосунку Flatpak, із `Exec=elyra`, піктограмою hicolor 256x256, файлом metainfo/appdata XML з рейтингом вмісту, описами та знімками екрана, а також файлом ліцензії.
5. Завдання випуску, яке запускає `flatpak-builder`, експортує один пакет `.flatpak`, виконує перевірку запуску через `flatpak run`, після чого подає застосунок до Flathub і проходить рев'ю. Перед тим як остаточно обрати формат, переконайтеся, що середовище виконання WebKitGTK 4.1 залишається доступним у всіх підтримуваних гілках.

Поверніться до цього питання, коли відповідальний за подання до Flathub буде призначено й CI зможе збирати та перевіряти збірку в пісочниці на кожному тегу.

## Оновлення та видалення

- **AppImage:** завантажте новий артефакт, повторно перевірте файл `.sha256`, замініть старий файл і збережіть наявний каталог даних XDG. Додаткові інструменти оновлення AppImage потрібно налаштовувати окремо.
- **Debian:** `sudo apt install ./Elyra-latest-x64.deb` оновлює на місці;
  `sudo apt remove elyra` видаляє пакет.
- **Arch:** `pacman -Suy elyra` після оновлення PKGBUILD;
  `sudo pacman -Rns elyra` видаляє пакет.
- Видалення не вилучає локальну пошту, налаштування та результати сканування. Відкрийте
  **Settings → Diagnostics** у застосунку, щоб побачити точний шлях до даних; видаляйте його вручну лише після резервного копіювання всього необхідного.

## Усунення несправностей

| Симптом | Причина / виправлення |
| :--- | :--- |
| `dlopen(): error loading libfuse.so.2` | Встановіть `libfuse2` (Debian/Ubuntu) або `fuse2` (Arch) чи запустіть `./Elyra.AppImage --appimage-extract-and-run`. |
| `version 'GLIBC_2.39' not found` | Двійковий файл зібрано на новішому раннері, ніж ваш дистрибутив. Зберіть із вихідного коду на старшій базі (див. [Мінімальна версія glibc](#glibc-baseline)). |
| `pkg-config could not find webkit2gtk-4.1` | Встановіть `libwebkit2gtk-4.1-dev` (Ubuntu 22.04+) / `webkit2gtk-4.1` (Arch). Пакет 4.0 має інший API й не працюватиме. |
| `Can't detect any appindicator library` | Встановіть `libayatana-appindicator3-dev` (для збірки) та його пакет середовища виконання або `libappindicator-gtk3` на Arch. |
| Порожнє вікно після запуску | Перевірте пакети WebKitGTK і піктограм, потім запустіть із термінала, щоб побачити помилки webview. Для віддалених робочих столів може знадобитися програмний рендеринг. |
| Elyra відсутня в меню застосунків | Встановіть `.deb` або PKGBUILD замість звичайного AppImage; запустіть `desktop-file-validate` і перевірте, що `elyra.png` міститься в `hicolor/256x256/apps`. |
| Сховище ключів або секретів не працює в Linux | Поточна інтеграція з keyring є лише Windows-специфічною у функціях Cargo; секрети можуть використовувати змінні середовища як резерв або бути недоступними. Перед використанням входу в хмару відстежуйте це як окреме завдання з зміцнення для Linux. |
| Невідповідність контрольної суми | Завантажте артефакт повторно й переконайтеся, що файл `.sha256` походить із того самого випуску. Ніколи не ігноруйте справжню невідповідність. |

## Пов'язані сторінки

- [Development](development.md) — передумови, команди та процес випуску.
- [Troubleshooting](troubleshooting.md) — загальні проблеми застосунку.
- [Configuration reference](reference/configuration.md) — змінні середовища.
- [`packaging/linux/README.md`](../../packaging/linux/README.md) — подробиці артефактів пакування.
