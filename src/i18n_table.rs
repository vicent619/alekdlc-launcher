pub fn lookup(lang: Language, key: &str) -> Option<&'static str> {
    let (ru, en) = table().get(key)?;
    Some(match lang {
        Language::Ru => ru.as_str(),
        Language::En => en.as_str(),
    })
}

fn put(map: &mut std::collections::HashMap<String, (String, String)>, key: &str, ru: &str, en: &str) {
    map.insert(key.to_owned(), (ru.to_owned(), en.to_owned()));
}

fn table() -> &'static std::collections::HashMap<String, (String, String)> {
    static TABLE: std::sync::OnceLock<std::collections::HashMap<String, (String, String)>> = std::sync::OnceLock::new();
    TABLE.get_or_init(build_table)
}

fn build_table() -> std::collections::HashMap<String, (String, String)> {
    let mut map = std::collections::HashMap::new();
    put(&mut map, crate::secret!("nav.home"), crate::secret!("Главная"), crate::secret!("Home"));
    put(&mut map, crate::secret!("nav.settings"), crate::secret!("Настройки"), crate::secret!("Settings"));
    put(&mut map, crate::secret!("win.minimize"), crate::secret!("Свернуть"), crate::secret!("Minimize"));
    put(&mut map, crate::secret!("win.close"), crate::secret!("Закрыть"), crate::secret!("Close"));
    put(&mut map, crate::secret!("win.closed"), crate::secret!("Лаунчер закрыт"), crate::secret!("Launcher closed"));
    put(&mut map, crate::secret!("win.reopen"), crate::secret!("Открыть снова"), crate::secret!("Open again"));
    put(&mut map, crate::secret!("win.closedGame"), crate::secret!("Игра запущена, лаунчер закрылся"), crate::secret!("Game launched, launcher closed"));
    put(&mut map, crate::secret!("splash.connecting"), crate::secret!("Подключение к серверу"), crate::secret!("Connecting to server"));
    put(&mut map, crate::secret!("splash.signing"), crate::secret!("Вход в аккаунт"), crate::secret!("Signing in"));
    put(&mut map, crate::secret!("offline.title"), crate::secret!("Нет соединения"), crate::secret!("No connection"));
    put(&mut map, crate::secret!("offline.lead"), crate::secret!("Не удалось связаться с сервером AlekDLC. Проверьте интернет и попробуйте ещё раз."), crate::secret!("Couldn't reach the AlekDLC server. Check your connection and try again."));
    put(&mut map, crate::secret!("offline.retry"), crate::secret!("Повторить"), crate::secret!("Retry"));
    put(&mut map, crate::secret!("update.title"), crate::secret!("Доступно обновление"), crate::secret!("Update available"));
    put(&mut map, crate::secret!("update.lead"), crate::secret!("Вышла версия {version}. Скачайте новый лаунчер, чтобы продолжить."), crate::secret!("Version {version} is out. Download the new launcher to continue."));
    put(&mut map, crate::secret!("update.download"), crate::secret!("Скачать лаунчер"), crate::secret!("Download launcher"));
    put(&mut map, crate::secret!("login.title"), crate::secret!("С возвращением"), crate::secret!("Welcome back"));
    put(&mut map, crate::secret!("login.lead"), crate::secret!("Войдите, чтобы начать играть"), crate::secret!("Sign in to start playing"));
    put(&mut map, crate::secret!("login.login"), crate::secret!("Логин или почта"), crate::secret!("Username or email"));
    put(&mut map, crate::secret!("login.password"), crate::secret!("Пароль"), crate::secret!("Password"));
    put(&mut map, crate::secret!("login.forgot"), crate::secret!("Забыли пароль?"), crate::secret!("Forgot password?"));
    put(&mut map, crate::secret!("login.submit"), crate::secret!("Войти"), crate::secret!("Sign in"));
    put(&mut map, crate::secret!("login.show"), crate::secret!("Показать пароль"), crate::secret!("Show password"));
    put(&mut map, crate::secret!("home.greeting"), crate::secret!("Привет, {name}"), crate::secret!("Hi, {name}"));
    put(&mut map, crate::secret!("home.ready"), crate::secret!("Всё готово к игре."), crate::secret!("Everything is ready to play."));
    put(&mut map, crate::secret!("home.checking"), crate::secret!("Сверяем файлы игры, это займёт пару секунд."), crate::secret!("Checking the game files, this takes a moment."));
    put(&mut map, crate::secret!("home.needDownload"), crate::secret!("Перед первым запуском скачаем файлы игры."), crate::secret!("We'll download the game files before the first launch."));
    put(&mut map, crate::secret!("home.needUpdate"), crate::secret!("Вышло обновление, скачаем его перед запуском."), crate::secret!("An update is out, we'll download it before launch."));
    put(&mut map, crate::secret!("home.maintenance"), crate::secret!("Идут технические работы"), crate::secret!("Maintenance in progress"));
    put(&mut map, crate::secret!("card.premium"), crate::secret!("Premium"), crate::secret!("Premium"));
    put(&mut map, crate::secret!("card.premiumActive"), crate::secret!("Активен до {date}"), crate::secret!("Active until {date}"));
    put(&mut map, crate::secret!("card.premiumForever"), crate::secret!("Активен"), crate::secret!("Active"));
    put(&mut map, crate::secret!("card.premiumNotActive"), crate::secret!("Не активен"), crate::secret!("Not active"));
    put(&mut map, crate::secret!("card.premiumInactive"), crate::secret!("Не активен. Откройте все возможности клиента."), crate::secret!("Not active. Unlock everything the client can do."));
    put(&mut map, crate::secret!("card.premiumBuy"), crate::secret!("Купить"), crate::secret!("Buy"));
    put(&mut map, crate::secret!("card.premiumExtend"), crate::secret!("Продлить"), crate::secret!("Extend"));
    put(&mut map, crate::secret!("card.account"), crate::secret!("Аккаунт"), crate::secret!("Account"));
    put(&mut map, crate::secret!("card.accountSince"), crate::secret!("С нами с {date}"), crate::secret!("With us since {date}"));
    put(&mut map, crate::secret!("card.premiumUntil"), crate::secret!("до {date}"), crate::secret!("until {date}"));
    put(&mut map, crate::secret!("card.premiumLifetime"), crate::secret!("Навсегда"), crate::secret!("Forever"));
    put(&mut map, crate::secret!("card.premiumLifetimeHint"), crate::secret!("Бессрочный доступ"), crate::secret!("Lifetime access"));
    put(&mut map, crate::secret!("card.premiumMore"), crate::secret!("Подробнее"), crate::secret!("Details"));
    put(&mut map, crate::secret!("card.premiumOff"), crate::secret!("Не подключён"), crate::secret!("Not subscribed"));
    put(&mut map, crate::secret!("card.premiumPitch"), crate::secret!("Откройте все возможности клиента"), crate::secret!("Unlock everything the client can do"));
    put(&mut map, crate::secret!("card.days.one"), crate::secret!("день"), crate::secret!("day"));
    put(&mut map, crate::secret!("card.days.few"), crate::secret!("дня"), crate::secret!("days"));
    put(&mut map, crate::secret!("card.days.many"), crate::secret!("дней"), crate::secret!("days"));
    put(&mut map, crate::secret!("home.buildSoon"), crate::secret!("Эта сборка клиента появится совсем скоро."), crate::secret!("This client build is coming very soon."));
    put(&mut map, crate::secret!("build.free"), crate::secret!("Бесплатная"), crate::secret!("Free"));
    put(&mut map, crate::secret!("build.premium"), crate::secret!("Premium"), crate::secret!("Premium"));
    put(&mut map, crate::secret!("error.BUILD_UNAVAILABLE"), crate::secret!("Эта сборка клиента ещё не опубликована."), crate::secret!("This client build has not been published yet."));
    put(&mut map, crate::secret!("error.PREMIUM_REQUIRED"), crate::secret!("Для этой сборки нужна активная подписка Premium."), crate::secret!("This build requires an active Premium subscription."));
    put(&mut map, crate::secret!("card.cabinet"), crate::secret!("Личный кабинет"), crate::secret!("Account page"));
    put(&mut map, crate::secret!("card.clientUnpublished"), crate::secret!("Сборка пока не опубликована"), crate::secret!("No build published yet"));
    put(&mut map, crate::secret!("role.ADMIN"), crate::secret!("Админ"), crate::secret!("Admin"));
    put(&mut map, crate::secret!("role.MODERATOR"), crate::secret!("Модератор"), crate::secret!("Moderator"));
    put(&mut map, crate::secret!("role.SUPPORT"), crate::secret!("Поддержка"), crate::secret!("Support"));
    put(&mut map, crate::secret!("role.MEDIA"), crate::secret!("Медиа"), crate::secret!("Media"));
    put(&mut map, crate::secret!("role.USER"), crate::secret!("Игрок"), crate::secret!("Player"));
    put(&mut map, crate::secret!("play.play"), crate::secret!("Играть"), crate::secret!("Play"));
    put(&mut map, crate::secret!("play.install"), crate::secret!("Установить"), crate::secret!("Install"));
    put(&mut map, crate::secret!("play.update"), crate::secret!("Обновить"), crate::secret!("Update"));
    put(&mut map, crate::secret!("play.cancel"), crate::secret!("Отмена"), crate::secret!("Cancel"));
    put(&mut map, crate::secret!("play.idleReady"), crate::secret!("Готово к запуску"), crate::secret!("Ready to launch"));
    put(&mut map, crate::secret!("play.idleSize"), crate::secret!("Нужно скачать {size}"), crate::secret!("{size} to download"));
    put(&mut map, crate::secret!("play.unavailable"), crate::secret!("Запуск недоступен"), crate::secret!("Launch unavailable"));
    put(&mut map, crate::secret!("play.checking"), crate::secret!("Проверка файлов"), crate::secret!("Checking files"));
    put(&mut map, crate::secret!("play.connectingClient"), crate::secret!("Подключение к серверу клиента"), crate::secret!("Connecting to the client server"));
    put(&mut map, crate::secret!("play.waitingClient"), crate::secret!("Подготовка клиента"), crate::secret!("Preparing the client"));
    put(&mut map, crate::secret!("play.downloadClient"), crate::secret!("Загрузка клиента"), crate::secret!("Downloading client"));
    put(&mut map, crate::secret!("play.downloadGame"), crate::secret!("Загрузка файлов игры"), crate::secret!("Downloading game files"));
    put(&mut map, crate::secret!("play.downloadMods"), crate::secret!("Загрузка модов"), crate::secret!("Downloading mods"));
    put(&mut map, crate::secret!("play.downloadJava"), crate::secret!("Загрузка Java"), crate::secret!("Downloading Java"));
    put(&mut map, crate::secret!("play.unpacking"), crate::secret!("Распаковка файлов"), crate::secret!("Unpacking files"));
    put(&mut map, crate::secret!("play.verifying"), crate::secret!("Проверка целостности"), crate::secret!("Verifying integrity"));
    put(&mut map, crate::secret!("play.authorizing"), crate::secret!("Подготовка сессии"), crate::secret!("Preparing session"));
    put(&mut map, crate::secret!("play.launching"), crate::secret!("Запуск игры"), crate::secret!("Launching game"));
    put(&mut map, crate::secret!("play.waitingWindow"), crate::secret!("Ожидание окна игры"), crate::secret!("Waiting for game window"));
    put(&mut map, crate::secret!("play.speed"), crate::secret!("{done} из {total} · {speed}/с"), crate::secret!("{done} of {total} · {speed}/s"));
    put(&mut map, crate::secret!("play.left"), crate::secret!("осталось {time}"), crate::secret!("{time} left"));
    put(&mut map, crate::secret!("settings.title"), crate::secret!("Настройки"), crate::secret!("Settings"));
    put(&mut map, crate::secret!("settings.game"), crate::secret!("Игра"), crate::secret!("Game"));
    put(&mut map, crate::secret!("settings.memory"), crate::secret!("Оперативная память"), crate::secret!("Memory"));
    put(&mut map, crate::secret!("settings.memoryHint"), crate::secret!("Рекомендуем от 4 ГБ. Доступно {total}."), crate::secret!("We recommend 4 GB or more. {total} available."));
    put(&mut map, crate::secret!("settings.folder"), crate::secret!("Папка игры"), crate::secret!("Game folder"));
    put(&mut map, crate::secret!("settings.open"), crate::secret!("Открыть"), crate::secret!("Open"));
    put(&mut map, crate::secret!("settings.launcher"), crate::secret!("Лаунчер"), crate::secret!("Launcher"));
    put(&mut map, crate::secret!("settings.language"), crate::secret!("Язык"), crate::secret!("Language"));
    put(&mut map, crate::secret!("settings.files"), crate::secret!("Файлы"), crate::secret!("Files"));
    put(&mut map, crate::secret!("settings.verify"), crate::secret!("Проверить и восстановить файлы"), crate::secret!("Verify and repair files"));
    put(&mut map, crate::secret!("settings.verifyHint"), crate::secret!("Заново сверит все файлы игры при следующем запуске"), crate::secret!("Re-checks all game files on next launch"));
    put(&mut map, crate::secret!("settings.verifyBtn"), crate::secret!("Проверить"), crate::secret!("Verify"));
    put(&mut map, crate::secret!("settings.verifyDone"), crate::secret!("Файлы будут проверены при запуске"), crate::secret!("Files will be verified on launch"));
    put(&mut map, crate::secret!("settings.version"), crate::secret!("Лаунчер {version}"), crate::secret!("Launcher {version}"));
    put(&mut map, crate::secret!("settings.site"), crate::secret!("Сайт"), crate::secret!("Website"));
    put(&mut map, crate::secret!("settings.support"), crate::secret!("Поддержка"), crate::secret!("Support"));
    put(&mut map, crate::secret!("menu.cabinet"), crate::secret!("Личный кабинет"), crate::secret!("Account page"));
    put(&mut map, crate::secret!("menu.logout"), crate::secret!("Выйти"), crate::secret!("Sign out"));
    put(&mut map, crate::secret!("gb"), crate::secret!("{n} ГБ"), crate::secret!("{n} GB"));
    put(&mut map, crate::secret!("toast.loggedOut"), crate::secret!("Вы вышли из аккаунта"), crate::secret!("You signed out"));
    put(&mut map, crate::secret!("toast.cancelled"), crate::secret!("Загрузка отменена"), crate::secret!("Download cancelled"));
    put(&mut map, crate::secret!("toast.openSite"), crate::secret!("Откроется в браузере: {url}"), crate::secret!("Opens in browser: {url}"));
    put(&mut map, crate::secret!("toast.openFolder"), crate::secret!("Откроется папка: {path}"), crate::secret!("Opens folder: {path}"));
    put(&mut map, crate::secret!("error.INVALID_CREDENTIALS"), crate::secret!("Неверный логин или пароль"), crate::secret!("Wrong username or password"));
    put(&mut map, crate::secret!("error.USER_BANNED"), crate::secret!("Аккаунт заблокирован"), crate::secret!("This account is banned"));
    put(&mut map, crate::secret!("error.HWID_TAKEN"), crate::secret!("К этому компьютеру уже привязан другой аккаунт"), crate::secret!("Another account is already bound to this PC"));
    put(&mut map, crate::secret!("error.HWID_MISMATCH"), crate::secret!("Аккаунт привязан к другому компьютеру. Сбросить привязку можно в личном кабинете."), crate::secret!("This account is bound to another PC. You can reset it on your account page."));
    put(&mut map, crate::secret!("error.HWID_INVALID"), crate::secret!("Не удалось определить ваш компьютер"), crate::secret!("Couldn't identify your PC"));
    put(&mut map, crate::secret!("error.INVALID_TOKEN"), crate::secret!("Сессия истекла, войдите снова"), crate::secret!("Session expired, please sign in again"));
    put(&mut map, crate::secret!("error.RATE_LIMITED"), crate::secret!("Слишком много попыток. Подождите минуту."), crate::secret!("Too many attempts. Wait a minute."));
    put(&mut map, crate::secret!("error.MAINTENANCE"), crate::secret!("Идут технические работы. Попробуйте позже."), crate::secret!("Maintenance in progress. Try again later."));
    put(&mut map, crate::secret!("error.VALIDATION"), crate::secret!("Заполните логин и пароль"), crate::secret!("Enter your username and password"));
    put(&mut map, crate::secret!("error.NOT_FOUND"), crate::secret!("Файл не найден на сервере"), crate::secret!("File not found on the server"));
    put(&mut map, crate::secret!("error.OFFLINE"), crate::secret!("Нет соединения с сервером"), crate::secret!("No connection to the server"));
    put(&mut map, crate::secret!("error.UPSTREAM_DOWN"), crate::secret!("Нет соединения с сервером"), crate::secret!("No connection to the server"));
    put(&mut map, crate::secret!("error.CHANNEL_UNTRUSTED"), crate::secret!("Не удалось подтвердить подлинность сервера"), crate::secret!("Couldn't verify the server"));
    put(&mut map, crate::secret!("error.CHECKSUM"), crate::secret!("Файл повреждён при загрузке. Попробуйте ещё раз."), crate::secret!("The file was corrupted during download. Try again."));
    put(&mut map, crate::secret!("error.NO_CLIENT"), crate::secret!("Сборка клиента пока не опубликована"), crate::secret!("No client build published yet"));
    put(&mut map, crate::secret!("error.CLIENT_REJECTED"), crate::secret!("Сервер не выдал клиент:"), crate::secret!("The server didn't hand out the client:"));
    put(&mut map, crate::secret!("error.CLIENT_UNREACHABLE"), crate::secret!("Не удалось подключиться к серверу клиента. Проверьте интернет и попробуйте ещё раз."), crate::secret!("Couldn't connect to the client server. Check your connection and try again."));
    put(&mut map, crate::secret!("error.CLIENT_TIMEOUT"), crate::secret!("Сервер слишком долго собирает клиент. Попробуйте ещё раз."), crate::secret!("The server took too long to build the client. Try again."));
    put(&mut map, crate::secret!("error.CLIENT_PROTOCOL"), crate::secret!("Сервер клиента ответил некорректно. Попробуйте позже."), crate::secret!("The client server sent an invalid reply. Try again later."));
    put(&mut map, crate::secret!("error.PROTECTION_FAILED"), crate::secret!("Не удалось подготовить защиту клиента. Попробуйте позже."), crate::secret!("Couldn't prepare client protection. Try again later."));
    put(&mut map, crate::secret!("error.LAUNCH_EXITED"), crate::secret!("Игра закрылась при запуске. Отчёт об ошибке лежит в папке игры, в crash-reports."), crate::secret!("The game closed during startup. The crash report is in the game folder, under crash-reports."));
    put(&mut map, crate::secret!("error.LAUNCH_FAILED"), crate::secret!("Не удалось запустить игру. Проверьте файлы в настройках и попробуйте ещё раз."), crate::secret!("Couldn't start the game. Verify the files in settings and try again."));
    put(&mut map, crate::secret!("error.IO"), crate::secret!("Не удалось записать файлы игры. Проверьте свободное место на диске."), crate::secret!("Couldn't write the game files. Check your free disk space."));
    put(&mut map, crate::secret!("error.INTERNAL"), crate::secret!("Что-то пошло не так. Попробуйте ещё раз."), crate::secret!("Something went wrong. Try again."));
    map
}
