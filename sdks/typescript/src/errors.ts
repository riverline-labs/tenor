/** Base error class for all Tenor SDK errors. */
export class TenorError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'TenorError';
  }
}

/** Thrown when the SDK cannot reach the Tenor evaluator server. */
export class ConnectionError extends TenorError {
  public readonly url: string;
  public declare readonly cause?: Error;

  constructor(url: string, cause?: Error) {
    super(`Failed to connect to Tenor evaluator at ${url}`);
    this.name = 'ConnectionError';
    this.url = url;
    this.cause = cause;
  }
}

/** Thrown when evaluation fails (400 from /evaluate). */
export class EvaluationError extends TenorError {
  public readonly details?: Record<string, unknown>;

  constructor(message: string, details?: Record<string, unknown>) {
    super(message);
    this.name = 'EvaluationError';
    this.details = details;
  }
}

/** Thrown when elaboration fails (400 from /elaborate). */
export class ElaborationError extends TenorError {
  public readonly details?: Record<string, unknown>;

  constructor(message: string, details?: Record<string, unknown>) {
    super(message);
    this.name = 'ElaborationError';
    this.details = details;
  }
}

/** Thrown when a referenced contract is not found (404). */
export class ContractNotFoundError extends TenorError {
  public readonly contractId: string;

  constructor(contractId: string) {
    super(`Contract '${contractId}' not found`);
    this.name = 'ContractNotFoundError';
    this.contractId = contractId;
  }
}
