# OpeniBank TypeScript SDK

Official TypeScript/JavaScript client library for the OpeniBank Open Banking Platform API.

## Requirements

- Node.js 16+ or modern browser with ES2020 support
- TypeScript 4.7+ (for TypeScript users)

## Installation

```bash
# npm
npm install @openibank/sdk

# yarn
yarn add @openibank/sdk

# pnpm
pnpm add @openibank/sdk
```

## Quick Start

```typescript
import { OpeniBank } from '@openibank/sdk';

const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
  environment: 'sandbox',
});

// List all accounts
const accounts = await client.accounts.list();

accounts.forEach((account) => {
  console.log(`${account.name}: ${account.balance?.amount} ${account.balance?.currency}`);
});

// Get transactions
const transactions = await client.transactions.list({
  accountId: accounts[0].id,
  limit: 10,
});

transactions.forEach((tx) => {
  console.log(`${tx.description}: ${tx.amount} ${tx.currency}`);
});
```

## Project Structure

```
@openibank/sdk/
├── src/
│   ├── index.ts           # Main exports
│   ├── client.ts          # OpeniBank client
│   ├── config.ts          # Configuration types
│   ├── auth/
│   │   ├── index.ts
│   │   ├── oauth.ts       # OAuth 2.0 implementation
│   │   └── token.ts       # Token management
│   ├── resources/
│   │   ├── index.ts
│   │   ├── accounts.ts    # Accounts API
│   │   ├── transactions.ts # Transactions API
│   │   ├── payments.ts    # Payments API
│   │   ├── consents.ts    # Consent management
│   │   └── institutions.ts # Financial institutions
│   ├── types/
│   │   ├── index.ts
│   │   ├── account.ts     # Account types
│   │   ├── transaction.ts # Transaction types
│   │   ├── payment.ts     # Payment types
│   │   └── common.ts      # Shared types
│   ├── realtime/
│   │   ├── index.ts
│   │   └── websocket.ts   # WebSocket client
│   ├── errors.ts          # Custom error classes
│   └── utils/
│       ├── http.ts        # HTTP utilities
│       └── retry.ts       # Retry logic
├── dist/                  # Compiled output
├── package.json
├── tsconfig.json
└── README.md
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

```typescript
import { OpeniBank, Config } from '@openibank/sdk';

const config: Config = {
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
  environment: 'sandbox',
  apiVersion: 'v2',
  timeout: 30000,
  maxRetries: 3,
  retryDelay: 1000,
  debug: false,
};

const client = new OpeniBank(config);
```

### Custom HTTP Client

```typescript
import { OpeniBank } from '@openibank/sdk';

// Use custom fetch implementation
const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
  fetch: customFetchFunction,
});
```

## Authentication

### Client Credentials Flow

```typescript
import { OpeniBank } from '@openibank/sdk';

// Credentials are used to obtain access tokens automatically
const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
});

// Access token is obtained and refreshed automatically
const accounts = await client.accounts.list();
```

### Authorization Code Flow

```typescript
import { OpeniBank } from '@openibank/sdk';

const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
});

// Step 1: Generate authorization URL
const authUrl = client.auth.getAuthorizationUrl({
  redirectUri: 'https://your-app.com/callback',
  scopes: ['accounts:read', 'transactions:read', 'payments:write'],
  state: 'random_state_string',
});
console.log(`Redirect user to: ${authUrl}`);

// Step 2: Handle callback and exchange code
const tokens = await client.auth.exchangeCode({
  code: 'authorization_code_from_callback',
  redirectUri: 'https://your-app.com/callback',
});

// Step 3: Use the access token
client.setAccessToken(tokens.accessToken);

// Step 4: Make authorized requests
const accounts = await client.accounts.list();
```

### Token Refresh

```typescript
// Automatic refresh (default)
const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
  autoRefresh: true, // Default
});

// Manual refresh
const newTokens = await client.auth.refreshToken(tokens.refreshToken);
client.setAccessToken(newTokens.accessToken);
```

## API Resources

### Accounts

```typescript
// List all accounts
const accounts = await client.accounts.list();

// List with filters
const filteredAccounts = await client.accounts.list({
  status: 'active',
  accountType: 'checking',
});

// Get single account
const account = await client.accounts.get('acc_123456');

// Get account balances
const balances = await client.accounts.getBalances('acc_123456');

// Get account details
const details = await client.accounts.getDetails('acc_123456');
```

### Transactions

```typescript
// List transactions
const transactions = await client.transactions.list({
  accountId: 'acc_123456',
  limit: 50,
});

// List with filters
const filteredTx = await client.transactions.list({
  accountId: 'acc_123456',
  dateFrom: new Date('2024-01-01'),
  dateTo: new Date('2024-01-31'),
  amountMin: 100.0,
  amountMax: 1000.0,
  bookingStatus: 'booked',
});

// Get single transaction
const transaction = await client.transactions.get({
  accountId: 'acc_123456',
  transactionId: 'tx_789',
});

// Iterate through all transactions
for await (const tx of client.transactions.iterate({ accountId: 'acc_123456' })) {
  console.log(tx.description);
}
```

### Payments

```typescript
// Create a payment
const payment = await client.payments.create({
  creditor: {
    name: 'John Doe',
    account: {
      iban: 'DE89370400440532013000',
    },
  },
  amount: {
    amount: '150.00',
    currency: 'EUR',
  },
  reference: 'Invoice #12345',
  debtorAccountId: 'acc_123456',
});

console.log(`Payment ID: ${payment.id}`);
console.log(`Status: ${payment.status}`);

// Get payment status
const status = await client.payments.get(payment.id);

// List payments
const payments = await client.payments.list({
  status: 'pending',
  limit: 20,
});

// Cancel a payment
await client.payments.cancel(payment.id);
```

### Consents

```typescript
// Create a consent
const consent = await client.consents.create({
  access: ['accounts', 'transactions', 'balances'],
  validUntil: '2024-12-31',
  recurringIndicator: true,
  frequencyPerDay: 4,
});

console.log(`Consent ID: ${consent.id}`);
console.log(`Authorization URL: ${consent.authorizationUrl}`);

// Get consent status
const consentStatus = await client.consents.get(consent.id);

// Revoke consent
await client.consents.revoke(consent.id);

// List all consents
const consents = await client.consents.list();
```

### Financial Institutions

```typescript
// List all supported institutions
const institutions = await client.institutions.list();

// Search institutions
const germanBanks = await client.institutions.list({
  country: 'DE',
  query: 'Deutsche',
});

// Get institution details
const institution = await client.institutions.get('inst_deutsche_bank');
console.log(`Name: ${institution.name}`);
console.log(`BIC: ${institution.bic}`);
console.log(`Logo: ${institution.logoUrl}`);
```

## Real-time WebSocket

```typescript
import { OpeniBank, EventType } from '@openibank/sdk';

const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
});

// Subscribe to events
const subscription = await client.realtime.subscribe({
  accountId: 'acc_123456',
  events: [
    EventType.TransactionCreated,
    EventType.BalanceUpdated,
    EventType.PaymentStatusChanged,
  ],
  handlers: {
    [EventType.TransactionCreated]: (event) => {
      console.log(`New transaction: ${event.data.description}`);
      console.log(`Amount: ${event.data.amount} ${event.data.currency}`);
    },
    [EventType.BalanceUpdated]: (event) => {
      console.log(`Balance updated: ${event.data.available} ${event.data.currency}`);
    },
    [EventType.PaymentStatusChanged]: (event) => {
      console.log(`Payment ${event.data.id} status: ${event.data.status}`);
    },
  },
});

// Keep connection alive
await subscription.wait();

// Or manually close
await subscription.close();
```

## Error Handling

```typescript
import {
  OpeniBank,
  OpeniBankError,
  AuthenticationError,
  AuthorizationError,
  ValidationError,
  NotFoundError,
  RateLimitError,
  ConflictError,
  ServerError,
  NetworkError,
} from '@openibank/sdk';

const client = new OpeniBank({
  clientId: 'your_client_id',
  clientSecret: 'your_client_secret',
});

try {
  const account = await client.accounts.get('acc_invalid');
} catch (error) {
  if (error instanceof AuthenticationError) {
    console.error(`Authentication failed: ${error.message}`);
    // Re-authenticate or check credentials
  } else if (error instanceof AuthorizationError) {
    console.error(`Access denied: ${error.message}`);
    console.error(`Required scopes: ${error.requiredScopes}`);
  } else if (error instanceof ValidationError) {
    console.error(`Invalid request: ${error.message}`);
    error.errors.forEach((e) => {
      console.error(`  - ${e.field}: ${e.message}`);
    });
  } else if (error instanceof NotFoundError) {
    console.error(`Resource not found: ${error.message}`);
    console.error(`Resource type: ${error.resourceType}`);
    console.error(`Resource ID: ${error.resourceId}`);
  } else if (error instanceof RateLimitError) {
    console.error(`Rate limited: ${error.message}`);
    console.error(`Retry after: ${error.retryAfter} seconds`);
    await sleep(error.retryAfter * 1000);
    // Retry the request
  } else if (error instanceof ConflictError) {
    console.error(`Conflict: ${error.message}`);
  } else if (error instanceof ServerError) {
    console.error(`Server error: ${error.message}`);
    console.error(`Request ID: ${error.requestId}`);
  } else if (error instanceof NetworkError) {
    console.error(`Network error: ${error.message}`);
    // Retry with exponential backoff
  } else if (error instanceof OpeniBankError) {
    // Catch-all for any SDK error
    console.error(`Error: ${error.message}`);
    console.error(`Code: ${error.code}`);
  }
}
```

## Pagination

```typescript
// Manual pagination
let page = await client.transactions.listPaginated({
  accountId: 'acc_123456',
  limit: 50,
});

while (page.hasNext) {
  console.log(`Processing ${page.items.length} transactions`);
  page.items.forEach((tx) => processTransaction(tx));
  page = await page.next();
}

// Automatic iteration
for await (const tx of client.transactions.iterate({ accountId: 'acc_123456' })) {
  processTransaction(tx);
}

// Collect all items
const allTransactions = await client.transactions.listAll({
  accountId: 'acc_123456',
});
```

## Idempotency

```typescript
// Idempotent payment creation
const payment = await client.payments.create(
  {
    creditor: { ... },
    amount: { ... },
    debtorAccountId: 'acc_123456',
  },
  { idempotencyKey: 'unique_request_id_12345' }
);

// Retry with same key returns same result
const paymentRetry = await client.payments.create(
  {
    creditor: { ... },
    amount: { ... },
    debtorAccountId: 'acc_123456',
  },
  { idempotencyKey: 'unique_request_id_12345' }
);

console.assert(payment.id === paymentRetry.id);
```

## Browser Usage

```typescript
import { OpeniBank } from '@openibank/sdk';

// Browser-compatible configuration
const client = new OpeniBank({
  clientId: 'your_client_id',
  // Don't expose client_secret in browser!
  // Use authorization code flow instead
  environment: 'sandbox',
});

// Use authorization code flow for browser apps
const authUrl = client.auth.getAuthorizationUrl({
  redirectUri: window.location.origin + '/callback',
  scopes: ['accounts:read', 'transactions:read'],
  state: crypto.randomUUID(),
});

// Redirect to authUrl for user authorization
window.location.href = authUrl;
```

## Testing

### Using the Sandbox

```typescript
import { OpeniBank } from '@openibank/sdk';

// Use sandbox environment
const client = new OpeniBank({
  clientId: 'sandbox_client_id',
  clientSecret: 'sandbox_client_secret',
  environment: 'sandbox',
});

// Sandbox provides test accounts and data
const accounts = await client.accounts.list();
```

### Mocking

```typescript
import { OpeniBank, Account, Balance } from '@openibank/sdk';
import { jest } from '@jest/globals';

// Mock the accounts resource
jest.spyOn(client.accounts, 'list').mockResolvedValue([
  {
    id: 'acc_test',
    name: 'Test Account',
    iban: 'DE89370400440532013000',
    balance: {
      amount: '1000.00',
      currency: 'EUR',
    },
  },
]);

const accounts = await client.accounts.list();
expect(accounts.length).toBe(1);
expect(accounts[0].id).toBe('acc_test');
```

## Contributing

See the [SDK Contributing Guide](../README.md#contributing-guidelines) for details.

## License

MIT License - see [LICENSE](../../LICENSE) for details.
