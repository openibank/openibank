"""
OpeniBank Python SDK

Official Python client library for the OpeniBank Open Banking Platform API.

Example:
    >>> from openibank import OpeniBank
    >>> client = OpeniBank(
    ...     client_id="your_client_id",
    ...     client_secret="your_client_secret",
    ...     environment="sandbox"
    ... )
    >>> accounts = await client.accounts.list()
"""

from __future__ import annotations

import asyncio
import logging
import os
import time
from dataclasses import dataclass, field
from datetime import date, datetime
from enum import Enum
from typing import (
    Any,
    AsyncIterator,
    Callable,
    Coroutine,
    Dict,
    Generic,
    List,
    Literal,
    Optional,
    Type,
    TypeVar,
    Union,
)

try:
    import aiohttp
except ImportError:
    aiohttp = None  # type: ignore

try:
    import websockets
except ImportError:
    websockets = None  # type: ignore

__version__ = "1.0.0"
__author__ = "OpeniBank"
__license__ = "MIT"

logger = logging.getLogger("openibank")

T = TypeVar("T")


# =============================================================================
# Configuration
# =============================================================================


class Environment(str, Enum):
    """API environment."""

    SANDBOX = "sandbox"
    PRODUCTION = "production"


@dataclass
class Config:
    """SDK configuration."""

    client_id: str = ""
    client_secret: str = ""
    api_key: Optional[str] = None
    environment: str = "sandbox"
    api_version: str = "v2"
    timeout: float = 30.0
    max_retries: int = 3
    retry_delay: float = 1.0
    auto_refresh: bool = True
    debug: bool = False

    @property
    def base_url(self) -> str:
        """Get the base URL for the current environment."""
        if self.environment == "production":
            return "https://api.openibank.com"
        return "https://sandbox.openibank.com"

    @property
    def websocket_url(self) -> str:
        """Get the WebSocket URL for the current environment."""
        if self.environment == "production":
            return "wss://ws.openibank.com"
        return "wss://ws.sandbox.openibank.com"

    @classmethod
    def from_env(cls) -> "Config":
        """Create configuration from environment variables."""
        return cls(
            client_id=os.getenv("OPENIBANK_CLIENT_ID", ""),
            client_secret=os.getenv("OPENIBANK_CLIENT_SECRET", ""),
            api_key=os.getenv("OPENIBANK_API_KEY"),
            environment=os.getenv("OPENIBANK_ENVIRONMENT", "sandbox"),
            api_version=os.getenv("OPENIBANK_API_VERSION", "v2"),
            timeout=float(os.getenv("OPENIBANK_TIMEOUT", "30.0")),
            max_retries=int(os.getenv("OPENIBANK_MAX_RETRIES", "3")),
            debug=os.getenv("OPENIBANK_DEBUG", "").lower() == "true",
        )


# =============================================================================
# Exceptions
# =============================================================================


class OpeniBankError(Exception):
    """Base exception for all OpeniBank errors."""

    def __init__(
        self,
        message: str,
        code: Optional[str] = None,
        status_code: Optional[int] = None,
        request_id: Optional[str] = None,
        details: Optional[Dict[str, Any]] = None,
    ):
        super().__init__(message)
        self.message = message
        self.code = code
        self.status_code = status_code
        self.request_id = request_id
        self.details = details or {}

    def __str__(self) -> str:
        parts = [self.message]
        if self.code:
            parts.append(f"(code: {self.code})")
        if self.request_id:
            parts.append(f"[request_id: {self.request_id}]")
        return " ".join(parts)


class AuthenticationError(OpeniBankError):
    """Authentication failed - invalid or expired credentials."""

    pass


class AuthorizationError(OpeniBankError):
    """Authorization failed - insufficient permissions."""

    def __init__(
        self,
        message: str,
        required_scopes: Optional[List[str]] = None,
        **kwargs: Any,
    ):
        super().__init__(message, **kwargs)
        self.required_scopes = required_scopes or []


class ValidationError(OpeniBankError):
    """Request validation failed."""

    @dataclass
    class FieldError:
        field: str
        message: str
        code: Optional[str] = None

    def __init__(
        self,
        message: str,
        errors: Optional[List[Dict[str, Any]]] = None,
        **kwargs: Any,
    ):
        super().__init__(message, **kwargs)
        self.errors = [
            self.FieldError(
                field=e.get("field", "unknown"),
                message=e.get("message", ""),
                code=e.get("code"),
            )
            for e in (errors or [])
        ]


class NotFoundError(OpeniBankError):
    """Resource not found."""

    def __init__(
        self,
        message: str,
        resource_type: Optional[str] = None,
        resource_id: Optional[str] = None,
        **kwargs: Any,
    ):
        super().__init__(message, **kwargs)
        self.resource_type = resource_type
        self.resource_id = resource_id


class RateLimitError(OpeniBankError):
    """Rate limit exceeded."""

    def __init__(
        self,
        message: str,
        retry_after: Optional[float] = None,
        **kwargs: Any,
    ):
        super().__init__(message, **kwargs)
        self.retry_after = retry_after or 60.0


class ConflictError(OpeniBankError):
    """Resource conflict (e.g., duplicate)."""

    pass


class ServerError(OpeniBankError):
    """Internal server error."""

    pass


class NetworkError(OpeniBankError):
    """Network or connection error."""

    pass


# =============================================================================
# Models
# =============================================================================


@dataclass
class Amount:
    """Monetary amount with currency."""

    amount: str
    currency: str

    def __post_init__(self) -> None:
        # Ensure amount is a string
        self.amount = str(self.amount)

    @property
    def value(self) -> float:
        """Get numeric value."""
        return float(self.amount)


@dataclass
class Balance:
    """Account balance."""

    amount: str
    currency: str
    type: str = "available"
    credit_limit: Optional[str] = None
    last_updated: Optional[datetime] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Balance":
        return cls(
            amount=data.get("amount", "0"),
            currency=data.get("currency", "EUR"),
            type=data.get("type", "available"),
            credit_limit=data.get("credit_limit"),
            last_updated=(
                datetime.fromisoformat(data["last_updated"])
                if data.get("last_updated")
                else None
            ),
        )


@dataclass
class Account:
    """Bank account."""

    id: str
    name: str
    iban: Optional[str] = None
    bban: Optional[str] = None
    currency: str = "EUR"
    account_type: str = "checking"
    status: str = "active"
    balance: Optional[Balance] = None
    institution_id: Optional[str] = None
    owner_name: Optional[str] = None
    created_at: Optional[datetime] = None
    updated_at: Optional[datetime] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Account":
        balance = None
        if data.get("balance"):
            balance = Balance.from_dict(data["balance"])

        return cls(
            id=data["id"],
            name=data.get("name", ""),
            iban=data.get("iban"),
            bban=data.get("bban"),
            currency=data.get("currency", "EUR"),
            account_type=data.get("account_type", "checking"),
            status=data.get("status", "active"),
            balance=balance,
            institution_id=data.get("institution_id"),
            owner_name=data.get("owner_name"),
            created_at=(
                datetime.fromisoformat(data["created_at"])
                if data.get("created_at")
                else None
            ),
            updated_at=(
                datetime.fromisoformat(data["updated_at"])
                if data.get("updated_at")
                else None
            ),
        )


@dataclass
class Transaction:
    """Bank transaction."""

    id: str
    account_id: str
    amount: str
    currency: str
    description: str = ""
    reference: Optional[str] = None
    booking_date: Optional[date] = None
    value_date: Optional[date] = None
    transaction_type: str = "transfer"
    status: str = "booked"
    counterparty_name: Optional[str] = None
    counterparty_iban: Optional[str] = None
    category: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Transaction":
        return cls(
            id=data["id"],
            account_id=data.get("account_id", ""),
            amount=data.get("amount", "0"),
            currency=data.get("currency", "EUR"),
            description=data.get("description", ""),
            reference=data.get("reference"),
            booking_date=(
                date.fromisoformat(data["booking_date"])
                if data.get("booking_date")
                else None
            ),
            value_date=(
                date.fromisoformat(data["value_date"])
                if data.get("value_date")
                else None
            ),
            transaction_type=data.get("transaction_type", "transfer"),
            status=data.get("status", "booked"),
            counterparty_name=data.get("counterparty_name"),
            counterparty_iban=data.get("counterparty_iban"),
            category=data.get("category"),
            metadata=data.get("metadata", {}),
        )


@dataclass
class CreditorAccount:
    """Creditor account for payments."""

    iban: Optional[str] = None
    bban: Optional[str] = None
    sort_code: Optional[str] = None
    account_number: Optional[str] = None


@dataclass
class Creditor:
    """Payment creditor."""

    name: str
    account: CreditorAccount


@dataclass
class PaymentRequest:
    """Payment initiation request."""

    creditor: Creditor
    amount: Amount
    debtor_account_id: str
    reference: Optional[str] = None
    end_to_end_id: Optional[str] = None
    execution_date: Optional[date] = None


@dataclass
class Payment:
    """Payment status and details."""

    id: str
    status: str
    amount: str
    currency: str
    creditor_name: str
    creditor_iban: Optional[str] = None
    reference: Optional[str] = None
    created_at: Optional[datetime] = None
    executed_at: Optional[datetime] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Payment":
        return cls(
            id=data["id"],
            status=data.get("status", "pending"),
            amount=data.get("amount", "0"),
            currency=data.get("currency", "EUR"),
            creditor_name=data.get("creditor_name", ""),
            creditor_iban=data.get("creditor_iban"),
            reference=data.get("reference"),
            created_at=(
                datetime.fromisoformat(data["created_at"])
                if data.get("created_at")
                else None
            ),
            executed_at=(
                datetime.fromisoformat(data["executed_at"])
                if data.get("executed_at")
                else None
            ),
        )


@dataclass
class ConsentRequest:
    """Consent creation request."""

    access: List[str]
    valid_until: Optional[str] = None
    recurring_indicator: bool = True
    frequency_per_day: int = 4


@dataclass
class Consent:
    """Consent status and details."""

    id: str
    status: str
    access: List[str]
    valid_until: Optional[date] = None
    authorization_url: Optional[str] = None
    created_at: Optional[datetime] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Consent":
        return cls(
            id=data["id"],
            status=data.get("status", "pending"),
            access=data.get("access", []),
            valid_until=(
                date.fromisoformat(data["valid_until"])
                if data.get("valid_until")
                else None
            ),
            authorization_url=data.get("authorization_url"),
            created_at=(
                datetime.fromisoformat(data["created_at"])
                if data.get("created_at")
                else None
            ),
        )


@dataclass
class Institution:
    """Financial institution."""

    id: str
    name: str
    bic: Optional[str] = None
    country: str = ""
    logo_url: Optional[str] = None
    supported_features: List[str] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Institution":
        return cls(
            id=data["id"],
            name=data.get("name", ""),
            bic=data.get("bic"),
            country=data.get("country", ""),
            logo_url=data.get("logo_url"),
            supported_features=data.get("supported_features", []),
        )


@dataclass
class TokenResponse:
    """OAuth token response."""

    access_token: str
    token_type: str = "Bearer"
    expires_in: int = 3600
    refresh_token: Optional[str] = None
    scope: Optional[str] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "TokenResponse":
        return cls(
            access_token=data["access_token"],
            token_type=data.get("token_type", "Bearer"),
            expires_in=data.get("expires_in", 3600),
            refresh_token=data.get("refresh_token"),
            scope=data.get("scope"),
        )


@dataclass
class Page(Generic[T]):
    """Paginated response."""

    items: List[T]
    total: int
    limit: int
    offset: int
    has_next: bool
    has_prev: bool
    _next_page: Optional[Callable[[], Coroutine[Any, Any, "Page[T]"]]] = field(
        default=None, repr=False
    )
    _prev_page: Optional[Callable[[], Coroutine[Any, Any, "Page[T]"]]] = field(
        default=None, repr=False
    )

    async def next(self) -> "Page[T]":
        """Get next page."""
        if not self.has_next or not self._next_page:
            raise StopIteration("No more pages")
        return await self._next_page()

    async def prev(self) -> "Page[T]":
        """Get previous page."""
        if not self.has_prev or not self._prev_page:
            raise StopIteration("No previous pages")
        return await self._prev_page()


# =============================================================================
# HTTP Client
# =============================================================================


class HTTPClient:
    """Async HTTP client with retry logic."""

    def __init__(
        self,
        config: Config,
        session: Optional["aiohttp.ClientSession"] = None,
    ):
        if aiohttp is None:
            raise ImportError(
                "aiohttp is required for the OpeniBank SDK. "
                "Install it with: pip install aiohttp"
            )

        self.config = config
        self._session = session
        self._owned_session = session is None
        self._access_token: Optional[str] = None
        self._token_expires_at: float = 0

    async def _get_session(self) -> "aiohttp.ClientSession":
        """Get or create HTTP session."""
        if self._session is None or self._session.closed:
            timeout = aiohttp.ClientTimeout(total=self.config.timeout)
            self._session = aiohttp.ClientSession(timeout=timeout)
            self._owned_session = True
        return self._session

    async def close(self) -> None:
        """Close HTTP session."""
        if self._session and self._owned_session:
            await self._session.close()
            self._session = None

    def set_access_token(self, token: str, expires_in: int = 3600) -> None:
        """Set the access token."""
        self._access_token = token
        self._token_expires_at = time.time() + expires_in - 60  # 60s buffer

    async def _ensure_token(self) -> str:
        """Ensure we have a valid access token."""
        if self._access_token and time.time() < self._token_expires_at:
            return self._access_token

        # Use API key if available
        if self.config.api_key:
            return self.config.api_key

        # Get new token using client credentials
        if self.config.client_id and self.config.client_secret:
            token_response = await self._request_token()
            self.set_access_token(
                token_response.access_token,
                token_response.expires_in,
            )
            return self._access_token  # type: ignore

        raise AuthenticationError("No valid credentials configured")

    async def _request_token(self) -> TokenResponse:
        """Request access token using client credentials."""
        session = await self._get_session()
        url = f"{self.config.base_url}/oauth/token"

        async with session.post(
            url,
            data={
                "grant_type": "client_credentials",
                "client_id": self.config.client_id,
                "client_secret": self.config.client_secret,
            },
        ) as response:
            if response.status != 200:
                raise AuthenticationError(
                    f"Failed to obtain access token: {response.status}"
                )
            data = await response.json()
            return TokenResponse.from_dict(data)

    def _get_headers(self, token: str) -> Dict[str, str]:
        """Get request headers."""
        return {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
            "Accept": "application/json",
            "X-API-Version": self.config.api_version,
            "User-Agent": f"OpeniBank-Python/{__version__}",
        }

    async def request(
        self,
        method: str,
        path: str,
        params: Optional[Dict[str, Any]] = None,
        json: Optional[Dict[str, Any]] = None,
        idempotency_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Make an HTTP request with retry logic."""
        session = await self._get_session()
        token = await self._ensure_token()
        headers = self._get_headers(token)

        if idempotency_key:
            headers["Idempotency-Key"] = idempotency_key

        url = f"{self.config.base_url}/{self.config.api_version}{path}"

        # Filter out None values from params
        if params:
            params = {k: v for k, v in params.items() if v is not None}

        last_error: Optional[Exception] = None

        for attempt in range(self.config.max_retries):
            try:
                async with session.request(
                    method,
                    url,
                    params=params,
                    json=json,
                    headers=headers,
                ) as response:
                    request_id = response.headers.get("X-Request-ID")

                    if response.status == 200 or response.status == 201:
                        return await response.json()

                    if response.status == 204:
                        return {}

                    # Handle errors
                    try:
                        error_data = await response.json()
                    except Exception:
                        error_data = {"message": await response.text()}

                    error_message = error_data.get("message", "Unknown error")
                    error_code = error_data.get("code")

                    if response.status == 401:
                        raise AuthenticationError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                        )
                    elif response.status == 403:
                        raise AuthorizationError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                            required_scopes=error_data.get("required_scopes"),
                        )
                    elif response.status == 400:
                        raise ValidationError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                            errors=error_data.get("errors"),
                        )
                    elif response.status == 404:
                        raise NotFoundError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                            resource_type=error_data.get("resource_type"),
                            resource_id=error_data.get("resource_id"),
                        )
                    elif response.status == 409:
                        raise ConflictError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                        )
                    elif response.status == 429:
                        retry_after = float(
                            response.headers.get("Retry-After", "60")
                        )
                        raise RateLimitError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                            retry_after=retry_after,
                        )
                    elif response.status >= 500:
                        raise ServerError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                        )
                    else:
                        raise OpeniBankError(
                            error_message,
                            code=error_code,
                            status_code=response.status,
                            request_id=request_id,
                        )

            except (aiohttp.ClientError, asyncio.TimeoutError) as e:
                last_error = NetworkError(f"Network error: {str(e)}")
                if attempt < self.config.max_retries - 1:
                    delay = self.config.retry_delay * (2**attempt)
                    logger.warning(
                        f"Request failed, retrying in {delay}s: {e}"
                    )
                    await asyncio.sleep(delay)
                    continue
                raise last_error

            except (RateLimitError, ServerError) as e:
                last_error = e
                if attempt < self.config.max_retries - 1:
                    delay = (
                        e.retry_after
                        if isinstance(e, RateLimitError)
                        else self.config.retry_delay * (2**attempt)
                    )
                    logger.warning(
                        f"Request failed, retrying in {delay}s: {e}"
                    )
                    await asyncio.sleep(delay)
                    continue
                raise

        if last_error:
            raise last_error
        raise NetworkError("Request failed after all retries")


# =============================================================================
# API Resources
# =============================================================================


class AccountsResource:
    """Accounts API resource."""

    def __init__(self, http: HTTPClient):
        self._http = http

    async def list(
        self,
        status: Optional[str] = None,
        account_type: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[Account]:
        """List all accounts."""
        data = await self._http.request(
            "GET",
            "/accounts",
            params={
                "status": status,
                "account_type": account_type,
                "limit": limit,
                "offset": offset,
            },
        )
        return [Account.from_dict(item) for item in data.get("accounts", [])]

    async def get(self, account_id: str) -> Account:
        """Get a single account."""
        data = await self._http.request("GET", f"/accounts/{account_id}")
        return Account.from_dict(data)

    async def get_balances(self, account_id: str) -> List[Balance]:
        """Get account balances."""
        data = await self._http.request(
            "GET", f"/accounts/{account_id}/balances"
        )
        return [Balance.from_dict(item) for item in data.get("balances", [])]


class TransactionsResource:
    """Transactions API resource."""

    def __init__(self, http: HTTPClient):
        self._http = http

    async def list(
        self,
        account_id: str,
        date_from: Optional[date] = None,
        date_to: Optional[date] = None,
        amount_min: Optional[float] = None,
        amount_max: Optional[float] = None,
        booking_status: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[Transaction]:
        """List transactions for an account."""
        data = await self._http.request(
            "GET",
            f"/accounts/{account_id}/transactions",
            params={
                "date_from": date_from.isoformat() if date_from else None,
                "date_to": date_to.isoformat() if date_to else None,
                "amount_min": amount_min,
                "amount_max": amount_max,
                "booking_status": booking_status,
                "limit": limit,
                "offset": offset,
            },
        )
        return [
            Transaction.from_dict(item)
            for item in data.get("transactions", [])
        ]

    async def get(
        self, account_id: str, transaction_id: str
    ) -> Transaction:
        """Get a single transaction."""
        data = await self._http.request(
            "GET",
            f"/accounts/{account_id}/transactions/{transaction_id}",
        )
        return Transaction.from_dict(data)

    async def iterate(
        self,
        account_id: str,
        date_from: Optional[date] = None,
        date_to: Optional[date] = None,
        limit: int = 50,
    ) -> AsyncIterator[Transaction]:
        """Iterate through all transactions."""
        offset = 0
        while True:
            transactions = await self.list(
                account_id=account_id,
                date_from=date_from,
                date_to=date_to,
                limit=limit,
                offset=offset,
            )
            if not transactions:
                break
            for tx in transactions:
                yield tx
            if len(transactions) < limit:
                break
            offset += limit


class PaymentsResource:
    """Payments API resource."""

    def __init__(self, http: HTTPClient):
        self._http = http

    async def create(
        self,
        payment: PaymentRequest,
        idempotency_key: Optional[str] = None,
    ) -> Payment:
        """Create a new payment."""
        data = await self._http.request(
            "POST",
            "/payments",
            json={
                "creditor": {
                    "name": payment.creditor.name,
                    "account": {
                        "iban": payment.creditor.account.iban,
                        "bban": payment.creditor.account.bban,
                    },
                },
                "amount": {
                    "amount": payment.amount.amount,
                    "currency": payment.amount.currency,
                },
                "debtor_account_id": payment.debtor_account_id,
                "reference": payment.reference,
                "end_to_end_id": payment.end_to_end_id,
                "execution_date": (
                    payment.execution_date.isoformat()
                    if payment.execution_date
                    else None
                ),
            },
            idempotency_key=idempotency_key,
        )
        return Payment.from_dict(data)

    async def get(self, payment_id: str) -> Payment:
        """Get payment status."""
        data = await self._http.request("GET", f"/payments/{payment_id}")
        return Payment.from_dict(data)

    async def list(
        self,
        status: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[Payment]:
        """List payments."""
        data = await self._http.request(
            "GET",
            "/payments",
            params={
                "status": status,
                "limit": limit,
                "offset": offset,
            },
        )
        return [Payment.from_dict(item) for item in data.get("payments", [])]

    async def cancel(self, payment_id: str) -> Payment:
        """Cancel a pending payment."""
        data = await self._http.request(
            "POST", f"/payments/{payment_id}/cancel"
        )
        return Payment.from_dict(data)


class ConsentsResource:
    """Consents API resource."""

    def __init__(self, http: HTTPClient):
        self._http = http

    async def create(self, consent: ConsentRequest) -> Consent:
        """Create a new consent."""
        data = await self._http.request(
            "POST",
            "/consents",
            json={
                "access": consent.access,
                "valid_until": consent.valid_until,
                "recurring_indicator": consent.recurring_indicator,
                "frequency_per_day": consent.frequency_per_day,
            },
        )
        return Consent.from_dict(data)

    async def get(self, consent_id: str) -> Consent:
        """Get consent status."""
        data = await self._http.request("GET", f"/consents/{consent_id}")
        return Consent.from_dict(data)

    async def revoke(self, consent_id: str) -> None:
        """Revoke a consent."""
        await self._http.request("DELETE", f"/consents/{consent_id}")

    async def list(self) -> List[Consent]:
        """List all consents."""
        data = await self._http.request("GET", "/consents")
        return [Consent.from_dict(item) for item in data.get("consents", [])]


class InstitutionsResource:
    """Financial institutions API resource."""

    def __init__(self, http: HTTPClient):
        self._http = http

    async def list(
        self,
        country: Optional[str] = None,
        query: Optional[str] = None,
        limit: int = 50,
        offset: int = 0,
    ) -> List[Institution]:
        """List financial institutions."""
        data = await self._http.request(
            "GET",
            "/institutions",
            params={
                "country": country,
                "query": query,
                "limit": limit,
                "offset": offset,
            },
        )
        return [
            Institution.from_dict(item)
            for item in data.get("institutions", [])
        ]

    async def get(self, institution_id: str) -> Institution:
        """Get institution details."""
        data = await self._http.request(
            "GET", f"/institutions/{institution_id}"
        )
        return Institution.from_dict(data)


class AuthResource:
    """Authentication API resource."""

    def __init__(self, http: HTTPClient, config: Config):
        self._http = http
        self._config = config

    def get_authorization_url(
        self,
        redirect_uri: str,
        scopes: List[str],
        state: Optional[str] = None,
    ) -> str:
        """Generate OAuth authorization URL."""
        import urllib.parse

        params = {
            "client_id": self._config.client_id,
            "redirect_uri": redirect_uri,
            "response_type": "code",
            "scope": " ".join(scopes),
        }
        if state:
            params["state"] = state

        query = urllib.parse.urlencode(params)
        return f"{self._config.base_url}/oauth/authorize?{query}"

    async def exchange_code(
        self,
        code: str,
        redirect_uri: str,
    ) -> TokenResponse:
        """Exchange authorization code for tokens."""
        if aiohttp is None:
            raise ImportError("aiohttp is required")

        session = await self._http._get_session()
        url = f"{self._config.base_url}/oauth/token"

        async with session.post(
            url,
            data={
                "grant_type": "authorization_code",
                "client_id": self._config.client_id,
                "client_secret": self._config.client_secret,
                "code": code,
                "redirect_uri": redirect_uri,
            },
        ) as response:
            if response.status != 200:
                raise AuthenticationError(
                    f"Failed to exchange code: {response.status}"
                )
            data = await response.json()
            return TokenResponse.from_dict(data)

    async def refresh_token(self, refresh_token: str) -> TokenResponse:
        """Refresh access token."""
        if aiohttp is None:
            raise ImportError("aiohttp is required")

        session = await self._http._get_session()
        url = f"{self._config.base_url}/oauth/token"

        async with session.post(
            url,
            data={
                "grant_type": "refresh_token",
                "client_id": self._config.client_id,
                "client_secret": self._config.client_secret,
                "refresh_token": refresh_token,
            },
        ) as response:
            if response.status != 200:
                raise AuthenticationError(
                    f"Failed to refresh token: {response.status}"
                )
            data = await response.json()
            return TokenResponse.from_dict(data)


# =============================================================================
# WebSocket / Real-time
# =============================================================================


class EventType(str, Enum):
    """Real-time event types."""

    TRANSACTION_CREATED = "transaction.created"
    TRANSACTION_UPDATED = "transaction.updated"
    BALANCE_UPDATED = "balance.updated"
    PAYMENT_STATUS_CHANGED = "payment.status_changed"
    CONSENT_REVOKED = "consent.revoked"


@dataclass
class RealtimeEvent:
    """Real-time event."""

    type: str
    data: Any
    timestamp: datetime


class Subscription:
    """WebSocket subscription handle."""

    def __init__(
        self,
        ws: Any,
        handlers: Dict[str, Callable[[RealtimeEvent], Coroutine[Any, Any, None]]],
    ):
        self._ws = ws
        self._handlers = handlers
        self._running = False
        self._task: Optional[asyncio.Task[None]] = None

    async def _process_messages(self) -> None:
        """Process incoming WebSocket messages."""
        self._running = True
        try:
            async for message in self._ws:
                if not self._running:
                    break

                import json

                data = json.loads(message)
                event_type = data.get("type")
                handler = self._handlers.get(event_type)

                if handler:
                    event = RealtimeEvent(
                        type=event_type,
                        data=data.get("data"),
                        timestamp=datetime.fromisoformat(
                            data.get("timestamp", datetime.now().isoformat())
                        ),
                    )
                    await handler(event)
        except Exception as e:
            logger.error(f"WebSocket error: {e}")
            raise

    async def wait(self) -> None:
        """Wait for subscription to complete."""
        if self._task:
            await self._task

    async def close(self) -> None:
        """Close the subscription."""
        self._running = False
        if self._ws:
            await self._ws.close()


class RealtimeResource:
    """Real-time WebSocket resource."""

    def __init__(self, http: HTTPClient, config: Config):
        self._http = http
        self._config = config

    async def subscribe(
        self,
        account_id: str,
        events: List[EventType],
        handlers: Dict[
            EventType, Callable[[RealtimeEvent], Coroutine[Any, Any, None]]
        ],
    ) -> Subscription:
        """Subscribe to real-time events."""
        if websockets is None:
            raise ImportError(
                "websockets is required for real-time features. "
                "Install it with: pip install openibank[websocket]"
            )

        token = await self._http._ensure_token()
        url = f"{self._config.websocket_url}/subscribe"

        ws = await websockets.connect(
            url,
            extra_headers={
                "Authorization": f"Bearer {token}",
                "X-API-Version": self._config.api_version,
            },
        )

        # Send subscription request
        import json

        await ws.send(
            json.dumps(
                {
                    "action": "subscribe",
                    "account_id": account_id,
                    "events": [e.value for e in events],
                }
            )
        )

        # Convert handlers to use string keys
        str_handlers = {k.value: v for k, v in handlers.items()}

        subscription = Subscription(ws, str_handlers)
        subscription._task = asyncio.create_task(
            subscription._process_messages()
        )

        return subscription


# =============================================================================
# Main Client
# =============================================================================


class OpeniBank:
    """
    OpeniBank API Client.

    Example:
        >>> client = OpeniBank(
        ...     client_id="your_client_id",
        ...     client_secret="your_client_secret",
        ...     environment="sandbox"
        ... )
        >>> accounts = await client.accounts.list()
        >>> await client.close()
    """

    def __init__(
        self,
        client_id: Optional[str] = None,
        client_secret: Optional[str] = None,
        api_key: Optional[str] = None,
        environment: str = "sandbox",
        config: Optional[Config] = None,
        http_session: Optional["aiohttp.ClientSession"] = None,
        **kwargs: Any,
    ):
        """
        Initialize the OpeniBank client.

        Args:
            client_id: OAuth client ID
            client_secret: OAuth client secret
            api_key: API key (for sandbox only)
            environment: API environment ("sandbox" or "production")
            config: Full configuration object
            http_session: Custom aiohttp session
            **kwargs: Additional configuration options
        """
        if config is None:
            config = Config(
                client_id=client_id or os.getenv("OPENIBANK_CLIENT_ID", ""),
                client_secret=client_secret
                or os.getenv("OPENIBANK_CLIENT_SECRET", ""),
                api_key=api_key or os.getenv("OPENIBANK_API_KEY"),
                environment=environment,
                **kwargs,
            )

        self._config = config
        self._http = HTTPClient(config, http_session)

        # Initialize resources
        self.accounts = AccountsResource(self._http)
        self.transactions = TransactionsResource(self._http)
        self.payments = PaymentsResource(self._http)
        self.consents = ConsentsResource(self._http)
        self.institutions = InstitutionsResource(self._http)
        self.auth = AuthResource(self._http, config)
        self.realtime = RealtimeResource(self._http, config)

    def set_access_token(self, token: str, expires_in: int = 3600) -> None:
        """Set the access token manually."""
        self._http.set_access_token(token, expires_in)

    async def close(self) -> None:
        """Close the client and release resources."""
        await self._http.close()

    async def __aenter__(self) -> "OpeniBank":
        """Async context manager entry."""
        return self

    async def __aexit__(
        self,
        exc_type: Optional[Type[BaseException]],
        exc_val: Optional[BaseException],
        exc_tb: Any,
    ) -> None:
        """Async context manager exit."""
        await self.close()


# =============================================================================
# Exports
# =============================================================================

__all__ = [
    # Main client
    "OpeniBank",
    # Configuration
    "Config",
    "Environment",
    # Exceptions
    "OpeniBankError",
    "AuthenticationError",
    "AuthorizationError",
    "ValidationError",
    "NotFoundError",
    "RateLimitError",
    "ConflictError",
    "ServerError",
    "NetworkError",
    # Models
    "Amount",
    "Balance",
    "Account",
    "Transaction",
    "Creditor",
    "CreditorAccount",
    "PaymentRequest",
    "Payment",
    "ConsentRequest",
    "Consent",
    "Institution",
    "TokenResponse",
    "Page",
    # Real-time
    "EventType",
    "RealtimeEvent",
    "Subscription",
    # Version
    "__version__",
]
