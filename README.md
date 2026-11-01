
## 1. Описание демонстрационного проекта

Данный проект иллюстрирует работу и архитектурные инварианты системы экстракторов фреймворка Axum 0.7.9:
- `FromRequestParts` — для безопасного и многократного извлечения метаданных запроса (заголовки, путь, состояние) через заимствование `&mut Parts`. В проекте реализован кастомный экстрактор `AuthUser`, проверяющий заголовок `Authorization: Bearer <token>`.
- `FromRequest` — для извлечения тела запроса с передачей единоличного владения потоком `Body`. В проекте реализован обобщенный экстрактор `ValidatedJson<T>`, десериализующий JSON и производящий проверку доменных инвариантов (`Validate`).
- Типизированные ошибки отклонения (`AuthRejection`, `ValidatedJsonRejection`), реализующие трейт `IntoResponse` и генерирующие стандартные HTTP-ответы (401 Unauthorized с заголовком `WWW-Authenticate`, 400 Bad Request, 422 Unprocessable Entity).

---

## 2. Сборка и запуск проекта

### Требования
- Установленный стабильный тулчейн Rust (1.75+ или выше, протестировано на Rust 1.78+ / 1.8x / 1.99).
- Утилита `curl` для проведения интеграционных тестов.

### Пошаговые команды

1. Перейти в каталог проекта:
   ```bash
   cd /home/obj/.gemini/antigravity/scratch/axum-extractor-research
   ```

2. Проверить компиляцию без предупреждений:
   ```bash
   cargo check
   ```

3. Запустить HTTP-сервер:
   ```bash
   cargo run
   ```
   Сервер запустится на адресе `http://127.0.0.1:3000`.

---

## 3. Примеры вызовов через cURL

### 3.1. Успешные запросы

1. **Проверка работоспособности сервиса (`GET /health`):**
   ```bash
   curl -i http://127.0.0.1:3000/health
   ```
   *Ожидаемый ответ:* HTTP `200 OK`, тело `OK`.

2. **Получение профиля по Bearer-токену (`GET /api/profile`):**
   ```bash
   curl -i -H "Authorization: Bearer secret-demo-token" http://127.0.0.1:3000/api/profile
   ```
   *Ожидаемый ответ:* HTTP `200 OK`, JSON:
   ```json
   {"status":"authenticated","token":"secret-demo-token","user_id":"usr_secret-demo-token"}
   ```

3. **Создание пользователя с валидными данными (`POST /api/users`):**
   ```bash
   curl -i -X POST http://127.0.0.1:3000/api/users \
     -H "Authorization: Bearer dev-operator-77" \
     -H "Content-Type: application/json" \
     -d '{"username": "rustacean", "email": "dev@rust-lang.org", "age": 25}'
   ```
   *Ожидаемый ответ:* HTTP `201 Created`, JSON:
   ```json
   {
     "status": "created",
     "user_id": "usr_dev-operator-77",
     "username": "rustacean",
     "email": "dev@rust-lang.org",
     "age": 25,
     "environment": "research-sandbox"
   }
   ```

---

### 3.2. Негативные сценарии (Активация Rejection)

4. **Ошибка авторизации: заголовок `Authorization` отсутствует:**
   ```bash
   curl -i http://127.0.0.1:3000/api/profile
   ```
   *Ожидаемый ответ:* HTTP `401 Unauthorized`, заголовок `WWW-Authenticate: Bearer`, JSON:
   ```json
   {"error":"unauthorized","message":"missing 'Authorization' header"}
   ```

5. **Ошибка авторизации: неверная схема (Basic вместо Bearer):**
   ```bash
   curl -i -H "Authorization: Basic dXNlcjpwYXNz" http://127.0.0.1:3000/api/profile
   ```
   *Ожидаемый ответ:* HTTP `401 Unauthorized`, JSON:
   ```json
   {"error":"unauthorized","message":"authorization scheme must be 'Bearer'"}
   ```

6. **Ошибка авторизации: отклоненный токен (`blocked-token`):**
   ```bash
   curl -i -H "Authorization: Bearer blocked-token" http://127.0.0.1:3000/api/profile
   ```
   *Ожидаемый ответ:* HTTP `401 Unauthorized`, JSON:
   ```json
   {"error":"unauthorized","message":"token is empty or rejected"}
   ```

7. **Синтаксическая ошибка в теле запроса (битый JSON):**
   ```bash
   curl -i -X POST http://127.0.0.1:3000/api/users \
     -H "Authorization: Bearer dev-token" \
     -H "Content-Type: application/json" \
     -d '{"username": "alice", "email":'
   ```
   *Ожидаемый ответ:* HTTP `400 Bad Request`, JSON:
   ```json
   {"error":"malformed_json","message":"Failed to parse the request body as JSON: email: EOF while parsing a value at line 1 column 30"}
   ```

8. **Ошибка бизнес-валидации экстрактора `ValidatedJson` (возраст < 18):**
   ```bash
   curl -i -X POST http://127.0.0.1:3000/api/users \
     -H "Authorization: Bearer dev-token" \
     -H "Content-Type: application/json" \
     -d '{"username": "junior", "email": "junior@test.com", "age": 16}'
   ```
   *Ожидаемый ответ:* HTTP `422 Unprocessable Entity`, JSON:
   ```json
   {"error":"validation_failed","message":"age must be at least 18"}
   ```

---

## 4. Пошаговый Walkthrough для отладки (Debugging в исходниках Axum)



### Контрольная точка 1: Разделение запроса на части и тело
- **Файл**: `axum/src/handler/mod.rs` (крейт `axum`, тег `axum-v0.7.9`)
- **Строка**: `224`
- **Код**:
  ```rust
  let (mut parts, body) = req.into_parts();
  ```
- **Что наблюдать в отладчике**:
  Объект `req: Request<Body>` распадается на `parts: http::request::Parts` (заголовки, URI, метод, расширения) и `body: axum::body::Body`. Обратите внимание, что владение телом сохраняется отдельно и не копируется.

### Контрольная точка 2: Последовательное извлечение `FromRequestParts`
- **Файл**: `axum/src/handler/mod.rs`
- **Строка**: `228`
- **Код**:
  ```rust
  let $ty = match $ty::from_request_parts(&mut parts, state).await {
      Ok(value) => value,
      Err(rejection) => return rejection.into_response(),
  };
  ```
- **Что наблюдать в отладчике**:
  Последовательный вызов экстракторов аргументов функции (`State`, затем `AuthUser`). Указатель `&mut parts` передается по ссылке. В случае если экстрактор возвращает ошибку (например, `AuthRejection::MissingHeader`), управление немедленно переходит на строку `230` и выполнение хэндлера прерывается с возвратом HTTP 401, даже не приступая к чтению тела запроса.

### Контрольная точка 3: Реконструкция `Request` перед поглощением тела
- **Файл**: `axum/src/handler/mod.rs`
- **Строка**: `234`
- **Код**:
  ```rust
  let req = Request::from_parts(parts, body);
  ```
- **Что наблюдать в отладчике**:
  Запрос повторно собирается из оставшихся частей `parts` и оригинального `body`.

### Контрольная точка 4: Вызов экстрактора тела запроса `FromRequest`
- **Файл**: `axum/src/handler/mod.rs`
- **Строка**: `236`
- **Код**:
  ```rust
  let $last = match $last::from_request(req, state).await {
      Ok(value) => value,
      Err(rejection) => return rejection.into_response(),
  };
  ```
- **Что наблюдать в отладчике**:
  Передача единоличного владения `req` в экстрактор `ValidatedJson`. Здесь вызывается метод `from_request`, который вычитывает поток байт через `http_body_util::BodyExt` и выполняет парсинг JSON.

### Контрольная точка 5: Реализация распаковки кортежей
- **Файл**: `axum-core/src/extract/tuple.rs` (крейт `axum-core`, тег `axum-core-v0.4.5`)
- **Строки**: `60–68`
- **Код**:
  Макрос `impl_from_request!`, реализующий `FromRequest<S>` для кортежей экстракторов `($($ty,)* $last,)`. Здесь на уровне системы типов зафиксировано правило: все элементы кортежа кроме последнего ограничены типами `FromRequestParts<S>`, и только последний `$last` может быть `FromRequest<S>`.

### Контрольная точка 6: Вызов пользовательской бизнес-логики
- **Файл**: `axum/src/handler/mod.rs`
- **Строка**: `241`
- **Код**:
  ```rust
  let res = self($($ty,)* $last,).await;
  ```
- **Что наблюдать в отладчике**:
  Все аргументы успешно извлечены и переданы как параметры в асинхронную функцию `create_user`.
