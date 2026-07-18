# Credits

This project is built substantially on the work of others. This file is the
human-readable counterpart to the machine-readable SPDX/REUSE metadata (see
[`LICENSES/`](./LICENSES/) and the per-file `SPDX-FileCopyrightText` headers).

## Upstream: wiremix

| Field      | Value                                                        |
|------------|--------------------------------------------------------------|
| Name       | wiremix                                                      |
| Author(s)  | Thomas Sowell and the wiremix contributors                  |
| License    | MIT OR Apache-2.0                                            |
| Source URL | <https://github.com/tsowell/wiremix>                        |
| Scope      | This project is a **fork** of upstream wiremix. The entire original TUI mixer, the `wirehose` PipeWire wrapper, the configuration/theme/char-set system, and the bulk of the codebase originate here. |

Upstream is dual-licensed `MIT OR Apache-2.0`. Per the fork's relicensing
(Spacecraft Software Standard §4.1), the combined work is distributed under
`GPL-3.0-or-later` — a relicensing permitted by the GPLv3-compatibility of both
MIT and Apache-2.0. Upstream copyright notices are preserved verbatim in source
headers, and the upstream license texts are retained in
[`LICENSES/MIT.txt`](./LICENSES/MIT.txt) and
[`LICENSES/Apache-2.0.txt`](./LICENSES/Apache-2.0.txt). Files not substantially
modified by this fork remain under their original `MIT OR Apache-2.0` terms.

## Design Lineage

- **[ncpamixer](https://github.com/fulhax/ncpamixer)** — the upstream wiremix
  interface is closely modeled on ncpamixer.
- **pavucontrol** — the PulseAudio Volume Control that inspired ncpamixer's
  layout and interaction model.

These are credited as design/UX prior art; no ncpamixer or pavucontrol code is
incorporated.

## Standards

This fork conforms to The Steelbore Standard, the Spacecraft Software CLI
Standard (v1.0.0), and the Agentic CLI layer. These are conventions the
project adheres to, not redistributed works.

---

**Maintainer:** Mohamed Hammad &lt;Mohamed.Hammad@SpacecraftSoftware.org&gt;
**Website:** <https://Wiremix.SpacecraftSoftware.org/>
