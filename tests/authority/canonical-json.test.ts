import test from 'node:test';

test.todo('DATA-03 canonical JSON matches RFC 8785 example vectors');
test.todo('DATA-03 canonical JSON orders object keys by UTF-16 code units recursively');
test.todo('DATA-03 canonical JSON uses ECMAScript number serialization');
test.todo('DATA-03 canonical JSON preserves Unicode without normalization');
test.todo('DATA-03 canonical JSON preserves array order');
test.todo('DATA-03 canonical JSON rejects non-finite and unsafe numbers');
test.todo('DATA-03 canonical JSON rejects undefined and unsupported JavaScript values');
test.todo('DATA-03 canonical JSON rejects sparse arrays and unsupported prototypes');
test.todo('DATA-03 canonical JSON rejects duplicate-key JSON before canonicalization');
test.todo('DATA-03 formatting and property insertion order do not change canonical identity');
test.todo('DATA-03 a one-byte source change changes its raw SHA-256');
test.todo('DATA-03 a payload change changes the artifact hash without self-hashing contentHash');
