## Basic features — MVP

Start with a single-user, local-first API client.

### Request builder

* HTTP methods: `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`, `OPTIONS`
* URL input with query-string parsing
* Query parameter key/value editor
* Header key/value editor
* Request body modes:
  * None
  * Raw text
  * JSON
  * `application/x-www-form-urlencoded`
* Send request and cancel an in-progress request
* Request timeout setting
* Redirect handling toggle

### Response viewer

* Status code and status text
* Response duration
* Response size
* Response headers
* Pretty-printed JSON
* Raw text view
* Basic HTML preview
* Copy response body
* Save response body to disk
* Clear error messages for DNS, TLS, timeout, and connection failures

### Request organization

* Save requests locally
* Rename and delete requests
* Collections and folders
* Duplicate request
* Recent-request history
* Search saved requests
* Unsaved-change indicator

### Desktop essentials

* Persistent window size and position
* Dark and light themes
* Keyboard shortcuts:
  * Send request
  * Save request
  * Create request
  * Search
* Native file import/export dialogs
* Basic application settings
* Automatic restoration of the last-opened workspace

---

## Basic-plus features

These make the application useful for regular development work without adding major infrastructure.

### More request body types

* Multipart form data
* File uploads
* Binary request bodies
* GraphQL request body editor
* Automatic `Content-Type` selection

### Authentication

* Basic authentication
* Bearer token
* API key in header or query parameter
* Inherit authentication from collection or folder
* Mask sensitive values in the interface

### Variables and environments

* Global variables
* Environment variables
* Collection variables
* Variable syntax such as `{{baseUrl}}`
* Active environment selector
* Variable autocomplete
* Variable-resolution preview
* Secret variable masking

A useful precedence model is:

```text
request > folder > collection > environment > global
```

### Request tabs

* Multiple open request tabs
* Pin tabs
* Close tabs individually or in groups
* Restore open tabs after restart
* Dirty-tab indicators

### Import and export

* Export your own JSON workspace format
* Import your own workspace format
* Import cURL commands
* Copy request as cURL
* Import a simple Postman Collection format subset
* Drag-and-drop collection files

---

## Medium-effort features

These differentiate the project from a basic HTTP form.

### Code generation

Generate request snippets for:

* cURL
* wget

Start with template-based generation rather than a full code-generation framework.

### Better editors

* JSON syntax highlighting
* JSON validation
* Auto-formatting
* Line numbers
* Search and replace
* Large-response fallback to plain text
* Configurable font size
* Read-only response editor

Monaco Editor or CodeMirror are common choices, but a lightweight editor can reduce initial bundle size.

---

### Command palette

* Open request
* Switch environment
* Send current request
* Format body
* Copy as cURL
* Toggle sidebar
* Open settings

---

## Suggested implementation order

### Phase 1: Functional HTTP client

1. Request builder
2. Native HTTP execution
3. Response viewer
4. Local request persistence
5. Collections and history
6. JSON formatting and validation

### Phase 2: Daily-driver features

1. Environments and variables
2. Authentication
3. Tabs
4. Multipart and file uploads
5. cURL import/export
6. Search and keyboard shortcuts

---

## Out of scope

1. Request scripting
2. API tests
3. Request chaining
4. Cookies
5. Proxy support
6. TLS and certificates
7. Protocol support: GraphQL, WebSocket, gRPC
8. Workspaces
9. User accounts