# Security Policy

## Supported Versions

`sitepulse` is currently pre-1.0. Security fixes are provided for the latest released version.

| Version | Supported |
| --- | --- |
| 0.1.x | Yes |

## Reporting a Vulnerability

Please report security vulnerabilities privately. Do not open a public GitHub issue for security-sensitive reports.

Use one of the following options:

- GitHub private vulnerability reporting, if enabled for the repository.
- Email the maintainer listed on the GitHub repository profile.

When reporting, please include:

- Affected version or commit.
- Description of the vulnerability.
- Steps to reproduce.
- Potential impact.
- Any suggested fix or mitigation, if available.

## Scope

Security-sensitive areas include, but are not limited to:

- Unsafe handling of URLs or redirects.
- Unexpected file overwrite behavior in export paths.
- Denial-of-service risks from unbounded network responses.
- Incorrect handling of untrusted sitemap, robots.txt, HTML, or JSON content.
- CI/CD or release pipeline issues.
