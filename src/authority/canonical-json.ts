export type JsonValue =
  | null
  | boolean
  | number
  | string
  | readonly JsonValue[]
  | { readonly [key: string]: JsonValue };

export const MAX_CANONICAL_DEPTH = 64;
export const MAX_CANONICAL_NODES = 100_000;
export const MAX_CANONICAL_STRING_BYTES = 2_000_000;

export type CanonicalJsonErrorCode =
  | 'cyclic_value'
  | 'duplicate_key'
  | 'invalid_json'
  | 'invalid_unicode'
  | 'max_depth'
  | 'max_nodes'
  | 'max_string_bytes'
  | 'non_finite_number'
  | 'sparse_array'
  | 'unsafe_number'
  | 'unsupported_property'
  | 'unsupported_prototype'
  | 'unsupported_type';

export class CanonicalJsonError extends Error {
  readonly code: CanonicalJsonErrorCode;
  readonly path: string;

  constructor(code: CanonicalJsonErrorCode, path: string, detail: string) {
    super(code + ' at ' + (path || '<root>') + ': ' + detail);
    this.name = 'CanonicalJsonError';
    this.code = code;
    this.path = path;
  }
}

type State = {
  nodes: number;
  stringBytes: number;
  readonly stack: WeakSet<object>;
};

function pointer(path: string, segment: string | number): string {
  const escaped = String(segment).replaceAll('~', '~0').replaceAll('/', '~1');
  return path + '/' + escaped;
}

function fail(code: CanonicalJsonErrorCode, path: string, detail: string): never {
  throw new CanonicalJsonError(code, path, detail);
}

function serializeString(value: string, path: string, state: State): string {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!Number.isInteger(next) || next < 0xdc00 || next > 0xdfff) {
        fail('invalid_unicode', path, 'unpaired high surrogate');
      }
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      fail('invalid_unicode', path, 'unpaired low surrogate');
    }
  }

  state.stringBytes += Buffer.byteLength(value, 'utf8');
  if (state.stringBytes > MAX_CANONICAL_STRING_BYTES) {
    fail('max_string_bytes', path, 'canonical strings exceed byte limit');
  }

  return JSON.stringify(value);
}

function serialize(value: unknown, depth: number, path: string, state: State): string {
  if (depth > MAX_CANONICAL_DEPTH) fail('max_depth', path, 'canonical value exceeds depth limit');
  state.nodes += 1;
  if (state.nodes > MAX_CANONICAL_NODES) fail('max_nodes', path, 'canonical value exceeds node limit');

  if (value === null) return 'null';
  if (typeof value === 'boolean') return value ? 'true' : 'false';
  if (typeof value === 'string') return serializeString(value, path, state);
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) fail('non_finite_number', path, 'number must be finite');
    if (Number.isInteger(value) && Math.abs(value) > Number.MAX_SAFE_INTEGER && Math.abs(value) < 1e21) {
      fail('unsafe_number', path, 'integer cannot be represented safely');
    }
    return JSON.stringify(value);
  }

  if (typeof value !== 'object') fail('unsupported_type', path, 'value is not JSON-compatible');

  if (state.stack.has(value)) fail('cyclic_value', path, 'cycle detected');
  state.stack.add(value);
  try {
    if (Array.isArray(value)) {
      if (value.length >= MAX_CANONICAL_NODES) fail('max_nodes', path, 'array exceeds node limit');
      const ownKeys = Reflect.ownKeys(value);
      for (const key of ownKeys) {
        if (key === 'length') continue;
        if (typeof key !== 'string' || !/^0$|^[1-9][0-9]*$/.test(key) || Number(key) >= value.length) {
          fail('unsupported_property', path, 'array has a non-index property');
        }
      }

      const parts: string[] = [];
      for (let index = 0; index < value.length; index += 1) {
        const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
        if (descriptor === undefined) fail('sparse_array', pointer(path, index), 'array slot is missing');
        if (!('value' in descriptor) || !descriptor.enumerable) {
          fail('unsupported_property', pointer(path, index), 'array slot must be an enumerable data property');
        }
        parts.push(serialize(descriptor.value, depth + 1, pointer(path, index), state));
      }
      return '[' + parts.join(',') + ']';
    }

    if (Object.getPrototypeOf(value) !== Object.prototype) {
      fail('unsupported_prototype', path, 'object prototype must be Object.prototype');
    }

    const ownKeys = Reflect.ownKeys(value);
    if (ownKeys.length + state.nodes > MAX_CANONICAL_NODES) {
      fail('max_nodes', path, 'object exceeds node limit');
    }
    const entries: Array<readonly [string, unknown]> = [];
    for (const key of ownKeys) {
      if (typeof key !== 'string') fail('unsupported_property', path, 'symbol keys are not JSON-compatible');
      const childPath = pointer(path, key);
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      if (descriptor === undefined || !('value' in descriptor) || !descriptor.enumerable) {
        fail('unsupported_property', childPath, 'object fields must be enumerable data properties');
      }
      entries.push([key, descriptor.value]);
    }
    entries.sort(([left], [right]) => (left < right ? -1 : left > right ? 1 : 0));
    return (
      '{' +
      entries
        .map(([key, child]) => {
          const childPath = pointer(path, key);
          return serializeString(key, childPath, state) + ':' + serialize(child, depth + 1, childPath, state);
        })
        .join(',') +
      '}'
    );
  } finally {
    state.stack.delete(value);
  }
}

export function canonicalJson(value: JsonValue): string {
  return serialize(value, 0, '', { nodes: 0, stringBytes: 0, stack: new WeakSet<object>() });
}

export function parseJsonWithDuplicateKeyCheck(text: string): JsonValue {
  let index = 0;

  function invalid(path: string): never {
    return fail('invalid_json', path, 'invalid JSON text');
  }

  function skipWhitespace(): void {
    while (index < text.length && /[\t\n\r ]/.test(text[index] ?? '')) index += 1;
  }

  function parseString(path: string): string {
    const start = index;
    if (text[index] !== '"') invalid(path);
    index += 1;
    while (index < text.length) {
      const character = text[index];
      if (character === '"') {
        index += 1;
        try {
          const parsed: unknown = JSON.parse(text.slice(start, index));
          if (typeof parsed !== 'string') invalid(path);
          return parsed;
        } catch {
          invalid(path);
        }
      }
      if (character === '\\') index += 1;
      index += 1;
    }
    return invalid(path);
  }

  function parsePrimitive(path: string): void {
    const start = index;
    while (index < text.length && !/[,\]}\t\n\r ]/.test(text[index] ?? '')) index += 1;
    try {
      const parsed: unknown = JSON.parse(text.slice(start, index));
      if (parsed !== null && typeof parsed === 'object') invalid(path);
    } catch {
      invalid(path);
    }
  }

  function parseValue(depth: number, path: string): void {
    if (depth > MAX_CANONICAL_DEPTH) fail('max_depth', path, 'JSON text exceeds depth limit');
    skipWhitespace();
    const character = text[index];
    if (character === '{') {
      parseObject(depth, path);
    } else if (character === '[') {
      parseArray(depth, path);
    } else if (character === '"') {
      parseString(path);
    } else {
      parsePrimitive(path);
    }
  }

  function parseObject(depth: number, path: string): void {
    index += 1;
    skipWhitespace();
    if (text[index] === '}') {
      index += 1;
      return;
    }

    const keys = new Set<string>();
    while (index < text.length) {
      const key = parseString(path);
      const childPath = pointer(path, key);
      if (keys.has(key)) fail('duplicate_key', childPath, 'object key appears more than once');
      keys.add(key);
      skipWhitespace();
      if (text[index] !== ':') invalid(childPath);
      index += 1;
      parseValue(depth + 1, childPath);
      skipWhitespace();
      if (text[index] === '}') {
        index += 1;
        return;
      }
      if (text[index] !== ',') invalid(path);
      index += 1;
      skipWhitespace();
    }
    invalid(path);
  }

  function parseArray(depth: number, path: string): void {
    index += 1;
    skipWhitespace();
    if (text[index] === ']') {
      index += 1;
      return;
    }

    let itemIndex = 0;
    while (index < text.length) {
      parseValue(depth + 1, pointer(path, itemIndex));
      itemIndex += 1;
      skipWhitespace();
      if (text[index] === ']') {
        index += 1;
        return;
      }
      if (text[index] !== ',') invalid(path);
      index += 1;
    }
    invalid(path);
  }

  parseValue(0, '');
  skipWhitespace();
  if (index !== text.length) invalid('');

  try {
    const parsed = JSON.parse(text) as JsonValue;
    canonicalJson(parsed);
    return parsed;
  } catch (error: unknown) {
    if (error instanceof CanonicalJsonError) throw error;
    return invalid('');
  }
}
