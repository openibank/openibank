# OpeniBank Python SDK

Official Python client library for the OpeniBank Open Banking Platform API.

## Requirements

- Python 3.8+
- `aiohttp` for async HTTP requests
- `pydantic` for data validation
- `websockets` for real-time features (optional)

## Installation

```bash
# Basic installation
pip install openibank

# With WebSocket support
pip install openibank[websocket]

# With all optional dependencies
pip install openibank[all]

# For development
pip install openibank[dev]
```

## Quick Start

```python
import asyncio
from openibank import OpeniBank

async def main():
    # Initialize the client
    client = OpeniBank(
        client_id="your_client_id",
        client_secret="your_client_secret",
        environment="sandbox"  # or "production"
    )

    try:
        # List all accounts
        accounts = await client.accounts.list()

        for account in accounts:
            print(f"Account: {account.name}")
            print(f"  IBAN: {account.iban}")
            print(f"  Balance: {account.balance.amount} {account.balance.currency}")
            print()

        # Get transactions for the first account
        if accounts:
            transactions = await client.transactions.list(
                account_id=accounts[0].id,
                limit=10
            )

            for tx in transactions:
                print(f"Transaction: {tx.description}")
                print(f"  Amount: {tx.amount} {tx.currency}")
                print(f"  Date: {tx.booking_date}")
                print()

    finally:
        await client.close()

if __name__ == "__main__":
    asyncio.run(main())
```

## Project Structure

```
openibank/
├── __init__.py          # Main client and exports
├── client.py            # OpeniBank client implementation
├── config.py            # Configuration management
├── auth/
│   ├── __init__.py
│   ├── oauth.py         # OAuth 2.0 implementation
│   └── token.py         # Token management
├── resources/
│   ├── __init__.py
│   ├── accounts.py      # Accounts API
│   ├── transactions.py  # Transactions API
│   ├── payments.py      # Payments API
│   ├── consents.py      # Consent management
│   └── institutions.py  # Financial institutions
├── models/
│   ├── __init__.py
│   ├── account.py       # Account models
│   ├── transaction.py   # Transaction models
│   ├── payment.py       # Payment models
│   └── common.py        # Shared models
├── realtime/
│   ├── __init__.py
│   └── websocket.py     # WebSocket client
├── exceptions.py        # Custom exceptions
└── utils/
    ├── __init__.py
    ├── http.py          # HTTP utilities
    └── retry.py         # Retry logic
```

## Configuration

### Environment Variables

```bash
export OPENIBANK_CLIENT_ID="your_client_id"
export OPENIBANK_CLIENT_SECRET="your_client_secret"
export OPENIBANK_ENVIRONMENT="sandbox"
export OPENIBANK_API_VERSION="v2"
```

### Client Configuration

```python
from openibank import OpeniBank, Config

config = Config(
    client_id="your_client_id",
    client_secret="your_client_secret",
    environment="sandbox",
    api_version="v2",
    timeout=30.0,
    max_retries=3,
    retry_delay=1.0,
    debug=False,
)

client = OpeniBank(config=config)
```

### Custom HTTP Client

```python
import aiohttp
from openibank import OpeniBank

# Use custom session with proxy
connector = aiohttp.TCPConnector(ssl=False)
session = aiohttp.ClientSession(connector=connector)

client = OpeniBank(
    client_id="your_client_id",
    client_secret="your_client_secret",
    http_session=session,
)
```

## Authentication

### Client Credentials Flow

```python
from openibank import OpeniBank

# Credentials are used to obtain access tokens automatically
client = OpeniBank(
    client_id="your_client_id",
    client_secret="your_client_secret",
)

# Access token is obtained and refreshed automatically
accounts = await client.accounts.list()
```

### Authorization Code Flow

```python
from openibank import OpeniBank

client = OpeniBank(
    client_id="your_client_id",
    client_secret="your_client_secret",
)

# Step 1: Generate authorization URL
auth_url = client.auth.get_authorization_url(
    redirect_uri="https://your-app.com/callback",
    scopes=["accounts:read", "transactions:read", "payments:write"],
    state="random_state_string",
)
print(f"Redirect user to: {auth_url}")

# Step 2: Handle callback and exchange code
tokens = await client.auth.exchange_code(
    code="authorization_code_from_callback",
    redirect_uri="https://your-app.com/callback",
)

# Step 3: Use the access token
client.set_access_token(tokens.access_token)

# Step 4: Make authorized requests
accounts = await client.accounts.list()
```

### Token Refresh

```python
# Automatic refresh (default)
client = OpeniBank(
    client_id="your_client_id",
    client_secret="your_client_secret",
    auto_refresh=True,  # Default
)

# Manual refresh
new_tokens = await client.auth.refresh_token(tokens.refresh_token)
client.set_access_token(new_tokens.access_token)
```

## API Resources

### Accounts

```python
# List all accounts
accounts = await client.accounts.list()

# List with filters
accounts = await client.accounts.list(
    status="active",
    account_type="checking",
)

# Get single account
account = await client.accounts.get("acc_123456")

# Get account balances
balances = await client.accounts.get_balances("acc_123456")

# Get account details
details = await client.accounts.get_details("acc_123456")
```

### Transactions

```python
from datetime import date, timedelta

# List transactions
transactions = await client.transactions.list(
    account_id="acc_123456",
    limit=50,
)

# List with filters
transactions = await client.transactions.list(
    account_id="acc_123456",
    date_from=date.today() - timedelta(days=30),
    date_to=date.today(),
    amount_min=100.00,
    amount_max=1000.00,
    booking_status="booked",
)

# Get single transaction
transaction = await client.transactions.get(
    account_id="acc_123456",
    transaction_id="tx_789",
)

# Paginate through all transactions
async for transaction in client.transactions.iterate(account_id="acc_123456"):
    print(transaction.description)
```

### Payments

```python
from openibank.models import PaymentRequest, Amount, Creditor, CreditorAccount

# Create a payment
payment = await client.payments.create(
    PaymentRequest(
        creditor=Creditor(
            name="John Doe",
            account=CreditorAccount(
                iban="DE89370400440532013000",
            ),
        ),
        amount=Amount(
            amount="150.00",
            currency="EUR",
        ),
        reference="Invoice #12345",
        debtor_account_id="acc_123456",
    )
)

print(f"Payment ID: {payment.id}")
print(f"Status: {payment.status}")

# Get payment status
payment = await client.payments.get(payment.id)

# List payments
payments = await client.payments.list(
    status="pending",
    limit=20,
)

# Cancel a payment (if supported)
await client.payments.cancel(payment.id)
```

### Consents

```python
from openibank.models import ConsentRequest

# Create a consent
consent = await client.consents.create(
    ConsentRequest(
        access=["accounts", "transactions", "balances"],
        valid_until="2024-12-31",
        recurring_indicator=True,
        frequency_per_day=4,
    )
)

print(f"Consent ID: {consent.id}")
print(f"Authorization URL: {consent.authorization_url}")

# Get consent status
consent = await client.consents.get(consent.id)

# Revoke consent
await client.consents.revoke(consent.id)

# List all consents
consents = await client.consents.list()
```

### Financial Institutions

```python
# List all supported institutions
institutions = await client.institutions.list()

# Search institutions
institutions = await client.institutions.list(
    country="DE",
    query="Deutsche",
)

# Get institution details
institution = await client.institutions.get("inst_deutsche_bank")
print(f"Name: {institution.name}")
print(f"BIC: {institution.bic}")
print(f"Logo: {institution.logo_url}")
```

## Real-time WebSocket

```python
from openibank import OpeniBank
from openibank.realtime import EventType

client = OpeniBank(
    client_id="your_client_id",
    client_secret="your_client_secret",
)

# Define event handlers
async def on_transaction(event):
    tx = event.data
    print(f"New transaction: {tx.description}")
    print(f"Amount: {tx.amount} {tx.currency}")

async def on_balance_update(event):
    balance = event.data
    print(f"Balance updated: {balance.available} {balance.currency}")

async def on_payment_status(event):
    payment = event.data
    print(f"Payment {payment.id} status: {payment.status}")

# Subscribe to events
subscription = await client.realtime.subscribe(
    account_id="acc_123456",
    events=[
        EventType.TRANSACTION_CREATED,
        EventType.BALANCE_UPDATED,
        EventType.PAYMENT_STATUS_CHANGED,
    ],
    handlers={
        EventType.TRANSACTION_CREATED: on_transaction,
        EventType.BALANCE_UPDATED: on_balance_update,
        EventType.PAYMENT_STATUS_CHANGED: on_payment_status,
    },
)

# Keep connection alive
await subscription.wait()

# Or manually close
await subscription.close()
```

## Error Handling

```python
from openibank import OpeniBank
from openibank.exceptions import (
    OpeniBankError,
    AuthenticationError,
    AuthorizationError,
    ValidationError,
    NotFoundError,
    RateLimitError,
    ConflictError,
    ServerError,
    NetworkError,
)

client = OpeniBank(
    client_id="your_client_id",
    client_secret="your_client_secret",
)

try:
    account = await client.accounts.get("acc_invalid")

except AuthenticationError as e:
    print(f"Authentication failed: {e.message}")
    # Re-authenticate or check credentials

except AuthorizationError as e:
    print(f"Access denied: {e.message}")
    print(f"Required scopes: {e.required_scopes}")

except ValidationError as e:
    print(f"Invalid request: {e.message}")
    for error in e.errors:
        print(f"  - {error.field}: {error.message}")

except NotFoundError as e:
    print(f"Resource not found: {e.message}")
    print(f"Resource type: {e.resource_type}")
    print(f"Resource ID: {e.resource_id}")

except RateLimitError as e:
    print(f"Rate limited: {e.message}")
    print(f"Retry after: {e.retry_after} seconds")
    await asyncio.sleep(e.retry_after)
    # Retry the request

except ConflictError as e:
    print(f"Conflict: {e.message}")

except ServerError as e:
    print(f"Server error: {e.message}")
    print(f"Request ID: {e.request_id}")

except NetworkError as e:
    print(f"Network error: {e.message}")
    # Retry with exponential backoff

except OpeniBankError as e:
    # Catch-all for any SDK error
    print(f"Error: {e.message}")
    print(f"Code: {e.code}")
```

## Pagination

```python
# Manual pagination
page = await client.transactions.list(
    account_id="acc_123456",
    limit=50,
)

while page.has_next:
    print(f"Processing {len(page.items)} transactions")
    for tx in page.items:
        process_transaction(tx)
    page = await page.next()

# Automatic iteration
async for tx in client.transactions.iterate(account_id="acc_123456"):
    process_transaction(tx)

# Collect all items
all_transactions = await client.transactions.list_all(
    account_id="acc_123456"
)
```

## Idempotency

```python
from openibank.models import PaymentRequest

# Idempotent payment creation
payment = await client.payments.create(
    PaymentRequest(...),
    idempotency_key="unique_request_id_12345",
)

# Retry with same key returns same result
payment_retry = await client.payments.create(
    PaymentRequest(...),
    idempotency_key="unique_request_id_12345",
)

assert payment.id == payment_retry.id
```

## Testing

### Using the Sandbox

```python
from openibank import OpeniBank

# Use sandbox environment
client = OpeniBank(
    client_id="sandbox_client_id",
    client_secret="sandbox_client_secret",
    environment="sandbox",
)

# Sandbox provides test accounts and data
accounts = await client.accounts.list()
```

### Mocking

```python
from unittest.mock import AsyncMock, patch
from openibank import OpeniBank
from openibank.models import Account, Balance

@patch.object(OpeniBank, 'accounts')
async def test_list_accounts(mock_accounts):
    mock_accounts.list = AsyncMock(return_value=[
        Account(
            id="acc_test",
            name="Test Account",
            iban="DE89370400440532013000",
            balance=Balance(amount="1000.00", currency="EUR"),
        )
    ])

    client = OpeniBank(
        client_id="test",
        client_secret="test",
    )

    accounts = await client.accounts.list()
    assert len(accounts) == 1
    assert accounts[0].id == "acc_test"
```

## Logging

```python
import logging

# Enable SDK logging
logging.basicConfig(level=logging.DEBUG)
logger = logging.getLogger("openibank")
logger.setLevel(logging.DEBUG)

# Or configure specific loggers
logging.getLogger("openibank.http").setLevel(logging.DEBUG)
logging.getLogger("openibank.auth").setLevel(logging.INFO)
```

## Contributing

See the [SDK Contributing Guide](../README.md#contributing-guidelines) for details.

## License

MIT License - see [LICENSE](../../LICENSE) for details.
