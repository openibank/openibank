/**
 * OpeniBank TypeScript SDK
 *
 * Official TypeScript client library for the OpeniBank Open Banking Platform API.
 *
 * @example
 * ```typescript
 * import { OpeniBank } from '@openibank/sdk';
 *
 * const client = new OpeniBank({
 *   clientId: 'your_client_id',
 *   clientSecret: 'your_client_secret',
 *   environment: 'sandbox'
 * });
 *
 * const accounts = await client.accounts.list();
 * ```
 *
 * @packageDocumentation
 */

// =============================================================================
// Version
// =============================================================================

export const VERSION = '1.0.0';

// =============================================================================
// Types & Interfaces
// =============================================================================

/**
 * API environment
 */
export type Environment = 'sandbox' | 'production';

/**
 * SDK configuration options
 */
export interface Config {
  /** OAuth client ID */
  clientId: string;
  /** OAuth client secret (server-side only) */
  clientSecret?: string;
  /** API key for sandbox testing */
  apiKey?: string;
  /** API environment */
  environment?: Environment;
  /** API version */
  apiVersion?: string;
  /** Request timeout in milliseconds */
  timeout?: number;
  /** Maximum number of retry attempts */
  maxRetries?: number;
  /** Delay between retries in milliseconds */
  retryDelay?: number;
  /** Automatically refresh tokens */
  autoRefresh?: boolean;
  /** Enable debug logging */
  debug?: boolean;
  /** Custom fetch implementation */
  fetch?: typeof fetch;
}

/**
 * Monetary amount with currency
 */
export interface Amount {
  /** Amount as string for precision */
  amount: string;
  /** ISO 4217 currency code */
  currency: string;
}

/**
 * Account balance
 */
export interface Balance {
  /** Balance amount */
  amount: string;
  /** ISO 4217 currency code */
  currency: string;
  /** Balance type (available, booked, etc.) */
  type?: string;
  /** Credit limit if applicable */
  creditLimit?: string;
  /** Last update timestamp */
  lastUpdated?: Date;
}

/**
 * Bank account
 */
export interface Account {
  /** Unique account identifier */
  id: string;
  /** Account display name */
  name: string;
  /** IBAN (International Bank Account Number) */
  iban?: string;
  /** BBAN (Basic Bank Account Number) */
  bban?: string;
  /** Account currency */
  currency: string;
  /** Account type (checking, savings, etc.) */
  accountType: string;
  /** Account status */
  status: string;
  /** Current balance */
  balance?: Balance;
  /** Financial institution ID */
  institutionId?: string;
  /** Account owner name */
  ownerName?: string;
  /** Creation timestamp */
  createdAt?: Date;
  /** Last update timestamp */
  updatedAt?: Date;
}

/**
 * Bank transaction
 */
export interface Transaction {
  /** Unique transaction identifier */
  id: string;
  /** Associated account ID */
  accountId: string;
  /** Transaction amount */
  amount: string;
  /** Transaction currency */
  currency: string;
  /** Transaction description */
  description: string;
  /** Payment reference */
  reference?: string;
  /** Booking date */
  bookingDate?: Date;
  /** Value date */
  valueDate?: Date;
  /** Transaction type */
  transactionType: string;
  /** Transaction status */
  status: string;
  /** Counterparty name */
  counterpartyName?: string;
  /** Counterparty IBAN */
  counterpartyIban?: string;
  /** Transaction category */
  category?: string;
  /** Additional metadata */
  metadata?: Record<string, unknown>;
}

/**
 * Creditor account for payments
 */
export interface CreditorAccount {
  /** IBAN */
  iban?: string;
  /** BBAN */
  bban?: string;
  /** UK sort code */
  sortCode?: string;
  /** Account number */
  accountNumber?: string;
}

/**
 * Payment creditor
 */
export interface Creditor {
  /** Creditor name */
  name: string;
  /** Creditor account */
  account: CreditorAccount;
}

/**
 * Payment request
 */
export interface PaymentRequest {
  /** Payment creditor */
  creditor: Creditor;
  /** Payment amount */
  amount: Amount;
  /** Debtor account ID */
  debtorAccountId: string;
  /** Payment reference */
  reference?: string;
  /** End-to-end ID */
  endToEndId?: string;
  /** Requested execution date */
  executionDate?: Date;
}

/**
 * Payment status and details
 */
export interface Payment {
  /** Unique payment identifier */
  id: string;
  /** Payment status */
  status: string;
  /** Payment amount */
  amount: string;
  /** Payment currency */
  currency: string;
  /** Creditor name */
  creditorName: string;
  /** Creditor IBAN */
  creditorIban?: string;
  /** Payment reference */
  reference?: string;
  /** Creation timestamp */
  createdAt?: Date;
  /** Execution timestamp */
  executedAt?: Date;
}

/**
 * Consent request
 */
export interface ConsentRequest {
  /** Access permissions requested */
  access: string[];
  /** Consent validity end date */
  validUntil?: string;
  /** Whether consent is recurring */
  recurringIndicator?: boolean;
  /** Maximum API calls per day */
  frequencyPerDay?: number;
}

/**
 * Consent status and details
 */
export interface Consent {
  /** Unique consent identifier */
  id: string;
  /** Consent status */
  status: string;
  /** Granted access permissions */
  access: string[];
  /** Consent validity end date */
  validUntil?: Date;
  /** Authorization URL for user redirect */
  authorizationUrl?: string;
  /** Creation timestamp */
  createdAt?: Date;
}

/**
 * Financial institution
 */
export interface Institution {
  /** Unique institution identifier */
  id: string;
  /** Institution name */
  name: string;
  /** BIC/SWIFT code */
  bic?: string;
  /** Country code */
  country: string;
  /** Institution logo URL */
  logoUrl?: string;
  /** Supported API features */
  supportedFeatures: string[];
}

/**
 * OAuth token response
 */
export interface TokenResponse {
  /** Access token */
  accessToken: string;
  /** Token type */
  tokenType: string;
  /** Token expiry in seconds */
  expiresIn: number;
  /** Refresh token */
  refreshToken?: string;
  /** Granted scopes */
  scope?: string;
}

/**
 * Paginated response
 */
export interface Page<T> {
  /** Page items */
  items: T[];
  /** Total count */
  total: number;
  /** Page size limit */
  limit: number;
  /** Page offset */
  offset: number;
  /** Has next page */
  hasNext: boolean;
  /** Has previous page */
  hasPrev: boolean;
  /** Get next page */
  next: () => Promise<Page<T>>;
  /** Get previous page */
  prev: () => Promise<Page<T>>;
}

/**
 * Real-time event types
 */
export enum EventType {
  TransactionCreated = 'transaction.created',
  TransactionUpdated = 'transaction.updated',
  BalanceUpdated = 'balance.updated',
  PaymentStatusChanged = 'payment.status_changed',
  ConsentRevoked = 'consent.revoked',
}

/**
 * Real-time event
 */
export interface RealtimeEvent<T = unknown> {
  /** Event type */
  type: EventType;
  /** Event data */
  data: T;
  /** Event timestamp */
  timestamp: Date;
}

/**
 * Request options
 */
export interface RequestOptions {
  /** Idempotency key for safe retries */
  idempotencyKey?: string;
  /** Request timeout override */
  timeout?: number;
  /** Additional headers */
  headers?: Record<string, string>;
}

// =============================================================================
// Errors
// =============================================================================

/**
 * Field validation error
 */
export interface FieldError {
  /** Field name */
  field: string;
  /** Error message */
  message: string;
  /** Error code */
  code?: string;
}

/**
 * Base error for all OpeniBank errors
 */
export class OpeniBankError extends Error {
  /** Error code */
  code?: string;
  /** HTTP status code */
  statusCode?: number;
  /** Request ID for support */
  requestId?: string;
  /** Additional details */
  details?: Record<string, unknown>;

  constructor(
    message: string,
    options?: {
      code?: string;
      statusCode?: number;
      requestId?: string;
      details?: Record<string, unknown>;
    }
  ) {
    super(message);
    this.name = 'OpeniBankError';
    this.code = options?.code;
    this.statusCode = options?.statusCode;
    this.requestId = options?.requestId;
    this.details = options?.details;
  }
}

/**
 * Authentication failed - invalid or expired credentials
 */
export class AuthenticationError extends OpeniBankError {
  constructor(message: string, options?: ConstructorParameters<typeof OpeniBankError>[1]) {
    super(message, options);
    this.name = 'AuthenticationError';
  }
}

/**
 * Authorization failed - insufficient permissions
 */
export class AuthorizationError extends OpeniBankError {
  /** Required scopes for the operation */
  requiredScopes?: string[];

  constructor(
    message: string,
    options?: ConstructorParameters<typeof OpeniBankError>[1] & { requiredScopes?: string[] }
  ) {
    super(message, options);
    this.name = 'AuthorizationError';
    this.requiredScopes = options?.requiredScopes;
  }
}

/**
 * Request validation failed
 */
export class ValidationError extends OpeniBankError {
  /** Field-level errors */
  errors: FieldError[];

  constructor(
    message: string,
    options?: ConstructorParameters<typeof OpeniBankError>[1] & { errors?: FieldError[] }
  ) {
    super(message, options);
    this.name = 'ValidationError';
    this.errors = options?.errors ?? [];
  }
}

/**
 * Resource not found
 */
export class NotFoundError extends OpeniBankError {
  /** Resource type */
  resourceType?: string;
  /** Resource ID */
  resourceId?: string;

  constructor(
    message: string,
    options?: ConstructorParameters<typeof OpeniBankError>[1] & {
      resourceType?: string;
      resourceId?: string;
    }
  ) {
    super(message, options);
    this.name = 'NotFoundError';
    this.resourceType = options?.resourceType;
    this.resourceId = options?.resourceId;
  }
}

/**
 * Rate limit exceeded
 */
export class RateLimitError extends OpeniBankError {
  /** Seconds to wait before retry */
  retryAfter: number;

  constructor(
    message: string,
    options?: ConstructorParameters<typeof OpeniBankError>[1] & { retryAfter?: number }
  ) {
    super(message, options);
    this.name = 'RateLimitError';
    this.retryAfter = options?.retryAfter ?? 60;
  }
}

/**
 * Resource conflict (e.g., duplicate)
 */
export class ConflictError extends OpeniBankError {
  constructor(message: string, options?: ConstructorParameters<typeof OpeniBankError>[1]) {
    super(message, options);
    this.name = 'ConflictError';
  }
}

/**
 * Internal server error
 */
export class ServerError extends OpeniBankError {
  constructor(message: string, options?: ConstructorParameters<typeof OpeniBankError>[1]) {
    super(message, options);
    this.name = 'ServerError';
  }
}

/**
 * Network or connection error
 */
export class NetworkError extends OpeniBankError {
  constructor(message: string, options?: ConstructorParameters<typeof OpeniBankError>[1]) {
    super(message, options);
    this.name = 'NetworkError';
  }
}

// =============================================================================
// HTTP Client
// =============================================================================

/**
 * Internal HTTP client with retry logic
 */
class HTTPClient {
  private config: Required<Config>;
  private accessToken?: string;
  private tokenExpiresAt: number = 0;
  private fetchFn: typeof fetch;

  constructor(config: Config) {
    this.config = {
      clientId: config.clientId,
      clientSecret: config.clientSecret ?? '',
      apiKey: config.apiKey ?? '',
      environment: config.environment ?? 'sandbox',
      apiVersion: config.apiVersion ?? 'v2',
      timeout: config.timeout ?? 30000,
      maxRetries: config.maxRetries ?? 3,
      retryDelay: config.retryDelay ?? 1000,
      autoRefresh: config.autoRefresh ?? true,
      debug: config.debug ?? false,
      fetch: config.fetch ?? globalThis.fetch.bind(globalThis),
    };
    this.fetchFn = this.config.fetch;
  }

  get baseUrl(): string {
    return this.config.environment === 'production'
      ? 'https://api.openibank.com'
      : 'https://sandbox.openibank.com';
  }

  get websocketUrl(): string {
    return this.config.environment === 'production'
      ? 'wss://ws.openibank.com'
      : 'wss://ws.sandbox.openibank.com';
  }

  getConfig(): Required<Config> {
    return this.config;
  }

  setAccessToken(token: string, expiresIn: number = 3600): void {
    this.accessToken = token;
    this.tokenExpiresAt = Date.now() + (expiresIn - 60) * 1000; // 60s buffer
  }

  private async ensureToken(): Promise<string> {
    if (this.accessToken && Date.now() < this.tokenExpiresAt) {
      return this.accessToken;
    }

    // Use API key if available
    if (this.config.apiKey) {
      return this.config.apiKey;
    }

    // Get new token using client credentials
    if (this.config.clientId && this.config.clientSecret) {
      const tokenResponse = await this.requestToken();
      this.setAccessToken(tokenResponse.accessToken, tokenResponse.expiresIn);
      return this.accessToken!;
    }

    throw new AuthenticationError('No valid credentials configured');
  }

  private async requestToken(): Promise<TokenResponse> {
    const url = `${this.baseUrl}/oauth/token`;

    const response = await this.fetchFn(url, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: new URLSearchParams({
        grant_type: 'client_credentials',
        client_id: this.config.clientId,
        client_secret: this.config.clientSecret,
      }),
    });

    if (!response.ok) {
      throw new AuthenticationError(`Failed to obtain access token: ${response.status}`);
    }

    const data = await response.json();
    return {
      accessToken: data.access_token,
      tokenType: data.token_type ?? 'Bearer',
      expiresIn: data.expires_in ?? 3600,
      refreshToken: data.refresh_token,
      scope: data.scope,
    };
  }

  private getHeaders(token: string, options?: RequestOptions): Record<string, string> {
    const headers: Record<string, string> = {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/json',
      Accept: 'application/json',
      'X-API-Version': this.config.apiVersion,
      'User-Agent': `OpeniBank-TypeScript/${VERSION}`,
      ...options?.headers,
    };

    if (options?.idempotencyKey) {
      headers['Idempotency-Key'] = options.idempotencyKey;
    }

    return headers;
  }

  async request<T>(
    method: string,
    path: string,
    options?: {
      params?: Record<string, string | number | boolean | undefined>;
      body?: unknown;
      requestOptions?: RequestOptions;
    }
  ): Promise<T> {
    const token = await this.ensureToken();
    const headers = this.getHeaders(token, options?.requestOptions);

    let url = `${this.baseUrl}/${this.config.apiVersion}${path}`;

    // Add query parameters
    if (options?.params) {
      const params = new URLSearchParams();
      Object.entries(options.params).forEach(([key, value]) => {
        if (value !== undefined) {
          params.append(key, String(value));
        }
      });
      const queryString = params.toString();
      if (queryString) {
        url += `?${queryString}`;
      }
    }

    let lastError: Error | undefined;

    for (let attempt = 0; attempt < this.config.maxRetries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

        const response = await this.fetchFn(url, {
          method,
          headers,
          body: options?.body ? JSON.stringify(options.body) : undefined,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        const requestId = response.headers.get('X-Request-ID') ?? undefined;

        if (response.ok) {
          if (response.status === 204) {
            return {} as T;
          }
          return await response.json();
        }

        // Handle errors
        let errorData: Record<string, unknown>;
        try {
          errorData = await response.json();
        } catch {
          errorData = { message: await response.text() };
        }

        const errorMessage = (errorData.message as string) ?? 'Unknown error';
        const errorCode = errorData.code as string | undefined;

        switch (response.status) {
          case 401:
            throw new AuthenticationError(errorMessage, {
              code: errorCode,
              statusCode: response.status,
              requestId,
            });
          case 403:
            throw new AuthorizationError(errorMessage, {
              code: errorCode,
              statusCode: response.status,
              requestId,
              requiredScopes: errorData.required_scopes as string[] | undefined,
            });
          case 400:
            throw new ValidationError(errorMessage, {
              code: errorCode,
              statusCode: response.status,
              requestId,
              errors: errorData.errors as FieldError[] | undefined,
            });
          case 404:
            throw new NotFoundError(errorMessage, {
              code: errorCode,
              statusCode: response.status,
              requestId,
              resourceType: errorData.resource_type as string | undefined,
              resourceId: errorData.resource_id as string | undefined,
            });
          case 409:
            throw new ConflictError(errorMessage, {
              code: errorCode,
              statusCode: response.status,
              requestId,
            });
          case 429: {
            const retryAfter = parseFloat(response.headers.get('Retry-After') ?? '60');
            throw new RateLimitError(errorMessage, {
              code: errorCode,
              statusCode: response.status,
              requestId,
              retryAfter,
            });
          }
          default:
            if (response.status >= 500) {
              throw new ServerError(errorMessage, {
                code: errorCode,
                statusCode: response.status,
                requestId,
              });
            }
            throw new OpeniBankError(errorMessage, {
              code: errorCode,
              statusCode: response.status,
              requestId,
            });
        }
      } catch (error) {
        if (error instanceof OpeniBankError) {
          // Retry on rate limit or server errors
          if (
            (error instanceof RateLimitError || error instanceof ServerError) &&
            attempt < this.config.maxRetries - 1
          ) {
            const delay =
              error instanceof RateLimitError
                ? error.retryAfter * 1000
                : this.config.retryDelay * Math.pow(2, attempt);
            await this.sleep(delay);
            lastError = error;
            continue;
          }
          throw error;
        }

        // Network errors
        if (error instanceof Error) {
          if (error.name === 'AbortError') {
            lastError = new NetworkError('Request timeout');
          } else {
            lastError = new NetworkError(`Network error: ${error.message}`);
          }

          if (attempt < this.config.maxRetries - 1) {
            const delay = this.config.retryDelay * Math.pow(2, attempt);
            await this.sleep(delay);
            continue;
          }
        }
      }
    }

    throw lastError ?? new NetworkError('Request failed after all retries');
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

// =============================================================================
// API Resources
// =============================================================================

/**
 * Account list options
 */
export interface AccountListOptions {
  /** Filter by status */
  status?: string;
  /** Filter by account type */
  accountType?: string;
  /** Page size limit */
  limit?: number;
  /** Page offset */
  offset?: number;
}

/**
 * Accounts API resource
 */
class AccountsResource {
  constructor(private http: HTTPClient) {}

  /**
   * List all accounts
   */
  async list(options?: AccountListOptions): Promise<Account[]> {
    const data = await this.http.request<{ accounts: RawAccount[] }>('GET', '/accounts', {
      params: {
        status: options?.status,
        account_type: options?.accountType,
        limit: options?.limit ?? 50,
        offset: options?.offset ?? 0,
      },
    });
    return data.accounts.map(parseAccount);
  }

  /**
   * Get a single account
   */
  async get(accountId: string): Promise<Account> {
    const data = await this.http.request<RawAccount>('GET', `/accounts/${accountId}`);
    return parseAccount(data);
  }

  /**
   * Get account balances
   */
  async getBalances(accountId: string): Promise<Balance[]> {
    const data = await this.http.request<{ balances: RawBalance[] }>(
      'GET',
      `/accounts/${accountId}/balances`
    );
    return data.balances.map(parseBalance);
  }

  /**
   * Get account details
   */
  async getDetails(accountId: string): Promise<Account> {
    const data = await this.http.request<RawAccount>('GET', `/accounts/${accountId}/details`);
    return parseAccount(data);
  }
}

/**
 * Transaction list options
 */
export interface TransactionListOptions {
  /** Account ID */
  accountId: string;
  /** Start date filter */
  dateFrom?: Date;
  /** End date filter */
  dateTo?: Date;
  /** Minimum amount filter */
  amountMin?: number;
  /** Maximum amount filter */
  amountMax?: number;
  /** Booking status filter */
  bookingStatus?: string;
  /** Page size limit */
  limit?: number;
  /** Page offset */
  offset?: number;
}

/**
 * Transactions API resource
 */
class TransactionsResource {
  constructor(private http: HTTPClient) {}

  /**
   * List transactions for an account
   */
  async list(options: TransactionListOptions): Promise<Transaction[]> {
    const data = await this.http.request<{ transactions: RawTransaction[] }>(
      'GET',
      `/accounts/${options.accountId}/transactions`,
      {
        params: {
          date_from: options.dateFrom?.toISOString().split('T')[0],
          date_to: options.dateTo?.toISOString().split('T')[0],
          amount_min: options.amountMin,
          amount_max: options.amountMax,
          booking_status: options.bookingStatus,
          limit: options.limit ?? 50,
          offset: options.offset ?? 0,
        },
      }
    );
    return data.transactions.map(parseTransaction);
  }

  /**
   * Get a single transaction
   */
  async get(options: { accountId: string; transactionId: string }): Promise<Transaction> {
    const data = await this.http.request<RawTransaction>(
      'GET',
      `/accounts/${options.accountId}/transactions/${options.transactionId}`
    );
    return parseTransaction(data);
  }

  /**
   * Iterate through all transactions
   */
  async *iterate(options: Omit<TransactionListOptions, 'offset'>): AsyncGenerator<Transaction> {
    const limit = options.limit ?? 50;
    let offset = 0;

    while (true) {
      const transactions = await this.list({ ...options, limit, offset });

      if (transactions.length === 0) {
        break;
      }

      for (const tx of transactions) {
        yield tx;
      }

      if (transactions.length < limit) {
        break;
      }

      offset += limit;
    }
  }

  /**
   * List all transactions
   */
  async listAll(options: Omit<TransactionListOptions, 'limit' | 'offset'>): Promise<Transaction[]> {
    const result: Transaction[] = [];
    for await (const tx of this.iterate(options)) {
      result.push(tx);
    }
    return result;
  }
}

/**
 * Payment list options
 */
export interface PaymentListOptions {
  /** Filter by status */
  status?: string;
  /** Page size limit */
  limit?: number;
  /** Page offset */
  offset?: number;
}

/**
 * Payments API resource
 */
class PaymentsResource {
  constructor(private http: HTTPClient) {}

  /**
   * Create a new payment
   */
  async create(payment: PaymentRequest, options?: RequestOptions): Promise<Payment> {
    const data = await this.http.request<RawPayment>('POST', '/payments', {
      body: {
        creditor: {
          name: payment.creditor.name,
          account: {
            iban: payment.creditor.account.iban,
            bban: payment.creditor.account.bban,
          },
        },
        amount: {
          amount: payment.amount.amount,
          currency: payment.amount.currency,
        },
        debtor_account_id: payment.debtorAccountId,
        reference: payment.reference,
        end_to_end_id: payment.endToEndId,
        execution_date: payment.executionDate?.toISOString().split('T')[0],
      },
      requestOptions: options,
    });
    return parsePayment(data);
  }

  /**
   * Get payment status
   */
  async get(paymentId: string): Promise<Payment> {
    const data = await this.http.request<RawPayment>('GET', `/payments/${paymentId}`);
    return parsePayment(data);
  }

  /**
   * List payments
   */
  async list(options?: PaymentListOptions): Promise<Payment[]> {
    const data = await this.http.request<{ payments: RawPayment[] }>('GET', '/payments', {
      params: {
        status: options?.status,
        limit: options?.limit ?? 50,
        offset: options?.offset ?? 0,
      },
    });
    return data.payments.map(parsePayment);
  }

  /**
   * Cancel a pending payment
   */
  async cancel(paymentId: string): Promise<Payment> {
    const data = await this.http.request<RawPayment>('POST', `/payments/${paymentId}/cancel`);
    return parsePayment(data);
  }
}

/**
 * Consents API resource
 */
class ConsentsResource {
  constructor(private http: HTTPClient) {}

  /**
   * Create a new consent
   */
  async create(consent: ConsentRequest): Promise<Consent> {
    const data = await this.http.request<RawConsent>('POST', '/consents', {
      body: {
        access: consent.access,
        valid_until: consent.validUntil,
        recurring_indicator: consent.recurringIndicator,
        frequency_per_day: consent.frequencyPerDay,
      },
    });
    return parseConsent(data);
  }

  /**
   * Get consent status
   */
  async get(consentId: string): Promise<Consent> {
    const data = await this.http.request<RawConsent>('GET', `/consents/${consentId}`);
    return parseConsent(data);
  }

  /**
   * Revoke a consent
   */
  async revoke(consentId: string): Promise<void> {
    await this.http.request('DELETE', `/consents/${consentId}`);
  }

  /**
   * List all consents
   */
  async list(): Promise<Consent[]> {
    const data = await this.http.request<{ consents: RawConsent[] }>('GET', '/consents');
    return data.consents.map(parseConsent);
  }
}

/**
 * Institution list options
 */
export interface InstitutionListOptions {
  /** Filter by country code */
  country?: string;
  /** Search query */
  query?: string;
  /** Page size limit */
  limit?: number;
  /** Page offset */
  offset?: number;
}

/**
 * Financial institutions API resource
 */
class InstitutionsResource {
  constructor(private http: HTTPClient) {}

  /**
   * List financial institutions
   */
  async list(options?: InstitutionListOptions): Promise<Institution[]> {
    const data = await this.http.request<{ institutions: RawInstitution[] }>(
      'GET',
      '/institutions',
      {
        params: {
          country: options?.country,
          query: options?.query,
          limit: options?.limit ?? 50,
          offset: options?.offset ?? 0,
        },
      }
    );
    return data.institutions.map(parseInstitution);
  }

  /**
   * Get institution details
   */
  async get(institutionId: string): Promise<Institution> {
    const data = await this.http.request<RawInstitution>('GET', `/institutions/${institutionId}`);
    return parseInstitution(data);
  }
}

/**
 * Authorization URL options
 */
export interface AuthorizationUrlOptions {
  /** OAuth redirect URI */
  redirectUri: string;
  /** Requested scopes */
  scopes: string[];
  /** State parameter for CSRF protection */
  state?: string;
}

/**
 * Code exchange options
 */
export interface ExchangeCodeOptions {
  /** Authorization code */
  code: string;
  /** OAuth redirect URI */
  redirectUri: string;
}

/**
 * Authentication API resource
 */
class AuthResource {
  constructor(
    private http: HTTPClient,
    private config: Required<Config>
  ) {}

  /**
   * Generate OAuth authorization URL
   */
  getAuthorizationUrl(options: AuthorizationUrlOptions): string {
    const baseUrl =
      this.config.environment === 'production'
        ? 'https://api.openibank.com'
        : 'https://sandbox.openibank.com';

    const params = new URLSearchParams({
      client_id: this.config.clientId,
      redirect_uri: options.redirectUri,
      response_type: 'code',
      scope: options.scopes.join(' '),
    });

    if (options.state) {
      params.append('state', options.state);
    }

    return `${baseUrl}/oauth/authorize?${params.toString()}`;
  }

  /**
   * Exchange authorization code for tokens
   */
  async exchangeCode(options: ExchangeCodeOptions): Promise<TokenResponse> {
    const baseUrl =
      this.config.environment === 'production'
        ? 'https://api.openibank.com'
        : 'https://sandbox.openibank.com';

    const response = await this.config.fetch(`${baseUrl}/oauth/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: new URLSearchParams({
        grant_type: 'authorization_code',
        client_id: this.config.clientId,
        client_secret: this.config.clientSecret,
        code: options.code,
        redirect_uri: options.redirectUri,
      }),
    });

    if (!response.ok) {
      throw new AuthenticationError(`Failed to exchange code: ${response.status}`);
    }

    const data = await response.json();
    return {
      accessToken: data.access_token,
      tokenType: data.token_type ?? 'Bearer',
      expiresIn: data.expires_in ?? 3600,
      refreshToken: data.refresh_token,
      scope: data.scope,
    };
  }

  /**
   * Refresh access token
   */
  async refreshToken(refreshToken: string): Promise<TokenResponse> {
    const baseUrl =
      this.config.environment === 'production'
        ? 'https://api.openibank.com'
        : 'https://sandbox.openibank.com';

    const response = await this.config.fetch(`${baseUrl}/oauth/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
      },
      body: new URLSearchParams({
        grant_type: 'refresh_token',
        client_id: this.config.clientId,
        client_secret: this.config.clientSecret,
        refresh_token: refreshToken,
      }),
    });

    if (!response.ok) {
      throw new AuthenticationError(`Failed to refresh token: ${response.status}`);
    }

    const data = await response.json();
    return {
      accessToken: data.access_token,
      tokenType: data.token_type ?? 'Bearer',
      expiresIn: data.expires_in ?? 3600,
      refreshToken: data.refresh_token,
      scope: data.scope,
    };
  }
}

// =============================================================================
// Real-time WebSocket
// =============================================================================

/**
 * Subscription options
 */
export interface SubscribeOptions {
  /** Account ID to subscribe to */
  accountId: string;
  /** Event types to subscribe to */
  events: EventType[];
  /** Event handlers */
  handlers: Partial<Record<EventType, (event: RealtimeEvent) => void | Promise<void>>>;
}

/**
 * WebSocket subscription handle
 */
export class Subscription {
  private running = false;
  private messageHandler?: (event: MessageEvent) => void;

  constructor(
    private ws: WebSocket,
    private handlers: SubscribeOptions['handlers']
  ) {
    this.running = true;
    this.processMessages();
  }

  private processMessages(): void {
    this.messageHandler = (event: MessageEvent) => {
      if (!this.running) return;

      try {
        const data = JSON.parse(event.data);
        const eventType = data.type as EventType;
        const handler = this.handlers[eventType];

        if (handler) {
          const realtimeEvent: RealtimeEvent = {
            type: eventType,
            data: data.data,
            timestamp: new Date(data.timestamp ?? Date.now()),
          };
          handler(realtimeEvent);
        }
      } catch (error) {
        console.error('WebSocket message error:', error);
      }
    };

    this.ws.addEventListener('message', this.messageHandler);
  }

  /**
   * Wait for subscription to complete
   */
  wait(): Promise<void> {
    return new Promise((resolve, reject) => {
      this.ws.addEventListener('close', () => resolve());
      this.ws.addEventListener('error', (error) => reject(error));
    });
  }

  /**
   * Close the subscription
   */
  close(): void {
    this.running = false;
    if (this.messageHandler) {
      this.ws.removeEventListener('message', this.messageHandler);
    }
    this.ws.close();
  }
}

/**
 * Real-time WebSocket resource
 */
class RealtimeResource {
  constructor(private http: HTTPClient) {}

  /**
   * Subscribe to real-time events
   */
  async subscribe(options: SubscribeOptions): Promise<Subscription> {
    const config = this.http.getConfig();
    const websocketUrl = this.http.websocketUrl;
    const url = `${websocketUrl}/subscribe`;

    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url);

      ws.onopen = () => {
        // Send subscription request
        ws.send(
          JSON.stringify({
            action: 'subscribe',
            account_id: options.accountId,
            events: options.events,
            api_version: config.apiVersion,
          })
        );

        resolve(new Subscription(ws, options.handlers));
      };

      ws.onerror = (error) => {
        reject(new NetworkError(`WebSocket connection failed: ${error}`));
      };
    });
  }
}

// =============================================================================
// Raw API Types (for parsing)
// =============================================================================

interface RawBalance {
  amount: string;
  currency: string;
  type?: string;
  credit_limit?: string;
  last_updated?: string;
}

interface RawAccount {
  id: string;
  name: string;
  iban?: string;
  bban?: string;
  currency: string;
  account_type: string;
  status: string;
  balance?: RawBalance;
  institution_id?: string;
  owner_name?: string;
  created_at?: string;
  updated_at?: string;
}

interface RawTransaction {
  id: string;
  account_id: string;
  amount: string;
  currency: string;
  description: string;
  reference?: string;
  booking_date?: string;
  value_date?: string;
  transaction_type: string;
  status: string;
  counterparty_name?: string;
  counterparty_iban?: string;
  category?: string;
  metadata?: Record<string, unknown>;
}

interface RawPayment {
  id: string;
  status: string;
  amount: string;
  currency: string;
  creditor_name: string;
  creditor_iban?: string;
  reference?: string;
  created_at?: string;
  executed_at?: string;
}

interface RawConsent {
  id: string;
  status: string;
  access: string[];
  valid_until?: string;
  authorization_url?: string;
  created_at?: string;
}

interface RawInstitution {
  id: string;
  name: string;
  bic?: string;
  country: string;
  logo_url?: string;
  supported_features: string[];
}

// =============================================================================
// Parsing Functions
// =============================================================================

function parseBalance(raw: RawBalance): Balance {
  return {
    amount: raw.amount,
    currency: raw.currency,
    type: raw.type,
    creditLimit: raw.credit_limit,
    lastUpdated: raw.last_updated ? new Date(raw.last_updated) : undefined,
  };
}

function parseAccount(raw: RawAccount): Account {
  return {
    id: raw.id,
    name: raw.name,
    iban: raw.iban,
    bban: raw.bban,
    currency: raw.currency,
    accountType: raw.account_type,
    status: raw.status,
    balance: raw.balance ? parseBalance(raw.balance) : undefined,
    institutionId: raw.institution_id,
    ownerName: raw.owner_name,
    createdAt: raw.created_at ? new Date(raw.created_at) : undefined,
    updatedAt: raw.updated_at ? new Date(raw.updated_at) : undefined,
  };
}

function parseTransaction(raw: RawTransaction): Transaction {
  return {
    id: raw.id,
    accountId: raw.account_id,
    amount: raw.amount,
    currency: raw.currency,
    description: raw.description,
    reference: raw.reference,
    bookingDate: raw.booking_date ? new Date(raw.booking_date) : undefined,
    valueDate: raw.value_date ? new Date(raw.value_date) : undefined,
    transactionType: raw.transaction_type,
    status: raw.status,
    counterpartyName: raw.counterparty_name,
    counterpartyIban: raw.counterparty_iban,
    category: raw.category,
    metadata: raw.metadata,
  };
}

function parsePayment(raw: RawPayment): Payment {
  return {
    id: raw.id,
    status: raw.status,
    amount: raw.amount,
    currency: raw.currency,
    creditorName: raw.creditor_name,
    creditorIban: raw.creditor_iban,
    reference: raw.reference,
    createdAt: raw.created_at ? new Date(raw.created_at) : undefined,
    executedAt: raw.executed_at ? new Date(raw.executed_at) : undefined,
  };
}

function parseConsent(raw: RawConsent): Consent {
  return {
    id: raw.id,
    status: raw.status,
    access: raw.access,
    validUntil: raw.valid_until ? new Date(raw.valid_until) : undefined,
    authorizationUrl: raw.authorization_url,
    createdAt: raw.created_at ? new Date(raw.created_at) : undefined,
  };
}

function parseInstitution(raw: RawInstitution): Institution {
  return {
    id: raw.id,
    name: raw.name,
    bic: raw.bic,
    country: raw.country,
    logoUrl: raw.logo_url,
    supportedFeatures: raw.supported_features,
  };
}

// =============================================================================
// Main Client
// =============================================================================

/**
 * OpeniBank API Client
 *
 * @example
 * ```typescript
 * const client = new OpeniBank({
 *   clientId: 'your_client_id',
 *   clientSecret: 'your_client_secret',
 *   environment: 'sandbox'
 * });
 *
 * const accounts = await client.accounts.list();
 * ```
 */
export class OpeniBank {
  private http: HTTPClient;

  /** Accounts API */
  public readonly accounts: AccountsResource;
  /** Transactions API */
  public readonly transactions: TransactionsResource;
  /** Payments API */
  public readonly payments: PaymentsResource;
  /** Consents API */
  public readonly consents: ConsentsResource;
  /** Institutions API */
  public readonly institutions: InstitutionsResource;
  /** Authentication API */
  public readonly auth: AuthResource;
  /** Real-time WebSocket API */
  public readonly realtime: RealtimeResource;

  /**
   * Create a new OpeniBank client
   */
  constructor(config: Config) {
    this.http = new HTTPClient(config);

    // Initialize resources
    this.accounts = new AccountsResource(this.http);
    this.transactions = new TransactionsResource(this.http);
    this.payments = new PaymentsResource(this.http);
    this.consents = new ConsentsResource(this.http);
    this.institutions = new InstitutionsResource(this.http);
    this.auth = new AuthResource(this.http, this.http.getConfig());
    this.realtime = new RealtimeResource(this.http);
  }

  /**
   * Set the access token manually
   */
  setAccessToken(token: string, expiresIn: number = 3600): void {
    this.http.setAccessToken(token, expiresIn);
  }
}

// =============================================================================
// Default Export
// =============================================================================

export default OpeniBank;
