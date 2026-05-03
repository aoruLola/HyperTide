# Security Policy

HyperTide Community Edition is open source and self-hostable. Please report
suspected vulnerabilities privately so maintainers can validate impact and
coordinate fixes before public disclosure.

## Supported Versions

Security fixes target the latest released `v0.x` line and the current `main`
branch until the project publishes a broader support policy.

## Reporting A Vulnerability

Use GitHub private vulnerability reporting when available. If private reporting
is unavailable for this repository, contact the maintainers through the security
contact listed on the GitHub repository profile.

Include:

- affected component, endpoint, CLI command, or deployment artifact
- reproduction steps
- expected and actual behavior
- impact assessment if known
- relevant logs, request samples, or configuration snippets

Do not include live credentials, customer data, private keys, or unrelated
secrets in reports.

## Response Expectations

Maintainers aim to acknowledge actionable reports within 5 business days, then
triage severity, prepare a fix, and publish release notes or advisories when
appropriate.

## Safe Testing

- Do not run disruptive tests against infrastructure you do not control.
- Do not publish exploit details before a coordinated fix is available.
- Do not attempt to access data that is not yours.
