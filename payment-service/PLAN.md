# Payment Service Implementation Plan

## Overview
This document tracks the development of the `payment-service` based on GitHub Epics and Stories.

## Epics

### 1. Payment Service Foundation (Epic #234)
**Overview:** Initialize the payment-service with standard architecture (Axum, MongoDB, service-core).

#### Stories
*   **Story: Service Skeleton & Configuration** (Story #235)
    *   [ ] **Task #236:** Initialize new Cargo project (`payment-service`) with dependencies (`axum`, `tokio`, `mongodb`, `service-core`).
*   **Story: Database Layer & Models** (Story #240)
    *   **Goal:** Store transaction data in MongoDB.
    *   **Acceptance Criteria:**
        *   [ ] MongoDB connection established on startup.
        *   [ ] `Transaction` and `PaymentMethod` models defined with Serde.
        *   [ ] Repository layer implemented for CRUD operations.
*   **Story: Observability & Security Middleware**
    *   *Pending breakdown*

### 2. Payment Processing (Epic #247)
**Overview:** Core payment flow integration (e.g., Razorpay).

#### Stories
*   **Story: Order Creation API** (Story #248)
    *   **Goal:** Allow clients to create payment orders.
    *   **Acceptance Criteria:**
        *   [ ] `POST /payments/orders` accepts amount and currency.
        *   [ ] Calls Razorpay API to create an order.
        *   [ ] Stores local transaction record with status `CREATED`.
        *   [ ] Returns Razorpay `order_id` to client.

### 3. Transaction History & API (Epic #263)
**Overview:** APIs for other services to query payment status and history.

#### Stories
*   **Story: Transaction History API**
    *   *Pending breakdown*
*   **Story: Idempotency**
    *   *Pending breakdown*

## Backlog / Cleanup
*   **#211:** Task: Initialize new Cargo project (Duplicate of #236) - *Close as duplicate*
*   **#230:** Epic: Transaction History & API (Duplicate of #263) - *Close as duplicate*
*   **#57:** Create service account schema and registration (Task) - *Check relevance to payment-service*
