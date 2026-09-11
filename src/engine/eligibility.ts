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

export type EligibilityReport = Readonly<{
  classification: 'unranked_partial_rules_unverified_authority';
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function parseGate(value: unknown, label: string): boolean {
  if (value !== true && value !== false) {
    throw new Error(`eligibility gate ${label} was invalid`);
  }
  return value;
}

/** Parses a TEST-04 eligibility report and rejects a ranked claim. */
export function parseEligibilityReport(value: unknown): EligibilityReport {
  if (!isRecord(value)
    || value.classification !== 'unranked_partial_rules_unverified_authority'
    || !isRecord(value.gates)
    || !Array.isArray(value.reasons)
    || (value.ranked !== true && value.ranked !== false)) {
    throw new Error('eligibility report did not match the expected contract');
  }
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
  if (value.ranked === true
    || !reasons.includes('partial-rules')
    || !reasons.includes('unverified-authority')) {
    throw new Error('eligibility must stay unranked while rules and authority are unverified');
  }
  return Object.freeze({
    classification: 'unranked_partial_rules_unverified_authority',
    gates,
    ranked: false,
    reasons,
  });
}
