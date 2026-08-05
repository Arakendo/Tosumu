# Browser Inspection Fixture Matrix v1

These reviewed fixtures support the static-first Tosumu browser inspection lab.
They are intentionally limited to page-zero compatibility evidence and stable
raw-byte rejection behavior. They do not authorize browser page verification,
record inspection, tree traversal, WAL inspection, or protector operations.

| Fixture | Expected boundary result | Reason |
| --- | --- | --- |
| `inspection-header-fixture-v1.tosumu` | Header observation | A fresh compatible Tosumu store. |
| `inspection-populated-fixture-v1.tosumu` | Header observation only | Demonstrates that a compatible container is not equivalent to browser record access. |
| `inspection-invalid-magic-v1.bin` | `FORMAT_NOT_TOSUMU` rejection | Invalid format identity. |
| `inspection-truncated-v1.bin` | `FORMAT_FILE_TRUNCATED` rejection | Input does not contain a complete page zero. |
| `inspection-newer-format-v1.bin` | `FORMAT_VERSION_UNSUPPORTED` rejection | The header declares a newer unsupported format version. |

Run `pwsh -NoProfile -File scripts/build-inspection-fixtures.ps1` from the
repository root to generate the missing derived fixtures. The script refuses
to overwrite reviewed files. The populated fixture is created through the
public Tosumu CLI; the invalid fixtures are derived only to exercise bounded
header validation.
