export type EligibilityGates = Readonly<{
  coverage: boolean;
  design: boolean;
  execution: boolean;
  legality: boolean;
  pinnedInput: boolean;
  replay: boolean;
  reporting: boolean;
}>;

export type EligibilityReason =
  | 'coverage-failed'
  | 'design-failed'
  | 'execution-failed'
  | 'legality-failed'
  | 'partial-rules'
  | 'pinned-input-failed'
  | 'replay-failed'
  | 'reporting-failed'
  | 'unverified-authority';

export type BatchClassification =
  | 'unranked_partial_rules_unverified_authority'
  | 'unranked_unverified_authority'
  | 'ranked';

export type EligibilityReport = Readonly<{
  classification: BatchClassification;
  gates: EligibilityGates;
  ranked: boolean;
  reasons: readonly EligibilityReason[];
}>;

const REASONS = new Set<EligibilityReason>([
  'coverage-failed',
  'design-failed',
  'execution-failed',
  'legality-failed',
  'partial-rules',
  'pinned-input-failed',
  'replay-failed',
  'reporting-failed',
  'unverified-authority',
]);

const CLASSIFICATIONS = new Set<BatchClassification>([
  'unranked_partial_rules_unverified_authority',
  'unranked_unverified_authority',
  'ranked',
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function parseGate(value: unknown, label: string): boolean {
  if (value !== true && value !== false) {
    throw new Error(`eligibility gate ${label} was invalid`);
  }
  return value;
}

/** Parses one public batch or eligibility classification string. */
export function parseBatchClassification(value: unknown): BatchClassification {
  if (typeof value !== 'string' || !CLASSIFICATIONS.has(value as BatchClassification)) {
    throw new Error('batch classification was invalid');
  }
  return value as BatchClassification;
}

function validateEligibilityContract(
  classification: BatchClassification,
  ranked: boolean,
  reasons: readonly EligibilityReason[],
): void {
  if (ranked) {
    if (classification !== 'ranked' || reasons.length > 0) {
      throw new Error('ranked eligibility did not match the expected contract');
    }
    return;
  }
  if (classification === 'ranked') {
    throw new Error('ranked classification requires ranked true');
  }
  if (classification === 'unranked_partial_rules_unverified_authority') {
    if (!reasons.includes('partial-rules') || !reasons.includes('unverified-authority')) {
      throw new Error('partial-rules classification was missing blocking reasons');
    }
    return;
  }
  if (reasons.includes('partial-rules')) {
    throw new Error('rules-complete classification must not include partial-rules');
  }
}

/** Parses a TEST-04 eligibility report and rejects an unexpected ranked claim. */
export function parseEligibilityReport(value: unknown): EligibilityReport {
  if (!isRecord(value)
    || !isRecord(value.gates)
    || !Array.isArray(value.reasons)
    || (value.ranked !== true && value.ranked !== false)) {
    throw new Error('eligibility report did not match the expected contract');
  }
  const classification = parseBatchClassification(value.classification);
  const gates = Object.freeze({
    coverage: parseGate(value.gates.coverage, 'coverage'),
    design: parseGate(value.gates.design, 'design'),
    execution: parseGate(value.gates.execution, 'execution'),
    legality: parseGate(value.gates.legality, 'legality'),
    pinnedInput: parseGate(value.gates.pinnedInput, 'pinnedInput'),
    replay: parseGate(value.gates.replay, 'replay'),
    reporting: parseGate(value.gates.reporting, 'reporting'),
  });
  const reasons = Object.freeze(value.reasons.map((reason) => {
    if (typeof reason !== 'string' || !REASONS.has(reason as EligibilityReason)) {
      throw new Error('eligibility reason was invalid');
    }
    return reason as EligibilityReason;
  }));
  validateEligibilityContract(classification, value.ranked, reasons);
  return Object.freeze({
    classification,
    gates,
    ranked: value.ranked,
    reasons,
  });
}
