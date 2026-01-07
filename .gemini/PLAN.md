# Frontend Migration Plan (Local Tracker)

Due to tooling restrictions preventing GitHub issue creation, this file tracks the tasks for Story #119.

## Story #119: Migrate Frontend to Bun Server (BFF Pattern)

### Tasks

- [x] **Task 1: Setup Bun server and project configuration** (Issue #120 - Created)
  - Create `src/server.ts`
  - Update `package.json` scripts
  - Configure environment variables

- [x] **Task 2: Implement static asset bundling and serving with Bun**
  - *Decision: Keep Vite for building (to support plugins), use Bun for serving.*
  - Update `server.ts` to serve `dist/` and handle SPA fallback
  - (Pending: User needs to run `bun run build`)

- [x] **Task 3: Implement BFF Proxy Layer with Secret Injection**
  - Create `/api/auth` proxy endpoints in `server.ts`
  - Inject `CLIENT_ID` and `CLIENT_SECRET` on the server
  - Handle CORS/Headers

- [x] **Task 4: Update frontend to use BFF endpoints**
  - Updated `src/lib/api.ts` to use `/api` and remove client-side signing
  - Updated `server.ts` to handle Request Signing and App Token injection

## Story #140: Enable Full Observability (Logs, Metrics, Traces)

### Tasks

- [x] **Task 1: Standardize Logging in secure-frontend** (#141)
  - **Goal:** Enable structured JSON logging to match auth-service.
  - **AC:**
    - `secure-frontend` uses `tracing-subscriber` with JSON formatter.
    - Logs include `level`, `message`, `timestamp`, `request_id`.

- [x] **Task 2: Configure Promtail for Universal Log Parsing**
  - **Goal:** Update `promtail.yaml` to parse JSON from both services correctly.
  - **AC:**
    - Promtail pipeline handles `message` (Rust standard) field.
    - `request_id` is extracted and available for correlation (tracing).
    - Labels `level` and `service` are correctly applied.

- [x] **Task 3: Verify Metrics & Dashboards**
  - **Goal:** Ensure Grafana visualizes data from both services.
  - **AC:**
    - Prometheus scrapes `secure-frontend` and `auth-service` successfully.
    - Grafana dashboard shows logs from both services.
