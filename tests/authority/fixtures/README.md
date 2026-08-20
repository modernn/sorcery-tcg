# Authority test fixture policy

Authority fixtures must be compact, deterministic, independently authored, and either synthetic or limited to the minimum facts needed by a test. Keep field order and exact bytes stable so hashes remain reproducible.

Do not add publisher PDFs, images, raw API/database corpora, or other copyrighted source collections. Do not copy code, tests, assets, card implementations, or data from Contested Realms (GPL-3.0) or the unlicensed spells.bar/playtest revision. External projects are behavioral references only unless a later recorded permission or product-license decision explicitly changes that boundary.

The bundle input fixture contains one synthetic card, one synthetic format, and the minimum reviewed source metadata needed to exercise the importer. `inputRootHash` is SHA-256 over the canonical JSON encoding of the path-ordered `files` array; it does not hash itself.
