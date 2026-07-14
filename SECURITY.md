# Security Policy

## Reporting

Do not open a public issue for a vulnerability. Use GitHub private vulnerability reporting for this repository.

Reports should include affected versions, reproduction steps, impact, and any proposed mitigation. Maintainers will acknowledge complete reports as soon as practical.

## Registry trust

Production signing keys are never committed. Clients must verify the detached Registry signature, Pack checksum, and Pack signature before installation.
