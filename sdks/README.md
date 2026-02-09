# OpeniBank SDKs

Official client libraries for the OpeniBank Open Banking Platform API.

## Overview

OpeniBank provides official SDKs for multiple programming languages to simplify integration with our Open Banking APIs. These SDKs handle authentication, request signing, error handling, and provide type-safe interfaces for all API operations.

### Key Features

- **Type Safety**: Full type definitions for all API requests and responses
- **Async Support**: Native async/await patterns for optimal performance
- **WebSocket Support**: Real-time transaction and balance notifications
- **Automatic Retry**: Configurable retry logic with exponential backoff
- **Error Handling**: Comprehensive error types with detailed messages
- **Pagination**: Built-in support for paginated endpoints
- **Idempotency**: Automatic idempotency key generation for safe retries

## Supported Languages

| Language | Status | Package | Version |
|----------|--------|---------|---------|
| Python | Stable | `openibank` | 1.0.0 |
| TypeScript/JavaScript | Stable | `@openibank/sdk` | 1.0.0 |
| Go | Beta | `github.com/openibank/sdk-go` | 0.9.0 |
| Java | Planned | `com.openibank:sdk` | - |
| Ruby | Planned | `openibank` | - |
| PHP | Planned | `openibank/sdk` | - |
| .NET | Planned | `OpeniBank.Sdk` | - |

## SDK Roadmap

### Q1 2024
- [x] Python SDK v1.0.0 stable release
- [x] TypeScript SDK v1.0.0 stable release
- [x] Go SDK v0.9.0 beta release

### Q2 2024
- [ ] Go SDK v1.0.0 stable release
- [ ] Java SDK v1.0.0 stable release
- [ ] Ruby SDK v1.0.0 stable release

### Q3 2024
- [ ] PHP SDK v1.0.0 stable release
- [ ] .NET SDK v1.0.0 stable release
- [ ] Mobile SDK (React Native) v1.0.0

### Q4 2024
- [ ] Mobile SDK (Flutter) v1.0.0
- [ ] Mobile SDK (Swift) v1.0.0
- [ ] Mobile SDK (Kotlin) v1.0.0

## API Compatibility Guarantees

### Versioning Policy

All SDKs follow [Semantic Versioning 2.0.0](https://semver.org/):

- **MAJOR** version for incompatible API changes
- **MINOR** version for backwards-compatible functionality additions
- **PATCH** version for backwards-compatible bug fixes

### API Version Support

Each SDK version supports specific API versions:

| SDK Version | API Versions Supported | End of Support |
|-------------|------------------------|----------------|
| 1.x | v1, v2 | December 2025 |
| 0.9.x (beta) | v1 | June 2024 |

### Breaking Change Policy

We commit to the following stability guarantees:

1. **Major Releases**: Breaking changes only in major versions with 6-month deprecation notices
2. **Minor Releases**: New features are opt-in and backwards compatible
3. **Patch Releases**: Bug fixes only, no behavioral changes
4. **Security Updates**: May be released as patches with behavioral changes if required for security

### Deprecation Process

1. Feature marked as deprecated with warning in documentation
2. Deprecation warning added to SDK (compile-time for typed languages)
3. Minimum 6-month period before removal
4. Feature removed in next major version

## Installation

### Python

```bash
pip install openibank

# With WebSocket support
pip install openibank[websocket]

# With all optional dependencies
pip install openibank[all]
```

### TypeScript/JavaScript

```bash
npm install @openibank/sdk

# or with yarn
yarn add @openibank/sdk

# or with pnpm
pnpm add @openibank/sdk
```

### Go

```bash
go get github.com/openibank/sdk-go
```

## Quick Start

### Python

```python
from openibank import OpeniBank

async def main():
    client = OpeniBank(
        client_id="your_client_id",
        client_secret="your_client_secret",
        environment="sandbox"
    )

    # Get accounts
    accounts = await client.accounts.list()
    for account in accounts:
        print(f"{account.name}: {account.balance.amount} {account.balance.currency}")

    await client.close()
```

### TypeScript

```typescript
import { OpeniBank } from '@openibank/sdk';

const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
  environment: 'sandbox',
});

// Get accounts
const accounts = await client.accounts.list();
accounts.forEach((account) => {
  console.log(`${account.name}: ${account.balance.amount} ${account.balance.currency}`);
});
```

### Go

```go
package main

import (
    "context"
    "fmt"
    openibank "github.com/openibank/sdk-go"
)

func main() {
    client := openibank.NewClient(
        openibank.WithClientCredentials("your_client_id", "your_client_secret"),
        openibank.WithEnvironment(openibank.Sandbox),
    )

    accounts, err := client.Accounts.List(context.Background())
    if err != nil {
        panic(err)
    }

    for _, account := range accounts {
        fmt.Printf("%s: %s %s\n", account.Name, account.Balance.Amount, account.Balance.Currency)
    }
}
```

## Authentication

All SDKs support the following authentication methods:

### OAuth 2.0 Client Credentials

For server-to-server integrations:

```python
client = OpeniBank(
    client_id="your_client_id",
    client_secret="your_client_secret"
)
```

### OAuth 2.0 Authorization Code

For user-authorized access:

```python
# Generate authorization URL
auth_url = client.auth.get_authorization_url(
    redirect_uri="https://your-app.com/callback",
    scopes=["accounts:read", "transactions:read"],
    state="random_state_string"
)

# Exchange code for tokens
tokens = await client.auth.exchange_code(
    code="authorization_code",
    redirect_uri="https://your-app.com/callback"
)

# Use access token
client.set_access_token(tokens.access_token)
```

### API Key (Development Only)

For quick testing in sandbox:

```python
client = OpeniBank(
    api_key="your_api_key",
    environment="sandbox"
)
```

## Error Handling

All SDKs provide consistent error types:

| Error Type | Description |
|------------|-------------|
| `AuthenticationError` | Invalid or expired credentials |
| `AuthorizationError` | Insufficient permissions |
| `ValidationError` | Invalid request parameters |
| `NotFoundError` | Resource not found |
| `RateLimitError` | Too many requests |
| `ConflictError` | Resource conflict (e.g., duplicate) |
| `ServerError` | Internal server error |
| `NetworkError` | Connection or timeout issues |

Example error handling:

```python
from openibank import OpeniBank
from openibank.exceptions import ValidationError, RateLimitError, NotFoundError

try:
    account = await client.accounts.get("invalid_id")
except NotFoundError as e:
    print(f"Account not found: {e.message}")
except ValidationError as e:
    print(f"Invalid request: {e.errors}")
except RateLimitError as e:
    print(f"Rate limited. Retry after: {e.retry_after} seconds")
```

## WebSocket Support

All SDKs support real-time notifications via WebSocket:

```python
async def on_transaction(event):
    print(f"New transaction: {event.data.amount} {event.data.currency}")

async def on_balance_change(event):
    print(f"Balance updated: {event.data.available} {event.data.currency}")

# Subscribe to events
await client.realtime.subscribe(
    account_id="acc_123",
    events=["transaction.created", "balance.updated"],
    handlers={
        "transaction.created": on_transaction,
        "balance.updated": on_balance_change,
    }
)
```

## Contributing Guidelines

We welcome contributions to our SDKs! Please follow these guidelines:

### Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes
4. Write or update tests
5. Ensure all tests pass
6. Submit a pull request

### Code Standards

#### All Languages

- Follow language-specific style guides
- Write comprehensive documentation
- Include unit tests for all new functionality
- Maintain backwards compatibility
- Update CHANGELOG.md

#### Python

- Follow PEP 8 style guide
- Use type hints for all functions
- Run `black` for formatting
- Run `mypy` for type checking
- Run `pytest` for testing

#### TypeScript

- Follow ESLint configuration
- Use strict TypeScript settings
- Run `prettier` for formatting
- Run `jest` for testing

#### Go

- Follow `gofmt` standards
- Run `golint` and `go vet`
- Use `go test` for testing

### Pull Request Process

1. Update README.md with any new features
2. Update CHANGELOG.md with your changes
3. Ensure CI passes
4. Request review from maintainers
5. Squash commits before merging

### Testing

All SDKs include:

- **Unit Tests**: Test individual components
- **Integration Tests**: Test against sandbox API
- **E2E Tests**: Full workflow testing

Run tests locally:

```bash
# Python
cd sdks/python
pytest

# TypeScript
cd sdks/typescript
npm test

# Go
cd sdks/go
go test ./...
```

### Reporting Issues

When reporting issues, please include:

1. SDK version
2. Language/runtime version
3. Operating system
4. Minimal reproduction code
5. Expected vs actual behavior
6. Error messages and stack traces

## Support

- **Documentation**: https://docs.openibank.com/sdks
- **API Reference**: https://api.openibank.com/docs
- **GitHub Issues**: https://github.com/openibank/sdks/issues
- **Discord**: https://discord.gg/openibank
- **Email**: support@openibank.com

## License

All OpeniBank SDKs are released under the MIT License. See [LICENSE](LICENSE) for details.
