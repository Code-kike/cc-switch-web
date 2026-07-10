# SQL Import Safety Options

## Existing Contract

- Manual browser import/export uses CC Switch SQL dump files.
- WebDAV/S3 synchronization also carries `db.sql`, with selected local-only tables omitted and restored around import.
- Imports execute against a temporary database before SQLite Backup copies the result into the live connection, which protects the live DB from ordinary SQL errors but does not contain SQLite side effects such as `ATTACH` or malicious persistent schema objects.
- The only format check is a forgeable first-line comment.

The accepted unauthenticated API posture intentionally lets reachable operators manage cc-switch data, including import/export. It does not require accepting SQL operations outside the product's tables and restore semantics.

## Option A. Strictly constrained SQL restore (Recommended)

Keep current `.sql` files and cloud-sync compatibility, but treat them as a narrow serialization format rather than trusted programs.

Required controls:

- Register a rusqlite/SQLite authorizer before preparing any uploaded statements.
- Allow only the exact statement classes emitted by `dump_sql`: approved table/index creation, inserts into approved CC Switch tables, transaction boundaries, and narrowly approved `foreign_keys`/`user_version` pragmas.
- Deny `ATTACH`, `DETACH`, temporary objects, triggers, views, virtual tables, unknown actions, unexpected functions/pragmas, and objects outside the schema allowlist.
- Reject unexpected tables, indexes, columns, or schema objects after execution.
- Run migrations, `PRAGMA integrity_check`, foreign-key validation, and product-state validation before replacing the live DB.
- Apply the same validator to browser import and WebDAV/S3 imports.

Trade-off: preserves existing backups and sync artifacts with moderate implementation complexity. The validator becomes a durable schema contract that must be updated when migrations add objects.

## Option B. Disable externally supplied SQL restore

- Remove browser SQL upload and reject remote SQL restore; retain only server-created `.db` backups under the local backup manager.
- Cloud sync would need to be disabled or changed before this fully closes the same attack class, because a compromised remote store can replace `db.sql` and its checksums.

Trade-off: fastest safe boundary for the Web endpoint, but breaks existing import/export and cloud-restore workflows and still requires a replacement sync format.

## Option C. Replace SQL with a structured snapshot format

- The application creates a trusted empty schema and imports typed rows from a versioned JSON/CBOR/archive manifest.
- Schema DDL is never supplied by the backup.
- Provide a one-time legacy SQL converter that runs under the strict Option A validator.

Trade-off: strongest and clearest long-term format, but the largest change. It affects manual import/export, WebDAV, S3, manifests, migrations, UI expectations, and backward compatibility.

## Recommendation

Adopt Option A for the first tranche so existing backups and sync remain usable, while documenting Option C as a possible future format migration. The first implementation must add adversarial tests before accepting the validator as complete.
