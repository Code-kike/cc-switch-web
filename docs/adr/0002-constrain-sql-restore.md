---
status: accepted
---

# Constrain SQL restore to CC Switch state

CC Switch SQL backups remain the browser import/export and WebDAV/S3 database format, but restore treats them as a narrow serialization of known product state rather than trusted SQLite programs. Imports must authorize only expected CC Switch schema/data operations, reject attachment and executable/unexpected schema objects, rebuild imported rows into a trusted canonical schema, and pass integrity validation before replacing the live database. This tranche does not disable restore or replace SQL with a structured snapshot because existing local and cloud backups must remain compatible; the trade-off is that every schema evolution must update the restore contract and its adversarial compatibility tests.
