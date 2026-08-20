import test from 'node:test';

test.todo('DATA-03 accepts a strict artifact envelope with stable ID schema version provenance and content hash');
test.todo('DATA-03 accepts stored sources only with their strict stored-byte shape');
test.todo('DATA-03 accepts manifest-only sources only with locator procedure hash and SourceRef binding');
test.todo('DATA-03 accepts reviewed community and external sources as non-normative provenance');
test.todo('DATA-03 prevents non-official authority classes from winning precedence');
test.todo('DATA-03 rejects unknown and missing envelope fields with exact JSON Pointer paths');
test.todo('DATA-03 rejects unsupported schema versions and invalid dates or hashes');
test.todo('DATA-03 rejects duplicate stable IDs and source IDs');
test.todo('DATA-03 rejects artifact tampering and recomputed-hash mismatch');
test.todo('DATA-03 rejects missing or broken parent and source references');
test.todo('DATA-03 rejects reference cycles');
test.todo('DATA-03 rejects relative traversal and absolute stored-source paths');
test.todo('DATA-03 rejects stored-source symlink escape');
test.todo('DATA-03 rehashes stored source bytes during offline validation');
test.todo('DATA-03 rejects manifest-only locator procedure hash byte hash and SourceRef tampering');
test.todo('DATA-03 bounds input bytes records nesting depth and diagnostic count');
test.todo('DATA-03 sorts diagnostics deterministically by JSON Pointer code and message');
