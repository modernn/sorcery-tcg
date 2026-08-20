import test from 'node:test';

test.todo('DATA-02 normalizes the same pinned synthetic card input byte-identically on repeated runs');
test.todo('DATA-02 reordered object properties produce identical canonical card bytes and hashes');
test.todo('DATA-02 retains official source identifiers and deterministic project stable IDs');
test.todo('DATA-02 rejects malformed cards and unknown fields without repair');
test.todo('DATA-02 rejects duplicate stable IDs and printing slugs');
test.todo('DATA-02 rejects invalid source dates and hashes');
test.todo('DATA-02 rejects unexplained input and output count differences');
test.todo('DATA-02 rejects changed input-lock roots before normalization');
test.todo('DATA-02 performs deterministic normalization with networking disabled');
test.todo('DATA-02 reports validation issues with exact deterministic paths');
