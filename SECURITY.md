# Security policy

The latest 0.1.x release receives security fixes. This project is early-stage; there is no guaranteed response time.

## Report privately

Use [GitHub private vulnerability reporting](https://github.com/M-Umer-Farooq-Dev/actually-done/security/advisories/new) for security-sensitive reports. Include the affected version, operating system, a minimal synthetic reproduction, impact, and any suggested mitigation. Do not include real credentials or private agent conversations. Do not open a public issue for an unpatched vulnerability.

## Trust boundaries

Transcripts are untrusted local input. The CLI never uploads them, but heuristic redaction is incomplete and receipts may contain private material. Review receipts before sharing.

Configured test commands run with your local user permissions and may modify files or access the network. Only use test commands and repository configurations you trust. `--no-test` explicitly waives this execution. A DONE receipt is not a security assessment or proof of correctness.

Dependency alerts and audits supplement review; neither proves the absence of vulnerabilities. Security fixes should include an anonymized regression and the normal CI checks.
