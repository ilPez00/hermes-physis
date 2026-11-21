# Praxis API Documentation

**Base URL:** `https://praxisweb.xyz/api`  
**Version:** 1.3.0  
**Last Updated:** March 25, 2026

---

## Authentication

Most endpoints require authentication via Bearer token in the `Authorization` header:

```bash
Authorization: Bearer <your_jwt_token>
```

Tokens are obtained via `/auth/login` or `/auth/signup` and should be stored securely.

---

## Health & Monitoring

### GET `/health`

Basic health check for uptime monitoring.

**Response:**

```json
{
  "status": "healthy",
  "timestamp": "2026-03-25T10:30:00.000Z",
  "uptime": 12345.67,
  "version": "1.3.0",
  "environment": "production"
}
```

### GET `/health/ready`

Readiness check - verifies database connectivity.

**Response (200 OK):**

```json
{
  "status": "ready",
  "timestamp": "2026-03-25T10:30:00.000Z",
  "database": "connected"
}
```

**Response (503 Service Unavailable):**

```json
{
  "status": "unhealthy",
  "error": "Database connection failed",
  "details": "Connection timeout"
}
```

### GET `/health/live`

Liveness check - basic process health.

**Response:**

```json
{
  "status": "alive",
  "timestamp": "2026-03-25T10:30:00.000Z",
  "pid": 12345,
  "memory": "256MB"
}
```

---

## Authentication Endpoints

### POST `/auth/signup`

Register a new user account.

**Request:**

```json
{
  "email": "user@example.com",
  "password": "SecurePass123",
  "name": "John Doe"
}
```

**Validation:**

- `email`: Required, valid email format, max 255 chars
- `password`: Required, min 8 chars, must contain letter + number
- `name`: Required, min 2 chars, max 100 chars

**Response (201 Created):**

```json
{
  "message": "User registered successfully. Please check your email for verification.",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "name": "John Doe"
  }
}
```

**Response (400 Bad Request):**

```json
{
  "error": "VALIDATION_ERROR",
  "message": "Invalid input data",
  "details": [{ "field": "email", "message": "Invalid email format" }]
}
```

### POST `/auth/login`

Authenticate user and receive access token.

**Request:**

```json
{
  "email": "user@example.com",
  "password": "SecurePass123"
}
```

**Response (200 OK):**

```json
{
  "message": "Login successful.",
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "name": "John Doe"
  },
  "token": "jwt_token_here"
}
```

---

## Goal Management

### POST `/goals/:userId/node`

Create a new goal node in the user's goal tree.

**Authentication:** Required  
**Rate Limit:** 25 PP cost per creation

**Request:**

```json
{
  "name": "Run a Marathon",
  "domain": "Fitness",
  "description": "Complete a full marathon",
  "completion_metric": "Complete 42.195km race",
  "target_date": "2026-12-31T00:00:00Z",
  "parent_id": "parent-goal-uuid"
}
```

**Validation:**

- `name`: Required, 3-200 chars
- `domain`: Required, one of: Fitness, Career, Learning, Relationships, Finance, Creative, Health, Spiritual, Business, Personal
- `description`: Optional, max 1000 chars
- `completion_metric`: Optional, max 500 chars
- `target_date`: Optional, must be future date (ISO 8601)
- `parent_id`: Optional, valid UUID

**Response (201 Created):**

```json
{
  "id": "goal-uuid",
  "name": "Run a Marathon",
  "domain": "Fitness",
  "progress": 0,
  "created_at": "2026-03-25T10:30:00.000Z"
}
```

### PATCH `/goals/:userId/node/:nodeId/progress`

Update progress on a goal.

**Request:**

```json
{
  "progress": 75
}
```

**Validation:**

- `progress`: Required, number 0-100

---

## Tracker System

### POST `/trackers/log`

Log a tracker entry.

**Authentication:** Required

**Request:**

```json
{
  "tracker_id": "tracker-uuid",
  "data": {
    "items": [{ "name": "Bench Press", "sets": 3, "reps": 10, "weight": 80 }]
  },
  "logged_at": "2026-03-25T10:30:00.000Z"
}
```

**Validation:**

- `tracker_id`: Required, valid UUID
- `data`: Required, non-empty object
- `logged_at`: Optional, ISO 8601 datetime

**Response (201 Created):**

```json
{
  "id": "entry-uuid",
  "tracker_id": "tracker-uuid",
  "data": { ... },
  "logged_at": "2026-03-25T10:30:00.000Z"
}
```

---

## Messaging

### POST `/messages` or `/messages/send`

Send a direct message to another user.

**Authentication:** Required

**Request:**

```json
{
  "receiver_id": "user-uuid",
  "content": "Hey! Want to collaborate on this goal?",
  "message_type": "text"
}
```

**Validation:**

- `receiver_id`: Required, valid UUID
- `content`: Required, 1-5000 chars
- `message_type`: Optional, one of: text, image, voice (default: text)

**Response (201 Created):**

```json
{
  "id": "message-uuid",
  "sender_id": "sender-uuid",
  "receiver_id": "receiver-uuid",
  "content": "Hey! Want to collaborate on this goal?",
  "message_type": "text",
  "created_at": "2026-03-25T10:30:00.000Z"
}
```

---

## Notebook & Journaling

### POST `/notebook/entries`

Create a notebook entry.

**Authentication:** Required

**Request:**

```json
{
  "entry_type": "note",
  "title": "Morning Reflection",
  "content": "Today I'm focusing on #fitness and #career goals",
  "mood": "😊 Good",
  "tags": ["morning", "reflection"],
  "goal_id": "goal-uuid",
  "domain": "Personal",
  "location_lat": 41.90278,
  "location_lng": 12.49636
}
```

**Response (201 Created):**

```json
{
  "id": "entry-uuid",
  "entry_type": "note",
  "title": "Morning Reflection",
  "content": "Today I'm focusing...",
  "occurred_at": "2026-03-25T10:30:00.000Z"
}
```

---

## Error Handling

All endpoints return errors in a consistent format:

### 400 Bad Request (Validation Error)

```json
{
  "error": "VALIDATION_ERROR",
  "message": "Invalid input data",
  "details": [
    { "field": "email", "message": "Invalid email format" },
    { "field": "password", "message": "Password must be at least 8 characters" }
  ]
}
```

### 401 Unauthorized

```json
{
  "error": "UNAUTHORIZED",
  "message": "Authentication required"
}
```

### 403 Forbidden

```json
{
  "error": "FORBIDDEN",
  "message": "Insufficient permissions"
}
```

### 404 Not Found

```json
{
  "error": "NOT_FOUND",
  "message": "Resource not found"
}
```

### 429 Too Many Requests

```json
{
  "error": "RATE_LIMITED",
  "message": "Too many requests. Please try again later.",
  "retryAfter": 60
}
```

### 500 Internal Server Error

```json
{
  "error": "INTERNAL_ERROR",
  "message": "An unexpected error occurred"
}
```

---

## Rate Limiting

API endpoints are rate-limited to prevent abuse:

| Endpoint Type          | Limit        | Window     |
| ---------------------- | ------------ | ---------- |
| Auth endpoints         | 10 requests  | 15 minutes |
| AI endpoints           | 20 requests  | 15 minutes |
| General API            | 100 requests | 15 minutes |
| Strict (sensitive ops) | 5 requests   | 15 minutes |

Rate limit headers are included in responses:

- `X-RateLimit-Limit`: Maximum requests allowed
- `X-RateLimit-Remaining`: Requests remaining
- `X-RateLimit-Reset`: Unix timestamp when limit resets

---

## Best Practices

1. **Store tokens securely** - Use HTTP-only cookies or secure storage
2. **Handle 401 errors** - Refresh tokens or re-authenticate
3. **Respect rate limits** - Implement exponential backoff
4. **Use HTTPS only** - All production traffic must be encrypted
5. **Validate on client** - Provide immediate feedback before API calls

---

## Support

For API issues or questions:

- GitHub Issues: https://github.com/ilPez00/praxis_webapp/issues
- Documentation: https://github.com/ilPez00/praxis_webapp/docs
