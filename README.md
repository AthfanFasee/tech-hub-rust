# TechHub

A Rust-idiomatic, production-grade platform powered by Actix Web and PostgreSQL. Built with TDD and comprehensive
end-to-end black-box tests.

TechHub offers post creation with full-text search and pagination, comments, likes, and admin-driven newsletters powered
by background workers for async and idempotent email delivery.

Implements hexagonal architecture and domain modeling with compile-time validation via Rust's type system. Features a
full authentication system including user registration, login, transactional email confirmation, secure password reset,
and token-based session authentication.

## 🚀 Features

- **Authentication System**: User registration, email confirmation, password reset
- **Content Management**: Posts with full-text search, comments, and likes
- **Newsletter System**: Admin-driven newsletters with background email delivery
- **Production Ready**: Tracing, metrics, error handling, and Docker deployment
- **Security Focused**: Property-based testing and fuzzing for critical paths

## 🛠 Tech Stack

- **Backend**: Rust, Actix Web, Tokio
- **Database**: PostgreSQL with full-text search
- **Cache**: Redis
- **Testing**: Property-based tests, end-to-end tests, fuzzing
- **Deployment**: Docker, AWS ECS, RDS, ElastiCache
- **Monitoring**: Tracing, metrics, structured logging

## 📦 Quick Start

### Prerequisites

- Rust 1.90+
- Docker & Docker Compose
- PostgreSQL 15+
- Redis 7+

### Run from Scratch

```bash
# Clone and setup
git clone <repository>
cd techhub

# Start all services (PostgreSQL, Redis, Migrations)
make run-scratch

# The API will be available at http://localhost:8000