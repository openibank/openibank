# OpeniBank Go SDK

Official Go client library for the OpeniBank Open Banking Platform API.

## Requirements

- Go 1.21+

## Installation

```bash
go get github.com/openibank/sdk-go
```

## Quick Start

```go
package main

import (
    "context"
    "fmt"
    "log"

    openibank "github.com/openibank/sdk-go"
)

func main() {
    // Create client
    client := openibank.NewClient(
        openibank.WithClientCredentials("your_client_id", "your_client_secret"),
        openibank.WithEnvironment(openibank.Sandbox),
    )

    ctx := context.Background()

    // List accounts
    accounts, err := client.Accounts.List(ctx, nil)
    if err != nil {
        log.Fatal(err)
    }

    for _, account := range accounts {
        fmt.Printf("%s: %s %s\n", account.Name, account.Balance.Amount, account.Balance.Currency)
    }

    // Get transactions
    if len(accounts) > 0 {
        transactions, err := client.Transactions.List(ctx, accounts[0].ID, nil)
        if err != nil {
            log.Fatal(err)
        }

        for _, tx := range transactions {
            fmt.Printf("%s: %s %s\n", tx.Description, tx.Amount, tx.Currency)
        }
    }
}
```

## Project Structure

```
sdk-go/
├── client.go           # Main client implementation
├── config.go           # Configuration options
├── auth.go             # OAuth 2.0 implementation
├── accounts.go         # Accounts API
├── transactions.go     # Transactions API
├── payments.go         # Payments API
├── consents.go         # Consent management
├── institutions.go     # Financial institutions
├── realtime.go         # WebSocket client
├── errors.go           # Error types
├── models.go           # Data models
├── http.go             # HTTP utilities
└── examples/
    ├── basic/
    │   └── main.go
    ├── payments/
    │   └── main.go
    └── realtime/
        └── main.go
```

## Configuration

### Client Options

```go
import openibank "github.com/openibank/sdk-go"

// Create client with options
client := openibank.NewClient(
    openibank.WithClientCredentials("client_id", "client_secret"),
    openibank.WithEnvironment(openibank.Production),
    openibank.WithAPIVersion("v2"),
    openibank.WithTimeout(30 * time.Second),
    openibank.WithMaxRetries(3),
    openibank.WithDebug(true),
)
```

### Environment Variables

```bash
export OPENIBANK_CLIENT_ID="your_client_id"
export OPENIBANK_CLIENT_SECRET="your_client_secret"
export OPENIBANK_ENVIRONMENT="sandbox"
export OPENIBANK_API_VERSION="v2"
```

```go
// Create client from environment
client := openibank.NewClientFromEnv()
```

### Custom HTTP Client

```go
import (
    "net/http"
    openibank "github.com/openibank/sdk-go"
)

httpClient := &http.Client{
    Timeout: 60 * time.Second,
    Transport: &http.Transport{
        MaxIdleConns:        100,
        MaxIdleConnsPerHost: 100,
    },
}

client := openibank.NewClient(
    openibank.WithClientCredentials("client_id", "client_secret"),
    openibank.WithHTTPClient(httpClient),
)
```

## Authentication

### Client Credentials Flow

```go
// Credentials are used to obtain access tokens automatically
client := openibank.NewClient(
    openibank.WithClientCredentials("client_id", "client_secret"),
)

// Access token is obtained and refreshed automatically
accounts, err := client.Accounts.List(ctx, nil)
```

### Authorization Code Flow

```go
// Step 1: Generate authorization URL
authURL := client.Auth.GetAuthorizationURL(
    "https://your-app.com/callback",
    []string{"accounts:read", "transactions:read"},
    "random_state_string",
)
fmt.Printf("Redirect user to: %s\n", authURL)

// Step 2: Handle callback and exchange code
tokens, err := client.Auth.ExchangeCode(ctx, openibank.ExchangeCodeParams{
    Code:        "authorization_code_from_callback",
    RedirectURI: "https://your-app.com/callback",
})
if err != nil {
    log.Fatal(err)
}

// Step 3: Use the access token
client.SetAccessToken(tokens.AccessToken)

// Step 4: Make authorized requests
accounts, err := client.Accounts.List(ctx, nil)
```

### Token Refresh

```go
// Automatic refresh (default)
client := openibank.NewClient(
    openibank.WithClientCredentials("client_id", "client_secret"),
    openibank.WithAutoRefresh(true), // Default
)

// Manual refresh
newTokens, err := client.Auth.RefreshToken(ctx, tokens.RefreshToken)
if err != nil {
    log.Fatal(err)
}
client.SetAccessToken(newTokens.AccessToken)
```

## API Resources

### Accounts

```go
ctx := context.Background()

// List all accounts
accounts, err := client.Accounts.List(ctx, nil)

// List with filters
accounts, err := client.Accounts.List(ctx, &openibank.AccountListParams{
    Status:      openibank.String("active"),
    AccountType: openibank.String("checking"),
    Limit:       openibank.Int(50),
})

// Get single account
account, err := client.Accounts.Get(ctx, "acc_123456")

// Get account balances
balances, err := client.Accounts.GetBalances(ctx, "acc_123456")
```

### Transactions

```go
ctx := context.Background()

// List transactions
transactions, err := client.Transactions.List(ctx, "acc_123456", nil)

// List with filters
transactions, err := client.Transactions.List(ctx, "acc_123456", &openibank.TransactionListParams{
    DateFrom:      openibank.Time(time.Now().AddDate(0, -1, 0)),
    DateTo:        openibank.Time(time.Now()),
    AmountMin:     openibank.Float64(100.0),
    AmountMax:     openibank.Float64(1000.0),
    BookingStatus: openibank.String("booked"),
    Limit:         openibank.Int(50),
})

// Get single transaction
transaction, err := client.Transactions.Get(ctx, "acc_123456", "tx_789")

// Iterate through all transactions
iter := client.Transactions.Iter(ctx, "acc_123456", nil)
for iter.Next() {
    tx := iter.Transaction()
    fmt.Printf("%s: %s %s\n", tx.Description, tx.Amount, tx.Currency)
}
if err := iter.Err(); err != nil {
    log.Fatal(err)
}
```

### Payments

```go
ctx := context.Background()

// Create a payment
payment, err := client.Payments.Create(ctx, openibank.PaymentCreateParams{
    Creditor: openibank.Creditor{
        Name: "John Doe",
        Account: openibank.CreditorAccount{
            IBAN: openibank.String("DE89370400440532013000"),
        },
    },
    Amount: openibank.Amount{
        Amount:   "150.00",
        Currency: "EUR",
    },
    DebtorAccountID: "acc_123456",
    Reference:       openibank.String("Invoice #12345"),
})
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Payment ID: %s\n", payment.ID)
fmt.Printf("Status: %s\n", payment.Status)

// Get payment status
payment, err = client.Payments.Get(ctx, payment.ID)

// List payments
payments, err := client.Payments.List(ctx, &openibank.PaymentListParams{
    Status: openibank.String("pending"),
    Limit:  openibank.Int(20),
})

// Cancel a payment
payment, err = client.Payments.Cancel(ctx, payment.ID)
```

### Consents

```go
ctx := context.Background()

// Create a consent
consent, err := client.Consents.Create(ctx, openibank.ConsentCreateParams{
    Access:             []string{"accounts", "transactions", "balances"},
    ValidUntil:         openibank.String("2024-12-31"),
    RecurringIndicator: openibank.Bool(true),
    FrequencyPerDay:    openibank.Int(4),
})
if err != nil {
    log.Fatal(err)
}

fmt.Printf("Consent ID: %s\n", consent.ID)
fmt.Printf("Authorization URL: %s\n", consent.AuthorizationURL)

// Get consent status
consent, err = client.Consents.Get(ctx, consent.ID)

// Revoke consent
err = client.Consents.Revoke(ctx, consent.ID)

// List all consents
consents, err := client.Consents.List(ctx)
```

### Financial Institutions

```go
ctx := context.Background()

// List all supported institutions
institutions, err := client.Institutions.List(ctx, nil)

// Search institutions
institutions, err := client.Institutions.List(ctx, &openibank.InstitutionListParams{
    Country: openibank.String("DE"),
    Query:   openibank.String("Deutsche"),
})

// Get institution details
institution, err := client.Institutions.Get(ctx, "inst_deutsche_bank")
fmt.Printf("Name: %s\n", institution.Name)
fmt.Printf("BIC: %s\n", institution.BIC)
fmt.Printf("Logo: %s\n", institution.LogoURL)
```

## Real-time WebSocket

```go
ctx := context.Background()

// Define event handlers
handlers := openibank.EventHandlers{
    OnTransactionCreated: func(event openibank.TransactionEvent) {
        fmt.Printf("New transaction: %s\n", event.Data.Description)
        fmt.Printf("Amount: %s %s\n", event.Data.Amount, event.Data.Currency)
    },
    OnBalanceUpdated: func(event openibank.BalanceEvent) {
        fmt.Printf("Balance updated: %s %s\n", event.Data.Amount, event.Data.Currency)
    },
    OnPaymentStatusChanged: func(event openibank.PaymentEvent) {
        fmt.Printf("Payment %s status: %s\n", event.Data.ID, event.Data.Status)
    },
    OnError: func(err error) {
        log.Printf("WebSocket error: %v\n", err)
    },
}

// Subscribe to events
subscription, err := client.Realtime.Subscribe(ctx, openibank.SubscribeParams{
    AccountID: "acc_123456",
    Events: []openibank.EventType{
        openibank.EventTransactionCreated,
        openibank.EventBalanceUpdated,
        openibank.EventPaymentStatusChanged,
    },
    Handlers: handlers,
})
if err != nil {
    log.Fatal(err)
}

// Wait for events (blocking)
err = subscription.Wait()

// Or close manually
subscription.Close()
```

## Error Handling

```go
import (
    "errors"
    openibank "github.com/openibank/sdk-go"
)

account, err := client.Accounts.Get(ctx, "acc_invalid")
if err != nil {
    var authErr *openibank.AuthenticationError
    var authzErr *openibank.AuthorizationError
    var valErr *openibank.ValidationError
    var notFoundErr *openibank.NotFoundError
    var rateLimitErr *openibank.RateLimitError
    var conflictErr *openibank.ConflictError
    var serverErr *openibank.ServerError
    var networkErr *openibank.NetworkError

    switch {
    case errors.As(err, &authErr):
        fmt.Printf("Authentication failed: %s\n", authErr.Message)

    case errors.As(err, &authzErr):
        fmt.Printf("Access denied: %s\n", authzErr.Message)
        fmt.Printf("Required scopes: %v\n", authzErr.RequiredScopes)

    case errors.As(err, &valErr):
        fmt.Printf("Invalid request: %s\n", valErr.Message)
        for _, e := range valErr.Errors {
            fmt.Printf("  - %s: %s\n", e.Field, e.Message)
        }

    case errors.As(err, &notFoundErr):
        fmt.Printf("Resource not found: %s\n", notFoundErr.Message)
        fmt.Printf("Resource type: %s\n", notFoundErr.ResourceType)
        fmt.Printf("Resource ID: %s\n", notFoundErr.ResourceID)

    case errors.As(err, &rateLimitErr):
        fmt.Printf("Rate limited: %s\n", rateLimitErr.Message)
        fmt.Printf("Retry after: %v\n", rateLimitErr.RetryAfter)
        time.Sleep(rateLimitErr.RetryAfter)
        // Retry the request

    case errors.As(err, &conflictErr):
        fmt.Printf("Conflict: %s\n", conflictErr.Message)

    case errors.As(err, &serverErr):
        fmt.Printf("Server error: %s\n", serverErr.Message)
        fmt.Printf("Request ID: %s\n", serverErr.RequestID)

    case errors.As(err, &networkErr):
        fmt.Printf("Network error: %s\n", networkErr.Message)
        // Retry with exponential backoff

    default:
        var apiErr *openibank.Error
        if errors.As(err, &apiErr) {
            fmt.Printf("API error: %s (code: %s)\n", apiErr.Message, apiErr.Code)
        }
    }
}
```

## Pagination

```go
ctx := context.Background()

// Using iterator
iter := client.Transactions.Iter(ctx, "acc_123456", nil)
for iter.Next() {
    tx := iter.Transaction()
    processTransaction(tx)
}
if err := iter.Err(); err != nil {
    log.Fatal(err)
}

// Manual pagination
var allTransactions []openibank.Transaction
offset := 0
limit := 50

for {
    transactions, err := client.Transactions.List(ctx, "acc_123456", &openibank.TransactionListParams{
        Limit:  openibank.Int(limit),
        Offset: openibank.Int(offset),
    })
    if err != nil {
        log.Fatal(err)
    }

    allTransactions = append(allTransactions, transactions...)

    if len(transactions) < limit {
        break
    }
    offset += limit
}
```

## Idempotency

```go
ctx := context.Background()

// Idempotent payment creation
payment, err := client.Payments.Create(ctx, openibank.PaymentCreateParams{
    Creditor: openibank.Creditor{...},
    Amount: openibank.Amount{...},
    DebtorAccountID: "acc_123456",
}, openibank.WithIdempotencyKey("unique_request_id_12345"))

// Retry with same key returns same result
paymentRetry, err := client.Payments.Create(ctx, openibank.PaymentCreateParams{
    Creditor: openibank.Creditor{...},
    Amount: openibank.Amount{...},
    DebtorAccountID: "acc_123456",
}, openibank.WithIdempotencyKey("unique_request_id_12345"))

// payment.ID == paymentRetry.ID
```

## Context and Cancellation

```go
// With timeout
ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
defer cancel()

accounts, err := client.Accounts.List(ctx, nil)
if err != nil {
    if errors.Is(err, context.DeadlineExceeded) {
        log.Println("Request timed out")
    }
}

// With cancellation
ctx, cancel := context.WithCancel(context.Background())

go func() {
    time.Sleep(5 * time.Second)
    cancel() // Cancel after 5 seconds
}()

iter := client.Transactions.Iter(ctx, "acc_123456", nil)
for iter.Next() {
    // Process transactions
}
if err := iter.Err(); err != nil {
    if errors.Is(err, context.Canceled) {
        log.Println("Request was cancelled")
    }
}
```

## Testing

### Using the Sandbox

```go
// Use sandbox environment
client := openibank.NewClient(
    openibank.WithClientCredentials("sandbox_client_id", "sandbox_client_secret"),
    openibank.WithEnvironment(openibank.Sandbox),
)

// Sandbox provides test accounts and data
accounts, err := client.Accounts.List(ctx, nil)
```

### Mocking

```go
// Create mock client
mockClient := &openibank.MockClient{
    AccountsService: &openibank.MockAccountsService{
        ListFunc: func(ctx context.Context, params *openibank.AccountListParams) ([]openibank.Account, error) {
            return []openibank.Account{
                {
                    ID:   "acc_test",
                    Name: "Test Account",
                    IBAN: openibank.String("DE89370400440532013000"),
                    Balance: &openibank.Balance{
                        Amount:   "1000.00",
                        Currency: "EUR",
                    },
                },
            }, nil
        },
    },
}

accounts, err := mockClient.Accounts.List(ctx, nil)
```

## Contributing

See the [SDK Contributing Guide](../README.md#contributing-guidelines) for details.

## License

MIT License - see [LICENSE](../../LICENSE) for details.
