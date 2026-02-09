// Package openibank provides a Go client for the OpeniBank Open Banking Platform API.
//
// Example usage:
//
//	client := openibank.NewClient(
//	    openibank.WithClientCredentials("your_client_id", "your_client_secret"),
//	    openibank.WithEnvironment(openibank.Sandbox),
//	)
//
//	accounts, err := client.Accounts.List(context.Background(), nil)
//	if err != nil {
//	    log.Fatal(err)
//	}
//
//	for _, account := range accounts {
//	    fmt.Printf("%s: %s %s\n", account.Name, account.Balance.Amount, account.Balance.Currency)
//	}
package openibank

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Version is the SDK version.
const Version = "0.9.0"

// Environment represents the API environment.
type Environment string

const (
	// Sandbox is the sandbox environment for testing.
	Sandbox Environment = "sandbox"
	// Production is the production environment.
	Production Environment = "production"
)

// Client is the OpeniBank API client.
type Client struct {
	// Accounts provides access to the Accounts API.
	Accounts *AccountsService
	// Transactions provides access to the Transactions API.
	Transactions *TransactionsService
	// Payments provides access to the Payments API.
	Payments *PaymentsService
	// Consents provides access to the Consents API.
	Consents *ConsentsService
	// Institutions provides access to the Institutions API.
	Institutions *InstitutionsService
	// Auth provides access to authentication methods.
	Auth *AuthService
	// Realtime provides access to WebSocket functionality.
	Realtime *RealtimeService

	config      *Config
	httpClient  *http.Client
	accessToken string
	tokenExpiry time.Time
	tokenMu     sync.RWMutex
}

// Config holds the client configuration.
type Config struct {
	ClientID     string
	ClientSecret string
	APIKey       string
	Environment  Environment
	APIVersion   string
	Timeout      time.Duration
	MaxRetries   int
	RetryDelay   time.Duration
	AutoRefresh  bool
	Debug        bool
	HTTPClient   *http.Client
}

// Option is a function that configures the client.
type Option func(*Config)

// WithClientCredentials sets the OAuth client credentials.
func WithClientCredentials(clientID, clientSecret string) Option {
	return func(c *Config) {
		c.ClientID = clientID
		c.ClientSecret = clientSecret
	}
}

// WithAPIKey sets the API key for sandbox testing.
func WithAPIKey(apiKey string) Option {
	return func(c *Config) {
		c.APIKey = apiKey
	}
}

// WithEnvironment sets the API environment.
func WithEnvironment(env Environment) Option {
	return func(c *Config) {
		c.Environment = env
	}
}

// WithAPIVersion sets the API version.
func WithAPIVersion(version string) Option {
	return func(c *Config) {
		c.APIVersion = version
	}
}

// WithTimeout sets the HTTP request timeout.
func WithTimeout(timeout time.Duration) Option {
	return func(c *Config) {
		c.Timeout = timeout
	}
}

// WithMaxRetries sets the maximum number of retry attempts.
func WithMaxRetries(retries int) Option {
	return func(c *Config) {
		c.MaxRetries = retries
	}
}

// WithRetryDelay sets the delay between retries.
func WithRetryDelay(delay time.Duration) Option {
	return func(c *Config) {
		c.RetryDelay = delay
	}
}

// WithAutoRefresh enables or disables automatic token refresh.
func WithAutoRefresh(enabled bool) Option {
	return func(c *Config) {
		c.AutoRefresh = enabled
	}
}

// WithDebug enables or disables debug logging.
func WithDebug(enabled bool) Option {
	return func(c *Config) {
		c.Debug = enabled
	}
}

// WithHTTPClient sets a custom HTTP client.
func WithHTTPClient(client *http.Client) Option {
	return func(c *Config) {
		c.HTTPClient = client
	}
}

// NewClient creates a new OpeniBank client with the given options.
func NewClient(opts ...Option) *Client {
	config := &Config{
		Environment: Sandbox,
		APIVersion:  "v2",
		Timeout:     30 * time.Second,
		MaxRetries:  3,
		RetryDelay:  time.Second,
		AutoRefresh: true,
		Debug:       false,
	}

	for _, opt := range opts {
		opt(config)
	}

	httpClient := config.HTTPClient
	if httpClient == nil {
		httpClient = &http.Client{
			Timeout: config.Timeout,
		}
	}

	client := &Client{
		config:     config,
		httpClient: httpClient,
	}

	// Initialize services
	client.Accounts = &AccountsService{client: client}
	client.Transactions = &TransactionsService{client: client}
	client.Payments = &PaymentsService{client: client}
	client.Consents = &ConsentsService{client: client}
	client.Institutions = &InstitutionsService{client: client}
	client.Auth = &AuthService{client: client}
	client.Realtime = &RealtimeService{client: client}

	return client
}

// NewClientFromEnv creates a new client from environment variables.
func NewClientFromEnv() *Client {
	return NewClient(
		WithClientCredentials(
			os.Getenv("OPENIBANK_CLIENT_ID"),
			os.Getenv("OPENIBANK_CLIENT_SECRET"),
		),
		WithAPIKey(os.Getenv("OPENIBANK_API_KEY")),
		WithEnvironment(Environment(os.Getenv("OPENIBANK_ENVIRONMENT"))),
		WithAPIVersion(getEnvOrDefault("OPENIBANK_API_VERSION", "v2")),
	)
}

func getEnvOrDefault(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

// SetAccessToken sets the access token manually.
func (c *Client) SetAccessToken(token string) {
	c.tokenMu.Lock()
	defer c.tokenMu.Unlock()
	c.accessToken = token
	c.tokenExpiry = time.Now().Add(time.Hour) // Assume 1 hour validity
}

// BaseURL returns the base URL for the current environment.
func (c *Client) BaseURL() string {
	if c.config.Environment == Production {
		return "https://api.openibank.com"
	}
	return "https://sandbox.openibank.com"
}

// WebSocketURL returns the WebSocket URL for the current environment.
func (c *Client) WebSocketURL() string {
	if c.config.Environment == Production {
		return "wss://ws.openibank.com"
	}
	return "wss://ws.sandbox.openibank.com"
}

// ensureToken ensures we have a valid access token.
func (c *Client) ensureToken(ctx context.Context) (string, error) {
	c.tokenMu.RLock()
	if c.accessToken != "" && time.Now().Before(c.tokenExpiry) {
		token := c.accessToken
		c.tokenMu.RUnlock()
		return token, nil
	}
	c.tokenMu.RUnlock()

	// Use API key if available
	if c.config.APIKey != "" {
		return c.config.APIKey, nil
	}

	// Get new token using client credentials
	if c.config.ClientID != "" && c.config.ClientSecret != "" {
		tokens, err := c.Auth.requestToken(ctx)
		if err != nil {
			return "", err
		}

		c.tokenMu.Lock()
		c.accessToken = tokens.AccessToken
		c.tokenExpiry = time.Now().Add(time.Duration(tokens.ExpiresIn-60) * time.Second)
		c.tokenMu.Unlock()

		return tokens.AccessToken, nil
	}

	return "", &AuthenticationError{Message: "No valid credentials configured"}
}

// RequestOption is an option for individual requests.
type RequestOption func(*requestConfig)

type requestConfig struct {
	idempotencyKey string
}

// WithIdempotencyKey sets an idempotency key for the request.
func WithIdempotencyKey(key string) RequestOption {
	return func(c *requestConfig) {
		c.idempotencyKey = key
	}
}

// request makes an HTTP request to the API.
func (c *Client) request(ctx context.Context, method, path string, params url.Values, body interface{}, result interface{}, opts ...RequestOption) error {
	reqConfig := &requestConfig{}
	for _, opt := range opts {
		opt(reqConfig)
	}

	token, err := c.ensureToken(ctx)
	if err != nil {
		return err
	}

	// Build URL
	reqURL := fmt.Sprintf("%s/%s%s", c.BaseURL(), c.config.APIVersion, path)
	if params != nil && len(params) > 0 {
		reqURL += "?" + params.Encode()
	}

	var bodyReader io.Reader
	if body != nil {
		bodyBytes, err := json.Marshal(body)
		if err != nil {
			return fmt.Errorf("failed to marshal request body: %w", err)
		}
		bodyReader = bytes.NewReader(bodyBytes)
	}

	var lastErr error
	for attempt := 0; attempt <= c.config.MaxRetries; attempt++ {
		req, err := http.NewRequestWithContext(ctx, method, reqURL, bodyReader)
		if err != nil {
			return fmt.Errorf("failed to create request: %w", err)
		}

		// Set headers
		req.Header.Set("Authorization", "Bearer "+token)
		req.Header.Set("Content-Type", "application/json")
		req.Header.Set("Accept", "application/json")
		req.Header.Set("X-API-Version", c.config.APIVersion)
		req.Header.Set("User-Agent", "OpeniBank-Go/"+Version)

		if reqConfig.idempotencyKey != "" {
			req.Header.Set("Idempotency-Key", reqConfig.idempotencyKey)
		}

		resp, err := c.httpClient.Do(req)
		if err != nil {
			lastErr = &NetworkError{Message: fmt.Sprintf("request failed: %v", err)}
			if attempt < c.config.MaxRetries {
				time.Sleep(c.config.RetryDelay * time.Duration(1<<attempt))
				continue
			}
			return lastErr
		}
		defer resp.Body.Close()

		requestID := resp.Header.Get("X-Request-ID")

		// Success
		if resp.StatusCode >= 200 && resp.StatusCode < 300 {
			if resp.StatusCode == 204 || result == nil {
				return nil
			}
			if err := json.NewDecoder(resp.Body).Decode(result); err != nil {
				return fmt.Errorf("failed to decode response: %w", err)
			}
			return nil
		}

		// Parse error response
		var errResp struct {
			Message        string       `json:"message"`
			Code           string       `json:"code"`
			Errors         []FieldError `json:"errors"`
			ResourceType   string       `json:"resource_type"`
			ResourceID     string       `json:"resource_id"`
			RequiredScopes []string     `json:"required_scopes"`
		}
		if err := json.NewDecoder(resp.Body).Decode(&errResp); err != nil {
			errResp.Message = "Unknown error"
		}

		switch resp.StatusCode {
		case 401:
			return &AuthenticationError{
				Message:    errResp.Message,
				Code:       errResp.Code,
				StatusCode: resp.StatusCode,
				RequestID:  requestID,
			}
		case 403:
			return &AuthorizationError{
				Message:        errResp.Message,
				Code:           errResp.Code,
				StatusCode:     resp.StatusCode,
				RequestID:      requestID,
				RequiredScopes: errResp.RequiredScopes,
			}
		case 400:
			return &ValidationError{
				Message:    errResp.Message,
				Code:       errResp.Code,
				StatusCode: resp.StatusCode,
				RequestID:  requestID,
				Errors:     errResp.Errors,
			}
		case 404:
			return &NotFoundError{
				Message:      errResp.Message,
				Code:         errResp.Code,
				StatusCode:   resp.StatusCode,
				RequestID:    requestID,
				ResourceType: errResp.ResourceType,
				ResourceID:   errResp.ResourceID,
			}
		case 409:
			return &ConflictError{
				Message:    errResp.Message,
				Code:       errResp.Code,
				StatusCode: resp.StatusCode,
				RequestID:  requestID,
			}
		case 429:
			retryAfter := 60 * time.Second
			if ra := resp.Header.Get("Retry-After"); ra != "" {
				if seconds, err := strconv.Atoi(ra); err == nil {
					retryAfter = time.Duration(seconds) * time.Second
				}
			}
			lastErr = &RateLimitError{
				Message:    errResp.Message,
				Code:       errResp.Code,
				StatusCode: resp.StatusCode,
				RequestID:  requestID,
				RetryAfter: retryAfter,
			}
			if attempt < c.config.MaxRetries {
				time.Sleep(retryAfter)
				continue
			}
			return lastErr
		default:
			if resp.StatusCode >= 500 {
				lastErr = &ServerError{
					Message:    errResp.Message,
					Code:       errResp.Code,
					StatusCode: resp.StatusCode,
					RequestID:  requestID,
				}
				if attempt < c.config.MaxRetries {
					time.Sleep(c.config.RetryDelay * time.Duration(1<<attempt))
					continue
				}
				return lastErr
			}
			return &Error{
				Message:    errResp.Message,
				Code:       errResp.Code,
				StatusCode: resp.StatusCode,
				RequestID:  requestID,
			}
		}
	}

	return lastErr
}

// =============================================================================
// Helper Functions
// =============================================================================

// String returns a pointer to a string.
func String(s string) *string {
	return &s
}

// Int returns a pointer to an int.
func Int(i int) *int {
	return &i
}

// Int64 returns a pointer to an int64.
func Int64(i int64) *int64 {
	return &i
}

// Float64 returns a pointer to a float64.
func Float64(f float64) *float64 {
	return &f
}

// Bool returns a pointer to a bool.
func Bool(b bool) *bool {
	return &b
}

// Time returns a pointer to a time.Time.
func Time(t time.Time) *time.Time {
	return &t
}

// =============================================================================
// Models
// =============================================================================

// Amount represents a monetary amount with currency.
type Amount struct {
	Amount   string `json:"amount"`
	Currency string `json:"currency"`
}

// Balance represents an account balance.
type Balance struct {
	Amount      string     `json:"amount"`
	Currency    string     `json:"currency"`
	Type        string     `json:"type,omitempty"`
	CreditLimit *string    `json:"credit_limit,omitempty"`
	LastUpdated *time.Time `json:"last_updated,omitempty"`
}

// Account represents a bank account.
type Account struct {
	ID            string     `json:"id"`
	Name          string     `json:"name"`
	IBAN          *string    `json:"iban,omitempty"`
	BBAN          *string    `json:"bban,omitempty"`
	Currency      string     `json:"currency"`
	AccountType   string     `json:"account_type"`
	Status        string     `json:"status"`
	Balance       *Balance   `json:"balance,omitempty"`
	InstitutionID *string    `json:"institution_id,omitempty"`
	OwnerName     *string    `json:"owner_name,omitempty"`
	CreatedAt     *time.Time `json:"created_at,omitempty"`
	UpdatedAt     *time.Time `json:"updated_at,omitempty"`
}

// Transaction represents a bank transaction.
type Transaction struct {
	ID               string                 `json:"id"`
	AccountID        string                 `json:"account_id"`
	Amount           string                 `json:"amount"`
	Currency         string                 `json:"currency"`
	Description      string                 `json:"description"`
	Reference        *string                `json:"reference,omitempty"`
	BookingDate      *time.Time             `json:"booking_date,omitempty"`
	ValueDate        *time.Time             `json:"value_date,omitempty"`
	TransactionType  string                 `json:"transaction_type"`
	Status           string                 `json:"status"`
	CounterpartyName *string                `json:"counterparty_name,omitempty"`
	CounterpartyIBAN *string                `json:"counterparty_iban,omitempty"`
	Category         *string                `json:"category,omitempty"`
	Metadata         map[string]interface{} `json:"metadata,omitempty"`
}

// CreditorAccount represents a creditor's account for payments.
type CreditorAccount struct {
	IBAN          *string `json:"iban,omitempty"`
	BBAN          *string `json:"bban,omitempty"`
	SortCode      *string `json:"sort_code,omitempty"`
	AccountNumber *string `json:"account_number,omitempty"`
}

// Creditor represents a payment creditor.
type Creditor struct {
	Name    string          `json:"name"`
	Account CreditorAccount `json:"account"`
}

// Payment represents a payment.
type Payment struct {
	ID           string     `json:"id"`
	Status       string     `json:"status"`
	Amount       string     `json:"amount"`
	Currency     string     `json:"currency"`
	CreditorName string     `json:"creditor_name"`
	CreditorIBAN *string    `json:"creditor_iban,omitempty"`
	Reference    *string    `json:"reference,omitempty"`
	CreatedAt    *time.Time `json:"created_at,omitempty"`
	ExecutedAt   *time.Time `json:"executed_at,omitempty"`
}

// Consent represents a consent.
type Consent struct {
	ID               string     `json:"id"`
	Status           string     `json:"status"`
	Access           []string   `json:"access"`
	ValidUntil       *time.Time `json:"valid_until,omitempty"`
	AuthorizationURL *string    `json:"authorization_url,omitempty"`
	CreatedAt        *time.Time `json:"created_at,omitempty"`
}

// Institution represents a financial institution.
type Institution struct {
	ID                string   `json:"id"`
	Name              string   `json:"name"`
	BIC               *string  `json:"bic,omitempty"`
	Country           string   `json:"country"`
	LogoURL           *string  `json:"logo_url,omitempty"`
	SupportedFeatures []string `json:"supported_features"`
}

// TokenResponse represents an OAuth token response.
type TokenResponse struct {
	AccessToken  string `json:"access_token"`
	TokenType    string `json:"token_type"`
	ExpiresIn    int    `json:"expires_in"`
	RefreshToken string `json:"refresh_token,omitempty"`
	Scope        string `json:"scope,omitempty"`
}

// =============================================================================
// Errors
// =============================================================================

// Error is the base error type for all API errors.
type Error struct {
	Message    string `json:"message"`
	Code       string `json:"code,omitempty"`
	StatusCode int    `json:"status_code,omitempty"`
	RequestID  string `json:"request_id,omitempty"`
}

func (e *Error) Error() string {
	if e.RequestID != "" {
		return fmt.Sprintf("%s (code: %s, request_id: %s)", e.Message, e.Code, e.RequestID)
	}
	if e.Code != "" {
		return fmt.Sprintf("%s (code: %s)", e.Message, e.Code)
	}
	return e.Message
}

// FieldError represents a validation error for a specific field.
type FieldError struct {
	Field   string `json:"field"`
	Message string `json:"message"`
	Code    string `json:"code,omitempty"`
}

// AuthenticationError indicates authentication failure.
type AuthenticationError struct {
	Message    string `json:"message"`
	Code       string `json:"code,omitempty"`
	StatusCode int    `json:"status_code,omitempty"`
	RequestID  string `json:"request_id,omitempty"`
}

func (e *AuthenticationError) Error() string {
	return fmt.Sprintf("authentication error: %s", e.Message)
}

// AuthorizationError indicates authorization failure.
type AuthorizationError struct {
	Message        string   `json:"message"`
	Code           string   `json:"code,omitempty"`
	StatusCode     int      `json:"status_code,omitempty"`
	RequestID      string   `json:"request_id,omitempty"`
	RequiredScopes []string `json:"required_scopes,omitempty"`
}

func (e *AuthorizationError) Error() string {
	return fmt.Sprintf("authorization error: %s", e.Message)
}

// ValidationError indicates request validation failure.
type ValidationError struct {
	Message    string       `json:"message"`
	Code       string       `json:"code,omitempty"`
	StatusCode int          `json:"status_code,omitempty"`
	RequestID  string       `json:"request_id,omitempty"`
	Errors     []FieldError `json:"errors,omitempty"`
}

func (e *ValidationError) Error() string {
	return fmt.Sprintf("validation error: %s", e.Message)
}

// NotFoundError indicates resource not found.
type NotFoundError struct {
	Message      string `json:"message"`
	Code         string `json:"code,omitempty"`
	StatusCode   int    `json:"status_code,omitempty"`
	RequestID    string `json:"request_id,omitempty"`
	ResourceType string `json:"resource_type,omitempty"`
	ResourceID   string `json:"resource_id,omitempty"`
}

func (e *NotFoundError) Error() string {
	return fmt.Sprintf("not found: %s", e.Message)
}

// RateLimitError indicates rate limit exceeded.
type RateLimitError struct {
	Message    string        `json:"message"`
	Code       string        `json:"code,omitempty"`
	StatusCode int           `json:"status_code,omitempty"`
	RequestID  string        `json:"request_id,omitempty"`
	RetryAfter time.Duration `json:"retry_after,omitempty"`
}

func (e *RateLimitError) Error() string {
	return fmt.Sprintf("rate limit exceeded: %s (retry after %v)", e.Message, e.RetryAfter)
}

// ConflictError indicates resource conflict.
type ConflictError struct {
	Message    string `json:"message"`
	Code       string `json:"code,omitempty"`
	StatusCode int    `json:"status_code,omitempty"`
	RequestID  string `json:"request_id,omitempty"`
}

func (e *ConflictError) Error() string {
	return fmt.Sprintf("conflict: %s", e.Message)
}

// ServerError indicates internal server error.
type ServerError struct {
	Message    string `json:"message"`
	Code       string `json:"code,omitempty"`
	StatusCode int    `json:"status_code,omitempty"`
	RequestID  string `json:"request_id,omitempty"`
}

func (e *ServerError) Error() string {
	return fmt.Sprintf("server error: %s (request_id: %s)", e.Message, e.RequestID)
}

// NetworkError indicates network or connection error.
type NetworkError struct {
	Message string `json:"message"`
}

func (e *NetworkError) Error() string {
	return fmt.Sprintf("network error: %s", e.Message)
}

// =============================================================================
// Services
// =============================================================================

// AccountsService provides access to the Accounts API.
type AccountsService struct {
	client *Client
}

// AccountListParams contains parameters for listing accounts.
type AccountListParams struct {
	Status      *string
	AccountType *string
	Limit       *int
	Offset      *int
}

// List lists all accounts.
func (s *AccountsService) List(ctx context.Context, params *AccountListParams) ([]Account, error) {
	values := url.Values{}
	if params != nil {
		if params.Status != nil {
			values.Set("status", *params.Status)
		}
		if params.AccountType != nil {
			values.Set("account_type", *params.AccountType)
		}
		if params.Limit != nil {
			values.Set("limit", strconv.Itoa(*params.Limit))
		}
		if params.Offset != nil {
			values.Set("offset", strconv.Itoa(*params.Offset))
		}
	}

	var result struct {
		Accounts []Account `json:"accounts"`
	}
	if err := s.client.request(ctx, "GET", "/accounts", values, nil, &result); err != nil {
		return nil, err
	}
	return result.Accounts, nil
}

// Get gets a single account.
func (s *AccountsService) Get(ctx context.Context, accountID string) (*Account, error) {
	var account Account
	if err := s.client.request(ctx, "GET", "/accounts/"+accountID, nil, nil, &account); err != nil {
		return nil, err
	}
	return &account, nil
}

// GetBalances gets account balances.
func (s *AccountsService) GetBalances(ctx context.Context, accountID string) ([]Balance, error) {
	var result struct {
		Balances []Balance `json:"balances"`
	}
	if err := s.client.request(ctx, "GET", "/accounts/"+accountID+"/balances", nil, nil, &result); err != nil {
		return nil, err
	}
	return result.Balances, nil
}

// TransactionsService provides access to the Transactions API.
type TransactionsService struct {
	client *Client
}

// TransactionListParams contains parameters for listing transactions.
type TransactionListParams struct {
	DateFrom      *time.Time
	DateTo        *time.Time
	AmountMin     *float64
	AmountMax     *float64
	BookingStatus *string
	Limit         *int
	Offset        *int
}

// List lists transactions for an account.
func (s *TransactionsService) List(ctx context.Context, accountID string, params *TransactionListParams) ([]Transaction, error) {
	values := url.Values{}
	if params != nil {
		if params.DateFrom != nil {
			values.Set("date_from", params.DateFrom.Format("2006-01-02"))
		}
		if params.DateTo != nil {
			values.Set("date_to", params.DateTo.Format("2006-01-02"))
		}
		if params.AmountMin != nil {
			values.Set("amount_min", strconv.FormatFloat(*params.AmountMin, 'f', 2, 64))
		}
		if params.AmountMax != nil {
			values.Set("amount_max", strconv.FormatFloat(*params.AmountMax, 'f', 2, 64))
		}
		if params.BookingStatus != nil {
			values.Set("booking_status", *params.BookingStatus)
		}
		if params.Limit != nil {
			values.Set("limit", strconv.Itoa(*params.Limit))
		}
		if params.Offset != nil {
			values.Set("offset", strconv.Itoa(*params.Offset))
		}
	}

	var result struct {
		Transactions []Transaction `json:"transactions"`
	}
	if err := s.client.request(ctx, "GET", "/accounts/"+accountID+"/transactions", values, nil, &result); err != nil {
		return nil, err
	}
	return result.Transactions, nil
}

// Get gets a single transaction.
func (s *TransactionsService) Get(ctx context.Context, accountID, transactionID string) (*Transaction, error) {
	var transaction Transaction
	if err := s.client.request(ctx, "GET", "/accounts/"+accountID+"/transactions/"+transactionID, nil, nil, &transaction); err != nil {
		return nil, err
	}
	return &transaction, nil
}

// TransactionIterator iterates through transactions.
type TransactionIterator struct {
	client    *Client
	accountID string
	params    *TransactionListParams
	limit     int
	offset    int
	current   []Transaction
	index     int
	err       error
	done      bool
}

// Iter returns an iterator for transactions.
func (s *TransactionsService) Iter(ctx context.Context, accountID string, params *TransactionListParams) *TransactionIterator {
	limit := 50
	if params != nil && params.Limit != nil {
		limit = *params.Limit
	}
	return &TransactionIterator{
		client:    s.client,
		accountID: accountID,
		params:    params,
		limit:     limit,
		offset:    0,
	}
}

// Next advances the iterator.
func (it *TransactionIterator) Next() bool {
	if it.err != nil || it.done {
		return false
	}

	it.index++
	if it.index < len(it.current) {
		return true
	}

	// Fetch next page
	params := &TransactionListParams{
		Limit:  &it.limit,
		Offset: &it.offset,
	}
	if it.params != nil {
		params.DateFrom = it.params.DateFrom
		params.DateTo = it.params.DateTo
		params.AmountMin = it.params.AmountMin
		params.AmountMax = it.params.AmountMax
		params.BookingStatus = it.params.BookingStatus
	}

	transactions, err := it.client.Transactions.List(context.Background(), it.accountID, params)
	if err != nil {
		it.err = err
		return false
	}

	if len(transactions) == 0 {
		it.done = true
		return false
	}

	it.current = transactions
	it.index = 0
	it.offset += len(transactions)

	if len(transactions) < it.limit {
		it.done = true
	}

	return true
}

// Transaction returns the current transaction.
func (it *TransactionIterator) Transaction() *Transaction {
	if it.index < 0 || it.index >= len(it.current) {
		return nil
	}
	return &it.current[it.index]
}

// Err returns any error encountered during iteration.
func (it *TransactionIterator) Err() error {
	return it.err
}

// PaymentsService provides access to the Payments API.
type PaymentsService struct {
	client *Client
}

// PaymentCreateParams contains parameters for creating a payment.
type PaymentCreateParams struct {
	Creditor        Creditor   `json:"creditor"`
	Amount          Amount     `json:"amount"`
	DebtorAccountID string     `json:"debtor_account_id"`
	Reference       *string    `json:"reference,omitempty"`
	EndToEndID      *string    `json:"end_to_end_id,omitempty"`
	ExecutionDate   *time.Time `json:"execution_date,omitempty"`
}

// Create creates a new payment.
func (s *PaymentsService) Create(ctx context.Context, params PaymentCreateParams, opts ...RequestOption) (*Payment, error) {
	body := map[string]interface{}{
		"creditor": map[string]interface{}{
			"name": params.Creditor.Name,
			"account": map[string]interface{}{
				"iban": params.Creditor.Account.IBAN,
				"bban": params.Creditor.Account.BBAN,
			},
		},
		"amount": map[string]interface{}{
			"amount":   params.Amount.Amount,
			"currency": params.Amount.Currency,
		},
		"debtor_account_id": params.DebtorAccountID,
	}
	if params.Reference != nil {
		body["reference"] = *params.Reference
	}
	if params.EndToEndID != nil {
		body["end_to_end_id"] = *params.EndToEndID
	}
	if params.ExecutionDate != nil {
		body["execution_date"] = params.ExecutionDate.Format("2006-01-02")
	}

	var payment Payment
	if err := s.client.request(ctx, "POST", "/payments", nil, body, &payment, opts...); err != nil {
		return nil, err
	}
	return &payment, nil
}

// Get gets payment status.
func (s *PaymentsService) Get(ctx context.Context, paymentID string) (*Payment, error) {
	var payment Payment
	if err := s.client.request(ctx, "GET", "/payments/"+paymentID, nil, nil, &payment); err != nil {
		return nil, err
	}
	return &payment, nil
}

// PaymentListParams contains parameters for listing payments.
type PaymentListParams struct {
	Status *string
	Limit  *int
	Offset *int
}

// List lists payments.
func (s *PaymentsService) List(ctx context.Context, params *PaymentListParams) ([]Payment, error) {
	values := url.Values{}
	if params != nil {
		if params.Status != nil {
			values.Set("status", *params.Status)
		}
		if params.Limit != nil {
			values.Set("limit", strconv.Itoa(*params.Limit))
		}
		if params.Offset != nil {
			values.Set("offset", strconv.Itoa(*params.Offset))
		}
	}

	var result struct {
		Payments []Payment `json:"payments"`
	}
	if err := s.client.request(ctx, "GET", "/payments", values, nil, &result); err != nil {
		return nil, err
	}
	return result.Payments, nil
}

// Cancel cancels a pending payment.
func (s *PaymentsService) Cancel(ctx context.Context, paymentID string) (*Payment, error) {
	var payment Payment
	if err := s.client.request(ctx, "POST", "/payments/"+paymentID+"/cancel", nil, nil, &payment); err != nil {
		return nil, err
	}
	return &payment, nil
}

// ConsentsService provides access to the Consents API.
type ConsentsService struct {
	client *Client
}

// ConsentCreateParams contains parameters for creating a consent.
type ConsentCreateParams struct {
	Access             []string `json:"access"`
	ValidUntil         *string  `json:"valid_until,omitempty"`
	RecurringIndicator *bool    `json:"recurring_indicator,omitempty"`
	FrequencyPerDay    *int     `json:"frequency_per_day,omitempty"`
}

// Create creates a new consent.
func (s *ConsentsService) Create(ctx context.Context, params ConsentCreateParams) (*Consent, error) {
	var consent Consent
	if err := s.client.request(ctx, "POST", "/consents", nil, params, &consent); err != nil {
		return nil, err
	}
	return &consent, nil
}

// Get gets consent status.
func (s *ConsentsService) Get(ctx context.Context, consentID string) (*Consent, error) {
	var consent Consent
	if err := s.client.request(ctx, "GET", "/consents/"+consentID, nil, nil, &consent); err != nil {
		return nil, err
	}
	return &consent, nil
}

// Revoke revokes a consent.
func (s *ConsentsService) Revoke(ctx context.Context, consentID string) error {
	return s.client.request(ctx, "DELETE", "/consents/"+consentID, nil, nil, nil)
}

// List lists all consents.
func (s *ConsentsService) List(ctx context.Context) ([]Consent, error) {
	var result struct {
		Consents []Consent `json:"consents"`
	}
	if err := s.client.request(ctx, "GET", "/consents", nil, nil, &result); err != nil {
		return nil, err
	}
	return result.Consents, nil
}

// InstitutionsService provides access to the Institutions API.
type InstitutionsService struct {
	client *Client
}

// InstitutionListParams contains parameters for listing institutions.
type InstitutionListParams struct {
	Country *string
	Query   *string
	Limit   *int
	Offset  *int
}

// List lists financial institutions.
func (s *InstitutionsService) List(ctx context.Context, params *InstitutionListParams) ([]Institution, error) {
	values := url.Values{}
	if params != nil {
		if params.Country != nil {
			values.Set("country", *params.Country)
		}
		if params.Query != nil {
			values.Set("query", *params.Query)
		}
		if params.Limit != nil {
			values.Set("limit", strconv.Itoa(*params.Limit))
		}
		if params.Offset != nil {
			values.Set("offset", strconv.Itoa(*params.Offset))
		}
	}

	var result struct {
		Institutions []Institution `json:"institutions"`
	}
	if err := s.client.request(ctx, "GET", "/institutions", values, nil, &result); err != nil {
		return nil, err
	}
	return result.Institutions, nil
}

// Get gets institution details.
func (s *InstitutionsService) Get(ctx context.Context, institutionID string) (*Institution, error) {
	var institution Institution
	if err := s.client.request(ctx, "GET", "/institutions/"+institutionID, nil, nil, &institution); err != nil {
		return nil, err
	}
	return &institution, nil
}

// AuthService provides authentication methods.
type AuthService struct {
	client *Client
}

// GetAuthorizationURL generates an OAuth authorization URL.
func (s *AuthService) GetAuthorizationURL(redirectURI string, scopes []string, state string) string {
	params := url.Values{}
	params.Set("client_id", s.client.config.ClientID)
	params.Set("redirect_uri", redirectURI)
	params.Set("response_type", "code")
	params.Set("scope", strings.Join(scopes, " "))
	if state != "" {
		params.Set("state", state)
	}
	return s.client.BaseURL() + "/oauth/authorize?" + params.Encode()
}

// ExchangeCodeParams contains parameters for exchanging an authorization code.
type ExchangeCodeParams struct {
	Code        string
	RedirectURI string
}

// ExchangeCode exchanges an authorization code for tokens.
func (s *AuthService) ExchangeCode(ctx context.Context, params ExchangeCodeParams) (*TokenResponse, error) {
	data := url.Values{}
	data.Set("grant_type", "authorization_code")
	data.Set("client_id", s.client.config.ClientID)
	data.Set("client_secret", s.client.config.ClientSecret)
	data.Set("code", params.Code)
	data.Set("redirect_uri", params.RedirectURI)

	req, err := http.NewRequestWithContext(ctx, "POST", s.client.BaseURL()+"/oauth/token",
		strings.NewReader(data.Encode()))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := s.client.httpClient.Do(req)
	if err != nil {
		return nil, &NetworkError{Message: err.Error()}
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return nil, &AuthenticationError{Message: fmt.Sprintf("failed to exchange code: %d", resp.StatusCode)}
	}

	var tokens TokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&tokens); err != nil {
		return nil, err
	}
	return &tokens, nil
}

// RefreshToken refreshes an access token.
func (s *AuthService) RefreshToken(ctx context.Context, refreshToken string) (*TokenResponse, error) {
	data := url.Values{}
	data.Set("grant_type", "refresh_token")
	data.Set("client_id", s.client.config.ClientID)
	data.Set("client_secret", s.client.config.ClientSecret)
	data.Set("refresh_token", refreshToken)

	req, err := http.NewRequestWithContext(ctx, "POST", s.client.BaseURL()+"/oauth/token",
		strings.NewReader(data.Encode()))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := s.client.httpClient.Do(req)
	if err != nil {
		return nil, &NetworkError{Message: err.Error()}
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return nil, &AuthenticationError{Message: fmt.Sprintf("failed to refresh token: %d", resp.StatusCode)}
	}

	var tokens TokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&tokens); err != nil {
		return nil, err
	}
	return &tokens, nil
}

// requestToken requests an access token using client credentials.
func (s *AuthService) requestToken(ctx context.Context) (*TokenResponse, error) {
	data := url.Values{}
	data.Set("grant_type", "client_credentials")
	data.Set("client_id", s.client.config.ClientID)
	data.Set("client_secret", s.client.config.ClientSecret)

	req, err := http.NewRequestWithContext(ctx, "POST", s.client.BaseURL()+"/oauth/token",
		strings.NewReader(data.Encode()))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := s.client.httpClient.Do(req)
	if err != nil {
		return nil, &NetworkError{Message: err.Error()}
	}
	defer resp.Body.Close()

	if resp.StatusCode != 200 {
		return nil, &AuthenticationError{Message: fmt.Sprintf("failed to obtain token: %d", resp.StatusCode)}
	}

	var tokens TokenResponse
	if err := json.NewDecoder(resp.Body).Decode(&tokens); err != nil {
		return nil, err
	}
	return &tokens, nil
}

// RealtimeService provides WebSocket functionality.
type RealtimeService struct {
	client *Client
}

// EventType represents a real-time event type.
type EventType string

const (
	// EventTransactionCreated is fired when a new transaction is created.
	EventTransactionCreated EventType = "transaction.created"
	// EventTransactionUpdated is fired when a transaction is updated.
	EventTransactionUpdated EventType = "transaction.updated"
	// EventBalanceUpdated is fired when an account balance changes.
	EventBalanceUpdated EventType = "balance.updated"
	// EventPaymentStatusChanged is fired when a payment status changes.
	EventPaymentStatusChanged EventType = "payment.status_changed"
	// EventConsentRevoked is fired when a consent is revoked.
	EventConsentRevoked EventType = "consent.revoked"
)

// TransactionEvent represents a transaction event.
type TransactionEvent struct {
	Type      EventType    `json:"type"`
	Data      Transaction  `json:"data"`
	Timestamp time.Time    `json:"timestamp"`
}

// BalanceEvent represents a balance event.
type BalanceEvent struct {
	Type      EventType `json:"type"`
	Data      Balance   `json:"data"`
	Timestamp time.Time `json:"timestamp"`
}

// PaymentEvent represents a payment event.
type PaymentEvent struct {
	Type      EventType `json:"type"`
	Data      Payment   `json:"data"`
	Timestamp time.Time `json:"timestamp"`
}

// EventHandlers contains handlers for real-time events.
type EventHandlers struct {
	OnTransactionCreated   func(TransactionEvent)
	OnTransactionUpdated   func(TransactionEvent)
	OnBalanceUpdated       func(BalanceEvent)
	OnPaymentStatusChanged func(PaymentEvent)
	OnConsentRevoked       func(event struct{ ConsentID string })
	OnError                func(error)
}

// SubscribeParams contains parameters for subscribing to events.
type SubscribeParams struct {
	AccountID string
	Events    []EventType
	Handlers  EventHandlers
}

// Subscription represents a WebSocket subscription.
type Subscription struct {
	done chan struct{}
}

// Wait waits for the subscription to complete.
func (s *Subscription) Wait() error {
	<-s.done
	return nil
}

// Close closes the subscription.
func (s *Subscription) Close() {
	close(s.done)
}

// Subscribe subscribes to real-time events.
// Note: This is a placeholder implementation. Full WebSocket implementation
// would require a WebSocket library like gorilla/websocket.
func (s *RealtimeService) Subscribe(ctx context.Context, params SubscribeParams) (*Subscription, error) {
	// In a full implementation, this would:
	// 1. Connect to WebSocket endpoint
	// 2. Send subscription message
	// 3. Handle incoming events

	return &Subscription{
		done: make(chan struct{}),
	}, nil
}
