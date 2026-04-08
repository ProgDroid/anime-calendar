# i18n Key Naming Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Normalize all translation keys to a consistent hierarchy, removing duplicate namespaces, fixing mixed error-key suffixes, and patching two bugs (missing `app.loading` key, mismatched `nameMaxLength`/`calendarNameMaxLength` between locales).

**Architecture:** All changes are pure renames in two JSON locale files plus matching `$t()`/`t()` call updates in five Vue components. No new files. No runtime behaviour changes.

**Tech Stack:** Vue 3 + vue-i18n 11, TypeScript, Vitest

---

## Key Rename Map

### Namespace restructure

| Old key | New key |
|---|---|
| `login.google.failed` | `auth.google.failed` |
| `components.login.title` | `auth.login.title` |
| `components.login.email` | `auth.login.email` |
| `components.login.password` | `auth.login.password` |
| `components.login.rememberMe` | `auth.login.rememberMe` |
| `components.login.forgotPassword` | `auth.login.forgotPassword` |
| `components.login.submit` | `auth.login.submit` |
| `components.login.emailPlaceholder` | `auth.login.emailPlaceholder` |
| `components.login.passwordPlaceholder` | `auth.login.passwordPlaceholder` |
| `components.register.title` | `auth.register.title` |
| `components.register.name` | `auth.register.name` |
| `components.register.email` | `auth.register.email` |
| `components.register.password` | `auth.register.password` |
| `components.register.confirmPassword` | `auth.register.confirmPassword` |
| `components.register.submit` | `auth.register.submit` |
| `components.register.noAccount` | `auth.register.noAccount` |
| `components.register.alreadyHaveAccount` | `auth.register.alreadyHaveAccount` |
| `components.register.usernamePlaceholder` | `auth.register.usernamePlaceholder` |
| `components.register.registering` | `auth.register.registering` |

Keys removed (unused or duplicate):
- `components.register.orContinueWith` — duplicate of `app.orContinueWith`, never referenced in code

### Error/success key standardisation (`*Failed` suffix, present tense)

| Old key | New key |
|---|---|
| `calendar.failedToLoad` | `calendar.loadFailed` |
| `calendar.failedToFetchItems` | `calendar.fetchItemsFailed` |
| `calendar.updatedFail` | `calendar.updateFailed` |
| `calendar.updatedSuccess` | `calendar.updateSuccess` |
| `calendars.loadingFailed` | `calendars.loadFailed` |
| `userSettings.fetchError` | `userSettings.fetchFailed` |
| `userSettings.updateError` | `userSettings.updateFailed` |
| `userDetails.failedToFetch` | `userDetails.fetchFailed` |

### Bug fixes

| Issue | Fix |
|---|---|
| `app.loading` used in `UserSettingsPage.vue:75` but missing from both JSONs | Add `app.loading` to both locales |
| `en.json` has `calendar.nameMaxLength`, `pt.json` has `calendar.calendarNameMaxLength` | Rename pt key to `nameMaxLength` |

---

## Files

- Modify: `frontend/src/locales/en.json`
- Modify: `frontend/src/locales/pt.json`
- Modify: `frontend/src/components/GoogleLoginButton.vue` (1 key)
- Modify: `frontend/src/components/LoginPage.vue` (auth.login.* + auth.register.*)
- Modify: `frontend/src/components/Register.vue` (auth.register.* + auth.login.*)
- Modify: `frontend/src/components/CalendarPage.vue` (error/success renames)
- Modify: `frontend/src/components/UserSettingsPage.vue` (fetchFailed, updateFailed)

---

## Task 1: Update `en.json`

**Files:**
- Modify: `frontend/src/locales/en.json`

- [ ] **Step 1: Replace locale file content**

New content for `frontend/src/locales/en.json`:

```json
{
  "app": {
    "title": "Anime Calendar",
    "myCalendars": "My Calendars",
    "myAccount": "My Account",
    "settings": "Settings",
    "logout": "Logout",
    "orContinueWith": "Or continue with",
    "cancel": "Cancel",
    "confirm": "Confirm",
    "loading": "Loading..."
  },
  "auth": {
    "google": {
      "failed": "Google login failed"
    },
    "login": {
      "title": "Login",
      "email": "Email",
      "password": "Password",
      "rememberMe": "Remember Me",
      "forgotPassword": "Forgot Password?",
      "submit": "Login",
      "emailPlaceholder": "Enter your email",
      "passwordPlaceholder": "Enter your password"
    },
    "register": {
      "title": "Register",
      "name": "Name",
      "email": "Email",
      "password": "Password",
      "confirmPassword": "Confirm Password",
      "submit": "Register",
      "noAccount": "Don't have an account?",
      "alreadyHaveAccount": "Already have an account?",
      "usernamePlaceholder": "Enter your username",
      "registering": "Registering..."
    }
  },
  "userSettings": {
    "title": "User Settings",
    "theme": "Theme",
    "light": "Light",
    "dark": "Dark",
    "language": "Language",
    "titleLanguage": "Title Language",
    "english": "English",
    "romaji": "Romaji",
    "native": "Native",
    "portuguese": "Portuguese",
    "timezone": "Timezone",
    "timezonePlaceholder": "e.g., UTC, Europe/London, America/New_York",
    "save": "Save Settings",
    "updateSuccess": "Settings updated successfully!",
    "updateFailed": "Failed to update user settings",
    "fetchFailed": "Failed to fetch user settings"
  },
  "calendar": {
    "edit": "Edit Calendar",
    "search": "Search",
    "itemName": "Item Name",
    "itemNamePlaceholder": "Enter item name",
    "mediaType": "Media Type",
    "mediaTypeAny": "Any",
    "mediaTypeAnime": "Anime",
    "mediaTypeManga": "Manga",
    "fetchingItems": "Fetching...",
    "fetchItems": "Fetch Items",
    "fetchedItems": "Fetched Items",
    "noImage": "No Image",
    "episodes": "Episodes",
    "alreadyInCalendar": "Already in Calendar",
    "addSelectedToCalendar": "Add Selected Items to Calendar",
    "title": "Calendar",
    "name": "Calendar Name",
    "namePlaceholder": "Enter calendar name",
    "language": "Language",
    "english": "English",
    "romaji": "Romaji",
    "native": "Native",
    "itemsInCalendar": "Items in Calendar",
    "remove": "Remove",
    "recommendedItems": "Recommended Items",
    "add": "Add",
    "clear": "Clear Calendar",
    "submitting": "Submitting...",
    "submit": "Submit Calendar",
    "noRecommendations": "No recommendations for your currently selected items.",
    "addItemsToSeeRecommendations": "Add items to your calendar to see recommendations.",
    "loadFailed": "Failed to load calendar",
    "enterName": "Please enter a name",
    "fetchItemsFailed": "Failed to fetch items",
    "enterCalendarName": "Please enter a name for the calendar",
    "nameMaxLength": "Calendar name must be {max_length} characters or less",
    "noItemsSelected": "Please add at least one item to the calendar",
    "updateSuccess": "Calendar {name} updated",
    "updateFailed": "Failed to update calendar"
  },
  "calendars": {
    "title": "My Calendars",
    "createNew": "Create New Calendar",
    "loading": "Loading calendars...",
    "notFound": "No calendars found.",
    "created": "Created",
    "updated": "Updated",
    "export": "Export",
    "edit": "Edit",
    "delete": "Delete",
    "pagePrevious": "Previous",
    "pageNext": "Next",
    "paginationText": "Showing {first} to {last} of {total} calendars",
    "loadFailed": "Failed to load calendars",
    "deleteFailed": "Failed to delete calendar",
    "deleteConfirmTitle": "Delete Calendar",
    "deleteConfirmMessage": "Are you sure you want to delete this calendar? This cannot be undone.",
    "exportFailed": "Failed to export calendar",
    "exportDownload": "Download .ics",
    "copyLink": "Copy subscription link",
    "linkCopied": "Subscription link copied to clipboard!",
    "openInGoogle": "Add to Google Calendar"
  },
  "userDetails": {
    "fetchFailed": "Failed to fetch user details",
    "updateSuccess": "User details updated successfully!",
    "updateFailed": "Failed to update user details",
    "passwordsDontMatch": "New passwords do not match",
    "passwordLength": "New password must be at least {min} characters long",
    "passwordContent": "New password must contain at least one uppercase letter, one lowercase letter, one digit, and one special character",
    "passwordMustBeDifferent": "New password must be different from the current password",
    "passwordUpdateSuccess": "Password updated successfully!",
    "currentPasswordIncorrect": "Current password is incorrect",
    "passwordUpdateFailed": "Failed to update password",
    "accountDelete": "Are you sure you want to delete your account? This action cannot be undone.",
    "accountDeleteFailed": "Failed to delete account",
    "title": "User Details",
    "loading": "Loading user details...",
    "name": "Name",
    "email": "Email",
    "editDetails": "Edit Details",
    "viewSettings": "View Settings",
    "username": "Username",
    "cancel": "Cancel",
    "update": "Save Details",
    "changePassword": "Update Password",
    "currentPassword": "Current Password",
    "newPassword": "New Password",
    "passwordHint": "Password must be at least 12 characters with uppercase, lowercase, digit, and special character",
    "confirmPassword": "Confirm New Password",
    "passwordUpdating": "Updating...",
    "passwordUpdate": "Update Password",
    "accountDeleteButton": "Delete Account",
    "accountDeleteWarning": "Deleting your account will remove all your data permanently. This action cannot be undone.",
    "accountDeleting": "Deleting..."
  },
  "errors": {
    "generic": "An error occurred. Please try again."
  }
}
```

- [ ] **Step 2: Verify JSON is valid**

```bash
cd frontend && node -e "JSON.parse(require('fs').readFileSync('src/locales/en.json','utf8')); console.log('OK')"
```

Expected: `OK`

---

## Task 2: Update `pt.json`

**Files:**
- Modify: `frontend/src/locales/pt.json`

- [ ] **Step 1: Replace locale file content**

New content for `frontend/src/locales/pt.json`:

```json
{
  "app": {
    "title": "Calendário de Anime",
    "myCalendars": "Meus Calendários",
    "myAccount": "Minha Conta",
    "settings": "Configurações",
    "logout": "Sair",
    "orContinueWith": "Ou continue com",
    "cancel": "Cancelar",
    "confirm": "Confirmar",
    "loading": "A carregar..."
  },
  "auth": {
    "google": {
      "failed": "Falhou ao entrar com Google"
    },
    "login": {
      "title": "Entrar",
      "email": "Email",
      "password": "Senha",
      "rememberMe": "Lembrar-me",
      "forgotPassword": "Esqueceu a Senha?",
      "submit": "Entrar",
      "emailPlaceholder": "Digite seu email",
      "passwordPlaceholder": "Digite sua senha"
    },
    "register": {
      "title": "Registar",
      "name": "Nome",
      "email": "Email",
      "password": "Senha",
      "confirmPassword": "Confirmar Senha",
      "submit": "Registar",
      "noAccount": "Não tem uma conta?",
      "alreadyHaveAccount": "Já tem uma conta?",
      "usernamePlaceholder": "Digite seu nome de usuário",
      "registering": "Registando..."
    }
  },
  "userSettings": {
    "title": "Configurações do Usuário",
    "theme": "Tema",
    "light": "Claro",
    "dark": "Escuro",
    "language": "Idioma",
    "titleLanguage": "Idioma de Títulos",
    "english": "Inglês",
    "romaji": "Romaji",
    "native": "Nativo",
    "portuguese": "Português",
    "timezone": "Fuso Horário",
    "timezonePlaceholder": "ex: UTC, Europe/London, America/New_York",
    "save": "Guardar Configurações",
    "updateSuccess": "Configurações atualizadas com sucesso!",
    "updateFailed": "Atualização de configurações falhou",
    "fetchFailed": "Erro a carregar configurações"
  },
  "calendar": {
    "edit": "Editar Calendário",
    "search": "Procurar",
    "itemName": "Nome do Item",
    "itemNamePlaceholder": "Introduza o nome do item",
    "mediaType": "Tipo de Média",
    "mediaTypeAny": "Qualquer",
    "mediaTypeAnime": "Anime",
    "mediaTypeManga": "Manga",
    "fetchingItems": "A Procurar...",
    "fetchItems": "Procurar Items",
    "fetchedItems": "Items Encontrados",
    "noImage": "Sem Imagem",
    "episodes": "Episódios",
    "alreadyInCalendar": "Presente no Calendário",
    "addSelectedToCalendar": "Adicionar Items Selecionados ao Calendário",
    "title": "Calendário",
    "name": "Nome do Calendário",
    "namePlaceholder": "Introduza o nome do Calendário",
    "language": "Linguagem",
    "english": "Inglês",
    "romaji": "Romaji",
    "native": "Nativo",
    "itemsInCalendar": "Items no Calendário",
    "remove": "Remover",
    "recommendedItems": "Items Recomendados",
    "add": "Adicionar",
    "clear": "Limpar Calendário",
    "submitting": "Submetendo...",
    "submit": "Submeter Calendar",
    "noRecommendations": "Sem recomendações para a sua seleção atual.",
    "addItemsToSeeRecommendations": "Adicione items ao seu calendário para ver recomendações.",
    "loadFailed": "Falha ao carregar calendário",
    "enterName": "Por favor introduza um nome",
    "fetchItemsFailed": "Falha ao procurar items",
    "enterCalendarName": "Por favor introduza um nome para o calendário",
    "nameMaxLength": "Nome do calendário deve ser {max_length} caracteres ou menos",
    "noItemsSelected": "Por favor acrescente pelo menos um item no calendário",
    "updateSuccess": "Calendário {name} atualizado",
    "updateFailed": "Falha ao atualizar calendário"
  },
  "calendars": {
    "title": "Meus Calendários",
    "createNew": "Criar Novo Calendário",
    "loading": "A carregar calendários...",
    "notFound": "Nenhum calendário encontrado.",
    "created": "Criado",
    "updated": "Atualizado",
    "export": "Exportar",
    "edit": "Editar",
    "delete": "Remover",
    "pagePrevious": "Anterior",
    "pageNext": "Seguinte",
    "paginationText": "A mostrar {first} até {last} de {total} calendários",
    "loadFailed": "Falha ao carregar calendários",
    "deleteFailed": "Falha ao remover calendário",
    "deleteConfirmTitle": "Excluir Calendário",
    "deleteConfirmMessage": "Tem certeza que deseja excluir este calendário? Isso não pode ser desfeito.",
    "exportFailed": "Falha ao exportar o calendário",
    "exportDownload": "Baixar .ics",
    "copyLink": "Copiar link de assinatura",
    "linkCopied": "Link de assinatura copiado para a área de transferência!",
    "openInGoogle": "Adicionar ao Google Agenda"
  },
  "userDetails": {
    "fetchFailed": "Falha ao obter detalhes de usuário",
    "updateSuccess": "Detalhes de usuário atualizados com successo!",
    "updateFailed": "Falha ao atualizar detalhes de usuário",
    "passwordsDontMatch": "Novas palavras pass não correspondem",
    "passwordLength": "Nova palavra pass tem de conter pelo menos {min} caracteres",
    "passwordContent": "Nova palavra pass tem de conter pelo menos uma letra maiúscula, uma letra minúscula, um dígito e um caracter especial",
    "passwordMustBeDifferent": "Nova palavra pass deve ser diferente da palavra pass atual",
    "passwordUpdateSuccess": "Palava pass atualizada com successo!",
    "currentPasswordIncorrect": "Palavra pass incorreta",
    "passwordUpdateFailed": "Falha ao atualizar palavra pass",
    "accountDelete": "Tem a certeza que deseja remover a sua conta? Esta ação não pode ser revertida.",
    "accountDeleteFailed": "Falha ao remover conta",
    "title": "Detalhes de Usuário",
    "loading": "A carregar detalhes de usuário...",
    "name": "Nome",
    "email": "Email",
    "editDetails": "Editar Detalhes",
    "viewSettings": "Ver Configurações",
    "username": "Nome de Usuário",
    "cancel": "Cancelar",
    "update": "Guardar Detalhes",
    "changePassword": "Atualizar Palavra Pass",
    "currentPassword": "Palavra Pass Atual",
    "newPassword": "Palavra Pass Nova",
    "passwordHint": "Palavra pass deve conter pelo menos 12 caracteres com letra maiúscula, letra minúscula, dígito e caracter especial",
    "confirmPassword": "Confirmar Nova Palavra Pass",
    "passwordUpdating": "Atualizando...",
    "passwordUpdate": "Atualizar Palavra Passe",
    "accountDeleteButton": "Remover Conta",
    "accountDeleteWarning": "Remover a sua conta vai remover todos os seus dados. Esta ação não pode ser revertida.",
    "accountDeleting": "Removendo..."
  },
  "errors": {
    "generic": "Ocorreu um erro. Tente novamente."
  }
}
```

- [ ] **Step 2: Verify JSON is valid**

```bash
cd frontend && node -e "JSON.parse(require('fs').readFileSync('src/locales/pt.json','utf8')); console.log('OK')"
```

Expected: `OK`

---

## Task 3: Update `GoogleLoginButton.vue`

**Files:**
- Modify: `frontend/src/components/GoogleLoginButton.vue`

- [ ] **Step 1: Rename key reference**

In `frontend/src/components/GoogleLoginButton.vue`, change:
```ts
error.value = t('login.google.failed')
```
to:
```ts
error.value = t('auth.google.failed')
```

---

## Task 4: Update `LoginPage.vue`

**Files:**
- Modify: `frontend/src/components/LoginPage.vue`

- [ ] **Step 1: Replace all `components.login.` with `auth.login.` and `components.register.` with `auth.register.`**

Apply every rename from the key map above — all occurrences of `components.login.*` → `auth.login.*` and `components.register.*` → `auth.register.*`.

---

## Task 5: Update `Register.vue`

**Files:**
- Modify: `frontend/src/components/Register.vue`

- [ ] **Step 1: Replace all `components.register.` and `components.login.` references**

Apply every rename:
- `components.register.*` → `auth.register.*`
- `components.login.emailPlaceholder` → `auth.login.emailPlaceholder`
- `components.login.passwordPlaceholder` → `auth.login.passwordPlaceholder`
- `components.login.submit` → `auth.login.submit`

---

## Task 6: Update `CalendarPage.vue`

**Files:**
- Modify: `frontend/src/components/CalendarPage.vue`

- [ ] **Step 1: Apply error/success key renames**

| Old | New |
|---|---|
| `calendar.enterName` | (unchanged) |
| `calendar.failedToFetchItems` | `calendar.fetchItemsFailed` |
| `calendar.enterCalendarName` | (unchanged) |
| `calendar.noItemsSelected` | (unchanged) |
| `calendar.updatedFail` | `calendar.updateFailed` |
| `calendar.failedToLoad` | `calendar.loadFailed` |
| `calendar.updatedSuccess` | `calendar.updateSuccess` |

---

## Task 7: Update `UserSettingsPage.vue`

**Files:**
- Modify: `frontend/src/components/UserSettingsPage.vue`

- [ ] **Step 1: Apply error key renames**

| Old | New |
|---|---|
| `userSettings.fetchError` | `userSettings.fetchFailed` |
| `userSettings.updateError` | `userSettings.updateFailed` |

---

## Task 8: Update `UserDetailsPage.vue`

**Files:**
- Modify: `frontend/src/components/UserDetailsPage.vue`

- [ ] **Step 1: Apply error key rename**

| Old | New |
|---|---|
| `userDetails.failedToFetch` | `userDetails.fetchFailed` |

---

## Task 9: Verify

- [ ] **Step 1: Run tests**

```bash
cd frontend && npm run test:unit
```

Expected: all pass

- [ ] **Step 2: Type-check + build**

```bash
cd frontend && npm run build
```

Expected: no errors

- [ ] **Step 3: Grep for any remaining old keys**

```bash
cd frontend && grep -r "components\.login\|components\.register\|login\.google\|updatedFail\|updatedSuccess\b\|fetchError\|updateError\|loadingFailed\|failedToFetch\b\|failedToLoad\|failedToFetchItems" src/
```

Expected: no output
